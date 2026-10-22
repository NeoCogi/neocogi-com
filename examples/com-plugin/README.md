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
    fn create(&self, extent: Extent, events: &ExtentBinding) -> Result<ComPtr<ExampleObject>>;
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
shared-library build enables `dynamic-plugin`. Its build script owns shared
linkage. `monolith` directly selects the implementation dependency retained by the
final executable; selecting both final modes is rejected before compilation.

## Shared plugin and host

From the repository root, on Linux:

```sh
cargo rustc -p neocogi-com-runtime --lib --crate-type cdylib
NC_COM_RUNTIME_LINK_DIR="$PWD/target/debug" cargo rustc -p neocogi-example-com-plugin --lib --crate-type cdylib --features dynamic-plugin
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

## Persistence and typed notifications

The object exposes `IPersistable`; `ExampleFactory::create` accepts the domain
`ExtentBinding`. The host implements `IExtentObserver::changed` and captures an
owned `ExtentNotification` in an ordinary local channel. Each fact contains the
new extent and a typed weak `ExampleObject` identity. No UI contract, Rust callback,
or channel representation crosses COM.

The persistence adapter retains the same observer separately from saved dimensions.
Restoration supplies it during construction, just like the ordinary factory.
There is no later observer installation. The host verifies the weak source against
the corresponding object, exercises successful live promotion, and checks expiry
after final release. Notifications cannot keep their producer alive.

The compatibility driver executes this sequence across a real shared-library
boundary; the build-modes test covers the monolith. The implementation library
also checks for Wasm. Assertions verify ordinary calls, output buffers, notifications,
persistence and final release before printing `factory object: 44 x 31`.

The allocation probe checks 10,000 typed notifications and verifies that unobserved
domain updates allocate nothing. It separately measures observer/channel setup
and delivered notifications, including standard-channel storage:

```sh
cargo run -p neocogi-example-com-plugin --example event_allocations --features monolith
```
