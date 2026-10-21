//! This independent final assembly imports the one external native COM runtime.

/// Selects shared linkage explicitly, without adding a runtime Rust dependency.
/// The driver owns the link directory and removes it before executing the tests.
fn main() {
    println!("cargo:rerun-if-env-changed=NC_COM_RUNTIME_LINK_DIR");
    let directory = std::env::var_os("NC_COM_RUNTIME_LINK_DIR")
        .filter(|directory| !directory.is_empty())
        .expect("NC_COM_RUNTIME_LINK_DIR must name the explicit runtime link directory");
    println!("cargo:rustc-link-search=native={}", std::path::PathBuf::from(directory).display());
    // This package has test executables rather than a Rust library that owns a
    // native dependency. Put their explicit import at the final link boundary:
    // Rust 1.85 can otherwise place rustc-link-lib before facade rlibs and lose
    // the dependency to ELF --as-needed before their runtime references appear.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-tests=nc_com.dll.lib");
    } else {
        println!("cargo:rustc-link-arg-tests=-lnc_com");
    }
}
