//! Windows dynamic-library loading through the Kernel32 loader APIs.
//!
//! Explicit filesystem paths are qualified once and encoded as UTF-16 for
//! `LoadLibraryExW`. This open's ordinary imports search only the selected DLL
//! directory and System32, subject to Windows module reuse/redirection. Later
//! native and delay loads retain the trusted provider's own policy. Successful
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

/// Opaque module handle returned by the Windows native loader.
type Hmodule = *mut c_void;
/// Untyped exported procedure address returned by `GetProcAddress`.
type Farproc = *mut c_void;

/// Search the selected DLL directory for this open's import dependencies.
const LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR: u32 = 0x0000_0100;
/// Search the OS directory without adding application, user, CWD or PATH entries.
const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;

#[link(name = "kernel32")]
unsafe extern "system" {
    /// Opens one qualified UTF-16 path with an explicit per-call import search.
    fn LoadLibraryExW(lpLibFileName: *const u16, hFile: *mut c_void, dwFlags: u32) -> Hmodule;
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
    /// Native handle returned by the successful native open.
    handle: NonNull<c_void>,
}

// SAFETY: Windows module handles are process resources. The wrapper performs
// immutable symbol lookup after loading.
unsafe impl Send for DynamicLibrary {}
// SAFETY: Symbol lookup does not mutate Rust-visible state in this wrapper.
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    /// Resolves an explicit filesystem path and opens it without ambient search.
    /// Original caller spelling remains with the outer error/provenance owner;
    /// neither this temporary absolute path nor its UTF-16 buffer is retained.
    pub(in crate::loader) fn open(path: &Path) -> Result<Self, Error> {
        if path.as_os_str().is_empty() {
            return Err(Error {
                kind: PluginLoadErrorKind::OpenLibrary,
                message: "library path is empty".to_string(),
            });
        }
        // Check the original native spelling before resolving it: a terminator
        // must never turn an explicitly selected path into a different image.
        if path.as_os_str().encode_wide().any(|unit| unit == 0) {
            return Err(Error {
                kind: PluginLoadErrorKind::OpenLibrary,
                message: "library path contains an interior NUL code unit".to_string(),
            });
        }
        // Qualification does not access the filesystem or resolve links. The
        // native loader remains the authority for missing/unloadable images.
        let absolute = std::path::absolute(path).map_err(|error| Error {
            kind: PluginLoadErrorKind::OpenLibrary,
            message: format!("cannot resolve library path: {error}"),
        })?;
        let mut wide: Vec<u16> = absolute.as_os_str().encode_wide().collect();
        wide.push(0);
        // SAFETY: wide owns a terminated native path for the entire call, the
        // reserved handle is null, and the flags constrain this open only.
        // No fallback or process-global directory mutation broadens the policy.
        let handle = unsafe {
            LoadLibraryExW(
                wide.as_ptr(),
                core::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
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
