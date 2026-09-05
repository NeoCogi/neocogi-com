//! Explicit fallback for targets without an nc-com dynamic-loader implementation.
//!
//! Both opening and symbol resolution return `UnsupportedPlatform` diagnostics,
//! allowing offline inspection to fail normally and required live activation to
//! follow its documented fatal boundary.

use core::ffi::c_void;
use std::path::Path;
use std::string::{String, ToString};

use super::{Error, PluginLoadErrorKind};

/// Uninhabited-in-practice placeholder for unsupported targets.
pub(in crate::loader) struct DynamicLibrary;

impl DynamicLibrary {
    /// Reports that opening dynamic libraries is unsupported on this target.
    pub(in crate::loader) fn open(_path: &Path) -> Result<Self, Error> {
        Err(Error {
            kind: PluginLoadErrorKind::UnsupportedPlatform,
            message: "dynamic loading is not implemented for this target".to_string(),
        })
    }

    /// Reports that symbol lookup is unsupported on this target.
    pub(in crate::loader) unsafe fn symbol(&self, _symbol: &[u8]) -> Result<*mut c_void, String> {
        Err("dynamic loading is not implemented for this target".to_string())
    }
}
