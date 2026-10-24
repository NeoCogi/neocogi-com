extern crate self as example_bindings;
mod example { include!(env!("IDL_EXAMPLE_BINDINGS")); include!(concat!(env!("OUT_DIR"), "/example_helpers.rs")); }
use neocogi_com::idl_metadata::InterfaceMetadata;
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
