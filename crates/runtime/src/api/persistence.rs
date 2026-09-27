//! C entry points for caller-scoped persistence contexts and snapshot delivery.
//!
//! These adapters validate pointers before creating Rust borrows. Algorithms
//! and registry ownership remain in `crate::persistence`; no global handle map,
//! erased Rust value, or interior-mutable registry is introduced.

use core::{ffi::c_void, mem, ptr, slice};
use neocogi_com_core::{
    ComRef, IUnknown, NcGuid, NcPersistenceApiV1, NcPersistenceContext, NcPersistenceLimitsV1, NcPersistenceWriteBytesV1, NcStatus, NC_COM_CORE_ABI_VERSION,
    NC_E_INVALID_ARG, NC_E_POINTER, NC_OK, persistence::IPersistenceFactoryAbi,
};
use crate::persistence::{Registry, snapshot_with_limits};

/// Immutable operation table; a getter lends it for the runtime's entire lifetime.
pub(super) static PERSISTENCE_API_V1: NcPersistenceApiV1 = NcPersistenceApiV1 {
    size: mem::size_of::<NcPersistenceApiV1>(),
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    create_context,
    destroy_context,
    register_factory,
    snapshot,
    restore,
};

/// Allocates an empty registry in this runtime and transfers its opaque ownership.
///
/// # Safety
/// Non-null `out` must be writable for one context pointer and not alias live runtime data.
unsafe extern "C" fn create_context(out: *mut *mut NcPersistenceContext) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The output is writable; no factory callbacks run during allocation.
    unsafe {
        out.write(ptr::null_mut());
        out.write(Box::into_raw(Box::new(Registry::new())).cast());
    }
    NC_OK
}

/// Consumes the unique context owner and releases all retained factories locally.
///
/// # Safety
/// Non-null `context` must be a live allocation from this table, with no active
/// calls or aliases that will be used again. It must be on its creating thread.
unsafe extern "C" fn destroy_context(context: *mut NcPersistenceContext) {
    if !context.is_null() {
        // SAFETY: Context ownership returns the original Box to its allocator.
        unsafe { drop(Box::from_raw(context.cast::<Registry>())) };
    }
}

/// Takes an explicit exclusive registry borrow and retains a compatible factory.
///
/// # Safety
/// Context is exclusively borrowed on its creating thread; factory is a live
/// common COM projection. Neither may be invalidated by reentrant callbacks.
unsafe extern "C" fn register_factory(context: *mut NcPersistenceContext, id: NcGuid, factory: neocogi_com_core::ComRaw<IPersistenceFactoryAbi>) -> NcStatus {
    if context.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: Validate the exact ABI before calling any factory-specific methods.
    let factory = match unsafe { ComRef::<IPersistenceFactoryAbi>::__nc_from_raw_borrowed_checked(factory) } {
        Ok(factory) => factory.add_ref(),
        Err(status) => return status,
    };
    // SAFETY: The ABI requires exclusive context access for this operation.
    match unsafe { &mut *context.cast::<Registry>() }.register(id, factory) {
        Ok(()) => NC_OK,
        Err(status) => status.normalize_error(),
    }
}

/// Delivers one completed document through a synchronous caller-owned byte sink.
///
/// # Safety
/// Object is a live common COM projection. The callback and its opaque data
/// satisfy the declared byte-sink contract and must not unwind across the ABI.
unsafe extern "C" fn snapshot(
    object: neocogi_com_core::ComRaw<IUnknown>,
    limits: NcPersistenceLimitsV1,
    write: Option<NcPersistenceWriteBytesV1>,
    user_data: *mut c_void,
) -> NcStatus {
    let write = match write {
        Some(write) => write,
        None => return NC_E_POINTER,
    };
    // SAFETY: Exact identity validation precedes the algorithm's safe COM calls.
    let object = match unsafe { ComRef::<IUnknown>::__nc_from_raw_borrowed_checked(object) } {
        Ok(object) => object,
        Err(status) => return status,
    };
    match snapshot_with_limits(object, limits.into()) {
        Ok(bytes) => {
            // SAFETY: The vector remains live for this single synchronous copy;
            // ownership stays in the runtime even if the receiver reports failure.
            unsafe { write(user_data, bytes.as_ptr(), bytes.len()) }
        }
        Err(status) => status.normalize_error(),
    }
}

/// Validates a bounded byte view, reconstructs a full tree and transfers its root.
///
/// # Safety
/// Context is shared-borrowed on its creating thread for the complete call.
/// Input is readable for len bytes and output accepts one distinct pointer write.
unsafe extern "C" fn restore(
    context: *const NcPersistenceContext,
    bytes: *const u8,
    len: usize,
    limits: NcPersistenceLimitsV1,
    out: *mut neocogi_com_core::ComRaw<IUnknown>,
) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: Initialize the caller's output before any validation or provider call.
    unsafe { out.write(neocogi_com_core::ComRaw::null()) };
    if context.is_null() || (len != 0 && bytes.is_null()) {
        return NC_E_POINTER;
    }
    // Reject impossible slice sizes before constructing a Rust reference. The
    // document budget is checked here as well as by the binary decoder.
    if len > isize::MAX as usize || len > limits.bytes {
        return NC_E_INVALID_ARG;
    }
    let bytes = match len {
        0 => &[],
        // SAFETY: The input contract and bounds check establish a valid slice.
        _ => unsafe { slice::from_raw_parts(bytes, len) },
    };
    // SAFETY: The ABI caller lends this exact runtime allocation immutably.
    let registry = unsafe { &*context.cast::<Registry>() };
    match registry.restore_with_limits(bytes, limits.into()) {
        Ok(root) => {
            // SAFETY: The output receives one owned COM reference, not a Rust Box.
            unsafe { out.write(root.into_raw()) };
            NC_OK
        }
        Err(status) => status.normalize_error(),
    }
}

#[cfg(test)]
mod tests {
    //! Raw callers receive deterministic failures before invalid inputs are read.

    use super::*;
    use neocogi_com_core::persistence::Limits;

    /// Rejects missing pointers and clears restore outputs on every failure path.
    #[test]
    fn invalid_arguments_never_publish_a_context_or_root() {
        let limits = Limits::default().into();
        let mut context = ptr::null_mut();
        let mut root = neocogi_com_core::ComRaw {
            object: ptr::dangling_mut(),
            vtable: ptr::null(),
        };
        // SAFETY: Null arguments deliberately exercise defensive rejection; all
        // non-null output slots and the allocated context remain valid here.
        unsafe {
            assert_eq!(create_context(ptr::null_mut()), NC_E_POINTER);
            assert_eq!(create_context(&mut context), NC_OK);
            assert!(!context.is_null());
            assert_eq!(
                register_factory(context, NcGuid::from_uuid_bytes([0; 16]), neocogi_com_core::ComRaw::null()),
                NC_E_POINTER
            );
            assert_eq!(restore(context, ptr::null(), 1, limits, &mut root), NC_E_POINTER);
            assert!(root.is_null());
            root = neocogi_com_core::ComRaw {
                object: ptr::dangling_mut(),
                vtable: ptr::null(),
            };
            assert_eq!(restore(ptr::null(), ptr::null(), 0, limits, &mut root), NC_E_POINTER);
            assert!(root.is_null());
            assert_eq!(restore(context, ptr::dangling(), usize::MAX, limits, &mut root), NC_E_INVALID_ARG);
            assert!(root.is_null());
            assert_eq!(snapshot(neocogi_com_core::ComRaw::null(), limits, None, ptr::null_mut()), NC_E_POINTER);
            destroy_context(context);
            destroy_context(ptr::null_mut());
        }
    }
}
