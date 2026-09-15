//! Generates the intrinsic COM event lease and dispatcher contracts from NCIDL.

/// Emits the typed Rust bindings and native ABI description for the event module.
fn main() {
    neocogi_idl_build::Module::directory("idl", "neocogi/com/events")
        .build()
        .expect("failed to build neocogi.com.events");
}
