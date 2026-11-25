//! Production codecs share typed values, bounded sessions and static registration.
#![cfg(feature = "plugin")]
use std::collections::BTreeMap;
use neocogi_com_sdk::{ComSerialize, ComDeserialize, NcCopyFromBufferOffset, NC_E_NOINTERFACE};
use neocogi_com_sdk::serialization::{Format, Limits};

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
/// The default static feature set has NCB and omits the optional JSON factory.
#[cfg(not(feature = "json"))]
#[test]
fn default_static_plugin_omits_json() {
    let root = neocogi_com_serialization::factory();
    assert_eq!(root.format(NcCopyFromBufferOffset::new(0, b"json")).unwrap_err(), NC_E_NOINTERFACE);
    let format = Format::new(root.default_format().unwrap(), Limits::default());
    assert_eq!(format.serialize(&()).unwrap(), b"NCB\x01");
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

/// A counted sequence with children must admit their next level when it opens.
#[test]
fn nonempty_sequence_preflights_its_next_depth() {
    use neocogi_com_sdk::NC_E_OUT_OF_RANGE;
    let root = neocogi_com_serialization::factory();
    for (name, _) in formats() {
        let factory = root.format(NcCopyFromBufferOffset::new(0, name.as_bytes())).unwrap();
        for depth in [2, 129] {
            let format = Format::new(factory.clone(), Limits { depth, ..Limits::default() });
            let ceiling = depth.min(128);
            for count in [0, 1] {
                let mut writer = format.serializer().unwrap();
                let mut view = writer.as_com_mut();
                for _ in 1..ceiling {
                    view.begin_sequence(1).unwrap();
                }
                if count == 0 {
                    view.begin_sequence(0).unwrap();
                    for _ in 0..ceiling {
                        view.end_sequence().unwrap();
                    }
                    view.finish().unwrap();
                } else {
                    assert_eq!(view.begin_sequence(1), Err(NC_E_OUT_OF_RANGE), "{name} at {depth}");
                    assert_eq!(view.finish(), Err(NC_E_OUT_OF_RANGE), "{name} is terminal");
                }
            }
        }
    }
}

/// Saved objects use four positional values in both production formats.
#[test]
fn saved_object_reads_exact_arity_and_json_uses_positions() {
    use neocogi_com_sdk::{NC_E_INVALID_ARG, persistence::SavedObject};
    let value = SavedObject {
        type_id: [7; 16],
        schema_version: 2,
        state: vec![42],
        children: vec![],
    };
    for (name, format) in formats() {
        let bytes = format.serialize(&value).unwrap();
        assert_eq!(format.deserialize::<SavedObject>(&bytes).unwrap(), value);
        for count in [3_u32, 5] {
            let malformed = if name == "ncb" {
                let mut prefix = b"NCB\x01".to_vec();
                prefix.extend(count.to_le_bytes());
                prefix
            } else {
                let slots = vec!["null"; count as usize].join(",");
                format!("[{slots}]").into_bytes()
            };
            let failure = format.deserialize::<SavedObject>(&malformed).unwrap_err();
            assert_eq!(failure.status, NC_E_INVALID_ARG);
            assert_eq!(failure.message, "invalid sequence length");
        }
        if name == "json" {
            assert_eq!(bytes, b"[[7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7],2,[42],[]]");
        } else {
            let mut expected = b"NCB\x01\x04\0\0\0\x10\0\0\0".to_vec();
            expected.extend([7; 16]);
            expected.extend(b"\x02\0\0\0\x01\0\0\0\x2a\0\0\0\0");
            assert_eq!(bytes, expected);
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
    use neocogi_com_sdk::serialization::{ISerializationDiagnosticsAbi, Deserializer};
    for (name, format) in formats() {
        let broken: &[u8] = match name {
            "ncb" => b"bad!",
            "json" => b"{\n \"count\": ]\n}",
            _ => unreachable!(),
        };
        let mut source = format.deserializer(broken).unwrap();
        assert!(Deserializer::new(source.as_com_mut()).deserialize::<Defaults>().is_err());
        let details = source.query_ref::<ISerializationDiagnosticsAbi>().unwrap();
        let status = details.failure_status().unwrap();
        assert!(status.is_err());
        assert!(details.message_len().unwrap() > 0);
        let mut short = neocogi_com_sdk::NcOutputBuffer::<u8>::new();
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
        assert!(source.finish().is_err());
        assert_eq!(source.query_ref::<ISerializationDiagnosticsAbi>().unwrap().failure_status().unwrap(), status);
    }
}

/// Explicit failure reporting preserves the first status and location across
/// owning serializer and source calls, even when the later diagnostic is invalid.
#[test]
fn owning_sessions_keep_the_first_origin_status() {
    use neocogi_com_sdk::{NC_E_INVALID_ARG, NC_E_NOT_FOUND, NC_E_OUT_OF_RANGE, NC_OK, NcStatus};
    use neocogi_com_sdk::serialization::{Deserializer, Error, ISerializationDiagnosticsAbi, ISerializationOutputAbi, Serializer};

    let custom = NcStatus::from_code(-731);
    let malformed = NcCopyFromBufferOffset::new(1, b"\xff");
    for (name, format) in formats() {
        let bytes = format.serialize(&17_u8).unwrap();
        for (status, message, expected) in [
            (custom, NcCopyFromBufferOffset::new(0, b"application failure"), custom),
            (NC_OK, NcCopyFromBufferOffset::new(0, b"application failure"), NC_E_INVALID_ARG),
            (NC_E_OUT_OF_RANGE, NcCopyFromBufferOffset::new(1, b"valid text"), NC_E_INVALID_ARG),
            (NC_E_NOT_FOUND, NcCopyFromBufferOffset::new(0, b"\xff"), NC_E_INVALID_ARG),
        ] {
            let mut writer = format.serializer().unwrap();
            assert_eq!(writer.fail(status, message), Err(expected), "{name}");
            let first = Error::capture(expected, &writer);
            assert_eq!(writer.query_ref::<ISerializationDiagnosticsAbi>().unwrap().failure_status(), Ok(expected));
            assert_eq!(writer.fail(NC_E_NOT_FOUND, malformed), Err(expected));
            assert_eq!(writer.finish(), Err(expected));
            assert_eq!(Error::capture(expected, &writer), first);
            assert_eq!(writer.query_ref::<ISerializationOutputAbi>().unwrap().byte_len(), Err(NC_E_INVALID_ARG));

            let mut reader = format.deserializer(&bytes).unwrap();
            assert_eq!(reader.fail(status, message), Err(expected), "{name}");
            let first = Error::capture(expected, &reader);
            assert_eq!(reader.query_ref::<ISerializationDiagnosticsAbi>().unwrap().failure_status(), Ok(expected));
            assert_eq!(reader.fail(NC_E_NOT_FOUND, malformed), Err(expected));
            assert_eq!(reader.finish(), Err(expected));
            assert_eq!(Error::capture(expected, &reader), first);
            if status == custom {
                assert_eq!(first.message, "application failure");
                assert_eq!(first.path, "$");
            }
        }

        let mut writer = format.serializer().unwrap();
        Serializer::new(writer.as_com_mut()).serialize(&17_u8).unwrap();
        assert_eq!(writer.fail(custom, malformed), Err(NC_E_INVALID_ARG));
        assert_eq!(
            writer.query_ref::<ISerializationDiagnosticsAbi>().unwrap().failure_status(),
            Err(NC_E_INVALID_ARG)
        );
        assert_eq!(writer.query_ref::<ISerializationOutputAbi>().unwrap().byte_len(), Ok(bytes.len() as u64));

        let mut reader = format.deserializer(&bytes).unwrap();
        assert_eq!(Deserializer::new(reader.as_com_mut()).deserialize::<u8>(), Ok(17));
        assert_eq!(reader.fail(custom, malformed), Err(NC_E_INVALID_ARG));
        assert_eq!(
            reader.query_ref::<ISerializationDiagnosticsAbi>().unwrap().failure_status(),
            Err(NC_E_INVALID_ARG)
        );

        let mut writer = format.serializer().unwrap();
        let status = writer.finish().unwrap_err();
        let first = Error::capture(status, &writer);
        assert_eq!(writer.fail(custom, malformed), Err(status));
        assert_eq!(Error::capture(status, &writer), first);

        let mut reader = format.deserializer(&bytes).unwrap();
        reader.enter_value().unwrap();
        let status = reader.deserialize_bool().unwrap_err();
        let first = Error::capture(status, &reader);
        assert_eq!(reader.fail(custom, malformed), Err(status));
        assert_eq!(Error::capture(status, &reader), first);
    }
}

/// The typed caller's resource and application errors have the same status as
/// terminal diagnostics from either built-in format.
#[test]
fn typed_failures_agree_with_both_codec_diagnostics() {
    use neocogi_com_sdk::{NC_E_OUT_OF_RANGE, NcStatus};
    use neocogi_com_sdk::serialization::{Deserializer, Error, ISerializationDiagnosticsAbi, Serializer};

    const CUSTOM: NcStatus = NcStatus::from_code(-731);
    #[derive(Debug)]
    struct Fails;
    impl ComSerialize for Fails {
        fn com_serialize(&self, _: &mut Serializer<'_>) -> Result<(), NcStatus> {
            Err(CUSTOM)
        }
    }
    impl ComDeserialize for Fails {
        fn com_deserialize(_: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
            Err(CUSTOM)
        }
    }

    for (_, format) in formats() {
        let bytes = format.serialize(&[1_u8, 2]).unwrap();
        let mut reader = format.deserializer(&bytes).unwrap();
        let limits = Limits { elements: 1, ..Limits::default() };
        assert_eq!(
            Deserializer::with_limits(reader.as_com_mut(), limits).deserialize::<Vec<u8>>(),
            Err(NC_E_OUT_OF_RANGE)
        );
        assert_eq!(
            reader.query_ref::<ISerializationDiagnosticsAbi>().unwrap().failure_status(),
            Ok(NC_E_OUT_OF_RANGE)
        );
        assert_eq!(Error::capture(NC_E_OUT_OF_RANGE, &reader).message, "collection element limit");

        let bytes = format.serialize("bound").unwrap();
        let mut reader = format.deserializer(&bytes).unwrap();
        let limits = Limits { bytes: 4, ..Limits::default() };
        assert_eq!(
            Deserializer::with_limits(reader.as_com_mut(), limits).deserialize::<String>(),
            Err(NC_E_OUT_OF_RANGE)
        );
        assert_eq!(
            reader.query_ref::<ISerializationDiagnosticsAbi>().unwrap().failure_status(),
            Ok(NC_E_OUT_OF_RANGE)
        );
        assert_eq!(Error::capture(NC_E_OUT_OF_RANGE, &reader).message, "document byte limit");

        let bytes = format.serialize(&17_u8).unwrap();
        let error = format.deserialize::<Fails>(&bytes).unwrap_err();
        assert_eq!(error.status, CUSTOM);
        assert_eq!(error.message, "invalid persistent value");
        let error = format.serialize(&Fails).unwrap_err();
        assert_eq!(error.status, CUSTOM);
        assert_eq!(error.message, "invalid persistent value");
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
