//! Platform-neutral dynamic-library handle abstraction.
//!
//! Conditional modules wrap Unix `dlopen`, Windows `LoadLibraryW`, or a clear
//! unsupported-platform error behind one `DynamicLibrary` API. The wrapper owns
//! exactly one native handle and unloads it on drop unless higher loader layers
//! deliberately pin or retain it.
//!
//! Each target implementation owns opening, symbol lookup, and closing. The
//! activation pipeline only sees this RAII handle and classified open errors.

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
