//! Runtime ABI v1 enumeration of registered plugin GUIDs.
//!
//! Names, versions, origin paths and dependencies belong to inspection. Counts
//! are infallible snapshots; fills report the current required length without
//! partial writes, permitting retry when registration races enumeration.

use crate::{NcGuid, NcStatus};

/// Maximum number of distinct dynamically registered plugins in runtime ABI v1.
///
/// Repeated provider loads reuse a registration and consume no slot.
/// Static registration and inspection-only opens do not enter this count; their pinned handles are
/// outside this limit and remain live until the inspector process exits.
pub const NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS: usize = 1_024;

/// Read-only process-global plugin registry for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcPluginRegistryV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Returns the number of registered plugin IDs.
    pub plugin_count: unsafe extern "C" fn() -> usize,
    /// Copies registered plugin IDs to caller-owned storage.
    pub plugin_ids: unsafe extern "C" fn(out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus,
}
