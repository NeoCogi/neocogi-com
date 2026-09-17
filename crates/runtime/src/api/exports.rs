//! Stable C symbols exported by the runtime shared library.
//!
//! Only the runtime ABI generation function and four immutable table getters
//! are exported by name. All operations remain private Rust functions reached
//! through those tables, keeping symbol discovery small and version checks
//! mandatory before callers dereference versioned layouts.
//!
//! The unversioned ABI-generation function is the mandatory discovery point.
//! A caller must compare it with the generation it understands before invoking
//! a versioned getter; a v3 runtime is not assumed to preserve v2 table shapes.

use neocogi_com_core::{NC_COM_RUNTIME_ABI_VERSION, NcDebugApiV3, NcInspectionApiV3, NcPluginRegistryV3, NcRuntimeApiV3};

use super::{DEBUG_API_V3, INSPECTION_API_V3, PLUGIN_REGISTRY_V3, RUNTIME_API_V3};

/// Returns the runtime shared-library ABI generation.
///
/// Callers must check this value before calling any versioned API getter.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_runtime_abi_version() -> u32 {
    NC_COM_RUNTIME_ABI_VERSION
}

/// Returns the immutable runtime-operation table for runtime ABI v3.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_runtime_api_v3() -> *const NcRuntimeApiV3 {
    &RUNTIME_API_V3
}

/// Returns the immutable runtime ABI v3 plugin-registry table.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_plugin_registry_v3() -> *const NcPluginRegistryV3 {
    &PLUGIN_REGISTRY_V3
}

/// Returns the immutable runtime ABI v3 inspection table.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_inspection_api_v3() -> *const NcInspectionApiV3 {
    &INSPECTION_API_V3
}

/// Returns the immutable runtime ABI v3 debug-messenger table.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_debug_api_v3() -> *const NcDebugApiV3 {
    &DEBUG_API_V3
}
