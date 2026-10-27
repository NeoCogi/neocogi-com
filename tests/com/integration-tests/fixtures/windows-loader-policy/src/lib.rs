//! Independent Windows provider with a real ordinary helper DLL import.

use neocogi_com_core::{
    ComRaw, IUnknown, NC_ABI_THREADING_THREAD_LOCAL, NC_COM_CORE_ABI_VERSION, NC_E_CLOSED, NC_E_POINTER, NC_PLUGIN_DESCRIPTOR_ABI_VERSION, NC_OK,
    NcPluginDescriptor, NcStaticUtf8, NcStatus, guid_from_qualified_name,
};

unsafe extern "C" {
    /// Imported from the same-basename genuine or alternative helper mapping.
    fn nc_policy_helper_origin() -> i32;
}

/// This test does not construct an object; successful lookup reaches this code.
unsafe extern "C" fn acquire(_host: *const neocogi_com_core::NcHostApiV1, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The runtime supplies one writable ABI output record.
    unsafe { *out = ComRaw::null() };
    NC_E_CLOSED
}

/// Process-lifetime immutable provider metadata owned by this mapping.
static DESCRIPTION: NcPluginDescriptor = NcPluginDescriptor {
    description_abi_version: NC_PLUGIN_DESCRIPTOR_ABI_VERSION,
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    plugin_id: guid_from_qualified_name("nc.tests.windows_policy"),
    qualified_name: NcStaticUtf8::from_static("nc.tests.windows_policy"),
    plugin_version: NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: core::ptr::null(),
    root_factory: IUnknown::INTERFACE_ID,
    root_factory_threading: NC_ABI_THREADING_THREAD_LOCAL,
    acquire: Some(acquire),
};

/// Supplies the real descriptor discovery ABI used by the native loader.
///
/// # Safety
/// `out` must be null or writable for one descriptor pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nc_get_plugin_descriptor_v1(out: *mut *const NcPluginDescriptor) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    // The retained getter calls the imported function, preventing the linker
    // from discarding the dependency. File I/O here is outside DllMain.
    // SAFETY: the native import has the exact C signature declared above.
    let origin = unsafe { nc_policy_helper_origin() };
    let marker = std::env::var_os("NC_COM_POLICY_GETTER").expect("getter marker path");
    std::fs::write(marker, origin.to_string()).expect("write getter marker");
    // SAFETY: The loader provides one output slot; metadata remains mapped.
    unsafe { *out = &DESCRIPTION };
    NC_OK
}
