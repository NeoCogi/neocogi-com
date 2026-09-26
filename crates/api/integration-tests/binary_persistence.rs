//! Persistent values are portable, deterministic, and reject malformed inputs.

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

/// A wholly fieldless enum follows a separate derive path from payload enums.
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
        assert!(binary::to_vec_with_limits(value, limits).is_err(), "writer accepted {value:?} under {limits:?}");
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
