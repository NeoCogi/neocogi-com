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

For small applications an optional mapper adapter is available:

```rust,ignore
let queue = neocogi_com::events::mapped_queue::<CounterEventQueueAbi, _>(
    sender, |origin, event| AppMessage::Counter(origin, event),
)?;
let binding = CounterEventBinding::Observed(queue);
let counter = factory.create(neocogi_com::NcValueRef::new(&binding)?)?;
```

The mapper receives `WeakComPtr<CounterAbi>` and an acquired `CounterEvent`.
`EventQueueInterface::receiver` similarly accepts a typed callback. These helpers
allocate a receiver; direct application implementations need neither a mapper
nor a separate receiver object per producer.

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
propagate without rolling back committed producer state. Callback adapters discard
translated messages successfully when their standard channel is disconnected;
a direct queue implementation defines its own capacity and error policy.

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

The `macros` feature exposes these ordinary generic library contracts. Receiver
allocation requires `alloc`; channel mapping also needs `std`. Neither introduces
a COM process runtime or event loop. Contracts and consumers must be rebuilt
together for core ABI v11.

See [the architecture and state diagrams](../../../docs/CLASS_EVENT_STREAMS.md),
[IDL syntax](../../../docs/IDL_MODULES.md), and
[UI scheduling](../../ui/runtime/docs/EVENTS.md).
