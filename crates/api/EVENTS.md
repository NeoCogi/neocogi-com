# COM events

A producer selects one common `IEventReceiver` at construction. Posting moves a
`ComBox<IEvent>` together with a weak widget or service identity. Concrete event
interfaces derive from `IEvent` and expose immutable captured data through COM
getters. A rejected transfer returns the original exclusive owner.

```rust,ignore
let (queue, inbox) = neocogi_com::events::channel();
let binding = neocogi_com::events::EventBinding::from_queue(Some(queue.as_com_ref()))?;
let counter = factory.create(NcValueRef::new(&binding)?)?;
// Later, after the producer's call returns:
let message = inbox.try_recv()?;
if let Some(event) = message.query::<ICounterChangedEventAbi>()? {
    println!("{}", event.value()?);
}
```

`query` lends a projection borrowed from the message's event owner. It adds no
weak token. An unsupported interface returns `None`; an ABI mismatch remains an
error. The source's independently retained weak identity does not own the producer,
so event facts remain available after widget removal.

The standard channel needs `std` and returns `NC_E_CLOSED` with the event owner
when disconnected. Independent hosts may implement the single receiver interface
with their own storage and capacity policy. There are no per-widget queue traits,
application callback adapters, or required per-form generated receivers.

Snapshot constructors return a `ComBox<IEventAbi>` directly. Applications query
typed immutable getters with the existing exclusive-owner query:

```rust,ignore
let event = snapshots::counter_changed(42)?;
let changed = event.query_ref::<ICounterChangedEventAbi>()?;
assert_eq!(changed.value()?, 42);
```

`query_ref` returns `NC_E_NOINTERFACE` for a missing projection; the envelope's
`message.query` maps only that status to `None` for filtering. An explicit
`event.query_weak` retains an observation that may outlive the envelope; its
getters report `NC_E_DROPPED` after owner destruction. There is no separate free
adapter or native output conversion layer. `EventPort` provides explicit
Unobserved/Observed/Closed delivery state for shared services. Ordinary widgets
store their receiver and use the runtime's current owner-scoped origin.

With `alloc`, `events::error(status, message)` constructs an ordinary exclusive
`IErrorEvent` with immutable status/message getters. Nonnegative statuses are
invalid; unknown negative domain codes are preserved. Reports use the same
receiver and weak-origin envelope, so their data outlives source destruction.
Reporting is explicit: queue failures return a status and never recursively
create reports. See [reported failures](../../../docs/contracts/COM_EVENT_INTERFACES.md#reported-failures).

See the [wire and ownership contract](../../../docs/contracts/COM_EVENT_INTERFACES.md),
[construction and lifetime rules](../../../docs/contracts/CLASS_EVENT_STREAMS.md),
[executed Button walkthrough](../../ui/api/docs/button_events.rs), and
[UI scheduling](../../ui/runtime/docs/EVENTS.md).
