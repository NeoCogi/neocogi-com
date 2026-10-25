#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{com_object, NcStatus};

extern crate self as fixture_ui_class_lifetime;
test_contracts::ui_class_lifetime!();

#[com_object(interfaces(IRead))]
struct Reader<'a> {
    value: &'a u32,
}

fn main() {}
