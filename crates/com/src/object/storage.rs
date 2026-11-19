//! Provider-local storage for generated objects. No registry state lives here.
//!
//! The policy-specific C header stays at offset zero. The access choices retain
//! their separate roles: a separate gate for shared ownership, no extra state
//! for exclusive ownership. Weak owners may retain this allocation after T dies.

use crate::__alloc::boxed::Box;
use core::{cell::UnsafeCell, ffi::c_void, mem::MaybeUninit};
use crate::{ComBox, ComObject, ComPtr, ComRaw, IUnknown, NcStatus, NC_E_POINTER};
use crate::lifetime::{BoxRead, BoxWrite, NcObjectTypeInfo, NcSharedLifetimeState, NcBoxedLifetimeState};
use super::{ComBorrowGate, ComReadGuard, ComWriteGuard, __ComWritePermit, __ComReadPermit};

/// Heap allocation for a concrete Rust COM implementation.
///
/// Lifetime is the concrete shared or boxed header. Access is the shared gate
/// or unit, selected statically; neither policy nor the concrete Rust layout
/// crosses an interface call boundary. Weak owners can retain this block after
/// `T` is destroyed, so the value slot does not itself own a `T` destructor.
#[doc(hidden)]
#[repr(C)]
pub struct ComObjectInner<T, Lifetime, Access> {
    /// Concrete shared or boxed lifetime header, selected statically at offset zero.
    control: Lifetime,
    /// Shared classes' call gate; exclusive classes borrow through `control`.
    access: Access,
    /// Initialized until owner destruction; retained storage never drops T twice.
    value: UnsafeCell<MaybeUninit<T>>,
}

/// Allocation layout for a shared provider with its own Rust borrow gate.
#[doc(hidden)]
pub type __ComSharedInner<T> = ComObjectInner<T, NcSharedLifetimeState, ComBorrowGate>;

/// Allocation layout for an exclusive provider using common-header admission.
#[doc(hidden)]
pub type __ComBoxedInner<T> = ComObjectInner<T, NcBoxedLifetimeState, ()>;

impl<T: ComObject, Lifetime: 'static, Access: 'static> ComObjectInner<T, Lifetime, Access> {
    /// Builds immutable interface metadata and callbacks for this exact storage layout.
    const fn type_info() -> NcObjectTypeInfo {
        NcObjectTypeInfo {
            destroy_value: Self::destroy_value,
            deallocate: Self::deallocate,
            interfaces: T::VTABLES.as_ptr(),
            interface_count: T::VTABLES.len(),
        }
    }

    /// Allocates one initialized value; only the matching ownership constructor
    /// can select the control state and convert the result into a safe owner.
    fn allocate(value: T, control: Lifetime, access: Access) -> ComRaw<IUnknown> {
        let storage = Box::into_raw(Box::new(Self {
            control,
            access,
            value: UnsafeCell::new(MaybeUninit::new(value)),
        }));
        ComRaw {
            object: storage.cast(),
            vtable: T::UNKNOWN_VTABLE,
        }
    }

    /// Destroys only T while the implicit weak owner still retains the header.
    unsafe extern "C" fn destroy_value(control: *mut c_void) {
        let storage = control.cast::<Self>();
        // SAFETY: the class descriptor belongs to this exact layout. The common
        // lifetime engine has excluded future value access and admitted calls.
        unsafe { (*(*storage).value.get()).assume_init_drop() };
    }

    /// Frees the exact provider allocation after value destruction and final weak release.
    unsafe extern "C" fn deallocate(control: *mut c_void) {
        // SAFETY: the final weak owner transfers this allocation once. MaybeUninit
        // suppresses a second T destructor, and Box preserves concrete alignment.
        unsafe { drop(Box::from_raw(control.cast::<Self>())) };
    }
}

impl<T: ComObject> __ComSharedInner<T> {
    /// Callbacks and interfaces belong to this concrete shared storage instantiation.
    const CLASS: NcObjectTypeInfo = Self::type_info();

    /// Allocates shared storage and transfers its initial strong owner.
    ///
    /// # Safety
    /// T's ComObject implementation and every paired table must recover exactly
    /// this storage layout and use its gate. T's threading proof must hold.
    pub unsafe fn into_shared(value: T) -> ComPtr<IUnknown, T::Threading> {
        let raw = Self::allocate(value, NcSharedLifetimeState::new(&Self::CLASS), ComBorrowGate::new());
        // SAFETY: allocation initialized one strong reference and its canonical table.
        unsafe { ComPtr::from_raw_owned(raw) }.expect("new shared object is present")
    }

    /// Recovers shared access through this allocation's one call gate.
    ///
    /// # Safety
    /// A non-null object must identify this exact initialized storage, retained
    /// by a strong owner for the entire returned lifetime. All T access uses its gate.
    pub unsafe fn borrow_ref<'a>(object: *mut c_void) -> Result<ComReadGuard<'a, T, __ComReadPermit<'a>>, NcStatus> {
        // SAFETY: caller retains matching storage; null is rejected before field access.
        let storage = unsafe { object.cast::<Self>().as_ref() }.ok_or(NC_E_POINTER)?;
        // SAFETY: the guard acquires the gate before constructing a reference to T.
        unsafe { ComReadGuard::acquire(&storage.access, &storage.value) }
    }

    /// Recovers exclusive access through the shared allocation's call gate.
    ///
    /// # Safety
    /// The same matching-storage and retained-owner obligations as borrow_ref apply.
    pub unsafe fn borrow_mut<'a>(object: *mut c_void) -> Result<ComWriteGuard<'a, T, __ComWritePermit<'a>>, NcStatus> {
        // SAFETY: caller retains matching storage throughout the guard.
        let storage = unsafe { object.cast::<Self>().as_ref() }.ok_or(NC_E_POINTER)?;
        // SAFETY: acquisition rejects readers/reentrant writers before creating &mut T.
        unsafe { ComWriteGuard::acquire(&storage.access, &storage.value) }
    }
}

impl<T: ComObject> __ComBoxedInner<T> {
    /// Callbacks and interfaces belong to this concrete boxed storage instantiation.
    const CLASS: NcObjectTypeInfo = Self::type_info();

    /// Allocates exclusive storage and transfers its sole owner.
    ///
    /// # Safety
    /// T's ComObject implementation and paired tables must recover exactly this
    /// layout and admit calls through the header before accessing T. Threading must match.
    pub unsafe fn into_exclusive(value: T) -> ComBox<IUnknown, T::Threading> {
        let raw = Self::allocate(value, NcBoxedLifetimeState::new(&Self::CLASS), ());
        // SAFETY: allocation initialized and transfers exactly one exclusive owner.
        unsafe { ComBox::from_raw_owned(raw) }.expect("new exclusive object is present")
    }

    /// Admits a reader atomically against exclusive owner removal.
    ///
    /// # Safety
    /// A non-null object identifies this exact storage, retained for the returned
    /// lifetime. T may already be destroyed; header admission must precede any T reference.
    pub unsafe fn borrow_ref<'a>(object: *mut c_void) -> Result<ComReadGuard<'a, T, BoxRead>, NcStatus> {
        // SAFETY: MaybeUninit keeps the retained storage valid even after T dies.
        let storage = unsafe { object.cast::<Self>().as_ref() }.ok_or(NC_E_POINTER)?;
        // SAFETY: acquisition checks owner presence and borrowing before referencing T.
        unsafe { ComReadGuard::acquire_box(object.cast(), &storage.value) }
    }

    /// Admits the unique writer atomically against owner removal and other calls.
    ///
    /// # Safety
    /// The same matching-storage and retained-storage obligations as borrow_ref apply.
    pub unsafe fn borrow_mut<'a>(object: *mut c_void) -> Result<ComWriteGuard<'a, T, BoxWrite>, NcStatus> {
        // SAFETY: caller retains the header; no T reference has been constructed.
        let storage = unsafe { object.cast::<Self>().as_ref() }.ok_or(NC_E_POINTER)?;
        // SAFETY: header admission excludes destruction and conflicting borrows.
        unsafe { ComWriteGuard::acquire_box(object.cast(), &storage.value) }
    }
}

#[cfg(test)]
mod tests {
    //! Physical layout is an invariant of the common C lifetime header.
    use super::*;
    use core::mem::{offset_of, size_of};

    /// Shared calls retain their gate; exclusive admission adds no second word.
    #[test]
    fn layouts_preserve_header_prefix_and_exclusive_zero_sized_access() {
        let word = size_of::<usize>();
        assert_eq!(offset_of!(__ComSharedInner<u8>, control), 0);
        assert_eq!(offset_of!(__ComBoxedInner<u8>, control), 0);
        assert_eq!(offset_of!(__ComSharedInner<u8>, value), 4 * word);
        assert_eq!(offset_of!(__ComBoxedInner<u8>, value), 3 * word);
        assert_eq!(size_of::<__ComSharedInner<u8>>(), 5 * word);
        assert_eq!(size_of::<__ComBoxedInner<u8>>(), 4 * word);
    }
}
