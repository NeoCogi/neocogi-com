//! Owned plugin identity, provenance, factory metadata and dependency inspection.
//!
//! Raw records borrow immutable descriptor text and catalog-owned library paths.
//! The facade validates and copies those views before returning, so applications
//! own their metadata without retaining a catalog lock or native handle. Missing
//! plugin lookups return `None`; dependency queries return an empty collection
//! for an absent plugin or a plugin with no dependencies. Unregistered trusted
//! libraries have a separate synchronous descriptor-inspection operation.

use std::path::{Path, PathBuf};
use std::string::String;
use std::vec::Vec;

use crate::{
    NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcPluginFactoryInfoV1, NcPluginInfoV1, NcStatus,
};

use super::{Runtime, read_utf8_view, read_vec, status_to_result};

/// Owned descriptor metadata for one registered plugin.
///
/// Look up a plugin GUID returned by loading or enumeration. This also works
/// for static registration; [`PluginOrigin`] explicitly distinguishes its origin.
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

/// Factory metadata inspection summary for one registered plugin.
///
/// Call [`Inspection::plugin_factory_info`] with a plugin ID obtained from the plugin registry
/// or library loading. This lookup works for both dynamically loaded and
/// statically registered plugins.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PluginFactoryInfo {
    /// Stable plugin identity.
    pub plugin_id: NcGuid,
    /// Exact root factory interface identity.
    pub root_factory: NcInterfaceId,
    /// Threading capability declared by the root factory.
    pub root_factory_threading: u32,
}

/// Read-only plugin identity, registration provenance and dependency queries.
#[derive(Clone, Copy, Debug)]
pub struct Inspection<'runtime> {
    /// Facade used to retrieve and validate the inspection ABI table.
    runtime: &'runtime Runtime,
}

/// Stack context that transfers a Rust `FnOnce` result through a C callback.
struct FactoryDescriptionVisitContext<F, R> {
    /// Visitor stored in an option so a repeated callback can be rejected.
    visitor: Option<F>,
    /// Value produced synchronously by the visitor on success.
    result: Option<R>,
}

/// Adapts the inspection ABI's factory-description visitor to a Rust closure.
unsafe extern "C" fn visit_factory_plugin_description<F, R>(plugin: *const NcFactoryPluginDescription, user_data: *mut core::ffi::c_void) -> NcStatus
where
    F: FnOnce(&NcFactoryPluginDescription) -> Result<R, NcStatus>,
{
    if plugin.is_null() || user_data.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `user_data` points to the live stack context supplied by
    // `with_plugin_library_description`, and the runtime invokes this callback
    // synchronously before that call returns.
    let context = unsafe { &mut *user_data.cast::<FactoryDescriptionVisitContext<F, R>>() };
    let Some(visitor) = context.visitor.take() else {
        return NC_E_INVALID_ARG;
    };
    // SAFETY: The runtime guarantees a validated factory-description graph for the
    // duration of this callback.
    match visitor(unsafe { &*plugin }) {
        Ok(result) => {
            context.result = Some(result);
            NC_OK
        }
        Err(status) => status.normalize_error(),
    }
}

impl<'runtime> Inspection<'runtime> {
    /// Creates an inspection context borrowing the runtime facade.
    pub(super) const fn new(runtime: &'runtime Runtime) -> Self {
        Self { runtime }
    }

    /// Opens a trusted plugin library and visits its validated factory description.
    ///
    /// This operation does not register the plugin, construct objects, or
    /// follow its declared plugin dependencies. The description and every
    /// pointer reachable from it are borrowed only for `visitor`; copy any
    /// metadata needed after this method returns. A successfully opened module
    /// remains pinned until process exit, including when later inspection or
    /// `visitor` fails.
    ///
    /// # Errors
    ///
    /// Returns a loader or description-validation status, or the error returned
    /// by `visitor`. A path that cannot be represented as UTF-8 returns
    /// [`NC_E_INVALID_ARG`].
    ///
    /// # Safety
    ///
    /// `path` must identify trusted code. Opening a dynamic library may execute
    /// arbitrary platform loader code. `visitor` must not panic because Rust
    /// panics cannot unwind through the C callback boundary.
    pub unsafe fn with_plugin_library_description<R, F>(self, path: impl AsRef<Path>, visitor: F) -> Result<R, NcStatus>
    where
        F: FnOnce(&NcFactoryPluginDescription) -> Result<R, NcStatus>,
    {
        let path = path.as_ref().to_str().ok_or(NC_E_INVALID_ARG)?;
        let mut context = FactoryDescriptionVisitContext { visitor: Some(visitor), result: None };
        // SAFETY: The caller accepted the trusted-library requirement. The
        // stack context stays live for the runtime's synchronous callback.
        let status = unsafe {
            (self.runtime.inspection_api().inspect_plugin_library_utf8)(
                path.as_ptr(),
                path.len(),
                Some(visit_factory_plugin_description::<F, R>),
                core::ptr::from_mut(&mut context).cast(),
            )
        };
        status_to_result(status)?;
        context.result.ok_or(NC_E_INVALID_ARG)
    }

    /// Returns owned descriptor metadata for one registered plugin.
    pub fn plugin_info(self, plugin_id: NcGuid) -> Result<Option<PluginInfo>, NcStatus> {
        let Some(info) = self.raw_plugin_info(plugin_id)? else {
            return Ok(None);
        };
        Ok(Some(PluginInfo {
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
        }))
    }

    /// Returns factory metadata for one registered plugin.
    pub fn plugin_factory_info(self, plugin_id: NcGuid) -> Result<Option<PluginFactoryInfo>, NcStatus> {
        let Some(info) = self.raw_plugin_factory_info(plugin_id)? else {
            return Ok(None);
        };
        Ok(Some(PluginFactoryInfo {
            plugin_id: info.plugin_id,
            root_factory: info.root_factory,
            root_factory_threading: info.root_factory_threading,
        }))
    }

    /// Returns hard dependency plugin IDs declared by one registered plugin.
    pub fn plugin_dependency_ids(self, plugin_id: NcGuid) -> Result<Vec<NcGuid>, NcStatus> {
        let api = self.runtime.inspection_api();
        read_vec(
            || unsafe { (api.plugin_dependency_count)(plugin_id) },
            |out, capacity, out_len| unsafe { (api.plugin_dependency_ids)(plugin_id, out, capacity, out_len) },
        )
    }

    /// Retrieves one borrowed-text plugin record from the raw inspection table.
    fn raw_plugin_info(self, plugin_id: NcGuid) -> Result<Option<NcPluginInfoV1>, NcStatus> {
        // MaybeUninit avoids constructing a fake sentinel record; the C contract
        // initializes the slot only when it returns success.
        let mut out = core::mem::MaybeUninit::uninit();
        // SAFETY: `out` is valid for one initialized record on success.
        let status = unsafe { (self.runtime.inspection_api().plugin_info)(plugin_id, out.as_mut_ptr()) };
        if status == NC_E_NOT_FOUND {
            return Ok(None);
        }
        status_to_result(status)?;
        // SAFETY: Successful inspection initialized `out`.
        Ok(Some(unsafe { out.assume_init() }))
    }

    /// Retrieves one factory inspection record from the raw inspection table.
    fn raw_plugin_factory_info(self, plugin_id: NcGuid) -> Result<Option<NcPluginFactoryInfoV1>, NcStatus> {
        let mut out = core::mem::MaybeUninit::uninit();
        // SAFETY: `out` is valid for one initialized record on success.
        let status = unsafe { (self.runtime.inspection_api().plugin_factory_info)(plugin_id, out.as_mut_ptr()) };
        if status == NC_E_NOT_FOUND {
            return Ok(None);
        }
        status_to_result(status)?;
        // SAFETY: Successful inspection initialized `out`.
        Ok(Some(unsafe { out.assume_init() }))
    }
}
