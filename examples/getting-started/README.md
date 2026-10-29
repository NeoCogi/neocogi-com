# Your first application and plugin

This example builds an application and a calculator plugin separately. The
application loads the plugin, calls `add(2, 3)`, and prints `2 + 3 = 5`. It also
checks that addition overflow returns the documented error.

Use Rust 1.85 or newer and your platform's native linker. Copy this entire
directory to a fresh location outside the repository. It contains every input:

| File | Purpose |
| --- | --- |
| [LICENSE-MIT](LICENSE-MIT) | License for copying this example |
| [build.rs](build.rs) | Generate the shared interface in each consumer |
| [calculator.ncidl](schemas/tutorial/calculator.ncidl) | The common interface |
| [host/Cargo.toml](host/Cargo.toml) | SDK, host implementation and build dependencies |
| [host/src/main.rs](host/src/main.rs) | Load the plugin and call the interface |
| [plugin/Cargo.toml](plugin/Cargo.toml) | SDK and build dependencies; native library output |
| [plugin/src/lib.rs](plugin/src/lib.rs) | Implement the interface and export discovery |

Both projects use matching published SDK versions. Each compiles the same IDL
source into its own Rust bindings. The host does not link the provider and the
provider does not select a host implementation. Application version `0.0.0` in
these manifests is independent of the SDK dependency version.

From the copied directory, build the plugin:

```sh
cargo build --manifest-path plugin/Cargo.toml
```

Then run the application with its explicit library path:

Linux:

```sh
cargo run --manifest-path host/Cargo.toml -- "$PWD/plugin/target/debug/libgetting_started_plugin.so"
```

macOS:

```sh
cargo run --manifest-path host/Cargo.toml -- "$PWD/plugin/target/debug/libgetting_started_plugin.dylib"
```

Windows PowerShell:

```powershell
cargo run --manifest-path host/Cargo.toml -- "$PWD/plugin/target/debug/getting_started_plugin.dll"
```

The output is:

```text
2 + 3 = 5
```

These paths assume Cargo's default target directory. If you set
`CARGO_TARGET_DIR`, pass the library from that directory instead. Keep any private
native dependencies beside the plugin. Load only plugins you trust: opening a
native library can run its initializers. A required plugin that cannot load
produces a diagnostic and terminates the host; interface method failures such
as overflow are ordinary returned statuses.

Install the matching inspector after the SDK has been published:

```sh
cargo install neocogi-inspect --version '=0.0.0' --locked
```

Inspect the library with `neocogi-inspect dump PATH_TO_LIBRARY`, or run
`neocogi-inspect check PATH_TO_LIBRARY`. Inspection opens trusted native code but
does not construct the calculator factory. `dump` prints plugin identity, binary
compatibility fields and the exact factory signature as JSON. `check` exits
successfully for this valid plugin.

The repository's `cargo xtask packages` gate copies these exact files, builds
both projects using only extracted SDK archives on Rust 1.85, executes the real
call and error case, and runs the extracted inspector against the plugin. That
pre-publication check is separate from verifying registry-only installation
after upload.
