# NeoCogi COM component design

A contract describes values, callable interfaces and required projections at
compile time. Rust owns concrete object fields and method bodies through
`#[com_object]`. `#[plugin]` publishes a typed acquisition function receiving host runtime access;
its root can be a finished service or an ordinary construction provider. Applications
can define typed receiver interfaces for their own notifications.

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
registry through their acquisition call. Contracts and facade helpers compile into
their consumers. Ordinary Cargo builds the host runtime and provider libraries;
[the native guide](../guides/COM_NATIVE.md) explains static and dynamic linking.
The [standalone sample](../../examples/com-plugin/README.md) also demonstrates the raw
shared-library boundary and monolithic linking.

## Construction and interface storage

`interface` is the only callable IDL declaration. In
`interface Service: IReadable + IWritable`, the first required interface provides
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

Inspection exposes plugin identity, dependencies, threading and the exact root
factory contract. `neocogi-inspect` reads provider metadata without acquiring a
root object. Generated `idl_metadata` describes interfaces and reachable values;
[recursive inspection](../contracts/RECURSIVE_VALUE_CONTRACT.md#com-inspection-graph)
defines the supported value graph.

## Executable documentation

The COM/IDL documentation test generates the marked language examples and
executes the factory bootstrap from the [component guide](../COM.md).
ABI doctests compile the normative examples against the current SDK. The
quality gate also checks documentation links and the supported feature selections.
