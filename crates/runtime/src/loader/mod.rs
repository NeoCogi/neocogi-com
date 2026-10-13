//! Trusted native library loading and plugin publication.
//!
//! Native open, descriptor entry and metadata validation run before the catalog
//! lock. Publication commits descriptor, dependencies and first library path in
//! one plugin record. Reopening the same descriptor returns its plugin GUID;
//! conflicting providers are rejected. Native mappings remain process-pinned,
//! including inspection-only opens and failures after opening. No public library
//! or module identity is allocated.
//!
//! Live loading remains non-nestable. Loader initializers and descriptor
//! entrypoints must not start another COM load. No plugin code runs under the
//! catalog write lock.

use core::ffi::c_void;
use std::path::Path;

use neocogi_com_core::{NC_E_POINTER, NC_GET_FACTORY_PLUGIN_DESCRIPTION_SYMBOL, NcFactoryPluginDescription, NcGetFactoryPluginDescription, NcGuid};

use crate::factory_plugin_registry::{PluginOrigin, RegisteredFactoryPlugin};
use crate::factory_registration::prepare_factory_plugin;
use crate::state::RuntimeState;

mod error;
mod inspection;
mod library;
mod plan;

pub use error::{PluginLoadError, PluginLoadErrorKind};
pub(crate) use inspection::inspect_factory_plugin_description;
pub use inspection::{FactoryInspection, PluginInspection, inspect_plugin_library};
pub use plan::{PluginDependencyPlanError, plan_plugin_load_order};

/// Process-pinned native library handle whose mapping backs registered metadata pointers.
pub(crate) struct PinnedLibrary {
    /// Native handle retained privately; its mapping is never unloaded.
    _library: Option<library::DynamicLibrary>,
}

impl PinnedLibrary {
    /// Opens one trusted path and classifies platform failures.
    fn open(path: &Path) -> Result<Self, PluginLoadError> {
        // Live activation and offline inspection decide separately whether the
        // opened handle is committed or pinned until process termination.
        let library = library::DynamicLibrary::open(path).map_err(|error| PluginLoadError::new(error.kind, path, None, error.message))?;
        Ok(Self { _library: Some(library) })
    }

    #[cfg(test)]
    /// Creates a handle-free library token for catalog publication tests.
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
                .expect("opened libraries own a native handle")
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
    /// Returns the plugin GUID after atomically publishing its descriptor and
    /// library provenance. The same descriptor reuses the first registration;
    /// another descriptor claiming its identity is rejected. Distinct dynamic
    /// providers are limited by `NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS`.
    /// Capacity errors reach the public mandatory-load boundary and abort.
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
    pub unsafe fn load_plugin_library(&self, path: impl AsRef<Path>) -> Result<NcGuid, PluginLoadError> {
        let path = path.as_ref();
        let library = PinnedLibrary::open(path)?;
        // SAFETY: The caller selects trusted code; the opened library stays
        // process-pinned even when symbol resolution or publication fails.
        let get = unsafe { library.get_factory_plugin_description(path)? };
        // SAFETY: The pinned mapping backs the descriptor and its bootstrap.
        let description = unsafe { factory_plugin_description_from_entrypoint(get, path)? };
        // SAFETY: The entrypoint promises immutable process-lifetime metadata.
        let mut plugin = unsafe { prepare_loaded_factory_plugin(path, description)? };
        let plugin_id = plugin.plugin_id;
        plugin.origin = PluginOrigin::Dynamic {
            path: path.to_path_buf(),
            _library: library,
        };
        // Descriptor, dependencies and provenance enter the catalog together.
        // Native loading and metadata validation above invoke no code under this lock.
        let status = self.write_catalog().commit_factory_plugin_registration(plugin);
        if status.is_err() {
            return Err(PluginLoadError::new(
                PluginLoadErrorKind::Registration,
                path,
                Some(status),
                "runtime registration rejected the factory plugin description",
            ));
        }
        Ok(plugin_id)
    }
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
unsafe fn prepare_loaded_factory_plugin(path: &Path, plugin: *const NcFactoryPluginDescription) -> Result<RegisteredFactoryPlugin, PluginLoadError> {
    // SAFETY: The descriptor and its referenced data remain backed by `module` during
    // this call and by retained ownership after a successful commit.
    unsafe { prepare_factory_plugin(plugin) }.map_err(|status| {
        PluginLoadError::new(
            PluginLoadErrorKind::Metadata,
            path,
            Some(status),
            "factory plugin description failed load validation",
        )
    })
}
