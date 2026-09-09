//! Versioned ABI tables exposed by the process-global runtime library.
//!
//! This module groups runtime mutation, registry enumeration, inspection, and
//! debug contracts while keeping their `repr(C)` layouts in the allocation-free
//! core crate. Every table begins with a byte size so compatible revisions can
//! append fields without changing the established prefix.

mod debug;
mod inspection;
mod plugin_registry;
mod runtime;
mod type_registry;

pub use debug::{
    NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS, NC_DEBUG_SEVERITY_ALL, NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_SEVERITY_INFO, NC_DEBUG_SEVERITY_WARNING,
    NC_DEBUG_TYPE_ALL, NC_DEBUG_TYPE_PLUGIN, NC_DEBUG_TYPE_RUNTIME, NC_DEBUG_TYPE_VALIDATION, NcDebugApiV1, NcDebugCallbackV1, NcDebugMessageV1,
    NcDebugMessengerCreateInfoV1,
};
pub use inspection::{NcClassInfoV1, NcInspectPluginDescriptorCallbackV1, NcInspectionApiV1, NcModuleInfoV1, NcPluginInfoV1};
pub use plugin_registry::{NC_COM_RUNTIME_MAX_LOADED_MODULES, NcPluginRegistryV1};
pub use runtime::NcRuntimeApiV1;
pub use type_registry::NcTypeRegistryV1;

/// Runtime shared-library ABI generation implemented by this release.
pub const NC_COM_RUNTIME_ABI_VERSION: u32 = 1;

/// Borrowed immutable UTF-8 bytes owned by the runtime.
///
/// When `len` is zero, `data` may be null. Otherwise `data` points to exactly
/// `len` readable bytes containing valid UTF-8.
///
/// Unless a narrower lifetime is documented, views returned by runtime
/// inspection calls remain valid until the runtime shared library is unloaded.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcUtf8View {
    /// Pointer to `len` readable UTF-8 bytes, or null when `len` is zero.
    pub data: *const u8,
    /// Number of bytes at `data`.
    pub len: usize,
}

impl NcUtf8View {
    /// Empty UTF-8 view.
    pub const EMPTY: Self = Self { data: core::ptr::null(), len: 0 };
}

// SAFETY: An `NcUtf8View` only provides immutable access. Its producer owns the
// referenced storage and is responsible for the documented lifetime.
unsafe impl Send for NcUtf8View {}
// SAFETY: Shared readers cannot mutate bytes through the const pointer.
unsafe impl Sync for NcUtf8View {}
