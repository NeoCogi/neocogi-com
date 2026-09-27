//! Immutable, runtime-owned child lists lent to persistence factories.
//!
//! Only COM reference counting changes after publication. Factories may retain
//! the list or individual children; their final Release always frees storage in
//! the runtime that allocated it. No facade object machinery is required here.

use core::{ffi::c_void, ptr, sync::atomic::AtomicU32};
use neocogi_com_core::{
    ComInterface, ComPtr, IUnknown, IUnknownVTable, NcInterfaceId, NcStatus, __com_add_ref_count, __com_release_ref_count, NC_E_ABI_VERSION, NC_E_NOINTERFACE,
    NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_OK,
    persistence::{IObjectChildrenAbi, IObjectChildrenVTable},
};

/// One allocation owns both interface projections and all constructed children.
struct Children {
    /// Shared strong count for the canonical identity and list projection.
    references: AtomicU32,
    /// Canonical identity, with the exact IUnknown interface identity.
    unknown: IUnknown,
    /// Read-only list projection carrying the persistence contract identity.
    interface: IObjectChildrenAbi,
    /// Fully constructed descendants, released when the final list owner leaves.
    values: Vec<ComPtr<IUnknown>>,
}

/// Publishes a complete immutable list with one owned interface reference.
pub(super) fn create(values: Vec<ComPtr<IUnknown>>) -> Result<ComPtr<IObjectChildrenAbi>, NcStatus> {
    // Reject a count that the u32 list contract cannot represent before publish.
    u32::try_from(values.len()).map_err(|_| NC_E_OUT_OF_RANGE)?;
    let object = Box::into_raw(Box::new(Children {
        references: AtomicU32::new(1),
        unknown: IUnknown { vtable: &UNKNOWN_VTABLE },
        interface: IObjectChildrenAbi { vtable: &CHILDREN_VTABLE },
        values,
    }));
    // SAFETY: This allocation is exclusively owned and not yet published.
    // Static vtables already describe both embedded headers before publication.
    unsafe { ComPtr::from_raw_owned(ptr::addr_of_mut!((*object).interface)).ok_or(NC_E_POINTER) }
}

/// Immutable lifecycle table used by canonical identity projections.
static UNKNOWN_VTABLE: IUnknownVTable = IUnknownVTable {
    // The embedded identity describes this projection, not its concrete storage.
    interface_id: &<IUnknown as ComInterface>::INTERFACE_ID,
    object_offset: core::mem::offset_of!(Children, unknown),
    query_interface,
    add_ref,
    release,
};

/// Immutable list table; both projections share the same lifecycle callbacks.
static CHILDREN_VTABLE: IObjectChildrenVTable = IObjectChildrenVTable {
    unknown: IUnknownVTable {
        interface_id: &<IObjectChildrenAbi as ComInterface>::INTERFACE_ID,
        object_offset: core::mem::offset_of!(Children, interface),
        query_interface,
        add_ref,
        release,
    },
    count,
    get,
};

/// Recovers immutable storage from either of this allocation's live projections.
///
/// # Safety
/// Non-null `this` must be one of our live interface headers for the returned borrow.
unsafe fn storage<'a>(this: *mut IUnknown) -> Result<&'a Children, NcStatus> {
    if this.is_null() {
        return Err(NC_E_POINTER);
    }
    // SAFETY: The static vtable offset locates this header inside its live Children allocation.
    unsafe { neocogi_com_core::__nc_object_from_interface(this.cast()).cast::<Children>().as_ref() }.ok_or(NC_E_POINTER)
}

/// Returns only exact interface identities and clears outputs before validation.
///
/// # Safety
/// `this` is live; non-null requested/out point to readable/writable ABI storage.
unsafe extern "C" fn query_interface(this: *mut IUnknown, requested: *const NcInterfaceId, out: *mut *mut c_void) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The caller grants writable output storage.
    unsafe { out.write(ptr::null_mut()) };
    if requested.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The live projection and readable identity satisfy this callback's contract.
    let (object, requested) = match unsafe { storage(this) } {
        Ok(object) => (object, unsafe { &*requested }),
        Err(status) => return status,
    };
    let projection = match requested.iid {
        id if id == IUnknown::IID => {
            if *requested != IUnknown::INTERFACE_ID {
                return NC_E_ABI_VERSION;
            }
            ptr::from_ref(&object.unknown).cast_mut().cast()
        }
        id if id == IObjectChildrenAbi::IID => {
            if *requested != IObjectChildrenAbi::INTERFACE_ID {
                return NC_E_ABI_VERSION;
            }
            ptr::from_ref(&object.interface).cast_mut().cast()
        }
        _ => return NC_E_NOINTERFACE,
    };
    __com_add_ref_count(&object.references);
    // SAFETY: The new reference owns the selected projection from this allocation.
    unsafe { out.write(projection) };
    NC_OK
}

/// Retains the allocation through either interface projection.
///
/// # Safety
/// Non-null `this` must be a live projection from `create`.
unsafe extern "C" fn add_ref(this: *mut IUnknown) -> u32 {
    // SAFETY: The caller owns a live reference for this synchronous call.
    match unsafe { storage(this) } {
        Ok(object) => __com_add_ref_count(&object.references),
        Err(_) => 0,
    }
}

/// Drops descendants and storage once both projections lose their final owner.
///
/// # Safety
/// Non-null `this` must transfer one owned reference from `create` or AddRef.
unsafe extern "C" fn release(this: *mut IUnknown) -> u32 {
    // SAFETY: The transferred reference keeps storage alive until the decrement.
    let object = match unsafe { storage(this) } {
        Ok(object) => object,
        Err(_) => return 0,
    };
    let pointer = ptr::from_ref(object).cast_mut();
    match __com_release_ref_count(&object.references) {
        Some(0) => {
            // SAFETY: The last reference owns the original Box allocation;
            // no access to object or its projections follows this destruction.
            unsafe { drop(Box::from_raw(pointer)) };
            0
        }
        Some(remaining) => remaining,
        None => 0,
    }
}

/// Reports the count checked before the list was published.
///
/// # Safety
/// `this` is a live list; non-null `out` accepts one u32 write.
unsafe extern "C" fn count(this: *mut IObjectChildrenAbi, out: *mut u32) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: Every list projection shares the common header prefix.
    match unsafe { storage(this.cast()) } {
        Ok(object) => {
            // SAFETY: The count fits u32 and the output is writable.
            unsafe { out.write(object.values.len() as u32) };
            NC_OK
        }
        Err(status) => status,
    }
}

/// Transfers one independently owned child while preserving list ownership.
///
/// # Safety
/// `this` is a live list; non-null `out` accepts one interface-pointer write.
unsafe extern "C" fn get(this: *mut IObjectChildrenAbi, index: u32, out: *mut *mut IUnknown) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: Clear failures before entering any child reference-count callback.
    unsafe { out.write(ptr::null_mut()) };
    // SAFETY: The live list carries our common storage prefix.
    let object = match unsafe { storage(this.cast()) } {
        Ok(object) => object,
        Err(status) => return status,
    };
    match object.values.get(index as usize) {
        Some(child) => {
            // SAFETY: Clone acquires a new owner and out transfers it to the caller.
            unsafe { out.write(child.clone().into_raw()) };
            NC_OK
        }
        None => NC_E_OUT_OF_RANGE,
    }
}
