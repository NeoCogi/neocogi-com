//! Typed storage hooks and reference-count primitives used by generated classes.

#[cfg(feature = "alloc")]
mod storage;
#[cfg(feature = "alloc")]
pub use storage::{__ComBoxedInner, __ComSharedInner};

use core::ffi::c_void;
use neocogi_com::{NcStatus, ThreadingModel};

pub use neocogi_com::{__com_add_ref_count, __com_release_ref_count};

/// Construction proof connecting generated projections to a concrete Rust value.
///
/// # Safety
///
/// An implementation recovers the exact initialized value from its class's
/// shared or boxed storage. The canonical tables and the lifetime header must
/// agree for every instance. Shared storage permits only shared value access.
/// The declared threading capability must cover every projection and final
/// destruction.
pub unsafe trait ComObject: Sized + 'static {
    /// Threading capability selected at construction.
    type Threading: ThreadingModel;
    /// Canonical IUnknown prefix for the declared ownership policy.
    const UNKNOWN_VTABLE: &'static crate::IUnknownVTable;
    /// Immutable exact interface tables paired with this concrete storage.
    const VTABLES: &'static [*const crate::IUnknownVTable];

    /// Recovers the live target for a shared borrow.
    ///
    /// # Safety
    /// A nonnull object must address the exact matching initialized storage.
    /// The caller must retain the provider and grant shared target access for
    /// the requested lifetime. A null receiver is rejected.
    unsafe fn borrow_abi_ref<'a>(object: *mut c_void) -> Result<&'a Self, NcStatus>;
}

/// Construction proof for the exclusive access of a uniquely owned COM value.
///
/// # Safety
/// The class must use boxed storage. Every mutable access derives from the
/// sole owner or an already authorized exclusive reborrow.
pub unsafe trait ComExclusiveObject: ComObject {
    /// Recovers an exclusive reborrow of the live target.
    ///
    /// # Safety
    /// A nonnull object must address the exact matching initialized storage.
    /// The caller must retain the provider and grant exclusive target access
    /// for the requested lifetime. A null receiver is rejected.
    unsafe fn borrow_abi_mut<'a>(object: *mut c_void) -> Result<&'a mut Self, NcStatus>;
}
