//! Complete immutable views of registered plugins and their single factories.
//!
//! Lookup and enumeration return the same record. Text, dependency GUID arrays
//! and interface signatures remain readable for the process lifetime: registration
//! is append-only, descriptors are static or pinned, and catalog-owned buffers
//! never change. Copy the views when independent application ownership is needed.

use core::ffi::c_void;
use super::NcUtf8View;
use crate::{NcFactoryPluginDescription, NcGuid, NcInterfaceId, NcStatus};

/// Maximum number of distinct dynamically registered plugins in runtime ABI v1.
/// Repeated loads consume no slot. Static registrations and inspection-only
/// pinned opens are outside this limit and remain live until process exit.
pub const NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS: usize = 1_024;

/// Receives one validated, unregistered descriptor synchronously.
/// `plugin` and reachable pointers are borrowed only during this callback and
/// must not be retained. `user_data` is passed through unchanged. The callback
/// must not unwind; its status is returned unchanged by the runtime.
pub type NcInspectFactoryPluginDescriptionCallbackV1 = unsafe extern "C" fn(plugin: *const NcFactoryPluginDescription, user_data: *mut c_void) -> NcStatus;

/// Exact contract of the one root factory, without acquiring a live reference.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcFactoryInfoV1 {
    /// Exact IID and full ABI signature, retained for the process lifetime.
    pub interface: NcInterfaceId,
    /// Valid `NcAbiThreadingMode` raw value declared by the factory.
    pub threading: u32,
}

/// Complete borrowed description of one registered plugin.
/// Contains no factory object or ownership token. Lookup and enumeration copy
/// this record; its referenced storage survives catalog growth and lock release.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcPluginInfoV1 {
    /// Stable plugin identity used for lookup and factory acquisition.
    pub plugin_id: NcGuid,
    /// Qualified plugin name in immutable descriptor storage.
    pub qualified_name: NcUtf8View,
    /// Plugin release version text in immutable descriptor storage.
    pub version: NcUtf8View,
    /// First successful dynamic registration path; empty means static origin.
    /// This is provenance, not a file that callers need to reopen.
    pub library_path: NcUtf8View,
    /// The plugin's single root factory contract.
    pub factory: NcFactoryInfoV1,
    /// Immutable, aligned GUID array in declared dependency order.
    /// May be null when `dependency_count` is zero; otherwise readable for exactly
    /// that many GUIDs. Registration validates and owns this process-lived array.
    pub dependencies: *const NcGuid,
    /// Number of hard provider GUIDs in `dependencies`.
    pub dependency_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn complete_plugin_view_has_explicit_c_layout() {
        assert_eq!(core::mem::size_of::<NcFactoryInfoV1>(), 32);
        assert_eq!(core::mem::offset_of!(NcFactoryInfoV1, threading), 24);
        assert_eq!(core::mem::size_of::<NcPluginInfoV1>(), 112);
        assert_eq!(core::mem::offset_of!(NcPluginInfoV1, library_path), 48);
        assert_eq!(core::mem::offset_of!(NcPluginInfoV1, factory), 64);
        assert_eq!(core::mem::offset_of!(NcPluginInfoV1, dependencies), 96);
        assert_eq!(core::mem::offset_of!(NcPluginInfoV1, dependency_count), 104);
    }
}
