//! Allocator-local lifetime operations used by generated COM objects.
//!
//! Concrete callbacks supply their static tables as data. Keeping these bodies
//! out of line lets the optimizer share operations for storage types with the
//! same layout instead of embedding different table addresses in every body.
//! Only public Arc/Weak operations are used; their representation never crosses
//! the COM boundary and typed destruction remains in the provider's Release.
use crate::__alloc::sync::{Arc, Weak};
use core::{ffi::c_void, mem::ManuallyDrop};
use crate::{ComRaw, IUnknown, IUnknownVTable, NcInterfaceId, NcStatus, NcWeakRaw, NcWeakVTable, NC_E_CLOSED, NC_E_POINTER, NC_OK};

/// Transfers one weak reference from a generated object's live strong allocation.
///
/// # Safety
/// A non-null object must identify Arc<T> storage retained throughout this call.
/// A non-null output must be writable and distinct from that storage. The table
/// must use the same T and allocator and remain available through every weak owner.
#[doc(hidden)]
#[inline(never)]
pub unsafe fn __com_arc_downgrade<T>(object: *mut c_void, out: *mut NcWeakRaw, vtable: &'static NcWeakVTable) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: the caller supplies a distinct writable output record.
    unsafe {
        *out = NcWeakRaw::null();
    }
    if object.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: borrow the existing strong reference without consuming its owner.
    let owner = ManuallyDrop::new(unsafe { Arc::<T>::from_raw(object.cast()) });
    let weak = Arc::downgrade(&owner);
    // SAFETY: transfer the new weak reference with the provider's matching table.
    unsafe {
        *out = NcWeakRaw {
            control: Weak::into_raw(weak).cast_mut().cast(),
            vtable,
        };
    }
    NC_OK
}

/// Promotes once and transfers that owner through the exact requested projection.
///
/// # Safety
/// A non-null control must be a borrowed Weak<T> token. Non-null input/output
/// records must be readable/writable and distinct. Tables must be nonempty,
/// immutable generated projections for T; their common Release must consume an
/// Arc<T> owner through this provider. They must outlive every weak token.
#[doc(hidden)]
#[inline(never)]
pub unsafe fn __com_arc_upgrade<T>(
    control: *mut c_void,
    requested: *const NcInterfaceId,
    out: *mut ComRaw<IUnknown>,
    tables: &[*const IUnknownVTable],
) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: reset the caller's distinct output before any fallible operation.
    unsafe {
        *out = ComRaw::null();
    }
    if control.is_null() || requested.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: this borrow leaves the caller's weak reference intact.
    let weak = ManuallyDrop::new(unsafe { Weak::<T>::from_raw(control.cast()) });
    let owner = match weak.upgrade() {
        Some(owner) => owner,
        None => return NC_E_CLOSED,
    };
    // Transfer the acquired reference immediately. Failure releases it through
    // the provider table, so this shared routine needs no T-specific destructor.
    let object = Arc::into_raw(owner).cast_mut().cast();
    // SAFETY: generated tables and the requested identity satisfy the lookup contract.
    match unsafe { crate::__nc_find_interface(tables, &*requested) } {
        Ok(vtable) => {
            // SAFETY: lookup validated this projection; the acquired owner transfers once.
            unsafe {
                *out = ComRaw { object, vtable };
            }
            NC_OK
        }
        Err(status) => {
            // SAFETY: every table belongs to T and releases through its allocating provider.
            unsafe {
                ((*tables[0]).release)(object);
            }
            status
        }
    }
}
