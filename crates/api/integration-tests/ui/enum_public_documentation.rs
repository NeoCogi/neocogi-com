//! Fully exported enum wire types work in a crate that requires complete public documentation.
#![deny(missing_docs)]
#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{ComValue};

extern crate self as fixture_ui_enum_public_documentation;
test_contracts::ui_enum_public_documentation!();

fn main() {
    let value = Documented::Named { weight: 2.0 };
    let (wire, _storage) = value.encode_value(&mut Default::default()).unwrap();
    // SAFETY: the checked encoder produced this fully initialized inline wire.
    let decoded = unsafe { Documented::decode_value(wire, &mut Default::default()) }.unwrap();
    assert!(matches!(decoded, Documented::Named { weight: 2.0 }));
}
