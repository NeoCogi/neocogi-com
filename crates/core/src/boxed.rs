//! Exclusive COM ownership and borrowing of its live target.

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::lifetime::{try_box_cast, NcBoxedLifetimeState};
use crate::ptr::NonNullComRaw;
use crate::{
    CanWeakenThreading, ComInterface, ComMut, ComRaw, ComRef, ComRustInterface, NcStatus, ThreadLocal, ThreadSafe, ThreadSend, ThreadSync, ThreadingModel,
    NC_E_POINTER,
};

/// The sole transferable owner of a live COM value.
///
/// Shared and mutable views borrow the target for the corresponding owner
/// borrow. Consuming interface selection moves this single owner. Drop destroys
/// the value and frees storage synchronously in the allocating provider.
///
/// ```compile_fail
/// use neocogi_com::{ComBox, IUnknown};
/// fn duplicate(owner: ComBox<IUnknown>) { let second = owner.clone(); }
/// ```
#[repr(transparent)]
pub struct ComBox<T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    /// Present object/table pair carrying the allocation's unique ownership token.
    raw: NonNullComRaw<T>,
    /// Preserves the provider's thread capability after type erasure.
    mode: PhantomData<M>,
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> ComBox<T, M> {
    /// Resolves an incoming owner's canonical table without changing its slot/token.
    /// Generated thunks save this table and validate every argument before
    /// adopting the owner with canonical dispatch. Rejected calls
    /// restore the original object/table pair, including an equivalent copied table.
    ///
    /// # Safety
    /// `slot` is writable, properly aligned call-scoped storage containing the
    /// sole boxed owner with the provider's threading guarantee M and a matching
    /// readable lifecycle table/header. The caller has ended its loans of the
    /// transferred value and its dependents. Rejection leaves that owner untouched.
    /// Shared backing violates this ABI input's ownership contract.
    #[doc(hidden)]
    pub unsafe fn __nc_validate_raw_input(slot: *mut ComRaw<T>) -> Result<*const T::VTable, NcStatus> {
        let raw = unsafe { slot.as_ref() }.ok_or(NC_E_POINTER)?;
        unsafe { crate::raw::validate_interface(*raw) }?;
        unsafe { try_box_cast(raw.object.cast(), &T::INTERFACE_ID) }.map(|table| table.cast())
    }

    /// Adopts the sole owner transferred by an exclusive provider, preserving its table.
    /// This unchecked transfer trusts the supplied pairing; checked ABI adoption
    /// independently validates identities and selects canonical dispatch.
    ///
    /// # Safety
    /// The present raw pair transfers one unique initialized owner, with common
    /// boxed storage, exact T identity, and mode M. The value has no outstanding
    /// access loans, and its provider stays loaded until its owner is released.
    /// Either null word makes the pair absent.
    pub unsafe fn from_raw_owned(raw: ComRaw<T>) -> Option<Self> {
        Some(Self {
            raw: NonNullComRaw::new(raw)?,
            mode: PhantomData,
        })
    }

    /// Checks a transferred factory result and adopts its canonical interface table.
    /// Validates the supplied identity independently before selecting the table;
    /// copied equivalent tables are accepted without retaining another reference.
    ///
    /// # Safety
    /// A present pair transfers the sole genuine boxed owner and its matching
    /// readable lifecycle table. Rejection consumes that boxed owner. The ABI
    /// producer guarantees boxed backing before identity validation.
    /// The provider establishes its threading promise; readable metadata alone
    /// cannot prove that foreign code obeys ownership or call-access obligations.
    #[doc(hidden)]
    pub unsafe fn __nc_from_raw_owned_checked(raw: ComRaw<T>) -> Result<Self, NcStatus> {
        if !raw.is_present() {
            return Err(NC_E_POINTER);
        }
        // The producer guarantees boxed backing; identity rejection consumes
        // that known boxed owner directly, without selecting a release policy.
        let checked = unsafe { crate::raw::validate_interface(raw) }.and_then(|()| unsafe { try_box_cast(raw.object.cast(), &T::INTERFACE_ID) });
        let table = match checked {
            Ok(table) => table,
            Err(error) => {
                unsafe { NcBoxedLifetimeState::release(raw.object.cast()) };
                return Err(error);
            }
        };
        // SAFETY: both identity authorities agree. Move the sole owner into the
        // canonical view; do not cache the equivalent incoming table or retain.
        unsafe { Self::from_raw_owned(ComRaw { object: raw.object, vtable: table.cast() }) }.ok_or(NC_E_POINTER)
    }

    /// Transfers ownership without destruction; the recipient must adopt it once.
    pub fn into_raw(self) -> ComRaw<T> {
        core::mem::ManuallyDrop::new(self).raw.as_raw()
    }

    /// Transfers the sole owner with weaker threading evidence.
    pub fn into_threading<N: ThreadingModel>(self) -> ComBox<T, N>
    where
        M: CanWeakenThreading<N>,
    {
        ComBox {
            raw: core::mem::ManuallyDrop::new(self).raw,
            mode: PhantomData,
        }
    }

    /// Borrows the raw C record without transferring or duplicating ownership.
    pub fn as_raw(&self) -> ComRaw<T> {
        self.raw.as_raw()
    }

    /// Lends shared target access while this owner keeps the value live.
    pub fn as_com_ref(&self) -> ComRef<'_, T, M> {
        // SAFETY: this owner keeps the initialized value alive throughout the borrow.
        unsafe { ComRef::from_raw_borrowed(self.as_raw()) }.expect("present exclusive owner")
    }

    /// Lends exclusive target access while this owner keeps the value live.
    pub fn as_com_mut(&mut self) -> ComMut<'_, T, M> {
        // SAFETY: The exclusive owner reborrow grants unique access to the live target.
        unsafe { ComMut::from_raw_borrowed(self.as_raw()) }.expect("present exclusive owner")
    }

    /// Selects an interface by moving, rather than duplicating, the sole owner.
    /// Failure returns the unchanged owner. Interface selection is not authority
    /// to destroy an object that the caller may still need (including an ancestor
    /// of an active receiver); the caller decides whether to retry or drop it.
    /// Selecting the same exact interface reuses this owner's validated table.
    pub fn query<U: ComInterface>(self) -> Result<ComBox<U, M>, (NcStatus, Self)> {
        // This owner already proves its exact interface contract. Resolve the
        // identity comparison during compilation: relying on LTO to fold ordinary
        // PartialEq kept per-type comparisons in size-optimized provider code.
        // A different revision must still take checked discovery, even for the
        // same IID. Both identities come from static unsafe ComInterface proofs.
        let identical = const { T::INTERFACE_ID.const_eq(U::INTERFACE_ID) };
        let table = match identical {
            true => self.as_raw().vtable.cast(),
            false => match unsafe { try_box_cast(self.as_raw().object.cast(), &U::INTERFACE_ID) } {
                Ok(table) => table,
                Err(status) => return Err((status, self)),
            },
        };
        let raw = self.into_raw();
        // SAFETY: exact identity or checked lookup establishes U. The unchanged
        // allocation transfers its sole owner; no retention or borrowing is added.
        Ok(unsafe { ComBox::from_raw_owned(ComRaw { object: raw.object, vtable: table.cast() }) }.expect("present owner"))
    }

    /// Lends shared target access through another exact interface.
    ///
    /// The view inherits the owner borrow and its threading permission. Missing
    /// contracts and revision mismatches remain distinct errors.
    pub fn query_ref<U: ComInterface>(&self) -> Result<ComRef<'_, U, M>, NcStatus> {
        let object = self.as_raw().object;
        // SAFETY: this owner retains common exclusive storage and immutable class
        // tables. Lookup checks U's exact identity without changing ownership.
        let vtable = unsafe { try_box_cast(object.cast(), &U::INTERFACE_ID) }?;
        // SAFETY: lookup certifies the table, and borrowing self retains the
        // matching storage and initialized value for the returned Rust lifetime.
        unsafe { ComRef::from_raw_borrowed(ComRaw { object, vtable: vtable.cast() }) }.ok_or(NC_E_POINTER)
    }

    /// Lends exclusive target access through another exact interface.
    pub fn query_mut<U: ComInterface>(&mut self) -> Result<ComMut<'_, U, M>, NcStatus> {
        let object = self.as_raw().object;
        // SAFETY: the exclusive owner borrow retains boxed storage and authorizes
        // mutable access. Lookup selects U's exact table without transferring it.
        let vtable = unsafe { try_box_cast(object.cast(), &U::INTERFACE_ID) }?;
        unsafe { ComMut::from_raw_borrowed(ComRaw { object, vtable: vtable.cast() }) }.ok_or(NC_E_POINTER)
    }
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> Drop for ComBox<T, M> {
    fn drop(&mut self) {
        // SAFETY: Drop consumes the sole owner after its loans have ended.
        unsafe { NcBoxedLifetimeState::release(self.raw.as_raw().object.cast()) };
    }
}

// SAFETY: The producer's transfer capability covers both use and destruction.
unsafe impl<T: ComInterface + ?Sized> Send for ComBox<T, ThreadSend> {}
unsafe impl<T: ComInterface + ?Sized> Send for ComBox<T, ThreadSafe> {}
// SAFETY: Sharing the owner by reference permits only shared target access.
unsafe impl<T: ComInterface + ?Sized> Sync for ComBox<T, ThreadSync> {}
unsafe impl<T: ComInterface + ?Sized> Sync for ComBox<T, ThreadSafe> {}

impl<T: ComRustInterface, M: ThreadingModel> Deref for ComBox<T, M> {
    type Target = T::Wrapper<M>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the owner borrow retains the target and grants shared access.
        T::__nc_wrapper_ref(unsafe { crate::ComInterfacePointer::borrow_pair(&self.raw) })
    }
}

impl<T: ComRustInterface, M: ThreadingModel> DerefMut for ComBox<T, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: the unique owner borrow grants exclusive access to the target.
        T::__nc_wrapper_mut(unsafe { crate::ComInterfacePointer::borrow_pair_mut(&mut self.raw) })
    }
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> core::fmt::Debug for ComBox<T, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ComBox").field(&self.as_raw().object).finish()
    }
}
