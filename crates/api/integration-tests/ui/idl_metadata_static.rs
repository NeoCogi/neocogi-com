extern crate self as example_bindings;
mod example { include!(env!("IDL_EXAMPLE_BINDINGS")); include!(concat!(env!("OUT_DIR"), "/example_helpers.rs")); }
use neocogi_com::idl_metadata::{InterfaceIdentity, InterfaceMetadata};
use crate::example::ExampleObjectAbi;

static FIRST: [u8; 32] = [7; 32];
static SECOND: [u8; 32] = [7; 32];
static LAST_BYTE_DIFFERS: [u8; 32] = {
    let mut bytes = [7; 32];
    bytes[31] ^= 1;
    bytes
};

// Generated FFI bindings are separate from the authored, safe metadata operations.
#[forbid(unsafe_code)]
fn main() {
    let first = InterfaceIdentity { abi_signature: &FIRST, ..ExampleObjectAbi::IDL_INTERFACE.identity };
    let second = InterfaceIdentity { abi_signature: &SECOND, ..first };
    let different = InterfaceIdentity { abi_signature: &LAST_BYTE_DIFFERS, ..first };
    assert_eq!(first, second);
    assert_ne!(first, different);
    let metadata = InterfaceMetadata {
        qualified_name: "example.Object",
        ..ExampleObjectAbi::IDL_INTERFACE
    };
    assert_eq!(metadata.qualified_name, "example.Object");
    assert_eq!(metadata.chain.first(), Some(&metadata.identity));
}
