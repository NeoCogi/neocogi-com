//! Platform-neutral dynamic-library handle abstraction.
//!
//! Conditional modules wrap Unix `dlopen`, Windows `LoadLibraryExW`, or a clear
//! unsupported-platform error behind one `DynamicLibrary` API. Loaded plugin
//! handles are pinned for the process lifetime because native COM vtables
//! and type descriptors remain embedded in their code and data segments.
//!
//! Each target implementation owns opening and symbol lookup.

use std::string::String;

use super::PluginLoadErrorKind;

/// Platform wrapper error before it is associated with a plugin path.
pub(super) struct Error {
    /// Loader phase that failed.
    pub(super) kind: PluginLoadErrorKind,
    /// Platform-specific human-readable detail.
    pub(super) message: String,
}

#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub(super) use unix::DynamicLibrary;
#[cfg(not(any(unix, windows)))]
pub(super) use unsupported::DynamicLibrary;
#[cfg(windows)]
pub(super) use windows::DynamicLibrary;
