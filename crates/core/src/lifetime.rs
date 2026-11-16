//! Shared and boxed lifetime algorithms over distinct C allocation headers.
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
/// Non-unwinding C callers fail-stop on exhaustion rather than publish an
/// uncounted owner. Reader-count exhaustion instead rejects call admission.
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

/// Retains a shared owner. Null and boxed ownership return zero.
///
/// # Safety
/// A nonnull allocation has current-core metadata and remains strongly owned
/// throughout this operation. A zero shared strong count cannot be resurrected.
#[inline(never)]
pub unsafe extern "C" fn nc_strong_retain(object: *mut c_void) -> u32 {
    let Ok(header) = (unsafe { shared_state(object) }) else { return 0 };
    retain(&header.strong_count).min(u32::MAX as usize) as u32
}

/// Releases shared ownership, or dispatches the sole boxed owner to boxed Drop.
///
/// # Safety
/// A nonnull address transfers one valid owner in the current-core lifetime ABI.
/// The caller must not use that owner after this call.
#[inline(never)]
pub unsafe extern "C" fn nc_strong_release(object: *mut c_void) -> u32 {
    if object.is_null() {
        return 0;
    }
    let info = unsafe { object_type_info(object) }.expect("valid owned lifetime metadata");
    if info.ownership == NC_OWNERSHIP_BOXED {
        unsafe { nc_box_release(object.cast()) };
        return 0;
    }
    let header = unsafe { shared_state(object) }.expect("shared lifetime metadata");
    let previous = header.strong_count.fetch_sub(1, Ordering::Release);
    assert!(previous > 0, "COM strong release without an owner");
    match previous {
        1 => {
            fence(Ordering::Acquire);
            // The implicit weak protects storage during reentrant weak-field Drop.
            unsafe { (info.destroy_value)(object) };
            unsafe { nc_weak_release(object) };
            0
        }
        _ => (previous - 1).min(u32::MAX as usize) as u32,
    }
}

/// Retains a weak reference in either policy, including after value destruction.
///
/// # Safety
/// A nonnull allocation is retained and has matching current-core metadata.
/// Null is accepted as a no-op for foreign cleanup.
#[inline(never)]
pub unsafe extern "C" fn nc_weak_retain(object: *mut c_void) {
    if !object.is_null() {
        let (count, _) = unsafe { weak_parts(object) };
        retain(count);
    }
}

/// Releases weak storage; the final reference deallocates in its provider.
///
/// # Safety
/// A nonnull address transfers one weak owner. If it is final, the value's
/// destructor has already returned. Null is a no-op. Do not access freed storage.
#[inline(never)]
pub unsafe extern "C" fn nc_weak_release(object: *mut c_void) {
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
pub unsafe extern "C" fn nc_downgrade(object: *mut c_void, out: *mut crate::NcWeakRaw) -> NcStatus {
    if out.is_null() {
        return NC_E_POINTER;
    }
    unsafe { *out = crate::NcWeakRaw::null() };
    if let Err(status) = unsafe { shared_state(object) } {
        return status;
    }
    unsafe {
        nc_weak_retain(object);
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
/// Nonnull object is retained and request/output records are valid and distinct.
/// Boxed allocations are rejected without acquiring another owner.
#[inline(never)]
pub unsafe extern "C" fn nc_query_interface(object: *mut c_void, requested: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
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
                nc_strong_retain(object);
                *out = ComRaw { object, vtable };
            }
            NC_OK
        }
        Err(status) => status,
    }
}

/// One root table forwards lifetime operations through immutable ownership admission.
pub static CANONICAL_UNKNOWN_VTABLE: IUnknownVTable = IUnknownVTable {
    interface_id: &IUnknown::INTERFACE_ID,
    query_interface: nc_query_interface,
    add_ref: nc_strong_retain,
    release: nc_strong_release,
    downgrade: Some(nc_downgrade),
};

/// Atomically promotes shared ownership before exact interface negotiation.
///
/// # Safety
/// A nonnull allocation is weak-retained through the call; request/output storage
/// is valid and distinct. Failure clears output. Expiration precedes negotiation.
#[inline(never)]
pub unsafe extern "C" fn nc_weak_upgrade(object: *mut c_void, requested: *const NcInterfaceId, out: *mut ComRaw<IUnknown>) -> NcStatus {
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
        assert!(count < isize::MAX as usize, "COM strong promotion counter exhausted");
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
            unsafe { nc_strong_release(object) };
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
/// The caller retains exclusive-class allocation storage throughout acquisition
/// and through the returned borrow. Null is rejected without accessing storage.
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
    /// `control` obeys the retained exclusive storage contract of `borrow_box`.
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
    /// `control` obeys the retained exclusive storage contract of `borrow_box`.
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
#[inline(never)]
pub unsafe extern "C" fn nc_box_release(control: *mut NcBoxedLifetimeState) {
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
        nc_weak_release(control.cast());
    }
}

/// Tries an exclusive interface cast, returning its table without retaining ownership.
/// Retained class metadata permits this cast even after the value is destroyed.
///
/// # Safety
/// The common header and requested contract are readable and retained for this call.
pub unsafe fn try_box_cast(control: *mut NcBoxedLifetimeState, requested: &NcInterfaceId) -> Result<*const IUnknownVTable, NcStatus> {
    // SAFETY: only the common immutable pointer is read before ownership admission.
    let header = unsafe { boxed_state(control.cast()) }?;
    // SAFETY: retained boxed storage keeps its metadata and interface tables live.
    unsafe { try_cast(&*header.type_info, requested) }
}

#[cfg(test)]
mod tests;
