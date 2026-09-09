//! Reference-count primitives and object hooks used by generated COM classes.
//!
//! Generated class storage embeds an atomic strong count and one header per
//! implemented interface. The helpers here implement saturating `AddRef` and
//! non-underflowing `Release`; the unsafe traits describe how generated thunks
//! recover the concrete Rust value from a header's opaque back-pointer, perform
//! interface negotiation, and destroy storage on the final release. They are
//! public only as hidden macro support and are not the ordinary end-user API.

use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use neocogi_com_core::{NC_E_BUSY, NcInterfaceId, NcStatus, ThreadSafe, ThreadingModel};

/// Sentinel held while one generated COM method has exclusive object access.
const EXCLUSIVE_CALL: usize = usize::MAX;

#[doc(hidden)]
/// Per-object borrow gate used by generated interface thunks.
///
/// Shared calls may overlap. A mutable call succeeds only while no other call
/// is active, which prevents cloned COM pointers or foreign reentry from
/// manufacturing overlapping Rust borrows of the stored implementation.
pub struct __ComCallGate {
    /// Zero means idle, `EXCLUSIVE_CALL` means mutable, and other values count readers.
    state: AtomicUsize,
}

impl __ComCallGate {
    /// Creates an idle gate for a newly allocated COM object.
    pub const fn new() -> Self {
        Self { state: AtomicUsize::new(0) }
    }

    /// Runs one shared call, or reports contention with an active mutable call.
    pub fn shared<R>(&self, call: impl FnOnce() -> R) -> Result<R, NcStatus> {
        let guard = self.try_shared()?;
        let result = call();
        drop(guard);
        Ok(result)
    }

    /// Runs one exclusive call, or reports contention with any active call.
    pub fn exclusive<R>(&self, call: impl FnOnce() -> R) -> Result<R, NcStatus> {
        let guard = self.try_exclusive()?;
        let result = call();
        drop(guard);
        Ok(result)
    }

    /// Acquires one reader slot without blocking or permitting counter overflow.
    fn try_shared(&self) -> Result<SharedCall<'_>, NcStatus> {
        let mut current = self.state.load(Ordering::Acquire);
        loop {
            if current == EXCLUSIVE_CALL || current == EXCLUSIVE_CALL - 1 {
                return Err(NC_E_BUSY);
            }
            match self.state.compare_exchange_weak(current, current + 1, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => return Ok(SharedCall { gate: self }),
                Err(observed) => current = observed,
            }
        }
    }

    /// Acquires the unique writer sentinel without waiting on active readers.
    fn try_exclusive(&self) -> Result<ExclusiveCall<'_>, NcStatus> {
        self.state
            .compare_exchange(0, EXCLUSIVE_CALL, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| ExclusiveCall { gate: self })
            .map_err(|_| NC_E_BUSY)
    }
}

impl Default for __ComCallGate {
    /// Creates the same idle gate as [`__ComCallGate::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Active shared-call lease that restores its reader slot on every return path.
struct SharedCall<'a> {
    /// Gate whose reader count this lease owns.
    gate: &'a __ComCallGate,
}

impl Drop for SharedCall<'_> {
    /// Releases exactly one reader slot after the delegated Rust call ends.
    fn drop(&mut self) {
        let previous = self.gate.state.fetch_sub(1, Ordering::Release);
        debug_assert!(previous > 0 && previous != EXCLUSIVE_CALL);
    }
}

/// Active mutable-call lease that restores the idle state on every return path.
struct ExclusiveCall<'a> {
    /// Gate whose exclusive sentinel this lease owns.
    gate: &'a __ComCallGate,
}

impl Drop for ExclusiveCall<'_> {
    /// Releases the exclusive sentinel after the delegated Rust call ends.
    fn drop(&mut self) {
        let previous = self.gate.state.swap(0, Ordering::Release);
        debug_assert_eq!(previous, EXCLUSIVE_CALL);
    }
}

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

/// Implemented by generated COM class storage.
///
/// # Safety
///
/// Implementors must map opaque generated storage pointers back to the exact
/// allocation and Rust value that produced them. Reference-count operations must
/// keep the allocation alive until the final `Release`, and `QueryInterface`
/// must only return interface headers owned by that allocation.
///
/// Interface thunks receive an opaque object pointer and use this trait to
/// recover the Rust implementation and delegate IUnknown behavior.
pub unsafe trait ComAbiObject: Sized {
    /// Runs a closure with shared access to the Rust value in generated storage.
    ///
    /// # Safety
    ///
    /// `object` must be an opaque storage pointer originally produced by the
    /// matching generated `#[com_object]` implementation.
    unsafe fn with_abi_ref<R>(object: *mut c_void, call: impl FnOnce(&Self) -> R) -> Result<R, NcStatus>;

    /// Runs a closure with exclusive access to the Rust value in generated storage.
    ///
    /// # Safety
    ///
    /// `object` must be an opaque storage pointer originally produced by the
    /// matching generated `#[com_object]` implementation.
    unsafe fn with_abi_mut<R>(object: *mut c_void, call: impl FnOnce(&mut Self) -> R) -> Result<R, NcStatus>;

    /// Implements `QueryInterface` for generated interface thunks.
    ///
    /// # Safety
    ///
    /// `object` must be valid generated storage, `interface_id` must point to a
    /// readable initialized identity, and `out` must point to a distinct slot
    /// valid for one pointer write. Given those preconditions, `NC_OK` writes
    /// one non-null owned interface reference to `*out`; every other returned
    /// status leaves `*out` null and transfers no reference. Null or otherwise
    /// invalid arguments violate this safety contract even when an
    /// implementation rejects them defensively.
    unsafe fn query_interface_from_abi_object(object: *mut c_void, interface_id: *const NcInterfaceId, out: *mut *mut c_void) -> NcStatus;

    /// Increments the reference count for generated object storage.
    ///
    /// The returned count is diagnostic only. Debug builds assert if the count
    /// would overflow; release builds saturate at `u32::MAX` instead of
    /// wrapping.
    ///
    /// # Safety
    ///
    /// `object` must be valid generated storage or null.
    unsafe fn add_ref_from_abi_object(object: *mut c_void) -> u32;

    /// Decrements the reference count for generated object storage.
    ///
    /// # Safety
    ///
    /// `object` must be valid generated storage or null. When the returned count
    /// reaches zero, the implementation must destroy the allocation immediately.
    /// Final release cannot report fallible cleanup errors, cannot defer
    /// destruction, and must not resurrect the object. Debug builds assert if
    /// `Release` observes an already-zero count; release builds refuse to
    /// underflow.
    unsafe fn release_from_abi_object(object: *mut c_void) -> u32;
}

/// Associates a generated COM implementation class with the strongest
/// threading capability supported by pointers to its object storage.
///
/// The generated class constructors, descriptor metadata, and checked
/// constructor thunk all derive their threading mode from this associated type.
/// A [`ThreadSafe`] class may still be exposed through a conservative
/// thread-local pointer; the associated type records the maximum capability the
/// class is allowed to produce.
///
/// | `Threading` | Generated owner | Descriptor value | Caller permission |
/// |---|---|---|---|
/// | [`crate::ThreadLocal`] | `ComPtr<_, ThreadLocal>` | `THREAD_LOCAL` | Current-thread use only |
/// | [`ThreadSafe`] | `ComPtr<_, ThreadSafe>` | `THREAD_SAFE` | Cross-thread movement, sharing, calls, and final release |
///
/// This associated type cannot by itself control a pointer forever. Once a
/// class is exposed through one of its interfaces, the concrete class type is
/// erased and several classes with different threading models may implement
/// that same interface. The `ComPtr` or `ComRef` threading parameter carries
/// this class promise into the erased handle type, where Rust can enforce
/// `Send` and `Sync`. The class associated type and pointer marker are therefore
/// the producer and consumer halves of one contract, not duplicate state.
///
/// # Safety
///
/// `Threading` must describe every interface header backed by this class's
/// generated object storage. Selecting [`ThreadSafe`] promises that all exposed
/// methods may be called concurrently from arbitrary threads and that final
/// destruction may occur on any thread. Implementations must also keep any
/// published [`crate::NcClassDescriptor::threading`] value consistent with this
/// associated type.
pub unsafe trait ComClass: ComAbiObject {
    /// Strongest threading capability supported by this class's COM objects.
    type Threading: ThreadingModel;
}

/// Marker for generated COM objects whose method implementations are safe to
/// call concurrently from multiple threads.
///
/// # Safety
///
/// Implementors must guarantee that every exposed COM interface method may be
/// called from any thread, including concurrently with calls through cloned
/// interface pointers. This usually means the object is immutable, internally
/// synchronized, or otherwise free of thread-affine state. Its [`ComClass`]
/// implementation must identify [`ThreadSafe`] as the class capability.
pub unsafe trait ThreadSafeComObject: ComClass<Threading = ThreadSafe> + Send + Sync {}

#[cfg(test)]
mod tests {
    use super::__ComCallGate;
    use crate::NC_E_BUSY;

    /// Confirms shared calls compose while an exclusive call requires an idle object.
    #[test]
    fn call_gate_enforces_rust_borrowing_across_reentry() {
        let gate = __ComCallGate::new();

        gate.shared(|| {
            assert_eq!(gate.shared(|| 7), Ok(7));
            assert_eq!(gate.exclusive(|| ()), Err(NC_E_BUSY));
        })
        .unwrap();

        gate.exclusive(|| {
            assert_eq!(gate.shared(|| ()), Err(NC_E_BUSY));
            assert_eq!(gate.exclusive(|| ()), Err(NC_E_BUSY));
        })
        .unwrap();
    }
}
