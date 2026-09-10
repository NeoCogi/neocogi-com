//! Safe owned metadata and relationship inspection.
//!
//! Raw inspection records contain borrowed UTF-8 pointers. Public methods copy
//! those views before returning, so callers never inherit a C lifetime or need
//! to hold a runtime lock. Module records own provenance (module ID, plugin ID,
//! and path); plugin descriptors remain the authoritative source for plugin
//! names and versions. [`Inspection::loaded_module`] joins both records into an
//! owned [`LoadedModuleInfo`]. Missing record lookups return `None`;
//! relationship queries return an empty collection when no edge exists.
//! Trusted, unregistered libraries can be opened and validated through a
//! callback-scoped descriptor visitor.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::string::String;
use std::vec::Vec;

use crate::{
    NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcAbiThreadingMode, NcClassInfoV1, NcAbiSignature, NcGuid, NcInterfaceId, NcModuleInfoV1,
    NcPluginDescriptor, NcPluginInfoV1, NcStatus,
};

use super::{LoadedModuleId, LoadedModuleInfo, Runtime, read_utf8_view, read_vec, status_to_result};

/// Owned descriptor metadata for one registered plugin.
///
/// Call [`Inspection::plugin_info`] with a plugin ID obtained from a registry,
/// class, or loaded-module record. This lookup also works for statically
/// registered plugins, which have no module path or [`LoadedModuleId`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginInfo {
    /// Stable plugin identity.
    pub plugin_id: NcGuid,
    /// Qualified plugin name.
    pub qualified_name: String,
    /// Plugin version text.
    pub version: String,
}

/// Owned metadata and provenance for one registered class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassInfo {
    /// Stable class identity.
    pub class_id: NcGuid,
    /// Plugin that registered the class.
    pub plugin_id: NcGuid,
    /// Loaded module that supplied the class, or `None` for static registration.
    pub module_id: Option<LoadedModuleId>,
    /// Qualified class name.
    pub qualified_name: String,
    /// Class version text.
    pub version: String,
    /// Constructor-frame ABI signature, or [`NcAbiSignature::EMPTY`] for no arguments.
    pub constructor_abi_signature: NcAbiSignature,
    /// Threading capability promised by constructed objects.
    pub threading: NcAbiThreadingMode,
}

/// Read-only metadata and relationship queries for the runtime state.
#[derive(Clone, Copy, Debug)]
pub struct Inspection<'runtime> {
    /// Facade used to retrieve and validate the inspection ABI table.
    runtime: &'runtime Runtime,
}

/// Stack context that transfers a Rust `FnOnce` result through a C callback.
struct DescriptorVisitContext<F, R> {
    /// Visitor stored in an option so a repeated callback can be rejected.
    visitor: Option<F>,
    /// Value produced synchronously by the visitor on success.
    result: Option<R>,
}

/// Adapts the inspection ABI's descriptor visitor to a Rust `FnOnce` closure.
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
    let context = unsafe { &mut *user_data.cast::<DescriptorVisitContext<F, R>>() };
    let Some(visitor) = context.visitor.take() else {
        return NC_E_INVALID_ARG;
    };
    // SAFETY: The runtime guarantees a validated descriptor graph for the
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

    /// Opens a trusted plugin library and visits its validated descriptor.
    ///
    /// This operation does not register the plugin, construct objects, or
    /// follow its declared neocogi-com dependencies. The descriptor and every
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
    pub unsafe fn with_plugin_library_descriptor<R, F>(self, path: impl AsRef<Path>, visitor: F) -> Result<R, NcStatus>
    where
        F: FnOnce(&NcPluginDescriptor) -> Result<R, NcStatus>,
    {
        let path = path.as_ref().to_str().ok_or(NC_E_INVALID_ARG)?;
        let mut context = DescriptorVisitContext { visitor: Some(visitor), result: None };
        // SAFETY: The caller accepted the trusted-library requirement. The
        // stack context stays live for the runtime's synchronous callback.
        let status = unsafe {
            (self.runtime.inspection_api().inspect_plugin_library_utf8)(
                path.as_ptr(),
                path.len(),
                Some(visit_plugin_descriptor::<F, R>),
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
        }))
    }

    /// Returns owned metadata and provenance for one registered class.
    pub fn class_info(self, class_id: NcGuid) -> Result<Option<ClassInfo>, NcStatus> {
        let Some(info) = self.raw_class_info(class_id)? else {
            return Ok(None);
        };
        let threading = NcAbiThreadingMode::from_raw(info.threading).ok_or(NC_E_INVALID_ARG)?;
        Ok(Some(ClassInfo {
            class_id: info.class_id,
            plugin_id: info.plugin_id,
            module_id: (info.module_id != 0).then(|| LoadedModuleId::from_raw(info.module_id)),
            qualified_name: read_utf8_view(info.qualified_name)?,
            version: read_utf8_view(info.version)?,
            constructor_abi_signature: info.constructor_abi_signature,
            threading,
        }))
    }

    /// Returns retained plugin modules in registry order.
    ///
    /// Each owned summary combines module provenance with the corresponding
    /// descriptor-backed plugin name and version.
    pub fn loaded_modules(self) -> Result<Vec<LoadedModuleInfo>, NcStatus> {
        // Resolve IDs one at a time through inspection so the returned summaries
        // are owned and the plugin registry remains only an existence catalog.
        self.runtime
            .plugin_registry()
            .loaded_module_ids()?
            .into_iter()
            .map(|module_id| self.loaded_module_required(module_id))
            .collect()
    }

    /// Returns one retained plugin module.
    ///
    /// Module inspection supplies the path and plugin ID; plugin inspection
    /// supplies the qualified name and version. The returned summary owns all
    /// text from both records.
    pub fn loaded_module(self, module_id: LoadedModuleId) -> Result<Option<LoadedModuleInfo>, NcStatus> {
        match self.loaded_module_required(module_id) {
            Ok(module) => Ok(Some(module)),
            Err(NC_E_NOT_FOUND) => Ok(None),
            Err(status) => Err(status),
        }
    }

    /// Returns the loaded module ID that supplied a class, or `None` for a static
    /// registration or a missing class.
    pub fn module_id_for_class(self, class_id: NcGuid) -> Result<Option<LoadedModuleId>, NcStatus> {
        Ok(self.class_info(class_id)?.and_then(|info| info.module_id))
    }

    /// Returns registered class IDs supplied by one loaded module.
    pub fn class_ids_in_module(self, module_id: LoadedModuleId) -> Result<Vec<NcGuid>, NcStatus> {
        let mut result = Vec::new();
        for class_id in self.runtime.type_registry().class_ids()? {
            if self.raw_class_info(class_id)?.is_some_and(|info| info.module_id == module_id.get()) {
                result.push(class_id);
            }
        }
        Ok(result)
    }

    /// Returns distinct interface ABI identities exposed by one loaded module.
    pub fn interface_ids_in_module(self, module_id: LoadedModuleId) -> Result<Vec<NcInterfaceId>, NcStatus> {
        let mut result = Vec::new();
        let mut known = HashSet::new();
        for class_id in self.class_ids_in_module(module_id)? {
            for interface_id in self.class_interface_ids(class_id)? {
                if known.insert(interface_id) {
                    result.push(interface_id);
                }
            }
        }
        Ok(result)
    }

    /// Returns IDs of classes registered by one plugin.
    pub fn plugin_class_ids(self, plugin_id: NcGuid) -> Result<Vec<NcGuid>, NcStatus> {
        let api = self.runtime.inspection_api();
        read_vec(
            || unsafe { (api.plugin_class_count)(plugin_id) },
            |out, capacity, out_len| unsafe { (api.plugin_class_ids)(plugin_id, out, capacity, out_len) },
        )
    }

    /// Returns hard dependency plugin IDs declared by one registered plugin.
    pub fn plugin_dependency_ids(self, plugin_id: NcGuid) -> Result<Vec<NcGuid>, NcStatus> {
        let api = self.runtime.inspection_api();
        read_vec(
            || unsafe { (api.plugin_dependency_count)(plugin_id) },
            |out, capacity, out_len| unsafe { (api.plugin_dependency_ids)(plugin_id, out, capacity, out_len) },
        )
    }

    /// Returns interface ABI identities implemented by one class.
    pub fn class_interface_ids(self, class_id: NcGuid) -> Result<Vec<NcInterfaceId>, NcStatus> {
        let api = self.runtime.inspection_api();
        read_vec(
            || unsafe { (api.class_interface_count)(class_id) },
            |out, capacity, out_len| unsafe { (api.class_interface_ids)(class_id, out, capacity, out_len) },
        )
    }

    /// Returns IDs of classes implementing one interface ABI identity.
    pub fn interface_implementor_class_ids(self, interface_id: NcInterfaceId) -> Result<Vec<NcGuid>, NcStatus> {
        let api = self.runtime.inspection_api();
        read_vec(
            || unsafe { (api.interface_implementor_class_count)(interface_id) },
            |out, capacity, out_len| unsafe { (api.interface_implementor_class_ids)(interface_id, out, capacity, out_len) },
        )
    }

    /// Joins a required raw module record with its plugin descriptor metadata.
    fn loaded_module_required(self, module_id: LoadedModuleId) -> Result<LoadedModuleInfo, NcStatus> {
        // A loaded module and its plugin record are committed together. Treat a
        // missing half as NC_E_NOT_FOUND rather than manufacturing partial data.
        let module = self.raw_module_info(module_id)?.ok_or(NC_E_NOT_FOUND)?;
        let plugin = self.plugin_info(module.plugin_id)?.ok_or(NC_E_NOT_FOUND)?;
        Ok(LoadedModuleInfo {
            module_id,
            path: PathBuf::from(read_utf8_view(module.path)?),
            plugin_id: module.plugin_id,
            qualified_name: plugin.qualified_name,
            plugin_version: plugin.version,
        })
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

    /// Retrieves one borrowed-text module record from the raw inspection table.
    fn raw_module_info(self, module_id: LoadedModuleId) -> Result<Option<NcModuleInfoV1>, NcStatus> {
        let mut out = core::mem::MaybeUninit::uninit();
        // SAFETY: `out` is valid for one initialized record on success.
        let status = unsafe { (self.runtime.inspection_api().module_info)(module_id.get(), out.as_mut_ptr()) };
        if status == NC_E_NOT_FOUND {
            return Ok(None);
        }
        status_to_result(status)?;
        // SAFETY: Successful inspection initialized `out`.
        Ok(Some(unsafe { out.assume_init() }))
    }

    /// Retrieves one borrowed-text class record from the raw inspection table.
    fn raw_class_info(self, class_id: NcGuid) -> Result<Option<NcClassInfoV1>, NcStatus> {
        let mut out = core::mem::MaybeUninit::uninit();
        // SAFETY: `out` is valid for one initialized record on success.
        let status = unsafe { (self.runtime.inspection_api().class_info)(class_id, out.as_mut_ptr()) };
        if status == NC_E_NOT_FOUND {
            return Ok(None);
        }
        status_to_result(status)?;
        // SAFETY: Successful inspection initialized `out`.
        Ok(Some(unsafe { out.assume_init() }))
    }
}
