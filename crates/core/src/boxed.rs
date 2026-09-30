//! Exclusive COM ownership and cached, non-owning interface access.
//!
//! A `ComBox` has exactly one owner. Container insertion moves that owner;
//! application fields keep `WeakComBox` handles. Weak method dispatch goes
//! directly through its cached table. The provider's generated ABI entry acquires
//! execution protection before touching the value, so owner removal races safely
//! with calls without manufacturing another owner. This protocol requires the
//! common lifetime header and checked entry points; arbitrary legacy COM objects
//! cannot be wrapped as boxes merely because their reference count happens to be one.

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use crate::{ComInterface, ComClientInterface, ComRaw, ComRef, NcStatus, ThreadLocal, ThreadSafe, ThreadingModel, NC_E_POINTER};
use crate::ptr::LiveInterface;
use crate::lifetime::{box_projection, nc_box_release, nc_weak_retain, nc_weak_release};

/// The sole transferable owner of an object exposing typed C ABI interfaces.
///
/// There is no `Clone`, shared-owner conversion, or owning weak upgrade. Selecting
/// a different interface consumes this owner and preserves the allocation.
/// Drop rejects new calls, waits for existing calls to finish, and destroys the
/// value and its owned fields before returning. It must not run recursively from
/// this object's active method or while holding resources an active call needs. Weak tokens keep
/// only storage and code lifetime, never the initialized value.
///
/// ```compile_fail
/// use neocogi_com_core::{ComBox, IUnknown};
/// fn duplicate(owner: ComBox<IUnknown>) { let second = owner.clone(); }
/// ```
#[repr(transparent)]
pub struct ComBox<T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    /// Present object/table pair carrying the allocation's unique ownership token.
    raw: LiveInterface<T>,
    /// Preserves the provider's thread-affinity guarantee after type erasure.
    mode: PhantomData<M>,
}

/// Cached typed access to an exclusively owned object, without ownership promotion.
///
/// Two pointers avoid interface lookup during ordinary method dispatch: the
/// retained control address is also the canonical object address, and the table
/// was validated when this handle was created. Every provider method must check
/// lifetime before reading the value. Expired handles return `NC_E_CLOSED`.
/// Clone/drop affect only weak bookkeeping; they never prolong widget resources.
///
/// ```compile_fail
/// use neocogi_com_core::{WeakComBox, IUnknown};
/// fn require_send<T: Send>() {}
/// require_send::<WeakComBox<IUnknown>>();
/// ```
#[repr(transparent)]
pub struct WeakComBox<T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    /// One retained weak token paired with the exact cached interface table.
    raw: LiveInterface<T>,
    /// Thread-local weak observers must remain on their originating thread.
    mode: PhantomData<M>,
}

/// Validates one interface and retains exactly the returned observer token.
/// Both owner-side and weak-side queries use this path, avoiding a temporary
/// downgrade/retain/release when an owner needs a different weak projection.
///
/// # Safety
/// `control` is retained common exclusive storage with immutable provider
/// metadata, and its declared threading capability permits M.
unsafe fn observe_projection<T: ComInterface, M: ThreadingModel>(
    control: *mut crate::lifetime::NcWeakControl,
) -> Result<WeakComBox<T, M>, NcStatus> {
    let table = unsafe { box_projection(control, &T::INTERFACE_ID) }?;
    // Validation precedes retention, so failure creates no token to release.
    unsafe { nc_weak_retain(control) };
    Ok(unsafe { WeakComBox::from_raw_owned(ComRaw { object: control.cast(), vtable: table.cast() }) }
        .expect("retained exclusive storage is present"))
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> ComBox<T, M> {
    /// Checks an incoming owner slot without acquiring or destroying its token.
    /// Generated thunks validate every argument and the receiver before adopting
    /// owners. A rejected call therefore leaves all caller slots unchanged.
    ///
    /// # Safety
    /// `slot` is writable, properly aligned call-scoped storage containing the
    /// sole live owner, with the provider's threading guarantee M. Its object
    /// and table must satisfy the common COM header's readable-storage contract.
    #[doc(hidden)]
    pub unsafe fn __nc_validate_raw_input(slot: *mut ComRaw<T>) -> Result<(), NcStatus> {
        let raw = unsafe { slot.as_ref() }.ok_or(NC_E_POINTER)?;
        unsafe { crate::raw::validate_interface(*raw) }?;
        unsafe { box_projection(raw.object.cast(), &T::INTERFACE_ID) }?;
        Ok(())
    }

    /// Adopts the sole owner transferred by an exclusive provider.
    ///
    /// # Safety
    /// The present raw pair transfers one unique initialized owner, with common
    /// boxed storage, checked ABI entry points, exact T identity, and mode M.
    /// Provider code must remain loaded through all weak tokens. Null is absence.
    pub unsafe fn from_raw_owned(raw: ComRaw<T>) -> Option<Self> {
        Some(Self {
            raw: LiveInterface::new(raw)?,
            mode: PhantomData,
        })
    }

    /// Checks a transferred factory result before adopting its exclusive owner.
    ///
    /// # Safety
    /// A present pair transfers exclusive storage under the common lifetime ABI.
    /// The provider establishes its threading promise; readable metadata alone
    /// cannot prove that foreign code obeys ownership or call-access obligations.
    #[doc(hidden)]
    pub unsafe fn __nc_from_raw_owned_checked(raw: ComRaw<T>) -> Result<Self, NcStatus> {
        // The common release slot can balance either lifetime policy when a
        // foreign result has a mismatched interface or ownership mode.
        let checked = unsafe { crate::raw::validate_interface(raw) }
            .and_then(|()| unsafe { box_projection(raw.object.cast(), &T::INTERFACE_ID) }.map(|_| ()));
        if let Err(error) = checked {
            unsafe { crate::raw::release(raw) };
            return Err(error);
        }
        unsafe { Self::from_raw_owned(raw) }.ok_or(NC_E_POINTER)
    }

    /// Reports whether this object has no active implementation borrows.
    /// This snapshot permits thread-local containers to reject recursive removal
    /// before committing it. It is not synchronization for concurrent removal:
    /// Drop itself stops new calls and waits for existing calls on other threads.
    pub fn is_idle(&self) -> bool {
        // SAFETY: the unique owner retains the common allocation header.
        unsafe { &*self.as_raw().object.cast::<crate::lifetime::NcWeakControl>() }.box_is_idle()
    }

    /// Transfers ownership without destruction; the recipient must adopt it once.
    pub fn into_raw(self) -> ComRaw<T> {
        core::mem::ManuallyDrop::new(self).raw.as_raw()
    }

    /// Borrows the raw C record without transferring or duplicating ownership.
    pub fn as_raw(&self) -> ComRaw<T> {
        self.raw.as_raw()
    }

    /// Borrows an interface for a synchronous call. Retaining this borrow as a
    /// shared owner is rejected by the provider's lifetime protocol.
    pub fn as_com_ref(&self) -> ComRef<'_, T, M> {
        // SAFETY: this owner keeps the initialized value alive throughout the borrow.
        unsafe { ComRef::from_raw_borrowed(self.as_raw()) }.expect("present exclusive owner")
    }

    /// Selects an interface by moving, rather than duplicating, the sole owner.
    /// Failure returns the unchanged owner. Interface selection is not authority
    /// to destroy an object that the caller may still need (including an ancestor
    /// of an active receiver); the caller decides whether to retry or drop it.
    pub fn query<U: ComInterface>(self) -> Result<ComBox<U, M>, (NcStatus, Self)> {
        let table = match unsafe { box_projection(self.as_raw().object.cast(), &U::INTERFACE_ID) } {
            Ok(table) => table,
            Err(status) => return Err((status, self)),
        };
        let raw = self.into_raw();
        // SAFETY: lookup validated U and the unchanged allocation transfers its owner.
        Ok(unsafe { ComBox::from_raw_owned(ComRaw { object: raw.object, vtable: table.cast() }) }.expect("present owner"))
    }

    /// Observes another validated interface without consuming this owner or
    /// creating a temporary weak handle for the original projection.
    pub fn query_weak<U: ComInterface>(&self) -> Result<WeakComBox<U, M>, NcStatus> {
        // SAFETY: this owner retains the immutable control and its threading mode.
        unsafe { observe_projection(self.as_raw().object.cast()) }
    }

    /// Creates a non-owning handle with this interface already cached.
    pub fn downgrade(&self) -> WeakComBox<T, M> {
        let raw = self.as_raw();
        // SAFETY: owner keeps the control initialized while a new weak token is retained.
        unsafe { nc_weak_retain(raw.object.cast()) };
        unsafe { WeakComBox::from_raw_owned(raw) }.expect("present owner")
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Drop for ComBox<T, M> {
    /// Revokes new access and destroys the value once admitted calls have returned.
    fn drop(&mut self) {
        unsafe { nc_box_release(self.raw.as_raw().object.cast()) };
    }
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> WeakComBox<T, M> {
    /// Adopts a transferred weak token with its selected table already validated.
    ///
    /// # Safety
    /// The pair owns one weak reference to boxed common storage. All selected
    /// methods must check lifetime before accessing the value; metadata/code must
    /// outlive this token. The provider establishes exact T and threading M.
    pub unsafe fn from_raw_owned(raw: ComRaw<T>) -> Option<Self> {
        Some(Self {
            raw: LiveInterface::new(raw)?,
            mode: PhantomData,
        })
    }

    /// Transfers this weak token without changing its count or accessing the value.
    pub fn into_raw(self) -> ComRaw<T> {
        core::mem::ManuallyDrop::new(self).raw.as_raw()
    }

    /// Borrows identity and cached dispatch metadata; does not prove value liveness.
    pub fn as_raw(&self) -> ComRaw<T> {
        self.raw.as_raw()
    }

    /// Changes the cached projection once, without touching or owning the value.
    /// Expired objects still have immutable interface metadata while weak-owned.
    pub fn query<U: ComInterface>(&self) -> Result<WeakComBox<U, M>, NcStatus> {
        // SAFETY: this weak token retains the immutable control and mode even
        // after owner destruction; observation does not touch the Rust value.
        unsafe { observe_projection(self.as_raw().object.cast()) }
    }

    /// Observes current eligibility only; subsequent calls still acquire their
    /// own execution lease because another thread may remove the owner meanwhile.
    pub fn is_alive(&self) -> bool {
        // SAFETY: this weak handle retains its allocation header independently of T.
        unsafe { &*self.as_raw().object.cast::<crate::lifetime::NcWeakControl>() }.box_is_alive()
    }

    /// Checks discovered catalog metadata without acquiring a shared owner.
    pub fn supports_interface_bytes(&self, iid: crate::NcGuid, signature: &[u8; 32]) -> Result<(), NcStatus> {
        let expected = crate::NcInterfaceId::new(iid, crate::NcAbiSignature::from_borrowed(signature));
        // SAFETY: this weak token retains the immutable class table even after
        // the value expires; projection validates the complete interface digest.
        unsafe { box_projection(self.as_raw().object.cast(), &expected) }.map(|_| ())
    }

    /// Erases only the selected interface while preserving stable object identity.
    pub fn into_unknown(self) -> WeakComBox<crate::IUnknown, M> {
        self.query().expect("every boxed COM object declares IUnknown")
    }

    /// Lends interface storage for a synchronous ABI argument, without pinning T.
    /// The recipient's generated methods acquire their own checked borrows and
    /// may return CLOSED. Holding this view cannot delay owner destruction.
    pub fn as_com_ref(&self) -> ComRef<'_, T, M> {
        // SAFETY: the weak handle retains the object header and immutable table;
        // every exclusive provider entry validates lifetime before accessing T.
        unsafe { ComRef::from_raw_borrowed(self.as_raw()) }.expect("present weak interface")
    }

    /// Compares stable control identities, including after value destruction.
    pub fn same_object<U: ComInterface + ?Sized, N: ThreadingModel>(&self, other: &WeakComBox<U, N>) -> bool {
        self.as_raw().object == other.as_raw().object
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Clone for WeakComBox<T, M> {
    /// Retains only control storage; no value or owner reference is acquired.
    fn clone(&self) -> Self {
        unsafe { nc_weak_retain(self.as_raw().object.cast()) };
        Self { raw: self.raw, mode: PhantomData }
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Drop for WeakComBox<T, M> {
    /// Frees expired storage through its original provider at final weak release.
    fn drop(&mut self) {
        unsafe { nc_weak_release(self.as_raw().object.cast()) };
    }
}

// SAFETY: only explicit provider guarantees permit cross-thread calls and final
// destruction; weak handles preserve the same guarantee after owner removal.
unsafe impl<T: ComInterface + ?Sized> Send for ComBox<T, ThreadSafe> {}
unsafe impl<T: ComInterface + ?Sized> Sync for ComBox<T, ThreadSafe> {}
unsafe impl<T: ComInterface + ?Sized> Send for WeakComBox<T, ThreadSafe> {}
unsafe impl<T: ComInterface + ?Sized> Sync for WeakComBox<T, ThreadSafe> {}

/// Shares the generated client body across unique owners and cached weak views.
/// No method body, vtable, or erased implementation payload is duplicated here.
macro_rules! client_view {
    ($handle:ident) => {
        impl<T: ComClientInterface, M: ThreadingModel> Deref for $handle<T, M> {
            type Target = T::Client<M>;
            /// Lends the transparent method view; checked provider entry protects execution.
            fn deref(&self) -> &Self::Target {
                unsafe { &*core::ptr::from_ref(&self.raw).cast() }
            }
        }
        impl<T: ComClientInterface, M: ThreadingModel> DerefMut for $handle<T, M> {
            /// Mutable lookup still uses the provider's common exclusive borrow gate.
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { &mut *core::ptr::from_mut(&mut self.raw).cast() }
            }
        }
    };
}
client_view!(ComBox);
client_view!(WeakComBox);

impl<T: ComInterface + ?Sized, M: ThreadingModel> core::fmt::Debug for ComBox<T, M> {
    /// Prints stable identity without invoking provider code or reading the value.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ComBox").field(&self.as_raw().object).finish()
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> core::fmt::Debug for WeakComBox<T, M> {
    /// Prints stable identity even when the owning container has disappeared.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("WeakComBox").field(&self.as_raw().object).finish()
    }
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> WeakComBox<T, M> {
    /// Adopts a weak output after exact cached-interface validation.
    ///
    /// # Safety
    /// The returned pair transfers one weak reference to exclusive common storage,
    /// including when the value has expired. The provider establishes threading M.
    #[doc(hidden)]
    pub unsafe fn __nc_from_raw_owned_checked(raw: ComRaw<T>) -> Result<Self, NcStatus> {
        let weak = unsafe { Self::from_raw_owned(raw) }.ok_or(NC_E_POINTER)?;
        unsafe { crate::raw::validate_interface(raw) }?;
        unsafe { box_projection(raw.object.cast(), &T::INTERFACE_ID) }?;
        Ok(weak)
    }
}
