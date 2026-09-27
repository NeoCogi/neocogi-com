//! Stable logical contracts and C-shaped plugin descriptor metadata.
//!
//! Logical contract types define what contributes to an interface ABI signature;
//! physical descriptors encode those contracts in immutable `repr(C)` records.
//! This module also centralizes text policy, human-readable signature rendering,
//! and canonical text validation.

mod contract;
mod descriptor;
mod inheritance;
mod projection;
mod header;
#[cfg(feature = "alloc")]
mod signature;
mod text;

pub use contract::*;
pub use descriptor::*;
pub use inheritance::*;
pub use projection::*;
pub use header::*;
#[cfg(feature = "alloc")]
pub use signature::{format_abi_signature, format_method_signature};
pub use text::*;
