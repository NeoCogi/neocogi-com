# neocogi-com

`neocogi-com` is the user-facing Rust facade for NeoCogi's explicit C-shaped
component ABI. It provides typed COM pointers, generated interface/object
support, factory-only plugins, and an optional facade over the linked
`libnc_com`.

Public cross-library contracts should be authored as Rust-like `.ncidl`
modules. See [IDL contract modules](../../../docs/IDL_MODULES.md) for their
source, crate, signature, and tooling model. The
[core ABI v7 specification](../docs/abi-v7.md) defines the binary boundary.

## Local object example

```rust
use neocogi_com::{ComPtr, IUnknown, NcStatus, com_interface, com_object};

// Direct macro use is for low-level fixtures; production contracts use NCIDL.
#[com_interface(auto_guid = "example.Calculator:v1", assigned_signature = b"00000000000000000000000000000001")]
pub trait ICalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
}

#[com_object(interfaces(ICalculator))]
pub struct Calculator;

impl ICalculator for Calculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus> {
        Ok(a + b)
    }
}

fn calculate() -> Result<i32, NcStatus> {
    let unknown: ComPtr<IUnknown> = Calculator::from_value(Calculator);
    unknown.query::<ICalculatorAbi>()?.add(2, 3)
}
```

`com_interface` generates the portable header, vtable, ABI signature, and one
borrowed `IFooClient` method view. Both `ComPtr<IFooAbi>` and
`ComRef<'_, IFooAbi>` lend that view through `Deref`, so ordinary calls use the
same `pointer.method()` syntax without importing an interface-specific trait.
A derived client view also dereferences to its base client view for inherited
method lookup. Obtaining an owned base projection still requires checked
`QueryInterface`.

`com_object` generates reference-counted storage and signature-checked
`QueryInterface`. It does not publish the object as something the runtime can
construct. Handwritten helpers such as `INcStringComPtrExt::to_string` remain
separate because they add bounded copying, allocation, and UTF-8 validation
rather than duplicate a generated interface method.

## Boundary rules

- Methods return `Result<T, NcStatus>`; `Result<()>` is the status-only form.
- Mutable operations use `&mut self` and the generated exclusive call gate.
- Use fixed-width ABI values and generated records/enums.
- Use `ComRef<'_, IFooAbi>` for borrowed interface parameters and
  `ComPtr<IFooAbi>` for owned interface returns.
- Use bounded copy views for call-scoped buffers and COM objects such as
  `INcStringAbi` or `INcBytesAbi` for owned variable-size data.
- Never expose Rust trait objects, `String`, `Vec<T>`, slices, arbitrary
  references, allocator ownership, panics, or `Any` across the ABI.
- Interface compatibility requires both the generated IID and exact ABI
  signature. Names and versions are diagnostic information.

Objects are thread-local by default. An IDL interface or factory declared
thread-safe produces `ComPtr<T, ThreadSafe>` and its implementation must satisfy
`Send + Sync`. A safe pointer may be weakened to thread-local; it cannot be
upgraded.

## IDL implementation and factory plugin

Given a generated contract crate, implementation code has this shape:

```rust,ignore
use my_contracts::{Button, ButtonAbi, UiFactory};
use neocogi_com::{ComPtr, NcStatus, com_factory, com_object};

#[com_object(my_contracts::Button)]
struct ButtonObject {
    label: String,
}

impl Button for ButtonObject {
    // IDL methods are implemented with ordinary typed Rust signatures.
}

#[com_factory(my_contracts::UiFactory)]
struct UiFactoryObject;

impl UiFactory for UiFactoryObject {
    fn create_button(&self) -> Result<ComPtr<ButtonAbi>, NcStatus> {
        ButtonObject::from_value(ButtonObject { label: String::new() }).query()
    }
}

neocogi_com::factory_plugin! {
    name = "example.ui",
    version = "1.0.0",
    factory = UiFactoryObject,
}

#[cfg(feature = "dynamic-plugin")]
neocogi_com::export_factory_plugin!();
```

`#[com_object(Interface)]` connects private storage to one interface contract.
`#[com_factory(Factory)]` adds the factory-role proof and typed bootstrap.
The plugin descriptor stores plugin metadata, full root identity/signature,
threading, and activation. Ordinary objects require an initialized value;
only factories receive a generated zero-argument constructor.

## Runtime flow

With the `runtime` feature, dynamic hosts use:

```rust,ignore
let runtime = neocogi_com::Runtime::global();
let loaded = unsafe { runtime.load_plugin_library(path) };
let factory = runtime.activate_plugin_factory::<my_contracts::UiFactoryAbi>(
    loaded.plugin_id(),
)?;
let button = factory.create_button()?;
```

`load_plugin_library` requires `nc_get_factory_plugin_description_v1`, validates
plugin metadata and hard dependencies, and registers the plugin atomically.
Activation compares the exact factory signature before invoking bootstrap.
Unconnected contract revisions can coexist; incompatible calls fail with
`NC_E_ABI_VERSION`. Required live-load failures diagnose and abort. Loaded
libraries remain mapped until process exit.

For static assemblies, call
`Runtime::register_factory_plugin_description(factory_plugin_description())`
and then use the same typed activation method. Plugin dependency IDs affect
load order but never trigger path discovery or recursive loading.

Offline inspection uses
`Inspection::with_plugin_library_description(path, visitor)`. It borrows the
validated `NcFactoryPluginDescription` only for the callback and does not mutate
the process registry. The inspection JSON schema is factory-native: each plugin
contains a factory IID/signature/threading record and hard dependencies.
The [sample plugin](../../../examples/com-plugin/README.md) demonstrates both
linkage modes with ordinary Cargo.

## Object persistence

`neocogi_com::persistence` exposes the built-in `IPersistable`,
`IPersistenceFactory`, and `IObjectChildren` contracts. Providers implement these
without enabling a runtime. `ObjectRecord` and `Limits` require only `alloc`.
With `runtime`, hosts can capture and reconstruct explicit initialization trees:

```rust,ignore
use neocogi_com::persistence::{FactoryRegistry, snapshot};

let mut recipes = FactoryRegistry::new()?;
recipes.register(recipe_id, factory)?;
let bytes = snapshot(object.as_com_ref())?;
let restored = recipes.restore(&bytes)?;
```

Each registry is thread-local and independently owned. The runtime retains its
factories until the registry is dropped; registration requires mutable access.
The same operations run through the shared runtime in native shared deployments
and through its linked implementation with `static-runtime`. The NCB-v1 document
format and provider interface identities are unchanged.

The former `neocogi-com-persistence` and `neocogi-com-persistence-contracts` crates
have been absorbed into core, runtime and this facade. Replace their Rust import
paths with `neocogi_com::persistence`, enable `runtime` for snapshot/restore callers,
and handle the new `Result` from `FactoryRegistry::new()`.
See [persistence semantics and ownership](../runtime/PERSISTENCE.md).

## Features and scope

- `std` (default) enables standard-library integrations and allocation.
- `alloc` enables generated heap-backed objects and owned data helpers.
- `macros` (default) exposes implementation/generation macros.
- `runtime` links the shared-runtime facade and implies `std`. Unification of
  this feature is intentional: the facade contains no registry state and all
  calls resolve to the same process-loaded runtime shared object.
- `static-runtime` links one runtime into the final binary.

This is not Windows COM, a source-reflection system, a sandbox, or a plugin
marketplace. Applications own installation, path discovery, trust, and policy.
