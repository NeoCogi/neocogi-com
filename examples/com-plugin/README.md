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
factory ExampleFactory {
    fn create(&self, extent: Extent) -> Result<ExampleObject>;
}
```

[src/lib.rs](src/lib.rs) implements private object state with plain fields and
`&mut self`, implements the unit factory, and declares the plugin:

```rust,ignore
neocogi_com::factory_plugin! {
    name = "nc.examples.factory_plugin_component",
    version = "1.0.0",
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
NC_COM_RUNTIME_LINK_DIR="$PWD/target/debug" cargo build -p neocogi-example-com-plugin --bin com-plugin-host --features host
LD_LIBRARY_PATH="$PWD/target/debug" target/debug/com-plugin-host "$PWD/target/debug/libneocogi_example_com_plugin.so"
```

The deployment contains the executable, `libnc_com_shared_runtime.so`, and the
plugin `.so`. No contract libraries are built or copied. On macOS use the
`.dylib` filenames and `DYLD_LIBRARY_PATH`; on Windows use the generated DLLs
beside the executable and point the link directory at their import libraries.
Normal platform/system dependencies still apply.

## Monolith

```sh
cargo build -p neocogi-example-com-plugin --lib
cargo run -p neocogi-example-com-plugin --bin com-plugin-host --features monolith
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
