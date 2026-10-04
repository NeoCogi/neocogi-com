# NeoCogi component design

A contract describes values and callable interfaces at compile time. A `class`
declares the projections of a concrete product; its private state and method
bodies live in Rust. A `factory` is the callable role eligible for the runtime's
zero-argument bootstrap. A plugin packages implementations and one root factory.
Events are ordinary typed values passed to an application-supplied queue.

## Responsibilities

```text
.ncidl + imported source graphs
  -> build-time resolution and transitive signatures
  -> generated Rust layouts, checked codecs, vtables and fixed signatures
  -> compiled independently into each host/plugin that uses the contract

plugin: private implementations + one factory descriptor
COM runtime: plugin registry + loaded-library lifetime + checked root activation
             + caller-owned persistence contexts
host: supplies paths/dependency order, calls typed factories, owns application state
```

Contracts produce ordinary Cargo libraries. Their Cargo `links` metadata carries
source graphs between build scripts; it does not create a contract shared library
or runtime contract-registration step. IDL parsing and macros run on the build
machine. The runtime compares already-computed identities and signatures.

Five production crates own COM: `core` supplies ABI and value support; `api`
supplies object helpers, ordinary event contracts and runtime clients; `runtime`
owns loading, registries and persistence execution; `macros` expands Rust object
implementations; `idl` owns parsing through Cargo integration. Only the process
runtime is deployed as the COM shared library (`libnc_com.so` on Linux). Contracts
and facade helpers compile into their consumers. The optional `cargo-neocogi`
driver assembles deployments; the [plain-Cargo sample](../../examples/com-plugin/README.md)
demonstrates both shared and monolithic builds without that driver.

## Construction and interface storage

`interface`, `class` and `factory` share the callable ABI machinery, with distinct
roles. In `class Button: IWidget + IActions`, the first required interface provides
the physical vtable prefix; the others provide queryable sibling projections.
Requirements are transitive and acyclic. IDL rejects weakening an inherited shared
threading promise.

A handle carries the object address and selected static interface table.
Interfaces add no fields to each object allocation. The compiler deduplicates
inherited projections, so a diamond does not duplicate object headers. Querying
selects another table for the same allocation after exact identity checks. The
[storage guide](../contracts/COM_INTERFACE_STORAGE.md) explains why the two-word handle replaced
recursive headers and records the measured tradeoffs of earlier attempts.

`#[com_object(Interface)]` and `#[com_class(Class)]` associate private Rust state
with its contract. Shared ownership generates `from_value` and `ComPtr` helpers;
`ownership = exclusive` generates `into_box` and non-clonable `ComBox` helpers.
A provider emits only its selected constructor family. `#[com_factory(Factory)]`
also proves bootstrap eligibility. Unit factories get a default bootstrap;
stateful roots implement that typed policy themselves. Factory methods accept
ordinary typed constructor arguments and return explicit owners.

Mutation uses `&mut self`. ABI dispatch obtains a checked borrow before creating
a Rust reference; conflicting or reentrant access returns `NC_E_BUSY`. Shared
objects keep their method borrow state separate from strong counting. Exclusive
objects combine owner presence and reader/writer admission in one atomic word.
The ABI implementation contains the necessary `UnsafeCell`; semantic state stays
in ordinary fields. See the [current ABI](../../crates/com/docs/README.md) for layouts,
ownership transfer and the limits of concurrent Drop.

## Events and UI ownership

A factory receives the complete `EventBinding` during construction.
All producers share the common `IEventReceiver` interface. Its synchronous `post`
captures the weak origin and moves a `ComBox<IEvent>` into the inbox. Application
behavior runs later under mutable access. There are no named-output connections,
late subscriptions, receiver installation or generated application routing.

For widgets, the owning parent/container holds `ComBox`; callers retain
`WeakComBox` observations. The producer supplies a cached weak origin to its receiver. Removing the owner rejects new calls immediately, waits for
admitted calls and destroys the widget and owned children before Drop returns.
There is no independently closed live widget. Close intent is an ordinary event:
the behavioral parent decides whether to remove its child after the call returns.

The retained UI schedules composite `IWidgetDispatcher::dispatch_next` implementations
descendant-first, using weak observations. A composite owns its own model and
receiver. A top-level `Dispatcher<Processor>` supplies `IApplicationDispatcher` for
application messages and empty-inbox `MessageProcessor::apply_pending_changes` visits.
The generated `ApplicationDispatch` outcome distinguishes message handling,
applied pending changes, and idle visits; both kinds of work revisit child
behavior and layout before later input. COM's process runtime owns no event loop. The
[UI event guide](../../crates/ui/runtime/docs/EVENTS.md) links the executable Button
walkthrough and defines dispatch and stale-message policy.

Shared services still use `ComPtr`, `WeakComPtr` and, where appropriate, `EventPort`.
A shared service may define its own close operation; its closed status is separate
from allocation lifetime. These are different ownership contracts, not alternate
widget paths. [Exclusive ownership](../contracts/EXCLUSIVE_WIDGET_OWNERSHIP.md) and
[lifetime errors](../contracts/COM_LIFETIME_ERRORS.md) define the distinction.

## Independent-build compatibility

The [IDL reference](../contracts/IDL_MODULES.md) owns canonical fingerprint encoding and
declaration version rules. Each exact declaration signature includes the module's
transitive revision. Generic binder spelling is normalized by parameter position;
authored members and policies remain contract inputs. Interface identity alone
does not establish compatibility with a different revision.

Plugin metadata publishes identity/version, hard dependencies, complete factory
IID/signature, threading and a bootstrap pointer. Registration validates and
publishes this metadata atomically. Activation compares the complete factory
identity before invoking bootstrap; a mismatch returns `NC_E_ABI_VERSION`.
Interface queries and pointer transfers also check their contracts.

Unconnected revisions can coexist in one process. Plugin dependencies define
registration order; the host supplies paths. Library loading, getters, bootstrap
and diagnostics run outside the catalog write lock. Required live-load failures
are diagnosed and fatal; typed activation mismatches return statuses. Successfully
loaded libraries stay mapped for process lifetime, including retained weak storage.

Dynamic and static assemblies use the same typed activation. Dynamic builds export
`nc_get_factory_plugin_description_v1`; monoliths register the same provider's Rust
descriptor and link the runtime statically. Independent compiler versions and
build flags are supported when the target C ABI, core generation and exact contract
signatures agree. The [compatibility matrix](../../crates/com/docs/compatibility.md)
records actual cross-build verification. The [current contract index](../../crates/com/docs/README.md)
separates core ABI generation from discovery versions and IDL hash encoding.

Inspection exposes plugin and factory compatibility metadata. A custom widget
catalog additionally transports the finite reachable value graph needed for
authoring; [recursive inspection](../contracts/RECURSIVE_VALUE_CONTRACT.md#com-inspection-graph)
defines that graph. Build-time IDL inventories and runtime widget catalogs have
different consumers and do not recreate a process-wide class registry.

## Why earlier guides disagreed

Earlier revisions used recursive interface headers, registered class constructors,
named event outputs and shared widget owners. Each had a concrete purpose at the
time, but their removal did not consistently update introductory examples. In
particular, `c2749802` introduced a then-valid shared Button origin;
`2cb90ec9` migrated widget queues to exclusive weak origins without migrating its
public walkthrough. A compile-only doctest could not detect the resulting runtime
ownership error.

Current examples therefore execute the exact source published in their guide:
the Button walkthrough covers input delivery, removal and queued stale origins;
the COM/IDL documentation test generates the marked language examples and executes
the factory bootstrap. Package overviews link to one current ABI index, checked
against the core version constant. Historical reports and measurements retain
their evidence with dated follow-up status rather than serving as current API guides.
