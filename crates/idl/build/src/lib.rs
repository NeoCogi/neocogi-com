//! Cargo build integration for one-crate-per-module NeoCogi IDL contracts.
//!
//! The pure [`generate_rust`] entrypoint supports tools and tests. [`Module`]
//! adds Cargo-specific source tracking, dependency metadata exchange, and
//! deterministic output ownership for a contract crate's `build.rs`.

#![deny(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod cargo;
mod generate;

pub use cargo::{BuildError, BuildOutput, Module};
pub use generate::{GenerateError, generate_rust};
