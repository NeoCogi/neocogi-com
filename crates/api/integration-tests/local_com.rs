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
