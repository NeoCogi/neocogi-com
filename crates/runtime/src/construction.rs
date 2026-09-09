//! Interface validation and construction against registered class metadata.
//!
//! Construction first resolves a class, checks requested interface and
//! threading capabilities from immutable registry records, and only then calls
//! the plugin constructor. Raw output slots are cleared before fallible work so
//! no failure can expose stale or partially initialized object pointers.
//!
//! Lookup happens under the runtime catalog read lock, but plugin constructors
//! run after the lock is released. Plugin code may re-enter runtime APIs, so
//! calling it while a catalog guard is held would deadlock an activation that
//! needs exclusive publication access.

use neocogi_com_core::{
    IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_ABI_THREADING_THREAD_SAFE, NC_E_ABI_VERSION, NC_E_INVALID_ARG, NC_E_NOINTERFACE, NC_E_NOT_FOUND, NC_E_POINTER,
    NcAbiHash, NcClassDescriptor, NcGuid, NcInterfaceId, NcStatus,
};

use crate::type_registry::TypeRegistry;

/// Requires that a registered class expose the exact requested IID/hash pair.
pub(crate) fn require_interface(
    registry: &TypeRegistry,
    class_id: NcGuid,
    interface_id: NcInterfaceId,
) -> Result<(), NcStatus> {
    let Some(interfaces) = registry.class_interface_ids(class_id) else {
        return Err(NC_E_NOT_FOUND);
    };
    require_interface_in_ids(&interfaces, interface_id)
}

/// Resolves and validates one class without invoking plugin code.
pub(crate) fn resolve_class(
    registry: &TypeRegistry,
    class_id: NcGuid,
    requested_threading: u32,
) -> Result<*const NcClassDescriptor, NcStatus> {
    let Some(class) = registry.class(class_id) else {
        return Err(NC_E_NOT_FOUND);
    };
    // SAFETY: Registry entries contain validated readable descriptors.
    let descriptor = unsafe { &*class.class };
    if !threading_request_is_compatible(descriptor.threading, requested_threading) {
        return Err(NC_E_INVALID_ARG);
    }
    Ok(class.class)
}

/// Constructs one object from a class resolved under the catalog read lock.
///
/// # Safety
///
/// A non-null `arguments` pointer must identify an immutable frame matching
/// `constructor_abi_hash` and remain readable through the constructor call.
/// `out` must be valid for one pointer write.
pub(crate) unsafe fn construct_class(
    class: Result<*const NcClassDescriptor, NcStatus>,
    constructor_abi_hash: NcAbiHash,
    arguments: *const core::ffi::c_void,
    out: *mut *mut IUnknown,
) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `out` is valid for one write by the caller's contract.
    unsafe { *out = core::ptr::null_mut() };
    let class = match class {
        Ok(class) => class,
        Err(status) => return status,
    };
    // Only the immutable descriptor pointer escaped the catalog guard.
    // Registration and module retention guarantee its lifetime across the call.
    // SAFETY: This forwards the caller's argument and output contracts.
    unsafe { construct_class_descriptor(class, constructor_abi_hash, arguments, out) }
}

/// Applies the capability rule from a class declaration to a host request.
fn threading_request_is_compatible(declared: u32, requested: u32) -> bool {
    match requested {
        NC_ABI_THREADING_THREAD_LOCAL => true,
        NC_ABI_THREADING_THREAD_SAFE => declared == NC_ABI_THREADING_THREAD_SAFE,
        _ => false,
    }
}

/// Distinguishes a missing IID from an IID present with a mismatched ABI hash.
fn require_interface_in_ids(
    interfaces: &[NcInterfaceId],
    interface_id: NcInterfaceId,
) -> Result<(), NcStatus> {
    for registered in interfaces {
        if registered.iid != interface_id.iid {
            continue;
        }
        // An IID match with a different hash is an ABI-version mismatch, not a
        // missing interface. Returning immediately preserves that distinction.
        if registered.abi_hash == interface_id.abi_hash {
            return Ok(());
        }
        return Err(NC_E_ABI_VERSION);
    }
    Err(NC_E_NOINTERFACE)
}

/// Validates and invokes one immutable registered constructor descriptor.
unsafe fn construct_class_descriptor(
    class: *const NcClassDescriptor,
    constructor_abi_hash: NcAbiHash,
    arguments: *const core::ffi::c_void,
    out: *mut *mut IUnknown,
) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `out` was checked non-null and is valid for one pointer write.
    unsafe { *out = core::ptr::null_mut() };
    if class.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: Registry entries only contain validated class pointers.
    let class = unsafe { &*class };
    let Some(construct) = class.construct else {
        return NC_E_INVALID_ARG;
    };
    if constructor_abi_hash != class.constructor_abi_hash {
        return NC_E_ABI_VERSION;
    }
    // Null is the complete no-argument frame. A typed frame is always non-null,
    // so the pointer shape can be rejected before plugin code sees it.
    if arguments.is_null() != (constructor_abi_hash == NcAbiHash::ZERO) {
        return NC_E_INVALID_ARG;
    }
    // SAFETY: The constructor and descriptor came from validated metadata.
    let status = unsafe { construct(class, arguments, out) };
    if status.is_err() {
        // Constructors are required to clear outputs on failure, but the runtime
        // enforces the invariant again before returning across its own C ABI.
        // SAFETY: `out` was checked above.
        unsafe { *out = core::ptr::null_mut() };
        return status;
    }
    // SAFETY: A successful constructor must write one owned object pointer.
    if unsafe { (*out).is_null() } {
        return NC_E_POINTER;
    }
    status
}
