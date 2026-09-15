//! Stable C symbols exported by the runtime shared library.
//!
//! Only the runtime ABI generation function and five immutable table getters
//! are exported by name. All operations remain private Rust functions reached
//! through those tables, keeping symbol discovery small and version checks
//! mandatory before callers dereference versioned layouts.
//!
//! The unversioned ABI-generation function is the mandatory discovery point.
//! A caller must compare it with the generation it understands before invoking
//! a versioned getter; a v2 runtime is not assumed to preserve v1 table shapes.

use neocogi_com_core::{NC_COM_RUNTIME_ABI_VERSION, NcContractRegistryV2, NcDebugApiV2, NcInspectionApiV2, NcPluginRegistryV2, NcRuntimeApiV2};

use super::{CONTRACT_REGISTRY_V2, DEBUG_API_V2, INSPECTION_API_V2, PLUGIN_REGISTRY_V2, RUNTIME_API_V2};

/// Returns the runtime shared-library ABI generation.
///
/// Callers must check this value before calling any versioned API getter.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_runtime_abi_version() -> u32 {
    NC_COM_RUNTIME_ABI_VERSION
}

/// Returns the immutable runtime-operation table for runtime ABI v2.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_runtime_api_v2() -> *const NcRuntimeApiV2 {
    &RUNTIME_API_V2
}

/// Returns the immutable runtime ABI v2 contract-registry table.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_contract_registry_v2() -> *const NcContractRegistryV2 {
    &CONTRACT_REGISTRY_V2
}

/// Returns the immutable runtime ABI v2 plugin-registry table.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_plugin_registry_v2() -> *const NcPluginRegistryV2 {
    &PLUGIN_REGISTRY_V2
}

/// Returns the immutable runtime ABI v2 inspection table.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_inspection_api_v2() -> *const NcInspectionApiV2 {
    &INSPECTION_API_V2
}

/// Returns the immutable runtime ABI v2 debug-messenger table.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_debug_api_v2() -> *const NcDebugApiV2 {
    &DEBUG_API_V2
}
