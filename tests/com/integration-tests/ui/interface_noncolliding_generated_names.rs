#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(env!("NC_COM_TEST_CONTRACTS")); }
extern crate self as fixture_ui_interface_noncolliding_generated_names;
test_contracts::ui_interface_noncolliding_generated_names!();

fn main() {}
