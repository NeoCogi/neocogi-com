# Native COM builds

IDL generation uses ordinary Cargo build dependencies; no application staging
tool or artifact dependencies are required. The checkout selects nightly for its
local deployment profiles; SDK qualification also checks stable and Rust 1.85.
The final application links the host rlib and passes its runtime access to
providers.

## Linked host with a shared provider

From the repository root on Linux:

```sh
cargo rustc -p neocogi-example-com-plugin --lib --crate-type cdylib --features dynamic-plugin --locked
cargo build -p neocogi-example-com-host --bin neocogi-example-com --locked
target/debug/neocogi-example-com "$PWD/target/debug/libnc_example_com.so"
cargo run -p neocogi-inspect --locked -- dump target/debug/libnc_example_com.so
```

Use `.dylib` on macOS and `.dll` on Windows. Deploy the host and provider library;
the COM runtime is linked into the host. Add `--release` to both build commands
and use `target/release` for optimized artifacts.

## Static provider

```sh
cargo run -p neocogi-example-com-host --features monolith --locked
```

The executable contains the host runtime and provider. Both modes check the
same contract and print `factory object: 44 x 31` after executing calls,
notifications, persistence, and ownership checks. See the
[complete example](../../examples/com-plugin/README.md).

## Independently built providers

The [linked-host regression](../../tests/com/integration-tests/fixtures/linked-host/README.md)
builds the application and two native providers. It exercises nested acquisition
through the linked host and checks that repeat initialization and a second thread
see the same catalog.

```sh
cargo test -p neocogi-com-tests --features runtime --test linked_host --locked
```

## Independent builds

```sh
cargo xtask compatibility --suite linked
cargo xtask compatibility --suite build-flags
```

The compatibility driver owns temporary source/build directories, compiles roles
independently, and executes relocated artifacts after removing their targets.
The `build-flags` suite requires both stable and nightly compilers to exercise
different layout, optimization, and panic policies. Optional safety and sanitizer
suites require the nightly components documented in the
[ABI qualification guide](../abi.md#independent-build-compatibility).
