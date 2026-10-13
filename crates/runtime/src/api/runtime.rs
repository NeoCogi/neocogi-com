//! C callbacks for runtime registration, loading, and factory activation.
//!
//! These adapters normalize raw-pointer validation, status mapping, and debug
//! emission before delegating to the Rust implementation. They never expose
//! Rust errors, paths, or owned containers across the ABI boundary. Live plugin
//! activation is fail-fatal after detailed diagnostic emission, whereas direct
//! registration and factory activation return ordinary statuses.

use neocogi_com_core::{
    IUnknown, NC_DEBUG_TYPE_PLUGIN, NC_DEBUG_TYPE_RUNTIME, NC_DEBUG_TYPE_VALIDATION, NC_E_INVALID_ARG, NC_E_LOAD_FAILED, NC_E_POINTER, NC_OK,
    NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcStatus,
};

use crate::factory_activation;
use crate::loader::PluginLoadErrorKind;
use crate::state::RuntimeState;

use super::{emit_error, path_from_utf8};

/// Validates and atomically registers one factory-only plugin description.
pub(super) unsafe extern "C" fn register_factory_plugin_description(plugin: *const NcFactoryPluginDescription) -> NcStatus {
    let runtime = RuntimeState::global();
    // SAFETY: The caller supplies the plugin and referenced metadata lifetime
    // required by process-wide registration.
    let status = unsafe { runtime.register_factory_plugin_description(plugin) };
    if status.is_err() {
        emit_error(NC_DEBUG_TYPE_VALIDATION, status, "factory plugin registration failed");
    }
    status
}

/// Performs required live activation and aborts after any load/registration failure.
pub(super) unsafe extern "C" fn load_plugin_library_utf8(path: *const u8, path_len: usize, out_plugin_id: *mut NcGuid) -> NcStatus {
    if out_plugin_id.is_null() {
        return NC_E_POINTER;
    }
    // Clearing first guarantees that every failure path leaves no stale handle
    // for a C caller to mistake for a successful load.
    // SAFETY: `out_plugin_id` is valid for one write by the caller's contract.
    unsafe { *out_plugin_id = NcGuid::nil() };
    let path = match unsafe { path_from_utf8(path, path_len) } {
        Ok(path) => path,
        Err(status) => return status,
    };
    let runtime = RuntimeState::global();

    // SAFETY: Loading executes trusted code selected by the caller.
    match unsafe { runtime.load_plugin_library(path) } {
        Ok(plugin_id) => {
            // SAFETY: `out_plugin_id` was checked above.
            unsafe { *out_plugin_id = plugin_id };
            NC_OK
        }
        Err(error) => {
            let status = error.status().unwrap_or_else(|| load_error_status(error.kind()));
            eprintln!("fatal neocogi-com plugin activation failure: {error}");
            emit_error(NC_DEBUG_TYPE_PLUGIN, status, &error.to_string());
            // Required live activation is fail-fatal. An opened library has
            // already been pinned by the loader, so aborting here cannot race
            // an unload of code reached by loader hooks or the entrypoint.
            std::process::abort()
        }
    }
}

/// Resolves and activates one exact typed root factory without holding locks.
pub(super) unsafe extern "C" fn activate_factory_plugin(
    plugin_id: NcGuid,
    factory_interface: NcInterfaceId,
    requested_threading: u32,
    out: *mut neocogi_com_core::ComRaw<IUnknown>,
) -> NcStatus {
    let runtime = RuntimeState::global();
    let activation = {
        let catalog = runtime.read_catalog();
        factory_activation::resolve_factory(&catalog, plugin_id, factory_interface, requested_threading)
    };
    // SAFETY: This forwards the caller's output contract after releasing the
    // catalog lock, allowing plugin code to re-enter runtime operations.
    let status = unsafe { factory_activation::activate_factory(activation, factory_interface, out) };
    if status.is_err() {
        emit_error(NC_DEBUG_TYPE_RUNTIME, status, "factory plugin activation failed");
    }
    status
}

/// Maps detailed internal loader phases to the stable public status categories.
pub(super) fn load_error_status(kind: PluginLoadErrorKind) -> NcStatus {
    // Platform diagnostics remain available through the debug callback. The C
    // status preserves only categories standardized by the runtime ABI.
    match kind {
        PluginLoadErrorKind::OpenLibrary | PluginLoadErrorKind::FindSymbol | PluginLoadErrorKind::Entrypoint | PluginLoadErrorKind::UnsupportedPlatform => {
            NC_E_LOAD_FAILED
        }
        PluginLoadErrorKind::Metadata | PluginLoadErrorKind::Registration => NC_E_INVALID_ARG,
    }
}
