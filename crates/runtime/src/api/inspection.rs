//! C callbacks for querying relationships and descriptor summaries.
//!
//! Record lookups distinguish missing entities with `NC_E_NOT_FOUND`.
//! Relationship counts instead use zero for either an absent identity or an
//! identity with no edges, matching the count/fill protocol documented by ABI
//! v1. The module also implements recoverable, non-registering inspection of a
//! trusted plugin library and lends the validated descriptor to a synchronous
//! caller callback without acquiring registry state.

use core::{ffi::c_void, ptr};

use neocogi_com_core::{
    NC_DEBUG_TYPE_PLUGIN, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcClassInfoV1, NcGuid,
    NcInspectPluginDescriptorCallbackV1, NcInterfaceId, NcModuleInfoV1, NcPluginInfoV1, NcStatus,
};

use crate::inspection as runtime_inspection;
use crate::loader::{LoadedModuleId, inspect_plugin_descriptor};
use crate::state::RuntimeState;

use super::runtime::load_error_status;
use super::{copy_slice_to_out, emit_error, path_from_utf8};

/// Opens and validates a trusted plugin, then invokes a synchronous descriptor callback.
pub(super) unsafe extern "C" fn inspect_plugin_library_utf8(
    path: *const u8,
    path_len: usize,
    callback: Option<NcInspectPluginDescriptorCallbackV1>,
    user_data: *mut c_void,
) -> NcStatus {
    let Some(callback) = callback else {
        return NC_E_POINTER;
    };
    let path = match unsafe { path_from_utf8(path, path_len) } {
        Ok(path) => path,
        Err(status) => return status,
    };

    // SAFETY: The ABI caller selected this path as trusted plugin code.
    let plugin = match unsafe { inspect_plugin_descriptor(path) } {
        Ok(plugin) => plugin,
        Err(error) => {
            let status = error
                .status()
                .unwrap_or_else(|| load_error_status(error.kind()));
            emit_error(NC_DEBUG_TYPE_PLUGIN, status, &error.to_string());
            return status;
        }
    };

    // SAFETY: Validation succeeded while the opened module was pinned. The
    // callback receives exactly the synchronous borrow promised by the ABI.
    unsafe { callback(plugin, user_data) }
}

/// Writes borrowed descriptor metadata for one registered plugin.
pub(super) unsafe extern "C" fn plugin_info(
    plugin_id: NcGuid,
    out: *mut NcPluginInfoV1,
) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    let catalog = RuntimeState::global().read_catalog();
    let Some(info) = runtime_inspection::plugin_info(&catalog.plugin_registry, plugin_id) else {
        return NC_E_NOT_FOUND;
    };
    // SAFETY: `out` is valid for one write by the caller's contract.
    unsafe { ptr::write(out, info) };
    NC_OK
}

/// Writes path and plugin provenance for one retained module ID.
pub(super) unsafe extern "C" fn module_info(module_id: u64, out: *mut NcModuleInfoV1) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    if module_id == 0 {
        return NC_E_NOT_FOUND;
    }
    let module_id = LoadedModuleId::from_raw(module_id);
    let catalog = RuntimeState::global().read_catalog();
    let Some(info) = runtime_inspection::module_info(&catalog.plugin_registry, module_id) else {
        return NC_E_NOT_FOUND;
    };
    // SAFETY: `out` is valid for one write by the caller's contract.
    unsafe { ptr::write(out, info) };
    NC_OK
}

/// Writes metadata and provenance for one registered class ID.
pub(super) unsafe extern "C" fn class_info(class_id: NcGuid, out: *mut NcClassInfoV1) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    let catalog = RuntimeState::global().read_catalog();
    let Some(info) = runtime_inspection::class_info(&catalog.type_registry, class_id) else {
        return NC_E_NOT_FOUND;
    };
    // SAFETY: `out` is valid for one write by the caller's contract.
    unsafe { ptr::write(out, info) };
    NC_OK
}

/// Returns the number of classes currently registered by a plugin.
pub(super) unsafe extern "C" fn plugin_class_count(plugin_id: NcGuid) -> usize {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_plugin_class_ids(plugin_id, <[NcGuid]>::len)
}

/// Returns the number of hard dependency IDs declared by a registered plugin.
pub(super) unsafe extern "C" fn plugin_dependency_count(plugin_id: NcGuid) -> usize {
    RuntimeState::global()
        .read_catalog()
        .plugin_registry
        .with_plugin_dependency_ids(plugin_id, <[NcGuid]>::len)
}

/// Copies a plugin's hard dependency IDs using the count/fill contract.
pub(super) unsafe extern "C" fn plugin_dependency_ids(
    plugin_id: NcGuid,
    out: *mut NcGuid,
    capacity: usize,
    out_len: *mut usize,
) -> NcStatus {
    RuntimeState::global()
        .read_catalog()
        .plugin_registry
        .with_plugin_dependency_ids(plugin_id, |values| {
            // SAFETY: The caller provides the output-buffer contract.
            unsafe { copy_slice_to_out(values, out, capacity, out_len) }
        })
}

/// Copies class IDs registered by a plugin using the count/fill contract.
pub(super) unsafe extern "C" fn plugin_class_ids(
    plugin_id: NcGuid,
    out: *mut NcGuid,
    capacity: usize,
    out_len: *mut usize,
) -> NcStatus {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_plugin_class_ids(plugin_id, |values| {
            // SAFETY: The caller provides the output-buffer contract.
            unsafe { copy_slice_to_out(values, out, capacity, out_len) }
        })
}

/// Returns the number of canonical interface identities exposed by a class.
pub(super) unsafe extern "C" fn class_interface_count(class_id: NcGuid) -> usize {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_class_interface_ids(class_id, <[NcInterfaceId]>::len)
}

/// Copies a class's interface identities using the count/fill contract.
pub(super) unsafe extern "C" fn class_interface_ids(
    class_id: NcGuid,
    out: *mut NcInterfaceId,
    capacity: usize,
    out_len: *mut usize,
) -> NcStatus {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_class_interface_ids(class_id, |values| {
            // SAFETY: The caller provides the output-buffer contract.
            unsafe { copy_slice_to_out(values, out, capacity, out_len) }
        })
}

/// Returns the number of registered classes implementing an interface identity.
pub(super) unsafe extern "C" fn interface_implementor_class_count(
    interface_id: NcInterfaceId,
) -> usize {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_interface_implementor_class_ids(interface_id, <[NcGuid]>::len)
}

/// Copies class IDs implementing an interface using the count/fill contract.
pub(super) unsafe extern "C" fn interface_implementor_class_ids(
    interface_id: NcInterfaceId,
    out: *mut NcGuid,
    capacity: usize,
    out_len: *mut usize,
) -> NcStatus {
    RuntimeState::global()
        .read_catalog()
        .type_registry
        .with_interface_implementor_class_ids(interface_id, |values| {
            // SAFETY: The caller provides the output-buffer contract.
            unsafe { copy_slice_to_out(values, out, capacity, out_len) }
        })
}
