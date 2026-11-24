//! Supplied access to the host's one runtime implementation.
//!
//! This record groups existing C operation tables without owning their state.
//! Admission checks only readable prefixes and supported layouts. The caller of
//! a consuming bootstrap separately guarantees immutable process-lived backing,
//! callable code and one coherent host owner for the complete reachable graph.

use core::{mem, ptr};

use super::{NcDebugApi, NcPersistenceApi, NcPluginApi};
use crate::{NC_COM_ABI_VERSION, NC_E_ABI_VERSION, NC_E_POINTER, NC_OK, NcStatus};

/// Immutable C description supplied by the host when acquiring a plugin factory.
///
/// All three tables belong to the same host implementation. Copies of this
/// record copy access, never a registry or another runtime owner. The host keeps
/// its tables, callback code and state alive for the process/Wasm-instance
/// lifetime, including after an authored application main returns. Host entrypoints
/// support concurrent access. Caller-owned persistence contexts remain subject
/// to their own thread-local and exclusive-borrow operation contracts.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcHostApi {
    /// Available record bytes, including the size/version prefix.
    pub size: usize,
    /// Native ABI version of the complete service graph.
    pub abi_version: u32,
    /// Reserved. Producers initialize this word to zero. Current readers ignore it.
    ///
    /// Legacy readers load both words before rejecting the new ABI version.
    /// A field here keeps that second read out of structure padding.
    pub reserved: u32,
    /// Required immutable plugin/catalog operation table in the host.
    pub runtime: *const NcPluginApi,
    /// Required immutable diagnostics operation table in the same host.
    pub debug: *const NcDebugApi,
    /// Required immutable caller-owned persistence operation table in that host.
    /// Its presence does not guarantee a serialization provider; see
    /// [`NcPersistenceApi`] for the application's explicit registration duty.
    pub persistence: *const NcPersistenceApi,
}

// SAFETY: Sharing the record exposes only const raw pointers. Dereferencing them
// still requires the documented backing/immutability/callable-code obligations;
// no safe operation on this record mutates or dereferences foreign storage.
unsafe impl Sync for NcHostApi {}

/// Checks the host record and required table prefixes without invoking code.
///
/// Null required pointers return `NC_E_POINTER`. An unsupported ABI version or a
/// record of the wrong size returns `NC_E_ABI_VERSION`.
///
/// # Safety
/// A nonnull host pointer is aligned and readable for its leading `usize`. If
/// its size matches the current record, the version immediately after the
/// size is readable. The current version requires the complete immutable host
/// record. Each nonnull child is aligned and readable for its leading size.
/// Children declaring the current table sizes have that complete layout and
/// belong to the same host. Adoption additionally requires process-lived
/// immutable backing for all three tables and their callback code.
pub unsafe fn validate_host_api(host: *const NcHostApi) -> NcStatus {
    // SAFETY: The caller guarantees the readable size prefix of a nonnull host.
    let status = unsafe { validate_size(host) };
    if status.is_err() {
        return status;
    }
    // SAFETY: The advertised complete size grants the stable version prefix.
    if unsafe { ptr::addr_of!((*host).abi_version).read() } != NC_COM_ABI_VERSION {
        return NC_E_ABI_VERSION;
    }
    // SAFETY: The size and supported version grant the complete host record.
    let host = unsafe { &*host };
    // SAFETY: The caller grants each reachable table's documented size prefix.
    let status = unsafe { validate_size(host.runtime) };
    if status.is_err() {
        return status;
    }
    // SAFETY: Same size prefix contract for the diagnostics service.
    let status = unsafe { validate_size(host.debug) };
    if status.is_err() {
        return status;
    }
    // SAFETY: Same size prefix contract for the persistence service.
    unsafe { validate_size(host.persistence) }
}

/// Checks a required record size without interpreting other fields.
///
/// # Safety
/// Nonnull storage is aligned and readable for its leading `usize`.
unsafe fn validate_size<T>(table: *const T) -> NcStatus {
    if table.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The caller supplies the aligned, initialized size prefix.
    if unsafe { table.cast::<usize>().read() } != mem::size_of::<T>() {
        return NC_E_ABI_VERSION;
    }
    NC_OK
}
