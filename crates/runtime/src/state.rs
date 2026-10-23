//! Process-wide coordination for the concrete runtime implementation.
//!
//! A lazily initialized singleton owns one append-only metadata catalog and the
//! independent debug messenger state. The catalog's single read/write lock lets
//! ordinary lookups proceed concurrently and publishes static registration or
//! dynamic acquisition as one blocking write transaction.

use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::debug::DebugState;
use crate::factory_plugin_registry::FactoryPluginRegistry;

/// One lazily constructed catalog per linked runtime image.
static STATE: OnceLock<RuntimeState> = OnceLock::new();

/// Process-global owner of all mutable runtime state.
pub(crate) struct RuntimeState {
    /// Atomically published plugin descriptors, dependencies and provenance.
    catalog: RwLock<FactoryPluginRegistry>,
    /// Owns callback registrations independently of registry locks.
    debug: DebugState,
}

impl RuntimeState {
    /// Returns the process-wide catalog without a host initialization protocol.
    /// The singleton is required by context-free C callbacks; it contains no
    /// fallible contract seeding or component-specific startup state.
    pub(crate) fn global() -> &'static Self {
        STATE.get_or_init(Self::new)
    }

    /// Constructs an empty, independent runtime state.
    ///
    /// This is separated from [`Self::global`] so tests can exercise registry
    /// behavior without sharing process-global registrations.
    pub(crate) fn new() -> Self {
        Self {
            catalog: RwLock::new(FactoryPluginRegistry::new()),
            debug: DebugState::new(),
        }
    }

    /// Acquires a coherent read snapshot of the complete runtime catalog.
    pub(crate) fn read_catalog(&self) -> RwLockReadGuard<'_, FactoryPluginRegistry> {
        self.catalog.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Acquires exclusive publication access to the complete runtime catalog.
    pub(crate) fn write_catalog(&self) -> RwLockWriteGuard<'_, FactoryPluginRegistry> {
        self.catalog.write().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Borrows the debug messenger registry owned by this runtime.
    pub(crate) fn debug(&self) -> &DebugState {
        &self.debug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier};

    use neocogi_com_core::{IUnknown, NC_E_CLOSED, NC_E_NOT_FOUND, NcPluginDependencyDescriptor, NcPluginDescriptor, NcStaticUtf8};

    use crate::factory_plugin_registry::PluginOrigin;
    use crate::factory_registration::prepare_factory_plugin;

    /// Registry fixtures use the same complete validation as production records.
    const fn description(name: &'static str) -> NcPluginDescriptor {
        /// Publication and lookup must never invoke this entrypoint.
        unsafe extern "C" fn acquire(_host: *const neocogi_com_core::NcHostApiV1, out: *mut neocogi_com_core::ComRaw<IUnknown>) -> neocogi_com_core::NcStatus {
            // SAFETY: the descriptor ABI grants one writable output on invocation.
            unsafe { out.write(neocogi_com_core::ComRaw::null()) };
            NC_E_CLOSED
        }
        NcPluginDescriptor {
            description_abi_version: neocogi_com_core::NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
            core_abi_version: neocogi_com_core::NC_COM_CORE_ABI_VERSION,
            plugin_id: neocogi_com_core::guid_from_qualified_name(name),
            qualified_name: NcStaticUtf8::from_static(name),
            plugin_version: NcStaticUtf8::from_static("1.0.0"),
            dependencies: core::ptr::null(),
            dependency_count: 0,
            root_factory: IUnknown::INTERFACE_ID,
            root_factory_threading: neocogi_com_core::NC_ABI_THREADING_THREAD_LOCAL,
            acquire: Some(acquire),
        }
    }

    /// Confirms dependency validation reads the sole factory plugin registry.
    #[test]
    fn catalog_resolves_factory_plugin_dependencies() {
        let runtime = RuntimeState::new();
        static DESCRIPTION: NcPluginDescriptor = description("nc.tests.state.dependency");
        let factory_id = DESCRIPTION.plugin_id;
        let missing_id = neocogi_com_core::guid_from_qualified_name("nc.tests.state.missing");

        {
            let mut catalog = runtime.write_catalog();
            // SAFETY: the fixture descriptor, buffers and code are process-lived.
            catalog.insert(unsafe { prepare_factory_plugin(&DESCRIPTION, PluginOrigin::Static) }.unwrap());
        }

        let catalog = runtime.read_catalog();
        assert!(catalog.contains_plugin(factory_id));
        assert_eq!(
            catalog.validate_plugin_dependencies(&[NcPluginDependencyDescriptor { plugin_id: factory_id }]),
            Ok(())
        );
        assert_eq!(
            catalog.validate_plugin_dependencies(&[NcPluginDependencyDescriptor { plugin_id: missing_id }]),
            Err(NC_E_NOT_FOUND)
        );
    }

    #[test]
    fn dynamic_activation_records_become_visible_in_one_catalog_publication() {
        let runtime = Arc::new(RuntimeState::new());
        let records_inserted = Arc::new(Barrier::new(2));
        let release_publication = Arc::new(Barrier::new(2));
        static DESCRIPTION: NcPluginDescriptor = description("nc.tests.state.publication");
        let plugin_id = DESCRIPTION.plugin_id;

        let writer = {
            let runtime = Arc::clone(&runtime);
            let records_inserted = Arc::clone(&records_inserted);
            let release_publication = Arc::clone(&release_publication);
            std::thread::spawn(move || {
                let mut catalog = runtime.write_catalog();
                // SAFETY: immutable static-lifetime metadata/code back this
                // provenance-only fixture; no fake native handle is required.
                catalog.insert(unsafe { prepare_factory_plugin(&DESCRIPTION, PluginOrigin::Dynamic { path: PathBuf::from("test-plugin") }) }.unwrap());

                // Pause after the complete plugin record has been inserted but
                // before the one publication guard is released.
                records_inserted.wait();
                release_publication.wait();
            })
        };

        records_inserted.wait();
        assert!(runtime.catalog.try_read().is_err());
        release_publication.wait();
        writer.join().unwrap();

        let catalog = runtime.read_catalog();
        let plugin = catalog.plugin(plugin_id).unwrap();
        assert!(matches!(&plugin.origin, PluginOrigin::Dynamic { path, .. } if path == Path::new("test-plugin")));
    }
}
