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
