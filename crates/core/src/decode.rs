//! Format-neutral, statically typed value decoding.
//!
//! [`ComDeserialize`] is deliberately independent from JSON and from any
//! serialization framework. Implementations recursively request concrete Rust
//! types from a [`ComDeserializer`]; records, sequences, maps, and enums expose
//! only the small amount of structural access needed to construct those types.
//! No runtime type registry, `Any`, erased value tree, or field-offset mutation
//! participates in decoding.

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};

/// Constructs a concrete Rust value from a typed COM decoder.
///
/// Implementations should decode their members directly and return a fully
/// initialized value. Macro-generated implementations follow the same static
/// call graph as handwritten Rust code, so class-registry lookup is not needed
/// while a value is decoded.
pub trait ComDeserialize: Sized {
    /// Decodes `Self` from one format-specific decoder value.
    fn com_deserialize<'de, D>(decoder: D) -> Result<Self, D::Error>
    where
        D: ComDeserializer<'de>;
}

impl ComDeserialize for crate::NcStatus {
    /// Reconstructs every raw signed code without treating success as a decoding error.
    fn com_deserialize<'de, D: ComDeserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        i32::com_deserialize(decoder).map(Self::from_code)
    }
}

impl ComDeserialize for crate::NcGuid {
    /// Requires exactly sixteen canonical UUID bytes before constructing the value.
    fn com_deserialize<'de, D: ComDeserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        <[u8; 16]>::com_deserialize(decoder).map(Self::from_uuid_bytes)
    }
}

/// Supplies primitive values and structural inputs to [`ComDeserialize`].
///
/// A format adapter implements this trait once. The adapter remains responsible
/// for syntax, input lifetimes, diagnostics, and resource limits; application
/// and contract types remain unaware of that format.
pub trait ComDeserializer<'de>: Sized {
    /// Error reported by this decoder and all access objects it creates.
    type Error;

    /// Decodes the unit value.
    fn deserialize_unit(self) -> Result<(), Self::Error>;
    /// Decodes a Boolean value.
    fn deserialize_bool(self) -> Result<bool, Self::Error>;
    /// Decodes an unsigned 8-bit integer.
    fn deserialize_u8(self) -> Result<u8, Self::Error>;
    /// Decodes an unsigned 16-bit integer.
    fn deserialize_u16(self) -> Result<u16, Self::Error>;
    /// Decodes an unsigned 32-bit integer.
    fn deserialize_u32(self) -> Result<u32, Self::Error>;
    /// Decodes an unsigned 64-bit integer.
    fn deserialize_u64(self) -> Result<u64, Self::Error>;
    /// Decodes an unsigned pointer-sized integer.
    fn deserialize_usize(self) -> Result<usize, Self::Error>;
    /// Decodes a signed 8-bit integer.
    fn deserialize_i8(self) -> Result<i8, Self::Error>;
    /// Decodes a signed 16-bit integer.
    fn deserialize_i16(self) -> Result<i16, Self::Error>;
    /// Decodes a signed 32-bit integer.
    fn deserialize_i32(self) -> Result<i32, Self::Error>;
    /// Decodes a signed 64-bit integer.
    fn deserialize_i64(self) -> Result<i64, Self::Error>;
    /// Decodes a signed pointer-sized integer.
    fn deserialize_isize(self) -> Result<isize, Self::Error>;
    /// Decodes a 32-bit floating-point value.
    fn deserialize_f32(self) -> Result<f32, Self::Error>;
    /// Decodes a 64-bit floating-point value.
    fn deserialize_f64(self) -> Result<f64, Self::Error>;

    /// Decodes an owned UTF-8 string.
    #[cfg(feature = "alloc")]
    fn deserialize_string(self) -> Result<String, Self::Error>;

    /// Decodes an absent or present value without erasing its element type.
    fn deserialize_option<T>(self) -> Result<Option<T>, Self::Error>
    where
        T: ComDeserialize;

    /// Decodes a named record through its statically generated constructor.
    fn deserialize_record<T>(self) -> Result<T, Self::Error>
    where
        T: ComRecord;

    /// Decodes an ordered sequence through its concrete collection builder.
    fn deserialize_sequence<T>(self) -> Result<T, Self::Error>
    where
        T: ComSequence;

    /// Decodes a named enum through its statically generated variant switch.
    fn deserialize_enum<T>(self) -> Result<T, Self::Error>
    where
        T: ComEnum;

    /// Decodes a key/value map through its concrete collection builder.
    #[cfg(feature = "alloc")]
    fn deserialize_map<T>(self) -> Result<T, Self::Error>
    where
        T: ComMap;
}

/// Static construction recipe for a named record.
///
/// `#[com_struct(deserialize)]` and `#[derive(ComDeserialize)]` generate this
/// trait for records. Field names are source metadata used only by the input
/// adapter; field values are always requested as concrete Rust types.
pub trait ComRecord: ComDeserialize {
    /// Human-readable record name used by format diagnostics.
    const NAME: &'static str;
    /// Accepted source field names in declaration order.
    const FIELDS: &'static [&'static str];

    /// Reads fields and constructs the record.
    fn read_record<'de, A>(record: &mut A) -> Result<Self, A::Error>
    where
        A: ComRecordAccess<'de>;
}

/// Incremental access to one named record supplied by a format adapter.
pub trait ComRecordAccess<'de> {
    /// Error reported by this record input.
    type Error;

    /// Returns the index of the next known field, or `None` at the record end.
    ///
    /// The adapter must reject names absent from `fields` rather than silently
    /// discarding data. The returned index therefore always addresses `fields`.
    fn next_field(&mut self, fields: &'static [&'static str]) -> Result<Option<u32>, Self::Error>;

    /// Decodes the value associated with the field most recently returned by
    /// [`Self::next_field`].
    fn next_value<T>(&mut self) -> Result<T, Self::Error>
    where
        T: ComDeserialize;

    /// Reports a field that appeared more than once.
    fn duplicate_field(&self, field: &'static str) -> Self::Error;

    /// Reports a required field that did not appear.
    fn missing_field(&self, field: &'static str) -> Self::Error;

    /// Reports an out-of-range index returned by a defective format adapter.
    fn invalid_field_index(&self, index: u32) -> Self::Error;
}

/// Static construction recipe for a sequence or owned sequence container.
pub trait ComSequence: ComDeserialize {
    /// Reads elements in order and constructs the concrete sequence type.
    fn read_sequence<'de, A>(sequence: &mut A) -> Result<Self, A::Error>
    where
        A: ComSequenceAccess<'de>;
}

/// Incremental access to an ordered sequence supplied by a format adapter.
pub trait ComSequenceAccess<'de> {
    /// Error reported by this sequence input.
    type Error;

    /// Decodes the next element, or returns `None` at the sequence end.
    fn next_element<T>(&mut self) -> Result<Option<T>, Self::Error>
    where
        T: ComDeserialize;

    /// Returns a conservative element-count hint when the format provides one.
    fn size_hint(&self) -> Option<usize>;

    /// Returns the adapter-enforced maximum number of sequence elements.
    ///
    /// Owned containers use this value only to bound eager allocation. The
    /// adapter must also reject inputs that exceed the limit while iterating.
    fn element_limit(&self) -> usize;

    /// Confirms that no elements remain after `actual` values were decoded.
    ///
    /// Fixed arrays and heterogeneous tuples call this method after their last
    /// expected element. The adapter can discard one format value internally
    /// to distinguish an exact end from a longer sequence without inventing a
    /// public dynamically typed placeholder.
    fn finish(&mut self, actual: usize) -> Result<(), Self::Error>;

    /// Reports a sequence whose actual element count does not match a required
    /// fixed length.
    fn invalid_length(&self, actual: usize, expected: usize) -> Self::Error;
}

/// Static construction recipe for a named enum.
pub trait ComEnum: ComDeserialize {
    /// Human-readable enum name used by format diagnostics.
    const NAME: &'static str;
    /// Accepted source variant names in declaration order.
    const VARIANTS: &'static [&'static str];

    /// Constructs the enum variant selected by `variant`.
    fn read_variant<'de, A>(variant: u32, value: A) -> Result<Self, A::Error>
    where
        A: ComVariantAccess<'de>;
}

/// Access to the payload of one enum variant supplied by a format adapter.
pub trait ComVariantAccess<'de>: Sized {
    /// Error reported by this enum payload input.
    type Error;

    /// Confirms that a fieldless variant has no payload.
    fn unit(self) -> Result<(), Self::Error>;

    /// Decodes the single value of a newtype variant.
    fn newtype<T>(self) -> Result<T, Self::Error>
    where
        T: ComDeserialize;

    /// Decodes the ordered fields of a tuple variant.
    fn tuple<T>(self, length: usize) -> Result<T, Self::Error>
    where
        T: ComSequence;

    /// Decodes the named fields of a struct variant.
    fn record<T>(self) -> Result<T, Self::Error>
    where
        T: ComRecord;

    /// Reports an out-of-range variant index returned by a defective adapter.
    fn invalid_variant(self, variant: u32, expected: &'static [&'static str]) -> Self::Error;
}

/// Static construction recipe for an owned key/value map.
#[cfg(feature = "alloc")]
pub trait ComMap: ComDeserialize {
    /// Reads entries and constructs the concrete map type.
    fn read_map<'de, A>(map: &mut A) -> Result<Self, A::Error>
    where
        A: ComMapAccess<'de>;
}

/// Incremental access to an owned key/value map supplied by a format adapter.
#[cfg(feature = "alloc")]
pub trait ComMapAccess<'de> {
    /// Error reported by this map input.
    type Error;

    /// Decodes the next key, or returns `None` at the map end.
    fn next_key<K>(&mut self) -> Result<Option<K>, Self::Error>
    where
        K: ComDeserialize;

    /// Decodes the value paired with the key most recently returned by
    /// [`Self::next_key`].
    fn next_value<V>(&mut self) -> Result<V, Self::Error>
    where
        V: ComDeserialize;

    /// Reports a duplicate key rejected by the concrete map implementation.
    fn duplicate_key(&self) -> Self::Error;
}

macro_rules! primitive_deserialize {
    ($($ty:ty => $method:ident),* $(,)?) => {
        $(
            impl ComDeserialize for $ty {
                /// Delegates primitive parsing to the format adapter.
                fn com_deserialize<'de, D>(decoder: D) -> Result<Self, D::Error>
                where
                    D: ComDeserializer<'de>,
                {
                    decoder.$method()
                }
            }
        )*
    };
}

primitive_deserialize!(
    () => deserialize_unit,
    bool => deserialize_bool,
    u8 => deserialize_u8,
    u16 => deserialize_u16,
    u32 => deserialize_u32,
    u64 => deserialize_u64,
    usize => deserialize_usize,
    i8 => deserialize_i8,
    i16 => deserialize_i16,
    i32 => deserialize_i32,
    i64 => deserialize_i64,
    isize => deserialize_isize,
    f32 => deserialize_f32,
    f64 => deserialize_f64,
);

#[cfg(feature = "alloc")]
impl ComDeserialize for String {
    /// Delegates owned string parsing and its byte limit to the format adapter.
    fn com_deserialize<'de, D>(decoder: D) -> Result<Self, D::Error>
    where
        D: ComDeserializer<'de>,
    {
        decoder.deserialize_string()
    }
}

impl<T> ComDeserialize for Option<T>
where
    T: ComDeserialize,
{
    /// Preserves the concrete optional element type through recursive decoding.
    fn com_deserialize<'de, D>(decoder: D) -> Result<Self, D::Error>
    where
        D: ComDeserializer<'de>,
    {
        decoder.deserialize_option()
    }
}

impl<T, const N: usize> ComDeserialize for [T; N]
where
    T: ComDeserialize,
{
    /// Decodes exactly `N` elements into initialized array storage.
    fn com_deserialize<'de, D>(decoder: D) -> Result<Self, D::Error>
    where
        D: ComDeserializer<'de>,
    {
        decoder.deserialize_sequence()
    }
}

impl<T, const N: usize> ComSequence for [T; N]
where
    T: ComDeserialize,
{
    /// Reads a fixed-size sequence without unsafe uninitialized memory.
    fn read_sequence<'de, A>(sequence: &mut A) -> Result<Self, A::Error>
    where
        A: ComSequenceAccess<'de>,
    {
        let mut values: [Option<T>; N] = core::array::from_fn(|_| None);
        for (index, slot) in values.iter_mut().enumerate() {
            *slot = Some(sequence.next_element()?.ok_or_else(|| sequence.invalid_length(index, N))?);
        }
        sequence.finish(N)?;

        Ok(values.map(|value| value.expect("every fixed-array slot is initialized before construction")))
    }
}

#[cfg(feature = "alloc")]
impl<T> ComDeserialize for Vec<T>
where
    T: ComDeserialize,
{
    /// Decodes an owned, adapter-bounded sequence.
    fn com_deserialize<'de, D>(decoder: D) -> Result<Self, D::Error>
    where
        D: ComDeserializer<'de>,
    {
        decoder.deserialize_sequence()
    }
}

#[cfg(feature = "alloc")]
impl<T> ComSequence for Vec<T>
where
    T: ComDeserialize,
{
    /// Grows one owned vector while respecting the adapter's allocation bound.
    fn read_sequence<'de, A>(sequence: &mut A) -> Result<Self, A::Error>
    where
        A: ComSequenceAccess<'de>,
    {
        // A count is not a memory-size proof: a short malformed document can
        // advertise many large records. Reserve at most 4 KiB before decoding
        // elements; normal amortized growth follows only validated values.
        let eager_elements = 4096 / core::mem::size_of::<T>().max(1);
        let capacity = sequence.size_hint().unwrap_or(0).min(sequence.element_limit()).min(eager_elements);
        let mut values = Vec::with_capacity(capacity);
        while let Some(value) = sequence.next_element()? {
            values.push(value);
        }
        Ok(values)
    }
}

#[cfg(feature = "alloc")]
impl<T> ComDeserialize for Box<[T]>
where
    T: ComDeserialize,
{
    /// Decodes a bounded vector and converts it into compact boxed slice storage.
    fn com_deserialize<'de, D>(decoder: D) -> Result<Self, D::Error>
    where
        D: ComDeserializer<'de>,
    {
        decoder.deserialize_sequence()
    }
}

#[cfg(feature = "alloc")]
impl<T> ComSequence for Box<[T]>
where
    T: ComDeserialize,
{
    /// Reuses the vector builder so sequence validation has one implementation.
    fn read_sequence<'de, A>(sequence: &mut A) -> Result<Self, A::Error>
    where
        A: ComSequenceAccess<'de>,
    {
        Vec::<T>::read_sequence(sequence).map(Vec::into_boxed_slice)
    }
}

#[cfg(feature = "alloc")]
impl<K, V> ComDeserialize for BTreeMap<K, V>
where
    K: ComDeserialize + Ord,
    V: ComDeserialize,
{
    /// Decodes an owned ordered map with concrete key and value types.
    fn com_deserialize<'de, D>(decoder: D) -> Result<Self, D::Error>
    where
        D: ComDeserializer<'de>,
    {
        decoder.deserialize_map()
    }
}

#[cfg(feature = "alloc")]
impl<K, V> ComMap for BTreeMap<K, V>
where
    K: ComDeserialize + Ord,
    V: ComDeserialize,
{
    /// Builds a deterministic map and rejects repeated keys.
    fn read_map<'de, A>(map: &mut A) -> Result<Self, A::Error>
    where
        A: ComMapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some(key) = map.next_key()? {
            let value = map.next_value()?;
            if values.insert(key, value).is_some() {
                return Err(map.duplicate_key());
            }
        }
        Ok(values)
    }
}
