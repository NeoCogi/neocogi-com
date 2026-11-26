//! Ownership regressions exercise the shared operations against real typed allocations.
use super::*;
use crate::ComInterface;
use core::mem::MaybeUninit;
use std::{boxed::Box, sync::Arc};

std::thread_local! {
    /// Deterministic test-only interleaving: consume the original owner after
    /// promotion acquires another, so failed negotiation rolls back the last one.
    pub(super) static RELEASE_OWNER_AFTER_PROMOTION: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Independent observations survive both value destruction and allocation release.
#[derive(Default)]
struct Observations {
    /// Number of concrete value destructors invoked.
    destroyed: AtomicUsize,
    /// Number of allocator callbacks invoked.
    freed: AtomicUsize,
}
/// A payload with an observable destructor and an owned weak self reference.
struct Value<Weak> {
    /// External counters avoid reading memory after final release.
    observations: Arc<Observations>,
    /// Optional explicit weak owner released from inside value destruction.
    self_weak: Option<Weak>,
}
impl<Weak> Drop for Value<Weak> {
    /// Reentrant weak release must not free the header before this destructor returns.
    fn drop(&mut self) {
        if let Some(control) = self.self_weak.take() {
            drop(control);
            assert_eq!(self.observations.freed.load(Ordering::SeqCst), 0);
        }
        self.observations.destroyed.fetch_add(1, Ordering::SeqCst);
    }
}
/// Header-first storage remains valid after value destruction until final weak release.
#[repr(C)]
struct Allocation<Lifetime = NcSharedLifetimeState, Weak = crate::WeakComPtr<IUnknown>> {
    /// Common prefix used without knowing this concrete storage type.
    control: Lifetime,
    /// Destroyed once on final strong release; never dropped by Box<Allocation>.
    value: MaybeUninit<Value<Weak>>,
    /// Allocation-only observer remains available after value destruction.
    observations: Arc<Observations>,
}
/// Provider-specific destruction knows the exact value layout.
unsafe extern "C" fn destroy<Lifetime, Weak>(control: *mut c_void) {
    unsafe {
        (*control.cast::<Allocation<Lifetime, Weak>>()).value.assume_init_drop();
    }
}
/// Provider-specific freeing never runs the concrete value destructor twice.
unsafe extern "C" fn free<Lifetime, Weak>(control: *mut c_void) {
    let allocation = unsafe { Box::from_raw(control.cast::<Allocation<Lifetime, Weak>>()) };
    allocation.observations.freed.fetch_add(1, Ordering::SeqCst);
    drop(allocation);
}
/// Static root projection uses the same shared entry points as generated objects will.
static ROOT: IUnknownVTable = IUnknownVTable {
    interface_id: &crate::IUNKNOWN_INTERFACE_ID,
    query_interface: nc_shared_query_interface,
    add_ref: nc_shared_retain,
    release: nc_shared_release,
    downgrade: None,
};
/// A real zero-method contract gives erased promotions a distinct valid table identity.
enum AlternateInterface {}
// SAFETY: its complete table is IUnknownVTable with this exact independent identity.
unsafe impl crate::ComInterface for AlternateInterface {
    const INTERFACE_ID: NcInterfaceId = NcInterfaceId::new(crate::NcGuid::from_fields(42, 1, 2, [3; 8]), crate::NcAbiDigest::from_static(&[4; 32]));
    type Threading = crate::ThreadLocal;
    type VTable = IUnknownVTable;
}
/// Shared callbacks with the alternate contract's complete immutable identity.
static ALTERNATE_ROOT: IUnknownVTable = IUnknownVTable {
    interface_id: &AlternateInterface::INTERFACE_ID,
    ..SHARED_UNKNOWN_VTABLE
};
/// Class data points to one valid root projection without retaining inspection graphs.
static CLASS: NcObjectTypeInfo = NcObjectTypeInfo {
    destroy_value: destroy::<NcSharedLifetimeState, crate::WeakComPtr<IUnknown>>,
    deallocate: free::<NcSharedLifetimeState, crate::WeakComPtr<IUnknown>>,
    interfaces: [&ROOT as *const IUnknownVTable, &ALTERNATE_ROOT as *const IUnknownVTable].as_ptr(),
    interface_count: 2,
};
/// Allocates a fully initialized provider object with one shared owner.
fn allocation() -> (*mut NcSharedLifetimeState, Arc<Observations>) {
    let observations = Arc::new(Observations::default());
    let storage: Box<Allocation> = Box::new(Allocation {
        control: NcSharedLifetimeState::new(&CLASS),
        value: MaybeUninit::new(Value {
            observations: observations.clone(),
            self_weak: None,
        }),
        observations: observations.clone(),
    });
    (Box::into_raw(storage).cast(), observations)
}

/// The exclusive fixture uses its own statically selected root callbacks.
static BOX_ROOT: IUnknownVTable = IUnknownVTable {
    downgrade: None,
    ..EXCLUSIVE_UNKNOWN_VTABLE
};

/// A distinct boxed projection with the same owner and backing.
static BOX_ALTERNATE_ROOT: IUnknownVTable = IUnknownVTable {
    interface_id: &AlternateInterface::INTERFACE_ID,
    ..BOX_ROOT
};

/// Canonical metadata for the single-interface boxed fixture.
static BOX_CLASS: NcObjectTypeInfo = NcObjectTypeInfo {
    destroy_value: destroy::<NcBoxedLifetimeState, ()>,
    deallocate: free::<NcBoxedLifetimeState, ()>,
    interfaces: [&BOX_ROOT as *const IUnknownVTable].as_ptr(),
    interface_count: 1,
};

/// The projection fixture also exposes an alternate exact interface.
static BOX_PROJECTION_CLASS: NcObjectTypeInfo = NcObjectTypeInfo {
    interfaces: [&BOX_ROOT as *const IUnknownVTable, &BOX_ALTERNATE_ROOT as *const IUnknownVTable].as_ptr(),
    interface_count: 2,
    ..BOX_CLASS
};

/// Allocates a boxed value without reinterpreting or mutating shared ownership.
fn boxed_allocation() -> (*mut NcBoxedLifetimeState, Arc<Observations>) {
    boxed_allocation_with_type_info(&BOX_CLASS)
}

/// Allocates the same concrete payload with its matching provider metadata.
fn boxed_allocation_with_type_info(type_info: &'static NcObjectTypeInfo) -> (*mut NcBoxedLifetimeState, Arc<Observations>) {
    let observations = Arc::new(Observations::default());
    let storage: Box<Allocation<NcBoxedLifetimeState, ()>> = Box::new(Allocation {
        control: NcBoxedLifetimeState::new(type_info),
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
        (*control).weak_retain();
        (*control.cast::<Allocation>()).value.assume_init_mut().self_weak = crate::WeakComPtr::from_raw_owned(crate::NcSharedWeakRaw { control });
        (*control).weak_retain(); // External weak reference survives destruction.
        assert_eq!(nc_shared_retain(control.cast()), 2);
        assert_eq!(nc_shared_release(control.cast()), 1);
        assert_eq!(nc_shared_release(control.cast()), 0);
        assert_eq!(observations.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observations.freed.load(Ordering::SeqCst), 0);
        assert_eq!(NcSharedLifetimeState::upgrade(control, &crate::IUNKNOWN_INTERFACE_ID).unwrap(), None);
        (*control).weak_retain(); // Expired weak references can still be cloned.
        NcSharedLifetimeState::weak_release(control);
        NcSharedLifetimeState::weak_release(control);
    }
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
}

/// Promotion/query failures do not leak counts, and successes each transfer one owner.
#[test]
fn checked_projection_balances_all_ownership_paths() {
    let (control, observations) = allocation();
    let wrong = NcInterfaceId::new(crate::IID_IUNKNOWN, crate::NcAbiDigest::from_static(&[0; 32]));
    unsafe {
        (*control).weak_retain();
        assert_eq!(NcSharedLifetimeState::upgrade(control, &wrong).unwrap_err(), crate::NC_E_ABI_VERSION);
        assert_eq!(nc_shared_retain(control.cast()), 2);
        assert_eq!(nc_shared_release(control.cast()), 1);
        let promoted = NcSharedLifetimeState::upgrade(control, &crate::IUNKNOWN_INTERFACE_ID).unwrap().unwrap();
        assert_eq!(nc_shared_release(promoted.object), 1);
        let mut out = ComRaw::null();
        assert_eq!(nc_shared_query_interface(control.cast(), &crate::IUNKNOWN_INTERFACE_ID, &mut out), NC_OK);
        assert_eq!(nc_shared_release(out.object), 1);
        assert_eq!(nc_shared_release(control.cast()), 0);
        assert_eq!(NcSharedLifetimeState::upgrade(control, &wrong).unwrap(), None);
        NcSharedLifetimeState::weak_release(control);
    }
    assert_eq!(observations.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observations.freed.load(Ordering::SeqCst), 1);
}

/// Output rejection and borrowed acquisition balance weak counts without promotion.
#[test]
fn checked_weak_transport_balances_tokens_before_and_after_destruction() {
    use crate::{ComValue, NcSharedWeakRaw, ValueBudget, WeakComPtr};
    let (control, observed) = allocation();
    // SAFETY: this fixture owns the initial strong reference and retains one
    // explicit weak owner across both iterations and final value destruction.
    unsafe {
        (*control).weak_retain();
    }
    for expired in [false, true] {
        // SAFETY: the explicit weak owner keeps this header readable throughout.
        let before = unsafe { (*control).weak_count.load(Ordering::Relaxed) };
        unsafe {
            (*control).weak_retain();
        }
        // SAFETY: transfer the additional weak token with intentionally wrong T.
        assert_eq!(
            unsafe { WeakComPtr::<crate::INcBytesAbi>::__nc_from_raw_owned_checked(NcSharedWeakRaw { control: control.cast() }) }.unwrap_err(),
            crate::NC_E_NOINTERFACE
        );
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        // SAFETY: the source lends a live header; acquisition adds only one weak owner.
        let acquired = unsafe { WeakComPtr::<IUnknown>::decode_value(&NcSharedWeakRaw { control: control.cast() }, &mut ValueBudget::default()) }.unwrap();
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before + 1);
        assert_eq!(unsafe { (*control).strong_count.load(Ordering::Relaxed) }, if expired { 0 } else { 1 });
        drop(acquired);
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        // A rejected conversion neither acquires nor releases the caller's token.
        assert_eq!(
            unsafe { WeakComPtr::<IUnknown>::decode_value(&NcSharedWeakRaw { control: control.cast() }, &mut ValueBudget::new(0, 0, 0)) }.unwrap_err(),
            crate::NC_E_OUT_OF_RANGE
        );
        assert_eq!(unsafe { (*control).weak_count.load(Ordering::Relaxed) }, before);
        if !expired {
            unsafe {
                nc_shared_release(control.cast());
            }
        }
    }
    // SAFETY: release the final explicit weak owner; no header reads follow.
    unsafe {
        NcSharedLifetimeState::weak_release(control);
    }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// The arithmetic helper fails before mutation; the external non-unwinding entry aborts.
#[test]
fn count_exhaustion_never_wraps_or_changes_the_counter() {
    let count = AtomicUsize::new(isize::MAX as usize);
    assert_eq!(retain(&count), None);
    assert_eq!(retain(&AtomicUsize::new(0)), None);
    assert_eq!(count.load(Ordering::Relaxed), isize::MAX as usize);
}

/// Exact projection keeps the sole owner and its allocation live.
#[test]
fn borrowed_box_queries_share_identity_and_preserve_the_owner() {
    let (control, observed) = boxed_allocation_with_type_info(&BOX_PROJECTION_CLASS);
    // SAFETY: this fixture transfers the initialized sole owner with its exact root table.
    let mut owner = unsafe {
        crate::ComBox::<IUnknown>::from_raw_owned(ComRaw {
            object: control.cast(),
            vtable: &BOX_ROOT,
        })
    }
    .unwrap();
    let raw = owner.as_raw();
    {
        let projection = owner.query_ref::<AlternateInterface>().unwrap();
        assert!(projection.same_object(owner.as_com_ref()));
        assert_eq!(projection.as_raw().vtable, &BOX_ALTERNATE_ROOT as *const _);
        assert!(matches!(owner.query_ref::<crate::INcBytesAbi>(), Err(crate::NC_E_NOINTERFACE)));
    }
    {
        let mut mutable = owner.query_mut::<AlternateInterface>().unwrap();
        assert_eq!(mutable.as_com_mut().as_raw().object, raw.object);
        assert_eq!(mutable.as_raw().vtable, &BOX_ALTERNATE_ROOT as *const _);
    }
    assert_eq!(owner.as_raw(), raw);
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 0);
    drop(owner);
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// Both policies keep their typed prefixes and common four-word metadata.
#[test]
fn policy_headers_keep_metadata_first_and_have_expected_sizes() {
    use core::mem::{align_of, offset_of, size_of};
    let word = size_of::<usize>();
    assert_eq!(size_of::<NcSharedLifetimeState>(), 3 * word);
    assert_eq!(size_of::<NcBoxedLifetimeState>(), word);
    assert_eq!(align_of::<NcSharedLifetimeState>(), align_of::<usize>());
    assert_eq!(align_of::<NcBoxedLifetimeState>(), align_of::<usize>());
    assert_eq!(size_of::<NcObjectTypeInfo>(), 4 * word);
    assert_eq!(offset_of!(NcObjectTypeInfo, destroy_value), 0);
    assert_eq!(offset_of!(NcObjectTypeInfo, deallocate), word);
    assert_eq!(offset_of!(NcObjectTypeInfo, interfaces), 2 * word);
    assert_eq!(offset_of!(NcObjectTypeInfo, interface_count), 3 * word);
    assert_eq!(offset_of!(NcSharedLifetimeState, type_info), 0);
    assert_eq!(offset_of!(NcSharedLifetimeState, strong_count), word);
    assert_eq!(offset_of!(NcSharedLifetimeState, weak_count), 2 * word);
    assert_eq!(offset_of!(NcBoxedLifetimeState, type_info), 0);
}

/// Deallocation is immediate after destruction when no observer survives.
#[test]
fn either_policy_frees_without_an_extra_weak_only_phase() {
    let (shared, shared_observed) = allocation();
    let (boxed, boxed_observed) = boxed_allocation();
    // SAFETY: each fixture transfers its one matching owner exactly once.
    unsafe {
        assert_eq!(nc_shared_release(shared.cast()), 0);
        NcBoxedLifetimeState::release(boxed);
    }
    for observed in [shared_observed, boxed_observed] {
        assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
    }
}

/// The two canonical prefixes preserve identity while selecting distinct release policies.
#[test]
fn canonical_prefixes_select_ownership_without_a_universal_release() {
    // Metadata contents define identity; independent constant backing may differ.
    unsafe {
        assert_eq!(*SHARED_UNKNOWN_VTABLE.interface_id, *EXCLUSIVE_UNKNOWN_VTABLE.interface_id);
    }
    assert!(!std::ptr::fn_addr_eq(SHARED_UNKNOWN_VTABLE.release, EXCLUSIVE_UNKNOWN_VTABLE.release));
    assert!(!std::ptr::fn_addr_eq(SHARED_UNKNOWN_VTABLE.add_ref, EXCLUSIVE_UNKNOWN_VTABLE.add_ref));
    let (shared, shared_observed) = allocation();
    let (boxed, boxed_observed) = boxed_allocation();
    unsafe {
        assert_eq!((SHARED_UNKNOWN_VTABLE.add_ref)(shared.cast()), 2);
        assert_eq!((SHARED_UNKNOWN_VTABLE.release)(shared.cast()), 1);
        assert_eq!((EXCLUSIVE_UNKNOWN_VTABLE.add_ref)(boxed.cast()), 0);
        assert_eq!((EXCLUSIVE_UNKNOWN_VTABLE.release)(boxed.cast()), 0);
        assert_eq!((SHARED_UNKNOWN_VTABLE.release)(shared.cast()), 0);
    }
    for observed in [shared_observed, boxed_observed] {
        assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
    }
}

/// Ordinary Rust weak clones still fail-stop on exhaustion rather than unwind.
#[test]
#[cfg(not(miri))]
fn weak_clone_exhaustion_cannot_unwind_to_its_caller() {
    const CHILD: &str = "NC_TEST_WEAK_COUNTER_EXHAUSTION";
    if std::env::var_os(CHILD).is_some() {
        let (control, _) = allocation();
        unsafe {
            (*control).weak_count.store(isize::MAX as usize, Ordering::Relaxed);
        }
        let weak = unsafe { crate::WeakComPtr::<IUnknown>::from_raw_owned(crate::NcSharedWeakRaw { control: control.cast() }) }.unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| weak.clone()));
        std::process::exit(97);
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lifetime::tests::weak_clone_exhaustion_cannot_unwind_to_its_caller", "--nocapture"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(97), "exhaustion unwound or returned");
    assert!(std::string::String::from_utf8_lossy(&output.stderr).contains("COM lifetime counter exhausted"));
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(output.status.signal(), Some(6));
    }
}

/// Typed local promotion negotiates exact tables and transfers only its acquired owner.
#[test]
fn typed_promotion_balances_success_and_negotiation_errors() {
    use crate::{raw, NC_E_NOINTERFACE};
    let (control, observed) = allocation();
    let missing = crate::INcBytesAbi::INTERFACE_ID;
    unsafe {
        (*control).weak_retain();
        assert_eq!(NcSharedLifetimeState::upgrade(control, &missing).unwrap_err(), NC_E_NOINTERFACE);
        assert_eq!(nc_shared_retain(control.cast()), 2);
        assert_eq!(nc_shared_release(control.cast()), 1);
        let out = NcSharedLifetimeState::upgrade(control, &AlternateInterface::INTERFACE_ID).unwrap().unwrap();
        assert_eq!(nc_shared_retain(control.cast()), 3);
        assert_eq!(nc_shared_release(control.cast()), 2);
        assert_eq!(raw::validate_interface(out), Err(NC_E_NOINTERFACE));
        assert_eq!(raw::validate_interface(out.cast::<AlternateInterface>()), Ok(()));
        assert_eq!(raw::release(out), 1);
        assert_eq!(nc_shared_release(control.cast()), 0);
        assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
        assert_eq!(NcSharedLifetimeState::upgrade(control, &missing).unwrap(), None);
        NcSharedLifetimeState::weak_release(control);
    }
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

/// A permitted owner-release schedule makes failed promotion's rollback the
/// final strong release. Weak-to-self cleanup cannot free its retained shell.
#[test]
fn failed_promotion_rollback_destroys_the_final_owner() {
    for requested in [
        crate::INcBytesAbi::INTERFACE_ID,
        NcInterfaceId::new(crate::IID_IUNKNOWN, crate::NcAbiDigest::from_static(&[0; 32])),
    ] {
        let (control, observed) = allocation();
        unsafe {
            (*control).weak_retain();
            (*control.cast::<Allocation>()).value.assume_init_mut().self_weak = crate::WeakComPtr::from_raw_owned(crate::NcSharedWeakRaw { control });
            (*control).weak_retain();
            RELEASE_OWNER_AFTER_PROMOTION.with(|release| release.set(true));
            let expected = if requested.iid == crate::IID_IUNKNOWN {
                crate::NC_E_ABI_VERSION
            } else {
                crate::NC_E_NOINTERFACE
            };
            assert_eq!(NcSharedLifetimeState::upgrade(control, &requested).unwrap_err(), expected);
            assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
            assert_eq!(observed.freed.load(Ordering::SeqCst), 0);
            assert_eq!((*control).strong_count.load(Ordering::Relaxed), 0);
            assert_eq!((*control).weak_count.load(Ordering::Relaxed), 1);
            assert_eq!(NcSharedLifetimeState::upgrade(control, &requested).unwrap(), None);
            NcSharedLifetimeState::weak_release(control);
        }
        assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
    }
}

/// Real C slots retain their null argument and cleared-output contracts even
/// though local typed promotion no longer uses raw request/output pointers.
#[test]
fn shared_prefix_null_arguments_clear_outputs_without_acquiring_owners() {
    let (control, observed) = allocation();
    let object = control.cast();
    let mut output = ComRaw { object, vtable: &ROOT };
    let mut weak = crate::NcSharedWeakRaw { control };
    unsafe {
        assert_eq!(nc_shared_query_interface(object, core::ptr::null(), &mut output), NC_E_POINTER);
        assert!(!output.is_present());
        assert_eq!(nc_shared_query_interface(object, &IUnknown::INTERFACE_ID, core::ptr::null_mut()), NC_E_POINTER);
        assert_eq!(nc_shared_downgrade(core::ptr::null_mut(), &mut weak), NC_E_POINTER);
        assert!(!weak.is_present());
        assert_eq!(nc_shared_downgrade(object, core::ptr::null_mut()), NC_E_POINTER);
        assert_eq!((*control).strong_count.load(Ordering::Relaxed), 1);
        assert_eq!((*control).weak_count.load(Ordering::Relaxed), 1);
        NcSharedLifetimeState::release(control);
    }
    assert_eq!(observed.destroyed.load(Ordering::SeqCst), 1);
    assert_eq!(observed.freed.load(Ordering::SeqCst), 1);
}

mod admission;
