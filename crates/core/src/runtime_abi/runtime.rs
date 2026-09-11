//! ABI table for operations that mutate runtime state or create objects.
//!
//! Runtime ABI v1 accepts static descriptors, loads trusted plugin libraries,
//! validates registered interface requirements, and constructs class instances.
//! The table is intentionally separate from read-only registry and inspection
//! functions so host responsibilities remain explicit.
//!
//! The table contains behavior, not registry storage. Its function pointers are
//! discovered through `nc_com_get_runtime_api_v1` only after the caller checks
//! `nc_com_runtime_abi_version`. The leading `size` permits append-only table
//! growth within runtime ABI v1.

use crate::{IUnknown, NcAbiSignature, NcContractModuleReference, NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcPluginDescriptor, NcStatus};

/// Process-global runtime operations for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcRuntimeApiV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Core descriptor/object ABI understood by this runtime.
    pub core_abi_version: u32,
    /// Validates and registers one canonical contract module dependency graph.
    pub register_contract_module: unsafe extern "C" fn(module: *const NcContractModuleReference) -> NcStatus,
    /// Validates and atomically registers one factory-only plugin plus all
    /// directly referenced contract module graphs.
    pub register_factory_plugin_description: unsafe extern "C" fn(plugin: *const NcFactoryPluginDescription) -> NcStatus,
    /// Validates and registers one static plugin descriptor.
    pub register_plugin_descriptor: unsafe extern "C" fn(plugin: *const NcPluginDescriptor) -> NcStatus,
    /// Loads and registers one trusted plugin shared library from a nonempty
    /// UTF-8 path without interior NUL bytes.
    pub load_plugin_library_utf8: unsafe extern "C" fn(path: *const u8, path_len: usize, out_module_id: *mut u64) -> NcStatus,
    /// Loads and registers one trusted factory-only plugin shared library from
    /// a nonempty UTF-8 path without interior NUL bytes.
    pub load_factory_plugin_library_utf8: unsafe extern "C" fn(path: *const u8, path_len: usize, out_module_id: *mut u64) -> NcStatus,
    /// Validates that a registered class implements one interface ABI identity.
    pub require_interface: unsafe extern "C" fn(class_id: NcGuid, interface_id: NcInterfaceId) -> NcStatus,
    /// Constructs one registered class with the requested threading capability
    /// and returns an owned `IUnknown` pointer.
    ///
    /// `requested_threading` is one of the `NC_ABI_THREADING_*` values.
    /// Thread-local is always a permitted conservative view; thread-safe is
    /// permitted only for a class whose descriptor promises thread safety.
    pub construct_class: unsafe extern "C" fn(
        class_id: NcGuid,
        requested_threading: u32,
        constructor_abi_signature: NcAbiSignature,
        arguments: *const core::ffi::c_void,
        out: *mut *mut IUnknown,
    ) -> NcStatus,
    /// Activates one registered plugin's root factory after checking its exact
    /// interface identity and requested threading capability.
    pub activate_factory_plugin:
        unsafe extern "C" fn(plugin_id: NcGuid, factory_interface: NcInterfaceId, requested_threading: u32, out: *mut *mut IUnknown) -> NcStatus,
}
