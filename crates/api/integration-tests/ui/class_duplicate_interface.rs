use neocogi_com::{NcStatus, com_object};

extern crate self as fixture_ui_class_duplicate_interface;
neocogi_com_test_contracts::ui_class_duplicate_interface!();

#[com_object(interfaces(IDuplicateClassInterface, IDuplicateClassInterface))]
pub struct DuplicateInterface;

fn main() {}
