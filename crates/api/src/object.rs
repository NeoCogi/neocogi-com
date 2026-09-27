//! Reference-count primitives and object hooks used by generated COM classes.
//!
//! Generated class storage embeds an atomic strong count and no interface headers.
//! Object/table handles select each implemented interface. The helpers here implement saturating `AddRef` and
//! non-underflowing `Release`; the unsafe traits describe how generated thunks
//! access the concrete Rust value from the allocation address, perform
//! interface negotiation, and destroy storage on the final release. They are
//! public only as hidden macro support and are not the ordinary end-user API.

use core::ffi::c_void;
use core::sync::atomic::{AtomicUsize, Ordering};

use neocogi_com_core::{NC_E_BUSY, NC_E_POINTER, NcInterfaceId, NcStatus, ThreadSafe, ThreadingModel};

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

#[doc(hidden)]
/// Shared typed access to a generated COM implementation while its call gate is held.
///
/// Generated thunks keep this lease alive for exactly one Rust method call. Keeping
/// the value reference and gate guard in one type prevents macro expansions from
/// releasing the gate while the reference is still usable.
pub struct __ComSharedLease<'a, T> {
    /// Concrete implementation protected by the shared gate acquisition.
    value: &'a T,
    /// Reader slot whose destructor releases the shared gate acquisition.
    _call: SharedCall<'a>,
}

impl<'a, T> __ComSharedLease<'a, T> {
    /// Acquires shared access to `value` through `gate`.
    ///
    /// # Safety
    ///
    /// `value` must be non-null, initialized, and stored in the same live generated
    /// COM allocation as `gate`. No code outside that allocation's call gate may
    /// mutate the value until the returned lease is dropped.
    pub unsafe fn acquire(gate: &'a __ComCallGate, value: *const T) -> Result<Self, NcStatus> {
        if value.is_null() {
            return Err(NC_E_POINTER);
        }
        let call = gate.try_shared()?;
        // SAFETY: The caller guarantees a live initialized value protected by
        // this exact gate, and the acquired reader slot excludes mutable calls.
        let value = unsafe { &*value };
        Ok(Self { value, _call: call })
    }
}

impl<T> core::ops::Deref for __ComSharedLease<'_, T> {
    type Target = T;

    /// Borrows the implementation for no longer than the active shared lease.
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

#[doc(hidden)]
/// Exclusive typed access to a generated COM implementation while its call gate is held.
///
/// Generated thunks use this lease instead of a closure-shaped borrow API, so one
/// borrow implementation is emitted per concrete class rather than per ABI method.
pub struct __ComExclusiveLease<'a, T> {
    /// Concrete implementation protected by the exclusive gate acquisition.
    value: &'a mut T,
    /// Writer slot whose destructor restores the gate to its idle state.
    _call: ExclusiveCall<'a>,
}

impl<'a, T> __ComExclusiveLease<'a, T> {
    /// Acquires exclusive access to `value` through `gate`.
    ///
    /// # Safety
    ///
    /// `value` must be non-null, initialized, and stored in the same live generated
    /// COM allocation as `gate`. Every access to the value must participate in that
    /// allocation's call gate until the returned lease is dropped.
    pub unsafe fn acquire(gate: &'a __ComCallGate, value: *mut T) -> Result<Self, NcStatus> {
        if value.is_null() {
            return Err(NC_E_POINTER);
        }
        let call = gate.try_exclusive()?;
        // SAFETY: The caller guarantees a live initialized value protected by
        // this exact gate, and the acquired writer slot excludes every other call.
        let value = unsafe { &mut *value };
        Ok(Self { value, _call: call })
    }
}

impl<T> core::ops::Deref for __ComExclusiveLease<'_, T> {
    type Target = T;

    /// Borrows the implementation for no longer than the active exclusive lease.
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> core::ops::DerefMut for __ComExclusiveLease<'_, T> {
    /// Mutably borrows the implementation while the lease owns the writer slot.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
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

pub use neocogi_com_core::{__com_add_ref_count, __com_release_ref_count};

/// Shared query body used by generated objects regardless of their Rust state.
/// The small generated adapter supplies its actual counter and static tables;
/// this function knows no field offsets or concrete implementation types.
///
/// A per-type table address prevents identical generated query bodies from being
/// merged. Passing that address as data preserves one copy of validation, lookup,
/// retention and output publication, rather than hoping the linker merges them.
///
/// # Safety
/// Object must identify the live allocation owning references. Tables must be
/// immutable matching projections of that allocation. Non-null requested/out
/// pointers must be readable/writable for one record and must not alias storage.
#[doc(hidden)]
#[inline(never)]
pub unsafe fn __com_query_interfaces(
    object: *mut c_void,
    references: &core::sync::atomic::AtomicU32,
    tables: &[*const crate::IUnknownVTable],
    requested: *const NcInterfaceId,
    out: *mut crate::ComRaw<crate::IUnknown>,
) -> NcStatus {
    if requested.is_null() || out.is_null() {
        return NC_E_POINTER;
    }
    // SAFETY: valid output storage is distinct from this allocation and request.
    unsafe {
        *out = crate::ComRaw::null();
    }
    // SAFETY: generated tables and the requested identity remain live for this call.
    match unsafe { crate::__nc_find_interface(tables, &*requested) } {
        Ok(vtable) => {
            __com_add_ref_count(references);
            // SAFETY: retain transferred one owner with the exact requested table.
            unsafe {
                *out = crate::ComRaw { object, vtable };
            }
            crate::NC_OK
        }
        Err(status) => status,
    }
}

/// Implemented by generated COM class storage.
///
/// # Safety
///
/// Implementors must map opaque generated storage pointers back to the exact
/// allocation and Rust value that produced them. Reference-count operations must
/// keep the allocation alive until the final `Release`, and `QueryInterface`
/// must only return tables paired with that same allocation address.
///
/// Interface thunks receive an opaque object pointer and use this trait to
/// recover the Rust implementation and delegate IUnknown behavior.
pub unsafe trait ComAbiObject: Sized {
    /// Acquires shared access to the Rust value in generated storage.
    ///
    /// # Safety
    ///
    /// `object` must be an opaque storage pointer originally produced by the
    /// matching generated `#[com_object]` implementation. The object allocation
    /// must remain live until the returned lease is dropped.
    unsafe fn borrow_abi_ref<'a>(object: *mut c_void) -> Result<__ComSharedLease<'a, Self>, NcStatus>;

    /// Acquires exclusive access to the Rust value in generated storage.
    ///
    /// # Safety
    ///
    /// `object` must be an opaque storage pointer originally produced by the
    /// matching generated `#[com_object]` implementation. The object allocation
    /// must remain live until the returned lease is dropped.
    unsafe fn borrow_abi_mut<'a>(object: *mut c_void) -> Result<__ComExclusiveLease<'a, Self>, NcStatus>;

    /// Implements `QueryInterface` for generated interface thunks.
    ///
    /// # Safety
    ///
    /// `object` must be valid generated storage, `interface_id` must point to a
    /// readable initialized identity, and `out` must point to a distinct slot
    /// valid for one ComRaw record write. Given those preconditions, `NC_OK` writes
    /// one present owned interface pair to `*out`; every other returned
    /// status leaves both fields of `*out` null and transfers no reference. Null or otherwise
    /// invalid arguments violate this safety contract even when an
    /// implementation rejects them defensively.
    unsafe extern "C" fn query_interface_from_abi_object(
        object: *mut c_void,
        interface_id: *const NcInterfaceId,
        out: *mut crate::ComRaw<crate::IUnknown>,
    ) -> NcStatus;

    /// Increments the reference count for generated object storage.
    ///
    /// The returned count is diagnostic only. Debug builds assert if the count
    /// would overflow; release builds saturate at `u32::MAX` instead of
    /// wrapping.
    ///
    /// # Safety
    ///
    /// `object` must be valid generated storage or null.
    unsafe extern "C" fn add_ref_from_abi_object(object: *mut c_void) -> u32;

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
    unsafe extern "C" fn release_from_abi_object(object: *mut c_void) -> u32;
}

/// Associates a generated COM implementation object with the strongest
/// threading capability supported by pointers to its object storage.
///
/// Generated pointers and IDL description associations derive their threading
/// mode from this associated type. A [`ThreadSafe`] object may still be exposed
/// through a conservative thread-local pointer; the associated type records the
/// maximum capability the implementation is allowed to produce.
///
/// | `Threading` | Generated owner | ABI value | Caller permission |
/// |---|---|---|---|
/// | [`crate::ThreadLocal`] | `ComPtr<_, ThreadLocal>` | `THREAD_LOCAL` | Current-thread use only |
/// | [`ThreadSafe`] | `ComPtr<_, ThreadSafe>` | `THREAD_SAFE` | Cross-thread movement, sharing, calls, and final release |
///
/// This associated type cannot by itself control a pointer forever. Once a
/// object is exposed through one of its interfaces, the concrete implementation
/// type is erased and several objects with different threading models may
/// implement that same interface. The `ComPtr` or `ComRef` threading parameter carries
/// this producer promise into the erased handle type, where Rust can enforce
/// `Send` and `Sync`. The object associated type and pointer marker are therefore
/// the producer and consumer halves of one contract, not duplicate state.
///
/// # Safety
///
/// `Threading` must describe every interface projection backed by this implementation's
/// generated object storage. Selecting [`ThreadSafe`] promises that all exposed
/// methods may be called concurrently from arbitrary threads and that final
/// destruction may occur on any thread. IDL-backed implementations derive this
/// associated type from their canonical description.
pub unsafe trait ComObject: ComAbiObject {
    /// Strongest threading capability supported by this implementation's COM objects.
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
/// synchronized, or otherwise free of thread-affine state. Its [`ComObject`]
/// implementation must identify [`ThreadSafe`] as the class capability.
pub unsafe trait ThreadSafeComObject: ComObject<Threading = ThreadSafe> + Send + Sync {}

#[cfg(test)]
mod tests {
    use super::{__ComCallGate, __ComExclusiveLease, __ComSharedLease};
    use crate::NC_E_BUSY;

    /// Confirms shared calls compose while an exclusive call requires an idle object.
    #[test]
    fn call_gate_enforces_rust_borrowing_across_reentry() {
        let gate = __ComCallGate::new();
        let mut value = 7;

        // SAFETY: `value` remains live and every test access participates in
        // this local gate for the complete lifetime of each returned lease.
        let shared = unsafe { __ComSharedLease::acquire(&gate, core::ptr::from_ref(&value)) }.unwrap();
        // SAFETY: A second shared lease observes the same live immutable value.
        let nested_shared = unsafe { __ComSharedLease::acquire(&gate, core::ptr::from_ref(&value)) }.unwrap();
        assert_eq!(*nested_shared, 7);
        // SAFETY: The pointer is valid, but the active reader leases must reject
        // this exclusive acquisition before exposing a mutable reference.
        assert!(matches!(
            unsafe { __ComExclusiveLease::acquire(&gate, core::ptr::from_mut(&mut value)) },
            Err(NC_E_BUSY)
        ));
        drop(nested_shared);
        drop(shared);

        // SAFETY: No lease remains and `value` is still owned by this scope.
        let mut exclusive = unsafe { __ComExclusiveLease::acquire(&gate, core::ptr::from_mut(&mut value)) }.unwrap();
        *exclusive = 9;
        // SAFETY: The pointer is valid, but the writer lease must reject another reader.
        assert!(matches!(
            unsafe { __ComSharedLease::acquire(&gate, core::ptr::from_ref(&value)) },
            Err(NC_E_BUSY)
        ));
        drop(exclusive);

        // SAFETY: Dropping the writer restored the idle state and the value remains live.
        let shared = unsafe { __ComSharedLease::acquire(&gate, core::ptr::from_ref(&value)) }.unwrap();
        assert_eq!(*shared, 9);
    }
}
