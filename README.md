# NeoCogi COM

A standalone Rust workspace for NeoCogi's experimental component ABI, IDL
compiler, object helpers, host runtime, and plugin inspection tools. Separately
built components communicate through explicit C ABI contracts.

Requires stable Rust 1.85 or newer. Repository checks use the unpublished Rust
[`xtask`](tools/xtask/README.md); native integration tests also require a C
compiler and the platform linker.

The checkout's Rust toolchain includes `rust-src` so compile-fail tests produce
consistent standard-library diagnostics locally and in CI.

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

The two unpublished example packages demonstrate a COM host and provider with
both static and shared-library deployment. ABI fixtures remain isolated,
unpublished workspaces so they can exercise independent compiler builds.

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
