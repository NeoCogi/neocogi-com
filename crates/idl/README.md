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

`interface` and `factory` share the callable model. `#[message]` marks concrete
structs/enums for checked COM posting. A named output such as `changed: TextChanged;`
generates a one-shot installation closure on `ComPtr` and `ComRef`. Producers
implement the ordinary generated receiver setter and store one optional concrete
COM receiver. `events.map(output(), mapper)?` replaces its destination and returns
`Result<()>`; there is no connection projection or subscription lifetime.
Application enums stay in Rust. Generated receivers depend on COM alone, with
no consumer queue dependency or separate support-version protocol.

All contracts, including events, are compile-time definitions. The build helper
emits no native artifact, exported getter, runtime module graph, or built-in
registration hook. Only generated layouts, wrappers, and exact signatures are
compiled into consumers. Source-part names and documentation do not affect ABI
fingerprints; ordered members and transitive dependency revisions do.

See [IDL modules](../../../docs/IDL_MODULES.md) for syntax and package setup.
