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

`#[observer]` on a thread-local interface groups its declared outputs into one
constructor-supplied destination. For `Textbox`, the compiler derives
`TextboxObserver` (one COM method per output) and `TextboxEvent` (an owned Rust
enum). `TextboxEvent::observer(callback)` stores the callback inline in one COM
allocation. It has no channel dependency; a Rust UI shim can capture an mpsc
sender and map snapshots into the owner's message type.

Factories declare `observer: Option<&TextboxObserver>` explicitly. `None` crosses
the ABI as null and allocates no observer. A present pointer undergoes the same
identity checks as a required borrowed interface, and the producer retains it
before publishing the widget. Observer producers have no generated output
setters. Their observer and factory signatures participate in module identity.
Inherited interfaces retain their own output contracts; the attribute groups
only outputs declared on the annotated interface.

`interface` and `factory` share the callable model. `#[message]` marks concrete
structs/enums for checked COM posting. On an unannotated producer, a named output such as `changed: TextChanged;`
generates a one-shot installation closure on `ComPtr` and `ComRef`. Producers
implement the ordinary generated receiver setter and store one optional concrete
COM receiver. `events::bind(output(), sender, mapper)?` replaces its destination and returns
`Result<()>`; there is no connection projection or subscription lifetime.
Application enums stay in Rust. Generated receivers depend on COM alone, with
no consumer queue dependency or separate support-version protocol.

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
copied-value/interface setup needs `macros`; modules declaring `#[message]` also
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

Core capability types `IUnknown` and `INcBytes` may be borrowed as inputs and
returned as owned references without an imported contract module. They cannot
be embedded in copied records or passed as unborrowed inputs. Their exact core
interface identities participate in generated signatures. `Guid` is the copied
`NcGuid` value type. These spellings are reserved and cannot be shadowed.

Methods may explicitly borrow a `#[message]` value as `value: &Message`. Generated
Rust uses `NcMessage<'_, Message>` and the message's concrete checked wire codec;
the callee calls `to_owned()` before retaining strings, arrays, or capabilities.
Unborrowed ownership-bearing message parameters and copied aggregate fields
remain rejected. Imported message parameters honor published Rust module paths.

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
