//! Accepted public names and real exclusive-constructor calls in emitted Rust.
pub mod exclusive;
pub mod generic;
pub mod plain;
pub mod weak_only;
use neocogi_com::*;
use neocogi_com::events::EventQueueInterface;
use std::sync::mpsc;

#[com_class(exclusive::Counter, ownership = exclusive)]
/// Exclusive product keeps its typed queue directly; no shared EventPort alias.
struct Counter {
    /// Constructor-selected receiver, released when the owner is dropped.
    queue: Option<ComPtr<exclusive::CounterEventQueueAbi>>,
    /// Non-owning identity used only during a synchronous post.
    origin: WeakComBox<exclusive::CounterAbi>,
}
impl exclusive::Counter for Counter {
    /// Deliver the owned snapshot through the generated concrete queue ABI.
    fn emit(&mut self, value: NcValueRef<'_, exclusive::Event>) -> Result<(), NcStatus> {
        match &self.queue {
            Some(queue) => exclusive::CounterEventQueueAbi::post(queue.as_com_ref(), self.origin.as_com_ref(), &value.to_owned()?),
            None => Ok(()),
        }
    }
}
#[com_factory(exclusive::Factory)]
/// Ordinary factory exercises construction with the generated binding alias.
struct Factory;
impl exclusive::Factory for Factory {
    /// Initialize delivery before moving the sole owner across COM.
    fn create(&self, events: NcValueRef<'_, exclusive::CounterEventBinding>) -> Result<ComBox<exclusive::CounterAbi>, NcStatus> {
        let queue = events.to_owned()?.into_queue();
        Counter::into_box_cyclic::<exclusive::CounterAbi>(move |origin| Counter { queue, origin })?
            .query()
            .map_err(|(status, _unpublished)| status)
    }
}
#[com_object(exclusive::CounterEventQueue)]
/// Implementing the readable alias also requires its matching VtblFor re-export.
struct Queue {
    /// Acquired application facts outlive the synchronous COM call.
    send: mpsc::Sender<(WeakComBox<exclusive::CounterAbi>, exclusive::Event)>,
}
impl exclusive::CounterEventQueue for Queue {
    /// Enqueue owned data and a weak origin without retaining the widget owner.
    fn post(&mut self, origin: ComRef<'_, exclusive::CounterAbi>, event: NcValueRef<'_, exclusive::Event>) -> Result<(), NcStatus> {
        self.send.send((origin.downgrade_box()?, event.to_owned()?)).map_err(|_| NC_E_CLOSED)
    }
}

/// Helper removal must leave constructors, queue implementations and metadata usable.
#[test]
fn exclusive_construction_posts_and_preserves_metadata() {
    let factory = Factory::from_value(Factory).query::<exclusive::FactoryAbi>().unwrap();
    let (send, received) = mpsc::channel();
    let queue = Queue::from_value(Queue { send }).query::<exclusive::CounterEventQueueAbi>().unwrap();
    let binding = exclusive::CounterEventBinding::from_queue(Some(queue.as_com_ref()));
    drop(queue);
    let owner = factory.create(NcValueRef::new(&binding).unwrap()).unwrap();
    drop(binding);
    let mut weak = owner.downgrade();
    weak.emit(NcValueRef::new(&exclusive::Event::Changed).unwrap()).unwrap();
    let (mut origin, event) = received.recv().unwrap();
    assert!(matches!(event, exclusive::Event::Changed));
    drop(owner);
    assert_eq!(origin.emit(NcValueRef::new(&exclusive::Event::Changed).unwrap()), Err(NC_E_CLOSED));
    assert!(matches!(received.try_recv(), Err(mpsc::TryRecvError::Disconnected)));
    let description = exclusive::CounterAbi::IDL_INTERFACE;
    let receiver = description.event_queue.unwrap();
    assert_eq!(receiver.abi.interface_id, exclusive::CounterEventQueueAbi::INTERFACE_DESCRIPTOR.interface_id);
    assert_eq!(receiver.message.qualified_name, exclusive::Event::IDL_TYPE.qualified_name);
    assert!(description.values.iter().any(|value| value.qualified_name == receiver.message.qualified_name));
}

/// These legal authored declarations would collide with unconditional helpers.
#[test]
fn non_emitted_names_are_available_to_user_declarations() {
    assert_eq!(exclusive::CounterEventPort { value: 17 }.value, 17);
    assert_eq!(weak_only::CounterEventPort { value: 19 }.value, 19);
    assert_eq!(plain::CounterEventBinding { value: 23 }.value, 23);
    assert_eq!(plain::CounterEventQueueAbi { value: 29 }.value, 29);
    assert_eq!(plain::CounterEventQueue { value: 31 }.value, 31);
    assert_eq!(plain::CounterEventQueueVtblFor { value: 37 }.value, 37);
    assert_eq!(plain::CounterEventPort { value: 41 }.value, 41);
    assert!(plain::CounterAbi::IDL_INTERFACE.event_queue.is_none());
    assert_eq!(
        weak_only::CounterAbi::IDL_INTERFACE.event_queue.unwrap().abi.interface_id,
        weak_only::CounterEventQueueAbi::INTERFACE_DESCRIPTOR.interface_id
    );
}

/// Closed generic products expose the same resolved association as their helpers.
#[test]
fn generic_constructor_helpers_match_runtime_metadata() {
    let port = generic::GENERIC_PRODUCTEventPort::new();
    assert!(!port.is_bound());
    let binding = generic::GENERIC_PRODUCTEventBinding::from_queue(None);
    assert!(binding.into_queue().is_none());
    let receiver = generic::GENERIC_PRODUCTAbi::IDL_INTERFACE.event_queue.unwrap();
    assert_eq!(
        receiver.abi.interface_id,
        generic::GENERIC_PRODUCTEventQueueAbi::INTERFACE_DESCRIPTOR.interface_id
    );
    assert!(
        generic::GENERIC_PRODUCTAbi::IDL_INTERFACE
            .values
            .iter()
            .any(|value| value.qualified_name == receiver.message.qualified_name)
    );
}
