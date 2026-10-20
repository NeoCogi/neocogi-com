//! End-to-end compiler/backend tests, owned by the highest layer they exercise.
//!
//! Keeping these with IDL avoids a backend dev-dependency back to its compiler
//! consumer. The same shipped tests use the backend's actual public entrypoints
//! in checkout, compiler-floor and extracted-archive verification.
mod identifier_hygiene_tests;
mod name_ownership_tests;
mod persistence_codegen;
mod support_resolution_tests;
mod test_consumer;
