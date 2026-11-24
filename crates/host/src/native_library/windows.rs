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

use super::OpenLibraryError;

/// Opaque module handle returned by the Windows native loader.
type Hmodule = *mut c_void;
/// Untyped exported procedure address returned by `GetProcAddress`.
type Farproc = *mut c_void;

/// Search the selected DLL directory for this open's import dependencies.
const LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR: u32 = 0x0000_0100;
/// Search the OS directory without adding application, user, CWD or PATH entries.
const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;

/// Keep the resolved image loaded until process termination.
const GET_MODULE_HANDLE_EX_FLAG_PIN: u32 = 0x0000_0001;
/// Interpret the second argument to GetModuleHandleExW as an address.
const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;

#[link(name = "kernel32")]
unsafe extern "system" {
    /// Opens one qualified UTF-16 path with an explicit per-call import search.
    fn LoadLibraryExW(lpLibFileName: *const u16, hFile: *mut c_void, dwFlags: u32) -> Hmodule;
    /// Resolves one NUL-terminated exported symbol name.
    fn GetProcAddress(hModule: Hmodule, lpProcName: *const c_char) -> Farproc;
    /// Pins the loaded module containing an address when asked to do so.
    fn GetModuleHandleExW(dwFlags: u32, lpModuleName: *const u16, phModule: *mut Hmodule) -> i32;
    /// Returns the calling thread's latest Windows error code.
    fn GetLastError() -> u32;
}

/// Non-null Windows module handle pinned for the process lifetime.
///
/// COM plugin vtables and static descriptors are embedded in the dynamic module;
/// dynamic unloading while references or static descriptors exist causes memory
/// faults. The handle is process-pinned and deliberately not freed on drop.
pub struct DynamicLibrary {
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
    ///
    /// # Safety
    /// The selected image and its loader initializers must be trusted native code.
    pub unsafe fn open(path: &Path) -> Result<Self, OpenLibraryError> {
        if path.as_os_str().is_empty() {
            return Err(OpenLibraryError::OpenLibrary("library path is empty".to_string()));
        }
        // Check the original native spelling before resolving it: a terminator
        // must never turn an explicitly selected path into a different image.
        if path.as_os_str().encode_wide().any(|unit| unit == 0) {
            return Err(OpenLibraryError::OpenLibrary("library path contains an interior NUL code unit".to_string()));
        }
        // Qualification does not access the filesystem or resolve links. The
        // native loader remains the authority for missing/unloadable images.
        let absolute = std::path::absolute(path).map_err(|error| OpenLibraryError::OpenLibrary(format!("cannot resolve library path: {error}")))?;
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
        NonNull::new(handle)
            .map(|handle| Self { handle })
            .ok_or_else(|| OpenLibraryError::OpenLibrary(last_error()))
    }

    /// Resolves one trusted NUL-free export of the selected module and pins
    /// the image supplying its address, which may be an explicit forward target.
    ///
    /// # Safety
    /// Callers must establish the resolved symbol's exact signature before use.
    pub unsafe fn symbol(&self, symbol: &[u8]) -> Result<*mut c_void, String> {
        let symbol = CString::new(symbol).map_err(|_| "symbol name contains an interior NUL byte".to_string())?;
        // SAFETY: `self.handle` is live and `symbol` is NUL-terminated.
        let ptr = unsafe { GetProcAddress(self.handle.as_ptr(), symbol.as_ptr()) };
        if ptr.is_null() {
            return Err(last_error());
        }
        let mut target = core::ptr::null_mut();
        // SAFETY: GetProcAddress supplied a non-null address. FROM_ADDRESS
        // interprets it as an address, and `target` is writable for one handle.
        // PIN keeps an explicit forward target alive independently of the
        // selected DLL's handle and any ordinary import relationships.
        let pinned = unsafe { GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN, ptr.cast(), &mut target) };
        if pinned == 0 {
            return Err(format!("cannot pin the image supplying the export: {}", last_error()));
        }
        Ok(ptr)
    }
}

/// Formats the current thread's Windows loader error code.
fn last_error() -> String {
    // SAFETY: `GetLastError` has no preconditions.
    let code = unsafe { GetLastError() };
    format!("Windows loader error {code}")
}
