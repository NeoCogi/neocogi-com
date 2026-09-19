# Changelog

All notable changes to the experimental component ABI are documented here.
Semantic-version compatibility begins only after an explicitly ratified ABI.

## Unreleased

- Replace producer event queues and state-owning sessions with `Events<Message>`:
  typed mappers post directly into one consumer FIFO and ordinary mutable state
  consumes application messages through a single runtime-owned update operation.
  Remove handler traits, registration transactions and dispatch from connection
  leases. Cancellation stops future posts and preserves committed consumer facts.
- Pump private component inboxes automatically in descendant-first passes, one
  message per component per visit, before consuming one application message.
  Share one update budget, preserve unattempted work on failure, revoke scoped
  borrowed dispatchers on return, and stop delivery immediately after mount removal.
  Composite models use ordinary mutable methods behind their single shared COM
  access boundary. Rust event binding API is 4; both deployment ABIs remain v1.
- Declare notification protocols with `port` and named `name: Protocol;` bindings.
  Generate typed accessors on owned and borrowed handles and concrete owned message
  types. File-dialog completion belongs directly to `IFileDialog`.

- Simplify contracts to compile-time Rust bindings and transitive signatures;
  remove native contract libraries/archives, module getters/registries, and
  intrinsic-event seeding. Plugins remain dynamically loadable or statically linked.
- Unify callable declarations as interfaces with factory/port roles. Keep typed
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
