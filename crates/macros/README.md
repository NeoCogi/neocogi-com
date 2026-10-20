# neocogi-com-macros

`neocogi-com-macros` implements the procedural macros re-exported by
`neocogi-com`. Application and plugin crates should use those re-exports rather
than depend on this implementation crate directly.

Public contracts should normally be written in `.ncidl` files. The generator
uses the common `com_interface` macro internally; implementation crates use
`com_object` and `plugin`.

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
state and method bodies. Typed facade storage contains one Rust value and the common lifetime header;
shared providers use one borrow gate and exclusive providers use header admission.
`ComObject` supplies the single implementation proof; the macro emits no private
storage companion or per-provider lifecycle bodies. Static tables implement signature-checked
`QueryInterface`; handles carry the allocation and selected table, so interfaces
add no per-object fields. The compiler resolves distinct inherited contracts,
including diamonds, without retaining full inspection metadata in dispatch.
See the [storage decision](../../../docs/contracts/COM_INTERFACE_STORAGE.md) for the
history of the reverted metadata-heavy version and the deliberate handle-size
tradeoff in the current representation.
`into_local(initialized_value)` returns a `LocalCom<Implementation>` when the
creating binary also needs typed access to the owned implementation. Its
`try_borrow` and `try_borrow_mut` guards share the generated COM call gate;
release them before calling an interface that enters the same object. Neither
guard performs a query, allocation, or reference-count operation. A local handle
cannot be recovered safely from an arbitrary foreign interface.
The local owner's `downgrade()` returns `WeakLocalCom<Implementation>`; promoting
it recovers a typed strong local owner while the allocation remains alive. This
works for handwritten and IDL-generated interface contracts and introduces
no additional allocation or borrow gate. IDL defines interfaces and values;
Rust object implementations and plugin acquisition own construction.
Ordinary unit objects have no generated zero-argument constructor.

`#[com_object(interfaces(IFoo, IBar))]` remains available for local or built-in
objects that are not implementations of a single interface contract. All externally
constructible component objects should instead be returned by a generated
interface method.

Mutable interface methods receive `&mut self`. Generated access gating rejects
conflicting and reentrant mutable calls with `NC_E_BUSY`; ordinary state should
not add `Cell` or `RefCell` merely to emulate mutability behind `&self`.

## Factory implementations and plugin roots

A plugin root is an ordinary interface. `#[com_object(Interface)]` owns the Rust
implementation's storage and lifetime; the acquisition function owns initialization.

```rust,ignore
#[com_object(my_contracts::UiFactory)]
struct UiFactoryObject;

#[neocogi_com::plugin(name = "example.ui")]
fn acquire_root() -> Result<neocogi_com::ComPtr<my_contracts::UiFactoryAbi>, neocogi_com::NcStatus> {
    UiFactoryObject::from_value(UiFactoryObject).query()
}
```

`#[plugin]` requires a safe zero-argument Rust function with an explicit
`Result<ComPtr<InterfaceAbi>, NcStatus>` return. It derives metadata from that
interface, defaults version to the Cargo package version, and optionally accepts
`version = "..."`, `plugin_id(...)`, `dependencies = [ID, ...]` and
`neocogi_com = facade_path`. The typed return must match the interface's threading.
Initialization failure leaves the ABI output empty. Success transfers exactly one
reference; acquiring an existing service and constructing a provider are both valid.

`plugin_descriptor()` lends static metadata without acquiring an object. The
provider declares a `dynamic-plugin = []` feature to enable the generated
`nc_get_plugin_descriptor_v1` export in a DLL. Static compositions leave that
feature disabled and register the accessor directly. Define one root per plugin
crate. Provider methods perform parameterized construction with ordinary typed
arguments. No bootstrap traits, object role markers or constructor tables exist.

## Interface and value lowering

`#[com_interface]` generates an interface marker and C-compatible vtable, one
typed `IFooClient` method view, exact ABI signature metadata, method
descriptions, and the audited `unsafe impl ComInterface`. IDL-generated
bindings use the same machinery. Both `ComPtr<IFooAbi>` and
`ComRef<'_, IFooAbi>` lend the client view, and derived clients dereference to
their base clients. The macro therefore emits each safe Rust-to-ABI lowering
once and requires no interface-specific extension-trait import at call sites.

Supported methods return `Result<T, NcStatus>`:

- plain values provide `AbiType` metadata and checked `ComValue` wire conversion;
- copied positions require `ComInlineValue`; native application layout stays local;
- `Result<()>` lowers to status only;
- a non-void return uses an output pointer written only on success;
- `ComRef<'_, IFooAbi>` is a borrowed interface parameter;
- `ComPtr<IFooAbi>` is an owned interface return;
- `WeakComPtr<IFooAbi>` returns one shared weak header, checked without promotion;
- bounded copy wrappers lower slices without transferring allocation ownership.

`#[com_struct]` defines fixed C-layout records. `#[com_enum]` defines explicit
tagged payload layouts. `com_int_enum!` defines a transparent integer newtype
that preserves unknown values. `ComDeserialize` and `ComSerialize` derive typed
local value decoding/encoding, separate from runtime object acquisition.
Record labels emitted by `ComSerialize` are unqualified declaration-name
literals; named enum payload records use their enclosing enum's name. They do
not identify types globally. The binary encoder ignores these labels and writes
fields positionally; generating them requires no `core::any` lookup.

`com_value` fields may contain `WeakComPtr<I>` through the recursive borrowed
value codec. Acquisition checks shared ownership and exact interface identity
before retaining weak bookkeeping. These fields are neither copied inline values
nor persistent object reconstruction. Use IDL for their public declarations.

`com_interface` accepts the generator-owned `threading = local|shared`.
The same expansion owns required sibling projections and one physical base.
`ComInterface` owns identity and threading for every closed interface.

Canonical typed ABI signatures are always represented by SHA-256 digests.
Interface identity is the generated IID plus that signature; human-readable
names are diagnostic metadata.

Generated value codecs and method adapters keep authored members/parameters
separate from positional private bindings, qualify their support types/macros,
and select required trait operations explicitly. Plugin support lookup and default
decoding have independent normal/printed semantic controls. See [generated identifier hygiene](../../../docs/contracts/COM_GENERATED_IDENTIFIER_HYGIENE.md)
for the contract, the corruption it prevents, and executable regression coverage.

Value wire/payload/storage definitions live in anonymous implementation scopes.
Use `<Value as neocogi_com::ComValue>::Wire` or `::Storage` for explicit raw-slot
integration; private sibling helper spellings are no longer generated APIs.
Codec proofs and conversions select their trait explicitly, independently of
same-named application methods/constants. Signature literals need no sibling static.

Executable package tests own their fixtures and ask Cargo for the actual core
and facade package locations. They also run against extracted SDK archives;
`bash crates/com/scripts/test-packages.sh` supplies the matching SDK offline and
checks all shipped test targets. Checkout-only integration suites remain in
repository CI. See [archive validation](../docs/compatibility.md#published-archive-validation).
