//! Format selection crosses COM; typed values never depend on a format's Rust storage.
use neocogi_com_serialization::ncb as binary;
use neocogi_com_sdk::{ComDeserialize, ComSerialize, NcStatus, NC_E_INVALID_ARG, NcCopyFromBufferOffset, NcOutputWriter};
use neocogi_com_sdk::serialization::*;
use binary::{NcbSerializer, NcbDeserializer};
use serde_json::Value;
use std::collections::BTreeMap;

/// One schema deliberately declares fields in a different order from JSON key sorting.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct Settings {
    /// Preserves numeric width through either provider.
    count: u32,
    /// Contains escapes and non-ASCII text to exercise copied UTF-8.
    label: String,
    /// Is decoded through the exact bool operation.
    enabled: bool,
}
/// Constructs the same logical input for local and independently built providers.
fn settings() -> Settings {
    Settings {
        count: 42,
        label: "Réglages \"quoted\"".into(),
        enabled: true,
    }
}

/// The built-in NCB object produces the unchanged binary representation.
#[test]
fn owning_ncb_objects_round_trip_through_the_format_factory() {
    let format = binary::format(Limits::default());
    let bytes = format.serialize(&settings()).unwrap();
    assert_eq!(bytes, binary::to_vec(&settings()).unwrap());
    assert_eq!(format.deserialize::<Settings>(&bytes).unwrap(), settings());
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(format.deserialize::<Settings>(&trailing).unwrap_err().status, NC_E_INVALID_ARG);
}

/// The owning COM adapter follows the same pending variant transitions.
#[test]
fn owning_ncb_probes_one_live_variant_frame() {
    #[derive(ComSerialize)]
    enum Value {
        First,
        Second(u8),
    }
    let format = binary::format(Limits::default());
    let bytes = format.serialize(&Value::Second(23)).unwrap();
    let source = format.deserializer(&bytes).unwrap();
    let mut view = source.as_com_ref();
    view.enter_value().unwrap();
    assert_eq!(view.variant_is(0, NcCopyFromBufferOffset::new(0, b"First")), Ok(false));
    assert_eq!(view.variant_is(1, NcCopyFromBufferOffset::new(0, b"Second")), Ok(true));
    view.enter_value().unwrap();
    assert_eq!(view.deserialize_u8(), Ok(23));
    view.leave_value().unwrap();
    view.leave_value().unwrap();
    view.finish().unwrap();

    let source = format.deserializer(&format.serialize(&Value::First).unwrap()).unwrap();
    assert_eq!(
        source.as_com_ref().variant_is(0, NcCopyFromBufferOffset::new(0, b"First")),
        Err(NC_E_INVALID_ARG)
    );
    let failure = Error::capture(NC_E_INVALID_ARG, &source);
    assert_eq!(failure.byte_offset, Some(4));
    assert_eq!(failure.message, "no active value");
    assert_eq!(source.as_com_ref().enter_value(), Err(NC_E_INVALID_ARG));
    assert_eq!(Error::capture(NC_E_INVALID_ARG, &source), failure);
}

/// Complete output remains readable after a rejected short-copy operation.
#[test]
fn output_is_committed_once_and_session_failures_are_terminal() {
    let writer = NcbSerializer::create(Limits::default()).unwrap();
    let output = writer.query_ref::<ISerializationOutputAbi>().unwrap();
    assert!(output.byte_len().is_err());
    let mut capability = writer.as_com_ref();
    capability.serialize_u32(7).unwrap();
    capability.finish().unwrap();
    assert_eq!(capability.finish(), Err(NC_E_INVALID_ARG));
    assert_eq!(capability.serialize_u8(3), Err(NC_E_INVALID_ARG));
    let mut short = neocogi_com_sdk::NcOutputBuffer::<u8>::new();
    assert!(output.copy_to(short.output(0, 1)).is_err());
    assert!(short.is_empty());
    let mut full = neocogi_com_sdk::NcOutputBuffer::<u8>::new();
    output.copy_to(full.output(0, output.byte_len().unwrap() as usize)).unwrap();
    assert_eq!(full.as_slice(), binary::to_vec(&7u32).unwrap());
    let poisoned = NcbSerializer::create(Limits::default()).unwrap();
    let mut capability = poisoned.as_com_ref();
    capability.begin_sequence(0).unwrap();
    assert_eq!(capability.end_map(), Err(NC_E_INVALID_ARG));
    assert_eq!(capability.end_sequence(), Err(NC_E_INVALID_ARG));
    assert!(poisoned.query_ref::<ISerializationOutputAbi>().unwrap().byte_len().is_err());
}

/// The constructor copies its input; later mutations of the caller's bytes cannot change it.
#[test]
fn source_owns_input_and_checks_raw_outputs_and_terminal_errors() {
    let format = binary::format(Limits::default());
    let mut bytes = binary::to_vec(&false).unwrap();
    let source = format.deserializer(&bytes).unwrap();
    bytes.fill(0xff);
    assert!(!Deserializer::new(source.as_com_ref()).deserialize::<bool>().unwrap());
    let mut capability = source.as_com_ref();
    assert_eq!(capability.enter_value(), Err(NC_E_INVALID_ARG));
    let invalid = binary::to_vec(&2u8).unwrap();
    let source = format.deserializer(&invalid).unwrap();
    let mut capability = source.as_com_ref();
    capability.enter_value().unwrap();
    let raw = capability.as_raw();
    let mut output = 0xabu8;
    // SAFETY: live source/table and a real initialized output slot. Invalid bool
    // bytes are ordinary readable storage; failure must leave output untouched.
    let status = unsafe { ((*raw.vtable).deserialize_bool)(raw.object, &mut output) };
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!(output, 0xab);
    assert_eq!(capability.deserialize_u8(), Err(NC_E_INVALID_ARG));
}

/// A value scope cannot implicitly replace its source container's explicit end call.
#[test]
fn source_containers_require_matching_end_calls() {
    let bytes = binary::to_vec(&Vec::<u8>::new()).unwrap();
    for shape in ["record", "sequence", "map"] {
        for close in [false, true] {
            let source = NcbDeserializer::create(bytes.clone(), Limits::default()).unwrap();
            let mut capability = source.as_com_ref();
            capability.enter_value().unwrap();
            match shape {
                "record" => {
                    capability.begin_record(0).unwrap();
                }
                "sequence" => {
                    capability.begin_sequence().unwrap();
                }
                "map" => {
                    capability.begin_map().unwrap();
                }
                _ => unreachable!(),
            }
            if close {
                match shape {
                    "record" => capability.end_record().unwrap(),
                    "sequence" => capability.end_sequence().unwrap(),
                    "map" => capability.end_map().unwrap(),
                    _ => unreachable!(),
                }
                capability.leave_value().unwrap();
                capability.finish().unwrap();
            } else {
                assert_eq!(capability.leave_value(), Err(NC_E_INVALID_ARG), "unclosed {shape}");
                assert_eq!(capability.finish(), Err(NC_E_INVALID_ARG));
            }
        }
    }
}

/// State for a deliberately small independent JSON provider used only by this proof.
/// Other schema shapes are rejected; the production protocol supports them explicitly.
enum JsonWrite {
    /// No document has started.
    Ready,
    /// A record accumulates named JSON syntax, not erased application objects.
    Record {
        expected: u32,
        fields: BTreeMap<String, Value>,
        selected: Option<String>,
    },
    /// All declared fields have been written; finish may encode them.
    Complete(Value),
    /// A fieldless enum case waiting for its matching end call.
    Variant(String),
    /// Immutable committed JSON output.
    Finished(Vec<u8>),
    /// No more format operations are allowed.
    Failed,
}
/// Independent format storage with ordinary exclusive COM ownership.
#[neocogi_com_sdk::com_object(interfaces(ISerializer, ISerializationOutput), ownership=exclusive)]
struct JsonWriter {
    state: JsonWrite,
}
impl JsonWriter {
    /// Makes unsupported operations and malformed input terminal.
    fn reject<T>(&mut self) -> Result<T, NcStatus> {
        self.state = JsonWrite::Failed;
        Err(NC_E_INVALID_ARG)
    }
    /// Adds one already typed scalar to its selected named field.
    fn store(&mut self, value: Value) -> Result<(), NcStatus> {
        if let JsonWrite::Record { fields, selected, .. } = &mut self.state {
            if let Some(name) = selected.take() {
                if fields.insert(name, value).is_none() {
                    return Ok(());
                }
            }
        }
        self.reject()
    }
}
/// Source phases retain JSON syntax and the exact selected scalar until acquisition.
enum JsonRead {
    /// Parsed input has not opened its root.
    Ready(BTreeMap<String, Value>),
    /// A JSON string naming a fieldless enum case.
    Variant(String),
    /// Named fields can be selected independently of their encoded order.
    Record { fields: BTreeMap<String, Value>, selected: Option<Value> },
    /// Every field has been acquired; the root may finish.
    Complete,
    /// Successful document completion.
    Finished,
    /// A failed source is never reused.
    Failed,
}
/// The independently implemented decoder knows only JSON and the COM protocol.
#[neocogi_com_sdk::com_object(IDeserializer, ownership=exclusive)]
struct JsonReader {
    /// Root and field-acquisition state.
    state: JsonRead,
    /// Zero outside a value, one at the root, two inside its scalar field.
    depth: u32,
    /// Admitted UTF-8 awaiting the caller's fresh output buffer.
    string: Option<String>,
}
impl JsonReader {
    /// Rejects unsupported operations and every partial read permanently.
    fn reject<T>(&mut self) -> Result<T, NcStatus> {
        self.state = JsonRead::Failed;
        Err(NC_E_INVALID_ARG)
    }
    /// Takes one selected JSON scalar inside its typed field scope.
    fn scalar(&mut self) -> Result<Value, NcStatus> {
        if self.depth == 2 {
            if let JsonRead::Record { selected, .. } = &mut self.state {
                if let Some(value) = selected.take() {
                    return Ok(value);
                }
            }
        }
        self.reject()
    }
}
/// Validates complete borrowed text before a test provider interprets it.
fn text(value: NcCopyFromBufferOffset<'_, u8>) -> Result<&str, NcStatus> {
    if value.offset() != 0 {
        return Err(NC_E_INVALID_ARG);
    }
    std::str::from_utf8(value.as_slice()).map_err(|_| NC_E_INVALID_ARG)
}
impl ISerializer for JsonWriter {
    fn serialize_unit(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_bool(&mut self, value: ::core::primitive::bool) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.store(Value::Bool(value))
    }
    fn serialize_u8(&mut self, _value: ::core::primitive::u8) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_u16(&mut self, _value: ::core::primitive::u16) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_u32(&mut self, value: ::core::primitive::u32) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.store(Value::from(value))
    }
    fn serialize_u64(&mut self, _value: ::core::primitive::u64) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_i8(&mut self, _value: ::core::primitive::i8) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_i16(&mut self, _value: ::core::primitive::i16) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_i32(&mut self, _value: ::core::primitive::i32) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_i64(&mut self, _value: ::core::primitive::i64) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_f32(&mut self, _value: ::core::primitive::f32) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_f64(&mut self, _value: ::core::primitive::f64) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn serialize_string(
        &mut self,
        value: ::neocogi_com_sdk::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        match text(value) {
            Ok(value) => self.store(Value::String(value.to_owned())),
            Err(_) => self.reject(),
        }
    }
    fn begin_option(&mut self, _present: ::core::primitive::bool) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn end_option(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn begin_record(&mut self, fields: ::core::primitive::u32) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        if matches!(self.state, JsonWrite::Ready) {
            self.state = JsonWrite::Record {
                expected: fields,
                fields: BTreeMap::new(),
                selected: None,
            };
            Ok(())
        } else {
            self.reject()
        }
    }
    fn field(&mut self, name: ::neocogi_com_sdk::NcCopyFromBufferOffset<'_, ::core::primitive::u8>) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        let name = match text(name) {
            Ok(name) => name,
            Err(_) => return self.reject(),
        };
        if let JsonWrite::Record { expected, fields, selected } = &mut self.state {
            if selected.is_none() && fields.len() < *expected as usize && !fields.contains_key(name) {
                *selected = Some(name.to_owned());
                return Ok(());
            }
        }
        self.reject()
    }
    fn end_record(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        let state = std::mem::replace(&mut self.state, JsonWrite::Failed);
        if let JsonWrite::Record { expected, fields, selected: None } = state {
            if fields.len() == expected as usize {
                self.state = JsonWrite::Complete(serde_json::to_value(fields).unwrap());
                return Ok(());
            }
        }
        self.reject()
    }
    fn begin_sequence(&mut self, _length: ::core::primitive::u32) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn end_sequence(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn begin_map(&mut self, _pairs: ::core::primitive::u32) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn end_map(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn begin_variant(&mut self, _index: u32, name: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus> {
        if matches!(self.state, JsonWrite::Ready) {
            self.state = JsonWrite::Variant(text(name)?.to_owned());
            Ok(())
        } else {
            self.reject()
        }
    }
    fn end_variant(&mut self) -> Result<(), NcStatus> {
        let state = std::mem::replace(&mut self.state, JsonWrite::Failed);
        if let JsonWrite::Variant(name) = state {
            self.state = JsonWrite::Complete(Value::String(name));
            Ok(())
        } else {
            self.reject()
        }
    }
    fn fail(
        &mut self,
        _message: ::neocogi_com_sdk::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn finish(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        let state = std::mem::replace(&mut self.state, JsonWrite::Failed);
        if let JsonWrite::Complete(fields) = state {
            self.state = JsonWrite::Finished(serde_json::to_vec(&fields).unwrap());
            Ok(())
        } else {
            self.reject()
        }
    }
}
impl IDeserializer for JsonReader {
    fn enter_value(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        match (&self.state, self.depth) {
            (JsonRead::Ready(_) | JsonRead::Variant(_), 0) | (JsonRead::Record { selected: Some(_), .. }, 1) => {
                self.depth += 1;
                Ok(())
            }
            _ => self.reject(),
        }
    }
    fn leave_value(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        if self.string.is_some() {
            return self.reject();
        }
        match (&self.state, self.depth) {
            (JsonRead::Complete, 1) | (JsonRead::Record { selected: None, .. }, 2) => {
                self.depth -= 1;
                Ok(())
            }
            _ => self.reject(),
        }
    }
    fn deserialize_unit(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_bool(&mut self) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com_sdk::NcStatus> {
        match self.scalar()? {
            Value::Bool(value) => Ok(value),
            _ => self.reject(),
        }
    }
    fn deserialize_u8(&mut self) -> ::core::result::Result<::core::primitive::u8, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_u16(&mut self) -> ::core::result::Result<::core::primitive::u16, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_u32(&mut self) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com_sdk::NcStatus> {
        match self.scalar()?.as_u64().and_then(|value| u32::try_from(value).ok()) {
            Some(value) => Ok(value),
            None => self.reject(),
        }
    }
    fn deserialize_u64(&mut self) -> ::core::result::Result<::core::primitive::u64, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_i8(&mut self) -> ::core::result::Result<::core::primitive::i8, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_i16(&mut self) -> ::core::result::Result<::core::primitive::i16, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_i32(&mut self) -> ::core::result::Result<::core::primitive::i32, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_i64(&mut self) -> ::core::result::Result<::core::primitive::i64, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_f32(&mut self) -> ::core::result::Result<::core::primitive::f32, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn deserialize_f64(&mut self) -> ::core::result::Result<::core::primitive::f64, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn string_len(&mut self) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com_sdk::NcStatus> {
        match self.scalar()? {
            Value::String(value) => {
                let len = value.len() as u32;
                self.string = Some(value);
                Ok(len)
            }
            _ => self.reject(),
        }
    }
    fn copy_string(
        &mut self,
        output: ::neocogi_com_sdk::NcOutputWriter<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com_sdk::NcStatus> {
        let Some(value) = self.string.take() else {
            return self.reject();
        };
        let mut output = output;
        if output.offset() != 0 || output.remaining() < value.len() {
            return self.reject();
        }
        output.extend_from_slice(value.as_bytes())?;
        Ok(value.len() as u32)
    }
    fn option_present(&mut self) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn begin_record(&mut self, maximum: ::core::primitive::u32) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com_sdk::NcStatus> {
        let state = std::mem::replace(&mut self.state, JsonRead::Failed);
        if self.depth == 1 {
            if let JsonRead::Ready(fields) = state {
                let count = fields.len();
                if count <= maximum as usize {
                    self.state = JsonRead::Record { fields, selected: None };
                    return Ok(count as u32);
                }
            }
        }
        self.reject()
    }
    fn field(
        &mut self,
        name: ::neocogi_com_sdk::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<::core::primitive::bool, ::neocogi_com_sdk::NcStatus> {
        let name = match text(name) {
            Ok(name) => name,
            Err(_) => return self.reject(),
        };
        if self.depth == 1 {
            if let JsonRead::Record { fields, selected } = &mut self.state {
                if selected.is_none() {
                    *selected = fields.remove(name);
                    return Ok(selected.is_some());
                }
            }
        }
        self.reject()
    }
    fn end_record(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        if let JsonRead::Record { fields, selected: None } = &self.state {
            if fields.is_empty() && self.depth == 1 {
                self.state = JsonRead::Complete;
                return Ok(());
            }
        }
        self.reject()
    }
    fn begin_sequence(&mut self) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn end_sequence(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn begin_map(&mut self) -> ::core::result::Result<::core::primitive::u32, ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn end_map(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn variant_is(&mut self, _index: u32, name: NcCopyFromBufferOffset<'_, u8>) -> Result<bool, NcStatus> {
        if self.depth == 1 {
            if let JsonRead::Variant(selected) = &self.state {
                if selected == text(name)? {
                    self.state = JsonRead::Complete;
                    return Ok(true);
                }
                return Ok(false);
            }
        }
        self.reject()
    }
    fn fail(
        &mut self,
        _message: ::neocogi_com_sdk::NcCopyFromBufferOffset<'_, ::core::primitive::u8>,
    ) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        self.reject()
    }
    fn finish(&mut self) -> ::core::result::Result<(), ::neocogi_com_sdk::NcStatus> {
        if self.depth == 0 && matches!(self.state, JsonRead::Complete) {
            self.state = JsonRead::Finished;
            Ok(())
        } else {
            self.reject()
        }
    }
}

impl ISerializationOutput for JsonWriter {
    fn byte_len(&self) -> Result<u64, NcStatus> {
        if let JsonWrite::Finished(bytes) = &self.state {
            Ok(bytes.len() as u64)
        } else {
            Err(NC_E_INVALID_ARG)
        }
    }
    fn copy_to(&self, mut output: NcOutputWriter<'_, u8>) -> Result<u64, NcStatus> {
        if let JsonWrite::Finished(bytes) = &self.state {
            output.extend_from_slice(bytes)?;
            Ok(bytes.len() as u64)
        } else {
            Err(NC_E_INVALID_ARG)
        }
    }
}
/// Independent provider factory; all cursor state stays inside its own COM objects.
#[neocogi_com_sdk::com_object(ISerializationFormat)]
struct JsonFormat;
impl ISerializationFormat for JsonFormat {
    fn create_serializer(&self, _bytes: u64, _elements: u32, _depth: u32, _values: u64) -> Result<neocogi_com_sdk::ComBox<ISerializerAbi>, NcStatus> {
        Ok(JsonWriter::into_box(JsonWriter { state: JsonWrite::Ready })
            .query()
            .unwrap_or_else(|_| unreachable!()))
    }
    fn create_deserializer(
        &self,
        input: NcCopyFromBufferOffset<'_, u8>,
        max_bytes: u64,
        _elements: u32,
        _depth: u32,
        _values: u64,
    ) -> Result<neocogi_com_sdk::ComBox<IDeserializerAbi>, NcStatus> {
        if input.offset() != 0 || input.len() as u64 > max_bytes {
            return Err(NC_E_INVALID_ARG);
        }
        let state = match serde_json::from_slice(input.as_slice()).map_err(|_| NC_E_INVALID_ARG)? {
            Value::Object(fields) => JsonRead::Ready(fields.into_iter().collect()),
            Value::String(case) => JsonRead::Variant(case),
            _ => return Err(NC_E_INVALID_ARG),
        };
        Ok(JsonReader::into_box(JsonReader { state, depth: 0, string: None })
            .query()
            .unwrap_or_else(|_| unreachable!()))
    }
}
/// The same generated value implementations select two runtime format factories.
#[test]
fn independently_implemented_named_format_uses_the_same_value_traits() {
    let object = JsonFormat::from_value(JsonFormat).query().unwrap();
    let json = Format::new(object, Limits::default());
    let encoded = json.serialize(&settings()).unwrap();
    let value: Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(value["count"], 42);
    assert_eq!(value["label"], settings().label);
    assert_eq!(value["enabled"], true);
    assert_ne!(encoded, binary::to_vec(&settings()).unwrap());
    assert_eq!(json.deserialize::<Settings>(&encoded).unwrap(), settings());
    // Reorder the physical source properties independently of declaration order.
    let reordered = br#"{"enabled":true,"label":"other","count":17}"#;
    assert_eq!(
        json.deserialize::<Settings>(reordered).unwrap(),
        Settings {
            count: 17,
            label: "other".into(),
            enabled: true
        }
    );
    assert!(json.deserialize::<Settings>(br#"{"enabled":true,"label":"other","unexpected":17}"#).is_err());
    assert!(json.deserialize::<Settings>(br#"{"enabled":true,"label":false,"count":17}"#).is_err());
    assert!(json.deserialize::<Settings>(br#"{"enabled":true,"count":17}"#).is_err());
}

/// A separately compiled library owns the format object, cursors, allocations and vtables.
#[cfg(feature = "runtime")]
#[test]
fn native_plugin_provides_format_objects_without_sharing_rust_layouts() {
    use std::{fs, path::Path, process::Command};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        format!(
            r#"
        [package]
        name="serialization-plugin-fixture"
        version="0.0.0"
        edition="2024"
        [workspace]
        [lib]
        name="serialization_plugin"
        crate-type=["cdylib"]
        [features]
        default=["dynamic-plugin"]
        dynamic-plugin=[]
        [dependencies]
        neocogi-com-sdk={{path={:?},features=["runtime"]}}
        neocogi-com-serialization={{path={:?}}}
    "#,
            root.join("crates/com"),
            root.join("crates/serialization")
        ),
    )
    .unwrap();
    fs::write(
        project.path().join("src/lib.rs"),
        r#"
        use neocogi_com_sdk::{ComPtr,NcStatus,RuntimeHandle};
        use neocogi_com_sdk::serialization::ISerializationFormatAbi;
        use neocogi_com_serialization::ncb::NcbFormat;
        #[neocogi_com_sdk::plugin(name="nc.tests.serialization_format",version="0.0.0")]
        fn acquire(_runtime:RuntimeHandle)->Result<ComPtr<ISerializationFormatAbi>,NcStatus>{
            NcbFormat::from_value(NcbFormat).query()
        }
    "#,
    )
    .unwrap();
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let build = Command::new("cargo")
        .args(["build", "--offline", "--message-format=json", "--manifest-path"])
        .arg(project.path().join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target)
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    let artifact = String::from_utf8(build.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact" && message["target"]["name"] == "serialization_plugin")
        .flat_map(|message| {
            message["filenames"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|path| path.as_str().map(std::path::PathBuf::from))
                .collect::<Vec<_>>()
        })
        .find(|path| path.extension().is_some_and(|extension| extension == std::env::consts::DLL_EXTENSION))
        .unwrap();
    // SAFETY: this test executable owns its host implementation for the whole process.
    let runtime = unsafe { neocogi_com_sdk::RuntimeHandle::__from_host_api(neocogi_com_host::initialize_process()) }.unwrap();
    // SAFETY: the fixture is trusted code just built by this test and stays process-pinned.
    let id = unsafe { runtime.load_plugin_library(artifact) };
    let format = Format::new(runtime.acquire_plugin_root::<ISerializationFormatAbi>(id).unwrap(), Limits::default());
    let bytes = format.serialize(&settings()).unwrap();
    assert_eq!(bytes, binary::to_vec(&settings()).unwrap());
    assert_eq!(format.deserialize::<Settings>(&bytes).unwrap(), settings());
}

/// A value-level failure must poison its format even after a complete scalar write.
struct RejectWrite;
impl ComSerialize for RejectWrite {
    /// Models application validation failing after a format operation succeeded.
    fn com_serialize(&self, writer: &mut Serializer<'_>) -> Result<(), NcStatus> {
        writer.serialize_u32(7)?;
        Err(neocogi_com_sdk::NC_E_CLOSED)
    }
}
/// A failed typed constructor must not leave a reusable partially decoded session.
struct RejectRead;
impl ComDeserialize for RejectRead {
    /// Models validation that rejects an otherwise valid input scalar.
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        u8::com_deserialize(reader)?;
        Err(neocogi_com_sdk::NC_E_CLOSED)
    }
}
/// Both callback failures preserve the caller's status and make the provider terminal.
#[test]
fn value_callback_failures_abort_the_com_session() {
    let writer = NcbSerializer::create(Limits::default()).unwrap();
    assert_eq!(Serializer::new(writer.as_com_ref()).serialize(&RejectWrite), Err(neocogi_com_sdk::NC_E_CLOSED));
    assert_eq!(writer.as_com_ref().finish(), Err(NC_E_INVALID_ARG));
    assert!(writer.query_ref::<ISerializationOutputAbi>().unwrap().byte_len().is_err());
    let reader = NcbDeserializer::create(binary::to_vec(&7u8).unwrap(), Limits::default()).unwrap();
    assert!(matches!(
        Deserializer::new(reader.as_com_ref()).deserialize::<RejectRead>(),
        Err(neocogi_com_sdk::NC_E_CLOSED)
    ));
    assert_eq!(reader.as_com_ref().leave_value(), Err(NC_E_INVALID_ARG));
    assert_eq!(reader.as_com_ref().finish(), Err(NC_E_INVALID_ARG));
}

/// Per-document limits apply to both owning implementations and their typed frontends.
#[test]
fn owning_formats_enforce_document_and_container_limits() {
    let value = settings();
    let bytes = binary::to_vec(&value).unwrap();
    let small = binary::format(Limits {
        bytes: bytes.len() - 1,
        ..Limits::default()
    });
    assert!(small.serialize(&value).is_err());
    assert!(small.deserialize::<Settings>(&bytes).is_err());
    let entries = binary::format(Limits { elements: 2, ..Limits::default() });
    assert!(entries.serialize(&value).is_err());
    assert!(entries.deserialize::<Settings>(&bytes).is_err());
    let exact = binary::format(Limits { bytes: bytes.len(), ..Limits::default() });
    assert_eq!(exact.serialize(&value).unwrap(), bytes);
    assert_eq!(exact.deserialize::<Settings>(&bytes).unwrap(), value);
}

/// Raw Rust spelling never enters the authored format schema.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct KeywordField {
    /// The semantic field name is type, without Rust's escape prefix.
    r#type: bool,
}
/// Named absence can retain a default before a later present field.
#[derive(Debug, Default, PartialEq, ComSerialize, ComDeserialize)]
#[com_deserialize(default)]
struct NamedDefaults {
    /// A missing leading member keeps its default.
    r#type: bool,
    /// A later present member is still selected by name.
    count: u32,
}
/// Named formats use authored case names independently of their ordinal.
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
enum KeywordCase {
    /// Probes a nonmatching ordinal before the selected case.
    Other,
    /// The semantic case name is type, without Rust's escape prefix.
    r#type,
}
/// Real named-format calls agree on the exact field and case names in both directions.
#[test]
fn authored_names_omit_rust_raw_identifier_prefixes() {
    let json = Format::new(JsonFormat::from_value(JsonFormat).query().unwrap(), Limits::default());
    let value = KeywordField { r#type: true };
    let bytes = json.serialize(&value).unwrap();
    assert_eq!(bytes, br#"{"type":true}"#);
    assert_eq!(json.deserialize::<KeywordField>(&bytes).unwrap(), value);
    assert_eq!(
        json.deserialize::<NamedDefaults>(br#"{"count":7}"#).unwrap(),
        NamedDefaults { r#type: false, count: 7 }
    );
    for case in [KeywordCase::Other, KeywordCase::r#type] {
        let bytes = json.serialize(&case).unwrap();
        assert_eq!(json.deserialize::<KeywordCase>(&bytes).unwrap(), case);
        if case == KeywordCase::r#type {
            assert_eq!(bytes, br#""type""#);
        }
    }
    assert!(json.deserialize::<KeywordCase>(br#""unknown""#).is_err());
}

/// Static registration uses the exact descriptor returned by the dynamic shim.
#[cfg(feature = "runtime")]
#[test]
fn statically_linked_plugin_acquires_the_common_format_root() {
    // SAFETY: this final executable owns the host tables for its complete lifetime.
    let runtime = unsafe { neocogi_com_sdk::RuntimeHandle::__from_host_api(neocogi_com_host::initialize_process()) }.unwrap();
    let descriptor = neocogi_com_serialization::plugin_descriptor();
    // SAFETY: the descriptor and all reachable code are linked into this executable.
    unsafe { runtime.register_plugin_descriptor(descriptor) }.unwrap();
    let root = runtime.acquire_plugin_root::<ISerializationPluginAbi>(descriptor.plugin_id).unwrap();
    let format = Format::new(root.default_format().unwrap(), Limits::default());
    assert_eq!(format.deserialize::<Settings>(&format.serialize(&settings()).unwrap()).unwrap(), settings());
}
