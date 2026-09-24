# Typed COM event bindings

This guide describes ordinary, unannotated IDL outputs and named component
exports. `#[observer]` producers instead generate a single concrete observer
for use by constructor APIs; they have no output setters. The UI facade supplies
channel shims for those observers. See the
[constructor observer guide](../../../docs/CONSTRUCTION_OBSERVERS.md).

`bind(producer.changed(), sender, mapper)` installs a concrete generated receiver.
The mapper immediately translates an owned event into the receiving binary's
private message and sends it through a standard Rust channel. The owner handles
that message later, after the producer and UI traversal release their borrows.

`neocogi_com::events` is available with the facade's `macros` feature.
`INamedOutputs` needs neither allocation nor `std`. The channel and named-output
adapters additionally require `std`. These features do not link the COM process
runtime or create an event loop. UI scheduling and persistent controller ownership
belong to `neocogi_ui::events` and the UI core `IEventOwner` contract.

## Binding ownership

`bind` accepts the output's generated installation closure, a
`std::sync::mpsc::Sender<Message>`, and a `FnMut(Event) -> Message` mapper. It
returns `Result<(), NcStatus>`; there is no subscription handle. Setup validates
the concrete output contract once. The producer retains the generated receiver,
which owns the mapper and sender. Mapping executes inside the producer's COM call
and should only translate values or attach routing information.

A replacement binding affects future emissions. Queued messages remain in their
original receiver. Producer closure releases its adapter without retracting
posted facts. If the channel is disconnected, a valid post still runs its mapper
and discards the undeliverable value, reporting success to the producer.
Decoding, validation and installation failures propagate; committed producer
state does not roll back. Captures must be owned or otherwise `'static`.

Application and component behavior uses an owned `Controller<Model>` in the UI
facade. There is no COM-facade dispatcher, borrowed callback bridge, weak model
driver, or revocation state. See the [UI event guide](../../ui/runtime/docs/EVENTS.md)
for construction, nested ownership, scheduling and retirement.

## Experimental named component outputs

For an unannotated producer, a provider owns a `NamedOutputs` table and registers private sources with
`export("volume_changed", slider, "changed")`. Generated `ComOutputInterface`
implementations validate the declared port and know its exact receiver contract.
Register an inherited output through the interface that declares that output.
Public names are component contracts, independent from widget layout paths.
For a document form, supply one child-to-form observer to each observed
widget factory. If that child is public, register a separate form-owned
`exported_observer()` projection with `export_component`. Named binding
replaces that proxy's sole application destination. The form receives the
child event first and chooses both visual actions and optional public delivery.

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

The facade owns `idl/neocogi/com/events/events.ncidl` and generates its named-output
contract into `OUT_DIR`. Its build script publishes the `neocogi.com.events`
module at `neocogi_com::events`. Consumers use `neocogi-com` with `macros`; output
receivers are generated alongside their concrete messages. There is no separate
events contract crate or event shared library. UI scheduling limits and lifecycle
statuses live in UI core contracts, independently of COM binding support.
