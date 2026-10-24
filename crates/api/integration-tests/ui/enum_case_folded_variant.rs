#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{ComValue};

extern crate self as fixture_ui_enum_case_folded_variant;
test_contracts::ui_enum_case_folded_variant!();

// Case folding is irrelevant to positional wire union fields.
fn main() {
    for value in [CaseFoldedVariant::Foo(1), CaseFoldedVariant::FOO(2), CaseFoldedVariant::Empty(3)] {
        let (wire, _storage) = value.encode_value(&mut Default::default()).unwrap();
        // SAFETY: valid inline encoding, with no foreign references.
        assert_eq!(unsafe { CaseFoldedVariant::decode_value(wire, &mut Default::default()) }.unwrap(), value);
    }
}
