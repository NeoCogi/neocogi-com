//! ABI records and queries for runtime metadata, provenance, and relationships.
//!
//! Registry tables answer which identities exist. Inspection complements them
//! with descriptive records and edges between plugins and modules. Text fields
//! are borrowed runtime-owned views and must be copied
//! by callers that need independent storage. The same table also provides a
//! synchronous, non-registering path that opens trusted plugin code and lends a
//! fully validated factory description to a caller callback.

use core::ffi::c_void;

use super::NcUtf8View;
use crate::{NcFactoryPluginDescription, NcGuid, NcStatus};

/// Callback that receives one validated factory description during inspection.
///
/// `plugin` and every pointer reachable from it are borrowed only for the
/// duration of the callback. The callback must not retain those pointers. The
/// runtime passes `user_data` through unchanged and returns the callback's
/// status to the caller. The callback must not unwind across the ABI boundary.
pub type NcInspectFactoryPluginDescriptionCallbackV3 = unsafe extern "C" fn(plugin: *const NcFactoryPluginDescription, user_data: *mut c_void) -> NcStatus;

/// Inspection record for one registered plugin.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcPluginInfoV3 {
    /// Stable plugin identity.
    pub plugin_id: NcGuid,
    /// Qualified plugin name.
    pub qualified_name: NcUtf8View,
    /// Plugin version text.
    pub version: NcUtf8View,
}

/// Inspection record for one retained loaded module.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcModuleInfoV3 {
    /// Process-local module ID.
    pub module_id: u64,
    /// Plugin supplied by this module.
    pub plugin_id: NcGuid,
    /// UTF-8 path used to load this module.
    pub path: NcUtf8View,
}

/// Read-only metadata and relationship inspection for runtime ABI v3.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcInspectionApiV3 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Writes metadata for one registered plugin.
    pub plugin_info: unsafe extern "C" fn(plugin_id: NcGuid, out: *mut NcPluginInfoV3) -> NcStatus,
    /// Writes metadata for one retained loaded module.
    pub module_info: unsafe extern "C" fn(module_id: u64, out: *mut NcModuleInfoV3) -> NcStatus,
    /// Returns the number of hard dependencies declared by one plugin.
    pub plugin_dependency_count: unsafe extern "C" fn(plugin_id: NcGuid) -> usize,
    /// Copies hard dependency plugin IDs declared by one plugin.
    pub plugin_dependency_ids: unsafe extern "C" fn(plugin_id: NcGuid, out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Opens and validates a trusted plugin library without registering it.
    ///
    /// On success, `callback` is invoked synchronously exactly once with the
    /// validated factory description and `user_data`, and its status is returned
    /// unchanged. The description graph is borrowed only for that invocation.
    /// Loading may execute platform loader code. Once opened, the library is
    /// pinned until process exit even if later inspection fails.
    ///
    /// `path` must point to `path_len` readable UTF-8 bytes with no interior
    /// NUL. `callback` must obey
    /// [`NcInspectFactoryPluginDescriptionCallbackV3`]'s
    /// pointer, lifetime, and no-unwind contract. `user_data` is passed through
    /// unchanged and may be null.
    pub inspect_plugin_library_utf8: unsafe extern "C" fn(
        path: *const u8,
        path_len: usize,
        callback: Option<NcInspectFactoryPluginDescriptionCallbackV3>,
        user_data: *mut c_void,
    ) -> NcStatus,
}

#[cfg(test)]
mod tests {
    use super::NcInspectionApiV3;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn inspection_api_v3_exposes_descriptor_inspection() {
        assert_eq!(core::mem::size_of::<NcInspectionApiV3>(), 48);
        assert_eq!(core::mem::align_of::<NcInspectionApiV3>(), 8);
        assert_eq!(core::mem::offset_of!(NcInspectionApiV3, inspect_plugin_library_utf8), 40);
    }
}
