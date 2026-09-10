//! ABI table for enumerating registered class and interface identities.
//!
//! Counts and caller-filled ID arrays expose deterministic registration order
//! without leaking runtime container types. Metadata and class/interface edges
//! remain in the separate inspection table.
//!
//! Interface entries use the complete [`NcInterfaceId`] rather than a bare IID,
//! because the ABI signature is part of interface compatibility. Enumeration follows
//! the same count-then-fill protocol as the plugin registry.

use crate::{NcGuid, NcInterfaceId, NcStatus};

/// Read-only process-global class and interface registry for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcTypeRegistryV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Returns the number of registered class IDs.
    pub class_count: unsafe extern "C" fn() -> usize,
    /// Copies registered class IDs to caller-owned storage.
    pub class_ids: unsafe extern "C" fn(out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Returns the number of distinct registered interface ABI identities.
    pub interface_count: unsafe extern "C" fn() -> usize,
    /// Copies distinct registered interface ABI identities to caller-owned storage.
    pub interface_ids: unsafe extern "C" fn(out: *mut NcInterfaceId, capacity: usize, out_len: *mut usize) -> NcStatus,
}
