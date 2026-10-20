use neocogi_com::{com_object, NcStatus};

extern crate self as fixture_ui_class_lifetime;
neocogi_com_test_contracts::ui_class_lifetime!();

#[com_object(interfaces(IRead))]
struct Reader<'a> {
    value: &'a u32,
}

fn main() {}
