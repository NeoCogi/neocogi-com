# neocogi-com

`neocogi-com` is the user-facing Rust facade for NeoCogi's explicit C-shaped
component ABI. It provides typed COM pointers, generated interface/object
support, factory-only plugins, and an optional facade over the linked
`libnc_com_shared_runtime`.

Public cross-library contracts should be authored as Rust-like `.ncidl`
modules. See [IDL contract modules](../../../docs/IDL_MODULES.md) for their
source, crate, native-linking, and tooling model. The
[core ABI v7 specification](../docs/abi-v7.md) defines the binary boundary.

## Local object example

```rust
use neocogi_com::{ComPtr, IUnknown, NcStatus, com_interface, com_object};

#[com_interface(auto_guid = "example.Calculator:v1")]
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
    let unknown: ComPtr<IUnknown> = Calculator::create();
    unknown.query::<ICalculatorAbi>()?.add(2, 3)
}
```

`com_interface` generates the portable header, vtable, ABI signature, and typed
call helpers. `com_object` generates reference-counted storage and
signature-checked `QueryInterface`. It does not publish the object as something
the runtime can construct.

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

Objects are thread-local by default. An IDL class or factory declared
thread-safe produces `ComPtr<T, ThreadSafe>` and its implementation must satisfy
`Send + Sync`. A safe pointer may be weakened to thread-local; it cannot be
upgraded.

## IDL implementation and factory plugin

Given a generated contract crate, implementation code has this shape:

```rust,ignore
use my_contracts::{Button, ButtonAbi, UiFactory, MODULE};
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
    contracts = [MODULE],
    factory = UiFactoryObject,
}

neocogi_com::export_factory_plugin!();
```

`#[com_object(Description)]` connects private storage to one IDL class.
`#[com_factory(Description)]` does the same for a factory and proves the type of
the root bootstrap. The plugin description stores plugin metadata, exact
contract references, and one activation pointer. It stores no class table or
constructor arguments.

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

`load_plugin_library` requires `nc_get_factory_plugin_description`, validates
the plugin and its exact contract dependency graph, and publishes them
atomically. A different fingerprint for an already known logical module returns
`NC_E_CONTRACT_MISMATCH`. Required live-load failures diagnose and abort rather
than expose partial state. Loaded libraries remain mapped until process exit.

For static assemblies, call
`Runtime::register_factory_plugin_description(factory_plugin_description())`
and then use the same typed activation method. Plugin dependency IDs affect
load order but never trigger path discovery or recursive loading.

Offline inspection uses
`Inspection::with_plugin_library_description(path, visitor)`. It borrows the
validated `NcFactoryPluginDescription` only for the callback and does not mutate
the process registry. The inspection JSON schema is factory-native: each plugin
contains a direct factory record and canonical contract-module graph.

## Features and scope

- `std` (default) enables standard-library integrations and allocation.
- `alloc` enables generated heap-backed objects and owned data helpers.
- `macros` (default) exposes implementation/generation macros.
- `runtime` links the shared-runtime facade and implies `std`.
- `static-runtime` links one runtime into the final binary.

This is not Windows COM, a source-reflection system, a sandbox, or a plugin
marketplace. Applications own installation, path discovery, trust, and policy.
