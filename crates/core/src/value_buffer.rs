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

/// Prepared converted output storage shared by the two sides of a call.
/// One vector initially holds input elements and the other reserves completion
/// capacity. Conversion fills the reserved vector; publication never allocates.
#[cfg(feature = "alloc")]
struct ConvertedOutput<T: ComInlineValue> {
    /// Validated native elements, separate from caller destinations until publication.
    values: Vec<T>,
    /// Initialized wire elements, separate from provider destinations until publication.
    wires: Vec<T::Wire>,
}

/// Exclusive writable argument; retains the destination until checked acquisition.
pub struct EncodedOutput<'a, T: ComInlineValue> {
    /// Logical destination offset understood by the receiving method.
    offset: u64,
    /// Original application destination, never aliased by another safe guard.
    target: &'a mut [T],
    /// Direct buffers need no scratch; converted buffers reserve both directions.
    #[cfg(feature = "alloc")]
    converted: Option<ConvertedOutput<T>>,
}
impl<'a, T: ComInlineValue> EncodedOutput<'a, T> {
    /// Prepares initialized in/out storage while preserving the exclusive borrow.
    pub fn new(value: NcCopyToBufferOffset<'a, T>) -> Result<Self, NcStatus> {
        let offset = value.offset();
        let target = value.into_mut_slice();
        #[cfg(feature = "alloc")]
        let converted = match T::BORROW_WIRE {
            true => None,
            false => Some(ConvertedOutput {
                wires: crate::encode_value_buffer(target)?,
                values: crate::value::reserve_value_buffer(target.len())?,
            }),
        };
        #[cfg(not(feature = "alloc"))]
        if !T::BORROW_WIRE {
            return Err(NC_E_OUT_OF_MEMORY);
        }
        Ok(Self {
            offset,
            target,
            #[cfg(feature = "alloc")]
            converted,
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
        if let Some(converted) = &mut self.converted {
            return converted.wires.as_mut_ptr();
        }
        self.target.as_mut_ptr().cast()
    }
    /// Validates and converts output into reserved scratch without changing the
    /// application's destination. Generated calls convert every output first,
    /// then publish them together so a later invalid buffer cannot partially
    /// update an earlier converted destination. Direct buffers already alias it.
    pub fn convert_output(&mut self) -> Result<(), NcStatus> {
        #[cfg(feature = "alloc")]
        if let Some(converted) = &mut self.converted {
            // SAFETY: successful raw calls leave initialized wire elements;
            // their restricted contents are checked before any T is published.
            unsafe { crate::value::decode_value_buffer_into::<T>(converted.wires.as_ptr(), converted.wires.len() as u64, &mut converted.values) }?;
        }
        Ok(())
    }
    /// Publishes completely converted elements without allocating or decoding.
    ///
    /// # Panics
    /// Nonempty converted output must have passed convert_output first. A failed
    /// conversion leaves fewer elements than the destination and cannot publish.
    pub fn publish(self) {
        #[cfg(feature = "alloc")]
        if let Some(converted) = self.converted {
            self.target.copy_from_slice(&converted.values);
        }
    }
    /// Converts and publishes one buffer using storage prepared before dispatch.
    /// Multi-buffer generated calls separate these phases across all outputs.
    pub fn finish(mut self) -> Result<(), NcStatus> {
        self.convert_output()?;
        self.publish();
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
    /// Native values and reserved wire scratch remain private until publication.
    #[cfg(feature = "alloc")]
    Converted(ConvertedOutput<T>),
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
                    // Decode the incoming values before reserving output scratch.
                    // Both fallible allocations precede the implementation call.
                    let values = unsafe { crate::decode_value_buffer::<T>(pointer, len) }?;
                    let wires = crate::value::reserve_value_buffer(values.len())?;
                    OutputValues::Converted(ConvertedOutput { values, wires })
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
            OutputValues::Converted(converted) => &mut converted.values,
        }
    }
    /// Converts into reserved wire scratch without publishing any raw output.
    /// Direct buffers retain their existing in-place behavior. Converted buffers
    /// can be validated as a group before any of their destinations are written.
    pub fn convert_output(&mut self) -> Result<(), NcStatus> {
        #[cfg(feature = "alloc")]
        if let OutputValues::Converted(converted) = &mut self.storage {
            crate::value::encode_value_buffer_into(&converted.values, &mut converted.wires)?;
        }
        Ok(())
    }
    /// Publishes a completely converted wire buffer without further resource work.
    ///
    /// # Panics
    /// Nonempty converted storage must have passed convert_output first. Length
    /// equality prevents an incomplete conversion from publishing a partial buffer.
    pub fn publish(self) {
        #[cfg(feature = "alloc")]
        if let OutputValues::Converted(converted) = self.storage {
            assert_eq!(converted.wires.len(), converted.values.len(), "output wire was not completely converted");
            if !converted.wires.is_empty() {
                // SAFETY: construction retains the caller's exclusive raw region
                // at this length; separately allocated scratch cannot overlap it.
                unsafe {
                    core::ptr::copy_nonoverlapping(converted.wires.as_ptr(), self.pointer, converted.wires.len());
                }
            }
        }
    }
    /// Converts and publishes one prepared buffer; group callers split the phases.
    pub fn finish(mut self) -> Result<(), NcStatus> {
        self.convert_output()?;
        self.publish();
        Ok(())
    }
}
