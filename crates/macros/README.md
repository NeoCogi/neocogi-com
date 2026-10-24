# neocogi-com-macros

`neocogi-com-macros` implements the procedural macros re-exported by
`neocogi-com`. Application and plugin crates should use those re-exports rather
than depend on this implementation crate directly.

Public contracts are written in `.ncidl` files. `neocogi-idl` passes bound typed
plans to its `codegen` module, which emits their complete Rust ABI. This thin
`proc-macro` crate calls the same library for `com_object`, `plugin`, `com_main`,
`ComSerialize`, and `ComDeserialize`. It owns no second emitter; IDL does not
depend on this crate. Macro expansion compiles the compiler library on the build
machine and adds no runtime compiler dependency.

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
See the [storage decision](https://github.com/NeoCogi/foundations/blob/master/docs/contracts/COM_INTERFACE_STORAGE.md) for the
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
works for generated interface contracts and introduces
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
fn acquire_root(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<my_contracts::UiFactoryAbi>, neocogi_com::NcStatus> {
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

## Contract and persistence ownership

IDL owns interface identity, signatures, required projections and copied/owning
value classification. The typed backend generates explicit C wires, one borrowed
client view and checked provider thunks without parsing Rust wrappers to recover
that information. Native records/enums have ordinary Rust application layout;
associated `ComValue::Wire` and `Storage` are their raw integration boundary.
The recursive identity-borrow proof checks size, alignment, field offsets and
content validity. There are no COM packing/alignment/external-field modes.
External native adapters explicitly convert to a generated contract's wire.

`ComSerialize` and `ComDeserialize` derive standalone positional NCB codecs.
Fields follow declaration order; variants follow ordinals. No field labels or
type names are serialized. `#[com_deserialize(default)]` retains trait defaults
only for absent trailing fields; malformed present data remains an error.
Packed native records cannot derive field-borrowing serialization: implement an
explicit codec that copies chosen fields into aligned local values instead.
IDL emits the same codec machinery directly for eligible declaration graphs.

Canonical typed ABI signatures are SHA-256 digests computed by IDL. Callable
identity is the generated IID plus that signature; names serve inspection.
Rust implementation macros never choose an interface revision or encode a schema.

Generated value codecs and method adapters keep authored members/parameters
separate from positional private bindings, qualify their support types/macros,
and select required trait operations explicitly. Plugin support lookup and default
decoding have independent normal/printed semantic controls. See [generated identifier hygiene](https://github.com/NeoCogi/foundations/blob/master/docs/contracts/COM_GENERATED_IDENTIFIER_HYGIENE.md)
for the contract, the corruption it prevents, and executable regression coverage.

Value wire/payload/storage definitions live in anonymous implementation scopes.
Use `<Value as neocogi_com::ComValue>::Wire` or `::Storage` for explicit raw-slot
integration; private sibling helper spellings are no longer generated APIs.
Codec proofs and conversions select their trait explicitly, independently of
same-named application methods/constants. Signature literals need no sibling static.
Provider/client generics and executable bindings are allocated against authored
syntax. Anonymous implementation scopes protect pattern lookup from ambient
constants even after printing. Plugin helper items stay behind `plugin_descriptor()`;
object tables live directly in `ComObject::VTABLES`. Intentional public companion
and dense-enum names share one core admission/emission policy.
Private lexical field projections preserve application `Self`, opaque type macros
and private native fields; no public field-resolution API is added.

Executable package tests own their fixtures and ask Cargo for the actual core
and facade package locations. They also run against extracted SDK archives;
`bash crates/com/scripts/test-packages.sh` supplies the matching SDK offline and
checks all shipped test targets. Checkout-only integration suites remain in
repository CI. See [archive validation](https://github.com/NeoCogi/foundations/blob/master/crates/com/docs/abi-v1.md#published-archive-validation).
