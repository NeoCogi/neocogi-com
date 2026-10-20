fn main() {
    println!("cargo:rerun-if-changed=idl/test/projection.ncidl");
    let source = std::fs::read_to_string("idl/test/projection.ncidl").unwrap();
    let module = neocogi_idl::parse_module(std::path::Path::new("test/projection.ncidl"), &source).unwrap();
    let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
    let bindings = neocogi_idl::generate_rust(&graph, "test.projection").unwrap();
    std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contracts.rs"), bindings).unwrap();

}
