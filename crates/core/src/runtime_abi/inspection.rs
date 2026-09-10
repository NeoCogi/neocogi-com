//! ABI records and queries for runtime metadata, provenance, and relationships.
//!
//! Registry tables answer which identities exist. Inspection complements them
//! with descriptive records and edges between plugins, modules, classes, and
//! interfaces. Text fields are borrowed runtime-owned views and must be copied
//! by callers that need independent storage. The same table also provides a
//! synchronous, non-registering path that opens trusted plugin code and lends a
//! fully validated descriptor graph to a caller callback.

use core::ffi::c_void;

use super::NcUtf8View;
use crate::{NcAbiSignature, NcGuid, NcInterfaceId, NcPluginDescriptor, NcStatus};

/// Callback that receives one validated plugin descriptor during inspection.
///
/// `plugin` and every pointer reachable from it are borrowed only for the
/// duration of the callback. The callback must not retain those pointers. The
/// runtime passes `user_data` through unchanged and returns the callback's
/// status to the caller. The callback must not unwind across the ABI boundary.
pub type NcInspectPluginDescriptorCallbackV1 = unsafe extern "C" fn(plugin: *const NcPluginDescriptor, user_data: *mut c_void) -> NcStatus;

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
}

/// Inspection record for one retained loaded module.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcModuleInfoV1 {
    /// Process-local module ID.
    pub module_id: u64,
    /// Plugin supplied by this module.
    pub plugin_id: NcGuid,
    /// UTF-8 path used to load this module.
    pub path: NcUtf8View,
}

/// Inspection record for one registered class.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcClassInfoV1 {
    /// Stable class identity.
    pub class_id: NcGuid,
    /// Plugin that registered this class.
    pub plugin_id: NcGuid,
    /// Process-local module ID, or zero for a static registration.
    pub module_id: u64,
    /// Qualified class name.
    pub qualified_name: NcUtf8View,
    /// Class version text.
    pub version: NcUtf8View,
    /// Constructor-frame ABI signature, or [`NcAbiSignature::EMPTY`] for no arguments.
    pub constructor_abi_signature: NcAbiSignature,
    /// Declared class threading capability (`NC_ABI_THREADING_*`).
    pub threading: u32,
}

/// Read-only metadata and relationship inspection for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcInspectionApiV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Writes metadata for one registered plugin.
    pub plugin_info: unsafe extern "C" fn(plugin_id: NcGuid, out: *mut NcPluginInfoV1) -> NcStatus,
    /// Writes metadata for one retained loaded module.
    pub module_info: unsafe extern "C" fn(module_id: u64, out: *mut NcModuleInfoV1) -> NcStatus,
    /// Writes metadata and provenance for one registered class.
    pub class_info: unsafe extern "C" fn(class_id: NcGuid, out: *mut NcClassInfoV1) -> NcStatus,
    /// Returns the number of hard dependencies declared by one plugin.
    pub plugin_dependency_count: unsafe extern "C" fn(plugin_id: NcGuid) -> usize,
    /// Copies hard dependency plugin IDs declared by one plugin.
    pub plugin_dependency_ids: unsafe extern "C" fn(plugin_id: NcGuid, out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Returns the number of classes registered by one plugin.
    pub plugin_class_count: unsafe extern "C" fn(plugin_id: NcGuid) -> usize,
    /// Copies class IDs registered by one plugin.
    pub plugin_class_ids: unsafe extern "C" fn(plugin_id: NcGuid, out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Returns the number of interfaces implemented by one class.
    pub class_interface_count: unsafe extern "C" fn(class_id: NcGuid) -> usize,
    /// Copies interface ABI identities implemented by one class.
    pub class_interface_ids: unsafe extern "C" fn(class_id: NcGuid, out: *mut NcInterfaceId, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Returns the number of registered classes implementing one interface.
    pub interface_implementor_class_count: unsafe extern "C" fn(interface_id: NcInterfaceId) -> usize,
    /// Copies IDs of registered classes implementing one interface.
    pub interface_implementor_class_ids: unsafe extern "C" fn(interface_id: NcInterfaceId, out: *mut NcGuid, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Opens and validates a trusted plugin library without registering it.
    ///
    /// On success, `callback` is invoked synchronously exactly once with the
    /// validated descriptor and `user_data`, and its status is returned
    /// unchanged. The descriptor graph is borrowed only for that invocation.
    /// Loading may execute platform loader code. Once opened, the library is
    /// pinned until process exit even if later inspection fails.
    ///
    /// `path` must point to `path_len` readable UTF-8 bytes with no interior
    /// NUL. `callback` must obey [`NcInspectPluginDescriptorCallbackV1`]'s
    /// pointer, lifetime, and no-unwind contract. `user_data` is passed through
    /// unchanged and may be null.
    pub inspect_plugin_library_utf8:
        unsafe extern "C" fn(path: *const u8, path_len: usize, callback: Option<NcInspectPluginDescriptorCallbackV1>, user_data: *mut c_void) -> NcStatus,
}

#[cfg(test)]
mod tests {
    use super::{NcClassInfoV1, NcInspectionApiV1};

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn class_info_v1_c_layout_includes_threading_field() {
        assert_eq!(core::mem::size_of::<NcClassInfoV1>(), 96);
        assert_eq!(core::mem::align_of::<NcClassInfoV1>(), 8);
        assert_eq!(core::mem::offset_of!(NcClassInfoV1, class_id), 0);
        assert_eq!(core::mem::offset_of!(NcClassInfoV1, plugin_id), 16);
        assert_eq!(core::mem::offset_of!(NcClassInfoV1, module_id), 32);
        assert_eq!(core::mem::offset_of!(NcClassInfoV1, qualified_name), 40);
        assert_eq!(core::mem::offset_of!(NcClassInfoV1, version), 56);
        assert_eq!(core::mem::offset_of!(NcClassInfoV1, constructor_abi_signature), 72);
        assert_eq!(core::mem::offset_of!(NcClassInfoV1, threading), 88);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn inspection_api_v1_appends_descriptor_inspection() {
        assert_eq!(core::mem::size_of::<NcInspectionApiV1>(), 104);
        assert_eq!(core::mem::align_of::<NcInspectionApiV1>(), 8);
        assert_eq!(core::mem::offset_of!(NcInspectionApiV1, inspect_plugin_library_utf8), 96);
    }
}
