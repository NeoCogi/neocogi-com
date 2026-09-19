//! Generates the COM dispatcher contract from its NCIDL declaration.

/// Emits typed Rust bindings and the source graph, without a separate native artifact.
fn main() {
    neocogi_idl_build::Module::directory("idl", "neocogi/com/events")
        .build()
        .expect("failed to build neocogi.com.events");
}
