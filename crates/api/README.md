# neocogi-com

`neocogi-com` is the user-facing Rust facade for NeoCogi's explicit C-shaped
component ABI. It provides typed COM pointers, generated interface/object
support, factory-only plugins, and an optional facade over the linked
`libnc_com`.

Public cross-library contracts should be authored as Rust-like `.ncidl`
modules. See [IDL contract modules](../../../docs/contracts/IDL_MODULES.md) for their
source, crate, signature, and tooling model. The
[current core ABI specification](../docs/README.md) defines the binary boundary.

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

`com_interface` generates the interface marker, vtable, ABI signature, and one
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
These shared objects contain state, lifetime counters, and one call-borrow guard;
interfaces add no fields to each allocation. `ComPtr` and `ComRef` carry
`{object, interface_vtable}` (16 bytes on 64-bit targets), while raw calls use
`ComRaw<T>`. Query changes the table and preserves the object address. Methods
receive that address directly: there are no embedded headers or owner offsets.
Shared ancestors occur once in the compiler's contract set; conflicting
signatures fail compilation. Runtime queries scan static vtable pointers whose
compact identities are separate from full inspection schemas. Inherited table
prefixes remain; this change does not deduplicate their function entries.
See [interface storage](../../../docs/contracts/COM_INTERFACE_STORAGE.md) and `ComPtr`
rustdoc for the architectural rationale, ownership rules, and cost tradeoff.

When the creating binary also needs the concrete implementation, use
`Implementation::into_local(value)` to obtain `LocalCom<Implementation>`.
Query and cache COM interfaces from that owner; use `try_borrow()` or
`try_borrow_mut()` between interface calls to access ordinary Rust fields.
These guards share the generated COM call gate and report `NC_E_BUSY` on
conflicting access. They perform no allocation, query, or reference-count
operation and cannot outlive or escape the threading mode of the local owner.
Cloning the local handle retains the same allocation. There is no safe downcast
from an arbitrary foreign interface to a local implementation.

`local.downgrade()?` creates a `WeakLocalCom<Implementation>` without retaining
the implementation. `weak.upgrade()?` returns `Some(LocalCom<Implementation>)`
while any local or interface strong owner remains, and `None` after final release.
Promotion acquires ownership without borrowing the value: a busy object still
upgrades, but conflicting access through its local guards reports `NC_E_BUSY`.
Clone and Debug operate on weak bookkeeping only. Threading and concrete Rust
type restrictions remain those of the original local owner; a foreign weak
interface cannot be converted into a typed local handle. To pass a weak reference
through a contract, select an interface with `local.query::<InterfaceAbi>()?`
and downgrade that interface pointer. The concrete implementation stays local.

## Strings and byte buffers

With `alloc` (also enabled by the default `std` feature), `NcString` and
`NcBytes` are constructor namespaces. They return owning COM pointers; they
are not Rust string/vector wrappers that expose their provider's allocation.

```rust
use neocogi_com::{INcStringComPtrExt, NcStatus, NcString};

let value = NcString::from_utf8_str("hello");
let owned = value.to_string()?;
assert_eq!(owned.as_str(), "hello");

let mut prefix = [0; 2];
value.copy_to(0, &mut prefix)?;
assert_eq!(&prefix, b"he");
# Ok::<(), NcStatus>(())
```

`INcStringComPtrExt` works on both `ComPtr<INcStringAbi>` and
`ComRef<'_, INcStringAbi>`. Its `to_string(&self) -> Result<String, NcStatus>`
copies through the ABI and validates UTF-8; `to_bytes` returns owned bytes.
`INcBytesComPtrExt::to_vec` and `to_i8_vec` copy byte objects. These helpers
default to a 64 MiB allocation ceiling; `_bounded(max_len)` variants select a
different maximum. `copy_to(offset, &mut [u8])` copies the requested range into
caller storage without allocating a complete result. The COM string contract
offers no borrowed `as_str` or provider-storage pointer.

Metadata is different: `NcStaticUtf8::as_str` is an **unsafe**, fallible borrow
with a caller-chosen lifetime, and `NcUtf8View` has no conversion methods.
Neither raw descriptor retains its backing library. See the
[core conversion table](../core/README.md#text-bytes-and-borrowed-storage).
Runtime inspection returns owned `PluginInfo` strings and `PluginOrigin` paths.
These values remain usable independently of subsequent catalog lookups.

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
  signature. Declaration names/versions are inputs to IDL identity and
  signature generation; plugin release-version text is diagnostic.

Objects are thread-local by default. An IDL interface or factory declared
thread-safe produces `ComPtr<T, ThreadSafe>` and its implementation must satisfy
`Send + Sync`. A safe pointer may be weakened to thread-local; it cannot be
upgraded.

## IDL implementation and factory plugin

Given a generated contract crate, implementation code has this shape:

```rust,ignore
use my_contracts::{Button, ButtonAbi, UiFactory};
use neocogi_com::{ComBox, NcStatus, com_factory, com_object};

#[com_object(my_contracts::Button, ownership = exclusive)]
struct ButtonObject {
    label: String,
}

impl Button for ButtonObject {
    // IDL methods are implemented with ordinary typed Rust signatures.
}

#[com_factory(my_contracts::UiFactory)]
struct UiFactoryObject;

impl UiFactory for UiFactoryObject {
    fn create_button(&self) -> Result<ComBox<ButtonAbi>, NcStatus> {
        ButtonObject::into_box(ButtonObject { label: String::new() }).query()
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
unit factories receive a generated zero-argument bootstrap; stateful roots
implement `ComFactoryBootstrap` explicitly.

## Runtime flow

With the `runtime` feature, dynamic hosts use:

```rust,ignore
let runtime = neocogi_com::Runtime::global();
let plugin_id = unsafe { runtime.load_plugin_library(path) };
let factory = runtime.activate_plugin_factory::<my_contracts::UiFactoryAbi>(
    plugin_id,
)?;
let button = factory.create_button()?;
```

`load_plugin_library` requires `nc_get_factory_plugin_description_v1`, validates
plugin metadata and hard dependencies, and registers the plugin atomically.
Activation compares the exact factory signature before invoking bootstrap.
Unconnected contract revisions can coexist; incompatible calls fail with
`NC_E_ABI_VERSION`. Every live plugin load is mandatory: path conversion,
loading, metadata validation, dependency and registration
failures diagnose and abort the entire application. This includes designer
startup. `load_plugin_library` returns the registered plugin GUID on success;
there is no recoverable `try_load_plugin_library` operation. Batch loading
uses the same policy for each library. Loaded libraries remain mapped until
process exit. `Inspection::plugin_info(plugin_id)` returns the plugin's identity,
release version and static/dynamic origin. Repeated provider loads reuse the
first registration and origin; conflicting providers are rejected. See
[runtime ABI v1](../docs/runtime-abi-v1.md).

Inspection is a separate operation that can report errors without registering
or activating a plugin. Its fallible result does not make live loading optional.

For static assemblies, register the descriptor before using the same typed
activation method:

```rust,ignore
// SAFETY: The linked descriptor and all its code/data live for the process lifetime.
unsafe {
    runtime.register_factory_plugin_description(factory_plugin_description())?;
}
```

This is unsafe even when the outer descriptor reference is `&'static`: all
reachable metadata pointers and bootstrap code must also remain valid, and the
producer must honor its threading promise. Plugin dependency IDs affect
load order but never trigger path discovery or recursive loading.

Offline inspection uses the unsafe
`runtime.inspection().with_plugin_library_description(path, visitor)` method.
The path must identify trusted code, and the visitor must not panic across its
C callback boundary. It borrows the
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

| Selection | Available behavior |
| --- | --- |
| `default-features = false` | `no_std` core ABI, pointer/client support, persistence contracts, and serialization traits; no runtime linkage |
| `alloc` | Heap-backed object helpers, owned string/byte objects, the binary codec, and `ObjectRecord` |
| `macros` | Macro/derive re-exports and built-in event contracts; heap-backed object storage also needs `alloc` |
| `std` | `alloc` plus standard-library integrations; no runtime linkage by itself |
| Defaults: `std`, `macros` | Local objects, owned data, and event adapters; no runtime linkage |
| `runtime` | Implies `std`; links the `nc_com` native runtime through internal C declarations and enables runtime/persistence clients |
| `static-runtime` | Implies `runtime`; links the `nc_com` implementation as an rlib inside the final native binary or browser module |

For shared mode, build and deploy `libnc_com.so` on Linux, `libnc_com.dylib` on
macOS, or `nc_com.dll` with its import library on Windows. Ordinary
`cargo build -p neocogi-com-runtime` produces an rlib; use
`cargo rustc -p neocogi-com-runtime --lib --crate-type cdylib` for a standalone
shared runtime, or use the assembly tool. `NC_COM_RUNTIME_LINK_DIR` supplies
native linker search paths; deployment must separately satisfy the OS loader.

Shared-mode `runtime` feature unification is intentional: facade instances
hold no catalog, and their external calls resolve to the staged runtime.
`static-runtime` wins if both are enabled. Keep it out of separate plugin
images; `--all-features` therefore selects static linkage rather than validating
a shared deployment.

This is not Windows COM, a source-reflection system, a sandbox, or a plugin
marketplace. Applications own installation, path discovery, trust, and policy.

Runtime discovery declarations live in `src/runtime/ffi.rs`. Shared builds may
set `NC_COM_RUNTIME_LINK_DIR` to the runtime/import-library directory. Without
`runtime`, the facade emits no runtime link requirement. `static-runtime` retains
the implementation rlib and takes precedence over shared linkage when features
unify; separately loaded plugins must not enable it.

The `macros` feature also exposes built-in event contracts through
[`neocogi_com::events`](EVENTS.md). Contracts need no allocation or runtime;
receiver allocation needs `alloc`, and applications using standard channels need `std`. Persistent UI
controllers and scheduling belong to the UI facade and core contracts. Event IDL compilation
is an optional build dependency of this feature and runs on the build machine.

### Construction-bound event receivers

All producers accept the common `IEventReceiver`. `events::channel()` supplies an
ordinary local inbox; applications needing another envelope or capacity policy
implement that one interface themselves. `post` transfers a `ComBox<IEvent>` and
captures a weak widget/service origin. The event remains a COM object until its
handler queries a derived event interface. A rejected post returns the original
owner. The standard channel reports `NC_E_CLOSED` when disconnected.

`EventBinding` describes constructor delivery policy. `#[events(...)]` advertises
the emitted immutable interfaces and their getter metadata. Factories fix the receiver and weak origin before publication.
`None` means permanently unobserved. Shared-service `EventPort` state can close
idempotently; widget destruction releases its stored receiver. Already queued
facts remain owned by the inbox. IDL generates no per-widget queue aliases or
application callback receivers. See [COM event objects](../../../docs/contracts/COM_EVENT_INTERFACES.md)
and [event ownership](../../../docs/contracts/CLASS_EVENT_STREAMS.md).


Concrete providers select one ownership policy. Shared ownership is the default;
`#[com_object(..., ownership = exclusive)]` and
`#[com_class(Contract, ownership = exclusive)]` select exclusive ownership.
Only that policy's constructors are generated: shared classes have `from_value` /
`from_value_cyclic` / `into_local`; exclusive classes have `into_box` /
`into_box_cyclic` / `into_local_box`. Factories always remain shared.

Exclusive providers use `into_box` / `into_box_cyclic` to construct a
non-clonable `ComBox`. `WeakComBox` caches its interface table and accesses the
provider through checked call entries, without promoting to a shared owner.
Factories can return boxes and containers can consume them through generated
ABI methods. Ordinary shared services continue using `ComPtr`. See
[the ownership rationale](../../../docs/contracts/EXCLUSIVE_WIDGET_OWNERSHIP.md).

## Lifetime failure categories

`NC_E_DROPPED` reports lost object ownership. `NC_E_CLOSED` reports a closed
service or channel whose COM object can still exist. A method can propagate
either from a dependency, so neither status identifies receiver lifetime. See the
[precise lifetime error contract](../../../docs/contracts/COM_LIFETIME_ERRORS.md).
