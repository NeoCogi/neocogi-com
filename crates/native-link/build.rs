//! Supplies Cargo's build/run search directory; runtime lookup remains app-relative.
#[path = "../../../tools/native-layout/src/profile.rs"]
mod profile;

/// Cargo uses this native search directory for initial test/executable loading.
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=NEOCOGI_NATIVE_OUTPUT_DIR");
    let output = std::env::var_os("OUT_DIR").expect("Cargo supplies OUT_DIR");
    let root = profile::output_root(std::path::Path::new(&output), std::env::var_os("NEOCOGI_NATIVE_OUTPUT_DIR").map(Into::into))
        .expect("native Cargo output directory");
    println!("cargo::rustc-link-search=native={}", root.join("lib").display());
    // Native import libraries are compilation inputs and stay outside app/lib.
    println!("cargo::rustc-link-search=native={}", root.join("lib-imports").display());
}
