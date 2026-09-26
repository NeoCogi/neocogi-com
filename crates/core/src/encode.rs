//! Format-neutral serialization of explicit persistent values.
//!
//! Implementations write typed members in declaration order. They never inspect
//! object memory, pointer fields, or ABI padding. Begin/end operations allow a
//! format adapter to stream nested values without an erased intermediate tree.

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};

/// Writes an explicit value schema through a format-specific writer.
pub trait ComSerialize {
    /// Emits all members of this value, preserving the declared nesting order.
    fn com_serialize<S: ComSerializer>(&self, serializer: &mut S) -> Result<(), S::Error>;
}

/// Streaming counterpart to ComDeserializer; all calls remain inside one binary.
///
/// Every begin must have a matching end. Counts describe actual fields, elements,
/// or map pairs. Implementations must not serialize process-local addresses.
/// A format error aborts the encoding: propagate it rather than continuing with
/// a partially written value or an unfinished container.
pub trait ComSerializer: Sized {
    /// Format-specific failure, including byte or nesting limits.
    type Error;
    /// Writes the unit value.
    fn serialize_unit(&mut self) -> Result<(), Self::Error>;
    /// Writes a bool value without relying on its native memory layout.
    fn serialize_bool(&mut self, value: bool) -> Result<(), Self::Error>;
    /// Writes a u8 value without relying on its native memory layout.
    fn serialize_u8(&mut self, value: u8) -> Result<(), Self::Error>;
    /// Writes a u16 value without relying on its native memory layout.
    fn serialize_u16(&mut self, value: u16) -> Result<(), Self::Error>;
    /// Writes a u32 value without relying on its native memory layout.
    fn serialize_u32(&mut self, value: u32) -> Result<(), Self::Error>;
    /// Writes a u64 value without relying on its native memory layout.
    fn serialize_u64(&mut self, value: u64) -> Result<(), Self::Error>;
    /// Writes a usize value without relying on its native memory layout.
    fn serialize_usize(&mut self, value: usize) -> Result<(), Self::Error>;
    /// Writes a i8 value without relying on its native memory layout.
    fn serialize_i8(&mut self, value: i8) -> Result<(), Self::Error>;
    /// Writes a i16 value without relying on its native memory layout.
    fn serialize_i16(&mut self, value: i16) -> Result<(), Self::Error>;
    /// Writes a i32 value without relying on its native memory layout.
    fn serialize_i32(&mut self, value: i32) -> Result<(), Self::Error>;
    /// Writes a i64 value without relying on its native memory layout.
    fn serialize_i64(&mut self, value: i64) -> Result<(), Self::Error>;
    /// Writes a isize value without relying on its native memory layout.
    fn serialize_isize(&mut self, value: isize) -> Result<(), Self::Error>;
    /// Writes a f32 value without relying on its native memory layout.
    fn serialize_f32(&mut self, value: f32) -> Result<(), Self::Error>;
    /// Writes a f64 value without relying on its native memory layout.
    fn serialize_f64(&mut self, value: f64) -> Result<(), Self::Error>;
    /// Writes a UTF-8 string; adapters own its length encoding.
    fn serialize_string(&mut self, value: &str) -> Result<(), Self::Error>;
    /// Writes presence followed by the present value, if any.
    fn serialize_option<T: ComSerialize>(&mut self, value: Option<&T>) -> Result<(), Self::Error>;
    /// Begins a named record with exactly the declared number of fields.
    fn begin_record(&mut self, name: &'static str, len: usize) -> Result<(), Self::Error>;
    /// Identifies the next record field; its value immediately follows.
    fn field(&mut self, name: &'static str) -> Result<(), Self::Error>;
    /// Ends the current record.
    fn end_record(&mut self) -> Result<(), Self::Error>;
    /// Begins an ordered collection of exactly len elements.
    fn begin_sequence(&mut self, len: usize) -> Result<(), Self::Error>;
    /// Ends the current sequence.
    fn end_sequence(&mut self) -> Result<(), Self::Error>;
    /// Begins a map of len pairs, written as alternating keys and values.
    fn begin_map(&mut self, len: usize) -> Result<(), Self::Error>;
    /// Ends the current map.
    fn end_map(&mut self) -> Result<(), Self::Error>;
    /// Begins a variant by declaration ordinal, followed by its typed payload.
    /// A unit case has no payload operation; a newtype writes its value, while
    /// tuple/record cases open a sequence/record containing their members.
    fn begin_variant(&mut self, name: &'static str, index: u32, variant: &'static str) -> Result<(), Self::Error>;
    /// Ends the current variant.
    fn end_variant(&mut self) -> Result<(), Self::Error>;
}

impl ComSerialize for () {
    /// Unit has no members but remains a distinct serializer operation.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        s.serialize_unit()
    }
}

/// Generates scalar forwarding without a dynamically typed value wrapper.
macro_rules! primitive_serialize {
    ($($ty:ty => $method:ident),* $(,)?) => { $(
        impl ComSerialize for $ty {
            /// Preserves this scalar's exact declared type.
            fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> { s.$method(*self) }
        }
    )* };
}
primitive_serialize!(
    bool => serialize_bool,
    u8 => serialize_u8,
    u16 => serialize_u16,
    u32 => serialize_u32,
    u64 => serialize_u64,
    usize => serialize_usize,
    i8 => serialize_i8,
    i16 => serialize_i16,
    i32 => serialize_i32,
    i64 => serialize_i64,
    isize => serialize_isize,
    f32 => serialize_f32,
    f64 => serialize_f64,
);

impl<T: ComSerialize + ?Sized> ComSerialize for &T {
    /// Borrows the referenced value without encoding the reference address.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        (**self).com_serialize(s)
    }
}
impl ComSerialize for str {
    /// Writes UTF-8 bytes through the format's string operation.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        s.serialize_string(self)
    }
}
impl<T: ComSerialize> ComSerialize for Option<T> {
    /// Records absence independently from a present default value.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        s.serialize_option(self.as_ref())
    }
}
impl<T: ComSerialize> ComSerialize for [T] {
    /// Streams elements without materializing a second collection.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        s.begin_sequence(self.len())?;
        for value in self {
            value.com_serialize(s)?;
        }
        s.end_sequence()
    }
}
impl<T: ComSerialize, const N: usize> ComSerialize for [T; N] {
    /// Uses the same sequence representation as a slice.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        self.as_slice().com_serialize(s)
    }
}
#[cfg(feature = "alloc")]
impl ComSerialize for String {
    /// Encodes text, never the String allocation header.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        self.as_str().com_serialize(s)
    }
}
#[cfg(feature = "alloc")]
impl<T: ComSerialize> ComSerialize for Vec<T> {
    /// Encodes vector contents in order.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        self.as_slice().com_serialize(s)
    }
}
#[cfg(feature = "alloc")]
impl<T: ComSerialize + ?Sized> ComSerialize for Box<T> {
    /// Serializes the owned value rather than the owning pointer.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        (**self).com_serialize(s)
    }
}
#[cfg(feature = "alloc")]
impl<K: ComSerialize + Ord, V: ComSerialize> ComSerialize for BTreeMap<K, V> {
    /// Ordered keys make binary snapshots deterministic.
    fn com_serialize<S: ComSerializer>(&self, s: &mut S) -> Result<(), S::Error> {
        s.begin_map(self.len())?;
        for (key, value) in self {
            key.com_serialize(s)?;
            value.com_serialize(s)?;
        }
        s.end_map()
    }
}
