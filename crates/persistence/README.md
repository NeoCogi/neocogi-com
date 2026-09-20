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
