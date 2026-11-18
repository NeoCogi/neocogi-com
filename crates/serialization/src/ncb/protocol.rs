//! NCB implementations of the shared COM format sessions and their bounded state machine.

use super::*;
use neocogi_com::{NcCopyFromBufferOffset, NcStatus, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
use neocogi_com::serialization::{ISerializer, IDeserializer};

/// A session starts active and ends permanently on either success or first failure.
#[derive(Clone, Copy)]
pub(super) enum Phase {
    /// Exactly one root may still be traversed.
    Active,
    /// The first failure is retained without allowing later operations to replace it.
    Failed(Error),
    /// The root is complete and output may be observed.
    Finished,
}
/// Matching begin/end operations are part of the session contract.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Container {
    /// Named fields precede their values.
    Record,
    /// An exact number of ordered elements follows.
    Sequence,
    /// Exactly twice the pair count follows as keys and values.
    Map,
    /// A present option contains exactly one child; absence contains none.
    Option,
    /// A case has at most one typed payload value.
    Variant,
}
/// Structural state for one currently open value.
#[derive(Clone, Copy)]
enum Value {
    /// A type has entered but has not selected its representation.
    Empty,
    /// One scalar or completed container has been acquired.
    Done,
    /// One validated UTF-8 string ends at the cursor and awaits its single copy.
    StringPending(u32),
    /// This decoder already read the ordinal and may probe another candidate.
    VariantProbe(u32),
    /// Named fields must each be selected before acquiring their value.
    Record { remaining: usize, selected: bool },
    /// Sequence elements, map keys/values or an optional child remain.
    Ordered { kind: Container, remaining: usize },
    /// Enum cases have zero or one payload value.
    Variant { children: usize },
}
/// Finite structural stack shared by writing and reading, without heap allocation.
/// The protocol permits at most 128 nested values even if a caller supplies a larger limit.
pub(super) struct Protocol {
    /// Open values; only the initialized prefix participates in the traversal.
    values: [Value; 128],
    /// Number of open entries.
    depth: usize,
    /// Whether the sole root has already been opened.
    root: bool,
}
impl Protocol {
    /// Starts with no open values or consumed root.
    pub(super) fn new() -> Self {
        Self {
            values: [Value::Empty; 128],
            depth: 0,
            root: false,
        }
    }
    /// Reports the only authoritative open value depth.
    pub(super) fn open_depth(&self) -> usize {
        self.depth
    }

    /// Reserves one child before placing its fresh value on the bounded stack.
    pub(super) fn enter(&mut self) -> Result<(), &'static str> {
        if self.depth == self.values.len() {
            return Err("value or nesting limit");
        }
        if self.depth == 0 {
            if self.root {
                return Err("multiple root values");
            }
            self.root = true;
        } else {
            match &mut self.values[self.depth - 1] {
                Value::Record { remaining, selected } if *remaining > 0 && *selected => {
                    *remaining -= 1;
                    *selected = false;
                }
                Value::Ordered { remaining, .. } if *remaining > 0 => *remaining -= 1,
                Value::Variant { children } if *children == 0 => *children = 1,
                _ => return Err("unexpected child value"),
            }
        }
        self.values[self.depth] = Value::Empty;
        self.depth += 1;
        Ok(())
    }
    /// Checks complete consumption before dropping the current value scope.
    pub(super) fn leave(&mut self) -> Result<(), &'static str> {
        self.complete()?;
        self.depth -= 1;
        Ok(())
    }
    /// Source records, sequences and maps must close explicitly before leaving
    /// their typed scope. Options and cases have no separate source end method.
    fn leave_source(&mut self) -> Result<(), &'static str> {
        self.complete()?;
        match self.values.get(self.depth.wrapping_sub(1)) {
            Some(Value::Done | Value::Variant { .. } | Value::Ordered { kind: Container::Option, remaining: 0 }) => self.leave(),
            _ => Err("unclosed container"),
        }
    }
    /// Returns the active frame before any representation-specific operation.
    fn current(&self) -> Result<Value, &'static str> {
        self.values.get(self.depth.wrapping_sub(1)).copied().ok_or("no active value")
    }
    /// Checks that materializing a representation cannot replace an open one.
    fn require_empty(&self) -> Result<(), &'static str> {
        match self.current()? {
            Value::Empty => Ok(()),
            _ => Err("value already consumed"),
        }
    }
    /// Advances the active frame's representation after its specific admission.
    /// The caller has already checked the active index and does not enter or
    /// leave a scope between that check and this assignment.
    fn set_current(&mut self, value: Value) {
        self.values[self.depth - 1] = value;
    }
    /// Selects the representation of an otherwise empty typed value.
    fn select(&mut self, value: Value) -> Result<(), &'static str> {
        self.require_empty()?;
        self.set_current(value);
        Ok(())
    }
    /// Validates a named field selection and reports genuine absent trailing fields.
    fn field(&mut self) -> Result<bool, &'static str> {
        match self.values.get_mut(self.depth.wrapping_sub(1)) {
            Some(Value::Record { remaining, selected }) if !*selected => {
                if *remaining == 0 {
                    Ok(false)
                } else {
                    *selected = true;
                    Ok(true)
                }
            }
            _ => Err("invalid field selection"),
        }
    }
    /// Validates that all children required by the current representation were read/written.
    fn complete(&self) -> Result<(), &'static str> {
        match self.values.get(self.depth.wrapping_sub(1)) {
            Some(Value::Done | Value::Variant { .. }) => Ok(()),
            Some(Value::Record { remaining: 0, selected: false } | Value::Ordered { remaining: 0, .. }) => Ok(()),
            Some(Value::Record { .. }) => Err("unconsumed record fields"),
            Some(Value::Ordered { .. }) => Err("unconsumed sequence elements"),
            _ => Err("incomplete value"),
        }
    }
    /// Closes a container before its enclosing typed value scope is left.
    fn check_container(&self, expected: Container) -> Result<(), &'static str> {
        let actual = match self.values.get(self.depth.wrapping_sub(1)) {
            Some(Value::Record { .. }) => Container::Record,
            Some(Value::Ordered { kind, .. }) => *kind,
            Some(Value::Variant { .. }) => Container::Variant,
            _ => return Err("no active container"),
        };
        if actual != expected {
            return Err("mismatched container end");
        }
        Ok(())
    }
    /// Completes a specifically matched container before its typed scope ends.
    fn end(&mut self, expected: Container) -> Result<(), &'static str> {
        self.check_container(expected)?;
        self.complete()?;
        self.values[self.depth - 1] = Value::Done;
        Ok(())
    }
    /// Checks the sole root and balanced scopes at the document commit boundary.
    fn finish(&self) -> Result<(), &'static str> {
        if self.root && self.depth == 0 { Ok(()) } else { Err("incomplete document") }
    }
}
/// Chooses stable schema diagnostics without borrowing arbitrary foreign message storage.
fn schema_message(message: &str) -> &'static str {
    match message {
        "missing field" => "missing field",
        "unknown enum variant" => "unknown enum variant",
        "invalid sequence length" => "invalid sequence length",
        "unknown record fields" => "unknown record fields",
        "unconsumed record fields" => "unconsumed record fields",
        "unconsumed sequence elements" => "unconsumed sequence elements",
        "duplicate map key" => "duplicate map key",
        "invalid UTF-8" => "invalid UTF-8",
        "invalid string length" => "invalid string length",
        "usize overflow" => "usize overflow",
        "isize overflow" => "isize overflow",
        "length exceeds u32" => "length exceeds u32",
        "collection element limit" => "collection element limit",
        "document byte limit" => "document byte limit",
        _ => "invalid persistent value",
    }
}
/// Maps an NCB diagnostic to the public COM status without exposing Rust errors.
pub(super) fn status(error: Error) -> NcStatus {
    if error.message.contains("limit") || error.message == "output buffer too small" {
        NC_E_OUT_OF_RANGE
    } else {
        NC_E_INVALID_ARG
    }
}
impl Encoder<'_> {
    /// Returns the original failed diagnostic or a schema failure at the current position.
    pub(super) fn failure(&self, fallback: &'static str) -> Error {
        match self.phase {
            Phase::Failed(error) => error,
            _ => self.error(fallback),
        }
    }
    /// Executes one operation or preserves the terminal session state on failure.
    fn run<T>(&mut self, operation: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, NcStatus> {
        match self.phase {
            Phase::Active => {}
            Phase::Failed(error) => return Err(status(error)),
            Phase::Finished => return Err(NC_E_INVALID_ARG),
        }
        operation(self).map_err(|error| {
            self.phase = Phase::Failed(error);
            status(error)
        })
    }
    /// Sets the newly opened container's exact semantic representation.
    fn shape(&mut self, value: Value) -> Result<(), Error> {
        self.protocol.select(value).map_err(|message| self.error(message))
    }
    /// Marks the current raw scalar as complete before leaving its value scope.
    pub(super) fn scalar_shape(&mut self) -> Result<(), Error> {
        self.shape(Value::Done)
    }
    /// Reports a completed byte document without transferring allocator ownership.
    pub fn bytes(&self) -> Result<&[u8], Error> {
        if !matches!(self.phase, Phase::Finished) {
            return Err(self.failure("document is not finished"));
        }
        match &self.output {
            #[cfg(feature = "alloc")]
            Output::Owned(bytes) => Ok(bytes),
            Output::Slice(bytes) => Ok(&bytes[..self.offset]),
            Output::Count => Err(self.error("counting output has no bytes")),
        }
    }
    /// Returns the retained format diagnostic after a failed COM operation.
    pub fn last_error(&self) -> Option<Error> {
        match self.phase {
            Phase::Failed(error) => Some(error),
            _ => None,
        }
    }
}
#[cfg(feature = "alloc")]
impl Encoder<'static> {
    /// Creates an owned COM-provider state with the NCB marker already admitted.
    pub fn new(limits: Limits) -> Result<Self, Error> {
        let mut writer = Self {
            output: Output::Owned(Vec::new()),
            offset: 0,
            budget: Budget::new(limits),
            protocol: Protocol::new(),
            phase: Phase::Active,
        };
        writer.write(MAGIC)?;
        Ok(writer)
    }
}
#[cfg(feature = "alloc")]
impl Encoder<'_> {
    /// Recovers complete provider-owned bytes after the COM session has finished.
    pub fn into_vec(self) -> Result<Vec<u8>, Error> {
        if !matches!(self.phase, Phase::Finished) {
            return Err(self.failure("document is not finished"));
        }
        match self.output {
            Output::Owned(bytes) => Ok(bytes),
            _ => Err(Error {
                offset: self.offset,
                message: "output is not owned",
            }),
        }
    }
}
impl Decoder<'_> {
    /// Returns the first format failure, preserving exact NCB error offsets.
    pub(super) fn failure(&self, fallback: &'static str) -> Error {
        match self.phase {
            Phase::Failed(error) => error,
            _ => self.error(fallback),
        }
    }
    /// Executes only active operations and makes every format failure terminal.
    fn run<T>(&mut self, operation: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, NcStatus> {
        match self.phase {
            Phase::Active => {}
            Phase::Failed(error) => return Err(status(error)),
            Phase::Finished => return Err(NC_E_INVALID_ARG),
        }
        operation(self).map_err(|error| {
            self.phase = Phase::Failed(error);
            status(error)
        })
    }
    /// Reads a fresh representation after checking the active frame. The byte
    /// operation may update the cursor and budget but cannot change scopes.
    fn read_value<T>(&mut self, read: impl FnOnce(&mut Self) -> Result<(Value, T), Error>) -> Result<T, NcStatus> {
        self.run(|reader| {
            reader.protocol.require_empty().map_err(|message| reader.error(message))?;
            let (value, result) = read(reader)?;
            reader.protocol.set_current(value);
            Ok(result)
        })
    }
    /// All primitive reads share phase admission and one successful completion.
    fn read_scalar<T>(&mut self, read: impl FnOnce(&mut Self) -> Result<T, Error>) -> Result<T, NcStatus> {
        self.read_value(|reader| Ok((Value::Done, read(reader)?)))
    }
    /// Returns the first failed source diagnostic for local format-specific tools.
    pub fn last_error(&self) -> Option<Error> {
        match self.phase {
            Phase::Failed(error) => Some(error),
            _ => None,
        }
    }
}
impl<'a> Decoder<'a> {
    /// Admits a borrowed document without allocating or copying it.
    pub(super) fn borrowed(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        Self::input(Input::Borrowed(bytes), limits)
    }
    /// Validates the complete input size and its format marker before publication.
    fn input(bytes: Input<'a>, limits: Limits) -> Result<Self, Error> {
        let mut reader = Self {
            bytes,
            offset: 0,
            budget: Budget::new(limits),
            protocol: Protocol::new(),
            phase: Phase::Active,
        };
        if reader.bytes.as_slice().len() > limits.bytes {
            return Err(reader.error("document byte limit"));
        }
        if reader.take(4)? != MAGIC {
            return Err(reader.error("unsupported binary format"));
        }
        Ok(reader)
    }
}
#[cfg(feature = "alloc")]
impl Decoder<'static> {
    /// Owns a bounded complete input document for a retainable COM session.
    pub fn new(bytes: Vec<u8>, limits: Limits) -> Result<Self, Error> {
        Self::input(Input::Owned(bytes), limits)
    }
}

impl ISerializer for Encoder<'_> {
    fn serialize_unit(&mut self) -> Result<(), NcStatus> {
        self.run(Self::serialize_unit)
    }
    fn serialize_bool(&mut self, value: bool) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_bool(value))
    }
    fn serialize_u8(&mut self, value: u8) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_u8(value))
    }
    fn serialize_u16(&mut self, value: u16) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_u16(value))
    }
    fn serialize_u32(&mut self, value: u32) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_u32(value))
    }
    fn serialize_u64(&mut self, value: u64) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_u64(value))
    }
    fn serialize_i8(&mut self, value: i8) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_i8(value))
    }
    fn serialize_i16(&mut self, value: i16) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_i16(value))
    }
    fn serialize_i32(&mut self, value: i32) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_i32(value))
    }
    fn serialize_i64(&mut self, value: i64) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_i64(value))
    }
    fn serialize_f32(&mut self, value: f32) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_f32(value))
    }
    fn serialize_f64(&mut self, value: f64) -> Result<(), NcStatus> {
        self.run(|writer| writer.serialize_f64(value))
    }
    fn serialize_string(&mut self, value: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.run(|writer| {
            let value = read_text(value).map_err(|message| writer.error(message))?;
            writer.serialize_string(value)
        })
    }
    fn begin_option(&mut self, present: bool) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.enter()?;
            writer.shape(Value::Ordered {
                kind: Container::Option,
                remaining: usize::from(present),
            })?;
            writer.write(&[u8::from(present)])
        })
    }
    fn end_option(&mut self) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.protocol.check_container(Container::Option).map_err(|message| writer.error(message))?;
            writer.leave()
        })
    }
    fn begin_record(&mut self, count: u32) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.begin_sequence(count as usize)?;
            writer.shape(Value::Record {
                remaining: count as usize,
                selected: false,
            })
        })
    }
    fn field(&mut self, name: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.run(|writer| {
            read_text(name).map_err(|message| writer.error(message))?;
            if !writer.protocol.field().map_err(|message| writer.error(message))? {
                return Err(writer.error("unknown record fields"));
            }
            Ok(())
        })
    }
    fn end_record(&mut self) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.protocol.check_container(Container::Record).map_err(|message| writer.error(message))?;
            writer.end_sequence()
        })
    }
    fn begin_sequence(&mut self, count: u32) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.begin_sequence(count as usize)?;
            writer.shape(Value::Ordered {
                kind: Container::Sequence,
                remaining: count as usize,
            })
        })
    }
    fn end_sequence(&mut self) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.protocol.check_container(Container::Sequence).map_err(|message| writer.error(message))?;
            writer.end_sequence()
        })
    }
    fn begin_map(&mut self, count: u32) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.begin_map(count as usize)?;
            writer.shape(Value::Ordered {
                kind: Container::Map,
                remaining: (count as usize).checked_mul(2).ok_or_else(|| writer.error("collection count overflow"))?,
            })
        })
    }
    fn end_map(&mut self) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.protocol.check_container(Container::Map).map_err(|message| writer.error(message))?;
            writer.end_map()
        })
    }
    fn begin_variant(&mut self, index: u32, name: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.run(|writer| {
            read_text(name).map_err(|message| writer.error(message))?;
            writer.begin_variant(index)?;
            writer.shape(Value::Variant { children: 0 })
        })
    }
    fn end_variant(&mut self) -> Result<(), NcStatus> {
        self.run(|writer| {
            writer.protocol.check_container(Container::Variant).map_err(|message| writer.error(message))?;
            writer.end_variant()
        })
    }
    fn fail(&mut self, message: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.run(|writer| {
            let message = read_text(message).map_err(|message| writer.error(message))?;
            Err(writer.error(schema_message(message)))
        })
    }
    fn finish(&mut self) -> Result<(), NcStatus> {
        self.run(|writer| writer.protocol.finish().map_err(|message| writer.error(message)))?;
        self.phase = Phase::Finished;
        Ok(())
    }
}
impl IDeserializer for Decoder<'_> {
    fn enter_value(&mut self) -> Result<(), NcStatus> {
        self.run(|reader| {
            reader.budget.enter(reader.offset, reader.protocol.open_depth())?;
            reader.protocol.enter().map_err(|message| reader.error(message))
        })
    }
    fn leave_value(&mut self) -> Result<(), NcStatus> {
        self.run(|reader| reader.protocol.leave_source().map_err(|message| reader.error(message)))
    }
    fn deserialize_unit(&mut self) -> Result<(), NcStatus> {
        self.read_scalar(Self::deserialize_unit)
    }
    fn deserialize_bool(&mut self) -> Result<bool, NcStatus> {
        self.read_scalar(Self::deserialize_bool)
    }
    fn deserialize_u8(&mut self) -> Result<u8, NcStatus> {
        self.read_scalar(Self::deserialize_u8)
    }
    fn deserialize_u16(&mut self) -> Result<u16, NcStatus> {
        self.read_scalar(Self::deserialize_u16)
    }
    fn deserialize_u32(&mut self) -> Result<u32, NcStatus> {
        self.read_scalar(Self::deserialize_u32)
    }
    fn deserialize_u64(&mut self) -> Result<u64, NcStatus> {
        self.read_scalar(Self::deserialize_u64)
    }
    fn deserialize_i8(&mut self) -> Result<i8, NcStatus> {
        self.read_scalar(Self::deserialize_i8)
    }
    fn deserialize_i16(&mut self) -> Result<i16, NcStatus> {
        self.read_scalar(Self::deserialize_i16)
    }
    fn deserialize_i32(&mut self) -> Result<i32, NcStatus> {
        self.read_scalar(Self::deserialize_i32)
    }
    fn deserialize_i64(&mut self) -> Result<i64, NcStatus> {
        self.read_scalar(Self::deserialize_i64)
    }
    fn deserialize_f32(&mut self) -> Result<f32, NcStatus> {
        self.read_scalar(Self::deserialize_f32)
    }
    fn deserialize_f64(&mut self) -> Result<f64, NcStatus> {
        self.read_scalar(Self::deserialize_f64)
    }
    fn string_len(&mut self) -> Result<u32, NcStatus> {
        self.read_value(|reader| {
            let len = reader.deserialize_u32()?;
            let start = reader.offset;
            let text = reader.take(len as usize)?;
            core::str::from_utf8(text).map_err(|_| Error {
                offset: start + len as usize,
                message: "invalid UTF-8",
            })?;
            Ok((Value::StringPending(len), len))
        })
    }
    fn copy_string(&mut self, mut output: neocogi_com::NcOutputWriter<'_, u8>) -> Result<u32, NcStatus> {
        self.run(|reader| {
            if output.offset() != 0 {
                return Err(reader.error("invalid output offset"));
            }
            let Ok(Value::StringPending(len)) = reader.protocol.current() else {
                return Err(reader.error("no admitted string"));
            };
            // No other frame state can read input until the copy completes, so
            // the admitted string still ends at the same cursor position.
            let end = reader.offset;
            let start = end - len as usize;
            if output.remaining() < len as usize {
                return Err(reader.error("output buffer too small"));
            }
            output
                .extend_from_slice(&reader.bytes.as_slice()[start..end])
                .map_err(|_| reader.error("output buffer too small"))?;
            reader.protocol.set_current(Value::Done);
            Ok(len)
        })
    }
    fn option_present(&mut self) -> Result<bool, NcStatus> {
        self.read_value(|reader| {
            let present = reader.deserialize_bool()?;
            Ok((
                Value::Ordered {
                    kind: Container::Option,
                    remaining: usize::from(present),
                },
                present,
            ))
        })
    }
    fn begin_record(&mut self, maximum: u32) -> Result<u32, NcStatus> {
        self.read_value(|reader| {
            let count = reader.count(1)?;
            if count > maximum as usize {
                return Err(reader.error("unknown record fields"));
            }
            Ok((Value::Record { remaining: count, selected: false }, count as u32))
        })
    }
    fn field(&mut self, name: NcCopyFromBufferOffset<'_, u8>) -> Result<bool, NcStatus> {
        self.run(|reader| {
            read_text(name).map_err(|message| reader.error(message))?;
            reader.protocol.field().map_err(|message| reader.error(message))
        })
    }
    fn end_record(&mut self) -> Result<(), NcStatus> {
        self.run(|reader| reader.protocol.end(Container::Record).map_err(|message| reader.error(message)))
    }
    fn begin_sequence(&mut self) -> Result<u32, NcStatus> {
        self.read_value(|reader| {
            let count = reader.count(1)?;
            Ok((
                Value::Ordered {
                    kind: Container::Sequence,
                    remaining: count,
                },
                count as u32,
            ))
        })
    }
    fn end_sequence(&mut self) -> Result<(), NcStatus> {
        self.run(|reader| reader.protocol.end(Container::Sequence).map_err(|message| reader.error(message)))
    }
    fn begin_map(&mut self) -> Result<u32, NcStatus> {
        self.read_value(|reader| {
            let count = reader.count(2)?;
            let remaining = count.checked_mul(2).ok_or_else(|| reader.error("collection count overflow"))?;
            Ok((Value::Ordered { kind: Container::Map, remaining }, count as u32))
        })
    }
    fn end_map(&mut self) -> Result<(), NcStatus> {
        self.run(|reader| reader.protocol.end(Container::Map).map_err(|message| reader.error(message)))
    }
    fn variant_is(&mut self, index: u32, name: NcCopyFromBufferOffset<'_, u8>) -> Result<bool, NcStatus> {
        self.run(|reader| {
            read_text(name).map_err(|message| reader.error(message))?;
            let ordinal = match reader.protocol.current().map_err(|message| reader.error(message))? {
                Value::Empty => reader.deserialize_u32()?,
                Value::VariantProbe(ordinal) => ordinal,
                _ => return Err(reader.error("value already consumed")),
            };
            let matched = ordinal == index;
            reader.protocol.set_current(if matched {
                Value::Variant { children: 0 }
            } else {
                Value::VariantProbe(ordinal)
            });
            Ok(matched)
        })
    }
    fn fail(&mut self, message: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        self.run(|reader| {
            let message = read_text(message).map_err(|message| reader.error(message))?;
            Err(reader.error(schema_message(message)))
        })
    }
    fn finish(&mut self) -> Result<(), NcStatus> {
        self.run(|reader| {
            reader.protocol.finish().map_err(|message| reader.error(message))?;
            if reader.offset != reader.bytes.as_slice().len() {
                return Err(reader.error("trailing bytes"));
            }
            Ok(())
        })?;
        self.phase = Phase::Finished;
        Ok(())
    }
}

/// Validates whole-string buffer semantics before a format consumes text.
fn read_text(value: NcCopyFromBufferOffset<'_, u8>) -> Result<&str, &'static str> {
    if value.offset() != 0 {
        return Err("invalid text offset");
    }
    core::str::from_utf8(value.as_slice()).map_err(|_| "invalid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use neocogi_com::serialization::{scope::ScopedComProvider, ISerializerAbi, ScopedSerializer};

    /// Calls the generated scoped deserializer and recovers the private cursor.
    fn decode_scoped<'a>(bytes: &'a [u8], call: impl FnOnce(neocogi_com::ComRef<'_, neocogi_com::serialization::IDeserializerAbi>)) -> Decoder<'a> {
        let scope = ScopedComProvider::new(Decoder::borrowed(bytes, Limits::default()).unwrap());
        // SAFETY: the generated table matches this exact retained decoder scope.
        let view = unsafe { scope.interface(&neocogi_com::serialization::ScopedDeserializer::<Decoder<'_>>::VTABLE) };
        call(view);
        scope.into_inner()
    }

    /// The real scoped prefix rejects out-of-phase byte reads before cursor movement.
    #[test]
    fn wrong_phase_byte_operations_preserve_the_cursor_and_first_failure() {
        type Call = fn(&mut neocogi_com::ComRef<'_, neocogi_com::serialization::IDeserializerAbi>) -> Result<(), NcStatus>;
        let calls: [Call; 7] = [
            |view| view.deserialize_u8().map(|_| ()),
            |view| view.option_present().map(|_| ()),
            |view| view.begin_record(1).map(|_| ()),
            |view| view.begin_sequence().map(|_| ()),
            |view| view.begin_map().map(|_| ()),
            |view| view.string_len().map(|_| ()),
            |view| view.variant_is(0, NcCopyFromBufferOffset::new(0, b"First")).map(|_| ()),
        ];
        for (entered, position, reason) in [(false, 4, "no active value"), (true, 5, "value already consumed")] {
            for call in calls {
                let reader = decode_scoped(b"NCB\x01\x01", |mut view| {
                    if entered {
                        view.enter_value().unwrap();
                        assert_eq!(view.deserialize_u8().unwrap(), 1);
                    }
                    assert_eq!(call(&mut view), Err(NC_E_INVALID_ARG));
                    assert_eq!(view.finish(), Err(NC_E_INVALID_ARG));
                });
                assert_eq!(reader.offset, position);
                assert_eq!(reader.last_error(), Some(Error { message: reason, offset: position }));
            }
        }
        // In a valid phase, malformed/truncated framing still reports its own error.
        let reader = decode_scoped(b"NCB\x01", |mut view| {
            view.enter_value().unwrap();
            assert_eq!(view.begin_sequence(), Err(NC_E_INVALID_ARG));
        });
        assert_eq!(reader.last_error().unwrap().message, "truncated value");
        let reader = decode_scoped(b"NCB\x01\x01\0\x01\0", |mut view| {
            view.enter_value().unwrap();
            assert_eq!(view.begin_sequence(), Err(NC_E_OUT_OF_RANGE));
        });
        assert_eq!(reader.last_error().unwrap().message, "collection or value limit");
    }

    /// A miss permits another probe of the same tag. Other operations cannot
    /// consume bytes or complete the frame before that tag is selected.
    #[test]
    fn pending_variant_reuses_the_tag_and_requires_selection() {
        let first = NcCopyFromBufferOffset::new(0, b"First");
        let second = NcCopyFromBufferOffset::new(0, b"Second");
        let reader = decode_scoped(b"NCB\x01\x01\0\0\0\x17", |mut view| {
            view.enter_value().unwrap();
            assert_eq!(view.variant_is(0, first), Ok(false));
            assert_eq!(view.variant_is(1, second), Ok(true));
            view.enter_value().unwrap();
            assert_eq!(view.deserialize_u8(), Ok(23));
            view.leave_value().unwrap();
            view.leave_value().unwrap();
            view.finish().unwrap();
        });
        assert!(reader.last_error().is_none());

        for probe_matches in [false, true] {
            let reader = decode_scoped(b"NCB\x01\x01\0\0\0", |mut view| {
                view.enter_value().unwrap();
                assert_eq!(view.variant_is(u32::from(probe_matches), second), Ok(probe_matches));
                if probe_matches {
                    assert_eq!(view.variant_is(0, first), Err(NC_E_INVALID_ARG));
                } else {
                    assert_eq!(view.leave_value(), Err(NC_E_INVALID_ARG));
                }
                assert_eq!(view.finish(), Err(NC_E_INVALID_ARG));
            });
            assert_eq!(reader.offset, 8);
        }
    }

    /// Pending strings retain their validated window until copied into fixed storage.
    #[test]
    fn pending_string_copies_once_into_stack_slots() {
        use core::mem::MaybeUninit;
        use neocogi_com::NcOutput;

        for (bytes, expected) in [(b"NCB\x01\0\0\0\0".as_slice(), ""), (b"NCB\x01\x02\0\0\0\xc3\xa9".as_slice(), "é")] {
            let reader = decode_scoped(bytes, |mut view| {
                view.enter_value().unwrap();
                assert_eq!(view.string_len(), Ok(expected.len() as u32));
                let mut slots = [MaybeUninit::new(0_u8); 2];
                let mut count = 0;
                assert_eq!(
                    view.copy_string(NcOutput::from_slots(0, &mut slots, &mut count).unwrap()),
                    Ok(expected.len() as u32)
                );
                assert_eq!(count, expected.len());
                // SAFETY: the fixture began with initialized bytes and the view only writes u8.
                let copied = slots.map(|slot| unsafe { slot.assume_init() });
                assert_eq!(&copied[..count], expected.as_bytes());
                view.leave_value().unwrap();
                view.finish().unwrap();
            });
            assert_eq!(reader.offset, bytes.len());
        }
        let bytes = b"NCB\x01\x02\0\0\0hi";
        for (offset, capacity, status) in [(0, 1, NC_E_OUT_OF_RANGE), (1, 2, NC_E_INVALID_ARG)] {
            let reader = decode_scoped(bytes, |mut view| {
                view.enter_value().unwrap();
                assert_eq!(view.string_len(), Ok(2));
                let mut slots = [MaybeUninit::uninit(); 2];
                let mut count = 9;
                assert_eq!(
                    view.copy_string(NcOutput::from_slots(offset, &mut slots[..capacity], &mut count).unwrap()),
                    Err(status)
                );
                assert_eq!(count, 0);
                assert_eq!(view.copy_string(NcOutput::from_slots(0, &mut slots, &mut count).unwrap()), Err(status));
            });
            assert_eq!(reader.offset, bytes.len());
        }
        let reader = decode_scoped(bytes, |mut view| {
            view.enter_value().unwrap();
            assert_eq!(view.string_len(), Ok(2));
            assert_eq!(view.begin_map(), Err(NC_E_INVALID_ARG));
        });
        assert_eq!(reader.offset, bytes.len());
        assert_eq!(reader.last_error().unwrap().message, "value already consumed");
        let reader = decode_scoped(bytes, |mut view| {
            view.enter_value().unwrap();
            assert_eq!(view.string_len(), Ok(2));
            let mut slots = [MaybeUninit::uninit(); 2];
            let mut count = 0;
            assert_eq!(view.copy_string(NcOutput::from_slots(0, &mut slots, &mut count).unwrap()), Ok(2));
            assert_eq!(
                view.copy_string(NcOutput::from_slots(0, &mut slots, &mut count).unwrap()),
                Err(NC_E_INVALID_ARG)
            );
            assert_eq!(count, 0);
        });
        assert_eq!(reader.last_error().unwrap().message, "no admitted string");
    }

    /// Counting storage exercises real format dispatch without any heap allocation.
    fn writer() -> Encoder<'static> {
        Encoder {
            output: Output::Count,
            offset: 0,
            budget: Budget::new(Limits::default()),
            protocol: Protocol::new(),
            phase: Phase::Active,
        }
    }

    /// Borrowed scopes cannot acquire ownership or weak references and obey their ABI gate.
    #[test]
    fn scoped_capabilities_reject_retention_and_reentrant_mutation() {
        let scope = ScopedComProvider::new(writer());
        // SAFETY: generated adapters use exactly this ScopedComProvider<Encoder> type.
        let mut interface: neocogi_com::ComRef<'_, ISerializerAbi> = unsafe { scope.interface(&ScopedSerializer::<Encoder<'_>>::VTABLE) };
        assert_eq!(interface.try_add_ref().unwrap_err(), neocogi_com::NC_E_INVALID_ARG);
        assert_eq!(interface.downgrade().unwrap_err(), neocogi_com::NC_E_NOINTERFACE);
        let raw = interface.as_raw();
        let mut output = raw.cast::<neocogi_com::IUnknown>();
        // SAFETY: the live capability and a real writable output slot meet the raw contract.
        let status = unsafe { ((*raw.vtable).unknown.query_interface)(raw.object, &neocogi_com::IUnknown::INTERFACE_ID, &mut output) };
        assert_eq!(status, neocogi_com::NC_E_INVALID_ARG);
        assert!(output.is_null());
        // SAFETY: this scope and the generated table share the exact provider type.
        let lease = unsafe { ScopedComProvider::<Encoder<'_>>::borrow_mut(raw.object) }.unwrap();
        assert_eq!(interface.serialize_u32(7), Err(neocogi_com::NC_E_BUSY));
        drop(lease);
        interface.serialize_u32(7).unwrap();
        interface.finish().unwrap();
    }

    /// ABI validation rejects malformed readable values before entering or poisoning the format.
    #[test]
    fn malformed_bool_wire_is_rejected_before_provider_state_changes() {
        let scope = ScopedComProvider::new(writer());
        // SAFETY: the exact generated table lends this retained scope's state.
        let mut interface: neocogi_com::ComRef<'_, ISerializerAbi> = unsafe { scope.interface(&ScopedSerializer::<Encoder<'_>>::VTABLE) };
        let raw = interface.as_raw();
        // SAFETY: only the scalar wire is malformed; the object/table pair is real and live.
        assert_eq!(unsafe { ((*raw.vtable).serialize_bool)(raw.object, 2) }, neocogi_com::NC_E_INVALID_ARG);
        interface.serialize_bool(true).unwrap();
        interface.finish().unwrap();
    }
}
