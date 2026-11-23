//! Shared and boxed lifetime algorithms over distinct C allocation headers.
//!
//! WIP ABI v4: these headers and callbacks are provisional, not frozen.
//! See [`crate::NC_COM_CORE_ABI_VERSION`] for the current candidate generation.
//!
//! Shared strong counts contain only counts. Boxed owner/access bits belong to
//! their own header. Construction and each ABI ownership contract establish the
//! matching state. No allocation metadata or mutable counter carries a kind tag.
//! Concrete provider storage and allocator callbacks stay in their owning binary.
use core::{
    ffi::c_void,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering, fence},
};
use crate::{ComRaw, IUnknown, IUnknownVTable, NcInterfaceId, NcStatus, NC_E_DROPPED, NC_E_POINTER, NC_E_INVALID_ARG, NC_OK};

/// Immutable cleanup callbacks and interface tables for one implementation.
///
/// Both typed lifetime states refer to this record. Their ownership policy is
/// established by construction or the ABI argument/result contract, never by a
/// runtime tag. The provider pairs these callbacks with its exact concrete storage.
/// Metadata and code remain valid through final weak release. Cleanup receives
/// the allocation address, must not unwind and must free in its allocating binary.
#[repr(C)]
pub struct NcObjectTypeInfo {
    /// Destroys the initialized value once, while the implicit weak retains storage.
    pub destroy_value: unsafe extern "C" fn(*mut c_void),
    /// Frees storage in its allocating provider after value destruction has returned.
    pub deallocate: unsafe extern "C" fn(*mut c_void),
    /// Permanent nonempty array of exact interface tables, including IUnknown.
    pub interfaces: *const *const IUnknownVTable,
    /// Number of initialized entries in the interface table array.
    pub interface_count: usize,
}
// SAFETY: immutable type information grants no mutable application-value access.
unsafe impl Sync for NcObjectTypeInfo {}

/// C allocation header for shared ownership, at offset zero of its storage.
///
/// The strong count is purely numeric: zero is terminal and cannot be promoted.
/// Weak owners keep this header and its metadata alive after value destruction.
/// One implicit weak reference remains until the value destructor has returned.
#[repr(C)]
pub struct NcSharedLifetimeState {
    /// Immutable provider metadata for this statically selected lifetime state.
    type_info: *const NcObjectTypeInfo,
    /// Number of shared owners; no ownership-kind or reader/writer flag bits.
    strong_count: AtomicUsize,
    /// Explicit weak owners plus the implicit reference through value destruction.
    weak_count: AtomicUsize,
}
impl NcSharedLifetimeState {
    /// Starts one shared owner and one implicit weak reference before publication.
    pub const fn new(type_info: &'static NcObjectTypeInfo) -> Self {
        Self {
            type_info,
            strong_count: AtomicUsize::new(1),
            weak_count: AtomicUsize::new(1),
        }
    }
}

/// C allocation header for a boxed owner, at offset zero of its storage.
///
/// Owner presence and admitted calls share one atomic word so destruction cannot
/// race a separate access gate. This word contains no shared strong count or
/// ownership-kind discriminator. Clearing the owner bit rejects new calls;
/// the dropping owner waits for admitted calls and destroys the value itself.
#[repr(C)]
pub struct NcBoxedLifetimeState {
    /// Immutable provider metadata for this statically selected lifetime state.
    type_info: *const NcObjectTypeInfo,
    /// Sole-owner presence, writer presence and reader count, updated atomically.
    owner_and_borrows: AtomicUsize,
    /// Explicit observers plus the implicit reference through value destruction.
    weak_count: AtomicUsize,
}
impl NcBoxedLifetimeState {
    /// Starts the sole owner and one implicit weak reference before publication.
    pub const fn new(type_info: &'static NcObjectTypeInfo) -> Self {
        Self {
            type_info,
            owner_and_borrows: AtomicUsize::new(BOX_OWNER),
            weak_count: AtomicUsize::new(1),
        }
    }
}

/// Increments an owned numeric counter without wrapping or reviving zero.
///
/// Zero and exhaustion return None before mutation. Owning operations convert
/// this result into fail-stop termination, including ordinary Rust weak clones.
/// Reader-count exhaustion instead rejects call admission.
fn retain(counter: &AtomicUsize) -> Option<usize> {
    let mut current = counter.load(Ordering::Relaxed);
    loop {
        if current == 0 || current >= isize::MAX as usize {
            return None;
        }
        match counter.compare_exchange_weak(current, current + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Some(current + 1),
            Err(observed) => current = observed,
        }
    }
}

/// Counter exhaustion must terminate through a non-unwinding boundary even for
/// ordinary Rust weak-handle clones. This cold local function is not a C API.
#[cold]
extern "C" fn counter_exhausted() -> ! {
    panic!("COM lifetime counter exhausted or unowned")
}

impl NcSharedLifetimeState {
    /// Adds one numeric shared owner without changing ownership policy.
    ///
    /// # Safety
    /// The caller retains a live shared owner throughout this operation.
    pub(crate) unsafe fn retain(&self) -> u32 {
        retain(&self.strong_count).unwrap_or_else(|| counter_exhausted()).min(u32::MAX as usize) as u32
    }
}

/// C adapter for the shared prefix; other policies never reach their counters.
///
/// # Safety
/// A nonnull object is a genuine shared allocation paired with the shared
/// prefix. A live strong owner retains its matching header and provider code.
unsafe extern "C" fn nc_shared_retain(object: *mut c_void) -> u32 {
    let Some(header) = (unsafe { object.cast::<NcSharedLifetimeState>().as_ref() }) else {
        return 0;
    };
    unsafe { header.retain() }
}

/// C adapter consuming exactly one shared owner through its typed Rust operation.
///
/// # Safety
/// A nonnull object transfers one valid shared owner exactly once; its immutable
/// metadata, callback code and matching complete header remain valid through release.
unsafe extern "C" fn nc_shared_release(object: *mut c_void) -> u32 {
    unsafe { NcSharedLifetimeState::release(object.cast()) }
}

/// C adapter consuming the sole exclusive owner; the common slot returns zero.
///
/// # Safety
/// A nonnull object transfers its sole valid boxed owner and obeys the non-reentrant
/// destruction/storage obligations of NcBoxedLifetimeState::release.
unsafe extern "C" fn nc_exclusive_release(object: *mut c_void) -> u32 {
    unsafe { NcBoxedLifetimeState::release(object.cast()) };
    0
}

impl NcSharedLifetimeState {
    /// Consumes one shared owner and destroys the value on the final release.
    ///
    /// # Safety
    /// A nonnull address transfers one valid owner in the current-core lifetime ABI.
    /// The caller must not use that owner after this call. The raw typed pointer
    /// avoids a protected &self argument across final deallocation.
    pub(crate) unsafe fn release(control: *mut Self) -> u32 {
        let object = control.cast::<c_void>();
        if object.is_null() {
            return 0;
        }
        let header = unsafe { &*control };
        let previous = header.strong_count.fetch_sub(1, Ordering::Release);
        assert!(previous > 0, "COM strong release without an owner");
        match previous {
            1 => {
                fence(Ordering::Acquire);
                // The implicit weak protects storage during reentrant weak-field Drop.
                unsafe { ((*header.type_info).destroy_value)(object) };
                unsafe { Self::weak_release(control) };
                0
            }
            _ => (previous - 1).min(u32::MAX as usize) as u32,
        }
    }
}

/// Decrements numeric weak ownership and synchronizes final provider cleanup.
/// The counter borrow ends on return, before its allocation can be deallocated.
fn release_weak_count(count: &AtomicUsize) -> bool {
    let previous = count.fetch_sub(1, Ordering::Release);
    assert!(previous > 0, "COM weak release without an owner");
    if previous == 1 {
        fence(Ordering::Acquire);
        true
    } else {
        false
    }
}

/// Creates one shared weak token while the value remains strongly retained.
///
/// # Safety
/// A nonnull object matches the shared prefix and is strongly retained shared
/// storage. A nonnull output is distinct, aligned and writable for this call.
#[inline(never)]
unsafe extern "C" fn nc_shared_downgrade(object: *mut c_void, out: *mut crate::NcSharedWeakRaw) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = crate::NcSharedWeakRaw::null() };
    let Some(header) = (unsafe { object.cast::<NcSharedLifetimeState>().as_ref() }) else {
        return NC_E_POINTER;
    };
    unsafe {
        header.weak_retain();
        *out = crate::NcSharedWeakRaw { control: object.cast() };
    }
    NC_OK
}

/// Selects an exact interface table from already-admitted immutable metadata.
///
/// # Safety
/// Metadata and every table remain valid through this lookup.
unsafe fn try_cast(info: &NcObjectTypeInfo, requested: &NcInterfaceId) -> Result<*const IUnknownVTable, NcStatus> {
    let tables = unsafe { core::slice::from_raw_parts(info.interfaces, info.interface_count) };
    unsafe { crate::__nc_find_interface(tables, requested) }
}

impl NcSharedLifetimeState {
    /// Looks up an exact interface under an established shared storage proof.
    /// No promotion, policy admission or application-value access is performed.
    ///
    /// # Safety
    /// The matching shared allocation retains valid metadata/tables through lookup.
    pub(crate) unsafe fn try_cast(&self, requested: &NcInterfaceId) -> Result<*const IUnknownVTable, NcStatus> {
        unsafe { try_cast(&*self.type_info, requested) }
    }
}

/// Selects a shared interface and transfers exactly one strong owner.
///
/// # Safety
/// A nonnull object is strongly retained shared storage paired with the shared
/// prefix. Request/output records are readable/writable as appropriate and
/// distinct. The matching provider contract already establishes ownership policy.
#[inline(never)]
unsafe extern "C" fn nc_shared_query_interface(object: *mut c_void, requested: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = ComRaw::null() };
    if requested.is_null() {
        return NC_E_POINTER;
    }
    let Some(header) = (unsafe { object.cast::<NcSharedLifetimeState>().as_ref() }) else {
        return NC_E_POINTER;
    };
    match unsafe { header.try_cast(&*requested) } {
        Ok(vtable) => {
            unsafe {
                header.retain();
                *out = ComRaw { object, vtable };
            }
            NC_OK
        }
        Err(status) => status,
    }
}

/// Shared ownership's canonical root prefix; all shared classes reuse its callbacks.
/// Generated interface prefixes copy these slots and replace only interface_id.
pub static SHARED_UNKNOWN_VTABLE: IUnknownVTable = IUnknownVTable {
    interface_id: &IUnknown::INTERFACE_ID,
    query_interface: nc_shared_query_interface,
    add_ref: nc_shared_retain,
    release: nc_shared_release,
    downgrade: Some(nc_shared_downgrade),
};

/// Exclusive ownership's canonical root prefix, with no shared-owner acquisition.
/// Release destroys synchronously; shared query/downgrade preserve their rejection status.
pub static EXCLUSIVE_UNKNOWN_VTABLE: IUnknownVTable = IUnknownVTable {
    interface_id: &IUnknown::INTERFACE_ID,
    query_interface: reject_shared_query,
    add_ref: reject_shared_retain,
    release: nc_exclusive_release,
    downgrade: Some(reject_shared_downgrade),
};

/// Rejects shared-owner acquisition without reading object storage or creating an owner.
unsafe extern "C" fn reject_shared_retain(_object: *mut c_void) -> u32 {
    0
}

/// Clears a shared query output and rejects exclusive ownership without table lookup.
///
/// # Safety
/// A nonnull output is aligned/writable for one raw pair and distinct from inputs.
/// Other nonnull arguments follow the selected exclusive vtable prefix contract.
unsafe extern "C" fn reject_shared_query(object: *mut c_void, requested: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = ComRaw::null() };
    if object.is_null() || requested.is_null() {
        NC_E_POINTER
    } else {
        NC_E_INVALID_ARG
    }
}

/// Exclusive observers do not implement shared weak promotion; preserve invalid-policy errors.
///
/// # Safety
/// A nonnull output is aligned/writable for one token; object follows the selected
/// exclusive prefix contract. No ownership or object value is accessed.
unsafe extern "C" fn reject_shared_downgrade(object: *mut c_void, out: *mut crate::NcSharedWeakRaw) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = crate::NcSharedWeakRaw::null() };
    if object.is_null() { NC_E_POINTER } else { NC_E_INVALID_ARG }
}

impl NcSharedLifetimeState {
    /// Acquires a shared owner before exact interface negotiation. Expiration
    /// returns None before lookup; a lookup error releases the acquired owner.
    /// This is local Rust code, not a foreign status/output handshake.
    ///
    /// # Safety
    /// The nonnull pointer is genuine shared storage retained by a weak token
    /// throughout this operation. Metadata/code and threading proofs remain valid.
    #[inline(never)]
    pub(crate) unsafe fn upgrade(control: *mut Self, requested: &NcInterfaceId) -> Result<Option<ComRaw<IUnknown>>, NcStatus> {
        let header = unsafe { &*control };
        let mut count = header.strong_count.load(Ordering::Relaxed);
        loop {
            if count == 0 {
                return Ok(None);
            }
            if count >= isize::MAX as usize {
                counter_exhausted();
            }
            match header
                .strong_count
                .compare_exchange_weak(count, count + 1, Ordering::Acquire, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(observed) => count = observed,
            }
        }
        // Test-only scheduling models another owner's release after the acquire
        // and before negotiation. It adds no callback or state to production.
        #[cfg(test)]
        tests::RELEASE_OWNER_AFTER_PROMOTION.with(|release| {
            if release.replace(false) {
                unsafe { Self::release(control) };
            }
        });
        match unsafe { header.try_cast(requested) } {
            Ok(vtable) => Ok(Some(ComRaw { object: control.cast(), vtable })),
            Err(status) => {
                // This acquired owner may now be final. Consume a raw pointer,
                // preserving the implicit weak through the entire destructor.
                unsafe { Self::release(control) };
                Err(status)
            }
        }
    }
}

/// Boxed owner presence shares one word with admission, but carries no kind tag.
const BOX_OWNER: usize = 1 << (usize::BITS - 1);
/// Exclusive writer presence excludes every reader.
const BOX_WRITER: usize = BOX_OWNER >> 1;
/// Reader count occupies the remaining bits and must not overflow into writer presence.
const BOX_READERS: usize = BOX_WRITER - 1;
/// Outstanding admitted calls keep the dropping owner waiting.
const BOX_BORROWS: usize = BOX_WRITER | BOX_READERS;

/// One shared implementation borrow, acquired atomically with the lifetime check.
/// Only generated provider entry points create these guards. Their destruction
/// releases access; it never destroys the implementation or takes ownership.
pub struct BoxRead {
    /// Retained header whose reader count includes this guard.
    control: NonNull<NcBoxedLifetimeState>,
}

/// One exclusive implementation borrow, using the same state as readers and Drop.
pub struct BoxWrite {
    /// Retained header whose writer bit belongs to this guard.
    control: NonNull<NcBoxedLifetimeState>,
}

/// Atomically admits one reader or the unique writer before any T reference exists.
///
/// # Safety
/// A nonnull pointer is genuine boxed storage with matching immutable metadata.
/// The caller retains storage through the complete returned guard/drop call.
/// Policy is already proven; owner presence and borrow conflicts are not.
/// Null is rejected without accessing storage.
#[inline(never)]
unsafe fn borrow_box(control: *mut NcBoxedLifetimeState, writer: bool) -> Result<NonNull<NcBoxedLifetimeState>, NcStatus> {
    let control = NonNull::new(control).ok_or(NC_E_POINTER)?;
    let header = unsafe { control.as_ref() };
    let mut state = header.owner_and_borrows.load(Ordering::Acquire);
    loop {
        let next = match state {
            expired if expired & BOX_OWNER == 0 => return Err(NC_E_DROPPED),
            busy if busy & BOX_WRITER != 0 => return Err(crate::NC_E_BUSY),
            busy if writer && busy & BOX_READERS != 0 => return Err(crate::NC_E_BUSY),
            full if !writer && full & BOX_READERS == BOX_READERS => return Err(crate::NC_E_BUSY),
            live if writer => live | BOX_WRITER,
            live => live + 1,
        };
        match header
            .owner_and_borrows
            .compare_exchange_weak(state, next, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => return Ok(control),
            Err(observed) => state = observed,
        }
    }
}
impl BoxRead {
    /// Acquires shared access, rejecting destruction and conflicting mutable calls.
    ///
    /// # Safety
    /// A nonnull `control` points to a genuine complete boxed header with valid
    /// metadata/code. A typed pointer cast alone does not prove this allocation
    /// policy. Retain storage through the complete returned guard/drop call.
    /// Null returns NC_E_POINTER; expired/conflicting access is checked atomically.
    pub unsafe fn enter(control: *mut NcBoxedLifetimeState) -> Result<Self, NcStatus> {
        Ok(Self {
            control: unsafe { borrow_box(control, false) }?,
        })
    }
}
impl BoxWrite {
    /// Acquires mutable access only when the live object has no active borrowers.
    ///
    /// # Safety
    /// A nonnull `control` points to a genuine complete boxed header with valid
    /// metadata/code. A typed pointer cast alone does not prove this allocation
    /// policy. Retain storage through the complete returned guard/drop call.
    /// Null returns NC_E_POINTER; expired/conflicting access is checked atomically.
    pub unsafe fn enter(control: *mut NcBoxedLifetimeState) -> Result<Self, NcStatus> {
        Ok(Self {
            control: unsafe { borrow_box(control, true) }?,
        })
    }
}
impl Drop for BoxRead {
    /// Publishes completion to the waiting owner without clearing its terminal bit.
    fn drop(&mut self) {
        // SAFETY: admission protects T; the caller retains header storage. Nothing
        // accesses T after this release, which may unblock its owning destructor.
        let previous = unsafe { self.control.as_ref() }.owner_and_borrows.fetch_sub(1, Ordering::Release);
        debug_assert!(previous & BOX_READERS != 0 && previous & BOX_WRITER == 0);
    }
}
impl Drop for BoxWrite {
    /// Releases only mutable access, preserving a concurrent owner's stop transition.
    fn drop(&mut self) {
        // A store of zero would resurrect or erase ownership state. Clear exactly
        // the writer bit and publish all implementation writes before destruction.
        let previous = unsafe { self.control.as_ref() }.owner_and_borrows.fetch_and(!BOX_WRITER, Ordering::Release);
        debug_assert!(previous & BOX_WRITER != 0 && previous & BOX_READERS == 0);
    }
}

impl NcBoxedLifetimeState {
    /// Observes whether an exclusive owner still accepts calls. This is a snapshot,
    /// not a borrow: provider entry must independently acquire checked access.
    pub fn box_is_alive(&self) -> bool {
        self.owner_and_borrows.load(Ordering::Acquire) & BOX_OWNER != 0
    }

    /// Observes an idle exclusive implementation without calling provider code.
    /// An owner can preflight destruction against reentry. This observation
    /// reserves no access; Drop still waits for calls admitted concurrently.
    pub fn box_is_idle(&self) -> bool {
        self.owner_and_borrows.load(Ordering::Acquire) == BOX_OWNER
    }
}

impl NcBoxedLifetimeState {
    /// Releases the unique owner and destroys its value before returning.
    ///
    /// New calls fail as soon as ownership is cleared. Calls already executing on
    /// other threads finish before destruction. The implicit weak keeps storage alive
    /// through T's destructor, including destruction of weak-to-self fields.
    ///
    /// # Safety
    /// Transfers the sole owner exactly once. The caller must not be executing a
    /// method/borrow of this object, or hold a resource required by an active call:
    /// waiting on such a call would deadlock. Defer destruction until that call returns.
    pub(crate) unsafe fn release(control: *mut Self) {
        if control.is_null() {
            return;
        }
        // SAFETY: the caller transfers a valid boxed owner in the current core ABI.
        let header = unsafe { &*control };
        let previous = header.owner_and_borrows.fetch_and(!BOX_OWNER, Ordering::AcqRel);
        assert!(previous & BOX_OWNER != 0, "exclusive owner released twice");
        // Destruction belongs only to this owner. Borrow release never runs a
        // destructor, so returning from Drop proves the entire owned value is gone.
        while header.owner_and_borrows.load(Ordering::Acquire) & BOX_BORROWS != 0 {
            core::hint::spin_loop();
            #[cfg(feature = "std")]
            std::thread::yield_now();
        }
        unsafe {
            ((*header.type_info).destroy_value)(control.cast());
            Self::weak_release(control);
        }
    }
}

/// Tries an exclusive interface cast, returning its table without retaining ownership.
/// Retained class metadata permits this cast even after the value is destroyed.
///
/// # Safety
/// A nonnull pointer identifies genuine complete boxed storage with matching
/// metadata/tables retained through the call, including after value destruction.
/// Policy must be proven by construction or the ABI producer/unsafe caller;
/// casting a pointer does not prove it. Null is rejected without reading storage.
pub unsafe fn try_box_cast(control: *mut NcBoxedLifetimeState, requested: &NcInterfaceId) -> Result<*const IUnknownVTable, NcStatus> {
    // SAFETY: the caller proves policy; null remains a documented error input.
    let header = unsafe { control.as_ref() }.ok_or(NC_E_POINTER)?;
    // SAFETY: retained boxed storage keeps its metadata and interface tables live.
    unsafe { try_cast(&*header.type_info, requested) }
}

impl NcSharedLifetimeState {
    /// Adds one weak storage reference without accessing or retaining the value.
    ///
    /// # Safety
    /// The matching allocation, metadata and code are retained by a live owner
    /// or weak token throughout this call. Policy was established before entry.
    pub(crate) unsafe fn weak_retain(&self) {
        retain(&self.weak_count).unwrap_or_else(|| counter_exhausted());
    }

    /// Consumes one matching weak reference, freeing storage on the final release.
    ///
    /// # Safety
    /// A nonnull pointer transfers one valid weak token of this policy. Final
    /// release requires that value destruction has returned. Metadata/code remain
    /// valid through provider-local deallocation; do not access storage afterwards.
    /// The raw argument avoids protecting a reference across its deallocation.
    pub(crate) unsafe fn weak_release(control: *mut Self) {
        if control.is_null() {
            return;
        }
        // The arithmetic borrow returns before a callback can free its backing.
        if release_weak_count(unsafe { &(*control).weak_count }) {
            let deallocate = unsafe { (*(*control).type_info).deallocate };
            unsafe { deallocate(control.cast()) };
        }
    }
}

impl NcBoxedLifetimeState {
    /// Adds one weak storage reference without accessing or retaining the value.
    ///
    /// # Safety
    /// The matching allocation, metadata and code are retained by a live owner
    /// or weak token throughout this call. Policy was established before entry.
    pub(crate) unsafe fn weak_retain(&self) {
        retain(&self.weak_count).unwrap_or_else(|| counter_exhausted());
    }

    /// Consumes one matching weak reference, freeing storage on the final release.
    ///
    /// # Safety
    /// A nonnull pointer transfers one valid weak token of this policy. Final
    /// release requires that value destruction has returned. Metadata/code remain
    /// valid through provider-local deallocation; do not access storage afterwards.
    /// The raw argument avoids protecting a reference across its deallocation.
    pub(crate) unsafe fn weak_release(control: *mut Self) {
        if control.is_null() {
            return;
        }
        // The arithmetic borrow returns before a callback can free its backing.
        if release_weak_count(unsafe { &(*control).weak_count }) {
            let deallocate = unsafe { (*(*control).type_info).deallocate };
            unsafe { deallocate(control.cast()) };
        }
    }
}

#[cfg(test)]
mod tests;
