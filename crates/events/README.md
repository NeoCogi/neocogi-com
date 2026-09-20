# Typed COM event mappings

`map(producer.changed(), sender, mapper)` installs a concrete generated receiver.
The mapper immediately translates an owned event into an application message;
the application consumes its standard channel later at a safe update boundary.

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
neocogi_com_events::bind_output(
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
