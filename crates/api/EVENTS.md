# Constructor-bound typed event queues

A producer's behavioral parent supplies its one queue during construction.
Several event kinds travel in one concrete enum. The application implements the
queue's concrete COM interface, acquires the event and a weak origin, then posts
its ordinary application message. No event-installation interface is implemented
by the published producer.

```text
interface IEventQueue<Object, Event> {
    fn post(&mut self, origin: &Object, event: &Event) -> Result<()>;
}
enum EventBinding<Object, Event> {
    Unobserved,
    Observed(ComPtr<IEventQueue<Object, Event>>),
}
class Counter { /* getters and setters */ }
factory Counters {
    fn create(&self, events: &EventBinding<Counter, CounterEvent>) -> Result<ComPtr<Counter>>;
}
```

One concrete `AppQueue` may implement several closed queue interfaces and serve
many producers. Each receives the corresponding projection of the same object.
A receiver's `post(&mut self, ComRef<CounterAbi>, NcValueRef<CounterEvent>)`
acquires the event with `to_owned()` and the identity with `origin.downgrade()`.
The latter touches only lifetime bookkeeping and needs no borrow of widget state.
The application later handles its owned messages with ordinary `&mut self`.
Its model, message type and channel never cross the ABI.

Implement the generated queue trait on the application's own COM object:

```rust,ignore
#[com_object(CounterEventQueue)]
struct AppQueue { sender: std::sync::mpsc::Sender<AppMessage> }
impl CounterEventQueue for AppQueue {
    fn post(&mut self, origin: ComRef<'_, CounterAbi>, event: NcValueRef<'_, CounterEvent>)
        -> Result<(), NcStatus>
    {
        let event = event.to_owned()?;
        // Apply any additional domain checks required by this event contract here.
        let _ = self.sender.send(AppMessage::Counter(origin.downgrade()?, event));
        Ok(())
    }
}
let queue = AppQueue::from_value(AppQueue { sender }).query::<CounterEventQueueAbi>()?;
let binding = CounterEventBinding::Observed(queue);
let counter = factory.create(neocogi_com::NcValueRef::new(&binding)?)?;
```

The same queue can serve many producers and implement additional queue traits.
There is no generated callback receiver or sender/mapper constructor. IDL generates
concrete queue methods and the producer's static event association, while the
application owns acquisition, channel capacity and dispatch policy.

The factory decodes the binding and constructs its private `CounterEventPort`
using `EventPort::with_queue(queue, origin)`. Generated `from_value_cyclic` /
`into_local_cyclic` constructors supply the weak public origin before publication;
it cannot upgrade until initialization succeeds. `Unobserved` permanently
suppresses delivery and allocates no dummy queue.
`EventPort::post` promotes its weak origin for the synchronous queue call;
`close` releases its queue and origin idempotently. Posting after close returns
`NC_E_CLOSED`. There is no attach, replace, detach, subscription list or second
consumer slot. Implementations own event storage explicitly.

Queue capture must not call back into the emitting widget's active mutable gate.
Application behavior runs after the emitting call returns. Receiver errors
propagate without rolling back committed producer state. The repository's concrete
application queues discard messages successfully after channel disconnection;
other implementations define their own capacity and error policy.

Weak references expire at final strong release, independently of logical closure.
A retained closed widget can still upgrade; domain operations reject it as closed.
Queued facts survive destruction and remain useful without an origin, for example
a completed dialog result. Application membership/generation rules decide whether
to dispatch an old fact; core weak lifetime does not impose a universal discard rule.

The backend derives queue aliases and `EventSource` metadata from constructor
arguments. `IDL_INTERFACE.event_queue` describes the exact receiver IID/signature,
checked public Rust implementation trait path, and complete reachable event graph.
It is not an interface in the producer's inheritance chain. COM catalogs transport
this metadata to independent hosts. Custom factories query the advertised typed
queue before constructing their widget.

The `macros` feature exposes these ordinary generic library contracts. COM object
allocation requires `alloc`; the examples using standard channels also need `std`. Neither introduces
a COM process runtime or event loop. Contracts and consumers must be rebuilt
together for core ABI v11.

See [the architecture and state diagrams](../../../docs/CLASS_EVENT_STREAMS.md),
[IDL syntax](../../../docs/IDL_MODULES.md), and
[UI scheduling](../../ui/runtime/docs/EVENTS.md).
