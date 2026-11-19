//! Explicit fallback for targets without an neocogi-com dynamic-loader implementation.
//!
//! Both opening and symbol resolution return `UnsupportedPlatform` diagnostics,
//! allowing offline inspection to fail normally and required live loading to
//! follow its documented fatal boundary.

use core::ffi::c_void;
use std::path::Path;
use std::string::{String, ToString};

use super::OpenLibraryError;

/// Uninhabited-in-practice placeholder for unsupported targets.
pub struct DynamicLibrary;

impl DynamicLibrary {
    /// Reports that opening dynamic libraries is unsupported on this target.
    ///
    /// # Safety
    /// The selected image and its loader initializers must be trusted native code.
    pub unsafe fn open(_path: &Path) -> Result<Self, OpenLibraryError> {
        Err(OpenLibraryError::UnsupportedPlatform)
    }

    /// Reports that symbol lookup is unsupported on this target.
    ///
    /// # Safety
    /// Callers must establish the resolved symbol's exact signature before use.
    pub unsafe fn symbol(&self, _symbol: &[u8]) -> Result<*mut c_void, String> {
        Err("dynamic loading is not implemented for this target".to_string())
    }
}
