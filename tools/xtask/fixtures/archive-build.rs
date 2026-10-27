//! A normal downstream build script; no source-tree layout assumptions.
fn main() {
    neocogi_idl::cargo::Module::new("idl", "proof/archive.ncidl").build().unwrap();
}
