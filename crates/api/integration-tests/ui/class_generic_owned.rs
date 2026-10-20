//! Concrete interfaces over generic owned state, without generic contracts or registered constructors.
use neocogi_com::{com_object, ComPtr, NcStatus};
use std::{cell::Cell, rc::Rc};

extern crate self as fixture_ui_class_generic_owned;
neocogi_com_test_contracts::ui_class_generic_owned!();

/// Rust-local handler trait keeps data and behavior explicit inside one binary.
trait Handler {
    /// Commits a value to the caller's concrete owned state.
    fn receive(&self, value: u32);
    /// Reads that same state without a typeless cast.
    fn value(&self) -> u32;
}
#[com_object(interfaces(ISink))]
/// Generic local implementation supports defaults, const parameters and where clauses.
struct Sink<T = Counter, const SCALE: u32 = 1>
where
    T: Handler,
{
    /// Owned handler data lives in this same COM allocation.
    handler: T,
}
impl<T: Handler, const SCALE: u32> IBase for Sink<T, SCALE> {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.handler.value())
    }
}
impl<T: Handler, const SCALE: u32> ISink for Sink<T, SCALE> {
    fn receive(&self, value: u32) -> Result<(), NcStatus> {
        self.handler.receive(value * SCALE);
        Ok(())
    }
}
/// Concrete application state and its final-release observation.
struct Counter {
    /// State outlives the creation stack through explicit shared ownership.
    value: Rc<Cell<u32>>,
    /// Proves final release destroys this specialization exactly once.
    drops: Rc<Cell<u32>>,
}
impl Handler for Counter {
    fn receive(&self, value: u32) {
        self.value.set(value);
    }
    fn value(&self) -> u32 {
        self.value.get()
    }
}
impl Drop for Counter {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}
#[com_object(interfaces(IBase), thread_safe)]
/// Thread-safe generic implementations must prove the existing Send + Sync contract.
struct Shared<T: Send + Sync + 'static> {
    /// Generic owned data retains its exact concrete destruction behavior.
    _state: T,
}
impl<T: Send + Sync + 'static> IBase for Shared<T> {
    fn value(&self) -> Result<u32, NcStatus> {
        Ok(42)
    }
}
fn main() {
    let value = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let pointer: ComPtr<ISinkAbi> = Sink::<_, 3>::from_value(Sink {
        handler: Counter {
            value: value.clone(),
            drops: drops.clone(),
        },
    })
    .query()
    .unwrap();
    assert_eq!(Sink::<Counter, 3>::COM_INTERFACE_COUNT, 3);
    pointer.receive(7).unwrap();
    assert_eq!(pointer.value().unwrap(), 21);
    let base = pointer.query::<IBaseAbi>().unwrap();
    drop(pointer);
    assert_eq!(base.value().unwrap(), 21);
    assert_eq!(drops.get(), 0);
    drop(base);
    assert_eq!(drops.get(), 1);
    let shared = Shared::from_value(Shared { _state: std::sync::Arc::new(9u32) });
    std::thread::spawn(move || {
        assert_eq!(shared.query::<IBaseAbi>().unwrap().value().unwrap(), 42);
    })
    .join()
    .unwrap();
}
