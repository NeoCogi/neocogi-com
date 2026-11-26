# NeoCogi COM

Development SDK `0.5.0-wip-alpha.1` uses **ABI v4 (WIP)**. See the
[current ABI versions](docs/abi.md#abi-identities-and-versions). Its layouts,
callbacks and ownership contracts are provisional and may change before release;
ABI v4 remains provisional.
Published 0.3.0 packages retain ABI v2/core generation 19; use their release-tag
documentation for that contract.

[![Crates.io](https://img.shields.io/crates/v/neocogi-com.svg)](https://crates.io/crates/neocogi-com)

NeoCogi COM helps you build Rust applications with plugins compiled separately
from the application. It provides interface definitions, object ownership,
plugin loading, and inspection tools through an explicit C-compatible component
protocol.

This is experimental software. Build the host, plugins and generated interfaces
with the same SDK release. Loaded plugins stay mapped until the process exits.

The **0.4.0** SDK includes eight packages and uses provisional ABI v3/core
generation 21. Follow the [0.4.0 walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/examples/getting-started/README.md) to build with the published packages.
Persistence applications explicitly supply their selected serialization provider;
see [host persistence](crates/host/PERSISTENCE.md).

The [application and calculator plugin archive example](examples/getting-started/README.md) includes every source file, build script and dependency, plus commands for
Linux, macOS and Windows. It requires Rust 1.85 or newer and a native linker.
The [0.3.0 walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.3.0/examples/getting-started/README.md)
uses the ABI v2 SDK.

| Package | Purpose |
| --- | --- |
| `neocogi-com` | Raw ABI types and traits; `no_std` by default |
| `neocogi-idl` | Contract compiler, shared code generation, Cargo integration |
| `neocogi-com-macros` | Object, plugin, persistence, and host entry macros |
| `neocogi-com-sdk` | Shared SDK for interface callers and providers, ownership, values, and runtime API wrappers |
| `neocogi-com-serialization` | Reusable serialization rlib; unconditional NCB and optional lightweight JSON |
| `neocogi-com-serialization-dylib` | Thin native plugin with built-in NCB and JSON; NCB is default |
| `neocogi-com-host` | Host-owned runtime and native plugin loader (`neocogi_com_host` rlib) |
| `neocogi-inspect` | Plugin metadata inspection, checks, plans, and comparisons |

The final application links the host rlib and supplies its runtime access to
every provider. Native providers, including the serialization plugin, can be
loaded from shared libraries.
The [Rust API renames](docs/COM.md#rust-names-in-040) are
recorded with migration names. The lifetime-policy split introduces distinct
shared and boxed headers in ABI v3. ABI v4 gives a box one owner and
owner-derived borrows. Shared persistence inputs retain counted ownership.
See the [provider migration guide](docs/guides/ABI_MIGRATION.md) when adopting it.

A **host** is the final application that owns one runtime implementation. An
**interface caller** calls a COM interface; a **provider** implements one. These
roles can overlap: hosts can implement callbacks, and plugins can call other
providers or host services. Both use the shared `neocogi-com-sdk` SDK. Static hosts
link `neocogi-com-host`.
The SDK's `runtime` feature supplies access to the selected host.

The [host, caller and provider guide](docs/COM.md#hosts-callers-and-providers)
defines their responsibilities, dependency graph and call flow. The SDK lives in
`crates/com`; the host implementation lives in `crates/host`.

The [advanced example](examples/com-plugin/README.md) adds static deployment,
callbacks and persistence to the same host/provider model. The host alone owns
the implementation; the SDK's `runtime` feature supplies API wrappers for its operations.
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

The package check builds the eight current source-package archives, extracts them into an
isolated temporary workspace, runs their shipped tests on Rust 1.85, and executes
a downstream consumer using only those extracted packages. Install that compiler with
`rustup toolchain install 1.85.0 --profile minimal`. The package check uses cached
registry inputs; run `cargo fetch --locked` first on a fresh machine.

Read the [component guide](docs/COM.md),
[ABI specification](docs/abi.md),
[IDL reference](docs/contracts/IDL_MODULES.md), and
[dynamic/static example](examples/com-plugin/README.md).

Licensed under [MIT](LICENSE-MIT).
