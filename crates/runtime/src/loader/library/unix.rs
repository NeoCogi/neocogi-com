//! Unix dynamic-library loading through `dlopen` and `dlsym`.
//!
//! Paths and symbols are converted to NUL-terminated byte strings before the
//! platform calls. Successful handles remain pinned for the process lifetime;
//! dropping the wrapper does not unload provider code or static descriptors.

use core::ffi::{c_char, c_int, c_void};
use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::NonNull;
use std::string::{String, ToString};

use super::{Error, PluginLoadErrorKind};

#[link(name = "dl")]
unsafe extern "C" {
    /// Opens a shared object and returns an opaque native handle.
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    /// Resolves one symbol address from a live native handle.
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    /// Returns and clears the calling thread's latest loader error.
    fn dlerror() -> *const c_char;
}

const RTLD_NOW: c_int = 2;

/// Non-null Unix dynamic-loader handle pinned for the process lifetime.
///
/// COM plugin vtables and static descriptors are embedded in the shared object;
/// dynamic unloading while references or static descriptors exist causes segmentation
/// faults. The handle is process-pinned and deliberately not closed on drop.
pub(in crate::loader) struct DynamicLibrary {
    /// Native handle returned by `dlopen`.
    handle: NonNull<c_void>,
}

// SAFETY: Dynamic loader handles are process resources. The loader APIs used
// here are thread-safe on supported Unix targets, and access is immutable after opening.
unsafe impl Send for DynamicLibrary {}
// SAFETY: Symbol lookup does not mutate Rust-visible state in this wrapper.
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    /// Opens a path with immediate symbol resolution.
    pub(in crate::loader) fn open(path: &Path) -> Result<Self, Error> {
        let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| Error {
            kind: PluginLoadErrorKind::OpenLibrary,
            message: "library path contains an interior NUL byte".to_string(),
        })?;
        // SAFETY: `path` is a NUL-terminated platform path and `RTLD_NOW` asks
        // the dynamic loader to resolve symbols before returning.
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW) };
        NonNull::new(handle).map(|handle| Self { handle }).ok_or_else(|| Error {
            kind: PluginLoadErrorKind::OpenLibrary,
            message: dl_error(),
        })
    }

    /// Resolves one trusted NUL-free symbol name from the live module.
    pub(in crate::loader) unsafe fn symbol(&self, symbol: &[u8]) -> Result<*mut c_void, String> {
        let symbol = CString::new(symbol).map_err(|_| "symbol name contains an interior NUL byte".to_string())?;
        // SAFETY: `self.handle` is live and `symbol` is NUL-terminated.
        let ptr = unsafe { dlsym(self.handle.as_ptr(), symbol.as_ptr()) };
        if ptr.is_null() { Err(dl_error()) } else { Ok(ptr) }
    }
}

/// Copies the current thread-local loader error into owned Rust text.
fn dl_error() -> String {
    // SAFETY: `dlerror` returns a thread-local C string pointer or null.
    let error = unsafe { dlerror() };
    if error.is_null() {
        return "unknown dynamic loader error".to_string();
    }
    // SAFETY: A non-null result is a NUL-terminated loader-owned string.
    unsafe { CStr::from_ptr(error) }.to_string_lossy().into_owned()
}
