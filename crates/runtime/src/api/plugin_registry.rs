//! C callbacks for enumerating registered plugins.
//!
//! These thin adapters expose deterministic counts and caller-filled ID arrays
//! from the process-global plugin registry. They intentionally reveal neither
//! internal locks nor owned vectors and use the shared no-partial-write fill
//! protocol.
//!
//! Each callback reads the authoritative ordered records under the catalog read lock.
//! Registry growth after a count is expected and is reported by the fill
//! protocol; locks are never held across separate C calls.

use neocogi_com_core::{NcGuid, NcStatus};

use crate::state::RuntimeState;

use super::copy_to_out;

/// Returns a snapshot count of registered plugin IDs.
pub(super) unsafe extern "C" fn plugin_count() -> usize {
    RuntimeState::global().read_catalog().len()
}

/// Copies registered plugin IDs using the ABI count/fill contract.
pub(super) unsafe extern "C" fn plugin_ids(out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus {
    let runtime = RuntimeState::global();
    let catalog = runtime.read_catalog();
    // SAFETY: The caller provides the output-buffer contract. The read guard
    // keeps both the record count and its ordered identities fixed during fill.
    unsafe { copy_to_out(catalog.plugins().map(|plugin| plugin.plugin_id), out, capacity, out_len) }
}
