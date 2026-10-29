# neocogi-com

`neocogi-com` is the user-facing Rust facade for NeoCogi's explicit C-shaped
component ABI. It provides typed COM pointers, generated interface/object
support, typed plugin roots, and optional clients for
host-supplied runtime services.

Start with the complete [first application and plugin](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/examples/getting-started/README.md)
walkthrough. It includes both dependency manifests, the shared IDL source,
build script, provider and host, and the commands to run and inspect them.
The SDK enables `std` and implementation macros by default. A final host also
selects `neocogi-com-host` and enables SDK `runtime` clients; a provider uses the
SDK and the host access supplied during acquisition.

## Hosts, callers and providers

An interface caller calls an object; a provider implements its interfaces. A
plugin can be both, and a host can provide callback objects. This package is the
shared SDK for all of them. The final host application additionally selects
`neocogi-com-host`, whose Rust crate is `neocogi_com_host`, to own registration,
loading and host services.

The SDK depends normally on core and optional implementation macros. Its
`runtime` feature enables clients for supplied host-service tables; it creates no
host state and adds no implementation dependency. A provider receives a copyable
`Runtime` during bootstrap and uses that access for any service calls. The host
implementation uses the SDK's shared ownership/contracts/codecs with `std`, while
calling its own raw service implementation stays inside the host package.

See [roles, dependencies and startup](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/COM.md#hosts-callers-and-providers)
for the complete dependency graph. A local object and its interface callers need
only their shared contract and SDK features, independently of plugin deployment.

Public cross-library contracts should be authored as Rust-like `.ncidl`
modules. See [IDL contract modules](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/contracts/IDL_MODULES.md) for their
source, crate, signature, and tooling model. The
[current core ABI specification](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md) defines the binary boundary.

## Contracts and concrete objects

Write the contract in shared `.ncidl` sources; each consumer can generate its
own bindings during its ordinary Cargo build:

```text
#[version(1)]
interface ICalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32>;
}
```

`neocogi-idl` binds references, computes canonical identity/signature bytes, and
passes typed method/value plans to its own `codegen` module. That shared backend emits the
complete trait, C vtable, checked codecs, clients and provider thunks directly.
There are no handwritten interface/value declaration attributes or signature
options. The checkout [calculator example](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/tests/com/examples/calculator.rs) and its
[contract source](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/tests/com/examples/schemas/nc/examples/calculator.ncidl) show the complete path.

Tools can read generated contracts through `neocogi_com::idl_metadata` without
depending on the compiler. `InterfaceAbi::IDL_INTERFACE` contains an
`InterfaceMetadata` with methods, argument/result ownership and reachable value
definitions. Its name, complete signature and ancestry use lifetime-bearing static
Rust references; safe readers need no raw descriptor borrowing. Physical ABI
descriptors remain separate and use the same resolved identities.
`Value::IDL_TYPE` contains a `TypeMetadata` with record fields,
enum cases or bit flags. These static Rust records stay inside their defining
binary; a domain owns any transported catalog and its application policy.

Concrete Rust state implements generated traits and uses `com_object`. This
runnable example uses the ordinary object-children contract:

```rust
use neocogi_com::{com_object, ComBox, ComPtr, IUnknown, NcStatus, NC_E_OUT_OF_RANGE};
use neocogi_com::persistence::{IObjectChildren, IObjectChildrenAbi};

#[com_object(neocogi_com::persistence::IObjectChildren, ownership = exclusive)]
struct EmptyChildren;
impl IObjectChildren for EmptyChildren {
    fn count(&self) -> Result<u32, NcStatus> { Ok(0) }
    fn get(&self, _: u32) -> Result<ComPtr<IUnknown>, NcStatus> { Err(NC_E_OUT_OF_RANGE) }
}

let owner: ComBox<IObjectChildrenAbi> = EmptyChildren::into_box(EmptyChildren).query().unwrap();
assert_eq!(owner.count().unwrap(), 0);
let observed = owner.downgrade();
drop(owner);
assert!(!observed.is_alive());
```

Generated `ComPtr<IFooAbi>` and `ComRef<'_, IFooAbi>` lend one `IFooClient` method
view, so calls use `pointer.method()` without an interface-specific trait import.
Derived clients lend their physical base client for inherited lookup. Owned
base projections require checked `QueryInterface`.

IDL rejects method names occupied by handle operations, such as `clone`, `as_raw`
and `downgrade`, using the core-owned member inventory. Legal data names cannot
capture implementation support or positional locals. See the
[identifier policy](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/contracts/COM_GENERATED_IDENTIFIER_HYGIENE.md).

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
See [interface storage](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/contracts/COM_INTERFACE_STORAGE.md) and `ComPtr`
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
[core conversion table](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/crates/core/README.md#text-bytes-and-borrowed-storage).
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

Objects are thread-local by default. An IDL interface declared with
`#[threading(shared)]` produces `ComPtr<T, ThreadSafe>` and its implementation must satisfy
`Send + Sync`. A safe pointer may be weakened to thread-local; it cannot be
upgraded.

## IDL implementation and factory plugin

Given locally generated bindings, implementation code has this shape:

```rust,ignore
use my_contracts::{Button, ButtonAbi, UiFactory};
use neocogi_com::{ComBox, NcStatus, com_object};

#[com_object(my_contracts::Button, ownership = exclusive)]
struct ButtonObject {
    label: String,
}

impl Button for ButtonObject {
    // IDL methods are implemented with ordinary typed Rust signatures.
}

#[com_object(my_contracts::UiFactory)]
struct UiFactoryObject;

impl UiFactory for UiFactoryObject {
    fn create_button(&self) -> Result<ComBox<ButtonAbi>, NcStatus> {
        ButtonObject::into_box(ButtonObject { label: String::new() })
            .query().map_err(|(status, _owner)| status)
    }
}

#[neocogi_com::plugin(name = "my.ui")]
fn acquire_root(_runtime: neocogi_com::Runtime) -> Result<neocogi_com::ComPtr<my_contracts::UiFactoryAbi>, neocogi_com::NcStatus> {
    UiFactoryObject::from_value(UiFactoryObject).query()
}
```

`#[com_object(Interface)]` connects private storage to any closed interface contract. The plugin
acquisition function owns initialization and its return supplies the root identity.

`#[plugin]` produces `plugin_descriptor() -> &'static NcPluginDescriptor` and
exports `nc_get_plugin_descriptor_v1` when the provider's `dynamic-plugin` feature
is enabled. A root can be any closed interface; no IDL construction role is required.

| Value | Producer and use | Ownership |
| --- | --- | --- |
| `NcPluginDescriptor` | Provider input to validation, registration or unregistered inspection. Includes the bootstrap function. | Borrows immutable assembly metadata and code; copying it does not retain the library. |
| `NcPluginInfoV1` / `PluginInfo` | Runtime output from registered lookup/enumeration. Includes first registration origin. | Raw views live for the process; the facade owns text, paths and GUID copies while retaining the process-lived signature view. |
| `ComPtr<FactoryAbi>` | Result of typed `acquire_plugin_root`. Used to call product construction methods. | Owns one COM reference, released when dropped. |

Reading either metadata record does not acquire a factory. Registration retains
the provider descriptor; it does not convert an inspection copy into a provider.

## Runtime flow

With the `runtime` feature, dynamic hosts use:

```rust,ignore
// runtime is supplied to #[com_main] and passed through application helpers.
let plugin_id = unsafe { runtime.load_plugin_library(path) };
let factory = runtime.acquire_plugin_root::<my_contracts::UiFactoryAbi>(
    plugin_id,
)?;
let button = factory.create_button()?;
```

`load_plugin_library` requires `nc_get_plugin_descriptor_v1`, validates
plugin metadata and hard dependencies, and registers the plugin atomically.
Acquisition compares the exact factory signature before invoking bootstrap.
Unconnected contract revisions can coexist; incompatible calls fail with
`NC_E_ABI_VERSION`. Every live plugin load is mandatory: path conversion,
loading, metadata validation, dependency and registration
failures diagnose and abort the entire application.
`load_plugin_library` returns the registered plugin GUID on success;
there is no recoverable `try_load_plugin_library` operation. Batch loading
uses the same policy for each library. Loaded libraries remain mapped until
process exit. `Runtime::plugin_info(plugin_id)` returns the plugin's identity,
release version, static/dynamic origin, single `FactoryInfo` and hard dependency
GUIDs. `Runtime::plugins()` returns these complete owned records in registration
order, without separate factory or dependency queries. Repeated provider loads reuse the
first registration and origin; conflicting providers are rejected. See
[runtime ABI v1](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md#runtime-and-global-registry).

Inspection is a separate operation that can report errors without registering
or acquiring a plugin factory. Its fallible result does not make live loading optional.

With `neocogi-com-host` on Windows, supply an explicit filesystem path. Relative
paths are resolved once; bare names do not request PATH discovery. Ordinary
imports for this open search the selected DLL directory and System32, subject to
Windows module reuse/redirection. Deploy private dependencies beside the DLL.
Later provider-issued loads and delay loads have their own search policy.
The SDK requires UTF-8 paths and preserves the original spelling as provenance.
Both the provider and its native dependencies remain trusted code; see the
[loading contract](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md#runtime-and-global-registry).

For static assemblies, register the descriptor before using the same typed
acquisition method:

```rust,ignore
// SAFETY: The linked descriptor and all its code/data live for the process lifetime.
unsafe {
    runtime.register_plugin_descriptor(plugin_descriptor())?;
}
```

This is unsafe even when the outer descriptor reference is `&'static`: all
reachable metadata pointers and bootstrap code must also remain valid, and the
producer must honor its threading promise. Plugin dependency IDs affect
load order but never trigger path discovery or recursive loading.

Offline inspection uses the unsafe
`runtime.with_plugin_library_descriptor(path, visitor)` method.
The path must identify trusted code, and the visitor must not panic across its
C callback boundary. It borrows the
validated `NcPluginDescriptor` only for the callback and does not mutate
the process registry. The inspection JSON schema is factory-native: each plugin
contains a factory IID/signature/threading record and hard dependencies.
The [sample plugin](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/examples/com-plugin/README.md) demonstrates both
linkage modes with ordinary Cargo.

## Object persistence

`neocogi_com::persistence` exposes the built-in `IPersistable`,
`IPersistenceFactory`, and `IObjectChildren` contracts. Providers implement these
without enabling a runtime. `ObjectRecord` and `Limits` require only `alloc`.
With `runtime`, hosts can capture and reconstruct explicit initialization trees:

```rust,ignore
use neocogi_com::persistence::{FactoryRegistry, snapshot};

let mut recipes = FactoryRegistry::new(&runtime)?;
recipes.register(recipe_id, factory)?;
let bytes = snapshot(&runtime, object.as_com_ref())?;
let restored = recipes.restore(&bytes)?;
```

Each persistence registry is thread-local and independently owned. Its factories
remain retained until that registry is dropped; registration requires mutable access.
The same supplied host allocates, operates on and destroys each context. The
NCB-v1 document format and provider interface identities remain unchanged.
See [persistence semantics and ownership](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/crates/host/PERSISTENCE.md).

## Features and scope

| Selection | Available behavior |
| --- | --- |
| `default-features = false` | `no_std` core ABI, pointer/client support, persistence contracts, and serialization traits; no runtime linkage |
| `alloc` | Heap-backed object helpers, owned string/byte objects, the binary codec, and `ObjectRecord` |
| `macros` | Implementation macro/derive re-exports |
| `std` | `alloc` plus standard-library integrations; no runtime linkage by itself |
| Defaults: `std`, `macros` | Local objects, owned data, and implementation macros; no runtime linkage |
| `runtime` | Implies `std`; enables runtime/persistence service clients. The host supplies admitted access. |

The final native/browser executable directly depends on `neocogi-com-host`
and initializes it through `#[neocogi_com::com_main]`. The facade has no
implementation dependency. Reusable helpers accept `Runtime`; plugin acquisition
receives it from the host. Tests may explicitly initialize their selected owner.
There is no COM runtime DSO, import bridge, named getter or fallback discovery.
Dynamic and static providers reach the same host registry through immutable C
service tables. Applications can define domain observer interfaces directly; the
[SDK example](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/examples/com-plugin/README.md) demonstrates this.


Concrete providers select one ownership policy. Shared ownership is the default;
`#[com_object(..., ownership = exclusive)]` and
`#[com_object(Contract, ownership = exclusive)]` select exclusive ownership.
Only that policy's ordinary constructors are generated: shared classes have
`from_value` / `into_local`; exclusive classes have `into_box` / `into_local_box`.
Every constructor takes a fully initialized Rust value. Factories obtain weak
identities afterward and finish private event wiring before exposing an object;
there are no cyclic constructors or unpublished-value promotion states. See
[the construction contract](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/contracts/COM_OBJECT_CONSTRUCTION.md).
Factories always remain shared.

Exclusive providers use `into_box` to construct a non-clonable `ComBox`. `WeakComBox` caches its interface table and accesses the
provider through checked call entries, without promoting to a shared owner.
Factories can return boxes and containers can consume them through generated
ABI methods. Ordinary shared services continue using `ComPtr`. See
the ownership rationale.

## Lifetime failure categories

`NC_E_DROPPED` reports lost object ownership. `NC_E_CLOSED` reports a closed
service or channel whose COM object can still exist. A method can propagate
either from a dependency, so neither status identifies receiver lifetime. See the
[precise lifetime error contract](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/contracts/COM_LIFETIME_ERRORS.md).
