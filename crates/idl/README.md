# COM-owned NeoCogi IDL compiler

The compiler owns contract parsing, canonical transitive ABI signatures, and
Rust binding generation. It runs during ordinary Cargo builds and is independent
of the optional `cargo-neocogi` assembly driver.

- `core` (`neocogi-idl`) parses files/module directories, resolves copied values
  and interfaces, validates roles/threading/required projections, and fingerprints
  the complete source graph.
- `build` (`neocogi-idl-build`) discovers sources, exchanges schema-4 source graphs
  through Cargo `links`/`DEP_*`, and emits formatted Rust bindings with fixed
  declaration signatures.
- `tools/idl/example-contracts` and `example-consumer-contracts` cover standalone
  and imported contracts. [examples/com-plugin](../../../examples/com-plugin)
  demonstrates their dynamic and monolithic use.

`interface` and `factory` share the callable model. `#[message]` marks concrete
structs/enums for checked COM posting. A named output such as `changed: TextChanged;`
generates a one-shot installation closure on `ComPtr` and `ComRef`. Producers
implement the ordinary generated receiver setter and store one optional concrete
COM receiver. `events::map(output(), sender, mapper)?` replaces its destination and returns
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
version = "0.1.0"
edition = "2024"
links = "nc_contract_example_widgets"

[package.metadata.neocogi-contract]
module = "example.widgets"
source = "idl/example/widgets"

[dependencies]
neocogi-com = { version = "0.1.0", default-features = false, features = ["macros"] }

[build-dependencies]
neocogi-idl-build = "0.1.0"
```

The version selectors assume published matching packages; inside this checkout
use the corresponding workspace dependencies or local paths. This minimal
copied-value/interface setup needs `macros`; modules declaring `#[message]` also
need the facade's `alloc` feature (or `std`) for their generated receiver objects
and owned payload decoding. `build.rs` owns generation:

```rust,no_run
/// Generates this contract crate's bindings during its ordinary Cargo build.
fn main() -> Result<(), neocogi_idl_build::BuildError> {
    // Resolve this directory's module and its imported source graphs through Cargo.
    neocogi_idl_build::Module::directory("idl", "example/widgets").build()?;
    Ok(())
}
```

The contract's `src/lib.rs` includes the generated bindings:

```rust,ignore
#![no_std]
include!(concat!(env!("OUT_DIR"), "/example_widgets.rs"));
```

`Module::new("idl", "example/widgets.ncidl")` selects a single-file module.
`Module::directory` recursively reads a module's `.ncidl` parts. Both validate
the exact Cargo `links` name and consume imported graph paths from `DEP_*`.
`ModuleSources` provides the same file/directory loading for tools; `generate_rust`
works on an already resolved graph without Cargo environment variables.

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
