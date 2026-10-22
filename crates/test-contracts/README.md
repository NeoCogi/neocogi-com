# Local IDL contract fixtures

This unpublished, development-only crate compiles the actual `.ncidl` sources in
`idl/` with the production resolver and Rust backend. Each exported, argument-free
fixture macro inserts those complete bindings into a test's local module. Tests
can then implement native defaults, inherent methods and concrete objects without
introducing a second contract language or relying on another package's `OUT_DIR`.

Each fixture resolves only its declared source dependencies. Deliberately
incompatible revisions with the same nominal IID belong to independent graphs;
computed signatures must differ. UI transport fixtures live in the UI test suites; this crate has no UI contract dependency.

Rust compile-fail tests retain object, call-borrow, thread-affinity and ownership
checks. Declaration-language admission belongs to the IDL parser/resolver suites.
