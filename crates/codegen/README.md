# NeoCogi COM Rust backend

Typed Rust emission shared by `neocogi-idl` and the implementation macros.
Canonical identities and contract validation belong to IDL; core remains
independent of the build tool and retains freshness-checked bootstrap bindings.

Resolved scalar plans emit transparent enum/flag wrappers, constants, checked
live conversion and eligible raw persistence directly. The same lexical binding
ownership used by interfaces and aggregates protects printed scalar Rust. No
declarative scalar frontend or macro-body source-map parser remains.

Compiler-dependent naming consumers and bootstrap freshness live in
`neocogi-idl::backend_tests`. The backend has no dependency back to its compiler,
including in tests. This keeps normal archive publication ordered core → backend
→ IDL → facade/macro consumers. Run the compiler's library tests for those real
IDL/ABI regressions and this package's unit tests for typed emission primitives.

Each method plan owns its authored documentation/origin, safe Rust types, physical
ABI types and explicit ownership categories. Traits, clients, raw vtable slots and
provider thunks retain the same method origin. Raw documentation prints the same
physical types used by dispatch; it never reconstructs wire pointers from native
element types or treats weak tokens as strong owners. Safe documentation continues
to describe native Rust values. The existing result ownership category supplies
the shared/exclusive/weak lifetime contract in both descriptions.

Generated raw vtable function-pointer fields narrowly allow Clippy's type-complexity
lint: their flattened signature is the physical IDL calling convention. The policy
does not cover safe methods, providers or ordinary algorithms. Introducing private
ABI aliases solely for this lint would add generated names without simplifying
the contract; other generated-code warnings remain checked by downstream gates.
