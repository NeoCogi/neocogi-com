// Public associated representations remain usable from a separate Rust crate.
use fixture::{Choice, Owned, Record};
use neocogi_com::{ComValue, ValueBudget, decode_inline, encode_inline};

/// Public raw members and typed backing remain accessible without helper names.
#[test]
fn public_wires_and_storage_preserve_bound_idl_types() {
    let native = Record {
        first: true,
        raw_0: 5,
        budget: 43,
        storage: 57,
        wire: 61,
        result: 79,
    };
    let mut wire: <Record as ComValue>::Wire = encode_inline(&native).unwrap();
    assert_eq!(
        (wire.first, wire.raw_0, wire.budget, wire.storage, wire.wire, wire.result),
        (1, 5, 43, 57, 61, 79)
    );
    wire.budget = 91;
    // SAFETY: encoding initialized every raw field; only a valid scalar changed.
    let decoded = unsafe { decode_inline::<Record>(wire) }.unwrap();
    assert_eq!(decoded, Record { budget: 91, ..native });

    for (tag, native) in [Choice::Named { first: true, raw_0: 7, budget: 73 }, Choice::Tuple(native), Choice::Empty]
        .into_iter()
        .enumerate()
    {
        let wire: <Choice as ComValue>::Wire = encode_inline(&native).unwrap();
        assert_eq!(wire.tag, tag as ::core::primitive::i64);
        // SAFETY: encoding initialized exactly the active payload selected by tag.
        let decoded = unsafe { decode_inline::<Choice>(wire) }.unwrap();
        assert_eq!(decoded, native);
    }

    let owned = Owned {
        first: true,
        raw_0: 11,
        budget: "owned backing".to_owned(),
        storage: vec![native],
    };
    let (wire, storage): (<Owned as ComValue>::Wire, <Owned as ComValue>::Storage) =
        <Owned as ComValue>::encode_value(&owned, &mut ValueBudget::default()).unwrap();
    assert_eq!((wire.first, wire.raw_0), (1, 11));
    // SAFETY: source text and converted Vec<Record> storage stay alive through decode.
    // Record's bool requires checked conversion, so this nonempty vector uses backing.
    let decoded = unsafe { <Owned as ComValue>::decode_value(wire, &mut ValueBudget::default()) }.unwrap();
    assert!(decoded.first);
    assert_eq!(decoded.raw_0, 11);
    assert_eq!(decoded.budget, "owned backing");
    assert_eq!(decoded.storage, [native]);
    drop(storage);
}
