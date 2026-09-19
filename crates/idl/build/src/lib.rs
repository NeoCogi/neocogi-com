//! Cargo build integration for one-crate-per-module NeoCogi IDL contracts.
//!
//! The pure [`generate_rust`] entrypoint supports tools and tests. [`Module`]
//! adds Cargo-specific source tracking, dependency metadata exchange, and
//! deterministic output ownership for a contract crate's `build.rs`.
//!
//! Concrete message data generates checked borrowed COM posting. Named outputs
//! expose one-shot installation closures; producers implement ordinary COM setters. Application
//! messages remain Rust values in consumer-owned FIFOs. Generated receivers depend
//! only on COM; queues and application scheduling stay in consumer code.

#![deny(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod cargo;
mod generate;
mod source;

pub use cargo::{BuildError, BuildOutput, Module};
pub use generate::{GenerateError, generate_rust};
pub use source::ModuleSources;
