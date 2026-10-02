//! Typed call guards for checked buffers, borrowing wire-identical values in place.
//!
//! The unsafe ComValue promise permits the direct path only when every bit
//! pattern is valid and conversion is exactly identity. Booleans, enums, and
//! aggregates containing restricted leaves always use checked temporary storage.
#[cfg(not(feature = "alloc"))]
use crate::NC_E_OUT_OF_MEMORY;
use crate::{ComInlineValue, NcCopyFromBufferOffset, NcStatus, ValueSlice};
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
