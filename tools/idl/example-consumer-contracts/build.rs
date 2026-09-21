//! Resolves a contract dependency through Cargo-published IDL graph metadata.

/// Builds the dependent example without locating another crate's source tree.
fn main() {
    neocogi_idl::cargo::Module::new("idl", "neocogi/idl_consumer.ncidl")
        .build()
        .expect("failed to build neocogi.idl_consumer");
}
