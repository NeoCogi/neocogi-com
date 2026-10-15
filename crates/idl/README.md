# COM-owned NeoCogi IDL compiler

The compiler owns contract parsing, canonical transitive ABI signatures, and
Rust binding generation. It runs during ordinary Cargo builds and is independent
of the optional `cargo-neocogi` module-authoring tool.

The single `neocogi-idl` package separates responsibilities into Rust modules:

- `model`, `parser`, and `resolve` parse files/module directories, resolves copied values
  and interfaces, validates threading/required projections, and fingerprints
  the complete source graph.
- `cargo` discovers sources and exchanges schema-5 source graphs through
  Cargo `links`/`DEP_*`. Build scripts use `neocogi_idl::cargo::Module`.
- `generate` emits formatted Rust bindings with fixed declaration signatures.
  Its pure entrypoints perform no Cargo environment access.
- `tools/idl/example-contracts` and `example-consumer-contracts` cover standalone
  and imported contracts. [examples/com-plugin](../../../examples/com-plugin)
  demonstrates their dynamic and monolithic use.

Structs, enums, and ordinary interfaces support type parameters. One expansion
pass substitutes closed uses into concrete declarations with deterministic
identities before ABI resolution. Fields, recursive layouts, and method slots
then follow exactly the same rules as nongeneric declarations. `class`
marks a concrete object contract, `interface` a reusable capability, and `factory`
a construction root. Records and enums share recursive checked conversion;
there is no separate message declaration category.

Construction-bound delivery uses the ordinary library enum
`neocogi.com.events.EventBinding` as a borrowed constructor argument. Its
`Observed(ComPtr<IEventReceiver>)` case supplies the common receiver;
`Unobserved` chooses permanent non-delivery. The factory retains the receiver
before returning its product. There is no Rust sender or closure in the ABI.

Producers advertise event interfaces explicitly using `#[events(IChanged, ...)]`.
Resolution verifies immutable getters and `IEvent` ancestry; generation exposes
`IDL_INTERFACE.events` and its reachable value graph. No constructor-derived helpers
or native payload associations are emitted. See [event metadata](../../../docs/contracts/IDL_EVENT_METADATA.md).
`com_object(Contract)` supplies class allocation without later receiver attachment.

All contracts, including events, are compile-time definitions. The build helper
emits no native artifact, exported getter, runtime module graph, or built-in
registration hook. Only generated layouts, wrappers, and exact signatures are
compiled into consumers. Source-part names and documentation do not affect ABI
fingerprints; ordered members and transitive dependency revisions do. Generic
parameter references are encoded by declaration position, so consistently
renaming a binder preserves the contract while changing which parameter is used
does not. See the [parameter-position contract](../../../docs/contracts/IDL_GENERIC_VALUES.md#parameter-positions-and-contract-signatures)
for examples, regeneration requirements, and cross-module COM regressions.

See [IDL modules](../../../docs/contracts/IDL_MODULES.md) for syntax and package setup.

## Contract crate setup

For a module named `example.widgets`, place its source parts under
`idl/example/widgets/`. Top-level declaration order and source-part filenames
do not change signatures; member order inside a declaration does.

```toml
[package]
name = "example-widgets-contracts"
version = "0.0.0"
edition = "2024"
links = "nc_contract_example_widgets"

[package.metadata.neocogi-contract]
module = "example.widgets"
source = "idl/example/widgets"

[dependencies]
neocogi-com = { version = "0.0.0", default-features = false, features = ["macros"] }

[build-dependencies]
neocogi-idl = "0.0.0"
```

The version selectors assume published matching packages; inside this checkout
use the corresponding workspace dependencies or local paths. This minimal
copied-value/interface setup needs `macros`; modules with owned values also
need the facade's `alloc` feature (or `std`) for owned value decoding. `build.rs` owns generation:

```rust,no_run
/// Generates this contract crate's bindings during its ordinary Cargo build.
fn main() -> Result<(), neocogi_idl::BuildError> {
    // Resolve this directory's module and its imported source graphs through Cargo.
    neocogi_idl::cargo::Module::directory("idl", "example/widgets").build()?;
    Ok(())
}
```

The contract's `src/lib.rs` includes the generated bindings:

```rust,ignore
#![no_std]
include!(concat!(env!("OUT_DIR"), "/example_widgets.include.rs"));
```

`Module::new("idl", "example/widgets.ncidl")` selects a single-file module.
`Module::directory` recursively reads a module's `.ncidl` parts. Both validate
the exact Cargo `links` name and consume imported graph paths from `DEP_*`.
`ModuleSources` provides the same file/directory loading for tools; `generate_rust`
works on an already resolved graph without Cargo environment variables.

The entry file (`BuildOutput::include`, Cargo `idl_include`) loads the definitions
as a normal private module and re-exports them. This lets editor navigation reach
the generated `.rs` rather than stopping at the include macro. Message allocation
paths resolve within the generated module and need no crate-root `alloc` import.

Every build also emits `<module_stem>.rs.map`, exposed as `BuildOutput::source_map`
and Cargo `idl_source_map` metadata. It maps the final formatted Rust's declarations,
fields, variants, methods and derived helpers to their IDL origins. Generated Rust
includes readable `IDL source: path:line:column` documentation. For in-memory tools,
`generate_rust_with_source_map` returns the Rust and map together; explicit Rust
locations use `generate::generate_rust_with_paths_and_source_map`. These provenance
artifacts do not affect ABI identity. See [source mapping](../../../docs/guides/IDL_SOURCE_MAPS.md)
for schema, coordinates, path resolution and macro-expansion limits.

The resolver gives IDL declarations a UUID derived from their qualified name and
binary version, or the explicit UUID from `#[id]`. Generated macros receive it as
`assigned_guid`. This differs from the low-level macro's `auto_guid` path, which
hashes a qualified-name string directly. The declaration signature then includes
the module fingerprint and declaration UUID; runtime calls compare those exact
identity/signature bytes.

`ComPtr<I>` is a compiler-known ownership constructor whose argument must be a
closed interface, class, or factory contract. Fields and owned method results
must use it explicitly; a bare object contract is not a stored value. Inputs
borrow an object as `&I`; direct `ComPtr<I>` inputs are rejected because the IDL
does not define ownership transfer for that position. Records containing object
references may be borrowed as `&Record` and recursively acquired by the receiver.

`WeakComPtr<I>` names shared weak ownership in fields and method results. Its
target follows the same closed-contract and threading rules as `ComPtr<I>`.
Fields may nest through arrays, sequences, options and generic values; pass such
records by reference. Direct weak inputs and copied weak values are rejected.
Acquisition validates the exact interface and retains only weak bookkeeping,
including when the implementation has already expired. Weak fields have no
implicit persistence codec and retain their ownership in design metadata.

`LocalCom<T>::downgrade()` instead creates `WeakLocalCom<T>` for the constructing
Rust binary. IDL-generated implementations support it through their ordinary
`into_local` constructor. `T` is a concrete implementation and never an IDL wire
type: project to an interface and downgrade that pointer when a contract needs
`WeakComPtr<I>`. Foreign interfaces cannot recover typed local access.

Core capabilities `IUnknown`, `INcBytes`, and `INcString` need no imported module.
For example, `Result<ComPtr<INcString>>` returns an owned COM string object;
`String` in a record is owned text data. `Result<String>` is rejected rather than
silently changing its meaning to an object. Exact core interface signatures are
included in contract signatures. `Guid` is the copied `NcGuid` type. These names
and `ComPtr` are reserved and cannot be shadowed.

Methods may borrow any named value as `value: &Record`. Generated Rust accepts
`NcValueRef<'_, Record>`; `NcValueRef::new(&value)?` retains the value and typed
wire storage through the call, and `to_owned()` validates and acquires retained
contents. `#[message]` has been removed and is rejected.

Scalar fields, records, enum payloads, fixed arrays, options, and sequences share
one recursive codec. Sequence fields use `Vec<T>` in both IDL and generated Rust;
aggregate elements are supported. `Option<Self>` is an invalid inline cycle,
while a `Vec<Self>` field provides explicit indirection. Booleans use checked bytes
even inside ordinary copied records, enum payloads, and method buffers.

Copied method slots and buffer elements accept scalars, finite copied
records/enums/bit sets, and arrays whose elements are recursively copied. Values containing strings, sequences or object fields must be borrowed through a named record/enum. Unsupported
copied positions fail during IDL resolution. See the normative
[recursive value contract](../../../docs/contracts/RECURSIVE_VALUE_CONTRACT.md) for limits,
ownership, wire memory obligations, and examples.

## Publishing contracts inside an existing Rust crate

A logical IDL module does not have to occupy a dedicated Rust crate. An owner
can use `Module::directory(...).rust_path("shared_api::contracts").build()` to
publish an explicit crate-rooted Rust path in its schema-5 Cargo source graph.
Dependent generators use that path for imported values, interfaces, and message
receivers. The owner still declares the logical module's existing `links` value.
Without an override, the conventional `<module>_contracts` crate name applies.

Rust paths are backend metadata: they never participate in resolution, UUIDs,
or ABI fingerprints. Conflicting owners and malformed paths fail generation.
The pure `generate::generate_rust_with_paths` API accepts a validated
`RustModulePaths` mapping for callers outside Cargo; it requires no global state.

COM owns the [binding-path grammar](../core/README.md#descriptor-text-validation)
through `validate_rust_path`. IDL calls it for explicit/conventional module paths
and complete published type paths; `syn` then builds the compiler's syntax tree.
Catalog producers and readers call the same COM validator. Paths have a 1024-byte
maximum and use ASCII identifiers, optional leading `::`, and legal `r#` escapes.
For example, `.rust_path("custom_contracts::r#type")` supports a handwritten
`pub mod r#type` containing generated declarations. Unescaped reserved keywords,
relative anchors, generic arguments, whitespace and Unicode are rejected at the
publication boundary. This is an explicit binding metadata subset of Rust 2024.
Application compilation still resolves dependencies, visibility and actual types;
syntax validation does not prove that a named type implements its advertised COM
contract. Logical dotted IDL names keep their separate grammar and identity.

Signed `i8`, `i16`, and `i32` enum representations can use negative
discriminants. The resolver checks their range and duplicate values before
generation. Only wholly fieldless enums may assign values; payload enums use
zero-based declaration-order tags and reject assignments on every variant,
including unit cases and unused generic templates.

The resolver reserves interface ABI, vtable, client, and provider companion
names before code generation. Collisions receive source-located diagnostics.
Payload enums use their native Rust type in copied fields, methods and buffers.
They do not emit public `FooAbi`, `FooTag`, `FooPayload` or per-variant application
records; those names remain available to authored declarations. Their actual
checked `ComValue::Wire` helpers start with `__` and cannot collide with IDL's
UpperCamel declarations. Rust enum layout never crosses COM.

Records, enums and bit sets acquire symmetric persistence automatically when
all reachable fields support it. Live COM references remain transportable but
cannot recreate their implementation objects from saved values, so containing
types have neither persistence trait. Payload enums use the ordinary variant
protocol; fieldless enums and bit sets preserve their raw scalar values.
Record-only `#[persistence(default)]` retains an omitted trailing suffix from a
host-authored Rust Default. Every NCB record is positional; `persistence(sequence)`
and the old decoder opt-ins are rejected. Both generic and concrete
contracts share canonical module encoding v10, including authored assignments,
versions, trailing-default policy and positional generic binders.

`Option<T>` comes from the bundled [ordinary IDL enum](stdlib/neocogi/lang.ncidl).
It follows lexical lookup and normal specialization; copied payloads permit
copied calls, and owned payloads require a borrow of the whole enum. See the
[generic value guide](../../../docs/contracts/IDL_GENERIC_VALUES.md) and
[compiled example](examples/generic_values.ncidl) for collection constructors,
borrowed optional COM capabilities, and the remaining explicit limitations.

## Directional method buffers

Readable `&[T]` lowers to `NcCopyFromBufferOffset`; `&mut [T]` and
`&mut Slice<T>` lower to fresh `NcOutputWriter` provider parameters and
`NcOutput` client arguments. Output expands to offset, capacity, wire pointer,
and initialized-prefix count pointer in ABI generation 17. Existing destination
contents are never input. A transformation needs an explicit readable argument
and a separate output. Recursive checked wire types remain authoritative; Rust
containers, enum layout, and persistence serialization do not cross these slots.
See [the output contract](../../../docs/contracts/COM_OUTPUT_BUFFERS.md).
