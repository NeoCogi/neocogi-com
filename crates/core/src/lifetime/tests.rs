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
        assert_eq!(nc_weak_upgrade(control, &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_E_CLOSED);
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
        assert_eq!(nc_weak_upgrade(control, &wrong, &mut out), NC_E_CLOSED);
        nc_weak_release(control);
    }
    assert_eq!(observations.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
}

/// Escaped weak tokens from abandoned construction can only observe permanent absence.
#[test]
fn abandoned_construction_frees_without_dropping_uninitialized_value() {
    let (control, observations) = allocation(false);
    unsafe {
        nc_weak_retain(control);
        nc_weak_release(control); // Constructor rollback transfers no strong owner.
        let mut out = ComRaw::null();
        assert_eq!(nc_weak_upgrade(control, &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_E_CLOSED);
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
        assert_eq!(nc_weak_upgrade(control, &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_E_CLOSED);
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
