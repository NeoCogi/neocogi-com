extern crate self as example_bindings;
mod example { include!(env!("IDL_EXAMPLE_BINDINGS")); include!(concat!(env!("OUT_DIR"), "/example_helpers.rs")); }
use neocogi_com::idl_metadata::InterfaceIdentity;
use crate::example::ExampleObjectAbi;

// Generated FFI bindings are separate from the authored, safe metadata operations.
#[forbid(unsafe_code)]
fn main() {
    let mut signature = *ExampleObjectAbi::IDL_INTERFACE.identity.abi_signature;
    signature[31] ^= 1;
    let _identity = InterfaceIdentity {
        abi_signature: &signature,
        ..ExampleObjectAbi::IDL_INTERFACE.identity
    };
}
