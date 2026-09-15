//! Generates the process-neutral COM event transport contracts from NCIDL.

/// Emits the typed Rust bindings and native ABI description for the event module.
fn main() {
    neocogi_idl_build::Module::directory("idl", "neocogi/com/events")
        .build()
        .expect("failed to build neocogi.com.events");
}
