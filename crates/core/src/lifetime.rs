//! Shared lifetime algorithms for allocations with the COM control header.
//!
//! Counters belong to the allocation, not an interface projection. These functions
//! are deliberately non-generic: only destruction, allocator release and the
//! immutable interface list depend on the concrete class. No Arc layout is read.
use core::{
    ffi::c_void,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering, fence},
};
use crate::{ComRaw, IUnknown, IUnknownVTable, NcInterfaceId, NcStatus, NC_E_DROPPED, NC_E_POINTER, NC_E_INVALID_ARG, NC_OK};

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
    /// Shared allocations count strong owners. Exclusive allocations pack their
    /// ownership bit and reader/writer access into this same word, so checked
    /// entry and destruction cannot race through separate lifetime/borrow gates.
    /// Publication and promotion use release/acquire; counters never wrap.
    state: AtomicUsize,
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
            state: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            class,
        }
    }
    /// Starts an unpublished allocation with one provisional weak owner and no value owner.
    pub const fn unpublished(class: &'static NcClassLifetime) -> Self {
        Self {
            state: AtomicUsize::new(0),
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
        debug_assert_eq!(self.state.load(Ordering::Relaxed), 0);
        self.state.store(1, Ordering::Release);
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
    // Exclusive objects never manufacture a second owner, including through
    // borrowed COM arguments or foreign AddRef calls.
    if control.state.load(Ordering::Relaxed) & BOXED != 0 { return 0; }
    retain(&control.state).min(u32::MAX as usize) as u32
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
    if control.state.load(Ordering::Relaxed) & BOXED != 0 {
        // SAFETY: the caller transfers the allocation's sole owner.
        unsafe { nc_box_release(pointer) };
        return 0;
    }
    let previous = control.state.fetch_sub(1, Ordering::Release);
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

/// Creates one weak owner for a strongly retained header allocation.
///
/// # Safety
/// Non-null object must begin with the common header and remain strongly owned.
/// Non-null output is a distinct writable record. Failure clears the output.
#[inline(never)]
pub unsafe extern "C" fn nc_downgrade(object: *mut c_void, out: *mut crate::NcWeakRaw) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe {
        *out = crate::NcWeakRaw::null();
    }
    if object.is_null() {
        return NC_E_POINTER;
    }
    let control = object.cast::<NcWeakControl>();
    if unsafe { (*control).state.load(Ordering::Acquire) } & BOXED != 0 {
        return NC_E_INVALID_ARG;
    }
    unsafe {
        nc_weak_retain(control);
        *out = crate::NcWeakRaw { control };
    }
    NC_OK
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

/// Checks a shared weak token's exact interface without promoting its value.
/// Expired and unpublished tokens retain immutable class metadata and are valid
/// weak values. The permanent ownership tag rejects exclusive control blocks.
///
/// # Safety
/// A non-null control must be retained by a weak or strong owner with valid class
/// metadata/code through this call. The requested identity must be readable.
pub(crate) unsafe fn shared_weak_projection(control: *mut NcWeakControl, requested: &NcInterfaceId) -> Result<(), NcStatus> {
    let control = core::ptr::NonNull::new(control).ok_or(NC_E_POINTER)?;
    // SAFETY: the caller retains this header, including after value destruction.
    let header = unsafe { control.as_ref() };
    if header.state.load(Ordering::Relaxed) & BOXED != 0 {
        return Err(NC_E_INVALID_ARG);
    }
    // SAFETY: metadata outlives all weak references; no value access is performed.
    unsafe { projection(header, requested) }.map(|_| ())
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
    if control.state.load(Ordering::Acquire) & BOXED != 0 {
        return NC_E_INVALID_ARG;
    }
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

/// Canonical strong root table, shared across all standard COM objects using `NcWeakControl`.
pub static CANONICAL_UNKNOWN_VTABLE: IUnknownVTable = IUnknownVTable {
    interface_id: &IUnknown::INTERFACE_ID,
    query_interface: nc_query_interface,
    add_ref: nc_strong_retain,
    release: nc_strong_release,
    downgrade: Some(nc_downgrade),
};

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
    let mut current = header.state.load(Ordering::Relaxed);
    loop {
        match current {
            0 => return NC_E_DROPPED,
            count if count & BOXED != 0 => return NC_E_INVALID_ARG,
            count => {
                assert!(count < isize::MAX as usize, "COM strong promotion counter exhausted");
                // Acquire pairs with initial publication. A concurrent final
                // release wins by setting zero or loses to this acquired owner.
                match header.state.compare_exchange_weak(count, count + 1, Ordering::Acquire, Ordering::Relaxed) {
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

/// Distinguishes exclusive allocations from ordinary reference-counted ones.
/// The tag is permanent, including after destruction, so weak observers cannot
/// accidentally interpret an expired box as an unpublished shared allocation.
const BOXED: usize = 1 << (usize::BITS - 1);
/// The sole owner exists while this bit is set. Clearing it rejects new borrows.
const BOX_OWNER: usize = BOXED >> 1;
/// Exclusive mutable access; it cannot coexist with any reader.
const BOX_WRITER: usize = BOX_OWNER >> 1;
/// Reader count occupies the remaining bits and must not overflow into the writer bit.
const BOX_READERS: usize = BOX_WRITER - 1;
/// Either borrow keeps Drop waiting after owner presence has already been cleared.
const BOX_BORROWS: usize = BOX_WRITER | BOX_READERS;

impl NcWeakControl {
    /// Constructs an exclusive owner with the same stable C header as shared COM.
    /// Publication must happen only after the provider has initialized its value.
    pub const fn boxed(class: &'static NcClassLifetime) -> Self {
        Self { state: AtomicUsize::new(BOXED | BOX_OWNER), weak: AtomicUsize::new(1), class }
    }

    /// Constructs unpublished exclusive storage for weak self-initialization.
    /// Its provisional weak reference must be released if initialization fails.
    pub const fn boxed_unpublished(class: &'static NcClassLifetime) -> Self {
        Self { state: AtomicUsize::new(BOXED), weak: AtomicUsize::new(1), class }
    }

    /// Publishes the only exclusive owner after complete value initialization.
    ///
    /// # Safety
    /// Called exactly once on unpublished boxed storage by its constructor.
    pub unsafe fn publish_box(&self) {
        debug_assert_eq!(self.state.load(Ordering::Relaxed), BOXED);
        self.state.store(BOXED | BOX_OWNER, Ordering::Release);
    }
}

/// One shared implementation borrow, acquired atomically with the lifetime check.
/// Only generated provider entry points create these guards. Their destruction
/// releases access; it never destroys the implementation or takes ownership.
pub struct BoxRead {
    /// Retained header whose reader count includes this guard.
    control: NonNull<NcWeakControl>,
}

/// One exclusive implementation borrow, using the same state as readers and Drop.
pub struct BoxWrite {
    /// Retained header whose writer bit belongs to this guard.
    control: NonNull<NcWeakControl>,
}

/// Atomically admits one reader or the unique writer before any T reference exists.
///
/// # Safety
/// The caller retains exclusive-class allocation storage throughout acquisition
/// and through the returned borrow. Null is rejected without accessing storage.
#[inline(never)]
unsafe fn borrow_box(control: *mut NcWeakControl, writer: bool) -> Result<NonNull<NcWeakControl>, NcStatus> {
    let control = NonNull::new(control).ok_or(NC_E_POINTER)?;
    let header = unsafe { control.as_ref() };
    let mut state = header.state.load(Ordering::Acquire);
    loop {
        let next = match state {
            invalid if invalid & BOXED == 0 => return Err(NC_E_INVALID_ARG),
            expired if expired & BOX_OWNER == 0 => return Err(NC_E_DROPPED),
            busy if busy & BOX_WRITER != 0 => return Err(crate::NC_E_BUSY),
            busy if writer && busy & BOX_READERS != 0 => return Err(crate::NC_E_BUSY),
            full if !writer && full & BOX_READERS == BOX_READERS => return Err(crate::NC_E_BUSY),
            live if writer => live | BOX_WRITER,
            live => live + 1,
        };
        match header.state.compare_exchange_weak(state, next, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => return Ok(control),
            Err(observed) => state = observed,
        }
    }
}
impl BoxRead {
    /// Acquires shared access, rejecting destruction and conflicting mutable calls.
    ///
    /// # Safety
    /// `control` obeys the retained exclusive storage contract of `borrow_box`.
    pub unsafe fn enter(control: *mut NcWeakControl) -> Result<Self, NcStatus> {
        Ok(Self { control: unsafe { borrow_box(control, false) }? })
    }
}
impl BoxWrite {
    /// Acquires mutable access only when the live object has no active borrowers.
    ///
    /// # Safety
    /// `control` obeys the retained exclusive storage contract of `borrow_box`.
    pub unsafe fn enter(control: *mut NcWeakControl) -> Result<Self, NcStatus> {
        Ok(Self { control: unsafe { borrow_box(control, true) }? })
    }
}
impl Drop for BoxRead {
    /// Publishes completion to the waiting owner without clearing its terminal bit.
    fn drop(&mut self) {
        // SAFETY: admission protects T; the caller retains header storage. Nothing
        // accesses T after this release, which may unblock its owning destructor.
        let previous = unsafe { self.control.as_ref() }.state.fetch_sub(1, Ordering::Release);
        debug_assert!(previous & BOX_READERS != 0 && previous & BOX_WRITER == 0);
    }
}
impl Drop for BoxWrite {
    /// Releases only mutable access, preserving a concurrent owner's stop transition.
    fn drop(&mut self) {
        // A store of zero would resurrect or erase ownership state. Clear exactly
        // the writer bit and publish all implementation writes before destruction.
        let previous = unsafe { self.control.as_ref() }.state.fetch_and(!BOX_WRITER, Ordering::Release);
        debug_assert!(previous & BOX_WRITER != 0 && previous & BOX_READERS == 0);
    }
}

impl NcWeakControl {
    /// Observes whether an exclusive owner still accepts calls. This is a snapshot,
    /// not a borrow: provider entry must independently acquire checked access.
    pub fn box_is_alive(&self) -> bool {
        self.state.load(Ordering::Acquire) & (BOXED | BOX_OWNER) == BOXED | BOX_OWNER
    }

    /// Observes an idle exclusive implementation without calling provider code.
    /// Thread-local tree removal uses this before committing a fallible edit to
    /// reject recursive destruction. Concurrent owners still synchronize in Drop.
    pub fn box_is_idle(&self) -> bool {
        self.state.load(Ordering::Acquire) == BOXED | BOX_OWNER
    }
}

/// Releases the unique owner and destroys its value before returning.
///
/// New calls fail as soon as ownership is cleared. Calls already executing on
/// other threads finish before destruction. The implicit weak keeps storage alive
/// through T's destructor, including destruction of weak-to-self fields.
///
/// # Safety
/// Transfers the sole owner exactly once. The caller must not be executing a
/// method/borrow of this object, or hold a resource required by an active call:
/// waiting on such a call would deadlock. UI self-removal uses queued requests.
#[inline(never)]
pub unsafe extern "C" fn nc_box_release(control: *mut NcWeakControl) {
    let Some(header) = (unsafe { control.as_ref() }) else { return };
    let previous = header.state.fetch_and(!BOX_OWNER, Ordering::AcqRel);
    assert!(previous & (BOXED | BOX_OWNER) == BOXED | BOX_OWNER, "exclusive owner released twice");
    // Destruction belongs only to this owner. Borrow release never runs a
    // destructor, so returning from Drop proves the entire owned value is gone.
    while header.state.load(Ordering::Acquire) & BOX_BORROWS != 0 {
        core::hint::spin_loop();
        #[cfg(feature = "std")]
        std::thread::yield_now();
    }
    unsafe {
        ((*header.class).destroy_value)(control);
        nc_weak_release(control);
    }
}

/// Resolves a boxed interface without creating ownership or touching its value.
/// Class metadata stays available until the final weak reference is released.
///
/// # Safety
/// The common header and requested contract are readable and retained for this call.
pub unsafe fn box_projection(control: *mut NcWeakControl, requested: &NcInterfaceId) -> Result<*const IUnknownVTable, NcStatus> {
    let header = unsafe { control.as_ref() }.ok_or(NC_E_POINTER)?;
    if header.state.load(Ordering::Acquire) & BOXED == 0 { return Err(NC_E_INVALID_ARG); }
    unsafe { projection(header, requested) }
}

#[cfg(test)]
mod tests;
