//! Production codecs share typed values, bounded sessions and static registration.
#![cfg(feature = "plugin")]
use std::collections::BTreeMap;
use neocogi_com::{ComSerialize, ComDeserialize, NcCopyFromBufferOffset, NC_E_NOINTERFACE};
use neocogi_com::serialization::{Format, Limits};

/// Exercises all derived payload shapes and named case probing.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
enum Case {
    Empty,
    One(u64),
    Pair(bool, String),
    Named { r#type: i32, values: Vec<u16> },
}
/// Representative nested document with integer endpoints and nontrivial UTF-8.
#[derive(Debug, PartialEq, ComSerialize, ComDeserialize)]
struct Document {
    label: String,
    flag: bool,
    unit: (),
    unsigned: u64,
    signed: i64,
    small: u8,
    short: i16,
    word: u32,
    real: f64,
    single: f32,
    choices: Vec<Case>,
    entries: BTreeMap<i32, String>,
    option: Option<Option<u8>>,
}
/// Includes both escaped control characters and Unicode line separators.
fn document() -> Document {
    Document {
        label: "é\n\"\\\0\u{85}\u{2028}\u{2029}".into(),
        flag: true,
        unit: (),
        unsigned: u64::MAX,
        signed: i64::MIN,
        small: u8::MAX,
        short: i16::MIN,
        word: u32::MAX,
        real: -0.0,
        single: 0.125,
        choices: vec![
            Case::Empty,
            Case::One(u64::MAX),
            Case::Pair(false, "true".into()),
            Case::Named { r#type: -42, values: vec![0, u16::MAX] },
        ],
        entries: BTreeMap::from([(-9, "left".into()), (17, "right".into())]),
        option: Some(None),
    }
}
/// Iterates only factories actually compiled into this one plugin.
fn formats() -> Vec<(&'static str, Format)> {
    let root = neocogi_com_serialization::factory();
    let mut result = Vec::new();
    for (name, present) in [("ncb", true), ("json", cfg!(feature = "json"))] {
        let factory = root.format(NcCopyFromBufferOffset::new(0, name.as_bytes()));
        if present {
            result.push((name, Format::new(factory.unwrap(), Limits::default())));
        } else {
            assert_eq!(factory.unwrap_err(), NC_E_NOINTERFACE);
        }
    }
    result
}
/// The default never changes when optional codecs are enabled or disabled.
#[test]
fn ncb_is_unconditional_and_default() {
    let root = neocogi_com_serialization::factory();
    let format = Format::new(root.default_format().unwrap(), Limits::default());
    assert_eq!(format.serialize(&42u32).unwrap(), b"NCB\x01\x2a\0\0\0");
    assert!(root.format(NcCopyFromBufferOffset::new(0, b"unknown")).is_err());
    assert_eq!(
        neocogi_com_serialization::ncb::format(Limits::default()).serialize(&42u32).unwrap(),
        b"NCB\x01\x2a\0\0\0"
    );
}
/// Integer endpoints, nested options, arbitrary map keys and every payload shape
/// survive actual COM calls through every enabled production provider.
#[test]
fn all_enabled_formats_preserve_the_same_typed_values() {
    let value = document();
    for (name, format) in formats() {
        let bytes = format.serialize(&value).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let decoded: Document = format
            .deserialize(&bytes)
            .unwrap_or_else(|e| panic!("{name}: {e:?}\n{}", String::from_utf8_lossy(&bytes)));
        assert_eq!(decoded, value, "{name}");
        assert_eq!(decoded.real.to_bits(), value.real.to_bits(), "negative zero: {name}");
        for option in [None, Some(None), Some(Some(7u8))] {
            assert_eq!(format.deserialize::<Option<Option<u8>>>(&format.serialize(&option).unwrap()).unwrap(), option);
        }
        let bytes = format.serialize(&Vec::<u32>::new()).unwrap();
        assert!(format.deserialize::<Vec<u32>>(&bytes).unwrap().is_empty());
    }
}
/// Exact encoded bytes and logical budgets apply independently of text framing.
#[test]
fn byte_value_container_and_depth_limits_are_enforced() {
    for (name, full) in formats() {
        let value = vec![1u32, 2, 3];
        let bytes = full.serialize(&value).unwrap();
        let factory = neocogi_com_serialization::factory()
            .format(NcCopyFromBufferOffset::new(0, name.as_bytes()))
            .unwrap();
        let exact = Limits {
            bytes: bytes.len(),
            elements: 3,
            depth: 2,
            values: 4,
        };
        let format = Format::new(factory.clone(), exact);
        assert_eq!(format.serialize(&value).unwrap(), bytes, "{name}");
        assert_eq!(format.deserialize::<Vec<u32>>(&bytes).unwrap(), value, "{name}");
        for limited in [
            Limits { bytes: exact.bytes - 1, ..exact },
            Limits { elements: 2, ..exact },
            Limits { depth: 1, ..exact },
            Limits { values: 3, ..exact },
        ] {
            let format = Format::new(factory.clone(), limited);
            assert!(format.serialize(&value).is_err(), "{name}: {limited:?}");
            assert!(format.deserialize::<Vec<u32>>(&bytes).is_err(), "{name}: {limited:?}");
        }
    }
}
/// Local typed defaults are applied only to genuinely absent fields.
#[derive(Debug, Default, PartialEq, ComSerialize, ComDeserialize)]
#[com_deserialize(default)]
struct Defaults {
    first: bool,
    count: u32,
}

/// Named formats accept authored field order independently of encoded order and
/// reject duplicate/unknown fields, malformed syntax and extra documents.
#[test]
fn named_formats_validate_syntax_and_schema() {
    for (name, format) in formats().into_iter().filter(|(name, _)| *name != "ncb") {
        let good = br#"{"count":7}"#;
        assert_eq!(format.deserialize::<Defaults>(good).unwrap(), Defaults { first: false, count: 7 });
        let invalid: Vec<&[u8]> = vec![
            br#"{"count":7,"count":8}"#,
            br#"{"count":7,"extra":0}"#,
            br#"{"count":true}"#,
            b"{} {}",
            b"[",
            b"\xff",
        ];
        for bytes in invalid {
            assert!(format.deserialize::<Defaults>(bytes).is_err(), "{name}: {:?}", bytes);
        }
        assert!(format.deserialize::<u8>(b"256").is_err());
        assert!(format.deserialize::<bool>(b"2").is_err());
        assert!(format.deserialize::<f32>(b"1e100").is_err());
        assert!(format.serialize(&f64::NAN).is_err());
        assert!(format.serialize(&f64::INFINITY).is_err());
        assert!(format.deserialize::<BTreeMap<u32, u32>>(br#"{"$map":[[1,2],[1,3]]}"#).is_err());
    }
}
/// Diagnostics survive destruction of the source object through the convenience
/// API, and remain queryable directly through the read-only COM projection.
#[test]
fn failures_report_their_message_and_location() {
    use neocogi_com::serialization::{ISerializationDiagnosticsAbi, Deserializer};
    for (name, format) in formats() {
        let broken: &[u8] = match name {
            "ncb" => b"bad!",
            "json" => b"{\n \"count\": ]\n}",
            _ => unreachable!(),
        };
        let source = format.deserializer(broken).unwrap();
        assert!(Deserializer::new(source.as_com_ref()).deserialize::<Defaults>().is_err());
        let details = source.query_ref::<ISerializationDiagnosticsAbi>().unwrap();
        let status = details.failure_status().unwrap();
        assert!(status.is_err());
        assert!(details.message_len().unwrap() > 0);
        let mut short = neocogi_com::NcOutputBuffer::<u8>::new();
        assert!(details.copy_message(short.output(0, 0)).is_err());
        assert!(short.is_empty());
        let error = format.deserialize::<Defaults>(broken).unwrap_err();
        assert_eq!(error.status, status);
        assert!(!error.message.is_empty());
        if name == "ncb" {
            assert!(error.byte_offset.is_some());
        } else {
            assert!(error.line.is_some());
            assert!(error.column.is_some());
        }
        assert_eq!(details.failure_status().unwrap(), status);
        assert!(source.as_com_ref().finish().is_err());
        assert_eq!(details.failure_status().unwrap(), status);
    }
}
/// Schema failures identify the actual nested field/index without formatting or
/// allocating a path string for successful scalar operations.
#[derive(Debug, ComDeserialize)]
#[allow(dead_code)]
struct Outer {
    items: Vec<Defaults>,
}

#[test]
fn named_schema_errors_report_the_field_path_and_expected_type() {
    for (name, format) in formats().into_iter().filter(|(name, _)| *name != "ncb") {
        let bytes: &[u8] = if name == "json" {
            br#"{"items":[{"count":true}]}"#
        } else {
            b"items:\n  - count: true\n"
        };
        let error = format.deserialize::<Outer>(bytes).unwrap_err();
        assert_eq!(error.path, "$.items[0].count", "{name}: {error}");
        assert!(error.message.contains("u32"), "{name}: {error}");
        assert!(error.message.contains("boolean"), "{name}: {error}");
    }
}

/// Floating serialization is lossless, including subnormals and negative zero.
#[test]
fn finite_float_bits_round_trip() {
    for (name, format) in formats() {
        for bits in [0, 1, 0x8000000000000000, 0x0010000000000000, 0x3fd5555555555555, 0x7fefffffffffffff] {
            let value = f64::from_bits(bits);
            let bytes = format.serialize(&value).unwrap();
            assert_eq!(format.deserialize::<f64>(&bytes).unwrap().to_bits(), bits, "{name}");
        }
    }
}
