//! Typed NCB serialization of explicit persistent values.
//!
//! Implementations write typed members in declaration order. They never inspect
//! object memory, pointer fields, or ABI padding. Begin/end operations allow a
//! bounded writer to stream nested values without an erased intermediate tree.

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};

/// Writes an explicit value schema through the bounded NCB writer.
pub trait ComSerialize {
    /// Emits all members of this value, preserving the declared nesting order.
    fn com_serialize(&self, serializer: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error>;
}

impl ComSerialize for () {
    /// Unit has no members but remains a distinct serializer operation.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        s.serialize_unit()
    }
}

impl ComSerialize for crate::NcStatus {
    /// Persists the exact signed code, including application extension codes.
    fn com_serialize(&self, serializer: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        serializer.serialize_i32(self.code())
    }
}

impl ComSerialize for crate::NcGuid {
    /// Persists the canonical UUID byte sequence, never the platform's struct layout.
    fn com_serialize(&self, serializer: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        self.to_uuid_bytes().com_serialize(serializer)
    }
}

/// Generates scalar forwarding without a dynamically typed value wrapper.
macro_rules! primitive_serialize {
    ($($ty:ty => $method:ident),* $(,)?) => { $(
        impl ComSerialize for $ty {
            /// Preserves this scalar's exact declared type.
            fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> { s.$method(*self) }
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
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        (**self).com_serialize(s)
    }
}
impl ComSerialize for str {
    /// Writes UTF-8 bytes through the NCB string operation.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        s.serialize_string(self)
    }
}
impl<T: ComSerialize> ComSerialize for Option<T> {
    /// Records absence independently from a present default value.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        s.serialize_option(self.as_ref())
    }
}
impl<T: ComSerialize> ComSerialize for [T] {
    /// Streams elements without materializing a second collection.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        s.begin_sequence(self.len())?;
        for value in self {
            value.com_serialize(s)?;
        }
        s.end_sequence()
    }
}
impl<T: ComSerialize, const N: usize> ComSerialize for [T; N] {
    /// Uses the same sequence representation as a slice.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        self.as_slice().com_serialize(s)
    }
}
#[cfg(feature = "alloc")]
impl ComSerialize for String {
    /// Encodes text, never the String allocation header.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        self.as_str().com_serialize(s)
    }
}
#[cfg(feature = "alloc")]
impl<T: ComSerialize> ComSerialize for Vec<T> {
    /// Encodes vector contents in order.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        self.as_slice().com_serialize(s)
    }
}
#[cfg(feature = "alloc")]
impl<T: ComSerialize + ?Sized> ComSerialize for Box<T> {
    /// Serializes the owned value rather than the owning pointer.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        (**self).com_serialize(s)
    }
}
#[cfg(feature = "alloc")]
impl<K: ComSerialize + Ord, V: ComSerialize> ComSerialize for BTreeMap<K, V> {
    /// Ordered keys make binary snapshots deterministic.
    fn com_serialize(&self, s: &mut crate::binary::Writer<'_>) -> Result<(), crate::binary::Error> {
        s.begin_map(self.len())?;
        for (key, value) in self {
            key.com_serialize(s)?;
            value.com_serialize(s)?;
        }
        s.end_map()
    }
}
