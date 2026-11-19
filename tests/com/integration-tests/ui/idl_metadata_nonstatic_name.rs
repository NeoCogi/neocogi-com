extern crate self as example_bindings;
pub mod example { include!(env!("IDL_EXAMPLE_BINDINGS")); include!(env!("NC_COM_TEST_EXAMPLE_HELPERS")); }
use neocogi_com_sdk::idl_metadata::InterfaceMetadata;
use crate::example::ExampleObjectAbi;

// Generated FFI bindings are separate from the authored, safe metadata operations.
#[forbid(unsafe_code)]
fn main() {
    let name = String::from("example.Object");
    let _metadata = InterfaceMetadata {
        qualified_name: &name,
        ..ExampleObjectAbi::IDL_INTERFACE
    };
}
