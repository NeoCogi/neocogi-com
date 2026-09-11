//! C callbacks for runtime mutation and object construction.
//!
//! These adapters normalize raw-pointer validation, status mapping, and debug
//! emission before delegating to the Rust implementation. They never expose
//! Rust errors, paths, or owned containers across the ABI boundary. Live plugin
//! activation is fail-fatal after detailed diagnostic emission, whereas direct
//! registration, requirement checks, and construction return ordinary statuses.

use neocogi_com_core::{
    IUnknown, NC_DEBUG_TYPE_PLUGIN, NC_DEBUG_TYPE_RUNTIME, NC_DEBUG_TYPE_VALIDATION, NC_E_INVALID_ARG, NC_E_LOAD_FAILED, NC_E_OUT_OF_RANGE, NC_E_POINTER,
    NC_OK, NcContractModuleReference, NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcPluginDescriptor, NcStatus,
};

use crate::construction;
use crate::factory_activation;
use crate::loader::PluginLoadErrorKind;
use crate::state::RuntimeState;

use super::{emit_error, path_from_utf8};

/// Validates and atomically registers one native contract dependency graph.
pub(super) unsafe extern "C" fn register_contract_module(module: *const NcContractModuleReference) -> NcStatus {
    // SAFETY: The caller provides the reference and transitive native graph
    // lifetime required by the canonical contract registry.
    let status = unsafe { RuntimeState::global().register_contract_module(module) };
    if status.is_err() {
        emit_error(NC_DEBUG_TYPE_VALIDATION, status, "contract module registration failed");
    }
    status
}

/// Validates and atomically registers one factory-only plugin description.
pub(super) unsafe extern "C" fn register_factory_plugin_description(plugin: *const NcFactoryPluginDescription) -> NcStatus {
    // SAFETY: The caller supplies the plugin and linked contract graph lifetime
    // required by process-wide registration.
    let status = unsafe { RuntimeState::global().register_factory_plugin_description(plugin) };
    if status.is_err() {
        emit_error(NC_DEBUG_TYPE_VALIDATION, status, "factory plugin registration failed");
    }
    status
}

/// Validates and atomically registers one caller-owned static descriptor graph.
pub(super) unsafe extern "C" fn register_plugin_descriptor(plugin: *const NcPluginDescriptor) -> NcStatus {
    // SAFETY: The caller provides the descriptor lifetime and readability
    // contract required by the registry.
    let status = unsafe { RuntimeState::global().register_plugin_descriptor(plugin) };
    if status.is_err() {
        emit_error(NC_DEBUG_TYPE_VALIDATION, status, "plugin descriptor registration failed");
    }
    status
}

/// Performs required live activation and aborts after any load/registration failure.
pub(super) unsafe extern "C" fn load_plugin_library_utf8(path: *const u8, path_len: usize, out_module_id: *mut u64) -> NcStatus {
    if out_module_id.is_null() {
        return NC_E_POINTER;
    }
    // Clearing first guarantees that every failure path leaves no stale handle
    // for a C caller to mistake for a successful load.
    // SAFETY: `out_module_id` is valid for one write by the caller's contract.
    unsafe { *out_module_id = 0 };
    let path = match unsafe { path_from_utf8(path, path_len) } {
        Ok(path) => path,
        Err(status) => return status,
    };

    // SAFETY: Loading executes trusted code selected by the caller.
    match unsafe { RuntimeState::global().load_plugin_library(path) } {
        Ok(module_id) => {
            // SAFETY: `out_module_id` was checked above.
            unsafe { *out_module_id = module_id.get() };
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

/// Checks a registered class against one complete interface identity.
pub(super) unsafe extern "C" fn require_interface(class_id: NcGuid, interface_id: NcInterfaceId) -> NcStatus {
    let result = {
        let catalog = RuntimeState::global().read_catalog();
        construction::require_interface(&catalog.type_registry, class_id, interface_id)
    };
    match result {
        Ok(()) => NC_OK,
        Err(status) => {
            emit_error(NC_DEBUG_TYPE_VALIDATION, status, "registered class does not satisfy the requested interface");
            status
        }
    }
}

/// Constructs one registered class after validating threading and arguments.
pub(super) unsafe extern "C" fn construct_class(
    class_id: NcGuid,
    requested_threading: u32,
    constructor_abi_signature: neocogi_com_core::NcAbiSignature,
    arguments: *const core::ffi::c_void,
    out: *mut *mut IUnknown,
) -> NcStatus {
    // Resolve under the catalog read lock, then release it before invoking the
    // plugin constructor or emitting a callback-driven diagnostic.
    let class = {
        let catalog = RuntimeState::global().read_catalog();
        construction::resolve_class(&catalog.type_registry, class_id, requested_threading)
    };
    // SAFETY: This forwards the caller's borrowed frame and output contracts.
    let status = unsafe { construction::construct_class(class, constructor_abi_signature, arguments, out) };
    if status.is_err() {
        emit_error(NC_DEBUG_TYPE_RUNTIME, status, "registered class construction failed");
    }
    status
}

/// Resolves and activates one exact typed root factory without holding locks.
pub(super) unsafe extern "C" fn activate_factory_plugin(
    plugin_id: NcGuid,
    factory_interface: NcInterfaceId,
    requested_threading: u32,
    out: *mut *mut IUnknown,
) -> NcStatus {
    let activation = {
        let catalog = RuntimeState::global().read_catalog();
        factory_activation::resolve_factory(&catalog.factory_plugin_registry, plugin_id, factory_interface, requested_threading)
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
        PluginLoadErrorKind::ResourceLimit => NC_E_OUT_OF_RANGE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loaded_module_capacity_maps_to_out_of_range() {
        assert_eq!(load_error_status(PluginLoadErrorKind::ResourceLimit), NC_E_OUT_OF_RANGE);
    }
}
