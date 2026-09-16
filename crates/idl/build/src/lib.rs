//! Cargo build integration for one-crate-per-module NeoCogi IDL contracts.
//!
//! The pure [`generate_rust`] entrypoint supports tools and tests. [`Module`]
//! adds Cargo-specific source tracking, dependency metadata exchange, and
//! deterministic output ownership for a contract crate's `build.rs`.
//!
//! Event ports generate private, source-specific `EventConnector` operations
//! alongside their public plain and checked subscription helpers. These operations
//! replace registration closures without adding source knowledge to COM event
//! transport. Their Rust support protocol is versioned separately from the NCIDL ABI.

#![deny(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod artifact;
mod cargo;
mod embedded;
mod generate;
mod native;
mod source;

pub use artifact::{ARTIFACT_SCHEMA, ArtifactRecord};
pub use cargo::{BuildError, BuildOutput, Module};
pub use embedded::EmbeddedModule;
pub use generate::{GenerateError, generate_rust};
pub use source::ModuleSources;

/// Rust API protocol required from `neocogi-com-events` by generated event bodies.
/// This is not an NCIDL ABI version: bump it together with the COM transport
/// support crate when compiler-generated adapter requirements or semantics change.
pub const EVENT_BINDING_API_VERSION: u32 = 2;
