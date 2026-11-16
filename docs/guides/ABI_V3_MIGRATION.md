# Moving providers to the development ABI v3

This guide applies to the unreleased `0.4.0-wip` sources. Published SDK `0.3.0`
uses ABI v2/core generation 19. Package versions, live object ABI versions,
interface identities and persistence document versions are separate authorities.
The [ABI specification](../abi-v3.md) owns the development binary contract.

## Rebuild the complete composition

Use matching development SDK sources for the application, every provider, IDL
compiler and generated bindings. Rebuild statically linked providers and native
libraries together; do not load an old provider by renaming its export. The new
header/type-information layouts cannot be interpreted as ABI v2.

| Boundary | Earlier ABI-v2 source | Development ABI v3 |
| --- | --- | --- |
| Core generation | 19 | 20 |
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

`NcObjectTypeInfo` now starts with integer `ownership`: `NC_OWNERSHIP_SHARED`
(value 1) or `NC_OWNERSHIP_BOXED` (value 2). Match the constructor to that metadata;
safe Rust constructors reject a mismatch. Keep metadata immutable through the
last weak release. The record also retains provider-local cleanup callbacks and
its exact interface table array/count; its size is now five pointer-width words.
Unknown kinds are invalid, including when a boxed owner has already expired.

The `destroy_value` and `deallocate` callbacks receive the canonical allocation
address as `*mut c_void`. Recover the exact concrete provider allocation type
inside its own callback. Destroy the initialized application value once, then
free its storage only after final weak release, using the allocating provider.
The SDK uses concrete `__ComStorage<T, Lifetime, Access>` instantiations; the ABI
address does not introduce an application payload registry or dynamic downcast.

`NcWeakRaw.control` transports that allocation address as `*mut c_void`.
Fallible raw entry points inspect the leading immutable metadata pointer/kind
before constructing a policy-specific header reference. Valid shared weak tokens
become typed `WeakComPtr`; a checked shared-weak adoption rejects boxed storage
and releases the transferred weak owner exactly once. Common weak retain/release
accept either known policy so cleanup does not depend on a strong-count tag.
Unsafe backing, alignment, storage retention and threading obligations still
apply; admission does not make an arbitrary pointer readable.

## Keep counts, access and storage lifetime distinct

A shared `strong_count` contains only a number. Positive counts retain the value;
zero is terminal and weak promotion reports `NC_E_DROPPED`. Shared method access
uses the separate `ComBorrowState`, so strong ownership does not grant a mutable
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
Verify live and expired cross-policy rejection, owner destruction before final
weak deallocation, failed-promotion output clearing and exact contract mismatch.
The [core regressions](../../crates/core/src/lifetime/tests.rs) and
[generated boxed-call tests](../../tests/com/integration-tests/exclusive_com.rs)
cover these boundaries and owner removal racing admitted readers/writers.
