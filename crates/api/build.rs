#[path = "build/test_fixtures.rs"]
mod test_fixtures;
/// Generates built-in contracts independently of optional runtime linkage.
fn main() {
    // Contract generation runs on the build machine. Emitted ABI/codec Rust
    // needs no procedural macro feature or linked runtime in the target.
    for name in ["calculator", "tree", "tagged_objects", "strings_and_bytes", "query_cost"] {
        let relative = format!("nc/examples/{name}.ncidl");
        let path = std::path::Path::new("examples/schemas").join(&relative);
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
    let shared = std::path::Path::new("../../schemas");
    if std::path::Path::new("integration-tests").is_dir() {
        test_fixtures::generate(shared);
        for module in neocogi_idl::cargo::SharedSources::new("../../examples/schemas")
            .module("neocogi.idl_example", "example_bindings::example")
            .build()
            .expect("example test bindings")
        {
            println!("cargo::rustc-env=IDL_EXAMPLE_BINDINGS={}", module.include.display());
        }
    }
}
