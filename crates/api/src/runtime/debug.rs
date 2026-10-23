//! Safe ownership for runtime debug messengers.
//!
//! The callback itself remains an unsafe C contract because the runtime cannot
//! prove the lifetime or thread-safety of opaque user data. The returned
//! `DebugMessenger` does provide safe single-owner destruction through `Drop`.

use crate::{NC_E_INVALID_ARG, NcDebugApiV1, NcDebugMessengerCreateInfoV1, NcStatus};

use super::{Runtime, status_to_result};

/// Callback-based runtime diagnostics.
#[derive(Clone, Copy, Debug)]
pub struct Debug<'runtime> {
    /// Supplied access to the already admitted debug ABI table.
    runtime: &'runtime Runtime,
}

impl<'runtime> Debug<'runtime> {
    /// Creates a debug context borrowing the supplied host access.
    pub(super) const fn new(runtime: &'runtime Runtime) -> Self {
        Self { runtime }
    }

    /// Creates a messenger that remains registered until its owner is dropped.
    ///
    /// At most [`crate::NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS`] messengers
    /// may be registered simultaneously. This returns `NC_E_OUT_OF_RANGE` when
    /// that capacity or the process-local handle space is exhausted. Dropping a
    /// messenger frees one active slot, but handles are never reused.
    ///
    /// # Safety
    ///
    /// The callback and `user_data` in `create_info` must remain valid for all
    /// matching calls until the returned messenger is dropped. Debug callbacks
    /// must not drop their own messenger or otherwise destroy a debug messenger.
    pub unsafe fn create_messenger(self, create_info: &NcDebugMessengerCreateInfoV1) -> Result<DebugMessenger, NcStatus> {
        let api = self.runtime.debug_api();
        let mut handle = 0;
        // SAFETY: The caller guarantees callback/user-data validity, and the
        // output slot is valid for one handle write.
        let status = unsafe { (api.create_messenger)(create_info, &mut handle) };
        status_to_result(status)?;
        if handle == 0 {
            return Err(NC_E_INVALID_ARG);
        }
        Ok(DebugMessenger { api, handle })
    }
}

/// Owned registration for one runtime debug callback.
pub struct DebugMessenger {
    /// Exact table used for creation, retained so destruction stays paired with
    /// the same runtime generation and needs no later facade lookup.
    api: &'static NcDebugApiV1,
    /// Nonzero process-local registration handle owned by this value.
    handle: u64,
}

impl core::fmt::Debug for DebugMessenger {
    /// Formats the diagnostic handle without exposing the raw API table.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DebugMessenger").field("handle", &self.handle).finish_non_exhaustive()
    }
}

impl DebugMessenger {
    /// Returns the process-local debug messenger handle.
    pub const fn handle(&self) -> u64 {
        self.handle
    }
}

impl Drop for DebugMessenger {
    /// Unregisters the callback and waits for its in-flight calls to finish.
    fn drop(&mut self) {
        // Runtime ABI v1 waits for in-flight callbacks before returning. The
        // unsafe creation contract forbids dropping a messenger from a callback,
        // which would otherwise wait for that callback itself.
        // SAFETY: This object owns the nonzero handle and destroys it once.
        unsafe { (self.api.destroy_messenger)(self.handle) };
    }
}
