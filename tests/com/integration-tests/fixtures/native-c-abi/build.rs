fn main() {
    println!("cargo:rerun-if-changed=schemas/nc/tests/native_c_abi.ncidl");
    let source = std::fs::read_to_string("schemas/nc/tests/native_c_abi.ncidl").unwrap();
    let module = neocogi_idl::parse_module(std::path::Path::new("nc/tests/native_c_abi.ncidl"), &source).unwrap();
    let graph = neocogi_idl::resolve_modules(vec![module]).unwrap();
    let bindings = neocogi_idl::generate_rust(&graph, "nc.tests.native_c_abi").unwrap();
    std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("contracts.rs"), bindings).unwrap();

    println!("cargo:rerun-if-changed=native/caller.c");

    cc::Build::new()
        .file("native/caller.c")
        .std("c11")
        .warnings(true)
        .compile("native_c_abi_fixture");
}
