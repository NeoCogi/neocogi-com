//! Portable final-assembly linkage; the reusable COM facade owns no implementation.
use std::{env, path::PathBuf};

/// Selects only this package's final native image, before compiling its artifacts.
fn main() {
    for name in [
        "NC_COM_RUNTIME_LINK_DIR",
        "CARGO_FEATURE_HOST",
        "CARGO_FEATURE_MONOLITH",
        "CARGO_FEATURE_DYNAMIC_PLUGIN",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let embedded = env::var_os("CARGO_FEATURE_MONOLITH").is_some();
    let plugin = env::var_os("CARGO_FEATURE_DYNAMIC_PLUGIN").is_some();
    if embedded && plugin {
        panic!("select one final assembly: dynamic-plugin imports shared nc_com; monolith embeds one implementation");
    }
    if embedded || (!plugin && env::var_os("CARGO_FEATURE_HOST").is_none()) {
        return;
    }
    if let Some(directory) = env::var_os("NC_COM_RUNTIME_LINK_DIR").filter(|directory| !directory.is_empty()) {
        let directory = PathBuf::from(directory);
        println!("cargo:rustc-link-search=native={}", directory.display());
        println!("cargo:rustc-link-search=native={}", directory.join("deps").display());
    }
    // Rust's MSVC cdylib import library has an exact .dll.lib name; GNU and Unix
    // linkers use their ordinary platform nc_com import spelling.
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-lib=dylib:+verbatim=nc_com.dll.lib");
    } else {
        println!("cargo:rustc-link-lib=dylib=nc_com");
    }
}
