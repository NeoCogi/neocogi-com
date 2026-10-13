//! C callbacks for querying plugin metadata and dependencies.
//!
//! Record lookups distinguish missing entities with `NC_E_NOT_FOUND`.
//! Relationship counts instead use zero for either an absent identity or an
//! identity with no edges, matching the count/fill protocol documented by ABI
//! v1. The module also implements recoverable, non-registering inspection of a
//! trusted plugin library and lends the validated factory description to a synchronous
//! caller callback without acquiring registry state.

use core::{ffi::c_void, ptr};

use neocogi_com_core::{
    NC_DEBUG_TYPE_PLUGIN, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcGuid, NcInspectFactoryPluginDescriptionCallbackV1, NcPluginFactoryInfoV1, NcPluginInfoV1,
    NcStatus,
};

use crate::inspection as runtime_inspection;
use crate::loader::inspect_factory_plugin_description;
use crate::state::RuntimeState;

use super::runtime::load_error_status;
use super::{copy_to_out, emit_error, path_from_utf8};

/// Opens and validates a trusted plugin, then invokes a synchronous factory-description callback.
pub(super) unsafe extern "C" fn inspect_plugin_library_utf8(
    path: *const u8,
    path_len: usize,
    callback: Option<NcInspectFactoryPluginDescriptionCallbackV1>,
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
    let plugin = match unsafe { inspect_factory_plugin_description(path) } {
        Ok(plugin) => plugin,
        Err(error) => {
            let status = error.status().unwrap_or_else(|| load_error_status(error.kind()));
            emit_error(NC_DEBUG_TYPE_PLUGIN, status, &error.to_string());
            return status;
        }
    };

    // SAFETY: Validation succeeded while the opened module was pinned. The
    // callback receives exactly the synchronous borrow promised by the ABI.
    unsafe { callback(plugin, user_data) }
}

/// Writes borrowed descriptor metadata for one registered plugin.
pub(super) unsafe extern "C" fn plugin_info(plugin_id: NcGuid, out: *mut NcPluginInfoV1) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    let runtime = RuntimeState::global();
    let catalog = runtime.read_catalog();
    let Some(info) = runtime_inspection::factory_plugin_info(&catalog, plugin_id) else {
        return NC_E_NOT_FOUND;
    };
    // SAFETY: `out` is valid for one write by the caller's contract.
    unsafe { ptr::write(out, info) };
    NC_OK
}

/// Writes factory metadata for one registered plugin.
pub(super) unsafe extern "C" fn plugin_factory_info(plugin_id: NcGuid, out: *mut NcPluginFactoryInfoV1) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    let runtime = RuntimeState::global();
    let catalog = runtime.read_catalog();
    let Some(info) = runtime_inspection::factory_plugin_factory_info(&catalog, plugin_id) else {
        return NC_E_NOT_FOUND;
    };
    // SAFETY: `out` is valid for one write by the caller's contract.
    unsafe { ptr::write(out, info) };
    NC_OK
}

/// Returns the number of hard dependency IDs declared by a registered plugin.
pub(super) unsafe extern "C" fn plugin_dependency_count(plugin_id: NcGuid) -> usize {
    RuntimeState::global().read_catalog().dependency_ids(plugin_id).len()
}

/// Copies a plugin's hard dependency IDs using the count/fill contract.
pub(super) unsafe extern "C" fn plugin_dependency_ids(plugin_id: NcGuid, out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus {
    let runtime = RuntimeState::global();
    let catalog = runtime.read_catalog();
    // SAFETY: The caller provides the output-buffer contract. The guard keeps
    // copied dependency storage live and immutable through the fill.
    unsafe { copy_to_out(catalog.dependency_ids(plugin_id).iter().copied(), out, capacity, out_len) }
}
