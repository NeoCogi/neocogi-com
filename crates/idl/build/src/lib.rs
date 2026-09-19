//! Cargo build integration for one-crate-per-module NeoCogi IDL contracts.
//!
//! The pure [`generate_rust`] entrypoint supports tools and tests. [`Module`]
//! adds Cargo-specific source tracking, dependency metadata exchange, and
//! deterministic output ownership for a contract crate's `build.rs`.
//!
//! Concrete message data generates checked borrowed COM posting. Named outputs
//! expose PortRef<T>; producer fields bind through the object macro. Application
//! messages remain Rust values in consumer-owned FIFOs. Generated support APIs
//! are versioned independently of NCIDL wire signatures.

#![deny(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod cargo;
mod generate;
mod source;

pub use cargo::{BuildError, BuildOutput, Module};
pub use generate::{GenerateError, generate_rust};
pub use source::ModuleSources;

/// Rust API protocol required from `neocogi-com-events` by generated event bodies.
/// This is not an NCIDL ABI version: bump it together with the COM transport
/// support crate when compiler-generated adapter requirements or semantics change.
pub const EVENT_BINDING_API_VERSION: u32 = 5;
