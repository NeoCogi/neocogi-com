# nc-com core ABI v5 Specification

Status: experimental draft (pre-alpha; not ratified)

Core ABI version: `NC_COM_CORE_ABI_VERSION = 5`

Dynamic plugin descriptor symbol: `nc_get_plugin_descriptor`

This document specifies the same-process C-shaped ABI used by `nc-com` core ABI v5.
It is intended to be precise enough for host/plugin authors, future nc-com
maintainers, and independent tooling that validates or inspects runtime
contracts.

Rust snippets are classified by their fence attributes and checked through the
dedicated ABI-spec doctest harness:

- `rust` is a complete normative example that must compile and run.
- `rust,no_run` is a complete normative example that must compile but is not
  executed.
- `rust,compile_fail` is a normative rejected example that must fail to
  compile.
- `rust,ignore` is an illustrative declaration or fragment whose surrounding
  prose, rather than standalone compilation, is normative.

## 1. Terminology

The words "must", "must not", "should", and "may" are normative.

- A host is the process or library that owns plugin loading, registration, and
  class lookup policy.
- A component is code that exports or statically provides COM-style classes.
- A dynamic plugin is a shared library that exports the plugin descriptor
  entrypoint.
- A descriptor is immutable metadata described by the `Nc*Descriptor` structs.
- An interface pointer is a pointer to a `repr(C)` interface header containing
  a vtable pointer and an opaque object pointer.
- An owned COM reference is a reference that must eventually be released with
  `Release`.
- A borrowed COM reference is valid only for the current call unless the callee
  acquires an owned reference with `AddRef`.
- Draft means the ABI design may change incompatibly and carries no
  cross-release compatibility promise.
- Ratified means a named release, source tag, and changelog entry have frozen
  core ABI v5 as a compatibility baseline.
- Published means introduced at or after that ratification baseline. Repository
  examples, draft golden vectors, and pre-alpha artifacts are not published ABI
  commitments merely because they are visible or distributed.

## 2. Scope

nc-com core ABI v5 defines an explicit Rust-to-Rust component boundary by using a
C-shaped ABI. The Rust compiler's native ABI is not part of the contract.

The explicit boundary consists of:

- `repr(C)` ABI structs and descriptor structs
- `repr(transparent)` status and integer wrapper values
- `extern "C"` vtable functions
- explicit `NcStatus` return values
- explicit `NcGuid` identifiers
- SHA-256 ABI hashes
- COM-style `QueryInterface`, `AddRef`, and `Release`
- plugin-owned descriptor metadata and versioned runtime API tables

The following must not cross the ABI boundary directly:

- Rust trait objects
- Rust references
- Rust slices
- raw buffers whose lifetime or ownership is implicit
- `String`, `Vec<T>`, `Box<T>`, or other Rust-owned containers
- Rust allocator ownership
- Rust drop obligations not represented by COM reference counting
- unwinding panics
- Rust-native enum layout
- Rust-native generic type identity

Owned or long-lived variable-size data must cross as COM objects or through
another explicit protocol. Bounded call-scoped copy buffers may cross through
the `CopyFromBufferOffset<T>` and `CopyToBufferOffset<T>` passing modes. The
built-in read-only byte and string interfaces are specified in section 15.

ABI-v5 descriptor metadata exists for runtime identity, compatibility
validation, registration, construction, and inspection. It is deliberately not
a source-level reflection schema. In particular, aggregate hashes are opaque,
one-way compatibility fingerprints: runtime consumers compare and propagate
them but do not reconstruct aggregate names, fields, variants, or declarations
from them. Core ABI v5 does not define reverse generation of C headers, Rust types,
or other language bindings from a plugin binary or inspection artifact.

## 3. Process ABI Requirements

nc-com core ABI v5 is a same-process raw pointer ABI. A host and component are
compatible only when they share the same process ABI:

- target architecture
- pointer width
- endianness
- C calling convention for `extern "C"`
- representation of Rust primitive types used in the ABI surface
- `NC_COM_CORE_ABI_VERSION`
- expected interface IDs and interface ABI hashes

The following may differ between independently built host and component
artifacts when the explicit ABI surface is unchanged:

- Rust compiler version
- Cargo dependency graph
- crate feature graph, including `std`, `alloc`, and `no_std` choices
- optimization level
- codegen units
- incremental compilation
- debug assertions
- LTO mode within one artifact
- static versus dynamic linkage
- allocator implementation, when allocator ownership does not cross the ABI
- `panic=abort` versus `panic=unwind`, because panics are not recoverable ABI
  errors

Cross-architecture, cross-endian, cross-process, or cross-machine
communication requires a serialization or RPC layer and is outside core ABI v5.

### 3.1 Rust-to-Rust trust boundary

Core ABI v5 is a Rust-to-Rust plugin contract. Supported hosts, runtimes, and
plugins use nc-com's safe wrappers and generated ABI implementations on both
sides of the shared-library boundary. The C-shaped records, raw pointers, and
`extern "C"` functions provide a stable process ABI; they do not define a
general-purpose defensive C API for mutually untrusted callers.

Unless an operation explicitly assigns null a meaning or specifies a status for
a null argument, its pointer requirements are safety preconditions. A required
input pointer must be non-null, correctly aligned, and point to a live value of
the declared ABI type that remains readable for the documented call duration. A
required output pointer must likewise point to suitably aligned writable storage
for the declared output. Ownership and borrowing requirements are also
preconditions. nc-com's safe wrappers and generated implementations establish
these invariants; independently handwritten unsafe adapters are responsible for
establishing the same invariants themselves.

Implementations may check for null defensively and return an error, but such a
check cannot validate an arbitrary non-null pointer. Unless the operation says
otherwise, callers must not rely on a particular status, output value, or other
postcondition after violating a pointer, lifetime, layout, aliasing, or ownership
precondition.

Trust does not bypass version negotiation. Artifacts built with different
nc-com releases may interoperate only when the core and runtime ABI versions,
interface IDs and ABI hashes, and append-only prefix rules in this specification
establish compatibility. An incompatible artifact must be rejected before its
incompatible ABI surface is used.

The pre-alpha verification matrix is intentionally little-endian. It compiles
the ABI crates for x86-64, x86-32, Armv7, and AArch64 across the Linux,
Windows, macOS, and iOS targets on which those architectures are supported. It
also executes the Rust layout/golden-vector tests and native C fixture on
32-bit i686 Linux in CI. Big-endian targets are outside the currently validated
deployment set; this draft makes no untested big-endian support claim.

### 3.2 C representation and conformance

For a fixed supported process ABI, Rust `#[repr(C)]` records use the target C
ABI's field order, padding, alignment, and aggregate-passing rules, and
`extern "C"` functions use its C calling convention. The normative ABI is the
field order, field types, function signatures, and semantics specified in this
document. Numeric sizes, alignments, and offsets are target-specific
consequences of those declarations, not a second target-independent contract.

Interface ABI hashes protect generated interface contracts, including method
order and lowered method types. They intentionally do not hash the physical
layout of core descriptor records or runtime API tables. Incompatible core
record changes are identified by `NC_COM_CORE_ABI_VERSION`; incompatible
runtime table or exported-function changes are identified by
`NC_COM_RUNTIME_ABI_VERSION`. Section 21 defines when each discriminator must
change.

The native C conformance fixture, or a deliberately handwritten unsafe adapter,
must declare the same fields and function signatures using the corresponding
target C types. core ABI v5 does not generate C headers, and a stale or incorrect
independently maintained C declaration does not make the specified Rust/C
representation ambiguous. Representative native-C calls and targeted layout
assertions are useful regression tests, especially for a changed root record or
table prefix, but they do not expand the supported Rust-to-Rust trust boundary
and conformance does not require an exhaustive duplicate C declaration or
numeric offset matrix for every ABI record. After ratification, the no-mutation
and versioning rules—not duplicated declarations—are the compatibility
authority.

## 4. Core Versioning

The v5 core ABI version is:

```rust
pub const NC_COM_CORE_ABI_VERSION: u32 = 5;
```

This value identifies the current draft binary generation; the number `5` is
not a claim that core ABI v5 has been ratified. Before ratification, hosts and
plugins built from different nc-com source releases are not guaranteed to
interoperate, even when both report this value. After an unreleased draft ABI
change, every host and plugin artifact must be rebuilt together. Once a draft
build has been distributed, a later incompatible binary shape must use a
different ABI discriminator so stale artifacts are rejected rather than
misinterpreted.

No ABI-v5 ratification milestone has occurred. Ratification requires an
explicit status change in this document, a named source tag/release, a changelog
entry identifying the compatibility baseline, and locked golden vectors for
the ratified contract. Until then, breaking draft changes and intentional
golden-vector updates are permitted. After ratification, the evolution rules in
section 21 become compatibility commitments.

Dynamic plugins expose descriptor metadata through this symbol:

```rust
pub const NC_GET_PLUGIN_DESCRIPTOR_SYMBOL: &str = "nc_get_plugin_descriptor";
```

Compatibility is decided by `core_abi_version` and the interface identities
referenced by the descriptor graph. A host must reject a descriptor whose
`core_abi_version` does not equal its supported `NC_COM_CORE_ABI_VERSION` before
interpreting descriptor fields.

An incompatible future core ABI generation must increment
`NC_COM_CORE_ABI_VERSION`. It may also define a new descriptor entrypoint symbol
if the entrypoint function shape or discovery rules change.

## 5. Status Codes

`NcStatus` is the ABI status type:

```rust
#[repr(transparent)]
pub struct NcStatus(i32);
```

Non-negative values are success. Negative values are errors. `NC_OK` is the
only built-in success status.

Built-in status values are:

| Symbol | Code | Meaning |
| --- | ---: | --- |
| `NC_OK` | `0` | Successful ABI status. |
| `NC_E_NOINTERFACE` | `-1` | An object or class does not support the requested interface. |
| `NC_E_POINTER` | `-2` | A required raw pointer argument is null. |
| `NC_E_INVALID_TAG` | `-3` | A tagged ABI enum contains an unknown tag value. |
| `NC_E_INVALID_ARG` | `-4` | A pointer-level valid value is invalid for the operation. |
| `NC_E_ABI_VERSION` | `-5` | A core ABI version or interface ABI hash is incompatible. |
| `NC_E_OUT_OF_RANGE` | `-6` | An index or length is outside its valid range, or a fixed runtime resource capacity is exhausted. |
| `NC_E_NOT_FOUND` | `-7` | A requested registry entity does not exist. |
| `NC_E_INSUFFICIENT_BUFFER` | `-8` | Caller-owned output storage is too small for the current result. |
| `NC_E_LOAD_FAILED` | `-9` | The platform loader cannot load or inspect a plugin library. |
| `NC_E_OUT_OF_MEMORY` | `-10` | A valid, policy-compliant allocation request could not be satisfied by the current allocator. |
| `NC_E_BUSY` | `-11` | A mutable object call overlapped another active call on the same object. |

Status code numbers are stable once published. They must not be renumbered,
reused for different meanings, or silently removed. Future built-in errors must
use previously unused negative values.

Unknown status values must be preserved when forwarding across the ABI. Rust
diagnostic text is not part of binary compatibility.

Rust adapters use `Result<T, NcStatus>` to preserve unknown component errors.
The `Err` variant must contain a negative status. A Rust adapter that receives
`Err(status)` with a non-negative value must return `NC_E_INVALID_ARG` across
the ABI instead of forwarding the invalid success status. Negative values,
including unknown errors, are preserved. core ABI v5 intentionally retains
`NcStatus` rather than introducing a separate negative-only Rust error type;
normalization at each user-controlled `Result`-to-status boundary enforces the
required invariant without changing the C-shaped ABI type.

## 6. GUIDs

`NcGuid` is the ABI identifier type:

```rust
#[repr(C)]
pub struct NcGuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}
```

GUIDs identify interfaces, classes, plugins, and ABI contracts. Every
user-defined plugin, class, and interface GUID is derived from its normative
qualified name. Rust type names, Rust module paths, crate names,
shared-library names, and version strings are not inputs unless the author
deliberately includes them in that qualified name.

The nil GUID is:

```rust
# use nc_com::NcGuid;
pub const NC_GUID_NIL: NcGuid = NcGuid::from_fields(0, 0, 0, [0; 8]);
```

The nil GUID must not be used as a valid plugin ID, class ID, or interface ID.
The all-zero `NcAbiHash` is the separate constructor sentinel for “takes no
arguments.”

External textual GUIDs use canonical UUID format:

```text
xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

`NcGuid::to_uuid_bytes()` and `NcGuid::from_uuid_bytes()` use canonical UUID
byte order:

- `data1` big-endian bytes
- `data2` big-endian bytes
- `data3` big-endian bytes
- `data4` as stored

This external byte order is distinct from the in-memory native-endian layout of
the `repr(C)` struct.

### 6.1 Qualified-name UUIDv5 derivation

The nc-com UUID namespace is:

```text
720d6b32-1f75-4828-a895-6fb73f8202ee
```

For a qualified name `N`, a conforming producer computes the identifier as the
RFC 4122 UUIDv5 value over:

```text
canonical UUID bytes(namespace) || N.as_bytes()
```

The hash operation is SHA-1. The first 16 digest bytes are used, bits 12..15 of
byte 6 are replaced with `0101` (version 5), and the high bits of byte 8 are
replaced with `10` (the RFC variant). `N.as_bytes()` means the exact UTF-8 byte
sequence. There is no Unicode normalization, case folding, trimming, separator
replacement, Rust-path expansion, or terminating NUL byte.

[`NC_COM_GUID_NAMESPACE`](../crates/nc-com-core/src/guid.rs) and the `const`,
allocation-free `guid_from_qualified_name` operation are the authoritative core
implementation. Proc macros and handwritten descriptor producers use the same
operation.

Both macro forms have identical semantics:

```rust
# use nc_com::com_interface;
#[com_interface(auto_guid = "example.Calculator:v1")]
# trait IAutomatic {}

#[com_interface(
    "bfec7c45-902c-5f94-84ec-77df282eb9f6",
    name = "example.Calculator:v1"
)]
# trait IPinned {}
```

The explicit UUID is a checked golden pin, not permission to select an
unrelated identity. Macro expansion rejects an explicit literal, an ID constant,
or an `auto_guid` input that does not match the descriptor qualified name.
Runtime descriptor validation independently recomputes every pair, so foreign
or handwritten descriptors follow the same rule.

Golden vectors are:

| Qualified name | UUIDv5 |
| --- | --- |
| `nc.tests.spec.Interface:v1` | `181a0cda-93b8-57de-96e8-f0bfeca9d583` |
| `nc.tests.spec.Class:class:v1` | `ab2de1f2-cfee-5ac2-b0f4-9c3e6da20f5c` |
| `nc.tests.spec.Plugin:v1` | `d3105b2d-858f-5b6c-a2cb-be35b0e4dde9` |

Within one inspected artifact set or live registry, the same GUID must always
have the same qualified name and the same qualified name must always have the
same GUID. Exact repeated interface pairs are allowed because multiple classes
may implement one interface. Mapping conflicts are rejected during planning or
registration. These checks run only during validation/registration and add no
cost to interface lookup or dispatch.

The conventional COM `IUnknown` identity is the sole built-in identity exception. Its
reserved qualified name is exactly `IUnknown`, and its IID remains the
conventional non-v5 value. User-defined metadata may not claim that reserved
IID/name pair except through the canonical root descriptor.

The v5 built-in COM interface identities are:

| Interface | Qualified name | IID | ABI hash |
| --- | --- | --- | --- |
| `IUnknown` | `IUnknown` | `00000000-0000-0000-c000-000000000046` | `7c076b8636ac223b155de050eb042608093757b7d69d09c0f64ceed0e1f0d84e` |
| `INcBytesAbi` | `nc.com.INcBytes:v1` | `b0b0d6d2-f170-54e9-b6d5-143f9452b951` | `6aa10a90eab4c2bc001f862c4a33321d6002aefbdfa27145fb090b48041d87fc` |
| `INcStringAbi` | `nc.com.INcString:v1` | `01a5b18c-be18-5d39-b71f-202df7c84664` | `855741991f89e09e401430d04d187b0b9be2b7fb7afb449d65ed892e379d3d3a` |

`IUnknown` hashes only its interface IID because it is the root object
contract. Its canonical `NcInterfaceDescriptor` has that IID and hash,
the reserved qualified name, `method_count == 0`, and no method table. The byte and string interface hashes
include their typed vtable methods.

Interface compatibility is represented by `NcInterfaceId`:

```rust,ignore
#[repr(C)]
pub struct NcInterfaceId {
    pub iid: NcGuid,
    pub abi_hash: NcAbiHash,
}
```

`iid` identifies the interface family. `abi_hash` identifies the exact lowered
ABI contract for that interface. `NcInterfaceId::NONE` is the nil GUID plus
zero ABI hash sentinel used where no interface is present.

## 7. ABI-Safe Values

`AbiType` marks values that may appear in an `extern "C"` ABI signature.
`AbiPlainValue` marks durable copyable values with no hidden pointer lifetime,
ownership obligation, or release protocol. `AbiPlainValue` does not mean that
every bit pattern is valid, that arbitrary storage is initialized, or that a
tagged union's tag and active member agree. Consumers must establish those
validity properties before materializing a typed Rust payload.

The built-in v5 plain value set is:

- `bool`
- `u8`, `u16`, `u32`, `u64`, `usize`
- `i8`, `i16`, `i32`, `i64`, `isize`
- `f32`, `f64`
- `NcGuid`
- `NcStatus`
- `NcAbiHash`

`usize` and `isize` are valid only for same-process ABI contracts with matching
pointer width. Public long-lived interfaces should prefer fixed-width integers
unless pointer-sized semantics are intentional.

User-defined plain values may be created by:

- `#[com_struct]`, for fixed-layout `repr(C)` records of plain fields
  with optional macro-owned `packed`, `packed(N)`, or `align(N)` layout;
  `#[com_struct(c_struct)]` permits external C-layout fields without COM traits
- `com_int_enum!`, for transparent integer newtypes that preserve unknown values
- `#[com_enum]`, for tagged payload values with explicit `repr(C)` ABI
  representations

A representative fixed-layout aggregate is declared directly from its ABI
fields:

```rust
use nc_com::com_struct;

#[com_struct]
#[derive(Debug, PartialEq)]
pub struct PacketHeader {
    pub tag: u8,
    pub count: u32,
    pub sequence: u64,
}
```

`#[com_struct]` owns the Rust representation attribute. Authors use
`#[com_struct]`, `#[com_struct(packed)]`, `#[com_struct(packed(N))]`, or
`#[com_struct(align(N))]`; raw `#[repr(...)]` attributes on the input struct are
rejected so the generated layout metadata and Rust representation cannot drift.

By default, every field must implement `AbiPlainValue`. With
`#[com_struct(c_struct)]`, fields need no nc-com traits or macro annotations.
Instead, generated code checks the whole record's FFI layout with rustc's
`improper_ctypes_definitions` lint promoted to an error, including nested field
layouts. An outer C representation does not make a nested Rust-layout struct
C-compatible. `Copy` requirements and syntactic rejection of references,
pointers, and ownership-bearing fields remain in effect. The option can be
combined with the packing or alignment options above.

Plain values must be `Copy + 'static`, must not contain Rust references, raw
pointers, borrowed addresses, implicit allocator ownership, or Rust drop glue,
and must not require cleanup across the ABI boundary.

For `c_struct`, the contract author must ensure those plain-value requirements
also hold inside external types: FFI layout checks cannot prove the absence of
hidden pointers or ownership protocols. Different dependency versions are
compatible only if their actual layouts and value semantics remain compatible.
See the [external-field example](../README.md#external-c-layout-fields).

`#[com_enum]` creates a Rust enum plus an explicit tagged ABI representation.
For an enum named `MyEnum`, the generated physical ABI shape is:

```rust,ignore
#[repr(transparent)]
pub struct MyEnumTag(pub TagTy);

#[repr(C)]
pub struct MyEnumVariantPayload {
    /* payload fields */
}

#[repr(C)]
pub union MyEnumPayload {
    pub variant: core::mem::ManuallyDrop<MyEnumVariantPayload>,
    pub _empty: (),
}

#[repr(C)]
pub struct MyEnumAbi {
    /* private in Rust; physically first */ tag: MyEnumTag,
    /* private in Rust; physically second */ payload: MyEnumPayload,
}
```

Tuple payload fields are named `_0`, `_1`, and so on. Named payload fields keep
their source names. Payload union fields use the ASCII-lowercased variant name;
unit variants use `_empty`. Payload variants whose names fold to the same union
field, or to the reserved `_empty` field, are rejected during macro expansion.
The fields are shown above to specify physical ABI layout. Generated Rust code
represents `MyEnumAbi` as a `repr(transparent)`
wrapper around a core-owned `repr(C)` tag/payload storage pair whose fields are
private to `nc-com-core`. The wrapper has the storage pair's ABI, so opacity does
not change its size, alignment, offsets, ABI hash, or external C shape. Using
core-owned storage also preserves macro use in local item contexts without
making the raw fields accessible to the invoking module.

Safe Rust construction preserves the tag/payload invariant:

- `From<MyEnum>` initializes the union member selected by the known tag.
- `MyEnumAbi::tag()` reads the tag without selecting a payload member.
- `MyEnumAbi::from_unknown_tag(tag)` accepts only an unknown tag and initializes
  the union through `_empty`; conversion returns `NC_E_INVALID_TAG` without
  reading another member.
- `MyEnumAbi::from_raw_parts_unchecked(tag, payload)` is unsafe. For a known tag,
  the corresponding union member must be active, every byte subsequently read
  must be initialized, and every selected field must contain a valid value of
  its Rust type. Nested tagged values obey the same rule.

Safe `TryFrom<MyEnumAbi>` is retained. It returns `NC_E_INVALID_TAG` for an
unknown tag. A malformed known-tag payload is not a recoverable `TryFrom` input:
it can arise only after violating the unsafe raw-construction or FFI producer
contract. Validation cannot safely be implemented by first reading a typed
field such as `bool`, because reading an invalid Rust representation is already
undefined behavior.

Tags default to one-based source order, but published ABI contracts should pin
every tag with `#[com_tag(...)]`.

## 8. Object Model

Every COM object implements `IUnknown`. The v5 `IUnknown` header is:

```rust,ignore
#[repr(C)]
pub struct IUnknown {
    pub vtable: *const IUnknownVTable,
    pub object: *mut c_void,
}
```

The v5 `IUnknown` vtable is:

```rust,ignore
#[repr(C)]
pub struct IUnknownVTable {
    /// Immutable static identity of the actual projected interface.
    pub interface_id: *const NcInterfaceId,

    pub query_interface: unsafe extern "C" fn(
        this: *mut IUnknown,
        interface_id: *const NcInterfaceId,
        out: *mut *mut c_void,
    ) -> NcStatus,

    pub add_ref: unsafe extern "C" fn(this: *mut IUnknown) -> u32,

    pub release: unsafe extern "C" fn(this: *mut IUnknown) -> u32,
}
```

`QueryInterface` rules:

- `this` must be a live borrowed interface pointer with a valid `IUnknown`
  header, vtable, and object-storage back-pointer.
- `interface_id` must point to a readable, initialized `NcInterfaceId` for the
  duration of the call.
- `out` must point to a distinct, aligned pointer slot that is writable for the
  duration of the call. Writing that slot must not invalidate or modify `this`,
  `interface_id`, the vtable, or object storage.
- Given those preconditions, the implementation writes null to `*out` before
  attempting interface lookup.
- `NC_OK` replaces that null with a non-null owned reference for the exact
  requested interface identity. Every other returned status leaves `*out` null
  and transfers no reference.
- Null, dangling, misaligned, aliased, or otherwise invalid arguments violate
  the caller contract. Implementations may reject some such inputs defensively,
  but core ABI v5 specifies no status or output postcondition for an invalid call.
- The returned pointer may have a different address from the input interface
  pointer.
- The call does not consume the borrowed reference represented by `this`.
- The caller must eventually release the returned owned reference.
- Unsupported interface IDs return `NC_E_NOINTERFACE`.
- Supported interface IDs with incompatible ABI hashes return
  `NC_E_ABI_VERSION`.

`AddRef` rules:

- `this` must be a live borrowed interface pointer.
- Success creates one additional owned reference.
- The returned `u32` count is diagnostic only.

`Release` rules:

- `this` must own one live reference.
- The call consumes that reference.
- When the remaining count reaches zero, the object is destroyed immediately.
- Final release cannot return a fallible cleanup status, defer destruction, or
  resurrect the object.
- The returned `u32` count is diagnostic only.

Generated nc-com objects use atomic `u32` reference counts. Debug builds assert
on overflow or release-after-zero. Release builds avoid wrapping: `AddRef`
saturates at `u32::MAX`, and `Release` refuses to underflow. Reaching either
edge is still a broken ownership contract.

Any live interface pointer passed to a vtable function must point to a readable
interface header whose `vtable` points to a vtable with the declared shape. Each
vtable function pointer must be valid for the duration of the call. The
`object` field is opaque, but it must be valid for the implementation behind
that vtable. Generated nc-com objects store a non-null object storage pointer
and return `NC_E_POINTER` defensively from fallible methods when the storage
pointer is null.

## 9. Generated Interface Layout

A generated interface named `IFoo` produces an ABI header named `IFooAbi`:

```rust,ignore
#[repr(C)]
pub struct IFooAbi {
    pub vtable: *const IFooVTable,
    pub object: *mut c_void,
}
```

A root interface vtable begins with the `IUnknown` prefix:

```rust,ignore
#[repr(C)]
pub struct IFooVTable {
    pub unknown: IUnknownVTable,
    /* interface-specific methods in source order */
}
```

The first two fields of every generated interface header have the same binary
shape as `IUnknown`. The interface vtable pointer points at a vtable whose first
bytes form the `IUnknownVTable` prefix, directly or through inherited bases. This is what makes the generated
`ComInterface::iunknown()` cast valid.

In the Rust binding, `ComInterface` is an unsealed unsafe trait. Implementing it
asserts that the Rust interface header and vtable have the shapes above, that
the advertised interface ID and ABI hash describe the actual vtable contract,
that live interface pointers retain their header, vtable, and object storage,
and that successful `QueryInterface` results have the required owned-reference
semantics. `ComInterface::iunknown()` must not dereference its input, must map
null to null, and must map every live interface pointer to the live `IUnknown`
header for the same object and reference-count identity.

`#[com_interface]` is the supported safe implementation path and generates this
unsafe implementation from the same interface definition used to generate the
header, vtable, metadata, and method thunks. The trait remains unsealed so a
low-level foreign-interface adapter can be written, but such an adapter must use
an explicit handwritten `unsafe impl` and assumes the entire contract above.
A handwritten safe `impl` cannot provide a `ComInterface` implementation. This
Rust safety boundary does not change the C-shaped binary layout.

Sealing is deliberately not used as the enforcement mechanism. An attribute
procedural macro has no special access to private items in the crate that defines
`ComInterface`; its generated implementation is ordinary code compiled in the
consumer crate. A genuinely private sealing supertrait would therefore reject
the `unsafe impl` generated by `#[com_interface]` as well as a handwritten one.
Making the sealing helper public but documentation-hidden would let the macro
name it, but would also let any consumer implement it, so it would not be a
soundness boundary. The unsafe-trait boundary provides the required rule without
blocking macro-generated interfaces: safe handwritten Rust cannot assert the
contract, while the macro and low-level adapters must cross an explicit audited
`unsafe impl` boundary.

Interface casts must be performed through `QueryInterface`, not by pointer
arithmetic or direct Rust casts. A single generated object stores one header per
exposed interface, and those headers may have different addresses.

### 9.1 Single-base interface inheritance

A generated trait may extend exactly one other generated interface. The Rust
trait requires the implementing class to implement that base; the generated ABI
places the entire base vtable at byte offset zero and appends only local slots.
This applies recursively and across contract crates. Use a qualified module
path when referring to a base declared in another module or crate.

```rust
use nc_com::{com_interface, com_object, ComPtr, NcStatus};
#[com_interface(auto_guid = "nc.tests.spec.Widget:v1")]
pub trait IWidget {
    fn value(&self) -> Result<u32, NcStatus>;
}
#[com_interface(auto_guid = "nc.tests.spec.Button:v1")]
pub trait IButton: IWidget {
    fn submit(&self) -> Result<(), NcStatus>;
}
#[com_object(interfaces(IButton))]
pub struct Button;
impl IWidget for Button {
    fn value(&self) -> Result<u32, NcStatus> { Ok(7) }
}
impl IButton for Button {
    fn submit(&self) -> Result<(), NcStatus> { Ok(()) }
}
let button: ComPtr<IButtonAbi> = Button::create().query().unwrap();
assert_eq!(button.value().unwrap(), 7); // inherited call, no QueryInterface
button.as_com_ref().submit().unwrap();
let widget: ComPtr<IWidgetAbi> = button.query().unwrap(); // owned base projection
assert_eq!(widget.value().unwrap(), 7);
```

`IButtonVTable` starts with `base: IWidgetVTable`, followed by `submit`.
Every class automatically exposes every ancestor. Class metadata deduplicates
repeated base identities; conflicting hashes for a repeated IID are rejected.
All projections retain one allocation, one reference count, and one canonical
IUnknown identity. Interface inheritance does **not** duplicate or inherit Rust
struct fields: implementations compose shared concrete state into their class.

Generated `IFooClient<M>` views support inherited method lookup through `Deref`.
These transparent, non-owning views only lend synchronous calls; obtaining a
view performs no allocation, QueryInterface, AddRef, or Release. Views cannot
outlive the lending `ComPtr` or `ComRef`, and preserve its threading restriction.
The optional unsafe `ComClientInterface` contract describes this Rust-only
mechanism; ordinary handwritten `ComInterface` adapters remain valid without it.
Local method names take precedence over an identically named inherited method.
Authors should use distinct method names to keep client calls unambiguous.

A derived header advertises the **derived** full identity, including when a base
method is called through its vtable prefix. An inherited view is therefore not
a checked base COM reference. To pass a `ComRef<IWidgetAbi>` argument, first
query the base projection and borrow it; do not cast or adopt the derived raw
pointer as an independently identified base interface. QueryInterface returns a
separate header with the base identity and transfers one owned reference.

Descriptor `base` stores the full explicit base identity, or `NONE` when there
is only the implicit IUnknown prefix. Base metadata must exist in the same
class's interface table, match its full identity, and form an acyclic chain.
`methods` stores local slots only. Inspection preserves `base` and local method
indices instead of flattening away the physical prefix boundary.

Hash grammar v2 gains tag `0x0a`: immediately after the derived Interface record,
encode its base IID and 32 hash bytes, then encode local method records indexed
from zero. This makes changes to any inherited ABI invalidate derived contracts.
Pointer-valued method references still encode only their IID, so a base method
may return its derived interface without creating a recursive hash. Root
interface hashes and the v2 hash domain remain unchanged. The descriptor layout
change requires **core ABI 3**; every host, runtime and plugin must be rebuilt.
Runtime API table version remains 1 and checks the core version before use.

## 10. Interface Method Lowering

A COM interface trait method uses one of these Rust shapes:

```rust,ignore
fn method(&self, args...) -> Result<T, NcStatus>;
fn mutate(&mut self, args...) -> Result<T, NcStatus>;
```

`&self` permits overlapping shared dispatch. `&mut self` requests exclusive
access to the stored implementation across every interface projection and
cloned pointer. Generated object storage returns `NC_E_BUSY` instead of creating
an overlapping Rust borrow when a conflicting call or reentrant callback is
already active.

The call-scoped buffer forms are complete interface inputs and compile as part
of this specification:

```rust
use nc_com::{
    NcCopyFromBufferOffset, NcCopyToBufferOffset, NcStatus, com_interface,
};

#[com_interface(auto_guid = "nc.tests.spec.Buffer:v1")]
pub trait IBuffer {
    fn write(&mut self, input: NcCopyFromBufferOffset<'_, u8>) -> Result<(), NcStatus>;
    fn read(&self, output: NcCopyToBufferOffset<'_, u8>) -> Result<(), NcStatus>;
}
```

Unsupported interface trait shapes are rejected by the macro:

- unsafe traits
- auto traits
- generic traits
- where clauses on the trait
- multiple supertraits, generic bases, or bases other than generated COM interfaces
- non-method trait items
- default method bodies
- `const`, `async`, `unsafe`, `extern`, variadic, or generic methods
- method where clauses
- receivers other than plain `&self` or `&mut self`
- argument patterns other than plain identifiers
- methods named `query_interface`, `add_ref`, or `release`, which collide with
  the generated `IUnknown` lifecycle thunks
- parameter names that would duplicate a lowered raw buffer slot or shadow a
  generated adapter binding
- return types other than `Result<T, NcStatus>`

Plain arguments lower by value:

```rust,ignore
fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
```

lowers to:

```rust,ignore
unsafe extern "C" fn(
    this: *mut ICalculatorAbi,
    a: i32,
    b: i32,
    out: *mut i32,
) -> NcStatus
```

Void success lowers without an output pointer:

```rust,ignore
fn flush(&self) -> Result<(), NcStatus>;
```

lowers to:

```rust,ignore
unsafe extern "C" fn(this: *mut IFlushableAbi) -> NcStatus
```

Borrowed COM parameters use `ComRef<'_, InterfaceAbi>` or
`ComRef<'_, InterfaceAbi, ThreadingMode>`. The anonymous `'_` lifetime is
required: named lifetimes, including `'static`, are rejected by the interface
macro. They lower to raw interface pointers valid for the current call:

```rust,ignore
fn append(&self, node: ComRef<'_, INodeAbi>) -> Result<(), NcStatus>;
```

lowers to a parameter of type:

```rust,ignore
*mut INodeAbi
```

The safe wrapper's borrow cannot escape the generated trait call. The callee
must call `AddRef` before retaining a borrowed argument after the call returns.
The interface macro rejects a `'static` call-scoped borrow:

```rust,compile_fail
use nc_com::{ComRef, NcStatus, com_interface};

#[com_interface(auto_guid = "nc.tests.spec.Node:v1")]
pub trait INode {}

#[com_interface(auto_guid = "nc.tests.spec.InvalidBorrow:v1")]
pub trait IInvalidBorrow {
    fn retain(&self, node: ComRef<'static, INodeAbi>) -> Result<(), NcStatus>;
}
```

Owned COM returns use `ComPtr<InterfaceAbi>` or
`ComPtr<InterfaceAbi, ThreadingMode>`. They lower to raw interface pointers
returned through `out`:

```rust,ignore
fn make_node(&self) -> Result<ComPtr<INodeAbi>, NcStatus>;
```

lowers to:

```rust,ignore
unsafe extern "C" fn(
    this: *mut IFactoryAbi,
    out: *mut *mut INodeAbi,
) -> NcStatus
```

On success, the output pointer owns one COM reference. The caller must release
it.

Generated method thunks reject null `this` with `NC_E_POINTER`. Non-void method
thunks reject null `out` with `NC_E_POINTER`. Borrowed COM argument thunks reject
null borrowed interface pointers with `NC_E_POINTER`.

For non-void methods, `out` is initialized only when the returned status is
successful. On method failure, the content of `out` is unspecified.

For generated Rust implementations, `Ok(value)` writes the output and returns
`NC_OK`. `Err(status)` returns the same status when it is negative. An invalid
non-negative status carried by `Err` is normalized to `NC_E_INVALID_ARG`, and
the output slot is not written. The same normalization applies to status-only
void methods.

## 11. ABI Hashes

`NcAbiHash` is a SHA-256 digest:

```rust
#[repr(C)]
pub struct NcAbiHash {
    pub bytes: [u8; 32],
}
```

`NcAbiHash::ZERO` is an all-zero sentinel and is invalid for registered
interface metadata.

Hash inputs are built from typed `NcAbiHashPart` values:

```rust,ignore
pub enum NcAbiHashPart<'a> {
    Interface { iid: NcGuid },
    BaseInterface { interface: NcInterfaceId },
    Method { index: u32, name: &'a str },
    Receiver { receiver: NcAbiReceiver },
    Arg { index: u32, passing: NcAbiPassing, ty: NcAbiType },
    Return { passing: NcAbiPassing, ty: NcAbiType },
    Status,
    Layout { size: usize, align: usize },
    Field { index: u32, offset: usize, ty: NcAbiType },
    Variant { index: u32, tag: i128, ty: NcAbiType },
    Aggregate { kind: NcAbiAggregateKind },
}

pub enum NcAbiType {
    Void,
    Bool, U8, U16, U32, U64, USize,
    I8, I16, I32, I64, ISize,
    F32, F64,
    NcGuid,
    NcStatus,
    NcAbiHash,
    Aggregate(NcAbiHash),
    Interface { iid: NcGuid },
}

pub enum NcAbiPassing {
    Void,
    Value,
    Out,
    CopyFromBufferOffset,
    CopyToBufferOffset,
    ComRef { mode: NcAbiThreadingMode },
    ComPtrOut { mode: NcAbiThreadingMode },
}
```

The digest algorithm is SHA-256 over:

```text
"nc-com:abi-hash:v2\0"
typed_part_0
typed_part_1
...
```

The hash grammar is binary and canonical:

- integer fields are big-endian
- signed `i128` variant tags are encoded as 16-byte big-endian two's-complement
  integers
- layout `usize` values are encoded as unsigned 64-bit big-endian integers
- strings are `u32` byte length followed by UTF-8 bytes
- GUIDs are `NcGuid::to_uuid_bytes()`
- nested hashes are the 32 digest bytes directly

An interface identity is:

- interface ID (`IID`)
- interface `ABI_HASH`

The local interface hash includes each referenced IID and its passing/threading
mode. It does not recursively include the referenced interface hash. Generated
thunks check borrowed arguments against the callee's expected full identity;
generated callers check owned returns against the caller's expected full identity
before constructing a typed pointer. Rejected owned returns are released exactly
once through the common prefix; rejected borrowed arguments transfer no ownership.
QueryInterface and constructor results receive the same owned-pointer check.
This permits self-references and mutually recursive contracts without weakening
checks of actual pointers. Descriptor metadata still records full target hashes.

`interface_id` describes the actual projected interface using the vtable, not
necessarily IUnknown. It points to immutable static metadata. Checking it performs
no AddRef/Release. The loader must validate core ABI generation 3 before accepting
descriptors or object pointers: generation 1 vtables lack this identity field, and
generation 2 descriptors lack the explicit interface-base metadata. The current
header still contains two pointers; projected identity belongs to its vtable. Raw callers remain
responsible for live, valid common headers/vtables; these checks cannot make an
arbitrary address safe. Threading promises remain the producer's obligation.

Together these fields form `NcInterfaceId`. The GUID alone is not enough to
prove compatibility because a pre-alpha interface may keep the same family ID
while its lowered ABI hash changes.

The hash input grammar is itself ABI policy. Changing this grammar is an
ABI-policy change even if the physical vtable layout would otherwise be the
same.

Typed part tags:

```text
0x01 Interface: guid
0x02 Method: u32 index, str name
0x03 Arg: u32 index, passing, type
0x04 Return: passing, type
0x05 Status
0x06 Layout: usize size, usize align
0x07 Field: u32 index, usize offset, type
0x08 Variant: u32 index, i128 tag, type
0x09 Aggregate: aggregate_kind
0x0a BaseInterface: guid iid, 32 raw hash bytes
0x0b Receiver: receiver_mode
```

Receiver modes:

```text
0x00 Shared
0x01 Mutable
```

Type tags:

```text
0x00 Void
0x01 Bool
0x02 U8
0x03 U16
0x04 U32
0x05 U64
0x06 USize
0x07 I8
0x08 I16
0x09 I32
0x0a I64
0x0b ISize
0x0c F32
0x0d F64
0x0e NcGuid
0x0f NcStatus
0x10 NcAbiHash
0x11 Aggregate: hash
0x12 Interface: guid
```

Passing tags:

```text
0x00 Void
0x01 Value
0x02 Out
0x03 CopyFromBufferOffset
0x04 CopyToBufferOffset
0x05 ComRef: threading_mode
0x06 ComPtrOut: threading_mode
```

`NcAbiThreadingMode::{ThreadLocal, ThreadSafe}` encodes as `0x00`, `0x01`.
`NcAbiAggregateKind::{Struct, Union, TaggedEnum}` encodes as `0x00`, `0x01`,
`0x02`.
`Bool` in the type table is an ABI type tag; no runtime Boolean value is
serialized into an interface or aggregate hash.

`CopyFromBufferOffset` and `CopyToBufferOffset` are logical ABI argument forms
with fixed lowered shapes. The Rust macro surface exposes them as
`NcCopyFromBufferOffset<'_, T>` and `NcCopyToBufferOffset<'_, T>` parameters.
The anonymous `'_` lifetime is required in interface methods; named lifetimes,
including `'static`, are rejected so the generated slice cannot escape the
call. They are not generic raw pointer descriptors:

```text
CopyFromBufferOffset<T> => offset: u64, len: u64, input: *const T
CopyToBufferOffset<T>   => offset: u64, len: u64, out: *mut T
```

For zero-length buffers, the lowered pointer may be null. For nonzero lengths,
the pointer must be non-null and valid for exactly `len` contiguous `T` elements.
`CopyFromBufferOffset` requires readable storage. `CopyToBufferOffset` requires
writable storage.

The lowered `len` must fit in the host Rust `usize`; generated wrappers return
`NC_E_OUT_OF_RANGE` otherwise. For nonzero buffers, the lowered pointer must be
properly aligned for `T` and valid for the duration of the call. Writable output
buffers must be exclusively writable for the duration of the call. The `offset`
is a logical method/object-specific offset unless the interface contract says
otherwise. Generated wrappers validate only length representability and
null-for-nonzero before constructing Rust slices; non-null pointer readability,
writability, alignment, and aliasing remain raw ABI caller obligations.

Generated interface hash parts:

```text
Interface { iid }
method_abi_parts(method 0)
method_abi_parts(method 1)
...
```

Generated method ABI parts:

```text
Method { index, name }
Receiver { receiver }
Arg { index: 0, ... }
Arg { index: 1, ... }
...
Return { ... }
Status
```

Plain arguments use:

```text
Arg { index, passing: Value, ty: <T as AbiType>::ABI_TYPE }
```

Borrowed COM arguments use:

```text
Arg {
    index,
    passing: ComRef { mode },
    ty: Interface { iid: referenced IID },
}
```

Void returns use:

```text
Return { passing: Void, ty: Void }
```

Plain success returns use an out pointer:

```text
Return { passing: Out, ty: <T as AbiType>::ABI_TYPE }
```

Owned COM returns use an out pointer that transfers one COM reference:

```text
Return {
    passing: ComPtrOut { mode },
    ty: Interface { iid: referenced IID },
}
```

Generated `#[com_struct]` types implement `ComStruct` with compile-time physical
layout metadata and implement `AbiType` as
`Aggregate(hash(Aggregate { Struct }, Layout { size, align }, Field { index, offset, ty }...))`.

In the default mode, each field's `ty` is its `AbiPlainValue` ABI type. With
`#[com_struct(c_struct)]`, every field's `ty` is instead
`Aggregate(hash(Layout { size: size_of::<Field>(), align: align_of::<Field>() }))`.
The enclosing record still hashes its size, alignment, field order, and offsets.
The field fingerprint does not describe internal fields or distinguish scalar
types of equal size and alignment. An incompatible change invisible to this
metadata requires a new interface identity; matching hashes alone cannot detect
it. These layout checks and fingerprints are computed at compile time.

Generated `#[com_enum]` ABI representations hash their payload structs, payload
union, tagged representation layout, tag field, payload field, and ordered
variant tag/type records using `Aggregate { Struct }`, `Aggregate { Union }`, and
`Aggregate { TaggedEnum }`.

The producer uses this physical layout metadata to calculate the aggregate
hash. Runtime method descriptors intentionally retain only that hash, not the
layout graph that produced it. A validator treats a nonzero aggregate hash as
the declared aggregate identity and incorporates it into the enclosing
interface hash; it does not and cannot reverse the digest into a type
definition. `ComStruct::ABI_STRUCT_LAYOUT` is producer-side compile-time
metadata and is not reachable from `NcPluginDescriptor`.

Source spelling, comments, and diagnostic signature strings are not ABI hash
inputs. The hash records canonical method names, receiver mode, argument order, passing mode,
value type identity, buffer direction and nullability when represented, COM
pointer threading mode, aggregate size/alignment/field offsets, and referenced
interface IID/hash.

Typed method signature text is diagnostic metadata rendered from
`NcMethodDescriptor` typed parameter and return descriptors. It currently uses
this form:

```text
{method}([&mut self, ]{name}: value<{type}>, {name}: com_ref<{mode}, {interface}>, {name}: copy_to_buffer_offset<{type}>, ...) -> {return} status
```

Return forms are `void`, `out<{type}>`, and
`com_ptr_out<{mode}, {interface}>`. `copy_from_buffer_offset<{type}>` and
`copy_to_buffer_offset<{type}>` describe logical safe buffer parameters; their
physical vtable lowering remains the fixed offset/length/pointer form described
above. Compatibility authority remains the IID plus ABI hash, not the
diagnostic signature string alone.

Conforming interface descriptors must advertise the ABI hash produced by this
typed metadata. The default descriptor validator recomputes the interface hash
from `NcMethodDescriptor` parameter and return metadata and rejects a descriptor
whose advertised `abi_hash` does not match.

## 12. Descriptor Layouts

All public descriptor structs are `repr(C)` exact v5 layouts. They do not carry
per-descriptor `size` fields. Consumers interpret descriptor fields only after
the plugin `core_abi_version` matches the host's supported
`NC_COM_CORE_ABI_VERSION`.

Existing v5 fields must not be reordered, removed, appended to, or
reinterpreted without a core ABI break.

Fields declared as `Option<extern "C" fn>` use the nullable function-pointer
representation: `None` is a null C function pointer and `Some(f)` is the direct
C function pointer. This rule applies to constructors and debug callbacks.

### 12.1 `NcStaticUtf8`

```rust,ignore
#[repr(C)]
pub struct NcStaticUtf8 {
    pub data: *const u8,
    pub len: u32,
}
```

If `len == 0`, `data` may be null and the value represents an empty string. If
`len != 0`, `data` must be non-null and point to `len` bytes that remain valid
while the descriptor is visible to the host.

Conforming producer metadata that is specified as text must be valid UTF-8.
The core descriptor validator checks pointer/count consistency and UTF-8 before
metadata is accepted. It also applies these v5 metadata policies:

- Plugin, class, and interface qualified names contain `1..=1024` bytes.
- A qualified name is a sequence of nonempty segments separated by `.` or `:`.
- Each segment starts with ASCII `A..Z`, `a..z`, or `_`.
- Remaining segment bytes are ASCII letters, digits, `_`, or `-`.
- Plugin and class version text contains `1..=128` ASCII graphic bytes; spaces
  and control characters are not allowed.

`nc-com-core` exposes the allocation-free const functions
`validate_qualified_name` and `validate_version_text` as the shared reference
implementation of this policy. The interface, class, and plugin proc macros
apply those functions to descriptor literals during macro expansion. Direct
descriptor producers remain responsible for validating their own metadata;
runtime graph validation applies the same shared functions before accepting it.

Qualified names are normative binary identity inputs under section 6.1.
Versions and method/parameter names remain diagnostic metadata. Metadata that
violates this policy is rejected with `NC_E_INVALID_ARG`.

### 12.2 Typed Method ABI Descriptors

```rust,ignore
#[repr(C)]
pub struct NcAbiTypeDescriptor {
    pub tag: u32,
    pub aggregate_hash: NcAbiHash,
    pub interface: NcInterfaceId,
}

#[repr(C)]
pub struct NcAbiPassingDescriptor {
    pub tag: u32,
    pub threading: u32,
}

#[repr(C)]
pub struct NcMethodParamDescriptor {
    pub name: NcStaticUtf8,
    pub passing: NcAbiPassingDescriptor,
    pub ty: NcAbiTypeDescriptor,
}

#[repr(C)]
pub struct NcMethodReturnDescriptor {
    pub passing: NcAbiPassingDescriptor,
    pub ty: NcAbiTypeDescriptor,
}
```

`NcAbiTypeDescriptor::tag` uses the ABI type tags from section 11.
`aggregate_hash` is meaningful only for `Aggregate`. `interface` is meaningful
only for `Interface`.
`NcAbiPassingDescriptor::tag` uses the ABI passing tags from section 11.
`threading` is meaningful only for `ComRef` and `ComPtrOut`; non-COM passing
modes must use `ThreadLocal`.

Valid parameter combinations are:

```text
Value, CopyFromBufferOffset, CopyToBufferOffset + non-Void, non-Interface type
ComRef                                           + Interface type
```

Valid success-return combinations are:

```text
Void      + Void type
Out       + non-Void, non-Interface type
ComPtrOut + Interface type
```

An `Aggregate` type must carry a nonzero aggregate hash. An `Interface` type
must carry a non-nil IID and nonzero ABI hash. Other combinations are rejected
with `NC_E_INVALID_ARG`.

Parameter order is the array order in `NcMethodDescriptor::params`. Parameter
names are diagnostic metadata only. Buffer parameters are one logical parameter
descriptor with `CopyFromBufferOffset` or `CopyToBufferOffset` passing, even
though the physical vtable slot lowers to offset/length/pointer arguments.

### 12.3 `NcMethodDescriptor`

```rust,ignore
#[repr(C)]
pub struct NcMethodDescriptor {
    pub name: NcStaticUtf8,
    pub receiver: u32,
    pub param_count: u32,
    pub params: *const NcMethodParamDescriptor,
    pub returns: NcMethodReturnDescriptor,
}
```

`receiver` is `NC_ABI_RECEIVER_SHARED` for `&self` and
`NC_ABI_RECEIVER_MUTABLE` for `&mut self`; other values are invalid. Method
order is the array order in `NcInterfaceDescriptor::methods`, after the
complete inherited vtable prefix (or IUnknown for root interfaces). If `param_count == 0`, `params` may be null. If
`param_count != 0`, `params` must point to `param_count` readable parameter
descriptors that remain valid while the descriptor is visible. `returns` stores
the typed success-return descriptor. Inspection tools render human-readable
typed signatures from these structured fields. Interface compatibility is the
interface descriptor's `abi_hash`, not a per-method hash.

The Rust `format_method_signature` helper is an unsafe raw-descriptor operation.
Its caller must establish that the method descriptor and every transitive
pointer/count pair are readable and immutable for the call. Structural checks
can reject null/count mismatches and invalid UTF-8, but cannot prove that an
arbitrary non-null address is readable. Macro-generated static descriptors meet
this contract. Descriptors accepted from a dynamically loaded plugin meet it
after the unsafe loading/validation boundary, and section 16 guarantees that a
successfully loaded plugin remains mapped until process exit. No metadata copy,
module pin, or separate lifetime token is required for registered descriptors.

### 12.4 `NcInterfaceDescriptor`

```rust,ignore
#[repr(C)]
pub struct NcInterfaceDescriptor {
    pub interface_id: NcGuid,
    pub qualified_name: NcStaticUtf8,
    pub abi_hash: NcAbiHash,
    pub base: NcInterfaceId,
    pub method_count: u32,
    pub methods: *const NcMethodDescriptor,
}
```

If `method_count == 0`, `methods` may be null. If `method_count != 0`, `methods`
must point to `method_count` readable descriptors that remain valid while the
descriptor is visible.

Registered metadata must use a valid normative `qualified_name`, its derived
non-nil `interface_id`, and a nonzero `abi_hash`.
The `abi_hash` must match the canonical hash computed from the descriptor's
typed method metadata and its explicit `base` identity, when present. On a
64-bit target this record is 128 bytes (base at 64, method count at 112, methods
at 120); on a 32-bit target it is 112 bytes (base at 56, count at 104, methods at
108). The inheritance rules in section 9.1 apply to every producer.

### 12.5 `NcConstructorFn`

```rust,ignore
pub type NcConstructorFn = unsafe extern "C" fn(
    class: *const NcClassDescriptor,
    arguments: *const c_void,
    out: *mut *mut IUnknown,
) -> NcStatus;
```

The constructor receives:

- the class descriptor that owns the constructor
- a call-scoped pointer to the immutable constructor frame, or null for no arguments
- an output slot for an owned `IUnknown` reference

The frame pointer is not self-describing. The caller host passes the frame's ABI
hash separately, and the runtime must match it against
`NcClassDescriptor::constructor_abi_hash` before calling this function.

`out` must be non-null and valid for one pointer write; a constructor called
with a null `out` must fail with `NC_E_POINTER`. On success, `out` receives one
owned `IUnknown` reference. The caller must eventually release it. On failure
after validating non-null `out`, conforming constructors must leave `*out` null.

### 12.6 `NcClassInterfaceDescriptor`

```rust,ignore
#[repr(C)]
pub struct NcClassInterfaceDescriptor {
    pub interface_descriptor: *const NcInterfaceDescriptor,
}
```

`interface_descriptor` must be non-null and must point to a valid interface
descriptor. The implemented interface ID is derived from the pointed
`NcInterfaceDescriptor`; it is not repeated in the class-interface descriptor.
Each class-interface table contains distinct interface IIDs.

### 12.7 `NcClassDescriptor`

```rust,ignore
#[repr(C)]
pub struct NcClassDescriptor {
    pub class_id: NcGuid,
    pub qualified_name: NcStaticUtf8,
    pub class_version: NcStaticUtf8,
    pub constructor_abi_hash: NcAbiHash,
    pub threading: u32,
    pub construct: Option<NcConstructorFn>,
    pub interface_count: u32,
    pub interfaces: *const NcClassInterfaceDescriptor,
}
```

`threading` is exactly one of `NC_ABI_THREADING_THREAD_LOCAL` or
`NC_ABI_THREADING_THREAD_SAFE`. Any other value makes the descriptor invalid.
It describes the strongest pointer capability that the constructor may return:
a thread-safe class may be consumed through a thread-local pointer as a
conservative downgrade, but a thread-local class must never be upgraded to a
thread-safe pointer.

`qualified_name` is normative and `class_id` must equal its section 6.1 UUIDv5
derivation. `class_id` must be non-nil and `construct` must be present. Every class implements
the root object contract, so every class-interface table must contain exactly
one canonical `IUnknown` descriptor. Consequently a valid class has
`interface_count >= 1`, and `interfaces` must point to `interface_count`
readable descriptors that remain valid while the class descriptor is visible.

`#[com_object(interfaces(...))]` treats `IUnknown` as implicit: users list only
non-root interfaces, and the macro rejects both an explicit `IUnknown` entry and
duplicate interface entries. Listed paths must have distinct final names because
those names determine generated fields and companion ABI types. Generated
tables place `IUnknown` first, followed by the listed interfaces in source
declaration order. This order is observable enumeration metadata but is not
part of an interface ABI hash; the core ABI defines no class-interface-set signature or
hash. Handwritten descriptor producers may use another order, but must still
provide exactly one canonical `IUnknown` and no duplicate IID.

For a constructor with parameters, `constructor_abi_hash` is
`NcAbiHash::from_abi_parts` over one `NcAbiHashPart::Arg` per logical parameter
in declaration order. Each part records its zero-based index, passing mode, and
ABI type according to sections 7, 10, and 11. Parameter names and physical
padding are not hash inputs. A constructor with no parameters uses
`NcAbiHash::ZERO` instead of hashing an empty part list.

`constructor_abi_hash == NcAbiHash::ZERO` means the constructor takes no
arguments and its `arguments` pointer must be null. A nonzero hash identifies
the complete ordered parameter contract of an immutable constructor frame, and
its `arguments` pointer must be non-null. The frame and any nested borrows remain
valid only for the constructor call. A successful constructor must return an object satisfying the
class descriptor's threading promise. This is an unsafe producer contract:
structural validation can validate the tag but cannot prove the behavior of
plugin code. `#[com_object(thread_safe, ...)]` is the supported generated path and
requires the Rust class to be `Send + Sync` in addition to the documented
concurrent-call and destruction contract.

### 12.8 `NcClassTable`

```rust,ignore
#[repr(C)]
pub struct NcClassTable {
    pub class_count: u32,
    pub classes: *const NcClassDescriptor,
}
```

If `class_count == 0`, `classes` may be null. If `class_count != 0`, `classes`
must point to `class_count` readable class descriptors that remain valid while
the plugin is registered.

### 12.9 `NcPluginDependencyDescriptor`

```rust,ignore
#[repr(C)]
pub struct NcPluginDependencyDescriptor {
    pub plugin_id: NcGuid,
}
```

Each entry declares one hard nc-com plugin dependency. `plugin_id` must be
non-nil, must not equal the containing plugin's ID, and must not be repeated in
the same dependency table. Dependencies are activation-order metadata only:
they do not name a path, version range, package, class, or interface, and they
never cause recursive loading. The named plugin must already be registered when
the dependent descriptor is registered.

Dependency declaration order has no semantic meaning. Inspection and planning
tools canonicalize graph output independently of descriptor order.

### 12.10 `NcPluginDescriptor`

```rust,ignore
#[repr(C)]
pub struct NcPluginDescriptor {
    pub core_abi_version: u32,
    pub plugin_id: NcGuid,
    pub qualified_name: NcStaticUtf8,
    pub plugin_version: NcStaticUtf8,
    pub dependency_count: u32,
    pub dependencies: *const NcPluginDependencyDescriptor,
    pub class_table: *const NcClassTable,
}
```

`core_abi_version` must equal `NC_COM_CORE_ABI_VERSION` for this draft. `plugin_id` must
equal the UUIDv5 derivation of `qualified_name` and be non-nil. Every class ID
and non-root interface ID in the graph follows the same rule. If
`dependency_count == 0`, `dependencies` may be null. Otherwise it
must point to `dependency_count` readable descriptors. `class_table` must be
non-null.

Descriptor pointer fields must remain valid while the plugin is registered and
while any plugin-created objects or factories may call plugin code.

Plugin descriptors carry explicit hard dependency plugin IDs. core ABI v5 does not
search for, resolve paths for, or recursively load those dependencies. An
inspector can combine descriptors from an application-selected artifact set,
reject duplicate IDs, missing dependencies, and cycles, and produce a
deterministic dependency-first load order. The live runtime only verifies that
every declared direct dependency is already registered; absence returns
`NC_E_NOT_FOUND` from fallible static registration and is fatal during dynamic
activation.

A native shared-library dependency mapped automatically by an operating-system
loader is not thereby registered as an nc-com plugin. Native loader dependencies
and declarative nc-com dependencies are distinct.

nc-com live activation calls must not be nested. While a host call to
`load_plugin_library_utf8` is in progress, code reached through native loader
initialization or the plugin descriptor entrypoint must not directly or
indirectly call nc-com live activation again. This is a trusted-plugin
precondition, not a dependency-resolution mechanism. It applies whether the
nested call targets the same library or a different one. core ABI v5 implementations
are not required to detect or diagnose a violation. Automatic mapping of ordinary
native dependencies by the operating-system loader is permitted and is not a
nested nc-com activation; those libraries remain outside the nc-com registry
unless the host activates their descriptors in separate calls.

Required external contracts must be represented in the ABI surface itself:
constructor arguments, method arguments, return values, dependency interfaces,
or an application-defined capability interface. The relevant interface IDs and
ABI hashes are the binary compatibility contract. Plugin dependency IDs define
activation order; they do not replace interface compatibility checks.

## 13. Plugin Descriptor Entrypoint

The Rust `plugin!` macro only generates descriptors. A final dynamic-library
root invokes `nc_com::export_plugin!()` (or passes an explicit descriptor function
path). Static plugin assemblies omit that export and register the same metadata.

A dynamic v5 plugin exports:

```rust,ignore
pub unsafe extern "C" fn nc_get_plugin_descriptor(
    out: *mut *const NcPluginDescriptor,
) -> NcStatus;
```

Entrypoint rules:

- `out` must be non-null.
- The function should clear `*out` before fallible validation.
- On success, `out` receives a non-null pointer to immutable plugin metadata.
- The metadata must remain valid while the shared library is loaded.
- The metadata must describe core ABI 3 when returned from the descriptor
  symbol.
- The entrypoint is a declarative metadata accessor. It must not call nc-com
  registration or loading APIs, recursively load declared nc-com dependencies,
  construct plugin objects, or perform plugin initialization. Normal native
  dependencies mapped by the operating-system loader remain outside this rule.

The helper `return_plugin_descriptor` performs full descriptor validation,
including pointer/count shape, UTF-8 and naming policy, identity uniqueness,
typed method metadata, and recomputed interface hashes. It writes the descriptor
pointer only on success.

Descriptor validation remains available in the allocation-free `nc-com-core`
configuration. When `alloc` is enabled, the reference implementation uses
`alloc::collections::BTreeSet` for duplicate dependency, class, and per-class
interface detection; without `alloc` it uses an allocation-free prefix scan.
Both paths report the same statuses, paths, and issue kinds. This is an
implementation scaling choice and does not change the descriptor ABI.

A host must reject at least:

- missing descriptor symbol
- null `out` when calling the entrypoint
- entrypoint error status
- null returned descriptor
- `core_abi_version != NC_COM_CORE_ABI_VERSION`
- nil plugin ID
- null dependency table with a nonzero count
- nil, self-referential, or duplicate dependency plugin IDs
- null class table
- malformed pointer/count combinations
- null descriptor graph pointers where the corresponding count or field
  requires a descriptor
- nil class or interface IDs where forbidden
- missing class constructors
- invalid constructor frame hashes or pointer shapes at activation
- invalid class threading values
- invalid referenced interface identities in typed method descriptors
- interface ABI hashes that do not match typed method descriptors
- duplicate class IDs within one plugin
- duplicate interface IDs within one class
- invalid UTF-8 in metadata used as text

The default `Runtime` performs additional policy validation described in
section 14.

Loading a dynamic library is a trust boundary. Platform loader code may run
before `nc_get_plugin_descriptor` is queried. core ABI v5 descriptor validation is
for trusted plugin metadata shape, not hostile plugin isolation. It does not
sandbox plugin code, prove plugin trustworthiness, or make an untrusted shared
library safe to load.

Descriptor producers must ensure that every non-null descriptor graph pointer
is readable for the advertised count and remains valid for the required
lifetime. Validation can reject null/count shape errors, but it cannot safely
probe arbitrary non-null raw pointers. An unreadable non-null descriptor pointer
is trusted-plugin undefined behavior and may crash the process rather than
produce a recoverable validation error.

## 14. Runtime API and Registry Policy

core ABI v5 does not define an operating-system registry, package database, or
activation service. `nc-com-runtime` owns one private process-global runtime
state containing separate plugin and type registries plus debug callback state.
Rust callers use the `nc-com` `Runtime` facade and its `PluginRegistry`,
`TypeRegistry`, `Inspection`, and `Debug` views. C callers discover five
immutable function tables through six exported symbols:

```rust,ignore
pub extern "C" fn nc_com_runtime_abi_version() -> u32;
pub extern "C" fn nc_com_get_runtime_api_v1() -> *const NcRuntimeApiV1;
pub extern "C" fn nc_com_get_plugin_registry_v1() -> *const NcPluginRegistryV1;
pub extern "C" fn nc_com_get_type_registry_v1() -> *const NcTypeRegistryV1;
pub extern "C" fn nc_com_get_inspection_api_v1() -> *const NcInspectionApiV1;
pub extern "C" fn nc_com_get_debug_api_v1() -> *const NcDebugApiV1;
```

The Rust facade keeps those raw tables private and groups operations by
responsibility:

```rust,ignore
let runtime = nc_com::Runtime::global();

let plugins = runtime.plugin_registry().plugin_ids()?;
let classes = runtime.type_registry().class_ids()?;
let plugin = runtime.inspection().plugin_info(plugins[0])?;
let diagnostics = runtime.debug();

# let _ = (classes, plugin, diagnostics);
# Ok::<(), nc_com::NcStatus>(())
```

`Runtime` owns registration, loading, interface validation, and construction.
`PluginRegistry` and `TypeRegistry` answer what exists. `Inspection` returns
owned metadata, provenance, and relationships. `Debug` creates owned messenger
registrations whose callbacks are removed when their `DebugMessenger` is
dropped.

`nc_com_runtime_abi_version` is mandatory and returns
`NC_COM_RUNTIME_ABI_VERSION`, currently `1`. This runtime ABI version is
independent of `NC_COM_CORE_ABI_VERSION`: a future runtime ABI v2 may be
incompatible with v1 while still consuming the same core descriptor ABI.
Callers must verify the runtime ABI version before calling a versioned getter.

For a compatible runtime ABI v1 library, every getter returns a non-null pointer
to an immutable static table. The table remains valid until the runtime shared
library is unloaded. Each table starts with its byte `size`; additions within a
compatible table generation are append-only. The returned pointer is aligned for
the corresponding table and is readable for at least the leading `usize` size
field, even when the reported table is shorter than the caller's complete known
table type. A conforming runtime reports the actual readable table extent in
`size` and provides storage readable for that many bytes.

Callers must read the leading `size` through the raw pointer or an explicit
size-only prefix before creating a reference to a complete table type. Creating
the complete reference first already requires storage for that complete type and
is not a valid way to inspect a potentially shorter table. Before reading or
calling any other table field, callers must verify that `size` reaches through
that field. A caller using every field shown in this specification may require
`size >= sizeof(the corresponding v1 table)` and may create a reference to that
known prefix only after this check. A larger table is compatible when the known
prefix is unchanged; a shorter table must not be read beyond its advertised
size.

All five tables target the same process-global runtime state. Loading multiple
runtime shared libraries creates independent process-global runtime instances; core ABI v5
does not merge them.

Runtime ABI v1 publishes two process-resource ceilings:

```rust
use nc_com::{
    NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS,
    NC_COM_RUNTIME_MAX_LOADED_MODULES,
};

assert_eq!(NC_COM_RUNTIME_MAX_LOADED_MODULES, 1_024);
assert_eq!(NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS, 64);
```

### 14.1 Runtime Operations

```rust,ignore
#[repr(C)]
pub struct NcRuntimeApiV1 {
    pub size: usize,
    pub core_abi_version: u32,
    pub register_plugin_descriptor:
        unsafe extern "C" fn(*const NcPluginDescriptor) -> NcStatus,
    pub load_plugin_library_utf8:
        unsafe extern "C" fn(*const u8, usize, *mut u64) -> NcStatus,
    pub require_interface:
        unsafe extern "C" fn(NcGuid, NcInterfaceId) -> NcStatus,
    pub construct_class:
        unsafe extern "C" fn(
            NcGuid,
            u32,
            NcAbiHash,
            *const c_void,
            *mut *mut IUnknown,
        ) -> NcStatus,
}
```

`register_plugin_descriptor` validates and registers static descriptor metadata.
The descriptor graph must remain live while registered. The reference runtime
validates each registration attempt once and assembles its plugin, class,
interface, dependency, identity, and diagnostic metadata before taking the
catalog write lock. Only checks against current runtime state and publication
execute inside the blocking transaction. A failed registration does not insert
that descriptor's plugin or classes. Registration never invokes plugin code and
never loads dependencies implicitly. Every hard dependency named by the incoming
descriptor must already exist in the plugin registry. A missing dependency
returns `NC_E_NOT_FOUND` without inserting the plugin or any classes.

The reference runtime stores deterministic registration-order vectors together
with runtime-private module, identity, and relationship indexes under one
catalog `RwLock`. Reads may proceed concurrently. Static registration and
dynamic activation acquire exclusive publication access, so every individual
runtime operation observes catalog state from before or after a commit, never
during it. Count/fill enumeration order is therefore independent of hash-map
iteration. Indexed lookup and precomputed relationship slices are implementation
details: they do not change any core ABI v5 table, descriptor, status, or ordering
rule.

The first successful registration owns its plugin ID and class IDs. Repeating
registration with the same root `NcPluginDescriptor` address is idempotent.
Simultaneous registrations block on the same catalog write lock; after the first
commit, another exact registration returns `NC_OK` without inserting duplicate
records. A different root descriptor with an already-registered plugin ID
returns `NC_E_INVALID_ARG`. An existing class ID is accepted only when it refers
to the same class descriptor address under the same plugin; every other reuse
returns `NC_E_INVALID_ARG`.

`load_plugin_library_utf8` is the required live-activation operation. It loads
trusted plugin code, obtains and registers its descriptor, retains the shared
library, and writes a nonzero process-local module ID. `out_module_id` must be
non-null and is set to zero before path validation. The path buffer is not
NUL-terminated: it must contain exactly `path_len` readable bytes. An empty
path, invalid UTF-8, or an interior NUL byte returns `NC_E_INVALID_ARG`; a null
path with nonzero length returns `NC_E_POINTER` before a load attempt begins.

Live activation is synchronous and blocking for its caller. Native library
opening, descriptor entrypoint invocation, and complete descriptor projection
occur without holding the catalog lock. The runtime then acquires exclusive
catalog publication access, repeats all checks that depend on current state, and
publishes the retained native handle, module record, plugin record, classes,
relationships, and identity mappings in one transaction. It releases the write
lock before returning or invoking plugin constructors, plugin methods, or debug
callbacks. Consequently, a class carrying a nonzero module ID is never visible
without the corresponding retained module and plugin provenance.

A live activation call must not be nested inside another live activation call.
In particular, native loader initialization and `nc_get_plugin_descriptor` must
not directly or indirectly invoke `load_plugin_library_utf8`. Hosts activate an
explicit dependency-first sequence through separate calls. Ordinary native
dependencies loaded internally by the operating-system loader do not violate
this rule, but they do not become registered nc-com plugins.

Once live activation attempts to open the supplied path, failure is process
fatal. Platform loading, symbol discovery, descriptor entrypoint, metadata
validation, dependency validation, duplicate checking, and registration
failures emit the diagnostic through the debug channel, print it to standard
error, and call `abort`; they do not return an error status to the caller. The
runtime retains at most `NC_COM_RUNTIME_MAX_LOADED_MODULES` dynamic modules. An
attempt to activate another module emits `NC_E_OUT_OF_RANGE` and follows the
same fatal path. If a module was opened before failure, its handle is pinned
before diagnostics and abort so plugin code cannot race `dlclose` or
`FreeLibrary`. `NC_OK` is therefore the only returned status after an actual
load attempt.

`require_interface` returns `NC_E_NOT_FOUND` when the class is absent,
`NC_E_NOINTERFACE` when the class does not implement the IID, and
`NC_E_ABI_VERSION` when the IID exists with a different ABI hash.

`construct_class` takes the requested pointer threading capability and
constructor ABI hash between the class ID and constructor frame pointer.
Thread-local is accepted for either class
mode. Thread-safe is accepted only when the registered class descriptor declares
`NC_ABI_THREADING_THREAD_SAFE`. Unknown requested values and attempts to upgrade
a thread-local class return `NC_E_INVALID_ARG` before the plugin constructor is
invoked. The function clears `*out` before fallible work. A missing class returns
`NC_E_NOT_FOUND`. A hash that differs from the registered descriptor returns
`NC_E_ABI_VERSION` before plugin code runs. Zero hash requires a null frame
pointer; a nonzero hash requires a non-null pointer borrowed only for the call.
On success, `out` receives one owned `IUnknown` reference with at least the
requested threading capability.

Runtime ABI v1 has no transactional C batch-loading operation. The Rust facade
provides sequential convenience loading for a dependency-first plan. A later
failure aborts the process, so no partially activated state returns to the host.

### 14.2 Plugin Registry

The plugin registry answers which plugins and loaded modules exist:

```rust,ignore
#[repr(C)]
pub struct NcPluginRegistryV1 {
    pub size: usize,
    pub plugin_count: unsafe extern "C" fn() -> usize,
    pub plugin_ids:
        unsafe extern "C" fn(*mut NcGuid, usize, *mut usize) -> NcStatus,
    pub loaded_module_count: unsafe extern "C" fn() -> usize,
    pub loaded_module_ids:
        unsafe extern "C" fn(*mut u64, usize, *mut usize) -> NcStatus,
}
```

Registered plugin IDs are unique registry identities. Loaded module IDs are
nonzero process-local diagnostic handles; they are not stable across processes,
program runs, or runtime instances. The reference runtime issues IDs from `1`
through `NC_COM_RUNTIME_MAX_LOADED_MODULES` without reuse.

### 14.3 Type Registry

The type registry answers which classes and interface ABI identities exist:

```rust,ignore
#[repr(C)]
pub struct NcTypeRegistryV1 {
    pub size: usize,
    pub class_count: unsafe extern "C" fn() -> usize,
    pub class_ids:
        unsafe extern "C" fn(*mut NcGuid, usize, *mut usize) -> NcStatus,
    pub interface_count: unsafe extern "C" fn() -> usize,
    pub interface_ids:
        unsafe extern "C" fn(*mut NcInterfaceId, usize, *mut usize) -> NcStatus,
}
```

The interface list contains distinct full `NcInterfaceId` values, not bare
IIDs. Two ABI-incompatible contracts sharing an IID are therefore not treated
as interchangeable. Because every registered class explicitly carries the
canonical `IUnknown` descriptor, the global list contains the canonical
`IUnknown` identity exactly once whenever at least one class is registered.

### 14.4 Count and Fill Protocol

Count functions return `usize` directly and never return `NcStatus`. Fill
functions take `out`, `capacity`, and a required non-null `out_len`:

- `NC_OK` means exactly `*out_len` elements were written.
- `NC_E_INSUFFICIENT_BUFFER` means the current required capacity was written to
  `*out_len`; no partial output was written. The required capacity must be
  greater than the supplied `capacity`; returning this status without reporting
  a larger requirement violates the protocol.
- `NC_E_POINTER` means `out_len` was null, or a nonempty result had sufficient
  capacity but `out` was null.

The registry is append-only, so it can grow between count and fill calls. A
caller must resize to the returned `out_len` and may retry after
`NC_E_INSUFFICIENT_BUFFER`. Counts cross an ABI trust boundary: callers should
apply an application resource ceiling, use fallible allocation, require strict
capacity progress, and impose a retry ceiling so a racing or broken provider
cannot force unbounded allocation or retries. Count functions with an identity
argument return zero for both a missing identity and an identity with no related
objects; inspection record lookup distinguishes absence with `NC_E_NOT_FOUND`.

The `nc-com` Rust runtime facade applies a host policy of at most
`NC_COM_RUNTIME_MAX_ENUMERATION_ITEMS` (65,536) elements per enumeration and
`NC_COM_RUNTIME_MAX_ENUMERATION_RETRIES` (8) retries after the initial fill
attempt. It returns `NC_E_OUT_OF_RANGE` for a larger reported count,
`NC_E_OUT_OF_MEMORY` when an in-policy reservation fails, and
`NC_E_INSUFFICIENT_BUFFER` for a non-progressing response or exhausted retry
budget. This is a Rust host-facade policy, not an additional limit on raw C
callers. Owned runtime inspection text is similarly limited to
`NC_COM_RUNTIME_MAX_INSPECTION_TEXT_BYTES` (1 MiB).

### 14.5 Inspection API

Registries answer what exists. Inspection answers metadata, provenance, and
plugin/class/interface relationships. It can also open a trusted plugin library
and synchronously expose its validated descriptor without registering it:

```rust,ignore
pub type NcInspectPluginDescriptorCallbackV1 = unsafe extern "C" fn(
    *const NcPluginDescriptor,
    *mut core::ffi::c_void,
) -> NcStatus;

#[repr(C)]
pub struct NcInspectionApiV1 {
    pub size: usize,
    pub plugin_info: unsafe extern "C" fn(NcGuid, *mut NcPluginInfoV1) -> NcStatus,
    pub module_info: unsafe extern "C" fn(u64, *mut NcModuleInfoV1) -> NcStatus,
    pub class_info: unsafe extern "C" fn(NcGuid, *mut NcClassInfoV1) -> NcStatus,
    pub plugin_dependency_count: unsafe extern "C" fn(NcGuid) -> usize,
    pub plugin_dependency_ids:
        unsafe extern "C" fn(NcGuid, *mut NcGuid, usize, *mut usize) -> NcStatus,
    pub plugin_class_count: unsafe extern "C" fn(NcGuid) -> usize,
    pub plugin_class_ids:
        unsafe extern "C" fn(NcGuid, *mut NcGuid, usize, *mut usize) -> NcStatus,
    pub class_interface_count: unsafe extern "C" fn(NcGuid) -> usize,
    pub class_interface_ids:
        unsafe extern "C" fn(NcGuid, *mut NcInterfaceId, usize, *mut usize) -> NcStatus,
    pub interface_implementor_class_count:
        unsafe extern "C" fn(NcInterfaceId) -> usize,
    pub interface_implementor_class_ids:
        unsafe extern "C" fn(NcInterfaceId, *mut NcGuid, usize, *mut usize) -> NcStatus,
    pub inspect_plugin_library_utf8: unsafe extern "C" fn(
        *const u8,
        usize,
        Option<NcInspectPluginDescriptorCallbackV1>,
        *mut core::ffi::c_void,
    ) -> NcStatus,
}
```

Plugin-dependency queries expose declared hard dependency edges and never load
anything. Class-interface relationship queries expose the descriptor table
model above: every registered class reports the canonical `IUnknown` identity
exactly once, and querying implementors of that identity reports every
registered class. Macro-generated classes report `IUnknown` first.

`inspect_plugin_library_utf8` takes an explicit UTF-8 path, opens the trusted
library, resolves `nc_get_plugin_descriptor`, and validates the complete
descriptor graph through the same validation gate used for live activation.
It does not register the plugin, construct objects, or follow declared nc-com
dependencies. On success it invokes the non-null callback synchronously exactly
once and returns the callback's `NcStatus` unchanged. The `user_data` pointer is
passed through unchanged and may be null.

The descriptor and every pointer reachable from it are borrowed only during the
callback. Callers must copy anything needed afterward and must not modify or
free plugin-owned metadata. A null callback returns `NC_E_POINTER`; malformed
path input returns `NC_E_POINTER` or `NC_E_INVALID_ARG`; native loading,
entrypoint, and validation failures return their mapped loader status and emit a
plugin diagnostic. These inspection failures are recoverable and do not abort
the process.

Opening a dynamic library can execute arbitrary platform loader code. The path
must therefore identify trusted code; descriptor validation is not a sandbox.
Once the library has opened, its handle remains pinned until process exit even
if symbol lookup, validation, or the callback later fails. The Rust facade
exposes this operation as
`Inspection::with_plugin_library_descriptor` and enforces the callback-scoped
borrow in its return type.

Inspection text uses borrowed immutable UTF-8 views:

```rust,ignore
#[repr(C)]
pub struct NcUtf8View {
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
pub struct NcPluginInfoV1 {
    pub plugin_id: NcGuid,
    pub qualified_name: NcUtf8View,
    pub version: NcUtf8View,
}

#[repr(C)]
pub struct NcModuleInfoV1 {
    pub module_id: u64,
    pub plugin_id: NcGuid,
    pub path: NcUtf8View,
}

#[repr(C)]
pub struct NcClassInfoV1 {
    pub class_id: NcGuid,
    pub plugin_id: NcGuid,
    pub module_id: u64,
    pub qualified_name: NcUtf8View,
    pub version: NcUtf8View,
    pub constructor_abi_hash: NcAbiHash,
    pub threading: u32,
}
```

For every `NcUtf8View`, `len == 0` permits a null `data` pointer and represents
empty text. When `len != 0`, `data` is non-null and points to exactly `len`
readable bytes containing valid UTF-8. Inspection views use the runtime lifetime
described below; debug message views use the narrower callback lifetime in
section 14.6.

Info lookups return `NC_E_POINTER` for null output and `NC_E_NOT_FOUND` for a
missing identity. A class `module_id` of zero denotes static registration.
Returned UTF-8 views remain valid until the runtime shared library is unloaded;
callers must not modify or free them. core ABI v5 does not support plugin unload.

The `nc-com-inspect` JSON file is a versioned tooling artifact, not another ABI
core v5 memory layout and not a source-reconstruction format. Its schema version is
independent of `NC_COM_CORE_ABI_VERSION`. A schema-v3 artifact records the producing
tool version, target triple, pointer width, endianness, core ABI version, and
owned plugin/class/interface identities, ABI hashes, and method diagnostics.
Readers must reject unknown fields and unsupported schema versions and must
validate identity derivation, duplicate entries, and method indices before
comparison.

The artifact intentionally does not serialize structured parameter, return, or
aggregate definitions. Human-readable method signatures are derived diagnostics
only and are not reverse-parsed to reconstruct or independently validate an
interface contract. The inspector validates the complete typed descriptor and
recomputes its advertised interface hash while the library is mapped, before
writing the artifact. core ABI v5 does not promise that an inspection artifact can
recreate Rust traits, C declarations, vtables, aggregate layouts, or source
definitions.

Exact artifact equality is not, by itself, an ABI compatibility policy. The
inspector therefore keeps exact comparison as the default/`--exact` mode and
provides a directional `--compat` check from an old artifact to a proposed new
artifact. Compatibility mode uses the interface `(IID, ABI hash)` pair as the
complete interface-contract authority; it does not parse method diagnostics.

Compatibility requires the same plugin ID, target facts, and core ABI version.
Every old class ID must remain present with the same constructor-frame ABI hash. A
`thread_safe` class must not become `thread_local`. For each old class, every
old interface IID must remain on that class with the same ABI hash. Removing a
class or interface, changing an existing IID's hash, changing constructor
arguments, weakening threading, or adding a hard plugin dependency is breaking.
Adding classes or new side-by-side interface IIDs, strengthening
`thread_local` to `thread_safe`, removing a hard dependency, and changing tool,
plugin-version, class-version, or rendered-signature diagnostics are compatible.
Diagnostic changes remain visible in the report but do not affect its exit
status.

### 14.6 Debug API

The debug layer is callback-based and observational. It does not change runtime
operation results:

```rust,ignore
pub const NC_DEBUG_SEVERITY_INFO: u32 = 1 << 0;
pub const NC_DEBUG_SEVERITY_WARNING: u32 = 1 << 1;
pub const NC_DEBUG_SEVERITY_ERROR: u32 = 1 << 2;
pub const NC_DEBUG_SEVERITY_ALL: u32 =
    NC_DEBUG_SEVERITY_INFO
    | NC_DEBUG_SEVERITY_WARNING
    | NC_DEBUG_SEVERITY_ERROR;

pub const NC_DEBUG_TYPE_RUNTIME: u32 = 1 << 0;
pub const NC_DEBUG_TYPE_PLUGIN: u32 = 1 << 1;
pub const NC_DEBUG_TYPE_VALIDATION: u32 = 1 << 2;
pub const NC_DEBUG_TYPE_ALL: u32 =
    NC_DEBUG_TYPE_RUNTIME
    | NC_DEBUG_TYPE_PLUGIN
    | NC_DEBUG_TYPE_VALIDATION;

#[repr(C)]
pub struct NcDebugMessageV1 {
    pub status: NcStatus,
    pub message: NcUtf8View,
    pub plugin_id: NcGuid,
    pub class_id: NcGuid,
    pub interface_id: NcInterfaceId,
    pub module_id: u64,
}

pub type NcDebugCallbackV1 = unsafe extern "C" fn(
    severity: u32,
    message_type: u32,
    message: *const NcDebugMessageV1,
    user_data: *mut c_void,
);

#[repr(C)]
pub struct NcDebugMessengerCreateInfoV1 {
    pub size: usize,
    pub severity_mask: u32,
    pub message_type_mask: u32,
    pub callback: Option<NcDebugCallbackV1>,
    pub user_data: *mut c_void,
}

#[repr(C)]
pub struct NcDebugApiV1 {
    pub size: usize,
    pub create_messenger: unsafe extern "C" fn(
        *const NcDebugMessengerCreateInfoV1,
        *mut u64,
    ) -> NcStatus,
    pub destroy_messenger: unsafe extern "C" fn(u64),
}
```

`create_messenger` requires non-null `create_info` and `out_handle`; either null
pointer returns `NC_E_POINTER`. When `out_handle` is non-null, it is set to zero
before create-info validation. `size` must equal
`sizeof(NcDebugMessengerCreateInfoV1)`. Unknown mask bits, a null callback, or an
incorrect size return `NC_E_INVALID_ARG`. Zero severity or message-type masks
are valid and create a messenger that receives no callbacks. Success writes a
nonzero process-local handle. At most
`NC_COM_RUNTIME_MAX_ACTIVE_DEBUG_MESSENGERS` registrations may exist
simultaneously. Additional creation returns `NC_E_OUT_OF_RANGE` and leaves the
cleared output handle at zero. Destroying a messenger frees one registration
slot. Handles increase monotonically and are not reused; exhaustion of the
nonzero `u64` handle space is permanent and also returns `NC_E_OUT_OF_RANGE`.

Callbacks are dispatched synchronously by the runtime operation producing the
diagnostic and may therefore run concurrently on any calling thread. The
`message` argument is non-null for the callback. `NcDebugMessageV1` contains the
associated status, callback-scoped UTF-8 message, and optional plugin, class,
interface, and module identities represented by nil/none/zero sentinels.
Message pointers and callback message text are valid only until the callback
returns.

The runtime invokes callbacks without holding its messenger registry lock, so a
callback may call runtime APIs other than `destroy_messenger`. A callback must
not destroy any debug messenger, including its own. `destroy_messenger` removes
the handle from future dispatch and waits for every already-started callback for
that messenger to return. Once destruction returns, no callback for that handle
is running or can begin, and the caller may release the callback and `user_data`.
Zero and unknown handles are ignored. The caller must make callback and
`user_data` use safe on every calling thread until that destruction boundary.

Callers must still treat all table calls as trusted in-process FFI. Null checks,
sizes, UTF-8 validation, and descriptor validation do not sandbox plugin code or
make arbitrary non-null pointers safe.

## 15. Built-In Data Interfaces

core ABI v5 defines two common read-only data interfaces.

### 15.1 `INcBytesAbi`

Interface ID:

```text
b0b0d6d2-f170-54e9-b6d5-143f9452b951
```

Layout:

```rust,ignore
#[repr(C)]
pub struct INcBytesAbi {
    pub vtable: *const INcBytesVTable,
    pub object: *mut c_void,
}

#[repr(C)]
pub struct INcBytesVTable {
    pub unknown: IUnknownVTable,
    pub len: unsafe extern "C" fn(
        this: *mut INcBytesAbi,
        out: *mut u64,
    ) -> NcStatus,
    pub byte_at: unsafe extern "C" fn(
        this: *mut INcBytesAbi,
        index: u64,
        out: *mut u8,
    ) -> NcStatus,
    pub copy_to: unsafe extern "C" fn(
        this: *mut INcBytesAbi,
        offset: u64,
        len: u64,
        out: *mut u8,
    ) -> NcStatus,
}
```

`len` returns the number of bytes. `byte_at` returns `NC_E_OUT_OF_RANGE` when
`index >= len`. `copy_to` is described by `CopyToBufferOffset<u8>` and lowers to
`offset`, `len`, and `out` ABI parameters. It copies `len` bytes from `offset`
into the caller's output buffer. It returns `NC_E_OUT_OF_RANGE` when
`offset..offset + len` is not within the byte sequence. When `len != 0`, `out`
must be non-null, valid for `len` byte writes, and non-overlapping with the
source byte range exposed by the object. When `len == 0`, `out` may be null.
The implementation must validate the complete request before writing: success
writes exactly `len` bytes, while any failure leaves the caller's output buffer
unchanged.

The allocation-backed `nc-com` Rust facade limits the convenience `to_vec` and
`to_i8_vec` copies to `NC_COM_DEFAULT_OWNED_COPY_MAX_BYTES` (64 MiB). Their
`to_vec_bounded` and `to_i8_vec_bounded` variants let the caller select another
maximum. A reported length over the selected limit or outside Rust `Vec`
capacity range returns `NC_E_OUT_OF_RANGE`; failure to reserve an in-limit,
representable vector returns `NC_E_OUT_OF_MEMORY`. Raw `copy_to` remains
caller-bounded and performs no host allocation.

### 15.2 `INcStringAbi`

Interface ID:

```text
01a5b18c-be18-5d39-b71f-202df7c84664
```

Layout:

```rust,ignore
#[repr(C)]
pub struct INcStringAbi {
    pub vtable: *const INcStringVTable,
    pub object: *mut c_void,
}

#[repr(C)]
pub struct INcStringVTable {
    pub unknown: IUnknownVTable,
    pub byte_len: unsafe extern "C" fn(
        this: *mut INcStringAbi,
        out: *mut u64,
    ) -> NcStatus,
    pub byte_at: unsafe extern "C" fn(
        this: *mut INcStringAbi,
        index: u64,
        out: *mut u8,
    ) -> NcStatus,
    pub copy_to: unsafe extern "C" fn(
        this: *mut INcStringAbi,
        offset: u64,
        len: u64,
        out: *mut u8,
    ) -> NcStatus,
}
```

`byte_len` returns the number of UTF-8 bytes. `byte_at` returns
`NC_E_OUT_OF_RANGE` when `index >= byte_len`. Objects implementing
`INcStringAbi` guarantee that the exposed byte sequence is valid UTF-8.

`copy_to` has the same bounded `CopyToBufferOffset<u8>` lowering and range,
pointer, and non-overlap requirements as `INcBytesAbi::copy_to`. It copies raw
UTF-8 bytes, so a requested subrange may split a code point. The implementation
must validate the complete request before writing: success writes exactly `len`
bytes, while any failure leaves the caller's output buffer unchanged. This
guarantees that copying a complete conforming string requires a bounded number
of ABI calls rather than one `byte_at` call per byte.

Standard nc-com string objects also support `INcBytesAbi` for the same UTF-8
byte sequence as an additional queryable capability; consumers do not need that
optional interface to copy an `INcStringAbi` efficiently.

The allocation-backed Rust `to_bytes` and `to_string` conveniences use the same
64 MiB default and error rules as byte objects. `to_bytes_bounded` and
`to_string_bounded` accept a caller-selected maximum; raw `copy_to` remains the
allocation-free, caller-bounded path.

## 16. Loaded Module Lifetime

Dynamic loading is owned by the process-global runtime. A successfully loaded
plugin library is stored by the host and remains mapped until process exit.
Registered class entries keep pointers into immutable descriptor metadata owned
by the loaded library, so descriptor lifetime is tied to the host's
loaded-module list.

Loaded modules have process-local `LoadedModuleId` values for diagnostics. Hosts
can use `Runtime::inspection` or the C runtime `nc_com_*` functions to list
loaded plugins, look up a loaded module by ID, and query which registered
classes and interfaces came from that module. These IDs are not part of the
cross-process ABI and are not passed to constructors or stored in generated
object storage.

core ABI v5 intentionally does not support explicit plugin unload or hot reload.
Dynamic library unload is not a reliable rollback boundary for in-process
plugins: loading a library may run platform loader code, initialize globals or
thread-local state, register callbacks, start threads, publish function
pointers, or otherwise mutate process-global state. Constructors may also
mutate host state. These effects cannot be proven
reversible by removing class metadata from a registry.

Dynamic libraries loaded successfully through `Runtime::load_plugin_library`
are kept by the process-global runtime for the process lifetime. A failed live
activation also never unloads an opened handle: the runtime pins it, emits the
diagnostic, and aborts. No live dynamic-load failure returns to normal host
operation.

Non-registering inspection is a separate fallible operation. The shared runtime
opens trusted plugin code, calls only the descriptor entrypoint, validates the
descriptor graph, and exposes it synchronously to the caller without registering
the plugin or following declared nc-com dependencies. Tooling such as
`nc-com-inspect` copies that borrowed metadata into owned records. Opened
inspection handles remain pinned until the inspector process exits, which is the
reliable unload boundary. Batch planning inspects every selected artifact and
reports every ordinary load or validation error it
receives before failing the plan.
Owned inspection output includes every plugin, class, and interface qualified
name beside its GUID so dependency planning can enforce the one-to-one identity
mapping across the complete selected artifact set.

## 17. Class Construction

Generated descriptor constructors use `construct_class_checked`. That helper:

- rejects null `out` with `NC_E_POINTER`
- clears `*out` before fallible work
- rejects null `class` with `NC_E_POINTER`
- validates the expected class ID
- validates the expected constructor-frame ABI hash
- validates that a constructor exists
- validates argument shape
- writes one owned `IUnknown` pointer on success
- preserves negative errors returned by the Rust builder and normalizes a
  non-negative status carried by `Result::Err` to `NC_E_INVALID_ARG`

If `constructor_abi_hash == NcAbiHash::ZERO`, `arguments` must be null. A
nonzero constructor hash requires a non-null frame pointer. The caller must
keep the frame and every nested buffer or COM borrow readable and immutable
until the constructor returns.

Construction through `Runtime` or `NcRuntimeApiV1::construct_class` performs
this descriptor-declared hash and pointer-shape validation before calling the
raw class constructor. Direct manual calls to an `NcConstructorFn` remain the
caller's responsibility.

The supported typed Rust path declares one `#[constructor]` function on a
callable `#[com_class]` trait. The macro lowers its ordered parameters into a
private `repr(C)` frame, generates a lifetime-bound wrapper and an
`arguments(...)` builder, stores the frame hash in the class descriptor, and
decodes the frame inside the implementation library. The frame supports plain
ABI values, immutable call-scoped buffers, and borrowed COM references. It is
not a COM object and has no runtime reflection or dynamically typed value bag.

## 18. Threading Model

Threading is represented in Rust pointer types and class metadata, not in the
raw ABI pointer layout. The same interface pointer layout is used for both
threading modes.

| Rust handle | Ownership and reference-count behavior | `Send` | `Sync` | Permitted weakening |
|---|---|:---:|:---:|---|
| `ComPtr<T, ThreadLocal>` | Owns one reference; clone performs `AddRef`, drop performs `Release` | No | No | None |
| `ComPtr<T, ThreadSafe>` | Owns one reference; clone performs `AddRef`, drop performs `Release` | Yes | Yes | Owned or borrowed `ThreadLocal` view |
| `ComRef<'a, T, ThreadLocal>` | Borrows a reference; copy and drop do not change the count | No | No | None |
| `ComRef<'a, T, ThreadSafe>` | Borrows a reference; copy and drop do not change the count | Yes | Yes | Borrowed `ThreadLocal` view |

`ComPtr<T>` and `ComRef<'a, T>` default to `ThreadLocal`. Calling
`ComRef::add_ref` performs `AddRef` and produces an owning `ComPtr` with the same
threading marker. `QueryInterface` also preserves the source pointer's marker
because changing the selected interface does not change the underlying
implementation's threading guarantee.

Pointer modes are capabilities carried by individual handles. `ThreadLocal`
means that handle carries no permission for cross-thread calls, sharing, or
final release; it does not assert that every other alias to the same object is
also thread-local. `ThreadSafe` is the stronger capability. It may be removed
explicitly with `ComPtr::into_thread_local`, `ComPtr::as_thread_local_ref`, or
`ComRef::into_thread_local`. These operations preserve the same raw interface
pointer and perform no reference-count operation. No safe operation upgrades a
thread-local pointer to thread-safe.

Objects are thread-local by default. `#[com_object(thread_safe, ...)]` requires
the Rust class to be `Send + Sync` and generates thread-safe COM pointers. This
compile-time assertion is necessary but not sufficient: the class author must
also ensure that every exposed COM method may be called from any thread,
including concurrently, and that final destruction may occur on any thread.

`NcClassDescriptor::threading` preserves this distinction across dynamic class
registration and construction. The Rust runtime facade's existing
`construct_class` and `construct_class_as` methods deliberately request
`ThreadLocal`. `construct_class_with_model::<M>` and
`construct_class_as_with_model::<T, M>` request the capability represented by
the sealed `ThreadingModel` marker. A `ThreadSafe` request is checked against the
descriptor before plugin code runs. `QueryInterface` preserves the marker of
the source `ComPtr`.

Generated Rust classes implement `ComClass`, whose `Threading` associated type
is the single source for their constructor pointer types, descriptor threading
field, and generated constructor validation. It records the strongest
capability supported by pointers to that class; callers may still choose a
conservative thread-local view.

Both levels are required. `ComClass::Threading` can describe the producer while
the concrete class type is statically known, but an interface pointer erases
that class type. The interface type `T` cannot replace it because multiple
classes with different threading promises may implement the same interface.
Nor can the raw pointer replace it: core ABI v5 deliberately gives both modes the
same representation and stores no per-pointer threading tag. The `ComPtr` or
`ComRef` marker therefore carries the validated producer evidence through class
type erasure so Rust can enforce `Send` and `Sync` without a runtime check on
every use. Although `NcClassDescriptor::threading` supplies a runtime producer
declaration, Rust auto traits cannot vary according to a field value. A checked
constructor validates that declaration once and materializes the result in its
statically selected pointer marker.

`Send` on an owning pointer authorizes moving the last owner, which may cause
final `Release` and destruction on the destination thread. `Sync` authorizes
shared access and consequently concurrent calls through aliases. These require
more than an atomic reference count: the counter only protects lifetime
transitions and does not synchronize class fields, method invariants, contained
objects, callbacks, foreign resources, or destructor requirements.

Atomic reference counting alone does not make an object thread-safe. The stored
Rust value, all exposed methods, foreign resources, contained COM pointers, and
destruction behavior must satisfy the class promise. Handwritten or foreign
descriptor producers that declare `THREAD_SAFE` assume this unsafe contract.

The built-in process-global runtime is thread-safe.

core ABI v5 does not define COM apartments, thread affinity marshaling, or an
inter-thread proxy system.

## 19. Panic and Error Boundaries

Expected cross-boundary failures must be returned as `NcStatus`.

nc-com does not catch panics in generated ABI thunks. With `panic=abort`, the
process aborts at the panic site. With unwinding enabled, a panic that reaches a
non-unwind `extern "C"` ABI boundary aborts under Rust's ABI unwind rules.

Final `Release` is not a fallible error boundary. If cleanup can fail, expose an
explicit method such as `close`, `flush`, or `shutdown` and require callers to
invoke it while they still own a COM reference.

## 20. Dynamic Loading

`Runtime::load_plugin_library` fatally activates one required plugin by explicit
trusted path.
`Runtime::load_plugin_libraries` is a Rust convenience that loads
multiple paths sequentially in dependency-first order. Neither operation
returns a recoverable load failure.

Hosts should inspect the application-selected artifact set, validate its hard
dependency graph, and pass the resulting deterministic order to
`Runtime::load_plugin_libraries`. Registration verifies but never searches for
or loads missing dependencies. Native dependencies loaded by the platform
remain distinct from nc-com registration and are not added to the plugin/type
registries merely because the operating-system loader mapped them.
Load or validation failures abort because dynamic loading may already have run
unsafe process-mutating code.

core ABI v5 does not define:

- search paths
- platform-specific file extensions
- install directories
- environment variables
- manifest files
- metadata caches
- automatic dependency loading
- dependency resolver semantics
- plugin package names
- application manifests or app launchers
- plugin downloading
- signature verification
- certificate policy
- plugin management UI

The current runtime loader supports Unix and Windows dynamic library APIs. On
targets that are neither Unix nor Windows, offline inspection reports
`PluginLoadErrorKind::UnsupportedPlatform`; required live activation diagnoses
the unsupported platform and aborts.

Unix loading uses immediate symbol resolution. Windows loading uses
`LoadLibraryW` and `GetProcAddress`.

The path passed to the loader must identify trusted code. ABI validation is not
a sandbox and does not make an untrusted library safe to load.

Directory scanning, discovery paths, downloading, installation, signing,
certificate verification, allowlists, and plugin management are application
policy. `nc-com` accepts explicit trusted paths and validates the in-process ABI
contracts exposed by those libraries.

## 21. ABI Evolution Rules

The no-mutation rules in this section apply to published contracts beginning at
the explicit ABI-v5 ratification baseline. During the experimental draft phase,
they describe the intended post-freeze policy rather than a promise of
compatibility with earlier pre-alpha revisions.

Published interfaces must not be mutated in place. The following require a new
versioned interface ID and therefore a new IID:

- adding a method
- removing a method
- renaming a method
- reordering methods
- changing argument types
- changing return types
- changing void versus out-pointer success shape
- changing borrowed versus owned COM semantics
- changing a referenced interface IID (referenced contract hashes are checked at pointer boundaries)
- changing the generated ABI hash grammar
- changing the meaning of an existing method
- changing the normative interface qualified name

Adding a method to the end of a vtable is still an ABI break.

The generated interface ABI hash changes when method order or another hashed
interface-contract input changes. Core descriptor records and runtime API
tables are outside that hash domain and use the version discriminators below.

Core ABI changes require a new core ABI version when they change:

- `IUnknown` layout or semantics
- descriptor field order
- descriptor field meaning
- status-code numbering
- calling convention
- object lifetime rules
- module lifetime rules
- constructor ownership rules
- hash serialization rules

Runtime API changes require a new runtime ABI generation when they change an
existing exported getter, table field layout, function signature, or field
semantics incompatibly. New fields may be appended to an existing v1 table only
when callers can detect them through the leading `size` field and the existing
prefix remains unchanged.

Changing descriptor layouts, including appending fields, requires a new core
ABI version unless a future ABI first defines an explicit extension mechanism.
A new descriptor symbol is required only when the entrypoint function shape or
discovery rules change.

## 22. Contract Crate Policy

Long-lived plugin ecosystems should publish ABI definitions in a small contract
crate.

The contract crate should contain only:

- `#[com_interface]` definitions
- `#[com_struct]` records
- `com_int_enum!` integer newtypes
- `#[com_enum]` tagged payload values
- plugin IDs derived from their normative qualified names
- class IDs derived from their normative qualified names
- shared constants
- golden ABI tests

The contract crate should not contain:

- implementations
- loader code
- `Runtime` registration policy
- heap-owned Rust containers crossing the boundary
- host-specific activation policy

Recommended release policy after ABI-v5 ratification:

- patch releases change documentation, tests, or non-ABI internals only
- minor releases add side-by-side interfaces, classes, records, or enum values
  while keeping old contracts available
- major releases may remove old contracts or intentionally change compatibility
  policy
- published interfaces are never mutated in place

Before ratification, golden tests describe the current draft deterministically
and may be updated intentionally together with a breaking draft change. Their
presence does not make the represented identities or layouts published ABI.

Golden tests should assert:

- interface IDs
- interface ABI hashes
- method order
- typed method descriptors and rendered diagnostic signature text
- class IDs
- plugin IDs
- important descriptor metadata

## 23. C Interop

The ABI is C-compatible when C declarations exactly match the current core layouts,
calling convention, status values, ownership rules, and pointer lifetimes.

core ABI v5 does not currently define a canonical generated C header format. Header
generation is a tooling layer on top of this specification. C consumers are
responsible for using matching declarations and for obeying COM reference
ownership.

Rust `bool` and pointer-sized integer use should be handled with particular
care in C-facing contracts. For C-first APIs, prefer explicit fixed-width
integer representations unless a contract crate and generated headers define
the exact mapping.

## 24. Non-Goals

core ABI v5 does not specify:

- Windows COM apartments
- Windows registry activation
- `HRESULT`
- system COM marshaling
- Rust trait object ABI
- an operating-system-wide or application package registry
- plugin discovery
- package manifests
- semantic version resolution
- cryptographic signing
- trust policy
- sandboxing
- hot reload
- explicit unload
- cross-process transport
- cross-architecture transport
- reverse generation of C headers from runtime descriptors
- reconstruction of source-level types or non-Rust language bindings

## 25. Application Responsibilities

The v5 core ABI is suitable for same-process Rust component boundaries. It
deliberately does not define an application shell, package manager, plugin
marketplace, or operating-system style activation database.

Applications that need a broader plugin ecosystem should define their own
policy for:

- plugin directory layout and scanning
- platform naming conventions for plugin artifacts
- downloading, installation, updates, and rollback
- package manifests and application manifests
- dependency resolver semantics and version policy
- trust policy, allowlists, content hashes, signatures, certificates, and
  trusted directories
- metadata caches for avoiding repeated library loads
- fail-fast required plugins versus optional plugin recovery
- plugin management UI and diagnostics
- application capability interfaces such as logging, diagnostics, cancellation,
  configuration, collections, streams, and dependency/capability discovery

Those policies can be built on top of `nc-com`, but they are outside the raw ABI
contract.
