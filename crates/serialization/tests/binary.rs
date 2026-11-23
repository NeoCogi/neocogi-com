//! The NCB cursor must remain usable by allocation-free contracts.
//! Run this target with --no-default-features as well as with alloc enabled.

use neocogi_com_serialization::ncb::{self as binary, Limits};

/// The fixed-buffer path uses the existing NCB framing, endian rules and budgets.
#[test]
fn fixed_output_roundtrips_with_exact_limits() {
    let value = [0x1234u16, 0xabcd];
    let expected = b"NCB\x01\x02\0\0\0\x34\x12\xcd\xab";
    let limits = Limits {
        bytes: expected.len(),
        elements: 2,
        values: 3,
        depth: 2,
    };
    let mut storage = [0xa5; 16];
    let written = binary::to_slice_with_limits(&value, &mut storage, limits).unwrap();
    assert_eq!(&storage[..written], expected);
    assert_eq!(&storage[written..], &[0xa5; 4]);
    assert_eq!(binary::encoded_len_with_limits(&value, limits), Ok(written));
    assert_eq!(binary::from_slice_with_limits::<[u16; 2]>(&storage[..written], limits), Ok(value));
    #[cfg(feature = "alloc")]
    assert_eq!(binary::to_vec_with_limits(&value, limits).unwrap(), expected);
}

/// A short caller buffer fails before writing outside its supplied bounds.
#[test]
fn insufficient_fixed_capacity_preserves_surrounding_storage() {
    for capacity in 0..12 {
        let mut storage = [0xa5; 16];
        let error = binary::to_slice(&[0x1234u16, 0xabcd], &mut storage[1..1 + capacity]).unwrap_err();
        assert_eq!(error.message, "output buffer too small");
        assert!(error.offset <= capacity);
        assert_eq!(storage[0], 0xa5);
        assert!(storage[1 + capacity..].iter().all(|byte| *byte == 0xa5));
    }
}

/// Storage selection cannot bypass any of the shared resource-policy dimensions.
#[test]
fn fixed_output_and_measurement_report_identical_limit_errors() {
    let value = [0x1234u16, 0xabcd];
    let exact = Limits {
        bytes: 12,
        elements: 2,
        values: 3,
        depth: 2,
    };
    let mut storage = [0; 16];
    let len = binary::to_slice_with_limits(&value, &mut storage, exact).unwrap();
    for limits in [
        Limits { bytes: 11, ..exact },
        Limits { elements: 1, ..exact },
        Limits { values: 2, ..exact },
        Limits { depth: 1, ..exact },
    ] {
        let mut output = [0; 16];
        assert_eq!(
            binary::to_slice_with_limits(&value, &mut output, limits),
            binary::encoded_len_with_limits(&value, limits),
        );
        assert!(binary::from_slice_with_limits::<[u16; 2]>(&storage[..len], limits).is_err());
    }
}

/// Header admission uses the same byte-limit classification as an active session.
#[test]
fn header_byte_limit_is_a_range_failure() {
    for bytes in 0..4 {
        let limits = Limits { bytes, ..Limits::default() };
        let error = binary::encoded_len_with_limits(&(), limits).unwrap_err();
        assert_eq!(
            error,
            binary::Error {
                offset: 0,
                message: "document byte limit"
            }
        );
        assert_eq!(binary::to_slice_with_limits(&(), &mut [0; 4], limits), Err(error));
        #[cfg(feature = "alloc")]
        {
            use neocogi_com_sdk::NC_E_OUT_OF_RANGE;
            let format = binary::format(limits);
            assert_eq!(format.serializer().err(), Some(NC_E_OUT_OF_RANGE));
            let error = format.serialize(&()).unwrap_err();
            assert_eq!(error.status, NC_E_OUT_OF_RANGE);
            assert_eq!(error.byte_offset, None);
        }
    }
    let limits = Limits { bytes: 4, ..Limits::default() };
    let mut output = [0; 5];
    assert_eq!(binary::encoded_len_with_limits(&(), limits), Ok(4));
    assert_eq!(binary::to_slice_with_limits(&(), &mut output, limits), Ok(4));
    assert_eq!(&output[..4], b"NCB\x01");
    let error = binary::Error {
        offset: 4,
        message: "document byte limit",
    };
    assert_eq!(binary::encoded_len_with_limits(&1u8, limits), Err(error));
    assert_eq!(binary::to_slice_with_limits(&1u8, &mut output, limits), Err(error));

    #[cfg(feature = "alloc")]
    {
        let format = binary::format(limits);
        assert_eq!(format.serialize(&()).unwrap(), b"NCB\x01");
        let error = format.serialize(&1u8).unwrap_err();
        assert_eq!(error.status, neocogi_com_sdk::NC_E_OUT_OF_RANGE);
        assert_eq!(error.message, "document byte limit");
        assert_eq!(error.byte_offset, Some(4));
    }
}

/// Leaving a scope admits a sibling at the same depth without refunding values.
#[test]
fn fixed_paths_charge_depth_without_refunding_values() {
    let tree = [[(); 2]; 2];
    let exact = Limits { depth: 3, values: 7, ..Limits::default() };
    let mut output = [0; 32];
    let len = binary::to_slice_with_limits(&tree, &mut output, exact).unwrap();
    assert_eq!(binary::from_slice_with_limits::<[[(); 2]; 2]>(&output[..len], exact), Ok(tree));
    for limits in [Limits { depth: 2, ..exact }, Limits { values: 6, ..exact }] {
        assert!(binary::from_slice_with_limits::<[[(); 2]; 2]>(&output[..len], limits).is_err());
        assert!(binary::encoded_len_with_limits(&tree, limits).is_err());
    }
    let expected = binary::Error {
        offset: 4,
        message: "value or nesting limit",
    };
    for limits in [Limits { depth: 0, ..exact }, Limits { values: 0, ..exact }] {
        assert_eq!(binary::encoded_len_with_limits(&(), limits), Err(expected));
        assert_eq!(binary::from_slice_with_limits::<()>(b"NCB\x01", limits), Err(expected));
    }
}

/// Direct owning calls reach the fixed frame ceiling without recursive Rust calls.
#[cfg(feature = "alloc")]
#[test]
fn owning_ncb_admits_exact_protocol_ceiling_and_rejects_the_next_frame() {
    use neocogi_com_sdk::{NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE};
    let format = binary::format(Limits { depth: 200, ..Limits::default() });
    let mut writer = format.serializer().unwrap();
    let mut sink = writer.as_com_mut();
    for _ in 0..127 {
        sink.begin_option(true).unwrap();
    }
    sink.serialize_unit().unwrap();
    for _ in 0..127 {
        sink.end_option().unwrap();
    }
    sink.finish().unwrap();

    let mut writer = format.serializer().unwrap();
    let mut sink = writer.as_com_mut();
    for _ in 0..128 {
        sink.begin_option(true).unwrap();
    }
    assert_eq!(sink.serialize_unit(), Err(NC_E_OUT_OF_RANGE));
    assert_eq!(sink.finish(), Err(NC_E_OUT_OF_RANGE));

    let mut document = [1; 4 + 128];
    document[..4].copy_from_slice(b"NCB\x01");
    for present in [127_usize, 128] {
        let mut source = format.deserializer(&document[..4 + present]).unwrap();
        let mut view = source.as_com_mut();
        for _ in 0..present {
            view.enter_value().unwrap();
            assert_eq!(view.option_present(), Ok(true));
        }
        if present == 128 {
            assert_eq!(view.enter_value(), Err(NC_E_OUT_OF_RANGE));
            assert_eq!(view.finish(), Err(NC_E_OUT_OF_RANGE));
        } else {
            view.enter_value().unwrap();
            view.deserialize_unit().unwrap();
            for _ in 0..=present {
                view.leave_value().unwrap();
            }
            view.finish().unwrap();
        }
    }

    for (values, expected) in [(1, NC_E_OUT_OF_RANGE), (2, NC_E_INVALID_ARG)] {
        let bounded = binary::format(Limits { values, ..Limits::default() });
        let mut writer = bounded.serializer().unwrap();
        let mut sink = writer.as_com_mut();
        sink.serialize_unit().unwrap();
        assert_eq!(sink.serialize_unit(), Err(expected));

        let mut source = bounded.deserializer(b"NCB\x01").unwrap();
        let mut view = source.as_com_mut();
        view.enter_value().unwrap();
        view.deserialize_unit().unwrap();
        view.leave_value().unwrap();
        assert_eq!(view.enter_value(), Err(expected));
    }
}
