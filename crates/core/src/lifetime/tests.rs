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
    self_weak: Option<*mut NcWeakControl>,
}
impl Drop for Value {
    /// Reentrant weak release must not free the header before this destructor returns.
    fn drop(&mut self) {
        if let Some(control) = self.self_weak.take() {
            unsafe {
                nc_weak_release(control);
            }
            assert_eq!(self.observations.freed.load(Ordering::SeqCst), 0);
        }
        self.observations.destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
/// Header-first provider storage; MaybeUninit separates value destruction from freeing.
#[repr(C)]
struct Allocation {
    /// Common prefix used without knowing this concrete storage type.
    control: NcWeakControl,
    /// Destroyed once on final strong release; never dropped by Box<Allocation>.
    value: MaybeUninit<Value>,
    /// Allocation-only observer remains initialized even if construction fails.
    observations: Arc<Observations>,
}
/// Provider-specific destruction knows the exact value layout.
unsafe extern "C" fn destroy(control: *mut NcWeakControl) {
    unsafe {
        (*control.cast::<Allocation>()).value.assume_init_drop();
    }
}
/// Provider-specific freeing never runs the concrete value destructor twice.
unsafe extern "C" fn free(control: *mut NcWeakControl) {
    let allocation = unsafe { Box::from_raw(control.cast::<Allocation>()) };
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
static CLASS: NcClassLifetime = NcClassLifetime {
    destroy_value: destroy,
    deallocate: free,
    interfaces: [&ROOT as *const IUnknownVTable].as_ptr(),
    interface_count: 1,
};
/// Allocates a real provider object, either initialized or permanently unpublished.
fn allocation(live: bool) -> (*mut NcWeakControl, Arc<Observations>) {
    let observations = Arc::new(Observations::default());
    let (control, value) = match live {
        true => (
            NcWeakControl::new(&CLASS),
            MaybeUninit::new(Value {
                observations: observations.clone(),
                self_weak: None,
            }),
        ),
        false => (NcWeakControl::unpublished(&CLASS), MaybeUninit::uninit()),
    };
    let storage = Box::new(Allocation {
        control,
        value,
        observations: observations.clone(),
    });
    (Box::into_raw(storage).cast(), observations)
}

/// Value lifetime ends independently of weak storage, including destructor-owned weak tokens.
#[test]
fn final_strong_drops_value_before_final_weak_frees_storage() {
    let (control, observations) = allocation(true);
    unsafe {
        nc_weak_retain(control);
        (*control.cast::<Allocation>()).value.assume_init_mut().self_weak = Some(control);
        nc_weak_retain(control); // External weak reference survives destruction.
        assert_eq!(nc_strong_retain(control.cast()), 2);
        assert_eq!(nc_strong_release(control.cast()), 1);
        assert_eq!(nc_strong_release(control.cast()), 0);
        assert_eq!(observations.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observations.freed.load(Ordering::SeqCst), 0);
        let mut out = ComRaw::null();
        assert_eq!(nc_weak_upgrade(control, &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_E_DROPPED);
        assert!(!out.is_present());
        nc_weak_retain(control); // Expired weak references can still be cloned.
        nc_weak_release(control);
        nc_weak_release(control);
    }
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
}

/// Promotion/query failures do not leak counts, and successes each transfer one owner.
#[test]
fn checked_projection_balances_all_ownership_paths() {
    let (control, observations) = allocation(true);
    let wrong = NcInterfaceId::new(crate::IID_IUNKNOWN, crate::NcAbiSignature::from_static(&[0; 32]));
    unsafe {
        nc_weak_retain(control);
        let mut out = ComRaw { object: control.cast(), vtable: &ROOT };
        assert_eq!(nc_weak_upgrade(control, &wrong, &mut out), crate::NC_E_ABI_VERSION);
        assert!(!out.is_present());
        assert_eq!(nc_strong_retain(control.cast()), 2);
        assert_eq!(nc_strong_release(control.cast()), 1);
        assert_eq!(nc_weak_upgrade(control, &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_OK);
        assert_eq!(nc_strong_release(out.object), 1);
        assert_eq!(nc_query_interface(control.cast(), &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_OK);
        assert_eq!(nc_strong_release(out.object), 1);
        assert_eq!(nc_strong_release(control.cast()), 0);
        assert_eq!(nc_weak_upgrade(control, &wrong, &mut out), NC_E_DROPPED);
        nc_weak_release(control);
    }
    assert_eq!(observations.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
}

/// Output rejection and borrowed acquisition balance weak counts without promotion.
#[test]
fn checked_weak_transport_balances_tokens_before_and_after_destruction() {
    use crate::{ComValue, NcWeakRaw, ValueBudget, WeakComPtr};
    let (control, observed) = allocation(true);
    // SAFETY: this fixture owns the initial strong reference and retains one
    // explicit weak owner across both iterations and final value destruction.
    unsafe { nc_weak_retain(control); }
    for expired in [false, true] {
        // SAFETY: the explicit weak owner keeps this header readable throughout.
        let before = unsafe { (*control).weak.load(Ordering::Relaxed) };
        unsafe { nc_weak_retain(control); }
        // SAFETY: transfer the additional weak token with intentionally wrong T.
        assert_eq!(unsafe {
            WeakComPtr::<crate::INcBytesAbi>::__nc_from_raw_owned_checked(NcWeakRaw { control })
        }.unwrap_err(), crate::NC_E_NOINTERFACE);
        assert_eq!(unsafe { (*control).weak.load(Ordering::Relaxed) }, before);
        // SAFETY: the source lends a live header; acquisition adds only one weak owner.
        let acquired = unsafe {
            WeakComPtr::<IUnknown>::decode_value(NcWeakRaw { control }, &mut ValueBudget::default())
        }.unwrap();
        assert_eq!(unsafe { (*control).weak.load(Ordering::Relaxed) }, before + 1);
        assert_eq!(unsafe { (*control).state.load(Ordering::Relaxed) }, if expired { 0 } else { 1 });
        drop(acquired);
        assert_eq!(unsafe { (*control).weak.load(Ordering::Relaxed) }, before);
        // A rejected conversion neither acquires nor releases the caller's token.
        assert_eq!(unsafe {
            WeakComPtr::<IUnknown>::decode_value(NcWeakRaw { control }, &mut ValueBudget::new(0, 0, 0))
        }.unwrap_err(), crate::NC_E_OUT_OF_RANGE);
        assert_eq!(unsafe { (*control).weak.load(Ordering::Relaxed) }, before);
        if !expired { unsafe { nc_strong_release(control.cast()); } }
    }
    // SAFETY: release the final explicit weak owner; no header reads follow.
    unsafe { nc_weak_release(control); }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// Escaped weak tokens from abandoned construction can only observe permanent absence.
#[test]
fn abandoned_construction_frees_without_dropping_uninitialized_value() {
    let (control, observations) = allocation(false);
    unsafe {
        nc_weak_retain(control);
        nc_weak_release(control); // Constructor rollback transfers no strong owner.
        let mut out = ComRaw::null();
        assert_eq!(nc_weak_upgrade(control, &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_E_DROPPED);
        nc_weak_release(control);
    }
    assert_eq!(observations.destroyed.load(Ordering::SeqCst), 0);
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
}

/// Publication starts strong lifetime only after the concrete value is initialized.
#[test]
fn publication_converts_constructor_weak_into_implicit_owner() {
    let (control, observations) = allocation(false);
    unsafe {
        let mut out = ComRaw::null();
        assert_eq!(nc_weak_upgrade(control, &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_E_DROPPED);
        (*control.cast::<Allocation>()).value.write(Value {
            observations: observations.clone(),
            self_weak: None,
        });
        (*control).publish();
        assert_eq!(nc_weak_upgrade(control, &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_OK);
        assert_eq!(nc_strong_release(out.object), 1);
        assert_eq!(nc_strong_release(control.cast()), 0);
    }
    assert_eq!(observations.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
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
    let (control, observed) = allocation(true);
    // SAFETY: this allocation has not escaped or been borrowed. Select exclusive
    // ownership before publication and install one properly retained weak field.
    unsafe {
        (*control).state.store(BOXED | BOX_OWNER, Ordering::Relaxed);
        nc_weak_retain(control);
        (*control.cast::<Allocation>()).value.assume_init_mut().self_weak = Some(control);
        nc_weak_retain(control);
        nc_box_release(control);
        assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observed.freed.load(Ordering::SeqCst), 0);
        assert!(matches!(BoxRead::enter(control), Err(NC_E_DROPPED)));
        assert!(matches!(BoxWrite::enter(control), Err(NC_E_DROPPED)));
        nc_weak_release(control);
    }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// A full reader count must reject admission without wrapping into the writer bit.
#[test]
fn exclusive_reader_overflow_and_writer_conflicts_preserve_header_state() {
    let header = NcWeakControl::boxed(&CLASS);
    let raw = core::ptr::from_ref(&header).cast_mut();
    header.state.store(BOXED | BOX_OWNER | BOX_READERS, Ordering::Relaxed);
    // SAFETY: both attempts reject before granting access; the test header is
    // retained and no provider destruction is attempted for this stack fixture.
    unsafe {
        assert!(matches!(BoxRead::enter(raw), Err(crate::NC_E_BUSY)));
        assert!(matches!(BoxWrite::enter(raw), Err(crate::NC_E_BUSY)));
    }
    assert_eq!(header.state.load(Ordering::Relaxed), BOXED | BOX_OWNER | BOX_READERS);
    header.state.store(BOXED | BOX_OWNER, Ordering::Relaxed);
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
