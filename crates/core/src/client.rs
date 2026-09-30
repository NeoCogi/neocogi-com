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
/// `Client<M>` must be a non-Copy, non-Clone `repr(transparent)` wrapper around
/// `NcClientPointer<Self, M>`. Its safe operations may only borrow the live
/// interface and may not manufacture an owning reference or an escaping
/// ComRef. Inherited views must preserve the exact base vtable prefix and the
/// supplied threading capability. Client operations must never overwrite the borrowed pair, including via a
/// mutable view: safe owners rely on its original non-null pointers. The
/// generated macro establishes these rules.
pub unsafe trait ComClientInterface: ComInterface + Sized {
    /// Borrowed method surface whose reference lifetime is tied to its pointer owner.
    type Client<M: ThreadingModel>;
}

/// Pointer storage for generated, non-owning method views.
///
/// This type has no public constructor, Clone, Copy, or ownership conversion.
/// A view is tied to a live owner/borrow or a retained exclusive weak token.
/// Exclusive providers must acquire lifetime protection at every ABI entry before
/// accessing their value; expiration returns NC_E_CLOSED. Its raw pair may
/// address an inherited prefix whose actual projected identity remains derived;
/// such a pointer must not be adopted as a base COM reference by a cast.
#[repr(transparent)]
pub struct NcClientPointer<T: ComInterface, M: ThreadingModel = ThreadLocal> {
    /// Same live object/table pair stored by the lending COM owner or borrow.
    pointer: crate::ComRaw<T>,
    /// Preserves the handle's thread-affinity restriction in the borrowed view.
    mode: PhantomData<M>,
}

impl<T: ComInterface, M: ThreadingModel> NcClientPointer<T, M> {
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
