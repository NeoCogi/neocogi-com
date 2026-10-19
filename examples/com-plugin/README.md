# One plugin, two builds

This example uses the generated contracts in
[tools/idl/example-contracts](../../tools/idl/example-contracts). The contract
crate's `build.rs` generates Rust bindings and source-graph metadata only.

```rust
#[version(1)]
interface ExampleObject: IMeasured {
    fn resize(&mut self, extent: Extent) -> Result<()>;
}

#[version(1)]
interface ExampleFactory {
    fn create(&self, extent: Extent, events: &EventBinding) -> Result<ComPtr<ExampleObject>>;
}
```

[src/lib.rs](src/lib.rs) implements private object state with plain fields and
`&mut self`, implements the unit factory, and declares the plugin:

```rust,ignore
#[neocogi_com::plugin(name = "nc.examples.factory_plugin_component")]
fn acquire_root() -> Result<neocogi_com::ComPtr<api::ExampleFactoryAbi>, neocogi_com::NcStatus> {
    DynamicExampleFactory::from_value(DynamicExampleFactory).query()
}
```

The Cargo library target defaults to `crate-type = ["rlib"]`. Only the final
shared-library build enables `dynamic-plugin`. Do not enable `static-runtime`
in separately loaded plugin images.

## Shared plugin and host

From the repository root, on Linux:

```sh
cargo rustc -p neocogi-example-com-plugin --lib --crate-type cdylib --features dynamic-plugin
cargo rustc -p neocogi-com-runtime --lib --crate-type cdylib
NC_COM_RUNTIME_LINK_DIR="$PWD/target/debug" cargo build -p neocogi-example-com-plugin --bin neocogi-example-com --features host
LD_LIBRARY_PATH="$PWD/target/debug" target/debug/neocogi-example-com "$PWD/target/debug/libnc_example_com.so"
```

The deployment contains the executable, `libnc_com.so`, and the
plugin `.so`. No contract libraries are built or copied. On macOS use the
`.dylib` filenames and `DYLD_LIBRARY_PATH`; on Windows use the generated DLLs
beside the executable and point the link directory at their import libraries.
Normal platform/system dependencies still apply.

## Monolith

```sh
cargo build -p neocogi-example-com-plugin --lib
cargo run -p neocogi-example-com-plugin --bin neocogi-example-com --features monolith
```

Cargo statically links the plugin's Rust `rlib` and the runtime into the host.
The executable is the only NeoCogi deployment artifact. The host registers the
same immutable descriptor directly and uses the same checked factory API. No
native export is emitted by the plugin in this mode. Rust monoliths consume an
`rlib`; they do not need an additional C `staticlib` archive or wrapper project.

Both hosts print `factory object: 44 x 31` only after all behavioral assertions.
The [compatibility driver](../../crates/com/scripts/compatibility.py) independently
builds the shared runtime, host and plugin with explicitly selected compilers and
flags, removes their build outputs, and executes the relocated three-file deployment.
The [build-modes integration test](tests/build_modes.rs) separately checks the
self-contained monolith. See the [matrix contract](../../crates/com/docs/compatibility.md).

`--release` can be added to each build command; use `target/release` for the
link and loader directories. `cargo-neocogi` is optional deployment tooling and
is not used by this example.

## Persistence and a typed class stream

The object exposes `IPersistable`; `ExampleFactory::create` accepts
`EventBinding`. The host uses `events::channel()` for ordinary and restored objects. `post`
moves the exclusive event owner and captures a weak service origin. The handler
queries `IExtentChangedEvent` and reads `extent()` directly; no Rust callback or
channel representation crosses COM.

The persistence adapter retains this same queue separately from saved dimensions.
Restoring supplies the queue during construction, just like the ordinary factory.
Neither path permits later receiver installation. The host verifies each queued
origin against the corresponding object; weak origins do not keep objects alive.

The compatibility driver executes this sequence across a real shared-library
boundary; the build-modes test covers the monolith. The implementation library also checks for Wasm.
The host's output remains `factory object: 44 x 31`; assertions verify the extra
snapshot and event behavior, successful weak promotion while live, and expired
weak references after final release before printing it.

An allocation probe uses `events::channel()` and checks the typed weak origin and
`IExtentChangedEvent` getter on each of 10,000 delivered snapshots. It also verifies
that 10,000 unobserved updates construct and discard 10,000 snapshots. It measures common receiver
and channel construction separately from exclusive scalar event delivery and
standard-channel storage:

```sh
cargo run -p neocogi-example-com-plugin --example event_allocations --features monolith
```

It reports allocation counts for setup and repeated posts; channel storage can
allocate. It is not a latency benchmark or a zero-allocation promise.

The probe also feeds the decoder a truncated collection advertising 65,536
large records and checks that its initial allocation stays within 4 KiB.
