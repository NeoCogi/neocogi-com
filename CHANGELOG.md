# Changelog

Development uses provisional **[ABI v4](docs/abi.md#abi-identities-and-versions)**.
Entries below record development; later entries can supersede older implementation details.
Historical draft numbers do not define additional supported COM baselines.

## 0.5.0-wip-alpha.1 (development)

- Use ABI version 4 for the host bundle and plugin descriptor. A fixed header
  marker distinguishes superseded layouts before either side reads the version.
  Providers export the stable `nc_get_plugin_descriptor` getter. Inspector JSON
  uses one `abi_version` field in schema 10.
- Declare shared ownership for persistence factories and snapshot roots. The
  Rust snapshot API takes `&ComPtr<IUnknown>`. Existing NCB-v1 documents retain
  their encoding.
- Separate live shared `ComRef` loans from move-only `ComMut` access.
- Add optional shared transfers, optional shared inputs and scoped borrowed
  getters. NCIDL v15 fingerprints distinguish the native passing contracts,
  while actual Rust type checks distinguish direct and fallible projections.
- Carry the originating status through serializer and deserializer failures and
  retain the first recorded provider diagnostic.

## 0.4.0

The eight SDK packages move together to provisional ABI v3/core generation 21.
Native plugins use discovery family 3 and export `nc_get_plugin_descriptor_v3`.
Rebuild the host, providers and generated bindings with matching SDK versions.
IDL encoding v14 and NCB persistence format v1 retain their version identities.

- Use `neocogi-com-sdk` for Rust callers and providers and `neocogi-com` for
  low-level ABI types. The SDK reexports those types. Regenerate bindings for
  the updated imports and the Rust ownership and method-wrapper APIs.
- Link `neocogi-com-host` into the application. It supplies runtime access to
  providers, including nested providers, and loads native plugin libraries.
  Persistence still requires the application to select a serialization provider.
- ABI v3 separates shared and boxed lifetime state. Declared ownership selects
  the required backing state, and core generation 21 identifies the four-word
  provider metadata layout.

See the [provider migration guide](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/docs/guides/ABI_V3_MIGRATION.md) and the
[standalone walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/examples/getting-started/README.md).

## 0.4.0-alpha.2

This candidate versions eight SDK packages together. It uses provisional
ABI v3/core generation 21, plugin discovery family 3, IDL encoding v14
and NCB v1. Rebuild the host, providers and generated bindings together.
The [migration guide](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0-alpha.2/docs/guides/ABI_V3_MIGRATION.md#update-the-sdk-packages-together)
describes the package-name changes.

- Rename the ABI and pointer package to `neocogi-com` and the Rust SDK to
  `neocogi-com-sdk`. The compiler uses the base package. Regenerated bindings
  and procedural macros use the SDK, which reexports the same ABI types.

- Applications link `neocogi-com-host` and pass its runtime access to providers.
  The optional alpha.1 host shim and its SDK loader are retired. The linked host
  continues to load native provider libraries, including the serialization
  plugin, through `nc_get_plugin_descriptor_v3`.
- Name the provider allocation `ComObjectInner`, its shared-object gate
  `ComBorrowGate`, and its read/write permits by access mode. Generated Rust uses
  `__ComSharedInner` and `__ComBoxedInner`. Mutable local access returns
  `LocalRefMut` alongside `LocalRef`.
- Import `ComBytesExt` and `ComStringExt` for the Rust methods on owned and
  borrowed byte and string interfaces.
- Use `ComValue::AbiRepr` for the live ABI representation and `KeepAlive` for
  temporary data it addresses. Regenerate Rust bindings for the new associated
  names and `BORROW_ABI_REPR`. ABI v3/core 21 and the NCB format stay the same.
- Name the shared weak token `NcSharedWeakRaw` and the host's plugin operation
  table `NcPluginApiV3` in Rust and C. Their field layouts and discovery symbols
  retain ABI v3/core 21.

## 0.4.0-alpha.1

This alpha versioned nine SDK packages together, including the optional native
host shim. It used provisional ABI v3/core 21. Rebuild the host, every
provider and all generated bindings with the same SDK version. See the
[ABI migration guide](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0-alpha.1/docs/guides/ABI_V3_MIGRATION.md) and
[standalone alpha walkthrough](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0-alpha.1/examples/getting-started/README.md).

- Restore checks declared child counts and object depth during decoding before
  reading descendants or reserving their record storage. A refused local
  child-storage allocation returns `NC_E_OUT_OF_MEMORY`.
- The NCB writer reports allocation refusal as `NC_E_OUT_OF_MEMORY` during
  factory creation and active writes. NCB-v1 documents keep their byte format.
- IDL reserves the canonical `IUnknown` IID for the built-in callable interface.
  Generated borrow docs explain how a successful retain grants ownership.
- Native qualification selects the actual inspector integration target on
  Linux, macOS and Windows. The Linux row also compares min-size diagnostics
  and covers native paths containing invalid UTF-8 bytes.

Known limitation. A generated interface with 384 distinct direct parents was
accepted by IDL and then hit Rust's default constant-evaluation limit on stable
and 1.85. The observed breadth depends on how many projections composition must
merge and on the compiler and graph shape.

- Name the private present interface pair `NonNullComRaw` and its borrowed
  method-pointer constructors `borrow_pair`/`borrow_pair_mut`. Presence checks
  remain separate from value liveness, interface identity, ownership and
  threading; representation and ABI v3/core21 remain unchanged.

- Name generated Rust method access with `ComRustInterface`,
  `ComInterfacePointer`, and `IFooWrapper`/`Wrapper<M>`. Regenerate checked-in
  bindings and migrate imported families, collision admission and Rust consumers;
  release the old generated `Client` suffix without compatibility aliases.
  Preserve C layouts, exact identities/signatures, canonical IDL encoding and
  WIP ABI v3/core21; this is a Rust API/generated-name change.

- Resolve persistence diagnostics and discover interface/value metadata without
  recursive nominal graph traversal. Preserve first-leaf reasons, source labels,
  derived-first ancestry, base-first methods and actual supplying projections;
  unary public capability queries also use loops for detached expressions.
  Keep syntax/specialization/runtime budgets separate and identity bytes unchanged.
  Refresh built-in bindings' reserved-name aliases left stale by the earlier
  boxed-input canonicalization; executable syntax and ABI constants stay unchanged.

- Canonicalize checked boxed-owner results and consuming inputs after independent
  incoming/canonical interface validation. Resolve input tables once, adopt only
  after full admission, and restore exact original caller pairs on rejection.
  Keep matching-policy cleanup, owner counts and C layouts unchanged; regenerate
  WIP Rust consumers for the input resolver's canonical-table result.

- Share one const IID/full-digest equality predicate between shared and boxed
  owner queries. Preserve compile-time identity fast paths, content equality
  across independent metadata storage and distinct retain/transfer/error behavior.
  No ABI or interface-identity change.

- Make consuming boxed weak interface erasure move its existing token after exact
  canonical IUnknown lookup. Preserve live/expired operation and threading without
  a transient weak retain/release, including at the weak counter limit. Borrowed
  queries and clones still retain independent observers. No ABI change.

- Remove runtime allocation-policy classification throughout raw weak transport,
  adoption, value decoding, provider construction and native bindings. Shared raw
  weak pointers name NcSharedLifetimeState; every ABI ownership position requires
  matching backing. Remove NcObjectTypeInfo.ownership, NC_OWNERSHIP_*, raw expected-
  kind admission and erased rejection dispatch. Interface failures release their
  declared state. Provider metadata shrinks to four words: select WIP ABI v3/core
  21 and reject provisional core 20 before use. Supersedes earlier mixed-policy
  rejection/cleanup promises; exact interface, lifetime/borrow and allocator rules
  remain. This is unreleased development, not a new release family or publication.

- Trust established shared/boxed lifetime policy in typed handles and matching
  callbacks. Admit policy at mixed raw boundaries, validate before typed weak
  construction, and balance rejected owned weak tokens under their actual policy.
  Keep borrowed rejection unchanged and preserve both incoming-table identity and
  canonical allocation lookup. Narrow the WIP boxed support safety contracts;
  remove erased normal weak helpers and local promotion's obsolete C-shaped
  handshake/revalidation. Preserve layouts, core 20, exact interface signatures,
  reader/writer admission and synchronous destruction.

- Select separate shared/exclusive canonical lifetime prefixes through
  ComObject::UNKNOWN_VTABLE. Move retain/release into typed Rust header operations,
  remove universal ownership dispatch, and keep C adapters private to actual slots.
  Weak bookkeeping/promotion are crate-private Rust helpers; foreign test callers
  use their own typed-handle adapters. Preserve non-unwinding counter exhaustion,
  exact errors, synchronous boxed destruction and provider-local cleanup. This
  narrows WIP v3 Rust support APIs without changing core 20 C layouts or signatures.

- Mark ABI v3 explicitly as WIP in agent guidance, the specification, READMEs
  and source documentation. Its current layouts and callbacks remain provisional
  until release freeze; existing version admission still rejects incompatible binaries.

- Split the fused lifetime header into NcSharedLifetimeState and
  NcBoxedLifetimeState. Shared strong_count is purely numeric; boxed
  owner_and_borrows combines only owner presence and call admission. Both retain
  their own weak_count and an immutable type-info pointer. Ownership kind moves
  to NcObjectTypeInfo, and raw admission checks it before interpreting either
  complete header. Weak arithmetic/cleanup is shared without a fused state type.
- Select incompatible ABI v3/core generation 20 and family-3 host/plugin
  bootstrap exports. Remove old record names and discovery fallbacks; rebuild
  all participants together. Preserve method-interface identities, IDL encoding
  v14, NCB-v1 documents, synchronous boxed destruction and the explicit codec
  provider requirement. This is unreleased development, not a publication.

- Document the shared/exclusive `NcLifetimeState` state machine, including
  reader/writer admission, owner removal, synchronous value destruction and
  final weak-reference deallocation. Explain the implicit weak reference and
  terminal expiration without adding state fields or changing lifetime behavior.

- Rename the common allocation header from `NcWeakControl` to `NcLifetimeState`
  across shared/exclusive ownership, weak tokens, provider storage and native C
  mirrors. The name covers strong ownership and exclusive call admission as well
  as weak storage retention. Remove the old Rust/C type names without aliases;
  field order, atomic state transitions, callback ABI, interface digests and
  ABI v2/core generation 19 are unchanged.

- Rename checked interface-cast helpers: `box_projection` becomes `try_box_cast`,
  `shared_weak_projection` becomes `check_shared_weak_cast`, and the private
  metadata lookup becomes `try_cast`. Update owner, weak-token and value-wire
  callers without aliases. Casts retain exact interface checks and existing
  ownership/error semantics; C layouts, callback signatures, interface digests
  and ABI v2/core generation 19 are unchanged.

- Clarify Rust ownership and representation names: `Runtime` becomes
  `RuntimeHandle`, `NcClassLifetime` becomes `NcObjectTypeInfo`, and
  `NcAbiSignature` becomes `NcAbiDigest`. Runtime diagnostics use
  `RuntimeDiagnostics`. Scoped serialization uses `ScopedComProvider` and
  `ScopedComWriteGuard`; record/sequence cursors use `RecordFieldReader` and
  `SequenceReader`. Host internals use `PluginRegistry` and `RegisteredPlugin`.
  Update generated bindings, macros, native C mirrors, examples and consumers
  together; no old-name aliases remain. Interface schemas, GUIDs, digest bytes,
  C layouts, NCB wire v1 and ABI v2/core generation 19 are unchanged.

- Rename the source-level IDL model enum from `AbiType` to `IdlType` across
  parsing, resolution, specialization and generation. The separate core
  `AbiType` trait remains unchanged. IDL spelling, canonical encoding and
  interface identities/signatures retain their existing contracts.

- Add the optional `neocogi-com-host-dylib` native shim, exporting
  `nc_get_host_api_v2` to lend the host rlib's canonical C service graph.
  Applications choose one static or dynamic host owner; providers and nested
  acquisitions receive the same supplied runtime and never initialize a catalog.
- Add unsafe SDK `RuntimeHandle::load_host_library` with explicit filesystem paths,
  process-pinned images, existing host-table admission and typed startup errors.
  Share the existing Unix/Windows loader without selecting a host implementation
  in the SDK. No fallback registry or unload path is introduced. ABI v2/core
  generation 19, existing C layouts and plugin bootstrap are unchanged.
- Native regressions build an independent application without a host dependency,
  two separately generated provider libraries and malformed bootstrap images.
  Exercise repeated bootstrap, nested A-to-B acquisition, registry visibility,
  retained calls, cross-thread runtime access and equivalent static deployment.
  Add the shim to archive qualification and shared-host parity to native CI.

- Rename the hidden Rust provider-table entry from `__ComProjection<T>` to
  `ComInterfaceTableEntry<T>`. Generated providers now expose `INTERFACE_TABLE`
  instead of `PROJECTIONS`, composed with `__nc_interface_table` and reduced to
  runtime vtable pointers with `__nc_interface_vtables`. Update core/SDK exports,
  IDL/object generation and checked-in persistence/serialization providers
  together; remove the old names without aliases. Rust consumers naming these
  helpers must update and regenerate bindings. ABI v2, core generation 19,
  interface identities/signatures, C layouts and ownership behavior are unchanged.

## 0.3.0 — 2026-11-15

- Published all eight packages on crates.io from `v0.3.0`, commit `35db184`,
  after the required native and safety CI jobs passed. Registry archive hashes
  and Git provenance, docs.rs pages, registry-only stable/Rust 1.85 consumers
  and the installed inspector were verified.

- Rename the provider-local Rust borrow helpers to `ComReadGuard` and
  `ComWriteGuard`, and their shared allocation bookkeeping to `ComBorrowState`.
  Update generated object implementations, storage recovery and local access
  together. Remove the old `__ComSharedLease`, `__ComExclusiveLease` and
  `__ComCallGate` names rather than keeping aliases. The borrow policy,
  allocation layouts, interface signatures and ABI v2/core generation 19 remain
  unchanged; Rust consumers naming the helpers must update and regenerate together.

- Promote the coordinated eight-package SDK from `0.3.0-alpha.1` to `0.3.0`.
  Keep ABI family 2, core generation 19, IDL encoding v14 and NCB wire v1.
  The application must explicitly register or load `nc.serialization` before
  persistence; existing `0.2.0` applications must migrate this deployment setup.
- Align exact internal dependencies, maintained fixture locks, the first-use
  tutorial, installation examples and release-tag links with `0.3.0`.
  The source tag preceded registry publication; uploads completed only after
  all required CI jobs passed for that exact tagged commit.

- Make the explicit serialization-provider requirement normative in the ABI-v2
  specification, raw persistence table docs and Rust caller entry points. Keep
  ABI family 2/core generation 19 and NCB v1; codec availability is an application
  deployment requirement, not a table-admission guarantee. Remove stale guidance
  suggesting frozen ABI-v2 tables can change in place. Provider absence/selection
  failure leaves contexts and registered recipes usable for retry.
- Exercise absence, explicit registration and recovery through the real C table
  using one context and generated COM factories. Assert no failed-call delivery,
  cleared restore output, exact NCB bytes and retained factory/capability owners.
  Clarify that codec owners cover codec/restore callbacks, while snapshot byte
  delivery uses an independent host-owned document after those owners are released.

## 0.3.0-alpha.1 — 2026-11-14

- Remove the host's runtime dependency on the serialization implementation.
  Snapshot and restore acquire `nc.serialization` through this host's own catalog
  and request named `ncb`, preserving the C ABI and NCB-v1 bytes. Register the
  static serialization descriptor or load `neocogi-com-serialization-dylib`
  explicitly before persistence. No implicit codec fallback remains.
- Preserve argument, ownership and capture-budget error precedence; missing
  providers return `NC_E_NOT_FOUND` and format-selection statuses propagate.
  Retain root/format capabilities through each operation and keep callbacks
  outside catalog locks. Preserve permissive `usize` budget ceilings at the
  existing `u32` format ABI boundary.
- Promote all eight COM packages together to `0.3.0-alpha.1`, superseding the
  unpublished host-only `0.2.1` candidate and removing its version-policy exception.
  Rust dependency compatibility with the `0.2.0` cohort is not promised; update
  downstream UI/platform compositions together. Published platform
  `0.3.0-alpha.1` still pins COM `0.2.0` and needs a corresponding update before
  sharing the new SDK's Rust types. ABI family 2, core generation 19, IDL encoding
  v14 and NCB wire v1 retain their existing authorities.
- Linux regressions cover static and real-DLL persistence, isolated missing
  providers, JSON defaults, selection failures, reentrant callbacks, exact bytes,
  truncation/trailing input and retained-owner cleanup. Existing malformed-input,
  budget, cycle and constructor-failure coverage remains. Final publication and
  native release qualification are separate from source preparation.
- Performance remains unqualified for this final release: the prototype passed
  its Vulkan 2% gate but one UI CPU case regressed 3.51%. Recheck against the final
  published host build; historical prototype results do not qualify this candidate.

## 0.2.0 — 2026-11-13

- Promote the qualified alpha.6 implementation to the coordinated eight-package
  0.2.0 SDK release. Update exact internal dependencies, maintained lockfiles,
  tutorial manifests, installation commands and release documentation together.
- Use ABI v2, core generation 19, IDL encoding v14 and NCB wire v1. Rebuild hosts,
  providers and generated bindings from this SDK; ABI v1 plugins are rejected
  before bootstrap.
- Include the COM serialization interfaces, default NCB and lightweight JSON
  codecs, diagnostic paths, borrowed wire decoding and guarded fixed-array
  construction described in the prerelease entries below. The native serialization
  plugin includes both formats; the reusable rlib keeps JSON optional.
- Carry forward bounded IDL syntax and prospective template-height admission,
  immutable admitted-model accessors and shallow layout proofs. See the
  [IDL contract](docs/contracts/IDL_MODULES.md) for migration and compiler limits.

## 0.2.0-alpha.6 — 2026-11-13

- Parse the supported IDL grammar directly after iterative token-group admission.
  Excessive groups or inline type depth return source-located `EIDL068` errors;
  deeply unsupported Rust syntax is rejected without building a general Rust AST.
- Keep admitted source declarations/templates and resolved values behind immutable
  accessors. Migrate direct readers to `Module::declarations()`, `templates()` and
  `ResolvedModules::values()`. Detached clones remain available for inspection;
  unchecked installation into compiler-owned graphs is no longer public.
- Check prospective normalized type height before template copying/substitution,
  returning `EIDL060` when valid source halves would compose beyond the limit.
  Preserve existing work/storage, combined descent and recursive-cache limits.
- Emit shallow constant layout proofs for broad copied records, preserving
  predicate order and short-circuit evaluation. Cover maximum inline shapes,
  small-caller cleanup, independent generated COM calls and failure publication.
- Remove imposed Cargo build-job caps from workspace configuration, CI and task
  subprocesses. Honor Cargo defaults and caller job settings; keep compiler-spawning
  test cases serial.
- Prepare the coordinated eight-package SDK as `0.2.0-alpha.6`, retaining ABI
  family 2, core generation 19, IDL encoding v14 and NCB wire v1. Rebuild hosts,
  providers and generated bindings together. Native/Cargo text APIs retain their
  documented process context; arbitrary nominal graph depth and total allocation
  are separate from inline syntax admission.

## 0.2.0-alpha.5 — 2026-11-13

- Construct fixed arrays with their actual element layout and one initialized
  prefix guard. Reject wrong sequence counts before child acquisition; preserve
  partial-owner cleanup, alignment, no-alloc support and completed-value cleanup
  on later framing failure.
- Borrow raw inputs throughout `ComValue` decoding and `decode_inline`, removing
  recursive whole-wire copies. Handwritten implementations now accept
  `&Self::Wire`; callers pass `&wire` or an initialized slot reference. Regenerate
  bindings with this SDK; no compatibility decoder API is retained.
- Report JSON container errors at their real container, preserve selected/missing
  field names and attempted array/map/option/enum child paths, and retain the
  first diagnostic. Cover actual generated COM calls on one-MiB callers plus
  owner counts, failure/unwind cleanup and min-size/runtime measurements.
- Walk authored layout/projection and module dependency graphs with explicit
  frames, so long shallow chains resolve on a one-MiB caller stack. Preserve
  cycle diagnostics, canonical dependency order and existing signature bytes.
- Reuse the SDK's bounded byte-copy contract for persistence state. Empty state
  needs no provider copy call; unrepresentable lengths return a range error
  before allocation. Remove duplicate copying and post-copy length checks.
- Prepare the coordinated eight-package SDK as `0.2.0-alpha.5`,
  updating exact internal requirements, maintained lockfiles, tutorial manifests,
  installation examples and current documentation links together.
- Retain ABI family 2, core generation 19, IDL encoding v14 and NCB wire v1.
  Rebuild hosts, providers and generated bindings from this SDK release; ABI v1
  plugins remain unsupported. Deprecated declaration version attributes are
  accepted with warnings and do not participate in contract identity.
- Include NCB and the custom JSON codec in the native serialization plugin, with
  NCB selected by default. The reusable serialization rlib keeps JSON optional.
  This source candidate does not claim crates.io publication or native
  macOS/Windows qualification; follow the release procedure before publishing.

## 0.2.0-alpha.4 — 2026-11-12

- Keep resolved qualification source paths in an equivalent ordinary Win32
  spelling where possible, so nested native builds do not inherit unnecessary
  extended-path names. Preserve paths that require the extended namespace and
  retain the macOS alias fix. Add a Windows driver regression that links a DLL
  beneath a temporary parent containing spaces and Unicode.
- Coordinate all eight SDK packages, exact internal requirements, maintained
  lockfiles, tutorials and current release links as `0.2.0-alpha.4`. ABI family 2,
  core generation 19, IDL v14 and NCB v1 retain their existing contracts. NCB and
  JSON remain built into the native serialization plugin, with NCB the default.
  This Git prerelease does not claim crates.io publication.

## 0.2.0-alpha.3 — 2026-11-12

- Resolve temporary source paths before creating qualification workspaces, so
  macOS `/var` and `/private/var` aliases cannot create duplicate Cargo package
  identities. Cover registry-patched and relative-path consumers with a real
  Cargo regression and retain source/output cleanup ownership.
- Check three ownership rejection fixtures through structured compiler errors
  and primary source locations. Keep their intended constructor/lifetime failures
  covered across Rust diagnostic layout changes while continuing to follow the
  rolling `nightly` channel.
- Coordinate all eight SDK packages, exact internal requirements, maintained
  lockfiles, tutorials and active release links as `0.2.0-alpha.3`. ABI family 2,
  core generation 19, IDL v14 and NCB v1 retain their existing contracts; the native
  serialization plugin still includes NCB and JSON with NCB selected by default.
  This Git prerelease does not claim crates.io publication or completed remote
  platform qualification.

## 0.2.0-alpha.2 — 2026-11-12

- Always include NCB and the custom JSON codec in the serialization dylib,
  including builds with default features disabled. NCB remains the default;
  the reusable static rlib retains its optional JSON feature.
- Repair qualification setup: recognize native doctest path separators,
  declare NCB on the standalone host-runtime regression project, and give
  independent compile-fail consumers explicit generated-file paths. Keep the
  intended interface-conflict diagnostics and native/sanitizer test coverage.
- Remove the historical ABI v1 specification from the current tree in a separate
  commit. The released baseline remains available at `v0.1.0`; ABI v2 continues
  rejecting old native plugins before bootstrap.
- Coordinate all eight SDK packages and exact internal requirements as
  `0.2.0-alpha.2`. ABI family 2, core generation 19, IDL v14 and NCB v1 stay
  unchanged. This Git prerelease does not claim crates.io publication or
  completed remote platform qualification.

## 0.2.0-alpha.1 — 2026-11-12

- Make compiler factory and raw weak-promotion tests check the exact returned
  interface before typed adoption or release. Erased transport preserves the
  selected interface identity; it does not convert the result to IUnknown.
- Return admitted `Runtime` access from the host's safe `initialize_runtime()`
  constructor. `com_main` requires that typed result from its selected path;
  it no longer adopts raw service pointers on behalf of arbitrary safe modules.
- Preserve loader phase names in functional Display output, including min-size
  diagnostics for failed opens, missing getters and rejected native metadata.
- Qualify every shipped target through ordinary Cargo test selection, including
  the cdylib-only serialization shim; preserve both required named IDL consumers
  and complete extracted-archive/tutorial execution on Rust 1.85.
- Preserve concrete inspector diff/compatibility/schema diagnostics in min-size
  builds using canonical serialized values and escaping. Target Debug stripping
  remains enabled.
- Convert external JSON numbers directly to the requested floating width,
  preserving negative zero and correctly rounded midpoint/subnormal/overflow
  behavior. Keep integer/f64 syntax admission, typed overflow errors and owned
  terminal diagnostics. Record the measured [size/performance cost](docs/SERIALIZATION_COSTS.md).
- Align current runtime/descriptor and package guidance with their authoritative
  constants, getter and published inventory; retain archived ABI v1 and NCB v1.
- Make serialization formats COM objects with `ISerializer`, `IDeserializer`,
  `ISerializationOutput` and an `ISerializationFormat` factory. The same typed
  value implementations can use application- or plugin-supplied formats at runtime.
  Named fields and enum cases are available to providers; NCB v1 bytes stay the same.
  Sessions own their buffers, finish once and reject reuse after a format or schema
  failure. Fixed-buffer NCB helpers use scoped COM calls without allocation.
  Move codecs into the reusable `neocogi-com-serialization` rlib with a thin
  `serialization-dylib` shim. NCB is unconditional/default; the custom JSON codec is
  optional. No runtime Serde/parser dependency is introduced. Preserve failure messages and locations through a COM
  diagnostic projection and caller-owned convenience errors.
  See the [format contract](docs/contracts/SERIALIZATION_FORMATS.md).
- Remove declaration versions from nominal identity and canonical SHA-256 input.
  Ordinary declarations derive IDs from their qualified names; explicit overrides
  use `#[id("UUID")]`. Use distinct named types for semantic distinctions.
- Accept `#[version(N)]` and numbered `#[id(N, "UUID")]` as deprecated syntax.
  Ignore their integers and report source-located `WIDL001` warnings through the
  parser API and both Cargo build helpers. Remove the obsolete numbers to clear
  warnings; malformed syntax and duplicate explicit UUID overrides remain errors.
- Move canonical IDL encoding to v14 and regenerate built-in persistence bindings.
  Rebuild generated contracts, providers and hosts together: nominal IDs and exact
  signatures change from 0.1.0. Exact transitive signatures still reject incompatible
  contracts, including changes that retain the same native layout.
- Select ABI v2 for this alpha: runtime and descriptor discovery family 2,
  core generation 19, and the native `nc_get_plugin_descriptor_v2` export.
  Rename supplied service records and callbacks from `V1` to `V2`; rebuild hosts,
  providers and generated bindings together. ABI v1 discovery is not a fallback.
  Keep [ABI v1](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md) as the released historical baseline.
- Version all eight SDK packages as `0.2.0-alpha.1` with exact internal
  requirements. Tutorial manifests and current documentation target that cohort.
  This Git prerelease does not claim crates.io publication or completed remote
  Linux/macOS/Windows qualification for this commit.


## 0.1.0 — 2026-10-29

- Publish the six-crate SDK as its first non-prerelease `0.1.0` cohort, with exact
  internal SDK requirements and Rust 1.85 as the supported compiler floor.
- Retain core generation 18, runtime/descriptor family 1, IDL encoding v13 and
  NCB format v1. Hosts, providers and generated bindings use the same SDK release.
- Qualify the cohort through native Linux, Windows and macOS CI, extracted crate
  tests, independent generated consumers and the complete host/plugin tutorial.
- Keep the non-UTF-8-directory graph-encoding fixture on Linux so macOS archive
  tests can run on filesystems that reject such directory names. Portable output
  safety checks remain enabled; compiler and runtime behavior are unchanged.

## 0.1.0-alpha.1 — 2026-10-29

Initial public prerelease, prepared on 2026-10-28.

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
COM activation or marshalling. See the [ABI contract](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md) and
[maintainer release procedure](docs/guides/RELEASING.md) for the complete rules.

Older entries below describe development history and may be superseded by this
baseline. They are not additional supported compatibility generations.

## Shared SDK and explicit host package

Use `neocogi-com` in `crates/com` for both interface callers and providers.
The implementation is `neocogi-com-host` in `crates/host`, with Rust library
`neocogi_com_host`. Host persistence code uses the shared SDK; raw host tables
continue to use core. Update default/aliased `com_main` linkage, manifests,
fixtures, archive admission, CI and documentation together. The `runtime` SDK
feature remains a supplied-runtime API feature. Core/service ABI names and
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
portable host/provider sample and update persistence API wrappers, real applications,
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
Protect provider/wrapper binders, argument/helper families and const-expression
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
See [ABI v1](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md).

All notable changes to the experimental component ABI are documented here.
Semantic-version compatibility begins only after an explicitly ratified ABI.

## Weak lifetime handles (core ABI v1)

Add the optional C-shaped weak downgrade operation to the common object prefix,
`WeakComPtr<T>` with checked promotion, and generated cyclic initialization for
weak self origins. Generated objects keep one Arc-backed allocation; final strong
release destroys the value, while weak references can retain its empty shell.
The two-word strong handle and exact interface negotiation remain unchanged.
Old core generations are rejected before dispatch. See [ABI v1](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md).

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

- Trust established shared/boxed lifetime policy in typed handles and matching
  callbacks. Admit policy at mixed raw boundaries, validate before typed weak
  construction, and balance rejected owned weak tokens under their actual policy.
  Keep borrowed rejection unchanged and preserve both incoming-table identity and
  canonical allocation lookup. Narrow the WIP boxed support safety contracts;
  remove erased normal weak helpers and local promotion's obsolete C-shaped
  handshake/revalidation. Preserve layouts, core 20, exact interface signatures,
  reader/writer admission and synchronous destruction.

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
are core v1, plugin descriptor v1, and runtime v1; see the [ABI guide](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md).

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
- Made the inherited generated interface wrapper the sole safe interface calling
  layer. `ComPtr` and `ComRef` retain identical method syntax without generating
  parallel interface-specific extension traits.
- Defined runtime resource ceilings of 1,024 retained dynamic modules and 64
  simultaneous debug messengers.
