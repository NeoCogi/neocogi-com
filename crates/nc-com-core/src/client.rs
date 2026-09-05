//! Borrowed Rust method views for statically declared COM interface inheritance.
//!
//! Views lend methods, never COM ownership. In particular, an inherited view's
//! header may still advertise the derived identity; obtaining an owned base
//! projection continues to require checked QueryInterface.

use core::{marker::PhantomData, ptr::NonNull};
use crate::{ComInterface, ThreadLocal, ThreadSafe, ThreadingModel};

/// Optional static method-view support generated for a COM interface.
///
/// Ordinary foreign interface adapters need only implement ComInterface. This
/// additional unsafe trait enables Deref-based method lookup without changing
/// the representation or ownership of a COM pointer.
///
/// # Safety
///
/// Client<M> must be a non-Copy, non-Clone repr(transparent) wrapper around
/// NcClientPointer<Self, M>. Its safe operations may only borrow the live
/// interface and may not manufacture an owning reference or an escaping
/// ComRef. Inherited views must preserve the exact base vtable prefix and the
/// supplied threading capability. The generated macro establishes these rules.
pub unsafe trait ComClientInterface: ComInterface + Sized {
    /// Borrowed method surface whose reference lifetime is tied to its pointer owner.
    type Client<M: ThreadingModel>;
}

/// Pointer storage for generated, non-owning method views.
///
/// This type has no public constructor, Clone, Copy, or ownership conversion.
/// Only a reference tied to a live ComPtr/ComRef is exposed. Its raw pointer may
/// address an inherited prefix whose actual projected identity remains derived;
/// such a pointer must not be adopted as a base COM reference by a cast.
#[repr(transparent)]
pub struct NcClientPointer<T: ComInterface, M: ThreadingModel = ThreadLocal> {
    /// Same live pointer word stored by the lending COM owner or borrow.
    pointer: NonNull<T>,
    /// Preserves the handle's thread-affinity restriction in the borrowed view.
    mode: PhantomData<M>,
}

impl<T: ComInterface, M: ThreadingModel> NcClientPointer<T, M> {
    /// Returns the raw vtable-prefix pointer used by generated synchronous calls.
    ///
    /// This does not transfer ownership or certify the prefix as a separately
    /// projected base interface. Raw ABI use remains the caller's unsafe obligation.
    #[doc(hidden)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.pointer.as_ptr()
    }
}

// SAFETY: A ThreadSafe view can only be lent by a handle whose construction
// guarantees concurrent method calls are supported. The view has no ownership.
unsafe impl<T: ComInterface> Send for NcClientPointer<T, ThreadSafe> {}
// SAFETY: Sharing the view grants the same synchronous call permission as its
// ThreadSafe owner; it does not extend the owner's lifetime.
unsafe impl<T: ComInterface> Sync for NcClientPointer<T, ThreadSafe> {}
