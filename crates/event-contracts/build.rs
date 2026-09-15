//! Generates the intrinsic COM event lease and dispatcher contracts from NCIDL.

/// Emits typed Rust bindings and the source graph, leaving native metadata to the runtime.
fn main() {
    neocogi_idl_build::Module::directory("idl", "neocogi/com/events")
        .build_runtime_builtin()
        .expect("failed to build neocogi.com.events");
}
