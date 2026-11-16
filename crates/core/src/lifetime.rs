//! Shared and boxed lifetime algorithms over distinct C allocation headers.
//!
//! WIP ABI v3: these headers and callbacks are provisional, not frozen.
//! See [`crate::NC_COM_CORE_ABI_VERSION`] for the current candidate generation.
//!
//! Shared strong counts contain only counts. Boxed owner/access bits belong to
//! their own header. Immutable type information selects the policy at raw ABI
//! admission; no ownership-kind tag is packed into either mutable state word.
//! Concrete provider storage and allocator callbacks stay in their owning binary.
use core::{
    ffi::c_void,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering, fence},
};
use crate::{ComRaw, IUnknown, IUnknownVTable, NcInterfaceId, NcStatus, NC_E_DROPPED, NC_E_POINTER, NC_E_INVALID_ARG, NC_OK};

/// Immutable type-information tag for shared strong-reference ownership.
pub const NC_OWNERSHIP_SHARED: u32 = 1;
/// Immutable type-information tag for one boxed owner with checked call admission.
pub const NC_OWNERSHIP_BOXED: u32 = 2;

/// Immutable ownership, cleanup callbacks and interface tables for one implementation.
///
/// Both lifetime headers begin with a pointer to this record. Raw operations
/// inspect that pointer and the integer ownership tag before borrowing the
/// complete policy-specific header. Unknown tags are rejected by fallible
/// admission; valid owned lifetime tokens always have one of the two known tags.
/// A raw object address must be aligned/readable for its leading metadata pointer,
/// whose nonnull target is a readable current-core record. A known ownership kind
/// promises its matching complete header. Storage, metadata and code remain
/// retained for each operation and any borrow it returns.
///
/// Metadata and code remain live through the final weak release. Cleanup
/// callbacks receive the allocation address, whose concrete type their provider
/// owns. They must not unwind, and deallocation must not destroy the value twice.
#[repr(C)]
pub struct NcObjectTypeInfo {
    /// Exactly NC_OWNERSHIP_SHARED or NC_OWNERSHIP_BOXED; never mutable access flags.
    pub ownership: u32,
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
    /// Immutable metadata pointer at the common ABI admission offset.
    type_info: *const NcObjectTypeInfo,
    /// Number of shared owners; no ownership-kind or reader/writer flag bits.
    strong_count: AtomicUsize,
    /// Explicit weak owners plus the implicit reference through value destruction.
    weak_count: AtomicUsize,
}
impl NcSharedLifetimeState {
    /// Starts one shared owner and one implicit weak reference before publication.
    pub const fn new(type_info: &'static NcObjectTypeInfo) -> Self {
        assert!(type_info.ownership == NC_OWNERSHIP_SHARED, "shared lifetime metadata required");
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
    /// Immutable metadata pointer at the common ABI admission offset.
    type_info: *const NcObjectTypeInfo,
    /// Sole-owner presence, writer presence and reader count, updated atomically.
    owner_and_borrows: AtomicUsize,
    /// Explicit observers plus the implicit reference through value destruction.
    weak_count: AtomicUsize,
}
impl NcBoxedLifetimeState {
    /// Starts the sole owner and one implicit weak reference before publication.
    pub const fn new(type_info: &'static NcObjectTypeInfo) -> Self {
        assert!(type_info.ownership == NC_OWNERSHIP_BOXED, "boxed lifetime metadata required");
        Self {
            type_info,
            owner_and_borrows: AtomicUsize::new(BOX_OWNER),
            weak_count: AtomicUsize::new(1),
        }
    }
}

/// Reads only the common immutable metadata pointer, then admits its ownership tag.
///
/// # Safety
/// A nonnull object is aligned/readable for its leading metadata pointer. Its
/// nonnull metadata is a readable current-core record and remains valid for the
/// chosen borrow. A known kind promises the corresponding complete header.
unsafe fn object_type_info<'a>(object: *mut c_void) -> Result<&'a NcObjectTypeInfo, NcStatus> {
    if object.is_null() {
        return Err(NC_E_POINTER);
    }
    // SAFETY: both policy-specific C records begin with exactly this pointer.
    let pointer = unsafe { object.cast::<*const NcObjectTypeInfo>().read() };
    let info = unsafe { pointer.as_ref() }.ok_or(NC_E_POINTER)?;
    match info.ownership {
        NC_OWNERSHIP_SHARED | NC_OWNERSHIP_BOXED => Ok(info),
        _ => Err(NC_E_INVALID_ARG),
    }
}

/// Admits shared policy before constructing a shared-header reference.
///
/// # Safety
/// Object and metadata satisfy object_type_info for the returned borrow.
unsafe fn shared_state<'a>(object: *mut c_void) -> Result<&'a NcSharedLifetimeState, NcStatus> {
    let info = unsafe { object_type_info(object) }?;
    if info.ownership != NC_OWNERSHIP_SHARED {
        return Err(NC_E_INVALID_ARG);
    }
    // SAFETY: admitted metadata selects this complete current-core representation.
    Ok(unsafe { &*object.cast::<NcSharedLifetimeState>() })
}

/// Admits boxed policy before constructing a boxed-header reference.
///
/// # Safety
/// Object and metadata satisfy object_type_info for the returned borrow.
unsafe fn boxed_state<'a>(object: *mut c_void) -> Result<&'a NcBoxedLifetimeState, NcStatus> {
    let info = unsafe { object_type_info(object) }?;
    if info.ownership != NC_OWNERSHIP_BOXED {
        return Err(NC_E_INVALID_ARG);
    }
    // SAFETY: admitted metadata selects this complete current-core representation.
    Ok(unsafe { &*object.cast::<NcBoxedLifetimeState>() })
}

/// Selects only weak bookkeeping shared by the two concrete ownership policies.
///
/// # Safety
/// The allocation has valid immutable current-core metadata and is retained for
/// the returned borrows. No reference to an application value is constructed.
unsafe fn weak_parts<'a>(object: *mut c_void) -> (&'a AtomicUsize, &'a NcObjectTypeInfo) {
    let info = unsafe { object_type_info(object) }.expect("valid lifetime metadata");
    let count = match info.ownership {
        NC_OWNERSHIP_SHARED => {
            // SAFETY: immutable metadata admitted exactly this header representation.
            let header = unsafe { &*object.cast::<NcSharedLifetimeState>() };
            &header.weak_count
        }
        NC_OWNERSHIP_BOXED => {
            // SAFETY: immutable metadata admitted exactly this header representation.
            let header = unsafe { &*object.cast::<NcBoxedLifetimeState>() };
            &header.weak_count
        }
        _ => unreachable!("ownership was admitted"),
    };
    (count, info)
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
/// Object/metadata satisfy NcObjectTypeInfo raw admission; accepted shared storage
/// has a retained strong owner throughout this call.
unsafe extern "C" fn nc_shared_retain(object: *mut c_void) -> u32 {
    let Ok(header) = (unsafe { shared_state(object) }) else { return 0 };
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
        let header = unsafe { shared_state(object) }.expect("shared lifetime metadata");
        let previous = header.strong_count.fetch_sub(1, Ordering::Release);
        assert!(previous > 0, "COM strong release without an owner");
        match previous {
            1 => {
                fence(Ordering::Acquire);
                // The implicit weak protects storage during reentrant weak-field Drop.
                unsafe { ((*header.type_info).destroy_value)(object) };
                unsafe { weak_release(object) };
                0
            }
            _ => (previous - 1).min(u32::MAX as usize) as u32,
        }
    }
}

/// Retains a weak reference in either policy, including after value destruction.
///
/// # Safety
/// A nonnull allocation is retained and has matching current-core metadata.
/// Null is accepted as a no-op for foreign cleanup.
#[inline(never)]
pub(crate) unsafe fn weak_retain(object: *mut c_void) {
    if !object.is_null() {
        let (count, _) = unsafe { weak_parts(object) };
        retain(count).unwrap_or_else(|| counter_exhausted());
    }
}

/// Releases weak storage; the final reference deallocates in its provider.
///
/// # Safety
/// A nonnull address transfers one weak owner. If it is final, the value's
/// destructor has already returned. Null is a no-op. Do not access freed storage.
#[inline(never)]
pub(crate) unsafe fn weak_release(object: *mut c_void) {
    if object.is_null() {
        return;
    }
    let (count, info) = unsafe { weak_parts(object) };
    let previous = count.fetch_sub(1, Ordering::Release);
    assert!(previous > 0, "COM weak release without an owner");
    if previous == 1 {
        fence(Ordering::Acquire);
        unsafe { (info.deallocate)(object) };
    }
}

/// Creates one shared weak token while the value remains strongly retained.
///
/// # Safety
/// Nonnull object metadata/header and distinct output storage are valid for this
/// call. Boxed policy is rejected before any shared count is interpreted.
#[inline(never)]
unsafe extern "C" fn nc_shared_downgrade(object: *mut c_void, out: *mut crate::NcWeakRaw) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = crate::NcWeakRaw::null() };
    if let Err(status) = unsafe { shared_state(object) } {
        return status;
    }
    unsafe {
        weak_retain(object);
        *out = crate::NcWeakRaw { control: object };
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

/// Checks a shared weak contract without promotion or value access.
///
/// # Safety
/// The retained allocation and current-core metadata satisfy object_type_info.
/// Exact mismatch and ownership errors create no additional reference.
pub(crate) unsafe fn check_shared_weak_cast(object: *mut c_void, requested: &NcInterfaceId) -> Result<(), NcStatus> {
    let header = unsafe { shared_state(object) }?;
    unsafe { try_cast(&*header.type_info, requested) }.map(|_| ())
}

/// Selects a shared interface and transfers exactly one strong owner.
///
/// # Safety
/// Nonnull object follows [`NcObjectTypeInfo`] raw admission; accepted
/// shared storage has a retained strong owner. Request/output records are valid
/// and distinct. Other policies are rejected without acquiring an owner.
#[inline(never)]
unsafe extern "C" fn nc_shared_query_interface(object: *mut c_void, requested: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = ComRaw::null() };
    if requested.is_null() {
        return NC_E_POINTER;
    }
    let header = match unsafe { shared_state(object) } {
        Ok(header) => header,
        Err(status) => return status,
    };
    match unsafe { try_cast(&*header.type_info, &*requested) } {
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
unsafe extern "C" fn reject_shared_downgrade(object: *mut c_void, out: *mut crate::NcWeakRaw) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = crate::NcWeakRaw::null() };
    if object.is_null() { NC_E_POINTER } else { NC_E_INVALID_ARG }
}

/// Atomically promotes shared ownership before exact interface negotiation.
///
/// # Safety
/// A nonnull allocation is weak-retained through the call; request/output storage
/// is valid and distinct. Failure clears output. Expiration precedes negotiation.
#[inline(never)]
pub(crate) unsafe fn weak_upgrade(object: *mut c_void, requested: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = ComRaw::null() };
    if requested.is_null() {
        return NC_E_POINTER;
    }
    let header = match unsafe { shared_state(object) } {
        Ok(header) => header,
        Err(status) => return status,
    };
    let mut count = header.strong_count.load(Ordering::Relaxed);
    loop {
        if count == 0 {
            return NC_E_DROPPED;
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
    match unsafe { try_cast(&*header.type_info, &*requested) } {
        Ok(vtable) => {
            unsafe { *out = ComRaw { object, vtable } };
            NC_OK
        }
        Err(status) => {
            unsafe { NcSharedLifetimeState::release(object.cast()) };
            status
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
/// A nonnull address and metadata satisfy object_type_info. A known kind
/// promises its matching complete header, even if it is rejected as non-boxed.
/// On successful boxed admission the caller retains storage through the complete
/// returned guard/drop call. Null is rejected without accessing storage.
#[inline(never)]
unsafe fn borrow_box(control: *mut NcBoxedLifetimeState, writer: bool) -> Result<NonNull<NcBoxedLifetimeState>, NcStatus> {
    // Admit immutable ownership metadata before interpreting boxed access bits.
    unsafe { boxed_state(control.cast()) }?;
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
    /// `control` follows [`NcObjectTypeInfo`] raw admission. Retain matching storage
    /// through the complete returned guard/drop call after successful admission.
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
    /// `control` follows [`NcObjectTypeInfo`] raw admission. Retain matching storage
    /// through the complete returned guard/drop call after successful admission.
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
        let header = unsafe { boxed_state(control.cast()) }.expect("boxed lifetime metadata");
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
            weak_release(control.cast());
        }
    }
}

/// Tries an exclusive interface cast, returning its table without retaining ownership.
/// Retained class metadata permits this cast even after the value is destroyed.
///
/// # Safety
/// Object follows [`NcObjectTypeInfo`] raw admission; the request is readable.
/// Retain storage/metadata through the call, including for expired boxed values.
/// Non-boxed kinds are rejected before a complete boxed header is borrowed.
pub unsafe fn try_box_cast(control: *mut NcBoxedLifetimeState, requested: &NcInterfaceId) -> Result<*const IUnknownVTable, NcStatus> {
    // SAFETY: only the common immutable pointer is read before ownership admission.
    let header = unsafe { boxed_state(control.cast()) }?;
    // SAFETY: retained boxed storage keeps its metadata and interface tables live.
    unsafe { try_cast(&*header.type_info, requested) }
}

#[cfg(test)]
mod tests;
