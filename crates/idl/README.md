# COM-owned NeoCogi IDL compiler

The compiler owns contract parsing, canonical transitive ABI signatures, and
Rust binding generation. It runs during ordinary Cargo builds; no standalone
IDL executable or contract package is required.

The single `neocogi-idl` package separates responsibilities into Rust modules:

- `model`, `parser`, and `resolve` parse files/module directories, resolves copied values
  and interfaces, validates threading/required projections, and fingerprints
  the complete source graph.
- `cargo::SharedSources` generates bindings directly from a shared IDL tree in
  each consuming crate, without Cargo ownership packages. `cargo::Module` also
  supports packaged source dependencies through Cargo `links`/`DEP_*` exchange.
- `generate` lowers bound ownership into typed method/value plans and emits complete
  Rust through this library's `codegen` module, with fixed declaration signatures. Generated
  contracts do not invoke interface/value attribute or scalar declaration macros,
  rewrite formatted declarations, or reparse Rust types to recover their contracts.
  Its pure entrypoints perform no Cargo environment access.
- `codegen` owns the shared Rust emitter and concrete implementation/persistence
  expansion used by `neocogi-com-macros`. The procedural macro crate delegates to
  this ordinary library; IDL never depends on the macro crate or runtime facade.
- The shared example IDL covers standalone and imported contracts. [examples/com-plugin](https://github.com/NeoCogi/neocogi-com/tree/master/examples/com-plugin)
  demonstrates their dynamic and monolithic use.

Structs, enums, and ordinary interfaces support type parameters. One expansion
pass substitutes closed uses into concrete declarations with deterministic
identities before ABI resolution. Fields, recursive layouts, and method slots
then follow exactly the same rules as nongeneric declarations. IDL interfaces
define public capabilities, including construction methods. Concrete classes and
plugin acquisition live in Rust implementations. The [language reference](https://github.com/NeoCogi/neocogi-com/blob/master/docs/contracts/IDL_MODULES.md)
owns the grammar. Records and enums share recursive checked conversion;
there is no separate message declaration category.

## One Rust generation backend

`neocogi_idl::codegen` emits interface traits, clients, physical vtables and
provider thunks from resolved plans. The same backend supplies checked value
conversion, persistence and Rust object/plugin/host-entry expansion. Its method
plans retain authored documentation, ownership and physical argument/result
types together; safe and raw documentation describe their respective types.
Code generation never recovers a contract by reparsing printed Rust.
Live-value case emission is internal to the compiler and consumes already-bound
IDL field types directly for raw wires, typed storage and checked conversion.
Standalone persistence expanders instead infer native field types from the actual
Rust construction destination, preserving private types, Self and opaque macros
without a separate type-resolution helper.

The compiler and its backend are one package. `neocogi-com-macros` remains a
separate `proc-macro` crate because Rust requires that crate type for procedural
entrypoints. Its calls to `neocogi_idl::codegen` reuse the same implementation
as IDL generation. A macro-only consumer therefore also compiles the ordinary
IDL library and its parser dependencies on the build machine. This adds no
runtime dependency or registry and does not change the unreleased ABI-v1 contract.

Core remains independent of the compiler. Its checked-in persistence ABI stays
freshness-checked against the same emitter. Backend emission unit
tests and compiler-dependent executable/name-binding regressions all belong to
the IDL package; run `cargo test -p neocogi-idl` for both groups.

Generated raw vtable fields narrowly allow Clippy's type-complexity lint because
their flattened signatures are the physical IDL calling convention. This does
not cover safe methods, providers or ordinary algorithms. No private ABI aliases
are generated solely to satisfy that lint; other downstream warnings stay checked.

## Shared sources in consuming crates

Keep authoritative `.ncidl` files in namespace trees owned by their subsystems. Each application or
library can generate its own bindings from that tree during its ordinary build:

```rust,no_run
fn main() -> Result<(), neocogi_idl::BuildError> {
    let modules = neocogi_idl::cargo::SharedSources::new("../../schemas")
        .module("example.widgets", "my_app::widgets")
        .build()?;
    println!("cargo::rustc-env=WIDGET_BINDINGS={}", modules[0].include.display());
    Ok(())
}
```

The consuming crate declares `neocogi-idl` as a build dependency and
`neocogi-com` as a normal dependency. It supplies an explicit self alias for the
generated Rust paths and includes its own build output:

```rust,ignore
extern crate self as my_app;
pub mod widgets {
    include!(env!("WIDGET_BINDINGS"));
}
```

The logical name selects `example/widgets.ncidl` or all parts beneath
`example/widgets/`; declaring both forms is an error. Declare reachable modules
with `.module(name, rust_path)` to generate them locally or `.import(name,
rust_path)` to use definitions exposed by an existing Rust library. Imported IDL
still comes from the shared tree and is resolved and fingerprinted in this build.
Use `.include_root("../another-subsystem/schemas")` to resolve dependencies from
additional subsystem trees. Each reachable module must have one source owner;
conflicting roots fail before any output is written. Root order never hides a
definition. Repeating the same canonical root is harmless. Source maps preserve
the actual owning directory of each imported declaration.

Existing inputs and directory membership are tracked by Cargo. Absent candidate
paths watch their nearest existing source directory, so unchanged builds stay
cached while added source parts or competing owners invalidate both consumers.

Both Cargo builders finish all requested Rust generation and encode the source
graph before creating or changing generated output. A rejected complete public
Rust path or generation-worker start therefore preserves prior output, including
bundled navigation copies. Filesystem provenance finishing and publication are
later fallible operations and do not promise rollback on IO failure.
All source maps finish and serialize before any main binding, map, include or
graph file is overwritten. A finishing failure may leave navigation copies or
directories; a failure during final writes can leave partial main output.

No `links`, contract package, runtime state or `DEP_*` variable is needed.

Independent generation produces distinct Rust types with the same physical ABI,
IDs and signatures. COM calls negotiate those exact identities. Ordinary Rust
helper calls still require their declared Rust types: compile helpers beside
local bindings or explicitly import the helper library's definitions. Changing
a Rust path never changes the ABI. Every selected external Rust value/interface
must prove its own resolved declaration identity during compilation; module
constants alone cannot certify a facade's reexports. Inheritance uses the checked
marker's associated client and table. A stale or differently named generated
definition therefore fails compilation before it can supply an incompatible codec
or table. The [binding contract](https://github.com/NeoCogi/neocogi-com/blob/master/docs/contracts/IDL_MODULES.md#shared-idl-sources)
also defines semantic-only templates and unused mappings, which require no
compiled owner.

An authored IDL import of a closed interface reexports its complete existing Rust
family: trait, `Abi`, `VTable`, `VtblFor` and `Client`. The consuming module can
therefore be used directly in `#[com_object(consumer::IBase)]` and
`ComPtr<consumer::IBaseAbi>`/`ComRef<'_, consumer::IBaseAbi>`. No new interface
types, signatures or tables are created. Import generation, source-aware name
admission and Cargo's explicit include exports share one public-name projection.
Value imports remain single-name; open-template imports expose no callable family.

A custom supplying Rust route must expose all five names, including on unused
interface imports. Add missing original exports to partial supplying facades.
This is separate from compatibility: the compiler checks the selected marker's
identity and uses its associated inherited representations. It does not certify
arbitrary separately authored Client/VTable aliases merely from their names.

Source maps retain authored shared-file paths;
output filenames use existing module UUIDs so case and separator variants remain
distinct even on case-insensitive filesystems.

## Packaged source dependencies

For a module named `example.widgets`, place its source parts under
`schemas/example/widgets/`. Top-level declaration order and source-part filenames
do not change signatures; member order inside a declaration does.

```toml
[package]
name = "example-widgets"
version = "0.0.0"
edition = "2024"
# Exact dotted IDL module `example.widgets`, UTF-8 encoded as hex.
# Demangle: decode hex pairs after nc_contract_; this is a Cargo owner, not a .so.
links = "nc_contract_6578616d706c652e77696467657473"

[package.metadata.neocogi-contract]
module = "example.widgets"
source = "schemas/example/widgets"

[dependencies]
neocogi-com = { version = "0.0.0", default-features = false }

[build-dependencies]
neocogi-idl = "0.0.0"
```

The version selectors assume published matching packages; inside this checkout
use the corresponding workspace dependencies or local paths. This minimal
copied-value/interface setup needs no procedural macros; modules with owned values
need the facade's `alloc` feature (or `std`) for owned value decoding. `build.rs` owns generation:

```rust,no_run
/// Generates this library's bindings during its ordinary Cargo build.
fn main() -> Result<(), neocogi_idl::BuildError> {
    // Resolve this directory's module and its imported source graphs through Cargo.
    neocogi_idl::cargo::Module::directory("schemas", "example/widgets").build()?;
    Ok(())
}
```

The library's `src/lib.rs` includes the generated bindings:

```rust,ignore
#![no_std]
include!(concat!(env!("OUT_DIR"), "/example_widgets.include.rs"));
```

`Module::new("schemas", "example/widgets.ncidl")` selects a single-file module.
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
artifacts do not affect ABI identity. See [source mapping](https://github.com/NeoCogi/neocogi-com/blob/master/docs/guides/IDL_SOURCE_MAPS.md)
for schema, coordinates, path resolution and macro-expansion limits.

The resolver gives IDL declarations a UUID derived from their qualified name and
binary version, or the explicit UUID from `#[id]`. The typed backend embeds that
identity directly in the generated ABI. The declaration signature includes
the module fingerprint and declaration UUID; runtime calls compare those exact
identity/signature bytes.

`ComPtr<I>` is a compiler-known ownership constructor whose argument must be a
closed interface contract. Fields and owned method results spell ownership
explicitly; a bare object contract is not a stored value. Inputs
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
[recursive value contract](https://github.com/NeoCogi/neocogi-com/blob/master/docs/contracts/RECURSIVE_VALUE_CONTRACT.md) for limits,
ownership, wire memory obligations, and examples.

## Publishing contracts inside an existing Rust crate

A logical IDL module does not have to occupy a dedicated Rust crate. An owner
can use `Module::directory(...).rust_path("shared_api::contracts").build()` to
publish an explicit crate-rooted Rust path in its versioned Cargo source graph.
Dependent generators use that path for imported values, interfaces, and message
receivers. The owner still declares the logical module's existing `links` value.
Without an override, the conventional `<module>_contracts` crate name applies.

Rust paths are backend metadata: they never participate in resolution, UUIDs,
or ABI fingerprints. Conflicting owners and malformed paths fail generation.
The pure `generate::generate_rust_with_paths` API accepts a validated
`RustModulePaths` mapping for callers outside Cargo; it requires no global state.

COM owns the [binding-path grammar](https://github.com/NeoCogi/neocogi-com/blob/master/crates/core/README.md#descriptor-text-validation)
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
contracts share canonical module encoding v13, including authored assignments,
versions, trailing-default policy and positional generic binders.

`Option<T>` comes from the bundled [ordinary IDL enum](stdlib/neocogi/lang.ncidl).
It follows lexical lookup and normal specialization; copied payloads permit
copied calls, and owned payloads require a borrow of the whole enum. See the
[generic value guide](https://github.com/NeoCogi/neocogi-com/blob/master/docs/contracts/IDL_GENERIC_VALUES.md) and
[compiled example](examples/generic_values.ncidl) for collection constructors,
borrowed optional COM capabilities, and the remaining explicit limitations.

## Directional method buffers

Readable `&[T]` lowers to `NcCopyFromBufferOffset`; `&mut [T]` and
`&mut Slice<T>` lower to fresh `NcOutputWriter` provider parameters and
`NcOutput` client arguments. Output expands to offset, capacity, wire pointer,
and initialized-prefix count pointer in ABI v1. Existing destination
contents are never input. A transformation needs an explicit readable argument
and a separate output. Recursive checked wire types remain authoritative; Rust
containers, enum layout, and persistence serialization do not cross these slots.
See [the output contract](https://github.com/NeoCogi/neocogi-com/blob/master/docs/contracts/COM_OUTPUT_BUFFERS.md).
