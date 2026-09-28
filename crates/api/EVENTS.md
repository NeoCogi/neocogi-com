# Constructor-bound typed COM receivers

A producer's behavioral parent supplies its one receiver during construction.
Several event kinds travel in one concrete enum. No event-installation interface
is implemented by the published producer.

```text
interface IMessageSink<E> {
    fn post(&mut self, message: &E) -> Result<()>;
}
enum EventBinding<E> {
    Unobserved,
    Observed(ComPtr<IMessageSink<E>>),
}
class Counter { /* commands */ }
factory Counters {
    fn create(&self, events: &EventBinding<CounterEvent>) -> Result<ComPtr<Counter>>;
}
```

The application constructs a receiver before calling the factory:

```rust,ignore
let sink = neocogi_com::events::mapped_sink::<CounterSinkAbi, _>(
    sender, AppMessage::Counter,
)?;
let binding = CounterEventBinding::Observed(sink);
let counter = factory.create(neocogi_com::NcValueRef::new(&binding)?)?;
```

The factory decodes the binding and initializes its private `CounterEventPort`
with `EventPort::with_sink(events.to_owned()?.into_sink())` before publication.
`Unobserved` permanently suppresses delivery and allocates no dummy receiver.
`EventPort::post` calls the retained typed sink; `close` releases it idempotently.
Posting after close returns `NC_E_CLOSED`. There is no attach, replace, detach,
subscription list, or second consumer slot. `com_class(Contract)` supplies only
normal class allocation; implementations own event storage explicitly.

`MessageSinkInterface::receiver` supports typed callbacks without an mpsc queue.
`mapped_sink` stores the mapping closure and sender inside that one COM object.
The decoder validates and acquires the event before calling the mapper, which
posts an owned application message. The application later handles it with
`&mut self`. Its model, message type, sender and closure never cross the ABI.
Mappers execute synchronously and must not reenter the producer's active COM gate.

Queued facts survive closure. A disconnected queue discards the translated value
successfully. Receiver errors propagate without rolling back committed producer
state. Capturing the producer or its owning UI in its own mapper can form a
reference cycle; ordinary mappings capture senders and immutable IDs.

The backend derives the product's sink aliases and `EventSource` binding metadata
from constructor arguments in its declaring module. `IDL_INTERFACE.event_sink`
is a receiver descriptor, not an interface in the producer's inheritance chain.
Its complete reachable event graph can be copied through a COM catalog. Custom
plugin catalogs accept a COM receiver capability and query their exact sink type
before constructing a widget; they do not interpret an untyped payload.

The `macros` feature exposes these ordinary generic library contracts. Receiver
allocation requires `alloc`; channel mapping also needs `std`. Neither introduces
a COM process runtime or event loop. Contracts and consumers must be rebuilt
together after this ABI change.

See [the architecture and state diagrams](../../../docs/CLASS_EVENT_STREAMS.md),
[IDL syntax](../../../docs/IDL_MODULES.md), and
[UI scheduling](../../ui/runtime/docs/EVENTS.md).
