//! Typed NCB value construction in declaration order.
//!
//! Persistence has one binary format and one concrete bounded cursor. Type
//! implementations construct fields directly; they carry no format-adapter
//! protocol, arbitrary field indices, names, duplicate-field flags or type erasure.

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use crate::binary::{Error, Reader};

/// Constructs a concrete value from its NCB initialization representation.
///
/// Reader::read enters the value's budget before invoking this method. Implementors
/// use reader.read() for each child, or delegate directly for a transparent scalar
/// wrapper. Format framing is handled by the record, sequence and variant methods.
/// Any error aborts the document; partially acquired fields drop normally.
pub trait ComDeserialize: Sized {
    /// Reads this already-entered value; complete documents use binary::from_slice.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error>;
}

impl ComDeserialize for crate::NcStatus {
    /// Preserves the signed status as one scalar value without an extra depth level.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
        i32::com_deserialize(reader).map(Self::from_code)
    }
}
impl ComDeserialize for crate::NcGuid {
    /// Reads exactly sixteen canonical UUID bytes with the existing sequence budget.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
        <[u8; 16]>::com_deserialize(reader).map(Self::from_uuid_bytes)
    }
}

/// Keeps primitive Rust types attached to their explicit NCB width and validation.
macro_rules! primitive_deserialize {
    ($($ty:ty => $method:ident),* $(,)?) => { $(
        impl ComDeserialize for $ty {
            /// Acquires the scalar representation inside its existing value scope.
            fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
                reader.$method()
            }
        }
    )* };
}
primitive_deserialize!(
    () => deserialize_unit, bool => deserialize_bool,
    u8 => deserialize_u8, u16 => deserialize_u16, u32 => deserialize_u32,
    u64 => deserialize_u64, usize => deserialize_usize,
    i8 => deserialize_i8, i16 => deserialize_i16, i32 => deserialize_i32,
    i64 => deserialize_i64, isize => deserialize_isize,
    f32 => deserialize_f32, f64 => deserialize_f64,
);

#[cfg(feature = "alloc")]
impl ComDeserialize for String {
    /// Acquires one validated UTF-8 string under the document's byte limit.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.deserialize_string()
    }
}
impl<T: ComDeserialize> ComDeserialize for Option<T> {
    /// The presence tag is framing; only Some opens a child value scope.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.deserialize_option()
    }
}
impl<T: ComDeserialize, const N: usize> ComDeserialize for [T; N] {
    /// Requires exactly N elements and drops initialized slots on every error.
    /// Option slots avoid unsafe uninitialized storage for non-Copy elements.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.sequence(|sequence| {
            let mut values: [Option<T>; N] = core::array::from_fn(|_| None);
            for slot in &mut values {
                *slot = Some(sequence.read_next()?.ok_or_else(|| sequence.error("invalid sequence length"))?);
            }
            Ok(values.map(|value| value.expect("every fixed-array slot was acquired")))
        })
    }
}
#[cfg(feature = "alloc")]
impl<T: ComDeserialize> ComDeserialize for Vec<T> {
    /// Bounds eager capacity before acquiring elements, then grows with validated values.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.sequence(|sequence| {
            // A count is not a size proof for large records. Retain the 4 KiB
            // eager-capacity ceiling; the cursor already checked element limits.
            let eager = 4096 / core::mem::size_of::<T>().max(1);
            let mut values = Vec::with_capacity(sequence.remaining().min(eager));
            while let Some(value) = sequence.read_next()? {
                values.push(value);
            }
            Ok(values)
        })
    }
}
#[cfg(feature = "alloc")]
impl<T: ComDeserialize> ComDeserialize for Box<[T]> {
    /// Shares the exact vector construction and budget without another value level.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
        Vec::<T>::com_deserialize(reader).map(Vec::into_boxed_slice)
    }
}
#[cfg(feature = "alloc")]
impl<K: ComDeserialize + Ord, V: ComDeserialize> ComDeserialize for BTreeMap<K, V> {
    /// Rejects duplicate keys instead of silently replacing an earlier value.
    fn com_deserialize(reader: &mut Reader<'_>) -> Result<Self, Error> {
        reader.map()
    }
}
