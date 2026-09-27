//! Typed call guards for checked buffers, borrowing wire-identical values in place.
//!
//! The unsafe ComValue promise permits the direct path only when every bit
//! pattern is valid and conversion is exactly identity. Booleans, enums, and
//! aggregates containing restricted leaves always use checked temporary storage.
#[cfg(not(feature = "alloc"))]
use crate::NC_E_OUT_OF_MEMORY;
use crate::{ComInlineValue, NcCopyFromBufferOffset, NcCopyToBufferOffset, NcStatus, ValueSlice};
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// Prepared readable argument with its original logical offset.
pub struct EncodedInput<'a, T: ComInlineValue> {
    /// Logical source offset, independent of the temporary allocation.
    offset: u64,
    /// Keeps either the original values or their converted wire storage live.
    storage: InputWire<'a, T>,
}
/// Statically typed alternatives for readable producer storage.
enum InputWire<'a, T: ComInlineValue> {
    /// Borrowing is justified by the recursive identity-conversion promise.
    Direct(&'a [T]),
    /// Restricted fields require a separately converted raw element array.
    #[cfg(feature = "alloc")]
    Converted(Vec<T::Wire>),
}
impl<'a, T: ComInlineValue> EncodedInput<'a, T> {
    /// Retains a direct borrow or prepares checked raw elements before the call.
    pub fn new(value: NcCopyFromBufferOffset<'a, T>) -> Result<Self, NcStatus> {
        let storage = match T::BORROW_WIRE {
            true => InputWire::Direct(value.as_slice()),
            false => {
                #[cfg(feature = "alloc")]
                {
                    InputWire::Converted(crate::encode_value_buffer(value.as_slice())?)
                }
                #[cfg(not(feature = "alloc"))]
                {
                    return Err(NC_E_OUT_OF_MEMORY);
                }
            }
        };
        Ok(Self { offset: value.offset(), storage })
    }
    /// Preserves the authored buffer operation's offset.
    pub fn offset(&self) -> u64 {
        self.offset
    }
    /// Returns the retained initialized element count.
    pub fn len(&self) -> u64 {
        match &self.storage {
            InputWire::Direct(values) => values.len() as u64,
            #[cfg(feature = "alloc")]
            InputWire::Converted(values) => values.len() as u64,
        }
    }
    /// Returns true when no element is addressed.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Borrows only raw representations with a justified identical-layout cast.
    pub fn as_ptr(&self) -> *const T::Wire {
        match &self.storage {
            InputWire::Direct(values) => values.as_ptr().cast(),
            #[cfg(feature = "alloc")]
            InputWire::Converted(values) => values.as_ptr(),
        }
    }
}

/// Exclusive writable argument; retains the destination until checked acquisition.
pub struct EncodedOutput<'a, T: ComInlineValue> {
    /// Logical destination offset understood by the receiving method.
    offset: u64,
    /// Original application destination, never aliased by another safe guard.
    target: &'a mut [T],
    /// Absent for wire-identical storage; present for restricted raw fields.
    #[cfg(feature = "alloc")]
    wires: Option<Vec<T::Wire>>,
}
impl<'a, T: ComInlineValue> EncodedOutput<'a, T> {
    /// Prepares initialized in/out storage while preserving the exclusive borrow.
    pub fn new(value: NcCopyToBufferOffset<'a, T>) -> Result<Self, NcStatus> {
        let offset = value.offset();
        let target = value.into_mut_slice();
        #[cfg(feature = "alloc")]
        let wires = match T::BORROW_WIRE {
            true => None,
            false => Some(crate::encode_value_buffer(target)?),
        };
        #[cfg(not(feature = "alloc"))]
        if !T::BORROW_WIRE {
            return Err(NC_E_OUT_OF_MEMORY);
        }
        Ok(Self {
            offset,
            target,
            #[cfg(feature = "alloc")]
            wires,
        })
    }
    /// Preserves the logical destination offset.
    pub fn offset(&self) -> u64 {
        self.offset
    }
    /// Returns the initialized in/out element count.
    pub fn len(&self) -> u64 {
        self.target.len() as u64
    }
    /// Returns true when no element is addressed.
    pub fn is_empty(&self) -> bool {
        self.target.is_empty()
    }
    /// Obtains a raw pointer through this guard's exclusive destination borrow.
    pub fn as_mut_ptr(&mut self) -> *mut T::Wire {
        #[cfg(feature = "alloc")]
        if let Some(wires) = &mut self.wires {
            return wires.as_mut_ptr();
        }
        self.target.as_mut_ptr().cast()
    }
    /// Validates converted output before replacing any application element.
    pub fn finish(self) -> Result<(), NcStatus> {
        #[cfg(feature = "alloc")]
        if let Some(wires) = self.wires {
            // SAFETY: a successful raw call leaves initialized elements. Their
            // restricted contents are checked before any destination is changed.
            let values = unsafe { crate::decode_value_buffer::<T>(wires.as_ptr(), wires.len() as u64) }?;
            self.target.copy_from_slice(&values);
        }
        Ok(())
    }
}

/// Acquired readable storage supplied to an application implementation.
pub enum DecodedInput<'a, T: ComInlineValue> {
    /// Raw storage is layout-identical and all initialized contents are valid.
    Direct(&'a [T]),
    /// Restricted fields have been recursively validated and acquired.
    #[cfg(feature = "alloc")]
    Converted(Vec<T>),
}
impl<'a, T: ComInlineValue> DecodedInput<'a, T> {
    /// Checks pointer shape and recursively validates any restricted elements.
    ///
    /// # Safety
    /// The raw caller keeps initialized elements readable for this call.
    pub unsafe fn from_raw(pointer: *const T::Wire, len: u64) -> Result<Self, NcStatus> {
        match T::BORROW_WIRE {
            true => {
                let raw = unsafe { ValueSlice { pointer, len }.elements() }?;
                // SAFETY: the unsafe codec explicitly proves equal layout and
                // validity of every initialized bit pattern without conversion.
                Ok(Self::Direct(unsafe { core::slice::from_raw_parts(raw.as_ptr().cast(), raw.len()) }))
            }
            false => {
                #[cfg(feature = "alloc")]
                {
                    Ok(Self::Converted(unsafe { crate::decode_value_buffer::<T>(pointer, len) }?))
                }
                #[cfg(not(feature = "alloc"))]
                {
                    Err(NC_E_OUT_OF_MEMORY)
                }
            }
        }
    }
    /// Borrows validated application values for the implementation method.
    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::Direct(values) => values,
            #[cfg(feature = "alloc")]
            Self::Converted(values) => values,
        }
    }
}

/// Acquired writable storage, retaining the caller's raw destination for commit.
pub struct DecodedOutput<'a, T: ComInlineValue> {
    /// Destination for converted fields; direct storage already addresses it.
    #[cfg(feature = "alloc")]
    pointer: *mut T::Wire,
    /// Exclusive application storage supplied to the implementation.
    storage: OutputValues<'a, T>,
}
/// Mutually exclusive direct-borrow and converted-ownership states.
enum OutputValues<'a, T: ComInlineValue> {
    /// The codec proves every raw element can be borrowed as an application value.
    Direct(&'a mut [T]),
    /// Restricted elements remain separate until successful conversion back.
    #[cfg(feature = "alloc")]
    Converted(Vec<T>),
}
impl<'a, T: ComInlineValue> DecodedOutput<'a, T> {
    /// Acquires an exclusive application buffer after checking active raw contents.
    ///
    /// # Safety
    /// The raw caller provides initialized, exclusively writable elements for this call.
    pub unsafe fn from_raw(pointer: *mut T::Wire, len: u64) -> Result<Self, NcStatus> {
        let storage = match T::BORROW_WIRE {
            true => {
                let count = unsafe { ValueSlice { pointer, len }.elements() }?.len();
                let pointer = match count {
                    0 => core::ptr::NonNull::<T>::dangling().as_ptr(),
                    _ => pointer.cast(),
                };
                // SAFETY: shape was checked above, the caller provides exclusive
                // access, and the codec proves identical layout and validity.
                OutputValues::Direct(unsafe { core::slice::from_raw_parts_mut(pointer, count) })
            }
            false => {
                #[cfg(feature = "alloc")]
                {
                    OutputValues::Converted(unsafe { crate::decode_value_buffer::<T>(pointer, len) }?)
                }
                #[cfg(not(feature = "alloc"))]
                {
                    return Err(NC_E_OUT_OF_MEMORY);
                }
            }
        };
        Ok(Self {
            #[cfg(feature = "alloc")]
            pointer,
            storage,
        })
    }
    /// Exposes only validated values through the exclusive application borrow.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match &mut self.storage {
            OutputValues::Direct(values) => values,
            #[cfg(feature = "alloc")]
            OutputValues::Converted(values) => values,
        }
    }
    /// Converts all restricted elements before copying any successful output back.
    pub fn finish(self) -> Result<(), NcStatus> {
        #[cfg(feature = "alloc")]
        if let OutputValues::Converted(values) = self.storage {
            let wires = crate::encode_value_buffer(&values)?;
            if !wires.is_empty() {
                // SAFETY: construction retains the caller's exclusive raw region
                // with this exact length; the separately allocated source cannot overlap.
                unsafe {
                    core::ptr::copy_nonoverlapping(wires.as_ptr(), self.pointer, wires.len());
                }
            }
        }
        Ok(())
    }
}
