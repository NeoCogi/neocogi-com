# Changelog

The current contract uses [core generation 18](docs/abi-v1.md). Entries below record
its development; later entries can supersede older implementation details.
Historical draft numbers do not define additional supported COM baselines.

## Unreleased

- Keep the non-UTF-8-directory graph-encoding fixture on Linux so macOS archive
  tests can run on filesystems that reject such directory names. Portable output
  safety checks remain enabled; compiler and runtime behavior are unchanged.

## 0.1.0-alpha.1 — release candidate

Prepared on 2026-10-28; publication is pending.

- Build a small host and calculator plugin from the complete
  [getting-started example](examples/getting-started/README.md), including contract
  generation, a real native call, checked errors and plugin inspection.
- Public crates package without omitted-test warnings. Checkout-only compiler
  and inspector suites have an unpublished owner, with their existing fixtures,
  exact compiler-floor cases and real CLI coverage preserved.
- Core compatibility generation 18 separates this baseline from retired
  experimental layouts. Incompatible metadata is rejected before factory
  bootstrap; runtime and descriptor discovery remain family 1. Hosts, providers
  and generated bindings should use the same SDK release. IDL encoding v13 and
  NCB format v1 retain their separate identities.
- Ordinary package verification and the final publish dry-run use the public
  registry index online. The independent archive gate still runs shipped tests,
  docs, consumers and the standalone tutorial on Rust 1.85.

Requires Rust 1.85 or newer. Native Linux, macOS and Windows CI includes archive
and linked execution; candidate-specific successful runs are required before
publication. Linux x86_64 is the local qualification environment. macOS/Windows
results, other CPU architectures and 32-bit execution are not inferred from it.
Plugins are trusted native code and remain loaded until process exit. Required
loading failures diagnose and terminate the host; ordinary method failures are
returned statuses. This custom component protocol provides no Windows system
COM activation or marshalling. See the [ABI contract](docs/abi-v1.md) and
[maintainer release procedure](docs/guides/RELEASING.md) for the complete rules.

Older entries below describe development history and may be superseded by this
baseline. They are not additional supported compatibility generations.

## Shared SDK and explicit host package

Use `neocogi-com` in `crates/com` for both interface callers and providers.
The implementation is `neocogi-com-host` in `crates/host`, with Rust library
`neocogi_com_host`. Host persistence code uses the shared SDK; raw host tables
continue to use core. Update default/aliased `com_main` linkage, manifests,
fixtures, archive admission, CI and documentation together. The `runtime` SDK
feature remains a supplied-service client feature. Core/service ABI names and
versions are unchanged. Older entries below record historical package/artifact
names and deployment models; they are not current linkage instructions.

Remove the SDK's obsolete host dev-dependency and unused test-only crate retention.
Host-dependent integration tests already live in `tests/com`. Workspace checks
enforce the host-to-SDK dependency direction and host-free SDK feature graphs.

## Host-owned services within ABI v1

Initialize one selected executable-owned implementation with `#[com_main]` and
pass copyable `Runtime` access into plugin acquisition and reusable libraries.
Keep the process-wide registry, pinned provider code and caller-owned context
lifetimes. Remove global/default discovery, named getters, the COM native-link
bridge and the runtime cdylib; the runtime crate remains the host's ordinary rlib.
Static and dynamic providers use the same supplied C service graph. Split the
portable host/provider sample and update persistence clients, real applications,
C layout/bootstrap coverage and native/SDK qualification. Core/runtime/plugin
stay unreleased ABI-v1; IDL v13, NCB v1 and fixed identities are unchanged.

## One pre-release ABI-v1 specification

Consolidate the core object, runtime discovery and compatibility specifications
into one guide. Core, runtime and plugin discovery identify the v1 baseline;
the IDL canonical encoding, catalog schemas and exact interface signatures retain
their separate identities. Rebuild contracts, providers and hosts together.
The native global registry remains owned by the shared COM runtime.

## UI-owned event contracts (pre-release)

Move event transport and snapshot publication into UI. COM and IDL retain only
ordinary interfaces, typed ownership and generic method/value introspection.
`ui_event_sources!` replaces the IDL event attribute; catalog publication validates
snapshot getters. Encoding v13 removes application event edges, so rebuild all
contracts and consumers. The native global registry remains in shared `nc_com`.
The standalone SDK example uses its own typed extent observer without UI dependencies.

## Foundational contract ownership (pre-release)

Canonical module encoding is now v12. Recursive type alternatives have explicit,
disjoint discriminants; a template parameter cannot share an output-buffer tag.
Regenerate all contract bindings and rebuild providers, plugins and hosts together.
Nominal IDs, core ABI v1 and runtime/plugin descriptor discovery ABI v1 retain
their existing definitions. Checked-in persistence bindings use the new signatures.

## Generated binding and public-name ownership (pre-release)

Centralize generated-name allocation and public interface/dense-enum inventories.
Protect provider/client binders, argument/helper families and const-expression
references. Anonymous implementation scopes distinguish generated patterns from
ambient constants through function-only imports that have no runtime state.
Resolve field syntax at its application owner before using canonical typed
projections in wire helpers and decoders; preserve Self and opaque type macros.
Scope plugin metadata/acquisition helpers behind their public accessor, evaluate
object tables directly in ComObject::VTABLES, and inline private method metadata.
IDL qualifies local type paths and diagnoses dense public-member conflicts early.
Normal/printed consumers synthesize collision names from actual generated bindings
and execute shared/exclusive, buffer, ownership, plugin and persistence boundaries.

## Generated value and support ownership (pre-release)

Checked value codecs explicitly select their ComValue proofs, operations and
associated types. Application methods/constants cannot alter codec budgets or
converted values. Wire/payload/storage helpers live in anonymous implementation
scopes; explicit raw consumers use ComValue::Wire instead of sibling helper names.
Static signature literals replace named sibling byte arrays. Metadata names are
emitted as literals and standard support traits/macros/derives use core paths.
Flags embed their mask expression, allowing an authored KNOWN_BITS flag.
Wire layouts, canonical identities and the shared runtime registry are unchanged.

## Bound base and event fingerprints (pre-release)

Use one lexical bound/free classifier for type expressions, direct bases and
emitted events. Positional binders no longer fail as unresolved declarations;
qualified free names retain their identities. Reference slots carry explicit
bound/free tags under canonical module encoding v11. Regenerate bindings and
rebuild hosts, plugins and contracts together. Nominal IDs, core ABI v1, discovery
ABI v1 and the one registry in the shared COM runtime remain unchanged.
Nested base/event argument spans now retain their authored source for diagnostics.

## Explicit generated support lookup (pre-release)

Plugin descriptors explicitly resolve Option::Some and the built-in env! macro.
Default-policy decoders explicitly invoke trait Default in derive/copied/owned
entrypoints. Adversarial normal/printed consumers preserve descriptor version,
actual root acquisition and missing-field semantics under application shadowing.
This fixes those COM8 plugin/default-decoder cases without changing COM layouts,
wire formats, Cargo owner encoding or the shared runtime registry.

## Borrowed event inspection (pre-release)

`ComBox::query_ref` lends an exact typed interface without weak bookkeeping.
`EventMessage::query` uses that view and maps only a missing interface to absence;
borrowed views cannot escape their owner. Explicit `query_weak` continues to
support independent observation and dropped-owner errors. The common exclusive
event transfer and weak-origin protocol are unchanged. Generic `ComRef` retention
is now exclusively fallible; clone an existing `ComPtr` for proven shared ownership.

## Typed object storage (pre-release)

Consolidate implementation proof in `ComObject` and provider-local typed storage.
Generated objects retain compact interface bindings, small borrow adapters and
their selected constructor family. Shared and exclusive borrowing, aligned
payloads, weak storage lifetime and provider-local freeing keep their existing
semantics. Threading uses one proof route. The global plugin registry continues
to live only in the shared COM runtime. No binary-size improvement is inferred.

## Ordinary object construction (pre-release ABI v1)

Remove generated cyclic constructors and the common header's unpublished-value
construction/publication methods. Objects always start from initialized Rust
values. Factories query weak identities afterward and connect event delivery
privately before returning a capability; direct FileDialog completion and
service events retain their existing immediate behavior. Window/popup factories
finish wiring in their exclusive manager transaction; menu records enter their
declaration fully connected.

Generated storage retains `MaybeUninit<T>` because weak references can keep
storage allocated after value destruction; there is no uninitialized-value
construction phase. Shared/exclusive borrowing, interface tables,
threading promises, descriptor ABI v1, core ABI v1 and fingerprint domain 10
remain unchanged. This supersedes the cyclic construction described in the
historical ABI 11/12 entries below. See
[the construction contract](docs/contracts/COM_OBJECT_CONSTRUCTION.md).

## Shared allocation lifetimes (experimental ABI 12)

Replace generated Arc storage and per-class weak operation tables with one common
COM control header and non-generic lifetime functions. Weak handles and nullable
weak handles now occupy one pointer; strong handles remain two. Only value
destruction, allocator release and interface projections remain class-specific.
Publish cyclic construction after initialization, preserve rollback for escaped
weak origins, and transfer promoted ownership without an extra query retain.
IDL fingerprints and core generation reject the former weak-token contract.
See [ABI v1](docs/abi-v1.md).

All notable changes to the experimental component ABI are documented here.
Semantic-version compatibility begins only after an explicitly ratified ABI.

## Weak lifetime handles (core ABI v1)

Add the optional C-shaped weak downgrade operation to the common object prefix,
`WeakComPtr<T>` with checked promotion, and generated cyclic initialization for
weak self origins. Generated objects keep one Arc-backed allocation; final strong
release destroys the value, while weak references can retain its empty shell.
The two-word strong handle and exact interface negotiation remain unchanged.
Old core generations are rejected before dispatch. See [ABI v1](docs/abi-v1.md).

## Shared binding-path validation

COM core owns the bounded, allocation-free Rust-2024 binding-path grammar and
exports it through the facade. IDL publication and COM widget catalog admission
share that validator; raw identifiers and optional leading `::` are supported,
while unescaped reserved words and unsupported syntax fail with field context.
`neocogi-inspect` also delegates version validation to the existing COM checker.
No new crate, runtime syntax-parser dependency, ABI or catalog schema is added.
Regression coverage compiles IDL published under `r#type`, copies its descriptor
through real catalog COM calls, compiles exported forms in a downstream crate,
and dispatches the typed event. See the [descriptor text contract](crates/core/README.md#descriptor-text-validation).

## Object/table handles (core ABI v1)

COM references now carry the allocation address and selected interface table.
Generated objects have no per-interface headers, and dispatch no longer recovers
owners using offsets. Query preserves object identity; all views share one
reference count and borrow guard. Raw nullable pairs distinguish absence from
half-null errors. Plugin factories, persistence, native C fixtures and consumers
use the new representation; older core generations are rejected. The deliberate
16-byte handle and its memory tradeoff are documented on `ComPtr` and in the
[storage decision](docs/contracts/COM_INTERFACE_STORAGE.md). Static inherited method
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
  for optional fields. See [the generic value contract](docs/contracts/IDL_GENERIC_VALUES.md) for syntax,
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
  See [the recursive value contract](docs/contracts/RECURSIVE_VALUE_CONTRACT.md) for the supported positions and validation evidence.

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
- Adopt NeoCogi COM package naming. The runtime package remains
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
are core v1, plugin descriptor v1, and runtime v1; see the [ABI guide](docs/abi-v1.md).

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
