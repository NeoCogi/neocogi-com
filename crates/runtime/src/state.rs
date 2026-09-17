//! Process-wide coordination for the concrete runtime implementation.
//!
//! A lazily initialized singleton owns one append-only metadata catalog and the
//! independent debug messenger state. The catalog's single read/write lock lets
//! ordinary lookups proceed concurrently and publishes static registration or
//! dynamic activation as one blocking write transaction.

use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use neocogi_com_core::NcGuid;

use crate::debug::DebugState;
use crate::factory_plugin_registry::FactoryPluginRegistry;
use crate::module_registry::ModuleRegistry;

/// One lazily constructed catalog per linked runtime image.
static STATE: OnceLock<RuntimeState> = OnceLock::new();

/// Process-global owner of all mutable runtime state.
pub(crate) struct RuntimeState {
    /// Atomically published module, plugin, and identity metadata.
    catalog: RwLock<RuntimeCatalog>,
    /// Owns callback registrations independently of registry locks.
    debug: DebugState,
}

/// Complete append-only runtime catalog published through one lock.
pub(crate) struct RuntimeCatalog {
    /// Factory-only plugin descriptions and activation roots.
    pub(crate) factory_plugin_registry: FactoryPluginRegistry,
    /// Native module handles and their process-local provenance.
    pub(crate) module_registry: ModuleRegistry,
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
            catalog: RwLock::new(RuntimeCatalog {
                factory_plugin_registry: FactoryPluginRegistry::new(),
                module_registry: ModuleRegistry::new(),
            }),
            debug: DebugState::new(),
        }
    }

    /// Acquires a coherent read snapshot of the complete runtime catalog.
    pub(crate) fn read_catalog(&self) -> RwLockReadGuard<'_, RuntimeCatalog> {
        self.catalog.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Acquires exclusive publication access to the complete runtime catalog.
    pub(crate) fn write_catalog(&self) -> RwLockWriteGuard<'_, RuntimeCatalog> {
        self.catalog.write().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Borrows the debug messenger registry owned by this runtime.
    pub(crate) fn debug(&self) -> &DebugState {
        &self.debug
    }
}

impl RuntimeCatalog {
    /// Returns whether the factory registry owns a plugin identity.
    pub(crate) fn contains_plugin(&self, plugin_id: NcGuid) -> bool {
        self.factory_plugin_registry.contains_plugin(plugin_id)
    }

    /// Requires every hard dependency to be registered first.
    pub(crate) fn validate_plugin_dependencies(&self, dependency_ids: &[NcGuid]) -> Result<(), neocogi_com_core::NcStatus> {
        if dependency_ids.iter().any(|dependency_id| !self.contains_plugin(*dependency_id)) {
            return Err(neocogi_com_core::NC_E_NOT_FOUND);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};

    use neocogi_com_core::{NC_E_NOT_FOUND, NcGuid};

    use crate::factory_plugin_registry::RegisteredFactoryPlugin;
    use crate::loader::{LoadedModule, LoadedModuleId, LoadedModuleInfo};

    /// Confirms dependency validation reads the sole factory plugin registry.
    #[test]
    fn catalog_resolves_factory_plugin_dependencies() {
        let runtime = RuntimeState::new();
        let factory_id = NcGuid::from_fields(0x6150_0012, 0x5100, 0x4100, [0x92; 8]);
        let missing_id = NcGuid::from_fields(0x6150_0013, 0x5100, 0x4100, [0x93; 8]);

        {
            let mut catalog = runtime.write_catalog();
            // These null descriptor roots are safe in this lookup-only test:
            // dependency validation reads only the owned IDs in each record.
            catalog.factory_plugin_registry.insert(RegisteredFactoryPlugin {
                plugin_id: factory_id,
                description: core::ptr::null(),
                dependency_ids: Vec::new(),
            });
        }

        let catalog = runtime.read_catalog();
        assert!(catalog.contains_plugin(factory_id));
        assert_eq!(catalog.validate_plugin_dependencies(&[factory_id]), Ok(()));
        assert_eq!(catalog.validate_plugin_dependencies(&[missing_id]), Err(NC_E_NOT_FOUND));
    }

    #[test]
    fn dynamic_activation_records_become_visible_in_one_catalog_publication() {
        let runtime = Arc::new(RuntimeState::new());
        let records_inserted = Arc::new(Barrier::new(2));
        let release_publication = Arc::new(Barrier::new(2));
        let plugin_id = NcGuid::from_fields(0x6150_0001, 0x5100, 0x4100, [0x81; 8]);

        let writer = {
            let runtime = Arc::clone(&runtime);
            let records_inserted = Arc::clone(&records_inserted);
            let release_publication = Arc::clone(&release_publication);
            std::thread::spawn(move || {
                let mut catalog = runtime.write_catalog();
                let module_id = catalog.module_registry.allocate_loaded_module_id().unwrap();
                catalog.factory_plugin_registry.insert(RegisteredFactoryPlugin {
                    plugin_id,
                    description: core::ptr::null(),
                    dependency_ids: Vec::new(),
                });
                catalog.module_registry.store_loaded_module(
                    LoadedModule::test_stub(),
                    LoadedModuleInfo::new(module_id, PathBuf::from("test-plugin"), plugin_id),
                );

                // Pause after every correlated record has been mutated but
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
        let module_id = LoadedModuleId::from_raw(1);
        let module = catalog.module_registry.module(module_id).unwrap();
        assert_eq!(module.plugin_id, plugin_id);
        assert!(catalog.factory_plugin_registry.contains_plugin(plugin_id));
    }
}
