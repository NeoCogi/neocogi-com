//! Borrowed Rust method views for statically declared COM interface inheritance.
//!
//! Views lend methods, never COM ownership. In particular, an inherited view's
//! table may still advertise the derived identity; obtaining an owned base
//! projection continues to require checked QueryInterface.

use core::marker::PhantomData;
use crate::{ComInterface, ThreadLocal, ThreadSafe, ThreadSend, ThreadSync, ThreadingModel};

/// Associates a COM ABI marker with its generated borrowed Rust method wrapper.
///
/// Ordinary foreign interface adapters need only implement ComInterface. This
/// additional unsafe trait enables Deref-based method lookup without changing
/// the representation or ownership of a COM pointer.
///
/// # Safety
///
/// `Wrapper<M>` must be an unsized `repr(transparent)` wrapper around
/// `ComInterfacePointer<Self, M>`. Its safe operations use the retained
/// interface/control/table storage and may not manufacture an owning reference
/// or extend the live target borrow. Inherited wrappers preserve the exact
/// base vtable prefix, target access and threading capability. Both hooks lend exactly their input
/// storage, preserving its lifetime, zero-length metadata and threading marker.
/// The opaque unsized field prevents safe replacement of the underlying pair,
/// including generic `mem::swap` from the generated type's own module.
pub unsafe trait ComRustInterface: ComInterface + Sized {
    /// Generated method wrapper whose lifetime is tied to its lending handle.
    type Wrapper<M: ThreadingModel>: ?Sized;

    /// Reconstructs the concrete shared method facade from core-owned opaque storage.
    #[doc(hidden)]
    fn __nc_wrapper_ref<M: ThreadingModel>(pointer: &ComInterfacePointer<Self, M>) -> &Self::Wrapper<M>;

    /// Reconstructs the concrete mutable facade without exposing replaceable pair storage.
    #[doc(hidden)]
    fn __nc_wrapper_mut<M: ThreadingModel>(pointer: &mut ComInterfacePointer<Self, M>) -> &mut Self::Wrapper<M>;
}

/// Certifies a declaring interface as a physical vtable prefix of this marker.
///
/// The IID selects a witness. The associated marker supplies exact Rust type
/// equality, independently of a native interface's nominal UUID. Generated
/// scoped helpers keep the actual live loan and use only this table prefix.
///
/// # Safety
/// `Base` must be this marker or an ancestor reached solely through first-base
/// edges. Its complete vtable must start at offset zero of this marker's table,
/// with the same object address and compatible call admission. This proof grants
/// no ownership transfer or separately queried base-interface identity.
#[doc(hidden)]
pub unsafe trait __NcScopedPrefix<const IID: u128>: ComInterface {
    /// Exact declaring marker selected through a checked first-base path.
    type Base: ComInterface;
}

/// Opaque borrowed object/table pointer for generated Rust interface methods.
///
/// This type has no public constructor, Clone, Copy, or ownership conversion.
/// A view is tied to the live target borrow supplied by its owner or lender.
/// Its raw pair may
/// address an inherited prefix whose actual projected identity remains derived;
/// such a pointer must not be adopted as a base COM reference by a cast.
/// Its unsized tail always has length zero: the dynamic extent is exactly the
/// pair's two words. References are fat, while all stored handles remain two words.
#[repr(C)]
pub struct ComInterfacePointer<T: ComInterface, M: ThreadingModel = ThreadLocal> {
    /// Object/table pair kept valid by the lending owner or borrow.
    pointer: crate::ComRaw<T>,
    /// Preserves the handle's thread-affinity restriction in the borrowed view.
    mode: PhantomData<M>,
    /// Makes the entire opaque storage immovable through a safe mutable reference.
    tail: [()],
}

impl<T: ComInterface, M: ThreadingModel> ComInterfacePointer<T, M> {
    /// Lends a zero-tail method wrapper over a handle's non-null object/table pair.
    ///
    /// # Safety
    /// The pair's value, storage and table must remain valid for the borrow, and
    /// it must supply shared target access under M.
    pub(crate) unsafe fn borrow_pair(pair: &crate::ptr::NonNullComRaw<T>) -> &Self {
        let pointer = core::ptr::slice_from_raw_parts(core::ptr::from_ref(pair).cast::<()>(), 0);
        // SAFETY: NonNullComRaw and ComRaw are repr(C) pairs of matching thin
        // pointer fields. NonNull has the pointer's layout/validity; both source
        // fields are present. The trailing [()] has alignment one and zero
        // extent, so zero metadata lends exactly the source's aligned extent.
        unsafe { &*(pointer as *const Self) }
    }

    /// Lends mutable method lookup without granting mutable access to the pair.
    ///
    /// # Safety
    /// The same obligations as borrow_pair apply. The lender must also supply
    /// exclusive target access for the entire facade borrow.
    pub(crate) unsafe fn borrow_pair_mut(pair: &mut crate::ptr::NonNullComRaw<T>) -> &mut Self {
        let pointer = core::ptr::slice_from_raw_parts_mut(core::ptr::from_mut(pair).cast::<()>(), 0);
        // SAFETY: the same layout/extent proof as borrow_pair, with an exclusive
        // source reborrow. No safe method can move or mutate the private pair.
        unsafe { &mut *(pointer as *mut Self) }
    }

    /// Lends a physical base prefix while retaining the actual selected pair.
    ///
    /// # Safety
    /// U's complete vtable must be a physical prefix of this table. This does
    /// not certify U's separately queried identity or transfer any ownership.
    #[doc(hidden)]
    pub unsafe fn prefix<U: ComInterface>(&self) -> &ComInterfacePointer<U, M> {
        // SAFETY: all opaque facades have the same pair layout and zero tail;
        // only the table's static pointee type changes under the caller's proof.
        unsafe { &*(core::ptr::from_ref(self) as *const ComInterfacePointer<U, M>) }
    }

    /// Lends mutable base lookup within this facade's exclusive source borrow.
    ///
    /// # Safety
    /// U must satisfy prefix's physical table proof. M and the source borrow
    /// lifetime and exclusive target access are preserved.
    #[doc(hidden)]
    pub unsafe fn prefix_mut<U: ComInterface>(&mut self) -> &mut ComInterfacePointer<U, M> {
        // SAFETY: same zero-tail prefix proof, preserving the exclusive reborrow.
        unsafe { &mut *(core::ptr::from_mut(self) as *mut ComInterfacePointer<U, M>) }
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

// SAFETY: A send-capable view inherits its lender’s permission for exclusive
// transfer. Moving a shared reference to a view requires its Sync mode.
unsafe impl<T: ComInterface> Send for ComInterfacePointer<T, ThreadSend> {}
unsafe impl<T: ComInterface> Send for ComInterfacePointer<T, ThreadSafe> {}
// SAFETY: Sharing the view grants the same synchronous call permission as its
// ThreadSafe owner; it does not extend the owner's lifetime.
unsafe impl<T: ComInterface> Sync for ComInterfacePointer<T, ThreadSync> {}
unsafe impl<T: ComInterface> Sync for ComInterfacePointer<T, ThreadSafe> {}
