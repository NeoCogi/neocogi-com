# IDL compiler location

The compiler and Cargo integration live in [`crates/com/idl`](../../crates/com/idl).
There is no standalone IDL binary or contract-package scaffolding tool.
Authoritative `.ncidl` sources stay under their owning subsystem; ordinary
libraries, applications and tests generate their own bindings. See the
[shared-source build contract](../../docs/contracts/IDL_MODULES.md#shared-idl-sources).
