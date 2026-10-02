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

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};
use crate::{ComDeserialize, ComSerialize};

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
#[cfg(feature = "alloc")]
pub fn to_vec<T: ComSerialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
    to_vec_with_limits(value, Limits::default())
}
/// Serializes one complete typed value under an explicit resource policy.
#[cfg(feature = "alloc")]
pub fn to_vec_with_limits<T: ComSerialize + ?Sized>(value: &T, limits: Limits) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    encode(value, limits, Output::Growable(&mut bytes))?;
    Ok(bytes)
}
/// Encodes one complete document into caller-owned storage without allocation.
/// The return value is the initialized prefix length; bytes after it are untouched.
/// On failure the caller must discard the partially written prefix.
pub fn to_slice<T: ComSerialize + ?Sized>(value: &T, output: &mut [u8]) -> Result<usize, Error> {
    to_slice_with_limits(value, output, Limits::default())
}
/// Uses exactly the same encoding and limits as allocated output and measurement.
pub fn to_slice_with_limits<T: ComSerialize + ?Sized>(value: &T, output: &mut [u8], limits: Limits) -> Result<usize, Error> {
    encode(value, limits, Output::Slice(output))
}
/// Measures a complete encoding under exactly the writer/reader resource policy.
/// No output buffer is allocated: the ordinary serializer visits the same values,
/// framing and limits. Errors have the same offsets as materialized encoding.
/// A user-written ComSerialize implementation can still allocate internally; this
/// API only guarantees that the codec itself does not store the encoded payload.
pub fn encoded_len_with_limits<T: ComSerialize + ?Sized>(value: &T, limits: Limits) -> Result<usize, Error> {
    encode(value, limits, Output::Count)
}
/// Runs one authoritative encoder with either owned output storage or counting only.
/// Measurement is an admission check, not a cached proof for a later mutable value.
fn encode<T: ComSerialize + ?Sized>(value: &T, limits: Limits, output: Output<'_>) -> Result<usize, Error> {
    let mut writer = Writer {
        output,
        offset: 0,
        budget: Budget::new(limits),
    };
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
    let value = reader.read()?;
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

/// Concrete NCB destinations; feature selection changes storage availability, never bytes.
enum Output<'a> {
    /// Counts an encoding without retaining any of its bytes.
    Count,
    /// Writes into caller-owned capacity without allocating.
    Slice(&'a mut [u8]),
    /// Grows provider-local storage fallibly under the same byte budget.
    #[cfg(feature = "alloc")]
    Growable(&'a mut Vec<u8>),
}

/// One bounded NCB encoding traversal, borrowed by a typed ComSerialize implementation.
///
/// All members are private. The public typed operations preserve fixed endian/width
/// rules and charge the shared structural budget. Errors abort the encoding; a
/// custom implementation must propagate errors and balance each begin/end pair.
/// Use to_vec, to_slice or encoded_len_with_limits to start a complete document.
pub struct Writer<'a> {
    /// Storage policy; counting and materialization visit the same encoding path.
    output: Output<'a>,
    /// Encoded bytes accepted so far, including framing, in every output mode.
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
            Output::Slice(output) => {
                let end = self.offset + bytes.len();
                let destination = output.get_mut(self.offset..end).ok_or(Error {
                    offset: self.offset,
                    message: "output buffer too small",
                })?;
                destination.copy_from_slice(bytes);
            }
            #[cfg(feature = "alloc")]
            Output::Growable(output) => {
                // Grow geometrically but never request capacity above this call's
                // byte ceiling. Reservation failure is a normal encoding error.
                let required = self.offset + bytes.len();
                if required > output.capacity() {
                    let capacity = required.max(output.capacity().saturating_mul(2)).min(self.budget.limits.bytes);
                    output.try_reserve_exact(capacity - output.len()).map_err(|_| Error {
                        offset: self.offset,
                        message: "output allocation failed",
                    })?;
                }
                output.extend_from_slice(bytes);
            }
            Output::Count => {}
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
impl Writer<'_> {
    /// Unit has no payload bytes.
    pub fn serialize_unit(&mut self) -> Result<(), Error> {
        self.scalar(&[])
    }
    /// Boolean has precisely two accepted encodings.
    pub fn serialize_bool(&mut self, value: bool) -> Result<(), Error> {
        self.scalar(&[u8::from(value)])
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_u8(&mut self, value: u8) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_u16(&mut self, value: u16) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_u32(&mut self, value: u32) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_u64(&mut self, value: u64) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_i8(&mut self, value: i8) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_i16(&mut self, value: i16) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_i32(&mut self, value: i32) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_i64(&mut self, value: i64) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_f32(&mut self, value: f32) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// Writes the declared width in little-endian order.
    pub fn serialize_f64(&mut self, value: f64) -> Result<(), Error> {
        self.scalar(&value.to_le_bytes())
    }
    /// usize always occupies eight bytes on disk.
    pub fn serialize_usize(&mut self, value: usize) -> Result<(), Error> {
        self.serialize_u64(value as u64)
    }
    /// isize always occupies eight bytes on disk.
    pub fn serialize_isize(&mut self, value: isize) -> Result<(), Error> {
        self.serialize_i64(value as i64)
    }
    /// Strings use a byte count, independent of the collection-element limit.
    pub fn serialize_string(&mut self, value: &str) -> Result<(), Error> {
        self.value(|writer| {
            let len = u32::try_from(value.len()).map_err(|_| writer.error("string exceeds u32"))?;
            // The byte length is format framing, not another schema value.
            writer.write(&len.to_le_bytes())?;
            writer.write(value.as_bytes())
        })
    }
    /// Presence is followed by a recursively encoded value.
    pub fn serialize_option<T: ComSerialize>(&mut self, value: Option<&T>) -> Result<(), Error> {
        self.value(|writer| {
            // Presence is framing; only a present payload opens a child value.
            writer.write(&[u8::from(value.is_some())])?;
            match value {
                Some(value) => value.com_serialize(writer),
                None => Ok(()),
            }
        })
    }
    /// Collections carry their element count.
    pub fn begin_sequence(&mut self, len: usize) -> Result<(), Error> {
        self.enter()?;
        self.count(len, 1)
    }
    /// Finishes a collection.
    pub fn end_sequence(&mut self) -> Result<(), Error> {
        self.leave()
    }
    /// Maps carry their pair count.
    pub fn begin_map(&mut self, len: usize) -> Result<(), Error> {
        self.enter()?;
        self.count(len, 2)
    }
    /// Finishes a map.
    pub fn end_map(&mut self) -> Result<(), Error> {
        self.leave()
    }
    /// Ordinals belong to the persistent schema, not to Rust enum memory layout.
    pub fn begin_variant(&mut self, index: u32) -> Result<(), Error> {
        self.enter()?;
        self.write(&index.to_le_bytes())
    }
    /// Finishes a variant's payload.
    pub fn end_variant(&mut self) -> Result<(), Error> {
        self.leave()
    }
}

/// Bounded NCB cursor supplied to a typed ComDeserialize implementation.
///
/// Read child values with read(), which charges their shared depth/value budget.
/// Record and sequence scopes consume fields in order; they never supply names or
/// arbitrary indices. Errors abort decoding and retain the exact failure offset.
/// Complete documents enter through from_slice or from_slice_with_limits.
pub struct Reader<'de> {
    /// Entire immutable document.
    bytes: &'de [u8],
    /// Next unread byte.
    offset: usize,
    /// Same structural accounting used by the writer before emitting values.
    budget: Budget,
}
impl<'de> Reader<'de> {
    /// Creates a non-allocating diagnostic at the current cursor.
    pub fn error(&self, message: &'static str) -> Error {
        Error { offset: self.offset, message }
    }
    /// Checks the range before advancing or allocating owned data.
    fn take(&mut self, len: usize) -> Result<&'de [u8], Error> {
        let end = self.offset.checked_add(len).ok_or_else(|| self.error("length overflow"))?;
        let value = self.bytes.get(self.offset..end).ok_or_else(|| self.error("truncated value"))?;
        self.offset = end;
        Ok(value)
    }
    /// Reads one typed child and charges its depth and cumulative value exactly once.
    pub fn read<T: ComDeserialize>(&mut self) -> Result<T, Error> {
        self.value(T::com_deserialize)
    }
    /// Opens one unnamed schema value, used for a tuple/record enum payload.
    /// This gives aggregate payloads the same budget as ordinary named values
    /// without generating private helper structs solely to invoke their decoder.
    pub fn value<T>(&mut self, read: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, Error> {
        self.budget.enter(self.offset)?;
        let result = read(self);
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
impl Reader<'_> {
    /// Unit consumes no payload bytes.
    pub(crate) fn deserialize_unit(&mut self) -> Result<(), Error> {
        Ok(())
    }
    /// Rejects every boolean tag other than zero or one.
    pub(crate) fn deserialize_bool(&mut self) -> Result<bool, Error> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(self.error("invalid boolean")),
        }
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_u8(&mut self) -> Result<u8, Error> {
        Ok(u8::from_le_bytes(self.take(1)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_i8(&mut self) -> Result<i8, Error> {
        Ok(i8::from_le_bytes(self.take(1)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_i16(&mut self) -> Result<i16, Error> {
        Ok(i16::from_le_bytes(self.take(2)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_i64(&mut self) -> Result<i64, Error> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_f32(&mut self) -> Result<f32, Error> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().expect("checked scalar width")))
    }
    /// Reads exactly the declared width without pointer casts.
    pub(crate) fn deserialize_f64(&mut self) -> Result<f64, Error> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().expect("checked scalar width")))
    }
    /// A value saved on a wider target must fit the receiving pointer width.
    pub(crate) fn deserialize_usize(&mut self) -> Result<usize, Error> {
        usize::try_from(self.deserialize_u64()?).map_err(|_| self.error("usize overflow"))
    }
    /// Signed pointer-width values are checked before conversion.
    pub(crate) fn deserialize_isize(&mut self) -> Result<isize, Error> {
        isize::try_from(self.deserialize_i64()?).map_err(|_| self.error("isize overflow"))
    }
    /// Validates UTF-8 before making the one owned string copy.
    #[cfg(feature = "alloc")]
    pub(crate) fn deserialize_string(&mut self) -> Result<String, Error> {
        let len = self.deserialize_u32()? as usize;
        let text = core::str::from_utf8(self.take(len)?).map_err(|_| self.error("invalid UTF-8"))?;
        Ok(text.into())
    }
    /// Uses the same strict tag as booleans.
    pub(crate) fn deserialize_option<T: ComDeserialize>(&mut self) -> Result<Option<T>, Error> {
        match self.deserialize_bool()? {
            true => self.read().map(Some),
            false => Ok(None),
        }
    }
    /// Reads a declaration-ordered record; only an omitted suffix may be absent.
    /// Reject excess fields before entering user construction, and require the
    /// constructor to consume every field that the document actually contains.
    pub fn record<T>(&mut self, maximum: usize, read: impl FnOnce(&mut Sequence<'_, '_>) -> Result<T, Error>) -> Result<T, Error> {
        let remaining = self.count(1)?;
        if remaining > maximum {
            return Err(self.error("unknown record fields"));
        }
        let mut fields = Sequence { reader: self, remaining };
        let value = read(&mut fields)?;
        fields.finish("unconsumed record fields")?;
        Ok(value)
    }
    /// Reads a counted sequence and requires complete consumption of its elements.
    pub fn sequence<T>(&mut self, read: impl FnOnce(&mut Sequence<'_, '_>) -> Result<T, Error>) -> Result<T, Error> {
        let remaining = self.count(1)?;
        let mut elements = Sequence { reader: self, remaining };
        let value = read(&mut elements)?;
        elements.finish("unconsumed sequence elements")?;
        Ok(value)
    }
    /// Reads and validates an enum ordinal before acquiring its selected payload.
    /// The ordinal is framing within the already charged enum value.
    pub fn variant(&mut self, count: usize) -> Result<u32, Error> {
        let index = self.deserialize_u32()?;
        match (index as usize) < count {
            true => Ok(index),
            false => Err(self.error("unknown enum variant")),
        }
    }
    /// Builds the sole supported owned map directly from bounded key/value pairs.
    /// Keys and values each consume a value; a pair adds no synthetic tuple level.
    #[cfg(feature = "alloc")]
    pub(crate) fn map<K: ComDeserialize + Ord, V: ComDeserialize>(&mut self) -> Result<alloc::collections::BTreeMap<K, V>, Error> {
        let count = self.count(2)?;
        let mut values = alloc::collections::BTreeMap::new();
        for _ in 0..count {
            let key = self.read()?;
            let value = self.read()?;
            if values.insert(key, value).is_some() {
                return Err(self.error("duplicate map key"));
            }
        }
        Ok(values)
    }
}

/// One counted sequence of positional fields or elements, borrowing its parent cursor.
///
/// The next value's type comes from the static schema. No field can be revisited,
/// reordered, or read beyond the encoded count. Parent scope completion checks
/// that the constructor consumed all values. This is concrete NCB state, not an
/// extensible input-format protocol.
pub struct Sequence<'a, 'de> {
    /// Cursor carrying the document-wide byte/depth/value limits.
    reader: &'a mut Reader<'de>,
    /// Encoded values still available in this container.
    remaining: usize,
}
impl Sequence<'_, '_> {
    /// Acquires the next concrete value, or reports a genuinely absent suffix.
    /// Truncated bytes remain an error and never masquerade as an omitted field.
    pub fn next<T: ComDeserialize>(&mut self) -> Result<Option<T>, Error> {
        match self.remaining {
            0 => Ok(None),
            _ => {
                self.remaining -= 1;
                self.reader.read().map(Some)
            }
        }
    }
    /// Returns the exact remaining count, already checked against collection limits.
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    /// Associates a schema-specific diagnostic with the current failure position.
    pub fn error(&self, message: &'static str) -> Error {
        self.reader.error(message)
    }
    /// Rejects unconsumed values before returning a completed container.
    fn finish(&self, message: &'static str) -> Result<(), Error> {
        match self.remaining {
            0 => Ok(()),
            _ => Err(self.error(message)),
        }
    }
}
