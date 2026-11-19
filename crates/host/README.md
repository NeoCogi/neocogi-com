# neocogi-com-host

The runtime owns plugin registration, trusted library loading, checked
root-factory acquisition, plugin provenance, diagnostics, and the
implementation of caller-owned persistence contexts. It contains
no contract-module registry, native contract descriptions, or event seeding.
Applications use `neocogi_com_sdk::RuntimeHandle`.

This development host uses **WIP ABI v3/core generation 21**, not a frozen
release contract. Published
0.3.0 uses ABI v2/core generation 19; deploy matching hosts and providers.

## Host implementation and shared SDK

A host is the final application that selects this implementation. The package
name is `neocogi-com-host` and the Rust library path is `neocogi_com_host`.
Interface callers and providers both use the separate `neocogi-com-sdk` SDK: callers
call interfaces, providers implement them, and either role can occur inside a
host or plugin. `neocogi_com_sdk::RuntimeHandle` is supplied access to this implementation's
service tables; it does not own or initialize another runtime.

This package depends on the SDK with defaults disabled and `std` enabled for
shared ownership, persistence contracts and codecs, plus core for raw service
tables and validation. The SDK has no dependency back to the host; host-dependent
integration tests live in the unpublished `tests/com` package.
Providers link the SDK and receive host access during bootstrap. The final
application links this implementation. Host-side inspection tools can reuse its
loader and metadata utilities without initializing the plugin catalog.

See [the role and dependency guide](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0-alpha.1/docs/COM.md#hosts-callers-and-providers)
for the complete graph and startup/call sequence.

A provider dynamic library exports `nc_get_plugin_descriptor_v3`, lending an
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
Loader errors retain their phase, caller path, platform/validation detail and
optional symbolic/raw status in all profiles, including min-size. Their Display
contract is independent of optional Debug formatting.

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
[native loading contract](../../docs/abi-v3.md#runtime-and-global-registry).

[Runtime ABI v3](../../docs/abi-v3.md#runtime-and-global-registry) has three C tables:
complete plugin operations, diagnostics and persistence. Plugin lookup and
enumeration share one projection containing identity, factory and dependencies. Loading
returns a plugin GUID. Plugin inspection reports static origin or the first
library path; native handles remain private. The same descriptor reuses its
registration, while conflicting descriptors are rejected. There is no module
ID, module registry or module-to-plugin join. The limit is 1,024 distinct dynamic
providers, excluding static registrations and repeats, and 64 active messengers.
Plugin descriptor and core object generations remain independently versioned; see
the [current ABI versions](../../docs/abi-v3.md#abi-identities-and-versions).

## Build and link

Native and browser applications can link the `neocogi_com_host` Rust rlib and use
`#[neocogi_com_sdk::com_main]` to obtain a copyable runtime handle. It supplies the
host's tables to providers and their nested acquisitions. The host requires
`std`. Core and the access handle retain reduced-feature support. See the
[native build guide](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0-alpha.1/docs/guides/COM_NATIVE.md#linked-host-with-a-shared-provider).

For linked startup, `initialize_runtime()` admits the host service graph and
returns a safe `neocogi_com_sdk::RuntimeHandle`. It invokes no plugin, and repeated
calls use the same host. `com_main` calls this typed constructor, including with
an explicit dependency/module path.
`initialize_process()` lends the raw C description for service ABI callers. Its
immutable `NcHostApiV3`, child tables, state and code remain process/instance-lived
even after application main returns. The three existing C tables remain separate
services. Callbacks execute here and reach this host's one private registry;
nested provider acquisition forwards this canonical bundle.

Persistence operations are available through `neocogi_com_sdk::persistence` when the
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

## Persistence provider

Applications explicitly supply `nc.serialization` before snapshot or restore:
register `neocogi_com_serialization::plugin_descriptor()` for static compositions,
or load `neocogi-com-serialization-dylib` for dynamic compositions. The host uses
the registered provider's named `ncb` factory in both directions, regardless of
its default format. It embeds no codec or fallback. Missing providers return
`NC_E_NOT_FOUND`; selection errors propagate. See [PERSISTENCE.md](PERSISTENCE.md)
for ownership, callback ordering, limits and error precedence.
