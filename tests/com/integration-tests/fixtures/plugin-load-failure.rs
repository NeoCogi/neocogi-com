//! Minimal independent C-ABI libraries for fatal loader subprocess tests.
//! Compile with missing_export, rejected_entrypoint, or null_descriptor.

/// Supplies no usable descriptor; metadata is never dereferenced by the loader.
///
/// # Safety
/// The runtime supplies an initialized writable pointer slot for the descriptor.
#[cfg(not(missing_export))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_plugin_descriptor_v1(out: *mut *const core::ffi::c_void) -> i32 {
    // SAFETY: the runtime passes one writable descriptor output slot. Pointer
    // erasure here represents only the C output address, not application data.
    unsafe { out.write(core::ptr::null()) };
    // NcStatus has transparent i32 ABI: -4 is NC_E_INVALID_ARG and 0 is NC_OK.
    // The success case deliberately violates the non-null descriptor contract.
    if cfg!(rejected_entrypoint) { -4 } else { 0 }
}
