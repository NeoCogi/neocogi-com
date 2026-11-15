//! Platform-neutral dynamic-library handle abstraction.
//!
//! Conditional modules wrap Unix `dlopen`, Windows `LoadLibraryExW`, or a clear
//! unsupported-platform error behind one `DynamicLibrary` API. Loaded plugin
//! handles are pinned for the process lifetime because native COM vtables
//! and type descriptors remain embedded in their code and data segments.
//!
//! Each target implementation owns opening and symbol lookup.

use std::string::String;

/// Native open failure before a caller adds its host/plugin context.
#[derive(Debug)]
pub enum OpenLibraryError {
    /// This target has no native loader implementation.
    UnsupportedPlatform,
    /// The platform loader rejected the selected path.
    OpenLibrary(String),
}

#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::DynamicLibrary;
#[cfg(not(any(unix, windows)))]
pub use unsupported::DynamicLibrary;
#[cfg(windows)]
pub use windows::DynamicLibrary;
