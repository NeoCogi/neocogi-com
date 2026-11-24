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

use super::OpenLibraryError;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod origin;

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

/// Native `RTLD_FIRST` values from the Apple and Solaris-family <dlfcn.h>.
/// These restrict symbol lookup without changing the open's visibility mode.
const RTLD_FIRST: Option<c_int> = if cfg!(target_vendor = "apple") {
    Some(0x100)
} else if cfg!(any(target_os = "solaris", target_os = "illumos")) {
    Some(0x2000)
} else {
    None
};

/// Non-null Unix dynamic-loader handle pinned for the process lifetime.
///
/// COM plugin vtables and static descriptors are embedded in the shared object;
/// dynamic unloading while references or static descriptors exist causes segmentation
/// faults. The handle is process-pinned and deliberately not closed on drop.
pub struct DynamicLibrary {
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
    ///
    /// # Safety
    /// The selected image and its loader initializers must be trusted native code.
    pub unsafe fn open(path: &Path) -> Result<Self, OpenLibraryError> {
        let path =
            CString::new(path.as_os_str().as_bytes()).map_err(|_| OpenLibraryError::OpenLibrary("library path contains an interior NUL byte".to_string()))?;
        // SAFETY: `path` is a NUL-terminated platform path and `RTLD_NOW` asks
        // the dynamic loader to resolve symbols before returning.
        // RTLD_FIRST restricts lookup on this handle where the loader offers it.
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW | RTLD_FIRST.unwrap_or_default()) };
        NonNull::new(handle)
            .map(|handle| Self { handle })
            .ok_or_else(|| OpenLibraryError::OpenLibrary(dl_error()))
    }

    /// Resolves one trusted NUL-free export of the selected module.
    ///
    /// # Safety
    /// Callers must establish the resolved symbol's exact signature before use.
    pub unsafe fn symbol(&self, symbol: &[u8]) -> Result<*mut c_void, String> {
        #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
        if RTLD_FIRST.is_none() {
            return Err("cannot verify selected module exports on this Unix loader".to_string());
        }
        let symbol = CString::new(symbol).map_err(|_| "symbol name contains an interior NUL byte".to_string())?;
        // SAFETY: `self.handle` is live and `symbol` is NUL-terminated.
        let ptr = unsafe { dlsym(self.handle.as_ptr(), symbol.as_ptr()) };
        let address = NonNull::new(ptr).ok_or_else(dl_error)?;
        #[cfg(all(target_os = "linux", target_env = "gnu"))]
        origin::require_local(self, address)?;
        Ok(address.as_ptr())
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
