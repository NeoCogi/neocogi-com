//! Build-time capture of the target triple embedded in inspection artifacts.

fn main() {
    let target = std::env::var("TARGET").expect("Cargo provides TARGET to build scripts");
    println!("cargo:rustc-env=NC_COM_INSPECT_TARGET={target}");
}
