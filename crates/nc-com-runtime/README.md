# nc-com-runtime

`nc-com-runtime` contains the process-global runtime implementation exported by
the `libnc_com_shared_runtime` shared library. The Cargo package is
`nc-com-runtime`; its Rust library target is `nc_com_shared_runtime`.

It owns the parts that need an operating-system runtime:

- process-global runtime state
- dynamic plugin loading
- plugin and type registries
- relationship and provenance inspection
- callback-based runtime diagnostics
- plugin descriptor inspection helpers
- shared-library C exports

Most component and host authors should depend on `nc-com` instead. The public
Rust facade is `nc_com::Runtime`, which calls the shared runtime
through `nc-com-runtime-sys`; its plugin registry, type registry, inspection,
and debug operations are exposed as separate contextual views. The
`nc-com-runtime` rlib remains available for
workspace tooling such as descriptor inspection, but its concrete registry type
is not the supported host-facing API.

The shared inspection table also provides a non-registering operation for
trusted plugin libraries. It opens and validates the plugin, invokes a caller
callback synchronously with the borrowed `NcPluginDescriptor`, and leaves the
process-global registries untouched. The descriptor graph is valid only for the
callback contract, while the opened module itself remains pinned until process
exit.

The crate has no optional features; the runtime C ABI exports are always
compiled.

The process registry is append-only. It preserves registration order in vectors
for deterministic ABI enumeration and maintains runtime-private hash indexes
for plugin, class, interface, relationship, and loaded-module lookup. One
catalog `RwLock` permits concurrent reads and publishes static registration or
dynamic activation as a blocking write transaction. A successful activation
makes its retained native handle, module provenance, plugin, classes,
relationships, and identity mappings visible together. No hash-map iteration
order is exposed through core ABI v3.

Plugin loading, descriptor invocation, and descriptor validation happen before
the catalog write lock is acquired. nc-com never invokes plugin constructors,
plugin methods, descriptor entrypoints, native loader hooks, or debug callbacks
while holding that lock. Live activation calls must not be nested: code reached
by the native loader or descriptor entrypoint must not directly or indirectly
call nc-com loading again. Native libraries mapped automatically by the
operating-system loader are not nested nc-com activations and are not registered
as nc-com plugins automatically.

The runtime is intentionally `std`-only. This indexing does not change the
`no_std` support of `nc-com-core`, `nc-com-runtime-sys`, or the reduced-feature
`nc-com` facade.

The crate builds both an `rlib` for workspace tooling and a `cdylib` for the
shared process-global runtime. Runtime calls should go through the
`cdylib` C ABI.

Module provenance and plugin metadata have separate authoritative sources.
`NcModuleInfoV1` exposes a loaded module's process-local ID, plugin ID, and the
single path retained by the module registry. `NcPluginInfoV1` exposes the
qualified name and version directly from the registered descriptor. C callers
enumerate module IDs through `NcPluginRegistryV1`, call `module_info`, and then
call `plugin_info` with the returned plugin ID when they also need plugin
metadata. The borrowed UTF-8 views remain runtime-owned; the `nc-com` Rust
facade performs both lookups and returns an owned `LoadedModuleInfo`. Statically
registered plugins have plugin information but no module information.

Runtime ABI v1 retains at most 1,024 dynamic modules. A 1,025th activation emits
an `NC_E_OUT_OF_RANGE` diagnostic and aborts through the normal fail-fatal load
path. The debug registry permits 64 simultaneous messenger registrations;
additional creation returns `NC_E_OUT_OF_RANGE`. Destroying a messenger frees
one slot, while its monotonic process-local handle is never reused.

The dynamic symbol surface contains only the mandatory runtime ABI version
function and five v1 table getters. Operation functions are immutable table
entries rather than individual exported symbols.
