# neocogi-com-runtime

`neocogi-com-runtime` contains the process-global runtime implementation linked
into the `libnc_com_shared_runtime` assembly library. The Cargo package is
`neocogi-com-runtime`; its Rust implementation target is `nc_com_shared_runtime`.

It owns the parts that need an operating-system runtime:

- process-global runtime state
- dynamic plugin loading
- canonical IDL contract, plugin, and type registries
- relationship and provenance inspection
- callback-based runtime diagnostics
- plugin descriptor inspection helpers
- shared-library C exports

Most component and host authors should depend on `neocogi-com` instead. The public
Rust facade is `neocogi_com::Runtime`, which calls the shared runtime
through `neocogi-com-runtime-sys`; its contract registry, plugin registry, type
registry, inspection, and debug operations are exposed as separate contextual
views.
The `neocogi-com-runtime` rlib supports workspace tooling such as descriptor
inspection, but its concrete registry type is not the supported host-facing API.

The shared inspection table also provides a non-registering operation for
trusted plugin libraries. It opens and validates the plugin, invokes a caller
callback synchronously with the borrowed `NcPluginDescriptor`, and leaves the
process-global registries untouched. The descriptor graph is valid only for the
callback contract, while the opened module itself remains pinned until process
exit.

Class activation compares the caller's constructor-frame ABI signature with the
class descriptor before invoking plugin code. An empty signature requires a null
frame pointer; a non-empty signature requires an immutable frame pointer. The runtime never
interprets frame fields or queries a constructor-argument object: the generated
class contract owns the frame layout and decodes it in the defining plugin.

The four mutually exclusive `com-validation-*` features select the exact ABI
signature representation accepted by a deployable runtime. A build with none of
those features is reserved for representation-neutral tooling: it validates
signatures according to each inspected descriptor but does not register a mixed
plugin assembly.

The process registry is append-only. A contract module registration follows
native dependency getters before taking the catalog lock, records dependencies
before dependents, and rejects one logical module ID paired with another ABI
fingerprint as `NC_E_CONTRACT_MISMATCH`. Exact registrations are idempotent.
It preserves registration order in vectors
for deterministic ABI enumeration and maintains runtime-private hash indexes
for plugin, class, interface, relationship, and loaded-module lookup. One
catalog `RwLock` permits concurrent reads and publishes static registration or
dynamic activation as a blocking write transaction. A successful activation
makes its retained native handle, module provenance, plugin, classes,
relationships, and identity mappings visible together. No hash-map iteration
order is exposed through core ABI v7.

Plugin loading, descriptor invocation, and descriptor validation happen before
the catalog write lock is acquired. neocogi-com never invokes plugin constructors,
plugin methods, descriptor entrypoints, native loader hooks, or debug callbacks
while holding that lock. Live activation calls must not be nested: code reached
by the native loader or descriptor entrypoint must not directly or indirectly
call neocogi-com loading again. Native libraries mapped automatically by the
operating-system loader are not nested neocogi-com activations and are not registered
as neocogi-com plugins automatically.

The runtime is intentionally `std`-only. This indexing does not change the
`no_std` support of `neocogi-com-core`, `neocogi-com-runtime-sys`, or the reduced-feature
`neocogi-com` facade.

The crate builds one `rlib` for workspace tooling and assembly composition.
`cargo-neocogi` links that implementation into the assembly's sole runtime
`cdylib`; runtime calls should go through that shared library's C ABI.

`plan_plugin_load_order` is the common collection-level validation gate used by
the inspection CLI and `cargo-neocogi` staging. Before dependency sorting it
requires every selected plugin to declare the same `NcAbiValidationMode`; a
mixed plan is rejected with both plugin IDs and both modes.

Module provenance and plugin metadata have separate authoritative sources.
`NcModuleInfoV1` exposes a loaded module's process-local ID, plugin ID, and the
single path retained by the module registry. `NcPluginInfoV1` exposes the
qualified name and version directly from the registered descriptor. C callers
enumerate module IDs through `NcPluginRegistryV1`, call `module_info`, and then
call `plugin_info` with the returned plugin ID when they also need plugin
metadata. The borrowed UTF-8 views remain runtime-owned; the `neocogi-com` Rust
facade performs both lookups and returns an owned `LoadedModuleInfo`. Statically
registered plugins have plugin information but no module information.

Runtime ABI v1 retains at most 1,024 dynamic modules. A 1,025th activation emits
an `NC_E_OUT_OF_RANGE` diagnostic and aborts through the normal fail-fatal load
path. The debug registry permits 64 simultaneous messenger registrations;
additional creation returns `NC_E_OUT_OF_RANGE`. Destroying a messenger frees
one slot, while its monotonic process-local handle is never reused.

The dynamic symbol surface contains only the mandatory runtime ABI version
function and six v1 table getters. Operation functions are immutable table
entries rather than individual exported symbols.
