//! Shared support for cross-platform integration-test dynamic libraries.

use neocogi_com::{NC_GET_PLUGIN_DESCRIPTOR_SYMBOL, NcGetPluginDescriptor};
use std::path::Path;

pub(crate) struct Library {
    handle: platform::Handle,
}

impl Library {
    pub(crate) fn open(path: &Path) -> Self {
        let handle = platform::open(path).unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display()));
        Self { handle }
    }

    pub(crate) fn get_plugin_descriptor(&self) -> NcGetPluginDescriptor {
        let pointer =
            platform::symbol(self.handle, NC_GET_PLUGIN_DESCRIPTOR_SYMBOL).unwrap_or_else(|error| panic!("failed to resolve plugin descriptor: {error}"));
        // SAFETY: The symbol name and ABI-v1 contract require this exact
        // function-pointer type, and the library remains loaded through self.
        unsafe { core::mem::transmute::<*mut core::ffi::c_void, NcGetPluginDescriptor>(pointer) }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        platform::close(self.handle);
    }
}

#[cfg(unix)]
mod platform {
    use std::ffi::{CStr, CString, c_char, c_int, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub(super) type Handle = *mut c_void;

    const RTLD_NOW: c_int = 2;

    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: c_int) -> Handle;
        fn dlsym(handle: Handle, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: Handle) -> c_int;
        fn dlerror() -> *const c_char;
    }

    pub(super) fn open(path: &Path) -> Result<Handle, String> {
        let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| "path contains NUL")?;
        // SAFETY: The path is a live NUL-terminated C string for this call.
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW) };
        if handle.is_null() { Err(last_error()) } else { Ok(handle) }
    }

    pub(super) fn symbol(handle: Handle, name: &str) -> Result<*mut c_void, String> {
        let name = CString::new(name).map_err(|_| "symbol contains NUL")?;
        // Clear an earlier loader error before distinguishing a null symbol.
        // SAFETY: `dlerror` has no preconditions.
        let _ = unsafe { dlerror() };
        // SAFETY: `handle` is live and `name` is NUL-terminated for this call.
        let pointer = unsafe { dlsym(handle, name.as_ptr()) };
        if pointer.is_null() { Err(last_error()) } else { Ok(pointer) }
    }

    pub(super) fn close(handle: Handle) {
        // SAFETY: Library owns one live handle and closes it exactly once.
        let status = unsafe { dlclose(handle) };
        assert_eq!(status, 0, "dlclose failed: {}", last_error());
    }

    fn last_error() -> String {
        // SAFETY: A non-null result points to loader-owned NUL-terminated text.
        let error = unsafe { dlerror() };
        if error.is_null() {
            "unknown dynamic-loader error".to_string()
        } else {
            // SAFETY: Established above.
            unsafe { CStr::from_ptr(error) }.to_string_lossy().into_owned()
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::{CString, c_char, c_void};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    pub(super) type Handle = *mut c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(path: *const u16) -> Handle;
        fn GetProcAddress(module: Handle, name: *const c_char) -> *mut c_void;
        fn FreeLibrary(module: Handle) -> i32;
        fn GetLastError() -> u32;
    }

    pub(super) fn open(path: &Path) -> Result<Handle, String> {
        let path: Vec<_> = path.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: The path is a live NUL-terminated UTF-16 string for this call.
        let handle = unsafe { LoadLibraryW(path.as_ptr()) };
        if handle.is_null() { Err(last_error()) } else { Ok(handle) }
    }

    pub(super) fn symbol(handle: Handle, name: &str) -> Result<*mut c_void, String> {
        let name = CString::new(name).map_err(|_| "symbol contains NUL")?;
        // SAFETY: `handle` is live and `name` is NUL-terminated for this call.
        let pointer = unsafe { GetProcAddress(handle, name.as_ptr()) };
        if pointer.is_null() { Err(last_error()) } else { Ok(pointer) }
    }

    pub(super) fn close(handle: Handle) {
        // SAFETY: Library owns one live handle and closes it exactly once.
        let status = unsafe { FreeLibrary(handle) };
        assert_ne!(status, 0, "FreeLibrary failed: {}", last_error());
    }

    fn last_error() -> String {
        // SAFETY: `GetLastError` has no preconditions.
        format!("Windows loader error {}", unsafe { GetLastError() })
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use std::ffi::c_void;
    use std::path::Path;

    pub(super) type Handle = *mut c_void;

    pub(super) fn open(_path: &Path) -> Result<Handle, String> {
        Err("dynamic-library integration tests require Unix or Windows".to_string())
    }

    pub(super) fn symbol(_handle: Handle, _name: &str) -> Result<*mut c_void, String> {
        Err("dynamic-library integration tests require Unix or Windows".to_string())
    }

    pub(super) fn close(_handle: Handle) {}
}
