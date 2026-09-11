//! ABI table for canonical IDL contract module enumeration and lookup.
//!
//! The table exposes immutable descriptions retained by the process-wide
//! runtime. Callers enumerate stable logical IDs and may then borrow the one
//! canonical description accepted for each ID.

use crate::{NcContractModuleDescription, NcGuid, NcStatus};

/// Read-only process-global contract module registry for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcContractRegistryV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Returns the number of registered logical module IDs.
    pub module_count: unsafe extern "C" fn() -> usize,
    /// Copies dependency-first logical module IDs to caller-owned storage.
    pub module_ids: unsafe extern "C" fn(out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Returns the canonical immutable description for one logical module ID.
    pub module_description: unsafe extern "C" fn(module_id: NcGuid, out_description: *mut *const NcContractModuleDescription) -> NcStatus,
}
