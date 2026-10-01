//! Weak COM lifetime is allocation-wide, independent of logical closure or queued payloads.
use neocogi_com::{com_interface, com_object, ComPtr, IUnknown, NcStatus, WeakComPtr, NC_E_CLOSED, NC_E_DROPPED};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[com_interface(auto_guid = "nc.tests.weak.Value:v1", assigned_signature = b"00000000000000000000000000000801")]
/// Test contract with closure distinct from object destruction.
pub trait IValue {
    /// Reads only while the domain is open.
    fn value(&self) -> Result<u32, NcStatus>;
    /// Logically closes a retained object without changing COM lifetime.
    fn close(&mut self) -> Result<(), NcStatus>;
}
#[com_interface(auto_guid = "nc.tests.weak.Derived:v1", assigned_signature = b"weak-derived-contract-revision01")]
/// Sibling/ancestor projections must share a single weak control identity.
pub trait IDerived: IValue {}

#[com_object(IDerived)]
/// Stores a weak self reference to prove no self-retention cycle is introduced.
struct Value {
    /// Destruction observations remain valid independently of the COM value.
    drops: Arc<AtomicUsize>,
    /// A closed object can still be successfully promoted while strongly retained.
    closed: bool,
    /// Optional self origin initialized before the object is published.
    origin: Option<WeakComPtr<IDerivedAbi>>,
}
impl IValue for Value {
    fn value(&self) -> Result<u32, NcStatus> {
        if self.closed { Err(NC_E_CLOSED) } else { Ok(42) }
    }
    fn close(&mut self) -> Result<(), NcStatus> {
        self.closed = true;
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
        closed: false,
        origin: None,
    });
    let derived = owner.query::<IDerivedAbi>().unwrap();
    let base = owner.query::<IValueAbi>().unwrap();
    assert!(derived.as_com_ref().same_object(base.as_com_ref()));
    let unrelated = Value::from_value(Value {
        drops: Arc::new(AtomicUsize::new(0)),
        closed: false,
        origin: None,
    });
    assert!(!derived.as_com_ref().same_object(unrelated.as_com_ref()));
    drop(unrelated);
    let weak_root = owner.downgrade().unwrap();
    let weak_derived = derived.as_com_ref().downgrade().unwrap();
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
        closed: false,
        origin: None,
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
    let mut owner: ComPtr<IValueAbi> = Value::from_value(Value {
        drops: drops.clone(),
        closed: false,
        origin: None,
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

/// Cyclic initialization provides a weak origin before publication, never a mutable self pointer.
#[test]
fn weak_self_initialization_is_unpublished_and_drops_without_a_cycle() {
    let drops = Arc::new(AtomicUsize::new(0));
    let local = Value::into_local_cyclic::<IDerivedAbi>(|origin| {
        assert!(origin.upgrade().unwrap().is_none());
        Value {
            drops: drops.clone(),
            closed: false,
            origin: Some(origin),
        }
    })
    .unwrap();
    let weak = local.try_borrow().unwrap().origin.as_ref().unwrap().clone();
    assert_eq!(weak.upgrade().unwrap().unwrap().value().unwrap(), 42);
    drop(local);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

#[com_interface(auto_guid = "nc.tests.weak.Shared:v1", threading = shared, assigned_signature = b"00000000000000000000000000000802")]
/// Minimal shared contract used to race final release with promotion.
pub trait IShared {
    /// Reads the initialized object under its ordinary shared COM call gate.
    fn value(&self) -> Result<u32, NcStatus>;
}
#[com_object(interfaces(IShared), thread_safe)]
/// Shared implementation contains only synchronized destruction observations.
struct SharedValue {
    /// Ordinary initialized data observed only after successful promotion.
    published: u32,
    /// Incremented exactly once when the last successful promotion/owner disappears.
    drops: Arc<AtomicUsize>,
}
impl IShared for SharedValue {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.published)
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
        let owner = SharedValue::from_value(SharedValue { published: 7, drops: drops.clone() })
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
    assert_eq!(std::mem::size_of::<neocogi_com::NcWeakRaw>(), std::mem::size_of::<usize>());
    assert_eq!(std::mem::size_of::<neocogi_com::lifetime::NcWeakControl>(), 3 * std::mem::size_of::<usize>());
    assert_eq!(std::mem::size_of::<neocogi_com::lifetime::NcClassLifetime>(), 4 * std::mem::size_of::<usize>());
}

/// Providers can decline weak lifetime support without inventing a wrapper lifetime.
#[test]
fn handwritten_provider_without_weak_support_returns_an_explicit_error() {
    let bytes = neocogi_com::NcBytes::from_slice(b"data");
    assert!(matches!(bytes.downgrade(), Err(neocogi_com::NC_E_NOINTERFACE)));
}

/// Shape rejection never constructs a dropping owner for an empty token.
#[test]
fn absent_weak_tokens_are_rejected_without_accessing_their_pointers() {
    // SAFETY: absent records transfer no reference and require no readable allocation.
    assert!(unsafe { WeakComPtr::<IUnknown>::from_raw_owned(neocogi_com::NcWeakRaw::null()) }.is_none());
}

/// Failed raw promotion clears the output and neither leaks nor transfers a strong owner.
#[test]
fn weak_promotion_checks_exact_interface_revision() {
    use neocogi_com::ComInterface;
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Value::from_value(Value {
        drops: drops.clone(),
        closed: false,
        origin: None,
    });
    let weak = owner.downgrade().unwrap();
    let raw = weak.as_raw();
    let wrong = neocogi_com::NcInterfaceId::new(IValueAbi::IID, neocogi_com::NcAbiSignature::from_static(&[0; 32]));
    let mut out = owner.as_raw();
    // SAFETY: a real live weak allocation and distinct valid records exercise a wrong signature.
    let status = unsafe { neocogi_com::lifetime::nc_weak_upgrade(raw.control, &wrong, &mut out) };
    assert_eq!(status, neocogi_com::NC_E_ABI_VERSION);
    assert!(!out.is_present());
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

/// A failed initializer cannot publish or later revive its escaped weak origin.
#[test]
fn failed_cyclic_initialization_never_publishes_a_value() {
    let mut escaped = None;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Value::from_value_cyclic::<IDerivedAbi>(|origin| {
            escaped = Some(origin);
            panic!("construction failed");
        });
    }));
    assert!(result.is_err());
    assert!(escaped.unwrap().upgrade().unwrap().is_none());
    let mut called = false;
    let result = Value::from_value_cyclic::<ISharedAbi>(|_| {
        called = true;
        panic!("unsupported identity must be rejected before initialization");
    });
    assert!(matches!(result, Err(neocogi_com::NC_E_NOINTERFACE)));
    assert!(!called);
}

/// Promotion transfers exactly one owner; neither negotiation nor failure retains extras.
#[test]
fn raw_promotion_balances_success_and_all_rejection_paths() {
    use neocogi_com::{ComInterface, NcGuid, NcInterfaceId, NC_E_NOINTERFACE, NC_E_POINTER, NC_OK};
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = Value::from_value(Value {
        drops: drops.clone(),
        closed: false,
        origin: None,
    });
    let weak = owner.downgrade().unwrap();
    let token = weak.as_raw();
    let table = owner.as_raw().vtable;
    let missing = NcInterfaceId::new(NcGuid::from_uuid_bytes([0x42; 16]), IValueAbi::INTERFACE_ID.abi_signature);
    let mut out = owner.as_raw();
    // SAFETY: all non-null arguments below are distinct live records for this allocation.
    unsafe {
        assert_eq!(neocogi_com::lifetime::nc_weak_upgrade(token.control, &missing, &mut out), NC_E_NOINTERFACE);
        assert!(!out.is_present());
        assert_eq!(neocogi_com::lifetime::nc_weak_upgrade(token.control, core::ptr::null(), &mut out), NC_E_POINTER);
        assert!(!out.is_present());
        assert_eq!(
            neocogi_com::lifetime::nc_weak_upgrade(core::ptr::null_mut(), &IValueAbi::INTERFACE_ID, &mut out),
            NC_E_POINTER
        );
        assert!(!out.is_present());
        assert_eq!(
            neocogi_com::lifetime::nc_weak_upgrade(token.control, &IValueAbi::INTERFACE_ID, core::ptr::null_mut()),
            NC_E_POINTER
        );
        // A balanced diagnostic retain/release proves failed promotion leaked no owner.
        assert_eq!(((*table).add_ref)(owner.as_raw().object), 2);
        assert_eq!(((*table).release)(owner.as_raw().object), 1);
        assert_eq!(neocogi_com::lifetime::nc_weak_upgrade(token.control, &IValueAbi::INTERFACE_ID, &mut out), NC_OK);
        assert_eq!(((*table).add_ref)(owner.as_raw().object), 3);
        assert_eq!(((*table).release)(owner.as_raw().object), 2);
        drop(ComPtr::<IUnknown>::from_raw_owned(out).unwrap());
        assert_eq!(((*table).add_ref)(owner.as_raw().object), 2);
        assert_eq!(((*table).release)(owner.as_raw().object), 1);
    }
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    // Expiration takes precedence over negotiation, preserving the original protocol.
    // SAFETY: the weak control survives value destruction; output is distinct and writable.
    assert_eq!(
        unsafe { neocogi_com::lifetime::nc_weak_upgrade(token.control, &missing, &mut out) },
        NC_E_DROPPED
    );
    assert!(!out.is_present());
}

#[com_object(IValue)]
#[repr(align(128))]
/// Overaligned payload ensures shared lifetime helpers respect the concrete allocation layout.
struct AlignedValue {
    /// Domain behavior and destruction observations owned by this allocation.
    value: Value,
}
impl IValue for AlignedValue {
    /// Forwards a read through the same generated call gate as ordinary storage.
    fn value(&self) -> Result<u32, NcStatus> {
        self.value.value()
    }
    /// Domain closure remains independent of the allocation's alignment and lifetime.
    fn close(&mut self) -> Result<(), NcStatus> {
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
            closed: false,
            origin: None,
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
        closed: false,
        origin: None,
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
    use neocogi_com::lifetime::*;
    let drops = Arc::new(AtomicUsize::new(0));
    let make = || Value {
        drops: drops.clone(),
        closed: false,
        origin: None,
    };
    let first = Value::from_value(make());
    let second = AlignedValue::from_value(AlignedValue { value: make() });
    for raw in [first.as_raw(), second.as_raw(), first.query::<IValueAbi>().unwrap().as_raw().cast()] {
        // SAFETY: both retained owners keep these immutable projection tables live.
        let table = unsafe { &*raw.vtable };
        assert!(std::ptr::fn_addr_eq(
            table.query_interface,
            nc_query_interface as unsafe extern "C" fn(_, _, _) -> _
        ));
        assert!(std::ptr::fn_addr_eq(table.add_ref, nc_strong_retain as unsafe extern "C" fn(_) -> _));
        assert!(std::ptr::fn_addr_eq(table.release, nc_strong_release as unsafe extern "C" fn(_) -> _));
        assert!(std::ptr::fn_addr_eq(table.downgrade.unwrap(), nc_downgrade as unsafe extern "C" fn(_, _) -> _));
    }
    drop((first, second));
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

/// A weak owner can cross threads before publication, but must not expose the
/// uninitialized payload. Promotion's Acquire observes the constructor's writes.
#[test]
fn concurrent_cyclic_origin_observes_only_published_value() {
    let drops = Arc::new(AtomicUsize::new(0));
    let (ready, started) = std::sync::mpsc::channel();
    let mut peer = None;
    let owner = SharedValue::from_value_cyclic::<ISharedAbi>(|origin| {
        peer = Some(std::thread::spawn(move || {
            assert!(origin.upgrade().unwrap().is_none());
            ready.send(()).unwrap();
            let limit = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                match origin.upgrade().unwrap() {
                    Some(value) => return value.value().unwrap(),
                    None => {
                        assert!(std::time::Instant::now() < limit, "constructor failed to publish");
                        std::thread::yield_now();
                    }
                }
            }
        }));
        started.recv().unwrap();
        SharedValue {
            published: 0x12345678,
            drops: drops.clone(),
        }
    })
    .unwrap();
    assert_eq!(peer.unwrap().join().unwrap(), 0x12345678);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
