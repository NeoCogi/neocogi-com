//! Checks shared ownership and alias exclusion between typed access and COM.

use neocogi_com::{com_interface, com_object, NcStatus, NC_E_BUSY};
use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

#[com_interface(auto_guid = "nc.tests.local.Counter:v1", assigned_signature = b"00000000000000000000000000000061")]
/// The foreign projection deliberately exposes less than the concrete model.
pub trait ICounter {
    /// Reads the authoritative value under the shared COM gate.
    fn value(&self) -> Result<u32, NcStatus>;
    /// Changes that same value under the exclusive COM gate.
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus>;
}

#[com_object(interfaces(ICounter))]
/// Owned state survives whichever kind of handle is dropped first.
struct Counter {
    /// Ordinary mutable state, with no RefCell coordinating local and COM access.
    value: u32,
    /// External test observation records final destruction without owning this model.
    drops: Rc<AtomicUsize>,
}

impl ICounter for Counter {
    /// Observes state only after the generated shared gate succeeds.
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value)
    }
    /// Writes state only after the generated exclusive gate succeeds.
    fn set_value(&mut self, value: u32) -> Result<(), NcStatus> {
        self.value = value;
        Ok(())
    }
}

impl Drop for Counter {
    /// Counts final destruction independently of which projection owns the last reference.
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

/// A local mutable borrow excludes all aliased COM and local access, then releases cleanly.
#[test]
fn local_and_foreign_calls_share_one_gate_and_one_model() {
    let drops = Rc::new(AtomicUsize::new(0));
    let local = Counter::into_local(Counter { value: 1, drops: drops.clone() });
    let alias = local.clone();
    let mut foreign = local.query::<ICounterAbi>().unwrap();
    {
        let mut model = local.try_borrow_mut().unwrap();
        model.value = 2;
        assert_eq!(foreign.value(), Err(NC_E_BUSY));
        assert_eq!(foreign.set_value(3), Err(NC_E_BUSY));
        assert!(matches!(alias.try_borrow(), Err(NC_E_BUSY)));
        assert!(matches!(alias.try_borrow_mut(), Err(NC_E_BUSY)));
    }
    assert_eq!(foreign.value(), Ok(2));
    {
        let model = local.try_borrow().unwrap();
        assert_eq!(foreign.value(), Ok(model.value));
        assert_eq!(foreign.set_value(3), Err(NC_E_BUSY));
        assert_eq!(alias.try_borrow().unwrap().value, 2);
    }
    foreign.set_value(4).unwrap();
    assert_eq!(local.try_borrow().unwrap().value, 4);
    drop(local);
    drop(alias);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert_eq!(foreign.value(), Ok(4));
    drop(foreign);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

/// Rust unwinding releases typed access without invalidating retained COM projections.
#[test]
fn local_guard_releases_on_unwind_and_outlives_foreign_handles() {
    let drops = Rc::new(AtomicUsize::new(0));
    let local = Counter::into_local(Counter { value: 1, drops: drops.clone() });
    let foreign = local.query::<ICounterAbi>().unwrap();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut model = local.try_borrow_mut().unwrap();
        model.value = 7;
        panic!("release the local guard");
    }));
    assert!(failure.is_err());
    assert_eq!(foreign.value(), Ok(7));
    drop(foreign);
    assert_eq!(local.try_borrow().unwrap().value, 7);
    drop(local);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

/// Interface owners keep typed weak promotion valid after all local owners leave.
#[test]
fn weak_local_lifetime_follows_every_strong_projection() {
    let drops = Rc::new(AtomicUsize::new(0));
    let local = Counter::into_local(Counter { value: 11, drops: drops.clone() });
    let weak = local.downgrade().unwrap();
    let alias = weak.clone();
    let foreign = local.query::<ICounterAbi>().unwrap();
    let weak_interface = foreign.downgrade().unwrap();
    drop(local);
    let promoted = alias.upgrade().unwrap().unwrap();
    assert_eq!(promoted.try_borrow().unwrap().value, 11);
    assert!(promoted.query::<ICounterAbi>().unwrap().as_com_ref().same_object(foreign.as_com_ref()));
    drop(foreign);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    promoted.try_borrow_mut().unwrap().value = 12;
    assert_eq!(weak_interface.upgrade().unwrap().unwrap().value(), Ok(12));
    drop(promoted);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
    assert!(weak.upgrade().unwrap().is_none());
    assert!(alias.clone().upgrade().unwrap().is_none());
    assert!(weak_interface.upgrade().unwrap().is_none());
    assert!(format!("{weak:?}").starts_with("WeakLocalCom("));
    let replacement = Counter::into_local(Counter { value: 99, drops: drops.clone() });
    assert!(weak.upgrade().unwrap().is_none(), "a new allocation cannot revive an old observer");
    drop((replacement, alias, weak));
    assert_eq!(drops.load(Ordering::Relaxed), 2);
}

/// Promotion takes ownership, while local and foreign calls keep sharing one gate.
#[test]
fn weak_local_promotion_preserves_busy_checks_and_unwind_recovery() {
    let drops = Rc::new(AtomicUsize::new(0));
    let local = Counter::into_local(Counter { value: 1, drops: drops.clone() });
    let weak = local.downgrade().unwrap();
    let mut foreign = local.query::<ICounterAbi>().unwrap();
    {
        let _read = local.try_borrow().unwrap();
        let promoted = weak.upgrade().unwrap().unwrap();
        assert_eq!(promoted.try_borrow().unwrap().value, 1);
        assert!(matches!(promoted.try_borrow_mut(), Err(NC_E_BUSY)));
        assert_eq!(foreign.set_value(2), Err(NC_E_BUSY));
    }
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let promoted = weak.upgrade().unwrap().unwrap();
        let mut write = promoted.try_borrow_mut().unwrap();
        write.value = 7;
        let another = promoted.downgrade().unwrap().upgrade().unwrap().unwrap();
        assert!(matches!(another.try_borrow(), Err(NC_E_BUSY)));
        assert!(matches!(another.try_borrow_mut(), Err(NC_E_BUSY)));
        assert_eq!(foreign.value(), Err(NC_E_BUSY));
        assert_eq!(foreign.set_value(8), Err(NC_E_BUSY));
        panic!("release promoted owner and guard");
    }));
    assert!(failure.is_err());
    assert_eq!(foreign.value(), Ok(7));
    weak.upgrade().unwrap().unwrap().try_borrow_mut().unwrap().value = 9;
    assert_eq!(foreign.value(), Ok(9));
    drop((local, foreign));
    assert_eq!(drops.load(Ordering::Relaxed), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

#[com_interface(auto_guid = "nc.tests.local.Shared:v1", threading = shared, assigned_signature = b"00000000000000000000000000000062")]
/// Shared service used to prove that typed weak observers preserve thread capability.
pub trait IShared {
    /// Reads ordinary state under the generated shared gate.
    fn value(&self) -> Result<u32, NcStatus>;
}

#[com_object(IShared)]
/// Plain state can be borrowed through a promoted local owner on another thread.
struct Shared {
    /// Updated only while the COM gate grants exclusive access.
    value: u32,
    /// Observes final release without participating in object ownership.
    drops: std::sync::Arc<AtomicUsize>,
}
impl IShared for Shared {
    fn value(&self) -> Result<u32, NcStatus> { Ok(self.value) }
}
impl Drop for Shared {
    fn drop(&mut self) { self.drops.fetch_add(1, Ordering::SeqCst); }
}

/// A successful promotion keeps the concrete value alive through the other owner's release.
#[test]
fn shared_weak_local_can_promote_and_become_the_last_owner_on_another_thread() {
    let drops = std::sync::Arc::new(AtomicUsize::new(0));
    let local = Shared::into_local(Shared { value: 1, drops: drops.clone() });
    let weak = local.downgrade().unwrap();
    fn assert_shared<T: Send + Sync>(_: &T) {}
    assert_shared(&weak);
    let (ready, admitted) = std::sync::mpsc::channel();
    let (release, finish) = std::sync::mpsc::channel();
    let worker_weak = weak.clone();
    let worker = std::thread::spawn(move || {
        let promoted = worker_weak.upgrade().unwrap().unwrap();
        promoted.try_borrow_mut().unwrap().value = 42;
        ready.send(()).unwrap();
        finish.recv().unwrap();
        assert_eq!(promoted.query::<ISharedAbi>().unwrap().value(), Ok(42));
    });
    admitted.recv().unwrap();
    drop(local);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    release.send(()).unwrap();
    worker.join().unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

/// Constructor helpers must not reserve any legal implementation generic name.
/// The const spelling deliberately resembles a private type binder.
#[allow(non_upper_case_globals)]
mod generic_constructor_names {
    use super::{ICounter, ICounterAbi, ICounterVtblFor, NcStatus};
    use neocogi_com::{com_object, NC_E_DROPPED};
    use core::marker::PhantomData;

    #[com_object(interfaces(ICounter))]
    /// Shared storage accepts private-looking names and an ordinary interface-like `I`.
    struct __NcInterface2<I, r#__NcInterface0 = (), const __NcInterface1: u32 = 7>
    where I: Copy + From<u32> + Into<u32> {
        /// Concrete application state is independent of the chosen COM interface.
        value: I,
        /// Keeps the authored type parameter in the implementation's type.
        marker: PhantomData<__NcInterface0>,
    }
    impl<I, P, const SCALE: u32> ICounter for __NcInterface2<I, P, SCALE>
    where I: Copy + From<u32> + Into<u32> {
        fn value(&self) -> Result<u32, NcStatus> { Ok(self.value.into() + SCALE) }
        fn set_value(&mut self, value: u32) -> Result<(), NcStatus> { self.value = value.into(); Ok(()) }
    }

    #[com_object(interfaces(ICounter), ownership = exclusive)]
    /// Exclusive storage exercises both ordinary constructors with the same names.
    struct Exclusive<I, r#__NcInterface0 = (), const __NcInterface1: u32 = 7>
    where I: Copy + From<u32> + Into<u32> {
        /// Owned application state, mutated through the existing COM call gate.
        value: I,
        /// Preserves the authored type parameter without introducing allocation.
        marker: PhantomData<__NcInterface0>,
    }
    impl<I, P, const SCALE: u32> ICounter for Exclusive<I, P, SCALE>
    where I: Copy + From<u32> + Into<u32> {
        fn value(&self) -> Result<u32, NcStatus> { Ok(self.value.into() + SCALE) }
        fn set_value(&mut self, value: u32) -> Result<(), NcStatus> { self.value = value.into(); Ok(()) }
    }

    /// Both ordinary shared entrypoints execute real ABI calls with legal generic names.
    #[test]
    fn shared_generic_binders_preserve_calls_and_weak_lifetimes() {
        type Model = __NcInterface2<u32>;
        let ordinary = Model::into_local(Model { value: 35, marker: PhantomData });
        assert_eq!(ordinary.query::<ICounterAbi>().unwrap().value(), Ok(42));
        assert_eq!(ordinary.try_borrow().unwrap().value, 35);
        for local_entry in [false, true] {
            let value = Model { value: 35, marker: PhantomData };
            let mut pointer = if local_entry {
                Model::into_local(value).query::<ICounterAbi>().unwrap()
            } else {
                Model::from_value(value).query::<ICounterAbi>().unwrap()
            };
            let weak = pointer.downgrade().unwrap();
            assert_eq!(pointer.value(), Ok(42));
            pointer.set_value(36).unwrap();
            assert_eq!(weak.upgrade().unwrap().unwrap().value(), Ok(43));
            drop(pointer);
            assert!(weak.upgrade().unwrap().is_none());
        }
    }

    /// Both ordinary exclusive entrypoints retain unique ownership and expire observers.
    #[test]
    fn exclusive_generic_binders_preserve_calls_and_weak_lifetimes() {
        type Model = Exclusive<u32>;
        let ordinary = Model::into_local_box(Model { value: 35, marker: PhantomData });
        assert_eq!(ordinary.query::<ICounterAbi>().unwrap().value(), Ok(42));
        for local_entry in [false, true] {
            let value = Model { value: 35, marker: PhantomData };
            if local_entry {
                let owner = Model::into_local_box(value);
                let mut weak = owner.query::<ICounterAbi>().unwrap();
                weak.set_value(36).unwrap();
                assert_eq!(weak.value(), Ok(43));
                drop(owner);
                assert_eq!(weak.value(), Err(NC_E_DROPPED));
            } else {
                let mut owner = Model::into_box(value).query::<ICounterAbi>().unwrap();
                let weak = owner.downgrade();
                owner.set_value(36).unwrap();
                assert_eq!(weak.value(), Ok(43));
                drop(owner);
                assert_eq!(weak.value(), Err(NC_E_DROPPED));
            }
        }
    }
}
