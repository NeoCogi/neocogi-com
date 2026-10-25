# NeoCogi component design

A contract describes values, callable interfaces and required projections at
compile time. Rust owns concrete object fields and method bodies through
`#[com_object]`. `#[plugin]` publishes a typed acquisition function receiving host runtime access;
its root can be a finished service or an ordinary construction provider. Events
are ordinary typed values passed to an application-supplied receiver.

## Responsibilities

```text
.ncidl + imported source graphs
  -> build-time resolution and transitive signatures
  -> generated Rust layouts, checked codecs, vtables and fixed signatures
  -> compiled independently into each host/plugin that uses the contract

plugin: private implementations + one typed root descriptor
COM runtime: plugin registry + loaded-library lifetime + checked root acquisition
             + caller-owned persistence contexts
host: supplies paths/dependency order, calls typed factories, owns application state
```

Contracts live in shared `.ncidl` sources. Consumers generate bindings in their
own crates through `cargo::SharedSources`; no contract package or `links` owner is
required. Existing Rust helper APIs may expose their local bindings for direct
Rust calls. Neither generation nor importing those definitions creates runtime
state. IDL parsing and macros run on the build machine; runtime calls compare the
already-computed identities and signatures.

COM responsibilities have concrete owners: `core` supplies ABI and value support;
`idl` owns parsing through Cargo integration, including its shared `codegen` module;
`macros` supplies thin Rust implementation/persistence entrypoints; `api` supplies
object helpers, static IDL metadata and runtime clients; `runtime` owns loading, the plugin catalog
and persistence execution. The final executable links its runtime implementation
and initializes it through `#[com_main]`. Plugins receive access to that one global
registry through their acquisition call. No COM runtime DSO is deployed. Contracts and facade helpers compile into their consumers. Cargo
artifact dependencies build native compositions directly; [the native guide](../guides/COM_NATIVE.md)
explains the graph and final link ownership.
The
[standalone sample](../../examples/com-plugin/README.md) also demonstrates the raw
shared-library boundary and monolithic linking.

## Construction and interface storage

`interface` is the only callable IDL declaration. In
`interface Button: IWidget + IActions`, the first required interface provides
the physical vtable prefix; the others provide queryable sibling projections.
Requirements are transitive and acyclic. IDL rejects weakening an inherited shared
threading promise.

A handle carries the object address and selected static interface table.
Interfaces add no fields to each object allocation. The compiler deduplicates
inherited projections, so a diamond does not duplicate object headers. Querying
selects another table for the same allocation after exact identity checks. The
[storage guide](../contracts/COM_INTERFACE_STORAGE.md) explains why the two-word handle replaced
recursive headers and records the measured tradeoffs of earlier attempts.

`#[com_object(Interface)]` associates private Rust state with any closed contract. Shared ownership generates `from_value` and `ComPtr` helpers;
`ownership = exclusive` generates `into_box` and non-clonable `ComBox` helpers.
A provider emits only its selected constructor family and explicitly initializes
its Rust state. `#[plugin]` wraps one typed Rust acquisition function in the fixed
root ABI; it needs no bootstrap role or implementation trait. Subsequent object
construction uses ordinary typed methods and explicit returned owners.

Mutation uses `&mut self`. ABI dispatch obtains a checked borrow before creating
a Rust reference; conflicting or reentrant access returns `NC_E_BUSY`. Shared
objects keep their method borrow state separate from strong counting. Exclusive
objects combine owner presence and reader/writer admission in one atomic word.
The ABI implementation contains the necessary `UnsafeCell`; semantic state stays
in ordinary fields. See the [current ABI](../abi-v1.md) for layouts,
ownership transfer and the limits of concurrent Drop.

## Events and UI ownership

COM provides ordinary callable interfaces and explicit ownership. Domains can
use typed observers without depending on UI, as the [standalone example](../../examples/com-plugin/README.md)
demonstrates. UI owns its event protocol and deferred delivery,
producer publication and widget origins.
Those contracts do not add event syntax or policy to IDL or the COM process runtime.

Ordinary widgets post through their owning traversal scope; services and explicit
out-of-traversal completions retain the weak origins justified by the UI contract.
Exclusive ownership and
[lifetime errors](../contracts/COM_LIFETIME_ERRORS.md) define synchronous owner
removal and its distinction from a service's optional close operation. Queues and
UI origin scopes remain under their own owners, independent of the single global
plugin registry in the host-owned `nc_com` implementation.

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
are diagnosed and fatal; typed acquisition mismatches return statuses. Successfully
loaded libraries stay mapped for process lifetime, including retained weak storage.

Dynamic and static assemblies use the same typed acquisition. Dynamic builds export
`nc_get_plugin_descriptor_v1`; monoliths register the same provider's Rust
descriptor and link the runtime statically. Independent compiler versions and
build flags are supported when the target C ABI, core generation and exact contract
signatures agree. The [compatibility matrix](../abi-v1.md#independent-build-compatibility)
records actual cross-build verification. The [current contract index](../abi-v1.md)
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
