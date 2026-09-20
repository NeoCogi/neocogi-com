//! Persistent values are portable, deterministic, and reject malformed inputs.

use neocogi_com::{
    binary::{self, Limits},
    ComDeserialize, ComSerialize,
};
use std::collections::BTreeMap;

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
