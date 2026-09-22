# Typed COM event mappings

`bind(producer.changed(), sender, mapper)` installs a concrete generated receiver.
The mapper immediately translates an owned event into an application message;
the application consumes its standard channel later at a safe update boundary.

`neocogi_com::events` is available with the facade's `macros` feature. Its
`IEventDispatcher` and `INamedOutputs` contracts work without `std` or allocation.
The channel, named-output, and dispatcher adapters additionally require `std`.
Neither feature links `neocogi-com-runtime` or owns a process event loop. The retained
UI runtime owns mounted-driver discovery, visit order, and the update budget.

## Mapping and dispatch

- `bind` takes the output's generated installation closure, a
  `std::sync::mpsc::Sender<Message>`, and a `FnMut(Event) -> Message` mapper.
  Installing returns `Result<(), NcStatus>`; there is no subscription handle.
  Mapping executes synchronously during the producer's COM call. If the
  channel is disconnected, mapping still runs and the undeliverable value is
  dropped; delivery reports success to the producer.
- `component_dispatcher` creates a separate `IEventDispatcher` over a weak
  reference to `Rc<RefCell<Owner>>`. It borrows the owner mutably before dequeuing
  and processes at most one message. `EVENT_E_BUSY` leaves queued work intact;
  an empty/disconnected queue or dropped owner returns `Ok(false)`. A handler
  error propagates after that message has been removed. This shared owner is
  needed because controller and mounted driver are distinct COM identities.
- The hidden framework helper `with_dispatcher` lends a stack callback through
  COM for one synchronous scope. It revokes the callback on return or unwind;
  retained projections subsequently return `EVENT_E_CLOSED`. Generated mutable
  COM call gating rejects recursive access with `NC_E_BUSY` before reborrowing
  the callback. The callback must not panic through the C dispatch boundary.

`EVENT_E_LIMIT` is a re-export from the event contracts, used by the UI runtime
when its update budget is exhausted. The COM process runtime does not schedule
these dispatchers. See the [UI event guide](../../ui/runtime/docs/EVENTS.md) for
application update and mounted-component integration.

## Experimental named component outputs

A provider owns a `NamedOutputs` table and registers private sources with
`export("volume_changed", slider, "changed")`. Generated `ComOutputInterface`
implementations validate the declared port and know its exact receiver contract.
Register an inherited output through the interface that declares that output.
Public names are component contracts, independent from widget layout paths.

The component implements `INamedOutputs::bind_output` by delegating to the
ordinary mutable table's `bind_com`, or publishes `NamedOutputs::into_com()`.
Only this capability needs to reach application code. The application calls:

```rust,ignore
neocogi_com::events::bind_output(
    component.as_com_ref(), "volume_changed", sender,
    |event: NumericChanged| AppEvent::Volume(event.value),
)?;
```

The event type is known to the application; the producing object is not.
`ComEvent` is generated for every message declaration and wraps the typed callback
in that message's concrete receiver. `IUnknown` is only the binding-time
capability carrier: generated code queries the exact receiver IID and signature
before invoking the producer's original typed setter. No Any, payload byte bus,
reflection-driven method invocation, or generic COM vtable is introduced.

Missing names and incompatible receivers fail without replacing a valid binding.
An output has one replaceable destination, matching existing mapping behavior.
Registration retains producers for the lifetime of the export table. Delivery
retains the installed receiver, not the table, so dropping the table cannot
retract already queued messages or disconnect an otherwise live producer.
Do not make application mapper captures retain their own producing component.

Lookup, interface queries, and the callback/COM-receiver allocations happen at
binding time. Events post directly to the installed receiver without traversing
the name table or an additional forwarding object. Scalars need no payload
allocation; strings/arrays use the existing owned-copy decoding, and standard
channel queue blocks may allocate. This is not a zero-allocation queue API.

This API supplies typed binding, not runtime discovery of arbitrary Rust types.
New payload contracts still require a consumer compiled against that contract.

## Contract ownership and build metadata

The facade owns `idl/neocogi/com/events/events.ncidl` and generates its contracts
into `OUT_DIR`. Its build script publishes the unchanged `neocogi.com.events`
module with the explicit Rust location `neocogi_com::events`. Contract consumers
import the same IDL module and depend directly on `neocogi-com` with `macros`;
there is no separate events contract crate or event shared library.

`EVENT_E_LIMIT` is -1007, `EVENT_E_CLOSED` is -1000, and `EVENT_E_BUSY` is -1001.
They describe update limits and adapter lifetime/reentry conditions, respectively.
The remaining `Rc<Cell<State>>` revokes a retained scoped dispatcher; the
component's `Rc<RefCell<Owner>>` coordinates its independent COM identities.
Channel queues require neither wrapper, and ordinary handlers use `&mut self`.
