# NeoCogi COM

NeoCogi COM helps you build Rust applications with plugins compiled separately
from the application. It provides interface definitions, object ownership,
plugin loading, and inspection tools through an explicit C-compatible component
protocol.

This is experimental software. Build the host, plugins and generated interfaces
with the same SDK release. Loaded plugins stay mapped until the process exits.

Start with the complete [application and calculator plugin walkthrough](examples/getting-started/README.md).
It includes every source file, build script and dependency, plus commands for
Linux, macOS and Windows. It requires Rust 1.85 or newer and a native linker.

| Package | Purpose |
| --- | --- |
| `neocogi-com-core` | Raw ABI types and traits; `no_std` by default |
| `neocogi-idl` | Contract compiler, shared code generation, Cargo integration |
| `neocogi-com-macros` | Object, plugin, persistence, and host entry macros |
| `neocogi-com` | Shared SDK for interface callers and providers, ownership, values, and host-service clients |
| `neocogi-com-host` | Host-owned runtime and native plugin loader (`neocogi_com_host` rlib) |
| `neocogi-inspect` | Plugin metadata inspection, checks, plans, and comparisons |

A **host** is the final application that owns one runtime implementation. An
**interface caller** calls a COM interface; a **provider** implements one. These
roles can overlap: hosts can implement callbacks, and plugins can call other
providers or host services. Both use the shared `neocogi-com` SDK. The host also
links `neocogi-com-host`; the SDK's `runtime` feature supplies service clients
without selecting that implementation.

The [host, caller and provider guide](docs/COM.md#hosts-callers-and-providers)
defines their responsibilities, dependency graph and call flow. The SDK lives in
`crates/com`; the host implementation lives in `crates/host`.

The [advanced example](examples/com-plugin/README.md) adds static deployment,
callbacks and persistence to the same host/provider model. The host alone owns
the implementation; the SDK's `runtime` feature supplies clients for its services.
Default SDK features provide `std` and macros. Reduced-feature users can select
`alloc` or use the core without standard-library support.

## Working on the repository

Repository checks use the unpublished Rust [`xtask`](tools/xtask/README.md).
Native integration tests also require a C compiler and platform linker.
The checkout toolchain includes `rust-src` for consistent compiler diagnostics.
Checkout-only integration fixtures remain unpublished.

```sh
cargo test --workspace --all-features --locked
cargo run -p neocogi-example-com-host --features monolith --locked
cargo xtask quality
cargo xtask msrv
cargo xtask packages
```

The package check builds the six actual crate archives, extracts them into an
isolated temporary workspace, runs their shipped tests on Rust 1.85, and executes
a downstream consumer using only those extracted packages. Install that compiler with
`rustup toolchain install 1.85.0 --profile minimal`. The package check uses cached
registry inputs; run `cargo fetch --locked` first on a fresh machine.

Read the [component guide](docs/COM.md),
[ABI specification](docs/abi-v1.md),
[IDL reference](docs/contracts/IDL_MODULES.md), and
[dynamic/static example](examples/com-plugin/README.md).

Licensed under [MIT](LICENSE-MIT).
