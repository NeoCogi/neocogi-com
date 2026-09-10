# neocogi-com-core

One of `com-validation-none`, `com-validation-sha-256`,
`com-validation-fnv-1a-128`, or `com-validation-mangling` selects the
assembly-wide ABI validation representation. Every mode uses the same ABI view.
The exact modes require descriptor signatures to match typed metadata; `none`
uses a reserved unchecked sentinel while retaining structural validation. A
build with no mode is reserved for representation-neutral inspection tooling.

`neocogi-com-core` contains the small C-shaped ABI contract used by `neocogi-com` hosts
and plugins. core ABI v6 is currently an experimental pre-alpha draft, not a
ratified cross-release compatibility promise.

This crate is intentionally minimal. It defines the C-shaped types that must be
shared across independently built artifacts, while higher-level Rust ergonomics
live in `neocogi-com`.

## What Is In Core

`neocogi-com-core` defines:

- `NcStatus` and built-in `NC_*` status constants
- `NcGuid` and GUID parsing/formatting
- `AbiType` and `AbiPlainValue`
- format-neutral `ComDeserialize` construction for typed records, enums,
  options, fixed arrays and, with `alloc`, owned sequences and ordered maps
- `IUnknown`, `IUnknownVTable`, and `ComInterface`
- raw read-only byte and UTF-8 string ABI interfaces
- static UTF-8 metadata
- method descriptors with typed parameter/return metadata, plus interface,
  class, class-table, and plugin descriptors
- immutable typed constructor frames identified by a class descriptor signature
- runtime, plugin registry, type registry, inspection, and debug API v1 table layouts
- `NC_COM_CORE_ABI_VERSION` and `NC_COM_RUNTIME_ABI_VERSION`
- `nc_get_plugin_descriptor` symbol and function signature
- `validate_plugin_descriptor_with_issues`, the descriptor validator with
  structured issue reporting and an allocation-free configuration

These types are `#[repr(C)]` or ABI marker traits where appropriate. Descriptor
structs are exact v1 `repr(C)` layouts. Layout changes require a core ABI
version change; pointer/count fields are still validated structurally before
registration. Every valid class descriptor contains exactly one canonical
zero-method `IUnknown` interface descriptor; validation rejects missing,
noncanonical, and duplicate root entries.

Class descriptors also declare whether constructed objects are thread-local or
thread-safe. Validation rejects unknown threading tags. A thread-safe declaration
is an unsafe producer promise about methods, contained state, and destruction;
the macro-generated path checks `Send + Sync` and emits matching pointer types.
Threading markers are per-pointer capabilities: a thread-safe `ComPtr` or
`ComRef` may be explicitly weakened to a thread-local view without changing its
raw pointer or reference count, while no safe operation performs the reverse
upgrade.

| Handle | Owns a reference | `Send` | `Sync` | Refcount on ordinary clone/drop |
|---|:---:|:---:|:---:|---|
| `ComPtr<T, ThreadLocal>` | Yes | No | No | `AddRef` / `Release` |
| `ComPtr<T, ThreadSafe>` | Yes | Yes | Yes | `AddRef` / `Release` |
| `ComRef<'a, T, ThreadLocal>` | No | No | No | None |
| `ComRef<'a, T, ThreadSafe>` | No | Yes | Yes | None |

The marker cannot be inferred from `T` because an interface can be implemented
by classes with different threading guarantees. It cannot be read from the raw
pointer either because both modes intentionally share one C representation.
The marker retains the producer's proof after the concrete class type has been
erased behind an interface. This matters for destruction as well as calls:
moving an owning pointer can move the thread on which its final `Release` runs.

`ComInterface` is an unsealed unsafe trait because safe `ComPtr` operations rely
on its implementation to certify the actual interface-header layout, vtable
prefix, GUID/signature identity, pointer lifetime, and `IUnknown` conversion.
Ordinary component contracts should use `#[com_interface]` from `neocogi-com`, which
generates the implementation. Handwritten implementations are reserved for
low-level foreign-interface adapters and require an explicit `unsafe impl`.

core ABI v6 plugin descriptors contain declarative metadata only. Hosts explicitly
load dependencies before dependents; registration does not discover or load
dependencies.

Descriptor validation reports trusted metadata shape problems. It is not hostile
plugin isolation, does not sandbox plugin code, and cannot safely probe
arbitrary non-null raw pointers beyond the unsafe descriptor contract.

Status values distinguish an invalid resource request
(`NC_E_OUT_OF_RANGE`) from failure to allocate a valid, policy-compliant request
(`NC_E_OUT_OF_MEMORY`). Allocation-backed crates should use fallible reservation
when mapping allocator failure to the latter; core itself remains
allocation-free without its optional `alloc` feature.

Interface descriptors use canonical `NcAbiSignature` byte views. Generated
interfaces serialize typed `NcAbiSignaturePart` records with grammar v1, then
store the mangle itself or its SHA-256/FNV-1a-128 result. Procedural macros emit
the records while constant evaluation resolves target layouts and nested
signatures. Descriptor validation walks the same records once and rejects
malformed or mismatched signatures. Rendered method signatures remain separate
diagnostic metadata.
The allocation-backed `format_method_signature` helper is unsafe because raw
descriptor records are publicly constructible: callers must establish every
transitive pointer/count invariant before formatting. Generated static
descriptors and registered descriptors from modules retained by the runtime
satisfy that lifetime requirement.

## Typed Source Decoding

`ComDeserialize` is a local Rust construction protocol, not an ABI marshalling
format and not binary reflection. A source adapter implements `ComDeserializer`
once. Concrete values then recursively request their declared member types;
`#[com_struct(deserialize)]`, `#[com_enum(..., deserialize)]`, and
`#[derive(ComDeserialize)]` generate the record and enum switches.

`Option<T>` and `[T; N]` work without allocation. The `alloc` feature adds
`String`, `Vec<T>`, `Box<[T]>`, and `BTreeMap<K, V>`. Borrowed slices are
intentionally absent because a general decoder cannot promise input storage with
the required lifetime. Decode an owned or fixed-capacity value and validate its
domain bound instead.

Format adapters own syntax, diagnostics, and allocation limits. Type
implementations own construction and defaults. Neither side uses `Any`, an
erased intermediate value tree, descriptor field offsets, or a class registry.
The protocol therefore adds no lookup to UI update, layout, or paint paths.

## What Is Not In Core

`neocogi-com-core` does not provide:

- generated object storage
- reference-count implementation helpers
- proc macros
- heap allocation helpers
- owned `String` or `Vec` wrapper objects
- a process-global runtime implementation
- dynamic library loading

Those belong in `neocogi-com` or in host-specific runtime code. Core only defines the
binary shapes and raw ABI contracts. The owned `NcBytes` and `NcString` helpers
live in `neocogi-com` because they allocate and manage reference-counted object
storage.

## no_std

`neocogi-com-core` is `no_std` by default and allocation-free. The optional `alloc`
feature enables owned diagnostic helpers such as method signature rendering
and uses `alloc::collections::BTreeSet` to accelerate duplicate validation. The
same validator remains available without `alloc`, using an allocation-free
fallback with identical diagnostics. The optional `std` feature implies
`alloc` while enabling standard error trait implementations; registry hash-map
indexes remain in the separate, intentionally `std`-only runtime crate.

```toml
neocogi-com-core = { version = "0.1", default-features = false }
```

## Plugin Discovery

Plugins expose a single descriptor entrypoint:

```rust
pub type NcGetPluginDescriptor = unsafe extern "C" fn(
    out: *mut *const NcPluginDescriptor,
) -> NcStatus;
```

The exported symbol name is:

```rust
pub const NC_GET_PLUGIN_DESCRIPTOR_SYMBOL: &str =
    "nc_get_plugin_descriptor";
```

The descriptor points to static metadata that remains valid while the shared
library is loaded. Hosts validate the descriptor graph once at load or
registration time, not on every method call.

`NcPluginDescriptor` contains a pointer/count table of
`NcPluginDependencyDescriptor` entries. Each entry is one hard plugin ID that
must already be registered. The table describes activation order only and never
causes path discovery or recursive loading.

Binary compatibility is decided by GUIDs and canonical ABI signatures. Names and versions in
descriptors remain useful for diagnostics and host policy, but they do not
replace GUID/signature checks.

## When To Depend On This Crate

Most component authors should depend on `neocogi-com`.

Depend on `neocogi-com-core` directly when you are writing low-level host runtime
code, a foreign-language bridge, a custom loader, or a component system that
needs the raw ABI without the higher-level Rust helpers.
