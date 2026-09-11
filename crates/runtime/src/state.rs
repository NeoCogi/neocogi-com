//! Process-wide coordination for the concrete runtime implementation.
//!
//! A lazily initialized singleton owns one append-only metadata catalog and the
//! independent debug messenger state. The catalog's single read/write lock lets
//! ordinary lookups proceed concurrently and publishes static registration or
//! dynamic activation as one blocking write transaction.

use std::collections::HashMap;
use std::string::String;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use neocogi_com_core::NcGuid;

use crate::contract_registry::ContractRegistry;
use crate::debug::DebugState;
use crate::factory_plugin_registry::FactoryPluginRegistry;
use crate::plugin_registry::PluginRegistry;
use crate::type_registry::TypeRegistry;

/// Process-global owner of all mutable runtime state.
pub(crate) struct RuntimeState {
    /// Atomically published module, plugin, type, and identity metadata.
    catalog: RwLock<RuntimeCatalog>,
    /// Owns callback registrations independently of registry locks.
    debug: DebugState,
}

/// Complete append-only runtime catalog published through one lock.
pub(crate) struct RuntimeCatalog {
    /// Canonical IDL module identities and native descriptions.
    pub(crate) contract_registry: ContractRegistry,
    /// Factory-only plugin descriptions and activation roots.
    pub(crate) factory_plugin_registry: FactoryPluginRegistry,
    /// One-to-one mappings checked during registration transactions.
    pub(crate) registration: RegistrationState,
    /// Plugin descriptors plus retained dynamic modules.
    pub(crate) plugin_registry: PluginRegistry,
    /// Classes, interfaces, and relationship indexes.
    pub(crate) type_registry: TypeRegistry,
}

/// Runtime-wide one-to-one identity indexes.
///
/// Both maps are kept so registration can reject a reused GUID and a reused
/// human-readable name without scanning either concrete registry. They are
/// updated only after every descriptor in a batch has passed validation.
pub(crate) struct RegistrationState {
    /// Maps each registered plugin or class GUID to its canonical name.
    pub(crate) name_by_id: HashMap<NcGuid, String>,
    /// Maps each canonical plugin or class name back to its GUID.
    pub(crate) id_by_name: HashMap<String, NcGuid>,
}

impl RuntimeState {
    /// Returns the lazily constructed process-wide runtime state.
    ///
    /// Exported C callbacks carry no explicit runtime context, so they all use
    /// this singleton. `OnceLock` makes its construction thread-safe and keeps
    /// the returned reference valid for the remainder of the process.
    pub(crate) fn global() -> &'static Self {
        // The C ABI has no runtime context parameter, so all exported callbacks
        // intentionally converge on this one state instance.
        static STATE: OnceLock<RuntimeState> = OnceLock::new();
        STATE.get_or_init(Self::new)
    }

    /// Constructs an empty, independent runtime state.
    ///
    /// This is separated from [`Self::global`] so tests can exercise registry
    /// behavior without sharing process-global registrations.
    pub(crate) fn new() -> Self {
        Self {
            catalog: RwLock::new(RuntimeCatalog {
                contract_registry: ContractRegistry::new(),
                factory_plugin_registry: FactoryPluginRegistry::new(),
                registration: RegistrationState {
                    name_by_id: HashMap::new(),
                    id_by_name: HashMap::new(),
                },
                plugin_registry: PluginRegistry::new(),
                type_registry: TypeRegistry::new(),
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
    /// Returns whether either activation model owns a plugin identity.
    ///
    /// A dependency edge names a plugin, not the registry used to activate it.
    /// Keeping this lookup on the complete catalog prevents callers from
    /// learning or duplicating the internal legacy/factory storage split.
    pub(crate) fn contains_plugin(&self, plugin_id: NcGuid) -> bool {
        self.plugin_registry.contains_plugin(plugin_id) || self.factory_plugin_registry.contains_plugin(plugin_id)
    }

    /// Requires every hard dependency to be present under either activation model.
    ///
    /// Registration remains dependency-first, while a legacy plugin may depend
    /// on a factory plugin and vice versa. The activation representation is not
    /// part of the dependency contract.
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

    use neocogi_com_core::{NC_E_NOT_FOUND, NcGuid, NcInterfaceId};

    use crate::factory_plugin_registry::RegisteredFactoryPlugin;
    use crate::loader::{LoadedModule, LoadedModuleInfo};
    use crate::plugin_registry::RegisteredPlugin;
    use crate::type_registry::RegisteredClass;

    /// Confirms plugin dependency identity is independent of activation storage.
    #[test]
    fn catalog_resolves_plugins_across_both_activation_models() {
        let runtime = RuntimeState::new();
        let legacy_id = NcGuid::from_fields(0x6150_0011, 0x5100, 0x4100, [0x91; 8]);
        let factory_id = NcGuid::from_fields(0x6150_0012, 0x5100, 0x4100, [0x92; 8]);
        let missing_id = NcGuid::from_fields(0x6150_0013, 0x5100, 0x4100, [0x93; 8]);

        {
            let mut catalog = runtime.write_catalog();
            // These null descriptor roots are safe in this lookup-only test:
            // dependency validation reads only the owned IDs in each record.
            catalog.plugin_registry.insert(RegisteredPlugin {
                plugin_id: legacy_id,
                plugin: core::ptr::null(),
                dependency_ids: Vec::new(),
            });
            catalog.factory_plugin_registry.insert(RegisteredFactoryPlugin {
                plugin_id: factory_id,
                description: core::ptr::null(),
                dependency_ids: Vec::new(),
            });
        }

        let catalog = runtime.read_catalog();
        assert!(catalog.contains_plugin(legacy_id));
        assert!(catalog.contains_plugin(factory_id));
        assert_eq!(catalog.validate_plugin_dependencies(&[legacy_id, factory_id]), Ok(()));
        assert_eq!(catalog.validate_plugin_dependencies(&[missing_id]), Err(NC_E_NOT_FOUND));
    }

    #[test]
    fn dynamic_activation_records_become_visible_in_one_catalog_publication() {
        let runtime = Arc::new(RuntimeState::new());
        let records_inserted = Arc::new(Barrier::new(2));
        let release_publication = Arc::new(Barrier::new(2));
        let plugin_id = NcGuid::from_fields(0x6150_0001, 0x5100, 0x4100, [0x81; 8]);
        let class_id = NcGuid::from_fields(0x6150_0002, 0x5100, 0x4100, [0x82; 8]);

        let writer = {
            let runtime = Arc::clone(&runtime);
            let records_inserted = Arc::clone(&records_inserted);
            let release_publication = Arc::clone(&release_publication);
            std::thread::spawn(move || {
                let mut catalog = runtime.write_catalog();
                let module_id = catalog.plugin_registry.allocate_loaded_module_id().unwrap();
                catalog.plugin_registry.insert(RegisteredPlugin {
                    plugin_id,
                    plugin: core::ptr::null(),
                    dependency_ids: Vec::new(),
                });
                catalog.type_registry.insert_classes(vec![RegisteredClass {
                    plugin_id,
                    module_id: Some(module_id),
                    class_id,
                    class: core::ptr::null(),
                    interface_ids: vec![NcInterfaceId::NONE],
                }]);
                catalog.registration.name_by_id.insert(plugin_id, "test.plugin".into());
                catalog.registration.id_by_name.insert("test.plugin".into(), plugin_id);
                catalog.plugin_registry.store_loaded_module(
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
        let class = catalog.type_registry.class(class_id).unwrap();
        let module_id = class.module_id.unwrap();
        let module = catalog.plugin_registry.module(module_id).unwrap();
        assert_eq!(module.plugin_id, plugin_id);
        assert_eq!(catalog.registration.name_by_id.get(&plugin_id).map(String::as_str), Some("test.plugin"));
    }
}
