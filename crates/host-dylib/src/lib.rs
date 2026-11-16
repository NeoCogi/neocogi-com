//! Thin native bootstrap for the reusable host runtime implementation.
//!
//! The application selects this image instead of statically linking a host.
//! All providers receive its existing C tables; the shim owns no second registry.
//! The image must remain mapped for process lifetime, including after main returns.
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

use neocogi_com_core::{NC_E_POINTER, NC_OK, NcHostApiV3, NcStatus};

/// Lends this image's canonical host API after idempotent initialization.
///
/// A null output returns `NC_E_POINTER`. A usable slot receives the immutable
/// host graph and `NC_OK`; this entry invokes no plugin and loads no library.
/// The application must select one runtime owner; this export cannot detect a
/// different host implementation linked or loaded elsewhere in the process.
///
/// # Safety
/// A nonnull `output` is aligned and writable for one host pointer. The caller
/// keeps this runtime image mapped for process lifetime before adopting access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_host_api_v3(output: *mut *const NcHostApiV3) -> NcStatus {
    if output.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: the caller supplies one writable output slot. Initialization lends
    // process-lived statics from the one implementation linked into this image.
    unsafe { *output = neocogi_com_host::initialize_process() };
    NC_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bootstrap rejects null and repeatedly returns the implementation's same graph.
    #[test]
    fn bootstrap_lends_one_valid_canonical_host() {
        // SAFETY: null is rejected before initialization or output access.
        assert_eq!(unsafe { nc_get_host_api_v3(core::ptr::null_mut()) }, NC_E_POINTER);
        let mut first = core::ptr::null();
        let mut second = core::ptr::null();
        // SAFETY: both output slots are writable and the test image remains live.
        unsafe {
            assert_eq!(nc_get_host_api_v3(&mut first), NC_OK);
            assert_eq!(nc_get_host_api_v3(&mut second), NC_OK);
            assert_eq!(neocogi_com_core::validate_host_api(first), NC_OK);
        }
        assert_eq!(first, second);
        assert!(core::ptr::eq(first, neocogi_com_host::initialize_process()));
    }
}
