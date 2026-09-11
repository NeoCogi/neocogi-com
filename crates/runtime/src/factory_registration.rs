//! Atomic registration of factory-only plugins and their contract graphs.
//!
//! Raw module traversal and string copying occur before the catalog write lock.
//! The locked phase checks all process state first, then publishes contract
//! modules, the plugin record, and its identity mapping as one transaction.

use std::collections::HashMap;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{
    NC_CONTRACT_DECLARATION_FACTORY, NC_E_CONTRACT_MISMATCH, NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_OK, NcFactoryPluginDescription, NcGuid, NcStatus,
    validate_factory_plugin_description,
};

use crate::contract_registration::build_contract_registration_batch;
use crate::contract_registry::RegisteredContractModule;
use crate::factory_plugin_registry::RegisteredFactoryPlugin;
use crate::state::{RuntimeCatalog, RuntimeState};

/// Fully validated records ready for one catalog transaction.
pub(crate) struct FactoryPluginRegistrationBatch {
    /// Runtime plugin record retaining the immutable description.
    plugin: RegisteredFactoryPlugin,
    /// Exact module revisions reached from every direct contract reference.
    modules: Vec<RegisteredContractModule>,
    /// Owned normative plugin name for collision checks and diagnostics.
    qualified_name: String,
}

impl FactoryPluginRegistrationBatch {
    /// Returns the plugin identity projected from validated metadata.
    pub(crate) fn plugin_id(&self) -> NcGuid {
        self.plugin.plugin_id
    }

    /// Borrows dependency-first canonical module records for offline copying.
    pub(crate) fn modules(&self) -> &[RegisteredContractModule] {
        &self.modules
    }
}

/// Builds one factory plugin batch without holding runtime locks.
///
/// # Safety
///
/// `plugin` must be null or reference an immutable description whose strings,
/// tables, contract getters, and returned transitive module graphs remain live
/// for registration and the subsequent process-wide registry lifetime.
pub(crate) unsafe fn build_factory_plugin_registration_batch(plugin: *const NcFactoryPluginDescription) -> Result<FactoryPluginRegistrationBatch, NcStatus> {
    // SAFETY: The caller supplies the complete description readability contract.
    let status = unsafe { validate_factory_plugin_description(plugin) };
    if status.is_err() {
        return Err(status);
    }
    // SAFETY: Successful validation rejected null and established root fields.
    let description = unsafe { &*plugin };
    // SAFETY: Validation established readable UTF-8 metadata.
    let qualified_name = unsafe { description.qualified_name.as_str() }.map_err(|_| NC_E_INVALID_ARG)?.to_owned();
    // SAFETY: Validation established the dependency table shape.
    let dependency_ids = unsafe { description.dependencies() }?.iter().map(|dependency| dependency.plugin_id).collect();

    let mut modules = Vec::<RegisteredContractModule>::new();
    let mut module_by_id = HashMap::<NcGuid, usize>::new();
    // SAFETY: Validation established the direct contract table shape.
    for reference in unsafe { description.contracts() }? {
        // SAFETY: The plugin registration contract keeps each linked getter and
        // transitive native graph live for the resulting registry lifetime.
        let batch = unsafe { build_contract_registration_batch(reference) }?;
        for module in batch.into_modules() {
            if let Some(index) = module_by_id.get(&module.module_id).copied() {
                let previous = &modules[index];
                if previous.abi_fingerprint != module.abi_fingerprint || previous.module_name != module.module_name {
                    return Err(NC_E_CONTRACT_MISMATCH);
                }
                continue;
            }
            module_by_id.insert(module.module_id, modules.len());
            modules.push(module);
        }
    }

    validate_root_factory(description, &modules)?;
    Ok(FactoryPluginRegistrationBatch {
        plugin: RegisteredFactoryPlugin {
            plugin_id: description.plugin_id,
            description: plugin,
            dependency_ids,
        },
        modules,
        qualified_name,
    })
}

/// Confirms the root identity and threading come from a canonical IDL factory.
fn validate_root_factory(description: &NcFactoryPluginDescription, modules: &[RegisteredContractModule]) -> Result<(), NcStatus> {
    let mut matched = false;
    for module in modules {
        // SAFETY: Batch construction retained only fully validated descriptions.
        let module_description = unsafe { &*module.description };
        // SAFETY: Module validation established its declaration table shape.
        for declaration in unsafe { module_description.declarations() }? {
            if declaration.declaration_id != description.root_factory_id {
                continue;
            }
            if declaration.kind != NC_CONTRACT_DECLARATION_FACTORY || declaration.threading != description.root_factory_threading || matched {
                return Err(NC_E_CONTRACT_MISMATCH);
            }
            matched = true;
        }
    }
    if matched { Ok(()) } else { Err(NC_E_CONTRACT_MISMATCH) }
}

impl RuntimeState {
    /// Validates and registers one static factory-only plugin description.
    ///
    /// # Safety
    ///
    /// The description and all linked contract graphs must remain readable for
    /// the process-wide registry lifetime.
    pub(crate) unsafe fn register_factory_plugin_description(&self, plugin: *const NcFactoryPluginDescription) -> NcStatus {
        // SAFETY: The caller provides the complete static metadata lifetime.
        let batch = match unsafe { build_factory_plugin_registration_batch(plugin) } {
            Ok(batch) => batch,
            Err(status) => return status,
        };
        self.write_catalog().commit_factory_plugin_registration(batch)
    }
}

impl RuntimeCatalog {
    /// Checks all state-dependent rules before publishing the complete batch.
    pub(crate) fn commit_factory_plugin_registration(&mut self, batch: FactoryPluginRegistrationBatch) -> NcStatus {
        let FactoryPluginRegistrationBatch { plugin, modules, qualified_name } = batch;

        if plugin
            .dependency_ids
            .iter()
            .any(|dependency_id| !self.factory_plugin_registry.contains_plugin(*dependency_id))
        {
            return NC_E_NOT_FOUND;
        }
        if self.plugin_registry.contains_plugin(plugin.plugin_id) {
            // A logical plugin belongs to exactly one activation model. This
            // keeps the factory-only path isolated from class-table semantics.
            return NC_E_INVALID_ARG;
        }
        if let Some(existing) = self.factory_plugin_registry.plugin(plugin.plugin_id) {
            return if existing.description == plugin.description {
                NC_OK
            } else {
                NC_E_INVALID_ARG
            };
        }
        if self.registration.name_by_id.get(&plugin.plugin_id).is_some_and(|name| name != &qualified_name)
            || self.registration.id_by_name.get(&qualified_name).is_some_and(|id| *id != plugin.plugin_id)
        {
            return NC_E_INVALID_ARG;
        }
        let contract_status = self.contract_registry.validate_modules(&modules);
        if contract_status.is_err() {
            return contract_status;
        }

        // Every validation above is read-only. These insertions are therefore
        // the single publication tail of the catalog write transaction.
        self.contract_registry.insert_modules(modules);
        self.registration.name_by_id.insert(plugin.plugin_id, qualified_name.clone());
        self.registration.id_by_name.insert(qualified_name, plugin.plugin_id);
        self.factory_plugin_registry.insert(plugin);
        NC_OK
    }
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use neocogi_com_core::{
        IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE, NC_COM_CORE_ABI_VERSION, NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        NcContractDeclarationDescription, NcContractFingerprint, NcContractModuleDescription, NcContractModuleReference, NcStaticUtf8,
        guid_from_qualified_name,
    };

    use super::*;

    /// Stable module identity for the factory registration fixture.
    const MODULE_ID: NcGuid = NcGuid::from_fields(1, 2, 3, [4; 8]);
    /// Exact nonzero revision of the fixture module.
    const FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new([5; 32]);
    /// Stable factory declaration identity retained by the module DSO.
    const FACTORY_ID: NcGuid = NcGuid::from_fields(6, 7, 8, [9; 8]);

    /// Returns the immutable factory module through its linked getter ABI.
    unsafe extern "C" fn get_module() -> *const NcContractModuleDescription {
        &MODULE
    }

    /// Provides a valid function pointer without constructing an object here.
    unsafe extern "C" fn activate_stub(_out: *mut *mut IUnknown) -> NcStatus {
        NC_E_INVALID_ARG
    }

    /// Canonical factory declaration used to verify root identity and threading.
    static DECLARATIONS: [NcContractDeclarationDescription; 1] = [NcContractDeclarationDescription {
        declaration_id: FACTORY_ID,
        version: 1,
        kind: NC_CONTRACT_DECLARATION_FACTORY,
        threading: NC_ABI_THREADING_THREAD_LOCAL,
        qualified_name: NcStaticUtf8::from_static("nc.tests.contract.RootFactory"),
    }];
    /// Canonical leaf module containing the fixture factory.
    static MODULE: NcContractModuleDescription = NcContractModuleDescription {
        description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        module_id: MODULE_ID,
        abi_fingerprint: FINGERPRINT,
        module_name: NcStaticUtf8::from_static("nc.tests.contract"),
        dependency_count: 0,
        dependencies: ptr::null(),
        declaration_count: DECLARATIONS.len() as u32,
        declarations: DECLARATIONS.as_ptr(),
        source: NcStaticUtf8::from_static("#[version(1)] factory RootFactory {}"),
    };
    /// Exact direct module reference linked into the fixture plugin.
    static CONTRACTS: [NcContractModuleReference; 1] = [NcContractModuleReference {
        module_id: MODULE_ID,
        abi_fingerprint: FINGERPRINT,
        get: Some(get_module),
    }];
    /// Valid factory-only plugin rooted in the canonical declaration above.
    static PLUGIN: NcFactoryPluginDescription = NcFactoryPluginDescription {
        core_abi_version: NC_COM_CORE_ABI_VERSION,
        validation_mode: neocogi_com_core::NC_ABI_VALIDATION_MODE,
        plugin_id: guid_from_qualified_name("nc.tests.factory_plugin"),
        qualified_name: NcStaticUtf8::from_static("nc.tests.factory_plugin"),
        plugin_version: NcStaticUtf8::from_static("1.0.0"),
        dependency_count: 0,
        dependencies: ptr::null(),
        contract_count: CONTRACTS.len() as u32,
        contracts: CONTRACTS.as_ptr(),
        root_factory_id: FACTORY_ID,
        root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
        activate: Some(activate_stub),
    };

    /// Confirms contracts and plugin metadata publish in one successful commit.
    #[test]
    fn registers_a_canonical_factory_plugin_and_contract_together() {
        let runtime = RuntimeState::new();
        // SAFETY: The complete fixture graph uses immutable static storage.
        assert_eq!(unsafe { runtime.register_factory_plugin_description(&PLUGIN) }, NC_OK);
        let catalog = runtime.read_catalog();
        assert!(catalog.contract_registry.module(MODULE_ID).is_some());
        assert!(catalog.factory_plugin_registry.contains_plugin(PLUGIN.plugin_id));
    }

    /// Confirms a duplicate plugin threading claim leaves the catalog untouched.
    #[test]
    fn rejects_root_threading_mismatch_without_partial_contract_registration() {
        let runtime = RuntimeState::new();
        let invalid = NcFactoryPluginDescription {
            root_factory_threading: NC_ABI_THREADING_THREAD_SAFE,
            ..PLUGIN
        };
        // SAFETY: The stack root borrows only immutable static child metadata.
        assert_eq!(unsafe { runtime.register_factory_plugin_description(&invalid) }, NC_E_CONTRACT_MISMATCH);
        let catalog = runtime.read_catalog();
        assert!(catalog.contract_registry.module(MODULE_ID).is_none());
        assert!(!catalog.factory_plugin_registry.contains_plugin(PLUGIN.plugin_id));
    }
}
