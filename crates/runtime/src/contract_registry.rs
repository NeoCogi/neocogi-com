//! Runtime-owned indexes for canonical IDL contract module descriptions.
//!
//! The registry copies stable identities and names while retaining pointers to
//! immutable native descriptions. Contract shared libraries are kept mapped by
//! executable linkage or by the runtime's process-lifetime plugin handles.

use std::collections::HashMap;
use std::string::String;
use std::vec::Vec;

use neocogi_com_core::{NC_E_CONTRACT_MISMATCH, NC_E_INVALID_ARG, NC_OK, NcContractFingerprint, NcContractModuleDescription, NcGuid, NcStatus};

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

    /// Checks every batch collision before publishing any new module record.
    pub(crate) fn commit(&mut self, batch: ContractRegistrationBatch) -> NcStatus {
        let modules = batch.into_modules();
        for module in &modules {
            if let Some(existing) = self.module(module.module_id) {
                if existing.abi_fingerprint != module.abi_fingerprint {
                    return NC_E_CONTRACT_MISMATCH;
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
        }

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
        NC_OK
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
}

// SAFETY: The pointer targets immutable validated metadata in a library whose
// lifetime is at least the process-wide registration lifetime.
unsafe impl Send for RegisteredContractModule {}
// SAFETY: The runtime never mutates through retained description pointers.
unsafe impl Sync for RegisteredContractModule {}
