# neocogi-com-runtime

The process runtime owns plugin registration, trusted library loading, checked
root-factory activation, loaded-module provenance, and diagnostics. It contains
no contract-module registry, native contract descriptions, or event seeding.
Applications use `neocogi_com::Runtime`.

A dynamic library exports `nc_get_factory_plugin_description_v1`, lending an
immutable descriptor with plugin metadata, hard dependencies, full root factory
IID/signature, threading, and bootstrap. Registration validates the metadata and
publishes one plugin atomically. Dependencies must already be registered;
registration never invokes bootstrap or discovers provider paths.

Activation first compares the requested IID, exact transitive signature, and
threading capability. A signature mismatch returns `NC_E_ABI_VERSION` before
calling plugin code. After bootstrap, the returned projection is checked again.
Parameterized construction happens through typed factory methods. Different
unconnected contract revisions may coexist in the process.

Library loading, getters, factory calls, and debug callbacks run outside the
catalog write lock. Required live-load failure is diagnosed and fatal; opened
libraries remain pinned for process lifetime. Inspection validates and borrows
metadata without registration or activation. It exposes plugin/factory data,
not a runtime inventory of contract declarations.

Runtime ABI v1 has five size-prefixed C tables: operations, plugin enumeration,
inspection, diagnostics, and caller-scoped persistence. Plugin descriptor ABI v1 and core object ABI v7
are independently versioned. Callers must check runtime discovery before
reading a versioned table. The runtime retains at most 1,024 loaded modules and
64 active debug messengers.

The crate defaults to an rlib. Build a standalone shared runtime with:

```sh
cargo rustc -p neocogi-com-runtime --lib --crate-type cdylib
```

A monolithic host enables the facade's `static-runtime` feature and links this
same implementation once. No generated wrapper project is required. See the
[plain-Cargo example](../../../examples/com-plugin/README.md) for both modes.
The runtime is `std`-only; core ABI and reduced-feature contracts retain
`no_std` support.

Persistence operations are available through `neocogi_com::persistence` when the
facade's `runtime` feature is enabled. `FactoryRegistry::new()` returns a
fallible owner of one runtime-allocated context. Registration requires mutable
access; restore only borrows the context. Contexts never populate the global
plugin catalog, and dropping a context releases its retained factories through
the same runtime that allocated it.

The persistence table validates inputs, captures bounded object trees, preflights
all recipes and provider state, and constructs descendants before their parents.
Snapshot bytes are delivered once through a synchronous borrowed-byte callback;
the Rust facade copies them into its own allocator. No Rust vector, map, or
allocator ownership crosses the ABI. Shared and static hosts execute the same
implementation, while providers need only the built-in core contracts.
