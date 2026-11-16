//! Real native bootstrap failure fixture; all advertised prefixes have backing storage.
use neocogi_com_core::{NcHostApiV3, NcStatus, NC_E_POINTER, NC_E_NOT_FOUND, NC_OK};
/// Complete outer storage with unsupported versions and unvisited null children.
static WRONG: NcHostApiV3 = NcHostApiV3 {
    size: core::mem::size_of::<NcHostApiV3>(),
    runtime_abi_version: u32::MAX,
    core_abi_version: 20,
    runtime: core::ptr::null(),
    debug: core::ptr::null(),
    persistence: core::ptr::null(),
};
/// Returns failure, no graph or an incompatible backed graph, as selected by the child.
///
/// # Safety
/// A nonnull output is aligned and writable for one host pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_host_api_v3(output: *mut *const NcHostApiV3) -> NcStatus {
    if output.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: caller supplies a writable slot; start with a cleared failure output.
    unsafe { *output = core::ptr::null() };
    match std::env::var("NC_TEST_HOST_FAILURE").as_deref() {
        Ok("status") => NC_E_NOT_FOUND,
        Ok("null") => NC_OK,
        _ => {
            // SAFETY: WRONG's complete readable storage lives in this pinned image.
            unsafe { *output = &WRONG };
            NC_OK
        }
    }
}
