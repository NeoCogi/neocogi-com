#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{com_object};

extern crate self as fixture_ui_class_missing_impl;
test_contracts::ui_class_missing_impl!();

#[com_object(interfaces(IMissingImpl))]
pub struct MissingImpl;

fn main() {
    let _object = MissingImpl::from_value(MissingImpl);
}
