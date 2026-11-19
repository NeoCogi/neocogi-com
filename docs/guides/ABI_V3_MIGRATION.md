# Moving providers to WIP ABI v3

**ABI v3 is work in progress.** Its layouts and callback/ownership contracts
are provisional and may change before the release freeze. This guide describes
the current candidate; rebuild all participants from matching source revisions.

This guide applies to the provisional `0.4.0-alpha.1` sources. Published SDK `0.3.0`
uses ABI v2/core generation 19. Package versions, live object ABI versions,
interface identities and persistence document versions are separate authorities.
The [ABI version table](../abi-v3.md#abi-identities-and-versions) owns the current
development generations; the rows below are checked against the core constants.

## Rebuild the complete composition

Use matching development SDK sources for the application, every provider, IDL
compiler and generated bindings. Rebuild statically linked providers and native
libraries together; do not load an old provider by renaming its export. The new
header/type-information layouts cannot be interpreted as ABI v2.

| Boundary | Earlier ABI-v2 source | Development ABI v3 |
| --- | --- | --- |
| Core generation | 19 | 21 |
| Runtime and descriptor families | 2 | 3 |
| Plugin discovery | `nc_get_plugin_descriptor_v2` | `nc_get_plugin_descriptor_v3` |
| Optional host-shim discovery | `nc_get_host_api_v2` in earlier development sources | `nc_get_host_api_v3` |
| Versioned service/metadata records | `NcHostApiV2`, `NcRuntimeApiV2`, other `V2` records | Corresponding `V3` records |

The optional host shim was never part of published 0.3.0. Family/core prefixes
are admitted before child service tables, full descriptors or acquisition
callbacks. Old getter symbols are rejected without fallback; a current getter
with a core-19 descriptor is also rejected. Exact interface IID/digest checks
still apply after core admission. Build the inspector from the same development
checkout (`cargo build -p neocogi-inspect`) to inspect these providers; the
published 0.3.0 inspector expects ABI v2/core generation 19.

Select one host implementation: the statically linked host rlib or the native
host shim. Forward its supplied `RuntimeHandle` through nested acquisition.
Copying access does not create another registry. Persistence still requires
explicit registration/loading of `nc.serialization` and selects named `ncb`.

## Update Rust method-wrapper names

Regenerate bindings and replace `ComClientInterface` with `ComRustInterface`,
`NcClientPointer` with `ComInterfacePointer`, the associated `Client<M>` with
`Wrapper<M>`, and generated `IFooClient` names with `IFooWrapper`.
The hooks are `__nc_wrapper_ref`/`__nc_wrapper_mut`; no old-name aliases remain.
The generated `Wrapper` suffix is reserved by IDL admission; the old `Client`
suffix is available to authored declarations. An authored interface literally
named `Client` keeps its name and generates `ClientAbi`/`ClientWrapper` normally.

This rename preserves borrowed lookup, threading, lifetime/call admission and
checked owned-base queries. It changes the Rust API and generated names, not
C record layouts, method signatures, IID/digest bytes or the current ABI/core
versions. WIP ABI v3/core21 remains the current candidate.

## Update generated object support names

Regenerate provider bindings for `__ComSharedInner` and `__ComBoxedInner`.
These two types select the shared and the boxed layout of `ComObjectInner`.
`ComBorrowGate` issues `__ComReadPermit` and `__ComWritePermit` for a shared
object's reader/writer admission. In handwritten Rust, use `LocalRefMut` for
the mutable guard returned by `LocalCom::try_borrow_mut`, alongside `LocalRef`.

Use `ComBytesExt` and `ComStringExt` for byte and string conversions on both
owning `ComPtr` handles and borrowed `ComRef` handles. The methods still perform
bounded copies through the selected interface.

## Name live ABI values and their temporary allocations

When implementing or calling `ComValue` explicitly, use `AbiRepr` for the raw
C-shaped record and `KeepAlive` for the converted data it points into. Retain the
source borrow and any keep-alive through the call. The `BORROW_ABI_REPR` proof
allows direct access only when the layouts and bit validity match recursively.
Regenerate bindings for those associated names. The same representations,
admission checks and IDL encoding apply under ABI v3/core 21.

## Choose one concrete ownership header

Ordinary `#[com_object]` users keep their shared/exclusive declaration and
construction APIs; regenerated code selects the matching storage. Authors of
custom common-lifetime providers must update their allocation and metadata.

| Allocation policy | Header constructor | Mutable ownership field |
| --- | --- | --- |
| Shared | `NcSharedLifetimeState::new(&TYPE_INFO)` | Numeric `strong_count` |
| Boxed | `NcBoxedLifetimeState::new(&TYPE_INFO)` | `owner_and_borrows` |

Replace `NcLifetimeState::new(...)` or `::boxed(...)` with the corresponding
constructor. There is no fused header or compatibility alias. Both headers begin
with `type_info`, then their own ownership field, then `weak_count`: three
pointer-width words on the supported 32/64-bit layouts. The pointer moved from
the last word to the first, so unchanged total size does not imply compatibility.

`NcObjectTypeInfo` contains only destroy/deallocate callbacks and the interface
array/count: four pointer-width words. Core 21 removes core 20's ownership field
and `NC_OWNERSHIP_*` constants. Rebuild all participants: older metadata layouts
are rejected by core admission, while ABI family 3 remains WIP.

The `destroy_value` and `deallocate` callbacks receive the canonical allocation
address as `*mut c_void`. Recover the exact concrete provider allocation type
inside its own callback. Destroy the initialized application value once, then
free its storage only after final weak release, using the allocating provider.
The SDK uses concrete `ComObjectInner<T, Lifetime, Access>` instantiations; the ABI
address does not introduce an application payload registry or dynamic downcast.

`NcWeakRaw.control` is now `*mut NcSharedLifetimeState`, with the same one-word
C pointer representation. Its ABI producer guarantees genuine shared backing.
Boxed owner/weak ABI positions likewise guarantee boxed backing before adoption.
Checked constructors/decoders check shape and exact interfaces, then retain or
consume only that declared policy. Wrong-policy tokens are producer contract
violations; there is no cross-policy rejection/cleanup promise, runtime kind tag,
expected-kind argument or erased weak dispatcher.

`BoxRead::enter`, `BoxWrite::enter` and `try_box_cast` require genuine boxed
storage. Caller contracts or construction establish backing; a raw `.cast()` does
not inspect it. Numeric reference limits, terminal expiration, reader/writer
admission, exact IID/digest checks and provider-local cleanup remain. Generic
strong/scoped interfaces retain their actual vtable contracts and need no header.
See [lifetime contracts](../contracts/COM_LIFETIME_ADMISSION.md).

## Select the provider's lifetime prefix

`ComObject::UNKNOWN_VTABLE` pairs the implementation's storage policy with either
`SHARED_UNKNOWN_VTABLE` or `EXCLUSIVE_UNKNOWN_VTABLE`. The object macro selects it;
the interface compiler copies its callbacks into root and inherited projections.
Handwritten interface support can use the same constant construction:

<!-- checked-example: lifetime-prefix -->
```rust
use neocogi_com::{ComObject, IUnknownVTable, NcInterfaceId};

pub const fn interface_prefix<T: ComObject>(id: &'static NcInterfaceId) -> IUnknownVTable {
    IUnknownVTable {
        interface_id: id,
        ..*T::UNKNOWN_VTABLE
    }
}
```

Pair the result with a matching complete method table and allocation before
publishing it. Copying a prefix neither validates foreign storage nor acquires
ownership. The compiler emits this construction for ordinary providers.

Replace direct imports of the retired `nc_strong_*`, `nc_weak_*`,
`nc_box_release` and `CANONICAL_UNKNOWN_VTABLE` support names. Ownership operations
live on the concrete header types inside core. C adapters such as
`nc_shared_release` and `nc_exclusive_release` are private; generated code uses
the selected prefix, and safe callers use `ComPtr`, `ComBox` and their weak handles.
Only actual vtable slots and provider cleanup callbacks retain a C calling
convention. Weak bookkeeping/promotion are crate-private Rust functions; there
is no standalone SDK weak C-helper surface. A foreign binding needing weak C
calls supplies its own adapter over typed handles, as the native C fixture does.

This support-API change remains WIP ABI v3, now core 21. Provider metadata is
four words and raw inputs require their declared ownership policy. Header sizes,
C interface slots and interface identities remain unchanged.

## Keep counts, access and storage lifetime distinct

A shared `strong_count` contains only a number. Positive counts retain the value;
zero is terminal and weak promotion reports `NC_E_DROPPED`. Shared method access
uses the separate `ComBorrowGate`, so strong ownership does not grant a mutable
borrow. `QueryInterface` still transfers one shared owner on success.

A boxed allocation has no strong counter. `owner_and_borrows` combines the owner
bit, writer bit and reader count in one atomic so call admission cannot race
owner removal. It contains no ownership-kind tag. Owner drop clears the owner
bit, rejects new calls, waits for admitted calls and destroys the value before
returning. Guards release access only. Drop cannot run inside its own admitted
call because it would wait for that call to finish.

Both policies have one [lifecycle](../contracts/COM_INTERFACE_STORAGE.md#lifetime-state-machine):
Live → Expired → Freed. Expired begins when strong ownership ends or the boxed
owner bit clears. Already admitted calls/destruction may still be running; an
implicit weak reference retains storage until the destructor returns. Surviving
weak observers retain metadata only. Final weak release frees storage; there is
no stored Freed flag or required intermediate weak-only allocation phase.

This split preserves interface GUID/digest bytes, IDL fingerprint encoding v14,
NCB-v1 documents, precise status handling and allocator-local destruction.
Independently implemented providers opting out of the common weak/header protocol
keep their own vtable lifecycle; do not cast arbitrary COM objects to these headers.

## Validate custom providers

Use the [independent C fixture](../../tests/com/integration-tests/fixtures/native-c-abi/native/caller.c)
as an exercised layout/callback example. Match atomic width, alignment and
synchronization on each target; C declarations alone do not prove compatibility.
Verify exact-contract rejection on live and expired matching-policy inputs, owner destruction before final
weak deallocation, binding-owned C adapter output clearing and exact contract
mismatch. Local Rust promotion returns Result<Option<_>, NcStatus> directly;
zero precedes lookup and failed negotiation releases its acquired strong owner.
The [core regressions](../../crates/core/src/lifetime/tests.rs) and
[generated boxed-call tests](../../tests/com/integration-tests/exclusive_com.rs)
cover these boundaries and owner removal racing admitted readers/writers.
