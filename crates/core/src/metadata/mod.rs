//! Stable logical contracts and C-shaped plugin descriptor metadata.
//!
//! Logical contract types define what contributes to an interface ABI signature;
//! physical descriptors encode those contracts in immutable `repr(C)` records.
//! This module also centralizes text policy, human-readable signature rendering,
//! and whole-graph descriptor validation.

mod contract;
mod descriptor;
mod inheritance;
#[cfg(feature = "alloc")]
mod signature;
mod text;
mod validation;

pub use contract::*;
pub use descriptor::*;
pub use inheritance::*;
#[cfg(feature = "alloc")]
pub use signature::format_method_signature;
pub use text::*;
pub use validation::{NcDescriptorPath, NcDescriptorValidationIssue, NcDescriptorValidationKind, validate_plugin_descriptor_with_issues};
