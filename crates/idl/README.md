# COM-owned NeoCogi IDL compiler

The compiler owns contract parsing, canonical transitive ABI signatures, and
Rust binding generation. It runs during ordinary Cargo builds and is independent
of the optional `cargo-neocogi` assembly driver.

The single `neocogi-idl` package separates responsibilities into Rust modules:

- `model`, `parser`, and `resolve` parse files/module directories, resolves copied values
  and interfaces, validates roles/threading/required projections, and fingerprints
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
`neocogi.com.events.EventBinding<E>` as a borrowed constructor argument. Its
`Observed(ComPtr<IMessageSink<E>>)` case supplies the one typed receiver;
`Unobserved` chooses permanent non-delivery. The factory retains the receiver
before returning its product. There is no Rust sender or closure in the ABI.

For methods returning `ComPtr<Product>` in the product's contract module, the
backend derives `ProductEventBinding`, `ProductSinkAbi`, and `ProductEventPort`
helpers from that argument and publishes `IDL_INTERFACE.event_sink`. That
metadata describes the **receiver**, not a projection implemented by Product.
Its event root and complete reachable value graph remain available to COM
catalogs. Multiple event arguments or constructor disagreement about the same
product's event type are rejected. Keeping the association in the declaring
module prevents downstream imports from changing its published metadata.
`com_class(Contract)` supplies ordinary class allocation, with no event field
annotation or later attachment method. See [the architecture guide](../../../docs/CLASS_EVENT_STREAMS.md).

All contracts, including events, are compile-time definitions. The build helper
emits no native artifact, exported getter, runtime module graph, or built-in
registration hook. Only generated layouts, wrappers, and exact signatures are
compiled into consumers. Source-part names and documentation do not affect ABI
fingerprints; ordered members and transitive dependency revisions do.

See [IDL modules](../../../docs/IDL_MODULES.md) for syntax and package setup.

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
copied-value/interface setup needs `macros`; modules declaring also
need the facade's `alloc` feature (or `std`) for their generated receiver objects
and owned payload decoding. `build.rs` owns generation:

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
artifacts do not affect ABI identity. See [source mapping](../../../docs/IDL_SOURCE_MAPS.md)
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
[recursive value contract](../../../docs/RECURSIVE_VALUE_CONTRACT.md) for limits,
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
generation; generated unit and payload enums retain the signed tag.

The resolver reserves every visible Rust companion emitted by generated
interfaces and payload enums before code generation. This includes interface
ABI, vtable, client, and provider types; and
payload enum tag, union, and variant records. Local declarations or explicit
imports that collide with these names receive source-located IDL diagnostics
instead of duplicate-item errors from generated Rust. Copyable message enums
also emit the tagged-enum companions; owned message wire helpers start with
`__` and cannot collide with the IDL's UpperCamel declarations.

Records and enums can request `#[deserialize]`, validated recursively before
Rust generation. `#[deserialize(if_supported)]` makes that capability conditional
for library values; it does not weaken an enclosing required decoder. Enum
payloads use the ordinary named variant protocol, including owned and recursive
payloads. Record-only `default` and `sequence` policies remain explicit.

`Option<T>` comes from the bundled [ordinary IDL enum](stdlib/neocogi/lang.ncidl).
It follows lexical lookup and normal specialization; copied payloads permit
copied calls, and owned payloads require a borrow of the whole enum. See the
[generic value guide](../../../docs/IDL_GENERIC_VALUES.md) and
[compiled example](examples/generic_values.ncidl) for collection constructors,
borrowed optional COM capabilities, and the remaining explicit limitations.
