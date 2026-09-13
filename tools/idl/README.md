# NeoCogi IDL tooling

The IDL implementation is build tooling rather than runtime platform code. It
lives beside `cargo-neocogi` while remaining split into reusable Cargo packages:

- `core` (`neocogi-idl`) parses single files or multi-file module directories,
  emits source-aware diagnostics, validates and resolves types, and fingerprints
  complete typed module graphs.
- `build` (`neocogi-idl-build`) owns canonical filesystem loading, the native
  artifact record schema, typed Rust binding generation, and the self-contained
  Rust description library emitted by contract-crate `build.rs` scripts.
- `example-contracts` and `example-consumer-contracts` are compile-checked
  fixtures covering standalone and cross-module generation.

`cargo-neocogi` orchestrates module management and application assembly, but it
does not own contract compilation. It uses `ModuleSources` from the build-support
crate when inspecting authored modules, so the CLI and contract build scripts
apply one file/directory discovery policy. It also consumes the exported
`ArtifactRecord` and `ARTIFACT_SCHEMA` definitions instead of maintaining a
second representation of build-script JSON.

Keeping orchestration separate from compilation allows an ordinary Cargo build
to regenerate a contract whenever any of its `.ncidl` source parts changes,
without locating or launching the CLI executable. Source-part names and
discovery order are not ABI inputs; only resolved declarations and their
ABI-significant member order are fingerprinted. The pure `neocogi-idl` language
model remains independent of Rust procedural-macro syntax trees.
