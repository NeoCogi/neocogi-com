//! Persistent values are portable, deterministic, and reject malformed inputs.

#[path = "support/allocation_probe.rs"]
mod allocation_probe;

use neocogi_com::{
    binary::{self, Limits},
    ComDeserialize, ComSerialize,
};
use std::collections::BTreeMap;

/// The former persistence crate's derive-based schema, retained as a wire baseline.
mod legacy {
    use neocogi_com::{ComDeserialize, ComSerialize};

    /// Preserve the old record name as well as its exact field order and types.
    #[derive(ComSerialize, ComDeserialize)]
    pub struct ObjectRecord {
        /// Canonical recipe UUID bytes.
        pub type_id: [u8; 16],
        /// Provider initialization revision.
        pub schema_version: u32,
        /// Opaque provider initialization payload.
        pub state: Vec<u8>,
        /// Recursively owned initialization records.
        pub children: Vec<ObjectRecord>,
    }
}

/// Moving the schema into core must leave existing nested NCB documents readable.
#[test]
fn builtin_object_records_preserve_the_existing_wire_format() {
    let previous = legacy::ObjectRecord {
        type_id: [7; 16],
        schema_version: 9,
        state: vec![1, 3, 5],
        children: vec![legacy::ObjectRecord {
            type_id: [8; 16],
            schema_version: 2,
            state: vec![4, 6],
            children: vec![],
        }],
    };
    // Decode bytes emitted by the old derive into the new core-owned schema,
    // then require the encoder to produce exactly the same bytes.
    let original = binary::to_vec(&previous).unwrap();
    let current: neocogi_com::persistence::ObjectRecord = binary::from_slice(&original).unwrap();
    assert_eq!(current.type_id, [7; 16]);
    assert_eq!(current.children[0].state, [4, 6]);
    assert_eq!(binary::to_vec(&current).unwrap(), original);
}

/// Representative recursive construction data, independent from live objects.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
enum Node {
    /// No child content.
    Empty,
    /// One owned label.
    Label(String),
    /// Positional heterogeneous payload.
    Size(i32, u64),
    /// Named properties and recursively owned children.
    Group { enabled: bool, children: Vec<Node> },
}

/// Values combine records, options, fixed arrays and deterministic maps.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct Document {
    /// Source-schema revision is independent from the binary codec marker.
    version: u32,
    /// Optional user-authored title.
    title: Option<String>,
    /// Application-chosen portable identifier bytes.
    kind: [u8; 16],
    /// Root construction tree.
    root: Node,
    /// Named external resource references.
    resources: BTreeMap<String, String>,
}

/// The fixture has every owned enum payload shape and non-ASCII strings.
fn document() -> Document {
    Document {
        version: 1,
        title: Some("Réglages".into()),
        kind: [7; 16],
        root: Node::Group {
            enabled: true,
            children: vec![Node::Empty, Node::Label("Save".into()), Node::Size(-3, u64::MAX)],
        },
        resources: BTreeMap::from([("icon".into(), "assets/save.png".into())]),
    }
}

/// Serialization preserves semantic values and produces deterministic bytes.
#[test]
fn nested_values_round_trip_and_reencode_identically() {
    let original = document();
    let encoded = binary::to_vec(&original).unwrap();
    let decoded: Document = binary::from_slice(&encoded).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(binary::to_vec(&decoded).unwrap(), encoded);
}

/// A golden scalar encoding catches native-endian or native-usize regressions.
#[test]
fn golden_portable_widths_and_strict_tags() {
    assert_eq!(binary::to_vec(&0x01020304_u32).unwrap(), b"NCB\x01\x04\x03\x02\x01");
    assert_eq!(binary::to_vec(&1_usize).unwrap().len(), 12);
    assert!(binary::from_slice::<bool>(b"NCB\x01\x02").is_err());
    assert!(binary::from_slice::<Node>(b"NCB\x01\xff\xff\xff\xff").is_err());
    assert!(binary::from_slice::<String>(b"NCB\x01\x01\0\0\0\xff").is_err());
    assert!(binary::from_slice::<u32>(b"NCB\x02\0\0\0\0").is_err());
}

/// No truncated prefix of a complete document may be accepted.
#[test]
fn truncation_and_trailing_data_fail() {
    let encoded = binary::to_vec(&document()).unwrap();
    for end in 0..encoded.len() {
        assert!(binary::from_slice::<Document>(&encoded[..end]).is_err(), "accepted prefix {end}");
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(binary::from_slice::<Document>(&trailing).is_err());
}

/// Limits cover recursive and zero-sized data before unbounded allocations.
#[test]
fn decoder_limits_and_fixed_lengths_are_enforced() {
    let encoded = binary::to_vec(&document()).unwrap();
    for limits in [
        Limits { bytes: 4, ..Limits::default() },
        Limits { depth: 2, ..Limits::default() },
        Limits { values: 3, ..Limits::default() },
    ] {
        assert!(binary::from_slice_with_limits::<Document>(&encoded, limits).is_err());
    }
    assert!(binary::from_slice::<Vec<()>>(b"NCB\x01\xff\xff\xff\xff").is_err());
    let three = binary::to_vec(&[1_u8, 2, 3]).unwrap();
    assert!(binary::from_slice::<[u8; 2]>(&three).is_err());
    assert!(binary::from_slice::<[u8; 4]>(&three).is_err());
    assert!(binary::to_vec_with_limits(&document(), Limits { bytes: 8, ..Limits::default() }).is_err());
}

/// A manual map writer exercises duplicate-key rejection in the actual decoder.
#[test]
fn repeated_keys_do_not_silently_overwrite() {
    let bytes = b"NCB\x01\x02\0\0\0\x01\x02\x01\x03";
    assert!(binary::from_slice::<BTreeMap<u8, u8>>(bytes).is_err());
}

/// A record makes the value budget span independent sibling collections.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct BudgetRecord {
    /// First collection consumes part of the shared cumulative allowance.
    first: Vec<()>,
    /// Second collection must use what remains, rather than a fresh allowance.
    second: Vec<()>,
}

/// Fieldless and payload enums share one ordinal decoder without synthetic unit payloads.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
enum BudgetChoice {
    /// No payload value follows this discriminant.
    First,
    /// A different ordinal must have the same structural accounting.
    Second,
}

/// Checks independently specified minimum structural budgets in both directions.
/// The golden bytes supply the exact byte boundary; lowering any other bound by
/// one must reject the same schema during writing and reading, including leaves.
fn assert_accounting<T: ComSerialize + ComDeserialize + std::fmt::Debug + PartialEq>(value: &T, values: usize, depth: usize, elements: usize) {
    let encoded = binary::to_vec(value).unwrap();
    let exact = Limits {
        bytes: encoded.len(),
        elements,
        depth,
        values,
    };
    assert_eq!(binary::encoded_len_with_limits(value, exact).unwrap(), encoded.len());
    assert_eq!(binary::to_vec_with_limits(value, exact).unwrap(), encoded);
    assert_eq!(&binary::from_slice_with_limits::<T>(&encoded, exact).unwrap(), value);

    // Internal length prefixes and tags must not consume extra values. Each
    // expected boundary is stated by the fixture's schema, not by codec state.
    let mut insufficient = vec![
        Limits { bytes: exact.bytes - 1, ..exact },
        Limits { values: exact.values - 1, ..exact },
        Limits { depth: exact.depth - 1, ..exact },
    ];
    if exact.elements != 0 {
        insufficient.push(Limits { elements: exact.elements - 1, ..exact });
    }
    for limits in insufficient {
        let written = binary::to_vec_with_limits(value, limits).unwrap_err();
        assert_eq!(
            binary::encoded_len_with_limits(value, limits).unwrap_err(),
            written,
            "measurement diverged from writer for {value:?} under {limits:?}"
        );
        assert!(
            binary::from_slice_with_limits::<T>(&encoded, limits).is_err(),
            "reader accepted {value:?} under {limits:?}"
        );
    }
}

/// Scalars are one value at depth one, even when they occupy no payload bytes.
#[test]
fn scalar_and_text_limits_count_values_instead_of_encoding_operations() {
    assert_accounting(&(), 1, 1, 0);
    assert_accounting(&true, 1, 1, 0);
    assert_accounting(&7u8, 1, 1, 0);
    assert_accounting(&7u16, 1, 1, 0);
    assert_accounting(&7u32, 1, 1, 0);
    assert_accounting(&7u64, 1, 1, 0);
    assert_accounting(&7usize, 1, 1, 0);
    assert_accounting(&-7i8, 1, 1, 0);
    assert_accounting(&-7i16, 1, 1, 0);
    assert_accounting(&-7i32, 1, 1, 0);
    assert_accounting(&-7i64, 1, 1, 0);
    assert_accounting(&-7isize, 1, 1, 0);
    assert_accounting(&1.25f32, 1, 1, 0);
    assert_accounting(&1.25f64, 1, 1, 0);
    // Text has explicit string semantics: its length prefix and UTF-8 bytes do
    // not behave like a vector's individual elements or values.
    assert_accounting(&String::new(), 1, 1, 0);
    assert_accounting(&"several UTF-8 bytes: é".to_owned(), 1, 1, 0);
}

/// Containers consume their own value, then share the remaining budget with children.
#[test]
fn collections_records_options_and_variants_have_symmetric_boundaries() {
    assert_accounting(&Vec::<u8>::new(), 1, 1, 0);
    assert_accounting(&vec![10u8, 20, 30], 4, 2, 3);
    assert_accounting(&vec![(); 3], 4, 2, 3);
    assert_accounting(&[10u8, 20, 30], 4, 2, 3);
    assert_accounting(&vec![vec![1u8, 2], vec![3]], 6, 3, 2);
    assert_accounting(&BudgetRecord { first: vec![(); 2], second: vec![(); 2] }, 7, 3, 2);
    assert_accounting(&BTreeMap::<u8, u8>::new(), 1, 1, 0);
    assert_accounting(&BTreeMap::from([(true, ())]), 3, 2, 1);
    assert_accounting(&BTreeMap::from([(7u8, vec![1u8, 2, 3])]), 6, 3, 3);
    assert_accounting(&Option::<u8>::None, 1, 1, 0);
    assert_accounting(&Some(7u8), 2, 2, 0);
    assert_accounting(&Some(Some(7u8)), 3, 3, 0);
    assert_accounting(&vec![None, Some(7u8)], 4, 3, 2);
    // Unit cases have no synthetic child; tuple and record payloads do have a
    // structural container in addition to the enclosing enum value.
    assert_accounting(&BudgetChoice::First, 1, 1, 0);
    assert_accounting(&BudgetChoice::Second, 1, 1, 0);
    assert_accounting(&Node::Empty, 1, 1, 0);
    assert_accounting(&Node::Label("label".into()), 2, 2, 0);
    assert_accounting(&Node::Size(7, 9), 4, 3, 2);
    assert_accounting(&Node::Group { enabled: true, children: vec![] }, 4, 3, 2);
}

/// A byte vector remains a collection even when a persistence envelope uses it
/// to store another encoded document. Saving must reject the same value limit
/// as restoring, rather than returning an artifact the receiver cannot accept.
#[test]
fn oversized_persistence_state_is_rejected_by_both_directions() {
    use neocogi_com::persistence::ObjectRecord;
    let record = ObjectRecord {
        type_id: [0; 16],
        schema_version: 1,
        state: vec![42; 1_048_576],
        children: vec![],
    };
    let limits = Limits {
        bytes: 16 * 1024 * 1024,
        elements: 16 * 1024 * 1024,
        depth: 192,
        values: 1_000_000,
    };
    assert!(binary::to_vec_with_limits(&record, limits).is_err());

    // A deliberately larger policy produces the formerly accepted envelope so
    // the reader's rejection is checked independently of the fixed writer.
    let permissive = Limits { values: 2_000_000, ..limits };
    let encoded = binary::to_vec_with_limits(&record, permissive).unwrap();
    assert_eq!(encoded.len(), 1_048_616);
    assert!(binary::from_slice_with_limits::<ObjectRecord>(&encoded, limits).is_err());
    assert_eq!(binary::from_slice_with_limits::<ObjectRecord>(&encoded, permissive).unwrap(), record);
}

/// Real records and virtual leaf envelopes must agree at every independent budget
/// boundary, including zero bytes, UUID elements and the final empty child vector.
#[test]
fn measured_leaf_envelopes_match_real_encoding_and_decoding() {
    use neocogi_com::persistence::ObjectRecord;
    for state_bytes in [0, 1, 15, 16, 17, 64] {
        let record = ObjectRecord {
            type_id: [0xab; 16],
            schema_version: u32::MAX,
            state: vec![0xff; state_bytes],
            children: vec![],
        };
        let encoded = binary::to_vec(&record).unwrap();
        let exact = Limits {
            bytes: encoded.len(),
            elements: state_bytes.max(16),
            depth: 3,
            values: state_bytes + 21,
        };
        assert_eq!(ObjectRecord::encoded_leaf_len(state_bytes, exact).unwrap(), encoded.len());
        assert_accounting(&record, exact.values, exact.depth, exact.elements);
        for limits in [
            Limits { bytes: exact.bytes - 1, ..exact },
            Limits { elements: exact.elements - 1, ..exact },
            Limits { depth: 2, ..exact },
            Limits { values: exact.values - 1, ..exact },
            Limits { bytes: 0, ..exact },
        ] {
            assert_eq!(
                ObjectRecord::encoded_leaf_len(state_bytes, limits).unwrap_err(),
                binary::to_vec_with_limits(&record, limits).unwrap_err()
            );
            assert!(binary::from_slice_with_limits::<ObjectRecord>(&encoded, limits).is_err());
        }
    }
}

/// Admission visits borrowed payloads without allocating their encoded copies;
/// actual encoding reports reservation failure instead of aborting the process.
#[test]
fn measurement_needs_no_output_allocation_and_encoding_failure_is_recoverable() {
    use allocation_probe::Probe;
    let text = "x".repeat(2 * 1024 * 1024);
    let probe = Probe::start(Some(0));
    let measured = binary::encoded_len_with_limits(&text, Limits::default());
    let envelope = neocogi_com::persistence::ObjectRecord::encoded_leaf_len(text.len(), Limits::default());
    let report = probe.finish();
    assert_eq!(measured.unwrap(), text.len() + 8);
    assert!(envelope.is_err());
    assert_eq!(report.attempts, 0);
    assert_eq!(report.denied, 0);

    let probe = Probe::start(Some(0));
    let failed = binary::to_vec(&text);
    let report = probe.finish();
    assert_eq!(failed.unwrap_err().message, "output allocation failed");
    assert_eq!(report.denied, 1);
}

/// Ordinary ordered fields must be required unless trailing defaults are explicit.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct OrderedState {
    /// First payload also checks that text remains one value rather than a byte sequence.
    label: String,
    /// Restricted scalar must remain checked even after acquiring the preceding field.
    enabled: bool,
}

/// Defaults apply only to missing trailing fields, never malformed present fields.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
#[com_deserialize(default)]
struct DefaultState {
    /// Retained when the encoded record has no fields.
    label: String,
    /// Retained when the encoded record contains only its label.
    enabled: bool,
}
impl Default for DefaultState {
    /// Uses nonzero defaults so omitted values cannot be confused with fabricated zeros.
    fn default() -> Self {
        Self { label: "untitled".into(), enabled: true }
    }
}

/// Literal bytes are independent of both generated codecs and their helpers.
#[test]
fn positional_records_preserve_golden_bytes_defaults_and_error_offsets() {
    let full = b"NCB\x01\x02\0\0\0\x04\0\0\0Save\x01";
    let value = OrderedState { label: "Save".into(), enabled: true };
    assert_eq!(binary::to_vec(&value).unwrap(), full);
    assert_eq!(binary::from_slice::<OrderedState>(full).unwrap(), value);
    let mut fixed = [0; 32];
    let len = binary::to_slice(&value, &mut fixed).unwrap();
    assert_eq!(&fixed[..len], full);

    let prefix = b"NCB\x01\x01\0\0\0\x04\0\0\0Save";
    let error = binary::from_slice::<OrderedState>(prefix).unwrap_err();
    assert_eq!(error, binary::Error { offset: 16, message: "missing field" });
    assert_eq!(
        binary::from_slice::<DefaultState>(prefix).unwrap(),
        DefaultState { label: "Save".into(), enabled: true }
    );
    assert_eq!(binary::from_slice::<DefaultState>(b"NCB\x01\0\0\0\0").unwrap(), DefaultState::default());

    // Encoded field count two promises a bool. Its absence is truncation, not
    // permission to substitute Default; invalid present data also stays invalid.
    assert_eq!(
        binary::from_slice::<DefaultState>(&full[..16]).unwrap_err(),
        binary::Error { offset: 16, message: "truncated value" }
    );
    let mut invalid = *full;
    invalid[16] = 2;
    assert_eq!(
        binary::from_slice::<DefaultState>(&invalid).unwrap_err(),
        binary::Error { offset: 17, message: "invalid boolean" }
    );
    invalid[8] = 5;
    assert_eq!(
        binary::from_slice::<OrderedState>(&invalid[..16]).unwrap_err(),
        binary::Error { offset: 12, message: "truncated value" }
    );
    invalid[4] = 3;
    assert_eq!(
        binary::from_slice::<DefaultState>(&invalid).unwrap_err(),
        binary::Error {
            offset: 8,
            message: "unknown record fields"
        }
    );
}

/// Counts currently live values in the sole partial-construction resource test.
static ACQUIRED_FIELDS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Monotonic acquisitions prove each failure occurs after acquiring prior fields.
static TOTAL_ACQUIRED_FIELDS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A decoded field with observable destruction and no serialized owner pointer.
#[derive(Debug)]
struct Acquired;
impl ComDeserialize for Acquired {
    /// Reads one byte inside the current value scope before publishing this test resource.
    fn com_deserialize(reader: &mut binary::Reader<'_>) -> Result<Self, binary::Error> {
        u8::com_deserialize(reader)?;
        ACQUIRED_FIELDS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        TOTAL_ACQUIRED_FIELDS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(Self)
    }
}
impl Drop for Acquired {
    /// Every acquired field must be destroyed on a later field's decoding failure.
    fn drop(&mut self) {
        ACQUIRED_FIELDS.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Uses direct named-field construction with a fallible final scalar.
#[derive(Debug, ComDeserialize)]
struct PartialRecord {
    /// Resource acquired before a later failure.
    _first: Acquired,
    /// A second resource proves cleanup is not limited to one temporary.
    _second: Acquired,
    /// Invalid tags force construction to unwind through ordinary Result propagation.
    _flag: bool,
}

/// Both multi-field payload forms must release acquired values without helper structs.
#[derive(Debug, ComDeserialize)]
#[allow(dead_code)] // The failure cases never publish a completed enum to inspect.
enum PartialChoice {
    /// Positional variant with one acquired resource and a failing scalar.
    Pair(Acquired, bool),
    /// Named variant with the same resource ownership but record diagnostics.
    Record {
        /// Resource acquired before the invalid tag.
        _first: Acquired,
        /// Restricted scalar rejected before constructing the variant.
        _flag: bool,
    },
}

/// Typed locals, arrays and enum constructor arguments must all clean up on error.
#[test]
fn partial_ordered_construction_drops_every_acquired_field() {
    assert_eq!(ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst), 0);
    let before = TOTAL_ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        binary::from_slice::<PartialRecord>(b"NCB\x01\x03\0\0\0\x01\x01\x02").unwrap_err(),
        binary::Error { offset: 11, message: "invalid boolean" }
    );
    assert_eq!(TOTAL_ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst) - before, 2);
    assert_eq!(ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst), 0);
    let before = TOTAL_ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        binary::from_slice::<[Acquired; 3]>(b"NCB\x01\x03\0\0\0\x01\x01").unwrap_err(),
        binary::Error { offset: 10, message: "truncated value" }
    );
    assert_eq!(TOTAL_ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst) - before, 2);
    assert_eq!(ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst), 0);
    for ordinal in [0u8, 1] {
        let mut bytes = *b"NCB\x01\0\0\0\0\x02\0\0\0\x01\x02";
        bytes[4] = ordinal;
        let before = TOTAL_ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            binary::from_slice::<PartialChoice>(&bytes).unwrap_err(),
            binary::Error { offset: 14, message: "invalid boolean" }
        );
        assert_eq!(TOTAL_ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst) - before, 1);
        assert_eq!(ACQUIRED_FIELDS.load(std::sync::atomic::Ordering::SeqCst), 0);
    }
}
