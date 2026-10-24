#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
extern crate self as fixture_ui_interface_noncolliding_generated_names;
test_contracts::ui_interface_noncolliding_generated_names!();

fn main() {}
