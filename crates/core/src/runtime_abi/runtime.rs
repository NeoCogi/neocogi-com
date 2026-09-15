//! ABI table for contract registration, plugin loading, and root activation.
//!
//! Runtime ABI v2 accepts static descriptors and loads trusted plugin libraries,
//! and activates the one typed root factory exported by each plugin.
//! The table is intentionally separate from read-only registry and inspection
//! functions so host responsibilities remain explicit.
//!
//! The table contains behavior, not registry storage. Its function pointers are
//! discovered through `nc_com_get_runtime_api_v2` only after the caller checks
//! `nc_com_runtime_abi_version`. The leading `size` permits append-only table
//! growth within runtime ABI v2.

use crate::{IUnknown, NcContractModuleReference, NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcStatus};

/// Process-global runtime operations for runtime ABI v2.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcRuntimeApiV2 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Core descriptor/object ABI understood by this runtime.
    pub core_abi_version: u32,
    /// Validates and registers one canonical contract module dependency graph.
    pub register_contract_module: unsafe extern "C" fn(module: *const NcContractModuleReference) -> NcStatus,
    /// Validates and atomically registers one factory-only plugin plus all
    /// directly referenced contract module graphs.
    pub register_factory_plugin_description: unsafe extern "C" fn(plugin: *const NcFactoryPluginDescription) -> NcStatus,
    /// Loads and registers one trusted factory plugin shared library from a
    /// nonempty UTF-8 path without interior NUL bytes.
    ///
    /// The library must export `nc_get_factory_plugin_description`. Class-table
    /// descriptor exports are not an activation mechanism.
    pub load_plugin_library_utf8: unsafe extern "C" fn(path: *const u8, path_len: usize, out_module_id: *mut u64) -> NcStatus,
    /// Activates one registered plugin's root factory after checking its exact
    /// interface identity and requested threading capability.
    pub activate_factory_plugin:
        unsafe extern "C" fn(plugin_id: NcGuid, factory_interface: NcInterfaceId, requested_threading: u32, out: *mut *mut IUnknown) -> NcStatus,
}
