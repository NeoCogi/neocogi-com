//! Safe Rust ownership over the linked runtime's persistence operation table.

use core::{
    ffi::c_void,
    mem,
    ptr::{self, NonNull},
    slice,
};
use std::vec::Vec;
use neocogi_com_core::{NcPersistenceApiV1, NcPersistenceContext};
use crate::runtime::ffi as runtime_sys;
use crate::{
    ComPtr, ComRef, IUnknown, NcGuid, NcStatus, NC_COM_CORE_ABI_VERSION, NC_COM_RUNTIME_ABI_VERSION, NC_E_ABI_VERSION, NC_E_INVALID_ARG, NC_E_OUT_OF_MEMORY,
    NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_OK,
};
use super::{IPersistenceFactoryAbi, Limits};

/// An explicitly owned, thread-local set of reconstruction factories.
///
/// Storage lives in the linked runtime and is released by that same runtime.
/// Registration requires `&mut self`; restore only borrows the registry. The
/// opaque non-null pointer deliberately makes this owner neither Send nor Sync,
/// matching the thread-local persistence provider contracts. Creating one does
/// not modify the process-global plugin registry or load any provider libraries.
pub struct FactoryRegistry {
    /// Unique context owner, never exposed as a dereferenceable Rust value.
    context: NonNull<NcPersistenceContext>,
    /// Checked immutable table retained for the context's complete lifetime.
    api: &'static NcPersistenceApiV1,
}

impl FactoryRegistry {
    /// Creates an empty context after checking both runtime and core ABI versions.
    pub fn new() -> Result<Self, NcStatus> {
        let api = persistence_api()?;
        let mut context = ptr::null_mut();
        // SAFETY: The checked table receives a valid local output slot.
        let status = unsafe { (api.create_context)(&mut context) };
        if status.is_err() {
            return Err(status);
        }
        let context = NonNull::new(context).ok_or(NC_E_POINTER)?;
        Ok(Self { context, api })
    }

    /// Retains one factory under a recipe ID; duplicates preserve the first entry.
    pub fn register(&mut self, id: NcGuid, factory: ComPtr<IPersistenceFactoryAbi>) -> Result<(), NcStatus> {
        // SAFETY: &mut self excludes competing operations, and factory lends a
        // live interface until the runtime has retained its own reference.
        let status = unsafe { (self.api.register_factory)(self.context.as_ptr(), id, factory.as_raw()) };
        if status.is_ok() { Ok(()) } else { Err(status) }
    }

    /// Restores a complete owned tree using the default finite resource policy.
    pub fn restore(&self, bytes: &[u8]) -> Result<ComPtr<IUnknown>, NcStatus> {
        self.restore_with_limits(bytes, Limits::default())
    }

    /// Validates every recipe and state before invoking any construction callback.
    pub fn restore_with_limits(&self, bytes: &[u8], limits: Limits) -> Result<ComPtr<IUnknown>, NcStatus> {
        let mut root = crate::ComRaw::null();
        // SAFETY: The context, input slice and output slot all live through the
        // synchronous call. The returned root transfers one COM reference.
        let status = unsafe { (self.api.restore)(self.context.as_ptr(), bytes.as_ptr(), bytes.len(), limits.into(), &mut root) };
        if status.is_err() {
            return Err(status);
        }
        // SAFETY: Successful restoration promises an owned common COM prefix;
        // the checked adoption verifies identity before exposing a safe owner.
        unsafe { ComPtr::__nc_from_raw_owned_checked(root) }
    }
}

impl Drop for FactoryRegistry {
    /// Returns the context and retained factories to their allocating runtime.
    fn drop(&mut self) {
        // SAFETY: This non-Clone owner is exclusive at destruction, and its
        // non-Send pointer keeps release on the context's creating thread.
        unsafe { (self.api.destroy_context)(self.context.as_ptr()) };
    }
}

/// Captures a shared opt-in object tree with the default limits.
/// A borrowed exclusive root returns `NC_E_INVALID_ARG`; its sole owner is
/// preserved. Supporting exclusive trees requires a separate observation protocol.
pub fn snapshot(object: ComRef<'_, IUnknown>) -> Result<Vec<u8>, NcStatus> {
    snapshot_with_limits(object, Limits::default())
}

/// Returns caller-owned document bytes after one successful shared-tree traversal.
/// Non-shareable roots fail before persistence getters or byte delivery run.
pub fn snapshot_with_limits(object: ComRef<'_, IUnknown>, limits: Limits) -> Result<Vec<u8>, NcStatus> {
    let api = persistence_api()?;
    let mut destination = SnapshotDestination {
        maximum: limits.binary.bytes,
        bytes: None,
    };
    // SAFETY: Object remains borrowed and destination stays at this address for
    // the whole synchronous operation. The runtime never retains either pointer.
    let status = unsafe { (api.snapshot)(object.as_raw(), limits.into(), Some(receive_snapshot), ptr::from_mut(&mut destination).cast()) };
    if status.is_err() {
        return Err(status);
    }
    destination.bytes.ok_or(NC_E_INVALID_ARG)
}

/// Typed state for a single synchronous byte-delivery callback.
struct SnapshotDestination {
    /// Maximum complete document size accepted by this caller.
    maximum: usize,
    /// None until the one successful delivery; Some rejects repeated delivery.
    bytes: Option<Vec<u8>>,
}

/// Copies borrowed runtime bytes using the caller's allocator exactly once.
///
/// # Safety
/// user_data points to the live exclusive SnapshotDestination in snapshot_with_limits;
/// nonempty input is readable for len bytes and does not overlap that destination.
unsafe extern "C" fn receive_snapshot(user_data: *mut c_void, bytes: *const u8, len: usize) -> NcStatus {
    if user_data.is_null() || (len != 0 && bytes.is_null()) {
        return NC_E_POINTER;
    }
    // SAFETY: Only snapshot_with_limits installs this callback/data pair.
    let destination = unsafe { &mut *user_data.cast::<SnapshotDestination>() };
    // Model delivery as a one-way transition: an existing result is immutable.
    match destination.bytes {
        Some(_) => NC_E_INVALID_ARG,
        None => {
            if len > destination.maximum || len > isize::MAX as usize {
                return NC_E_OUT_OF_RANGE;
            }
            let mut output = Vec::new();
            if output.try_reserve_exact(len).is_err() {
                return NC_E_OUT_OF_MEMORY;
            }
            let input = match len {
                0 => &[],
                // SAFETY: The runtime lends readable bytes and the length was checked.
                _ => unsafe { slice::from_raw_parts(bytes, len) },
            };
            // Publish the completed copy only after all fallible allocation has finished.
            output.extend_from_slice(input);
            destination.bytes = Some(output);
            NC_OK
        }
    }
}

/// Checks mandatory ABI discovery before reading the size-prefixed operation table.
fn persistence_api() -> Result<&'static NcPersistenceApiV1, NcStatus> {
    // SAFETY: The unversioned discovery symbol is callable before table lookup.
    if unsafe { runtime_sys::nc_com_runtime_abi_version() } != NC_COM_RUNTIME_ABI_VERSION {
        return Err(NC_E_ABI_VERSION);
    }
    // SAFETY: A matching generation guarantees a static size-prefixed v1 table.
    let api = unsafe { runtime_sys::nc_com_get_persistence_api_v1() };
    if api.is_null() {
        return Err(NC_E_ABI_VERSION);
    }
    // SAFETY: Read only the common prefix before creating a full table reference.
    if unsafe { api.cast::<usize>().read() } < mem::size_of::<NcPersistenceApiV1>() {
        return Err(NC_E_ABI_VERSION);
    }
    // SAFETY: The advertised prefix now includes every function this client uses.
    let api = unsafe { &*api };
    match api.core_abi_version {
        NC_COM_CORE_ABI_VERSION => Ok(api),
        _ => Err(NC_E_ABI_VERSION),
    }
}
