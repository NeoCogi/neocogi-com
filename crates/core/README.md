# neocogi-com-core

The C-shaped ABI shared by generated contracts, hosts, plugins, and the process
runtime. Its default configuration is allocation-free; optional `alloc` helpers
own decoded values and binary documents. Most authors use the `neocogi-com`
facade. Core object ABI v7 remains experimental.

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
with operation, plugin-registry, inspection, debug, and persistence tables. See the
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

## Text, bytes, and borrowed storage

These types have different ownership and conversion rules:

| Type | Conversion | Ownership and validation |
| --- | --- | --- |
| `NcStaticUtf8` | `unsafe as_str<'a>(self) -> Result<&'a str, NcStatus>` and `unsafe as_bytes<'a>(self) -> Result<&'a [u8], NcStatus>` | Borrows descriptor storage; the caller establishes memory validity and the returned lifetime. `as_str` validates UTF-8. |
| `NcUtf8View` | Public `data: *const u8`, `len: usize`, and `EMPTY`; no conversion methods | Runtime-owned bytes with an externally specified lifetime. The facade copies inspection text into owned strings. |
| `NcAbiSignature` | `unsafe as_bytes<'a>(self) -> Result<&'a [u8], NcStatus>` | Binary signature storage, not text. `from_static` accepts an empty sentinel or exactly 32 bytes. |
| `NcCopyFromBufferOffset<'a, T>` | `as_slice(&self) -> &'a [T]` | Holds an actual Rust slice. The logical offset does not slice or shift its storage. |
| `NcCopyToBufferOffset<'a, T>` | `as_mut_slice(&mut self) -> &mut [T]` | Holds exclusive caller-provided storage; counts are elements, not necessarily bytes. |
| `INcStringAbi` / `INcBytesAbi` | Raw length, indexed-byte, and bulk-copy vtable operations | A COM reference retains the provider. Safe allocating conversions live in the facade. |

`NcStaticUtf8` uses a `u32` byte count. An empty view returns an empty slice/text
even with a null pointer; a nonempty null view returns `NC_E_POINTER`. Invalid
UTF-8 returns `NC_E_INVALID_ARG`. Neither check proves that another non-null
pointer is readable. Its `Copy` implementation copies only the pointer/count,
and the returned `'a` is chosen by the unsafe caller rather than tied to `self`.
The word “Static” describes metadata's intended lifetime in its owning module;
the value neither owns those bytes nor pins a shared library.

For a readable `NcCopyFromBufferOffset<u8>`, use
`core::str::from_utf8(buffer.as_slice())` when text is required. There is no
universal safe `as_str(&self) -> &str` on raw ABI views. See the
[facade's data helpers](../api/README.md#strings-and-byte-buffers) for owned copies.

## Experimental persistent values

`ComSerialize` streams explicit initialization records through `ComSerializer`.
The facade derives `ComSerialize` and `ComDeserialize` for owned records and
enums, including recursive vectors and named/tuple payloads. Both derives honor
the existing `com_deserialize(sequence)` and `rename_all` policy. This is value
serialization; opting a live object into persistence remains its provider's job.

With `alloc`, `binary::to_vec` and `binary::from_slice` use NCB version 1: a
four-byte marker, little-endian scalars, u32 lengths/variant ordinals, and
declaration-order record fields. usize/isize occupy eight bytes; reads check
the target range. No native padding, pointers, or ABI signatures are saved.
Record/variant schema evolution must be versioned by the enclosing object
record; a changed COM method does not change the binary initialization schema.

Decoding rejects trailing bytes, truncated data, invalid booleans/UTF-8/variants,
unknown record fields, and duplicate map keys. Limits bound document bytes,
container counts, recursive construction, and total decoded values. Missing
trailing record fields only work with explicit default-backed decoding.
The wire format is experimental and is not yet a promised long-term file ABI.

Owned sequence decoding caps initial reservation at 4 KiB regardless of the
advertised element count or Rust element size. Capacity grows only as typed
elements are successfully decoded, so malformed counts cannot force a large
eager allocation before the first element is checked.

Built-in persistence contracts live in `persistence`: provider traits, interface
headers, clients and metadata require neither allocation nor a runtime. With
`alloc`, the module also provides the portable `ObjectRecord` schema and limits.
The COM facade re-exports these contracts and supplies concrete-provider thunks.

The canonical IDL is `idl/neocogi/com/persistence/objects.ncidl`. Its built-in
bindings are checked in because core cannot depend on its own IDL compiler.
Verify them with `cargo test -p neocogi-com-macros persistence_bindings_match_idl`;
set `NC_UPDATE_PERSISTENCE_BINDINGS=1` on that command to regenerate both halves.


### Binary resource limits

The encoder and decoder apply the same `binary::Limits` policy. For the built-in
and derived codec pairs, encoding a valid value successfully produces a document
that the matching decoder accepts under the same limits. Custom serializer and
deserializer implementations must describe the same schema and consume all its
members. Format/schema validation and provider activation remain separate checks.

- `bytes` bounds the complete encoding, including the NCB marker, lengths, and tags.
- `elements` bounds immediate sequence elements, record fields, or map pairs in
  each container. It is not a cumulative count across sibling collections.
- `values` bounds all scalar and container values throughout the document. A
  vector of N scalar elements costs N + 1; a map costs one plus its keys and
  values. Empty containers and zero-sized unit values still cost one each.
- `depth` bounds the number of values on a root-to-leaf path. A scalar or empty
  container root requires one; a nonempty vector of scalars requires two.

`None` costs one value; `Some(x)` costs one plus `x`. An enum costs one value,
with no additional payload for a unit case. Newtype cases contain their value;
tuple/record cases contain a sequence/record value and its fields. Text costs one
value and its bytes consume `bytes`; string lengths, enum tags, and presence tags
are framing, not extra values or depth levels.

`Vec<u8>` always remains a collection: each byte consumes a value and an element
of that collection. In particular, `ObjectRecord.state` receives this accounting
even though its bytes may themselves encode provider initialization. A separate
opaque-blob type would require an explicit schema/codec operation; the current
codec does not implicitly convert byte vectors to blobs. A byte ceiling therefore
does not promise that every smaller document fits the other limits.
