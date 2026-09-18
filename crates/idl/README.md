# COM-owned NeoCogi IDL compiler

The compiler owns contract parsing, canonical transitive ABI signatures, and
Rust binding generation. It runs during ordinary Cargo builds and is independent
of the optional `cargo-neocogi` assembly driver.

- `core` (`neocogi-idl`) parses files/module directories, resolves copied values
  and interfaces, validates roles/threading/required projections, and fingerprints
  the complete source graph.
- `build` (`neocogi-idl-build`) discovers sources, exchanges schema-4 source graphs
  through Cargo `links`/`DEP_*`, and emits formatted Rust bindings with fixed
  declaration signatures.
- `tools/idl/example-contracts` and `example-consumer-contracts` cover standalone
  and imported contracts. [examples/com-plugin](../../../examples/com-plugin)
  demonstrates their dynamic and monolithic use.

`interface`, `factory`, and `port` share one callable model. The port role
requires mutable, unit-returning delivery and generates owned snapshots and typed sinks.
A source binds `changed: IChangedSink;` alongside ordinary methods.
It generates `.changed()` accessors for `ComPtr` and `ComRef` and the standard
`subscribe_changed(&self, sink)` connection operation. Every binding has a name;
each retains its own subscription even when protocols are shared.
Ports require the canonical `neocogi.com.events.IEventSubscription` lease;
a foreign same-named lease is rejected. Generated adapters check their event
support API version at compile time independently of wire signatures.

All contracts, including events, are compile-time definitions. The build helper
emits no native artifact, exported getter, runtime module graph, or built-in
registration hook. Only generated layouts, wrappers, and exact signatures are
compiled into consumers. Source-part names and documentation do not affect ABI
fingerprints; ordered members and transitive dependency revisions do.

See [IDL modules](../../../docs/IDL_MODULES.md) for syntax and package setup.
