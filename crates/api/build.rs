/// Generates built-in contracts independently of optional runtime linkage.
fn main() {
    // Contract generation runs on the build machine. Emitted ABI/codec Rust
    // needs no procedural macro feature or linked runtime in the target.
    neocogi_idl::cargo::Module::directory("idl", "neocogi/com/events")
        .rust_path("neocogi_com::events")
        .build()
        .expect("failed to generate built-in COM event contracts");
    for name in ["calculator", "tree", "tagged_objects", "strings_and_bytes", "query_cost"] {
        let relative = format!("nc/examples/{name}.ncidl");
        let path = std::path::Path::new("examples/idl").join(&relative);
        println!("cargo:rerun-if-changed={}", path.display());
        let source = std::fs::read_to_string(&path).expect("example contract source");
        let module = neocogi_idl::parse_module(std::path::Path::new(&relative), &source).expect("parse example contract");
        let logical = module.name.to_string();
        let graph = neocogi_idl::resolve_modules(vec![module]).expect("resolve example contract");
        let rust = neocogi_idl::generate_rust(&graph, &logical).expect("generate example contract");
        std::fs::write(
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join(format!("example_{name}.rs")),
            rust,
        )
        .expect("write example bindings");
    }
}
