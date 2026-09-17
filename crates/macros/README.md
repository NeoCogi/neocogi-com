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
state and method bodies. Generated storage contains one canonical `IUnknown`
projection, one header per required interface, reference counting,
signature-checked `QueryInterface`, and `from_value(initialized_value)`.
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
`factory_plugin_description()` directly. There is no class table, generated
constructor function pointer, erased argument frame, or legacy plugin export.

## Interface and value lowering

`#[com_interface]` generates a C-compatible interface header and vtable, one
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
that preserves unknown values. `ComDeserialize` derives typed local decoding;
it is unrelated to runtime activation.

`com_interface` also accepts the generator-owned `threading = local|shared`
and `factory` role. The same expansion owns required sibling projections and
one physical base. `ComInterface` owns identity and threading;
`ComFactoryInterface` is only a role marker. No duplicate object-description
trait or `OBJECT_*` constants are emitted.

Canonical typed ABI signatures are always represented by SHA-256 digests.
Interface identity is the generated IID plus that signature; human-readable
names are diagnostic metadata.
