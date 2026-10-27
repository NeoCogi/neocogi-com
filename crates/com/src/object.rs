//! Reference-count primitives and object hooks used by generated COM classes.
//!
//! Generated class storage uses the common COM strong/weak control header and no interface headers.
//! Object/table handles select each implemented interface. Shared lifetime functions
//! live in COM core; the unsafe implementation contract describes how generated thunks
//! access the concrete Rust value from the allocation address, perform
//! interface negotiation, and destroy storage on the final release. They are
//! public only as hidden macro support and are not the ordinary end-user API.

#[cfg(feature = "alloc")]
mod storage;
#[cfg(feature = "alloc")]
pub use storage::{__ComBoxStorage, __ComSharedStorage, __ComStorage};

use core::ffi::c_void;
use core::{cell::UnsafeCell, mem::MaybeUninit};
use core::sync::atomic::{AtomicUsize, Ordering};

use neocogi_com_core::{NC_E_BUSY, NcStatus, ThreadingModel};

/// Sentinel held while one generated COM method has exclusive object access.
const EXCLUSIVE_CALL: usize = usize::MAX;

#[doc(hidden)]
/// Borrow gate for shared COM classes; exclusive classes use their common header.
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

    /// Acquires one reader slot without blocking or permitting counter overflow.
    fn try_shared(&self) -> Result<__ComSharedCall<'_>, NcStatus> {
        let mut current = self.state.load(Ordering::Acquire);
        loop {
            if current == EXCLUSIVE_CALL || current == EXCLUSIVE_CALL - 1 {
                return Err(NC_E_BUSY);
            }
            match self.state.compare_exchange_weak(current, current + 1, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => return Ok(__ComSharedCall { gate: self }),
                Err(observed) => current = observed,
            }
        }
    }

    /// Acquires the unique writer sentinel without waiting on active readers.
    fn try_exclusive(&self) -> Result<__ComExclusiveCall<'_>, NcStatus> {
        self.state
            .compare_exchange(0, EXCLUSIVE_CALL, Ordering::Acquire, Ordering::Relaxed)
            .map(|_| __ComExclusiveCall { gate: self })
            .map_err(|_| NC_E_BUSY)
    }
}

#[doc(hidden)]
/// Shared typed access to a generated COM implementation while its call gate is held.
///
/// Generated thunks keep this lease alive for exactly one Rust method call. Keeping
/// the typed value slot and gate guard in one type ties every actual value reborrow
/// to the lease. The lease must not store `&T`: moving such a guard into a drop
/// function can protect that reference until after its gate has been released.
pub struct __ComSharedLease<'a, T, G = __ComSharedCall<'a>> {
    /// Retained storage remains valid after T is destroyed. UnsafeCell permits
    /// a newly admitted writer/destructor after this lease releases its gate.
    value: &'a UnsafeCell<MaybeUninit<T>>,
    /// Reader slot whose destructor releases the shared gate acquisition.
    _call: G,
}

impl<'a, T> __ComSharedLease<'a, T> {
    /// Acquires shared access to `value` through `gate`.
    ///
    /// # Safety
    ///
    /// `value` must contain initialized T while admission is held and belong to
    /// the same allocation as `gate`. Every value access uses this gate. The
    /// caller retains slot storage through the complete lease/drop invocation;
    /// a newly admitted writer may change T immediately after guard release.
    pub unsafe fn acquire(gate: &'a __ComCallGate, value: &'a UnsafeCell<MaybeUninit<T>>) -> Result<Self, NcStatus> {
        let call = gate.try_shared()?;
        Ok(Self { value, _call: call })
    }
}

impl<'a, T> __ComSharedLease<'a, T, crate::lifetime::BoxRead> {
    /// Acquires lifetime and Rust borrowing in one atomic header transition.
    ///
    /// # Safety
    /// `control` and `value` belong to this retained exclusive allocation; all
    /// implementation access participates in its common borrowing protocol.
    /// Storage must outlive the complete lease/drop call, not only gate release.
    pub unsafe fn acquire_box(control: *mut crate::lifetime::NcWeakControl, value: &'a UnsafeCell<MaybeUninit<T>>) -> Result<Self, NcStatus> {
        // SAFETY: the caller retains the matching control and value slot. A
        // successful admission guarantees initialized T until the guard releases.
        let call = unsafe { crate::lifetime::BoxRead::enter(control) }?;
        Ok(Self { value, _call: call })
    }
}

impl<T, G> core::ops::Deref for __ComSharedLease<'_, T, G> {
    type Target = T;

    /// Borrows the implementation for no longer than the active shared lease.
    fn deref(&self) -> &Self::Target {
        // SAFETY: admission proves initialization and excludes writers/destruction.
        // This T loan ends with the borrow of the lease, before gate release;
        // only a reference to the interior-mutable storage travels through Drop.
        unsafe { (&*self.value.get()).assume_init_ref() }
    }
}

#[doc(hidden)]
/// Exclusive typed access to a generated COM implementation while its call gate is held.
///
/// Generated thunks use this lease instead of a closure-shaped borrow API, so one
/// borrow implementation is emitted per concrete class rather than per ABI method.
pub struct __ComExclusiveLease<'a, T, G = __ComExclusiveCall<'a>> {
    /// Typed retained slot, without a T reference protected across gate release.
    value: &'a UnsafeCell<MaybeUninit<T>>,
    /// Writer slot whose destructor restores the gate to its idle state.
    _call: G,
}

impl<'a, T> __ComExclusiveLease<'a, T> {
    /// Acquires exclusive access to `value` through `gate`.
    ///
    /// # Safety
    ///
    /// `value` must contain initialized T while admission is held and belong to
    /// the same allocation as `gate`. Every value access uses this gate. The
    /// caller retains slot storage through the complete lease/drop invocation,
    /// including after a new operation can obtain admission.
    pub unsafe fn acquire(gate: &'a __ComCallGate, value: &'a UnsafeCell<MaybeUninit<T>>) -> Result<Self, NcStatus> {
        let call = gate.try_exclusive()?;
        Ok(Self { value, _call: call })
    }
}

impl<'a, T> __ComExclusiveLease<'a, T, crate::lifetime::BoxWrite> {
    /// Acquires lifetime and Rust borrowing in one atomic header transition.
    ///
    /// # Safety
    /// `control` and `value` belong to this retained exclusive allocation; all
    /// implementation access participates in its common borrowing protocol.
    /// Storage must outlive the complete lease/drop call, not only gate release.
    pub unsafe fn acquire_box(control: *mut crate::lifetime::NcWeakControl, value: &'a UnsafeCell<MaybeUninit<T>>) -> Result<Self, NcStatus> {
        // SAFETY: the caller retains matching storage; admission excludes every
        // competing value access before Deref/DerefMut may create a T loan.
        let call = unsafe { crate::lifetime::BoxWrite::enter(control) }?;
        Ok(Self { value, _call: call })
    }
}

impl<T, G> core::ops::Deref for __ComExclusiveLease<'_, T, G> {
    type Target = T;

    /// Borrows the implementation for no longer than the active exclusive lease.
    fn deref(&self) -> &Self::Target {
        // SAFETY: the held writer guard proves initialization and excludes all
        // other value access. The returned reference borrows this lease only.
        unsafe { (&*self.value.get()).assume_init_ref() }
    }
}

impl<T, G> core::ops::DerefMut for __ComExclusiveLease<'_, T, G> {
    /// Mutably borrows the implementation while the lease owns the writer slot.
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: the unique lease reborrow and writer guard jointly grant this
        // T loan. No T reference remains stored in the lease during its Drop.
        unsafe { (&mut *self.value.get()).assume_init_mut() }
    }
}

// SAFETY: these are the same value/guard bounds as the former &T field. The
// gate excludes mutation, and moving the lease transfers its guard and loans.
unsafe impl<T: Sync, G: Send> Send for __ComSharedLease<'_, T, G> {}
// SAFETY: shared lease access can create only &T; the held guard excludes writers.
unsafe impl<T: Sync, G: Sync> Sync for __ComSharedLease<'_, T, G> {}
// SAFETY: these are the former &mut T bounds. Moving exclusive access is valid
// for Send T only when the selected guard may itself move to that thread.
unsafe impl<T: Send, G: Send> Send for __ComExclusiveLease<'_, T, G> {}
// SAFETY: sharing &lease exposes only &T. DerefMut still requires unique access
// to the lease, while G's Sync proof governs sharing the admission token.
unsafe impl<T: Sync, G: Sync> Sync for __ComExclusiveLease<'_, T, G> {}

impl Default for __ComCallGate {
    /// Creates the same idle gate as [`__ComCallGate::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Active shared-call lease that restores its reader slot on every return path.
pub struct __ComSharedCall<'a> {
    /// Gate whose reader count this lease owns.
    gate: &'a __ComCallGate,
}

impl Drop for __ComSharedCall<'_> {
    /// Releases exactly one reader slot after the delegated Rust call ends.
    fn drop(&mut self) {
        let previous = self.gate.state.fetch_sub(1, Ordering::Release);
        debug_assert!(previous > 0 && previous != EXCLUSIVE_CALL);
    }
}

/// Active mutable-call lease that restores the idle state on every return path.
pub struct __ComExclusiveCall<'a> {
    /// Gate whose exclusive sentinel this lease owns.
    gate: &'a __ComCallGate,
}

impl Drop for __ComExclusiveCall<'_> {
    /// Releases the exclusive sentinel after the delegated Rust call ends.
    fn drop(&mut self) {
        let previous = self.gate.state.swap(0, Ordering::Release);
        debug_assert_eq!(previous, EXCLUSIVE_CALL);
    }
}

pub use neocogi_com_core::{__com_add_ref_count, __com_release_ref_count};

/// Implementation proof emitted together with typed storage bindings.
///
/// # Safety
///
/// Implementors must map opaque generated storage pointers back to the exact
/// allocation and Rust value that produced them. Every allocation must begin with
/// `lifetime::NcWeakControl`, whose permanent class descriptor lists its matching
/// projections and concrete destruction/deallocation callbacks. Shared lifetime operations must
/// keep the allocation alive until the final `Release`, and `QueryInterface`
/// must only return tables paired with that same allocation address. Every
/// instance of one implementation must use its declared lifetime policy:
/// shared instances require a strong owner throughout each call; exclusive
/// instances acquire their reader/writer bit in the lifetime header before
/// referencing the value and release it only after all T loans have ended.
/// Guards must not store a T reference protected across their release operation;
/// callers retain storage through the complete lease/drop invocation even when
/// a concurrent owner destroys T immediately after admission is released.
///
/// Interface thunks receive an opaque object pointer and use this trait to
/// recover the Rust implementation. VTABLES must contain canonical IUnknown and
/// exactly the immutable, signature-checked tables paired with this storage.
/// Threading must hold for every projection, call and final destruction.
pub unsafe trait ComObject: Sized + 'static {
    /// Strongest threading permission the generated provider can expose.
    type Threading: ThreadingModel;
    /// Compact immutable interface tables paired with this concrete storage.
    /// Includes canonical IUnknown; full inspection schemas are compile-time only.
    const VTABLES: &'static [*const crate::IUnknownVTable];
    /// Shared borrowing policy: the shared class gate or exclusive header reader.
    /// This provider-local guard never crosses the C ABI.
    type SharedGuard<'a>;
    /// Mutable borrowing policy: the shared class gate or exclusive header writer.
    type ExclusiveGuard<'a>;

    /// Acquires shared access to the Rust value in generated storage.
    ///
    /// # Safety
    ///
    /// `object` must be an opaque storage pointer originally produced by the
    /// matching generated `#[com_object]` implementation. The object allocation
    /// must remain live until the returned lease is dropped. A shared class
    /// requires a retained strong owner; an exclusive class admits a call before
    /// accessing its value and rejects calls after owner removal.
    unsafe fn borrow_abi_ref<'a>(object: *mut c_void) -> Result<__ComSharedLease<'a, Self, Self::SharedGuard<'a>>, NcStatus>;

    /// Acquires exclusive access to the Rust value in generated storage.
    ///
    /// # Safety
    ///
    /// `object` must be an opaque storage pointer originally produced by the
    /// matching generated `#[com_object]` implementation. The object allocation
    /// must remain live until the returned lease is dropped. A shared class
    /// requires a retained strong owner; an exclusive class admits a call before
    /// accessing its value and rejects calls after owner removal.
    unsafe fn borrow_abi_mut<'a>(object: *mut c_void) -> Result<__ComExclusiveLease<'a, Self, Self::ExclusiveGuard<'a>>, NcStatus>;
}

#[cfg(test)]
mod tests {
    use super::{__ComCallGate, __ComExclusiveLease, __ComSharedLease};
    use core::{cell::UnsafeCell, mem::MaybeUninit};
    use crate::NC_E_BUSY;

    /// Releases actual admission and immediately gives a new writer the slot.
    /// This makes the old reference-bearing guard's protected-drop bug
    /// deterministic under Miri, without a scheduler or timing assumption.
    struct WriteAfterRelease<'a, G> {
        /// Admission being handed off, consumed exactly once in Drop.
        call: Option<G>,
        /// The same real gate must admit the following write.
        gate: &'a __ComCallGate,
        /// Typed storage, deliberately containing no reference to the value.
        value: &'a UnsafeCell<MaybeUninit<u32>>,
    }
    impl<G> Drop for WriteAfterRelease<'_, G> {
        fn drop(&mut self) {
            drop(self.call.take());
            // SAFETY: the previous admission has ended; this fresh writer uses
            // the same gate and initialized slot before changing its value.
            let mut next = unsafe { __ComExclusiveLease::acquire(self.gate, self.value) }.unwrap();
            *next = 99;
        }
    }

    /// Passing a lease by value to Drop must not protect T after gate release.
    #[test]
    fn lease_release_does_not_keep_a_protected_value_reference() {
        let gate = __ComCallGate::new();
        let value = UnsafeCell::new(MaybeUninit::new(7));
        // SAFETY: all accesses participate in this gate and storage outlives
        // every lease, including the release callback's next admission.
        let first = unsafe { __ComSharedLease::acquire(&gate, &value) }.unwrap();
        let lease = __ComSharedLease {
            value: first.value,
            _call: WriteAfterRelease {
                call: Some(first._call),
                gate: &gate,
                value: &value,
            },
        };
        assert_eq!(*lease, 7);
        drop(lease);
        let first = unsafe { __ComExclusiveLease::acquire(&gate, &value) }.unwrap();
        let mut lease = __ComExclusiveLease {
            value: first.value,
            _call: WriteAfterRelease {
                call: Some(first._call),
                gate: &gate,
                value: &value,
            },
        };
        assert_eq!(*lease, 99);
        *lease = 8;
        drop(lease);
        let observed = unsafe { __ComSharedLease::acquire(&gate, &value) }.unwrap();
        assert_eq!(*observed, 99);
    }

    /// The typed slot keeps the old reference's positive threading bounds;
    /// local owner modes and non-Send guards still impose their own restriction.
    #[test]
    fn shared_storage_lease_threading_capabilities_are_preserved() {
        fn shared<T: Send + Sync>() {}
        shared::<__ComSharedLease<'_, u32>>();
        shared::<__ComExclusiveLease<'_, u32>>();
    }

    /// Provider policy must not reintroduce optional tokens or boxed admission
    /// into shared calls: these helper records never need an enum discriminator.
    #[test]
    fn call_leases_store_only_the_selected_lifetime_policy() {
        use core::mem::size_of;
        use crate::lifetime::{BoxRead, BoxWrite};
        assert_eq!(size_of::<__ComSharedLease<'_, u32>>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<__ComExclusiveLease<'_, u32>>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<__ComSharedLease<'_, u32, BoxRead>>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<__ComExclusiveLease<'_, u32, BoxWrite>>(), 2 * size_of::<usize>());
    }

    /// Confirms shared calls compose while an exclusive call requires an idle object.
    #[test]
    fn call_gate_enforces_rust_borrowing_across_reentry() {
        let gate = __ComCallGate::new();
        let value = UnsafeCell::new(MaybeUninit::new(7));

        // SAFETY: `value` remains live and every test access participates in
        // this local gate for the complete lifetime of each returned lease.
        let shared = unsafe { __ComSharedLease::acquire(&gate, &value) }.unwrap();
        // SAFETY: A second shared lease observes the same live immutable value.
        let nested_shared = unsafe { __ComSharedLease::acquire(&gate, &value) }.unwrap();
        assert_eq!(*nested_shared, 7);
        // SAFETY: The pointer is valid, but the active reader leases must reject
        // this exclusive acquisition before exposing a mutable reference.
        assert!(matches!(unsafe { __ComExclusiveLease::acquire(&gate, &value) }, Err(NC_E_BUSY)));
        drop(nested_shared);
        drop(shared);

        // SAFETY: No lease remains and `value` is still owned by this scope.
        let mut exclusive = unsafe { __ComExclusiveLease::acquire(&gate, &value) }.unwrap();
        *exclusive = 9;
        // SAFETY: The pointer is valid, but the writer lease must reject another reader.
        assert!(matches!(unsafe { __ComSharedLease::acquire(&gate, &value) }, Err(NC_E_BUSY)));
        drop(exclusive);

        // SAFETY: Dropping the writer restored the idle state and the value remains live.
        let shared = unsafe { __ComSharedLease::acquire(&gate, &value) }.unwrap();
        assert_eq!(*shared, 9);
    }
}
