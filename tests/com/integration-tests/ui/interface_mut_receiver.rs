#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(env!("NC_COM_TEST_CONTRACTS")); }
extern crate self as fixture_ui_interface_mut_receiver;
test_contracts::ui_interface_mut_receiver!();

fn main() {}
