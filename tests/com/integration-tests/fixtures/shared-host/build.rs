//! Generates the same public contract independently in every consumer.
fn main() {
    let name = std::env::var("CARGO_PKG_NAME").unwrap().replace('-', "_");
    let modules = neocogi_idl::cargo::SharedSources::new("../schemas")
        .module("test.shared_host", format!("{name}::contracts"))
        .build()
        .unwrap();
    println!("cargo::rustc-env=SHARED_HOST_BINDINGS={}", modules[0].include.display());
}
