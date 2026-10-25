# Experimental COM object persistence

The runtime persistence service saves and reconstructs explicit initialization trees. It
supports shared components, monolithic applications, and WebAssembly through
the same generated COM interfaces. It has no filesystem or dynamic-loader
policy and does not register factories in the process-global COM runtime.

A provider implements `IPersistable` to export a recipe GUID, initialization
schema version, typed binary state, and ordered owned children. Runtime hosts,
GPU handles, receivers, pointers, caches, and parent links are excluded. Parent
state owns edge properties such as grid placement. Repeated noncyclic references
are copied as separate tree instances; graph aliasing is not preserved.

Through `neocogi_com::persistence`, the application registers context-bound `IPersistenceFactory` objects in a
`FactoryRegistry`. Creating a registry is fallible: `FactoryRegistry::new(&runtime)?`. `restore` decodes a bounded `ObjectRecord` tree, validates
all recipe IDs, revisions and provider state, then constructs descendants
before their parents. Provider `validate` must be observational; `restore`
must still validate its input when called directly through COM. A factory can
migrate an explicitly supported old revision before constructing its object.
Unknown recipes or revisions fail rather than loading code or guessing defaults.

The registry releases partially built trees on failure. Providers must release
external resources and undo unpublished topology on construction failure; this
is not an atomic multi-window publication protocol. Keep construction detached
until the complete root and its application bindings have been accepted.

`snapshot` queries the optional capability, checks owned-path cycles and tree
limits, and obtains provider state through a bounded COM byte-buffer copy. The
result is an NCB-v1 document, which the application may embed or save wherever
its platform permits. Saving observes the provider's current persistent fields.
It is not an automatic dump of arbitrary private COM implementation memory.

The wire format and these APIs are experimental. The tests exercise recursive
real COM objects, exact snapshot round trips, whole-tree preflight, bounded
traversal, duplicate recipe rejection, and cleanup after parent failure.

## Executable integration examples

The [plain-Cargo plugin example](../../examples/com-plugin/README.md) snapshots
objects and attaches typed class streams across a relocated shared library and in a
monolith. Its adapter reconstructs through the provider's existing typed factory.

The UI round-trip test
implements a context-bound settings-view provider with real button, checkbox,
and linear widgets. Application code receives a mountable root and looks up
concrete children through its composition interface. It clicks the normal UI, snapshots the edited checkbox,
restores a new component, and rebinds it with the same application messages.
That example defines one supported composition; it does not automatically add
persistence to every built-in widget. The UI document layer
and UI designer define their own authored
UI schema and integration. Those application contracts are separate from this
generic COM object-persistence service.

## Ownership and linkage

Persistence is implemented in `neocogi-com-runtime` and called through
`NcPersistenceApiV1`, supplied in the admitted host API description. Enable the
facade's `runtime` feature for callers. All final hosts link the implementation
rlib and pass access to registry creation and capture. The unreleased ABI-v1
tables change in place; rebuild the host and providers together. Providers only implementing the contracts need neither
feature. The built-in contracts remain available without allocation; portable
`ObjectRecord` authoring requires `alloc` but no linked runtime.

`FactoryRegistry` is an owning, non-Clone, thread-local handle. Its runtime
allocation contains an ordinary `BTreeMap`; `register(&mut self, ...)` is the
only mutating operation, and `restore(&self, ...)` borrows it. Dropping the
handle destroys its context and releases retained factories in the allocating
runtime. Independently owned restored objects remain valid afterward.

Snapshot capture currently supports shared ownership trees. It must retain
canonical identities along the active traversal path; a live exclusive root is
a valid borrow but cannot grant a shared reference. Both snapshot entry points
return `NC_E_INVALID_ARG` before persistence getters or byte delivery run, leaving
the sole owner intact. Exclusive-tree snapshots require a separate observation
protocol and are not provided by this service. The raw factory registration
table also rejects a non-shareable borrowed factory before registry mutation;
the Rust registry facade already requires an owned `ComPtr`.

Snapshot traversal produces the document once. A synchronous byte callback
copies the completed document into caller-owned storage; failure publishes no
partial document. The runtime releases its temporary bytes after the callback.
Restoration accepts a borrowed byte range and returns one owned COM root.
Neither direction exposes Rust container layout or transfers allocator ownership.

## Limits and failure behavior

`snapshot_with_limits` and `FactoryRegistry::restore_with_limits` accept the
same `Limits`. Defaults allow 4,096 objects and at most 48 objects along a
root-to-leaf path. The nested binary policy defaults to a 16 MiB document,
65,536 elements/fields per container, 64 levels of value nesting, and 1,000,000
decoded values. Binary nesting counts the encoded record/sequence structure,
so it can be exhausted before the object-depth budget. Provider-state bytes
and children also count toward binary container limits.

Registration consumes an owned factory pointer. The runtime retains one
reference on success; duplicate IDs return `NC_E_INVALID_ARG` and preserve
the existing provider. Restoration reports `NC_E_NOT_FOUND` for an unregistered
recipe. Providers decide which schema versions they support and return their
own validation errors. No factory constructor runs until the full preflight
succeeds, although provider `validate` calls have already run.

Cycles return `NC_E_INVALID_ARG`; explicit object/state traversal budget checks
return `NC_E_OUT_OF_RANGE`. Codec failures are mapped to `NC_E_INVALID_ARG`,
including codec limits, so callers must not assume every limit failure has one
status. Application bindings and publication remain the caller's responsibility
after a successful restore.

## Rust access

Snapshot and restore callers enable `neocogi-com/runtime`. Construct an owned
context with `FactoryRegistry::new(&runtime)?`; capture with
`snapshot(&runtime, object)` or `snapshot_with_limits(&runtime, object, limits)`.
The supplied handle already admits the required runtime/core versions. Contexts
retain their exact creation table for operations and Drop. Provider interface
identities, application recipe schemas and NCB-v1 bytes remain independent.
