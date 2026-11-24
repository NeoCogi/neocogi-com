# Migrate a provider to WIP ABI 24

SDK `0.5.0-wip-alpha.1` uses [ABI 24](../abi.md). The
[0.4.0 release](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/docs/abi-v3.md)
uses ABI v3/core generation 21. SDK `0.3.0` uses ABI v2/core generation 19.

| Boundary | SDK 0.4.0 | Development SDK |
| --- | --- | --- |
| Native ABI | v3/core 21 | 24 |
| Plugin discovery | `nc_get_plugin_descriptor_v3` | `nc_get_plugin_descriptor` |
| Service and metadata records | `NcHostApiV3`, `NcPluginApiV3`, other `V3` records | `NcHostApi`, `NcPluginApi`, and other unsuffixed records |

Rebuild the host, providers, compiler and generated bindings from the same SDK
revision. The plugin descriptor and host bundle both carry `abi_version`.
The generated adapter checks it before interpreting service tables or acquiring
factories. Exact interface IID and signature checks still apply.

The persistence service borrows shared-owned factory and snapshot-root pairs.
Pass a `ComPtr<IPersistenceFactoryAbi>` to registry registration and a
`&ComPtr<IUnknown>` to the snapshot operation. Handwritten native providers
must lend the corresponding live shared interface for the full raw call.
Persisted NCB-v1 documents keep their encoding.

Both serialization session `fail` slots take the originating `Status` before the
text buffer. A format implementation records that
status with the first failure and keeps it for subsequent operations.

Link `neocogi-com-host` and forward its supplied `RuntimeHandle` to nested
providers. Register or load `nc.serialization` before snapshot and restore.

## Compose a native interface prefix

Concrete COM storage selects its lifetime policy at construction. For a
handwritten method table over SDK-managed storage, copy the prefix selected by
the implementing type and install the exact interface identity.

<!-- checked-example: lifetime-prefix -->
```rust
use neocogi_com_sdk::{ComObject, IUnknownVTable, NcInterfaceId};

pub const fn interface_prefix<T: ComObject>(id: &'static NcInterfaceId) -> IUnknownVTable {
    IUnknownVTable {
        interface_id: id,
        ..*T::UNKNOWN_VTABLE
    }
}
```

Pair the prefix with the complete method table and matching allocation. The
prefix copy leaves ownership unchanged. Generated providers use the same
construction. C lifecycle adapters stay private to core and are called through
the selected table.
