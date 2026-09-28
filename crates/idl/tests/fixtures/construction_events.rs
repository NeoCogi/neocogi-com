//! Independent application/plugin roles communicating only through generated ABI calls.
pub mod contract;
use neocogi_com::*;
use std::{cell::Cell, rc::Rc, sync::mpsc};

#[com_class(contract::Counter)]
/// Owns only the final constructor-selected delivery policy.
struct Counter {
    /// Typed receiver retained until explicit or final closure.
    events: contract::CounterEventPort,
}
impl contract::Base for Counter {}
impl contract::Counter for Counter {
    /// Acquires the call-scoped event before its storage expires.
    fn emit(&mut self, value: NcValueRef<'_, contract::Event>) -> Result<(), NcStatus> {
        self.events.post(&value.to_owned()?)
    }
    /// Repeated release cannot reconnect this producer.
    fn close(&mut self) -> Result<(), NcStatus> {
        self.events.close();
        Ok(())
    }
}
#[com_factory(contract::Factory)]
/// Stateless plugin factory; application messages never enter its contract.
struct Factory;
impl contract::Factory for Factory {
    /// Validates before publication and transfers the decoded sink into the new object.
    fn create(&self, reject: bool, events: NcValueRef<'_, contract::CounterEventBinding>) -> Result<ComPtr<contract::CounterAbi>, NcStatus> {
        let events = events.to_owned()?.into_sink();
        if reject {
            return Err(NC_E_INVALID_ARG);
        }
        Counter::from_value(Counter {
            events: events::EventPort::with_sink(events),
        })
        .query()
    }
}
/// Counts capture release across failed construction, explicit close and final release.
struct Lifetime(Rc<Cell<usize>>);
impl Drop for Lifetime {
    /// Runs in the application that owns the mapper's concrete captured value.
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
/// Private application message; never appears in the plugin IDL.
enum Message {
    /// Wraps the whole typed producer event without changing its payload.
    Counter(contract::Event),
}

#[test]
fn factory_mapping_ownership_and_metadata() {
    let description = contract::CounterAbi::IDL_INTERFACE;
    let receiver = description.event_sink.expect("constructor metadata");
    assert_eq!(receiver.abi.interface_id, contract::CounterSinkAbi::INTERFACE_DESCRIPTOR.interface_id);
    assert!(
        !description.chain.iter().any(|abi| abi.interface_id == receiver.abi.interface_id),
        "receiver is not a producer projection"
    );
    assert_eq!(receiver.message.qualified_name, contract::Event::IDL_TYPE.qualified_name);
    assert!(
        description
            .values
            .iter()
            .any(|value| value.qualified_name == contract::Changed::IDL_TYPE.qualified_name)
    );
    let factory = Factory::from_value(Factory).query::<contract::FactoryAbi>().unwrap();
    for explicit in [false, true] {
        let drops = Rc::new(Cell::new(0));
        let capture = Lifetime(drops.clone());
        let (tx, rx) = mpsc::channel();
        let sink = events::mapped_sink::<contract::CounterSinkAbi, _>(tx, move |event| {
            let _ = &capture;
            Message::Counter(event)
        })
        .unwrap();
        let binding = contract::CounterEventBinding::Observed(sink);
        let mut counter = factory.create(false, NcValueRef::new(&binding).unwrap()).unwrap();
        drop(binding);
        let event = contract::Event::Changed(contract::Changed { text: "call-owned".into(), enabled: true });
        counter.emit(NcValueRef::new(&event).unwrap()).unwrap();
        drop(event);
        let Message::Counter(contract::Event::Changed(value)) = rx.recv().unwrap() else {
            panic!("wrong event")
        };
        assert_eq!(value.text, "call-owned");
        assert!(value.enabled);
        drop(counter.clone());
        assert_eq!(drops.get(), 0);
        if explicit {
            counter.close().unwrap();
            counter.close().unwrap();
            assert_eq!(drops.get(), 1);
            assert_eq!(counter.emit(NcValueRef::new(&contract::Event::Submitted).unwrap()), Err(NC_E_CLOSED));
        }
        drop(counter);
        assert_eq!(drops.get(), 1);
    }
    let drops = Rc::new(Cell::new(0));
    let capture = Lifetime(drops.clone());
    let (tx, _rx) = mpsc::channel();
    let sink = events::mapped_sink::<contract::CounterSinkAbi, _>(tx, move |_| {
        let _ = &capture;
        ()
    })
    .unwrap();
    let binding = contract::CounterEventBinding::Observed(sink);
    assert_eq!(factory.create(true, NcValueRef::new(&binding).unwrap()).unwrap_err(), NC_E_INVALID_ARG);
    drop(binding);
    assert_eq!(drops.get(), 1, "failed construction cannot retain mapper captures");
    let mut unobserved = factory
        .create(false, NcValueRef::new(&contract::CounterEventBinding::Unobserved).unwrap())
        .unwrap();
    unobserved.emit(NcValueRef::new(&contract::Event::Submitted).unwrap()).unwrap();
    unobserved.close().unwrap();
    unobserved.close().unwrap();
    assert_eq!(unobserved.emit(NcValueRef::new(&contract::Event::Submitted).unwrap()), Err(NC_E_CLOSED));
}

/// A valid pointer to an invalid binding tag is rejected through the real factory thunk.
#[test]
fn malformed_constructor_binding_never_publishes_a_product() {
    let factory = Factory::from_value(Factory).query::<contract::FactoryAbi>().unwrap();
    let value = contract::CounterEventBinding::Unobserved;
    let guard = NcValueRef::new(&value).unwrap();
    // SAFETY: the copied raw record stays initialized and alive for this synchronous call.
    let mut raw = unsafe { guard.as_ptr().read() };
    raw.tag = 99;
    // SAFETY: raw is live aligned storage; from_raw/decoding must validate its tag.
    let borrowed = unsafe { NcValueRef::from_raw(&raw) }.unwrap();
    assert_eq!(factory.create(false, borrowed).unwrap_err(), NC_E_INVALID_TAG);
}
