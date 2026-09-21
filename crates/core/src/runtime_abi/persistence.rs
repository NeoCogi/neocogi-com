//! Caller-scoped persistence operations exported by the shared runtime.
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
pub struct NcPersistenceLimitsV1 {
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
pub type NcPersistenceWriteBytesV1 = unsafe extern "C" fn(user_data: *mut c_void, bytes: *const u8, len: usize) -> NcStatus;

/// Size-prefixed persistence table, discovered after the runtime ABI version check.
#[repr(C)]
pub struct NcPersistenceApiV1 {
    /// Available table bytes, including this prefix.
    pub size: usize,
    /// Core interface ABI accepted by the operations below.
    pub core_abi_version: u32,
    /// Creates one empty, thread-local context; `out` must be writable.
    /// Failure clears the output. Successful ownership ends at `destroy_context`.
    pub create_context: unsafe extern "C" fn(out: *mut *mut NcPersistenceContext) -> NcStatus,
    /// Releases every retained factory and frees the context in its own runtime.
    /// Null is accepted; non-null must be exclusively owned with no active calls.
    pub destroy_context: unsafe extern "C" fn(context: *mut NcPersistenceContext),
    /// Registers a live borrowed factory, retaining one reference on success.
    /// Requires exclusive context access; duplicate IDs leave the map unchanged.
    pub register_factory: unsafe extern "C" fn(context: *mut NcPersistenceContext, id: NcGuid, factory: *mut IPersistenceFactoryAbi) -> NcStatus,
    /// Captures a live borrowed object and emits one complete document.
    /// The callback and its data must remain valid until this synchronous call ends.
    pub snapshot: unsafe extern "C" fn(
        object: *mut IUnknown,
        limits: NcPersistenceLimitsV1,
        write: Option<NcPersistenceWriteBytesV1>,
        user_data: *mut c_void,
    ) -> NcStatus,
    /// Restores a complete tree using a shared borrow of a live context.
    /// `bytes` must be readable for `len` bytes (null is valid only for zero).
    /// `out` must be writable and receives one owned reference only on success.
    pub restore: unsafe extern "C" fn(
        context: *const NcPersistenceContext,
        bytes: *const u8,
        len: usize,
        limits: NcPersistenceLimitsV1,
        out: *mut *mut IUnknown,
    ) -> NcStatus,
}

#[cfg(feature = "alloc")]
impl From<crate::persistence::Limits> for NcPersistenceLimitsV1 {
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
impl From<NcPersistenceLimitsV1> for crate::persistence::Limits {
    /// Reconstructs the local policy without changing any caller-selected budget.
    fn from(limits: NcPersistenceLimitsV1) -> Self {
        Self {
            binary: crate::binary::Limits {
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
