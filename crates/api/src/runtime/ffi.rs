//! Stateless runtime discovery declarations, independent of dependency features.
//!
//! Final native assemblies supply the shared import; final monolith/browser/test
//! assemblies retain the implementation once. The facade never chooses or owns
//! an implementation image, so additive feature unification cannot split catalogs.

use neocogi_com_core::{NcPersistenceApiV1, NcDebugApiV1, NcRuntimeApiV1};

unsafe extern "C" {
    /// Returns the runtime ABI v1 caller-scoped persistence table.
    pub(crate) fn nc_com_get_persistence_api_v1() -> *const NcPersistenceApiV1;

    /// Returns the runtime shared-library ABI generation.
    pub(crate) fn nc_com_runtime_abi_version() -> u32;

    /// Returns the runtime-operation table for runtime ABI v1.
    pub(crate) fn nc_com_get_runtime_api_v1() -> *const NcRuntimeApiV1;

    /// Returns the runtime ABI v1 debug-messenger table.
    pub(crate) fn nc_com_get_debug_api_v1() -> *const NcDebugApiV1;
}
