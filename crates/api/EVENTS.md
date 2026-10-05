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
if let Some(event) = message.downcast::<ICounterChangedEventAbi>()? {
    println!("{}", event.value()?);
}
```

`downcast` returns a weak event projection, leaving the owner in the message.
An unsupported interface returns `None`; an ABI mismatch remains an error.
Dropping the event owner expires its projections. The source's weak identity does
not own the producer, so event facts remain available after widget removal.

The standard channel needs `std` and returns `NC_E_CLOSED` with the event owner
when disconnected. Independent hosts may implement the single receiver interface
with their own storage and capacity policy. There are no per-widget queue traits,
application callback adapters, or required per-form generated receivers.

Snapshot constructors return a `ComBox<IEventAbi>` directly. Applications query
typed immutable getters; there is no native output conversion layer. `EventPort`
provides explicit Unobserved/Observed/Closed delivery state for shared services.
Widgets store their common receiver and cached `WeakComBox` origin directly.

See the [wire and ownership contract](../../../docs/contracts/COM_EVENT_INTERFACES.md),
[construction and lifetime rules](../../../docs/contracts/CLASS_EVENT_STREAMS.md),
[executed Button walkthrough](../../ui/api/docs/button_events.rs), and
[UI scheduling](../../ui/runtime/docs/EVENTS.md).
