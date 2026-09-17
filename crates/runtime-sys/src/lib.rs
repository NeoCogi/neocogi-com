//! Raw link-time bindings for the `neocogi-com-runtime` shared library.
//!
//! By default, applications link one compatible `neocogi-com-runtime` shared library.
//! The `static-runtime` feature instead links that same implementation into the
//! final binary, including browser modules. Both modes verify
//! [`nc_com_runtime_abi_version`] before using a versioned table getter.
//!
//! Set `NC_COM_RUNTIME_LINK_DIR` to the directory containing the built runtime
//! in shared mode when Cargo needs a native linker search path (and its
//! import library on Windows).

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

pub use neocogi_com_core::{NcDebugApiV3, NcInspectFactoryPluginDescriptionCallbackV3, NcInspectionApiV3, NcPluginRegistryV3, NcRuntimeApiV3};

// Keep the implementation in the link graph so the exact same exported C
// symbols resolve from its rlib. No alternate tables or catalog are introduced.
#[cfg(feature = "static-runtime")]
extern crate nc_com_shared_runtime as _;

#[cfg_attr(all(not(test), not(feature = "static-runtime")), link(name = "nc_com_shared_runtime"))]
unsafe extern "C" {
    /// Returns the runtime shared-library ABI generation.
    pub fn nc_com_runtime_abi_version() -> u32;

    /// Returns the runtime-operation table for runtime ABI v3.
    pub fn nc_com_get_runtime_api_v3() -> *const NcRuntimeApiV3;

    /// Returns the runtime ABI v3 plugin-registry table.
    pub fn nc_com_get_plugin_registry_v3() -> *const NcPluginRegistryV3;

    /// Returns the runtime ABI v3 inspection table.
    pub fn nc_com_get_inspection_api_v3() -> *const NcInspectionApiV3;

    /// Returns the runtime ABI v3 debug-messenger table.
    pub fn nc_com_get_debug_api_v3() -> *const NcDebugApiV3;
}
