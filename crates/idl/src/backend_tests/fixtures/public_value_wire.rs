// Public associated representations remain usable from a separate Rust crate.
use fixture::{Choice, Owned, Record};
use neocogi_com_sdk::{ComValue, ValueBudget, decode_inline, encode_inline};

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
    let mut wire: <Record as ComValue>::AbiRepr = encode_inline(&native).unwrap();
    assert_eq!(
        (wire.first, wire.raw_0, wire.budget, wire.storage, wire.wire, wire.result),
        (1, 5, 43, 57, 61, 79)
    );
    wire.budget = 91;
    // SAFETY: encoding initialized every raw field; only a valid scalar changed.
    let decoded = unsafe { decode_inline::<Record>(&wire) }.unwrap();
    assert_eq!(decoded, Record { budget: 91, ..native });

    for (tag, native) in [Choice::Named { first: true, raw_0: 7, budget: 73 }, Choice::Tuple(native), Choice::Empty]
        .into_iter()
        .enumerate()
    {
        let wire: <Choice as ComValue>::AbiRepr = encode_inline(&native).unwrap();
        assert_eq!(wire.tag, tag as ::core::primitive::i64);
        // SAFETY: encoding initialized exactly the active payload selected by tag.
        let decoded = unsafe { decode_inline::<Choice>(&wire) }.unwrap();
        assert_eq!(decoded, native);
    }

    let owned = Owned {
        first: true,
        raw_0: 11,
        budget: "owned backing".to_owned(),
        storage: vec![native],
    };
    let (repr, keep_alive): (<Owned as ComValue>::AbiRepr, <Owned as ComValue>::KeepAlive) =
        <Owned as ComValue>::encode_value(&owned, &mut ValueBudget::default()).unwrap();
    assert_eq!((repr.first, repr.raw_0), (1, 11));
    // SAFETY: the source text and the temporary array of converted records
    // remain live. The records' booleans require conversion.
    let decoded = unsafe { <Owned as ComValue>::decode_value(&repr, &mut ValueBudget::default()) }.unwrap();
    assert!(decoded.first);
    assert_eq!(decoded.raw_0, 11);
    assert_eq!(decoded.budget, "owned backing");
    assert_eq!(decoded.storage, [native]);
    drop(keep_alive);
}
