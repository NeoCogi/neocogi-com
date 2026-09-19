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
generates a `PortRef<TextChanged>` accessor on `ComPtr` and `ComRef`. Producers bind
`Port<TextChanged>` fields using `com_object(..., ports(...))`; the compiler supplies
the connection getter and one typed receiver operation. No authored callback
protocol, invented message name, or handwritten subscription is needed.
Application enums stay in Rust. Generated support checks API version 5 separately
from native wire signatures, and keeps the canonical connection lease internal.

All contracts, including events, are compile-time definitions. The build helper
emits no native artifact, exported getter, runtime module graph, or built-in
registration hook. Only generated layouts, wrappers, and exact signatures are
compiled into consumers. Source-part names and documentation do not affect ABI
fingerprints; ordered members and transitive dependency revisions do.

See [IDL modules](../../../docs/IDL_MODULES.md) for syntax and package setup.
