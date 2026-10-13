# neocogi-com-runtime

The runtime owns plugin registration, trusted library loading, checked
root-factory acquisition, plugin provenance, diagnostics, and the
implementation of caller-owned persistence contexts. It contains
no contract-module registry, native contract descriptions, or event seeding.
Applications use `neocogi_com::Runtime`.

A dynamic library exports `nc_get_factory_plugin_description_v1`, lending an
immutable descriptor with plugin metadata, hard dependencies, full root factory
IID/signature, threading, and bootstrap. Registration validates the metadata and
publishes one plugin atomically. Dependencies must already be registered;
registration never invokes bootstrap or discovers provider paths.

Acquisition first compares the requested IID, exact transitive signature, and
threading capability. A signature mismatch returns `NC_E_ABI_VERSION` before
calling plugin code. After bootstrap, the returned projection is checked again.
Parameterized construction happens through typed factory methods. Different
unconnected contract revisions may coexist in the process.

Library loading, getters, factory calls, and debug callbacks run outside the
catalog write lock. Every application plugin load is mandatory, including
loads requested by the designer. Load failure diagnoses and aborts the process;
there is no recoverable loading facade. The internal loader's `Result` carries
diagnostics and lets it pin an opened library before the C boundary aborts.
It is not an application retry policy. The Rust facade also aborts on path
conversion or raw argument-validation errors.

Opened libraries remain pinned for process lifetime. Inspection separately
validates and borrows metadata without registration or acquisition and can return
errors. It exposes plugin/factory data, not a runtime inventory of contract
declarations. Static registration and later factory calls retain their existing
status-returning contracts; they are separate operations from live loading.

[Runtime ABI v1](../docs/runtime-abi-v1.md) has three C tables:
complete plugin operations, diagnostics and persistence. Plugin lookup and
enumeration share one projection containing identity, factory and dependencies. Loading
returns a plugin GUID. Plugin inspection reports static origin or the first
library path; native handles remain private. The same descriptor reuses its
registration, while conflicting descriptors are rejected. There is no module
ID, module registry or module-to-plugin join. The limit is 1,024 distinct dynamic
providers, excluding static registrations and repeats, and 64 active messengers.
Plugin descriptor ABI v1 and core object ABI v17 remain independently versioned.

The crate defaults to an rlib. Build a standalone shared runtime with:

```sh
cargo rustc -p neocogi-com-runtime --lib --crate-type cdylib
```

The Cargo package is `neocogi-com-runtime`, but its Rust library target is
`nc_com`. The command emits `libnc_com.so` on Linux, `libnc_com.dylib` on macOS,
or `nc_com.dll` on Windows. `cargo-neocogi` uses a generated pure-cdylib root
with that same target name. Plain `cargo build -p neocogi-com-runtime` only
builds the implementation rlib; it does not assemble a shared deployment.

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
implementation, while providers need only the built-in core contracts and
the facade's implementation helpers. See [persistence](PERSISTENCE.md) for
limits, provider obligations, and reconstruction semantics.

The public Rust API of `nc_com` also includes `inspect_plugin_library` and
`plan_plugin_load_order`, used by the build and inspection tools. Inspection
returns owned metadata without registering or activating a plugin, but it pins
every successfully opened library for the inspector process lifetime. A
short-lived process bounds that retention. Inspection does not add plugin
catalog records, so its pinned handles are outside the 1,024-record live-load
ceiling. The planner sorts already-inspected
descriptions; it does not find paths or load dependencies.
