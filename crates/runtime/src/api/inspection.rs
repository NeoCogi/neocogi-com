//! Complete registered-plugin queries and recoverable unregistered inspection.
//! Callbacks run without holding catalog state; metadata fills use one read guard.

use core::{ffi::c_void, ptr};

use neocogi_com_core::{NC_DEBUG_TYPE_PLUGIN, NC_E_NOT_FOUND, NC_E_POINTER, NC_OK, NcGuid, NcInspectFactoryPluginDescriptionCallbackV1, NcPluginInfoV1, NcStatus};

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
    let Some(plugin) = catalog.plugin(plugin_id) else {
        return NC_E_NOT_FOUND;
    };
    // SAFETY: `out` is valid for one write by the caller's contract.
    unsafe { ptr::write(out, runtime_inspection::plugin_info(plugin)) };
    NC_OK
}

/// Returns a snapshot count of authoritative plugin records.
pub(super) unsafe extern "C" fn plugin_count() -> usize {
    RuntimeState::global().read_catalog().len()
}

/// Fills complete records using exactly the lookup projection, without staging IDs.
pub(super) unsafe extern "C" fn plugin_infos(out: *mut NcPluginInfoV1, capacity: usize, out_len: *mut usize) -> NcStatus {
    let runtime = RuntimeState::global();
    let catalog = runtime.read_catalog();
    // SAFETY: Caller supplies valid nonoverlapping writable outputs. One guard
    // fixes record count and contents for the entire no-partial-write fill.
    unsafe { copy_to_out(catalog.plugins().map(runtime_inspection::plugin_info), out, capacity, out_len) }
}
