# Runtime ABI v1: plugin provenance

This is the current runtime contract. It replaces the module bookkeeping described
in older core drafts. Core object ABI v17, plugin descriptor discovery v1, factory
signatures and COM ownership are unchanged. Runtime ABI remains v1: this
pre-release contract changes in place and promises no cross-revision compatibility.
Applications, DLLs and the sole shared `nc_com`
library must be rebuilt from the same checkout; no old-contract fallback exists.

## Identity and ownership

The catalog has one append-only record per registered plugin GUID. It retains the
immutable descriptor, copied hard dependency IDs and first registration origin:
static storage, or a dynamic library path with a private process-pinned native
owner. The descriptor is authoritative for name, version and factory contract.
The library is provenance, not another public identity. No module IDs, module
registry, module enumeration or module-to-plugin metadata join exist.
The runtime locks the plugin catalog directly; there is no separate catalog
wrapper. Counts and ordered GUID enumeration derive from these records, without
an ID vector that registration must keep in sync.

Native mappings remain pinned until process exit, including inspection-only opens
and failures after opening. Repeating a load still opens the native library and
validates its descriptor; catalog idempotency does not promise to suppress native
loader calls or initialization. There is no unloading, path discovery, recursive
dependency loading or application-injected runtime instance.

## Publication and repeated loading

Factory acquisition is named `acquire`, including the descriptor's `acquire`
bootstrap and the runtime's `acquire_factory_plugin` operation. A successful
request transfers one owned COM reference after complete interface/signature
and threading checks. The provider may construct a fresh factory or return a
retained existing instance; acquisition does not promise a singleton. Loading
registers the plugin and pins its code without acquiring its factory. Typed
factory methods construct the plugin's products after acquisition.

Native open, symbol lookup, descriptor entry and validation run outside the catalog
lock. Under one write guard, registration requires all hard dependencies to exist,
checks provider identity and capacity, then publishes descriptor and provenance
together. No plugin code or callback runs under that guard.

The same GUID and descriptor address reuse the first registration, preserving its
origin and consuming no extra catalog capacity. A different descriptor claiming
that GUID returns `NC_E_INVALID_ARG`; matching names or release text alone do not
make providers interchangeable. Repeated static registration is also idempotent.
Unrelated factory acquisition and inspection never reopen the original path.

`NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS` limits distinct dynamically registered
providers to 1,024. Static registrations, repeats and inspection-only opens are
excluded. Counts derive from catalog records. Inspection-only pinned mappings
remain outside that live-provider bound, so build tooling uses a short-lived
inspection process. Registration failure publishes no partial plugin record.

## C ABI and facade

Check `nc_com_runtime_abi_version() == 1` before obtaining any API table:

- `nc_com_get_runtime_api_v1` returns `NcRuntimeApiV1`. Live loading accepts UTF-8
  path bytes and an output `NcGuid`, cleared to nil before argument validation.
  Success returns the registered plugin GUID. Raw argument errors return statuses;
  native loading, metadata, dependency, identity or capacity failures diagnose and
  abort. The facade also aborts on local path conversion or raw argument failure.
- `nc_com_get_plugin_registry_v1` returns `NcPluginRegistryV1`, enumerating plugin
  GUIDs in successful registration order through the no-partial-write count/fill
  protocol. There are no module enumeration functions.
- `nc_com_get_inspection_api_v1` returns `NcInspectionApiV1`. `plugin_info` writes
  `NcPluginInfoV1`: GUID, qualified name, release version and library path. An empty
  path denotes static origin; a dynamic path is nonempty validated UTF-8 from the
  first registration. Borrowed views remain valid for the runtime's lifetime.
  Factory and dependency inspection retain their existing semantics.
- Diagnostics retain `NcDebugApiV1` and its v1 getter and lifecycle contract;
  `NcDebugMessageV1` drops its unused loaded-module ID. Messages can identify
  related plugins and interfaces. `NcPersistenceApiV1` and its v1 getter remain
  unchanged; persistence contexts stay independent of the global plugin catalog.

`Runtime::load_plugin_library` returns `NcGuid`; batch loading returns `Vec<NcGuid>`.
`Inspection::plugin_info` copies text into owned `PluginInfo`, whose `PluginOrigin`
is `Static` or `Dynamic { path }`. Missing lookups return `None`. Native handles
never cross this facade. Static hosts use the same implementation and C contracts,
linked once; DLL consumers import the sole shared runtime.
