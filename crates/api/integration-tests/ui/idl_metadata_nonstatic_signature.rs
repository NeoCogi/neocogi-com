#![forbid(unsafe_code)]
use neocogi_com::idl_metadata::InterfaceIdentity;
use neocogi_idl_example_contracts::ExampleObjectAbi;

fn main() {
    let mut signature = *ExampleObjectAbi::IDL_INTERFACE.identity.abi_signature;
    signature[31] ^= 1;
    let _identity = InterfaceIdentity {
        abi_signature: &signature,
        ..ExampleObjectAbi::IDL_INTERFACE.identity
    };
}
