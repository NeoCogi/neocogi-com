# neocogi-com-host

The runtime owns plugin registration, trusted library loading, checked
root-factory acquisition, plugin provenance, diagnostics, and the
implementation of caller-owned persistence contexts. It contains
no contract-module registry, native contract descriptions, or event seeding.
Applications use `neocogi_com::Runtime`.

## Host implementation and shared SDK

A host is the final application that selects this implementation. The package
name is `neocogi-com-host` and the Rust library path is `neocogi_com_host`.
Interface callers and providers both use the separate `neocogi-com` SDK: callers
call interfaces, providers implement them, and either role can occur inside a
host or plugin. `neocogi_com::Runtime` is supplied access to this implementation's
service tables; it does not own or initialize another runtime.

This package depends on the SDK with defaults disabled and `std` enabled for
shared ownership, persistence contracts and codecs, plus core for raw service
tables and validation. The SDK has no dependency back to the host; host-dependent
integration tests live in the unpublished `tests/com` package.
Providers link the SDK and receive host access during bootstrap; the final host
links this package once. Host-side inspection tools can reuse its loader and
metadata utilities without initializing the plugin catalog.

See [the role and dependency guide](https://github.com/NeoCogi/neocogi-com/blob/master/docs/COM.md#hosts-callers-and-providers)
for the complete graph and startup/call sequence.

A dynamic library exports `nc_get_plugin_descriptor_v1`, lending an
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
catalog write lock. Every application plugin load is mandatory.
Load failure diagnoses and aborts the process;
there is no recoverable loading facade. The internal loader's `Result` carries
diagnostics and lets it pin an opened library before the C boundary aborts.
It is not an application retry policy. The Rust facade also aborts on path
conversion or raw argument-validation errors.

Opened libraries remain pinned for process lifetime. Inspection separately
validates and borrows metadata without registration or acquisition and can return
errors. It exposes plugin/factory data, not a runtime inventory of contract
declarations. Static registration and later factory calls retain their existing
status-returning contracts; they are separate operations from live loading.

On Windows, all three loading/inspection paths resolve the caller's filesystem
path once and use `LoadLibraryExW` with
`LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32`. Deploy private
ordinary import dependencies beside the selected DLL. This open excludes
application, user-added, CWD and PATH search directories, without changing global
process policy or retrying with broader flags. Windows module reuse/redirection
still applies; later provider-issued loads and delay loads retain their own
policy. Bare names select CWD-relative files rather than requesting PATH lookup.
The caller's original path remains in errors and first-registration provenance.
Native inspection accepts UTF-16 paths; SDK path adapters require UTF-8. Paths
and ABI identities do not authenticate native code: providers and their native
dependencies must be trusted. See the
[native loading contract](https://github.com/NeoCogi/neocogi-com/blob/master/docs/abi-v1.md#runtime-and-global-registry).

[Runtime ABI v1](https://github.com/NeoCogi/neocogi-com/blob/master/docs/abi-v1.md#runtime-and-global-registry) has three C tables:
complete plugin operations, diagnostics and persistence. Plugin lookup and
enumeration share one projection containing identity, factory and dependencies. Loading
returns a plugin GUID. Plugin inspection reports static origin or the first
library path; native handles remain private. The same descriptor reuses its
registration, while conflicting descriptors are rejected. There is no module
ID, module registry or module-to-plugin join. The limit is 1,024 distinct dynamic
providers, excluding static registrations and repeats, and 64 active messengers.
Plugin descriptor ABI v1 and core object ABI v1 remain independently versioned.

## Build and link

The package builds the ordinary Rust `neocogi_com_host` rlib. Native and browser final
hosts depend on it directly and use `#[neocogi_com::com_main]` to initialize its
existing singleton and obtain a copyable access handle. Reusable libraries take
that handle from their caller. Plugin acquisition receives it through the C
bootstrap; providers select no implementation. No runtime DSO or native bridge
is built or deployed. The runtime requires `std`; core and the access handle
retain reduced-feature support. See the [sample host](https://github.com/NeoCogi/neocogi-com/tree/master/examples/com-host).

`initialize_process()` is infallible/idempotent and invokes no plugin. Its
immutable `NcHostApiV1`, child tables, state and code remain process/instance-lived
even after application main returns. The three existing C tables remain separate
services. Callbacks execute here and reach this host's one private registry;
nested provider acquisition forwards this canonical bundle.

Persistence operations are available through `neocogi_com::persistence` when the
facade's `runtime` feature is enabled. `FactoryRegistry::new(&runtime)` returns a
fallible owner of one runtime-allocated context. Registration requires mutable
access; restore only borrows the context. Contexts never populate the global
plugin catalog, and dropping a context releases its retained factories through
the same runtime that allocated it.

The persistence table validates inputs, captures bounded object trees, preflights
all recipes and provider state, and constructs descendants before their parents.
Snapshot bytes are delivered once through a synchronous borrowed-byte callback;
the Rust facade copies them into its own allocator. No Rust vector, map, or
allocator ownership crosses the ABI. Native and browser hosts execute the same
implementation, while providers need only the built-in core contracts and
the facade's implementation helpers. See [persistence](PERSISTENCE.md) for
limits, provider obligations, and reconstruction semantics.

The public Rust API of `neocogi_com_host` also includes `inspect_plugin_library` and
`plan_plugin_load_order`, used by the build and inspection tools. Inspection
returns owned metadata without registering or acquiring a plugin root, but it pins
every successfully opened library for the inspector process lifetime. A
short-lived process bounds that retention. Inspection does not add plugin
catalog records, so its pinned handles are outside the 1,024-record live-load
ceiling. The planner sorts already-inspected
descriptions; it does not find paths or load dependencies.

Native loading, owned inspection and synchronous descriptor callbacks share one
open-and-admit boundary returning the existing validated descriptor wrapper.
Inspection borrows its admitted text, signature and dependency table without
revalidating those fields; only the public owned result copies them. Malformed
metadata returns the shared validation status before projection or callbacks.
Registration separately checks dependency availability, identity conflicts and
capacity at publication. Inspection neither performs those catalog checks nor
registers a plugin or invokes its factory. All metadata/code remains process-pinned.
