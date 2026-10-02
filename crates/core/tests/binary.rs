//! The NCB cursor must remain usable by allocation-free contracts.
//! Run this target with --no-default-features as well as with alloc enabled.

use neocogi_com_core::binary::{self, Limits};

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
