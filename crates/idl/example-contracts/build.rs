//! Generates the example module whenever its IDL source or dependency graph changes.

/// Entrypoint kept intentionally declarative so the shared helper owns all
/// source tracking, identity validation, and generated output paths.
fn main() {
    neocogi_idl_build::Module::new("idl", "neocogi/idl_example.ncidl")
        .build()
        .expect("failed to build neocogi.idl_example");
}
