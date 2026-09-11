# neocogi-com

`neocogi-com` is the user-facing crate for cross-platform Rust component interop
through an explicit C-shaped ABI. It generates COM-style interface headers,
vtables, status returns, GUID/signature identity, class metadata, and reference-counted
object storage without exposing Rust trait objects or Rust allocator ownership
across the binary boundary.

Most users should depend on this crate. The lower-level workspace crates contain
the core ABI types, proc-macro implementation, linked runtime declarations,
runtime shared library, and inspection CLI.

## Documentation

- [API documentation](https://docs.rs/neocogi-com) contains the compiled examples and
  feature-sensitive item documentation.
- The [workspace guide](https://github.com/NeoCogi/neocogi-platform#readme) is the canonical
  long-form guide and owns the exact build/test compatibility matrix.
- The [core ABI v7 specification](https://github.com/NeoCogi/neocogi-platform/blob/master/crates/com/docs/abi-v7.md)
  is the normative binary contract.

This package README is intentionally a concise entry point. Exact test rows,
release procedures, and detailed compatibility tables live only in the workspace
guide so they cannot drift across copies.

## Quick Example

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

The equivalent crate-level rustdoc example is compiled by the workspace doctest
suite. `com_interface` generates the portable interface header/vtable and the
unsafe `ComInterface` certification required by safe `ComPtr` operations.
`com_object` generates COM object storage, canonical `IUnknown` metadata,
signature-checked `QueryInterface`, and reference counting.

Macro documentation examples are compile-checked in the facade rustdoc, where
the complete public API is available; intentionally incomplete lowering and
generated-shape sketches are marked as text. The runtime-path integration fixture
also builds its generated rustdoc with warnings denied, covering the explicit
facade override without parsing Cargo metadata.

## Boundary Rules

- Interface methods return `Result<T, NcStatus>`; `Result<(), NcStatus>` is the
  status-only void-success form.
- Interface methods named `query_interface`, `add_ref`, or `release` are
  rejected because those names belong to generated `IUnknown` lifecycle
  thunks. Parameter names remain source-derived and readable; the macro rejects
  only names that would duplicate a lowered buffer slot or shadow a generated
  adapter binding.
- A method's `Result::Err` must contain a negative `NcStatus`. Generated thunks
  preserve negative errors and normalize an invalid non-negative error to
  `NC_E_INVALID_ARG` before returning across the ABI. For non-void methods, the
  output slot remains unwritten on this contract violation.
- Plain values must implement the ABI marker traits. Use `com_struct` for fixed
  records, `com_int_enum!` for transparent integer values, and `com_enum` for
  explicit tag/payload layouts. `#[com_struct(c_struct)]` lets a record contain
  external C-layout fields without neocogi-com traits. It checks nested FFI layouts;
  authors must ensure plain-value semantics and use a new interface identity
  for incompatible changes its field size/alignment signature cannot detect.
  See the `com_struct` API docs.
- Use `ComRef<'_, IFooAbi>` for borrowed interface parameters and
  `ComPtr<IFooAbi>` for owned interface returns.
- Use bounded copy views for call-scoped buffers and COM interfaces such as
  `INcStringAbi` or `INcBytesAbi` for owned or long-lived variable-size data.
- Owned `INcBytesAbi`/`INcStringAbi` conversions default to a 64 MiB ceiling.
  Use the `_bounded` conversions to choose another maximum or `copy_to` to fill
  caller-owned chunks without host allocation. Oversized claims return
  `NC_E_OUT_OF_RANGE`; failed in-limit reservations return
  `NC_E_OUT_OF_MEMORY`.
- Never pass Rust `String`, `Vec<T>`, slices, references, trait objects, or raw
  allocator ownership across the ABI.
- `IUnknown` is the implicit canonical class root. Class macros add it exactly
  once and reject explicit or duplicate interface entries.
- Interface/class/plugin GUIDs are derived from their normative qualified names.
  Interface compatibility is the derived IID plus generated ABI signature.
- A panic is not an ABI error. Return `NcStatus` for expected failures.

## Typed Source Decoding

`ComDeserialize` separates concrete Rust construction from an input format.
Use `#[com_struct(deserialize)]` for an ABI record,
`#[com_enum(tag = u32, deserialize)]` for a payload enum, and
`#[derive(ComDeserialize)]` for an ordinary local record or fieldless enum.
Named records reject unknown, duplicate, and missing fields. Add `default` to a
COM struct option list, or `#[com_deserialize(default)]` to a derive, when
omitted fields should retain `Default` values. Add `sequence` for positional
record input such as a compact RGBA tuple.

An input crate implements `ComDeserializer` once and keeps any framework-specific
visitor machinery private. The value implementations recursively request exact
member types. Arrays, options, owned vectors, boxed slices, and ordered maps use
blanket implementations; borrowed slices do not, because arbitrary input formats
cannot provide a sound borrow lifetime.

This protocol is separate from COM ABI descriptor metadata. It performs no
registry lookup, offset-based mutation, `Any` downcast, or erased value-tree
allocation, and it is not a promise to reconstruct Rust declarations from a
plugin binary.

## Runtime And Plugins

The optional `runtime` feature exposes the stateless `Runtime` facade over the
separately built and linked `libnc_com_shared_runtime` shared library. The shared
runtime owns process-global registry state. Successfully loaded modules remain
mapped until process exit; unload and hot reload are not core ABI v7 operations.

IDL-based plugins export `nc_get_factory_plugin_description`. Their descriptions
contain plugin identity, explicit hard dependency IDs, exact generated contract
module references, and one root factory bootstrap. `factory_plugin!` obtains the
root ID and threading mode from the type associated by `#[com_factory]`; plugin
authors do not repeat those values. `Runtime::register_factory_plugin_description`
supports static assemblies, while `Runtime::load_plugin_library` detects this
description model and maps the DSO plus its fingerprint-qualified contract
dependencies. Hosts obtain the root through
`Runtime::activate_plugin_factory::<GeneratedFactoryAbi>` and perform all later
construction through typed factory methods.

The earlier `nc_get_plugin_descriptor`/`plugin!` class-table path remains
isolated for components that have not migrated. It contains class constructor
frames; new IDL components should not add classes to that registry. The common
loader rejects a DSO exporting both description entrypoints. It does not
recursively load neocogi-com plugin paths: hosts obtain an explicit
dependency-first plan and load plugin dependencies before dependents.
Live activation calls must not be nested. Native loader initialization and the
description entrypoint must not directly or indirectly call neocogi-com loading;
operating-system loading of ordinary native dependencies remains permitted and
does not register them as neocogi-com plugins.

Live activation is fail-fatal. A path, native-loader, symbol, description,
dependency, or registration failure emits a diagnostic and aborts rather than
returning partial runtime state. Inspection is non-registering:
`Inspection::with_plugin_library_descriptor` is the callback-scoped legacy
descriptor view. `neocogi-inspect` auto-detects either activation model and
copies owned metadata—including a factory's canonical contract module graph—in
its short-lived process. Neither operation changes a registry.

Dynamic loading returns an owned `LoadedModuleInfo`. Its module
ID, path, and plugin ID come from module inspection; its qualified name and
version come from the registered plugin description through plugin inspection.
Use `runtime.inspection().loaded_module(module_id)` to retrieve the combined
summary later, or `runtime.inspection().plugin_info(plugin_id)` when only plugin
metadata is needed. The latter also works for statically registered plugins,
which do not have a loaded-module path. Common plugin enumeration and summary
inspection include both activation models; factory plugins simply have no
class-table relationships.

Runtime ABI v1 retains at most 1,024 dynamically loaded modules; exceeding that
limit is a fatal activation failure. It permits 64 simultaneous debug messenger
registrations; creating another returns `NC_E_OUT_OF_RANGE`, while dropping a
messenger frees one active slot without reusing its handle.

The Rust runtime facade copies at most 65,536 elements per registry enumeration
and retries append-only growth at most eight times after the initial fill call.
It rejects non-progressing `NC_E_INSUFFICIENT_BUFFER` responses and limits copied
inspection text to 1 MiB. Raw C count/fill callers remain responsible for their
own allocation and retry policy.

The exact stable/nightly producer rows, panic/LTO skews, raw descriptor checks,
and public runtime activation paths are maintained in the canonical
[Build Compatibility Matrix](https://github.com/NeoCogi/neocogi-platform#build-compatibility-matrix)
and enforced by the packaged `cross_version_component` integration test.

## Features

- `std` (default) enables allocation and standard-library integrations.
- `alloc` enables generated heap-backed classes and owned data helpers.
- `macros` (default) re-exports the proc macros.
- `static-runtime` links the existing runtime implementation into one final binary, including browsers; it includes `runtime` and needs no shared library.
- `runtime` enables `std` and the linked shared-runtime facade.
- With default features disabled, core interfaces and `ComPtr` remain available
  without allocation.

## Scope And Status

core ABI v7 is an experimental pre-alpha draft. Cross-release compatibility begins
only when a named future release explicitly ratifies a baseline. Until then,
intentional breaking draft changes and golden-vector updates are permitted.

`neocogi-com` is not Windows COM, a C SDK generator, a source/reflection schema, a
sandbox, or a plugin manager. Installation, path discovery, downloading,
signature policy, allowlists, and trust decisions belong to the application.
