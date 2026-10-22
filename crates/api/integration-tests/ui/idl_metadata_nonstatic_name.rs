#![forbid(unsafe_code)]
use neocogi_com::idl_metadata::InterfaceMetadata;
use neocogi_idl_example_contracts::ExampleObjectAbi;

fn main() {
    let name = String::from("example.Object");
    let _metadata = InterfaceMetadata {
        qualified_name: &name,
        ..ExampleObjectAbi::IDL_INTERFACE
    };
}
