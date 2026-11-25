//! Format-neutral typed value construction through explicit COM sessions.
//!
//! Values stay statically typed in their owning binary. Only scalars, UTF-8,
//! named fields and bounded container operations cross the format interface.
//! Sessions have one root, finish once, and must be discarded after any error.

#[rustfmt::skip]
mod interfaces;
pub use interfaces::*;
#[doc(hidden)]
pub mod scope;
#[rustfmt::skip]
mod scoped;
#[doc(hidden)]
pub use scoped::{ISerializerVtblFor as ScopedSerializer, IDeserializerVtblFor as ScopedDeserializer};

use crate::{ComDeserialize, ComSerialize, ComMut, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_MEMORY, NC_E_OUT_OF_RANGE};

/// Finite resource policy shared by format providers and typed construction.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum complete encoded document size, including format framing.
    pub bytes: usize,
    /// Maximum elements, record fields or map pairs in any container.
    pub elements: usize,
    /// Maximum root-to-leaf value depth; the root consumes one level.
    pub depth: usize,
    /// Maximum cumulative scalar and container values in the document.
    pub values: usize,
}
impl Default for Limits {
    /// Bounded initialization documents are the default, independent of format.
    fn default() -> Self {
        Self {
            bytes: 16 * 1024 * 1024,
            elements: 65_536,
            depth: 64,
            values: 1_000_000,
        }
    }
}

/// Typed writing facade over an actual mutable COM format capability.
pub struct Serializer<'a> {
    /// Caller-retained or scoped provider; no Rust implementation layout crosses COM.
    interface: ComMut<'a, ISerializerAbi>,
}
impl<'a> Serializer<'a> {
    /// Borrows a format object. The caller retains it until this facade is dropped.
    pub fn new(interface: ComMut<'a, ISerializerAbi>) -> Self {
        Self { interface }
    }
    /// Writes one root and commits the session only after its complete success.
    pub fn serialize<T: ComSerialize + ?Sized>(&mut self, value: &T) -> Result<(), NcStatus> {
        self.write_root(value, true)
    }
    /// Writes one root for the SDK's owning document driver.
    ///
    /// With reporting disabled, the driver must discard its exclusive session
    /// on error. Explicit error reporting by the value still works as usual.
    #[doc(hidden)]
    pub fn write_root<T: ComSerialize + ?Sized>(&mut self, value: &T, report_failure: bool) -> Result<(), NcStatus> {
        if let Err(status) = value.com_serialize(self) {
            return Err(if report_failure {
                self.fail(status, "invalid persistent value")
            } else {
                status.normalize_error()
            });
        }
        self.interface.finish()
    }
    /// Reports an invalid application value or schema and returns `NC_E_INVALID_ARG`.
    pub fn error(&mut self, message: &str) -> NcStatus {
        self.fail(NC_E_INVALID_ARG, message)
    }
    /// Reports a detected error without asking the provider to choose its status.
    fn fail(&mut self, status: NcStatus, message: &str) -> NcStatus {
        let status = status.normalize_error();
        let _ = self.interface.fail(status, text(message));
        status
    }
    /// Converts a local count before it crosses the fixed-width COM boundary.
    fn count(&mut self, count: usize) -> Result<u32, NcStatus> {
        u32::try_from(count).map_err(|_| self.fail(NC_E_OUT_OF_RANGE, "length exceeds u32"))
    }
    /// Streams optional presence and its statically typed child.
    pub fn serialize_option<T: ComSerialize>(&mut self, value: Option<&T>) -> Result<(), NcStatus> {
        self.interface.begin_option(value.is_some())?;
        if let Some(value) = value {
            value.com_serialize(self)?;
        }
        self.interface.end_option()
    }
    /// Pointer-sized unsigned values have a portable u64 schema.
    pub fn serialize_usize(&mut self, value: usize) -> Result<(), NcStatus> {
        self.interface.serialize_u64(value as u64)
    }
    /// Pointer-sized signed values have a portable i64 schema.
    pub fn serialize_isize(&mut self, value: isize) -> Result<(), NcStatus> {
        self.interface.serialize_i64(value as i64)
    }
    /// Opens a named record; fields must be identified before their values.
    pub fn begin_record(&mut self, count: usize) -> Result<(), NcStatus> {
        let count = self.count(count)?;
        self.interface.begin_record(count)
    }
    /// Opens an ordered collection with its complete count.
    pub fn begin_sequence(&mut self, count: usize) -> Result<(), NcStatus> {
        let count = self.count(count)?;
        self.interface.begin_sequence(count)
    }
    /// Opens a map whose typed keys and values alternate.
    pub fn begin_map(&mut self, count: usize) -> Result<(), NcStatus> {
        let count = self.count(count)?;
        self.interface.begin_map(count)
    }
}

/// Typed construction facade over an actual mutable COM source capability.
pub struct Deserializer<'a> {
    /// Format-owned cursor and lifetime, retained by the caller or enclosing scope.
    interface: ComMut<'a, IDeserializerAbi>,
    /// Caller-side caps also bound allocation requests from foreign implementations.
    limits: Limits,
    /// Whether to request a provider diagnostic after an allocation failure.
    report_oom: bool,
}
impl<'a> Deserializer<'a> {
    /// Uses default construction limits with the caller-retained format object.
    pub fn new(interface: ComMut<'a, IDeserializerAbi>) -> Self {
        Self::with_limits(interface, Limits::default())
    }
    /// Supplies explicit allocation and collection caps in addition to provider limits.
    pub fn with_limits(interface: ComMut<'a, IDeserializerAbi>, limits: Limits) -> Self {
        Self { interface, limits, report_oom: true }
    }
    /// Restore consumes the owning source. Its discard path can return an
    /// allocation failure without asking the provider to allocate diagnostics.
    #[cfg(feature = "alloc")]
    pub(crate) fn for_restore(interface: ComMut<'a, IDeserializerAbi>, limits: Limits) -> Self {
        Self { interface, limits, report_oom: false }
    }
    /// Constructs one root and requires complete document consumption.
    pub fn deserialize<T: ComDeserialize>(&mut self) -> Result<T, NcStatus> {
        self.deserialize_with(T::com_deserialize)
    }
    /// Completes a document whose typed reader needs caller-owned context.
    pub(crate) fn deserialize_with<T>(&mut self, read: impl FnOnce(&mut Self) -> Result<T, NcStatus>) -> Result<T, NcStatus> {
        let value = self.value(read)?;
        self.interface.finish()?;
        Ok(value)
    }
    /// Acquires one typed child under the provider's structural budget.
    pub fn read<T: ComDeserialize>(&mut self) -> Result<T, NcStatus> {
        self.value(T::com_deserialize)
    }
    /// Opens a scope for an aggregate enum payload without erasing its type.
    pub fn value<T>(&mut self, read: impl FnOnce(&mut Self) -> Result<T, NcStatus>) -> Result<T, NcStatus> {
        self.interface.enter_value()?;
        let value = read(self).map_err(|status| self.fail(status, "invalid persistent value"))?;
        self.interface.leave_value()?;
        Ok(value)
    }
    /// Reports an invalid application value or schema and returns `NC_E_INVALID_ARG`.
    pub fn error(&mut self, message: &str) -> NcStatus {
        self.fail(NC_E_INVALID_ARG, message)
    }
    /// Reports the originating status, except on the allocation-free restore discard path.
    fn fail(&mut self, status: NcStatus, message: &str) -> NcStatus {
        let status = status.normalize_error();
        if self.report_oom || status != NC_E_OUT_OF_MEMORY {
            let _ = self.interface.fail(status, text(message));
        }
        status
    }
    /// Checks a foreign count before constructing any collection storage.
    fn count(&mut self, count: u32) -> Result<usize, NcStatus> {
        let count = count as usize;
        if count > self.limits.elements {
            return Err(self.fail(NC_E_OUT_OF_RANGE, "collection element limit"));
        }
        Ok(count)
    }
    /// Opens a record whose fields may be supplied in a format-specific order.
    pub fn record<T>(&mut self, maximum: usize, read: impl FnOnce(&mut RecordFieldReader<'_, 'a>) -> Result<T, NcStatus>) -> Result<T, NcStatus> {
        let maximum = u32::try_from(maximum).map_err(|_| self.fail(NC_E_OUT_OF_RANGE, "length exceeds u32"))?;
        let count = self.interface.begin_record(maximum)?;
        let count = self.count(count)?;
        if count > maximum as usize {
            return Err(self.error("unknown record fields"));
        }
        let mut fields = RecordFieldReader { reader: self, remaining: count };
        let value = read(&mut fields)?;
        fields.reader.interface.end_record()?;
        if fields.remaining != 0 {
            // A provider that completed the record must agree with its advertised count.
            return Err(fields.error("unconsumed record fields"));
        }
        Ok(value)
    }
    /// Opens an ordered, counted collection and requires complete consumption.
    pub fn sequence<T>(&mut self, read: impl FnOnce(&mut SequenceReader<'_, 'a>) -> Result<T, NcStatus>) -> Result<T, NcStatus> {
        let count = self.interface.begin_sequence()?;
        let count = self.count(count)?;
        let mut sequence = SequenceReader { reader: self, remaining: count };
        let value = read(&mut sequence)?;
        if sequence.remaining != 0 {
            return Err(sequence.error("unconsumed sequence elements"));
        }
        sequence.reader.interface.end_sequence()?;
        Ok(value)
    }
    /// Maps authored case names and ordinals to this source's selected payload.
    pub fn variant(&mut self, names: &[&str]) -> Result<u32, NcStatus> {
        for (index, name) in names.iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| self.fail(NC_E_OUT_OF_RANGE, "length exceeds u32"))?;
            if self.interface.variant_is(index, text(name))? {
                return Ok(index);
            }
        }
        Err(self.error("unknown enum variant"))
    }
    /// Pointer-sized schema integers must fit the receiving target.
    pub fn deserialize_usize(&mut self) -> Result<usize, NcStatus> {
        let value = self.interface.deserialize_u64()?;
        usize::try_from(value).map_err(|_| self.fail(NC_E_OUT_OF_RANGE, "usize overflow"))
    }
    /// Signed pointer-sized schema integers must fit the receiving target.
    pub fn deserialize_isize(&mut self) -> Result<isize, NcStatus> {
        let value = self.interface.deserialize_i64()?;
        isize::try_from(value).map_err(|_| self.fail(NC_E_OUT_OF_RANGE, "isize overflow"))
    }
    /// Distinguishes absence from a present unit/default value.
    pub fn deserialize_option<T: ComDeserialize>(&mut self) -> Result<Option<T>, NcStatus> {
        if self.interface.option_present()? { self.read().map(Some) } else { Ok(None) }
    }
    /// Acquires validated UTF-8 through fresh output storage, never a foreign String.
    #[cfg(feature = "alloc")]
    pub fn deserialize_string(&mut self) -> Result<alloc::string::String, NcStatus> {
        let len = self.interface.string_len()? as usize;
        if len > self.limits.bytes {
            return Err(self.fail(NC_E_OUT_OF_RANGE, "document byte limit"));
        }
        let mut output = crate::NcOutputBuffer::<u8>::new();
        let written = self.interface.copy_string(output.output(0, len))? as usize;
        if output.len() != len || written != len {
            return Err(self.error("invalid string length"));
        }
        alloc::string::String::from_utf8(output.into_vec()).map_err(|_| self.error("invalid UTF-8"))
    }
    /// Constructs a bounded typed map, rejecting duplicate decoded keys.
    #[cfg(feature = "alloc")]
    pub fn map<K: ComDeserialize + Ord, V: ComDeserialize>(&mut self) -> Result<alloc::collections::BTreeMap<K, V>, NcStatus> {
        let count = self.interface.begin_map()?;
        let count = self.count(count)?;
        let mut result = alloc::collections::BTreeMap::new();
        for _ in 0..count {
            let key = self.read()?;
            let value = self.read()?;
            if result.insert(key, value).is_some() {
                return Err(self.error("duplicate map key"));
            }
        }
        self.interface.end_map()?;
        Ok(result)
    }
}

/// Reads named fields from one schema-selected record through the parent cursor.
/// This is a call-scoped reader, not an owned collection of fields.
pub struct RecordFieldReader<'a, 's> {
    /// Parent cursor, exclusively borrowed through field construction.
    reader: &'a mut Deserializer<'s>,
    /// Actual encoded fields not yet acquired; unknown fields cannot disappear.
    remaining: usize,
}
impl RecordFieldReader<'_, '_> {
    /// Requests one named field, returning None only for genuine absence.
    pub fn read_field<T: ComDeserialize>(&mut self, name: &str) -> Result<Option<T>, NcStatus> {
        self.read_field_with(name, T::com_deserialize)
    }
    /// Reads the field under the same value scope while passing local context to its reader.
    pub(crate) fn read_field_with<T>(&mut self, name: &str, read: impl FnOnce(&mut Deserializer<'_>) -> Result<T, NcStatus>) -> Result<Option<T>, NcStatus> {
        if self.reader.interface.field(text(name))? {
            if self.remaining == 0 {
                return Err(self.error("unknown record fields"));
            }
            self.remaining -= 1;
            self.reader.value(read).map(Some)
        } else {
            Ok(None)
        }
    }
    /// Marks a field-construction failure at the provider's current position.
    pub fn error(&mut self, message: &str) -> NcStatus {
        self.reader.error(message)
    }
}
/// Reads one bounded ordered collection through the borrowed parent cursor.
/// The remaining count tracks encoded elements, without owning a value tree.
pub struct SequenceReader<'a, 's> {
    /// Parent cursor borrowed until all elements have been acquired.
    reader: &'a mut Deserializer<'s>,
    /// Elements still present in the bounded source.
    remaining: usize,
}
impl SequenceReader<'_, '_> {
    /// Acquires the next typed element, or None at a genuine collection end.
    pub fn read_next<T: ComDeserialize>(&mut self) -> Result<Option<T>, NcStatus> {
        self.read_next_with(T::com_deserialize)
    }
    /// Reads an element with local context after opening its ordinary value scope.
    pub(crate) fn read_next_with<T>(&mut self, read: impl FnOnce(&mut Deserializer<'_>) -> Result<T, NcStatus>) -> Result<Option<T>, NcStatus> {
        if self.remaining == 0 {
            Ok(None)
        } else {
            self.remaining -= 1;
            self.reader.value(read).map(Some)
        }
    }
    /// Gives a bounded count for conservative preallocation.
    pub fn remaining(&self) -> usize {
        self.remaining
    }
    /// Marks a collection-construction failure at the current position.
    pub fn error(&mut self, message: &str) -> NcStatus {
        self.reader.error(message)
    }
}

impl Serializer<'_> {
    /// Writes a unit through the COM provider.
    pub fn serialize_unit(&mut self) -> Result<(), NcStatus> {
        self.interface.serialize_unit()
    }
    /// Writes one typed bool through the COM provider.
    pub fn serialize_bool(&mut self, value: bool) -> Result<(), NcStatus> {
        self.interface.serialize_bool(value)
    }
    /// Writes one typed u8 through the COM provider.
    pub fn serialize_u8(&mut self, value: u8) -> Result<(), NcStatus> {
        self.interface.serialize_u8(value)
    }
    /// Writes one typed u16 through the COM provider.
    pub fn serialize_u16(&mut self, value: u16) -> Result<(), NcStatus> {
        self.interface.serialize_u16(value)
    }
    /// Writes one typed u32 through the COM provider.
    pub fn serialize_u32(&mut self, value: u32) -> Result<(), NcStatus> {
        self.interface.serialize_u32(value)
    }
    /// Writes one typed u64 through the COM provider.
    pub fn serialize_u64(&mut self, value: u64) -> Result<(), NcStatus> {
        self.interface.serialize_u64(value)
    }
    /// Writes one typed i8 through the COM provider.
    pub fn serialize_i8(&mut self, value: i8) -> Result<(), NcStatus> {
        self.interface.serialize_i8(value)
    }
    /// Writes one typed i16 through the COM provider.
    pub fn serialize_i16(&mut self, value: i16) -> Result<(), NcStatus> {
        self.interface.serialize_i16(value)
    }
    /// Writes one typed i32 through the COM provider.
    pub fn serialize_i32(&mut self, value: i32) -> Result<(), NcStatus> {
        self.interface.serialize_i32(value)
    }
    /// Writes one typed i64 through the COM provider.
    pub fn serialize_i64(&mut self, value: i64) -> Result<(), NcStatus> {
        self.interface.serialize_i64(value)
    }
    /// Writes one typed f32 through the COM provider.
    pub fn serialize_f32(&mut self, value: f32) -> Result<(), NcStatus> {
        self.interface.serialize_f32(value)
    }
    /// Writes one typed f64 through the COM provider.
    pub fn serialize_f64(&mut self, value: f64) -> Result<(), NcStatus> {
        self.interface.serialize_f64(value)
    }
    /// Forwards the format-neutral serialize_string operation.
    pub fn serialize_string(&mut self, value: &str) -> Result<(), NcStatus> {
        self.interface.serialize_string(text(value))
    }
    /// Forwards the format-neutral field operation.
    pub fn field(&mut self, name: &str) -> Result<(), NcStatus> {
        self.interface.field(text(name))
    }
    /// Forwards the format-neutral end_record operation.
    pub fn end_record(&mut self) -> Result<(), NcStatus> {
        self.interface.end_record()
    }
    /// Forwards the format-neutral end_sequence operation.
    pub fn end_sequence(&mut self) -> Result<(), NcStatus> {
        self.interface.end_sequence()
    }
    /// Forwards the format-neutral end_map operation.
    pub fn end_map(&mut self) -> Result<(), NcStatus> {
        self.interface.end_map()
    }
    /// Forwards the format-neutral begin_variant operation.
    pub fn begin_variant(&mut self, index: u32, name: &str) -> Result<(), NcStatus> {
        self.interface.begin_variant(index, text(name))
    }
    /// Forwards the format-neutral end_variant operation.
    pub fn end_variant(&mut self) -> Result<(), NcStatus> {
        self.interface.end_variant()
    }
}
impl Deserializer<'_> {
    /// Reads a unit through the COM provider.
    pub fn deserialize_unit(&mut self) -> Result<(), NcStatus> {
        self.interface.deserialize_unit()
    }
    /// Reads one typed bool through the COM provider.
    pub fn deserialize_bool(&mut self) -> Result<bool, NcStatus> {
        self.interface.deserialize_bool()
    }
    /// Reads one typed u8 through the COM provider.
    pub fn deserialize_u8(&mut self) -> Result<u8, NcStatus> {
        self.interface.deserialize_u8()
    }
    /// Reads one typed u16 through the COM provider.
    pub fn deserialize_u16(&mut self) -> Result<u16, NcStatus> {
        self.interface.deserialize_u16()
    }
    /// Reads one typed u32 through the COM provider.
    pub fn deserialize_u32(&mut self) -> Result<u32, NcStatus> {
        self.interface.deserialize_u32()
    }
    /// Reads one typed u64 through the COM provider.
    pub fn deserialize_u64(&mut self) -> Result<u64, NcStatus> {
        self.interface.deserialize_u64()
    }
    /// Reads one typed i8 through the COM provider.
    pub fn deserialize_i8(&mut self) -> Result<i8, NcStatus> {
        self.interface.deserialize_i8()
    }
    /// Reads one typed i16 through the COM provider.
    pub fn deserialize_i16(&mut self) -> Result<i16, NcStatus> {
        self.interface.deserialize_i16()
    }
    /// Reads one typed i32 through the COM provider.
    pub fn deserialize_i32(&mut self) -> Result<i32, NcStatus> {
        self.interface.deserialize_i32()
    }
    /// Reads one typed i64 through the COM provider.
    pub fn deserialize_i64(&mut self) -> Result<i64, NcStatus> {
        self.interface.deserialize_i64()
    }
    /// Reads one typed f32 through the COM provider.
    pub fn deserialize_f32(&mut self) -> Result<f32, NcStatus> {
        self.interface.deserialize_f32()
    }
    /// Reads one typed f64 through the COM provider.
    pub fn deserialize_f64(&mut self) -> Result<f64, NcStatus> {
        self.interface.deserialize_f64()
    }
}

/// Borrows complete UTF-8 text through the canonical readable-buffer argument.
fn text(value: &str) -> crate::NcCopyFromBufferOffset<'_, u8> {
    crate::NcCopyFromBufferOffset::new(0, value.as_bytes())
}
