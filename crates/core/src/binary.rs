//! Experimental portable binary value format (NCB version 1).
//!
//! Integers and IEEE floats are little-endian; lengths and variant ordinals are
//! u32; pointer-sized integers occupy eight bytes. Records store field counts and
//! values in declaration order, not ABI offsets. Changing order or types requires
//! an initialization-schema revision. Each complete document starts with NCB/1.
//! This codec never activates objects: factories validate decoded initialization.
//!
//! Encoding and decoding share one structural budget. Each scalar or container
//! is one value; present options and enum payloads contain further values. The
//! root has depth one. Lengths, tags, and other framing consume bytes only.
//! Vectors always remain collections, including `Vec<u8>`; there is no implicit
//! opaque-blob conversion that would bypass their element or value limits.

use alloc::{string::String, vec::Vec};
use crate::{
    ComDeserialize, ComDeserializer, ComEnum, ComMap, ComMapAccess, ComRecord, ComRecordAccess, ComSequence, ComSequenceAccess, ComSerialize, ComSerializer,
    ComVariantAccess,
};

/// Wire-format marker, including the independently versioned codec revision.
const MAGIC: &[u8; 4] = b"NCB\x01";

/// Finite resource policy applied to encoding and decoding.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum complete document size, including the marker.
    pub bytes: usize,
    /// Maximum immediate sequence elements, record fields, or map pairs per container.
    /// Nested containers each receive this same per-container cap.
    pub elements: usize,
    /// Maximum values on a root-to-leaf path; even a scalar root has depth one.
    /// Format tags and lengths do not add nesting levels.
    pub depth: usize,
    /// Maximum cumulative scalar and container values in either direction.
    /// A vector of N scalar values costs N + 1; unit values also consume budget.
    /// Text counts as one value, while its UTF-8 payload consumes the byte limit.
    pub values: usize,
}
impl Default for Limits {
    /// Defaults suit initialization documents rather than unbounded streams.
    fn default() -> Self {
        Self {
            bytes: 16 * 1024 * 1024,
            elements: 65_536,
            depth: 64,
            values: 1_000_000,
        }
    }
}

/// A deterministic parse/encoding failure with a byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    /// Position at which the operation failed.
    pub offset: usize,
    /// Static diagnostic; decoding never allocates an error string.
    pub message: &'static str,
}
impl core::fmt::Display for Error {
    /// Formats a diagnostic only when the caller asks for one.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} at byte {}", self.message, self.offset)
    }
}
impl core::error::Error for Error {}

/// Serializes one complete typed value with the default limits.
pub fn to_vec<T: ComSerialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
    to_vec_with_limits(value, Limits::default())
}
/// Serializes one complete typed value under an explicit resource policy.
pub fn to_vec_with_limits<T: ComSerialize + ?Sized>(value: &T, limits: Limits) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    encode(value, limits, Some(&mut bytes))?;
    Ok(bytes)
}
/// Measures a complete encoding under exactly the writer/reader resource policy.
/// No output buffer is allocated: the ordinary serializer visits the same values,
/// framing and limits. Errors have the same offsets as materialized encoding.
/// A user-written ComSerialize implementation can still allocate internally; this
/// API only guarantees that the codec itself does not store the encoded payload.
pub fn encoded_len_with_limits<T: ComSerialize + ?Sized>(value: &T, limits: Limits) -> Result<usize, Error> {
    encode(value, limits, None)
}
/// Runs one authoritative encoder with either owned output storage or counting only.
/// Measurement is an admission check, not a cached proof for a later mutable value.
fn encode<T: ComSerialize + ?Sized>(value: &T, limits: Limits, output: Option<&mut Vec<u8>>) -> Result<usize, Error> {
    let mut writer = Writer { output, offset: 0, budget: Budget::new(limits) };
    writer.write(MAGIC)?;
    value.com_serialize(&mut writer)?;
    if writer.budget.depth != 0 {
        return Err(writer.error("unclosed serialized container"));
    }
    Ok(writer.offset)
}
/// Decodes one complete value; trailing data and unknown format versions fail.
pub fn from_slice<T: ComDeserialize>(bytes: &[u8]) -> Result<T, Error> {
    from_slice_with_limits(bytes, Limits::default())
}
/// Decodes under explicit byte, collection, depth, and total-value limits.
pub fn from_slice_with_limits<T: ComDeserialize>(bytes: &[u8], limits: Limits) -> Result<T, Error> {
    let mut reader = Reader {
        bytes,
        offset: 0,
        budget: Budget::new(limits),
    };
    if bytes.len() > limits.bytes {
        return Err(reader.error("document byte limit"));
    }
    if reader.take(4)? != MAGIC {
        return Err(reader.error("unsupported binary format"));
    }
    let value = reader.value()?;
    if reader.offset != bytes.len() {
        return Err(reader.error("trailing bytes"));
    }
    Ok(value)
}

/// Structural resource accounting shared by both directions of the binary codec.
///
/// Each schema value enters once and leaves after its children. Internal wire
/// operations never enter, so changing a length/tag encoding cannot change the
/// logical value count. Collection preflight checks a lower bound before either
/// writing its elements or allocating storage while reading them.
struct Budget {
    /// Values currently open on the root-to-leaf path.
    depth: usize,
    /// Values that may still be entered anywhere in this document.
    remaining: usize,
    /// Immutable policy supplied for this complete encoding or decoding call.
    limits: Limits,
}
impl Budget {
    /// Starts an empty traversal with the complete cumulative value allowance.
    fn new(limits: Limits) -> Self {
        Self {
            depth: 0,
            remaining: limits.values,
            limits,
        }
    }

    /// Charges one value before any of its contents are visited.
    fn enter(&mut self, offset: usize) -> Result<(), Error> {
        if self.depth >= self.limits.depth || self.remaining == 0 {
            return Err(Error {
                offset,
                message: "value or nesting limit",
            });
        }
        self.depth += 1;
        self.remaining -= 1;
        Ok(())
    }

    /// Ends a value's depth scope without refunding its cumulative charge.
    fn leave(&mut self, offset: usize) -> Result<(), Error> {
        self.depth = self.depth.checked_sub(1).ok_or(Error { offset, message: "unmatched value end" })?;
        Ok(())
    }

    /// Checks immediate cardinality and the minimum values required by its entries.
    /// Records/sequences need one value per entry; maps need a key and a value.
    fn collection(&self, len: usize, values_per_entry: usize, offset: usize) -> Result<(), Error> {
        let fits_values = len.checked_mul(values_per_entry).is_some_and(|minimum| minimum <= self.remaining);
        if len > self.limits.elements || !fits_values {
            return Err(Error {
                offset,
                message: "collection or value limit",
            });
        }
        Ok(())
    }
}

/// One encoding traversal; optional storage never changes its resource accounting.
struct Writer<'a> {
    /// Some borrows output exclusively; None validates/counts without materializing bytes.
    output: Option<&'a mut Vec<u8>>,
    /// Encoded bytes accepted so far, including framing, in either output mode.
    offset: usize,
    /// Shared definition of logical values, nesting, and collection bounds.
    budget: Budget,
}
impl Writer<'_> {
    /// Associates a static diagnostic with the current output position.
    fn error(&self, message: &'static str) -> Error {
        Error { offset: self.offset, message }
    }
    /// Checks the complete append before touching the buffer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.budget.limits.bytes.saturating_sub(self.offset) {
            return Err(self.error("document byte limit"));
        }
        match &mut self.output {
            Some(output) => {
                // Grow geometrically but never request capacity above this call's
                // byte ceiling. Reservation failure is a normal encoding error.
                let required = self.offset + bytes.len();
                if required > output.capacity() {
                    let capacity = required.max(output.capacity().saturating_mul(2)).min(self.budget.limits.bytes);
                    output.try_reserve_exact(capacity - output.len()).map_err(|_| Error {
                        offset: self.offset, message: "output allocation failed",
                    })?;
                }
                output.extend_from_slice(bytes);
            }
            None => {}
        }
        self.offset += bytes.len();
        Ok(())
    }
    /// Opens a schema value; callers must close it after all its children.
    fn enter(&mut self) -> Result<(), Error> {
        self.budget.enter(self.offset)
    }
    /// Ends a previously opened schema value, including an empty container.
    fn leave(&mut self) -> Result<(), Error> {
        self.budget.leave(self.offset)
    }
    /// Runs a leaf or option value within one budget scope.
    /// Any error aborts serialization; the caller discards the incomplete output
    /// rather than attempting to continue through unfinished child containers.
    fn value(&mut self, write: impl FnOnce(&mut Self) -> Result<(), Error>) -> Result<(), Error> {
        self.enter()?;
        let result = write(self);
        self.leave()?;
        result
    }
    /// Encodes one scalar without counting its individual representation bytes.
    fn scalar(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.value(|writer| writer.write(bytes))
    }
    /// Writes a count as framing after the shared collection preflight succeeds.
    fn count(&mut self, len: usize, values_per_entry: usize) -> Result<(), Error> {
        self.budget.collection(len, values_per_entry, self.offset)?;
        let count = u32::try_from(len).map_err(|_| self.error("length exceeds u32"))?;
        self.write(&count.to_le_bytes())
    }
}
impl ComSerializer for Writer<'_> {
    type Error = Error;
    /// Unit has no payload bytes.
    fn serialize_unit(&mut self) -> Result<(), Error> {
        self.scalar(&[])
    }
    /// Boolean has precisely two accepted encodings.
    fn serialize_bool(&mut self, value: bool) -> Result<(), Error> {
        self.scalar(&[u8::from(value)])
    }
    /// Writes the declared width in little-endian order.
    fn serialize_u8(&mut self, value: u8) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_u16(&mut self, value: u16) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_u32(&mut self, value: u32) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_u64(&mut self, value: u64) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_i8(&mut self, value: i8) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_i16(&mut self, value: i16) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_i32(&mut self, value: i32) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_i64(&mut self, value: i64) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_f32(&mut self, value: f32) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    fn serialize_f64(&mut self, value: f64) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// usize always occupies eight bytes on disk.
    fn serialize_usize(&mut self, value: usize) -> Result<(), Error> {
        self.serialize_u64(value as u64)
    }
    /// isize always occupies eight bytes on disk.
    fn serialize_isize(&mut self, value: isize) -> Result<(), Error> {
        self.serialize_i64(value as i64)
    }
    /// Strings use a byte count, independent of the collection-element limit.
    fn serialize_string(&mut self, value: &str) -> Result<(), Error> {
        self.value(|writer| {
            let len = u32::try_from(value.len()).map_err(|_| writer.error("string exceeds u32"))?;
            // The byte length is format framing, not another schema value.
            writer.write(&len.to_le_bytes())?;
            writer.write(value.as_bytes())
        })
    }
    /// Presence is followed by a recursively encoded value.
    fn serialize_option<T: ComSerialize>(&mut self, value: Option<&T>) -> Result<(), Error> {
        self.value(|writer| {
            // Presence is framing; only a present payload opens a child value.
            writer.write(&[u8::from(value.is_some())])?;
            match value {
                Some(value) => value.com_serialize(writer),
                None => Ok(()),
            }
        })
    }
    /// Records are positional within their explicit initialization schema.
    fn begin_record(&mut self, _: &'static str, len: usize) -> Result<(), Error> {
        self.enter()?;
        self.count(len, 1)
    }
    /// Field names are supplied by the schema, not repeated in binary data.
    fn field(&mut self, _: &'static str) -> Result<(), Error> {
        Ok(())
    }
    /// Finishes a record.
    fn end_record(&mut self) -> Result<(), Error> {
        self.leave()
    }
    /// Collections carry their element count.
    fn begin_sequence(&mut self, len: usize) -> Result<(), Error> {
        self.enter()?;
        self.count(len, 1)
    }
    /// Finishes a collection.
    fn end_sequence(&mut self) -> Result<(), Error> {
        self.leave()
    }
    /// Maps carry their pair count.
    fn begin_map(&mut self, len: usize) -> Result<(), Error> {
        self.enter()?;
        self.count(len, 2)
    }
    /// Finishes a map.
    fn end_map(&mut self) -> Result<(), Error> {
        self.leave()
    }
    /// Ordinals belong to the persistent schema, not to Rust enum memory layout.
    fn begin_variant(&mut self, _: &'static str, index: u32, _: &'static str) -> Result<(), Error> {
        self.enter()?;
        self.write(&index.to_le_bytes())
    }
    /// Finishes a variant's payload.
    fn end_variant(&mut self) -> Result<(), Error> {
        self.leave()
    }
}

/// Bounded cursor over borrowed input; decoded values acquire their own storage.
struct Reader<'de> {
    /// Entire immutable document.
    bytes: &'de [u8],
    /// Next unread byte.
    offset: usize,
    /// Same structural accounting used by the writer before emitting values.
    budget: Budget,
}
impl<'de> Reader<'de> {
    /// Creates a non-allocating diagnostic at the current cursor.
    fn error(&self, message: &'static str) -> Error {
        Error { offset: self.offset, message }
    }
    /// Checks the range before advancing or allocating owned data.
    fn take(&mut self, len: usize) -> Result<&'de [u8], Error> {
        let end = self.offset.checked_add(len).ok_or_else(|| self.error("length overflow"))?;
        let value = self.bytes.get(self.offset..end).ok_or_else(|| self.error("truncated value"))?;
        self.offset = end;
        Ok(value)
    }
    /// Every recursive construction consumes depth and total-value budgets.
    fn value<T: ComDeserialize>(&mut self) -> Result<T, Error> {
        self.budget.enter(self.offset)?;
        let result = T::com_deserialize(&mut *self);
        self.budget.leave(self.offset)?;
        result
    }
    /// Rejects large collection counts before Vec preallocation.
    fn count(&mut self, values_per_entry: usize) -> Result<usize, Error> {
        let len = self.deserialize_u32()? as usize;
        self.budget.collection(len, values_per_entry, self.offset)?;
        Ok(len)
    }
}
impl<'de> ComDeserializer<'de> for &mut Reader<'de> {
    type Error = Error;
    /// Unit consumes no payload bytes.
    fn deserialize_unit(self) -> Result<(), Error> {
        Ok(())
    }
    /// Rejects every boolean tag other than zero or one.
    fn deserialize_bool(self) -> Result<bool, Error> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(self.error("invalid boolean")),
        }
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_u8(self) -> Result<u8, Error> {
        Ok(u8::from_le_bytes(self.take(1)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_u16(self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_u32(self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_u64(self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_i8(self) -> Result<i8, Error> {
        Ok(i8::from_le_bytes(self.take(1)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_i16(self) -> Result<i16, Error> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_i32(self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_i64(self) -> Result<i64, Error> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_f32(self) -> Result<f32, Error> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    fn deserialize_f64(self) -> Result<f64, Error> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().expect("checked scalar width")))
    }
    /// A value saved on a wider target must fit the receiving pointer width.
    fn deserialize_usize(self) -> Result<usize, Error> {
        usize::try_from(self.deserialize_u64()?).map_err(|_| self.error("usize overflow"))
    }
    /// Signed pointer-width values are checked before conversion.
    fn deserialize_isize(self) -> Result<isize, Error> {
        isize::try_from(self.deserialize_i64()?).map_err(|_| self.error("isize overflow"))
    }
    /// Validates UTF-8 before making the one owned string copy.
    fn deserialize_string(self) -> Result<String, Error> {
        let len = self.deserialize_u32()? as usize;
        let text = core::str::from_utf8(self.take(len)?).map_err(|_| self.error("invalid UTF-8"))?;
        Ok(text.into())
    }
    /// Uses the same strict tag as booleans.
    fn deserialize_option<T: ComDeserialize>(self) -> Result<Option<T>, Error> {
        match self.deserialize_bool()? {
            true => self.value().map(Some),
            false => Ok(None),
        }
    }
    /// Feeds declaration-order indices to the existing generated record decoder.
    fn deserialize_record<T: ComRecord>(self) -> Result<T, Error> {
        let len = self.count(1)?;
        if len > T::FIELDS.len() {
            return Err(self.error("unknown record fields"));
        }
        let mut access = Record {
            reader: self,
            len,
            index: 0,
            pending: false,
        };
        let value = T::read_record(&mut access)?;
        if access.index != len || access.pending {
            return Err(access.reader.error("unconsumed record fields"));
        }
        Ok(value)
    }
    /// Verifies that the typed reader consumed the entire declared sequence.
    fn deserialize_sequence<T: ComSequence>(self) -> Result<T, Error> {
        let remaining = self.count(1)?;
        let mut access = Sequence { reader: self, remaining };
        let value = T::read_sequence(&mut access)?;
        access.finish(0)?;
        Ok(value)
    }
    /// Unknown variant ordinals fail before reading any payload.
    fn deserialize_enum<T: ComEnum>(self) -> Result<T, Error> {
        let index = self.deserialize_u32()?;
        if index as usize >= T::VARIANTS.len() {
            return Err(self.error("unknown enum variant"));
        }
        T::read_variant(index, self)
    }
    /// Alternating key/value access is checked explicitly.
    fn deserialize_map<T: ComMap>(self) -> Result<T, Error> {
        let remaining = self.count(2)?;
        let mut access = Map { reader: self, remaining, pending: false };
        let value = T::read_map(&mut access)?;
        if access.remaining != 0 || access.pending {
            return Err(access.reader.error("unconsumed map entries"));
        }
        Ok(value)
    }
}

/// Named-field decoder state; a value must follow each reported field index.
struct Record<'a, 'de> {
    /// Shared cursor borrowed for this record only.
    reader: &'a mut Reader<'de>,
    /// Encoded field count.
    len: usize,
    /// Next declaration-order field index.
    index: usize,
    /// A field name has been supplied but its value has not been read.
    pending: bool,
}
impl<'de> ComRecordAccess<'de> for Record<'_, 'de> {
    type Error = Error;
    /// Returns one declaration-order field at a time.
    fn next_field(&mut self, _: &'static [&'static str]) -> Result<Option<u32>, Error> {
        if self.pending {
            return Err(self.reader.error("record value required"));
        }
        match self.index < self.len {
            true => {
                let index = self.index;
                self.index += 1;
                self.pending = true;
                Ok(Some(index as u32))
            }
            false => Ok(None),
        }
    }
    /// Consumes the pending field through its concrete decoder.
    fn next_value<T: ComDeserialize>(&mut self) -> Result<T, Error> {
        if !self.pending {
            return Err(self.reader.error("record field required"));
        }
        self.pending = false;
        self.reader.value()
    }
    /// Binary fields are positional, but custom readers still receive diagnostics.
    fn duplicate_field(&self, _: &'static str) -> Error {
        self.reader.error("duplicate field")
    }
    /// Missing required fields are rejected by the generated constructor.
    fn missing_field(&self, _: &'static str) -> Error {
        self.reader.error("missing field")
    }
    /// Reports an invalid custom field index.
    fn invalid_field_index(&self, _: u32) -> Error {
        self.reader.error("invalid field index")
    }
}
/// Remaining sequence count prevents under- and over-consumption.
struct Sequence<'a, 'de> {
    /// Borrowed document cursor.
    reader: &'a mut Reader<'de>,
    /// Elements still unread.
    remaining: usize,
}
impl<'de> ComSequenceAccess<'de> for Sequence<'_, 'de> {
    type Error = Error;
    /// Advances only after proving an element remains.
    fn next_element<T: ComDeserialize>(&mut self) -> Result<Option<T>, Error> {
        match self.remaining {
            0 => Ok(None),
            _ => {
                self.remaining -= 1;
                self.reader.value().map(Some)
            }
        }
    }
    /// Gives the already validated allocation bound.
    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
    /// Retains the caller's per-collection cap.
    fn element_limit(&self) -> usize {
        self.reader.budget.limits.elements
    }
    /// Fixed arrays and custom readers must consume every encoded element.
    fn finish(&mut self, _: usize) -> Result<(), Error> {
        match self.remaining {
            0 => Ok(()),
            _ => Err(self.reader.error("unconsumed sequence elements")),
        }
    }
    /// Reports an array or tuple cardinality mismatch.
    fn invalid_length(&self, _: usize, _: usize) -> Error {
        self.reader.error("invalid sequence length")
    }
}
impl<'de> ComVariantAccess<'de> for &mut Reader<'de> {
    type Error = Error;
    /// Unit variants have no payload.
    fn unit(self) -> Result<(), Error> {
        Ok(())
    }
    /// Newtype variants wrap one typed value.
    fn newtype<T: ComDeserialize>(self) -> Result<T, Error> {
        self.value()
    }
    /// Tuple payloads reuse collection length checks.
    fn tuple<T: ComSequence>(self, _: usize) -> Result<T, Error> {
        self.value()
    }
    /// Record payloads reuse field checks.
    fn record<T: ComRecord>(self) -> Result<T, Error> {
        self.value()
    }
    /// Rejects unexpected ordinals from a custom enum implementation.
    fn invalid_variant(self, _: u32, _: &'static [&'static str]) -> Error {
        self.error("invalid variant")
    }
}
/// A map must alternate keys and values, including for zero-sized payloads.
struct Map<'a, 'de> {
    /// Borrowed document cursor.
    reader: &'a mut Reader<'de>,
    /// Pairs whose keys have not yet been read.
    remaining: usize,
    /// A key is waiting for its corresponding value.
    pending: bool,
}
impl<'de> ComMapAccess<'de> for Map<'_, 'de> {
    type Error = Error;
    /// Reads the next key while preserving pair boundaries.
    fn next_key<K: ComDeserialize>(&mut self) -> Result<Option<K>, Error> {
        if self.pending {
            return Err(self.reader.error("map value required"));
        }
        match self.remaining {
            0 => Ok(None),
            _ => {
                self.remaining -= 1;
                self.pending = true;
                self.reader.value().map(Some)
            }
        }
    }
    /// Reads the pending value, never a value without its key.
    fn next_value<V: ComDeserialize>(&mut self) -> Result<V, Error> {
        if !self.pending {
            return Err(self.reader.error("map key required"));
        }
        self.pending = false;
        self.reader.value()
    }
    /// Ordered-map construction rejects duplicate keys instead of overwriting.
    fn duplicate_key(&self) -> Error {
        self.reader.error("duplicate map key")
    }
}
