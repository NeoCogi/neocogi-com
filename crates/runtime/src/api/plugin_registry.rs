//! C callbacks for enumerating registered plugins and loaded modules.
//!
//! These thin adapters expose deterministic counts and caller-filled ID arrays
//! from the process-global plugin registry. They intentionally reveal neither
//! internal locks nor owned vectors and use the shared no-partial-write fill
//! protocol.
//!
//! Each callback reads one immutable ordered slice under the catalog read lock.
//! Registry growth after a count is expected and is reported by the fill
//! protocol; locks are never held across separate C calls.

use neocogi_com_core::{NcGuid, NcStatus};

use crate::state::RuntimeState;

use super::copy_slice_to_out;

/// Returns a snapshot count of registered plugin IDs.
pub(super) unsafe extern "C" fn plugin_count() -> usize {
    RuntimeState::global()
        .read_catalog()
        .plugin_registry
        .with_registered_plugin_ids(<[NcGuid]>::len)
}

/// Copies registered plugin IDs using the ABI count/fill contract.
pub(super) unsafe extern "C" fn plugin_ids(out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus {
    RuntimeState::global().read_catalog().plugin_registry.with_registered_plugin_ids(|values| {
        // SAFETY: The caller provides the output-buffer contract.
        unsafe { copy_slice_to_out(values, out, capacity, out_len) }
    })
}

/// Returns a snapshot count of retained dynamic module IDs.
pub(super) unsafe extern "C" fn loaded_module_count() -> usize {
    RuntimeState::global().read_catalog().plugin_registry.with_loaded_module_ids(<[u64]>::len)
}

/// Copies retained module IDs using the ABI count/fill contract.
pub(super) unsafe extern "C" fn loaded_module_ids(out: *mut u64, capacity: usize, out_len: *mut usize) -> NcStatus {
    RuntimeState::global().read_catalog().plugin_registry.with_loaded_module_ids(|values| {
        // SAFETY: The caller provides the output-buffer contract.
        unsafe { copy_slice_to_out(values, out, capacity, out_len) }
    })
}
