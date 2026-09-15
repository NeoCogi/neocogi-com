//! Assembly of runtime ABI v2 operations and shared C callback utilities.
//!
//! Only the version function and four table getters are exported as dynamic
//! symbols. Operation callbacks remain private function pointers inside these
//! immutable tables, keeping the shared-library symbol surface small while
//! still grouping behavior by responsibility. Common helpers validate borrowed
//! UTF-8 paths, implement count/fill copies, and route detailed failures through
//! the process debug messenger registry.

use core::{mem, ptr, slice, str};
use std::path::Path;

use crate::debug;
use neocogi_com_core::{
    NC_COM_CORE_ABI_VERSION, NC_DEBUG_SEVERITY_ERROR, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK, NcDebugApiV2, NcDebugMessageV2,
    NcContractRegistryV2, NcInspectionApiV2, NcPluginRegistryV2, NcRuntimeApiV2, NcStatus, NcUtf8View,
};

mod contract_registry;
mod exports;
mod inspection;
mod plugin_registry;
mod runtime;

use inspection::{inspect_plugin_library_utf8, module_info, plugin_dependency_count, plugin_dependency_ids, plugin_info};
use contract_registry::{module_count as contract_module_count, module_description as contract_module_description, module_ids as contract_module_ids};
use plugin_registry::{loaded_module_count, loaded_module_ids, plugin_count, plugin_ids};
use runtime::{activate_factory_plugin, load_plugin_library_utf8, register_contract_module, register_factory_plugin_description};

/// Immutable runtime-operation table returned by the exported v2 getter.
pub(super) static RUNTIME_API_V2: NcRuntimeApiV2 = NcRuntimeApiV2 {
    size: mem::size_of::<NcRuntimeApiV2>(),
    core_abi_version: NC_COM_CORE_ABI_VERSION,
    register_contract_module,
    register_factory_plugin_description,
    load_plugin_library_utf8,
    activate_factory_plugin,
};

/// Immutable contract-registry table returned by the exported v2 getter.
pub(super) static CONTRACT_REGISTRY_V2: NcContractRegistryV2 = NcContractRegistryV2 {
    size: mem::size_of::<NcContractRegistryV2>(),
    module_count: contract_module_count,
    module_ids: contract_module_ids,
    module_description: contract_module_description,
};

// These values are immutable for the process lifetime. Getters may therefore
// return their addresses without allocation, locking, or ownership transfer.
/// Immutable plugin-registry table returned by the exported v2 getter.
pub(super) static PLUGIN_REGISTRY_V2: NcPluginRegistryV2 = NcPluginRegistryV2 {
    size: mem::size_of::<NcPluginRegistryV2>(),
    plugin_count,
    plugin_ids,
    loaded_module_count,
    loaded_module_ids,
};

/// Immutable inspection table returned by the exported v2 getter.
pub(super) static INSPECTION_API_V2: NcInspectionApiV2 = NcInspectionApiV2 {
    size: mem::size_of::<NcInspectionApiV2>(),
    plugin_info,
    module_info,
    plugin_dependency_count,
    plugin_dependency_ids,
    inspect_plugin_library_utf8,
};

/// Immutable debug-messenger table returned by the exported v2 getter.
pub(super) static DEBUG_API_V2: NcDebugApiV2 = NcDebugApiV2 {
    size: mem::size_of::<NcDebugApiV2>(),
    create_messenger: debug::create_messenger,
    destroy_messenger: debug::destroy_messenger,
};

/// Validates a borrowed ABI UTF-8 path and returns a call-scoped `Path` view.
unsafe fn path_from_utf8<'a>(path: *const u8, path_len: usize) -> Result<&'a Path, NcStatus> {
    // This creates a call-scoped borrowed path view; dynamic loading consumes it
    // synchronously and copies the path into the retained module summary.
    if path_len == 0 {
        return Err(NC_E_INVALID_ARG);
    }
    if path.is_null() {
        return Err(NC_E_POINTER);
    }
    // SAFETY: The caller guarantees `path` is readable for `path_len` bytes.
    let bytes = unsafe { slice::from_raw_parts(path, path_len) };
    if bytes.contains(&0) {
        return Err(NC_E_INVALID_ARG);
    }
    let path = str::from_utf8(bytes).map_err(|_| NC_E_INVALID_ARG)?;
    Ok(Path::new(path))
}

/// Implements the no-partial-write count/fill protocol for a borrowed slice.
unsafe fn copy_slice_to_out<T: Copy>(items: &[T], out: *mut T, capacity: usize, out_len: *mut usize) -> NcStatus {
    // Always publish the current required length first. In particular, callers
    // may pass a null/undersized data pointer to learn that length without any
    // partial output being written.
    if out_len.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: `out_len` is valid for one write by the caller's contract.
    unsafe { *out_len = items.len() };
    if items.len() > capacity {
        return NC_E_INSUFFICIENT_BUFFER;
    }
    if items.is_empty() {
        return NC_OK;
    }
    if out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: The capacity check proves `out` has enough caller-owned slots.
    unsafe { ptr::copy_nonoverlapping(items.as_ptr(), out, items.len()) };
    NC_OK
}

/// Delivers one synchronous error message through the global debug registry.
fn emit_error(message_type: u32, status: NcStatus, text: &str) {
    // Debug dispatch is synchronous, so this borrowed text remains valid for
    // every callback and need not be copied into runtime-owned storage.
    let message = NcDebugMessageV2::new(status, NcUtf8View { data: text.as_ptr(), len: text.len() });
    debug::emit(NC_DEBUG_SEVERITY_ERROR, message_type, &message);
}

#[cfg(test)]
mod tests {
    use super::exports::{
        nc_com_get_contract_registry_v2, nc_com_get_debug_api_v2, nc_com_get_inspection_api_v2, nc_com_get_plugin_registry_v2, nc_com_get_runtime_api_v2,
        nc_com_runtime_abi_version,
    };
    use super::*;
    use core::ffi::c_void;
    use core::sync::atomic::{AtomicI32, Ordering};
    use neocogi_com_core::{
        NC_COM_RUNTIME_ABI_VERSION, NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_TYPE_VALIDATION, NC_E_NOT_FOUND, NcDebugMessengerCreateInfoV2, NcGuid, NcModuleInfoV2,
        NcFactoryPluginDescription,
    };

    unsafe extern "C" fn accept_inspected_description(_plugin: *const NcFactoryPluginDescription, _user_data: *mut c_void) -> NcStatus {
        NC_OK
    }

    unsafe extern "C" fn record_debug_status(_severity: u32, _message_type: u32, message: *const NcDebugMessageV2, user_data: *mut c_void) {
        if message.is_null() || user_data.is_null() {
            return;
        }
        // SAFETY: The test installs pointers to live values for the messenger
        // lifetime, and the runtime guarantees the message for this callback.
        let status = unsafe { (*message).status.code() };
        // SAFETY: `user_data` points to the test's live `AtomicI32`.
        unsafe { &*user_data.cast::<AtomicI32>() }.store(status, Ordering::SeqCst);
    }

    #[test]
    fn exported_runtime_version_is_independent_and_current() {
        assert_eq!(nc_com_runtime_abi_version(), NC_COM_RUNTIME_ABI_VERSION);
    }

    #[test]
    fn getters_return_complete_static_tables() {
        let runtime = unsafe { &*nc_com_get_runtime_api_v2() };
        let contracts = unsafe { &*nc_com_get_contract_registry_v2() };
        let plugins = unsafe { &*nc_com_get_plugin_registry_v2() };
        let inspection = unsafe { &*nc_com_get_inspection_api_v2() };
        let debug = unsafe { &*nc_com_get_debug_api_v2() };

        assert_eq!(runtime.size, mem::size_of::<NcRuntimeApiV2>());
        assert_eq!(runtime.core_abi_version, NC_COM_CORE_ABI_VERSION);
        assert_eq!(contracts.size, mem::size_of::<NcContractRegistryV2>());
        assert_eq!(plugins.size, mem::size_of::<NcPluginRegistryV2>());
        assert_eq!(inspection.size, mem::size_of::<NcInspectionApiV2>());
        assert_eq!(debug.size, mem::size_of::<NcDebugApiV2>());
    }

    #[test]
    fn direct_counts_and_fill_contract_are_separate() {
        let api = unsafe { &*nc_com_get_plugin_registry_v2() };
        let count = unsafe { (api.plugin_count)() };
        let mut out_len = usize::MAX;
        let mut values = Vec::<NcGuid>::with_capacity(count);
        let status = unsafe { (api.plugin_ids)(values.as_mut_ptr(), count, &mut out_len) };
        assert_eq!(status, NC_OK);
        assert_eq!(out_len, count);
    }

    #[test]
    fn fill_reports_required_capacity_without_partial_output() {
        let items = [10_u32, 20_u32];
        let mut out = [99_u32];
        let mut out_len = 0;
        let status = unsafe { copy_slice_to_out(&items, out.as_mut_ptr(), 1, &mut out_len) };
        assert_eq!(status, NC_E_INSUFFICIENT_BUFFER);
        assert_eq!(out_len, 2);
        assert_eq!(out, [99]);
    }

    #[test]
    fn missing_inspection_entities_report_not_found() {
        let mut info = mem::MaybeUninit::<NcModuleInfoV2>::uninit();
        assert_eq!(unsafe { module_info(0, info.as_mut_ptr()) }, NC_E_NOT_FOUND);
    }

    #[test]
    fn load_path_validation_is_platform_independent() {
        assert_eq!(unsafe { path_from_utf8(ptr::null(), 0) }, Err(NC_E_INVALID_ARG));
        assert_eq!(unsafe { path_from_utf8(ptr::null(), 1) }, Err(NC_E_POINTER));

        let interior_nul = b"plugin\0suffix";
        assert_eq!(unsafe { path_from_utf8(interior_nul.as_ptr(), interior_nul.len()) }, Err(NC_E_INVALID_ARG));

        let invalid_utf8 = [0xff];
        assert_eq!(unsafe { path_from_utf8(invalid_utf8.as_ptr(), invalid_utf8.len()) }, Err(NC_E_INVALID_ARG));
    }

    #[test]
    fn load_path_failure_clears_module_output() {
        let mut module_id = 99;
        assert_eq!(unsafe { load_plugin_library_utf8(ptr::null(), 0, &mut module_id) }, NC_E_INVALID_ARG);
        assert_eq!(module_id, 0);
    }

    #[test]
    fn descriptor_inspection_validates_callback_and_path() {
        let inspection = unsafe { &*nc_com_get_inspection_api_v2() };
        assert_eq!(
            unsafe { (inspection.inspect_plugin_library_utf8)(ptr::null(), 0, None, ptr::null_mut()) },
            NC_E_POINTER
        );
        assert_eq!(
            unsafe { (inspection.inspect_plugin_library_utf8)(ptr::null(), 0, Some(accept_inspected_description), ptr::null_mut(),) },
            NC_E_INVALID_ARG
        );
    }

    #[test]
    fn debug_messenger_observes_runtime_validation_failure() {
        let observed = AtomicI32::new(0);
        let create_info = NcDebugMessengerCreateInfoV2 {
            size: mem::size_of::<NcDebugMessengerCreateInfoV2>(),
            severity_mask: NC_DEBUG_SEVERITY_ERROR,
            message_type_mask: NC_DEBUG_TYPE_VALIDATION,
            callback: Some(record_debug_status),
            user_data: ptr::from_ref(&observed).cast_mut().cast(),
        };
        let debug = unsafe { &*nc_com_get_debug_api_v2() };
        let mut handle = 0;
        assert_eq!(unsafe { (debug.create_messenger)(&create_info, &mut handle) }, NC_OK);

        let runtime = unsafe { &*nc_com_get_runtime_api_v2() };
        assert_eq!(unsafe { (runtime.register_contract_module)(ptr::null()) }, NC_E_POINTER);
        assert_eq!(observed.load(Ordering::SeqCst), NC_E_POINTER.code());

        unsafe { (debug.destroy_messenger)(handle) };
    }
}
