//! Unified plugin operations supplied by the host-owned `neocogi_com_host` runtime.
//! Registration, loading, factory acquisition, lookup, enumeration and unregistered
//! inspection share this table. Diagnostics and persistence have independent
//! lifecycles and tables. Callers admit this immutable size-prefixed table through
//! the host-supplied API description. Runtime discovery stays family 1;
//! the separately checked core generation freezes compatible table layouts and
//! callbacks. Deploy all participants from the same published SDK release.

use core::ffi::c_void;
use crate::{IUnknown, NcPluginDescriptor, NcGuid, NcInterfaceId, NcStatus};
use super::{NcInspectPluginDescriptorCallbackV1, NcPluginInfoV1};

/// Process-global runtime operations for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcRuntimeApiV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Core descriptor/object ABI understood by this runtime.
    pub core_abi_version: u32,
    /// Validates and atomically registers one plugin and its hard provider IDs.
    /// The entire immutable descriptor graph and bootstrap code must remain live
    /// for the process lifetime. Null returns `NC_E_POINTER`; invalid descriptors,
    /// missing dependencies and conflicting identities return validation statuses.
    pub register_plugin_descriptor: unsafe extern "C" fn(plugin: *const NcPluginDescriptor) -> NcStatus,
    /// Loads and registers one trusted factory plugin, returning its stable GUID.
    /// Reopening the same descriptor reuses its registration and first path;
    /// a distinct descriptor claiming that identity is rejected. The output is
    /// cleared to the nil GUID before validation and initialized on success.
    /// The path is nonempty UTF-8 without interior NUL bytes.
    ///
    /// The library must export `nc_get_plugin_descriptor_v1`. Class-table
    /// descriptor exports are not a loading mechanism. After argument
    /// validation, any native loading, description, dependency, or registration
    /// failure emits diagnostics and aborts the entire application. There is no
    /// recoverable loading mode. Raw argument errors return a status before
    /// loading; application loading wrappers must treat those as fatal too.
    pub load_plugin_library_utf8: unsafe extern "C" fn(path: *const u8, path_len: usize, out_plugin_id: *mut NcGuid) -> NcStatus,
    /// Acquires one registered plugin's root factory after checking its exact
    /// interface identity and requested threading capability.
    /// Success transfers the exact factory object/table pair, erased only in
    /// its raw table type. It does not advertise canonical IUnknown identity.
    /// The output must be writable for one pair and is cleared before validation.
    /// Unknown plugins, incompatible contracts and bootstrap failures return
    /// ordinary statuses. Each success transfers one owned reference; factories
    /// may be newly created or retained existing objects.
    pub acquire_factory_plugin:
        unsafe extern "C" fn(plugin_id: NcGuid, factory_interface: NcInterfaceId, requested_threading: u32, out: *mut crate::ComRaw<IUnknown>) -> NcStatus,
    /// Writes one complete plugin record, without acquiring its factory.
    /// `out` must be writable and aligned for one record. Null returns
    /// `NC_E_POINTER`; an unknown GUID returns `NC_E_NOT_FOUND`. Failure leaves
    /// storage untouched. Successful borrowed views remain process-lived.
    pub plugin_info: unsafe extern "C" fn(plugin_id: NcGuid, out: *mut NcPluginInfoV1) -> NcStatus,
    /// Returns a snapshot count of registered plugins, including static ones.
    /// Registration can grow the count before the subsequent fill.
    pub plugin_count: unsafe extern "C" fn() -> usize,
    /// Copies complete plugin records in successful registration order.
    /// `out_len` must be writable for one length; otherwise `NC_E_POINTER`.
    /// The required length is written first. Insufficient capacity returns
    /// `NC_E_INSUFFICIENT_BUFFER` without writing records. Empty fills accept a
    /// null `out`; nonempty fills require aligned storage for `capacity` records.
    /// Outputs must not overlap. One fill holds one catalog read guard, giving
    /// a coherent snapshot; returned views survive the guard and catalog growth.
    pub plugin_infos: unsafe extern "C" fn(out: *mut NcPluginInfoV1, capacity: usize, out_len: *mut usize) -> NcStatus,
    /// Opens a trusted library without registration or factory acquisition.
    /// `path` is readable for `path_len` nonempty UTF-8 bytes without NUL.
    /// A nonnull callback obeys its synchronous borrow/no-unwind contract;
    /// `user_data` is passed through unchanged and may be null. Null callback
    /// returns `NC_E_POINTER`; path and loader/validation failures return statuses.
    /// Success invokes the callback exactly once without holding a catalog lock,
    /// returning its status unchanged. Once opened, mappings remain pinned even
    /// on subsequent failure. Opening may execute platform loader code.
    pub inspect_plugin_library_utf8:
        unsafe extern "C" fn(path: *const u8, path_len: usize, callback: Option<NcInspectPluginDescriptorCallbackV1>, user_data: *mut c_void) -> NcStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn unified_runtime_table_has_explicit_c_layout() {
        assert_eq!(core::mem::size_of::<NcRuntimeApiV1>(), 72);
        assert_eq!(core::mem::offset_of!(NcRuntimeApiV1, plugin_info), 40);
        assert_eq!(core::mem::offset_of!(NcRuntimeApiV1, plugin_count), 48);
        assert_eq!(core::mem::offset_of!(NcRuntimeApiV1, plugin_infos), 56);
        assert_eq!(core::mem::offset_of!(NcRuntimeApiV1, inspect_plugin_library_utf8), 64);
    }
}
