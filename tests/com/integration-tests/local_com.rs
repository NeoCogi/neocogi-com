//! Checks shared ownership and provider field access through local and COM views.

#[allow(unused_imports, unused_macros)]
mod test_contracts {
    include!(concat!(env!("OUT_DIR"), "/test_contracts.rs"));
}
use neocogi_com_sdk::{com_object, NcStatus, NC_E_BUSY};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

extern crate self as fixture_local_com;
test_contracts::local_com!();

#[com_object(interfaces(ICounter))]
/// One shared owner may mutate the value through the same field used by COM.
struct Counter {
    value: RefCell<u32>,
    drops: Rc<AtomicUsize>,
}

impl ICounter for Counter {
    fn value(&self) -> Result<u32, NcStatus> {
        self.value.try_borrow().map(|value| *value).map_err(|_| NC_E_BUSY)
    }
    fn set_value(&self, value: u32) -> Result<(), NcStatus> {
        *self.value.try_borrow_mut().map_err(|_| NC_E_BUSY)? = value;
        Ok(())
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn local_and_foreign_calls_share_one_field() {
    let drops = Rc::new(AtomicUsize::new(0));
    let local = Counter::into_local(Counter {
        value: RefCell::new(1),
        drops: drops.clone(),
    });
    let alias = local.clone();
    let foreign = local.query::<ICounterAbi>().unwrap();
    {
        let mut field = local.get().value.borrow_mut();
        *field = 2;
        assert_eq!(foreign.value(), Err(NC_E_BUSY));
        assert_eq!(foreign.set_value(3), Err(NC_E_BUSY));
        assert!(core::ptr::eq(alias.get(), local.get()));
    }
    let field = local.get().value.borrow();
    assert_eq!(foreign.value(), Ok(*field));
    assert_eq!(foreign.set_value(3), Err(NC_E_BUSY));
    drop(field);
    foreign.set_value(4).unwrap();
    assert_eq!(*local.get().value.borrow(), 4);
    drop((local, alias));
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    assert_eq!(foreign.value(), Ok(4));
    drop(foreign);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

/// A provider's field guard is released during ordinary Rust unwinding.
#[test]
fn field_guard_releases_on_unwind_and_outlives_foreign_handles() {
    let drops = Rc::new(AtomicUsize::new(0));
    let local = Counter::into_local(Counter {
        value: RefCell::new(1),
        drops: drops.clone(),
    });
    let foreign = local.query::<ICounterAbi>().unwrap();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut field = local.get().value.borrow_mut();
        *field = 7;
        panic!("release the field guard");
    }));
    assert!(failure.is_err());
    assert_eq!(foreign.value(), Ok(7));
    drop(foreign);
    assert_eq!(*local.get().value.borrow(), 7);
    drop(local);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
}

/// Interface owners keep typed weak promotion valid after all local owners leave.
#[test]
fn weak_local_lifetime_follows_every_strong_projection() {
    let drops = Rc::new(AtomicUsize::new(0));
    let local = Counter::into_local(Counter {
        value: RefCell::new(11),
        drops: drops.clone(),
    });
    let weak = local.downgrade().unwrap();
    let alias = weak.clone();
    let foreign = local.query::<ICounterAbi>().unwrap();
    let weak_interface = foreign.downgrade().unwrap();
    drop(local);
    let promoted = alias.upgrade().unwrap().unwrap();
    assert_eq!(*promoted.get().value.borrow(), 11);
    assert!(promoted.query::<ICounterAbi>().unwrap().as_com_ref().same_object(foreign.as_com_ref()));
    drop(foreign);
    assert_eq!(drops.load(Ordering::Relaxed), 0);
    *promoted.get().value.borrow_mut() = 12;
    assert_eq!(weak_interface.upgrade().unwrap().unwrap().value(), Ok(12));
    drop(promoted);
    assert_eq!(drops.load(Ordering::Relaxed), 1);
    assert!(weak.upgrade().unwrap().is_none());
    assert!(alias.clone().upgrade().unwrap().is_none());
    assert!(weak_interface.upgrade().unwrap().is_none());
    assert!(format!("{weak:?}").starts_with("WeakLocalCom("));
    let replacement = Counter::into_local(Counter {
        value: RefCell::new(99),
        drops: drops.clone(),
    });
    assert!(weak.upgrade().unwrap().is_none(), "a new allocation cannot revive an old observer");
    drop((replacement, alias, weak));
    assert_eq!(drops.load(Ordering::Relaxed), 2);
}

/// Each promotion acquires ownership while calls still share one actual field.
#[test]
fn weak_local_promotion_preserves_field_contention() {
    let drops = Rc::new(AtomicUsize::new(0));
    let local = Counter::into_local(Counter {
        value: RefCell::new(1),
        drops: drops.clone(),
    });
    let weak = local.downgrade().unwrap();
    let promoted = weak.upgrade().unwrap().unwrap();
    let foreign = promoted.query::<ICounterAbi>().unwrap();
    let field = local.get().value.borrow();
    assert_eq!(foreign.value(), Ok(1));
    assert_eq!(foreign.set_value(2), Err(NC_E_BUSY));
    drop(field);
    foreign.set_value(2).unwrap();
    assert_eq!(*local.get().value.borrow(), 2);
    drop((local, foreign, promoted));
    assert_eq!(drops.load(Ordering::Relaxed), 1);
    assert!(weak.upgrade().unwrap().is_none());
}

#[com_object(IShared)]
/// Plain state can be borrowed through a promoted local owner on another thread.
struct Shared {
    /// Published atomically across concurrent owners.
    value: std::sync::atomic::AtomicU32,
    /// Observes final release without participating in object ownership.
    drops: std::sync::Arc<AtomicUsize>,
}
impl IShared for Shared {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.value.load(Ordering::Acquire))
    }
}
impl Drop for Shared {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

/// A successful promotion keeps the concrete value alive through the other owner's release.
#[test]
fn shared_weak_local_can_promote_and_become_the_last_owner_on_another_thread() {
    let drops = std::sync::Arc::new(AtomicUsize::new(0));
    let local = Shared::into_local(Shared {
        value: std::sync::atomic::AtomicU32::new(1),
        drops: drops.clone(),
    });
    let weak = local.downgrade().unwrap();
    fn assert_shared<T: Send + Sync>(_: &T) {}
    assert_shared(&weak);
    let (ready, admitted) = std::sync::mpsc::channel();
    let (release, finish) = std::sync::mpsc::channel();
    let worker_weak = weak.clone();
    let worker = std::thread::spawn(move || {
        let promoted = worker_weak.upgrade().unwrap().unwrap();
        promoted.get().value.store(42, Ordering::Release);
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
    use super::{Cell, ICounter, ICounterAbi, ICounterVtblFor, NcStatus};
    use neocogi_com_sdk::com_object;
    use core::marker::PhantomData;

    #[com_object(interfaces(ICounter))]
    /// Shared storage accepts private-looking names and an ordinary interface-like `I`.
    struct __NcInterface2<I, r#__NcInterface0 = (), const __NcInterface1: u32 = 7>
    where
        I: Copy + From<u32> + Into<u32>,
    {
        /// Concrete application state is independent of the chosen COM interface.
        value: Cell<I>,
        /// Keeps the authored type parameter in the implementation's type.
        marker: PhantomData<__NcInterface0>,
    }
    impl<I, P, const SCALE: u32> ICounter for __NcInterface2<I, P, SCALE>
    where
        I: Copy + From<u32> + Into<u32>,
    {
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(self.value.get().into() + SCALE)
        }
        fn set_value(&self, value: u32) -> Result<(), NcStatus> {
            self.value.set(value.into());
            Ok(())
        }
    }

    #[com_object(interfaces(ICounter), ownership = exclusive)]
    /// Exclusive storage exercises both ordinary constructors with the same names.
    struct Exclusive<I, r#__NcInterface0 = (), const __NcInterface1: u32 = 7>
    where
        I: Copy + From<u32> + Into<u32>,
    {
        /// Owned state can be changed under a shared receiver.
        value: Cell<I>,
        /// Preserves the authored type parameter without introducing allocation.
        marker: PhantomData<__NcInterface0>,
    }
    impl<I, P, const SCALE: u32> ICounter for Exclusive<I, P, SCALE>
    where
        I: Copy + From<u32> + Into<u32>,
    {
        fn value(&self) -> Result<u32, NcStatus> {
            Ok(self.value.get().into() + SCALE)
        }
        fn set_value(&self, value: u32) -> Result<(), NcStatus> {
            self.value.set(value.into());
            Ok(())
        }
    }

    /// Both ordinary shared entrypoints execute real ABI calls with legal generic names.
    #[test]
    fn shared_generic_binders_preserve_calls_and_weak_lifetimes() {
        type Model = __NcInterface2<u32>;
        let ordinary = Model::into_local(Model {
            value: Cell::new(35),
            marker: PhantomData,
        });
        assert_eq!(ordinary.query::<ICounterAbi>().unwrap().value(), Ok(42));
        assert_eq!(ordinary.get().value.get(), 35);
        for local_entry in [false, true] {
            let value = Model {
                value: Cell::new(35),
                marker: PhantomData,
            };
            let pointer = if local_entry {
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

    /// Both boxed entrypoints lend and transfer the same sole owner.
    #[test]
    fn exclusive_generic_binders_preserve_calls_and_owner_projection() {
        type Model = Exclusive<u32>;
        let ordinary = Model::into_local_box(Model {
            value: Cell::new(35),
            marker: PhantomData,
        });
        assert_eq!(ordinary.query_ref::<ICounterAbi>().unwrap().value(), Ok(42));
        for local_entry in [false, true] {
            let value = Model {
                value: Cell::new(35),
                marker: PhantomData,
            };
            let owner = if local_entry {
                Model::into_local_box(value).into_interface::<ICounterAbi>().unwrap()
            } else {
                Model::into_box(value).query::<ICounterAbi>().unwrap()
            };
            owner.set_value(36).unwrap();
            assert_eq!(owner.value(), Ok(43));
        }
    }
}

/// Storage no longer reserves a synthesized companion name or captures prelude names.
mod storage_names {
    use super::*;

    /// This name previously collided with generated private storage.
    #[allow(dead_code)]
    struct GenericComObject;

    #[com_object(interfaces(ICounter))]
    /// Generic state may use an authored name that shadows the Result constructor.
    struct Generic<Result> {
        /// Concrete owned state remains local to this provider.
        value: Cell<u32>,
        /// Owned generic data exercises specialization without changing the interface.
        _state: Result,
    }

    impl<Result> ICounter for Generic<Result> {
        fn value(&self) -> ::core::result::Result<u32, NcStatus> {
            Ok(self.value.get())
        }
        fn set_value(&self, value: u32) -> ::core::result::Result<(), NcStatus> {
            self.value.set(value);
            Ok(())
        }
    }

    /// Real COM calls share one allocation and the same field as local generic access.
    #[test]
    fn generic_names_and_former_storage_companion_are_ordinary_user_names() {
        let local = Generic::into_local(Generic { value: Cell::new(3u32), _state: () });
        let foreign = local.query::<ICounterAbi>().unwrap();
        foreign.set_value(17).unwrap();
        assert_eq!(local.get().value.get(), 17);
        assert_eq!(foreign.value(), Ok(17));
    }
}
