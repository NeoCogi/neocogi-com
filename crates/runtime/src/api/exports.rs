//! Stable C symbols exported by the runtime shared library.
//!
//! Only the runtime ABI generation function and three immutable table getters
//! are exported by name. All operations remain private Rust functions reached
//! through those tables, keeping symbol discovery small and version checks
//! mandatory before callers dereference versioned layouts.
//!
//! The unversioned ABI-generation function is the mandatory discovery point.
//! A caller must compare it with the generation it understands before invoking
//! a versioned getter. During pre-alpha both sides must also use the same
//! checkout revision, because pre-alpha generations do not promise cross-revision compatibility.

use neocogi_com_core::{NC_COM_RUNTIME_ABI_VERSION, NcPersistenceApiV1, NcDebugApiV1, NcRuntimeApiV1};

use super::{PERSISTENCE_API_V1, DEBUG_API_V1, RUNTIME_API_V1};

/// Returns the runtime shared-library ABI generation.
///
/// Callers must check this value before calling any versioned API getter.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_runtime_abi_version() -> u32 {
    NC_COM_RUNTIME_ABI_VERSION
}

/// Returns the immutable runtime-operation table for runtime ABI v1.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_runtime_api_v1() -> *const NcRuntimeApiV1 {
    &RUNTIME_API_V1
}

/// Returns the immutable runtime ABI v1 debug-messenger table.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_debug_api_v1() -> *const NcDebugApiV1 {
    &DEBUG_API_V1
}

/// Returns caller-scoped persistence operations for runtime ABI v1.
#[unsafe(no_mangle)]
pub extern "C" fn nc_com_get_persistence_api_v1() -> *const NcPersistenceApiV1 {
    &PERSISTENCE_API_V1
}
