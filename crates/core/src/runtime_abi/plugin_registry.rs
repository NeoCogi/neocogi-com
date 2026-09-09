//! ABI table for enumerating plugins and retained dynamic modules.
//!
//! The table exposes stable plugin IDs and process-local loaded-module IDs using
//! the shared count/fill convention. It describes catalog existence only;
//! names, versions, paths, and relationships are provided by inspection APIs.
//!
//! Counts are deliberately infallible snapshots. Fill calls report the current
//! required length so callers can retry if append-only runtime state grows
//! between the count and fill operations.

use crate::{NcGuid, NcStatus};

/// Maximum number of dynamic plugin modules retained by runtime ABI v1.
pub const NC_COM_RUNTIME_MAX_LOADED_MODULES: usize = 1_024;

/// Read-only process-global plugin and loaded-module registry for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcPluginRegistryV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Returns the number of registered plugin IDs.
    pub plugin_count: unsafe extern "C" fn() -> usize,
    /// Copies registered plugin IDs to caller-owned storage.
    pub plugin_ids:
        unsafe extern "C" fn(out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Returns the number of retained loaded modules, which cannot exceed
    /// [`NC_COM_RUNTIME_MAX_LOADED_MODULES`].
    pub loaded_module_count: unsafe extern "C" fn() -> usize,
    /// Copies retained loaded-module IDs to caller-owned storage.
    pub loaded_module_ids:
        unsafe extern "C" fn(out: *mut u64, capacity: usize, out_len: *mut usize) -> NcStatus,
}
