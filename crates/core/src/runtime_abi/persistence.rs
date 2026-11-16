//! Caller-scoped persistence operations supplied by the host runtime.
//!
//! Contexts belong to their creating runtime and thread. Foreign callers must
//! keep them alive for each call, exclusively borrow them for registration and
//! destruction, and never destroy or mutate them during a restore callback.
//! No Rust container layout or allocator ownership crosses this boundary.

use core::ffi::c_void;
use crate::{IUnknown, NcGuid, NcStatus, persistence::IPersistenceFactoryAbi};

/// Opaque, runtime-owned recipe registry; callers may only pass its pointer back.
#[repr(C)]
pub struct NcPersistenceContext {
    /// Prevents consumers from inspecting or constructing runtime storage.
    _private: [u8; 0],
}

/// Fixed C representation of the complete persistence resource policy.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NcPersistenceLimitsV3 {
    /// Maximum encoded document bytes and aggregate captured provider state.
    pub bytes: usize,
    /// Maximum binary elements or fields in an individual container.
    pub elements: usize,
    /// Maximum binary value/container nesting.
    pub value_depth: usize,
    /// Maximum total decoded binary values.
    pub values: usize,
    /// Maximum objects across the complete initialization tree.
    pub objects: usize,
    /// Maximum objects along one ownership path.
    pub object_depth: usize,
}

/// Receives one complete encoded snapshot synchronously, after successful capture.
///
/// The byte range is borrowed until this callback returns. The callback may copy
/// it into its own allocator, must not unwind, and returns a failure status if
/// it cannot accept the document. The runtime calls it exactly once on success
/// and never retains `user_data`. A failed capture does not call it at all.
pub type NcPersistenceWriteBytesV3 = unsafe extern "C" fn(user_data: *mut c_void, bytes: *const u8, len: usize) -> NcStatus;

/// Size-prefixed persistence table admitted through the host API description.
///
/// Table admission establishes ABI v3/core-generation-21 compatibility,
/// not serialization-provider availability. Callers still owe the host API's
/// backing and lifetime guarantees. The application must explicitly
/// register or load `nc.serialization` in this host's catalog before snapshot
/// or restore. Each operation acquires its root and requests named `ncb`;
/// the provider's default format is irrelevant. No codec is registered or loaded
/// implicitly, and no embedded codec is used as a fallback.
///
/// A missing provider returns [`crate::NC_E_NOT_FOUND`]. Root acquisition and
/// named-format selection errors propagate; encoding/decoding errors after
/// selection map to [`crate::NC_E_INVALID_ARG`]. Neither provider absence nor a
/// failed selection poisons a context: once the cause is resolved, the caller
/// may retry with the same live context. This does not authorize replacing a
/// registered plugin descriptor. NCB-v1 bytes and the table layout do
/// not depend on how the application supplies that provider.
#[repr(C)]
pub struct NcPersistenceApiV3 {
    /// Available table bytes, including this prefix.
    pub size: usize,
    /// Core interface ABI accepted by the operations below.
    pub core_abi_version: u32,
    /// Creates one empty, thread-local context; `out` must be writable.
    /// Failure clears the output. Successful ownership ends at `destroy_context`.
    /// Does not require or register a serialization provider.
    pub create_context: unsafe extern "C" fn(out: *mut *mut NcPersistenceContext) -> NcStatus,
    /// Releases every retained factory and frees the context in its own runtime.
    /// Null is accepted; non-null must be exclusively owned with no active calls.
    pub destroy_context: unsafe extern "C" fn(context: *mut NcPersistenceContext),
    /// Registers a live borrowed factory, retaining one reference on success.
    /// Requires exclusive context access; duplicate IDs leave the map unchanged.
    /// Does not require or register a serialization provider.
    pub register_factory: unsafe extern "C" fn(context: *mut NcPersistenceContext, id: NcGuid, factory: crate::ComRaw<IPersistenceFactoryAbi>) -> NcStatus,
    /// Captures a live borrowed object and emits one complete document.
    /// The callback and its data must remain valid until this synchronous call ends.
    /// Argument, ownership, capability and capture-budget checks precede codec
    /// selection. Provider/selection failure never calls the byte callback.
    pub snapshot: unsafe extern "C" fn(
        object: crate::ComRaw<IUnknown>,
        limits: NcPersistenceLimitsV3,
        write: Option<NcPersistenceWriteBytesV3>,
        user_data: *mut c_void,
    ) -> NcStatus,
    /// Restores a complete tree using a shared borrow of a live context.
    /// `bytes` must be readable for `len` bytes (null is valid only for zero).
    /// `out` must be writable and receives one owned reference only on success.
    /// Clears `out` and checks arguments/input-byte bounds before selecting NCB.
    /// Provider/selection failure runs no recipe validation or construction.
    pub restore: unsafe extern "C" fn(
        context: *const NcPersistenceContext,
        bytes: *const u8,
        len: usize,
        limits: NcPersistenceLimitsV3,
        out: *mut crate::ComRaw<IUnknown>,
    ) -> NcStatus,
}

#[cfg(feature = "alloc")]
impl From<crate::persistence::Limits> for NcPersistenceLimitsV3 {
    /// Lowers every local limit explicitly instead of depending on Rust layout.
    fn from(limits: crate::persistence::Limits) -> Self {
        Self {
            bytes: limits.binary.bytes,
            elements: limits.binary.elements,
            value_depth: limits.binary.depth,
            values: limits.binary.values,
            objects: limits.objects,
            object_depth: limits.depth,
        }
    }
}

#[cfg(feature = "alloc")]
impl From<NcPersistenceLimitsV3> for crate::persistence::Limits {
    /// Reconstructs the local policy without changing any caller-selected budget.
    fn from(limits: NcPersistenceLimitsV3) -> Self {
        Self {
            binary: crate::serialization::Limits {
                bytes: limits.bytes,
                elements: limits.elements,
                depth: limits.value_depth,
                values: limits.values,
            },
            objects: limits.objects,
            depth: limits.object_depth,
        }
    }
}
