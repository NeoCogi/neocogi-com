//! Assembly of runtime ABI v1 operations and shared C callback utilities.
//!
//! Only the version function and three table getters are exported as dynamic
//! symbols. Operation callbacks remain private function pointers inside these
//! immutable tables, keeping the shared-library symbol surface small while
//! still grouping behavior by responsibility. Common helpers validate borrowed
//! UTF-8 paths, implement count/fill copies, and route detailed failures through
//! the process debug messenger registry.

use core::{mem, slice, str};
use std::path::Path;

use crate::debug;
use neocogi_com_core::{
    NC_COM_CORE_ABI_VERSION, NC_DEBUG_SEVERITY_ERROR, NC_E_INSUFFICIENT_BUFFER, NC_E_INVALID_ARG, NC_E_POINTER, NC_OK, NcDebugApiV1, NcDebugMessageV1,
    NcRuntimeApiV1, NcStatus, NcUtf8View,
};

mod exports;
mod inspection;
mod persistence;
mod runtime;
use persistence::PERSISTENCE_API_V1;

use inspection::{inspect_plugin_library_utf8, plugin_count, plugin_info, plugin_infos};
use runtime::{acquire_factory_plugin, load_plugin_library_utf8, register_plugin_descriptor};

/// Immutable runtime-operation table returned by the exported generation-checked getter.
pub(super) static RUNTIME_API_V1: NcRuntimeApiV1 = NcRuntimeApiV1 {
    size: mem::size_of::<NcRuntimeApiV1>(),
    core_abi_version: NC_COM_CORE_ABI_VERSION,

    register_plugin_descriptor,
    load_plugin_library_utf8,
    acquire_factory_plugin,
    plugin_info,
    plugin_count,
    plugin_infos,
    inspect_plugin_library_utf8,
};

/// Immutable debug-messenger table returned by the exported generation-checked getter.
pub(super) static DEBUG_API_V1: NcDebugApiV1 = NcDebugApiV1 {
    size: mem::size_of::<NcDebugApiV1>(),
    create_messenger: debug::create_messenger,
    destroy_messenger: debug::destroy_messenger,
};

/// Validates a borrowed ABI UTF-8 path and returns a call-scoped `Path` view.
unsafe fn path_from_utf8<'a>(path: *const u8, path_len: usize) -> Result<&'a Path, NcStatus> {
    // This creates a call-scoped borrowed path view; dynamic loading consumes it
    // synchronously and copies the path into the registered plugin origin.
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

/// Fills from an exact-size record projection without a parallel ID collection.
///
/// # Safety
///
/// The iterator must report its actual length (internal callers use slice
/// projections). Nonnull outputs must provide writable, nonoverlapping storage
/// for `capacity` elements and one length respectively.
unsafe fn copy_to_out<T: Copy>(items: impl ExactSizeIterator<Item = T>, out: *mut T, capacity: usize, out_len: *mut usize) -> NcStatus {
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
    if items.len() == 0 {
        return NC_OK;
    }
    if out.is_null() {
        return NC_E_POINTER;
    }
    for (index, item) in items.enumerate() {
        // SAFETY: ExactSizeIterator is provided only by internal slice/record
        // projections; its checked length bounds every caller-owned slot.
        unsafe { out.add(index).write(item) };
    }
    NC_OK
}

/// Delivers one synchronous error message through the global debug registry.
fn emit_error(value_type: u32, status: NcStatus, text: &str) {
    // Debug dispatch is synchronous, so this borrowed text remains valid for
    // every callback and need not be copied into runtime-owned storage.
    let message = NcDebugMessageV1::new(status, NcUtf8View { data: text.as_ptr(), len: text.len() });
    debug::emit(NC_DEBUG_SEVERITY_ERROR, value_type, &message);
}

#[cfg(test)]
mod tests {
    use core::ptr;
    use super::exports::{nc_com_get_persistence_api_v1, nc_com_get_debug_api_v1, nc_com_get_runtime_api_v1, nc_com_runtime_abi_version};
    use super::*;
    use core::ffi::c_void;
    use core::sync::atomic::{AtomicI32, Ordering};
    use neocogi_com_core::{NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_TYPE_VALIDATION, NC_E_NOT_FOUND, NcDebugMessengerCreateInfoV1, NcGuid, NcPluginDescriptor};

    unsafe extern "C" fn accept_inspected_descriptor(_plugin: *const NcPluginDescriptor, _user_data: *mut c_void) -> NcStatus {
        NC_OK
    }

    unsafe extern "C" fn record_debug_status(_severity: u32, _value_type: u32, message: *const NcDebugMessageV1, user_data: *mut c_void) {
        if message.is_null() || user_data.is_null() {
            return;
        }
        // SAFETY: The test installs pointers to live values for the messenger
        // lifetime, and the runtime guarantees the message for this callback.
        let status = unsafe { (*message).status.code() };
        // SAFETY: `user_data` points to the test's live `AtomicI32`.
        unsafe { &*user_data.cast::<AtomicI32>() }.store(status, Ordering::SeqCst);
    }

    /// The pre-release contract changes in place within runtime ABI v1.
    #[test]
    fn exported_runtime_version_remains_one() {
        assert_eq!(nc_com_runtime_abi_version(), 1);
    }

    #[test]
    fn getters_return_complete_static_tables() {
        let runtime = unsafe { &*nc_com_get_runtime_api_v1() };
        let debug = unsafe { &*nc_com_get_debug_api_v1() };
        let persistence = unsafe { &*nc_com_get_persistence_api_v1() };

        assert_eq!(persistence.size, mem::size_of::<neocogi_com_core::NcPersistenceApiV1>());
        assert_eq!(persistence.core_abi_version, NC_COM_CORE_ABI_VERSION);
        assert_eq!(runtime.size, mem::size_of::<NcRuntimeApiV1>());
        assert_eq!(runtime.core_abi_version, NC_COM_CORE_ABI_VERSION);
        assert_eq!(debug.size, mem::size_of::<NcDebugApiV1>());
    }

    #[test]
    fn direct_counts_and_fill_contract_are_separate() {
        let api = unsafe { &*nc_com_get_runtime_api_v1() };
        let count = unsafe { (api.plugin_count)() };
        let mut out_len = usize::MAX;
        let mut values = Vec::<neocogi_com_core::NcPluginInfoV1>::with_capacity(count);
        let status = unsafe { (api.plugin_infos)(values.as_mut_ptr(), count, &mut out_len) };
        assert_eq!(status, NC_OK);
        assert_eq!(out_len, count);
    }

    #[test]
    fn fill_reports_required_capacity_without_partial_output() {
        let items = [10_u32, 20_u32];
        let mut out = [99_u32];
        let mut out_len = 0;
        let status = unsafe { copy_to_out(items.into_iter(), out.as_mut_ptr(), 1, &mut out_len) };
        assert_eq!(status, NC_E_INSUFFICIENT_BUFFER);
        assert_eq!(out_len, 2);
        assert_eq!(out, [99]);
    }

    /// Empty enumerations allow a null buffer; nonempty successful fills require
    /// storage. Invalid length pointers cannot change any output slots.
    #[test]
    fn fill_observes_pointer_and_empty_output_boundaries() {
        let mut len = usize::MAX;
        assert_eq!(unsafe { copy_to_out(core::iter::empty::<u32>(), ptr::null_mut(), 0, &mut len) }, NC_OK);
        assert_eq!(len, 0);
        assert_eq!(
            unsafe { copy_to_out([10_u32].into_iter(), ptr::null_mut(), 0, &mut len) },
            NC_E_INSUFFICIENT_BUFFER
        );
        assert_eq!(len, 1);
        assert_eq!(unsafe { copy_to_out([10_u32].into_iter(), ptr::null_mut(), 1, &mut len) }, NC_E_POINTER);
        let mut out = [99_u32; 3];
        assert_eq!(
            unsafe { copy_to_out([10_u32, 20].into_iter(), out.as_mut_ptr(), 3, ptr::null_mut()) },
            NC_E_POINTER
        );
        assert_eq!(out, [99; 3]);
        assert_eq!(unsafe { copy_to_out([10_u32, 20].into_iter(), out.as_mut_ptr(), 3, &mut len) }, NC_OK);
        assert_eq!(len, 2);
        assert_eq!(out, [10, 20, 99]);
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
    fn load_path_failure_clears_plugin_output() {
        let mut plugin_id = NcGuid::from_fields(99, 0, 0, [0; 8]);
        assert_eq!(unsafe { load_plugin_library_utf8(ptr::null(), 0, &mut plugin_id) }, NC_E_INVALID_ARG);
        assert_eq!(plugin_id, NcGuid::nil());
        assert_eq!(unsafe { load_plugin_library_utf8(ptr::null(), 0, ptr::null_mut()) }, NC_E_POINTER);
        plugin_id = NcGuid::from_fields(99, 0, 0, [0; 8]);
        assert_eq!(unsafe { load_plugin_library_utf8(ptr::null(), 1, &mut plugin_id) }, NC_E_POINTER);
        assert_eq!(plugin_id, NcGuid::nil());
    }

    #[test]
    fn descriptor_inspection_validates_callback_and_path() {
        assert_eq!(
            unsafe { (RUNTIME_API_V1.inspect_plugin_library_utf8)(ptr::null(), 0, None, ptr::null_mut()) },
            NC_E_POINTER
        );
        assert_eq!(
            unsafe { (RUNTIME_API_V1.inspect_plugin_library_utf8)(ptr::null(), 0, Some(accept_inspected_descriptor), ptr::null_mut(),) },
            NC_E_INVALID_ARG
        );
    }

    #[test]
    fn debug_messenger_observes_runtime_validation_failure() {
        let observed = AtomicI32::new(0);
        let create_info = NcDebugMessengerCreateInfoV1 {
            size: mem::size_of::<NcDebugMessengerCreateInfoV1>(),
            severity_mask: NC_DEBUG_SEVERITY_ERROR,
            value_type_mask: NC_DEBUG_TYPE_VALIDATION,
            callback: Some(record_debug_status),
            user_data: ptr::from_ref(&observed).cast_mut().cast(),
        };
        let debug = unsafe { &*nc_com_get_debug_api_v1() };
        let mut handle = 0;
        assert_eq!(unsafe { (debug.create_messenger)(&create_info, &mut handle) }, NC_OK);

        let runtime = unsafe { &*nc_com_get_runtime_api_v1() };
        assert_eq!(unsafe { (runtime.register_plugin_descriptor)(ptr::null()) }, NC_E_POINTER);
        assert_eq!(observed.load(Ordering::SeqCst), NC_E_POINTER.code());

        unsafe { (debug.destroy_messenger)(handle) };
    }

    #[test]
    fn complete_plugin_lookup_validates_pointer_and_unknown_id() {
        assert_eq!(unsafe { plugin_info(NcGuid::nil(), ptr::null_mut()) }, NC_E_POINTER);
        let mut info = core::mem::MaybeUninit::<neocogi_com_core::NcPluginInfoV1>::uninit();
        assert_eq!(unsafe { plugin_info(NcGuid::nil(), info.as_mut_ptr()) }, NC_E_NOT_FOUND);
    }
}
