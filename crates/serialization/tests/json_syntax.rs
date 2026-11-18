//! Strict custom JSON grammar, Unicode, numeric precision and error coordinates.
#![cfg(feature = "json")]
use neocogi_com::serialization::{Format, Limits, ISerializationDiagnosticsAbi};

#[path = "support/json_numbers.rs"]
mod json_numbers;

/// Direct-width external conversion is independent from own-writer round trips.
#[test]
fn external_numbers_round_directly_and_preserve_owned_terminal_state() {
    json_numbers::assert_json_numbers(&format());
}

/// Exercises the normal owning COM factory rather than a private parser shortcut.
fn format() -> Format {
    Format::new(neocogi_com_serialization::json::factory(), Limits::default())
}

/// Parsing completes before a source is returned; an active source has no failure
/// even before a caller chooses which concrete application type to construct.
#[test]
fn valid_json_grammar_is_accepted_without_a_type_assumption() {
    let format = format();
    for bytes in [
        "null",
        "true",
        "false",
        "0",
        "-0",
        "-0.5",
        "1E+3",
        "1e-3",
        "[]",
        "{}",
        " \r\n\t[true, null, {\"é\":\"x\"}] \n",
    ] {
        let source = format.deserializer(bytes.as_bytes()).unwrap();
        let details = source.query_ref::<ISerializationDiagnosticsAbi>().unwrap();
        assert!(details.failure_status().is_err(), "rejected {bytes:?}");
    }
}
/// Each malformed document fails during syntax admission, independently of any
/// typed-value mismatch that might otherwise mask acceptance of invalid JSON.
#[test]
fn malformed_json_is_rejected_before_typed_construction() {
    let format = format();
    for bytes in [
        "",
        " ",
        "True",
        "undefined",
        "NaN",
        "Infinity",
        "+1",
        "01",
        "-01",
        "-",
        "--1",
        ".1",
        "1.",
        "1.e2",
        "1e",
        "1e+",
        "1e-",
        "1e9999",
        "18446744073709551616",
        "-9223372036854775809",
        "[1,]",
        "[,1]",
        "[1 2]",
        "[",
        "{",
        "{x:1}",
        "{\"x\" 1}",
        "{\"x\":1,}",
        "{\"x\":1 \"y\":2}",
        "{\"x\":1,\"x\":2}",
        "true false",
        "nullX",
        "\u{b}0",
        "\u{a0}0",
        "\u{feff}{}",
        "\"unterminated",
        "\"a\nb\"",
        r#""\x20""#,
        r#""\u0""#,
        r#""\u00xz""#,
        r#""\ud800""#,
        r#""\udfff""#,
        r#""\ud800\u0041""#,
        r#""\ud800x\udc00""#,
    ] {
        let source = format.deserializer(bytes.as_bytes()).unwrap();
        let details = source.query_ref::<ISerializationDiagnosticsAbi>().unwrap();
        assert!(details.failure_status().is_ok_and(|status| status.is_err()), "accepted {bytes:?}");
        assert!(details.message_len().unwrap() > 0);
    }
}
/// Escapes decode to scalar values, including surrogate pairs and NUL; the writer
/// preserves every ASCII control and copies ordinary Unicode as UTF-8 in bulk.
#[test]
fn strings_preserve_unicode_and_escape_semantics() {
    let format = format();
    for (source, expected) in [
        (r#""\"\\\/\b\f\n\r\t""#, "\"\\/\u{8}\u{c}\n\r\t"),
        (r#""\u0000\u0041\u00e9""#, "\0Aé"),
        (r#""\ud83d\ude03""#, "😃"),
        (r#""\udbff\udfff""#, "\u{10ffff}"),
        ("\"é\u{85}\u{2028}\u{2029}\"", "é\u{85}\u{2028}\u{2029}"),
    ] {
        assert_eq!(format.deserialize::<String>(source.as_bytes()).unwrap(), expected);
    }
    let all_ascii: String = (0u8..=127).map(char::from).collect();
    let encoded = format.serialize(&all_ascii).unwrap();
    assert_eq!(format.deserialize::<String>(&encoded).unwrap(), all_ascii);
    assert_eq!(format.serialize(&"\0\u{8}\u{c}\n\r\t\"\\").unwrap(), br#""\u0000\b\f\n\r\t\"\\""#);
}
/// Shortest standard-library float formatting and parsing retain finite bits,
/// including generated exponent/mantissa combinations across the entire range.
#[test]
fn generated_finite_floats_round_trip_bit_for_bit() {
    let format = format();
    let mut bits = 0x123456789abcdef0u64;
    for _ in 0..2048 {
        bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let value = f64::from_bits(bits);
        if value.is_finite() {
            let bytes = format.serialize(&value).unwrap();
            assert_eq!(
                format.deserialize::<f64>(&bytes).unwrap().to_bits(),
                bits,
                "{}",
                String::from_utf8_lossy(&bytes)
            );
        }
        let value = f32::from_bits(bits as u32);
        if value.is_finite() {
            let bytes = format.serialize(&value).unwrap();
            assert_eq!(format.deserialize::<f32>(&bytes).unwrap().to_bits(), value.to_bits());
        }
    }
}
/// Coordinates count UTF-8 bytes for offsets and Unicode scalars for columns.
#[test]
fn syntax_errors_identify_the_exact_byte_line_and_column() {
    let bytes = "{\n \"name\": \"é\",\n \"bad\": ]\n}".as_bytes();
    let source = format().deserializer(bytes).unwrap();
    let details = source.query_ref::<ISerializationDiagnosticsAbi>().unwrap();
    assert_eq!(details.byte_offset().unwrap(), 25);
    assert_eq!(details.line().unwrap(), 3);
    assert_eq!(details.column().unwrap(), 9);
    let error = format().deserialize::<String>(b"\"a\xff\"").unwrap_err();
    assert_eq!(error.byte_offset, Some(2));
    assert_eq!((error.line, error.column), (Some(1), Some(3)));
}
/// Syntax's minimum two framing slots do not permit parsing an extra subtree.
#[test]
fn collection_admission_stops_before_the_next_subtree() {
    let format = Format::new(neocogi_com_serialization::json::factory(), Limits { elements: 0, ..Limits::default() });
    let error = format.deserialize::<Vec<()>>(b"[null,null,INVALID]").unwrap_err();
    assert_eq!(error.status, neocogi_com::NC_E_OUT_OF_RANGE);
    assert_eq!(error.byte_offset, Some(11));
}

/// Entered, selected and missing fields share the unambiguous name rendering.
#[test]
fn logical_paths_quote_ambiguous_literal_field_names() {
    use neocogi_com::{ComDeserialize, NC_E_INVALID_ARG, NcCopyFromBufferOffset, NcStatus};
    use neocogi_com::serialization::{Deserializer, Error};

    #[derive(Debug, ComDeserialize)]
    #[allow(dead_code)]
    struct Nested {
        a: Inner,
    }
    #[derive(Debug, ComDeserialize)]
    #[allow(dead_code)]
    struct Inner {
        b: bool,
    }

    let format = format();
    let nested = format.deserialize::<Nested>(br#"{"a":{"b":7}}"#).unwrap_err();
    assert_eq!(nested.path, "$.a.b");
    let option = format.deserialize::<Option<bool>>(br#"{"$some":7}"#).unwrap_err();
    assert_eq!(option.path, "$.$some");

    /// Reads a single literal dotted name through the typed record facade.
    #[derive(Debug)]
    struct Dotted;
    impl ComDeserialize for Dotted {
        fn com_deserialize(reader: &mut Deserializer<'_>) -> Result<Self, NcStatus> {
            reader.record(1, |fields| {
                fields.read_field::<bool>("a.b")?.map(|_| Self).ok_or_else(|| fields.error("missing field"))
            })
        }
    }
    let literal = format.deserialize::<Dotted>(br#"{"a.b":7}"#).unwrap_err();
    assert_eq!(literal.path, r#"$["a.b"]"#);

    for (name, document, path) in [
        ("a.b", r#"{"a.b":7}"#, r#"$["a.b"]"#),
        ("a\"b", r#"{"a\"b":7}"#, r#"$["a\"b"]"#),
        ("a\\b", r#"{"a\\b":7}"#, r#"$["a\\b"]"#),
        ("a[0]", r#"{"a[0]":7}"#, r#"$["a[0]"]"#),
        ("", r#"{"":7}"#, r#"$[""]"#),
        ("a\x01\n", r#"{"a\u0001\n":7}"#, r#"$["a\u0001\n"]"#),
        ("é", r#"{"é":7}"#, r#"$["é"]"#),
        ("$some", r#"{"$some":7}"#, r#"$["$some"]"#),
    ] {
        let key = NcCopyFromBufferOffset::new(0, name.as_bytes());
        for present in [false, true] {
            let limits = Limits { depth: 1, ..Limits::default() };
            let bounded = Format::new(neocogi_com_serialization::json::factory(), limits);
            let source = bounded.deserializer(if present { document.as_bytes() } else { b"{}" }).unwrap();
            let mut view = source.as_com_ref();
            view.enter_value().unwrap();
            assert_eq!(view.begin_record(1), Ok(u32::from(present)));
            assert_eq!(view.field(key), Ok(present));
            let error = if present {
                view.enter_value().unwrap_err()
            } else {
                view.fail(NcCopyFromBufferOffset::new(0, b"missing field")).unwrap_err()
            };
            assert_eq!(error, if present { neocogi_com::NC_E_OUT_OF_RANGE } else { NC_E_INVALID_ARG });
            let first = Error::capture(error, &source);
            assert_eq!(first.path, path);
            assert_eq!(view.finish(), Err(error));
            assert_eq!(Error::capture(error, &source), first);
        }
    }
}
