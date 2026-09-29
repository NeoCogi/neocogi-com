use neocogi_com::{com_enum, ComValue};

#[derive(Debug, PartialEq)]
#[com_enum(tag = u8, assigned_signature = b"77777777777777777777777777777777")]
enum CaseFoldedVariant {
    Foo(u8),
    FOO(u16),
    _EMPTY(u32),
}

// Case folding is irrelevant to positional wire union fields.
fn main() {
    for value in [CaseFoldedVariant::Foo(1), CaseFoldedVariant::FOO(2), CaseFoldedVariant::_EMPTY(3)] {
        let (wire, _storage) = value.encode_value(&mut Default::default()).unwrap();
        // SAFETY: valid inline encoding, with no foreign references.
        assert_eq!(unsafe { CaseFoldedVariant::decode_value(wire, &mut Default::default()) }.unwrap(), value);
    }
}
