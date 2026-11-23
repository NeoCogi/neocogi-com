# Your first application and plugin

This example builds an application and a calculator plugin separately. The
application loads the plugin, calls `add(2, 3)`, and prints `2 + 3 = 5`. It also
checks that addition overflow returns the documented error.

For ABI v2, use the [0.3.0 walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.3.0/examples/getting-started/README.md).
This directory uses development SDK `0.5.0-wip-alpha.1` with the
[WIP ABI](../../docs/abi-v4.md#abi-identities-and-versions). Each project compiles
the same IDL into its own Rust bindings. The application selects the static host;
the plugin receives that host through its bootstrap.

## Standalone build

For a crates.io build, copy the [0.4.0 walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/examples/getting-started/README.md)
from its release tag. To build this development candidate from the checkout,
use the SDK archive qualification below. Both use Rust 1.85 or newer and a
native linker.

From the directory copied from the release tag, build the plugin:

```sh
cargo build --manifest-path plugin/Cargo.toml
```

On Linux, run:

```sh
cargo run --manifest-path host/Cargo.toml -- "$PWD/plugin/target/debug/libgetting_started_plugin.so"
```

On macOS, run:

```sh
cargo run --manifest-path host/Cargo.toml -- "$PWD/plugin/target/debug/libgetting_started_plugin.dylib"
```

On Windows PowerShell, run:

```powershell
cargo run --manifest-path host/Cargo.toml -- "$PWD/plugin/target/debug/getting_started_plugin.dll"
```

The host prints `2 + 3 = 5`. The paths assume Cargo's default target directory.
With a custom target directory, pass the path of the shared library produced
there. The host loads the explicitly selected plugin and pins it for the process.

Install the matching inspector, then run `neocogi-inspect dump PATH_TO_LIBRARY`
and `neocogi-inspect check PATH_TO_LIBRARY` on the same library:

```sh
cargo install neocogi-inspect --version '=0.4.0' --locked
```

## SDK archive qualification

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
