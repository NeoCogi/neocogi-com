# Native COM builds

IDL generation uses ordinary Cargo build dependencies; no application staging
tool or artifact dependencies are required. The checkout selects nightly for its
local deployment profiles; SDK qualification also checks stable and Rust 1.85.
The final application selects one host runtime, statically linked or loaded from
the optional host shim, and passes its access to providers.

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

## Shared host runtime (0.4.0 work in progress)

This unreleased route loads the host implementation itself from a native image.
Build the thin shim from the checkout:

```sh
cargo build -p neocogi-com-host-dylib --locked
```

The Linux image is `target/debug/libneocogi_com_host_dylib.so`; macOS uses
`libneocogi_com_host_dylib.dylib`, and Windows uses `neocogi_com_host_dylib.dll`.
The application depends on `neocogi-com` with `runtime` enabled and selects this
image instead of depending on the static `neocogi-com-host` implementation:

```rust,no_run
use neocogi_com::RuntimeHandle;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("pass the packaged host runtime path")?;
    // SAFETY: deployment supplies the trusted SDK host image. The SDK pins it
    // for process lifetime before adopting its coherent immutable C tables.
    let runtime = unsafe { RuntimeHandle::load_host_library(path) }?;
    // Register/load the application's providers through this same runtime.
    assert_eq!(runtime.core_abi_version(), 21);
    Ok(())
}
```

The default `#[com_main]` remains static startup. A dynamic application can use
ordinary `main`, as above, or configure an explicit initializer path. Nested
provider acquisition forwards the selected host tables again. Choose one static
or dynamic owner; loading separate runtime images can create separate registries.
Opened runtime images remain pinned, including after bootstrap rejection. Host
bootstrap errors return `HostLoadError` without a fallback; ordinary provider
loading retains its mandatory/fatal policy.

The [shared-host regression](../../tests/com/integration-tests/fixtures/shared-host/README.md)
contains a complete independent dynamic application, an optional static build,
two separately generated providers and native bootstrap error cases. Run it with:

```sh
cargo test -p neocogi-com-tests --features runtime --test shared_host --locked
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
[ABI qualification guide](../abi-v3.md#independent-build-compatibility).
