//! Borrowed Rust method views for statically declared COM interface inheritance.
//!
//! Views lend methods, never COM ownership. In particular, an inherited view's
//! table may still advertise the derived identity; obtaining an owned base
//! projection continues to require checked QueryInterface.

use core::marker::PhantomData;
use crate::{ComInterface, ThreadLocal, ThreadSafe, ThreadingModel};

/// Optional static method-view support generated for a COM interface.
///
/// Ordinary foreign interface adapters need only implement ComInterface. This
/// additional unsafe trait enables Deref-based method lookup without changing
/// the representation or ownership of a COM pointer.
///
/// # Safety
///
/// `Client<M>` must be an unsized `repr(transparent)` wrapper around
/// `NcClientPointer<Self, M>`. Its safe operations may only borrow the live
/// interface and may not manufacture an owning reference or an escaping
/// ComRef. Inherited views must preserve the exact base vtable prefix and the
/// supplied threading capability. Both hooks must lend exactly their input
/// storage, preserving its lifetime, zero-length metadata and threading marker.
/// The opaque unsized field prevents safe replacement of the underlying pair,
/// including generic `mem::swap` from the generated type's own module.
pub unsafe trait ComClientInterface: ComInterface + Sized {
    /// Borrowed method surface whose reference lifetime is tied to its pointer owner.
    type Client<M: ThreadingModel>: ?Sized;

    /// Reconstructs the concrete shared method facade from core-owned opaque storage.
    #[doc(hidden)]
    fn __nc_client_ref<M: ThreadingModel>(pointer: &NcClientPointer<Self, M>) -> &Self::Client<M>;

    /// Reconstructs the concrete mutable facade without exposing replaceable pair storage.
    #[doc(hidden)]
    fn __nc_client_mut<M: ThreadingModel>(pointer: &mut NcClientPointer<Self, M>) -> &mut Self::Client<M>;
}

/// Pointer storage for generated, non-owning method views.
///
/// This type has no public constructor, Clone, Copy, or ownership conversion.
/// A view is tied to a live owner/borrow or a retained exclusive weak token.
/// Exclusive providers must acquire lifetime protection at every ABI entry before
/// accessing their value; expiration returns NC_E_DROPPED. Its raw pair may
/// address an inherited prefix whose actual projected identity remains derived;
/// such a pointer must not be adopted as a base COM reference by a cast.
/// Its unsized tail always has length zero: the dynamic extent is exactly the
/// pair's two words. References are fat, while all stored handles remain two words.
#[repr(C)]
pub struct NcClientPointer<T: ComInterface, M: ThreadingModel = ThreadLocal> {
    /// Same live object/table pair stored by the lending COM owner or borrow.
    pointer: crate::ComRaw<T>,
    /// Preserves the handle's thread-affinity restriction in the borrowed view.
    mode: PhantomData<M>,
    /// Makes the entire opaque storage immovable through a safe mutable reference.
    tail: [()],
}

impl<T: ComInterface, M: ThreadingModel> NcClientPointer<T, M> {
    /// Lends a zero-tail facade over the enclosing handle's proven-present pair.
    ///
    /// # Safety
    /// The pair's storage/table must remain valid for the borrow. It must permit
    /// T's calls under M; an expired exclusive weak view must still have live
    /// control storage and thunks which reject access before touching its value.
    pub(crate) unsafe fn borrow_live(pair: &crate::ptr::LiveInterface<T>) -> &Self {
        let pointer = core::ptr::slice_from_raw_parts(core::ptr::from_ref(pair).cast::<()>(), 0);
        // SAFETY: LiveInterface and ComRaw are repr(C) pairs of matching thin
        // pointer fields. NonNull has the pointer's layout/validity; both source
        // fields are present. The trailing [()] has alignment one and zero
        // extent, so zero metadata lends exactly the source's aligned extent.
        unsafe { &*(pointer as *const Self) }
    }

    /// Lends mutable method lookup without granting mutable access to the pair.
    ///
    /// # Safety
    /// The same obligations as borrow_live apply. The exclusive source borrow
    /// must cover this entire facade borrow; it does not prove object uniqueness.
    pub(crate) unsafe fn borrow_live_mut(pair: &mut crate::ptr::LiveInterface<T>) -> &mut Self {
        let pointer = core::ptr::slice_from_raw_parts_mut(core::ptr::from_mut(pair).cast::<()>(), 0);
        // SAFETY: the same layout/extent proof as borrow_live, with an exclusive
        // source reborrow. No safe method can move or mutate the private pair.
        unsafe { &mut *(pointer as *mut Self) }
    }

    /// Lends a physical base prefix while retaining the actual selected pair.
    ///
    /// # Safety
    /// U's complete vtable must be a physical prefix of this table. This does
    /// not certify U's separately queried identity or transfer any ownership.
    #[doc(hidden)]
    pub unsafe fn prefix<U: ComInterface>(&self) -> &NcClientPointer<U, M> {
        // SAFETY: all opaque facades have the same pair layout and zero tail;
        // only the table's static pointee type changes under the caller's proof.
        unsafe { &*(core::ptr::from_ref(self) as *const NcClientPointer<U, M>) }
    }

    /// Lends mutable base lookup within this facade's exclusive source borrow.
    ///
    /// # Safety
    /// U must satisfy prefix's physical table proof. M and the source borrow
    /// lifetime are preserved; provider call gates still arbitrate object access.
    #[doc(hidden)]
    pub unsafe fn prefix_mut<U: ComInterface>(&mut self) -> &mut NcClientPointer<U, M> {
        // SAFETY: same zero-tail prefix proof, preserving the exclusive reborrow.
        unsafe { &mut *(core::ptr::from_mut(self) as *mut NcClientPointer<U, M>) }
    }

    /// Returns the raw object/table pair used by generated synchronous calls.
    ///
    /// This does not transfer ownership or certify the prefix as a separately
    /// projected base interface. Raw ABI use remains the caller's unsafe obligation.
    #[doc(hidden)]
    pub fn as_raw(&self) -> crate::ComRaw<T> {
        self.pointer
    }
}

// SAFETY: A ThreadSafe view can only be lent by a handle whose construction
// guarantees concurrent method calls are supported. The view has no ownership.
unsafe impl<T: ComInterface> Send for NcClientPointer<T, ThreadSafe> {}
// SAFETY: Sharing the view grants the same synchronous call permission as its
// ThreadSafe owner; it does not extend the owner's lifetime.
unsafe impl<T: ComInterface> Sync for NcClientPointer<T, ThreadSafe> {}
