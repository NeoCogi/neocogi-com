# neocogi-com-core

`neocogi-com-core` defines the allocation-free, C-shaped ABI shared by generated
contract crates, hosts, plugins, and `libnc_com_shared_runtime`. Most component
authors should depend on `neocogi-com`; this crate is for generators, loaders,
foreign-language adapters, and the runtime implementation.

Core ABI v7 is an experimental pre-alpha draft. Its layouts are explicit and
tested, but cross-release compatibility begins only after a future ratification.

## What the ABI contains

- `NcStatus`, built-in `NC_*` status values, and `NcGuid`.
- ABI type records and canonical SHA-256 signature records.
- `IUnknown`, generated interface headers and vtables, `ComPtr`, and `ComRef`.
- object-description traits used by generated class and factory bindings.
- canonical `NcContractModuleDescription` graphs and exact
  `NcContractModuleReference` values.
- `NcFactoryPluginDescription`, containing plugin metadata, direct contract
  references, hard plugin dependencies, and one root-factory bootstrap.
- the runtime ABI v2 operation, registry, inspection, and debug table layouts.

The runtime ABI has no class table, class-ID constructor lookup, erased
constructor frame, or generic value bag. A plugin exposes one factory; ordinary
typed factory methods create every later object.

## Descriptions and generated layouts

A contract module has two complementary representations:

1. Its generated Rust crate contains C-layout types, vtable declarations, typed
   call wrappers, and a `MODULE: NcContractModuleReference` that imports one
   native getter.
2. Its fingerprint-qualified native library owns the single canonical
   `NcContractModuleDescription`: module identity, ABI fingerprint, declaration
   summaries, and exact dependency getters.

Hosts and plugins therefore compile the types they call, but do not embed
duplicate canonical description graphs. When different builds link incompatible
revisions of one logical module, their different native filenames can coexist;
the shared runtime sees the common module ID and different fingerprints and
returns `NC_E_CONTRACT_MISMATCH`.

`NcFactoryPluginDescription` identifies the plugin and its linked contract
modules. It also records the root factory ID, threading promise, and activation
pointer. The exported getter is `nc_get_factory_plugin_description`. The
description remains valid while the plugin library is mapped.

## Object and interface safety

`ComInterface` is an unsealed unsafe trait because safe pointer operations rely
on its implementation to certify header layout, vtable prefix, identity,
signature, lifetime, and conversion to `IUnknown`. Generated interfaces provide
the normal implementation; handwritten implementations are a low-level escape
hatch.

Threading is an explicit pointer capability:

| Handle | Owns a reference | `Send` | `Sync` |
|---|:---:|:---:|:---:|
| `ComPtr<T, ThreadLocal>` | Yes | No | No |
| `ComPtr<T, ThreadSafe>` | Yes | Yes | Yes |
| `ComRef<'a, T, ThreadLocal>` | No | No | No |
| `ComRef<'a, T, ThreadSafe>` | No | Yes | Yes |

The marker cannot be inferred from an interface because different object
implementations may expose the same interface with different guarantees. A
thread-safe view can be weakened safely; no safe API upgrades a thread-local
view.

## Typed source decoding

`ComDeserialize` is a local Rust construction protocol, not ABI marshalling or
binary reflection. A format implements `ComDeserializer`; concrete Rust types
then request their declared members. The protocol uses no `Any`, descriptor
offset mutation, erased value tree, or runtime type registry.

## Features

- No default `std` requirement: the core ABI is `no_std` and allocation-free.
- `alloc` enables owned decoding and diagnostic helpers.
- `std` implies `alloc` and adds standard error integrations.
- Generated ABI signatures are always SHA-256 digests of canonical typed
  streams; their representation is not feature-selectable.

The crate deliberately does not implement dynamic loading, process-global
storage, object allocation, path discovery, trust policy, or plugin management.
