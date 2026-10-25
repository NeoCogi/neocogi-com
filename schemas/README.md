# Shared COM schemas

These are authoritative COM test protocols shared by their consumers. Each
consumer generates bindings in its own build; this directory is not a Cargo
package. The IDL compiler remains in `crates/idl`.

Language and persistence built-ins stay in the compiler/core SDK packages so
published archives contain their own inputs. UI, rendering and other application
schemas stay under their respective subsystem’s `schemas/` directory.
