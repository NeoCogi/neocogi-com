//! Recursive conversion between application values and checked COM wire fields.
//!
//! The wire contains raw values that can be inspected before constructing Rust
//! booleans, enums, or owned containers. Producer storage is statically typed and
//! remains owned by the call guard. No application model or allocation crosses
//! the boundary, and no erased allocation registry participates in conversion.

use core::marker::PhantomData;
use crate::{NcStatus, NC_E_INVALID_ARG, NC_E_INVALID_TAG, NC_E_OUT_OF_RANGE, NC_E_POINTER};
#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};
#[cfg(feature = "alloc")]
use crate::NC_E_OUT_OF_MEMORY;

/// Default cumulative copied/temporary storage allowed for one value conversion.
pub const MAX_VALUE_BYTES: usize = 8 * 1024 * 1024;
/// Default cumulative number of values visited in one conversion.
pub const MAX_VALUE_COUNT: usize = 1_000_000;
/// Default maximum number of values on a root-to-leaf conversion path.
pub const MAX_VALUE_DEPTH: usize = 64;

/// One conversion's finite resource policy and traversal state.
/// Shared mutable access passes down the generated value graph; child codecs do
/// not silently reset the budget when entering a named record or enum payload.
pub struct ValueBudget {
    /// Remaining storage bytes, charged before allocation or copying.
    bytes: usize,
    /// Remaining logical values, including empty aggregates and unit values.
    values: usize,
    /// Remaining depth available below the current traversal position.
    depth: usize,
}
impl ValueBudget {
    /// Selects explicit storage, cumulative-value, and nesting bounds.
    pub const fn new(bytes: usize, values: usize, depth: usize) -> Self {
        Self { bytes, values, depth }
    }
    /// Charges one value and restores the nesting allowance after its children.
    pub fn value<T>(&mut self, operation: impl FnOnce(&mut Self) -> Result<T, NcStatus>) -> Result<T, NcStatus> {
        if self.values == 0 || self.depth == 0 {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.values -= 1;
        self.depth -= 1;
        let result = operation(self);
        self.depth += 1;
        result
    }
    /// Charges an allocation/copy before touching producer-controlled storage.
    pub fn bytes(&mut self, amount: usize) -> Result<(), NcStatus> {
        self.bytes = self.bytes.checked_sub(amount).ok_or(NC_E_OUT_OF_RANGE)?;
        Ok(())
    }
    /// Preflights a sequence's minimum work and checked element storage size.
    pub fn sequence<T>(&mut self, len: usize) -> Result<(), NcStatus> {
        if len > self.values {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.bytes(len.checked_mul(core::mem::size_of::<T>().max(1)).ok_or(NC_E_OUT_OF_RANGE)?)
    }
}
impl Default for ValueBudget {
    /// Uses the same finite defaults for producer encoding and receiver decoding.
    fn default() -> Self {
        Self::new(MAX_VALUE_BYTES, MAX_VALUE_COUNT, MAX_VALUE_DEPTH)
    }
}

/// One recursively checked application-value/wire conversion.
///
/// # Safety
/// `Wire` has a concrete C field layout with no references or drop glue. Active
/// scalar fields must be readable without already assuming restricted Rust
/// values are valid. Encoding returns storage that keeps every generated wire
/// pointer live while the application value is borrowed. Decoding validates raw
/// tags/fields before constructing Rust values and acquires all retained data.
pub unsafe trait ComValue: Sized + 'static {
    /// True only when Self and Wire have identical layout and all initialized
    /// bit patterns require no content validation, ownership, or conversion.
    /// The unsafe implementor must prove this for every field recursively.
    const BORROW_WIRE: bool = false;
    /// Call-scoped raw representation, recursively composed of wire fields.
    type Wire: Copy;
    /// Concrete producer-owned backing storage for converted indirect fields.
    type Storage;
    /// Encodes this value, retaining every temporary wire allocation in Storage.
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, Self::Storage), NcStatus>;
    /// Validates a raw value and acquires independent application-owned contents.
    ///
    /// # Safety
    /// Active raw storage and pointees must be initialized, aligned, readable,
    /// and live for this call. Interface fields must address valid COM headers.
    /// These memory obligations do not require raw boolean/tag values to be valid.
    unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus>;
}

/// A named value's compiler-assigned identity, independent of its ownership.
pub trait ComValueType: ComValue {
    /// Complete resolved IDL revision of this value and its referenced types.
    const VALUE_SIGNATURE: crate::NcAbiSignature;
}

/// An inline value whose wire has no borrowed storage or transferred ownership.
///
/// # Safety
/// The wire is self-contained and can be returned through a copied ABI out slot.
/// Dropping the original application value cannot invalidate any wire field.
pub unsafe trait ComInlineValue: ComValue + Copy {}

/// Typed owner of a value's raw record and all temporary producer wire storage.
/// The application borrow prevents original string/interface backing from being
/// destroyed before the call, while Storage retains converted sequence elements.
pub struct NcValueRef<'a, T: ComValue> {
    /// Raw record passed only to the synchronous COM call.
    wire: T::Wire,
    /// Encoding owns storage; raw receiver reconstruction borrows producer storage.
    _storage: Option<T::Storage>,
    /// Keeps original application allocations live for the complete call guard.
    owner: PhantomData<&'a T>,
}
impl<'a, T: ComValue> NcValueRef<'a, T> {
    /// Prepares a checked wire value, propagating bounded allocation failures.
    pub fn new(value: &'a T) -> Result<Self, NcStatus> {
        let (wire, storage) = value.encode_value(&mut ValueBudget::default())?;
        Ok(Self {
            wire,
            _storage: Some(storage),
            owner: PhantomData,
        })
    }
    /// Acquires a fully validated application value before producer storage expires.
    pub fn to_owned(&self) -> Result<T, NcStatus> {
        // SAFETY: construction retains the application borrow and wire storage;
        // raw construction requires the equivalent active-field memory contract.
        unsafe { T::decode_value(self.wire, &mut ValueBudget::default()) }
    }
    /// Borrows the raw record while this guard retains its typed backing storage.
    pub fn as_ptr(&self) -> *const T::Wire {
        &self.wire
    }
    /// Reconstructs a borrowed call guard at a generated COM entry point.
    ///
    /// # Safety
    /// The pointer and active pointees must satisfy ComValue::decode_value for
    /// this call. Raw scalar/tag contents may be invalid and will be checked.
    pub unsafe fn from_raw(pointer: *const T::Wire) -> Result<Self, NcStatus> {
        if pointer.is_null() {
            return Err(NC_E_POINTER);
        }
        if !(pointer as usize).is_multiple_of(core::mem::align_of::<T::Wire>()) {
            return Err(NC_E_INVALID_ARG);
        }
        // SAFETY: the caller provides initialized, aligned raw wire storage.
        Ok(Self {
            wire: unsafe { pointer.read() },
            _storage: None,
            owner: PhantomData,
        })
    }
}

/// Implements scalar leaves whose initialized bit patterns are all valid.
macro_rules! scalar_values {
    ($($ty:ty),* $(,)?) => {$(
        // SAFETY: these primitive wire values have no restricted bit patterns,
        // borrowed fields, or ownership obligations.
        unsafe impl ComValue for $ty {
            const BORROW_WIRE: bool = true;
            type Wire = Self;
            type Storage = ();
            fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self, ()), NcStatus> {
                budget.value(|_| Ok((*self, ())))
            }
            unsafe fn decode_value(wire: Self, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
                budget.value(|_| Ok(wire))
            }
        }
        // SAFETY: primitive scalar wires are self-contained copied values.
        unsafe impl ComInlineValue for $ty {}
    )*};
}
scalar_values!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64, crate::NcGuid, crate::NcStatus);

// SAFETY: boolean wire storage is an unrestricted byte, checked before conversion.
unsafe impl ComValue for bool {
    type Wire = u8;
    type Storage = ();
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(u8, ()), NcStatus> {
        budget.value(|_| Ok((u8::from(*self), ())))
    }
    unsafe fn decode_value(wire: u8, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|_| match wire {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(NC_E_INVALID_ARG),
        })
    }
}
// SAFETY: boolean wire bytes do not borrow any source storage.
unsafe impl ComInlineValue for bool {}

/// Raw optional storage reads its payload only when the validated tag is present.
#[repr(C)]
pub struct ValueOption<W: Copy> {
    /// Zero denotes absent; one denotes the initialized payload.
    pub present: u8,
    /// Absent values do not require an initialized payload.
    pub payload: core::mem::MaybeUninit<W>,
}
impl<W: Copy> Copy for ValueOption<W> {}
impl<W: Copy> Clone for ValueOption<W> {
    /// Copies raw storage without inspecting an absent payload.
    fn clone(&self) -> Self {
        *self
    }
}
// SAFETY: the optional tag is inspected before the recursively checked payload.
unsafe impl<T: ComValue> ComValue for Option<T> {
    type Wire = ValueOption<T::Wire>;
    type Storage = Option<T::Storage>;
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, Self::Storage), NcStatus> {
        budget.value(|budget| match self {
            None => Ok((
                ValueOption {
                    present: 0,
                    payload: core::mem::MaybeUninit::uninit(),
                },
                None,
            )),
            Some(value) => {
                let (wire, storage) = value.encode_value(budget)?;
                Ok((
                    ValueOption {
                        present: 1,
                        payload: core::mem::MaybeUninit::new(wire),
                    },
                    Some(storage),
                ))
            }
        })
    }
    unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|budget| match wire.present {
            0 => Ok(None),
            // SAFETY: the validated present tag selects the initialized raw payload.
            1 => unsafe { T::decode_value(wire.payload.assume_init(), budget) }.map(Some),
            _ => Err(NC_E_INVALID_TAG),
        })
    }
}

/// C-layout wrapper keeps arrays out of native Rust by-value array ABI positions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ValueArray<W: Copy, const N: usize> {
    /// Ordered raw elements, each recursively validated before acquisition.
    pub elements: [W; N],
}
// SAFETY: every array element uses its declared checked raw representation.
unsafe impl<T: ComValue, const N: usize> ComValue for [T; N] {
    type Wire = ValueArray<T::Wire, N>;
    type Storage = [T::Storage; N];
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, Self::Storage), NcStatus> {
        budget.value(|budget| {
            let mut slots: [Option<(T::Wire, T::Storage)>; N] = core::array::from_fn(|_| None);
            for (slot, value) in slots.iter_mut().zip(self) {
                *slot = Some(value.encode_value(budget)?);
            }
            let elements = core::array::from_fn(|index| slots[index].as_ref().expect("initialized wire slot").0);
            let storage = slots.map(|slot| slot.expect("initialized storage slot").1);
            Ok((ValueArray { elements }, storage))
        })
    }
    unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|budget| {
            let mut values: [Option<T>; N] = core::array::from_fn(|_| None);
            for (slot, raw) in values.iter_mut().zip(wire.elements) {
                // SAFETY: the caller guarantees initialized raw elements and pointees.
                *slot = Some(unsafe { T::decode_value(raw, budget) }?);
            }
            Ok(values.map(|value| value.expect("initialized application slot")))
        })
    }
}

/// A raw sequence points to wire elements rather than application Rust values.
#[repr(C)]
pub struct ValueSlice<W: Copy> {
    /// Live producer-owned raw elements for the duration of one call.
    pub pointer: *const W,
    /// Number of elements, validated before constructing a borrowed raw slice.
    pub len: u64,
}
impl<W: Copy> Copy for ValueSlice<W> {}
impl<W: Copy> Clone for ValueSlice<W> {
    /// Copies only the raw descriptor, not the producer's ownership.
    fn clone(&self) -> Self {
        *self
    }
}
impl<W: Copy> ValueSlice<W> {
    /// Checks numeric shape; memory readability remains the raw caller's obligation.
    pub(crate) unsafe fn elements<'a>(&self) -> Result<&'a [W], NcStatus> {
        let len = usize::try_from(self.len).map_err(|_| NC_E_OUT_OF_RANGE)?;
        if len == 0 {
            return Ok(&[]);
        }
        if self.pointer.is_null() {
            return Err(NC_E_POINTER);
        }
        if !(self.pointer as usize).is_multiple_of(core::mem::align_of::<W>()) {
            return Err(NC_E_INVALID_ARG);
        }
        if len.checked_mul(core::mem::size_of::<W>()).is_none_or(|bytes| bytes > isize::MAX as usize) {
            return Err(NC_E_OUT_OF_RANGE);
        }
        // SAFETY: structural checks passed; the caller guarantees backing memory.
        Ok(unsafe { core::slice::from_raw_parts(self.pointer, len) })
    }
}

/// Typed temporary arrays retain both raw elements and their indirect backing.
#[cfg(feature = "alloc")]
pub struct ValueSequenceStorage<T: ComValue> {
    /// Contiguous wire elements addressed by the exported sequence descriptor.
    wires: Vec<T::Wire>,
    /// Storage for every element, retained until the synchronous call returns.
    children: Vec<T::Storage>,
}
#[cfg(feature = "alloc")]
// SAFETY: raw sequence elements are independently encoded and their storage lives
// in the call guard; decoding validates each element before returning Vec<T>.
unsafe impl<T: ComValue> ComValue for Vec<T> {
    type Wire = ValueSlice<T::Wire>;
    type Storage = ValueSequenceStorage<T>;
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, Self::Storage), NcStatus> {
        budget.value(|budget| {
            budget.sequence::<T::Wire>(self.len())?;
            // The second allocation owns child guards as well as the wire array.
            budget.bytes(self.len().checked_mul(core::mem::size_of::<T::Storage>()).ok_or(NC_E_OUT_OF_RANGE)?)?;
            let mut storage = ValueSequenceStorage { wires: Vec::new(), children: Vec::new() };
            storage.wires.try_reserve_exact(self.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            storage.children.try_reserve_exact(self.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            for value in self {
                let (wire, child) = value.encode_value(budget)?;
                storage.wires.push(wire);
                storage.children.push(child);
            }
            let wire = ValueSlice {
                pointer: storage.wires.as_ptr(),
                len: storage.wires.len() as u64,
            };
            Ok((wire, storage))
        })
    }
    unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|budget| {
            let len = usize::try_from(wire.len).map_err(|_| NC_E_OUT_OF_RANGE)?;
            budget.sequence::<T>(len)?;
            // SAFETY: the caller supplies live raw elements; size checks precede this borrow.
            let elements = unsafe { wire.elements() }?;
            let mut values = Vec::new();
            values.try_reserve_exact(len).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            for element in elements {
                // SAFETY: each element and active backing satisfy the enclosing raw contract.
                values.push(unsafe { T::decode_value(*element, budget) }?);
            }
            Ok(values)
        })
    }
}

#[cfg(feature = "alloc")]
// SAFETY: wire text is raw bytes; UTF-8 is checked before creating String.
unsafe impl ComValue for String {
    type Wire = ValueSlice<u8>;
    type Storage = ();
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, ()), NcStatus> {
        budget.value(|budget| {
            budget.bytes(self.len())?;
            Ok((
                ValueSlice {
                    pointer: self.as_ptr(),
                    len: self.len() as u64,
                },
                (),
            ))
        })
    }
    unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|budget| {
            budget.bytes(usize::try_from(wire.len).map_err(|_| NC_E_OUT_OF_RANGE)?)?;
            // SAFETY: the caller supplies live bytes and the size was preflighted.
            let text = core::str::from_utf8(unsafe { wire.elements() }?).map_err(|_| NC_E_INVALID_ARG)?;
            let mut owned = String::new();
            owned.try_reserve_exact(text.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            owned.push_str(text);
            Ok(owned)
        })
    }
}

// SAFETY: wire object fields borrow a valid COM projection. Acquisition checks
// its complete interface identity and obtains a reference before returning.
unsafe impl<T: crate::ComInterface + 'static, M: crate::ThreadingModel> ComValue for crate::ComPtr<T, M> {
    type Wire = *mut T;
    type Storage = ();
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, ()), NcStatus> {
        budget.value(|_| Ok((self.as_mut_ptr(), ())))
    }
    unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|_| {
            // SAFETY: raw callers provide a live common header; this operation
            // checks identity before acquiring a separately owned reference.
            Ok(unsafe { crate::ComRef::<T, M>::__nc_from_raw_borrowed_checked(wire) }?.add_ref())
        })
    }
}

// SAFETY: optional and array wires remain self-contained when their elements do.
unsafe impl<T: ComInlineValue> ComInlineValue for Option<T> {}
unsafe impl<T: ComInlineValue, const N: usize> ComInlineValue for [T; N] {}

/// Converts a copied input or output using the same recursive checked codec.
/// Inline proof ensures discarded storage cannot invalidate the returned wire.
pub fn encode_inline<T: ComInlineValue>(value: &T) -> Result<T::Wire, NcStatus> {
    value.encode_value(&mut ValueBudget::default()).map(|(wire, _storage)| wire)
}
/// Acquires a copied value after validating every active field.
///
/// # Safety
/// The raw value must satisfy ComValue::decode_value's initialized-storage contract.
pub unsafe fn decode_inline<T: ComInlineValue>(wire: T::Wire) -> Result<T, NcStatus> {
    unsafe { T::decode_value(wire, &mut ValueBudget::default()) }
}

/// Encodes an initialized buffer into raw elements without assuming Rust layout.
#[cfg(feature = "alloc")]
pub fn encode_value_buffer<T: ComInlineValue>(values: &[T]) -> Result<Vec<T::Wire>, NcStatus> {
    let mut budget = ValueBudget::default();
    budget.sequence::<T::Wire>(values.len())?;
    let mut result = Vec::new();
    result.try_reserve_exact(values.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
    for value in values {
        result.push(value.encode_value(&mut budget)?.0);
    }
    Ok(result)
}

/// Validates every raw element before exposing an application buffer.
///
/// # Safety
/// The pointer addresses len initialized raw elements, live for this call.
#[cfg(feature = "alloc")]
pub unsafe fn decode_value_buffer<T: ComInlineValue>(pointer: *const T::Wire, len: u64) -> Result<Vec<T>, NcStatus> {
    let mut budget = ValueBudget::default();
    let count = usize::try_from(len).map_err(|_| NC_E_OUT_OF_RANGE)?;
    budget.sequence::<T>(count)?;
    let descriptor = ValueSlice { pointer, len };
    let elements = unsafe { descriptor.elements() }?;
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| NC_E_OUT_OF_MEMORY)?;
    for element in elements {
        result.push(unsafe { T::decode_value(*element, &mut budget) }?);
    }
    Ok(result)
}

// SAFETY: unit has a concrete byte representation; only its sole value is accepted.
unsafe impl ComValue for () {
    type Wire = u8;
    type Storage = ();
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(u8, ()), NcStatus> {
        budget.value(|_| Ok((0, ())))
    }
    unsafe fn decode_value(wire: u8, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|_| match wire {
            0 => Ok(()),
            _ => Err(NC_E_INVALID_ARG),
        })
    }
}
// SAFETY: the unit byte is self-contained.
unsafe impl ComInlineValue for () {}
