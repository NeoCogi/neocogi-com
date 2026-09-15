//! Runtime-owned indexes for canonical IDL contract module descriptions.
//!
//! The registry copies stable identities and names while retaining pointers to
//! immutable native descriptions. Contract shared libraries are kept mapped by
//! executable linkage or by the runtime's process-lifetime plugin handles.

use std::collections::HashMap;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{
    NC_E_CONTRACT_MISMATCH, NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_OK, NcContractFingerprint, NcContractModuleDescription, NcGuid, NcStatus,
    validate_contract_module_description,
};

use crate::contract_registration::ContractRegistrationBatch;

/// Append-only canonical contract module catalog.
pub(crate) struct ContractRegistry {
    /// Records in dependency-first registration order.
    modules: Vec<RegisteredContractModule>,
    /// Enumeration-only identities in the same dependency-first order.
    module_ids: Vec<NcGuid>,
    /// Logical module ID to ordered-record index.
    module_by_id: HashMap<NcGuid, usize>,
    /// Exact module name to stable logical ID.
    id_by_name: HashMap<String, NcGuid>,
}

impl ContractRegistry {
    /// Creates an empty process-local module registry.
    pub(crate) fn new() -> Self {
        Self {
            modules: Vec::new(),
            module_ids: Vec::new(),
            module_by_id: HashMap::new(),
            id_by_name: HashMap::new(),
        }
    }

    /// Seeds a generated runtime-owned description before the catalog is published.
    ///
    /// This uses mutable construction rather than a lock or interior mutability:
    /// no client can observe the candidate catalog until initialization ends.
    pub(crate) fn seed_builtin(&mut self, description: &'static NcContractModuleDescription) -> Result<(), NcStatus> {
        // SAFETY: A shared static reference establishes the complete generated
        // description's readability and process-wide lifetime.
        let status = unsafe { validate_contract_module_description(description) };
        if status.is_err() {
            return Err(status);
        }
        if description.dependency_count != 0 {
            return Err(NC_E_INVALID_ARG);
        }
        // SAFETY: Whole-root validation established a readable UTF-8 name.
        let name = unsafe { description.module_name.as_str() }.map_err(|_| NC_E_INVALID_ARG)?;
        let record = RegisteredContractModule {
            module_id: description.module_id,
            abi_fingerprint: description.abi_fingerprint,
            description: core::ptr::from_ref(description),
            module_name: name.to_owned(),
            origin: ContractOrigin::Builtin,
        };
        let status = self.validate_modules(core::slice::from_ref(&record));
        if status.is_err() {
            return Err(status);
        }
        self.insert_modules(vec![record]);
        Ok(())
    }

    /// Checks that each requested built-in exists with the exact seeded revision.
    pub(crate) fn validate_builtin_dependencies(&self, requirements: &[(NcGuid, NcContractFingerprint)]) -> NcStatus {
        for &(module_id, expected) in requirements {
            let Some(actual) = self.module(module_id) else {
                return NC_E_NOT_FOUND;
            };
            if actual.origin != ContractOrigin::Builtin {
                return NC_E_INVALID_ARG;
            }
            if actual.abi_fingerprint != expected {
                return NC_E_CONTRACT_MISMATCH;
            }
        }
        NC_OK
    }

    /// Checks every batch collision before publishing any new module record.
    pub(crate) fn commit(&mut self, batch: ContractRegistrationBatch) -> NcStatus {
        let (modules, builtin_dependencies) = batch.into_parts();
        let status = self.validate_builtin_dependencies(&builtin_dependencies);
        if status.is_err() {
            return status;
        }
        let status = self.validate_modules(&modules);
        if status.is_err() {
            return status;
        }
        self.insert_modules(modules);
        NC_OK
    }

    /// Checks a complete module set without changing observable registry state.
    pub(crate) fn validate_modules(&self, modules: &[RegisteredContractModule]) -> NcStatus {
        let mut batch_by_id = HashMap::<NcGuid, &RegisteredContractModule>::new();
        let mut batch_id_by_name = HashMap::<&str, NcGuid>::new();
        for module in modules {
            if let Some(existing) = self.module(module.module_id) {
                if existing.abi_fingerprint != module.abi_fingerprint {
                    return NC_E_CONTRACT_MISMATCH;
                }
                if existing.origin != module.origin {
                    return NC_E_INVALID_ARG;
                }
                if existing.module_name != module.module_name {
                    return NC_E_INVALID_ARG;
                }
            }
            if self
                .id_by_name
                .get(&module.module_name)
                .is_some_and(|existing_id| *existing_id != module.module_id)
            {
                return NC_E_INVALID_ARG;
            }
            if let Some(previous) = batch_by_id.insert(module.module_id, module) {
                if previous.abi_fingerprint != module.abi_fingerprint || previous.module_name != module.module_name {
                    return NC_E_CONTRACT_MISMATCH;
                }
            }
            if batch_id_by_name
                .insert(module.module_name.as_str(), module.module_id)
                .is_some_and(|previous_id| previous_id != module.module_id)
            {
                return NC_E_INVALID_ARG;
            }
        }
        NC_OK
    }

    /// Inserts a module set that already passed [`Self::validate_modules`].
    pub(crate) fn insert_modules(&mut self, modules: Vec<RegisteredContractModule>) {
        for module in modules {
            if self.module_by_id.contains_key(&module.module_id) {
                // Exact duplicate revisions are idempotent, including the case
                // where the platform mapped the same bytes at another address.
                continue;
            }
            let index = self.modules.len();
            self.module_ids.push(module.module_id);
            self.module_by_id.insert(module.module_id, index);
            self.id_by_name.insert(module.module_name.clone(), module.module_id);
            self.modules.push(module);
        }
    }

    /// Looks up one canonical module record by stable logical identity.
    pub(crate) fn module(&self, module_id: NcGuid) -> Option<&RegisteredContractModule> {
        self.module_by_id.get(&module_id).map(|index| &self.modules[*index])
    }

    /// Borrows ordered module IDs for count/fill registry APIs.
    pub(crate) fn with_module_ids<R>(&self, read: impl FnOnce(&[NcGuid]) -> R) -> R {
        read(&self.module_ids)
    }
}

/// One validated native description retained by the registry.
pub(crate) struct RegisteredContractModule {
    /// Stable identity derived from the exact dotted module name.
    pub(crate) module_id: NcGuid,
    /// Exact transitive ABI fingerprint accepted for this logical module.
    pub(crate) abi_fingerprint: NcContractFingerprint,
    /// Immutable canonical description owned by a retained native library.
    pub(crate) description: *const NcContractModuleDescription,
    /// Owned module name used for collision checks and diagnostics.
    pub(crate) module_name: String,
    /// Whether this pointer belongs to the runtime image or a linked producer.
    pub(crate) origin: ContractOrigin,
}

/// Provenance required when resolving a description without a getter.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ContractOrigin {
    /// Generated immutable description embedded in the shared runtime.
    Builtin,
    /// Immutable description reached through a producer's native getter.
    Native,
}

// SAFETY: The pointer targets immutable validated metadata in a library whose
// lifetime is at least the process-wide registration lifetime.
unsafe impl Send for RegisteredContractModule {}
// SAFETY: The runtime never mutates through retained description pointers.
unsafe impl Sync for RegisteredContractModule {}
