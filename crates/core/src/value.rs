//! Recursive conversion between application values and checked COM wire fields.
//!
//! The wire contains raw values that can be inspected before constructing Rust
//! booleans, enums, or owned containers. Producer storage is statically typed and
//! remains owned by the call guard. No application model or allocation crosses
//! the boundary, and no erased allocation registry participates in conversion.

use core::marker::PhantomData;
use crate::{ValueCost, NcStatus, NC_E_INVALID_ARG, NC_E_INVALID_TAG, NC_E_OUT_OF_RANGE, NC_E_POINTER};
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
    /// Counts one entire allocation-free value, including its nested fields.
    /// Fixed preflight consumes the exact count once and observes the same depth
    /// bound without entering each descendant. Failure may leave the budget spent;
    /// callers must discard a failed conversion rather than reuse its partial work.
    #[inline(always)]
    pub fn fixed(&mut self, cost: ValueCost) -> Result<(), NcStatus> {
        if self.depth < cost.depth {
            return Err(NC_E_OUT_OF_RANGE);
        }
        self.values = self.values.checked_sub(cost.values).ok_or(NC_E_OUT_OF_RANGE)?;
        Ok(())
    }
    /// Counts an aggregate and all of its known fixed fields together.
    ///
    /// Generated code converts those fields with their prepaid validators and
    /// passes this budget to every remaining dynamic field. Descending one level
    /// keeps those dynamic fields at the same depth as an ordinary traversal.
    #[doc(hidden)]
    pub fn aggregate<T>(&mut self, cost: ValueCost, operation: impl FnOnce(&mut Self) -> Result<T, NcStatus>) -> Result<T, NcStatus> {
        self.fixed(cost)?;
        // Every aggregate cost includes its root, so preflight ensured depth > 0.
        self.depth -= 1;
        let result = operation(self);
        self.depth += 1;
        result
    }
    /// Prepays fixed elements or an affordable allocation-free upper bound.
    /// Exact costs reject impossible buffers before allocation. A maximum that
    /// does not fit falls back to actual variant accounting; it never rejects
    /// an ordinary buffer merely because a more expensive variant exists.
    #[cfg(feature = "alloc")]
    fn fixed_elements<T: ComValue>(&mut self, len: usize) -> Result<bool, NcStatus> {
        match T::FIXED_COST {
            Some(cost) => {
                self.fixed(cost.elements(len).ok_or(NC_E_OUT_OF_RANGE)?)?;
                Ok(true)
            }
            None => match T::MAX_COST.and_then(|cost| cost.elements(len)) {
                Some(cost) if cost.values <= self.values && cost.depth <= self.depth => {
                    self.fixed(cost)?;
                    Ok(true)
                }
                _ => Ok(false),
            },
        }
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
    #[inline(always)]
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
    /// Exact cost when every active alternative has the same allocation-free shape.
    /// Some(cost) promises that the hidden fixed converters validate the same raw
    /// fields as ordinary conversion without allocating or visiting dynamic data.
    const FIXED_COST: Option<ValueCost> = None;
    /// Upper bound for allocation-free shapes, including differing enum alternatives.
    ///
    /// A fresh inline conversion can omit bookkeeping when every possible shape
    /// fits its complete budget. A bound that does not fit is never a rejection:
    /// conversion falls back to exact accounting for the actual value instead.
    /// Both prechecked converters must work for every active shape covered here.
    const MAX_COST: Option<ValueCost> = Self::FIXED_COST;
    /// Call-scoped raw representation, recursively composed of wire fields.
    type Wire: Copy;
    /// Concrete producer-owned backing storage for converted indirect fields.
    type Storage;
    /// Encodes this value, retaining every temporary wire allocation in Storage.
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, Self::Storage), NcStatus>;
    /// Converts fields after their resource requirements have been checked.
    /// This is macro infrastructure, not an alternative unchecked wire format.
    ///
    /// # Safety
    /// Either FIXED_COST or MAX_COST was charged to the shared conversion
    /// budget, or MAX_COST fits the entire budget of an isolated conversion
    /// with no subsequent work. Charging a maximum prepays any selected variant.
    /// Every active child must have a finite allocation-free MAX_COST. Content
    /// validation remains required even though budget bookkeeping is omitted.
    #[doc(hidden)]
    unsafe fn encode_prechecked(&self) -> Result<(Self::Wire, Self::Storage), NcStatus> {
        unreachable!("unbounded codec cannot use prechecked conversion")
    }
    /// Validates raw fields after proving the entire conversion fits its budget.
    ///
    /// # Safety
    /// The caller must satisfy decode_value's memory contract and encode_prechecked's
    /// resource preflight contract. Restricted boolean/tag values still require checks.
    #[doc(hidden)]
    unsafe fn decode_prechecked(_wire: Self::Wire) -> Result<Self, NcStatus> {
        unreachable!("unbounded codec cannot use prechecked conversion")
    }
    /// Validates a raw value and acquires independent application-owned contents.
    ///
    /// # Safety
    /// Active raw storage and pointees must be initialized, aligned, readable,
    /// and live for this call. Interface fields must pair live COM objects with their matching tables.
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

/// Guard for a value's raw record and its call-scoped backing storage.
///
/// A producer created by [`Self::new`] owns temporary wire storage and borrows
/// original string/interface backing through `'a`. A receiver reconstructed by
/// [`Self::from_raw`] borrows the producer's backing storage for the call only.
/// COM parameter declarations must use `NcValueRef<'_, T>`; receivers acquire an
/// owned `T` with [`Self::to_owned`] before retaining data or posting it to a queue.
/// Even a static application value can require a temporary array of wire records:
/// a static source lifetime on a producer guard never grants the receiver a static
/// borrow of those records. The macro enforces this at the declaration boundary.
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
    /// The pointer must be initialized, aligned, and readable for this operation.
    /// Active pointees must satisfy [`ComValue::decode_value`] for all of `'a`,
    /// including every nested allocation retained by the producer's wire storage.
    /// The returned guard does not own that storage; `'a` must not outlive it.
    /// Generated entry points bound this borrow to the synchronous call.
    /// Raw scalar/tag contents may be invalid and will be checked by `to_owned`.
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
            const FIXED_COST: Option<ValueCost> = Some(ValueCost::SCALAR);
            type Wire = Self;
            type Storage = ();
            /// Counts this scalar and copies its unrestricted value.
            fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self, ()), NcStatus> {
                budget.value(|_| Ok((*self, ())))
            }
            /// Counts the scalar before accepting any initialized bit pattern.
            unsafe fn decode_value(wire: Self, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
                budget.value(|_| Ok(wire))
            }
            /// Copies a scalar whose resource requirements were already checked.
            unsafe fn encode_prechecked(&self) -> Result<(Self, ()), NcStatus> { Ok((*self, ())) }
            /// Accepts unrestricted scalar bits after the enclosing resource check.
            unsafe fn decode_prechecked(wire: Self) -> Result<Self, NcStatus> { Ok(wire) }
        }
        // SAFETY: primitive scalar wires are self-contained copied values.
        unsafe impl ComInlineValue for $ty {}
    )*};
}
scalar_values!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64, crate::NcGuid, crate::NcStatus);

// SAFETY: boolean wire storage is an unrestricted byte, checked before conversion.
unsafe impl ComValue for bool {
    const FIXED_COST: Option<ValueCost> = Some(ValueCost::SCALAR);
    type Wire = u8;
    type Storage = ();
    /// Counts the boolean and lowers its known Rust value to a valid wire byte.
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(u8, ()), NcStatus> {
        budget.value(|_| Ok((u8::from(*self), ())))
    }
    /// Counts this leaf and validates its raw byte before constructing a Rust value.
    unsafe fn decode_value(wire: u8, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|_| unsafe { Self::decode_prechecked(wire) })
    }
    /// Encodes the sole active leaf after its enclosing resource check.
    unsafe fn encode_prechecked(&self) -> Result<(u8, ()), NcStatus> {
        Ok((u8::from(*self), ()))
    }
    /// Rejects invalid raw bytes even when no budget bookkeeping is needed.
    unsafe fn decode_prechecked(wire: u8) -> Result<Self, NcStatus> {
        match wire {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(NC_E_INVALID_ARG),
        }
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
    const MAX_COST: Option<ValueCost> = ValueCost::aggregate(&[T::MAX_COST]);
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
    /// Converts the selected optional payload after the complete bound was checked.
    unsafe fn encode_prechecked(&self) -> Result<(Self::Wire, Self::Storage), NcStatus> {
        match self {
            None => Ok((
                ValueOption {
                    present: 0,
                    payload: core::mem::MaybeUninit::uninit(),
                },
                None,
            )),
            Some(value) => {
                // SAFETY: the enclosing bound includes the present child's maximum cost.
                let (wire, storage) = unsafe { value.encode_prechecked() }?;
                Ok((
                    ValueOption {
                        present: 1,
                        payload: core::mem::MaybeUninit::new(wire),
                    },
                    Some(storage),
                ))
            }
        }
    }
    /// Validates presence before inspecting the prechecked optional payload.
    unsafe fn decode_prechecked(wire: Self::Wire) -> Result<Self, NcStatus> {
        match wire.present {
            0 => Ok(None),
            // SAFETY: the tag selects initialized storage and the child bound fits.
            1 => unsafe { T::decode_prechecked(wire.payload.assume_init()) }.map(Some),
            _ => Err(NC_E_INVALID_TAG),
        }
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
    // The repr(C) wrapper contains exactly the element array at offset zero.
    const BORROW_WIRE: bool = T::BORROW_WIRE
        && core::mem::size_of::<Self>() == core::mem::size_of::<Self::Wire>()
        && core::mem::align_of::<Self>() == core::mem::align_of::<Self::Wire>();
    const FIXED_COST: Option<ValueCost> = ValueCost::array(T::FIXED_COST, N);
    const MAX_COST: Option<ValueCost> = ValueCost::array(T::MAX_COST, N);
    type Wire = ValueArray<T::Wire, N>;
    type Storage = [T::Storage; N];
    /// Preflights a fixed array once or shares dynamic accounting with its children.
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, Self::Storage), NcStatus> {
        match Self::FIXED_COST {
            Some(cost) => {
                budget.fixed(cost)?;
                // SAFETY: every element's exact fixed cost is included above.
                unsafe { Self::encode_prechecked(self) }
            }
            None => budget.value(|budget| encode_array(self, |value| value.encode_value(budget))),
        }
    }
    /// Active raw elements remain checked in both accounting modes.
    unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        match Self::FIXED_COST {
            Some(cost) => {
                budget.fixed(cost)?;
                // SAFETY: raw element memory is valid and their exact cost is paid.
                unsafe { Self::decode_prechecked(wire) }
            }
            None => budget.value(|budget| decode_array(wire, |raw| unsafe { T::decode_value(raw, budget) })),
        }
    }
    /// Converts each statically accounted child without double charging it.
    unsafe fn encode_prechecked(&self) -> Result<(Self::Wire, Self::Storage), NcStatus> {
        encode_array(self, |value| unsafe { T::encode_prechecked(value) })
    }
    /// Validates each statically accounted child before assembling application storage.
    unsafe fn decode_prechecked(wire: Self::Wire) -> Result<Self, NcStatus> {
        decode_array(wire, |raw| unsafe { T::decode_prechecked(raw) })
    }
}

/// Retains initialized array guards and drops a partial prefix on conversion failure.
fn encode_array<T: ComValue, const N: usize>(
    values: &[T; N],
    mut convert: impl FnMut(&T) -> Result<(T::Wire, T::Storage), NcStatus>,
) -> Result<(ValueArray<T::Wire, N>, [T::Storage; N]), NcStatus> {
    let mut slots: [Option<(T::Wire, T::Storage)>; N] = core::array::from_fn(|_| None);
    for (slot, value) in slots.iter_mut().zip(values) {
        *slot = Some(convert(value)?);
    }
    let elements = core::array::from_fn(|index| slots[index].as_ref().expect("initialized wire slot").0);
    let storage = slots.map(|slot| slot.expect("initialized storage slot").1);
    Ok((ValueArray { elements }, storage))
}

/// Acquires an array transactionally without leaking an already-decoded prefix.
fn decode_array<T: ComValue, const N: usize>(
    wire: ValueArray<T::Wire, N>,
    mut convert: impl FnMut(T::Wire) -> Result<T, NcStatus>,
) -> Result<[T; N], NcStatus> {
    let mut values: [Option<T>; N] = core::array::from_fn(|_| None);
    for (slot, raw) in values.iter_mut().zip(wire.elements) {
        *slot = Some(convert(raw)?);
    }
    Ok(values.map(|value| value.expect("initialized application slot")))
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
            let fixed = budget.fixed_elements::<T>(self.len())?;
            let mut storage = ValueSequenceStorage { wires: Vec::new(), children: Vec::new() };
            storage.wires.try_reserve_exact(self.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            storage.children.try_reserve_exact(self.len()).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            for value in self {
                let (wire, child) = match fixed {
                    // SAFETY: fixed_elements prepaid this element's exact cost.
                    true => unsafe { value.encode_prechecked() },
                    false => value.encode_value(budget),
                }?;
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
            let fixed = budget.fixed_elements::<T>(len)?;
            // SAFETY: the caller supplies live raw elements; size checks precede this borrow.
            let elements = unsafe { wire.elements() }?;
            let mut values = Vec::new();
            values.try_reserve_exact(len).map_err(|_| NC_E_OUT_OF_MEMORY)?;
            for element in elements {
                // SAFETY: each element and active backing satisfy the enclosing raw contract.
                values.push(match fixed {
                    // SAFETY: elements are initialized; their fixed costs were prepaid.
                    true => unsafe { T::decode_prechecked(*element) },
                    false => unsafe { T::decode_value(*element, budget) },
                }?);
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
    type Wire = crate::ComRaw<T>;
    type Storage = ();
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(Self::Wire, ()), NcStatus> {
        budget.value(|_| Ok((self.as_raw(), ())))
    }
    unsafe fn decode_value(wire: Self::Wire, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|_| {
            // SAFETY: raw callers provide a live common lifecycle pair; this operation
            // checks identity before acquiring a separately owned reference.
            unsafe { crate::ComRef::<T, M>::__nc_from_raw_borrowed_checked(wire) }?.try_add_ref()
        })
    }
}

// SAFETY: optional and array wires remain self-contained when their elements do.
unsafe impl<T: ComInlineValue> ComInlineValue for Option<T> {}
unsafe impl<T: ComInlineValue, const N: usize> ComInlineValue for [T; N] {}

/// Converts a copied input or output using the same recursive checked codec.
/// Inline proof ensures discarded storage cannot invalidate the returned wire.
#[inline(always)]
pub fn encode_inline<T: ComInlineValue>(value: &T) -> Result<T::Wire, NcStatus> {
    match T::MAX_COST {
        Some(cost) if cost.values <= MAX_VALUE_COUNT && cost.depth <= MAX_VALUE_DEPTH => {
            // SAFETY: every possible allocation-free shape fits this isolated conversion.
            unsafe { value.encode_prechecked() }.map(|(wire, _storage)| wire)
        }
        // A large bound is not grounds to reject a smaller active alternative.
        _ => value.encode_value(&mut ValueBudget::default()).map(|(wire, _storage)| wire),
    }
}
/// Acquires a copied value after validating every active field.
///
/// # Safety
/// The raw value must satisfy ComValue::decode_value's initialized-storage contract.
#[inline(always)]
pub unsafe fn decode_inline<T: ComInlineValue>(wire: T::Wire) -> Result<T, NcStatus> {
    match T::MAX_COST {
        Some(cost) if cost.values <= MAX_VALUE_COUNT && cost.depth <= MAX_VALUE_DEPTH => {
            // SAFETY: raw storage is initialized; the complete bound fits an isolated
            // conversion. This still validates the selected tag and every active field.
            unsafe { T::decode_prechecked(wire) }
        }
        // SAFETY: the caller supplies the same raw backing for ordinary accounting.
        _ => unsafe { T::decode_value(wire, &mut ValueBudget::default()) },
    }
}

/// Reserves bounded element storage without initializing any application values.
/// Call guards use this before dispatch, so completion never asks the allocator
/// for space. The byte limit applies to the actual native/wire element layout.
#[cfg(feature = "alloc")]
pub(crate) fn reserve_value_buffer<T>(len: usize) -> Result<Vec<T>, NcStatus> {
    ValueBudget::default().sequence::<T>(len)?;
    let mut result = Vec::new();
    result.try_reserve_exact(len).map_err(|_| NC_E_OUT_OF_MEMORY)?;
    Ok(result)
}

/// Computes the shared conversion policy before allocation or raw element reads.
/// `Element` supplies recursive costs; `Stored` determines this direction's bytes.
#[cfg(feature = "alloc")]
fn buffer_budget<Element: ComInlineValue, Stored>(len: usize) -> Result<(ValueBudget, bool), NcStatus> {
    let mut budget = ValueBudget::default();
    budget.sequence::<Stored>(len)?;
    let prechecked = budget.fixed_elements::<Element>(len)?;
    Ok((budget, prechecked))
}

/// Encodes into capacity already owned by the caller, using the shared codec loop.
/// Capacity is an internal preparation invariant; no reserve occurs in this loop.
#[cfg(feature = "alloc")]
fn encode_buffer_elements<T: ComInlineValue>(values: &[T], result: &mut Vec<T::Wire>, mut budget: ValueBudget, prechecked: bool) -> Result<(), NcStatus> {
    assert!(result.capacity() >= values.len(), "output wire capacity was not prepared");
    result.clear();
    for value in values {
        result.push(
            match prechecked {
                // SAFETY: the complete fixed/maximum cost was prepaid for every element.
                true => unsafe { value.encode_prechecked() },
                false => value.encode_value(&mut budget),
            }?
            .0,
        );
    }
    Ok(())
}

/// Decodes into reserved native storage without publishing partially checked data.
/// The caller keeps the complete wire slice readable throughout this conversion.
#[cfg(feature = "alloc")]
unsafe fn decode_buffer_elements<T: ComInlineValue>(
    elements: &[T::Wire],
    result: &mut Vec<T>,
    mut budget: ValueBudget,
    prechecked: bool,
) -> Result<(), NcStatus> {
    assert!(result.capacity() >= elements.len(), "output value capacity was not prepared");
    result.clear();
    for element in elements {
        result.push(match prechecked {
            // SAFETY: initialized wire elements and their prepaid recursive costs
            // satisfy the codec contract; restricted bits still require validation.
            true => unsafe { T::decode_prechecked(*element) },
            false => unsafe { T::decode_value(*element, &mut budget) },
        }?);
    }
    Ok(())
}

/// Encodes an initialized buffer into raw elements without assuming Rust layout.
#[cfg(feature = "alloc")]
pub fn encode_value_buffer<T: ComInlineValue>(values: &[T]) -> Result<Vec<T::Wire>, NcStatus> {
    let (budget, prechecked) = buffer_budget::<T, T::Wire>(values.len())?;
    let mut result = reserve_value_buffer(values.len())?;
    encode_buffer_elements(values, &mut result, budget, prechecked)?;
    Ok(result)
}

/// Reuses a prepared wire buffer; only codec validation/accounting can fail.
#[cfg(feature = "alloc")]
pub(crate) fn encode_value_buffer_into<T: ComInlineValue>(values: &[T], result: &mut Vec<T::Wire>) -> Result<(), NcStatus> {
    let (budget, prechecked) = buffer_budget::<T, T::Wire>(values.len())?;
    encode_buffer_elements(values, result, budget, prechecked)
}

/// Validates every raw element before exposing an application buffer.
///
/// # Safety
/// The pointer addresses len initialized raw elements, live for this call.
#[cfg(feature = "alloc")]
pub unsafe fn decode_value_buffer<T: ComInlineValue>(pointer: *const T::Wire, len: u64) -> Result<Vec<T>, NcStatus> {
    let count = usize::try_from(len).map_err(|_| NC_E_OUT_OF_RANGE)?;
    let (budget, prechecked) = buffer_budget::<T, T>(count)?;
    // SAFETY: the raw producer supplies live initialized storage for this call.
    let elements = unsafe { ValueSlice { pointer, len }.elements() }?;
    let mut result = reserve_value_buffer(count)?;
    // SAFETY: the validated slice borrows the same live raw elements.
    unsafe { decode_buffer_elements(elements, &mut result, budget, prechecked) }?;
    Ok(result)
}

/// Reuses native scratch so acquiring output cannot allocate a replacement vector.
///
/// # Safety
/// The wire elements must satisfy the same live-storage contract as decode_value_buffer.
#[cfg(feature = "alloc")]
pub(crate) unsafe fn decode_value_buffer_into<T: ComInlineValue>(pointer: *const T::Wire, len: u64, result: &mut Vec<T>) -> Result<(), NcStatus> {
    let count = usize::try_from(len).map_err(|_| NC_E_OUT_OF_RANGE)?;
    let (budget, prechecked) = buffer_budget::<T, T>(count)?;
    // SAFETY: the raw producer supplies live initialized storage for this call.
    let elements = unsafe { ValueSlice { pointer, len }.elements() }?;
    // SAFETY: decoding checks restricted contents before constructing any T.
    unsafe { decode_buffer_elements(elements, result, budget, prechecked) }
}

// SAFETY: unit has a concrete byte representation; only its sole value is accepted.
unsafe impl ComValue for () {
    const FIXED_COST: Option<ValueCost> = Some(ValueCost::SCALAR);
    type Wire = u8;
    type Storage = ();
    fn encode_value(&self, budget: &mut ValueBudget) -> Result<(u8, ()), NcStatus> {
        budget.value(|_| Ok((0, ())))
    }
    /// Counts this leaf and validates its raw byte before constructing a Rust value.
    unsafe fn decode_value(wire: u8, budget: &mut ValueBudget) -> Result<Self, NcStatus> {
        budget.value(|_| unsafe { Self::decode_prechecked(wire) })
    }
    /// Encodes the sole active leaf after its enclosing resource check.
    unsafe fn encode_prechecked(&self) -> Result<(u8, ()), NcStatus> {
        Ok((0, ()))
    }
    /// Rejects invalid raw bytes even when no budget bookkeeping is needed.
    unsafe fn decode_prechecked(wire: u8) -> Result<Self, NcStatus> {
        match wire {
            0 => Ok(()),
            _ => Err(NC_E_INVALID_ARG),
        }
    }
}
// SAFETY: the unit byte is self-contained.
unsafe impl ComInlineValue for () {}
