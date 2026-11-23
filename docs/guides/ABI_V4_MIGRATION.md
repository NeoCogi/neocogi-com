# Migrate a provider to WIP ABI v4

SDK `0.5.0-wip-alpha.1` uses [ABI v4](../abi-v4.md), which is provisional.
The [0.4.0 release](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/docs/abi-v3.md)
uses ABI v3/core generation 21. SDK `0.3.0` uses ABI v2/core generation 19.

| Boundary | ABI v3 in SDK 0.4.0 | Development ABI v4 |
| --- | --- | --- |
| Core generation | 21 | 22 |
| Runtime and descriptor families | 3 | 4 |
| Plugin discovery | `nc_get_plugin_descriptor_v3` | `nc_get_plugin_descriptor_v4` |
| Versioned service and metadata records | `NcHostApiV3`, `NcPluginApiV3`, other `V3` records | Corresponding `V4` records |

Rebuild the host, providers, compiler and generated bindings from the same SDK
revision. Generated providers and the inspector use the current discovery symbol.
Discovery accepts the current family and core before interpreting service tables
or acquiring factories. Exact interface IID and signature checks still apply.

The persistence service borrows shared-owned factory and snapshot-root pairs.
Pass a `ComPtr<IPersistenceFactoryAbi>` to registry registration and a
`&ComPtr<IUnknown>` to the snapshot operation. Handwritten native providers
must lend the corresponding live shared interface for the full raw call.
Persisted NCB-v1 documents keep their encoding.

Link `neocogi-com-host` and forward its supplied `RuntimeHandle` to nested
providers. Register or load `nc.serialization` before snapshot and restore.
