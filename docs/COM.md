# neocogi-com

NeoCogi's component layer lets separately built Rust artifacts communicate
through an explicit C-shaped ABI. Rust-like IDL is the source of public contract
truth; generated COM interfaces carry calls, and plugins expose one typed root
factory.

The current core ABI v7 is experimental and may change before ratification.

## Component model

```text
IDL module -> generated Rust types + fingerprinted native description library
plugin     -> private implementations + one root factory description
host       -> links generated types and requests that exact factory
runtime    -> validates, registers, inspects, and retains libraries
```

The host and plugin both compile the generated ABI layouts they use. They do not
each own an independent runtime description. Their generated `MODULE` constants
import the canonical getter from the exact fingerprint-qualified contract
library. If they link different revisions of one logical module, the shared
runtime observes the equal module ID and unequal fingerprints and rejects the
combination.

See [IDL contract modules](../../docs/IDL_MODULES.md) for authoring and native
linking, [component design](../../docs/COM_DESIGN.md) for responsibility
boundaries, and [ABI v7](docs/abi-v7.md) for the binary model.

## IDL

Contracts use `struct`, `enum`, `trait`, `class`, and `factory`. All declarations
and members are public by definition:

```rust
#[version(1)]
struct Extent {
    width: u32,
    height: u32,
}

#[version(1)]
trait IMeasured {
    fn extent(&self) -> Result<Extent>;
}

#[version(1)]
class ExampleObject: IMeasured {
    fn resize(&mut self, extent: Extent) -> Result<()>;
}

#[version(1)]
factory ExampleFactory {
    fn create(&self, extent: Extent) -> Result<ExampleObject>;
}
```

The file path supplies the dotted module name. Declaration names preserve exact
casing, so `IWidget` in `neocogi/ui.ncidl` is `neocogi.ui.IWidget`.
`#[version(N)]` derives identity; `#[id(N, "UUID")]` supplies an explicit one.
An enum may select `u8`, `u16`, `u32`, `i8`, `i16`, or `i32` after its name;
the default is `u32`.

Every module is an ordinary Cargo crate with `build.rs`. The build helper tracks
the `.ncidl` source and transitive module graph, generates Rust bindings into
`OUT_DIR`, and builds the correct native contract library. Cargo therefore
regenerates both after an IDL edit.

## Implementing generated descriptions

`#[com_object(contract::Class)]` associates private Rust storage with one IDL
class. `#[com_factory(contract::Factory)]` does the same for a factory:

```rust,ignore
#[com_object(example_contracts::ExampleObject)]
struct ExampleObjectImpl {
    extent: Extent,
}

impl IMeasured for ExampleObjectImpl {
    fn extent(&self) -> Result<Extent, NcStatus> {
        Ok(self.extent)
    }
}

impl ExampleObject for ExampleObjectImpl {
    fn resize(&mut self, extent: Extent) -> Result<(), NcStatus> {
        self.extent = extent;
        Ok(())
    }
}

#[com_factory(example_contracts::ExampleFactory)]
struct ExampleFactoryImpl;

impl ExampleFactory for ExampleFactoryImpl {
    fn create(&self, extent: Extent) -> Result<ComPtr<ExampleObjectAbi>, NcStatus> {
        ExampleObjectImpl::from_value(ExampleObjectImpl { extent }).query()
    }
}
```

Class descriptions own public identity, threading, callable methods, and
required interfaces. Implementations own fields and method bodies. Classes have
no runtime constructors; factory methods are the only public construction path.

Local objects that are not published as IDL classes may use
`#[com_object(interfaces(IFoo, IBar))]`. Generated storage supplies `IUnknown`,
reference counting, signature-checked `QueryInterface`, and call-access checks.
Use `&mut self` for mutation and keep ordinary state in plain fields.

## Publishing a plugin

```rust,ignore
neocogi_com::factory_plugin! {
    name = "example.component",
    version = "1.0.0",
    contracts = [example_contracts::MODULE],
    factory = ExampleFactoryImpl,
}

neocogi_com::export_factory_plugin!();
```

The plugin DSO exports `nc_get_factory_plugin_description`. Its immutable
description includes plugin identity/version, hard plugin dependencies, exact
contract module references, root factory
identity/threading, and one fixed bootstrap pointer. It has no class table,
erased constructor frame, or general-purpose value container.

Static assemblies call `factory_plugin_description()` directly and do not
export a dynamic symbol.

## Host activation

```rust,ignore
let runtime = neocogi_com::Runtime::global();
let loaded = unsafe { runtime.load_plugin_library(path) };
let factory = runtime.activate_plugin_factory::<ExampleFactoryAbi>(
    loaded.plugin_id(),
)?;
let object = factory.create(Extent { width: 10, height: 20 })?;
```

Loading validates the factory description and exact contract graph before one
atomic registration. Plugin dependencies must already be registered; the
runtime never discovers or recursively loads their paths. Required live-load
failure is diagnosed and aborts rather than exposing partial state. Successful
libraries remain mapped until process exit.

Offline inspection uses
`Runtime::inspection().with_plugin_library_description` and never registers the
borrowed plugin graph. `neocogi-inspect` copies it into a factory-native
schema-7 artifact and computes dependency-first assembly plans.

## ABI rules

- Interface calls are `extern "C"` vtable functions returning `NcStatus`.
- Compatibility requires both generated interface ID and exact ABI signature.
- `ComPtr<T>` owns one reference; `ComRef<'_, T>` borrows for one call.
- Both handle kinds lend the same generated client method view; derived client
  views provide inherited lookup without generating another calling trait.
- `Result<()>` lowers without an output slot; other successful returns write
  one explicit output slot.
- Fixed records and enums use generated C layouts.
- Owned strings, bytes, and other variable-size values cross as COM objects.
- Borrowed bulk data uses explicit bounded pointer/count wrappers.
- Rust trait objects, `String`, `Vec<T>`, arbitrary references, allocator
  ownership, unwinding, and `Any` do not cross the ABI.

Object pointers carry either `ThreadLocal` or `ThreadSafe`. The latter is a
positive producer promise enforced on generated implementations with
`Send + Sync`; it can be weakened but not safely upgraded.

`neocogi-idl-build` computes one SHA-256 fingerprint from the canonical typed
module graph, derives a distinct signature for each declaration, and embeds
the final 32-byte values. rustc compiles those constants and the complete typed
descriptors; it does not serialize or hash contracts. Dynamic and monolithic
assemblies therefore perform the same exact identity and signature checks
without a validation-mode build setting.

## Typed source decoding

`ComDeserialize` and `ComDeserializer` provide format-neutral, statically typed
construction inside one Rust artifact. They do not use `Any`, an erased value
tree, runtime class lookup, or descriptor-offset mutation, and they are not a
cross-library marshalling format.

## Crates

- `neocogi-com-core`: raw ABI layouts and traits (`no_std` by default).
- `neocogi-com-macros`: procedural macro implementation.
- `neocogi-com`: user-facing facade and generated object helpers.
- `neocogi-com-runtime-sys`: linked runtime discovery declarations.
- `neocogi-com-runtime`: process-global runtime implementation.
- `neocogi-idl` and `neocogi-idl-build` under `tools/idl`: IDL parser/resolver
  and Cargo build integration shared by contract crates and `cargo-neocogi`.

This runtime is trusted same-process native code, not a sandbox or plugin
marketplace. Applications own installation, trust, signatures, and path policy.
