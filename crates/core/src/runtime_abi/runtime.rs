//! ABI table for factory-plugin registration, plugin loading, and root activation.
//!
//! Runtime ABI v1 accepts static descriptors and loads trusted plugin libraries,
//! and activates the one typed root factory exported by each plugin.
//! The table is intentionally separate from read-only registry and inspection
//! functions so host responsibilities remain explicit.
//!
//! The table contains behavior, not registry storage. Its function pointers are
//! discovered through `nc_com_get_runtime_api_v1` only after the caller checks
//! `nc_com_runtime_abi_version`. The leading `size` is checked by consumers;
//! this pre-release v1 contract changes in place across repository revisions.

use crate::{IUnknown, NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcStatus};

/// Process-global runtime operations for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcRuntimeApiV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Core descriptor/object ABI understood by this runtime.
    pub core_abi_version: u32,
    /// Validates and atomically registers one plugin and its hard provider IDs.
    pub register_factory_plugin_description: unsafe extern "C" fn(plugin: *const NcFactoryPluginDescription) -> NcStatus,
    /// Loads and registers one trusted factory plugin, returning its stable GUID.
    /// Reopening the same descriptor reuses its registration and first path;
    /// a distinct descriptor claiming that identity is rejected. The output is
    /// cleared to the nil GUID before validation and initialized on success.
    /// The path is nonempty UTF-8 without interior NUL bytes.
    ///
    /// The library must export `nc_get_factory_plugin_description_v1`. Class-table
    /// descriptor exports are not an activation mechanism. After argument
    /// validation, any native loading, description, dependency, or registration
    /// failure emits diagnostics and aborts the entire application. There is no
    /// recoverable activation mode. Raw argument errors return a status before
    /// activation; application loading wrappers must treat those as fatal too.
    pub load_plugin_library_utf8: unsafe extern "C" fn(path: *const u8, path_len: usize, out_plugin_id: *mut NcGuid) -> NcStatus,
    /// Activates one registered plugin's root factory after checking its exact
    /// interface identity and requested threading capability.
    /// Success transfers the exact factory object/table pair, erased only in
    /// its raw table type. It does not advertise canonical IUnknown identity.
    pub activate_factory_plugin:
        unsafe extern "C" fn(plugin_id: NcGuid, factory_interface: NcInterfaceId, requested_threading: u32, out: *mut crate::ComRaw<IUnknown>) -> NcStatus,
}
