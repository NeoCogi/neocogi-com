//! Versioned ABI tables supplied by the host's process-global runtime owner.
//!
//! This module groups plugin operations, diagnostics and persistence contracts while keeping their `repr(C)` layouts in the allocation-free
//! core crate. Every table begins with a byte size checked by consumers. This
//! runtime-family-1 discovery uses the separately checked core generation to
//! select compatible frozen layouts; deploy all roles from one SDK release.

mod debug;
mod host;
mod persistence;
mod plugin;
mod runtime;
pub use persistence::{NcPersistenceApiV1, NcPersistenceContext, NcPersistenceLimitsV1, NcPersistenceWriteBytesV1};

pub use debug::{
    NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS, NC_DEBUG_SEVERITY_ALL, NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_SEVERITY_INFO, NC_DEBUG_SEVERITY_WARNING,
    NC_DEBUG_TYPE_ALL, NC_DEBUG_TYPE_PLUGIN, NC_DEBUG_TYPE_RUNTIME, NC_DEBUG_TYPE_VALIDATION, NcDebugApiV1, NcDebugCallbackV1, NcDebugMessageV1,
    NcDebugMessengerCreateInfoV1,
};
pub use plugin::{NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS, NcFactoryInfoV1, NcInspectPluginDescriptorCallbackV1, NcPluginInfoV1};
pub use runtime::NcRuntimeApiV1;
pub use host::{NcHostApiV1, validate_host_api};

/// Pre-release host-service ABI generation.
///
/// Version one identifies this pre-release family, not cross-revision binary
/// compatibility. Contracts may change in place; all roles must be rebuilt from
/// the same checkout. Plugin provenance and the unified operation table replace
/// earlier runtime layouts without a new generation or a compatibility path.
pub const NC_COM_RUNTIME_ABI_VERSION: u32 = 1;

/// Borrowed immutable UTF-8 bytes owned by the runtime.
///
/// When `len` is zero, `data` may be null. Otherwise `data` points to exactly
/// `len` readable bytes containing valid UTF-8.
///
/// Unless a narrower lifetime is documented, views returned by runtime
/// inspection calls remain valid for the process/Wasm-instance lifetime. The
/// host retains its runtime state and process-pins opened provider mappings.
/// The view itself owns no storage and encodes no Rust borrow lifetime.
/// Its public API consists of the raw fields and [`Self::EMPTY`]; it has no
/// `as_str` or `as_bytes` method. The COM facade validates and copies inspection
/// text into owned `String` values before returning it to application code.
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
