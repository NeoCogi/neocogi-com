//! C callbacks exposing the canonical contract module registry.

use neocogi_com_core::{NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcContractModuleDescription, NcGuid, NcStatus};

use crate::state::RuntimeState;

use super::copy_slice_to_out;

/// Returns the current number of canonical module identities.
pub(super) unsafe extern "C" fn module_count() -> usize {
    RuntimeState::global().read_catalog().contract_registry.with_module_ids(<[NcGuid]>::len)
}

/// Copies the dependency-first module identity sequence to caller storage.
pub(super) unsafe extern "C" fn module_ids(out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus {
    RuntimeState::global().read_catalog().contract_registry.with_module_ids(|ids| {
        // SAFETY: The caller supplies the output pointer contracts; the
        // shared helper validates capacity before copying.
        unsafe { copy_slice_to_out(ids, out, capacity, out_len) }
    })
}

/// Looks up the canonical immutable description for one logical module ID.
pub(super) unsafe extern "C" fn module_description(module_id: NcGuid, out_description: *mut *const NcContractModuleDescription) -> NcStatus {
    if out_description.is_null() {
        return NC_E_POINTER;
    }
    // Clear the result before lookup so failure never exposes stale metadata.
    // SAFETY: The caller guarantees one writable pointer slot.
    unsafe { *out_description = core::ptr::null() };
    let catalog = RuntimeState::global().read_catalog();
    let Some(module) = catalog.contract_registry.module(module_id) else {
        return NC_E_NOT_FOUND;
    };
    // SAFETY: The output slot was checked and registry retention keeps the
    // immutable description live for the process-wide API lifetime.
    unsafe { *out_description = module.description };
    NC_OK
}
