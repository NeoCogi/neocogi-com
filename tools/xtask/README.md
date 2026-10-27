# Repository tasks

`xtask` is an unpublished Rust workspace member. From the checkout, use
`cargo xtask help` to list commands. No Python, Bash, PowerShell, pip packages or
external archive utility is required by the task runner.

| Command | Checks |
| --- | --- |
| `cargo test -p xtask --locked` | Driver regressions, real harness admission, archive extraction and report failures |
| `cargo xtask workspace` | Locked workspace/fixture resolution and a compiler-free no-default consumer |
| `cargo xtask quality` | Formatting, strict Clippy, allocation modes, ABI snippets, Markdown links and strict rustdoc |
| `cargo xtask markdown` | Local Markdown files and heading anchors; external links are not fetched |
| `cargo xtask msrv` | Rust 1.85 checks, library tests, exact semantic integration cases and feature boundaries |
| `cargo xtask packages` | Six actual crate archives, shipped tests/docs and downstream COM consumers on Rust 1.85 |
| `cargo xtask abi-32` | Core and native-C tests for `NC_COM_32_BIT_TARGET`, defaulting to `i686-unknown-linux-gnu` |
| `cargo xtask compatibility --suite linked` | Native C/ABI checks, relocated host-owned test executables and independent providers |
| `cargo xtask compatibility --suite cross-version` | Stable host/nightly providers and the reverse direction |
| `cargo xtask compatibility --suite build-flags` | Both cross-version rows plus release panic/LTO variations |
| `cargo xtask compatibility --suite safety` | Miri ownership and raw-storage cases |
| `cargo xtask compatibility --suite address` | AddressSanitizer host/provider execution |
| `cargo xtask compatibility --suite thread` | ThreadSanitizer host/provider execution |
| `cargo xtask compatibility --suite sanitizers` | Both sanitizer workloads sequentially |

Rustup must have the selected toolchains/components installed. Quality uses
stable rustfmt/Clippy; MSRV and archives use `1.85.0`; compiler variations use
stable/nightly; Miri needs nightly Miri and sanitizer builds need nightly
`rust-src`. Sanitizers currently execute only on `x86_64-unknown-linux-gnu`.
Native checks still require the platform C compiler and linker; 32-bit checks
also require the selected target and matching native multilib support.
`cargo fetch --locked` primes registry inputs for offline archive verification.

Tasks invoke argument vectors directly and never evaluate shell commands. The
runner removes ambient compiler/profile/loader overrides, retains tool discovery
and registry credentials, and forces `CARGO_BUILD_JOBS=1` and
`RUST_TEST_THREADS=1` for every child. The checkout also bootstraps Cargo with
one build job and configures Cargo-launched tests to use one thread, so plain
workspace test commands retain the same bound. No task runs concurrent compilers.
CI has four jobs total.

Each isolated task owns sources and targets below `target/xtask-work`, on the
checkout's filesystem rather than a potentially memory-backed system temporary
directory. Sources remain alive while tests use embedded manifest paths. Copied
fixtures exclude build outputs, Git data and private plans/reviews. Independently
built native roles are copied and cleaned before relocated execution. Task-owned
sources/targets are explicitly retired before success; failure unwinding also
attempts cleanup. Interrupted processes can leave scratch, removed by
`cargo clean`. A running task must finish before cleaning the workspace.

Archive verification extracts with Rust gzip/tar libraries into a workspace
containing only archive members and generated consumers, without shared package
metadata. SDK patches point exclusively at extracted siblings. The authoritative
strings/bytes example and its schema are copied from the checkout as consumer
inputs; no checkout SDK code supplies an archive dependency.

Compatibility accepts `--report PATH`. Relative paths belong to the caller's
working directory. Receipts record the selected compiler/panic/LTO policies,
nonempty exact and complete test execution, and SHA-256 of relocated host test
executables. A new invocation writes `running` before validating or copying its
checkout, including when the binary's original checkout no longer exists, then
`passed` only after every selected case and source teardown finish; failures
write `failed`, retaining completed cases. An interruption leaves `running`,
which is never success. Exit status is authoritative even if output contains a
success marker. Missing components, artifacts, cases and unsupported native
execution fail rather than silently skipping work.
