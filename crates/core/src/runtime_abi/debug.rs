//! Callback-based diagnostics modeled after explicit debug messenger APIs.
//!
//! The ABI records define severity and category masks, borrowed diagnostic
//! messages, messenger creation parameters, and the lifecycle function table.
//! Storage and synchronized callback dispatch are implemented by the shared
//! runtime rather than this layout-only core crate.
//!
//! A messenger is selected by severity and message-type bitmasks. Message text
//! and related IDs are borrowed for the callback only; the callback must copy
//! anything it wants to retain. Messenger handles are process-local ownership
//! tokens, not stable ABI identities.

use core::ffi::c_void;

use super::NcUtf8View;
use crate::{NC_OK, NcGuid, NcInterfaceId, NcStatus};

/// Maximum number of runtime debug messengers registered simultaneously.
pub const NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS: usize = 64;

/// Informational diagnostic severity bit.
pub const NC_DEBUG_SEVERITY_INFO: u32 = 1 << 0;
/// Warning diagnostic severity bit.
pub const NC_DEBUG_SEVERITY_WARNING: u32 = 1 << 1;
/// Error diagnostic severity bit.
pub const NC_DEBUG_SEVERITY_ERROR: u32 = 1 << 2;
/// All diagnostic severity bits defined by runtime ABI v1.
pub const NC_DEBUG_SEVERITY_ALL: u32 = NC_DEBUG_SEVERITY_INFO | NC_DEBUG_SEVERITY_WARNING | NC_DEBUG_SEVERITY_ERROR;

/// General runtime diagnostic type bit.
pub const NC_DEBUG_TYPE_RUNTIME: u32 = 1 << 0;
/// Plugin loading and registration diagnostic type bit.
pub const NC_DEBUG_TYPE_PLUGIN: u32 = 1 << 1;
/// Registry validation diagnostic type bit.
pub const NC_DEBUG_TYPE_VALIDATION: u32 = 1 << 2;
/// All diagnostic type bits defined by runtime ABI v1.
pub const NC_DEBUG_TYPE_ALL: u32 = NC_DEBUG_TYPE_RUNTIME | NC_DEBUG_TYPE_PLUGIN | NC_DEBUG_TYPE_VALIDATION;

/// One diagnostic delivered to an [`NcDebugCallbackV1`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NcDebugMessageV1 {
    /// Status associated with the diagnostic, or [`NC_OK`] for information.
    pub status: NcStatus,
    /// Human-readable UTF-8 message, valid only for the callback invocation.
    pub message: NcUtf8View,
    /// Related plugin ID, or the nil GUID when not applicable.
    pub plugin_id: NcGuid,
    /// Retained class-correlation ABI slot; current factory-only runtime
    /// diagnostics leave it nil because there is no class registry.
    pub class_id: NcGuid,
    /// Related interface ID, or [`NcInterfaceId::NONE`] when not applicable.
    pub interface_id: NcInterfaceId,
    /// Related loaded-module ID, or zero when not applicable.
    pub module_id: u64,
}

impl NcDebugMessageV1 {
    /// Creates a diagnostic with no related registry identities.
    pub const fn new(status: NcStatus, message: NcUtf8View) -> Self {
        Self {
            status,
            message,
            plugin_id: NcGuid::nil(),
            class_id: NcGuid::nil(),
            interface_id: NcInterfaceId::NONE,
            module_id: 0,
        }
    }

    /// Creates an informational diagnostic with no related identities.
    pub const fn info(message: NcUtf8View) -> Self {
        Self::new(NC_OK, message)
    }
}

/// Runtime debug callback for runtime ABI v1.
///
/// The runtime supplies a non-null message pointer that remains valid only for
/// this call. Callbacks may run concurrently on any thread and must not destroy
/// a debug messenger.
pub type NcDebugCallbackV1 = unsafe extern "C" fn(severity: u32, value_type: u32, message: *const NcDebugMessageV1, user_data: *mut c_void);

/// Configuration used to create one runtime debug messenger.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcDebugMessengerCreateInfoV1 {
    /// Must be `size_of::<NcDebugMessengerCreateInfoV1>()`.
    pub size: usize,
    /// Bitwise OR of `NC_DEBUG_SEVERITY_*` values.
    pub severity_mask: u32,
    /// Bitwise OR of `NC_DEBUG_TYPE_*` values.
    pub value_type_mask: u32,
    /// Callback invoked for matching diagnostics; null is invalid.
    pub callback: Option<NcDebugCallbackV1>,
    /// Opaque caller data passed to `callback`.
    pub user_data: *mut c_void,
}

/// Callback-based runtime diagnostics for runtime ABI v1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NcDebugApiV1 {
    /// Size of this table in bytes.
    pub size: usize,
    /// Creates a debug messenger and writes its nonzero process-local handle.
    ///
    /// Both pointers are required. When `out_handle` is non-null, it is cleared
    /// before create-info validation. Returns `NC_E_OUT_OF_RANGE` when
    /// [`NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS`] are already registered or
    /// the process-local handle space is permanently exhausted.
    pub create_messenger: unsafe extern "C" fn(create_info: *const NcDebugMessengerCreateInfoV1, out_handle: *mut u64) -> NcStatus,
    /// Destroys a messenger and waits for its in-flight callbacks to return.
    ///
    /// Zero and unknown handles are ignored. This function must not be called
    /// from a debug callback.
    pub destroy_messenger: unsafe extern "C" fn(handle: u64),
}
