/// Generates built-in contracts independently of optional runtime linkage.
fn main() {
    // Contract generation runs on the build machine. Allocation-free facade
    // builds without macros do not compile the optional IDL build dependency.
    #[cfg(feature = "macros")]
    neocogi_idl::cargo::Module::directory("idl", "neocogi/com/events")
        .rust_path("neocogi_com::events")
        .build()
        .expect("failed to generate built-in COM event contracts");
}
