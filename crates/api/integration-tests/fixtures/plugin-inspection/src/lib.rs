//! Metadata-only native fixture: core types only, with no embedded runtime or registry.
mod descriptors;
use neocogi_com_core::{NcPluginDescriptor, NcStatus, NC_E_POINTER, NC_OK};

/// Supplies one immutable case selected when this particular native binary is compiled.
///
/// # Safety
/// A nonnull out pointer must be aligned and writable for one descriptor pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_plugin_descriptor_v1(out: *mut *const NcPluginDescriptor) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    let case = option_env!("NC_COM_INSPECTION_CASE").unwrap_or("0").parse::<usize>().unwrap();
    // SAFETY: the native loader supplies one output slot. The selected case and
    // every referenced field are immutable and owned by this pinned mapping.
    unsafe { *out = &descriptors::CASES[case] };
    NC_OK
}
