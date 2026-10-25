//! Generates this final test owner's local fixture bindings.
#[path = "../../../build/test_fixtures.rs"]
mod test_fixtures;
#[path = "../../../../../examples/schemas/build.rs"]
mod example_build;
fn main() { test_fixtures::generate(std::path::Path::new("../../../../../schemas")); example_build::generate("../../../../../examples/schemas", "example_bindings::example"); }
