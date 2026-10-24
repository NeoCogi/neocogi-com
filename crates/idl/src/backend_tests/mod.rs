//! End-to-end compiler/backend tests, owned by the highest layer they exercise.
//!
//! The compiler and its shared `codegen` backend live in this ordinary library.
//! The same shipped tests use actual emission and macro-expansion entrypoints
//! in checkout, compiler-floor and extracted-archive verification. Generated
//! consumer projects supply the facade only during testing; the compiler has no
//! production dependency on the facade or its procedural macros.
mod identifier_hygiene_tests;
mod name_ownership_tests;
mod persistence_codegen;
mod support_resolution_tests;
mod test_consumer;

mod public_name_tests;

mod documentation_tests;

mod host_entry_tests;

mod shared_sources_tests;
