# One plugin, two builds

This example uses the generated contracts in
[tools/idl/example-contracts](../../tools/idl/example-contracts). The contract
crate's `build.rs` generates Rust bindings and source-graph metadata only.

```rust
#[version(1)]
class ExampleObject: IMeasured {
    fn resize(&mut self, extent: Extent) -> Result<()>;
}

#[version(1)]
factory ExampleFactory {
    fn create(&self, extent: Extent, events: &EventBinding<ExtentChanged>) -> Result<ComPtr<ExampleObject>>;
}
```

[src/lib.rs](src/lib.rs) implements private object state with plain fields and
`&mut self`, implements the unit factory, and declares the plugin:

```rust,ignore
neocogi_com::factory_plugin! {
    name = "nc.examples.factory_plugin_component",
    version = "0.0.0",
    factory = DynamicExampleFactory,
}

#[cfg(feature = "dynamic-plugin")]
neocogi_com::export_factory_plugin!();
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

Both hosts print `factory object: 44 x 31`. The
[build-modes integration test](tests/build_modes.rs) builds both with plain
Cargo, relocates them, and runs the construction/mutation sequence without any
contract artifacts or inherited library-search paths.

`--release` can be added to each build command; use `target/release` for the
link and loader directories. `cargo-neocogi` is optional deployment tooling and
is not used by this example.

## Persistence and a typed class stream

The object exposes `IPersistable`; `ExampleFactory::create` accepts
`EventBinding<ExtentChanged>`. The host creates a typed COM sink before the
factory call, saves dimensions, and restores through an explicitly registered
adapter. That adapter retains live application queue context separately from the
saved recipe and supplies a fresh receiver during each restoration. Neither
ordinary nor restored products expose a later event-installation operation.

The same relocation test executes this sequence across a real shared-library
boundary and in the monolith. The implementation library also checks for Wasm.
The host's output remains `factory object: 44 x 31`; assertions verify the extra
snapshot and event behavior before printing it.

An allocation probe measures one typed receiver allocation and scalar event delivery,
including standard-channel storage:

```sh
cargo run -p neocogi-example-com-plugin --example event_allocations --features monolith
```

It reports allocation counts for setup and repeated posts; channel storage can
allocate. It is not a latency benchmark or a zero-allocation promise.

The probe also feeds the decoder a truncated collection advertising 65,536
large records and checks that its initial allocation stays within 4 KiB.
