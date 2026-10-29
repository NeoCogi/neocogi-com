//! Supplied access to the host's one runtime implementation.
//!
//! This record groups existing C operation tables without owning their state.
//! Admission checks only readable prefixes and supported layouts. The caller of
//! a consuming bootstrap separately guarantees immutable process-lived backing,
//! callable code and one coherent host owner for the complete reachable graph.

use core::{mem, ptr};

use super::{NC_COM_RUNTIME_ABI_VERSION, NcDebugApiV1, NcPersistenceApiV1, NcRuntimeApiV1};
use crate::{NC_COM_CORE_ABI_VERSION, NC_E_ABI_VERSION, NC_E_POINTER, NC_OK, NcStatus};

/// Immutable C description supplied by the host when acquiring a plugin factory.
///
/// All three tables belong to the same host implementation. Copies of this
/// record copy access, never a registry or another runtime owner. The host keeps
/// its tables, callback code and state alive for the process/Wasm-instance
/// lifetime, including after an authored application main returns. Host entrypoints
/// support concurrent access. Caller-owned persistence contexts remain subject
/// to their own thread-local and exclusive-borrow operation contracts.
///
/// Core generation 18 freezes this layout and its callbacks. Deploy host and
/// providers from one published SDK release; discovery family 1 is separate.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcHostApiV1 {
    /// Available record bytes, including the size/version prefix.
    pub size: usize,
    /// Runtime table family; must equal [`NC_COM_RUNTIME_ABI_VERSION`].
    pub runtime_abi_version: u32,
    /// Core object/descriptor ABI understood by the supplied implementation.
    pub core_abi_version: u32,
    /// Required immutable plugin/catalog operation table in the host.
    pub runtime: *const NcRuntimeApiV1,
    /// Required immutable diagnostics operation table in the same host.
    pub debug: *const NcDebugApiV1,
    /// Required immutable caller-owned persistence operation table in that host.
    pub persistence: *const NcPersistenceApiV1,
}

// SAFETY: Sharing the record exposes only const raw pointers. Dereferencing them
// still requires the documented backing/immutability/callable-code obligations;
// no safe operation on this record mutates or dereferences foreign storage.
unsafe impl Sync for NcHostApiV1 {}

/// Checks the host record and required table prefixes without invoking code.
///
/// Null pointers return `NC_E_POINTER`; short records or unsupported runtime/core
/// versions return `NC_E_ABI_VERSION`. No full table reference is constructed
/// before admission. Success proves neither lifetime nor common ownership.
///
/// # Safety
///
/// A nonnull host pointer is aligned and readable for its leading `usize`. If
/// its size includes the current record, its version prefix is readable. Only
/// supported versions require the complete immutable current host record. Each
/// nonnull child is aligned and readable for its size prefix; a sufficiently
/// large core-versioned child also has a readable core-version field. Matching
/// supported children describe valid current tables. This validator retains no
/// pointer, so backing need only survive this call; adopting process-lived access
/// additionally requires the complete immutable coherent graph for that lifetime.
pub unsafe fn validate_host_api(host: *const NcHostApiV1) -> NcStatus {
    // SAFETY: The caller guarantees the readable size prefix of a nonnull host.
    let status = unsafe { validate_size(host) };
    if status.is_err() {
        return status;
    }
    // Read only version fields before borrowing a supported complete layout.
    // SAFETY: A sufficient advertised size grants this stable version prefix.
    let (runtime_version, core_version) = unsafe {
        (
            ptr::addr_of!((*host).runtime_abi_version).read(),
            ptr::addr_of!((*host).core_abi_version).read(),
        )
    };
    if runtime_version != NC_COM_RUNTIME_ABI_VERSION || core_version != NC_COM_CORE_ABI_VERSION {
        return NC_E_ABI_VERSION;
    }
    // SAFETY: Supported versions and size now grant the complete host record.
    let host = unsafe { &*host };
    // SAFETY: The caller grants each reachable table's documented prefix.
    let status = unsafe { validate_core_table(host.runtime) };
    if status.is_err() {
        return status;
    }
    // Diagnostics has only a byte-size prefix, versioned by the outer record.
    // SAFETY: The caller grants the debug table's size prefix.
    let status = unsafe { validate_size(host.debug) };
    if status.is_err() {
        return status;
    }
    // SAFETY: The caller grants the persistence table's size/core prefix.
    unsafe { validate_core_table(host.persistence) }
}

/// Checks a required table's size without interpreting any function pointers.
///
/// # Safety
/// Nonnull storage is aligned and readable for one leading `usize`; all callers
/// use records whose first C field is that prefix.
unsafe fn validate_size<T>(table: *const T) -> NcStatus {
    if table.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The checked nonnull pointer has the caller-provided size prefix.
    if unsafe { table.cast::<usize>().read() } < mem::size_of::<T>() {
        NC_E_ABI_VERSION
    } else {
        NC_OK
    }
}

/// Shared stable prefix of plugin/catalog and persistence operation tables.
#[repr(C)]
struct CoreTablePrefix {
    /// Advertised complete table size.
    size: usize,
    /// Core object ABI understood by that table's callbacks.
    core_abi_version: u32,
}

/// Checks size and core generation before any complete table borrow.
///
/// # Safety
/// Nonnull storage has a readable leading size. A sufficiently large table has
/// the `CoreTablePrefix` fields; callers pass only tables with this C prefix.
unsafe fn validate_core_table<T>(table: *const T) -> NcStatus {
    // SAFETY: Forwarded readable leading-size obligation.
    let status = unsafe { validate_size(table) };
    if status.is_err() {
        return status;
    }
    let prefix = table.cast::<CoreTablePrefix>();
    // SAFETY: The caller grants this stable field after the advertised-size check.
    if unsafe { ptr::addr_of!((*prefix).core_abi_version).read() } != NC_COM_CORE_ABI_VERSION {
        NC_E_ABI_VERSION
    } else {
        NC_OK
    }
}
