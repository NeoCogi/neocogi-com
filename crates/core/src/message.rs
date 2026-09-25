//! Generated owned messages and their call-scoped, concrete C representations.

use core::marker::PhantomData;
use crate::{NcAbiSignature, NcStatus, NC_E_POINTER, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
#[cfg(feature = "alloc")]
use crate::NC_E_OUT_OF_MEMORY;

/// Maximum owned data copied while decoding one call-scoped message.
pub const MAX_MESSAGE_DECODE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum nested message records decoded in one delivery.
pub const MAX_MESSAGE_DECODE_RECORDS: usize = 64;

/// One shared limit follows every nested message and every copied field.
/// The counter is ordinary mutable state local to one decode call.
pub struct MessageDecodeBudget {
    /// Bytes still available for consumer-owned strings and slices.
    remaining_bytes: usize,
    /// Message records still available before recursive decoding stops.
    remaining_records: usize,
}

impl MessageDecodeBudget {
    /// Creates explicit limits for a receiver that needs a smaller accepted payload.
    pub const fn new(bytes: usize, records: usize) -> Self {
        Self { remaining_bytes: bytes, remaining_records: records }
    }

    /// Charges one concrete message before examining its active fields.
    pub fn record(&mut self) -> Result<(), NcStatus> {
        self.remaining_records = self.remaining_records.checked_sub(1).ok_or(NC_E_OUT_OF_RANGE)?;
        Ok(())
    }

    /// Checks a copy against the remaining limit without changing it.
    pub fn check_bytes(&self, amount: usize) -> Result<(), NcStatus> {
        if amount > self.remaining_bytes { Err(NC_E_OUT_OF_RANGE) } else { Ok(()) }
    }

    /// Charges storage before a fallible allocation or element copy.
    pub fn bytes(&mut self, amount: usize) -> Result<(), NcStatus> {
        self.check_bytes(amount)?;
        self.remaining_bytes -= amount;
        Ok(())
    }
}

impl Default for MessageDecodeBudget {
    /// Applies the same finite policy to every ordinary COM message delivery.
    fn default() -> Self {
        Self::new(MAX_MESSAGE_DECODE_BYTES, MAX_MESSAGE_DECODE_RECORDS)
    }
}

/// Copies an already valid scalar slice after charging its full owned size.
#[cfg(feature = "alloc")]
#[doc(hidden)]
pub fn copy_message_slice<T: Copy>(source: &[T], budget: &mut MessageDecodeBudget) -> Result<alloc::vec::Vec<T>, NcStatus> {
    let bytes = source.len().checked_mul(core::mem::size_of::<T>()).ok_or(NC_E_OUT_OF_RANGE)?;
    budget.bytes(bytes.max(source.len()))?;
    let mut owned = alloc::vec::Vec::new();
    owned.try_reserve_exact(source.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
    owned.extend_from_slice(source);
    Ok(owned)
}

/// Validates UTF-8 before making one fallible, budgeted owned copy.
#[cfg(feature = "alloc")]
#[doc(hidden)]
pub fn copy_message_text(source: &[u8], budget: &mut MessageDecodeBudget) -> Result<alloc::string::String, NcStatus> {
    core::str::from_utf8(source).map_err(|_| NC_E_INVALID_ARG)?;
    let bytes = copy_message_slice(source, budget)?;
    alloc::string::String::from_utf8(bytes).map_err(|_| NC_E_INVALID_ARG)
}

/// Converts the explicit one-byte boolean wire form only after validating all elements.
#[cfg(feature = "alloc")]
#[doc(hidden)]
pub fn copy_message_bools(source: &[u8], budget: &mut MessageDecodeBudget) -> Result<alloc::vec::Vec<bool>, NcStatus> {
    if source.iter().any(|value| *value > 1) {
        return Err(NC_E_INVALID_ARG);
    }
    budget.bytes(source.len())?;
    let mut owned = alloc::vec::Vec::new();
    owned.try_reserve_exact(source.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
    owned.extend(source.iter().map(|value| *value == 1));
    Ok(owned)
}

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
    unsafe fn decode_with_budget(wire: Self::Wire, budget: &mut MessageDecodeBudget) -> Result<Self, NcStatus>;

    /// Applies the standard cumulative limit to a complete message delivery.
    ///
    /// # Safety
    /// The same initialized-wire and backing-storage rules as decode_with_budget apply.
    unsafe fn decode(wire: Self::Wire) -> Result<Self, NcStatus> {
        let mut budget = MessageDecodeBudget::default();
        // SAFETY: the caller guarantees that this concrete wire and its fields are live.
        unsafe { Self::decode_with_budget(wire, &mut budget) }
    }
}

/// A concrete message's wire data, borrowing its backing storage for one COM call.
/// Generated receivers decode it before enqueueing; this wrapper never enters a FIFO.
pub struct NcMessage<'a, M: ComMessage> {
    /// Copyable C representation whose pointers refer only to call-scoped storage.
    wire: M::Wire,
    /// Prevents safe callers from retaining the wire after its owner is released.
    owner: PhantomData<&'a M>,
}
impl<'a, M: ComMessage> Copy for NcMessage<'a, M> {}
impl<'a, M: ComMessage> Clone for NcMessage<'a, M> {
    /// Duplicates only the call-scoped wire record, retaining the same borrowed owner lifetime.
    fn clone(&self) -> Self {
        *self
    }
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
    /// Rejects an oversized buffer before constructing a reference from its raw pointer.
    ///
    /// # Safety
    /// For an accepted length, the pointer must still address initialized,
    /// readable elements for the duration of this borrow.
    pub unsafe fn as_slice_bounded(&self, budget: &MessageDecodeBudget) -> Result<&[T], NcStatus> {
        let len = usize::try_from(self.len).map_err(|_| NC_E_OUT_OF_RANGE)?;
        let bytes = len.checked_mul(core::mem::size_of::<T>()).ok_or(NC_E_OUT_OF_RANGE)?;
        budget.check_bytes(bytes.max(len))?;
        // SAFETY: the caller provides live storage; as_slice checks null,
        // alignment, and the final Rust object-size bound.
        unsafe { self.as_slice() }
    }

    /// Encodes initialized elements without copying or transferring their allocation.
    pub fn new(values: &[T]) -> Self {
        Self {
            pointer: values.as_ptr(),
            len: values.len() as u64,
        }
    }
}

/// Generated typed callback construction for one declared event payload.
/// The concrete receiver is queried through IUnknown only during binding.
#[cfg(feature = "alloc")]
pub trait ComEvent: ComMessage {
    /// Wraps a local mapper in the payload's exact receiver interface.
    fn make_receiver(consume: alloc::boxed::Box<dyn FnMut(Self) -> Result<(), NcStatus>>) -> Result<crate::ComPtr<crate::IUnknown>, NcStatus>;
}

/// Generated named installation on one producer interface, used only at setup.
/// This local Rust trait does not introduce a generic COM vtable or event payload.
pub trait ComOutputInterface: crate::ComInterface {
    /// Looks up the exact receiver contract without allocating an adapter.
    fn output_receiver(name: &str) -> Result<crate::NcInterfaceId, NcStatus>;
    /// Checks the destination and installs it through the concrete typed setter.
    fn bind_output(source: crate::ComRef<'_, Self>, name: &str, receiver: crate::ComRef<'_, crate::IUnknown>) -> Result<(), NcStatus>;
}

impl NcMessageSlice<u8> {
    /// Borrows valid Rust booleans as their specified one-byte 0/1 wire form.
    /// The enclosing NcMessage keeps the input slice alive for the COM call.
    pub fn from_bools(values: &[bool]) -> Self {
        Self { pointer: values.as_ptr().cast::<u8>(), len: values.len() as u64 }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_MESSAGE_DECODE_BYTES, MessageDecodeBudget, NcMessageSlice};
    use crate::NC_E_OUT_OF_RANGE;

    /// The size gate must run before a raw pointer is turned into a Rust slice.
    #[test]
    fn oversized_slice_is_rejected_without_accessing_its_pointer() {
        let wire = NcMessageSlice {
            pointer: core::ptr::NonNull::<u8>::dangling().as_ptr(),
            len: (MAX_MESSAGE_DECODE_BYTES + 1) as u64,
        };
        let budget = MessageDecodeBudget::default();
        // SAFETY: this length exceeds the budget and is rejected before the
        // pointer is dereferenced or used to construct a reference.
        let result = unsafe { wire.as_slice_bounded(&budget) };
        assert_eq!(result, Err(NC_E_OUT_OF_RANGE));
    }
}
