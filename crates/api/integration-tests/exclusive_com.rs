//! Exclusive ownership through real generated ABI calls, including destruction races.
use neocogi_com::{com_interface, com_object, ComInterface, ComBox, WeakComBox, IUnknown, NcStatus, NC_E_BUSY, NC_E_CLOSED, NC_E_INVALID_ARG};
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

#[com_object(interfaces(IValue), thread_safe)]
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
    let identity = weak.query::<IUnknown>().unwrap();
    assert!(identity.same_object(&weak));
    owner.set(7).unwrap();
    assert_eq!(weak.value(), Ok(7));
    weak.set(9).unwrap();
    assert_eq!(owner.value(), Ok(9));
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(weak.value(), Err(NC_E_CLOSED));
    assert_eq!(weak.set(10), Err(NC_E_CLOSED));
    assert!(matches!(weak.borrow(), Err(NC_E_CLOSED)));
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

/// Owner removal is nonblocking and rejects new calls while an earlier reader finishes.
#[test]
fn thread_safe_owner_drop_waits_only_for_admitted_execution() {
    let state = value();
    let drops = state.drops.clone();
    let entered = state.entered.clone();
    let finish = state.finish.clone();
    let owner = Value::into_box(state).query::<IValueAbi>().unwrap();
    let weak = owner.downgrade();
    let peer = weak.clone();
    let worker = std::thread::spawn(move || peer.wait_read());
    entered.wait();
    assert_eq!(weak.value(), Ok(42), "readers can overlap");
    drop(owner);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(weak.value(), Err(NC_E_CLOSED));
    finish.wait();
    assert_eq!(worker.join().unwrap(), Ok(42));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(weak.value(), Err(NC_E_CLOSED));
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

/// Publication is the only transition from uninitialized to callable boxed storage.
#[test]
fn cyclic_construction_and_failure_never_expose_uninitialized_values() {
    let owner = Value::into_box_cyclic::<IValueAbi>(|weak| {
        assert_eq!(weak.value(), Err(NC_E_CLOSED));
        value()
    })
    .unwrap();
    assert_eq!(owner.query::<IValueAbi>().unwrap().value(), Ok(42));
    let mut escaped = None;
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Value::into_box_cyclic::<IValueAbi>(|weak| {
            escaped = Some(weak);
            panic!("constructor rollback");
        });
    }));
    assert!(failed.is_err());
    assert_eq!(escaped.unwrap().value(), Err(NC_E_CLOSED));
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
                    Err(NC_E_CLOSED) => return,
                    result => panic!("unexpected read result: {result:?}"),
                }
            }
        });
        drop(owner);
        worker.join().unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(weak.value(), Err(NC_E_CLOSED));
    }
}

/// The optimized weak handle caches a table without adding another allocation.
#[test]
fn owners_and_cached_weak_handles_are_two_words() {
    assert_eq!(size_of::<ComBox<IUnknown>>(), 2 * size_of::<usize>());
    assert_eq!(size_of::<WeakComBox<IUnknown>>(), 2 * size_of::<usize>());
    assert_eq!(size_of::<Option<ComBox<IUnknown>>>(), 2 * size_of::<usize>());
}
