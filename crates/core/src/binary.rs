//! Experimental portable binary value format (NCB version 1).
//!
//! Integers and IEEE floats are little-endian; lengths and variant ordinals are
//! u32; pointer-sized integers occupy eight bytes. Records store field counts and
//! values in declaration order, not ABI offsets. Changing order or types requires
//! an initialization-schema revision. Each complete document starts with NCB/1.
//! This codec never acquires objects: factories validate decoded initialization.
//!
//! Encoding and decoding share one structural budget. Each scalar or container
//! is one value; present options and enum payloads contain further values. The
//! root has depth one. Lengths, tags, and other framing consume bytes only.
//! Vectors always remain collections, including `Vec<u8>`; there is no implicit
//! opaque-blob conversion that would bypass their element or value limits.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use crate::{ComDeserialize, ComSerialize};

/// Wire-format marker, including the independently versioned codec revision.
const MAGIC: &[u8; 4] = b"NCB\x01";

pub use crate::serialization::Limits;

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
    encode(value, limits, Output::Owned(Vec::new()))?.into_vec()
}
/// Encodes one complete document into caller-owned storage without allocation.
/// The return value is the initialized prefix length; bytes after it are untouched.
/// On failure the caller must discard the partially written prefix.
pub fn to_slice<T: ComSerialize + ?Sized>(value: &T, output: &mut [u8]) -> Result<usize, Error> {
    to_slice_with_limits(value, output, Limits::default())
}
/// Uses exactly the same encoding and limits as allocated output and measurement.
pub fn to_slice_with_limits<T: ComSerialize + ?Sized>(value: &T, output: &mut [u8], limits: Limits) -> Result<usize, Error> {
    encode(value, limits, Output::Slice(output)).map(|writer| writer.offset)
}
/// Measures a complete encoding under exactly the writer/reader resource policy.
/// No output buffer is allocated: the ordinary serializer visits the same values,
/// framing and limits. Errors have the same offsets as materialized encoding.
/// A user-written ComSerialize implementation can still allocate internally; this
/// API only guarantees that the codec itself does not store the encoded payload.
pub fn encoded_len_with_limits<T: ComSerialize + ?Sized>(value: &T, limits: Limits) -> Result<usize, Error> {
    encode(value, limits, Output::Count).map(|writer| writer.offset)
}
/// Runs one authoritative encoder with either owned output storage or counting only.
/// Measurement is an admission check, not a cached proof for a later mutable value.
fn encode<'a, T: ComSerialize + ?Sized>(value: &T, limits: Limits, output: Output<'a>) -> Result<Encoder<'a>, Error> {
    let mut encoder = Encoder {
        output,
        offset: 0,
        budget: Budget::new(limits),
        protocol: Protocol::new(),
        phase: Phase::Active,
    };
    encoder.write(MAGIC)?;
    let scoped = crate::serialization::scope::Scoped::new(encoder);
    let result = {
        // SAFETY: this generated table guards exactly Scoped<Encoder> and rejects ownership.
        let interface = unsafe { scoped.interface(&crate::serialization::ScopedSerializer::<Encoder<'_>>::VTABLE) };
        crate::serialization::Serializer::new(interface).serialize(value)
    };
    let encoder = scoped.into_inner();
    result.map_err(|_| encoder.failure("invalid persistent value"))?;
    Ok(encoder)
}
/// Decodes one complete value; trailing data and unknown format versions fail.
pub fn from_slice<T: ComDeserialize>(bytes: &[u8]) -> Result<T, Error> {
    from_slice_with_limits(bytes, Limits::default())
}
/// Decodes under explicit byte, collection, depth, and total-value limits.
pub fn from_slice_with_limits<T: ComDeserialize>(bytes: &[u8], limits: Limits) -> Result<T, Error> {
    let decoder = Decoder::borrowed(bytes, limits)?;
    let scoped = crate::serialization::scope::Scoped::new(decoder);
    let result = {
        // SAFETY: the table and the borrowed state have this scope's exact provider type.
        let interface = unsafe { scoped.interface(&crate::serialization::ScopedDeserializer::<Decoder<'_>>::VTABLE) };
        crate::serialization::Deserializer::with_limits(interface, limits).deserialize()
    };
    let decoder = scoped.into_inner();
    result.map_err(|_| decoder.failure("invalid persistent value"))
}

mod format;
use format::{Phase, Protocol};
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
    Owned(Vec<u8>),
}

/// Concrete provider state implementing the COM serializer's NCB operations.
///
/// Private operations preserve fixed endian/width rules and charge the shared
/// structural budget. Typed values use serialization::Serializer through COM;
/// they never depend on this state's layout. Format failures are terminal.
/// Binary helpers lend this state through a scoped object; the facade also
/// supplies an owning NcbSerializer object for independently retained sessions.
pub struct Encoder<'a> {
    /// Storage policy; counting and materialization visit the same encoding path.
    output: Output<'a>,
    /// Encoded bytes accepted so far, including framing, in every output mode.
    offset: usize,
    /// Shared definition of logical values, nesting, and collection bounds.
    budget: Budget,
    /// Exact logical container state, independent of NCB byte offsets.
    protocol: Protocol,
    /// Terminal failure or successful finish; neither permits further operations.
    phase: Phase,
}
impl Encoder<'_> {
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
            Output::Owned(output) => {
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
        self.budget.enter(self.offset)?;
        self.protocol.enter().map_err(|message| self.error(message))
    }
    /// Ends a previously opened schema value, including an empty container.
    fn leave(&mut self) -> Result<(), Error> {
        self.protocol.leave().map_err(|message| self.error(message))?;
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
        self.value(|writer| {
            writer.scalar_shape()?;
            writer.write(bytes)
        })
    }
    /// Writes a count as framing after the shared collection preflight succeeds.
    fn count(&mut self, len: usize, values_per_entry: usize) -> Result<(), Error> {
        self.budget.collection(len, values_per_entry, self.offset)?;
        let count = u32::try_from(len).map_err(|_| self.error("length exceeds u32"))?;
        self.write(&count.to_le_bytes())
    }
}
impl Encoder<'_> {
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
    /// Strings use a byte count, independent of the collection-element limit.
    fn serialize_string(&mut self, value: &str) -> Result<(), Error> {
        self.value(|writer| {
            writer.scalar_shape()?;
            let len = u32::try_from(value.len()).map_err(|_| writer.error("string exceeds u32"))?;
            // The byte length is format framing, not another schema value.
            writer.write(&len.to_le_bytes())?;
            writer.write(value.as_bytes())
        })
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
    fn begin_variant(&mut self, index: u32) -> Result<(), Error> {
        self.enter()?;
        self.write(&index.to_le_bytes())
    }
    /// Finishes a variant's payload.
    fn end_variant(&mut self) -> Result<(), Error> {
        self.leave()
    }
}

/// Concrete provider state implementing the COM deserializer's NCB operations.
///
/// Typed construction uses serialization::Deserializer through COM. NCB resolves
/// fields positionally and cases by ordinal while charging the same depth/value
/// budget as encoding. Failures are terminal and retain their exact byte offset.
/// Binary helpers borrow input; the facade's NcbDeserializer owns its document.
pub struct Decoder<'de> {
    /// Entire immutable document.
    bytes: Input<'de>,
    /// Next unread byte.
    offset: usize,
    /// Same structural accounting used by the writer before emitting values.
    budget: Budget,
    /// Exact logical container state, independent of NCB byte offsets.
    protocol: Protocol,
    /// Terminal failure or successful finish; neither permits further operations.
    phase: Phase,
    /// Validated string bytes awaiting one output-copy operation.
    pending_string: Option<core::ops::Range<usize>>,
    /// An ordinal being probed by authored case names, before selecting its payload.
    pending_variant: Option<u32>,
}
impl<'de> Decoder<'de> {
    /// Creates a non-allocating diagnostic at the current cursor.
    fn error(&self, message: &'static str) -> Error {
        Error { offset: self.offset, message }
    }
    /// Checks the range before advancing or allocating owned data.
    fn take(&mut self, len: usize) -> Result<&[u8], Error> {
        let end = self.offset.checked_add(len).ok_or_else(|| self.error("length overflow"))?;
        let value = self.bytes.as_slice().get(self.offset..end).ok_or_else(|| self.error("truncated value"))?;
        self.offset = end;
        Ok(value)
    }
    /// Rejects large collection counts before Vec preallocation.
    fn count(&mut self, values_per_entry: usize) -> Result<usize, Error> {
        let len = self.deserialize_u32()? as usize;
        self.budget.collection(len, values_per_entry, self.offset)?;
        Ok(len)
    }
}
impl Decoder<'_> {
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
}

/// Input ownership is explicit: allocation-free borrows and owned COM sources share parsing.
enum Input<'a> {
    /// Caller-owned bytes retained by a scoped operation.
    Borrowed(&'a [u8]),
    /// Format-owned bytes that may outlive the creating call.
    #[cfg(feature = "alloc")]
    Owned(Vec<u8>),
}
impl Input<'_> {
    /// Borrows immutable bytes without exposing allocation ownership.
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Borrowed(bytes) => bytes,
            #[cfg(feature = "alloc")]
            Self::Owned(bytes) => bytes,
        }
    }
}
