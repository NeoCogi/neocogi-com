//! Typed value construction through COM format objects.
//!
//! Type implementations construct fields directly through a borrowed COM format
//! source. Names, scalars and container operations cross the format boundary;
//! application values remain statically typed in their owning binary.

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use crate::{NcStatus, serialization::Deserializer};

/// Constructs a concrete value from its selected format representation.
///
/// Deserializer::read enters the value's budget before invoking this method. Implementors
/// use reader.read() for each child, or delegate directly for a transparent scalar
/// wrapper. Format framing is handled by the record, sequence and variant methods.
/// Any error aborts the document; partially acquired fields drop normally.
pub trait ComDeserialize: Sized {
    /// Reads this already-entered value; complete documents use Deserializer::deserialize.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus>;
}

impl ComDeserialize for crate::NcStatus {
    /// Preserves the signed status as one scalar value without an extra depth level.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        i32::com_deserialize(reader).map(Self::from_code)
    }
}
impl ComDeserialize for crate::NcGuid {
    /// Reads exactly sixteen canonical UUID bytes with the existing sequence budget.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        <[u8; 16]>::com_deserialize(reader).map(Self::from_uuid_bytes)
    }
}

/// Keeps primitive Rust types attached to their explicit NCB width and validation.
macro_rules! primitive_deserialize {
    ($($ty:ty => $method:ident),* $(,)?) => { $(
        impl ComDeserialize for $ty {
            /// Acquires the scalar representation inside its existing value scope.
            fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
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
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        reader.deserialize_string()
    }
}
impl<T: ComDeserialize> ComDeserialize for Option<T> {
    /// The presence tag is framing; only Some opens a child value scope.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        reader.deserialize_option()
    }
}
impl<T: ComDeserialize, const N: usize> ComDeserialize for [T; N] {
    /// Requires exactly N elements and drops initialized slots on every error.
    /// Length admission precedes child construction; one guard owns the prefix.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        reader.sequence(|sequence| {
            if sequence.remaining() != N {
                return Err(sequence.error("invalid sequence length"));
            }
            crate::array::try_from_fn(|_| sequence.read_next()?.ok_or_else(|| sequence.error("invalid sequence length")))
        })
    }
}
#[cfg(feature = "alloc")]
impl<T: ComDeserialize> ComDeserialize for Vec<T> {
    /// Bounds eager capacity before acquiring elements, then grows with validated values.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        reader.sequence(|sequence| sequence.read_vec(|| crate::NC_E_OUT_OF_MEMORY))
    }
}
#[cfg(feature = "alloc")]
impl<T: ComDeserialize> ComDeserialize for Box<[T]> {
    /// Shares the exact vector construction and budget without another value level.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        Vec::<T>::com_deserialize(reader).map(Vec::into_boxed_slice)
    }
}
#[cfg(feature = "alloc")]
impl<K: ComDeserialize + Ord, V: ComDeserialize> ComDeserialize for BTreeMap<K, V> {
    /// Rejects duplicate keys instead of silently replacing an earlier value.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        reader.map()
    }
}
