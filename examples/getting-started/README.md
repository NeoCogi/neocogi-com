# Your first application and plugin

This example builds an application and a calculator plugin separately. The
application loads the plugin, calls `add(2, 3)`, and prints `2 + 3 = 5`. It also
checks that addition overflow returns the documented error.

For a standalone crates.io build, use the [released 0.3.0 walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.3.0/examples/getting-started/README.md).
This directory exercises the current development SDK with the
[WIP ABI](../../docs/abi-v3.md#abi-identities-and-versions). Each project compiles
the same IDL into its own Rust bindings. The application selects the static host;
the plugin receives that host through its bootstrap.

From the checkout root, install the [archive gate prerequisites](../../tools/xtask/README.md)
including Rust 1.85.0 and a native linker. Fetch the locked dependencies for a
fresh checkout and run the gate:

```sh
cargo fetch --locked
cargo xtask packages
```

The gate copies these inputs into a temporary directory and resolves both
projects against the extracted SDK archives. It verifies the selected package
origins, builds both projects with the resolved locks, runs the calculator, and
checks that addition overflow returns the documented status. The calculator
prints `2 + 3 = 5`. The extracted inspector also runs `dump` and `check` against
the built plugin. The SDK dependency versions in the manifests select the
matching archives during this check.

| File | Purpose |
| --- | --- |
| [LICENSE-MIT](LICENSE-MIT) | License for copying this example |
| [build.rs](build.rs) | Generate the shared interface in each consumer |
| [calculator.ncidl](schemas/tutorial/calculator.ncidl) | The common interface |
| [host/Cargo.toml](host/Cargo.toml) | SDK, host implementation and build dependencies |
| [host/src/main.rs](host/src/main.rs) | Load the plugin and call the interface |
| [plugin/Cargo.toml](plugin/Cargo.toml) | SDK and build dependencies; native library output |
| [plugin/src/lib.rs](plugin/src/lib.rs) | Implement the interface and export discovery |
