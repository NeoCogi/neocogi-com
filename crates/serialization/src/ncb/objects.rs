//! Owned NCB COM objects. Ordinary mutable object gates serialize foreign calls.
use neocogi_com::serialization::*;
use crate::ncb as binary;
use neocogi_com::{ComBox, NcStatus, NC_E_INVALID_ARG, NcOutputWriter};

/// Owns one NCB document session; finishing freezes its bytes for output queries.
#[neocogi_com::com_object(interfaces(ISerializer, ISerializationOutput, ISerializationDiagnostics), ownership = exclusive)]
pub struct NcbSerializer {
    /// Concrete codec state, accessed through the ordinary object-wide call gate.
    encoder: binary::Encoder<'static>,
    /// Optional application message copied only on a schema failure.
    message: Option<neocogi_com::__alloc::string::String>,
}

/// Owns immutable input and a mutable typed decoding cursor.
#[neocogi_com::com_object(interfaces(IDeserializer, ISerializationDiagnostics), ownership = exclusive)]
pub struct NcbDeserializer {
    /// Owned input remains valid through every call and is released with the object.
    source: Source,
    /// Optional application schema message, never a borrowed foreign string.
    message: Option<neocogi_com::__alloc::string::String>,
}

/// Stateless shared factory whose document sessions each have exclusive ownership.
#[neocogi_com::com_object(ISerializationFormat)]
pub struct NcbFormat;

impl NcbSerializer {
    /// Allocates the sole owner of a fresh NCB format object; no host is linked.
    pub fn create(limits: Limits) -> Result<ComBox<ISerializerAbi>, binary::Error> {
        let value = Self {
            encoder: binary::Encoder::new(limits)?,
            message: None,
        };
        Ok(Self::into_box(value)
            .query()
            .unwrap_or_else(|_| unreachable!("generated serializer projection")))
    }
}
impl NcbDeserializer {
    /// Allocates one exclusive source object after admitting the document header.
    pub fn create(bytes: neocogi_com::__alloc::vec::Vec<u8>, limits: Limits) -> Result<ComBox<IDeserializerAbi>, binary::Error> {
        let value = Self {
            source: Source::Ready(binary::Decoder::new(bytes, limits)?),
            message: None,
        };
        Ok(Self::into_box(value)
            .query()
            .unwrap_or_else(|_| unreachable!("generated deserializer projection")))
    }
}
impl ISerializer for NcbSerializer {
    /// Delegates serialize_unit to the canonical NCB format state.
    fn serialize_unit(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_unit(&mut self.encoder)
    }
    /// Delegates serialize_bool to the canonical NCB format state.
    fn serialize_bool(&mut self, value: ::core::primitive::bool) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_bool(&mut self.encoder, value)
    }
    /// Delegates serialize_u8 to the canonical NCB format state.
    fn serialize_u8(&mut self, value: ::core::primitive::u8) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_u8(&mut self.encoder, value)
    }
    /// Delegates serialize_u16 to the canonical NCB format state.
    fn serialize_u16(&mut self, value: ::core::primitive::u16) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_u16(&mut self.encoder, value)
    }
    /// Delegates serialize_u32 to the canonical NCB format state.
    fn serialize_u32(&mut self, value: ::core::primitive::u32) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_u32(&mut self.encoder, value)
    }
    /// Delegates serialize_u64 to the canonical NCB format state.
    fn serialize_u64(&mut self, value: ::core::primitive::u64) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_u64(&mut self.encoder, value)
    }
    /// Delegates serialize_i8 to the canonical NCB format state.
    fn serialize_i8(&mut self, value: ::core::primitive::i8) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_i8(&mut self.encoder, value)
    }
    /// Delegates serialize_i16 to the canonical NCB format state.
    fn serialize_i16(&mut self, value: ::core::primitive::i16) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_i16(&mut self.encoder, value)
    }
    /// Delegates serialize_i32 to the canonical NCB format state.
    fn serialize_i32(&mut self, value: ::core::primitive::i32) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_i32(&mut self.encoder, value)
    }
    /// Delegates serialize_i64 to the canonical NCB format state.
    fn serialize_i64(&mut self, value: ::core::primitive::i64) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_i64(&mut self.encoder, value)
    }
    /// Delegates serialize_f32 to the canonical NCB format state.
    fn serialize_f32(&mut self, value: ::core::primitive::f32) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_f32(&mut self.encoder, value)
    }
    /// Delegates serialize_f64 to the canonical NCB format state.
    fn serialize_f64(&mut self, value: ::core::primitive::f64) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_f64(&mut self.encoder, value)
    }
    /// Delegates serialize_string to the canonical NCB format state.
    fn serialize_string(
        &mut self,
        value: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::serialize_string(&mut self.encoder, value)
    }
    /// Delegates begin_option to the canonical NCB format state.
    fn begin_option(&mut self, present: ::core::primitive::bool) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::begin_option(&mut self.encoder, present)
    }
    /// Delegates end_option to the canonical NCB format state.
    fn end_option(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::end_option(&mut self.encoder)
    }
    /// Delegates begin_record to the canonical NCB format state.
    fn begin_record(&mut self, fields: ::core::primitive::u32) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::begin_record(&mut self.encoder, fields)
    }
    /// Delegates field to the canonical NCB format state.
    fn field(&mut self, name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::field(&mut self.encoder, name)
    }
    /// Delegates end_record to the canonical NCB format state.
    fn end_record(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::end_record(&mut self.encoder)
    }
    /// Delegates begin_sequence to the canonical NCB format state.
    fn begin_sequence(&mut self, length: ::core::primitive::u32) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::begin_sequence(&mut self.encoder, length)
    }
    /// Delegates end_sequence to the canonical NCB format state.
    fn end_sequence(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::end_sequence(&mut self.encoder)
    }
    /// Delegates begin_map to the canonical NCB format state.
    fn begin_map(&mut self, pairs: ::core::primitive::u32) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::begin_map(&mut self.encoder, pairs)
    }
    /// Delegates end_map to the canonical NCB format state.
    fn end_map(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::end_map(&mut self.encoder)
    }
    /// Delegates begin_variant to the canonical NCB format state.
    fn begin_variant(
        &mut self,
        index: ::core::primitive::u32,
        name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::begin_variant(&mut self.encoder, index, name)
    }
    /// Delegates end_variant to the canonical NCB format state.
    fn end_variant(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::end_variant(&mut self.encoder)
    }
    /// Delegates fail to the canonical NCB format state.
    fn fail(&mut self, message: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        if self.encoder.last_error().is_none() && message.offset() == 0 {
            self.message = core::str::from_utf8(message.as_slice()).ok().map(neocogi_com::__alloc::string::String::from);
        }
        <binary::Encoder as ISerializer>::fail(&mut self.encoder, message)
    }
    /// Delegates finish to the canonical NCB format state.
    fn finish(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Encoder as ISerializer>::finish(&mut self.encoder)
    }
}
impl IDeserializer for NcbDeserializer {
    /// Delegates enter_value to the canonical NCB format state.
    fn enter_value(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::enter_value(self.decoder()?)
    }
    /// Delegates leave_value to the canonical NCB format state.
    fn leave_value(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::leave_value(self.decoder()?)
    }
    /// Delegates deserialize_unit to the canonical NCB format state.
    fn deserialize_unit(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_unit(self.decoder()?)
    }
    /// Delegates deserialize_bool to the canonical NCB format state.
    fn deserialize_bool(&mut self) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_bool(self.decoder()?)
    }
    /// Delegates deserialize_u8 to the canonical NCB format state.
    fn deserialize_u8(&mut self) -> ::core::result::Result<::core::primitive::u8, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_u8(self.decoder()?)
    }
    /// Delegates deserialize_u16 to the canonical NCB format state.
    fn deserialize_u16(&mut self) -> ::core::result::Result<::core::primitive::u16, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_u16(self.decoder()?)
    }
    /// Delegates deserialize_u32 to the canonical NCB format state.
    fn deserialize_u32(&mut self) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_u32(self.decoder()?)
    }
    /// Delegates deserialize_u64 to the canonical NCB format state.
    fn deserialize_u64(&mut self) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_u64(self.decoder()?)
    }
    /// Delegates deserialize_i8 to the canonical NCB format state.
    fn deserialize_i8(&mut self) -> ::core::result::Result<::core::primitive::i8, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_i8(self.decoder()?)
    }
    /// Delegates deserialize_i16 to the canonical NCB format state.
    fn deserialize_i16(&mut self) -> ::core::result::Result<::core::primitive::i16, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_i16(self.decoder()?)
    }
    /// Delegates deserialize_i32 to the canonical NCB format state.
    fn deserialize_i32(&mut self) -> ::core::result::Result<::core::primitive::i32, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_i32(self.decoder()?)
    }
    /// Delegates deserialize_i64 to the canonical NCB format state.
    fn deserialize_i64(&mut self) -> ::core::result::Result<::core::primitive::i64, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_i64(self.decoder()?)
    }
    /// Delegates deserialize_f32 to the canonical NCB format state.
    fn deserialize_f32(&mut self) -> ::core::result::Result<::core::primitive::f32, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_f32(self.decoder()?)
    }
    /// Delegates deserialize_f64 to the canonical NCB format state.
    fn deserialize_f64(&mut self) -> ::core::result::Result<::core::primitive::f64, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::deserialize_f64(self.decoder()?)
    }
    /// Delegates string_len to the canonical NCB format state.
    fn string_len(&mut self) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::string_len(self.decoder()?)
    }
    /// Delegates copy_string to the canonical NCB format state.
    fn copy_string(
        &mut self,
        output: ::neocogi_com::NcOutputWriter<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::copy_string(self.decoder()?, output)
    }
    /// Delegates option_present to the canonical NCB format state.
    fn option_present(&mut self) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::option_present(self.decoder()?)
    }
    /// Delegates begin_record to the canonical NCB format state.
    fn begin_record(&mut self, maximum: ::core::primitive::u32) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::begin_record(self.decoder()?, maximum)
    }
    /// Delegates field to the canonical NCB format state.
    fn field(
        &mut self,
        name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::field(self.decoder()?, name)
    }
    /// Delegates end_record to the canonical NCB format state.
    fn end_record(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::end_record(self.decoder()?)
    }
    /// Delegates begin_sequence to the canonical NCB format state.
    fn begin_sequence(&mut self) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::begin_sequence(self.decoder()?)
    }
    /// Delegates end_sequence to the canonical NCB format state.
    fn end_sequence(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::end_sequence(self.decoder()?)
    }
    /// Delegates begin_map to the canonical NCB format state.
    fn begin_map(&mut self) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::begin_map(self.decoder()?)
    }
    /// Delegates end_map to the canonical NCB format state.
    fn end_map(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::end_map(self.decoder()?)
    }
    /// Delegates variant_is to the canonical NCB format state.
    fn variant_is(
        &mut self,
        index: ::core::primitive::u32,
        name: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::variant_is(self.decoder()?, index, name)
    }
    /// Delegates fail to the canonical NCB format state.
    fn fail(&mut self, message: ::neocogi_com::NcCopyFromBufferOffset<'_, ::core::primitive::u8>) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        if self.details().is_err() && message.offset() == 0 {
            self.message = core::str::from_utf8(message.as_slice()).ok().map(neocogi_com::__alloc::string::String::from);
        }
        <binary::Decoder as IDeserializer>::fail(self.decoder()?, message)
    }
    /// Delegates finish to the canonical NCB format state.
    fn finish(&mut self) -> ::core::result::Result<(), ::neocogi_com::NcStatus> {
        <binary::Decoder as IDeserializer>::finish(self.decoder()?)
    }
}
impl ISerializationOutput for NcbSerializer {
    /// Publishes a byte count only after the format session committed successfully.
    fn byte_len(&self) -> Result<u64, NcStatus> {
        self.encoder.bytes().map(|bytes| bytes.len() as u64).map_err(|_| NC_E_INVALID_ARG)
    }
    /// Copies only a complete document and never publishes a partial prefix.
    fn copy_to(&self, mut output: NcOutputWriter<'_, u8>) -> Result<u64, NcStatus> {
        let bytes = self.encoder.bytes().map_err(|_| NC_E_INVALID_ARG)?;
        if output.offset() != 0 {
            return Err(NC_E_INVALID_ARG);
        }
        output.extend_from_slice(bytes)?;
        Ok(bytes.len() as u64)
    }
}

impl ISerializationFormat for NcbFormat {
    /// Creates a fresh independently bounded NCB writer object.
    fn create_serializer(&self, max_bytes: u64, max_elements: u32, max_depth: u32, max_values: u64) -> Result<ComBox<ISerializerAbi>, NcStatus> {
        NcbSerializer::create(limits(max_bytes, max_elements, max_depth, max_values)?).map_err(|_| neocogi_com::NC_E_INVALID_ARG)
    }
    /// Acquires the caller's complete input before returning a source object.
    fn create_deserializer(
        &self,
        input: neocogi_com::NcCopyFromBufferOffset<'_, u8>,
        max_bytes: u64,
        max_elements: u32,
        max_depth: u32,
        max_values: u64,
    ) -> Result<ComBox<IDeserializerAbi>, NcStatus> {
        let limits = limits(max_bytes, max_elements, max_depth, max_values)?;
        if input.offset() != 0 {
            return Err(neocogi_com::NC_E_INVALID_ARG);
        }
        if input.len() > limits.bytes {
            return Err(neocogi_com::NC_E_OUT_OF_RANGE);
        }
        let mut bytes = neocogi_com::__alloc::vec::Vec::new();
        bytes.try_reserve_exact(input.len()).map_err(|_| neocogi_com::NC_E_OUT_OF_MEMORY)?;
        bytes.extend_from_slice(input.as_slice());
        Ok(NcbDeserializer::session(bytes, limits))
    }
}
/// Validates fixed-width factory limits before local allocation or cursor construction.
fn limits(bytes: u64, elements: u32, depth: u32, values: u64) -> Result<Limits, NcStatus> {
    Ok(Limits {
        bytes: usize::try_from(bytes).map_err(|_| neocogi_com::NC_E_OUT_OF_RANGE)?,
        elements: elements as usize,
        depth: depth as usize,
        values: usize::try_from(values).map_err(|_| neocogi_com::NC_E_OUT_OF_RANGE)?,
    })
}

/// Failed input admission still returns an owning diagnostic-bearing source from
/// the COM factory. The strict local constructor returns its native Error directly.
// Keeping the already allocated COM object's decoder inline avoids a second
// heap allocation and indirection for every successfully decoded document.
#[allow(clippy::large_enum_variant)]
enum Source {
    /// Input header and size were admitted; subsequent failures stay in the decoder.
    Ready(binary::Decoder<'static>),
    /// Initial parsing failed before a decoder could be published.
    Rejected(binary::Error),
}
impl NcbDeserializer {
    /// Creates a source whose first read reports an initial parse failure, if any.
    fn session(bytes: neocogi_com::__alloc::vec::Vec<u8>, limits: Limits) -> ComBox<IDeserializerAbi> {
        let source = match binary::Decoder::new(bytes, limits) {
            Ok(value) => Source::Ready(value),
            Err(error) => Source::Rejected(error),
        };
        Self::into_box(Self { source, message: None })
            .query()
            .unwrap_or_else(|_| unreachable!("NCB source projection"))
    }
    /// Rejects all operations on an initially failed source without losing details.
    fn decoder(&mut self) -> Result<&mut binary::Decoder<'static>, NcStatus> {
        match &mut self.source {
            Source::Ready(value) => Ok(value),
            Source::Rejected(error) => Err(super::protocol::status(*error)),
        }
    }
    /// Borrows the original native diagnostic without allocating during queries.
    fn details(&self) -> Result<crate::diagnostics::Details<'_>, NcStatus> {
        let error = match &self.source {
            Source::Ready(value) => value.last_error(),
            Source::Rejected(error) => Some(*error),
        }
        .ok_or(NC_E_INVALID_ARG)?;
        Ok(crate::diagnostics::Details {
            status: super::protocol::status(error),
            message: self.message.as_deref().unwrap_or(error.message),
            path: "$",
            offset: error.offset as u64,
            line: 0,
            column: 0,
        })
    }
}
impl NcbSerializer {
    /// Exposes the first failed NCB operation; active/finished documents have no error.
    fn details(&self) -> Result<crate::diagnostics::Details<'_>, NcStatus> {
        let error = self.encoder.last_error().ok_or(NC_E_INVALID_ARG)?;
        Ok(crate::diagnostics::Details {
            status: super::protocol::status(error),
            message: self.message.as_deref().unwrap_or(error.message),
            path: "$",
            offset: error.offset as u64,
            line: 0,
            column: 0,
        })
    }
}
diagnostic_projection!(NcbSerializer);
diagnostic_projection!(NcbDeserializer);
