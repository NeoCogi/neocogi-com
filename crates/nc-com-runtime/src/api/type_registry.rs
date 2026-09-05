//! C callbacks for enumerating registered classes and interfaces.
//!
//! The adapters borrow ordered identity slices from the process-global type
//! registry only for the duration of a count or fill call, then delegate raw
//! pointer validation and no-partial-write behavior to the shared API utility.
//!
//! Counts are snapshots and therefore intentionally return plain `usize`.
//! IDs were validated and indexed during registration, so count and fill read
//! ordered slices without descriptor walks or temporary allocations.

use nc_com_core::{NcGuid, NcInterfaceId, NcStatus};

use crate::state::RuntimeState;

use super::copy_slice_to_out;

/// Returns a snapshot count of registered class IDs.
pub(super) unsafe extern "C" fn class_count() -> usize {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_registered_class_ids(<[NcGuid]>::len)
}

/// Copies registered class IDs using the ABI count/fill contract.
pub(super) unsafe extern "C" fn class_ids(
    out: *mut NcGuid,
    capacity: usize,
    out_len: *mut usize,
) -> NcStatus {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_registered_class_ids(|values| {
            // SAFETY: The caller provides the output-buffer contract.
            unsafe { copy_slice_to_out(values, out, capacity, out_len) }
        })
}

/// Returns a snapshot count of distinct registered interface identities.
pub(super) unsafe extern "C" fn interface_count() -> usize {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_registered_interface_ids(<[NcInterfaceId]>::len)
}

/// Copies registered interface identities using the ABI count/fill contract.
pub(super) unsafe extern "C" fn interface_ids(
    out: *mut NcInterfaceId,
    capacity: usize,
    out_len: *mut usize,
) -> NcStatus {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_registered_interface_ids(|values| {
            // SAFETY: The caller provides the output-buffer contract.
            unsafe { copy_slice_to_out(values, out, capacity, out_len) }
        })
}
