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
does not publish application objects atomically. Keep construction detached
until the complete root and its application bindings have been accepted.

`snapshot` queries the optional capability, checks owned-path cycles and tree
limits, and obtains provider state through a bounded COM byte-buffer copy. The
result is an NCB-v1 document, which the application may embed or save wherever
its platform permits. Saving observes the provider's current persistent fields.
It is not an automatic dump of arbitrary private COM implementation memory.

State copying uses the SDK's `INcBytesComPtrExt::to_vec_bounded` contract. The
provider's length is checked before host-local allocation or copying; lengths
above the selected limit or Rust vector capacity return `NC_E_OUT_OF_RANGE`.
A successfully reported zero length yields empty state without calling the
provider's `copy_to` method. Length-query errors still propagate for empty state;
nonempty copy errors discard private storage and publish no snapshot. A copy
callback is therefore not an empty-state validation hook.

The wire format and these APIs are experimental. The tests exercise recursive
real COM objects, exact snapshot round trips, whole-tree preflight, bounded
traversal, duplicate recipe rejection, and cleanup after parent failure.

## Executable integration examples

The [plain-Cargo plugin example](../../examples/com-plugin/README.md) snapshots
objects and attaches typed class streams across a relocated shared library and in a
monolith. Its adapter reconstructs through the provider's existing typed factory.

## Ownership and linkage

The final application must supply `nc.serialization` in this host's catalog
before snapshot or restore. A static/monolithic application explicitly registers
`neocogi_com_serialization::plugin_descriptor()`; a dynamic application loads
`neocogi-com-serialization-dylib`. Host initialization and empty context creation
remain provider-independent. The host has no runtime dependency on a codec
implementation, implicit registration, loader policy, format registry or fallback.

Each operation acquires `ISerializationPluginAbi` through this host's own admitted
runtime tables and requests `format("ncb")`, never `default_format()`. The shipped
provider defaults to NCB; even an external provider with a different default
cannot redirect persistence away from named NCB. The root and selected format
factory are retained through codec use, including restoration preflight and
construction. Snapshot encoding returns independent host-owned bytes, releasing
the codec objects before the raw adapter calls the byte sink. Each document uses
an independent exclusive session. Catalog locks
are released before bootstrap, selection, session and recipe callbacks. Existing
thread-local context and reentrancy rules still apply; this adds no global codec
state or persistence-context mutation through shared access.

Persistence is implemented in `neocogi-com-host` and called through
`NcPersistenceApiV2`, supplied in the admitted host API description. Enable the
facade's `runtime` feature for callers. All final hosts link the implementation
rlib and pass access to registry creation and capture. ABI v2/core generation 19
freezes the current table layouts and calling/ownership contract; the explicit
codec requirement adds no table field, callback, interface signature or NCB-v1
wire change. COM `0.2.0` embedded the codec; applications upgrading must now
supply the provider explicitly. Package versions and application deployment
requirements are separate from the ABI family. Incompatible changes to frozen
contracts require advancing the applicable compatibility authority, not editing
them in place. Providers only implementing the contracts need neither runtime
nor host linkage. The built-in contracts remain available without allocation; portable
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
The format COM ABI represents container and value-depth ceilings as `u32`.
Persistence saturates larger `usize` ceilings to `u32::MAX` at that boundary,
so permissive caller policy still accepts small NCB-v1 documents.

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

An unregistered serialization provider returns `NC_E_NOT_FOUND`; root acquisition
and named-format selection errors propagate unchanged. They never trigger another
format or an embedded implementation. ABI argument admission precedes selection,
including restore's input-byte budget and cleared output. Snapshot completes its
ownership, capability, cycle and capture-budget checks before selection. Restore
selects and decodes before any recipe callback. Encoding/decoding errors after
selection retain their existing `NC_E_INVALID_ARG` mapping and cleanup semantics.
Provider absence or selection failure does not invalidate a context or remove
its registered recipes. Register/load a missing provider, or resolve a transient
selection failure, then retry on the same live context. This does not permit
replacing an already registered descriptor. Context creation and recipe
registration never require codec availability.

## Rust access

Snapshot and restore callers enable `neocogi-com/runtime`. Construct an owned
context with `FactoryRegistry::new(&runtime)?`; capture with
`snapshot(&runtime, object)` or `snapshot_with_limits(&runtime, object, limits)`.
The supplied handle already admits the required runtime/core versions. Contexts
retain their exact creation table for operations and Drop. Provider interface
identities, application recipe schemas and NCB-v1 bytes remain independent.
