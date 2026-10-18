//! Exclusive ownership through real generated ABI calls, including destruction races.
use neocogi_com::{com_interface, com_object, ComInterface, ComBox, WeakComBox, IUnknown, NcStatus, NC_E_BUSY, NC_E_DROPPED, NC_E_INVALID_ARG};
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};

#[com_interface(auto_guid = "nc.tests.exclusive.Value:v1", threading = shared, assigned_signature = b"00000000000000000000000000000901")]
/// Shared readers and exclusive writers exercise the same object-wide gate.
trait IValue {
    /// Returns state without acquiring an owning reference.
    fn value(&self) -> Result<u32, NcStatus>;
    /// Mutates state only when no reader or writer is already executing.
    fn set(&mut self, value: u32) -> Result<(), NcStatus>;
    /// Holds a reader inside the provider while its owner is removed elsewhere.
    fn wait_read(&self) -> Result<u32, NcStatus>;
    /// Holds a writer to prove conflicting weak calls fail without blocking.
    fn wait_write(&mut self) -> Result<(), NcStatus>;
}

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

/// Cached weak projections do not retain the concrete value or create strong owners.
#[test]
fn container_owner_destruction_expires_all_cached_interfaces() {
    let state = value();
    let drops = state.drops.clone();
    let mut owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    let mut weak = owner.downgrade();
    let identity = owner.query_weak::<IUnknown>().unwrap();
    // Failed observation must leave the unique owner and existing observers live.
    assert!(matches!(owner.query_weak::<IContainerAbi>(), Err(neocogi_com::NC_E_NOINTERFACE)));
    assert!(identity.same_object(&weak));
    owner.set(7).unwrap();
    assert_eq!(weak.value(), Ok(7));
    weak.set(9).unwrap();
    assert_eq!(owner.value(), Ok(9));
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
    assert_eq!(weak.set(10), Err(NC_E_DROPPED));
    assert_eq!(weak.as_com_ref().value(), Err(NC_E_DROPPED));
    assert!(identity.same_object(&weak.clone()));
    drop((weak, identity));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

/// Existing shared-ownership protocols must not bypass the exclusive owner.
#[test]
fn borrowed_arguments_and_raw_queries_cannot_create_shared_owners() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let borrow = owner.as_com_ref();
    assert!(matches!(borrow.try_add_ref(), Err(NC_E_INVALID_ARG)));
    assert!(matches!(borrow.downgrade(), Err(NC_E_INVALID_ARG)));
    let raw = owner.as_raw();
    let mut out = neocogi_com::ComRaw::null();
    // SAFETY: exact live common prefix and distinct writable output; the call
    // attempts a prohibited ownership acquisition, never malformed storage.
    unsafe {
        let table = &*raw.vtable.cast::<neocogi_com::IUnknownVTable>();
        assert_eq!((table.add_ref)(raw.object), 0);
        assert_eq!((table.query_interface)(raw.object, &IValueAbi::INTERFACE_ID, &mut out), NC_E_INVALID_ARG);
        assert!(!out.is_present());
    }
    assert_eq!(owner.value(), Ok(42));
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
        let dropping = std::thread::spawn(move || { drop(owner); finished.send(()).unwrap(); });
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

/// An ABI argument view retains bookkeeping, not the implementation. Keeping
/// one across Drop cannot reproduce the old BoxBorrow execution pin.
#[test]
fn weak_interface_views_do_not_delay_synchronous_destruction() {
    let state = value();
    let drops = state.drops.clone();
    let owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    let weak = owner.downgrade();
    let view = weak.as_com_ref();
    assert_eq!(view.value(), Ok(42));
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(view.value(), Err(NC_E_DROPPED));
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
    let result = (|| -> Result<neocogi_com::ComBox<IValueAbi, neocogi_com::ThreadSafe>, NcStatus> {
        let local = Value::into_local_box(state);
        escaped = Some(local.query::<IValueAbi>()?);
        Err(neocogi_com::NC_E_INVALID_ARG)
    })();
    assert_eq!(result.unwrap_err(), neocogi_com::NC_E_INVALID_ARG);
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

#[com_interface(auto_guid = "nc.tests.exclusive.Container:v1", assigned_signature = b"00000000000000000000000000000902")]
/// A real ABI container transfers the only owner and exposes only weak child access.
trait IContainer {
    /// Consumes its input on success and returns the same owner on failure.
    fn insert(&mut self, child: neocogi_com::ComBox<IValueAbi, neocogi_com::ThreadSafe>, reject: bool) -> Result<(), (NcStatus, neocogi_com::ComBox<IValueAbi, neocogi_com::ThreadSafe>)>;
    /// Returns an observer while the container remains the sole owner.
    fn child(&self) -> Result<WeakComBox<IValueAbi, neocogi_com::ThreadSafe>, NcStatus>;
    /// Removes the child permanently; there is no detach or close operation.
    fn remove(&mut self) -> Result<(), NcStatus>;
}
#[com_object(IContainer, ownership = exclusive)]
/// Ordinary owned state is sufficient; the ABI gate coordinates access.
struct Container {
    /// The only strong owner, transferred into this field by insert.
    child: Option<ComBox<IValueAbi, neocogi_com::ThreadSafe>>,
}
impl IContainer for Container {
    /// Validation failure leaves the input for caller-side Drop after this call.
    fn insert(&mut self, child: neocogi_com::ComBox<IValueAbi, neocogi_com::ThreadSafe>, reject: bool) -> Result<(), (NcStatus, neocogi_com::ComBox<IValueAbi, neocogi_com::ThreadSafe>)> {
        if reject {
            return Err((NC_E_INVALID_ARG, child));
        }
        self.child = Some(child);
        Ok(())
    }
    /// Cached weak output crosses the same C ABI without a shared owner.
    fn child(&self) -> Result<WeakComBox<IValueAbi, neocogi_com::ThreadSafe>, NcStatus> {
        self.child.as_ref().map(ComBox::downgrade).ok_or(neocogi_com::NC_E_NOT_FOUND)
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
        drop(ComBox::<IValueAbi, neocogi_com::ThreadSafe>::from_raw_owned(slot));
    }
    assert_eq!(weak.value(), Err(NC_E_DROPPED));
    assert_eq!(container.remove(), Ok(()));
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
    observer.with(|read| {
        assert_eq!(read.value, 17);
        assert_eq!(interface.set(18), Err(NC_E_BUSY));
    }).unwrap();
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

#[com_interface(auto_guid = "nc.tests.exclusive.Pair:v1", assigned_signature = b"00000000000000000000000000000903")]
/// Two ownership inputs prove the ABI commits or returns the complete set.
trait IPair {
    /// A false choice deliberately returns a malformed nonnegative error status;
    /// normalization must preserve both owners while reporting INVALID_ARG.
    fn offer(&mut self, left: ComBox<IValueAbi, neocogi_com::ThreadSafe>, right: ComBox<IValueAbi, neocogi_com::ThreadSafe>, accept: bool)
        -> Result<u32, (NcStatus, ComBox<IValueAbi, neocogi_com::ThreadSafe>, ComBox<IValueAbi, neocogi_com::ThreadSafe>)>;
}
#[com_object(IPair, ownership = exclusive)]
/// Ordinary fields own accepted values; dispatch admission belongs to COM.
struct Pair {
    /// Both inputs commit together, or neither is retained.
    values: Option<[ComBox<IValueAbi, neocogi_com::ThreadSafe>; 2]>,
}
impl IPair for Pair {
    /// Match expresses the whole rejection/commit decision without transfer flags.
    fn offer(&mut self, left: ComBox<IValueAbi, neocogi_com::ThreadSafe>, right: ComBox<IValueAbi, neocogi_com::ThreadSafe>, accept: bool)
        -> Result<u32, (NcStatus, ComBox<IValueAbi, neocogi_com::ThreadSafe>, ComBox<IValueAbi, neocogi_com::ThreadSafe>)> {
        match accept {
            false => Err((neocogi_com::NC_OK, left, right)),
            true => { self.values = Some([left, right]); Ok(2) },
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

/// Failed consuming interface selection also returns the original allocation,
/// so a convenience wrapper cannot accidentally discard a rejected ancestor.
#[test]
fn rejected_owner_query_returns_the_same_live_owner() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let weak = owner.downgrade();
    let (status, owner) = owner.query::<IContainerAbi>().unwrap_err();
    assert_eq!(status, neocogi_com::NC_E_NOINTERFACE);
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

#[com_interface(auto_guid = "nc.tests.exclusive.Value:v1", threading = shared, assigned_signature = b"00000000000000000000000000000902")]
/// Same nominal interface with a different ABI revision must not reuse a cached table.
trait IValueOtherRevision {
    /// This declaration is intentionally incompatible with the published Value contract.
    fn value(&self) -> Result<u64, NcStatus>;
}

/// Comparing only GUIDs would incorrectly allow this revision to bypass lookup.
#[test]
fn owner_projection_checks_the_complete_revision_before_reusing_a_table() {
    let owner = Value::into_box(value()).query::<IValueAbi>().unwrap();
    let original = owner.as_raw();
    assert_eq!(IValueAbi::IID, IValueOtherRevisionAbi::IID);
    let (status, owner) = owner.query::<IValueOtherRevisionAbi>().unwrap_err();
    assert_eq!(status, neocogi_com::NC_E_ABI_VERSION);
    assert_eq!(owner.as_raw(), original);
    assert_eq!(owner.value(), Ok(42));
}
