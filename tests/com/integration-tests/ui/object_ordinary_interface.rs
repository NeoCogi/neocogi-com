#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{com_object};

extern crate self as fixture_ui_object_ordinary_interface;
test_contracts::ui_object_ordinary_interface!();

#[com_object(IObject)]
/// The same object macro supports services and construction providers.
struct Object;

impl IObject for Object {}

fn main() {}
