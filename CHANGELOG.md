# Changelog

All notable changes to the experimental component ABI are documented here.
Semantic-version compatibility begins only after an explicitly ratified ABI.

## Unreleased

- Generate typed `EventPort` values for `EventSession::subscribe` and require
  mutable event sinks. Remove UI-family session duplication, connector helpers,
  handler interior mutability, and detached-batch restoration. Rust binding
  protocol 3 requires regenerated bindings; plugin and runtime ABIs remain v1.
- Model session registration as Ready/Busy/Closed and retain owned source FIFOs,
  explicit cancellation, terminal facts, and completed-request delivery.

- Simplify contracts to compile-time Rust bindings and transitive signatures;
  remove native contract libraries/archives, module getters/registries, and
  intrinsic-event seeding. Plugins remain dynamically loadable or statically linked.
- Unify callable declarations as interfaces with factory/event roles. Keep typed
  factory construction, required sibling projections, exact signatures, and
  mutable call gating; remove duplicate object descriptions and ordinary unit
  objects' generated zero-argument constructors.
- Keep plugin descriptor and runtime table ABIs at v1 during pre-alpha, with
  matching exported symbols and Rust types; use inspection schema 9.
  Reject incompatible factory signatures before bootstrap. Add an ordinary-Cargo
  sample exercising both dynamic deployment and a self-contained monolith.


- Added Rust-like `.ncidl` modules with `struct`, `enum`, `trait`, `class`, and
  `factory` declarations. Every module is an ordinary Cargo crate whose
  `build.rs` regenerates Rust bindings and the native contract artifact.
- Added fingerprint-qualified native contract libraries. Generated host and
  plugin bindings import the exact description getter instead of embedding
  duplicate canonical description graphs.
- Added canonical runtime contract registration. A repeated logical module ID
  with another fingerprint returns `NC_E_CONTRACT_MISMATCH` before publication.
- Fixed optimized contract-graph traversal so completing a module always removes
  it from the active path. Release inspectors now accept valid diamond graphs
  instead of reporting the shared dependency as a cycle.
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
- Made the inherited generated client view the sole safe interface calling
  layer. `ComPtr` and `ComRef` retain identical method syntax without generating
  parallel interface-specific extension traits.
- Defined runtime resource ceilings of 1,024 retained dynamic modules and 64
  simultaneous debug messengers.
