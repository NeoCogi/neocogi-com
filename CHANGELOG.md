# Changelog

## Shared allocation lifetimes (experimental ABI 12)

Replace generated Arc storage and per-class weak operation tables with one common
COM control header and non-generic lifetime functions. Weak handles and nullable
weak handles now occupy one pointer; strong handles remain two. Only value
destruction, allocator release and interface projections remain class-specific.
Publish cyclic construction after initialization, preserve rollback for escaped
weak origins, and transfer promoted ownership without an extra query retain.
IDL fingerprints and core generation reject the former weak-token contract.
See [ABI v12](docs/abi-v12.md).

All notable changes to the experimental component ABI are documented here.
Semantic-version compatibility begins only after an explicitly ratified ABI.

## Weak lifetime handles (core ABI 11)

Add the optional C-shaped weak downgrade operation to the common object prefix,
`WeakComPtr<T>` with checked promotion, and generated cyclic initialization for
weak self origins. Generated objects keep one Arc-backed allocation; final strong
release destroys the value, while weak references can retain its empty shell.
The two-word strong handle and exact interface negotiation remain unchanged.
Old core generations are rejected before dispatch. See [ABI v11](docs/abi-v11.md).

## Shared binding-path validation

COM core owns the bounded, allocation-free Rust-2024 binding-path grammar and
exports it through the facade. IDL publication and COM widget catalog admission
share that validator; raw identifiers and optional leading `::` are supported,
while unescaped reserved words and unsupported syntax fail with field context.
`neocogi-inspect` also delegates version validation to the existing COM checker.
No new crate, runtime syntax-parser dependency, ABI or catalog schema is added.
Regression coverage compiles IDL published under `r#type`, copies its descriptor
through real catalog COM calls, compiles exported forms in a downstream crate,
and dispatches the typed event. See the [descriptor text contract](core/README.md#descriptor-text-validation).

## Object/table handles (core ABI 10)

COM references now carry the allocation address and selected interface table.
Generated objects have no per-interface headers, and dispatch no longer recovers
owners using offsets. Query preserves object identity; all views share one
reference count and borrow guard. Raw nullable pairs distinguish absence from
half-null errors. Plugin factories, persistence, native C fixtures and consumers
use the new representation; older core generations are rejected. The deliberate
16-byte handle and its memory tradeoff are documented on `ComPtr` and in the
[storage decision](../../docs/contracts/COM_INTERFACE_STORAGE.md). Static inherited method
prefixes and full explicit inspection remain supported.


## Single-pointer interface headers (experimental ABI 9)

- Store one vtable pointer per interface and recover its owner through the
  common vtable's compiler-derived offset. Inherited prefixes retain the actual
  outer projection's offset, and all interfaces keep one borrow guard/refcount.
- Update generated, built-in, persistence, foreign-fixture, and native C bindings
  together. Discovery rejects the previous two-pointer-header ABI generation.
- Retain one-word COM handles and full explicit inspection metadata. Object
  allocation savings and linked deployment changes are measured separately.

## Unreleased

- Make IDL collection ownership explicit with `Vec<T>`, `Slice<T>` / `[T]`, and
  `Array<T, N>` / `[T; N]`. Support recursively copied arrays in methods and
  buffers through checked wire codecs. Move `Option<T>` into an ordinary bundled
  generic enum, remove special optional lowering, and require borrowing the whole
  enum for owned payloads. Expose enum deserialization and conditional library
  decoder policies. COM designer catalog schema 5 uses named enum definitions
  for optional fields. See [the generic value contract](../../docs/contracts/IDL_GENERIC_VALUES.md) for syntax,
  ownership, migration rules, and the remaining explicit limits.

- Unify IDL records/enums as recursively checked values and remove `#[message]`.
  Separate application storage, raw wire fields, and typed producer guards;
  check bool bytes and selected enum payloads in copied slots, borrowed values,
  and buffers. Reject inline recursion, allow explicit recursive sequences, and
  share byte/work/depth budgets through nested conversion. Preserve zero-copy
  numeric buffers only through an explicit layout/content proof. Advance core
  ABI to 8 and canonical IDL encoding to 3; rebuild all components together.
- Transport complete structured value definitions, named recursion, and enum tags
  through COM designer catalog schema 4. Keep one stream/one lifetime consumer
  and explicit application handlers; values no longer manufacture receivers.
  See [the recursive value contract](../../docs/contracts/RECURSIVE_VALUE_CONTRACT.md) for the supported positions and validation evidence.

- Enforce the same byte, element, value, and depth policy when encoding and
  decoding binary persistence values. Count scalar and container values through
  a shared budget, exclude format framing, and give unit enum cases no synthetic
  payload. Byte vectors remain collections. Over-limit snapshots and authored
  documents now fail on save instead of producing data rejected on reopening.
  Add exact-boundary codec, COM snapshot/restore, and document regressions.

- Consolidate nine COM production crates into five: move runtime link declarations
  into the facade, combine IDL parsing/generation/Cargo integration in
  `neocogi-idl`, and expose built-in event contracts and channel adapters through
  `neocogi_com::events`. Remove the runtime-sys, events, events-contracts, and
  idl-build packages. Runtime/shared-library ownership and C ABI signatures remain
  unchanged; `nc_com` remains the sole deployed COM runtime library.
- Add explicit crate-rooted Rust contract paths to source graph schema 5 so a
  logical IDL module can live inside a facade without changing its identity or
  fingerprints. Build scripts use `neocogi_idl::cargo::Module`; event consumers
  depend directly on `neocogi-com` with `macros` (plus `std` for channel adapters).

- Consolidate persistence contracts and the portable `ObjectRecord` schema into
  core, provider helpers into the facade, and snapshot/restore execution into
  the runtime's fifth ABI v1 table. `FactoryRegistry::new()` is fallible;
  registration uses `&mut self` and contexts remain caller-owned and thread-local.
- Adopt NeoCogi Foundations naming. The runtime package remains
  `neocogi-com-runtime`, its Rust target is `nc_com`, and native shared builds
  emit `libnc_com.so`, `libnc_com.dylib`, or `nc_com.dll`.
- Align crate guides and rustdoc with the current ABI, feature/linkage choices,
  raw text lifetimes, owned string copying, factory bootstrap, persistence,
  and the division between event adapters and UI scheduling.

- Replace custom event queues and posting wrappers with `std::sync::mpsc`.
  Applications and composites own ordinary receivers; typed COM mappings hold
  senders and translate each concrete event into a local Rust message. Receiver
  destruction discards later delivery; mapping still runs before sending fails.
- Dispatch exactly one message per COM visit and discover nested component drivers
  automatically. The retained UI runtime owns descendant-first scheduling, frame
  boundaries, retirement checks and one finite update budget. Reaching the exact ceiling stops
  before another dequeue; unattempted work remains for the next update.
- Declare published outputs as concrete `#[message]` data with named
  `name: Message;` bindings. Producers own one optional typed receiver, installed
  by an ordinary mutable setter. Remove output projections, subscriptions, leases,
  generations and support-version negotiation. Both deployment ABIs remain v1.

- Simplify contracts to compile-time Rust bindings and transitive signatures;
  remove native contract libraries/archives, module getters/registries, and
  intrinsic-event seeding. Plugins remain dynamically loadable or statically linked.
- Unify callable declarations as interfaces and factories. Keep typed
  factory construction, required sibling projections, exact signatures, and
  mutable call gating; remove duplicate object descriptions and ordinary unit
  objects' generated zero-argument constructors.
- Keep plugin descriptor and runtime table ABIs at v1 during pre-alpha, with
  matching exported symbols and Rust types; use inspection schema 9.
  Reject incompatible factory signatures before bootstrap. Add an ordinary-Cargo
  sample exercising both dynamic deployment and a self-contained monolith.

## Earlier pre-alpha iterations

These entries record superseded designs, not the current API. Native contract
artifacts, `trait`/`class` IDL declarations, global contract registration, and
runtime ABI v2 were subsequently removed or replaced. Current binary contracts
are core v12, factory-plugin v1, and runtime v1; see the [ABI guide](docs/abi-v12.md).

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
