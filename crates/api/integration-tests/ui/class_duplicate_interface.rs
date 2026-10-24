#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{NcStatus, com_object};

extern crate self as fixture_ui_class_duplicate_interface;
test_contracts::ui_class_duplicate_interface!();

#[com_object(interfaces(IDuplicateClassInterface, IDuplicateClassInterface))]
pub struct DuplicateInterface;

fn main() {}
