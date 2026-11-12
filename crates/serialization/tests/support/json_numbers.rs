//! The same numeric contract crosses statically linked and native COM factories.
use neocogi_com::{ComDeserialize, NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
use neocogi_com::serialization::{Deserializer, Error, Format, ISerializationDiagnosticsAbi};

/// Generated construction must mix narrow and wide fields without narrowing the
/// wide-only value or losing the path of a later narrow acquisition failure.
#[derive(Debug, ComDeserialize)]
struct Numbers {
    /// External decimal and integral spellings acquired at their requested width.
    values: Vec<f32>,
    /// A finite f64 that cannot be represented by f32.
    wide: f64,
}

/// Check known binary32 results at midpoint, sign, integer and range boundaries.
/// Every convenience call creates a fresh owning COM source for that one width.
pub fn assert_json_numbers(format: &Format) {
    for (text, bits) in [
        ("0", 0x00000000),
        ("-0", 0x80000000),
        ("-0.0", 0x80000000),
        ("-0e0", 0x80000000),
        ("1.0000000596046447", 0x3f800000),
        ("1.0000000596046448", 0x3f800001),
        ("-1.0000000596046447", 0xbf800000),
        ("-1.0000000596046448", 0xbf800001),
        ("16777217.000000001", 0x4b800001),
        ("-16777217.000000001", 0xcb800001),
        ("7.006492321624086e-46", 0x00000001),
        ("-7.006492321624086e-46", 0x80000001),
        ("7.006492321624085e-46", 0x00000000),
        ("-7.006492321624085e-46", 0x80000000),
        ("9223372586610589697", 0x5f000001),
        ("4611686293305294849", 0x5e800001),
        ("-4611686293305294849", 0xde800001),
        ("340282356779733661637539395458142568447.0", 0x7f7fffff),
        ("-340282356779733661637539395458142568447.0", 0xff7fffff),
        ("1e-9999", 0x00000000),
        ("-1e-9999", 0x80000000),
    ] {
        assert_eq!(format.deserialize::<f32>(text.as_bytes()).unwrap().to_bits(), bits, "{text}");
        assert_eq!(text.parse::<f32>().unwrap().to_bits(), bits, "reference for {text}");
        assert_eq!(
            format.deserialize::<f64>(text.as_bytes()).unwrap().to_bits(),
            text.parse::<f64>().unwrap().to_bits(),
            "wide {text}"
        );
    }
    for text in [
        "1e100",
        "-1e100",
        "340282356779733661637539395458142568448.0",
        "340282356779733661637539395458142568449.0",
        "-340282356779733661637539395458142568448.0",
    ] {
        assert!(format.deserialize::<f64>(text.as_bytes()).unwrap().is_finite());
        assert_eq!(format.deserialize::<f32>(text.as_bytes()).unwrap_err().status, NC_E_OUT_OF_RANGE, "{text}");
    }
    macro_rules! integer_zero {
        ($($ty:ty),*) => {$({
            assert_eq!(format.deserialize::<$ty>(b"-0").unwrap(), 0);
            for text in [b"-0.0".as_slice(), b"-0e0", b"1.0", b"1e0"] {
                assert_eq!(format.deserialize::<$ty>(text).unwrap_err().status, NC_E_INVALID_ARG);
            }
        })*};
    }
    integer_zero!(u8, u16, u32, u64, i8, i16, i32, i64);

    // No raw caller input is retained: mutate and drop it before acquiring values.
    let mut input = br#"{"values":[1.0000000596046448,-0],"wide":1e100}"#.to_vec();
    let source = format.deserializer(&input).unwrap();
    input.fill(b'x');
    drop(input);
    let value = Deserializer::new(source.as_com_ref()).deserialize::<Numbers>().unwrap();
    assert_eq!(value.values.iter().map(|v| v.to_bits()).collect::<Vec<_>>(), [0x3f800001, 0x80000000]);
    assert_eq!(value.wide, 1e100);

    // f32 overflow is admitted syntax. Its first typed failure stays immutable
    // across closing/reentry attempts and read-only diagnostic projections.
    let source = format.deserializer(br#"{"values":[1e100],"wide":1}"#).unwrap();
    let details = source.query_ref::<ISerializationDiagnosticsAbi>().unwrap();
    assert!(details.failure_status().is_err());
    let status = Deserializer::new(source.as_com_ref()).deserialize::<Numbers>().unwrap_err();
    assert_eq!(status, NC_E_OUT_OF_RANGE);
    let first = Error::capture(status, &source);
    assert_eq!(first.path, "$.values[0]");
    assert!(first.message.contains("f32"));
    assert_eq!(source.as_com_ref().finish(), Err(status));
    assert_eq!(source.as_com_ref().enter_value(), Err(status));
    assert_eq!(details.failure_status().unwrap(), status);
    assert_eq!(Error::capture(status, &source), first);

    for text in [b"1e9999".as_slice(), b"18446744073709551616", b"-9223372036854775809"] {
        let source = format.deserializer(text).unwrap();
        assert_eq!(
            source.query_ref::<ISerializationDiagnosticsAbi>().unwrap().failure_status().unwrap(),
            NC_E_OUT_OF_RANGE
        );
    }
}
