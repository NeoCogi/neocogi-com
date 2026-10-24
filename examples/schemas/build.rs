//! Consumer-owned example generation, sharing only authoritative source files.
/// Emits the example protocol at this consumer's actual Rust location.
pub fn generate(root: &str, rust_path: &str) {
    for module in neocogi_idl::cargo::SharedSources::new(root)
        .module("neocogi.idl_example", rust_path)
        .build()
        .expect("generate example bindings")
    {
        println!("cargo::rustc-env=IDL_EXAMPLE_BINDINGS={}", module.include.display());
    }
}
