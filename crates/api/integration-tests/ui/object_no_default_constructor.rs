use neocogi_com::{com_object};

extern crate self as fixture_ui_object_no_default_constructor;
neocogi_com_test_contracts::ui_object_no_default_constructor!();

#[com_object(IObject)]
/// Even a unit implementation must be passed explicitly to `from_value`.
struct Object;

impl IObject for Object {}

fn main() {
    let _ = Object::create();
}
