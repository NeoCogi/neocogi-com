//! Raw discovery of the runtime selected by the facade feature graph.
//!
//! Shared callers import the C symbols from nc_com; static callers retain the
//! implementation rlib. Both routes use the same versioned tables and checks.

use neocogi_com_core::{NcPersistenceApiV1, NcDebugApiV1, NcInspectionApiV1, NcPluginRegistryV1, NcRuntimeApiV1};

// Keep the implementation in the link graph so the exact same exported C
// symbols resolve from its rlib. No alternate tables or catalog are introduced.
#[cfg(feature = "static-runtime")]
extern crate nc_com as _;

#[cfg_attr(all(not(test), not(feature = "static-runtime")), link(name = "nc_com"))]
unsafe extern "C" {
    /// Returns the runtime ABI v1 caller-scoped persistence table.
    pub(crate) fn nc_com_get_persistence_api_v1() -> *const NcPersistenceApiV1;

    /// Returns the runtime shared-library ABI generation.
    pub(crate) fn nc_com_runtime_abi_version() -> u32;

    /// Returns the runtime-operation table for runtime ABI v1.
    pub(crate) fn nc_com_get_runtime_api_v1() -> *const NcRuntimeApiV1;

    /// Returns the runtime ABI v1 plugin-registry table.
    pub(crate) fn nc_com_get_plugin_registry_v1() -> *const NcPluginRegistryV1;

    /// Returns the runtime ABI v1 inspection table.
    pub(crate) fn nc_com_get_inspection_api_v1() -> *const NcInspectionApiV1;

    /// Returns the runtime ABI v1 debug-messenger table.
    pub(crate) fn nc_com_get_debug_api_v1() -> *const NcDebugApiV1;
}
