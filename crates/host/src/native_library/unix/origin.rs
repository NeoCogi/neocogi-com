//! Confirms local definitions when glibc `dlsym` searches native dependencies.

use core::{
    ffi::{c_char, c_int, c_void},
    mem::MaybeUninit,
    ptr::{self, NonNull},
};
use std::string::{String, ToString};

use super::DynamicLibrary;

/// The public `Dl_info` record filled by the native address lookup.
#[repr(C)]
struct DlInfo {
    /// Loader spelling of the image containing the address.
    filename: *const c_char,
    /// Base address of that image.
    base: *mut c_void,
    /// Name of the nearest symbol, when available.
    symbol: *const c_char,
    /// Address of that symbol, when available.
    address: *mut c_void,
}

/// Uses glibc's `RTLD_DI_LINKMAP` and `RTLD_DL_LINKMAP` operations to compare
/// opaque map identities, independently of the path used to open either image.
pub(super) fn require_local(library: &DynamicLibrary, address: NonNull<c_void>) -> Result<(), String> {
    /// Requests the `link_map` associated with a live `dlopen` handle.
    const RTLD_DI_LINKMAP: c_int = 2;
    /// Requests the `link_map` of the image containing an address.
    const RTLD_DL_LINKMAP: c_int = 2;

    #[link(name = "dl")]
    unsafe extern "C" {
        /// Queries the selected image by native handle.
        fn dlinfo(handle: *mut c_void, request: c_int, info: *mut c_void) -> c_int;
        /// Queries the image containing a resolved address, including its link map.
        fn dladdr1(address: *const c_void, info: *mut DlInfo, extra_info: *mut *mut c_void, flags: c_int) -> c_int;
    }

    let mut selected = ptr::null_mut::<c_void>();
    // SAFETY: the open pins the handle, and this request writes one map pointer.
    if unsafe { dlinfo(library.handle.as_ptr(), RTLD_DI_LINKMAP, ptr::from_mut(&mut selected).cast()) } != 0 || selected.is_null() {
        return Err("cannot identify the selected native module".to_string());
    }
    let mut info = MaybeUninit::uninit();
    let mut owner = ptr::null_mut();
    // SAFETY: the symbol came from the loader. This request fills a Dl_info
    // and one map pointer, which is compared without dereferencing either map.
    if unsafe { dladdr1(address.as_ptr(), info.as_mut_ptr(), &mut owner, RTLD_DL_LINKMAP) } == 0 || owner.is_null() {
        return Err("cannot identify the native module containing the symbol".to_string());
    }
    if selected != owner {
        return Err("resolved symbol belongs to a different native module".to_string());
    }
    Ok(())
}
