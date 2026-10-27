//! Generate the copied authoritative example with the extracted IDL compiler.
fn main() {
    let source = std::fs::read_to_string("contract.ncidl").unwrap();
    let module = neocogi_idl::parse_module(
        std::path::Path::new("nc/examples/strings_and_bytes.ncidl"), &source,
    ).unwrap();
    let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
    let rust = neocogi_idl::generate_rust(&graph, "nc.examples.strings_and_bytes").unwrap();
    std::fs::write(
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("example_strings_and_bytes.rs"), rust,
    ).unwrap();
}
