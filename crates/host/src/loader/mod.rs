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

use neocogi_com_core::{NC_E_POINTER, NC_GET_PLUGIN_DESCRIPTOR_SYMBOL, NcPluginDescriptor, NcGetPluginDescriptor, NcGuid};

use crate::factory_plugin_registry::{PluginOrigin, RegisteredFactoryPlugin};
use crate::descriptor::ValidatedPluginDescriptor;
use crate::state::RuntimeState;

mod error;
mod inspection;
mod library;
mod plan;

pub use error::{PluginLoadError, PluginLoadErrorKind};
pub use inspection::{FactoryInspection, PluginInspection, inspect_plugin_library};
pub use plan::{PluginDependencyPlanError, plan_plugin_load_order};

/// Open handle for symbol resolution in a process-pinned native mapping.
/// The platform wrapper never unloads on drop; registration retains metadata,
/// while failed/repeated loads and inspection have the same mapping lifetime.
pub(crate) struct PinnedLibrary {
    /// Native handle retained privately; its mapping is never unloaded.
    library: library::DynamicLibrary,
}

impl PinnedLibrary {
    /// Opens one trusted path and classifies platform failures.
    fn open(path: &Path) -> Result<Self, PluginLoadError> {
        // Both live loading and inspection pin every successfully opened mapping;
        // only live loading additionally publishes a plugin record.
        let library = library::DynamicLibrary::open(path).map_err(|error| PluginLoadError::new(error.kind, path, None, error.message))?;
        Ok(Self { library })
    }

    /// Resolves and types the required plugin descriptor entrypoint.
    unsafe fn get_plugin_descriptor(&self, path: &Path) -> Result<NcGetPluginDescriptor, PluginLoadError> {
        // SAFETY: The caller selected trusted neocogi-com plugin code and the
        // returned address is called with the exact public plugin-descriptor ABI type.
        let symbol = unsafe { self.library.symbol(NC_GET_PLUGIN_DESCRIPTOR_SYMBOL.as_bytes()) }
            .map_err(|message| PluginLoadError::new(PluginLoadErrorKind::FindSymbol, path, None, message))?;

        // SAFETY: The symbol name uniquely identifies this function signature.
        Ok(unsafe { core::mem::transmute::<*mut c_void, NcGetPluginDescriptor>(symbol) })
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
    /// exit. A post-open failure pins the module; the public live-loading ABI
    /// emits the diagnostic and aborts instead of returning to the host.
    ///
    /// # Safety
    ///
    /// `path` must identify a trusted neocogi-com plugin library. Loading a dynamic
    /// library can run arbitrary platform loader code, and the exported
    /// descriptor symbol is trusted FFI metadata until validation rejects it.
    /// That code must not directly or indirectly start nested neocogi-com loading.
    pub unsafe fn load_plugin_library(&self, path: impl AsRef<Path>) -> Result<NcGuid, PluginLoadError> {
        let path = path.as_ref();
        // SAFETY: The selected library promises process-lived immutable metadata and code.
        let descriptor = unsafe { open_plugin_descriptor(path) }?;
        let plugin = RegisteredFactoryPlugin {
            descriptor,
            origin: PluginOrigin::Dynamic { path: path.to_path_buf() },
        };
        let plugin_id = plugin.descriptor.id();
        // Descriptor, dependencies and provenance enter the catalog together.
        // Native loading and metadata validation above invoke no code under this lock.
        let status = self.write_catalog().commit_factory_plugin_registration(plugin);
        if status.is_err() {
            return Err(PluginLoadError::new(
                PluginLoadErrorKind::Registration,
                path,
                Some(status),
                "runtime registration rejected the plugin descriptor",
            ));
        }
        Ok(plugin_id)
    }
}

/// Invokes one resolved plugin descriptor entrypoint.
unsafe fn plugin_descriptor_from_entrypoint(get_descriptor: NcGetPluginDescriptor, path: &Path) -> Result<*const NcPluginDescriptor, PluginLoadError> {
    let mut plugin = core::ptr::null();
    // SAFETY: The resolved ABI receives one valid output pointer slot.
    let status = unsafe { get_descriptor(&mut plugin) };
    if status.is_err() {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Entrypoint,
            path,
            Some(status),
            "plugin descriptor entrypoint returned an error",
        ));
    }
    if plugin.is_null() {
        return Err(PluginLoadError::new(
            PluginLoadErrorKind::Entrypoint,
            path,
            Some(NC_E_POINTER),
            "plugin descriptor entrypoint returned null",
        ));
    }
    Ok(plugin)
}

/// Opens and admits immutable metadata once, without publication or factory acquisition.
/// Owned inspection, synchronous callbacks and live loading share this boundary.
/// Native opening pins the mapping on success and every later failure.
///
/// # Safety
/// The path selects plugin code honoring the descriptor ABI and the process
/// lifetime of all reachable metadata/code. Entry must not start a nested load.
pub(crate) unsafe fn open_plugin_descriptor(path: &Path) -> Result<ValidatedPluginDescriptor, PluginLoadError> {
    let module = PinnedLibrary::open(path)?;
    // SAFETY: the selected pinned module supplies its fixed descriptor entrypoint.
    let get = unsafe { module.get_plugin_descriptor(path)? };
    // SAFETY: the mapping keeps the descriptor and all referenced storage alive.
    let raw = unsafe { plugin_descriptor_from_entrypoint(get, path)? };
    // SAFETY: the entrypoint promises complete immutable process-lived metadata/code.
    unsafe { ValidatedPluginDescriptor::new(raw) }
        .map_err(|status| PluginLoadError::new(PluginLoadErrorKind::Metadata, path, Some(status), "invalid plugin descriptor"))
}
