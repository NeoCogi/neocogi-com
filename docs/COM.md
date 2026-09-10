# neocogi-com

## Optional ABI validation

The `abi-validation` feature generates a contract-specific ABI signature for
every interface, constructor, struct, and enum and makes descriptor validation
recompute those signatures. Without it, generated non-empty contracts carry
one explicit unchecked sentinel while pointer/count, identity, name, method,
passing-mode, and type-tag validation remains active. Empty constructors retain
their distinct empty sentinel so argument-pointer validation is unchanged.

Enable or disable this feature consistently for the runtime and the complete
application/plugin Cargo graph. A validation-enabled runtime rejects unchecked
component descriptors. The high-level `neocogi-com` crate enables validation by
default; assembly declarations that disable component default features must add
`abi-validation` explicitly when strict compatibility checking is required.

`neocogi-com` is a small COM-style runtime for cross-platform Rust components that
need an explicit binary boundary intended for long-term compatibility after ABI
ratification.

The primary goal is **Rust-to-Rust component interoperability through an
explicit C-shaped ABI**. A producer and consumer can be compiled as separate
artifacts, with different crate graphs, Rust compiler versions, optimization
settings, or linking modes, and still communicate through explicit `#[repr(C)]`
interface headers, vtables, status codes, and reference-counted object lifetime.

This crate is not trying to expose Rust trait objects across a binary boundary.
Rust trait object layout, enum layout, `String`, `Vec<T>`, slices, references,
and allocator ownership are not stable ABI contracts. `neocogi-com` keeps those out
of the boundary and generates a C-shaped ABI instead.

## Component Pattern

`neocogi-com` uses Rust as the contract authoring language, but the Rust ABI is not
the contract. A trait marked with `#[com_interface]` is lowered into an explicit
C-shaped interface: a `#[repr(C)]` header, a vtable, `extern "C"` function
pointers, `NcStatus` returns, GUID identity, and a canonical ABI signature.

The generated ABI header receives an `unsafe impl ComInterface` because that
trait certifies raw header, vtable, identity, and `IUnknown` conversion
invariants used by safe `ComPtr` operations. Component authors should use
`#[com_interface]`, which supplies the audited implementation automatically.
`ComInterface` remains unsealed for low-level foreign-interface adapters, but a
handwritten implementation requires `unsafe impl` and assumes the complete
contract documented by the trait.

A component exposes classes that implement one or more interfaces. A host loads
or receives a plugin descriptor, validates class and interface metadata, then
constructs objects through registry APIs. After construction, calls cross the
boundary only through COM-style interface pointers.

Rust callers usually do not manage raw COM pointers directly. `ComPtr<T>` owns
one COM reference and releases it from `Drop`; cloning a `ComPtr` calls
`AddRef`. `ComRef<'_, T>` is a borrowed call-scoped view that does not release on
drop. If the callee needs to keep it, it explicitly calls `add_ref()` to obtain a
`ComPtr<T>`. This gives Rust code normal RAII-style object handles while keeping
the ABI ownership protocol explicit.

This is different from crates such as `abi_stable` or `stabby`, which try to make
a useful subset of Rust-shaped APIs stable across a dynamic boundary. `neocogi-com`
does not stabilize Rust trait objects, `String`, `Vec<T>`, slices, futures, enum
layout, allocator ownership, or Rust drop behavior. Those remain implementation
details behind the component boundary.

The pattern is closer to the WebAssembly Component Model in spirit: define a
typed component contract, lower it to an explicit ABI shape, and keep
implementation language details private. The difference is that `neocogi-com` is a trusted
same-process native ABI for Rust components, not a sandboxed language-neutral
WebAssembly runtime.

## Scope

`neocogi-com` is Rust-to-Rust component ABI glue. It is intended for hosts and
components that are both written in Rust but cannot share Rust-native ABI
assumptions because they are built separately.

The explicit contract is the generated C-shaped surface:

- `#[repr(C)]` interface headers with vtable and object pointers
- `extern "C"` vtable functions
- explicit `NcStatus` return values
- explicit GUIDs and ABI signatures
- COM-style reference counting with `AddRef` and `Release`
- host-owned plugin/class registration metadata

Everything else is local implementation detail. Rust traits, generic types,
Rust references, trait objects, `String`, `Vec<T>`, slices, allocator ownership,
and unwinding panics do not cross the ABI boundary.

This is not Windows COM. There are no apartments, no Windows registry
activation, no `HRESULT`, no `IUnknown` binary contract from the operating
system, and no system marshalling. The object model is COM-style because that
shape is a useful cross-platform ABI pattern.

The default runtime exposes one process-global `Runtime`. A host chooses
which plugin descriptors to accept, validates their metadata, and owns the
resulting class table. Successfully loaded plugin libraries remain mapped for
the process lifetime. Its dynamic C surface is one mandatory runtime ABI version
function plus runtime, plugin registry, type registry, inspection, and debug v1
table getters.

The Rust facade follows the same responsibilities: `Runtime` owns loading and
construction, `PluginRegistry` and `TypeRegistry` enumerate what exists,
`Inspection` returns metadata and relationships and can expose a validated
descriptor without registration, and `Debug` owns callback messenger
registrations.

### Accessing Loaded Plugin Information

Loading returns an owned summary with both module provenance and plugin
descriptor metadata:

```rust
fn show_plugin(path: &std::path::Path) -> Result<(), neocogi_com::NcStatus> {
    let runtime = neocogi_com::Runtime::global();
    let loaded = unsafe { runtime.load_plugin_library(path) };

    println!("module {}: {}", loaded.module_id(), loaded.path().display());
    println!("plugin {} {}", loaded.qualified_name(), loaded.plugin_version());

    let same_module = runtime
        .inspection()
        .loaded_module(loaded.module_id())?
        .expect("loaded module");
    let plugin = runtime
        .inspection()
        .plugin_info(loaded.plugin_id())?
        .expect("registered plugin");

    assert_eq!(same_module, loaded);
    assert_eq!(plugin.qualified_name, loaded.qualified_name());
    Ok(())
}
```

The module catalog is the source for module ID, path, and plugin ID. The
registered descriptor is the source for qualified name and version. The Rust
`LoadedModuleInfo` joins and owns both results. `plugin_info` also works for a
statically registered descriptor; such a plugin has no loaded module or path.

Runtime ABI v1 retains at most 1,024 dynamically loaded modules. Attempting a
1,025th load follows the same diagnostic-and-abort path as other live-activation
failures. Up to 64 debug messengers may be registered simultaneously;
`Debug::create_messenger` returns `NC_E_OUT_OF_RANGE` at capacity, and dropping
a messenger frees one slot without reusing its process-local handle.

The Rust runtime facade copies at most 65,536 elements in one registry
enumeration and retries append-only growth at most eight times after the initial
fill call. It rejects non-progressing `NC_E_INSUFFICIENT_BUFFER` responses and
limits one copied runtime inspection text view to 1 MiB. These are host-facade
safeguards; raw C count/fill callers choose their own allocation and retry
policy.

## What It Does

`neocogi-com` provides:

- `#[com_interface]` for defining a Rust implementation trait and generating a
  C-compatible `*Abi` interface header, `*VTable`, and safe `ComPtr` extension
  methods.
- `#[com_class]` for declaring a callable class contract with a typed constructor.
- `#[com_object]` for wrapping a Rust struct in generated COM object storage with
  `IUnknown`, signature-checked `QueryInterface`, one header per exposed interface,
  and atomic reference counting.
- `ComPtr<T>` for owned COM references.
- `ComRef<'_, T>` for call-scoped borrowed interface parameters.
- `NcStatus` for ABI errors.
- `NcGuid` for interface IDs.
- `#[com_struct]` for fixed-size ABI-safe records.
- `com_int_enum!` and `#[com_enum]` for explicit ABI-safe enum representations.

The generated ABI shape is deliberately simple:

```text
Rust trait method:

fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;

Generated ABI vtable slot:

unsafe extern "C" fn(
    this: *mut ICalculatorAbi,
    a: i32,
    b: i32,
    out: *mut i32,
) -> NcStatus
```

The safe Rust wrapper hides the raw vtable call and reads `out` only when the
returned status is successful.

Void success methods use `Result<(), NcStatus>` and lower to a status-only ABI
slot with no `out` pointer:

```text
Rust trait method:

fn flush(&self) -> Result<(), NcStatus>;

Generated ABI vtable slot:

unsafe extern "C" fn(this: *mut IFlushableAbi) -> NcStatus
```

## What It Is Not

`neocogi-com` is not a C SDK generator. The ABI is C-compatible and is tested from
native C, but the focus is Rust components using a C ABI as the stable
compatibility layer across independently built Rust artifacts after an explicit
ABI ratification milestone. The current ABI-v6 definition is still a pre-alpha
draft.

C header generation, language bindings, and C-first distribution should be a
separate project/tooling layer that consumes original contract definitions or
other build-time inputs. ABI-v6 runtime descriptors are validation metadata,
not a reflection schema, and are never intended for reverse generation of
source declarations. This crate should remain focused on defining and enforcing
the Rust-side ABI model.

It is also not Windows COM. It borrows the useful cross-platform parts of the
COM object model: interface IDs, signature-checked `QueryInterface`, `AddRef`,
`Release`, and vtables. It does not implement Windows apartments, registry activation,
`HRESULT`, or system COM marshalling.

## Documentation Map

This root README is the canonical long-form usage guide and the only place that
maintains the exact local/CI compatibility-test matrix. The published
`neocogi-com` crate README is a shorter package entry point, crate-level rustdoc owns
compiled API examples and feature-sensitive API documentation, and
[`docs/abi-v6.md`](docs/abi-v6.md) is the normative ABI specification. Shorter
documents link here for detailed workflows instead of copying exact test rows.

## Example

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

fn use_component() -> Result<i32, NcStatus> {
    let unknown: ComPtr<IUnknown> = Calculator::create();
    let calculator = unknown.query::<ICalculatorAbi>()?;

    calculator.add(2, 3)
}
```

`#[com_interface]` keeps `ICalculator` as the Rust implementation surface and
generates:

- `ICalculatorAbi`
- `ICalculatorVTable`
- `ICalculatorComPtrExt`

It also generates the unsafe `ComInterface` implementation for
`ICalculatorAbi`; defining and using the interface through the macro remains
safe. Manually implementing `ComInterface` is an advanced unsafe FFI extension
point, not an alternative contract-authoring path.

`#[com_object]` generates object storage and constructors. Unit structs receive
`create()`. Structs with fields receive `from_value(value)`.

### Callable class contracts

Use `#[com_class]` on a trait when one registered class owns the callable
surface. This avoids declaring a one-off `IThing` interface merely to call the
class. The shared contract crate declares the class once:

```rust
use neocogi_com::{NcStatus, com_class};

#[com_class(auto_guid = "example.Counter:v1")]
pub trait Counter {
    #[constructor]
    fn new(initial: u32) -> Result<Self, NcStatus>;

    fn value(&self) -> Result<u32, NcStatus>;
}
```

The implementation crate keeps its state private and binds it to that contract:

```rust
# use neocogi_com::{NcStatus, com_class, com_object};
# #[com_class(auto_guid = "example.Counter:v1")]
# pub trait Counter {
#     #[constructor]
#     fn new(initial: u32) -> Result<Self, NcStatus>;
#     fn value(&self) -> Result<u32, NcStatus>;
# }
#[com_object(Counter)]
struct CounterObject {
    initial: u32,
}

impl Counter for CounterObject {
    fn new(initial: u32) -> Result<Self, NcStatus> {
        Ok(Self { initial })
    }

    fn value(&self) -> Result<u32, NcStatus> {
        Ok(self.initial)
    }
}
```

`#[com_object(Counter)]` automatically installs the primary `CounterAbi` projection
and copies the class ID, name, version, and required interface set into the
implementation descriptor. Every class supertrait is a separate
`QueryInterface` projection on the same object; the implementation struct does
not repeat that list.
After registering the containing plugin, a host activates and calls the class
without spelling its ID or querying a companion interface:

```rust,ignore
let arguments = CounterAbi::arguments(42)?;
let counter = neocogi_com::Runtime::global().construct::<CounterAbi>(arguments)?;
assert_eq!(counter.value()?, 42);
```

The `#[constructor]` function is not placed in the class vtable. Its parameters
instead generate a private `repr(C)` frame, a lifetime-safe argument wrapper,
and the `CounterAbi::arguments(...)` builder. Plain ABI values,
`NcCopyFromBufferOffset<'_, T>`, and `ComRef<'_, T>` are supported. The runtime
passes the immutable frame only for the constructor call and verifies its ABI
signature before entering plugin code. There is no `Any`, reflection, property bag,
or constructor-argument COM object.

Unregistered implementation classes may have Rust type or const parameters,
default type arguments, and ordinary `where` bounds. Their exposed interfaces
remain concrete ABI contracts. This supports an application-owned typed handler
without putting generics, a Rust trait object, or borrowed stack state on the
binary boundary:

```rust
#[com_object(interfaces(IButtonSink))]
struct ButtonSink<State: ButtonHandler> {
    state: State,
}
impl<State: ButtonHandler> IButtonSink for ButtonSink<State> {
    fn submitted(&self, label: ComRef<'_, INcStringAbi>) -> Result<(), NcStatus> {
        self.state.submitted(label)
    }
}
```

Here `ButtonHandler` is an ordinary application-local trait with an owned concrete
implementation, and `IButtonSink` is a separately declared concrete COM interface.
`ButtonSink::from_value(...)` allocates that exact specialization. It preserves
inherited interface calls, canonical identity, threading policy and final-release
destruction. Type/const arguments do not change interface IDs or layouts.

The generated implementation requires the complete stored value to be `'static`.
A type argument cannot smuggle a short-lived reference into a retainable COM
pointer. Class lifetime parameters are rejected explicitly. Prefer directly
owned state with `&mut self` methods when the COM operation mutates it. Shared
ownership remains available when it is part of the implementation's actual
domain model. `thread_safe` classes must still prove `Send + Sync` for all
permitted specializations and honor the concurrent-call contract.

Generic implementation classes cannot declare registered class descriptors:
a plugin constructor has no Rust type arguments. Use a concrete registered
factory to construct owned generic helper objects when needed. This adds no
generic interfaces, type-name lookup, or runtime registration of monomorphizations.

## ABI Rules

Interface methods use a shared receiver for observation or a mutable receiver
for mutation:

```rust
fn method(&self, args...) -> Result<T, NcStatus>;
fn mutate(&mut self, args...) -> Result<T, NcStatus>;
```

Generated object storage arbitrates calls across every cloned interface pointer.
Shared calls may overlap; a mutable call receives exclusive access or returns
`NC_E_BUSY` when another call is active. Safe Rust callers must mutably borrow
their `ComPtr` or `ComRef` handle to invoke an `&mut self` method.

Unsupported trait shapes are rejected by the macro:

- generic traits or generic methods
- supertraits
- default method bodies
- `async`, `const`, `unsafe`, or `extern` methods
- non-method trait items
- receivers other than `&self` or `&mut self`
- methods named `query_interface`, `add_ref`, or `release`
- parameter names that collide with an actual generated adapter binding or
  lowered buffer slot

Plain method arguments and non-void successful return payloads must implement
`AbiType`. `Result<(), NcStatus>` is the void-success form and lowers to a
status-only ABI function with no `out` parameter.

The built-in value set includes:

- `bool`
- fixed-width integers
- pointer-sized integers
- floating point scalars
- `NcStatus`
- `NcGuid`
- fixed-size records generated by `#[com_struct]`
- transparent integer newtypes generated by `com_int_enum!`

`NcStatus` is an opaque `#[repr(transparent)]` status code. Use the exported
`NC_*` constants for built-in status values, `NcStatus::from_code(code)` when
preserving a raw ABI code from foreign or newer components, and `status.code()`
when handing the value back to C-shaped code. Rust callers should use
`is_ok()` and `is_err()` to mirror `Result` terminology. `name()` and `Display`
provide stable diagnostic text for built-in statuses and report unknown codes
as `NC_STATUS_UNKNOWN(<code>)`.

Rust APIs use `Result<T, NcStatus>` so unknown negative component errors can be
preserved without a closed error enum. `Result::Err` must contain a negative
status. Generated interface thunks and `construct_class_checked` preserve a
negative error and normalize an invalid non-negative error to
`NC_E_INVALID_ARG`; non-void thunks do not write their output slot in that case.

These are intentionally rejected as plain ABI values:

- Rust references
- raw pointers in user method signatures
- `String`
- `Vec<T>`
- slices
- `ComPtr<T>` as an argument
- `ComRef<'_, T>` as a return value

`ComPtr<T>` is rejected as an argument because it means "owned reference". At
the raw ABI layer, both borrowed and owned COM arguments are just `IFooAbi *`,
so accepting `ComPtr<T>` would make ownership transfer ambiguous. A callee might
accidentally release the caller's reference, or a caller might incorrectly assume
the callee took ownership.

Use `ComRef<'_, T>` for COM object arguments. It represents a borrowed interface
pointer that is valid for the duration of the call. The interface macro requires
the anonymous `'_` lifetime and rejects named lifetimes, including `'static`, so
safe implementations cannot retain the borrow itself:

```rust
use neocogi_com::{ComPtr, ComRef, NcStatus, com_interface};

#[com_interface(auto_guid = "example.Node:v1")]
pub trait INode {
    fn id(&self) -> Result<u64, NcStatus>;
}

#[com_interface(auto_guid = "example.Tree:v1")]
pub trait ITree {
    fn append_child(&self, node: ComRef<'_, INodeAbi>) -> Result<(), NcStatus>;
    fn make_node(&self, id: u64) -> Result<ComPtr<INodeAbi>, NcStatus>;
}
```

The ABI contract is:

- `ComRef<'_, IFooAbi>` arguments lower to borrowed `IFooAbi *` pointers.
- If the callee wants to store a borrowed interface pointer, it calls
  `ComRef::add_ref()` and stores the returned `ComPtr`.
- `ComPtr<IFooAbi>` returns lower to owned `IFooAbi **out` values.
- Successful owned returns transfer one reference to the caller.
- `Result<(), NcStatus>` returns lower to a status-only function with no `out`
  pointer.

For example, a document can own nodes by taking its own reference during the
call:

```rust
use neocogi_com::{ComPtr, ComRef, NcStatus};

pub struct Document {
    children: Vec<ComPtr<INodeAbi>>,
}

impl ITree for Document {
    fn append_child(&mut self, node: ComRef<'_, INodeAbi>) -> Result<(), NcStatus> {
        self.children.push(node.add_ref());
        Ok(())
    }

    fn make_node(&self, id: u64) -> Result<ComPtr<INodeAbi>, NcStatus> {
        todo!("construct a node COM object")
    }
}
```

Cloning `ComRef` itself only copies the borrowed view. It does not call
`AddRef`. The method that creates ownership is explicitly named `add_ref()`.

## Asynchronous Work

Async Rust futures do not cross the `neocogi-com` ABI. `Future`, `Pin`, executor
wakers, async trait lowering, and borrowed future state are Rust implementation
details, not stable binary contracts.

Keep core `neocogi-com` interfaces as bounded synchronous vtable calls that return
`NcStatus`. Model asynchronous work as explicit COM objects instead:

- operation objects with state, cancellation, and result interfaces
- completion callback interfaces
- byte streams or domain-specific progress/state interfaces
- state machines whose state is queried through normal COM methods

A future `neocogi-com-async` extension crate can provide standard operation,
completion, cancellation, and Rust `.await` adapter helpers on top of these
ordinary COM interfaces. The core ABI should remain executor-neutral and should
not expose Rust `Future` values directly.

## Variable-Size Data

Owned or long-lived variable-size data should cross the ABI as COM objects, not
as Rust containers, slices, or raw buffers. Bounded call-scoped copy buffers use
the explicit `NcCopyFromBufferOffset<'_, T>` and `NcCopyToBufferOffset<'_, T>`
parameter types. Interface methods must use the anonymous `'_` lifetime; named
lifetimes such as `'static` are rejected so the generated slice cannot escape
the call.

This includes:

- strings
- byte buffers
- arrays
- streams
- documents or trees with owned child data

Rust `String`, `Vec<T>`, slices, and references encode layout, allocator,
capacity, provenance, and lifetime assumptions that are not stable across
independently compiled Rust artifacts. Raw byte pointers have similar problems
unless they are produced by an explicit `neocogi-com` passing mode that names the
offset, length, element type, mutability, and call-scoped lifetime.

For common read-only data, `neocogi-com` provides `INcBytesAbi`/`NcBytes` for
`Vec<u8>` and bit-preserving `Vec<i8>`, plus `INcStringAbi`/`NcString` for
UTF-8 strings. Both interfaces provide bounded bulk copying into caller-owned
storage. Standard `NcString` objects can additionally be queried for
`INcBytesAbi`, but foreign string implementations need only `INcStringAbi` for
efficient copying.

Model domain-specific variable-size data with explicit interfaces:

```rust
use neocogi_com::{ComRef, NcCopyToBufferOffset, NcStatus, com_interface};

#[com_interface(auto_guid = "example.ReadOnlyBytes:v1")]
pub trait IReadOnlyBytes {
    fn len(&self) -> Result<u64, NcStatus>;
    fn byte_at(&self, index: u64) -> Result<u8, NcStatus>;
    fn copy_to(&self, output: NcCopyToBufferOffset<'_, u8>) -> Result<(), NcStatus>;
}

#[com_interface(auto_guid = "example.ByteConsumer:v1")]
pub trait IByteConsumer {
    fn consume(&self, bytes: ComRef<'_, IReadOnlyBytesAbi>) -> Result<u64, NcStatus>;
}
```

A UTF-8 string can be a semantic interface over bytes:

```rust
use neocogi_com::{ComPtr, NcCopyToBufferOffset, NcStatus, com_interface};

#[com_interface(auto_guid = "example.Utf8String:v1")]
pub trait IUtf8String {
    fn byte_len(&self) -> Result<u64, NcStatus>;
    fn byte_at(&self, index: u64) -> Result<u8, NcStatus>;
    fn copy_to(&self, output: NcCopyToBufferOffset<'_, u8>) -> Result<(), NcStatus>;
}

#[com_interface(auto_guid = "example.StringProvider:v1")]
pub trait IStringProvider {
    fn display_name(&self) -> Result<ComPtr<IUtf8StringAbi>, NcStatus>;
}
```

For common read-only strings, this is already available as `INcStringAbi` and
`NcString`.

The allocation-backed Rust convenience conversions use a 64 MiB default
ceiling. `to_vec_bounded`, `to_i8_vec_bounded`, `to_bytes_bounded`, and
`to_string_bounded` accept an application-specific maximum; `copy_to` instead
fills caller-owned storage without allocating. A reported length above the
selected limit returns `NC_E_OUT_OF_RANGE`, while failure to reserve an
in-limit allocation returns `NC_E_OUT_OF_MEMORY`.

Streams should return owned byte objects in caller-bounded chunks:

```rust
use neocogi_com::{ComPtr, INcBytesAbi, NcStatus, com_interface};

#[com_interface(auto_guid = "example.ByteStream:v1")]
pub trait IByteStream {
    fn next_chunk(&self, max_len: u64) -> Result<ComPtr<INcBytesAbi>, NcStatus>;
}
```

This keeps ownership explicit:

- `ComRef<'_, IFooAbi>` borrows an object for one call.
- `ComPtr<IFooAbi>` transfers one owned COM reference.
- The provider remains responsible for its own storage and allocator.
- `next_chunk(max_len)` lets the caller bound allocation and copying work; a
  successful chunk must contain at most `max_len` bytes.
- An empty chunk can represent end-of-stream. If `max_len == 0`, return a
  domain-specific invalid-argument status so empty chunks keep one meaning.
- Prefer small capability interfaces like `INcBytesAbi`, `INcStringAbi`, and
  `IByteStream` over raw pointer copying until the crate has a deliberate
  pointer and borrowing story.

## Shared Library Classes

A shared library exposes a plugin descriptor through one entrypoint. The plugin
descriptor carries the core ABI version, plugin identity, and the class table:

```rust
#[neocogi_com::com_class(auto_guid = "example.Widget:class:v1")]
pub trait Widget {
    #[constructor]
    fn new(width: u32) -> Result<Self, neocogi_com::NcStatus>;

    fn width(&self) -> Result<u32, neocogi_com::NcStatus>;
}

#[neocogi_com::com_object(Widget)]
pub struct WidgetObject {
    width: u32,
}

impl Widget for WidgetObject {
    fn new(width: u32) -> Result<Self, neocogi_com::NcStatus> {
        Ok(Self { width })
    }

    fn width(&self) -> Result<u32, neocogi_com::NcStatus> {
        Ok(self.width)
    }
}

neocogi_com::plugin! {
    plugin_id(auto_guid = "example.widget_plugin:v1"),
    name = "example.widget_plugin:v1",
    version = "1.0.0",
    dependencies = [FOUNDATION_PLUGIN_ID],
    classes = [WidgetObject],
}
```

Callable class contracts derive their descriptor name and class ID from one
`auto_guid = "stable.Name:v1"` input. Implementations use
`#[com_object(Class)]` and cannot restate the class ID, name, version,
constructor policy, or public projections. Core
ABI v6 descriptors are declarative and have no initialization callback.

The `plugin!` macro emits `pub fn plugin_descriptor() -> &'static
neocogi_com::NcPluginDescriptor` for static registration. It exports no dynamic
symbol, so several plugin implementations can be linked into one binary.
The final shared-library root publishes its one dynamic entrypoint explicitly:

```rust,ignore
neocogi_com::export_plugin!();
// Or select a descriptor in another module/crate:
// neocogi_com::export_plugin!(implementation::plugin_descriptor);
```

Both deployment modes use the same descriptor and validation. Keep the export
behind the final library's dynamic feature when supporting both modes.

The class table enumerates constructible classes. Each class descriptor carries
a stable class ID, static qualified name/version metadata, the constructor
frame ABI signature, generated interface metadata, and a constructor function:

Every object interface table contains the canonical `IUnknown` descriptor
exactly once. For local interface-only objects, `#[com_object]` inserts it first
and then preserves the source order of `interfaces(...)`. For registered
objects, the callable class contract supplies the class projection and required
protocols. Explicit `IUnknown` and duplicate local entries are rejected.

```rust
static CLASSES: [neocogi_com::NcClassDescriptor; 1] = [neocogi_com::NcClassDescriptor {
    class_id: WidgetAbi::CLASS_ID,
    qualified_name: neocogi_com::NcStaticUtf8::from_static("example.Widget"),
    class_version: neocogi_com::NcStaticUtf8::from_static("1.0.0"),
    constructor_abi_signature: WidgetAbi::CONSTRUCTOR_ABI_SIGNATURE,
    threading: neocogi_com::NC_ABI_THREADING_THREAD_LOCAL,
    construct: Some(construct_widget),
    interface_count: WidgetObject::COM_INTERFACES.len() as u32,
    interfaces: WidgetObject::COM_INTERFACES.as_ptr(),
}];

static CLASS_TABLE: neocogi_com::NcClassTable = neocogi_com::NcClassTable {
    class_count: CLASSES.len() as u32,
    classes: CLASSES.as_ptr(),
};

static PLUGIN: neocogi_com::NcPluginDescriptor = neocogi_com::NcPluginDescriptor {
    core_abi_version: neocogi_com::NC_COM_CORE_ABI_VERSION,
    plugin_id: PLUGIN_ID,
    qualified_name: neocogi_com::NcStaticUtf8::from_static("example.widget_plugin"),
    plugin_version: neocogi_com::NcStaticUtf8::from_static("1.0.0"),
    dependency_count: 0,
    dependencies: core::ptr::null(),
    class_table: &CLASS_TABLE,
};
```

Hosts can validate raw descriptors with
`validate_plugin_descriptor_with_issues`. Passing `|_| {}` gives status-style
validation without retaining details, while collecting the reported issues
preserves paths and machine-readable failure kinds for diagnostics.
Descriptor validation is for trusted plugin metadata shape. It is not hostile
plugin isolation, does not sandbox plugin code, and cannot safely probe arbitrary
non-null raw pointers beyond the unsafe descriptor contract.

Plugin dependency entries are hard plugin-ID edges used for activation order.
They do not recursively load a library or replace interface compatibility.
External contracts remain visible as constructor arguments, method arguments,
return values, or explicit capability interfaces; their interface IDs and ABI
signatures remain the binary contract.

The generated constructor receives the descriptor it is being called through,
an erased pointer to the immutable constructor frame, and an output slot for the
new object's owned `IUnknown`:

```rust,ignore
unsafe extern "C" fn construct_widget(
    class: *const neocogi_com::NcClassDescriptor,
    arguments: *const core::ffi::c_void,
    out: *mut *mut neocogi_com::IUnknown,
) -> neocogi_com::NcStatus;
```

Application code normally never writes this function. The class macro emits it,
checks the descriptor and frame signature, decodes the generated frame, invokes the
trait constructor, and publishes the resulting owned COM object.

Constructor rules:

- The constructor validates that `class.class_id` is the class it knows how to
  build.
- `NcAbiSignature::EMPTY` plus a null pointer is the canonical no-argument frame.
- A typed constructor uses a non-empty generated ABI signature and a non-null frame
  pointer. The runtime rejects a signature or pointer-shape mismatch before plugin
  code runs.
- Success writes one owned `IUnknown`; callers use signature-checked
  `QueryInterface` for the interface they need.
- Failure clears `out`.
- `Runtime::load_plugin_library` loads a shared library by explicit path,
  resolves the descriptor entrypoint, registers the descriptor, and keeps the
  loaded library in the process-global runtime for diagnostics and descriptor
  lifetime.
- `Runtime::load_plugin_libraries` loads required plugin paths
  sequentially in dependency-first order. Any failure prints/emits its
  diagnostic and aborts; partial activation never returns to the host.

See `component_plugin`, `component_plugin_counter`, `component_plugin_flag`,
`component_plugin_extra`, `component_plugin_metric`,
`component_plugin_state`, `component_host`, and `multi_plugin_host` for Unix
shared-library examples.

### Running The Shared Library Example

The examples use the default `neocogi-com` feature set, including `std` and the proc
macros. They are intentionally not supported under
`--no-default-features`; feature-minimal checks should target the library, not
the examples.

Build the plugin example as a shared library:

```sh
cargo build --example component_plugin
```

On Linux, run the host example against the produced shared object:

```sh
cargo run --example component_host -- target/debug/examples/libcomponent_plugin.so
```

Expected output:

```text
class: nc.examples.component.Greeter
greeting code: 123
```

The host example uses Unix `dlopen` directly to show the raw descriptor ABI.
`Runtime::load_plugin_library` provides the crate-owned loader path for
hosts that want the process-global runtime to own loaded libraries.

Build the plugins and run the multi-plugin host example:

```sh
cargo build -p neocogi-com-runtime
NC_COM_RUNTIME_LINK_DIR="$(pwd)/target/debug" \
  cargo build -p neocogi-com --example component_plugin_counter \
  --example component_plugin_flag --example component_plugin_extra \
  --example component_plugin_metric --example component_plugin_state \
  --no-default-features --features macros,std,runtime
LD_LIBRARY_PATH=target/debug \
  NC_COM_RUNTIME_LINK_DIR="$(pwd)/target/debug" \
  cargo run -p neocogi-com --example multi_plugin_host \
  --no-default-features --features macros,std,runtime -- \
  target/debug/examples/libcomponent_plugin_counter.so \
  target/debug/examples/libcomponent_plugin_flag.so \
  target/debug/examples/libcomponent_plugin_metric.so \
  target/debug/examples/libcomponent_plugin_state.so \
  target/debug/examples/libcomponent_plugin_extra.so
```

The host explicitly loads all five plugin paths in the supplied order and
verifies every class through the same process-global runtime. Each plugin and
the host independently declare the same callable class contract, so no
companion one-class interface or handwritten class ID is required.

### Startup Loading

Bundled required plugins should be loaded before normal host operation:

```rust
let runtime = neocogi_com::Runtime::global();
let loaded = unsafe {
    runtime.load_plugin_libraries([
        "plugins/base.so",
        "plugins/tools.so",
    ])
};
let base = &loaded[0];
# let _ = base;
```

core ABI v6 does not discover paths or recursively load plugin dependencies. Use
`neocogi-inspect plan` to validate dependencies and produce this explicit order.
Live activation calls must not be nested: native loader initialization and the
descriptor entrypoint must not directly or indirectly call neocogi-com loading while
another activation is in progress. If the operating-system loader maps a native
shared-library dependency automatically, that library is
still not registered as an neocogi-com plugin; the host must explicitly load its
neocogi-com descriptor when its classes are required through the runtime. Required
cross-plugin contracts should be expressed through constructor arguments and
COM capability interfaces. Declarative dependency resolution is deferred to a
future ABI generation.

For each activation, the runtime opens the library and validates and projects
its descriptor graph before taking the catalog write lock. The short blocking
phase checks current dependency availability and identity/class collisions,
then publishes the retained native handle, module record, plugin, classes,
relationships, and identity mappings atomically. Raw descriptor registration
uses the same catalog transaction without a module record, so a missing
dependency can be registered later and the original descriptor registration
retried without a partial catalog entry.

This is not a process rollback boundary. Loading a dynamic library can run
platform loader code, initialize globals or TLS, register callbacks, start
threads, or otherwise mutate process state before validation later fails.
`neocogi-com` therefore does not support dynamic plugin unload or hot reload. The
live loader emits the diagnostic, pins any opened module, and aborts on failure.

Directory scanning, plugin discovery paths, downloading, installation,
signature verification, certificate policy, allowlists, and plugin management UI
are application responsibilities. `neocogi-com` loads the explicit trusted paths the
host gives it and validates the in-process ABI contracts it finds there.

### Inspecting Shared Libraries

The linked runtime provides
`runtime.inspection().with_plugin_library_descriptor(path, visitor)` for trusted
libraries. It opens and validates the descriptor, invokes `visitor`
synchronously, and leaves the plugin and type registries unchanged. The
descriptor graph is borrowed only inside the visitor; copy any data needed
afterward. Opened handles remain pinned until process exit, and normal platform
loader code may run even though nothing is registered.

`neocogi-inspect` reads the actual plugin shared library, resolves
`nc_get_plugin_descriptor`, validates the descriptor graph, and copies the
plugin/dependency/class/interface/method metadata into an owned JSON or text
artifact. It does not register the plugin, construct objects, or follow declared
neocogi-com dependencies. Opened handles remain pinned until the short-lived
inspector process exits. Loading can still run normal platform loader code.
Inspection is therefore useful for trusted artifacts and diagnostics, not for
making an untrusted shared library safe to load.

```sh
cargo run -p neocogi-inspect -- dump target/debug/examples/libcomponent_plugin.so
cargo run -p neocogi-inspect -- dump target/debug/examples/libcomponent_plugin.so --format text
cargo run -p neocogi-inspect -- check target/debug/examples/libcomponent_plugin.so
cargo run -p neocogi-inspect -- plan plugins/base.so plugins/tools.so
cargo run -p neocogi-inspect -- diff old.nccom.json new.nccom.json
cargo run -p neocogi-inspect -- diff old.nccom.json new.nccom.json --compat
```

JSON artifacts use a strict versioned schema. They record the producer and core
ABI versions, target triple, pointer width, endianness, interface
identities/signatures, and derived method diagnostics. They intentionally omit
structured parameter, return, and aggregate definitions and are not a stable
source/interface-reconstruction contract. `diff` validates both artifacts
before comparing them, without loading either library; malformed identities,
duplicates, and invalid method indices are rejected.

By default, comparison is exact metadata equality and returns a nonzero status
for any difference. `--compat` performs a directional old-to-new release check
using interface IID/signature identity and the class, constructor, threading,
target/core ABI, and dependency metadata not covered by that signature. Compatible
side-by-side additions and diagnostic version/signature changes do not fail
compatibility mode. Class output includes `thread_local` or `thread_safe`;
weakening that capability is breaking while strengthening it is compatible.

## Structs

Use `#[com_struct]` for small fixed-size records that should cross the ABI by
value:

```rust
use neocogi_com::com_struct;

#[com_struct]
#[derive(Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

The macro owns the C-compatible representation, derives `Clone` and `Copy` when
they are not already listed, records physical size/alignment/field offsets in
`ComStruct`, and implements `AbiType` and `AbiPlainValue` for the struct. By
default, it verifies that every field implements `AbiPlainValue`.

Use `#[com_struct(packed)]`, `#[com_struct(packed(N))]`, or
`#[com_struct(align(N))]` when a contract requires non-default layout. Do not add
a raw `#[repr(...)]` attribute; the macro is the single source of truth for the
representation. Aggregate ABI signatures include size, alignment, field order, field
offsets, and field ABI types.

Struct fields are durable copied data, so the default check uses the stricter
`AbiPlainValue` marker rather than the lower-level `AbiType` marker. The macro
also rejects these types when they appear in field syntax:

- Rust references
- raw pointers
- `String`
- `Vec<T>`
- slices
- `ComPtr<T>`
- `ComRef<'_, T>`

Use `#[com_struct]` for small plain values such as points, rectangles, colors,
IDs, counters, and options. Use COM interfaces for variable-size data, borrowed
data, owned resources, or anything with explicit lifetime or release behavior.

### External C-layout fields

Use `#[com_struct(c_struct)]` when a record contains C-layout types from a
dependency that does not implement neocogi-com traits:

```rust
use neocogi_com::com_struct;

// Represents a dependency's type, with no neocogi-com dependency or traits.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

#[com_struct(c_struct)]
pub struct Vertex {
    pub position: Vec2f,
    pub uv: Vec2f,
}
```

`c_struct` is an option on `com_struct`. The enclosing record receives the COM
traits; its field types need no neocogi-com annotations or trait implementations.
This lets contract crates use existing C-layout math types directly without
coupling neocogi-com to a math library or converting between duplicate records.
The option also works with `packed`, `packed(N)`, or `align(N)`.

The macro checks the whole record with rustc's `improper_ctypes_definitions`
lint promoted to an error. An incompatible nested Rust-layout struct fails to
compile: the outer `repr(C)` does not change the layout of its fields. `Copy`
requirements and the syntactic field rejections above still apply. These checks
run at compile time and add no runtime conversion or allocation.

C layout alone does not establish plain-value semantics. The contract author
must ensure external fields contain no hidden pointers, references, borrowed
addresses, or ownership/release obligations, including inside nested types.

In this mode, **every field** is represented by its size and alignment, along
with its index and offset in the enclosing record. Field internals and scalar
type identities are not encoded. An incompatible field change that preserves
size and alignment therefore requires a new interface identity. Dependency
versions may differ only while the actual field layouts and value semantics
remain compatible; `repr(C)` does not freeze a dependency's future definitions.

## Object Lifetime

Object lifetime is reference-counted.

`QueryInterface` returns an owned reference on success. `ComPtr` releases its
owned reference on drop. Cloning a `ComPtr` calls `AddRef`.

When `Release` transitions the count from one to zero, generated storage is
destroyed immediately. Final release is not a fallible cleanup boundary, cannot
defer destruction, and must not resurrect the object. If cleanup can fail, expose
an explicit domain method such as `close`, `flush`, or `shutdown` and require
callers to invoke it while they still hold a COM reference.

Reference counts are `u32` values. Debug builds assert invalid edge states such
as overflow or release-after-zero. Release builds keep the counter from wrapping,
but those edge states still represent a broken ownership contract.

## Threading

Reference counting is atomic, but object methods are not assumed to be
thread-safe.

| Rust handle | Ownership and reference-count behavior | `Send` | `Sync` | Safe weakening |
|---|---|:---:|:---:|---|
| `ComPtr<T, ThreadLocal>` | Owns one reference; clone calls `AddRef`, drop calls `Release` | No | No | None |
| `ComPtr<T, ThreadSafe>` | Owns one reference; clone calls `AddRef`, drop calls `Release` | Yes | Yes | `into_thread_local` or `as_thread_local_ref` |
| `ComRef<'a, T, ThreadLocal>` | Borrows a reference; copy and drop do not change the count | No | No | None |
| `ComRef<'a, T, ThreadSafe>` | Borrows a reference; copy and drop do not change the count | Yes | Yes | `into_thread_local` |

The marker is needed even though generated classes have a threading declaration.
The interface type `T` describes a method/vtable contract, not one concrete
implementation: a thread-local class and a thread-safe class can both implement
the same `T`. After construction or `QueryInterface`, `ComPtr<T, M>` has erased
the concrete Rust class type. The raw interface pointer cannot fill the gap
because thread-local and thread-safe pointers have exactly the same C layout and
carry no runtime threading tag. `M` is therefore the compile-time evidence that
survives type erasure. Class descriptors do carry a runtime declaration, but a
runtime field cannot make a Rust value conditionally implement `Send` or `Sync`;
checked constructors first validate that declaration and then return the
corresponding statically marked pointer type.

`Send` and `Sync` represent different parts of the promise. Sending an owning
pointer allows its eventual final `Release`—and therefore the Rust value's
destructor—to run on another thread. Sharing a pointer permits simultaneous
method calls through aliases. An atomic reference count makes concurrent
`AddRef` and `Release` possible, but it does not synchronize method state,
thread-affine OS handles, contained pointers, callbacks, or destruction.

By default, generated classes produce:

```rust
ComPtr<T, ThreadLocal>
```

Thread-local pointers are not `Send` or `Sync`.

If a class implementation is safe to call from multiple threads, including
concurrent calls, declare it as thread-safe:

```rust
#[com_object(thread_safe, interfaces(ICalculator))]
pub struct Calculator;
```

That produces:

```rust
ComPtr<T, ThreadSafe>
```

The macro requires the Rust class to be `Send + Sync`, but that is only the
baseline. The implementation must also make its exposed methods logically safe
under concurrent calls and permit final destruction on any thread.

Every generated class implements `ComClass`; its `Threading` associated type is
the single source for the class's constructor pointer types and descriptor
metadata. A thread-safe owner may explicitly discard its stronger capability
without changing the COM reference:

```rust,ignore
let local_owner = thread_safe_owner.into_thread_local();
let local_borrow = another_thread_safe_owner.as_thread_local_ref();
```

Borrowed `ComRef<_, ThreadSafe>` values provide the corresponding
`into_thread_local` operation. These are one-way capability downgrades; there is
no safe thread-local-to-thread-safe conversion. Weakening removes permission and
therefore needs no new object proof. Upgrading would invent a class guarantee
that is absent from both the handle and raw pointer. This also means
`ThreadLocal` is a property of a particular handle, not necessarily the
underlying object: deliberately weakened local handles can coexist with
thread-safe aliases to the same object.

Generated class descriptors preserve the same distinction for dynamic hosts.
`Runtime::construct_class` remains a conservative thread-local view. A host that
needs the stronger capability requests it explicitly:

```rust,ignore
let calculator = unsafe {
    runtime.construct_class_as_with_model::<ICalculatorAbi, ThreadSafe>(
        CALCULATOR_CLASS_ID,
        core::ptr::null_mut(),
    )?
};
```

The runtime rejects a thread-safe request for a thread-local class before the
class constructor runs. A thread-safe class may always be consumed as
thread-local; that is a capability downgrade, not an unsafe upgrade. Atomic
reference counting alone is not sufficient—the stored Rust value, methods,
contained COM pointers, foreign resources, and destruction behavior must all
satisfy the threading promise.

## Interface Identity

Prefer stable `auto_guid` names for new interfaces:

```rust
#[com_interface(auto_guid = "my.product.ICalculator:v1")]
pub trait ICalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
}
```

The exact string is stored as normative qualified-name metadata and is the sole
UUIDv5 input. Do not use a Rust module path unless refactoring that path should
intentionally change the interface ID.

Use an explicit UUID as a golden assertion of the same derivation:

```rust
#[com_interface(
    "59fb4b83-32b5-5719-a30c-d77fc36a26a9",
    name = "my.product.ICalculator:v1"
)]
pub trait ICalculator {
    fn add(&self, a: i32, b: i32) -> Result<i32, NcStatus>;
}
```

Automatic, explicit, and constant-path IDs must all equal
`guid_from_qualified_name(name)`. Changing the qualified name, method order,
argument types, return types, ownership semantics, or interface ID is an ABI
break. Create a new interface version instead.

## ABI Compatibility Policy

The current draft core ABI v6 specification lives in
[`docs/abi-v6.md`](docs/abi-v6.md). core ABI v6 uses an unversioned plugin descriptor
entrypoint. core ABI v6 is an experimental pre-alpha draft and has not been
ratified. Cross-release compatibility begins only when a named release/tag and
changelog entry explicitly establish the ABI-v6 freeze baseline. Until then,
breaking draft changes and intentional golden-vector updates are permitted;
hosts and plugins from different neocogi-com source releases are not guaranteed to
interoperate.
`IUnknown::QueryInterface` receives an `NcInterfaceId`, which pairs the IID
with the expected interface ABI signature. Class descriptors carry a separate
signature for their immutable typed constructor frame.

For interfaces, binary compatibility is identified by the pair:

- interface ID (`IID`)
- generated interface `ABI_SIGNATURE`

The signature is a canonical mangled byte string over the typed ABI contract:
interface IID, method order, method names, argument and return ABI types,
aggregate size/alignment/field offsets, passing modes, void versus `out`
returns, `ComRef` borrowed parameters, `ComPtr` owned returns, safe buffer
passing modes, referenced interface IIDs, inherited interface signatures, and
the `NcStatus` status convention. Grammar v1 uses fixed record tags and
fixed-width hexadecimal values. Procedural macros emit declaration records;
const evaluation resolves layouts and nested identities unavailable during
macro expansion.

The signature grammar is itself part of the ABI policy. Once core ABI v6 has been
ratified and an interface has been published, changing how `neocogi-com`
canonicalizes the same Rust interface into a mangle is an ABI-policy event,
even if the generated vtable layout would
otherwise be unchanged. Golden tests should cover representative interface IDs,
interface signatures, method order, typed method descriptors, rendered
diagnostic method text, and descriptor metadata. GUIDs and canonical mangles
are the compatibility authority. Descriptor validation walks each advertised
interface signature against its typed metadata once and rejects mismatches.
Calling `format_method_signature` on a raw public descriptor is unsafe because
the caller must establish every transitive pointer/count invariant. Generated
static descriptors meet that contract; successfully loaded plugin descriptors
remain live because the runtime keeps their modules mapped until process exit.

Qualified names in plugin, class, and interface descriptors are normative
UUIDv5 identity inputs. Versions remain for humans, diagnostics, and host
policy. A successful binary match requires the expected qualified-name-derived
GUIDs and interface ABI signatures.

The descriptor macros reject invalid names and versions during expansion, and
the default `Runtime` applies the same policy again before registering direct
or foreign descriptor producers. Qualified names must contain `1..=1024` bytes
of non-empty ASCII identifier segments separated by `.` or `:`, such as
`example.image.Decoder:v1`; version strings must contain `1..=128` printable
ASCII bytes without whitespace. The allocation-free const functions
`validate_qualified_name` and `validate_version_text` expose the shared policy
for direct metadata producers. Hosts that need richer package or dependency
version policy should encode it in application-owned manifests, capability
interfaces, or other metadata outside the raw plugin descriptor.

Do not mutate an existing interface contract in place. These changes require a
new versioned interface name and therefore a new IID:

- adding, removing, renaming, or reordering methods
- changing any lowered argument or return type
- changing `ComRef` versus plain value ownership
- changing `ComPtr` returned-interface ownership or target interface
- changing a referenced interface IID (complete referenced identities are checked at pointer boundaries)
- changing whether success returns through an `out` pointer or is void

Adding a method to the end of a vtable is still an ABI change for `neocogi-com`
interfaces. Keep the old interface available and expose a new side-by-side
interface such as `example.IFooNext`.

For core descriptor structs, compatibility is controlled by
`NC_COM_CORE_ABI_VERSION`. Descriptor structs are exact v1 `#[repr(C)]` layouts;
field order, field meaning, and descriptor layout changes require a core ABI
version break unless a future ABI first defines an explicit extension mechanism.

`NC_COM_CORE_ABI_VERSION` must be bumped for changes to `IUnknown`, descriptor
field order, descriptor field meaning, status-code numbering,
calling convention, layout, or cross-boundary ownership rules.

The descriptor entrypoint symbol name is `nc_get_plugin_descriptor`. Hosts must
still reject descriptors whose `core_abi_version` does not match their
supported `NC_COM_CORE_ABI_VERSION` before interpreting version-specific fields.
A future ABI only needs a new symbol if the entrypoint function shape or
discovery rules change.

The shared runtime has an independent `NC_COM_RUNTIME_ABI_VERSION`. Incompatible
changes to exported getters or existing API-table prefixes require a new runtime
ABI generation even when the core descriptor ABI is unchanged.

After ratification, status code numbers are stable once published. Do not
renumber existing `NC_*` values, do not reuse removed values, and add new
built-in error codes by assigning previously unused negative values.

After ratification, removing a published interface, status code, descriptor
field, or plugin symbol requires a documented compatibility break. Prefer
keeping old interfaces available side by side with new versioned interfaces for
at least one supported release line.

At object adoption boundaries, validate the registered class/interface contract
before normal typed use:

```rust
use neocogi_com::{ComInterface, ComPtr, IUnknown, NcGuid, NcStatus, Runtime};

/// # Safety
/// `raw_unknown` must transfer exactly one owned `IUnknown` reference.
unsafe fn adopt_registered<T: ComInterface>(
    class_id: NcGuid,
    raw_unknown: *mut IUnknown,
) -> Result<ComPtr<T>, NcStatus> {
    let runtime = Runtime::global();
    // SAFETY: the caller provides the ownership guarantee documented above.
    unsafe { runtime.adopt_unknown_as::<T>(class_id, raw_unknown) }
}
```

After adoption, keep the returned typed `ComPtr<T>` and call methods directly.
`ComPtr::query::<T>()` also validates the live object's `T::IID` and
`T::ABI_SIGNATURE`, so interface casts stay tied to the generated ABI contract. The
linked runtime integration suite compiles and executes this owned-reference
adoption path.

`NcGuid` supports canonical UUID formatting and parsing:

```rust
use neocogi_com::NcGuid;

let iid: NcGuid = "7b3fb46c-83ab-49e1-a5c8-2bc6d7380d2e".parse()?;
assert_eq!(iid.to_string(), "7b3fb46c-83ab-49e1-a5c8-2bc6d7380d2e");
# Ok::<(), neocogi_com::ParseGuidError>(())
```

For logs, manifests, or external metadata, use `to_uuid_bytes()` and
`from_uuid_bytes()`. Those APIs use canonical UUID byte order, not native memory
order of the `#[repr(C)]` struct.

## Enums

Use `com_int_enum!` for enum-like scalar values that must preserve unknown
values:

```rust
use neocogi_com::com_int_enum;

com_int_enum! {
    pub enum NodeKind: u16 {
        ELEMENT = 1,
        TEXT = 3,
        DOCUMENT = 9,
    }
}
```

Use `#[com_enum]` for tagged payload values with an explicit C-shaped
representation:

```rust
use neocogi_com::com_enum;

#[com_enum(tag = u32)]
#[derive(Debug, PartialEq)]
pub enum Mutation {
    #[com_tag(1)]
    None,
    #[com_tag(2)]
    ChildAdded(u64),
}
```

Payload fields must implement `AbiPlainValue`. Owning Rust values and borrowed
pointers are rejected because the generated tagged representation has no
drop/release protocol.

The generated `MutationAbi` is opaque to safe Rust while retaining its
`repr(C)` tag + union layout. Use `MutationAbi::from(value)`, `tag()`, and
`Mutation::try_from(abi)` for known variants. `from_unknown_tag` creates a value
that safely reports `NC_E_INVALID_TAG`; low-level raw tag/payload construction
requires the documented unsafe `from_raw_parts_unchecked` API.

## Features

Default features:

```toml
default = ["macros", "std"]
```

Feature behavior:

- `std`: enables `alloc` and standard-library integrations.
  Panics are not caught by `neocogi-com`; `panic=abort` aborts at the panic site,
  and an unwinding panic that reaches an `extern "C"` ABI thunk aborts under
  Rust's ABI unwind rules instead of becoming an `NcStatus`.
- `alloc`: required for `#[com_object]` because generated object storage is heap
  allocated.
- `macros`: re-exports `neocogi-com-macros`.
- `runtime`: enables `std` and the process-global `Runtime` facade backed
  by the linked `libnc_com_shared_runtime` C ABI. It requires an already built runtime
  shared library on the native link path and discoverable by the platform loader.
- `no_std`: supported when `std` is disabled; generated interfaces and `ComPtr`
  work without allocation, while generated classes require `alloc` and
  `Runtime` requires `runtime`.

The core descriptor validator remains available without `alloc`. Allocation-
enabled builds use `alloc::collections::BTreeSet` to accelerate duplicate
detection while preserving the same diagnostics. Runtime registry hash indexes
live only in the intentionally `std`-only `neocogi-com-runtime` implementation; they
do not change the reduced-feature facade or ABI crates.

## C Interop Status

The ABI is C-compatible and includes a native C fixture that calls generated
vtables, signature-checked `QueryInterface`, `AddRef`, `Release`, borrowed
interface parameters, owned interface returns, both offset-buffer lowering
directions, and a representative padded aggregate. The fixture also locks the
changed root descriptor layouts and all five runtime-table prefixes with C
`_Static_assert`s, then compares the C results with Rust `size_of`, `align_of`,
and `offset_of` values at runtime.

That proves the ABI shape is consumable from C when the C side has correct
declarations. This repository does not currently generate public C headers.
Header generation and C-first distribution are separate concerns, not
requirements for first-class Rust component interoperability. Aggregate
signatures expose the canonical layout and nested type records needed for ABI
validation, but not Rust field names, source paths, or a reconstructable source
declaration.

## Compatibility Focus

The main compatibility target is Rust code loaded or linked across the explicit
C-shaped ABI boundary, which becomes stable across releases after ratification:

- host/plugin systems
- independently compiled components
- dynamic or static libraries with different crate graphs
- long-lived binary interfaces where Rust compiler or dependency versions may
  differ
- builds that use different optimization settings or link-time choices

After ratification, compatibility applies only to the explicit C-shaped surface
generated by `neocogi-com`. Do not pass Rust-owned containers, borrowed Rust
references, trait objects, or allocation ownership across the boundary unless
they are wrapped in an explicit COM interface with defined ownership rules.

## Contract Crate Workflow

For product or plugin ecosystems, put shared ABI definitions in a small contract
crate instead of duplicating interface traits in each host and plugin. The
contract crate should be `no_std` and contain only `#[com_interface]`,
`#[com_struct]`, `com_int_enum!`, `#[com_enum]`, qualified-name-derived
class/plugin IDs, and shared constants. It should not contain implementations,
loader code, `Runtime`, heap-owned Rust containers, or host policy.

Hosts and plugins can then depend on the same contract crate while still being
built with different Rust compiler versions, dependency graphs, and build flags.
The contract crate prevents accidental source drift, but binary compatibility is
still decided by `NC_COM_CORE_ABI_VERSION`, interface IDs, and interface
`ABI_SIGNATURE` values at runtime.

Recommended contract-crate release policy after ABI ratification:

- patch releases are documentation, tests, or non-ABI changes only
- minor releases add new side-by-side interfaces, records, enum values, or class
  IDs while keeping old contracts available
- major releases may remove old contracts or intentionally change compatibility
  policy
- published interfaces are not mutated in place; use a new side-by-side
  interface name for any method, type, order, ownership, or signature-grammar change
- golden tests should assert interface IDs, ABI signatures, method order, typed
  method descriptors, rendered diagnostic signatures, descriptor metadata, and
  important class/plugin IDs

Before ratification these goldens describe the current draft and may be updated
intentionally with an ABI change; they are not permanent publication merely by
being present in the repository.

The repository includes a contract-crate workflow example under
`workflow-examples/contract-crate`. Its `abi` crate exports the shared contract,
its `plugin` crate implements that contract, and its `host` crate consumes the
same contract crate, checks golden ABI metadata, loads the plugin through
`Runtime::load_plugin_library`, constructs a class, and performs
cross-boundary calls. The host side also demonstrates a fail-fast policy for
required in-process components: panic with the loader or `NcStatus` diagnostic
on plugin contract violations, including required method-call failures, rather
than continuing with potentially corrupted process state.

## Test Toolchain Requirements

The workspace declares Rust 1.85 as its minimum supported Rust version (MSRV).
Running the complete local verification suite therefore requires the exact
1.85.0 toolchain and the pinned 1.97.1 diagnostic baseline in addition to the
moving `stable` and `nightly` toolchains used by compatibility, Miri, and
sanitizer coverage. Install them with `rustup`:

```sh
rustup toolchain install 1.85.0 1.97.1 --profile minimal
rustup toolchain install stable nightly --profile minimal
```

The root `rust-toolchain.toml` selects Rust 1.97.1 for ordinary unqualified
`cargo` commands. This keeps exact trybuild stderr snapshots reproducible;
moving-toolchain compatibility scripts continue to select `stable` and
`nightly` explicitly.

Verify the pinned MSRV compiler with:

```sh
rustc +1.85.0 --version
```

The MSRV checks must run with `cargo +1.85.0`; current `stable` is not a
substitute because a newer compiler does not restrict source or standard-library
features to those available in Rust 1.85. The individual scripts report any
additional components they require and do not silently install or update
toolchains.

Run the same pinned MSRV gate used by CI with:

```sh
scripts/test-msrv.sh
```

It compiles every workspace target and feature combination, runs workspace
library and inspector tests, and checks the default, no-default, and alloc-only
facade configurations. Compiler-diagnostic snapshot tests run on the pinned Rust
1.97.1 baseline because their rendered output intentionally varies between
compiler releases; the MSRV gate still compiles their harness and every Cargo
test target.

## Linked Runtime Test Suite

The ordinary workspace test command intentionally covers the default
configuration without the linked `runtime` feature. Run the complete facade,
shared-library, contract-workflow, and cross-version suite with:

```sh
# Unix
scripts/test-linked-runtime.sh

# Windows PowerShell
./scripts/test-linked-runtime.ps1
```

Both scripts build the `neocogi-com-runtime` shared library, configure the native
link and platform loader paths, enumerate the all-feature tests, assert that
representative runtime and dynamic-library tests were discovered, and then run
`cargo test -p neocogi-com --all-features --locked`. Stable and nightly toolchains
are required because the cross-version fixture builds producers with both. CI
runs the same linked suite on Linux, macOS, and Windows in addition to the
ordinary workspace matrix.

## Build Compatibility Matrix

This matrix describes what may differ between independently built host and
component artifacts while preserving the `neocogi-com` ABI contract.

The executable local matrix is:

```sh
scripts/test-build-flags.sh
```

CI separately compile-checks the little-endian x86-64, x86-32, Armv7, and
AArch64 targets supported across Linux, Windows, macOS, and iOS. It also runs a
real 32-bit Linux ABI row that executes core layout/GUID/signature tests and the
native C fixture. On an x86-64 Linux host with the i686 Rust target and
multilib C toolchain installed, run the same row with:

```sh
scripts/test-abi-32.sh
```

Big-endian targets are intentionally outside the current pre-alpha validation
matrix. A host and component must still have identical target ABI facts.

It is Unix-only and requires both `stable` and `nightly` Rust toolchains to be
installed through `rustup`:

```sh
rustup toolchain install stable nightly
```

The script runs workspace test targets for both toolchains. For each toolchain
it runs a default dev test row, then release rows for `panic=unwind` and
`panic=abort` crossed with `lto=off`, `lto=thin`, and `lto=fat`. Every row also
builds the real `neocogi-com-runtime` shared library with the same profile and flags,
links a runtime-enabled `neocogi-com` facade test binary against it, calls the
process-global runtime API, and loads and invokes a fixture plugin. Test-name
sentinels make the matrix fail if either linked smoke test is not discovered.
Each linked row has its own target and native-loader paths so artifacts from a
different profile cannot satisfy it accidentally. The complete linked-runtime
suite above retains the more expensive contract and cross-version workflows.
Exact trybuild stderr snapshots that depend on rustc diagnostic wording are
stable-only when nightly diagnostics drift. The script deliberately does not
pass `--locked`.

The cross-version component integration test also builds a shared-library
fixture with stable and nightly debug producers plus selected release producer
skews: stable `panic=abort`/thin LTO and nightly `panic=unwind`/fat LTO. Each
artifact is exercised in its own subprocess through both the raw plugin
descriptor entrypoint and `Runtime::load_plugin_library`, then validated through
registry metadata, registry construction, and cross-boundary COM method calls.
Process isolation is required because all variants intentionally publish the
same stable plugin/class identities and successfully activated modules remain
pinned in the process-global runtime. Fixture builds use the committed lockfile
without modifying it.

| Build factor | Can differ across artifacts? | Notes |
| --- | --- | --- |
| Rust compiler version | Yes | The ABI is C-shaped, not Rust-native. Local tests cover stable and nightly producers, including selected release build-flag skews. The generated `NC_COM_CORE_ABI_VERSION` and interface ABI signatures still have to match the host's expectations. |
| Cargo dependency graph | Yes | Implementation dependencies are private. Types that cross the boundary must be lowered to `neocogi-com` ABI types or COM interfaces. |
| `neocogi-com-core` ABI version | Only if compatible | Plugin descriptors carry `NC_COM_CORE_ABI_VERSION`; hosts reject incompatible core ABI versions. |
| Interface method order, types, GUID, ABI signature | No | This is the actual interface contract. Changing it creates a different ABI and should use a new interface version or GUID. |
| `opt-level` | Yes | Optimization level is private to each artifact. It must not change the explicit `repr(C)` ABI surface. |
| LTO | Yes, within an artifact | LTO can optimize code inside the host or component, but the ABI boundary is still the generated `extern "C"` vtable. Proc macros run at compile time and are not part of the target artifact that LTO links. |
| Codegen units and incremental compilation | Yes | These affect how Rust builds an artifact, not the C-shaped interface layout. |
| Debug assertions | Yes | They may change local checking behavior, but they are not part of the ABI. Do not rely on debug-only panics for cross-component error handling. |
| `panic=abort` vs `panic=unwind` | Mixed builds are callable, but panics are not recoverable ABI errors | `neocogi-com` does not catch panics. `panic=abort` aborts at the panic site. With unwinding, a panic that reaches an `extern "C"` thunk aborts under Rust's ABI unwind rules. Use `NcStatus` for expected failures. |
| `std`, `alloc`, `no_std` features | Yes, when the exposed ABI is unchanged | Generated interfaces and `ComPtr` work without allocation. Generated classes require `alloc`. No `std` type crosses the ABI boundary. |
| Dynamic vs static linking | Yes | Shared-library plugins and statically linked components can both use the same descriptors and vtables. Do not rely on Rust symbol or type identity across the boundary. |
| Target triple, pointer width, endianness, C calling convention | No for one process boundary | Host and component must be built for the same process ABI. Cross-architecture or cross-endian communication needs a serialization/RPC layer, not raw vtables. |
| Allocator choice | Yes, if ownership does not cross | Allocation and deallocation stay inside the object that owns the memory. Transfer ownership with COM references, not Rust heap containers. |

## Quality and Package Gates

The local quality gate is:

```sh
scripts/test-quality.sh
```

It runs formatting, Clippy over all targets and features with warnings denied,
the supported reduced-feature checks, compiled normative examples embedded in
`docs/abi-v6.md`, repository-local Markdown links, and rustdoc with warnings
and missing public documentation denied for the workspace/all-feature, facade
default-feature, no-default, and alloc-only modes. Each shipped Rust crate also
denies `missing_docs` at its crate root, so ordinary checks reject undocumented
public API additions before the documentation build. CI invokes this script
directly.

The quality gate compiles the canonical facade macro examples and, through a
separate non-publishable harness, the ABI specification's blocks marked `rust`;
`rust,compile_fail` blocks must be rejected, while schematic declarations are
explicitly `rust,ignore`. The
runtime-path integration test runs both `cargo check` and warning-denied rustdoc
for generated items using an explicit facade override. Macro expansion otherwise
uses the canonical `::neocogi_com` path and does not inspect Cargo metadata.

### Registry Scaling Benchmark

Run the manual release-mode registry benchmark with:

```sh
scripts/bench-registry.sh
```

It measures ordered index construction, class lookup, and relationship reads at
10, 1,000, and 10,000 classes. It reports timings for comparison rather than
enforcing machine-dependent wall-clock thresholds. Registry enumeration remains
registration-ordered; hash indexes are never enumerated through the ABI.

The local package gate is:

```sh
scripts/test-packages.sh
```

It builds all six `.crate` archives, requires each archive to contain its README,
source, manifest, and an exact copy of `LICENSE-MIT`, rejects repository-only
files, extracts the artifacts, and compiles them in dependency publication order.
Internal dependencies resolve only to package artifacts verified earlier in that
order. The script first fetches the workspace's locked third-party dependencies;
verification of the extracted archives is then offline. CI invokes the same
script.

## Safety Regression Tests

The executable local safety suite is:

```sh
scripts/test-safety.sh
```

It is Unix-only and requires the nightly toolchain plus Miri components:

```sh
rustup toolchain install nightly
rustup +nightly component add miri rust-src
```

The script runs Miri over pure Rust targets: `neocogi-com-core`, `neocogi-com-runtime`,
`neocogi-com`, and the Rust-only COM lifecycle integration tests. Discovery
sentinels require a five-descriptor dependency-planning/registration test and a
five-object generated COM lifecycle test, so a feature or `cfg` change cannot
silently reduce those rows to empty targets. The script intentionally excludes
the native C fixture, shared-library loading, trybuild diagnostics, and
subprocess panic tests because those are not useful Miri targets. Real
multi-plugin shared-library loading remains covered by normal linked tests, the
build-flag compatibility matrix, and the sanitizer suite below. The script
deliberately does not pass `--locked`.

The executable local sanitizer suite is:

```sh
scripts/test-sanitizers.sh
```

It is currently Linux-only for the `x86_64-unknown-linux-gnu` host target and
requires the nightly toolchain plus `rust-src`:

```sh
rustup toolchain install nightly
rustup +nightly component add rust-src
```

The script runs AddressSanitizer and ThreadSanitizer over `neocogi-com-core`,
`neocogi-com-runtime`, `neocogi-com`, and the Rust-only COM lifecycle integration tests. It uses
target-specific sanitizer flags and `-Z build-std` so proc macros are not
instrumented as host tools while the test binaries and standard library are
built with matching sanitizer ABI settings. AddressSanitizer leak detection is
disabled because LeakSanitizer is not reliable under ptrace-style test runners;
Miri and explicit drop-count lifecycle tests cover those ownership regressions.
The script deliberately does not pass `--locked`.

Miri and both sanitizer runs are also release gates in the scheduled/manual
`Release Safety` CI workflow. CI invokes the same repository scripts and retains
their complete logs as workflow artifacts even when a gate fails.

### Borrowed interface calls

Generated interface extension traits work on both `ComPtr<T, M>` and
`ComRef<'_, T, M>`. Both use the same argument, status, and returned-pointer
validation. A synchronous borrowed call does not acquire an owned reference:
`value.as_com_ref().method()` keeps the original owner responsible for lifetime.
Use `add_ref()` only when retaining the object after the borrowed call. The same
rule applies to the built-in string and byte helpers. A borrowed pointer can still
be retained explicitly; it must never wrap a stack-borrowing COM implementation.

### Runtime deployment modes

`runtime` links the shared runtime. Build `neocogi-com-runtime` first, provide its
link directory, and package that one library alongside native plugins. UI/plugin
implementation crates normally need only contracts, objects, and macros.

`static-runtime` includes `runtime` and links the exact same implementation once
into the final executable or browser module. It needs no shared runtime library.
Register each `plugin_descriptor()` explicitly in dependency order. The browser
uses static registration; native dynamic loading remains unsupported there.
Do not link a static runtime into each native plugin: those would be separate
registries. Select the mode at the final application, and avoid `--all-features`
for native shared deployment because it enables `static-runtime` too.

Mode-specific verification:

```sh
cargo test -p neocogi-com --features static-runtime --test com_usage --test plugin_assembly
cargo check -p neocogi-com --features static-runtime --target wasm32-unknown-unknown
# The linked-runtime script builds/tests the independent shared mode.
crates/com/scripts/test-linked-runtime.sh
```
