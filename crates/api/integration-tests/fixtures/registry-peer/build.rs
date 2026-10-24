//! Builds peer-owned bindings, independently of host and provider A.
#[path = "../../../../../../examples/schemas/build.rs"]
mod example_build;
fn main() { example_build::generate("../../../../../../examples/schemas", "nc_registry_peer::contracts"); }
