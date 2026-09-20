# Experimental COM object persistence

This optional layer saves and reconstructs explicit initialization trees. It
supports shared components, monolithic applications, and WebAssembly through
the same generated COM interfaces. It has no filesystem or dynamic-loader
policy and does not register factories in the process-global COM runtime.

A provider implements `IPersistable` to export a recipe GUID, initialization
schema version, typed binary state, and ordered owned children. Runtime hosts,
GPU handles, receivers, pointers, caches, and parent links are excluded. Parent
state owns edge properties such as grid placement. Repeated noncyclic references
are copied as separate tree instances; graph aliasing is not preserved.

The application registers context-bound `IPersistenceFactory` objects in a
`FactoryRegistry`. `restore` decodes a bounded `ObjectRecord` tree, validates
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

The [plain-Cargo plugin example](../../../examples/com-plugin/README.md) snapshots
objects and binds named outputs across a relocated shared library and in a
monolith. Its adapter reconstructs through the provider's existing typed factory.

The [UI round-trip test](../../ui/runtime/src/runtime/tests/retained/persistence.rs)
implements a context-bound settings-view provider with real button, checkbox,
and linear widgets. Application code receives only an opaque mountable root and
named output capability. It clicks the normal UI, snapshots the edited checkbox,
restores a new component, and rebinds it with the same application messages.
That example is one explicitly supported composition, not a complete authored UI
schema or automatic persistence for every built-in widget. A future UI editor
will need to define those initialization schemas and property inspectors.
