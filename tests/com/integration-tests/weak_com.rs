//! Weak COM lifetime is allocation-wide, independent of logical closure or queued payloads.
#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
use neocogi_com_sdk::{com_object, ComPtr, IUnknown, NcStatus, WeakComPtr, NC_E_CLOSED};
use std::cell::{Cell, OnceCell};
use std::sync::{
    Arc,
    atomic::{AtomicU32, AtomicUsize, Ordering},
};

extern crate self as fixture_weak_com;
test_contracts::weak_com!();

#[com_object(IDerived)]
/// Stores a weak self reference to prove no self-retention cycle is introduced.
struct Value {
    /// Destruction observations remain valid independently of the COM value.
    drops: Arc<AtomicUsize>,
    /// A closed object can still be successfully promoted while strongly retained.
    closed: Cell<bool>,
    /// Optional self origin connected after allocation, before the factory returns.
    origin: OnceCell<WeakComPtr<IDerivedAbi>>,
}
impl IValue for Value {
    fn value(&self) -> Result<u32, NcStatus> {
        if self.closed.get() { Err(NC_E_CLOSED) } else { Ok(42) }
    }
    fn close(&self) -> Result<(), NcStatus> {
        self.closed.set(true);
        Ok(())
    }
}
impl IDerived for Value {}
impl Drop for Value {
    /// The weak self token is released by ordinary field destruction afterward.
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Last strong release drops the value exactly once, regardless of queued weak references.
#[test]
fn weak_tokens_do_not_retain_values_and_all_projections_share_identity() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Value::from_value(Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    });
    let derived = owner.query::<IDerivedAbi>().unwrap();
    let base = owner.query::<IValueAbi>().unwrap();
    assert!(derived.as_com_ref().same_object(base.as_com_ref()));
    let unrelated = Value::from_value(Value {
        drops: Arc::new(AtomicUsize::new(0)),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    });
    assert!(!derived.as_com_ref().same_object(unrelated.as_com_ref()));
    drop(unrelated);
    let weak_root = owner.downgrade().unwrap();
    let weak_derived = derived.downgrade().unwrap();
    let weak_base = base.downgrade().unwrap();
    assert!(weak_root.same_object(&weak_derived));
    assert!(weak_base.same_object(&weak_derived));
    assert_eq!(weak_derived.upgrade().unwrap().unwrap().value().unwrap(), 42);
    drop((owner, derived, base));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak_root.upgrade().unwrap().is_none());
    assert!(weak_derived.clone().upgrade().unwrap().is_none());
    assert!(weak_base.upgrade().unwrap().is_none());
    // A new object never resolves an old token, even if its allocator would reuse addresses.
    let replacement = Value::from_value(Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    });
    assert!(!weak_root.same_object(&replacement.downgrade().unwrap()));
    assert!(weak_root.upgrade().unwrap().is_none());
    drop(replacement);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

/// Logical closure is observable on a promoted live object and does not invalidate weak ownership.
#[test]
fn logical_close_does_not_expire_a_retained_object() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner: ComPtr<IValueAbi> = Value::from_value(Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    })
    .query()
    .unwrap();
    let weak = owner.downgrade().unwrap();
    owner.close().unwrap();
    let promoted = weak.upgrade().unwrap().unwrap();
    assert_eq!(promoted.value(), Err(NC_E_CLOSED));
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 0, "promotion owns one real strong reference");
    drop(promoted);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

/// A factory can connect a weak identity through ordinary checked local access;
/// destroying the initialized value releases that field without a strong cycle.
#[test]
fn weak_self_connection_after_allocation_drops_without_a_cycle() {
    let drops = Arc::new(AtomicUsize::new(0));
    let local = Value::into_local(Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    });
    let origin = local.query::<IDerivedAbi>().unwrap().downgrade().unwrap();
    assert!(local.get().origin.set(origin).is_ok());
    let weak = local.get().origin.get().unwrap().clone();
    assert_eq!(weak.upgrade().unwrap().unwrap().value().unwrap(), 42);
    drop(local);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

#[com_object(interfaces(IShared), thread_safe)]
/// Shared implementation contains only synchronized destruction observations.
struct SharedValue {
    /// Ordinary initialized data observed only after successful promotion.
    published: AtomicU32,
    /// Incremented exactly once when the last successful promotion/owner disappears.
    drops: Arc<AtomicUsize>,
}
impl IShared for SharedValue {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.published.load(Ordering::Acquire))
    }
}
impl Drop for SharedValue {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Concurrent upgrades either own the initialized value or report expiration, never resurrect it.
#[test]
fn shared_promotion_races_final_release_without_resurrection() {
    for _ in 0..64 {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = SharedValue::from_value(SharedValue {
            published: AtomicU32::new(7),
            drops: drops.clone(),
        })
        .query::<ISharedAbi>()
        .unwrap();
        let weak = owner.downgrade().unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let peer_weak = weak.clone();
        let peer_barrier = barrier.clone();
        let peer = std::thread::spawn(move || {
            peer_barrier.wait();
            for _ in 0..32 {
                match peer_weak.upgrade().unwrap() {
                    Some(value) => assert_eq!(value.value().unwrap(), 7),
                    None => break,
                }
            }
        });
        barrier.wait();
        drop(owner);
        peer.join().unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert!(weak.upgrade().unwrap().is_none());
    }
}

/// Core smart pointers do not grow when weak lifetime support is added to allocations.
#[test]
fn strong_handles_keep_two_words_and_weak_handles_use_one() {
    assert_eq!(std::mem::size_of::<ComPtr<IUnknown>>(), 2 * std::mem::size_of::<usize>());
    assert_eq!(std::mem::size_of::<WeakComPtr<IUnknown>>(), std::mem::size_of::<usize>());
    assert_eq!(std::mem::size_of::<Option<WeakComPtr<IUnknown>>>(), std::mem::size_of::<usize>());
    assert_eq!(std::mem::size_of::<neocogi_com_sdk::NcSharedWeakRaw>(), std::mem::size_of::<usize>());
    assert_eq!(std::mem::offset_of!(neocogi_com_sdk::NcSharedWeakRaw, control), 0);
    assert_eq!(
        std::mem::size_of::<neocogi_com_sdk::lifetime::NcSharedLifetimeState>(),
        3 * std::mem::size_of::<usize>()
    );
    assert_eq!(
        std::mem::size_of::<neocogi_com_sdk::lifetime::NcBoxedLifetimeState>(),
        std::mem::size_of::<usize>()
    );
    assert_eq!(
        std::mem::size_of::<neocogi_com_sdk::lifetime::NcObjectTypeInfo>(),
        4 * std::mem::size_of::<usize>()
    );
}

/// Providers can decline weak lifetime support without inventing a wrapper lifetime.
#[test]
fn handwritten_provider_without_weak_support_returns_an_explicit_error() {
    let bytes = neocogi_com_sdk::NcBytes::from_slice(b"data");
    assert!(matches!(bytes.downgrade(), Err(neocogi_com_sdk::NC_E_NOINTERFACE)));
}

/// Shape rejection never constructs a dropping owner for an empty token.
#[test]
fn absent_weak_tokens_are_rejected_without_accessing_their_pointers() {
    // SAFETY: absent records transfer no reference and require no readable allocation.
    assert!(unsafe { WeakComPtr::<IUnknown>::from_raw_owned(neocogi_com_sdk::NcSharedWeakRaw::null()) }.is_none());
}

/// Generated weak selection preserves exact revision errors without acquiring owners.
#[test]
fn weak_queries_check_exact_interface_revision() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Value::from_value(Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    });
    let weak = owner.downgrade().unwrap();
    assert_eq!(weak.query::<WrongRevision>().unwrap_err(), neocogi_com_sdk::NC_E_ABI_VERSION);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

/// A factory rejection after identity connection drops initialized state and
/// expires any retained bookkeeping, without a construction rollback protocol.
#[test]
fn failed_factory_after_connection_drops_the_value_and_expires_its_weak_identity() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut escaped = None;
    let result = (|| -> Result<ComPtr<IValueAbi>, NcStatus> {
        let local = Value::into_local(Value {
            drops: drops.clone(),
            closed: Cell::new(false),
            origin: OnceCell::new(),
        });
        let origin = local.query::<IDerivedAbi>()?.downgrade()?;
        assert!(local.get().origin.set(origin.clone()).is_ok());
        escaped = Some(origin);
        Err(neocogi_com_sdk::NC_E_INVALID_ARG)
    })();
    assert_eq!(result.unwrap_err(), neocogi_com_sdk::NC_E_INVALID_ARG);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(escaped.unwrap().upgrade().unwrap().is_none());
}

/// Generated promotion transfers one owner and does not prolong expiration after it drops.
#[test]
fn typed_promotion_balances_owners_and_expiration() {
    use neocogi_com_sdk::{NC_E_NOINTERFACE, raw};
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Value::from_value(Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    });
    let weak = owner.query::<IValueAbi>().unwrap().downgrade().unwrap();
    assert_eq!(weak.query::<ISharedAbi>().unwrap_err(), NC_E_NOINTERFACE);
    let promoted = weak.upgrade().unwrap().unwrap();
    unsafe {
        assert_eq!(raw::validate_interface(promoted.as_raw()), Ok(()));
        assert_eq!(raw::add_ref(owner.as_raw()), 3);
        assert_eq!(raw::release(owner.as_raw()), 2);
    }
    drop(promoted);
    unsafe {
        assert_eq!(raw::add_ref(owner.as_raw()), 2);
        assert_eq!(raw::release(owner.as_raw()), 1);
    }
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

#[com_object(IValue)]
#[repr(align(128))]
/// Overaligned payload ensures shared lifetime helpers respect the concrete allocation layout.
struct AlignedValue {
    /// Domain behavior and destruction observations owned by this allocation.
    value: Value,
}
impl IValue for AlignedValue {
    /// Reads through the generated shared receiver on over-aligned storage.
    fn value(&self) -> Result<u32, NcStatus> {
        self.value.value()
    }
    /// Domain closure remains independent of the allocation's alignment and lifetime.
    fn close(&self) -> Result<(), NcStatus> {
        self.value.close()
    }
}

/// Header-first storage must preserve nondefault value alignment through promotion and final freeing.
#[test]
fn aligned_objects_keep_typed_weak_lifetimes() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = AlignedValue::from_value(AlignedValue {
        value: Value {
            drops: drops.clone(),
            closed: Cell::new(false),
            origin: OnceCell::new(),
        },
    });
    let weak = owner.query::<IValueAbi>().unwrap().downgrade().unwrap();
    let promoted = weak.upgrade().unwrap().unwrap();
    drop(owner);
    assert_eq!(promoted.value().unwrap(), 42);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(promoted);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

/// Universal identity conversion transfers the weak token without touching a value,
/// and preserves both live promotion and the expired control identity.
#[test]
fn weak_unknown_conversion_preserves_identity_without_retaining_the_value() {
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Value::from_value(Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    });
    let derived = owner.query::<IDerivedAbi>().unwrap().downgrade().unwrap();
    let identity = derived.clone().into_unknown();
    assert!(identity.same_object(&derived));
    let promoted = identity.upgrade().unwrap().unwrap();
    assert!(promoted.as_com_ref().same_object(owner.as_com_ref()));
    drop((owner, promoted));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    let expired = derived.into_unknown();
    assert!(expired.same_object(&identity));
    assert!(expired.upgrade().unwrap().is_none());
}

/// All generated concrete types publish the actual shared entry points, even in
/// debug builds: code sharing does not depend on linker folding or generics.
#[test]
fn unrelated_classes_publish_the_same_lifetime_functions() {
    use neocogi_com_sdk::lifetime::SHARED_UNKNOWN_VTABLE;
    let drops = Arc::new(AtomicUsize::new(0));
    let make = || Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    };
    let first = Value::from_value(make());
    let second = AlignedValue::from_value(AlignedValue { value: make() });
    for raw in [first.as_raw(), second.as_raw(), first.query::<IValueAbi>().unwrap().as_raw().cast()] {
        // SAFETY: both retained owners keep these immutable projection tables live.
        let table = unsafe { &*raw.vtable };
        assert!(std::ptr::fn_addr_eq(table.query_interface, SHARED_UNKNOWN_VTABLE.query_interface));
        assert!(std::ptr::fn_addr_eq(table.add_ref, SHARED_UNKNOWN_VTABLE.add_ref));
        assert!(std::ptr::fn_addr_eq(table.release, SHARED_UNKNOWN_VTABLE.release));
        assert!(std::ptr::fn_addr_eq(table.downgrade.unwrap(), SHARED_UNKNOWN_VTABLE.downgrade.unwrap()));
    }
    drop((first, second));
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

/// Passing a weak capability to another thread after factory configuration
/// observes the complete initialized state without a separate publication API.
#[test]
fn weak_identity_sent_after_configuration_observes_the_complete_value() {
    let drops = Arc::new(AtomicUsize::new(0));
    let local = SharedValue::into_local(SharedValue {
        published: AtomicU32::new(0),
        drops: drops.clone(),
    });
    local.get().published.store(0x12345678, Ordering::Release);
    let origin = local.query::<ISharedAbi>().unwrap().downgrade().unwrap();
    let peer = std::thread::spawn(move || origin.upgrade().unwrap().unwrap().value().unwrap());
    assert_eq!(peer.join().unwrap(), 0x12345678);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(local);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// A different revision of an existing IID is deliberately never implemented.
enum WrongRevision {}
// SAFETY: this marker describes IValue's physical table with a different revision;
// only rejection tests use it, and no provider publishes that revision.
unsafe impl neocogi_com_sdk::ComInterface for WrongRevision {
    const INTERFACE_ID: neocogi_com_sdk::NcInterfaceId =
        neocogi_com_sdk::NcInterfaceId::new(IValueAbi::IID, neocogi_com_sdk::NcAbiDigest::from_static(&[0; 32]));
    type VTable = IValueVTable;
    type Threading = neocogi_com_sdk::ThreadLocal;
}

/// Wire checking validates immutable metadata even when promotion is impossible.
#[test]
fn checked_weak_wire_preserves_expiration_and_rejects_wrong_contracts() {
    use neocogi_com_sdk::{ComValue, NcSharedWeakRaw, ValueBudget, NC_E_ABI_VERSION, NC_E_NOINTERFACE, NC_E_POINTER};
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Value::from_value(Value {
        drops: drops.clone(),
        closed: Cell::new(false),
        origin: OnceCell::new(),
    });
    let weak = owner.query::<IValueAbi>().unwrap().downgrade().unwrap();
    let mut owner = Some(owner);
    for expired in [false, true] {
        // SAFETY: each clone transfers one weak owner with valid common metadata.
        let checked = unsafe { WeakComPtr::<IValueAbi>::__nc_from_raw_owned_checked(weak.clone().into_raw()) }.unwrap();
        assert!(weak.same_object(&checked));
        assert_eq!(checked.upgrade().unwrap().is_none(), expired);
        let (wire, _) = weak.encode_value(&mut ValueBudget::default()).unwrap();
        // SAFETY: weak retains the borrowed wire's header through decoding.
        let acquired = unsafe { WeakComPtr::<IValueAbi>::decode_value(&wire, &mut ValueBudget::default()) }.unwrap();
        assert!(weak.same_object(&acquired));
        // SAFETY: wrong interface/revision tokens still transfer valid weak storage.
        assert_eq!(
            unsafe { WeakComPtr::<ISharedAbi>::__nc_from_raw_owned_checked(weak.clone().into_raw()) }.unwrap_err(),
            NC_E_NOINTERFACE
        );
        assert_eq!(
            unsafe { WeakComPtr::<WrongRevision>::__nc_from_raw_owned_checked(weak.clone().into_raw()) }.unwrap_err(),
            NC_E_ABI_VERSION
        );
        // SAFETY: null is valid malformed wire storage, never a valid pointer.
        assert_eq!(
            unsafe { WeakComPtr::<IValueAbi>::__nc_from_raw_owned_checked(NcSharedWeakRaw::null()) }.unwrap_err(),
            NC_E_POINTER
        );
        assert_eq!(
            unsafe { WeakComPtr::<IValueAbi>::decode_value(&NcSharedWeakRaw::null(), &mut ValueBudget::default()) }.unwrap_err(),
            NC_E_POINTER
        );
        drop((checked, acquired));
        if !expired {
            drop(owner.take());
        }
    }
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
