//! Native entry point with NCB and JSON always linked from the reusable rlib.
//! NCB remains the default format; this shim owns no codec implementation.
use neocogi_com_sdk::{NcPluginDescriptor, NcStatus};

/// Lends the reusable library's immutable plugin descriptor to the native loader.
///
/// # Safety
/// `output` must be null or writable for one descriptor pointer. The caller must
/// keep this library loaded while using the descriptor or any reachable COM code.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_plugin_descriptor(output: *mut *const NcPluginDescriptor) -> NcStatus {
    // SAFETY: this forwards the native entry point's output and library obligations.
    unsafe { neocogi_com_sdk::return_plugin_descriptor(output, neocogi_com_serialization::plugin_descriptor()) }
}

#[cfg(test)]
mod tests {
    /// The shim lends the rlib's exact descriptor and validates nullable output.
    #[test]
    fn discovery_preserves_descriptor_identity() {
        // SAFETY: a null slot is explicitly rejected by the native discovery ABI.
        assert_eq!(unsafe { super::nc_get_plugin_descriptor(core::ptr::null_mut()) }, neocogi_com_sdk::NC_E_POINTER);
        let mut descriptor = core::ptr::null();
        // SAFETY: the local slot is aligned and writable for one descriptor pointer.
        assert_eq!(unsafe { super::nc_get_plugin_descriptor(&mut descriptor) }, neocogi_com_sdk::NC_OK);
        assert!(core::ptr::eq(descriptor, neocogi_com_serialization::plugin_descriptor()));
        // SAFETY: the descriptor graph lives in this final test executable.
        assert_eq!(unsafe { neocogi_com_sdk::validate_plugin_descriptor(descriptor) }, neocogi_com_sdk::NC_OK);
    }
}
