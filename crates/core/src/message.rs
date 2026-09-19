//! Generated owned messages and their call-scoped, concrete C representations.

use core::marker::PhantomData;
use crate::{NcAbiSignature, NcStatus, NC_E_POINTER, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};

/// Compiler-generated conversion between an owned message and its borrowed wire data.
/// This trait is implementation support, not a dynamically typed message protocol.
///
/// # Safety
/// `Wire` must have a concrete C layout with no Rust references or drop glue.
/// Encoding must borrow only from `self`. Decoding must validate tags and borrowed
/// pointers, copy buffers, and retain objects before returning any owned value.
#[doc(hidden)]
pub unsafe trait ComMessage: Sized + 'static {
    /// Exact C record/union emitted for this message declaration.
    type Wire: Copy;
    /// Declaration fingerprint identifies both the fields and their ownership rules.
    const SIGNATURE: NcAbiSignature;
    /// Borrows fields without transferring Rust allocations to another binary.
    fn encode(&self) -> Self::Wire;
    /// Validates a call-scoped wire value and acquires all consumer-owned storage.
    ///
    /// # Safety
    /// The active fields and their referenced memory must be initialized, aligned
    /// and readable for this call. Interface pointers must have valid COM headers.
    unsafe fn decode(wire: Self::Wire) -> Result<Self, NcStatus>;
}

/// A concrete message's wire data, borrowing its backing storage for one COM call.
/// Generated receivers decode it before enqueueing; this wrapper never enters a FIFO.
pub struct NcMessage<'a, M: ComMessage> {
    /// Copyable C representation whose pointers refer only to call-scoped storage.
    wire: M::Wire,
    /// Prevents safe callers from retaining the wire after its owner is released.
    owner: PhantomData<&'a M>,
}
impl<'a, M: ComMessage> NcMessage<'a, M> {
    /// Encodes a borrowed message without relinquishing any producer-owned allocation.
    pub fn new(message: &'a M) -> Self {
        Self {
            wire: message.encode(),
            owner: PhantomData,
        }
    }
    /// Acquires a concrete owned message while every borrowed field is still live.
    pub fn to_owned(&self) -> Result<M, NcStatus> {
        // SAFETY: safe construction borrows a live message; raw construction below
        // requires the equivalent ABI lifetime and initialized-payload contract.
        unsafe { M::decode(self.wire) }
    }
    /// Borrows this exact generated C record for the immediate vtable call.
    #[doc(hidden)]
    pub fn as_ptr(&self) -> *const M::Wire {
        &self.wire
    }
    /// Restores a scoped wrapper at the generated COM entry point.
    ///
    /// # Safety
    /// A non-null pointer must address initialized, aligned `M::Wire` storage.
    /// Its active borrowed fields must satisfy `ComMessage::decode` for `'a`.
    #[doc(hidden)]
    pub unsafe fn from_raw(pointer: *const M::Wire) -> Result<Self, NcStatus> {
        if pointer.is_null() {
            return Err(NC_E_POINTER);
        }
        // SAFETY: the raw caller guarantees the record's storage and field lifetimes.
        Ok(Self {
            wire: unsafe { pointer.read() },
            owner: PhantomData,
        })
    }
}

/// Explicit pointer/length pair used only inside generated borrowed message records.
#[doc(hidden)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcMessageSlice<T: Copy> {
    /// Borrowed elements; the surrounding NcMessage guards their lifetime.
    pub pointer: *const T,
    /// Number of initialized elements, checked before materializing a consumer slice.
    pub len: u64,
}
impl<T: Copy> NcMessageSlice<T> {
    /// Checks the numeric range before exposing borrowed message storage.
    ///
    /// # Safety
    /// If the pointer and length pass these structural checks, the resulting
    /// range must contain initialized readable elements for this borrow.
    pub unsafe fn as_slice(&self) -> Result<&[T], NcStatus> {
        let len = usize::try_from(self.len).map_err(|_| NC_E_OUT_OF_RANGE)?;
        if len == 0 {
            return Ok(&[]);
        }
        if self.pointer.is_null() {
            return Err(NC_E_POINTER);
        }
        if !self.pointer.is_aligned() {
            return Err(NC_E_INVALID_ARG);
        }
        // Prevent slice construction from overflowing Rust's object-size limit.
        match len.checked_mul(core::mem::size_of::<T>()) {
            Some(bytes) if bytes <= isize::MAX as usize => {
                // SAFETY: structural checks passed; the raw caller owns readable storage.
                Ok(unsafe { core::slice::from_raw_parts(self.pointer, len) })
            }
            _ => Err(NC_E_OUT_OF_RANGE),
        }
    }
    /// Encodes initialized elements without copying or transferring their allocation.
    pub fn new(values: &[T]) -> Self {
        Self {
            pointer: values.as_ptr(),
            len: values.len() as u64,
        }
    }
}
