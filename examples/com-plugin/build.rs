//! Independently generates this consumer's bindings from shared IDL.
#[path = "../../examples/schemas/build.rs"]
mod example_build;
fn main() {
    example_build::generate("../schemas", "neocogi_example_com_plugin::contracts");
}
