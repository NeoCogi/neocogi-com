use std::{env, path::PathBuf};

/// Optional search path for the shared-mode runtime library.
const RUNTIME_LINK_DIR: &str = "NC_COM_RUNTIME_LINK_DIR";

/// Provides shared-library search paths without building or locating a runtime
/// in static mode. Cargo's ordinary dependency graph links the static runtime.
fn main() {
    println!("cargo:rerun-if-env-changed={RUNTIME_LINK_DIR}");

    if env::var_os("CARGO_FEATURE_RUNTIME").is_none() || env::var_os("CARGO_FEATURE_STATIC_RUNTIME").is_some() {
        return;
    }

    let Some(runtime_dir) = env::var_os(RUNTIME_LINK_DIR).filter(|value| !value.is_empty()) else {
        return;
    };
    let runtime_dir = PathBuf::from(runtime_dir);

    println!("cargo:rustc-link-search=native={}", runtime_dir.display());
    println!("cargo:rustc-link-search=native={}", runtime_dir.join("deps").display());
}
