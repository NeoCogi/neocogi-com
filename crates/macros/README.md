# neocogi-com-macros

`neocogi-com-macros` implements the procedural macros re-exported by
`neocogi-com`. Application and plugin crates should use those re-exports rather
than depend on this implementation crate directly.

Public contracts should normally be written in `.ncidl` files. The generator
uses the common `com_interface` macro internally; implementation crates use
`com_object`, `com_factory`, and `factory_plugin`.

## Object implementations

`#[com_object(Interface)]` implements one generated interface contract:

```rust,ignore
#[com_object(my_contracts::Button)]
struct ButtonObject {
    label: String,
}
```

The interface supplies the public identity, threading capability, callable
surface, and required interfaces. The implementation supplies private
state and method bodies. Generated storage contains one Rust value, one reference
count, and one borrow guard. Static tables implement signature-checked
`QueryInterface`; handles carry the allocation and selected table, so interfaces
add no per-object fields. The compiler resolves distinct inherited contracts,
including diamonds, without retaining full inspection metadata in dispatch.
See the [storage decision](../../../docs/COM_INTERFACE_STORAGE.md) for the
history of the reverted metadata-heavy version and the deliberate handle-size
tradeoff in the current representation.
`into_local(initialized_value)` returns a `LocalCom<Implementation>` when the
creating binary also needs typed access to the owned implementation. Its
`try_borrow` and `try_borrow_mut` guards share the generated COM call gate;
release them before calling an interface that enters the same object. Neither
guard performs a query, allocation, or reference-count operation. A local handle
cannot be recovered safely from an arbitrary foreign interface.
Ordinary unit objects have no generated zero-argument constructor.

`#[com_object(interfaces(IFoo, IBar))]` remains available for local or built-in
objects that are not implementations of a single interface contract. All externally
constructible component objects should instead be returned by a generated
factory method.

Mutable interface methods receive `&mut self`. Generated access gating rejects
conflicting and reentrant mutable calls with `NC_E_BUSY`; ordinary state should
not add `Cell` or `RefCell` merely to emulate mutability behind `&self`.

## Factory implementations and plugin roots

`#[com_factory(Factory)]` implements one generated IDL factory and proves
that the Rust type matches that exact factory interface. Unit factories gain
the default `ComFactoryBootstrap` implementation. Stateful roots implement the
trait explicitly and return the associated typed `ComPtr`.

```rust,ignore
#[com_factory(my_contracts::UiFactory)]
struct UiFactoryObject;

neocogi_com::factory_plugin! {
    name = "example.ui",
    version = "1.0.0",
    factory = UiFactoryObject,
}

#[cfg(feature = "dynamic-plugin")]
neocogi_com::export_factory_plugin!();
```

`factory_plugin!` creates immutable Rust-accessible metadata. The final dynamic
library calls `export_factory_plugin!` exactly once to export
`nc_get_factory_plugin_description_v1`. Static assemblies call
`factory_plugin_description()` directly. The descriptor contains one fixed
bootstrap function pointer for the root factory; there is no per-object
constructor table or erased argument frame.

## Interface and value lowering

`#[com_interface]` generates an interface marker and C-compatible vtable, one
typed `IFooClient` method view, exact ABI signature metadata, method
descriptions, and the audited `unsafe impl ComInterface`. IDL-generated
bindings use the same machinery. Both `ComPtr<IFooAbi>` and
`ComRef<'_, IFooAbi>` lend the client view, and derived clients dereference to
their base clients. The macro therefore emits each safe Rust-to-ABI lowering
once and requires no interface-specific extension-trait import at call sites.

Supported methods return `Result<T, NcStatus>`:

- plain parameters and returns implement `AbiType`;
- `Result<()>` lowers to status only;
- a non-void return uses an output pointer written only on success;
- `ComRef<'_, IFooAbi>` is a borrowed interface parameter;
- `ComPtr<IFooAbi>` is an owned interface return;
- bounded copy wrappers lower slices without transferring allocation ownership.

`#[com_struct]` defines fixed C-layout records. `#[com_enum]` defines explicit
tagged payload layouts. `com_int_enum!` defines a transparent integer newtype
that preserves unknown values. `ComDeserialize` and `ComSerialize` derive typed
local value decoding/encoding, separate from runtime object activation.
Record labels emitted by `ComSerialize` are unqualified declaration-name
literals; named enum payload records use their enclosing enum's name. They do
not identify types globally. The binary encoder ignores these labels and writes
fields positionally; generating them requires no `core::any` lookup.

`com_interface` also accepts the generator-owned `threading = local|shared`
and `factory` role. The same expansion owns required sibling projections and
one physical base. `ComInterface` owns identity and threading;
`ComFactoryInterface` is only a role marker. No duplicate object-description
trait or `OBJECT_*` constants are emitted.

Canonical typed ABI signatures are always represented by SHA-256 digests.
Interface identity is the generated IID plus that signature; human-readable
names are diagnostic metadata.

Generated value codecs and method adapters keep authored members/parameters
separate from positional private bindings and qualify their support types. See [generated identifier hygiene](../../../docs/COM_GENERATED_IDENTIFIER_HYGIENE.md)
for the contract, the corruption it prevents, and executable regression coverage.
