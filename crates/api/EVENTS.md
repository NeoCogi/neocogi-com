# Typed COM event streams

An emitting concrete class requires the ordinary generic interface
`IEventStream<Event>`. Its one `attach` method accepts `IMessageSink<Event>`.
The complete event type is a concrete IDL `#[message]` struct or enum; an enum
represents several event kinds without creating several ports.

```text
interface IMessageSink<T> {
    fn post(&mut self, message: &T) -> Result<()>;
}
interface IEventStream<E> {
    fn attach(&mut self, consumer: &IMessageSink<E>) -> Result<()>;
}
class Counter: IEventStream<CounterEvent> { /* commands */ }
```

Factories create unbound objects. The behavioral parent attaches a typed receiver
before publishing the object. `EventPort<Sink>` implements the irreversible
`Unbound -> Bound -> Closed` lifetime with ordinary mutable state. A second
attachment returns `NC_E_ALREADY_BOUND`; closure drops the receiver and later
attachment/posting returns `NC_E_CLOSED`. Unbound posting is a successful no-op.
There is no receiver replacement or subscription handle.

`#[com_class(Counter, event = events)]` delegates attachment to the implementation's
explicit `CounterEventPort` field. The producer commits state, then calls
`self.events.post(&event)`; teardown calls `self.events.close()`. A class with
different transitive event stream types is rejected by generation. A stream
inherited through another capability shares the concrete class's one slot.

```rust,ignore
neocogi_com::events::bind_source(counter.as_com_ref(), sender, |event| {
    AppMessage::Counter(event)
})?;
```

`bind_source` queries the concrete class's resolved stream and creates one COM
sink containing the typed mapper and sender. The sink validates and acquires
borrowed payload data before running the mapper, then sends the owned application
message through `std::sync::mpsc`. The model handles that message later with
`&mut self`. The widget retains no shared mutable parent pointer. Mappers run
inside the producer call and must not reenter that producer.

`bind_stream` accepts a stream directly. `attach_source` attaches an existing
sink and checks its concrete COM projection. `MessageSinkInterface::receiver`
is available for custom typed consumption without an mpsc queue. UI helpers add
domain checks and familiar convenience constructors. The older generic `bind`
helper accepts an installation closure; window/menu convenience capabilities
use it to attach the same single-consumer protocol, not replace a destination.

Queued facts retain their original values after closure. A disconnected channel
still runs the mapper and discards the translated value successfully. Decode or
receiver errors propagate without rolling back committed producer state. Avoid
capturing a producer or its owning UI in its own mapper: that can form a reference
cycle. The application owns queue capacity, scheduling, and its model lifetime.

The `macros` feature exposes these generated contracts and adapters. Closed
specializations and receiver allocation require `alloc`; channel binding needs
`std`. They do not link the COM process runtime or introduce an event loop.
The facade publishes `neocogi.com.events` as build-time IDL templates. Consuming
modules generate concrete vtables, deterministic IIDs/signatures, ergonomic
class aliases, and resolved authoring metadata. Application-private types and
Rust closures remain in the consuming binary.

The legacy event-port syntax, observer annotations, and named-output relays have
been removed. Generated forms bind actual widget streams directly. Independent
plugins publish class and message metadata through `IDesignWidgetCatalog` for
authoring; compiled applications still name those types through contract crates.

See [the complete architecture and state diagrams](../../../docs/CLASS_EVENT_STREAMS.md),
[IDL syntax](../../../docs/IDL_MODULES.md), and
[UI scheduling](../../ui/runtime/docs/EVENTS.md).
