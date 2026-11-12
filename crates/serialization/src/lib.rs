//! Static or dynamic COM serialization plugin with an unconditional NCB default.
//!
//! `json` adds the optional lightweight JSON format. `plugin` supplies the
//! statically linked descriptor; the separate serialization-dylib crate exports
//! the native entry point. Without default features, fixed-buffer NCB remains available.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
#[macro_use]
mod diagnostics;

pub mod ncb;

#[cfg(feature = "plugin")]
mod plugin;
#[cfg(feature = "plugin")]
pub use plugin::{plugin_descriptor, factory};

#[cfg(feature = "json")]
pub mod json;
