# Runtime ABI v1: plugins, factories and inspection

This is the current runtime contract. It supersedes runtime/module bookkeeping
and factory activation names in the core drafts. Core object ABI v17 and plugin
and runtime discovery v1 remain unchanged. The descriptor bootstrap is named
`acquire`. Pre-release v1 contracts change in place: applications, DLLs and the
sole shared `nc_com` must be rebuilt from the same checkout. There are no old
getters, facade wrappers or compatibility aliases.

## Why keep a shared registry?

The process needs one authority for plugin GUIDs, dependency validation,
registration order, first registration provenance and pinned native code. The
registry lives in the shared `nc_com`, not in consumer facades or individual
DLLs. Passing a runtime implementation into each component would permit
independent registries and break those process-wide assumptions.

`Runtime` is a stateless Rust handle to that implementation. Static/browser hosts
link the same implementation once; shared hosts and DLL consumers import the
sole shared runtime. The internal catalog remains useful, but does not require
public registry or inspection context objects. Plugin operations use one table;
diagnostics and persistence retain separate tables because their callback and
caller-owned context lifecycles are independent of plugin registration.

## One authoritative record

The catalog has one append-only record per registered plugin GUID and one
GUID-to-record index. A record owns copied hard dependency GUIDs and its first
registration origin: static storage, or an owned dynamic library path and a
private pinned native handle. It retains the validated descriptor, which supplies
name, release version and the exact single factory contract.

Lookup and enumeration use the same allocation-free projection of these records.
There are no parallel ID vectors, cached metadata summaries or module-to-plugin
joins. Enumeration returns complete records, removing the need for repeated
lookups to assemble plugin, factory and dependency information. This completes
the S4 registry-enumeration simplification recorded in the optimization register.

| `NcPluginInfoV1` field | Meaning and owner |
| --- | --- |
| `plugin_id` | Stable GUID derived from the qualified plugin name; acquisition and lookup key. |
| `qualified_name` | Borrowed immutable descriptor text; copied to `PluginInfo.qualified_name`. |
| `version` | Plugin release text, distinct from interface ABI signatures. |
| `library_path` | First successful dynamic caller path; empty denotes static origin. Provenance, without reopening a file. |
| `factory.interface` | Exact root factory IID and full ABI signature retained in static or pinned storage. |
| `factory.threading` | Valid raw `NcAbiThreadingMode`; the Rust facade returns the typed enum. |
| `dependencies` / `dependency_count` | Immutable aligned GUID array in declared dependency order, copied from validated registration data. |

`NcFactoryInfoV1` contains only interface and threading information: the enclosing
plugin already supplies identity. There is one factory per plugin, so a separate
factory GUID or enumeration is unnecessary. Neither record contains a live COM
reference. Reading metadata never invokes the factory bootstrap.

All views returned by registered-plugin lookup and enumeration remain readable
for the process lifetime. Descriptor storage is static or pinned; dependency and
path buffers are immutable heap allocations. Catalog vector growth moves their
owners, not their backing allocations. The projection never points into a
movable record, stack temporary or temporary metadata copy. The caller may read
views after the catalog read guard is released.

The facade copies text, paths and GUID arrays into owned `PluginInfo` values.
Changing those values cannot change registration state. `FactoryInfo.interface`
retains the existing process-lived signature view rather than copying a contract
or acquiring a native handle. The raw ABI trusts valid aligned/readable pointer
storage; scalar shape checks are not arbitrary-pointer validation.

## Unified C table

Check `nc_com_runtime_abi_version() == 1` before obtaining a versioned table. Check
its size prefix before reading its full layout, then match `core_abi_version`.
The pre-release v1 layouts are revision-specific even when sizes happen to match.

Only the version export and three getters remain:

- `nc_com_get_runtime_api_v1`: `NcRuntimeApiV1`, all plugin operations below.
- `nc_com_get_debug_api_v1`: `NcDebugApiV1`, debug messenger lifecycle.
- `nc_com_get_persistence_api_v1`: `NcPersistenceApiV1`, caller-owned persistence contexts.

| Runtime operation | Result and purpose |
| --- | --- |
| `register_factory_plugin_description` | Validate and atomically register a static descriptor. No factory acquisition. |
| `load_plugin_library_utf8` | Open a mandatory trusted plugin library, pin its code and register it; output its GUID. |
| `acquire_factory_plugin` | Check exact interface/signature/threading, call bootstrap and transfer one owned factory reference. |
| `plugin_info` | Write one complete borrowed record, or return `NC_E_NOT_FOUND`. |
| `plugin_count` | Snapshot number of all registered plugins, including static ones. |
| `plugin_infos` | Fill complete records in successful registration order under one read guard. |
| `inspect_plugin_library_utf8` | Open and validate an unregistered library, lending its descriptor to a synchronous callback. |

The former `NcPluginRegistryV1` and `NcInspectionApiV1` tables and getters are
removed. Separate factory-info and dependency count/fill operations are removed.
An unknown GUID is distinguishable from a registered plugin with no dependencies:
lookup returns `NC_E_NOT_FOUND` for the former and a complete zero-dependency
record for the latter. Lookup checks a null output first (`NC_E_POINTER`) and
writes nothing on failure.

## Enumeration protocol

The raw count is a snapshot, not a reservation. Registration can grow the catalog
between count and fill. Each fill acquires one read guard, fixes the ordered set
of records, projects every field and releases the guard before returning.

1. `out_len` must point to writable, aligned storage for one `usize`. Null returns
   `NC_E_POINTER` without writing any record.
2. Write the current required length to `out_len`.
3. If capacity is insufficient, return `NC_E_INSUFFICIENT_BUFFER`. No record is
   written, including when `out` is null. The caller may allocate and retry.
4. If the required length is zero, return `NC_OK`; null `out` is accepted.
5. Otherwise `out` must provide aligned writable storage for the advertised
   capacity. Null returns `NC_E_POINTER`. Data and length outputs must not overlap.
6. Write exactly the required prefix and return `NC_OK`. Spare slots are untouched.

A successful fill is one coherent metadata snapshot; later registration does not
change it. It does not snapshot live factory objects or their reference counts.
The Rust `plugins()` wrapper retains the existing bounded count/fill reader:
65,536 elements, at most eight strictly growing retries after the initial fill,
and fallible capacity reservation. Dependency copies use the same element bound;
individual text copies allow at most 1 MiB. Invalid raw scalar/count shapes return
statuses before allocation or dereferencing those arrays. These are facade
resource limits, separate from the dynamic-provider limit.

## Rust application API

Registered metadata is available directly on `Runtime`; no intermediate
`Inspection` or `PluginRegistry` context is needed:

```rust,ignore
let runtime = neocogi_com::Runtime::global();
for plugin in runtime.plugins()? {
    println!("{} {}: {:?}", plugin.qualified_name, plugin.version, plugin.origin);
    println!("factory {:?}, providers {:?}", plugin.factory.interface.iid,
             plugin.dependencies);
}

let info = runtime.plugin_info(plugin_id)?; // Option<PluginInfo>, complete record
let factory = runtime.acquire_plugin_factory::<my_contracts::UiFactoryAbi>(plugin_id)?;
let button = factory.create_button()?;
```

`plugin_info` returns `None` only for an unknown GUID. `plugins` returns owned
complete records in registration order; clients no longer enumerate IDs and then
perform three extra metadata queries per plugin. Applications can use the factory
contract to explain what the plugin supplies without creating a factory object.
The generated factory example in the COM/IDL documentation tests exercises actual
typed acquisition; catalog tests exercise these direct methods against C tables.

A generic host that does not know the factory's Rust type calls
`acquire_registered_plugin_root(plugin_id)`. The facade reads the registered exact
interface without allocating owned strings or dependency lists, acquires that
factory with a thread-local request and queries canonical `IUnknown`. An erased
raw factory table has the common lifecycle prefix but is not itself canonical
`IUnknown`; the query is required before typed adoption. The original bootstrap
reference is released on every path. The returned handle stays on the caller's
thread even if the provider supports shared threading.

## Acquisition and ownership

Acquisition means obtaining one owned root factory COM reference. The descriptor
stores `acquire: Option<NcAcquirePluginFactory>`; generated bootstrap code adapts
the implementation's typed construction policy to that fixed no-argument ABI.
A provider may create a fresh factory or retain an existing instance. The runtime
promises no singleton and no identity reuse between successful calls.

The runtime compares the requested IID, full signature and threading capability
before invoking bootstrap. It releases the catalog guard before plugin code runs,
then validates the returned object/table pair and its declared contract. Rejected
returned references are released. The raw output starts null and only success
transfers one owner. Typed facade adoption preserves the requested IDL factory
contract and threading model. Product construction occurs through factory methods.

Loading and metadata queries never acquire factories. Registration does not need
an application factory type and does not traverse the IDL contract graph. Exact
compatibility belongs to factory acquisition and subsequent interface transfers.

## Publication, provenance and repeated loading

Native open, symbol lookup, descriptor entry and validation run outside the catalog
lock. Registration copies dependency GUIDs, then takes one write guard to require
all hard providers, check identity/capacity and publish descriptor and origin
together. Failure leaves the catalog unchanged. Plugin code and user callbacks
never run while a catalog guard is held.

The same GUID and descriptor address reuse the first registration and consume no
extra capacity. A different descriptor claiming that GUID returns
`NC_E_INVALID_ARG`; matching name or version text alone does not make providers
interchangeable. Repeated static registration is idempotent. A later dynamic open
of the same already-static descriptor preserves its original static origin.

Native mappings remain pinned until process exit, including inspection-only opens
and failures after opening. Repeated loads still open and validate the library;
catalog idempotency does not suppress native loader calls or initialization.
Once registered, metadata and factory acquisition do not reopen the origin path,
so the plugin remains callable after its file is removed.

`NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS` bounds distinct dynamic registrations at
1,024. Static registrations, repeats and inspection-only opens are excluded. The
count derives from authoritative records. Inspection-only pins therefore remain
outside that bound; build tooling inspects in a short-lived process. There is no
unloading, path discovery, recursive dependency loading or injected runtime.

## Unregistered inspection and errors

`Runtime::with_plugin_library_description(path, visitor)` remains unsafe: the
path selects trusted native code, and opening can execute platform loader code.
It validates a descriptor and invokes the visitor synchronously exactly once,
without registering the plugin, acquiring a factory or following dependencies.
The descriptor graph is borrowed only during the callback; copy needed metadata.
Do not retain descriptor pointers or panic across the C callback boundary. Once
opened, the mapping is pinned even if validation or the visitor later fails.

| Operation | Failure policy |
| --- | --- |
| Static registration | Ordinary validation/dependency/identity status; no partial publication. |
| Required live loading | Native loading, description, dependency, identity or capacity failures diagnose and abort. |
| Raw live-load arguments | Invalid pointers/path shape return statuses before loading; the application facade treats these as fatal too. GUID output is cleared to nil first. |
| Factory acquisition | Ordinary unknown-plugin, interface/signature/threading or bootstrap status. |
| Registered metadata | `None` for missing Rust lookup; raw `NC_E_NOT_FOUND`; copy/allocation errors remain statuses. |
| Unregistered inspection | Recoverable loader/validation/visitor status. Callback status is returned unchanged by C; Rust visitor errors are normalized. |

## Verification boundaries

The regression suites check complete lookup/enumeration agreement, dependency
ordering, failed publication, duplicate registration, owned copies, borrowed
views across catalog growth, null/empty/undersized buffers and untouched spare
slots. They also verify that metadata does not invoke bootstrap, that ABI remains
v1, and that independent C layouts match the combined table and plugin record.
Native-library tests cover provenance, identity conflicts, post-unlink metadata
and retained factory code. Both shared-runtime and static configurations exercise
the same contracts; generated factories still undergo exact identity checks.
