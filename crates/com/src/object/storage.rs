//! Generated class allocation with typed backing and allocator-local cleanup.

use core::{ffi::c_void, mem::MaybeUninit, ptr::NonNull};

use crate::__alloc::boxed::Box;
use crate::lifetime::{NcBoxedLifetimeState, NcObjectTypeInfo, NcSharedLifetimeState};
use crate::{ComBox, ComObject, ComPtr, ComRaw, IUnknown, NcStatus, NC_E_POINTER};

/// Live value stored immediately after its matching policy header, with T's
/// alignment. The value is initialized before publication and dropped before
/// storage is freed.
#[doc(hidden)]
#[repr(C)]
pub struct ComObjectInner<T, Lifetime> {
    /// Concrete shared or boxed prefix at offset zero.
    control: Lifetime,
    /// Uninitialized during final deallocation after the value destructor runs.
    value: MaybeUninit<T>,
}

/// Allocation layout for a shared provider.
#[doc(hidden)]
pub type __ComSharedInner<T> = ComObjectInner<T, NcSharedLifetimeState>;

/// Allocation layout for a unique boxed provider.
#[doc(hidden)]
pub type __ComBoxedInner<T> = ComObjectInner<T, NcBoxedLifetimeState>;

impl<T: ComObject, Lifetime: 'static> ComObjectInner<T, Lifetime> {
    /// Exact callbacks and tables for this particular storage instantiation.
    const CLASS: NcObjectTypeInfo = NcObjectTypeInfo {
        destroy_value: Self::destroy_value,
        deallocate: Self::deallocate,
        interfaces: T::VTABLES.as_ptr(),
        interface_count: T::VTABLES.len(),
    };

    /// Allocates the initialized object before publishing an owner.
    fn allocate(value: T, control: Lifetime) -> ComRaw<IUnknown> {
        let storage = Box::into_raw(Box::new(Self { control, value: MaybeUninit::new(value) }));
        ComRaw {
            object: storage.cast(),
            vtable: T::UNKNOWN_VTABLE,
        }
    }

    /// Destroys T while the allocation remains available to provider cleanup.
    unsafe extern "C" fn destroy_value(control: *mut c_void) {
        let storage = control.cast::<Self>();
        // SAFETY: the class descriptor selects this exact storage and all T
        // borrows have ended before the final owner invokes the destructor.
        unsafe { (*storage).value.assume_init_drop() };
    }

    /// Frees the complete allocation using the provider's exact layout.
    unsafe extern "C" fn deallocate(control: *mut c_void) {
        // SAFETY: the allocation is returned once. MaybeUninit suppresses a
        // second T destructor after the value was destroyed.
        unsafe { drop(Box::from_raw(control.cast::<Self>())) };
    }

    /// Recovers the shared Rust value from an admitted receiver.
    ///
    /// # Safety
    /// A nonnull object identifies this exact live storage. The caller retains
    /// its lender and supplies shared target access for the returned lifetime.
    pub unsafe fn borrow_ref<'a>(object: *mut c_void) -> Result<&'a T, NcStatus> {
        let storage = NonNull::new(object.cast::<Self>()).ok_or(NC_E_POINTER)?;
        // SAFETY: the caller's live shared loan covers the initialized value.
        // Project the field before constructing a reference.
        Ok(unsafe { (*storage.as_ptr()).value.assume_init_ref() })
    }
}

impl<T: ComObject> __ComSharedInner<T> {
    /// Creates and transfers the first strong owner of initialized shared storage.
    ///
    /// # Safety
    /// T's tables and class proof must match this exact storage and threading.
    pub unsafe fn into_shared(value: T) -> ComPtr<IUnknown, T::Threading> {
        let raw = Self::allocate(value, NcSharedLifetimeState::new(&Self::CLASS));
        // SAFETY: freshly constructed storage owns one reference and exact table.
        unsafe { ComPtr::from_raw_owned(raw) }.expect("new shared object is present")
    }
}

impl<T: ComObject> __ComBoxedInner<T> {
    /// Creates and transfers the sole owner of initialized boxed storage.
    ///
    /// # Safety
    /// T's tables and class proof must match this exact storage and threading.
    pub unsafe fn into_exclusive(value: T) -> ComBox<IUnknown, T::Threading> {
        let raw = Self::allocate(value, NcBoxedLifetimeState::new(&Self::CLASS));
        // SAFETY: freshly constructed storage owns the sole token and exact table.
        unsafe { ComBox::from_raw_owned(raw) }.expect("new exclusive object is present")
    }

    /// Recovers the exclusive Rust value from an admitted receiver.
    ///
    /// # Safety
    /// A nonnull object identifies this exact live boxed storage. The caller
    /// supplies exclusive target access and retains the lender for the lifetime.
    pub unsafe fn borrow_mut<'a>(object: *mut c_void) -> Result<&'a mut T, NcStatus> {
        let storage = NonNull::new(object.cast::<Self>()).ok_or(NC_E_POINTER)?;
        // SAFETY: the exclusive loan authorizes a reference to this value slot.
        // The metadata prefix is independent of the value borrow.
        Ok(unsafe { (*storage.as_ptr()).value.assume_init_mut() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    /// Both typed policies put metadata first and preserve provider alignment.
    #[test]
    fn layouts_preserve_policy_prefix_and_value_alignment() {
        #[repr(align(64))]
        struct Aligned;

        assert_eq!(offset_of!(__ComSharedInner<Aligned>, control), 0);
        assert_eq!(offset_of!(__ComBoxedInner<Aligned>, control), 0);
        assert_eq!(size_of::<NcSharedLifetimeState>(), 3 * size_of::<usize>());
        assert_eq!(size_of::<NcBoxedLifetimeState>(), size_of::<usize>());
        assert_eq!(offset_of!(__ComSharedInner<Aligned>, value), 64);
        assert_eq!(offset_of!(__ComBoxedInner<Aligned>, value), 64);
    }

    /// Local and projected loans use the same live value slot through cleanup.
    #[cfg(feature = "std")]
    #[test]
    fn boxed_target_reborrows_and_cleanup_share_one_allocation() {
        use crate::lifetime::EXCLUSIVE_UNKNOWN_VTABLE;
        use crate::{ComExclusiveObject, IUnknownVTable, LocalBox, ThreadSend};
        use core::{
            cell::Cell,
            sync::atomic::{AtomicUsize, Ordering},
        };
        use std::sync::Arc;

        struct Value {
            count: Cell<u32>,
            drops: Arc<AtomicUsize>,
        }

        impl Drop for Value {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::Relaxed);
            }
        }

        // SAFETY: this fixture publishes only the canonical boxed root. Cell
        // makes the target Send only, and every reborrow comes from the owner.
        unsafe impl ComObject for Value {
            type Threading = ThreadSend;
            const UNKNOWN_VTABLE: &'static IUnknownVTable = &EXCLUSIVE_UNKNOWN_VTABLE;
            const VTABLES: &'static [*const IUnknownVTable] = &[Self::UNKNOWN_VTABLE];

            unsafe fn borrow_abi_ref<'a>(object: *mut c_void) -> Result<&'a Self, NcStatus> {
                unsafe { __ComBoxedInner::<Self>::borrow_ref(object) }
            }
        }

        // SAFETY: the fixture uses the matching boxed layout and one unique owner.
        unsafe impl ComExclusiveObject for Value {
            unsafe fn borrow_abi_mut<'a>(object: *mut c_void) -> Result<&'a mut Self, NcStatus> {
                unsafe { __ComBoxedInner::<Self>::borrow_mut(object) }
            }
        }

        let drops = Arc::new(AtomicUsize::new(0));
        let value = Value {
            count: Cell::new(1),
            drops: drops.clone(),
        };
        // SAFETY: these are the exact layout and typed owner implemented above.
        let mut owner = unsafe { LocalBox::<Value>::__from_owned(__ComBoxedInner::into_exclusive(value)) };
        *owner.get_mut().count.get_mut() = 2;
        let mut projection = owner.query_mut::<IUnknown>().unwrap();
        std::thread::scope(|scope| {
            let mut reborrow = projection.as_com_mut();
            scope.spawn(move || {
                // SAFETY: the scoped reborrow uniquely lends this exact boxed value.
                let value = unsafe { Value::borrow_abi_mut(reborrow.as_com_mut().as_raw().object) }.unwrap();
                *value.count.get_mut() += 1;
            });
        });
        assert_eq!(owner.get().count.get(), 3);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        let erased = owner.into_interface::<IUnknown>().unwrap();
        std::thread::scope(|scope| scope.spawn(move || drop(erased)).join().unwrap());
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }
}
