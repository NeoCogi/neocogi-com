# NeoCogi COM

Development sources use **ABI v3 — WORK IN PROGRESS (WIP)**, currently core
generation 20. Its layouts, callbacks and ownership contracts are provisional
and may change before release; ABI v3 is not frozen.
Published 0.3.0 packages retain ABI v2/core generation 19; use their release-tag
documentation for that contract.

[![Crates.io](https://img.shields.io/crates/v/neocogi-com.svg)](https://crates.io/crates/neocogi-com)

NeoCogi COM helps you build Rust applications with plugins compiled separately
from the application. It provides interface definitions, object ownership,
plugin loading, and inspection tools through an explicit C-compatible component
protocol.

This is experimental software. Build the host, plugins and generated interfaces
with the same SDK release. Loaded plugins stay mapped until the process exits.

The current release is **[0.3.0](https://crates.io/crates/neocogi-com/0.3.0)**.
All eight packages listed below are published on crates.io with ABI v2/core
generation 19. The development contract is [ABI v3](docs/abi-v3.md), with core
generation 20.
Persistence applications explicitly supply their selected serialization provider;
see [host persistence](crates/host/PERSISTENCE.md).

Start with the complete [application and calculator plugin walkthrough](examples/getting-started/README.md).
It includes every source file, build script and dependency, plus commands for
Linux, macOS and Windows. It requires Rust 1.85 or newer and a native linker.

| Package | Purpose |
| --- | --- |
| `neocogi-com-core` | Raw ABI types and traits; `no_std` by default |
| `neocogi-idl` | Contract compiler, shared code generation, Cargo integration |
| `neocogi-com-macros` | Object, plugin, persistence, and host entry macros |
| `neocogi-com` | Shared SDK for interface callers and providers, ownership, values, and host-service clients |
| `neocogi-com-serialization` | Reusable serialization rlib; unconditional NCB and optional lightweight JSON |
| `neocogi-com-serialization-dylib` | Thin native plugin with built-in NCB and JSON; NCB is default |
| `neocogi-com-host` | Host-owned runtime and native plugin loader (`neocogi_com_host` rlib) |
| `neocogi-inspect` | Plugin metadata inspection, checks, plans, and comparisons |

Development on `0.4.0-wip` additionally includes
[`neocogi-com-host-dylib`](crates/host-dylib/README.md), an optional native host
bootstrap shim. It is unreleased. Applications choose one static or dynamic host
owner and supply that owner's runtime access to every plugin.
The approved [Rust API renames](docs/COM.md#rust-names-in-040-development) are
recorded with migration names. The lifetime-policy split introduces distinct
shared and boxed headers and advances the development binary contract to ABI v3.
See the [provider migration guide](docs/guides/ABI_V3_MIGRATION.md) when adopting it.

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
The checkout follows `nightly` with `rust-src` so `cargo build --profile min-size`
rebuilds std and applies the complete deployment settings directly. Independent
SDK consumers retain Rust 1.85 support; qualification tests those portable sources
without inheriting the checkout's nightly Cargo configuration.
Checkout-only integration fixtures remain unpublished.
Ownership rejection cases check Cargo's structured errors at the invalid source
expressions, so updated Rust diagnostic layouts do not change their contract.

Tracked text uses LF line endings on every platform. Git applies the repository's
`.gitattributes` policy even when `core.autocrlf` is enabled, keeping checked-in
generated bindings and crate contents consistent across operating systems.
Use a fresh clone if an older checkout retains CRLF copies of generated files.

```sh
cargo test --workspace --all-features --locked
cargo run -p neocogi-example-com-host --features monolith --locked
cargo xtask quality
cargo xtask msrv
cargo xtask packages
```

The package check builds the nine current source-package archives, extracts them into an
isolated temporary workspace, runs their shipped tests on Rust 1.85, and executes
a downstream consumer using only those extracted packages. Install that compiler with
`rustup toolchain install 1.85.0 --profile minimal`. The package check uses cached
registry inputs; run `cargo fetch --locked` first on a fresh machine.

Read the [component guide](docs/COM.md),
[ABI specification](docs/abi-v3.md),
[IDL reference](docs/contracts/IDL_MODULES.md), and
[dynamic/static example](examples/com-plugin/README.md).

Licensed under [MIT](LICENSE-MIT).
