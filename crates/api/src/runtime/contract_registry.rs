//! Safe Rust view of canonical contract modules retained by the runtime.

use std::vec::Vec;

use crate::{NC_E_POINTER, NcContractModuleDescription, NcGuid, NcStatus};

use super::{Runtime, read_vec, status_to_result};

/// Read-only process-wide catalog of canonical IDL module descriptions.
#[derive(Clone, Copy, Debug)]
pub struct ContractRegistry<'runtime> {
    /// Runtime facade used for generation and table validation on every call.
    runtime: &'runtime Runtime,
}

impl<'runtime> ContractRegistry<'runtime> {
    /// Creates a contract-registry context borrowing the runtime facade.
    pub(super) const fn new(runtime: &'runtime Runtime) -> Self {
        Self { runtime }
    }

    /// Returns logical module IDs in dependency-first registration order.
    pub fn module_ids(self) -> Result<Vec<NcGuid>, NcStatus> {
        let api = self.runtime.contract_registry_api();
        read_vec(
            || unsafe { (api.module_count)() },
            |out, capacity, out_len| unsafe { (api.module_ids)(out, capacity, out_len) },
        )
    }

    /// Returns the canonical immutable description for one logical module ID.
    pub fn module(self, module_id: NcGuid) -> Result<Option<&'runtime NcContractModuleDescription>, NcStatus> {
        let api = self.runtime.contract_registry_api();
        let mut description = core::ptr::null();
        // SAFETY: The output slot is writable and a successful lookup returns
        // runtime-retained immutable metadata.
        let status = unsafe { (api.module_description)(module_id, &mut description) };
        if status == crate::NC_E_NOT_FOUND {
            return Ok(None);
        }
        status_to_result(status)?;
        if description.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: Success guarantees a runtime-retained immutable root. The
        // facade borrows the process-global runtime for `'runtime`.
        Ok(Some(unsafe { &*description }))
    }
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use crate::{NC_CONTRACT_DESCRIPTION_ABI_VERSION, NcContractFingerprint, NcContractModuleReference, NcStaticUtf8};

    use super::*;

    /// Stable ID reserved for the facade-level contract registry fixture.
    const MODULE_ID: NcGuid = NcGuid::from_fields(0x7777_0001, 2, 3, [4; 8]);
    /// Exact fixture revision expected by its generated-style reference.
    const FINGERPRINT: NcContractFingerprint = NcContractFingerprint::new([0x77; 32]);
    /// Canonical immutable module returned through the native getter ABI.
    static MODULE: NcContractModuleDescription = NcContractModuleDescription {
        description_abi_version: NC_CONTRACT_DESCRIPTION_ABI_VERSION,
        module_id: MODULE_ID,
        abi_fingerprint: FINGERPRINT,
        module_name: NcStaticUtf8::from_static("nc.tests.facade"),
        dependency_count: 0,
        dependencies: ptr::null(),
        declaration_count: 0,
        declarations: ptr::null(),
    };

    /// Returns the facade test's immutable native module description.
    unsafe extern "C" fn get_module() -> *const NcContractModuleDescription {
        &MODULE
    }

    #[test]
    fn facade_registers_enumerates_and_borrows_a_canonical_module() {
        let runtime = Runtime::global();
        let reference = NcContractModuleReference {
            module_id: MODULE_ID,
            abi_fingerprint: FINGERPRINT,
            get: Some(get_module),
        };
        // SAFETY: The reference and complete description graph are test statics.
        unsafe { runtime.register_contract_module(&reference) }.unwrap();
        assert!(runtime.contract_registry().module_ids().unwrap().contains(&MODULE_ID));
        let description = runtime.contract_registry().module(MODULE_ID).unwrap().expect("registered module");
        assert_eq!(description.abi_fingerprint, FINGERPRINT);
    }
}
