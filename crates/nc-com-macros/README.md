# nc-com-macros

`nc-com-macros` implements the procedural macros used by `nc-com`.

Most users should not depend on this crate directly. Depend on `nc-com` and use
the macros re-exported from there:

```rust
use nc_com::{com_class, com_interface, com_struct, com_enum, plugin};
```

Direct use is mainly useful for development of the `nc-com` workspace itself or
for advanced facade crates that deliberately re-export the runtime under a
different name.

## Macros

`#[com_interface]` keeps a Rust trait as the implementation surface and
generates:

- `ITraitAbi`: a C-compatible interface pointer header
- `ITraitVTable`: a C-compatible vtable
- `ITraitComPtrExt`: safe call helpers for `nc_com::ComPtr<ITraitAbi>`
- an `unsafe impl nc_com::ComInterface` that certifies the generated header,
  vtable prefix, interface identity, ABI hash, and `IUnknown` pointer conversion
- `ABI_HASH`, method descriptors with typed method hashes and typed parameter
  and return descriptors, plus rendered diagnostic signature text for tools

Using the generated interface and its safe `ComPtr` helpers requires no unsafe
code. `ComInterface` is unsealed for low-level FFI adapters, but handwritten
implementations must use `unsafe impl` and uphold its complete safety contract;
the attribute macro is the supported safe implementation path.

Generated `ABI_HASH` values are SHA-256 hashes of typed ABI parts. They include
method order, method names, argument and return ABI types, passing modes, void
versus `out` returns, `ComRef` borrowed parameters, `ComPtr` owned returns, safe
buffer passing modes, COM threading mode, referenced interface IDs, and
referenced interface ABI hashes.

`#[com_class]` has two related forms. On a trait it declares one registered
class's primary callable contract and generates the same C-shaped call machinery
without requiring a separately authored `IClass` interface. On a struct it
wraps the private implementation in generated COM object storage with:

- `IUnknown`
- one ABI header per exposed interface
- reference counting
- `QueryInterface`
- constructors such as `from_value` and, for unit structs, `create`
- generated class interface metadata
- optional generated class descriptors when `class_id(...)`, `name`,
  `version`, and `constructor(...)` are provided, including
  `class_descriptor() -> &'static NcClassDescriptor`

A contract-backed implementation uses
`#[com_class(contract(Class), constructor(args = none), interfaces(...))]`.
The contract is automatically its first non-`IUnknown` projection and supplies
the descriptor's class ID, name, and version. The explicit interface list is
therefore reserved for additional queried capabilities.

`IUnknown` is implicit in `#[com_class(interfaces(...))]`. The generated
interface table contains its canonical descriptor first, followed by the listed
interfaces in declaration order. Explicit `IUnknown` and duplicate entries are
rejected. Listed paths must also have distinct final names because those names
determine generated storage fields and companion ABI types.

Classes are thread-local by default. `#[com_class(thread_safe, ...)]` requires
the Rust value to be `Send + Sync`, returns `ComPtr<_, ThreadSafe>`, and records
the choice once as `ComClass::Threading`. Generated constructor pointer types,
constructor validation, and `NC_ABI_THREADING_THREAD_SAFE` descriptor metadata
all derive from that associated type so dynamic hosts can request the capability
without an unchecked raw-pointer conversion.

| Class declaration | `ComClass::Threading` | Generated pointer | Descriptor threading |
|---|---|---|---|
| `#[com_class(...)]` | `ThreadLocal` | `ComPtr<_, ThreadLocal>` | `THREAD_LOCAL` |
| `#[com_class(thread_safe, ...)]` | `ThreadSafe` | `ComPtr<_, ThreadSafe>` | `THREAD_SAFE` |

The associated type keeps generation internally consistent while the class is
known. The pointer still needs its own marker because returning an interface
erases the class type, and the same interface may be implemented by classes from
either row.

`#[com_struct]` verifies that a fixed-size record is plain ABI data, owns its
C-compatible representation, records physical size/alignment/field offsets, and
implements `ComStruct`, `AbiType`, and `AbiPlainValue`. Use
`#[com_struct(packed)]`, `#[com_struct(packed(N))]`, or
`#[com_struct(align(N))]` when a contract requires non-default layout.

By default, each field must implement `AbiPlainValue`.
`#[com_struct(c_struct)]` accepts external C-layout fields without nc-com traits
or annotations. It checks nested FFI layouts at compile time with rustc's
`improper_ctypes_definitions` lint promoted to an error; `Copy` and syntactic
field checks still apply. The author must ensure external types contain no
hidden pointers, references, or ownership obligations. In this mode, field
fingerprints contain only size and alignment; the enclosing record also records
field order and offsets. Incompatible changes invisible to that metadata require
a new interface identity. See the [external-field guide](../README.md#external-c-layout-fields)
for an example and the precise limits of this option.

`#[com_enum]` keeps a Rust payload enum for local use and generates an explicit
tag + union ABI representation next to it. The generated `NameAbi` keeps its
tag/payload fields opaque so safe Rust cannot select an uninitialized or invalid
union member. Use `From<Name>`, `TryFrom<NameAbi>`, `tag()`, and
`from_unknown_tag` for safe construction and conversion. Raw tag/payload pairing
is available only through the documented unsafe `from_raw_parts_unchecked`
escape hatch.

`plugin!` assembles generated class descriptors into one static plugin
descriptor. It emits `pub fn plugin_descriptor() -> &'static NcPluginDescriptor`
for in-process hosts. Call `nc_com::export_plugin!()` at the final shared
library root to export `nc_get_plugin_descriptor`:

```rust
#[com_class(
    class_id(auto_guid = "example.Echo:class:v1"),
    name = "example.Echo:class:v1",
    version = "1.0.0",
    constructor(args = none),
    interfaces(IEcho)
)]
pub struct Echo;

plugin! {
    plugin_id(auto_guid = "example.echo_plugin:v1"),
    name = "example.echo_plugin:v1",
    version = "1.0.0",
    dependencies = [FOUNDATION_PLUGIN_ID],
    classes = [Echo],
}
```

`class_id(...)` and `plugin_id(...)` accept an explicit UUID string,
`auto_guid = "stable.Name:v1"`, or a path to an `NcGuid` constant. Every form
is checked against the adjacent normative `name`; explicit values are golden
pins, not arbitrary IDs. Contract crates should define constant paths with
`guid_from_qualified_name` so hosts and plugins share one source of identity.
The optional `dependencies = [...]` list contains hard
plugin IDs that must already be registered. It never triggers path discovery or
recursive loading. core ABI v3 descriptors remain declarative and have no
initialization hook.

## ABI Lowering

Supported interface methods have this shape:

```rust
fn method(&self, args...) -> Result<T, nc_com::NcStatus>;
```

Lowering rules:

- plain values must implement `nc_com::AbiType`
- `Result<(), NcStatus>` lowers to a status-only ABI function
- non-void returns lower to an out pointer plus `NcStatus`
- `Result::Err` must contain a negative status; generated thunks preserve
  negative errors and map a non-negative error to `NC_E_INVALID_ARG` without
  writing the output slot
- `ComRef<'_, IFooAbi>` arguments lower to borrowed raw interface pointers
- `ComPtr<IFooAbi>` returns lower to owned raw interface pointers
- `NcCopyFromBufferOffset<'_, T>` arguments lower to `offset`, `len`, and
  readable `*const T` raw ABI slots
- `NcCopyToBufferOffset<'_, T>` arguments lower to `offset`, `len`, and
  writable `*mut T` raw ABI slots

Methods named `query_interface`, `add_ref`, or `release` conflict with generated
`IUnknown` lifecycle thunks and are rejected with a focused diagnostic. The
macro also rejects a parameter only when its name would duplicate a derived raw
buffer slot or shadow a generated adapter binding. Unsupported trait shapes are
likewise rejected during macro expansion with span-aware diagnostics.

## Interface IDs

Interfaces can use an explicit UUID:

```rust
#[com_interface(
    "bfec7c45-902c-5f94-84ec-77df282eb9f6",
    name = "example.Calculator:v1"
)]
pub trait ICalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, nc_com::NcStatus>;
}
```

They can also use deterministic UUIDv5 generation from a qualified ABI name:

```rust
#[com_interface(auto_guid = "example.Calculator:v1")]
pub trait ICalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, nc_com::NcStatus>;
}
```

The qualified-name string is stored in the interface descriptor and is the
sole UUIDv5 input. Changing it changes the interface ID. Explicit UUID syntax
requires `name = "..."` and macro expansion verifies the same derivation.

## Renamed Runtime Crates

Generated code normally discovers the `nc-com` runtime crate through Cargo
metadata. If a facade or renamed dependency needs to be used, pass an explicit
runtime path:

```rust
#[com_interface(auto_guid = "example.Foo:v1", nc_com = my_runtime)]
pub trait IFoo {
    fn value(&self) -> Result<u32, my_runtime::NcStatus>;
}
```
