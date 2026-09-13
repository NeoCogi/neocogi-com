# Changelog

All notable changes to the experimental component ABI are documented here.
Semantic-version compatibility begins only after an explicitly ratified ABI.

## Unreleased

- Added Rust-like `.ncidl` modules with `struct`, `enum`, `trait`, `class`, and
  `factory` declarations. Every module is an ordinary Cargo crate whose
  `build.rs` regenerates Rust bindings and the native contract artifact.
- Added fingerprint-qualified native contract libraries. Generated host and
  plugin bindings import the exact description getter instead of embedding
  duplicate canonical description graphs.
- Added canonical runtime contract registration. A repeated logical module ID
  with another fingerprint returns `NC_E_CONTRACT_MISMATCH` before publication.
- Added factory-only plugin descriptions and runtime ABI v2. Plugins expose one
  typed root factory; all later construction occurs through IDL factory methods.
- Removed runtime class tables, class-ID construction, constructor function
  pointers, erased constructor frames, and their legacy plugin export.
- Made offline inspection factory-native and advanced its artifact schema to 7.
  Assembly planning validates the same canonical contract graph as live loading.
- Added `cargo neocogi module new/check/generate/build/graph/remove` and
  transactional staging of deduplicated fingerprinted contract artifacts.
- Generate metadata-only description libraries as self-contained `no_std` Rust.
  Hosts and plugins retain only typed bindings and exact getter imports.
- Added `dynamic` and `monolith` assemblies. Dynamic distributions contain one
  shared runtime and factory plugins; monoliths register the same descriptions
  directly.
- Made SHA-256 ABI signatures unconditional and removed validation-mode build
  features and assembly configuration.
- Added typed `&mut self` call gating, generated thread-local/thread-safe pointer
  capabilities, bounded buffer lowering, ABI-safe records/enums, and
  format-neutral `ComDeserialize` without `Any` or erased value trees.
- Defined runtime resource ceilings of 1,024 retained dynamic modules and 64
  simultaneous debug messengers.
