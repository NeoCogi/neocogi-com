/// Generates built-in contracts independently of optional runtime linkage.
fn main() {
    // Contract generation runs on the build machine. Emitted ABI/codec Rust
    // needs no procedural macro feature or linked runtime in the target.
    neocogi_idl::cargo::Module::directory("idl", "neocogi/com/events")
        .rust_path("neocogi_com::events")
        .build()
        .expect("failed to generate built-in COM event contracts");
}
