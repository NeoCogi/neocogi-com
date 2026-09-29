//! Shared lifetime algorithms for allocations with the COM control header.
//!
//! Counters belong to the allocation, not an interface projection. These functions
//! are deliberately non-generic: only destruction, allocator release and the
//! immutable interface list depend on the concrete class. No Arc layout is read.
use core::{
    ffi::c_void,
    sync::atomic::{AtomicUsize, Ordering, fence},
};
use crate::{ComRaw, IUnknown, IUnknownVTable, NcInterfaceId, NcStatus, NC_E_CLOSED, NC_E_POINTER, NC_OK};

/// Concrete-class facts used by the common lifetime operations.
///
/// The descriptor and its interface tables are immutable and must remain loaded
/// until the final weak owner releases every referencing allocation. Neither
/// callback may unwind. Both receive the allocation's header at offset zero.
#[repr(C)]
pub struct NcClassLifetime {
    /// Destroys only the initialized Rust/foreign value, exactly once. The header
    /// and allocation remain valid throughout this call, including reentrant
    /// release of weak fields. Strong promotion has already become impossible.
    pub destroy_value: unsafe extern "C" fn(*mut NcWeakControl),
    /// Frees the storage through its allocating provider, after value destruction
    /// or abandoned initialization. Must not attempt to destroy the value again.
    pub deallocate: unsafe extern "C" fn(*mut NcWeakControl),
    /// Static array of matching class interface tables, including IUnknown.
    pub interfaces: *const *const IUnknownVTable,
    /// Number of initialized entries in `interfaces`; must be nonzero.
    pub interface_count: usize,
}
// SAFETY: immutable metadata carries no permission to access any object value.
unsafe impl Sync for NcClassLifetime {}

/// Common C-shaped allocation header; the concrete value follows at its own alignment.
///
/// A live allocation owns one implicit weak reference until destruction completes.
/// Unpublished construction owns that reference provisionally with strong == 0.
/// Publication is the only permitted zero-to-one transition; failed construction
/// releases its provisional weak owner without ever invoking destroy_value.
#[repr(C)]
pub struct NcWeakControl {
    /// Number of strong owners. Publication and promotion use release/acquire;
    /// zero after final release is terminal. Counts never wrap.
    strong: AtomicUsize,
    /// Explicit weak owners plus the provisional/implicit construction owner.
    weak: AtomicUsize,
    /// Provider-owned class metadata, stable through the final weak release.
    class: *const NcClassLifetime,
}
impl NcWeakControl {
    /// Starts a fully initialized allocation with one strong and one implicit weak owner.
    /// The caller must initialize the complete concrete value before publishing it.
    pub const fn new(class: &'static NcClassLifetime) -> Self {
        Self {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            class,
        }
    }
    /// Starts an unpublished allocation with one provisional weak owner and no value owner.
    pub const fn unpublished(class: &'static NcClassLifetime) -> Self {
        Self {
            strong: AtomicUsize::new(0),
            weak: AtomicUsize::new(1),
            class,
        }
    }
    /// Publishes initialization and converts the provisional weak owner into the implicit owner.
    ///
    /// # Safety
    /// Called exactly once by the constructor, after initializing the concrete
    /// value and before publishing any strong reference. The constructor owns the
    /// provisional weak reference and must transfer it, without releasing it.
    pub unsafe fn publish(&self) {
        debug_assert_eq!(self.strong.load(Ordering::Relaxed), 0);
        self.strong.store(1, Ordering::Release);
    }
}

/// Increments an already owned counter without wrapping, for both ownership kinds.
/// Public lifetime entry points use the non-unwinding C ABI, so exhaustion aborts
/// instead of exposing a wrapped count or allowing premature destruction.
fn retain(counter: &AtomicUsize) -> usize {
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        assert!(current > 0 && current < isize::MAX as usize, "COM lifetime counter exhausted or unowned");
        match counter.compare_exchange_weak(current, current + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return current + 1,
            Err(observed) => current = observed,
        }
    }
}

/// Retains a live generated owner; null returns zero without accessing storage.
///
/// # Safety
/// A non-null object must begin with a valid header and remain strongly owned
/// throughout the call. This operation cannot resurrect a zero strong count.
#[inline(never)]
pub unsafe extern "C" fn nc_strong_retain(object: *mut c_void) -> u32 {
    let Some(control) = (unsafe { object.cast::<NcWeakControl>().as_ref() }) else {
        return 0;
    };
    retain(&control.strong).min(u32::MAX as usize) as u32
}

/// Releases a strong owner, destroying the value before releasing its implicit weak owner.
///
/// # Safety
/// A non-null object must own one reference to a matching header allocation.
/// The caller must not access the allocation after transferring that reference.
#[inline(never)]
pub unsafe extern "C" fn nc_strong_release(object: *mut c_void) -> u32 {
    let pointer = object.cast::<NcWeakControl>();
    let Some(control) = (unsafe { pointer.as_ref() }) else { return 0 };
    let previous = control.strong.fetch_sub(1, Ordering::Release);
    assert!(previous > 0, "COM strong release without an owner");
    match previous {
        1 => {
            // All preceding owners' writes complete before the one destructor.
            fence(Ordering::Acquire);
            // SAFETY: class metadata remains retained by the implicit weak owner.
            unsafe { ((*control.class).destroy_value)(pointer) };
            // The destructor may release weak fields; our implicit owner kept
            // storage alive until it returned. Do not read control after this call.
            unsafe { nc_weak_release(pointer) };
            0
        }
        _ => (previous - 1).min(u32::MAX as usize) as u32,
    }
}

/// Retains an explicit weak owner, including after value destruction.
///
/// # Safety
/// A non-null control must remain weak-owned or strongly owned during this call.
/// Null is accepted as a no-op for foreign cleanup paths.
#[inline(never)]
pub unsafe extern "C" fn nc_weak_retain(control: *mut NcWeakControl) {
    if let Some(control) = unsafe { control.as_ref() } {
        retain(&control.weak);
    }
}

/// Releases an explicit weak owner; only the final release frees the allocation.
///
/// # Safety
/// A non-null control transfers exactly one weak owner. Its value must already
/// be destroyed or unpublished if this is the final owner. Null is a no-op.
#[inline(never)]
pub unsafe extern "C" fn nc_weak_release(control: *mut NcWeakControl) {
    let Some(header) = (unsafe { control.as_ref() }) else { return };
    let previous = header.weak.fetch_sub(1, Ordering::Release);
    assert!(previous > 0, "COM weak release without an owner");
    if previous == 1 {
        fence(Ordering::Acquire);
        // SAFETY: the last weak owner still keeps immutable metadata readable;
        // the provider deallocates exactly its own storage without dropping T.
        unsafe { ((*header.class).deallocate)(control) };
    }
}

/// Resolves an exact projection without altering ownership.
///
/// # Safety
/// Header, class metadata and requested identity must be live and valid.
unsafe fn projection(control: &NcWeakControl, requested: &NcInterfaceId) -> Result<*const IUnknownVTable, NcStatus> {
    // SAFETY: the class contract supplies a permanent nonempty table array.
    let class = unsafe { &*control.class };
    let tables = unsafe { core::slice::from_raw_parts(class.interfaces, class.interface_count) };
    unsafe { crate::__nc_find_interface(tables, requested) }
}

/// Queries a live allocation and transfers exactly one new strong reference.
///
/// # Safety
/// Non-null object begins with the matching header and remains strongly owned.
/// Non-null requested/output records are valid and distinct from the allocation.
#[inline(never)]
pub unsafe extern "C" fn nc_query_interface(object: *mut c_void, requested: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe {
        *out = ComRaw::null();
    }
    if object.is_null() || requested.is_null() {
        return NC_E_POINTER;
    }
    let control = unsafe { &*object.cast::<NcWeakControl>() };
    match unsafe { projection(control, &*requested) } {
        Ok(vtable) => {
            unsafe {
                nc_strong_retain(object);
                *out = ComRaw { object, vtable };
            }
            NC_OK
        }
        Err(status) => status,
    }
}

/// Atomically promotes and transfers the acquired owner without QueryInterface retaining again.
///
/// # Safety
/// A non-null control is weak-owned throughout the call. Non-null request/output
/// records are valid and distinct. Success transfers one strong owner; every
/// failure clears output. Expiration takes precedence over interface negotiation.
#[inline(never)]
pub unsafe extern "C" fn nc_weak_upgrade(control: *mut NcWeakControl, requested: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe {
        *out = ComRaw::null();
    }
    if control.is_null() || requested.is_null() {
        return NC_E_POINTER;
    }
    let header = unsafe { &*control };
    let mut current = header.strong.load(Ordering::Relaxed);
    loop {
        match current {
            0 => return NC_E_CLOSED,
            count => {
                assert!(count < isize::MAX as usize, "COM strong promotion counter exhausted");
                // Acquire pairs with initial publication. A concurrent final
                // release wins by setting zero or loses to this acquired owner.
                match header.strong.compare_exchange_weak(count, count + 1, Ordering::Acquire, Ordering::Relaxed) {
                    Ok(_) => break,
                    Err(observed) => current = observed,
                }
            }
        }
    }
    match unsafe { projection(header, &*requested) } {
        Ok(vtable) => {
            unsafe {
                *out = ComRaw { object: control.cast(), vtable };
            }
            NC_OK
        }
        Err(status) => {
            // May destroy the value if another thread released the former last
            // owner while we queried. The caller's weak token still owns storage.
            unsafe {
                nc_strong_release(control.cast());
            }
            status
        }
    }
}

#[cfg(test)]
mod tests;
