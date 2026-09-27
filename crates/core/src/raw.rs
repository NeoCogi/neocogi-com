//! Explicit C representation of an object and one selected interface table.
//!
//! A raw pair carries no Rust ownership or lifetime. Safe ComPtr/ComRef wrappers
//! establish those obligations before exposing generated methods. No interface
//! headers are embedded in the object and no pointer arithmetic recovers its owner.

use core::ffi::c_void;
use crate::{ComInterface, IUnknown};

/// Two-word C ABI capability: one allocation and one interface's dispatch table.
///
/// The canonical absent value has both fields null. A present value has both
/// fields non-null and a table generated for this exact allocation's concrete
/// implementation. A half-null pair is malformed, not an optional absence.
/// Copying this record does not AddRef; ownership is specified by the enclosing
/// ABI operation. Public fields support foreign adapters without typeless Rust
/// downcasts. Dereferencing either pointer remains an unsafe operation.
#[repr(C)]
pub struct ComRaw<T: ComInterface + ?Sized> {
    /// Shared implementation allocation, passed unchanged to every method.
    pub object: *mut c_void,
    /// Immutable table for the selected contract and concrete implementation.
    pub vtable: *const T::VTable,
}

impl<T: ComInterface + ?Sized> Copy for ComRaw<T> {}
impl<T: ComInterface + ?Sized> Clone for ComRaw<T> {
    /// Copies the ABI record without acquiring another reference.
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ComInterface + ?Sized> ComRaw<T> {
    /// Constructs the canonical absent pair used by optional inputs and failed outputs.
    pub const fn null() -> Self {
        Self {
            object: core::ptr::null_mut(),
            vtable: core::ptr::null(),
        }
    }
    /// Reports canonical absence; a half-null malformed pair returns false.
    pub const fn is_null(self) -> bool {
        self.object.is_null() && self.vtable.is_null()
    }
    /// Checks only presence of both pointers, without proving memory or interface validity.
    pub const fn is_present(self) -> bool {
        !self.object.is_null() && !self.vtable.is_null()
    }
    /// Reinterprets the raw table type without validation, ownership, or dispatch.
    ///
    /// Like a raw-pointer cast, this does not prove the destination contract.
    /// Safe wrappers must validate identity or receive that proof from an unsafe caller.
    /// Generated inherited method views additionally rely on physical table prefixes.
    pub const fn cast<U: ComInterface>(self) -> ComRaw<U> {
        ComRaw {
            object: self.object,
            vtable: self.vtable.cast(),
        }
    }
    /// Selects the common lifecycle prefix without changing either address.
    pub const fn erase(self) -> ComRaw<IUnknown> {
        self.cast()
    }
}
impl<T: ComInterface + ?Sized> Default for ComRaw<T> {
    /// Defaults to absence, never to an owned object.
    fn default() -> Self {
        Self::null()
    }
}
impl<T: ComInterface + ?Sized> core::fmt::Debug for ComRaw<T> {
    /// Prints addresses without reading foreign storage.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ComRaw").field("object", &self.object).field("vtable", &self.vtable).finish()
    }
}
impl<T: ComInterface + ?Sized> PartialEq for ComRaw<T> {
    /// Compares the selected view, not merely the allocation's identity.
    fn eq(&self, other: &Self) -> bool {
        self.object == other.object && self.vtable == other.vtable
    }
}
impl<T: ComInterface + ?Sized> Eq for ComRaw<T> {}
