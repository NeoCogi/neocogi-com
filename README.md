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
| `neocogi-com` | Typed interface ownership, values, and optional host clients |
| `neocogi-com-runtime` | Host-owned runtime and native plugin loader (`nc_com` rlib) |
| `neocogi-inspect` | Plugin metadata inspection, checks, plans, and comparisons |

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
