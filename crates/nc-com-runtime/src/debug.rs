//! Process-global debug messenger storage and dispatch.
//!
//! Messengers are indexed by monotonic handles, filtered by severity/category,
//! snapshotted outside the registry lock for callback delivery, and guarded by
//! per-messenger lifecycle state so destruction waits for in-flight callbacks
//! without deadlocking recursive diagnostic emission.
//!
//! Dispatch snapshots matching registrations while holding the registry mutex,
//! then invokes user code after releasing it. Per-messenger in-flight guards let
//! destruction wait for callback quiescence without holding the registry lock.

use core::ffi::c_void;
use std::sync::{Arc, Condvar, Mutex};
use std::vec::Vec;

use nc_com_core::{
    NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS, NC_DEBUG_SEVERITY_ALL, NC_DEBUG_TYPE_ALL,
    NC_E_INVALID_ARG, NC_E_OUT_OF_RANGE, NC_E_POINTER, NC_OK, NcDebugCallbackV1, NcDebugMessageV1,
    NcDebugMessengerCreateInfoV1, NcStatus,
};

use crate::state::RuntimeState;

/// Immutable callback registration plus synchronized destruction state.
struct Messenger {
    // Handles provide stable removal keys even when the Vec is compacted.
    handle: u64,
    severity_mask: u32,
    message_type_mask: u32,
    callback: NcDebugCallbackV1,
    user_data: *mut c_void,
    lifecycle: Mutex<MessengerLifecycle>,
    idle: Condvar,
}

/// Mutable activity tracked independently for each messenger.
struct MessengerLifecycle {
    /// Whether new callback invocations may begin.
    active: bool,
    /// Number of callbacks that began and have not yet returned.
    in_flight: usize,
}

// SAFETY: `user_data` is opaque and is never dereferenced by the runtime. The
// caller is responsible for making its callback and user data valid on every
// thread selected by the messenger masks.
unsafe impl Send for Messenger {}
// SAFETY: Messenger fields are immutable after creation, lifecycle access is
// synchronized, and the caller guarantees callback/user-data thread safety.
unsafe impl Sync for Messenger {}

impl Messenger {
    /// Tests the registration's severity and category masks.
    fn matches(&self, severity: u32, message_type: u32) -> bool {
        self.severity_mask & severity != 0 && self.message_type_mask & message_type != 0
    }

    /// Marks one callback in flight unless destruction already deactivated it.
    fn begin_callback(&self) -> Option<CallbackGuard<'_>> {
        let mut lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !lifecycle.active {
            return None;
        }
        lifecycle.in_flight += 1;
        Some(CallbackGuard { messenger: self })
    }

    /// Prevents new callbacks and blocks until every current invocation exits.
    fn deactivate_and_wait(&self) {
        let mut lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lifecycle.active = false;
        while lifecycle.in_flight != 0 {
            lifecycle = self
                .idle
                .wait(lifecycle)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

/// RAII token that decrements a messenger's in-flight count on callback exit.
struct CallbackGuard<'a> {
    /// Messenger whose lifecycle counter this invocation owns.
    messenger: &'a Messenger,
}

impl Drop for CallbackGuard<'_> {
    /// Completes one callback and wakes destroyers when the count reaches zero.
    fn drop(&mut self) {
        let mut lifecycle = self
            .messenger
            .lifecycle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lifecycle.in_flight -= 1;
        if lifecycle.in_flight == 0 {
            self.messenger.idle.notify_all();
        }
    }
}

/// Process-owned synchronized debug messenger collection.
pub(crate) struct DebugState {
    /// Handle allocator and active registrations protected as one state.
    registry: Mutex<DebugRegistry>,
}

/// Data protected by [`DebugState::registry`].
struct DebugRegistry {
    /// Next monotonic handle, or zero after permanent exhaustion.
    next_handle: u64,
    /// Active registrations in creation order.
    messengers: Vec<Arc<Messenger>>,
}

impl DebugState {
    /// Creates an empty registry whose first public handle is one.
    pub(crate) fn new() -> Self {
        Self::with_next_handle(1)
    }

    /// Creates an empty registry with a selected next handle for edge-case tests.
    fn with_next_handle(next_handle: u64) -> Self {
        Self {
            registry: Mutex::new(DebugRegistry {
                next_handle,
                messengers: Vec::new(),
            }),
        }
    }

    /// Validates raw creation data, reserves a slot, and publishes a handle.
    unsafe fn create_messenger(
        &self,
        create_info: *const NcDebugMessengerCreateInfoV1,
        out_handle: *mut u64,
    ) -> NcStatus {
        if out_handle.is_null() {
            return NC_E_POINTER;
        }
        // SAFETY: `out_handle` is valid for one write by the caller's contract.
        unsafe { *out_handle = 0 };
        if create_info.is_null() {
            return NC_E_POINTER;
        }
        // SAFETY: `create_info` is readable for one value by the caller's contract.
        let create_info = unsafe { &*create_info };
        let Some(callback) = create_info.callback else {
            return NC_E_INVALID_ARG;
        };
        if create_info.size != core::mem::size_of::<NcDebugMessengerCreateInfoV1>()
            || create_info.severity_mask & !NC_DEBUG_SEVERITY_ALL != 0
            || create_info.message_type_mask & !NC_DEBUG_TYPE_ALL != 0
        {
            return NC_E_INVALID_ARG;
        }

        let mut registry = self.lock_registry();
        if registry.messengers.len() >= NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS {
            return NC_E_OUT_OF_RANGE;
        }
        let handle = registry.next_handle;
        if handle == 0 {
            return NC_E_OUT_OF_RANGE;
        }
        // Zero is the permanent exhausted sentinel. The final nonzero handle
        // remains usable, but no later creation can wrap or reuse an ID.
        registry.next_handle = handle.checked_add(1).unwrap_or(0);
        registry.messengers.push(Arc::new(Messenger {
            handle,
            severity_mask: create_info.severity_mask,
            message_type_mask: create_info.message_type_mask,
            callback,
            user_data: create_info.user_data,
            lifecycle: Mutex::new(MessengerLifecycle {
                active: true,
                in_flight: 0,
            }),
            idle: Condvar::new(),
        }));
        // SAFETY: `out_handle` was checked and remains valid for this call.
        unsafe { *out_handle = handle };
        NC_OK
    }

    /// Deactivates one handle, waits for quiescence, and removes its record.
    fn destroy_messenger(&self, handle: u64) {
        let messenger = {
            let registry = self.lock_registry();
            let Some(messenger) = registry
                .messengers
                .iter()
                .find(|messenger| messenger.handle == handle)
            else {
                return;
            };
            Arc::clone(messenger)
        };

        // Mark inactive before waiting. Snapshots that have not entered their
        // callback will now skip it, while concurrent destroy calls wait on the
        // same lifecycle state.
        messenger.deactivate_and_wait();

        let mut registry = self.lock_registry();
        if let Some(index) = registry
            .messengers
            .iter()
            .position(|registered| Arc::ptr_eq(registered, &messenger))
        {
            registry.messengers.remove(index);
        }
    }

    /// Snapshots matching registrations and invokes them outside the registry lock.
    fn emit(&self, severity: u32, message_type: u32, message: &NcDebugMessageV1) {
        // Snapshotting reference-counted records keeps them alive after releasing
        // the registry lock. The active/in-flight transition below decides
        // whether each snapshotted callback may still begin.
        let callbacks = {
            let registry = self.lock_registry();
            registry
                .messengers
                .iter()
                .filter(|messenger| messenger.matches(severity, message_type))
                .cloned()
                .collect::<Vec<_>>()
        };

        for messenger in callbacks {
            let Some(_guard) = messenger.begin_callback() else {
                continue;
            };
            // SAFETY: The callback and opaque user pointer were supplied by the
            // caller. The in-flight guard keeps their registration active until
            // the callback returns, and the message remains live for the call.
            unsafe { (messenger.callback)(severity, message_type, message, messenger.user_data) };
        }
    }

    /// Acquires the registry mutex while recovering usable state after poisoning.
    fn lock_registry(&self) -> std::sync::MutexGuard<'_, DebugRegistry> {
        self.registry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// C table adapter for creating a process-global debug messenger.
pub(crate) unsafe extern "C" fn create_messenger(
    create_info: *const NcDebugMessengerCreateInfoV1,
    out_handle: *mut u64,
) -> NcStatus {
    // SAFETY: The C caller supplies the create-info and output-pointer contracts.
    unsafe {
        RuntimeState::global()
            .debug()
            .create_messenger(create_info, out_handle)
    }
}

/// C table adapter that destroys a messenger and waits for callback quiescence.
pub(crate) unsafe extern "C" fn destroy_messenger(handle: u64) {
    RuntimeState::global().debug().destroy_messenger(handle);
}

/// Sends a borrowed diagnostic through every matching active messenger.
pub(crate) fn emit(severity: u32, message_type: u32, message: &NcDebugMessageV1) {
    RuntimeState::global()
        .debug()
        .emit(severity, message_type, message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use nc_com_core::{
        NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_TYPE_RUNTIME, NcDebugMessengerCreateInfoV1, NcUtf8View,
    };
    use std::sync::{Arc, Barrier};

    unsafe extern "C" fn count_callback(
        _severity: u32,
        _message_type: u32,
        _message: *const NcDebugMessageV1,
        user_data: *mut c_void,
    ) {
        // SAFETY: The test installs a pointer to a live `AtomicUsize`.
        unsafe { &*(user_data.cast::<AtomicUsize>()) }.fetch_add(1, Ordering::Relaxed);
    }

    unsafe extern "C" fn noop_callback(
        _severity: u32,
        _message_type: u32,
        _message: *const NcDebugMessageV1,
        _user_data: *mut c_void,
    ) {
    }

    #[test]
    fn messenger_filters_and_stops_after_destroy() {
        let debug = DebugState::new();
        let count = AtomicUsize::new(0);
        let info = NcDebugMessengerCreateInfoV1 {
            size: core::mem::size_of::<NcDebugMessengerCreateInfoV1>(),
            severity_mask: NC_DEBUG_SEVERITY_ERROR,
            message_type_mask: NC_DEBUG_TYPE_RUNTIME,
            callback: Some(count_callback),
            user_data: core::ptr::from_ref(&count).cast_mut().cast(),
        };
        let mut handle = 0;
        assert_eq!(unsafe { debug.create_messenger(&info, &mut handle) }, NC_OK);
        assert_ne!(handle, 0);

        let message = NcDebugMessageV1::info(NcUtf8View::EMPTY);
        debug.emit(NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_TYPE_RUNTIME, &message);
        assert_eq!(count.load(Ordering::Relaxed), 1);

        debug.destroy_messenger(handle);
        debug.emit(NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_TYPE_RUNTIME, &message);
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn active_messenger_limit_frees_slots_without_reusing_handles() {
        let debug = DebugState::new();
        let count = AtomicUsize::new(0);
        let info = NcDebugMessengerCreateInfoV1 {
            size: core::mem::size_of::<NcDebugMessengerCreateInfoV1>(),
            severity_mask: NC_DEBUG_SEVERITY_ERROR,
            message_type_mask: NC_DEBUG_TYPE_RUNTIME,
            callback: Some(count_callback),
            user_data: core::ptr::from_ref(&count).cast_mut().cast(),
        };
        let mut handles = Vec::with_capacity(NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS);
        for expected in 1..=NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS as u64 {
            let mut handle = 0;
            assert_eq!(unsafe { debug.create_messenger(&info, &mut handle) }, NC_OK);
            assert_eq!(handle, expected);
            handles.push(handle);
        }

        let mut rejected = 99;
        assert_eq!(
            unsafe { debug.create_messenger(&info, &mut rejected) },
            NC_E_OUT_OF_RANGE
        );
        assert_eq!(rejected, 0);

        debug.destroy_messenger(handles.pop().unwrap());
        let mut replacement = 0;
        assert_eq!(
            unsafe { debug.create_messenger(&info, &mut replacement) },
            NC_OK
        );
        assert_eq!(
            replacement,
            NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS as u64 + 1
        );
    }

    #[test]
    fn messenger_handle_exhaustion_is_permanent_and_never_publishes_zero() {
        let debug = DebugState::with_next_handle(u64::MAX);
        let count = AtomicUsize::new(0);
        let info = NcDebugMessengerCreateInfoV1 {
            size: core::mem::size_of::<NcDebugMessengerCreateInfoV1>(),
            severity_mask: NC_DEBUG_SEVERITY_ERROR,
            message_type_mask: NC_DEBUG_TYPE_RUNTIME,
            callback: Some(count_callback),
            user_data: core::ptr::from_ref(&count).cast_mut().cast(),
        };

        let mut final_handle = 0;
        assert_eq!(
            unsafe { debug.create_messenger(&info, &mut final_handle) },
            NC_OK
        );
        assert_eq!(final_handle, u64::MAX);
        debug.destroy_messenger(final_handle);

        for _ in 0..2 {
            let mut rejected = 99;
            assert_eq!(
                unsafe { debug.create_messenger(&info, &mut rejected) },
                NC_E_OUT_OF_RANGE
            );
            assert_eq!(rejected, 0);
        }
    }

    #[test]
    fn concurrent_messenger_creation_cannot_cross_the_active_limit() {
        const WORKERS: usize = 80;
        let debug = Arc::new(DebugState::new());
        let start = Arc::new(Barrier::new(WORKERS + 1));
        let mut workers = Vec::with_capacity(WORKERS);
        for _ in 0..WORKERS {
            let debug = Arc::clone(&debug);
            let start = Arc::clone(&start);
            workers.push(std::thread::spawn(move || {
                let info = NcDebugMessengerCreateInfoV1 {
                    size: core::mem::size_of::<NcDebugMessengerCreateInfoV1>(),
                    severity_mask: 0,
                    message_type_mask: 0,
                    callback: Some(noop_callback),
                    user_data: core::ptr::null_mut(),
                };
                let mut handle = 0;
                start.wait();
                let status = unsafe { debug.create_messenger(&info, &mut handle) };
                (status, handle)
            }));
        }
        start.wait();

        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        let mut handles = results
            .iter()
            .filter_map(|(status, handle)| (*status == NC_OK).then_some(*handle))
            .collect::<Vec<_>>();
        handles.sort_unstable();
        assert_eq!(
            handles,
            (1..=NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS as u64).collect::<Vec<_>>()
        );
        assert!(results.iter().all(|(status, handle)| {
            (*status == NC_OK && *handle != 0) || (*status == NC_E_OUT_OF_RANGE && *handle == 0)
        }));
    }

    struct BlockingCallback {
        entered: Barrier,
        release: Barrier,
    }

    unsafe extern "C" fn blocking_callback(
        _severity: u32,
        _message_type: u32,
        _message: *const NcDebugMessageV1,
        user_data: *mut c_void,
    ) {
        // SAFETY: The test retains the `Arc` that owns this value until all
        // callback and destruction threads have joined.
        let state = unsafe { &*user_data.cast::<BlockingCallback>() };
        state.entered.wait();
        state.release.wait();
    }

    #[test]
    fn destroy_waits_for_in_flight_callback() {
        let debug = Arc::new(DebugState::new());
        let callback = Arc::new(BlockingCallback {
            entered: Barrier::new(2),
            release: Barrier::new(2),
        });
        let info = NcDebugMessengerCreateInfoV1 {
            size: core::mem::size_of::<NcDebugMessengerCreateInfoV1>(),
            severity_mask: NC_DEBUG_SEVERITY_ERROR,
            message_type_mask: NC_DEBUG_TYPE_RUNTIME,
            callback: Some(blocking_callback),
            user_data: Arc::as_ptr(&callback).cast_mut().cast(),
        };
        let mut handle = 0;
        assert_eq!(unsafe { debug.create_messenger(&info, &mut handle) }, NC_OK);

        let emitting = {
            let debug = Arc::clone(&debug);
            std::thread::spawn(move || {
                let message = NcDebugMessageV1::info(NcUtf8View::EMPTY);
                debug.emit(NC_DEBUG_SEVERITY_ERROR, NC_DEBUG_TYPE_RUNTIME, &message);
            })
        };
        callback.entered.wait();

        let destroy_returned = Arc::new(AtomicBool::new(false));
        let destroying = {
            let debug = Arc::clone(&debug);
            let destroy_returned = Arc::clone(&destroy_returned);
            std::thread::spawn(move || {
                debug.destroy_messenger(handle);
                destroy_returned.store(true, Ordering::Release);
            })
        };

        loop {
            let messenger = {
                let registry = debug.lock_registry();
                Arc::clone(&registry.messengers[0])
            };
            let active = messenger
                .lifecycle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .active;
            if !active {
                break;
            }
            std::thread::yield_now();
        }
        assert!(!destroy_returned.load(Ordering::Acquire));

        callback.release.wait();
        emitting.join().unwrap();
        destroying.join().unwrap();
        assert!(destroy_returned.load(Ordering::Acquire));
    }
}
