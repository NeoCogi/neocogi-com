# neocogi-com-core

The allocation-free C-shaped ABI shared by generated contracts, hosts, plugins,
and the process runtime. Most authors use the `neocogi-com` facade. Core object
ABI v7 remains experimental.

Core defines statuses, GUIDs, exact signatures, typed metadata, interface headers
and vtables, `IUnknown`, `ComPtr`, and `ComRef`. `ComInterface` owns identity and
the minimum threading promise. `ComFactoryInterface` marks interfaces eligible
for plugin bootstrap; `ComFactoryImplementation` associates private factory
storage with that interface.

A contract crate generates Rust bindings and fixed signatures. It has no native
module-description library or runtime registry entry. The IDL build helper hashes
the complete transitive graph and derives each declaration's final 32 bytes.
Core compares those bytes at factory/interface boundaries and does not hash or
reconstruct an IDL graph.

Plugin descriptor ABI v1 publishes plugin identity/version, hard dependencies,
full root-factory IID/signature, threading, and one zero-argument bootstrap.
Its getter is `nc_get_factory_plugin_description_v1`. Runtime discovery is v1,
with operation, plugin-registry, inspection, and debug tables. See the
[ABI specification](../docs/abi-v7.md) for layouts and ownership rules.

## Interface safety

`ComInterface` is an unsafe extension point: implementing it certifies header
layout, vtable prefix, exact identity, lifetime, and conversion to `IUnknown`.
Generated interfaces provide the normal implementation.

| Handle | Owns a reference | `Send` / `Sync` |
|---|:---:|:---:|
| `ComPtr<T, ThreadLocal>` | Yes | No |
| `ComPtr<T, ThreadSafe>` | Yes | Yes |
| `ComRef<'a, T, ThreadLocal>` | No | No |
| `ComRef<'a, T, ThreadSafe>` | No | Yes |

The interface states a minimum promise; a particular implementation may support
a stronger capability. Pointer markers preserve that implementation guarantee.
A thread-safe view may be weakened safely; no safe API upgrades a local view.

`ComDeserialize` is typed local source decoding. It does not use `Any`, an erased
value tree, descriptor-offset mutation, or a runtime type registry.

Core is `no_std` by default. `alloc` enables owned decoding/diagnostic helpers;
`std` implies `alloc`. Loading, process state, object allocation, and deployment
policy belong in other crates.
