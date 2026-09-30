//! Independent application/plugin roles communicating only through generated ABI calls.
pub mod contract;
use neocogi_com::*;
use std::sync::mpsc;

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
    /// Validates before publication and transfers the decoded queue into the new object.
    fn create(&self, reject: bool, events: NcValueRef<'_, contract::CounterEventBinding>) -> Result<ComPtr<contract::CounterAbi>, NcStatus> {
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
#[com_object(contract::CounterEventQueue)]
/// Concrete application queue owns posting access and destruction instrumentation.
struct Queue {
    /// Captured facts outlive call-scoped producer storage.
    send: mpsc::Sender<Message>,
    /// The fixture observes release on rejected construction, close and final drop.
    _lifetime: Lifetime,
}
impl contract::CounterEventQueue for Queue {
    /// Own the value before returning, without retaining the producer.
    fn post(&mut self, _: ComRef<'_, contract::CounterAbi>, event: NcValueRef<'_, contract::Event>) -> Result<(), NcStatus> {
        let _ = self.send.send(Message::Counter(event.to_owned()?));
        Ok(())
    }
}
/// Private application message; never appears in the plugin IDL.
enum Message {
    /// Wraps the whole typed producer event without changing its payload.
    Counter(contract::Event),
}

#[test]
fn factory_queue_ownership_and_metadata() {
    let description = contract::CounterAbi::IDL_INTERFACE;
    let receiver = description.event_queue.expect("constructor metadata");
    assert_eq!(receiver.abi.interface_id, contract::CounterEventQueueAbi::INTERFACE_DESCRIPTOR.interface_id);
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
        let (released, drops) = mpsc::channel();
        let capture = Lifetime(released);
        let (tx, rx) = mpsc::channel();
        let queue = Queue::from_value(Queue { send: tx, _lifetime: capture })
            .query::<contract::CounterEventQueueAbi>()
            .unwrap();
        let binding = contract::CounterEventBinding::Observed(queue);
        let mut counter = factory.create(false, NcValueRef::new(&binding).unwrap()).unwrap();
        drop(binding);
        let event = contract::Event::Changed(contract::Changed { text: "call-owned".into(), enabled: true, raw_0: 73, budget: 109 });
        counter.emit(NcValueRef::new(&event).unwrap()).unwrap();
        drop(event);
        let Message::Counter(contract::Event::Changed(value)) = rx.recv().unwrap() else {
            panic!("wrong event")
        };
        assert_eq!(value.text, "call-owned");
        assert!(value.enabled);
        assert_eq!((value.raw_0, value.budget), (73, 109));
        drop(counter.clone());
        assert!(matches!(drops.try_recv(), Err(mpsc::TryRecvError::Empty)));
        if explicit {
            counter.close().unwrap();
            counter.close().unwrap();
            assert!(matches!(drops.try_recv(), Ok(())));
            assert_eq!(counter.emit(NcValueRef::new(&contract::Event::Submitted).unwrap()), Err(NC_E_CLOSED));
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
        .query::<contract::CounterEventQueueAbi>()
        .unwrap();
    let binding = contract::CounterEventBinding::Observed(queue);
    assert_eq!(factory.create(true, NcValueRef::new(&binding).unwrap()).unwrap_err(), NC_E_INVALID_ARG);
    drop(binding);
    assert!(matches!(drops.try_recv(), Ok(())), "failed construction cannot retain the queue");
    assert!(matches!(drops.try_recv(), Err(mpsc::TryRecvError::Disconnected)));
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

#[com_class(contract::SharedOrigin)]
/// Thread-safe capability whose local producer context owns its queue separately.
struct SharedOrigin;
impl contract::SharedOrigin for SharedOrigin {}

/// A local queue may borrow a shared capability without erasing its threading mode.
#[test]
fn shared_origin_keeps_its_threading_capability_through_the_queue() {
    use neocogi_com::events::EventQueueInterface;
    let object = SharedOrigin::from_value(SharedOrigin).query::<contract::SharedOriginAbi>().unwrap();
    let (send, events) = mpsc::channel();
    let queue = SharedQueue::from_value(SharedQueue { send })
        .query::<contract::SharedOriginEventQueueAbi>()
        .unwrap();
    contract::SharedOriginEventQueueAbi::post(queue.as_com_ref(), object.as_com_ref(), &contract::Event::Submitted).unwrap();
    drop(object);
    let (origin, event) = events.recv().unwrap();
    assert!(origin.upgrade().unwrap().is_none());
    assert!(matches!(event, contract::Event::Submitted));
}

#[com_object(contract::SharedOriginEventQueue)]
/// A local application queue retains the declared thread-safe weak origin type.
struct SharedQueue {
    /// The output explicitly preserves the origin's Send + Sync contract.
    send: mpsc::Sender<(WeakComPtr<contract::SharedOriginAbi, ThreadSafe>, contract::Event)>,
}
impl contract::SharedOriginEventQueue for SharedQueue {
    /// Borrowing a shared origin must not silently weaken it to the local mode.
    fn post(&mut self, origin: ComRef<'_, contract::SharedOriginAbi, ThreadSafe>, event: NcValueRef<'_, contract::Event>) -> Result<(), NcStatus> {
        let origin = origin.downgrade()?;
        /// Compile-time assertion remains independent of runtime delivery.
        fn shared<T: Send + Sync>(_: &T) {}
        shared(&origin);
        let _ = self.send.send((origin, event.to_owned()?));
        Ok(())
    }
}
