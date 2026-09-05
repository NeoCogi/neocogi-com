//! Raw link-time bindings for the `nc-com-runtime` shared library.
//!
//! By default, applications link one compatible `nc-com-runtime` shared library.
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

pub use nc_com_core::{
    NcDebugApiV1, NcInspectPluginDescriptorCallbackV1, NcInspectionApiV1, NcPluginRegistryV1,
    NcRuntimeApiV1, NcTypeRegistryV1,
};

// Keep the implementation in the link graph so the exact same exported C
// symbols resolve from its rlib. No alternate tables or catalog are introduced.
#[cfg(feature = "static-runtime")]
extern crate nc_com_runtime as _;

#[cfg_attr(all(not(test), not(feature = "static-runtime")), link(name = "nc_com_runtime"))]
unsafe extern "C" {
    /// Returns the runtime shared-library ABI generation.
    pub fn nc_com_runtime_abi_version() -> u32;

    /// Returns the runtime-operation table for runtime ABI v1.
    pub fn nc_com_get_runtime_api_v1() -> *const NcRuntimeApiV1;

    /// Returns the plugin-registry table for runtime ABI v1.
    pub fn nc_com_get_plugin_registry_v1() -> *const NcPluginRegistryV1;

    /// Returns the type-registry table for runtime ABI v1.
    pub fn nc_com_get_type_registry_v1() -> *const NcTypeRegistryV1;

    /// Returns the inspection table for runtime ABI v1.
    pub fn nc_com_get_inspection_api_v1() -> *const NcInspectionApiV1;

    /// Returns the debug-messenger table for runtime ABI v1.
    pub fn nc_com_get_debug_api_v1() -> *const NcDebugApiV1;
}
