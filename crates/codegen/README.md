# NeoCogi COM Rust backend

Typed Rust emission shared by `neocogi-idl` and the implementation macros.
Canonical identities and contract validation belong to IDL; core remains
independent of the build tool and retains freshness-checked bootstrap bindings.

Compiler-dependent naming consumers and bootstrap freshness live in
`neocogi-idl::backend_tests`. The backend has no dependency back to its compiler,
including in tests. This keeps normal archive publication ordered core → backend
→ IDL → facade/macro consumers. Run the compiler's library tests for those real
IDL/ABI regressions and this package's unit tests for typed emission primitives.
