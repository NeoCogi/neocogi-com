fn main() {
    println!("cargo:rerun-if-changed=idl/nc/tests/runtime_path.ncidl");
    let source = std::fs::read_to_string("idl/nc/tests/runtime_path.ncidl").unwrap();
    let module = neocogi_idl::parse_module(std::path::Path::new("nc/tests/runtime_path.ncidl"), &source).unwrap();
    let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
    let bindings = neocogi_idl::generate_rust(&graph, "nc.tests.runtime_path").unwrap();
    std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contracts.rs"), bindings).unwrap();

}
