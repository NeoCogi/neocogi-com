#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(env!("NC_COM_TEST_CONTRACTS")); }
use neocogi_com_sdk::{ComPtr, NcStatus};

extern crate self as fixture_ui_interface_wrapper_cannot_outlive_owner;
test_contracts::ui_interface_wrapper_cannot_outlive_owner!();

// A method view cannot survive the owner which lends it, including after
// dereferencing through an inherited base view.
fn outlive_owner(owner: ComPtr<IDerivedAbi>) -> &'static IBaseWrapper {
    &owner
}

fn main() {}
