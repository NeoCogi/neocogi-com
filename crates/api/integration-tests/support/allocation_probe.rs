//! Thread-local allocation fault injection, isolated from parallel test threads.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Sentinel modes cannot collide with any realistic allocation ordinal.
const OFF: usize = usize::MAX;
/// Observe allocations without denying them.
const OBSERVE: usize = usize::MAX - 1;
thread_local! {
    /// Zero rejects all requests; positive ordinals reject exactly one request.
    static LIMIT: AtomicUsize = const { AtomicUsize::new(OFF) };
    /// Requests observed during this test thread's current probe.
    static ATTEMPTS: AtomicUsize = const { AtomicUsize::new(0) };
    /// Requests denied during this test thread's current probe.
    static DENIED: AtomicUsize = const { AtomicUsize::new(0) };
}

/// Allocator wrapper used only by this regression executable.
struct TestAllocator;
#[global_allocator]
static ALLOCATOR: TestAllocator = TestAllocator;

/// Never allocates or panics, including while thread-local storage is tearing down.
fn deny() -> bool {
    LIMIT
        .try_with(|limit| match limit.load(Relaxed) {
            OFF => false,
            mode => ATTEMPTS
                .try_with(|count| {
                    let ordinal = count.fetch_add(1, Relaxed) + 1;
                    let denied = mode == 0 || mode == ordinal;
                    if denied {
                        let _ = DENIED.try_with(|count| count.fetch_add(1, Relaxed));
                    }
                    denied
                })
                .unwrap_or(false),
        })
        .unwrap_or(false)
}

// SAFETY: successful requests and all deallocations delegate unchanged to System.
// Failure returns null as GlobalAlloc requires; tracking performs no allocation.
unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match deny() {
            true => std::ptr::null_mut(),
            false => unsafe { System.alloc(layout) },
        }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        match deny() {
            true => std::ptr::null_mut(),
            false => unsafe { System.alloc_zeroed(layout) },
        }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        match deny() {
            true => std::ptr::null_mut(),
            false => unsafe { System.realloc(ptr, layout, size) },
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
    }
}

/// Restores normal allocation on every exit, including unwinding.
pub struct Probe;
/// Primitive counters can be inspected after normal allocation is restored.
pub struct Report {
    /// Allocation and reallocation requests made during the probe.
    pub attempts: usize,
    /// Requests for which the allocator returned null.
    pub denied: usize,
}
impl Probe {
    /// Starts observation; zero denies all, None only counts, n rejects request n.
    pub fn start(fail_at: Option<usize>) -> Self {
        LIMIT.with(|limit| assert_eq!(limit.load(Relaxed), OFF, "nested allocation probe"));
        ATTEMPTS.with(|count| count.store(0, Relaxed));
        DENIED.with(|count| count.store(0, Relaxed));
        LIMIT.with(|limit| limit.store(fail_at.unwrap_or(OBSERVE), Relaxed));
        Self
    }
    /// Stops injection before assertions or diagnostic formatting can allocate.
    pub fn finish(self) -> Report {
        drop(self);
        Report {
            attempts: ATTEMPTS.with(|count| count.load(Relaxed)),
            denied: DENIED.with(|count| count.load(Relaxed)),
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        LIMIT.with(|limit| limit.store(OFF, Relaxed));
    }
}

/// Called at implementation entry to catch either side allocating after commit.
#[allow(dead_code)]
pub fn deny_remaining() {
    LIMIT.with(|limit| limit.store(0, Relaxed));
}
