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
use crate::lifetime::{BoxCall, box_projection, nc_box_release, nc_weak_retain, nc_weak_release};

/// The sole transferable owner of an object exposing typed C ABI interfaces.
///
/// There is no `Clone`, shared-owner conversion, or owning weak upgrade. Selecting
/// a different interface consumes this owner and preserves the allocation.
/// Dropping it rejects new weak calls immediately; an already executing call
/// finishes before the allocating provider destroys the value. Weak tokens keep
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

impl<T: ComInterface + ?Sized, M: ThreadingModel> ComBox<T, M> {
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
        let owner = unsafe { Self::from_raw_owned(raw) }.ok_or(NC_E_POINTER)?;
        unsafe { crate::raw::validate_interface(raw) }?;
        unsafe { box_projection(raw.object.cast(), &T::INTERFACE_ID) }?;
        Ok(owner)
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
    /// Failure drops the consumed owner, matching other consuming conversions.
    pub fn query<U: ComInterface>(self) -> Result<ComBox<U, M>, NcStatus> {
        let table = unsafe { box_projection(self.as_raw().object.cast(), &U::INTERFACE_ID) }?;
        let raw = self.into_raw();
        // SAFETY: lookup validated U and the unchanged allocation transfers its owner.
        Ok(unsafe { ComBox::from_raw_owned(ComRaw { object: raw.object, vtable: table.cast() }) }.expect("present owner"))
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
        let table = unsafe { box_projection(self.as_raw().object.cast(), &U::INTERFACE_ID) }?;
        unsafe { nc_weak_retain(self.as_raw().object.cast()) };
        Ok(unsafe {
            WeakComBox::from_raw_owned(ComRaw {
                object: self.as_raw().object,
                vtable: table.cast(),
            })
        }
        .expect("present token"))
    }

    /// Acquires bounded execution access for passing a live borrowed ABI argument.
    /// The returned guard has no ownership conversion and must not escape its
    /// lexical use. Ordinary generated method calls need no extra guard here.
    pub fn borrow(&self) -> Result<BoxBorrow<'_, T, M>, NcStatus> {
        let call = unsafe { BoxCall::enter(self.as_raw().object.cast()) }?;
        Ok(BoxBorrow {
            raw: self.raw,
            _call: call,
            lifetime: PhantomData,
        })
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

/// Bounded lifetime protection for a borrowed argument from a weak box.
/// This is an execution lease, not transferable or clonable ownership. It exists
/// only when another ABI method needs a live origin for the duration of a call.
pub struct BoxBorrow<'a, T: ComInterface + ?Sized, M: ThreadingModel = ThreadLocal> {
    /// Validated pair whose value is protected by the active call below.
    raw: LiveInterface<T>,
    /// Ends lifetime protection after borrowed interface views have expired.
    _call: BoxCall,
    /// Prevents this lease from escaping its weak handle and threading mode.
    lifetime: PhantomData<&'a WeakComBox<T, M>>,
}
impl<T: ComInterface + ?Sized, M: ThreadingModel> BoxBorrow<'_, T, M> {
    /// Borrows the protected value for no longer than this execution lease.
    pub fn as_com_ref(&self) -> ComRef<'_, T, M> {
        unsafe { ComRef::from_raw_borrowed(self.raw.as_raw()) }.expect("present execution lease")
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
