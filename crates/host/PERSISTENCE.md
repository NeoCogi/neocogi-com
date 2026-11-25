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

Through `neocogi_com_sdk::persistence`, the application registers context-bound `IPersistenceFactory` objects in a
`FactoryRegistry`. Create it with `FactoryRegistry::new(&runtime)?`.
`restore_object_tree` decodes a bounded `SavedObject` tree, validates
all recipe IDs, revisions and provider state, then constructs descendants
before their parents. Provider `validate` must be observational; `restore`
must still validate its input when called directly through COM. A factory can
migrate an explicitly supported old revision before constructing its object.
Unknown recipes or revisions fail rather than loading code or guessing defaults.

The registry releases partially built trees on failure. Providers must release
external resources and undo unpublished topology on construction failure; this
does not publish application objects atomically. Keep construction detached
until the complete root and its application bindings have been accepted.

`save_object_tree` opens the selected NCB writer, then reads the live tree as
the ordinary serializer writes it. Each object is a sequence of exactly four
values in order: recipe UUID bytes, schema version, initialization bytes, and
ordered child objects. The host keeps the active ownership path to detect cycles
and uses one 4 KiB buffer for state copies. It queries each state length once,
checks its representability, and opens the state sequence before copying.
An empty state causes zero copy calls. On success NCB emits the same v1 bytes.

The application supplies a stable view for the complete synchronous save.
That includes recipe IDs, versions, state lengths and bytes, and ordered children
through reentrant callbacks. An immutable view can be saved while the live model
advances independently.

The wire format and these APIs are experimental. The tests exercise recursive
real COM objects, exact save and restore round trips, whole-tree preflight, bounded
traversal, duplicate recipe rejection, and cleanup after parent failure.

## Executable integration examples

The [plain-Cargo plugin example](../../examples/com-plugin/README.md) saves
objects and attaches typed class streams across a relocated shared library and in a
monolith. Its adapter reconstructs through the provider's existing typed factory.

## Ownership and linkage

The final application must supply `nc.serialization` in this host's catalog
before saving or restoring. A static/monolithic application explicitly registers
`neocogi_com_serialization::plugin_descriptor()`; a dynamic application loads
`neocogi-com-serialization-dylib`. Host initialization and empty context creation
remain provider-independent. The host has no runtime dependency on a codec
implementation, implicit registration, loader policy, format registry or fallback.

Each operation acquires `ISerializationPluginAbi` through this host's own admitted
runtime tables and requests `format("ncb")`, never `default_format()`. The shipped
provider defaults to NCB; even an external provider with a different default
cannot redirect persistence away from named NCB. The root and selected format
factory are retained through codec use, including restoration preflight and
construction. Saving returns independent host-owned bytes, releasing
the codec objects before the raw adapter calls the byte sink. Each document uses
an independent exclusive session. Catalog locks
are released before bootstrap, selection, session and recipe callbacks. Existing
thread-local context and reentrancy rules still apply; this adds no global codec
state or persistence-context mutation through shared access.

Persistence is implemented in `neocogi-com-host` and called through
`NcPersistenceApi`, supplied in the admitted host API description. Enable the
facade's `runtime` feature for callers. The application links the host rlib and passes
its access to registry creation and capture.
WIP ABI v5 identifies the current table layouts and
calling/ownership contract; these remain provisional until release freeze.
The explicit codec requirement adds no table field, callback, interface signature or NCB-v1
wire change. COM `0.2.0` embedded the codec; applications upgrading must now
supply the provider explicitly. Package versions and application deployment
requirements are separate from the ABI family. Incompatible changes to frozen
contracts require advancing the applicable compatibility authority, not editing
them in place. Providers only implementing the contracts need neither runtime
nor host linkage. The built-in contracts remain available without allocation; portable
`SavedObject` authoring requires `alloc` but no linked runtime.

`FactoryRegistry` is an owning, non-Clone, thread-local handle. Its runtime
allocation contains an ordinary `BTreeMap`; `register(&mut self, ...)` is the
only mutating operation, and `restore_object_tree(&self, ...)` borrows it. Dropping the
handle destroys its context and releases retained factories in the allocating
runtime. Independently owned restored objects remain valid afterward.

Saving traverses shared ownership trees and retains canonical identities along
the active traversal path. The Rust SDK takes `&ComPtr<IUnknown>`. The raw
root supplied for saving and the raw registration factory must each be a live shared pair
borrowed for the entire call. The raw entry points validate pair shape and exact
interface identity. The producer's declared shared backing supplies the ownership
proof. A persistence factory borrows `IObjectChildren` during `restore` and can
keep the independently owned references returned by `get`.

The save operation produces one document. A synchronous byte callback
copies the completed document into caller-owned storage; failure publishes no
partial document. The runtime releases its temporary bytes after the callback.
Restoration accepts a borrowed byte range and returns one owned COM root.
Neither direction exposes Rust container layout or transfers allocator ownership.

## Limits and failure behavior

`save_object_tree_with_limits` and `FactoryRegistry::restore_object_tree_with_limits` accept the
same `Limits`. Defaults allow 4,096 objects and at most 48 objects along a
root-to-leaf path. The nested binary policy defaults to a 16 MiB document,
65,536 elements/fields per container, 64 levels of value nesting, and 1,000,000
logical values. Each format session honors the caller's document limits. A
format may add stricter limits. The built-in NCB and JSON codecs enforce
document bytes, total logical values, elements per container, and depth.
They lack separate per-saved-object quotas, object offset tables, and
cross-document links. The host charges each declared child slot, including
pending siblings, before the node's state or child getters. The writer checks
wire counts and depth as it opens the actual sequences. It may reject a child's
required prefix before the host acquires that child.

Restore charges the root and each declared child count during decoding. Pending
siblings count toward the object limit as soon as their sequence's binary count
is admitted. An impossible count or path returns `NC_E_OUT_OF_RANGE` before
reading the children or reserving their record storage. Child storage grows
fallibly as child records complete. A refused host reservation returns
`NC_E_OUT_OF_MEMORY` and discards the source without requesting a diagnostic.
The selected format owns its input copy and binary limits for the document and
provider-state bytes.
The format COM ABI represents container and value-depth ceilings as `u32`.
Persistence saturates larger `usize` ceilings to `u32::MAX` at that boundary,
so permissive caller policy still accepts small NCB-v1 documents.

The Rust registration method takes a `ComPtr` by value and lends its pair to
the runtime. The runtime retains its own reference on success. Duplicate IDs
return `NC_E_INVALID_ARG` and preserve the existing provider. Restoration reports
`NC_E_NOT_FOUND` for an unregistered recipe. Providers decide which schema
versions they support and return their own validation errors. No factory
constructor runs until the full preflight succeeds, although provider `validate`
calls have already run.

Cycles return `NC_E_INVALID_ARG`. Saving checks the root and its object
budget, retains it, and opens named NCB before reading object data. The first
failed operation determines the save status, including codec range and allocation
errors. A pre-delivery failure leaves the sink untouched. A sink that declines a
complete document returns its own status after that one call.

An unregistered serialization provider returns `NC_E_NOT_FOUND`. Root acquisition
and named-format selection errors propagate. Restore checks the input-byte budget
and clears its output before selection. It selects and fully decodes before any
recipe callback. Codec errors during restore map to `NC_E_INVALID_ARG`. An earlier
codec failure takes precedence over an object limit. An admitted count exceeding
the object budget takes precedence over unread child bytes and recipe failures.

Provider absence or selection failure does not invalidate a context or remove
its registered recipes. Register/load a missing provider, or resolve a transient
selection failure, then retry on the same live context. This does not permit
replacing an already registered descriptor. Context creation and recipe
registration never require codec availability.

## Large assets and document relationships

A format session has a single document budget. Applications coordinating several
documents can cap their combined bytes, objects, values, and link work across
the operation. A large texture can use an NCB manifest and separate page payloads
in an asset file. A 16384 × 16384 RGBA8 texture is 1 GiB before mipmaps. Dividing
it into smaller NCB sequences still charges all the pixel bytes and values to
the same document.

An asset loader that defines offsets checks the range with checked arithmetic
and verifies that each offset identifies an admitted page or object boundary
in its owning document. For cross-document links, resolve against an explicit
set of admitted document versions and check target identity, type, and
relationship rules before construction. Bound page payloads and decoded sizes
independently of the manifest. The COM persistence document embeds its children
as nested values and supplies a recipe GUID for construction.

## Rust access

Callers enable `neocogi-com-sdk/runtime` to save and restore trees. Construct an owned
context with `FactoryRegistry::new(&runtime)?`; capture a shared `ComPtr<IUnknown>` with
`save_object_tree(&runtime, &object)` or `save_object_tree_with_limits(&runtime, &object, limits)`.
The supplied handle already admits the native ABI version. Contexts
retain their exact creation table for operations and Drop. Provider interface
identities, application recipe schemas and NCB-v1 bytes remain independent.
