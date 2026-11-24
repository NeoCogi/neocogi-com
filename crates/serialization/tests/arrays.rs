//! Array cardinality and cleanup cross production and fault-injected COM sessions.
#![cfg(feature = "alloc")]
use std::cell::Cell;
use neocogi_com_sdk::{ComDeserialize, ComSerialize, NcStatus, NC_E_INVALID_ARG};
use neocogi_com_sdk::serialization::{Deserializer, ScopedDeserializer, scope::ScopedComProvider};
use neocogi_com_serialization::ncb;
#[path = "support/array_source.rs"]
mod array_source;
use array_source::{FaultSource, Stage};

std::thread_local! {
    /// Counts complete owners acquired/dropped on this test thread only.
    static OWNERS: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}
/// A non-Copy owner whose wire is a zero-byte NCB unit.
#[derive(Debug)]
struct Acquired;
impl ComDeserialize for Acquired {
    fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
        reader.deserialize_unit()?;
        OWNERS.with(|count| {
            let (made, dropped) = count.get();
            count.set((made + 1, dropped));
        });
        Ok(Self)
    }
}
impl Drop for Acquired {
    fn drop(&mut self) {
        OWNERS.with(|count| {
            let (made, dropped) = count.get();
            count.set((made, dropped + 1));
        });
    }
}
/// Production derived record framing surrounds the array under test.
#[derive(ComSerialize)]
struct Input {
    /// Encoded field selected by the independently derived decoder.
    values: [(); 3],
}
/// Its independent decoder acquires owners before a later completion failure.
#[derive(ComDeserialize)]
struct Output {
    /// Normal record destruction owns the complete array after construction.
    values: [Acquired; 3],
}

/// Calls a real scoped COM vtable, then recovers its observations after borrowing.
fn decode<T: ComDeserialize>(bytes: Vec<u8>, stage: Stage) -> (Result<T, NcStatus>, FaultSource) {
    let mut scoped = ScopedComProvider::new(FaultSource::new(bytes, stage));
    let result = {
        // SAFETY: the generated scoped table matches this exact provider type.
        let interface = unsafe { scoped.interface(&ScopedDeserializer::<FaultSource>::VTABLE) };
        Deserializer::new(interface).deserialize()
    };
    (result, scoped.into_inner())
}

#[test]
fn wrong_array_count_rejects_before_any_child_acquisition() {
    for count in [0u32, 2, 4] {
        OWNERS.with(|owners| owners.set((0, 0)));
        let mut bytes = b"NCB\x01\0\0\0\0".to_vec();
        bytes[4..].copy_from_slice(&count.to_le_bytes());
        let (result, source) = decode::<[Acquired; 3]>(bytes, Stage::None);
        assert!(matches!(result, Err(NC_E_INVALID_ARG)));
        assert_eq!(source.units, 0);
        assert_eq!(OWNERS.with(Cell::get), (0, 0));
    }
    // The production diagnostic identifies the count boundary, before missing data.
    assert_eq!(
        ncb::from_slice::<[u8; 3]>(b"NCB\x01\x02\0\0\0").unwrap_err(),
        ncb::Error {
            offset: 8,
            message: "invalid sequence length"
        }
    );
    assert!(ncb::from_slice::<[(); 0]>(b"NCB\x01\0\0\0\0").is_ok());
    assert!(ncb::from_slice::<[(); 0]>(b"NCB\x01\x01\0\0\0").is_err());
    // Provider admission precedes typed shape checks and child side effects.
    OWNERS.with(|owners| owners.set((0, 0)));
    let limits = neocogi_com_sdk::serialization::Limits { elements: 1, ..Default::default() };
    assert_eq!(
        ncb::from_slice_with_limits::<[Acquired; 3]>(b"NCB\x01\x02\0\0\0", limits).unwrap_err().message,
        "collection or value limit"
    );
    assert_eq!(OWNERS.with(Cell::get), (0, 0));
}

#[test]
fn completed_array_and_record_drop_owners_on_each_late_failure() {
    for stage in [Stage::Begin, Stage::End, Stage::Leave, Stage::Finish] {
        OWNERS.with(|owners| owners.set((0, 0)));
        let (result, source) = decode::<[Acquired; 3]>(ncb::to_vec(&[(); 3]).unwrap(), stage);
        assert!(matches!(result, Err(status) if status == stage.status()));
        let expected = if stage == Stage::Begin { 0 } else { 3 };
        assert_eq!(source.units, expected);
        assert_eq!(OWNERS.with(Cell::get), (expected, expected));
        OWNERS.with(|owners| owners.set((0, 0)));
        let (result, source) = decode::<Output>(ncb::to_vec(&Input { values: [(); 3] }).unwrap(), stage);
        assert!(matches!(result, Err(status) if status == stage.status()));
        assert_eq!(source.units, expected);
        assert_eq!(OWNERS.with(Cell::get), (expected, expected));
    }
    OWNERS.with(|owners| owners.set((0, 0)));
    let (result, _) = decode::<Output>(ncb::to_vec(&Input { values: [(); 3] }).unwrap(), Stage::None);
    let result = result.unwrap();
    assert_eq!(result.values.len(), 3);
    assert_eq!(OWNERS.with(Cell::get), (3, 0));
    drop(result);
    assert_eq!(OWNERS.with(Cell::get), (3, 3));
}

#[test]
fn empty_arrays_still_execute_provider_completion_and_begin_errors_win() {
    for stage in [Stage::Begin, Stage::End, Stage::Leave, Stage::Finish] {
        let (result, source) = decode::<[Acquired; 0]>(ncb::to_vec(&[(); 0]).unwrap(), stage);
        assert!(matches!(result, Err(status) if status == stage.status()));
        assert_eq!(source.units, 0);
    }
    // A provider failure before count return takes precedence over fixed shape.
    let (result, source) = decode::<[Acquired; 3]>(ncb::to_vec(&[(); 0]).unwrap(), Stage::Begin);
    assert!(matches!(result, Err(status) if status == Stage::Begin.status()));
    assert_eq!(source.units, 0);
}

/// Defaulting permits construction before the provider checks remaining fields.
#[derive(Debug, Default, PartialEq, ComDeserialize)]
#[com_deserialize(default)]
struct Settings {
    enabled: bool,
    count: u32,
}

#[test]
fn provider_record_completion_precedes_the_facade_count_check() {
    #[derive(ComSerialize)]
    struct Empty {}
    let bytes = ncb::to_vec(&Empty {}).unwrap();
    assert_eq!(ncb::from_slice::<Settings>(&bytes).unwrap(), Settings::default());

    let (result, source) = decode::<Settings>(bytes.clone(), Stage::RecordEndFailure);
    assert_eq!(result, Err(Stage::RecordEndFailure.status()));
    assert_eq!(source.record_calls, ["end_record", "generic failure"]);

    let (result, source) = decode::<Settings>(bytes, Stage::RecordRemainder);
    assert_eq!(result, Err(NC_E_INVALID_ARG));
    assert_eq!(source.record_calls, ["end_record", "remainder failure", "generic failure"]);

    // A real positional provider also rejects a record with unconsumed children.
    struct Skipped;
    impl ComDeserialize for Skipped {
        fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
            reader.record(1, |_| Ok(Self))
        }
    }
    assert_eq!(
        ncb::from_slice::<Skipped>(b"NCB\x01\x01\0\0\0").err(),
        Some(ncb::Error {
            offset: 8,
            message: "unconsumed record fields"
        })
    );
}

#[cfg(feature = "json")]
#[test]
fn defaulted_record_completion_reports_the_enclosing_json_path() {
    use neocogi_com_sdk::serialization::{Error, Format, Limits};

    #[allow(dead_code)]
    #[derive(Debug, ComDeserialize)]
    struct Required {
        enabled: bool,
        count: u32,
    }
    #[allow(dead_code)]
    #[derive(Debug, ComDeserialize)]
    struct Nested {
        outer: [Settings; 1],
    }
    let format = Format::new(neocogi_com_serialization::json::factory(), Limits::default());
    assert_eq!(format.deserialize::<Settings>(b"{}").unwrap(), Settings::default());
    assert_eq!(
        format.deserialize::<Settings>(br#"{"count":7}"#).unwrap(),
        Settings { enabled: false, count: 7 }
    );

    let mut source = format.deserializer(br#"{"rogue":true}"#).unwrap();
    let status = Deserializer::new(source.as_com_mut()).deserialize::<Settings>().unwrap_err();
    let error = Error::capture(status, &source);
    assert_eq!(error.status, NC_E_INVALID_ARG);
    assert!(error.message.starts_with("unknown or unconsumed record fields"), "{}", error.message);
    assert_eq!(error.path, "$");
    assert_eq!(source.as_com_mut().finish(), Err(status));
    assert_eq!(Error::capture(status, &source), error);

    let nested = format.deserialize::<Nested>(br#"{"outer":[{"rogue":true}]}"#).unwrap_err();
    assert_eq!(nested.message, error.message);
    assert_eq!(nested.path, "$.outer[0]");
    assert_eq!(format.deserialize::<Required>(b"{}").unwrap_err().path, "$.enabled");
    assert_eq!(format.deserialize::<Settings>(br#"{"enabled":1}"#).unwrap_err().path, "$.enabled");
    let excessive = format.deserialize::<Settings>(br#"{"enabled":true,"count":1,"extra":0}"#).unwrap_err();
    assert_eq!(excessive.status, NC_E_INVALID_ARG);
    assert!(excessive.message.starts_with("expected record with declared fields"), "{}", excessive.message);
    assert_eq!(excessive.path, "$");
}

#[cfg(feature = "json")]
#[test]
fn json_uses_the_same_exact_count_admission() {
    use neocogi_com_sdk::serialization::{Format, Limits};
    let format = Format::new(neocogi_com_serialization::json::factory(), Limits::default());
    for bytes in [b"[]".as_slice(), b"[null,null]", b"[null,null,null,null]"] {
        OWNERS.with(|owners| owners.set((0, 0)));
        let error = format.deserialize::<[Acquired; 3]>(bytes).unwrap_err();
        assert_eq!(error.message, "invalid sequence length");
        assert_eq!(error.path, "$");
        assert_eq!(OWNERS.with(Cell::get), (0, 0));
    }
    drop(format.deserialize::<[Acquired; 3]>(b"[null,null,null]").unwrap());
    assert_eq!(OWNERS.with(Cell::get), (3, 3));
}

#[cfg(feature = "json")]
#[test]
fn json_container_errors_keep_real_paths_and_child_entry_keeps_attempted_index() {
    use neocogi_com_sdk::serialization::{Format, Limits, Error};
    use std::collections::BTreeMap;
    /// Nested arrays separate the enclosing container path from its next child.
    #[derive(Debug, ComDeserialize)]
    #[allow(dead_code)]
    struct Nested {
        /// A wrong inner shape should identify the inner container itself.
        items: [[u8; 2]; 2],
    }
    let format = Format::new(neocogi_com_serialization::json::factory(), Limits::default());
    let error = format.deserialize::<Nested>(br#"{"items":[[1],[2,3]]}"#).unwrap_err();
    assert_eq!(error.path, "$.items[0]");
    let missing = format.deserialize::<Nested>(b"{}").unwrap_err();
    assert_eq!(missing.path, "$.items");
    let duplicate = format.deserialize::<BTreeMap<u32, u32>>(br#"{"$map":[[1,2],[1,3]]}"#).unwrap_err();
    assert_eq!(duplicate.message, "duplicate map key");
    assert_eq!(duplicate.path, "$");
    let mut source = format.deserializer(b"[]").unwrap();
    source.as_com_mut().enter_value().unwrap();
    assert_eq!(source.as_com_mut().begin_sequence().unwrap(), 0);
    let status = source.as_com_mut().enter_value().unwrap_err();
    let error = Error::capture(status, &source);
    assert_eq!(error.path, "$[0]");
    assert_eq!(source.as_com_mut().finish(), Err(status));
    assert_eq!(Error::capture(status, &source), error);
    // Schema failure after field selection keeps that field's real name.
    let mut source = format.deserializer(br#"{"items":[]}"#).unwrap();
    source.as_com_mut().enter_value().unwrap();
    assert_eq!(source.as_com_mut().begin_record(1).unwrap(), 1);
    assert!(source.as_com_mut().field(neocogi_com_sdk::NcCopyFromBufferOffset::new(0, b"items")).unwrap());
    let status = source
        .as_com_mut()
        .fail(NC_E_INVALID_ARG, neocogi_com_sdk::NcCopyFromBufferOffset::new(0, b"invalid selected field"))
        .unwrap_err();
    assert_eq!(Error::capture(status, &source).path, "$.items");
    // Child budget failures happen before pushing their path, including payloads.
    let shallow = Format::new(neocogi_com_serialization::json::factory(), Limits { depth: 1, ..Default::default() });
    let child = shallow.deserialize::<[u8; 1]>(b"[1]").unwrap_err();
    assert_eq!(child.status, neocogi_com_sdk::NC_E_OUT_OF_RANGE);
    assert_eq!(child.path, "$[0]");
    let child = shallow.deserialize::<Option<u8>>(br#"{"$some":1}"#).unwrap_err();
    assert_eq!(child.status, neocogi_com_sdk::NC_E_OUT_OF_RANGE);
    assert_eq!(child.path, "$.$some");
    /// A matched enum has one selected payload even before its scope is entered.
    #[derive(Debug, ComSerialize, ComDeserialize)]
    enum Payload {
        Value(u8),
    }
    let bytes = format.serialize(&Payload::Value(1)).unwrap();
    let child = shallow.deserialize::<Payload>(&bytes).unwrap_err();
    assert_eq!(child.status, neocogi_com_sdk::NC_E_OUT_OF_RANGE);
    assert_eq!(child.path, "$.$value");
}
