# NeoCogi IDL tooling

The IDL implementation is build tooling rather than runtime platform code. It
lives beside `cargo-neocogi` while remaining split into reusable Cargo packages:

- `core` (`neocogi-idl`) parses single files or multi-file module directories,
  emits source-aware diagnostics, validates and resolves types, and fingerprints
  complete typed module graphs.
- `build` (`neocogi-idl-build`) integrates generation and native contract
  artifacts with contract-crate `build.rs` scripts.
- `example-contracts` and `example-consumer-contracts` are compile-checked
  fixtures covering standalone and cross-module generation.

`cargo-neocogi` orchestrates module management and application assembly, but it
does not own contract compilation. Keeping the packages separate allows an
ordinary Cargo build to regenerate a contract whenever any of its `.ncidl`
source parts changes, without locating or launching the CLI executable.
Source-part names and discovery order are not ABI inputs; only resolved
declarations and their ABI-significant member order are fingerprinted.
