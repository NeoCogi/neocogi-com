//! Generates the shared contract separately in each consuming application.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_name = std::env::var("CARGO_PKG_NAME")?.replace('-', "_");
    let modules = neocogi_idl::cargo::SharedSources::new("../schemas")
        .module("tutorial.calculator", format!("{crate_name}::contracts"))
        .build()?;
    println!("cargo::rustc-env=CALCULATOR_BINDINGS={}", modules[0].include.display());
    Ok(())
}
