//! Independent application/plugin roles communicating only through generated ABI calls.
pub mod contract;
use neocogi_com::*;
use std::sync::mpsc;

#[com_class(contract::Counter)]
/// Owns only the final constructor-selected delivery policy.
struct Counter {
    /// Typed receiver retained until explicit or final closure.
    events: events::EventPort,
}
impl contract::Base for Counter {}
impl contract::Counter for Counter {
    /// Acquires the call-scoped event before its storage expires.
    fn emit(&mut self, value: NcValueRef<'_, contract::Changed>) -> Result<(), NcStatus> {
        self.events.post(Changed::into_box(Changed(value.to_owned()?)).query().map_err(|(status, _)| status)?)
    }
    /// Emits a marker event through the same permanent destination.
    fn submit(&mut self) -> Result<(), NcStatus> { self.events.post(Submitted::into_box(Submitted).query().map_err(|(status, _)| status)?) }
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
    /// Validates before publication and transfers the decoded queue into the new object.
    fn create(&self, reject: bool, events: NcValueRef<'_, events::EventBinding>) -> Result<ComPtr<contract::CounterAbi>, NcStatus> {
        let events = events.to_owned()?.into_queue();
        if reject {
            return Err(NC_E_INVALID_ARG);
        }
        Counter::from_value_cyclic::<contract::CounterAbi>(move |origin| Counter {
            events: events::EventPort::with_queue(events, origin),
        })?
        .query()
    }
}
/// Reports receiver destruction without shared mutable fixture storage.
struct Lifetime(mpsc::Sender<()>);
impl Drop for Lifetime {
    /// Queue fields are released in the application that owns their concrete value.
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}
#[com_object(interfaces(neocogi_com::events::IEventReceiver))]
/// Concrete application queue owns posting access and destruction instrumentation.
struct Queue {
    /// Captured facts outlive call-scoped producer storage.
    send: mpsc::Sender<events::EventMessage>,
    /// The fixture observes release on rejected construction, close and final drop.
    _lifetime: Lifetime,
}
impl neocogi_com::events::IEventReceiver for Queue {
    /// Transfer the exclusive event to the inbox; a failed send restores its owner.
    fn post(&mut self, origin: neocogi_com::NcValueRef<'_, neocogi_com::events::EventOrigin>, event: neocogi_com::ComBox<neocogi_com::events::IEventAbi>) -> Result<(), (neocogi_com::NcStatus, neocogi_com::ComBox<neocogi_com::events::IEventAbi>)> {
        let message = neocogi_com::events::EventMessage::capture(origin, event)?;
        self.send.send(message).map_err(|error| (neocogi_com::NC_E_CLOSED, error.0.event))
    }
}
#[test]
fn factory_queue_ownership_and_metadata() {
    let description = contract::CounterAbi::IDL_INTERFACE;
    assert_eq!(description.events.len(), 2);
    assert_eq!(description.events[0].abi.interface_id, contract::IChangedEventAbi::INTERFACE_DESCRIPTOR.interface_id);
    assert_eq!(description.events[0].fields.iter().map(|field| field.name).collect::<Vec<_>>(), ["text", "enabled", "raw_0", "budget"]);
    assert!(!description.chain.iter().any(|abi| abi.interface_id == events::IEventReceiverAbi::INTERFACE_DESCRIPTOR.interface_id));
    assert!(description.values.iter().any(|value| value.qualified_name == contract::Changed::IDL_TYPE.qualified_name));
    let factory = Factory::from_value(Factory).query::<contract::FactoryAbi>().unwrap();
    for explicit in [false, true] {
        let (released, drops) = mpsc::channel();
        let capture = Lifetime(released);
        let (tx, rx) = mpsc::channel();
        let queue = Queue::from_value(Queue { send: tx, _lifetime: capture })
            .query::<events::IEventReceiverAbi>()
            .unwrap();
        let binding = events::EventBinding::Observed(queue);
        let mut counter = factory.create(false, NcValueRef::new(&binding).unwrap()).unwrap();
        drop(binding);
        let event = contract::Changed { text: "call-owned".into(), enabled: true, raw_0: 73, budget: 109 };
        counter.emit(NcValueRef::new(&event).unwrap()).unwrap();
        drop(event);
        let message = rx.recv().unwrap();
        let value = message.downcast::<contract::IChangedEventAbi>().unwrap().unwrap();
        assert_eq!(value.text().unwrap().to_string().unwrap(), "call-owned");
        assert!(value.enabled().unwrap());
        assert_eq!((value.raw_0().unwrap(), value.budget().unwrap()), (73, 109));
        drop(counter.clone());
        assert!(matches!(drops.try_recv(), Err(mpsc::TryRecvError::Empty)));
        if explicit {
            counter.close().unwrap();
            counter.close().unwrap();
            assert!(matches!(drops.try_recv(), Ok(())));
            assert_eq!(counter.submit(), Err(NC_E_CLOSED));
        }
        drop(counter);
        if !explicit {
            assert!(matches!(drops.try_recv(), Ok(())));
        }
        assert!(matches!(drops.try_recv(), Err(mpsc::TryRecvError::Disconnected)));
    }
    let (released, drops) = mpsc::channel();
    let capture = Lifetime(released);
    let (tx, _rx) = mpsc::channel();
    let queue = Queue::from_value(Queue { send: tx, _lifetime: capture })
        .query::<events::IEventReceiverAbi>()
        .unwrap();
    let binding = events::EventBinding::Observed(queue);
    assert_eq!(factory.create(true, NcValueRef::new(&binding).unwrap()).unwrap_err(), NC_E_INVALID_ARG);
    drop(binding);
    assert!(matches!(drops.try_recv(), Ok(())), "failed construction cannot retain the queue");
    assert!(matches!(drops.try_recv(), Err(mpsc::TryRecvError::Disconnected)));
    let mut unobserved = factory
        .create(false, NcValueRef::new(&events::EventBinding::Unobserved).unwrap())
        .unwrap();
    unobserved.submit().unwrap();
    unobserved.close().unwrap();
    unobserved.close().unwrap();
    assert_eq!(unobserved.submit(), Err(NC_E_CLOSED));
}

/// A valid pointer to an invalid binding tag is rejected through the real factory thunk.
#[test]
fn malformed_constructor_binding_never_publishes_a_product() {
    let factory = Factory::from_value(Factory).query::<contract::FactoryAbi>().unwrap();
    let value = events::EventBinding::Unobserved;
    let guard = NcValueRef::new(&value).unwrap();
    // SAFETY: the copied raw record stays initialized and alive for this synchronous call.
    let mut raw = unsafe { guard.as_ptr().read() };
    raw.tag = 99;
    // SAFETY: raw is live aligned storage; from_raw/decoding must validate its tag.
    let borrowed = unsafe { NcValueRef::from_raw(&raw) }.unwrap();
    assert_eq!(factory.create(false, borrowed).unwrap_err(), NC_E_INVALID_TAG);
}

#[com_class(contract::SharedOrigin)]
/// Thread-safe capability whose local producer context owns its queue separately.
struct SharedOrigin;
impl contract::SharedOrigin for SharedOrigin {}

/// A shared producer may use a thread-local inbox; transport deliberately narrows permissions.
#[test]
fn shared_origin_is_weak_in_the_local_inbox() {
    let object = SharedOrigin::from_value(SharedOrigin).query::<contract::SharedOriginAbi>().unwrap();
    let (queue, messages) = events::channel();
    let mut port = events::EventPort::with_queue(Some(queue), object.downgrade().unwrap());
    port.post(Submitted::into_box(Submitted).query().unwrap()).unwrap();
    drop(object);
    let message = messages.recv().unwrap();
    let origin = message.origin.service::<contract::SharedOriginAbi>().unwrap().unwrap();
    assert!(origin.upgrade().unwrap().is_none());
    assert!(message.downcast::<contract::ISubmittedEventAbi>().unwrap().is_some());
}

#[com_object(interfaces(contract::IChangedEvent), ownership = exclusive)]
/// Owned snapshot has no reference to its producer's mutable storage.
struct Changed(contract::Changed);
impl events::IEvent for Changed {}
impl contract::IChangedEvent for Changed {
    fn text(&self) -> Result<ComPtr<INcStringAbi>, NcStatus> { Ok(NcString::from_utf8_str(&self.0.text)) }
    fn enabled(&self) -> Result<bool, NcStatus> { Ok(self.0.enabled) }
    fn raw_0(&self) -> Result<u8, NcStatus> { Ok(self.0.raw_0) }
    fn budget(&self) -> Result<u32, NcStatus> { Ok(self.0.budget) }
}
#[com_object(interfaces(contract::ISubmittedEvent), ownership = exclusive)]
struct Submitted;
impl events::IEvent for Submitted {}
impl contract::ISubmittedEvent for Submitted {}
