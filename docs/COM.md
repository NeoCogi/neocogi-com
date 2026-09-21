# neocogi-com

NeoCogi's component layer lets separately built Rust artifacts communicate
through an explicit C-shaped ABI. Rust-like IDL is the source of public contract
truth; generated COM interfaces carry calls, and plugins expose one typed root
factory.

The current core ABI v7 is experimental and may change before ratification.

## Component model

```text
IDL module -> generated Rust types + exact transitive signatures
plugin     -> private implementations + one typed factory descriptor
host       -> compiles contract types and requests the exact factory
runtime    -> registers plugins, checks factories, retains loaded libraries
```

Contract sources are resolved at build time; generated interfaces, vtables,
metadata, and signatures remain in their consumers. No separate contract `.so`, native description
archive, getter, runtime inventory, or event-contract seeding is produced.
`interface` describes callable behavior; `factory` marks construction roots.
An object is private Rust state. Concrete message data defines its published outputs.

The [plain-Cargo sample](../../examples/com-plugin/README.md) builds the same
plugin as a shared library and as an rlib statically linked into a monolith.

See [IDL contract modules](../../docs/IDL_MODULES.md) for authoring and native
builds, [component design](../../docs/COM_DESIGN.md) for responsibility
boundaries, and [ABI v7](docs/abi-v7.md) for the binary model.

## IDL

The COM-owned [IDL compiler](idl) generates checked COM posting for concrete
`#[message]` structs/enums and receiver setters for `name: Message;` outputs.
Producers own one optional typed receiver; consumers call
`events::map(producer.changed(), sender, AppEvent::Changed)?` to install or replace it.
[`neocogi-com-events`](events) connects standard channels to scoped and component dispatch.
There are no producer queues, output projections, subscription leases or registries.
Generated receivers depend on COM alone; standard channel endpoints stay in the consuming binary.

Contracts use `struct`, `enum`, `bitenum`, `interface`, and `factory`. All declarations
and members are public by definition:

```rust
#[version(1)]
struct Extent {
    width: u32,
    height: u32,
}

#[version(1)]
interface IMeasured {
    fn extent(&self) -> Result<Extent>;
}

#[version(1)]
interface ExampleObject: IMeasured {
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
`OUT_DIR`, and publishes the graph through Cargo metadata. Ordinary Cargo
regenerates bindings after an IDL edit; no separate native artifact is built.

## Implementing interfaces

`#[com_object(contract::Interface)]` associates private Rust storage with one
interface contract. `#[com_factory(contract::Factory)]` does the same for a factory:

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

Interfaces own public identity, threading, callable methods, and required
projections. Implementations own fields and method bodies; factory methods are the only public construction path.

Objects exposing several independent projections may use
`#[com_object(interfaces(IFoo, IBar))]`. Generated storage supplies `IUnknown`,
reference counting, signature-checked `QueryInterface`, and call-access checks.
Use `&mut self` for mutation and keep ordinary state in plain fields.

## Publishing a plugin

```rust,ignore
neocogi_com::factory_plugin! {
    name = "example.component",
    version = "1.0.0",
    factory = ExampleFactoryImpl,
}

#[cfg(feature = "dynamic-plugin")]
neocogi_com::export_factory_plugin!();
```

The plugin exports `nc_get_factory_plugin_description_v1`. Descriptor ABI v1
includes plugin identity/version, hard dependencies, the complete factory
IID/signature, threading, and one bootstrap pointer. Static assemblies call
`factory_plugin_description()` directly. Only factories receive generated
zero-argument construction; ordinary objects use `from_value`.

## Host activation

```rust,ignore
let runtime = neocogi_com::Runtime::global();
let loaded = unsafe { runtime.load_plugin_library(path) };
let factory = runtime.activate_plugin_factory::<ExampleFactoryAbi>(
    loaded.plugin_id(),
)?;
let object = factory.create(Extent { width: 10, height: 20 })?;
```

Loading validates the factory descriptor and hard dependencies before one
atomic registration. Plugin dependencies must already be registered; the
runtime never discovers or recursively loads their paths. Required live-load
failure is diagnosed and aborts rather than exposing partial state. Successful
libraries remain mapped until process exit.

Runtime ABI v1 has operation, plugin-registry, inspection, debug, and
caller-scoped persistence tables.
Activation checks the exact factory signature before bootstrap. Unconnected
contract revisions can coexist; incompatible pointer transfers are rejected.
`neocogi-inspect` emits schema 9 with plugin/dependency/root-factory metadata,
without constructing objects. IDL tools retain build-time source inventories.

## ABI rules

- Interface calls are `extern "C"` vtable functions returning `NcStatus`.
- Compatibility requires both generated interface ID and exact ABI signature.
- `ComPtr<T>` owns one reference; `ComRef<'a, T>` borrows a live interface for
  `'a`. Generated interface parameters restrict that borrow to the ABI call.
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

`neocogi-idl` computes one SHA-256 fingerprint from the canonical typed
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
- `neocogi-com-runtime`: plugin catalog, loader, and persistence implementation;
  its Rust library target is `nc_com`.
- `neocogi-com-events-contracts`: generated dispatch and named-output contracts.
- `neocogi-com-events`: standard-channel mappings and component/scoped dispatch adapters.
- `neocogi-idl` under `crates/com/idl`: IDL parser/resolver
  and Cargo build integration shared by contract crates and `cargo-neocogi`.

The facade's defaults (`std`, `macros`) support local objects without linking a
runtime. `runtime` enables clients of the shared `nc_com` library;
`static-runtime` implies `runtime` and links its implementation rlib instead.
See the [feature and deployment table](api/README.md#features-and-scope).
The [core data guide](core/README.md#text-bytes-and-borrowed-storage) distinguishes
unsafe descriptor borrows such as `NcStaticUtf8::as_str` from owned string copies.

This runtime is trusted same-process native code, not a sandbox or plugin
marketplace. Applications own installation, trust, signatures, and path policy.

## Experimental object persistence and named integration

[`neocogi_com::persistence`](runtime/PERSISTENCE.md) provides runtime-owned opt-in semantic snapshots and explicit,
context-bound factory reconstruction. The portable value codec remains separate
from object activation; arbitrary private object memory is never serialized.
[`events`](events/README.md) also supports named outputs on opaque components.
Generated receiver adapters keep concrete payload types and direct delivery,
while application code binds only to a public name and its message contract.

## Checking the documentation

Run these commands from the workspace root:

```sh
cargo test -p neocogi-com-core -p neocogi-com --doc --all-features
python3 crates/com/scripts/check-markdown-links.py
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features \
  -p neocogi-com-core -p neocogi-com -p neocogi-com-macros \
  -p neocogi-com-runtime \
  -p neocogi-com-events-contracts -p neocogi-com-events \
  -p neocogi-idl
```

The facade's runnable README examples participate in doctests. Examples marked
`ignore` are integration sketches requiring application-specific contracts or
plugin paths. `--all-features` uses static runtime linkage for these checks;
the [plain-Cargo fixture](../../examples/com-plugin/README.md) separately checks
real shared and monolithic deployments.
