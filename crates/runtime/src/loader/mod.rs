//! Trusted dynamic-library activation and module lifetime ownership.
//!
//! Loading has four ordered phases: open the native module, resolve its standard
//! factory-description entrypoint, build one validated registration batch,
//! then commit that batch atomically with the retained handle and module
//! provenance.
//! Failed post-open live activation pins the handle before the public ABI emits
//! its diagnostic and aborts. Successful activation associates an append-only
//! process-local module ID and diagnostic path with the atomically registered
//! plugin; inspection uses the same symbol and validation primitives without
//! committing registry state.
//!
//! Live activation is non-nestable. Native loader initialization and descriptor
//! entrypoints must not directly or indirectly start another neocogi-com activation.
//! The runtime invokes no plugin code while holding its catalog write lock.

use core::ffi::c_void;
use std::path::{Path, PathBuf};

use neocogi_com_core::{
    NC_COM_RUNTIME_MAX_LOADED_MODULES, NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_GET_FACTORY_PLUGIN_DESCRIPTION_SYMBOL, NcFactoryPluginDescription,
    NcGetFactoryPluginDescription, NcGuid,
};

use crate::factory_registration::{FactoryPluginRegistrationBatch, build_factory_plugin_registration_batch};
use crate::state::RuntimeState;

mod error;
mod inspection;
mod library;
mod plan;

pub use error::{PluginLoadError, PluginLoadErrorKind};
pub(crate) use inspection::inspect_factory_plugin_description;
pub use inspection::{
    ContractDeclarationInspection, ContractDeclarationKind, ContractModuleInspection, ContractReferenceInspection, FactoryInspection, PluginInspection,
    inspect_plugin_library,
};
pub use plan::{PluginDependencyPlanError, plan_plugin_load_order};

/// Process-local identifier for a plugin library retained by the runtime.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LoadedModuleId(u64);

impl LoadedModuleId {
    /// Wraps a nonzero ID previously allocated by the runtime.
    pub(crate) const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the process-local numeric module identifier.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl core::fmt::Display for LoadedModuleId {
    /// Formats the process-local numeric identifier.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

/// Registry metadata for a plugin library retained by the runtime.
///
/// Plugin name and version remain authoritative in the registered descriptor
/// and are projected through plugin inspection instead of being copied here.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct LoadedModuleInfo {
    /// Monotonic process-local module identifier.
    module_id: LoadedModuleId,
    /// Exact caller path retained for diagnostics and inspection.
    path: PathBuf,
    /// Plugin identity projected from the validated registration batch.
    plugin_id: NcGuid,
}

impl LoadedModuleInfo {
    /// Creates retained provenance for one successfully activated module.
    pub(crate) fn new(module_id: LoadedModuleId, path: PathBuf, plugin_id: NcGuid) -> Self {
        Self { module_id, path, plugin_id }
    }

    /// Returns the process-local loaded module identifier.
    pub fn module_id(&self) -> LoadedModuleId {
        self.module_id
    }

    /// Returns the loaded library path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the plugin identifier from the descriptor.
    pub fn plugin_id(&self) -> NcGuid {
        self.plugin_id
    }
}

/// Owned native library handle whose mapping backs registered metadata pointers.
pub(crate) struct LoadedModule {
    // Dropping this field invokes the platform unload operation. Successful
    // modules are moved into PluginRegistry so that cannot happen prematurely.
    _library: Option<library::DynamicLibrary>,
}

impl LoadedModule {
    /// Opens one trusted path and classifies platform failures.
    fn open(path: &Path) -> Result<Self, PluginLoadError> {
        // Live activation and offline inspection decide separately whether the
        // opened handle is committed or pinned until process termination.
        let library = library::DynamicLibrary::open(path).map_err(|error| PluginLoadError::new(error.kind, path, None, error.message))?;
        Ok(Self { _library: Some(library) })
    }

    #[cfg(test)]
    /// Creates a handle-free module token for catalog publication tests.
    pub(crate) fn test_stub() -> Self {
        Self { _library: None }
    }

    /// Resolves and types the required factory plugin description entrypoint.
    unsafe fn get_factory_plugin_description(&self, path: &Path) -> Result<NcGetFactoryPluginDescription, PluginLoadError> {
        // SAFETY: The caller selected trusted neocogi-com plugin code and the
        // returned address is called with the exact public factory ABI type.
        let symbol = unsafe {
            self._library
                .as_ref()
                .expect("opened modules own a native handle")
                .symbol(NC_GET_FACTORY_PLUGIN_DESCRIPTION_SYMBOL.as_bytes())
        }
        .map_err(|message| PluginLoadError::new(PluginLoadErrorKind::FindSymbol, path, None, message))?;

        // SAFETY: The symbol name uniquely identifies this function signature.
        Ok(unsafe { core::mem::transmute::<*mut c_void, NcGetFactoryPluginDescription>(symbol) })
    }
}

impl RuntimeState {
    /// Loads a factory plugin shared library and registers its description.
    ///
    /// On success, the process-global runtime stores the loaded library and
    /// resolves the required factory-only IDL description and associates it
    /// with a process-local [`LoadedModuleId`]. A library that exports only the
    /// historical class-table symbol is rejected as an ordinary symbol error.
    /// At most [`NC_COM_RUNTIME_MAX_LOADED_MODULES`] dynamic modules may be
    /// retained; exhaustion is returned to the public fail-fatal C boundary.
    ///
    /// There is intentionally no explicit plugin unload API. Successful loads
    /// are stored in the process-global runtime and remain mapped until process
    /// exit. A post-open failure pins the module; the public live-activation ABI
    /// emits the diagnostic and aborts instead of returning to the host.
    ///
    /// # Safety
    ///
    /// `path` must identify a trusted neocogi-com plugin library. Loading a dynamic
    /// library can run arbitrary platform loader code, and the exported
    /// descriptor symbol is trusted FFI metadata until validation rejects it.
    /// That code must not directly or indirectly start nested neocogi-com activation.
    pub unsafe fn load_plugin_library(&self, path: impl AsRef<Path>) -> Result<LoadedModuleId, PluginLoadError> {
        let path = path.as_ref();
        let mut module = Some(LoadedModule::open(path)?);
        let activation = (|| {
            let loaded = module.as_ref().expect("opened module remains owned");
            // SAFETY: The caller selected trusted plugin code. Live activation
            // recognizes only the factory description symbol.
            let get = unsafe { loaded.get_factory_plugin_description(path)? };
            // SAFETY: The resolved entrypoint remains backed by `loaded`
            // throughout validation and atomic registration.
            let plugin = unsafe { factory_plugin_description_from_entrypoint(get, path)? };
            // SAFETY: The module retains description and contract code.
            let batch = unsafe { factory_plugin_registration_batch_from_description(path, plugin)? };
            register_loaded_factory_plugin(self, path, &mut module, batch)
        })();

        match activation {
            Ok(module_id) => Ok(module_id),
            Err(error) => {
                // Live activation is process-fatal at its public C boundary.
                // Pin an opened module before returning the diagnostic to that
                // boundary so no code is unmapped before it emits and aborts.
                core::mem::forget(module.take().expect("failed activation owns its module"));
                Err(error)
            }
        }
    }
}

/// Commits one validated factory plugin and transfers its module handle.
fn register_loaded_factory_plugin(
    runtime: &RuntimeState,
    path: &Path,
    module: &mut Option<LoadedModule>,
    batch: FactoryPluginRegistrationBatch,
) -> Result<LoadedModuleId, PluginLoadError> {
    let mut catalog = runtime.write_catalog();
    let module_id = allocate_loaded_module_id(&mut catalog, path)?;
    let plugin_id = batch.plugin_id();
    let status = catalog.commit_factory_plugin_registration(batch);
    if status.is_err() {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Registration,
            path,
            Some(status),
            "runtime registration rejected the factory plugin description",
        ));
    }
    let loaded = LoadedModuleInfo::new(module_id, path.to_path_buf(), plugin_id);
    catalog
        .module_registry
        .store_loaded_module(module.take().expect("successful activation owns its module"), loaded);
    Ok(module_id)
}

/// Reserves one process-local handle ID from the common loaded-module store.
fn allocate_loaded_module_id(catalog: &mut crate::state::RuntimeCatalog, path: &Path) -> Result<LoadedModuleId, PluginLoadError> {
    catalog.module_registry.allocate_loaded_module_id().ok_or_else(|| {
        PluginLoadError::new(
            PluginLoadErrorKind::ResourceLimit,
            path,
            Some(NC_E_OUT_OF_RANGE),
            format!("runtime loaded-module limit of {NC_COM_RUNTIME_MAX_LOADED_MODULES} reached"),
        )
    })
}

/// Invokes one resolved factory-only description entrypoint.
unsafe fn factory_plugin_description_from_entrypoint(
    get_description: NcGetFactoryPluginDescription,
    path: &Path,
) -> Result<*const NcFactoryPluginDescription, PluginLoadError> {
    let mut plugin = core::ptr::null();
    // SAFETY: The resolved ABI receives one valid output pointer slot.
    let status = unsafe { get_description(&mut plugin) };
    if status.is_err() {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Entrypoint,
            path,
            Some(status),
            "factory plugin description entrypoint returned an error",
        ));
    }
    if plugin.is_null() {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Entrypoint,
            path,
            Some(NC_E_POINTER),
            "factory plugin description entrypoint returned null",
        ));
    }
    Ok(plugin)
}

/// Validates and projects one loaded factory description into owned records.
unsafe fn factory_plugin_registration_batch_from_description(
    path: &Path,
    plugin: *const NcFactoryPluginDescription,
) -> Result<FactoryPluginRegistrationBatch, PluginLoadError> {
    // SAFETY: The pointer and contract getters remain backed by `module` during
    // this call and by retained ownership after a successful commit.
    unsafe { build_factory_plugin_registration_batch(plugin) }.map_err(|status| {
        PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(status),
            "factory plugin description failed load validation",
        )
    })
}
