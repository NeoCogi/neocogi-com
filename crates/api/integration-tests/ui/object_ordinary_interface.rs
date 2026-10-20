use neocogi_com::{com_object};

extern crate self as fixture_ui_object_ordinary_interface;
neocogi_com_test_contracts::ui_object_ordinary_interface!();

#[com_object(IObject)]
/// The same object macro supports services and construction providers.
struct Object;

impl IObject for Object {}

fn main() {}
