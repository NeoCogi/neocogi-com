//! Exact strong-only lifetime counters for built-in COM providers.
//!
//! Bytes, strings and immutable persistence child lists do not need weak storage.
//! They still obey the generated lifetime engine's ownership rule: every retain
//! that returns has recorded one additional owner. Counter exhaustion must never
//! manufacture an uncounted owner, including in optimized builds.

use core::sync::atomic::{AtomicU32, Ordering};

#[doc(hidden)]
/// Atomically increments a live COM object's reference count.
///
/// Success records exactly one new owner and returns its count. Zero and
/// exhaustion return `None` without modifying storage in every profile. Provider
/// `extern "C"` entry points must terminate on failure before returning an owner;
/// arithmetic itself does not depend on a panic strategy or panic hook.
///
/// The caller must already keep the allocation alive for the whole operation.
/// This arithmetic helper cannot validate a foreign pointer or revive freed
/// storage. Its `u32` is the exact private count, not a capped diagnostic count.
pub fn __com_add_ref_count(ref_count: &AtomicU32) -> Option<u32> {
    let mut current = ref_count.load(Ordering::Relaxed);
    loop {
        if current == 0 || current == u32::MAX {
            return None;
        }
        let next = current + 1;
        // A concurrent retain/release can change the observed count. Retry the
        // invariant check as well as the increment after a failed exchange.
        // Existing ownership keeps storage live; retaining publishes no data.
        match ref_count.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Some(next),
            Err(observed) => current = observed,
        }
    }
}

#[doc(hidden)]
/// Atomically decrements a live COM object's reference count.
///
/// Returns the remaining count, or `None` if the count was already zero. The
/// acquire-release decrement makes prior accesses visible to the thread that
/// performs final destruction.
pub fn __com_release_ref_count(ref_count: &AtomicU32) -> Option<u32> {
    let mut current = ref_count.load(Ordering::Relaxed);
    loop {
        if current == 0 {
            return None;
        }

        let remaining = current.saturating_sub(1);
        // Successful decrements release this reference's prior accesses.
        // The final decrement also acquires the release sequence before drop.
        match ref_count.compare_exchange_weak(current, remaining, Ordering::AcqRel, Ordering::Relaxed) {
            Ok(_) => return Some(remaining),
            Err(observed) => current = observed,
        }
    }
}
