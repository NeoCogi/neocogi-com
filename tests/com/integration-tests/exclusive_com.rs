//! Exclusive ownership through real generated ABI calls, including destruction races.
#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
use neocogi_com_sdk::{com_object, ComInterface, ComBox, WeakComBox, IUnknown, NcStatus, NC_E_BUSY, NC_E_DROPPED, NC_E_INVALID_ARG};
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};

extern crate self as fixture_exclusive_com;
test_contracts::exclusive_com!();
// Independent revisions deliberately share an IID and negotiate by signature.
extern crate self as fixture_exclusive_com_other_revision;
test_contracts::exclusive_com_other_revision!();

#[com_object(interfaces(IValue), thread_safe, ownership = exclusive)]
/// Test object whose observations and rendezvous outlive its exclusive owner.
struct Value {
    /// Ordinary mutable state needs no RefCell or lock: the ABI gate protects it.
    value: u32,
    /// Counts concrete value destruction independently of weak storage lifetime.
    drops: Arc<AtomicUsize>,
    /// Signals that the provider is executing with admitted lifetime protection.
    entered: Arc<Barrier>,
    /// Holds execution until the test has removed ownership or attempted reentry.
    finish: Arc<Barrier>,
}
impl IValue for Value {
    /// Reads under the generated shared gate.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
    /// Writes under the generated exclusive gate.
    fn set(&mut self, value: u32) -> Result<(), NcStatus> {
        self.value = value;
        Ok(())
    }
    /// Does not release execution protection while blocked inside foreign dispatch.
    fn wait_read(&self) -> Result<u32, NcStatus> {
        self.entered.wait();
        self.finish.wait();
        Ok(self.value)
    }
    /// Keeps exclusive access through the complete rendezvous.
    fn wait_write(&mut self) -> Result<(), NcStatus> {
        self.entered.wait();
        self.finish.wait();
        self.value += 1;
        Ok(())
    }
}
impl Drop for Value {
    /// Must run exactly once after the final admitted call returns.
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Constructs a test value and independent destruction/rendezvous observations.
fn value() -> Value {
    Value {
        value: 42,
        drops: Arc::new(AtomicUsize::new(0)),
        entered: Arc::new(Barrier::new(2)),
        finish: Arc::new(Barrier::new(2)),
    }
}

/// Private dispatch observation distinguishes the copy from identical promoted constants.
static COPIED_VALUE_CALLS: AtomicUsize = AtomicUsize::new(0);
/// Forwards the exact generated checked call while observing use of copied dispatch.
unsafe extern "C" fn copied_value_call(object: *mut core::ffi::c_void, output: *mut u32) -> NcStatus {
    COPIED_VALUE_CALLS.fetch_add(1, Ordering::SeqCst);
    // SAFETY: this matching Value allocation/output satisfies the original thunk's contract.
    unsafe { (IValueVtblFor::<Value>::VTABLE.value)(object, output) }
}
/// An equivalent method table with observable forwarding, rather than pointer aliases.
static COPIED_VALUE_TABLE: IValueVTable = IValueVTable {
    value: copied_value_call,
    ..IValueVtblFor::<Value>::VTABLE
};
/// The supplied revision is stale only in its final byte; all method storage is valid.
static STALE_VALUE_DIGEST: [u8; 32] = {
    // SAFETY: generated interface identity has permanent initialized digest bytes.
    let mut bytes = unsafe { *IValueAbi::ABI_SIGNATURE.as_digest().unwrap() };
    bytes[31] ^= 1;
    bytes
};
/// Exact IID with an intentionally different expected contract revision.
static STALE_VALUE_ID: neocogi_com_sdk::NcInterfaceId =
    neocogi_com_sdk::NcInterfaceId::new(IValueAbi::IID, neocogi_com_sdk::NcAbiDigest::from_static(&STALE_VALUE_DIGEST));
/// Matching Value entry points with stale incoming identity for raw rejection tests.
static STALE_VALUE_TABLE: IValueVTable = IValueVTable {
    unknown: neocogi_com_sdk::IUnknownVTable {
        interface_id: &STALE_VALUE_ID,
        ..COPIED_VALUE_TABLE.unknown
    },
    ..COPIED_VALUE_TABLE
};

/// Transfers a genuine Value owner into its equivalent noncanonical table.
fn copied_value_owner(state: Value) -> ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe> {
    let expected = state.value;
    let raw = Value::into_box(state).query::<IValueAbi>().unwrap().into_raw();
    assert_ne!(raw.vtable, &COPIED_VALUE_TABLE as *const _);
    // SAFETY: object, threading, identity and methods match this concrete Value.
    // Move its existing sole owner; the copied table and provider code are static.
    let owner: ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe> = unsafe {
        ComBox::from_raw_owned(neocogi_com_sdk::ComRaw {
            object: raw.object,
            vtable: &COPIED_VALUE_TABLE,
        })
    }
    .unwrap();
    let calls = COPIED_VALUE_CALLS.load(Ordering::SeqCst);
    assert_eq!(owner.value(), Ok(expected));
    assert_eq!(COPIED_VALUE_CALLS.load(Ordering::SeqCst), calls + 1);
    owner
}

/// Cached weak projections do not retain the concrete value or create strong owners.
#[test]
fn container_owner_destruction_expires_all_cached_interfaces() {
    let state = value();
    let drops = state.drops.clone();
    let mut owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    let mut weak = owner.downgrade();
    let identity = owner.query_weak::<IUnknown>().unwrap();
    // Failed observation must leave the unique owner and existing observers live.
    assert!(matches!(owner.query_weak::<IContainerAbi>(), Err(neocogi_com_sdk::NC_E_NOINTERFACE)));
    assert!(identity.same_object(&weak));
    owner.set(7).unwrap();
    assert_eq!(weak.value(), Ok(7));
    weak.set(9).unwrap();
    assert_eq!(owner.value(), Ok(9));
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
    assert_eq!(weak.set(10), Err(NC_E_DROPPED));
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
    assert!(identity.same_object(&weak.clone()));
    drop((weak, identity));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Consuming weak erasure preserves canonical dispatch, expiration and both threading modes.
#[test]
fn weak_erasure_reprojects_live_and_expired_generated_interfaces() {
    for expired in [false, true] {
        let state = value();
        let drops = state.drops.clone();
        let root = Value::into_box(state);
        let canonical_unknown = root.as_raw().vtable;
        let owner = root.query::<IValueAbi>().unwrap();
        let source = owner.downgrade();
        let object = source.as_raw().object;
        let owner = match expired {
            true => {
                drop(owner);
                None
            }
            false => Some(owner),
        };
        let erased: WeakComBox<IUnknown, neocogi_com_sdk::ThreadSafe> = source.into_unknown();
        assert_eq!(erased.as_raw().object, object);
        assert_eq!(erased.as_raw().vtable, canonical_unknown);
        assert_eq!(erased.is_alive(), !expired);
        let mut projected = erased.query::<IValueAbi>().unwrap();
        assert!(erased.same_object(&projected));
        let expected = if expired { Err(NC_E_DROPPED) } else { Ok(42) };
        std::thread::scope(|scope| {
            // The erased/reprojected handle retains the provider's ThreadSafe permission.
            assert_eq!(scope.spawn(|| projected.value()).join().unwrap(), expected);
        });
        assert_eq!(projected.set(57), if expired { Err(NC_E_DROPPED) } else { Ok(()) });
        if let Some(owner) = &owner {
            assert_eq!(owner.value(), Ok(57));
        }
        let local: WeakComBox<IValueAbi, neocogi_com_sdk::ThreadLocal> = projected.clone().into_thread_local();
        let local_root: WeakComBox<IUnknown, neocogi_com_sdk::ThreadLocal> = local.into_unknown();
        assert!(erased.same_object(&local_root));
        assert_eq!(local_root.as_raw().vtable, canonical_unknown);
        let local_projection = local_root.query::<IValueAbi>().unwrap();
        assert_eq!(local_projection.value(), if expired { Err(NC_E_DROPPED) } else { Ok(57) });
        assert_eq!(drops.load(Ordering::SeqCst), usize::from(expired));
        drop(owner);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(projected.value(), Err(NC_E_DROPPED));
        assert_eq!(projected.set(91), Err(NC_E_DROPPED));
        assert_eq!(local_projection.value(), Err(NC_E_DROPPED));
        drop((projected, local_projection, local_root, erased));
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
}

/// Existing shared-ownership protocols must not bypass the exclusive owner.
#[test]
fn borrowed_arguments_and_raw_queries_cannot_create_shared_owners() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let borrow = owner.as_com_ref();
    let raw = borrow.as_raw();
    let mut out = neocogi_com_sdk::ComRaw::null();
    // SAFETY: exact live common prefix and distinct writable output; the call
    // attempts a prohibited ownership acquisition, never malformed storage.
    unsafe {
        let table = &*raw.vtable.cast::<neocogi_com_sdk::IUnknownVTable>();
        assert_eq!((table.add_ref)(raw.object), 0);
        assert_eq!((table.query_interface)(raw.object, &IValueAbi::INTERFACE_ID, &mut out), NC_E_INVALID_ARG);
        assert!(!out.is_present());
        let mut weak = neocogi_com_sdk::NcSharedWeakRaw::null();
        assert_eq!((table.downgrade.unwrap())(raw.object, &mut weak), NC_E_INVALID_ARG);
    }
    assert_eq!(borrow.value(), Ok(42));
}

/// Drop stops new calls, waits for an admitted reader/writer, and destroys the
/// value on the dropping thread before returning. No timing-based sleep is needed.
#[test]
fn thread_safe_owner_drop_waits_for_admitted_readers_and_writers() {
    for writer in [false, true] {
        let state = value();
        let drops = state.drops.clone();
        let entered = state.entered.clone();
        let finish = state.finish.clone();
        let owner = Value::into_box(state).query::<IValueAbi>().unwrap();
        let weak = owner.downgrade();
        let mut peer = weak.clone();
        let worker = std::thread::spawn(move || match writer {
            true => peer.wait_write().map(|()| 43),
            false => peer.wait_read(),
        });
        entered.wait();
        assert_eq!(weak.value(), if writer { Err(NC_E_BUSY) } else { Ok(42) });
        let (finished, received) = std::sync::mpsc::channel();
        let dropping = std::thread::spawn(move || {
            drop(owner);
            finished.send(()).unwrap();
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while weak.is_alive() {
            assert!(std::time::Instant::now() < deadline, "owner did not start destruction");
            std::thread::yield_now();
        }
        assert_eq!(weak.value(), Err(NC_E_DROPPED));
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert!(received.try_recv().is_err(), "Drop must wait for the active method");
        finish.wait();
        assert_eq!(worker.join().unwrap(), Ok(if writer { 43 } else { 42 }));
        received.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        dropping.join().unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(weak.value(), Err(NC_E_DROPPED));
    }
}

/// A cached weak handle keeps bookkeeping alive across synchronous owner Drop.
#[test]
fn weak_interface_views_do_not_delay_synchronous_destruction() {
    let state = value();
    let drops = state.drops.clone();
    let owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    let weak = owner.downgrade();
    assert_eq!(weak.value(), Ok(42));
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
}

/// The cached fast path preserves the exclusive method gate across threads.
#[test]
fn concurrent_mutable_access_reports_busy_without_waiting() {
    let state = value();
    let entered = state.entered.clone();
    let finish = state.finish.clone();
    let owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    let mut weak = owner.downgrade();
    let mut peer = weak.clone();
    let worker = std::thread::spawn(move || peer.wait_write());
    entered.wait();
    assert_eq!(weak.value(), Err(NC_E_BUSY));
    assert_eq!(weak.set(3), Err(NC_E_BUSY));
    finish.wait();
    assert_eq!(worker.join().unwrap(), Ok(()));
    assert_eq!(weak.value(), Ok(43));
}

/// Rejection after ordinary allocation destroys the fully initialized value;
/// a weak identity saved during private wiring cannot keep it alive or callable.
#[test]
fn failed_factory_after_allocation_expires_its_exclusive_identity() {
    let state = value();
    let drops = state.drops.clone();
    let mut escaped = None;
    let result = (|| -> Result<neocogi_com_sdk::ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>, NcStatus> {
        let local = Value::into_local_box(state);
        escaped = Some(local.query::<IValueAbi>()?);
        Err(neocogi_com_sdk::NC_E_INVALID_ARG)
    })();
    assert_eq!(result.unwrap_err(), neocogi_com_sdk::NC_E_INVALID_ARG);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(escaped.unwrap().value(), Err(NC_E_DROPPED));
}

/// Racing callers either finish under protection or observe terminal expiration.
#[test]
fn repeated_cross_thread_calls_cannot_resurrect_removed_owners() {
    for _ in 0..100 {
        let state = value();
        let drops = state.drops.clone();
        let owner = Value::into_box(state).query::<IValueAbi>().unwrap();
        let weak = owner.downgrade();
        let peer = weak.clone();
        let worker = std::thread::spawn(move || {
            for _ in 0..100 {
                match peer.value() {
                    Ok(value) => assert_eq!(value, 42),
                    Err(NC_E_DROPPED) => return,
                    result => panic!("unexpected read result: {result:?}"),
                }
            }
        });
        drop(owner);
        worker.join().unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(weak.value(), Err(NC_E_DROPPED));
    }
}

/// The optimized weak handle caches a table without adding another allocation.
#[test]
fn owners_and_cached_weak_handles_are_two_words() {
    assert_eq!(size_of::<ComBox<IUnknown>>(), 2 * size_of::<usize>());
    assert_eq!(size_of::<WeakComBox<IUnknown>>(), 2 * size_of::<usize>());
    assert_eq!(size_of::<Option<ComBox<IUnknown>>>(), 2 * size_of::<usize>());
}

#[com_object(IContainer, ownership = exclusive)]
/// Ordinary owned state is sufficient; the ABI gate coordinates access.
struct Container {
    /// The only strong owner, transferred into this field by insert.
    child: Option<ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>>,
}
impl IContainer for Container {
    /// Validation failure leaves the input for caller-side Drop after this call.
    fn insert(
        &mut self,
        child: neocogi_com_sdk::ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
        reject: bool,
    ) -> Result<(), (NcStatus, neocogi_com_sdk::ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>)> {
        // Admission must give the implementation canonical dispatch, even when
        // the caller supplies a distinct equivalent table or this method rejects.
        assert_eq!(child.as_raw().vtable, child.query_ref::<IValueAbi>().unwrap().as_raw().vtable);
        if reject {
            return Err((NC_E_INVALID_ARG, child));
        }
        self.child = Some(child);
        Ok(())
    }
    /// Cached weak output crosses the same C ABI without a shared owner.
    fn child(&self) -> Result<WeakComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>, NcStatus> {
        self.child.as_ref().map(ComBox::downgrade).ok_or(neocogi_com_sdk::NC_E_NOT_FOUND)
    }
    /// Releasing the owner expires every external weak projection.
    fn remove(&mut self) -> Result<(), NcStatus> {
        self.child = None;
        Ok(())
    }
}

/// Tests consuming inputs and weak outputs through the generated C method table.
#[test]
fn abi_container_transfer_and_removal_preserve_exclusive_ownership() {
    let mut container = Container::into_box(Container { child: None }).query::<IContainerAbi>().unwrap();
    let child = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let weak = child.downgrade();
    container.insert(child, false).unwrap();
    assert_eq!(weak.value(), Ok(42));
    assert!(container.child().unwrap().same_object(&weak));
    container.remove().unwrap();
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
    let child = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let rejected = child.downgrade();
    let (status, child) = container.insert(child, true).unwrap_err();
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!(rejected.value(), Ok(42), "rejection returns a live owner");
    container.insert(child, false).unwrap();
    container.remove().unwrap();
    assert_eq!(rejected.value(), Err(NC_E_DROPPED));
    let child = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let expired_call = child.downgrade();
    let mut expired_container = container.downgrade();
    drop(container);
    let (status, child) = expired_container.insert(child, false).unwrap_err();
    assert_eq!(status, NC_E_DROPPED);
    assert_eq!(expired_call.value(), Ok(42), "expired receiver returns an untouched input");
    drop(child);
    assert_eq!(expired_call.value(), Err(NC_E_DROPPED));
}

/// Raw malformed inputs fail before transfer and leave the caller's owner balanced.
#[test]
fn raw_transfer_rejection_preserves_the_input_slot() {
    let mut container = Container::into_box(Container { child: None }).query::<IContainerAbi>().unwrap();
    let child = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let weak = child.downgrade();
    let raw = child.into_raw();
    let mut slot = raw;
    let target = container.as_raw();
    // SAFETY: both objects, the writable slot, and raw bool storage are valid.
    // Bool 2 is deliberately rejected by the checked codec, not read as Rust bool.
    unsafe {
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 2), NC_E_INVALID_ARG);
        assert_eq!(slot.object, raw.object);
        assert_eq!(slot.vtable, raw.vtable);
        assert_eq!(weak.value(), Ok(42), "malformed later argument cannot consume input");
        drop(ComBox::<IValueAbi, neocogi_com_sdk::ThreadSafe>::from_raw_owned(slot));
    }
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
    assert_eq!(container.remove(), Ok(()));
}

/// Metadata lookup precedes admission, while canonical adoption waits for the
/// complete call. Every rejection preserves both original caller pointer words.
#[test]
fn copied_boxed_input_tables_preserve_rejections_and_commit_canonical_dispatch() {
    let local = Container::into_local_box(Container { child: None });
    let mut container = local.query::<IContainerAbi>().unwrap();
    let state = value();
    let drops = state.drops.clone();
    let child = copied_value_owner(state);
    let canonical = child.query_ref::<IValueAbi>().unwrap().as_raw().vtable;
    let weak = child.downgrade();
    let original = child.as_raw();
    let mut slot = child.into_raw();
    let target = container.as_raw();
    // SAFETY: real retained objects, distinct writable slot and matching method
    // entry points. Only identity or an initialized raw bool is malformed.
    unsafe {
        slot.vtable = &STALE_VALUE_TABLE;
        let stale = slot;
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 0), neocogi_com_sdk::NC_E_ABI_VERSION);
        assert_eq!(slot, stale);
        slot = original;
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 2), NC_E_INVALID_ARG);
        assert_eq!(slot, original);
        let held = local.try_borrow().unwrap();
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 0), NC_E_BUSY);
        assert_eq!(slot, original);
        drop(held);
        assert_eq!(((*target.vtable).insert)(target.object, &mut slot, 1), NC_E_INVALID_ARG);
        assert_eq!(slot, original);
        assert_eq!(weak.value(), Ok(42));
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        // Rebuild the same live owner only after proving the slot was returned.
        let child = ComBox::<IValueAbi, neocogi_com_sdk::ThreadSafe>::from_raw_owned(slot).unwrap();
        let (status, child) = container.insert(child, true).unwrap_err();
        assert_eq!(status, NC_E_INVALID_ARG);
        assert_eq!(child.as_raw(), original);
        container.insert(child, false).unwrap();
    }
    {
        let state = local.try_borrow().unwrap();
        assert_eq!(state.child.as_ref().unwrap().as_raw().vtable, canonical);
    }
    container.remove().unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
}

/// Concrete initialization uses the same borrow gate and cannot keep a moved owner alive.
#[test]
fn local_box_observers_follow_owner_transfer_and_gate_conflicts() {
    let state = value();
    let drops = state.drops.clone();
    let mut local = Value::into_local_box(state);
    local.try_borrow_mut().unwrap().value = 17;
    let mut observer = local.downgrade();
    let mut interface = local.query::<IValueAbi>().unwrap();
    observer
        .with(|read| {
            assert_eq!(read.value, 17);
            assert_eq!(interface.set(18), Err(NC_E_BUSY));
        })
        .unwrap();
    let owner = local.into_interface::<IValueAbi>().unwrap();
    observer.with_mut(|value| value.value = 19).unwrap();
    assert_eq!(interface.value(), Ok(19));
    assert!(interface.is_alive());
    // The exclusive owner establishes the layout needed for a weak observer.
    let origin = owner.downgrade();
    assert!(origin.same_object(&interface));
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!interface.is_alive());
    assert_eq!(observer.with(|_| ()), Err(NC_E_DROPPED));
    assert_eq!(observer.with_mut(|_| ()), Err(NC_E_DROPPED));
    assert_eq!(origin.value(), Err(NC_E_DROPPED));
}

#[com_object(IPair, ownership = exclusive)]
/// Ordinary fields own accepted values; dispatch admission belongs to COM.
struct Pair {
    /// Both inputs commit together, or neither is retained.
    values: Option<[ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>; 2]>,
}
impl IPair for Pair {
    /// Match expresses the whole rejection/commit decision without transfer flags.
    fn offer(
        &mut self,
        left: ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
        right: ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
        accept: bool,
    ) -> Result<
        u32,
        (
            NcStatus,
            ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
            ComBox<IValueAbi, neocogi_com_sdk::ThreadSafe>,
        ),
    > {
        for owner in [&left, &right] {
            assert_eq!(owner.as_raw().vtable, owner.query_ref::<IValueAbi>().unwrap().as_raw().vtable);
        }
        match accept {
            false => Err((neocogi_com_sdk::NC_OK, left, right)),
            true => {
                self.values = Some([left, right]);
                Ok(2)
            }
        }
    }
}

/// Both rejection and receiver admission failure return both live original owners;
/// a later successful call transfers them and writes its ordinary scalar output.
#[test]
fn multiple_owner_inputs_survive_rejection_busy_admission_and_retry() {
    let mut local = Pair::into_local_box(Pair { values: None });
    let mut pair = local.query::<IPairAbi>().unwrap();
    let left = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let right = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let observed = [left.downgrade(), right.downgrade()];
    let (status, left, right) = pair.offer(left, right, false).unwrap_err();
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!(left.as_raw().object, observed[0].as_raw().object);
    assert_eq!(right.as_raw().object, observed[1].as_raw().object);
    let guard = local.try_borrow_mut().unwrap();
    let (status, left, right) = pair.offer(left, right, true).unwrap_err();
    assert_eq!(status, NC_E_BUSY);
    assert!(observed.iter().all(|weak| weak.value() == Ok(42)));
    drop(guard);
    assert_eq!(pair.offer(left, right, true).unwrap(), 2);
    drop(local);
    assert!(observed.iter().all(|weak| weak.value() == Err(NC_E_DROPPED)));
}

/// A later bad input must leave earlier copied tables untouched; body rejection
/// restores every original pair after seeing canonical owners, then retry commits.
#[test]
fn copied_multi_owner_inputs_restore_each_original_table_before_retry() {
    let local = Pair::into_local_box(Pair { values: None });
    let mut pair = local.query::<IPairAbi>().unwrap();
    let left_state = value();
    let left_drops = left_state.drops.clone();
    let right_state = value();
    let right_drops = right_state.drops.clone();
    let left = copied_value_owner(left_state);
    let right = copied_value_owner(right_state);
    let canonical = left.query_ref::<IValueAbi>().unwrap().as_raw().vtable;
    let observed = [left.downgrade(), right.downgrade()];
    let originals = [left.as_raw(), right.as_raw()];
    let mut left_slot = left.into_raw();
    let mut right_slot = right.into_raw();
    let target = pair.as_raw();
    let mut output = 17;
    // SAFETY: both retained Value allocations/tables and all scalar/slot storage
    // are valid. The second incoming identity is deliberately stale, then repaired.
    let (left, right) = unsafe {
        right_slot.vtable = &STALE_VALUE_TABLE;
        let stale = right_slot;
        assert_eq!(
            ((*target.vtable).offer)(target.object, &mut left_slot, &mut right_slot, 1, &mut output),
            neocogi_com_sdk::NC_E_ABI_VERSION
        );
        assert_eq!(left_slot, originals[0]);
        assert_eq!(right_slot, stale);
        assert_eq!(output, 17);
        right_slot = originals[1];
        (
            ComBox::<IValueAbi, neocogi_com_sdk::ThreadSafe>::from_raw_owned(left_slot).unwrap(),
            ComBox::<IValueAbi, neocogi_com_sdk::ThreadSafe>::from_raw_owned(right_slot).unwrap(),
        )
    };
    let (status, left, right) = pair.offer(left, right, false).unwrap_err();
    assert_eq!(status, NC_E_INVALID_ARG);
    assert_eq!(left.as_raw(), originals[0]);
    assert_eq!(right.as_raw(), originals[1]);
    assert!(observed.iter().all(|weak| weak.value() == Ok(42)));
    assert_eq!(pair.offer(left, right, true).unwrap(), 2);
    {
        let state = local.try_borrow().unwrap();
        assert!(state.values.as_ref().unwrap().iter().all(|owner| owner.as_raw().vtable == canonical));
    }
    drop(local);
    assert!(observed.iter().all(|weak| weak.value() == Err(NC_E_DROPPED)));
    assert_eq!(left_drops.load(Ordering::SeqCst), 1);
    assert_eq!(right_drops.load(Ordering::SeqCst), 1);
}

/// Failed consuming interface selection also returns the original allocation,
/// so a convenience wrapper cannot accidentally discard a rejected ancestor.
#[test]
fn rejected_owner_query_returns_the_same_live_owner() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let weak = owner.downgrade();
    let (status, owner) = owner.query::<IContainerAbi>().unwrap_err();
    assert_eq!(status, neocogi_com_sdk::NC_E_NOINTERFACE);
    assert_eq!(owner.value(), Ok(42));
    drop(owner);
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
}

/// An identity projection moves the sole token without changing its table or
/// adding ownership; destruction still expires every external weak projection.
#[test]
fn owner_identity_projection_preserves_cached_table_and_destroys_once() {
    let state = value();
    let drops = state.drops.clone();
    let owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    let weak = owner.downgrade();
    let original = owner.as_raw();
    let mut owner = owner.query::<IValueAbi>().unwrap();
    assert_eq!(owner.as_raw(), original);
    owner.set(57).unwrap();
    assert_eq!(weak.value(), Ok(57));
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
    drop(weak);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Comparing only GUIDs would incorrectly allow this revision to bypass lookup.
#[test]
fn owner_projection_checks_the_complete_revision_before_reusing_a_table() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let original = owner.as_raw();
    assert_eq!(IValueAbi::IID, IValueOtherRevisionAbi::IID);
    let (status, owner) = owner.query::<IValueOtherRevisionAbi>().unwrap_err();
    assert_eq!(status, neocogi_com_sdk::NC_E_ABI_VERSION);
    assert_eq!(owner.as_raw(), original);
    assert_eq!(owner.value(), Ok(42));
}

#[com_object(interfaces(), ownership = exclusive)]
#[repr(align(128))]
/// Generic over-aligned state exercises typed exclusive allocation and final weak freeing.
struct AlignedBox<T: 'static> {
    /// Concrete payload survives local/interface projection changes.
    marker: T,
    /// Counts value destruction while weak storage remains retained.
    drops: Arc<AtomicUsize>,
}
impl<T> Drop for AlignedBox<T> {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// Exclusive storage preserves payload alignment and destroys T before final weak storage release.
#[test]
fn generic_exclusive_storage_preserves_alignment_and_weak_expiration() {
    let drops = Arc::new(AtomicUsize::new(0));
    let local = AlignedBox::into_local_box(AlignedBox { marker: 29u32, drops: drops.clone() });
    let concrete = local.downgrade();
    let unknown = local.query::<IUnknown>().unwrap();
    assert_eq!(unknown.as_raw().object as usize % 128, 0);
    concrete
        .with(|state| {
            assert_eq!(state.marker, 29);
            assert_eq!(state as *const _ as usize % 128, 0);
        })
        .unwrap();
    let owner = local.into_interface::<IUnknown>().unwrap();
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(!unknown.is_alive());
    assert_eq!(concrete.with(|_| ()), Err(NC_E_DROPPED));
    drop((concrete, unknown));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Borrowed getters/writers execute the real ABI, preserve threading, and share concrete admission.
#[test]
fn borrowed_interface_queries_preserve_identity_errors_and_the_call_gate() {
    let local = Value::into_local_box(value());
    let observer = local.downgrade();
    let mut owner = local.into_interface::<IValueAbi>().unwrap();
    let projection = owner.query_ref::<IValueAbi>().unwrap();
    assert_eq!(projection.as_raw(), owner.as_raw());
    assert_eq!(projection.value(), Ok(42));
    {
        let mut edit = owner.as_com_mut();
        assert_eq!(edit.set(53), Ok(()));
        observer.with(|_| assert_eq!(edit.set(71), Err(NC_E_BUSY))).unwrap();
        let projection = edit.as_com_ref();
        std::thread::scope(|scope| {
            scope.spawn(|| assert_eq!(projection.value(), Ok(53))).join().unwrap();
        });
    }
    assert!(matches!(owner.query_ref::<IContainerAbi>(), Err(neocogi_com_sdk::NC_E_NOINTERFACE)));
    assert!(matches!(owner.query_ref::<IValueOtherRevisionAbi>(), Err(neocogi_com_sdk::NC_E_ABI_VERSION)));
    assert_eq!(owner.value(), Ok(53));
}

/// Root and generated projections reuse only the exclusive callback family.
#[test]
fn generated_exclusive_projections_select_their_policy_prefix() {
    use neocogi_com_sdk::lifetime::{SHARED_UNKNOWN_VTABLE, EXCLUSIVE_UNKNOWN_VTABLE};
    let state = value();
    let drops = state.drops.clone();
    let owner = Value::into_box(state);
    let root = owner.as_raw();
    let interface = owner.query::<IValueAbi>().unwrap();
    for raw in [root, interface.as_raw().cast()] {
        // SAFETY: interface retains the allocation and immutable tables for both views.
        let table = unsafe { &*raw.vtable };
        assert!(std::ptr::fn_addr_eq(table.release, EXCLUSIVE_UNKNOWN_VTABLE.release));
        assert!(std::ptr::fn_addr_eq(table.add_ref, EXCLUSIVE_UNKNOWN_VTABLE.add_ref));
        assert!(std::ptr::fn_addr_eq(table.query_interface, EXCLUSIVE_UNKNOWN_VTABLE.query_interface));
        assert!(std::ptr::fn_addr_eq(table.downgrade.unwrap(), EXCLUSIVE_UNKNOWN_VTABLE.downgrade.unwrap()));
        assert!(!std::ptr::fn_addr_eq(table.release, SHARED_UNKNOWN_VTABLE.release));
    }
    drop(interface);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
