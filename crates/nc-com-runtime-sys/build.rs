use std::{env, path::PathBuf};

const RUNTIME_LINK_DIR: &str = "NC_COM_RUNTIME_LINK_DIR";

fn main() {
    println!("cargo:rerun-if-env-changed={RUNTIME_LINK_DIR}");

    let Some(runtime_dir) = env::var_os(RUNTIME_LINK_DIR).filter(|value| !value.is_empty()) else {
        return;
    };
    let runtime_dir = PathBuf::from(runtime_dir);

    println!("cargo:rustc-link-search=native={}", runtime_dir.display());
    println!(
        "cargo:rustc-link-search=native={}",
        runtime_dir.join("deps").display()
    );
}
