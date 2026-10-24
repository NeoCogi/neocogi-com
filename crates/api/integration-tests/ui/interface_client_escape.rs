#[allow(unused_imports, unused_macros)]
mod test_contracts { include!(concat!(env!("OUT_DIR"), "/test_contracts.rs")); }
use neocogi_com::{ComPtr, NcStatus};

extern crate self as fixture_ui_interface_client_escape;
test_contracts::ui_interface_client_escape!();

// A method view cannot survive the owner which lends it, including after
// dereferencing through an inherited base view.
fn escape(owner: ComPtr<IDerivedAbi>) -> &'static IBaseClient {
    &owner
}

fn main() {}
