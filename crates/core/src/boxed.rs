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
use crate::lifetime::{try_box_cast, NcBoxedLifetimeState};

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
/// lifetime before reading the value. Expired handles return `NC_E_DROPPED`.
/// Clone/drop affect only weak bookkeeping; they never prolong the object's value lifetime.
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
unsafe fn observe_projection<T: ComInterface, M: ThreadingModel>(control: *mut crate::lifetime::NcBoxedLifetimeState) -> Result<WeakComBox<T, M>, NcStatus> {
    let table = unsafe { try_box_cast(control, &T::INTERFACE_ID) }?;
    // Validation precedes retention, so failure creates no token to release.
    unsafe { (*control).weak_retain() };
    Ok(unsafe {
        WeakComBox::from_raw_owned(ComRaw {
            object: control.cast(),
            vtable: table.cast(),
        })
    }
    .expect("retained exclusive storage is present"))
}

impl<T: ComInterface + ?Sized, M: ThreadingModel> ComBox<T, M> {
    /// Checks an incoming owner slot without acquiring or destroying its token.
    /// Generated thunks validate every argument and the receiver before adopting
    /// owners. A rejected call therefore leaves all caller slots unchanged.
    ///
    /// # Safety
    /// `slot` is writable, properly aligned call-scoped storage containing the
    /// sole boxed owner with the provider's threading guarantee M and a matching
    /// readable lifecycle table/header. Rejection leaves that owner untouched.
    /// Shared backing violates this ABI input's ownership contract.
    #[doc(hidden)]
    pub unsafe fn __nc_validate_raw_input(slot: *mut ComRaw<T>) -> Result<(), NcStatus> {
        let raw = unsafe { slot.as_ref() }.ok_or(NC_E_POINTER)?;
        unsafe { crate::raw::validate_interface(*raw) }?;
        unsafe { try_box_cast(raw.object.cast(), &T::INTERFACE_ID) }?;
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
        let checked = unsafe { crate::raw::validate_interface(raw) }.and_then(|()| unsafe { try_box_cast(raw.object.cast(), &T::INTERFACE_ID) }.map(|_| ()));
        if let Err(error) = checked {
            unsafe { NcBoxedLifetimeState::release(raw.object.cast()) };
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
        unsafe { &*self.as_raw().object.cast::<crate::lifetime::NcBoxedLifetimeState>() }.box_is_idle()
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
    /// Selecting the same exact interface reuses this owner's validated table.
    pub fn query<U: ComInterface>(self) -> Result<ComBox<U, M>, (NcStatus, Self)> {
        // This owner already proves its exact interface contract. Resolve the
        // identity comparison during compilation: relying on LTO to fold ordinary
        // PartialEq kept per-type comparisons in size-optimized provider code.
        // A different revision must still take checked discovery, even for the
        // same IID. Both identities come from static unsafe ComInterface proofs.
        let identical = const {
            let left = T::INTERFACE_ID;
            let right = U::INTERFACE_ID;
            left.iid.data1 == right.iid.data1
                && left.iid.data2 == right.iid.data2
                && left.iid.data3 == right.iid.data3
                && u64::from_ne_bytes(left.iid.data4) == u64::from_ne_bytes(right.iid.data4)
                && left.abi_signature.__const_eq(right.abi_signature)
        };
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

    /// Borrows another exact interface without retaining weak bookkeeping.
    ///
    /// The view cannot outlive this sole owner. Methods use the same provider
    /// admission as every other exclusive projection; no value reference or
    /// execution lease is created by the query itself. Threading permission is
    /// preserved. Missing contracts and revision mismatches remain distinct errors.
    pub fn query_ref<U: ComInterface>(&self) -> Result<ComRef<'_, U, M>, NcStatus> {
        let object = self.as_raw().object;
        // SAFETY: this owner retains common exclusive storage and immutable class
        // tables. Lookup checks U's exact identity without changing ownership.
        let vtable = unsafe { try_box_cast(object.cast(), &U::INTERFACE_ID) }?;
        // SAFETY: lookup certifies the table, and borrowing self retains the
        // matching storage and initialized value for the returned Rust lifetime.
        unsafe { ComRef::from_raw_borrowed(ComRaw { object, vtable: vtable.cast() }) }.ok_or(NC_E_POINTER)
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
        unsafe { (*raw.object.cast::<NcBoxedLifetimeState>()).weak_retain() };
        unsafe { WeakComBox::from_raw_owned(raw) }.expect("present owner")
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Drop for ComBox<T, M> {
    /// Revokes new access and destroys the value once admitted calls have returned.
    fn drop(&mut self) {
        unsafe { NcBoxedLifetimeState::release(self.raw.as_raw().object.cast()) };
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
        unsafe { &*self.as_raw().object.cast::<crate::lifetime::NcBoxedLifetimeState>() }.box_is_alive()
    }

    /// Checks discovered catalog metadata without acquiring a shared owner.
    pub fn supports_interface_bytes(&self, iid: crate::NcGuid, signature: &[u8; 32]) -> Result<(), NcStatus> {
        let expected = crate::NcInterfaceId::new(iid, crate::NcAbiDigest::from_borrowed(signature));
        // SAFETY: this weak token retains the immutable class table even after
        // the value expires; projection validates the complete interface digest.
        unsafe { try_box_cast(self.as_raw().object.cast(), &expected) }.map(|_| ())
    }

    /// Selects canonical IUnknown while transferring this weak token unchanged.
    /// Works before and after value destruction; never acquires another weak or
    /// strong reference, so erasure also succeeds at the weak counter limit.
    /// Stable object identity and threading permission are preserved.
    pub fn into_unknown(self) -> WeakComBox<crate::IUnknown, M> {
        // SAFETY: self retains boxed storage and immutable provider metadata,
        // including after value destruction. Resolve the exact root table before
        // consuming self, so a violated provider invariant still drops its token.
        let table = unsafe { try_box_cast(self.as_raw().object.cast(), &crate::IUnknown::INTERFACE_ID) }.expect("every boxed COM object declares IUnknown");
        let raw = self.into_raw();
        // SAFETY: move the existing weak token to the exact canonical root table;
        // its allocation, retained code and threading permission are unchanged.
        unsafe { WeakComBox::from_raw_owned(ComRaw { object: raw.object, vtable: table.cast() }) }.expect("existing observer is present")
    }

    /// Restricts cross-thread permission while preserving this observer's projection.
    pub fn into_thread_local(self) -> WeakComBox<T, ThreadLocal> {
        let raw = core::mem::ManuallyDrop::new(self).raw.as_raw();
        // SAFETY: only the threading permission is restricted; ownership is transferred.
        unsafe { WeakComBox::from_raw_owned(raw) }.expect("existing observer is present")
    }

    /// Lends interface storage for a synchronous ABI argument, without pinning T.
    /// The recipient's generated methods acquire their own checked borrows and
    /// return `NC_E_DROPPED` after owner destruction. An admitted live method can
    /// return its own error, including `NC_E_CLOSED`. Holding this view cannot
    /// delay owner destruction.
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
        unsafe { (*self.as_raw().object.cast::<NcBoxedLifetimeState>()).weak_retain() };
        Self { raw: self.raw, mode: PhantomData }
    }
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> Drop for WeakComBox<T, M> {
    /// Frees expired storage through its original provider at final weak release.
    fn drop(&mut self) {
        unsafe { NcBoxedLifetimeState::weak_release(self.as_raw().object.cast()) };
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
                // SAFETY: the owner or retained weak token keeps control/table
                // storage live; provider entry checks exclusive value lifetime.
                T::__nc_client_ref(unsafe { crate::NcClientPointer::borrow_live(&self.raw) })
            }
        }
        impl<T: ComClientInterface, M: ThreadingModel> DerefMut for $handle<T, M> {
            /// Mutable lookup still uses the provider's common exclusive borrow gate.
            fn deref_mut(&mut self) -> &mut Self::Target {
                // SAFETY: the source borrow bounds this nonreplaceable view;
                // it grants no stronger token or object access than the handle.
                T::__nc_client_mut(unsafe { crate::NcClientPointer::borrow_live_mut(&mut self.raw) })
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
    /// A present pair transfers one weak reference to genuine boxed storage with
    /// matching metadata, provider table/code and threading M, even after value
    /// destruction. The ABI producer guarantees boxed backing. Rejection consumes
    /// that boxed weak reference; shared backing violates the producer contract.
    /// Both-null and half-null shapes transfer nothing and reject without reading
    /// either component. Only a present pair carries a cleanup obligation.
    #[doc(hidden)]
    pub unsafe fn __nc_from_raw_owned_checked(raw: ComRaw<T>) -> Result<Self, NcStatus> {
        if !raw.is_present() {
            return Err(NC_E_POINTER);
        }
        // The supplied table and canonical allocation metadata are independent
        // authorities. Neither check may publish an unadmitted typed owner.
        let checked = unsafe { crate::raw::validate_interface(raw) }.and_then(|()| unsafe { try_box_cast(raw.object.cast(), &T::INTERFACE_ID) });
        let table = match checked {
            Ok(table) => table,
            Err(status) => {
                // SAFETY: the producer transfers a boxed weak reference. Identity
                // rejection consumes it directly, without a premature typed owner.
                unsafe { NcBoxedLifetimeState::weak_release(raw.object.cast()) };
                return Err(status);
            }
        };
        // SAFETY: both identity authorities are validated; the producer proves boxed backing.
        unsafe { Self::from_raw_owned(ComRaw { object: raw.object, vtable: table.cast() }) }.ok_or(NC_E_POINTER)
    }
}
