# neocogi-com

NeoCogi's component layer lets separately built Rust artifacts communicate
through an explicit C-shaped ABI. Rust-like IDL is the source of public contract
truth; generated COM interfaces carry calls, and plugins expose one typed root
factory.

The [current COM contract](abi-v1.md) is experimental and may change before ratification.

## Component model

```text
IDL module -> generated Rust types + exact transitive signatures
plugin     -> private implementations + descriptor for one typed root factory
host       -> compiles contract types and requests the exact factory
runtime    -> registers plugins, checks factories, retains loaded libraries
```

Contract sources are resolved at build time; generated interfaces, vtables,
metadata, and signatures remain in their consumers. No separate contract `.so`, native description
archive, getter, runtime inventory, or event-contract seeding is produced.
`interface` describes callable behavior and required projections. Implementations
own private Rust state through `#[com_object]`; `#[plugin]` publishes a typed
acquisition function. Ordinary methods construct parameterized objects. A domain
may define its own observer contracts; UI supplies its own event protocol.

The [plain-Cargo sample](../examples/com-plugin/README.md) builds the same
plugin as a shared library and as an rlib statically linked into a monolith.

See [IDL contract modules](contracts/IDL_MODULES.md) for authoring and native
builds, [component design](architecture/COM_DESIGN.md) for responsibility
boundaries, and [the current ABI specification](abi-v1.md) for the binary model.

## IDL

The COM-owned [IDL compiler](../crates/idl) generates checked ABIs for ordinary values
and callable interfaces. Domains can define their own observer contracts, as in
the [standalone example](../examples/com-plugin/README.md). UI owns its event
protocol, publication and widget origin policy; see UI event delivery
and owner-scoped origins.

IDL declares public values and interfaces; the [language reference](contracts/IDL_MODULES.md)
owns the accepted syntax. Construction surfaces are ordinary interfaces. Concrete
classes and plugin acquisition are Rust implementation concerns. All IDL
declarations and members are public by definition:

<!-- checked-example: idl -->
```ncidl
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
interface ExampleFactory {
    fn create(&self, extent: Extent) -> Result<ComPtr<ExampleObject>>;
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

`neocogi_com::idl_metadata` supplies static descriptions that tools can read
without depending on the compiler. `InterfaceAbi::IDL_INTERFACE` describes
methods and their argument/result types; `Value::IDL_TYPE` describes fields,
enum cases or bit flags. These records stay local to their defining binary.
Domains own any transported catalog format and its application rules; see
[generated IDL metadata](contracts/IDL_MODULES.md#generated-idl-metadata).

## Implementing interfaces

`#[com_object(contract::Interface)]` associates private Rust storage with one
interface contract, including an interface whose methods create other objects:

<!-- checked-example: implementation -->
```rust,ignore
use neocogi_com::{ComPtr, NcStatus, com_object};
use example_contracts::*;

/// Private state; shared COM ownership protects its allocation.
#[com_object(example_contracts::ExampleObject)]
struct ExampleObjectImpl {
    /// Current dimensions read through IMeasured.
    extent: Extent,
}

impl IMeasured for ExampleObjectImpl {
    /// Copies the current dimensions through the declared value codec.
    fn extent(&self) -> Result<Extent, NcStatus> {
        Ok(self.extent)
    }
}

impl ExampleObject for ExampleObjectImpl {
    /// Changes plain fields under the generated exclusive method borrow.
    fn resize(&mut self, extent: Extent) -> Result<(), NcStatus> {
        self.extent = extent;
        Ok(())
    }
}

/// Stateless root with the generated supplied-host bootstrap.
#[com_object(example_contracts::ExampleFactory)]
struct ExampleFactoryImpl;

impl ExampleFactory for ExampleFactoryImpl {
    /// Publishes one explicitly owned interface with initialized private state.
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

<!-- checked-example: plugin -->
```rust,ignore
/// Acquires one initialized provider; returns its exact interface projection.
#[neocogi_com::plugin(name = "example.component", version = "1.0.0")]
fn acquire_root(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<example_contracts::ExampleFactoryAbi>, neocogi_com::NcStatus> {
    ExampleFactoryImpl::from_value(ExampleFactoryImpl).query()
}
```

The plugin exports `nc_get_plugin_descriptor_v1`. Descriptor ABI v1
includes plugin identity/version, hard dependencies, the complete factory
IID/signature, threading, and one bootstrap pointer. Static assemblies call
`plugin_descriptor()` directly. Both paths borrow the provider-authored
`NcPluginDescriptor`; neither acquires a factory. The runtime later returns
`PluginInfo` for inspection, adding registration origin and omitting bootstrap
code. Every shared object uses `from_value(initialized_value)`. The supplied-host
Rust acquisition function owns initialization and transfers one reference. A root
may be a finished service or an ordinary provider with typed creation methods.

## Host acquisition

For a monolith, enable `neocogi-com/runtime`, directly depend on
`neocogi-com-runtime`, and retain that implementation in the final assembly.
Register the linked provider before requesting its exact factory. The following complete function
accepts access supplied by the host entry and uses the declarations above:

<!-- checked-example: static-host -->
```rust,ignore
/// Executes bootstrap, construction, inherited reads and a mutable COM call.
fn static_example(runtime: neocogi_com::Runtime) -> Result<(), NcStatus> {
    let descriptor = plugin_descriptor();
    // SAFETY: linked plugin code and macro-generated static metadata remain
    // available for the process lifetime, including every returned COM handle.
    unsafe { runtime.register_plugin_descriptor(descriptor) }?;
    let info = runtime.plugin_info(descriptor.plugin_id)?
        .ok_or(neocogi_com::NC_E_NOT_FOUND)?;
    assert_eq!(info.factory.interface, descriptor.root_factory);
    assert_eq!(info.origin, neocogi_com::PluginOrigin::Static);
    let factory = runtime.acquire_plugin_root::<ExampleFactoryAbi>(info.plugin_id)?;
    let mut object = factory.create(Extent { width: 10, height: 20 })?;
    assert_eq!(object.extent()?, Extent { width: 10, height: 20 });
    object.resize(Extent { width: 44, height: 31 })?;
    assert_eq!(object.extent()?, Extent { width: 44, height: 31 });
    Ok(())
}
```

These four marked blocks are extracted verbatim by the IDL documentation test:
IDL resolution and Rust generation feed compilation of the provider, plugin
export, bootstrap and actual calls. The `ignore` fence here avoids pretending
that one Rust block can run without its preceding generated contract.

```sh
cargo test -p neocogi-idl --test documentation
```

Dynamic hosts use `runtime.load_plugin_library(path)` for an application-selected
trusted library, then call `acquire_plugin_root` with its plugin ID. Loading
is unsafe because the library contains native code. The
[plain-Cargo example](../examples/com-plugin/README.md) covers full shared
and monolithic builds rather than leaving that deployment setup implicit.

Loading validates the factory descriptor and hard dependencies before one
atomic registration. Plugin dependencies must already be registered; the
runtime never discovers or recursively loads their paths. Required live-load
failure is diagnosed and aborts rather than exposing partial state. Successful
libraries remain mapped until process exit.

Runtime ABI v1 has one complete plugin-operation table, plus debug and
caller-scoped persistence tables. Lookup and enumeration return the same complete
plugin description, including its factory contract and dependency GUIDs. Loading returns the plugin GUID; inspection
reports its first static/dynamic origin. Separate module identities and module
queries were removed. See [the runtime contract](abi-v1.md#runtime-and-global-registry).
Acquisition checks the exact factory signature before bootstrap. Unconnected
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
- Copied records and payload enums use generated checked wire layouts.
- Variable-size values use call-scoped checked views that the receiver acquires;
  shared byte/string services can also cross as explicit COM owners.
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

## Typed NCB persistence

`ComSerialize` and `ComDeserialize` operate on concrete bounded NCB cursors
inside one Rust artifact. Struct fields are constructed in declaration order;
enums select an ordinal and construct only their typed payload. The
[persistence contract](../crates/core/README.md#experimental-persistent-values) specifies
the format, shared limits, trailing defaults and allocation-free entrypoints.
Other file formats own their own codecs; they do not implement a COM format-adapter
protocol. Live COM calls retain their separate checked wire representation.

## Crates

- `neocogi-com-core`: raw ABI layouts and traits (`no_std` by default).
- `neocogi-com-macros`: thin procedural entrypoints over IDL's shared backend.
- `neocogi-com`: typed object/data helpers and optional supplied-host service clients.
- `neocogi-com-runtime`: plugin catalog, loader, and persistence implementation;
  its Rust library target is `nc_com`.
- `neocogi-idl` under `crates/idl`: IDL parser/resolver, the shared `codegen`
  backend for contracts and Rust implementation macros, and Cargo build integration
  used directly by consuming libraries and applications.

The facade's defaults (`std`, `macros`) support local objects without linking a
runtime. `runtime` enables service clients; every final native/browser host
explicitly links the `nc_com` rlib and supplies access to reusable libraries/plugins.
See the [feature and deployment table](../crates/api/README.md#features-and-scope).
The [core data guide](../crates/core/README.md#text-bytes-and-borrowed-storage) distinguishes
unsafe descriptor borrows such as `NcStaticUtf8::as_str` from owned string copies.

This runtime is trusted same-process native code, not a sandbox or plugin
marketplace. Applications own installation, trust, signatures, and path policy.

## Experimental object persistence and typed integration

[`neocogi_com::persistence`](../crates/runtime/PERSISTENCE.md) provides runtime-owned opt-in semantic snapshots and explicit,
context-bound factory reconstruction. The portable value codec remains separate
from object acquisition; arbitrary private object memory is never serialized.
`events` binds concrete class streams directly. Generated form
fields expose checked widget interfaces, and typed receivers post application
messages without an intermediate component relay.

## Executed compatibility checks

The [compatibility matrix](abi-v1.md#independent-build-compatibility) defines independent host/runtime and
provider builds, mixed compiler/profile policies, relocated execution,
and the separate Miri/sanitizer checks. One standard-library Python driver owns
these checks; shell, PowerShell and CI entry points delegate to it. An ordinary
`cargo build -p neocogi-com-runtime` produces its ordinary host rlib.
`#[com_main]` initializes that implementation; dynamic and static providers share
its registry through supplied C service tables. No runtime DSO or native bridge
is deployed. Generic Cargo artifacts and resource staging still belong to the
native application composition; see the [native guide](guides/COM_NATIVE.md).

## Checking the documentation

The complete maintained quality gate is `bash scripts/test-quality.sh`.
It checks workspace formatting, all SDK targets/features with strict Clippy,
supported allocation modes, ABI snippets, local links and strict rustdoc. The COM
CI job runs that same script without dropping dependency lint coverage. Native
backend and cross-platform driver checks retain their separate CI rows.

Run these commands from the workspace root:

```sh
cargo test -p neocogi-com-core -p neocogi-com --doc --all-features
python3 scripts/check-markdown-links.py
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features \
  -p neocogi-com-core -p neocogi-com -p neocogi-com-macros \
  -p neocogi-com-runtime \
  -p neocogi-idl
```

The facade's runnable README examples participate in doctests. Examples marked
`ignore` are integration sketches requiring application-specific contracts or
plugin paths. `--all-features` leaves implementation selection with each final test owner;
the [plain-Cargo fixture](../examples/com-plugin/README.md) separately checks
real shared and monolithic deployments.
