//! Array cardinality and cleanup cross production and fault-injected COM sessions.
#![cfg(feature = "alloc")]
use std::cell::Cell;
use neocogi_com::{ComDeserialize, ComSerialize, NcStatus, NC_E_INVALID_ARG};
use neocogi_com::serialization::{Deserializer, ScopedDeserializer, scope::Scoped};
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
    values: [(); 3],
}
/// Its independent decoder acquires owners before a later completion failure.
#[derive(ComDeserialize)]
struct Output {
    values: [Acquired; 3],
}

/// Calls a real scoped COM vtable, then recovers its observations after borrowing.
fn decode<T: ComDeserialize>(bytes: Vec<u8>, stage: Stage) -> (Result<T, NcStatus>, FaultSource) {
    let scoped = Scoped::new(FaultSource::new(bytes, stage));
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
    let limits = neocogi_com::serialization::Limits { elements: 1, ..Default::default() };
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

#[cfg(feature = "json")]
#[test]
fn json_uses_the_same_exact_count_admission() {
    use neocogi_com::serialization::{Format, Limits};
    let format = Format::new(neocogi_com_serialization::json::factory(), Limits::default());
    for bytes in [b"[]".as_slice(), b"[null,null]", b"[null,null,null,null]"] {
        OWNERS.with(|owners| owners.set((0, 0)));
        assert!(format.deserialize::<[Acquired; 3]>(bytes).is_err());
        assert_eq!(OWNERS.with(Cell::get), (0, 0));
    }
    drop(format.deserialize::<[Acquired; 3]>(b"[null,null,null]").unwrap());
    assert_eq!(OWNERS.with(Cell::get), (3, 3));
}
