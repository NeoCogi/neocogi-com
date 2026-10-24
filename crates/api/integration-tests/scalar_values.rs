//! Direct IDL scalar emission preserves checked live domains and raw integer persistence.
#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
use neocogi_com::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering::SeqCst},
};

extern crate self as fixture_scalar_values;
test_contracts::scalar_values!();

#[com_object(IScalars)]
/// Counts actual generated provider entry separately from caller-side validation.
struct Scalars {
    /// Shared only with the test observer; no state or allocator crosses the ABI.
    calls: Arc<AtomicUsize>,
}
impl IScalars for Scalars {
    fn byte(&self, value: Byte) -> Result<Byte, NcStatus> {
        self.calls.fetch_add(1, SeqCst);
        Ok(value)
    }
    fn flags(&self, value: TestOptions) -> Result<TestOptions, NcStatus> {
        self.calls.fetch_add(1, SeqCst);
        Ok(value)
    }
    fn tiny(&self, value: TinyFlags) -> Result<TinyFlags, NcStatus> {
        self.calls.fetch_add(1, SeqCst);
        Ok(value)
    }
    fn values(&self, value: Values) -> Result<Values, NcStatus> {
        self.calls.fetch_add(1, SeqCst);
        Ok(value)
    }
}

/// Every initialized byte is tested at both safe-client and raw-provider boundaries.
#[test]
fn every_byte_checks_the_enum_domain_and_known_flag_bits_before_dispatch() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = Scalars::from_value(Scalars { calls: calls.clone() }).query::<IScalarsAbi>().unwrap();
    let raw = provider.as_raw();
    let mut expected_calls = 0;
    for value in 0..=u8::MAX {
        let known = value == 0 || value == 255;
        let result = provider.byte(Byte::from_raw(value));
        assert_eq!(result, if known { Ok(Byte::from_raw(value)) } else { Err(NC_E_INVALID_TAG) });
        let mut output = std::mem::MaybeUninit::<u8>::uninit();
        // SAFETY: a live generated provider receives an initialized scalar and writable exact wire slot.
        let status = unsafe { ((*raw.vtable).byte)(raw.object, value, output.as_mut_ptr()) };
        if known {
            assert_eq!(status, NC_OK);
            // SAFETY: the successful generated thunk initialized this exact scalar output.
            assert_eq!(unsafe { output.assume_init() }, value);
            expected_calls += 2;
        } else {
            assert_eq!(status, NC_E_INVALID_TAG);
        }
        let valid_flags = matches!(value, 0 | 1 | 128 | 129);
        let result = provider.tiny(TinyFlags::from_raw(value));
        assert_eq!(result, if valid_flags { Ok(TinyFlags::from_raw(value)) } else { Err(NC_E_INVALID_TAG) });
        let mut output = std::mem::MaybeUninit::<u8>::uninit();
        // SAFETY: every raw byte is initialized; restricted bits are validated by the actual thunk.
        let status = unsafe { ((*raw.vtable).tiny)(raw.object, value, output.as_mut_ptr()) };
        if valid_flags {
            assert_eq!(status, NC_OK);
            // SAFETY: the successful generated thunk initialized this exact scalar output.
            assert_eq!(unsafe { output.assume_init() }, value);
            expected_calls += 2;
        } else {
            assert_eq!(status, NC_E_INVALID_TAG);
        }
        assert_eq!(calls.load(SeqCst), expected_calls);
    }
}

/// Named aliases and every previously exposed typed bit operation retain their meaning.
#[test]
fn typed_combinations_and_dense_names_preserve_their_public_api() {
    let mut options = TestOptions::FIRST | TestOptions::SECOND;
    assert_eq!(options, TestOptions::BOTH);
    assert!(options.contains(TestOptions::FIRST));
    assert!(options.intersects(TestOptions::SECOND));
    assert!(options.is_valid());
    options &= TestOptions::FIRST;
    assert_eq!(options.raw(), 1);
    options ^= TestOptions::BOTH;
    assert_eq!(options, TestOptions::SECOND);
    options |= TestOptions::FIRST;
    assert_eq!((options & TestOptions::SECOND).raw(), 2);
    assert_eq!((options ^ TestOptions::FIRST).raw(), 2);
    assert!(TestOptions::NONE.is_empty());
    assert_eq!(TestOptions::default(), TestOptions::NONE);
    assert!(!TestOptions::from_raw(4).is_valid());
    assert_eq!(Order::COUNT, 2);
    assert_eq!(Order::ALL, [Order::FirstItem, Order::LastItem]);
    assert_eq!(Order::NAMES, ["first_item", "last_item"]);
    assert_eq!(Order::LastItem.canonical_name(), Some("last_item"));
    assert_eq!(Order::from_raw(2).canonical_name(), None);
}

/// Each scalar width persists its exact raw number, including values rejected by live COM.
fn persistent_roundtrip<T: ComSerialize + ComDeserialize + PartialEq + std::fmt::Debug>(value: T, bytes: usize) {
    let encoded = binary::to_vec(&value).unwrap();
    assert_eq!(&encoded[..4], b"NCB\x01");
    assert_eq!(encoded.len(), 4 + bytes, "only the document marker precedes the raw scalar");
    let decoded = binary::from_slice::<T>(&encoded).unwrap();
    assert_eq!(decoded, value);
}

/// Signed extrema, high unsigned bits and unknown values never narrow or acquire a tag protocol.
#[test]
fn fixed_width_storage_and_raw_persistence_cover_every_supported_representation() {
    persistent_roundtrip(Byte::Maximum, 1);
    persistent_roundtrip(Byte::from_raw(42), 1);
    persistent_roundtrip(Short::Maximum, 2);
    persistent_roundtrip(Short::from_raw(42), 2);
    persistent_roundtrip(Word::Maximum, 4);
    persistent_roundtrip(Word::from_raw(42), 4);
    for value in [
        SignedByte::Minimum,
        SignedByte::Negative,
        SignedByte::Zero,
        SignedByte::Maximum,
        SignedByte::from_raw(42),
    ] {
        persistent_roundtrip(value, 1);
    }
    for value in [SignedShort::Minimum, SignedShort::Maximum, SignedShort::from_raw(42)] {
        persistent_roundtrip(value, 2);
    }
    for value in [SignedWord::Minimum, SignedWord::Maximum, SignedWord::from_raw(42)] {
        persistent_roundtrip(value, 4);
    }
    persistent_roundtrip(TinyFlags::from_raw(0xFF), 1);
    persistent_roundtrip(TestOptions::from_raw(0xFFFF), 2);
    persistent_roundtrip(WideFlags::from_raw(0xFFFFFFFF), 4);
    assert_eq!(std::mem::size_of::<Byte>(), 1);
    assert_eq!(std::mem::size_of::<Short>(), 2);
    assert_eq!(std::mem::size_of::<Word>(), 4);
    assert_eq!(std::mem::align_of::<SignedWord>(), std::mem::align_of::<i32>());
    assert_eq!(encode_inline(&Word::Maximum), Ok(u32::MAX));
    assert_eq!(encode_inline(&SignedByte::Minimum), Ok(i8::MIN));
    assert_eq!(encode_inline(&SignedShort::Minimum), Ok(i16::MIN));
    assert_eq!(encode_inline(&SignedWord::Minimum), Ok(i32::MIN));
    assert_eq!(encode_inline(&WideFlags::HIGH), Ok(1 << 31));
    const { assert!(!Byte::BORROW_WIRE && !TestOptions::BORROW_WIRE) };
    assert_eq!(Byte::FIXED_COST, <u8 as ComValue>::FIXED_COST);
    assert!(<Byte as ComValue>::encode_value(&Byte::Zero, &mut ValueBudget::new(0, 0, 1)).is_err());
    assert_eq!(<Byte as ComValue>::encode_value(&Byte::Zero, &mut ValueBudget::new(0, 1, 1)), Ok((0, ())));
}

/// Nested fields and safe flag arguments retain domain checks after maximum-cost admission.
#[test]
fn aggregate_conversion_and_combined_flags_cannot_bypass_validation() {
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = Scalars::from_value(Scalars { calls: calls.clone() }).query::<IScalarsAbi>().unwrap();
    for value in 0..8 {
        assert_eq!(
            provider.flags(TestOptions::from_raw(value)),
            if value < 4 { Ok(TestOptions::from_raw(value)) } else { Err(NC_E_INVALID_TAG) }
        );
    }
    let value = Values {
        byte: Byte::Maximum,
        flags: TestOptions::BOTH,
    };
    assert_eq!(provider.values(value), Ok(value));
    assert_eq!(provider.values(Values { byte: Byte::from_raw(42), ..value }), Err(NC_E_INVALID_TAG));
    let raw = provider.as_raw();
    let mut wire = encode_inline(&value).unwrap();
    wire.flags = 4;
    let mut output = std::mem::MaybeUninit::uninit();
    // SAFETY: generated wire fields are initialized raw integers, including the invalid flag bits.
    assert_eq!(unsafe { ((*raw.vtable).values)(raw.object, wire, output.as_mut_ptr()) }, NC_E_INVALID_TAG);
    assert_eq!(calls.load(SeqCst), 5);
}
