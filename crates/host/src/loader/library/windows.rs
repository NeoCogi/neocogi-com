//! Windows dynamic-library loading through the Kernel32 loader APIs.
//!
//! Rust paths are encoded as NUL-terminated UTF-16 for `LoadLibraryW`; exported
//! symbol names use NUL-terminated bytes for `GetProcAddress`. Successful module
//! handles remain process-pinned; dropping the wrapper does not free the module.

use core::ffi::{c_char, c_void};
use std::ffi::CString;
use std::format;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::NonNull;
use std::string::{String, ToString};
use std::vec::Vec;

use super::{Error, PluginLoadErrorKind};

/// Opaque module handle returned by `LoadLibraryW`.
type Hmodule = *mut c_void;
/// Untyped exported procedure address returned by `GetProcAddress`.
type Farproc = *mut c_void;

#[link(name = "kernel32")]
unsafe extern "system" {
    /// Opens a module identified by a NUL-terminated UTF-16 path.
    fn LoadLibraryW(lpLibFileName: *const u16) -> Hmodule;
    /// Resolves one NUL-terminated exported symbol name.
    fn GetProcAddress(hModule: Hmodule, lpProcName: *const c_char) -> Farproc;
    /// Returns the calling thread's latest Windows error code.
    fn GetLastError() -> u32;
}

/// Non-null Windows module handle pinned for the process lifetime.
///
/// COM plugin vtables and static descriptors are embedded in the dynamic module;
/// dynamic unloading while references or static descriptors exist causes memory
/// faults. The handle is process-pinned and deliberately not freed on drop.
pub(in crate::loader) struct DynamicLibrary {
    /// Native handle returned by `LoadLibraryW`.
    handle: NonNull<c_void>,
}

// SAFETY: Windows module handles are process resources. The wrapper performs
// immutable symbol lookup after loading.
unsafe impl Send for DynamicLibrary {}
// SAFETY: Symbol lookup does not mutate Rust-visible state in this wrapper.
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    /// Opens a Windows library path after UTF-16 encoding.
    pub(in crate::loader) fn open(path: &Path) -> Result<Self, Error> {
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        // Reject before entering the native loader: otherwise LoadLibraryW
        // silently opens the prefix before the first zero, including DLL code.
        if wide.contains(&0) {
            return Err(Error {
                kind: PluginLoadErrorKind::OpenLibrary,
                message: "library path contains an interior NUL code unit".to_string(),
            });
        }
        wide.push(0);
        // SAFETY: `wide` is a NUL-terminated UTF-16 path.
        let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
        NonNull::new(handle).map(|handle| Self { handle }).ok_or_else(|| Error {
            kind: PluginLoadErrorKind::OpenLibrary,
            message: last_error(),
        })
    }

    /// Resolves one trusted NUL-free export name from the live module.
    pub(in crate::loader) unsafe fn symbol(&self, symbol: &[u8]) -> Result<*mut c_void, String> {
        let symbol = CString::new(symbol).map_err(|_| "symbol name contains an interior NUL byte".to_string())?;
        // SAFETY: `self.handle` is live and `symbol` is NUL-terminated.
        let ptr = unsafe { GetProcAddress(self.handle.as_ptr(), symbol.as_ptr()) };
        if ptr.is_null() { Err(last_error()) } else { Ok(ptr) }
    }
}

/// Formats the current thread's Windows loader error code.
fn last_error() -> String {
    // SAFETY: `GetLastError` has no preconditions.
    let code = unsafe { GetLastError() };
    format!("Windows loader error {code}")
}
