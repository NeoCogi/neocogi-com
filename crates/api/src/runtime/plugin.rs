//! Owned complete plugin records and unregistered-library inspection.
//! Text and dependencies are copied; interface signatures remain in static or
//! pinned process-lived storage. No operation holds a catalog lock in user code.

use std::path::{Path, PathBuf};
use std::string::String;
use std::vec::Vec;

use crate::{NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcPluginDescriptor, NcGuid, NcInterfaceId, NcPluginInfoV1, NcStatus};

use super::{Runtime, read_utf8_view, read_vec, status_to_result};

/// Owned inspection metadata for one registered plugin.
///
/// Look up a plugin GUID returned by loading or enumeration. This also works
/// for static registration; [`PluginOrigin`] explicitly distinguishes its origin.
/// Text, paths and dependency GUIDs are owned copies; the factory signature view
/// remains backed by process-lived contract storage. This is a runtime output,
/// unlike the provider-authored [`NcPluginDescriptor`] registration input.
/// Reading it does not acquire a factory, and changing it does not alter the catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginInfo {
    /// Stable plugin identity.
    pub plugin_id: NcGuid,
    /// Qualified plugin name.
    pub qualified_name: String,
    /// Plugin version text.
    pub version: String,
    /// Static registration or the first successful dynamic library path.
    pub origin: PluginOrigin,
    /// Exact interface and threading contract of the plugin's single factory.
    pub factory: FactoryInfo,
    /// Hard provider GUIDs in declared order, copied from registration storage.
    pub dependencies: Vec<NcGuid>,
}

/// Owned provenance of a registered plugin, independent of native handles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginOrigin {
    /// Metadata and code are supplied by statically linked process storage.
    Static,
    /// Registered from a trusted library; the runtime keeps its mapping pinned.
    Dynamic {
        /// First successful caller path, preserved for diagnostics without reopening.
        path: PathBuf,
    },
}

/// The contract of a plugin's single factory, without acquiring a live object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FactoryInfo {
    /// Exact IID and ABI signature in static or pinned contract storage.
    pub interface: NcInterfaceId,
    /// Validated factory threading capability; typed acquisition enforces it.
    pub threading: crate::NcAbiThreadingMode,
}

/// Stack context that transfers a Rust `FnOnce` result through a C callback.
struct PluginDescriptorVisitContext<F, R> {
    /// Visitor stored in an option so a repeated callback can be rejected.
    visitor: Option<F>,
    /// Value produced synchronously by the visitor on success.
    result: Option<R>,
}

/// Adapts the inspection ABI's plugin-descriptor visitor to a Rust closure.
unsafe extern "C" fn visit_plugin_descriptor<F, R>(plugin: *const NcPluginDescriptor, user_data: *mut core::ffi::c_void) -> NcStatus
where
    F: FnOnce(&NcPluginDescriptor) -> Result<R, NcStatus>,
{
    if plugin.is_null() || user_data.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `user_data` points to the live stack context supplied by
    // `with_plugin_library_descriptor`, and the runtime invokes this callback
    // synchronously before that call returns.
    let context = unsafe { &mut *user_data.cast::<PluginDescriptorVisitContext<F, R>>() };
    let Some(visitor) = context.visitor.take() else {
        return NC_E_INVALID_ARG;
    };
    // SAFETY: The runtime guarantees a validated plugin-descriptor graph for the
    // duration of this callback.
    match visitor(unsafe { &*plugin }) {
        Ok(result) => {
            context.result = Some(result);
            NC_OK
        }
        Err(status) => status.normalize_error(),
    }
}

impl Runtime {
    /// Opens a trusted plugin library and visits its validated plugin descriptor.
    ///
    /// This operation does not register the plugin, construct objects, or
    /// follow its declared plugin dependencies. The descriptor and every
    /// pointer reachable from it are borrowed only for `visitor`; copy any
    /// metadata needed after this method returns. A successfully opened module
    /// remains pinned until process exit, including when later inspection or
    /// `visitor` fails.
    ///
    /// # Errors
    ///
    /// Returns a loader or descriptor-validation status, or the error returned
    /// by `visitor`. A path that cannot be represented as UTF-8 returns
    /// [`NC_E_INVALID_ARG`].
    ///
    /// # Safety
    ///
    /// `path` must identify trusted code. Opening a dynamic library may execute
    /// arbitrary platform loader code. `visitor` must not panic because Rust
    /// panics cannot unwind through the C callback boundary.
    pub unsafe fn with_plugin_library_descriptor<R, F>(&self, path: impl AsRef<Path>, visitor: F) -> Result<R, NcStatus>
    where
        F: FnOnce(&NcPluginDescriptor) -> Result<R, NcStatus>,
    {
        let path = path.as_ref().to_str().ok_or(NC_E_INVALID_ARG)?;
        let mut context = PluginDescriptorVisitContext { visitor: Some(visitor), result: None };
        // SAFETY: The caller accepted the trusted-library requirement. The
        // stack context stays live for the runtime's synchronous callback.
        let status = unsafe {
            (self.runtime_api().inspect_plugin_library_utf8)(
                path.as_ptr(),
                path.len(),
                Some(visit_plugin_descriptor::<F, R>),
                core::ptr::from_mut(&mut context).cast(),
            )
        };
        status_to_result(status)?;
        context.result.ok_or(NC_E_INVALID_ARG)
    }

    /// Returns one complete owned plugin record, or `None` for an unknown GUID.
    /// Metadata lookup never acquires a factory or reopens its provenance path.
    pub fn plugin_info(&self, plugin_id: NcGuid) -> Result<Option<PluginInfo>, NcStatus> {
        self.raw_plugin_info(plugin_id)?.map(owned_plugin_info).transpose()
    }

    /// Returns a coherent snapshot of complete records in registration order.
    /// One successful raw fill supplies every field; registration racing the
    /// preceding count is retried within the facade's allocation/retry bounds.
    pub fn plugins(&self) -> Result<Vec<PluginInfo>, NcStatus> {
        let api = self.runtime_api();
        let records = read_vec(
            || unsafe { (api.plugin_count)() },
            |out, capacity, out_len| unsafe { (api.plugin_infos)(out, capacity, out_len) },
        )?;
        let mut plugins = super::try_vec_with_capacity(records.len())?;
        for record in records {
            plugins.push(owned_plugin_info(record)?);
        }
        Ok(plugins)
    }

    /// Retrieves the complete borrowed record without allocating owned metadata.
    /// Generic acquisition uses this to select the exact factory interface.
    pub(super) fn raw_plugin_info(&self, plugin_id: NcGuid) -> Result<Option<NcPluginInfoV1>, NcStatus> {
        let mut out = core::mem::MaybeUninit::uninit();
        // SAFETY: `out` is writable and initialized by a successful lookup.
        let status = unsafe { (self.runtime_api().plugin_info)(plugin_id, out.as_mut_ptr()) };
        if status == NC_E_NOT_FOUND {
            return Ok(None);
        }
        status_to_result(status)?;
        // SAFETY: Success initialized one complete record, with process-lived views.
        Ok(Some(unsafe { out.assume_init() }))
    }
}

/// Validates scalar/array shape and copies the runtime's process-lived views.
/// Pointer readability comes from the linked runtime contract, not these checks.
fn owned_plugin_info(info: NcPluginInfoV1) -> Result<PluginInfo, NcStatus> {
    let threading = crate::NcAbiThreadingMode::from_raw(info.factory.threading).ok_or(NC_E_INVALID_ARG)?;
    let count = super::checked_enumeration_capacity(info.dependency_count)?;
    if count != 0 && info.dependencies.is_null() {
        return Err(NC_E_POINTER);
    }
    let mut dependencies = super::try_vec_with_capacity(count)?;
    if count != 0 {
        // SAFETY: Runtime records supply the descriptor's immutable, aligned,
        // process-lived dependency table. Destination GUID storage is independent.
        let borrowed = unsafe { core::slice::from_raw_parts(info.dependencies, count) };
        dependencies.extend(borrowed.iter().map(|dependency| dependency.plugin_id));
    }
    Ok(PluginInfo {
        plugin_id: info.plugin_id,
        qualified_name: read_utf8_view(info.qualified_name)?,
        version: read_utf8_view(info.version)?,
        origin: if info.library_path.len == 0 {
            PluginOrigin::Static
        } else {
            PluginOrigin::Dynamic {
                path: PathBuf::from(read_utf8_view(info.library_path)?),
            }
        },
        factory: FactoryInfo {
            interface: info.factory.interface,
            threading,
        },
        dependencies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Reject malformed scalar/count shapes before reading memory or allocating;
    /// empty arrays accept null. These checks do not validate arbitrary pointers.
    #[test]
    fn owned_records_validate_dependency_and_threading_shapes() {
        let mut raw = NcPluginInfoV1 {
            plugin_id: NcGuid::nil(),
            qualified_name: crate::NcUtf8View::EMPTY,
            version: crate::NcUtf8View::EMPTY,
            library_path: crate::NcUtf8View::EMPTY,
            factory: crate::NcFactoryInfoV1 {
                interface: crate::IUnknown::INTERFACE_ID,
                threading: crate::NC_ABI_THREADING_THREAD_LOCAL,
            },
            dependencies: core::ptr::null(),
            dependency_count: 0,
        };
        assert!(owned_plugin_info(raw).unwrap().dependencies.is_empty());
        raw.dependency_count = 1;
        assert_eq!(owned_plugin_info(raw), Err(NC_E_POINTER));
        raw.dependency_count = super::super::NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS + 1;
        assert_eq!(owned_plugin_info(raw), Err(crate::NC_E_OUT_OF_RANGE));
        raw.dependency_count = 0;
        raw.factory.threading = u32::MAX;
        assert_eq!(owned_plugin_info(raw), Err(NC_E_INVALID_ARG));
    }
}
