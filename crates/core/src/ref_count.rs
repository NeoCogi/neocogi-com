//! Shared lifetime primitives for built-in and generated COM objects.

use core::sync::atomic::{AtomicU32, Ordering};

#[doc(hidden)]
/// Atomically increments a live COM object's reference count.
///
/// The count saturates at `u32::MAX` rather than wrapping to zero. A zero count
/// means the object was already destroyed and is left unchanged.
pub fn __com_add_ref_count(ref_count: &AtomicU32) -> u32 {
    let mut current = ref_count.load(Ordering::Relaxed);
    loop {
        debug_assert!(current > 0, "COM AddRef called after final Release");
        if current == 0 {
            return 0;
        }

        let next = current.saturating_add(1);
        debug_assert!(next > current, "COM reference count overflow");
        if next == current {
            return current;
        }

        match ref_count.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return next,
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
        debug_assert!(current > 0, "COM Release called after final Release");
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
