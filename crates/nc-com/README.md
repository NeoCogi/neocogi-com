# nc-com

`nc-com` is the user-facing crate for cross-platform Rust component interop
through an explicit C-shaped ABI. It generates COM-style interface headers,
vtables, status returns, GUID/hash identity, class metadata, and reference-counted
object storage without exposing Rust trait objects or Rust allocator ownership
across the binary boundary.

Most users should depend on this crate. The lower-level workspace crates contain
the core ABI types, proc-macro implementation, linked runtime declarations,
runtime shared library, and inspection CLI.

## Documentation

- [API documentation](https://docs.rs/nc-com) contains the compiled examples and
  feature-sensitive item documentation.
- The [workspace guide](https://github.com/neocogi/nc-com#readme) is the canonical
  long-form guide and owns the exact build/test compatibility matrix.
- The [core ABI v5 specification](https://github.com/neocogi/nc-com/blob/master/docs/abi-v5.md)
  is the normative binary contract.

This package README is intentionally a concise entry point. Exact test rows,
release procedures, and detailed compatibility tables live only in the workspace
guide so they cannot drift across copies.

## Quick Example

```rust
use nc_com::{ComPtr, IUnknown, NcStatus, com_interface, com_object};

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
hash-checked `QueryInterface`, and reference counting.

Macro documentation examples are compile-checked in the facade rustdoc, where
the complete public API is available; intentionally incomplete lowering and
generated-shape sketches are marked as text. The renamed-dependency integration
fixture also builds its generated rustdoc with warnings denied, covering both
automatic dependency-name resolution and an explicit facade override.

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
  external C-layout fields without nc-com traits. It checks nested FFI layouts;
  authors must ensure plain-value semantics and use a new interface identity
  for incompatible changes its field size/alignment fingerprints cannot detect.
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
  Interface compatibility is the derived IID plus generated ABI hash.
- A panic is not an ABI error. Return `NcStatus` for expected failures.

## Runtime And Plugins

The optional `runtime` feature exposes the stateless `Runtime` facade over the
separately built and linked `libnc_com_shared_runtime` shared library. The shared
runtime owns process-global registry state. Successfully loaded modules remain
mapped until process exit; unload and hot reload are not core ABI v5 operations.

Plugins export `nc_get_plugin_descriptor`. Generated descriptors contain plugin
identity, explicit hard dependency IDs, classes, constructor contracts,
threading modes, and interface metadata. core ABI v5 has no plugin initialization
callback and never discovers or recursively loads plugin paths: hosts obtain an
explicit dependency-first plan and load dependencies before dependents.
Live activation calls must not be nested. Native loader initialization and the
descriptor entrypoint must not directly or indirectly call nc-com loading;
operating-system loading of ordinary native dependencies remains permitted and
does not register them as nc-com plugins.

Live activation is fail-fatal. A path, native-loader, symbol, descriptor,
dependency, or registration failure emits a diagnostic and aborts rather than
returning partial runtime state. Inspection is non-registering:
`Inspection::with_plugin_library_descriptor` opens a trusted artifact, validates
its descriptor, and exposes that descriptor only during a synchronous callback
without changing either registry. `nc-com-inspect` copies this kind of metadata
in its short-lived process and can plan or compare artifacts without treating
diagnostic text as the compatibility authority.

`Runtime::load_plugin_library` returns an owned `LoadedModuleInfo`. Its module
ID, path, and plugin ID come from module inspection; its qualified name and
version come from the registered plugin descriptor through plugin inspection.
Use `runtime.inspection().loaded_module(module_id)` to retrieve the combined
summary later, or `runtime.inspection().plugin_info(plugin_id)` when only plugin
metadata is needed. The latter also works for statically registered plugins,
which do not have a loaded-module path.

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
[Build Compatibility Matrix](https://github.com/neocogi/nc-com#build-compatibility-matrix)
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

core ABI v5 is an experimental pre-alpha draft. Cross-release compatibility begins
only when a named future release explicitly ratifies a baseline. Until then,
intentional breaking draft changes and golden-vector updates are permitted.

`nc-com` is not Windows COM, a C SDK generator, a source/reflection schema, a
sandbox, or a plugin manager. Installation, path discovery, downloading,
signature policy, allowlists, and trust decisions belong to the application.
