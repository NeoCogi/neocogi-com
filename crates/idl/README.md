# COM-owned NeoCogi IDL compiler

The IDL compiler defines the public COM contract language, its canonical ABI
fingerprints, and the generated Rust and native binding protocol. It is build
tooling rather than process-global runtime code, but it belongs to the COM
subsystem because generated COM bindings and the COM event transport must evolve
together. It remains split into reusable Cargo packages:

- `core` (`neocogi-idl`) parses single files or multi-file module directories,
  emits source-aware diagnostics, validates and resolves types, and fingerprints
  complete typed module graphs.
- `build` (`neocogi-idl-build`) owns canonical filesystem loading, the native
  artifact record schema, typed Rust binding generation, and the self-contained
  Rust description library emitted by contract-crate `build.rs` scripts.
- `tools/idl/example-contracts` and `tools/idl/example-consumer-contracts`
  are compile-checked fixtures covering standalone and cross-module generation.

The `event` keyword is an intrinsic COM language feature. Its port lowering
uses the canonical `neocogi.com.events.IEventSubscription` contract, and its
generated adapters target a versioned API in `neocogi-com-events`. The compiler
rejects a different declaration with the same short name, while generated
modules check their required support API at compile time. A workspace test
compares the compiler and support protocol constants; released consumers get
the generated compile-time check. ABI fingerprints
identify resolved IDL wire contracts; a compiler-only Rust binding change does
not silently change a native module fingerprint.

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
