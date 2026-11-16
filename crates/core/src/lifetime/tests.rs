//! Ownership regressions exercise the shared operations against real typed allocations.
use super::*;
use core::mem::MaybeUninit;
use std::{boxed::Box, sync::Arc};

/// Independent observations survive both value destruction and allocation release.
#[derive(Default)]
struct Observations {
    /// Number of concrete value destructors invoked.
    destroyed: AtomicUsize,
    /// Number of allocator callbacks invoked.
    freed: AtomicUsize,
}
/// A payload with an observable destructor and an owned weak self reference.
struct Value {
    /// External counters avoid reading memory after final release.
    observations: Arc<Observations>,
    /// Optional explicit weak owner released from inside value destruction.
    self_weak: Option<*mut c_void>,
}
impl Drop for Value {
    /// Reentrant weak release must not free the header before this destructor returns.
    fn drop(&mut self) {
        if let Some(control) = self.self_weak.take() {
            unsafe {
                nc_weak_release(control.cast());
            }
            assert_eq!(self.observations.freed.load(Ordering::SeqCst), 0);
        }
        self.observations.destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
/// Header-first storage remains valid after value destruction until final weak release.
#[repr(C)]
struct Allocation<Lifetime = NcSharedLifetimeState> {
    /// Common prefix used without knowing this concrete storage type.
    control: Lifetime,
    /// Destroyed once on final strong release; never dropped by Box<Allocation>.
    value: MaybeUninit<Value>,
    /// Allocation-only observer remains available after value destruction.
    observations: Arc<Observations>,
}
/// Provider-specific destruction knows the exact value layout.
unsafe extern "C" fn destroy<Lifetime>(control: *mut c_void) {
    unsafe {
        (*control.cast::<Allocation<Lifetime>>()).value.assume_init_drop();
    }
}
/// Provider-specific freeing never runs the concrete value destructor twice.
unsafe extern "C" fn free<Lifetime>(control: *mut c_void) {
    let allocation = unsafe { Box::from_raw(control.cast::<Allocation<Lifetime>>()) };
    allocation.observations.freed.fetch_add(1, Ordering::SeqCst);
    drop(allocation);
}
/// Static root projection uses the same shared entry points as generated objects will.
static ROOT: IUnknownVTable = IUnknownVTable {
    interface_id: &crate::IUNKNOWN_INTERFACE_ID,
    query_interface: nc_query_interface,
    add_ref: nc_strong_retain,
    release: nc_strong_release,
    downgrade: None,
};
/// Class data points to one valid root projection without retaining inspection graphs.
static CLASS: NcObjectTypeInfo = NcObjectTypeInfo {
    ownership: NC_OWNERSHIP_SHARED,
    destroy_value: destroy::<NcSharedLifetimeState>,
    deallocate: free::<NcSharedLifetimeState>,
    interfaces: [&ROOT as *const IUnknownVTable].as_ptr(),
    interface_count: 1,
};
/// Allocates a fully initialized provider object with one shared owner.
fn allocation() -> (*mut NcSharedLifetimeState, Arc<Observations>) {
    let observations = Arc::new(Observations::default());
    let storage = Box::new(Allocation {
        control: NcSharedLifetimeState::new(&CLASS),
        value: MaybeUninit::new(Value {
            observations: observations.clone(),
            self_weak: None,
        }),
        observations: observations.clone(),
    });
    (Box::into_raw(storage).cast(), observations)
}

/// Boxed metadata is immutable and independent of the boxed owner/access word.
static BOX_CLASS: NcObjectTypeInfo = NcObjectTypeInfo {
    ownership: NC_OWNERSHIP_BOXED,
    destroy_value: destroy::<NcBoxedLifetimeState>,
    deallocate: free::<NcBoxedLifetimeState>,
    interfaces: [&ROOT as *const IUnknownVTable].as_ptr(),
    interface_count: 1,
};

/// Allocates a boxed value without reinterpreting or mutating shared ownership.
fn boxed_allocation() -> (*mut NcBoxedLifetimeState, Arc<Observations>) {
    let observations = Arc::new(Observations::default());
    let storage = Box::new(Allocation {
        control: NcBoxedLifetimeState::new(&BOX_CLASS),
        value: MaybeUninit::new(Value {
            observations: observations.clone(),
            self_weak: None,
        }),
        observations: observations.clone(),
    });
    (Box::into_raw(storage).cast(), observations)
}

/// Value lifetime ends independently of weak storage, including destructor-owned weak tokens.
#[test]
fn final_strong_drops_value_before_final_weak_frees_storage() {
    let (control, observations) = allocation();
    unsafe {
        nc_weak_retain(control.cast());
        (*control.cast::<Allocation>()).value.assume_init_mut().self_weak = Some(control.cast());
        nc_weak_retain(control.cast()); // External weak reference survives destruction.
        assert_eq!(nc_strong_retain(control.cast()), 2);
        assert_eq!(nc_strong_release(control.cast()), 1);
        assert_eq!(nc_strong_release(control.cast()), 0);
        assert_eq!(observations.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observations.freed.load(Ordering::SeqCst), 0);
        let mut out = ComRaw::null();
        assert_eq!(nc_weak_upgrade(control.cast(), &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_E_DROPPED);
        assert!(!out.is_present());
        nc_weak_retain(control.cast()); // Expired weak references can still be cloned.
        nc_weak_release(control.cast());
        nc_weak_release(control.cast());
    }
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
}

/// Promotion/query failures do not leak counts, and successes each transfer one owner.
#[test]
fn checked_projection_balances_all_ownership_paths() {
    let (control, observations) = allocation();
    let wrong = NcInterfaceId::new(crate::IID_IUNKNOWN, crate::NcAbiDigest::from_static(&[0; 32]));
    unsafe {
        nc_weak_retain(control.cast());
        let mut out = ComRaw { object: control.cast(), vtable: &ROOT };
        assert_eq!(nc_weak_upgrade(control.cast(), &wrong, &mut out), crate::NC_E_ABI_VERSION);
        assert!(!out.is_present());
        assert_eq!(nc_strong_retain(control.cast()), 2);
        assert_eq!(nc_strong_release(control.cast()), 1);
        assert_eq!(nc_weak_upgrade(control.cast(), &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_OK);
        assert_eq!(nc_strong_release(out.object), 1);
        assert_eq!(nc_query_interface(control.cast(), &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_OK);
        assert_eq!(nc_strong_release(out.object), 1);
        assert_eq!(nc_strong_release(control.cast()), 0);
        assert_eq!(nc_weak_upgrade(control.cast(), &wrong, &mut out), NC_E_DROPPED);
        nc_weak_release(control.cast());
    }
    assert_eq!(observations.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
}

/// Output rejection and borrowed acquisition balance weak counts without promotion.
#[test]
fn checked_weak_transport_balances_tokens_before_and_after_destruction() {
    use crate::{ComValue, NcWeakRaw, ValueBudget, WeakComPtr};
    let (control, observed) = allocation();
    // SAFETY: this fixture owns the initial strong reference and retains one
    // explicit weak owner across both iterations and final value destruction.
    unsafe {
        nc_weak_retain(control.cast());
    }
    for expired in [false, true] {
        // SAFETY: the explicit weak owner keeps this header readable throughout.
        let before = unsafe { (*control).weak_count.load(Ordering::Relaxed) };
        unsafe {
            nc_weak_retain(control.cast());
        }
        // SAFETY: transfer the additional weak token with intentionally wrong T.
        assert_eq!(
            unsafe { WeakComPtr::<crate::INcBytesAbi>::__nc_from_raw_owned_checked(NcWeakRaw { control: control.cast() }) }.unwrap_err(),
            crate::NC_E_NOINTERFACE
        );
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        // SAFETY: the source lends a live header; acquisition adds only one weak owner.
        let acquired = unsafe { WeakComPtr::<IUnknown>::decode_value(&NcWeakRaw { control: control.cast() }, &mut ValueBudget::default()) }.unwrap();
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before + 1);
        assert_eq!(unsafe { (*control).strong_count.load(Ordering::Relaxed) }, if expired { 0 } else { 1 });
        drop(acquired);
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        // A rejected conversion neither acquires nor releases the caller's token.
        assert_eq!(
            unsafe { WeakComPtr::<IUnknown>::decode_value(&NcWeakRaw { control: control.cast() }, &mut ValueBudget::new(0, 0, 0)) }.unwrap_err(),
            crate::NC_E_OUT_OF_RANGE
        );
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        if !expired {
            unsafe {
                nc_strong_release(control.cast());
            }
        }
    }
    // SAFETY: release the final explicit weak owner; no header reads follow.
    unsafe {
        nc_weak_release(control.cast());
    }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// The arithmetic helper fails before mutation; the external non-unwinding entry aborts.
#[test]
fn count_exhaustion_never_wraps_or_changes_the_counter() {
    let count = AtomicUsize::new(isize::MAX as usize);
    assert!(std::panic::catch_unwind(|| retain(&count)).is_err());
    assert_eq!(count.load(Ordering::Relaxed), isize::MAX as usize);
}

/// An absent invocation header must fail before touching any provider storage.
#[test]
fn exclusive_call_rejects_an_absent_header() {
    // SAFETY: null is explicitly accepted as an error input by BoxRead::enter.
    assert!(matches!(unsafe { super::BoxRead::enter(core::ptr::null_mut()) }, Err(crate::NC_E_POINTER)));
}

/// The owner keeps allocation storage alive through weak-to-self destruction;
/// the last external weak releases storage without running T's destructor again.
#[test]
fn exclusive_destruction_is_immediate_and_storage_release_is_separate() {
    let (control, observed) = boxed_allocation();
    // SAFETY: this boxed allocation has not escaped or been borrowed. Install
    // one properly retained weak field before transferring its owner.
    unsafe {
        nc_weak_retain(control.cast());
        (*control.cast::<Allocation<NcBoxedLifetimeState>>()).value.assume_init_mut().self_weak = Some(control.cast());
        nc_weak_retain(control.cast());
        nc_box_release(control);
        assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observed.freed.load(Ordering::SeqCst), 0);
        assert!(matches!(BoxRead::enter(control), Err(NC_E_DROPPED)));
        assert!(matches!(BoxWrite::enter(control), Err(NC_E_DROPPED)));
        nc_weak_release(control.cast());
    }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// A full reader count must reject admission without wrapping into the writer bit.
#[test]
fn exclusive_reader_overflow_and_writer_conflicts_preserve_header_state() {
    let header = NcBoxedLifetimeState::new(&BOX_CLASS);
    let raw = core::ptr::from_ref(&header).cast_mut();
    header.owner_and_borrows.store(BOX_OWNER | BOX_READERS, Ordering::Relaxed);
    // SAFETY: both attempts reject before granting access; the test header is
    // retained and no provider destruction is attempted for this stack fixture.
    unsafe {
        assert!(matches!(BoxRead::enter(raw), Err(crate::NC_E_BUSY)));
        assert!(matches!(BoxWrite::enter(raw), Err(crate::NC_E_BUSY)));
    }
    assert_eq!(header.owner_and_borrows.load(Ordering::Relaxed), BOX_OWNER | BOX_READERS);
    header.owner_and_borrows.store(BOX_OWNER, Ordering::Relaxed);
    let read1 = unsafe { BoxRead::enter(raw) }.unwrap();
    let read2 = unsafe { BoxRead::enter(raw) }.unwrap();
    assert!(matches!(unsafe { BoxWrite::enter(raw) }, Err(crate::NC_E_BUSY)));
    drop(read1);
    assert!(!header.box_is_idle());
    drop(read2);
    assert!(header.box_is_idle());
    let write = unsafe { BoxWrite::enter(raw) }.unwrap();
    assert!(matches!(unsafe { BoxRead::enter(raw) }, Err(crate::NC_E_BUSY)));
    drop(write);
    assert!(header.box_is_idle());
}

/// Borrowed exclusive queries select tables without retaining storage or changing owner state.
#[test]
fn borrowed_box_queries_preserve_weak_count_and_explicit_observers_still_expire() {
    let (control, observed) = boxed_allocation();
    // SAFETY: transfer the initialized sole owner and its exact canonical projection.
    let owner = unsafe { crate::ComBox::<IUnknown>::from_raw_owned(ComRaw { object: control.cast(), vtable: &ROOT }) }.unwrap();
    let before = unsafe { (*control).weak_count.load(Ordering::Relaxed) };
    let state = unsafe { (*control).owner_and_borrows.load(Ordering::Relaxed) };
    for _ in 0..16 {
        let projection = owner.query_ref::<IUnknown>().unwrap();
        assert_eq!(projection.as_raw(), owner.as_raw());
        assert!(matches!(owner.query_ref::<crate::INcBytesAbi>(), Err(crate::NC_E_NOINTERFACE)));
        // SAFETY: the owner retains this fixture's readable common header.
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        assert_eq!(unsafe { (*control).owner_and_borrows.load(Ordering::Relaxed) }, state);
    }
    let observer = owner.query_weak::<IUnknown>().unwrap();
    assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before + 1);
    drop(owner);
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 0);
    assert!(!observer.is_alive());
    drop(observer);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// Ownership is admitted from metadata before a counter or access mask is read.
#[test]
fn raw_policy_mismatch_is_rejected_while_live_and_expired() {
    let boxed = NcBoxedLifetimeState::new(&BOX_CLASS);
    let pointer = core::ptr::from_ref(&boxed).cast_mut();
    for state in [BOX_OWNER, 0] {
        boxed.owner_and_borrows.store(state, Ordering::Relaxed);
        let mut output = ComRaw::null();
        // SAFETY: valid boxed header/metadata and distinct complete outputs.
        unsafe {
            assert_eq!(check_shared_weak_cast(pointer.cast(), &crate::IUNKNOWN_INTERFACE_ID), Err(NC_E_INVALID_ARG));
            assert_eq!(nc_weak_upgrade(pointer.cast(), &crate::IUNKNOWN_INTERFACE_ID, &mut output), NC_E_INVALID_ARG);
            assert!(!output.is_present());
            assert_eq!(nc_query_interface(pointer.cast(), &crate::IUNKNOWN_INTERFACE_ID, &mut output), NC_E_INVALID_ARG);
            assert_eq!(nc_strong_retain(pointer.cast()), 0);
        }
        assert_eq!(boxed.owner_and_borrows.load(Ordering::Relaxed), state);
        assert_eq!(boxed.weak_count.load(Ordering::Relaxed), 1);
    }
    let shared = NcSharedLifetimeState::new(&CLASS);
    let pointer = core::ptr::from_ref(&shared).cast_mut();
    // SAFETY: only the immutable pointer is inspected before policy rejection.
    unsafe {
        assert_eq!(try_box_cast(pointer.cast(), &crate::IUNKNOWN_INTERFACE_ID), Err(NC_E_INVALID_ARG));
        assert!(matches!(BoxRead::enter(pointer.cast()), Err(NC_E_INVALID_ARG)));
        assert!(matches!(BoxWrite::enter(pointer.cast()), Err(NC_E_INVALID_ARG)));
    }
    assert_eq!(shared.strong_count.load(Ordering::Relaxed), 1);
}

/// Unknown policy needs only a readable metadata-pointer prefix, never a full header.
#[test]
fn raw_admission_rejects_unknown_kind_before_borrowing_a_complete_header() {
    let info = NcObjectTypeInfo {
        ownership: 99,
        destroy_value: destroy::<NcSharedLifetimeState>,
        deallocate: free::<NcSharedLifetimeState>,
        interfaces: [&ROOT as *const IUnknownVTable].as_ptr(),
        interface_count: 1,
    };
    // This single pointer has no readable count/access words following it.
    let prefix = core::ptr::from_ref(&info);
    let pointer = core::ptr::from_ref(&prefix).cast_mut().cast();
    // SAFETY: the complete prefix and immutable metadata live through these calls.
    unsafe {
        assert!(matches!(shared_state(pointer), Err(NC_E_INVALID_ARG)));
        assert!(matches!(boxed_state(pointer), Err(NC_E_INVALID_ARG)));
        assert!(matches!(object_type_info(core::ptr::null_mut()), Err(NC_E_POINTER)));
    }
}

/// Both policies retain compact headers but never share an overloaded mutable field.
#[test]
fn policy_headers_keep_metadata_first_and_have_three_words() {
    use core::mem::{align_of, offset_of, size_of};
    let word = size_of::<usize>();
    assert_eq!(size_of::<NcSharedLifetimeState>(), 3 * word);
    assert_eq!(size_of::<NcBoxedLifetimeState>(), 3 * word);
    assert_eq!(align_of::<NcSharedLifetimeState>(), align_of::<usize>());
    assert_eq!(align_of::<NcBoxedLifetimeState>(), align_of::<usize>());
    assert_eq!(size_of::<NcObjectTypeInfo>(), 5 * word);
    assert_eq!(offset_of!(NcObjectTypeInfo, ownership), 0);
    assert_eq!(offset_of!(NcObjectTypeInfo, destroy_value), word);
    assert_eq!(offset_of!(NcObjectTypeInfo, deallocate), 2 * word);
    assert_eq!(offset_of!(NcObjectTypeInfo, interfaces), 3 * word);
    assert_eq!(offset_of!(NcObjectTypeInfo, interface_count), 4 * word);
    assert_eq!(offset_of!(NcSharedLifetimeState, type_info), 0);
    assert_eq!(offset_of!(NcSharedLifetimeState, strong_count), word);
    assert_eq!(offset_of!(NcSharedLifetimeState, weak_count), 2 * word);
    assert_eq!(offset_of!(NcBoxedLifetimeState, type_info), 0);
    assert_eq!(offset_of!(NcBoxedLifetimeState, owner_and_borrows), word);
    assert_eq!(offset_of!(NcBoxedLifetimeState, weak_count), 2 * word);
}

/// Deallocation is immediate after destruction when no observer survives.
#[test]
fn either_policy_frees_without_an_extra_weak_only_phase() {
    let (shared, shared_observed) = allocation();
    let (boxed, boxed_observed) = boxed_allocation();
    // SAFETY: each fixture transfers its one matching owner exactly once.
    unsafe {
        assert_eq!(nc_strong_release(shared.cast()), 0);
        nc_box_release(boxed);
    }
    for observed in [shared_observed, boxed_observed] {
        assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
    }
}

/// Safe constructors never publish a header paired with the other policy's metadata.
#[test]
fn typed_constructors_reject_mismatched_immutable_policy() {
    assert!(std::panic::catch_unwind(|| NcSharedLifetimeState::new(&BOX_CLASS)).is_err());
    assert!(std::panic::catch_unwind(|| NcBoxedLifetimeState::new(&CLASS)).is_err());
}
