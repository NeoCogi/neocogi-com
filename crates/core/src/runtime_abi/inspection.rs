//! Runtime ABI v1 records for plugin identity, provenance and dependencies.
//!
//! Plugin and factory views derive from one descriptor. A nonempty library path
//! reports the first dynamic registration; an empty path reports static origin.
//! Borrowed text remains process-live and callers copy it for independent
//! ownership. Inspection of an unregistered library separately lends its
//! validated descriptor only for the synchronous callback.

use core::ffi::c_void;

use super::NcUtf8View;
use crate::{NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcStatus};

/// Callback that receives one validated factory description during inspection.
///
/// `plugin` and every pointer reachable from it are borrowed only for the
/// duration of the callback. The callback must not retain those pointers. The
/// runtime passes `user_data` through unchanged and returns the callback's
/// status to the caller. The callback must not unwind across the ABI boundary.
pub type NcInspectFactoryPluginDescriptionCallbackV1 = unsafe extern "C" fn(plugin: *const NcFactoryPluginDescription, user_data: *mut c_void) -> NcStatus;

/// Inspection record for one registered plugin.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcPluginInfoV1 {
    /// Stable plugin identity.
    pub plugin_id: NcGuid,
    /// Qualified plugin name.
    pub qualified_name: NcUtf8View,
    /// Plugin version text.
    pub version: NcUtf8View,
    /// Library path used for the first successful dynamic registration.
    /// Static registrations return an empty view. Dynamic paths are nonempty;
    /// storage is immutable and lives for the process lifetime.
    pub library_path: NcUtf8View,
}

/// Factory metadata inspection record for one registered plugin.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcPluginFactoryInfoV1 {
    /// Stable plugin identity.
    pub plugin_id: NcGuid,
    /// Exact root factory interface identity.
    pub root_factory: NcInterfaceId,
    /// Threading capability declared by the root factory.
    pub root_factory_threading: u32,
}

/// Read-only plugin metadata, provenance and dependencies for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcInspectionApiV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Writes metadata for one registered plugin.
    pub plugin_info: unsafe extern "C" fn(plugin_id: NcGuid, out: *mut NcPluginInfoV1) -> NcStatus,
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
    /// [`NcInspectFactoryPluginDescriptionCallbackV1`]'s
    /// pointer, lifetime, and no-unwind contract. `user_data` is passed through
    /// unchanged and may be null.
    pub inspect_plugin_library_utf8: unsafe extern "C" fn(
        path: *const u8,
        path_len: usize,
        callback: Option<NcInspectFactoryPluginDescriptionCallbackV1>,
        user_data: *mut c_void,
    ) -> NcStatus,
    /// Writes factory metadata for one registered plugin.
    pub plugin_factory_info: unsafe extern "C" fn(plugin_id: NcGuid, out: *mut NcPluginFactoryInfoV1) -> NcStatus,
}

#[cfg(test)]
mod tests {
    use super::{NcInspectionApiV1, NcPluginFactoryInfoV1};

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn inspection_api_v1_exposes_descriptor_inspection() {
        assert_eq!(core::mem::size_of::<NcInspectionApiV1>(), 48);
        assert_eq!(core::mem::align_of::<NcInspectionApiV1>(), 8);
        assert_eq!(core::mem::offset_of!(NcInspectionApiV1, inspect_plugin_library_utf8), 32);
        assert_eq!(core::mem::offset_of!(NcInspectionApiV1, plugin_factory_info), 40);

        // The pre-release v1 contract adds provenance to the identity record, not a second ID.
        assert_eq!(core::mem::size_of::<super::NcPluginInfoV1>(), 64);
        assert_eq!(core::mem::offset_of!(super::NcPluginInfoV1, library_path), 48);

        assert_eq!(core::mem::size_of::<NcPluginFactoryInfoV1>(), 48);
        assert_eq!(core::mem::align_of::<NcPluginFactoryInfoV1>(), 8);
        assert_eq!(core::mem::offset_of!(NcPluginFactoryInfoV1, plugin_id), 0);
        assert_eq!(core::mem::offset_of!(NcPluginFactoryInfoV1, root_factory), 16);
        assert_eq!(core::mem::offset_of!(NcPluginFactoryInfoV1, root_factory_threading), 40);
    }
}
