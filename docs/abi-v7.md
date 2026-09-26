# neocogi-com core ABI v7

This document specifies the current experimental Rust-to-Rust component ABI.
The authoritative layouts are the public `#[repr(C)]` definitions in
`neocogi-com-core`; this guide explains how those layouts compose.

Core ABI v7 is pre-alpha and not yet a cross-release compatibility promise.
Runtime discovery is separately versioned and currently uses runtime ABI v1.

## 1. Scope

The binary boundary is C-shaped:

- fixed-width values and explicit `#[repr(C)]` records;
- `extern "C"` function pointers;
- `NcStatus` return values;
- GUID plus exact ABI-signature identity;
- COM-style `QueryInterface`, `AddRef`, and `Release` ownership;
- immutable interface identities and factory-plugin descriptions.

Rust trait objects, native enum layout, slices, arbitrary references,
allocator-owned `String`/`Vec`, unwinding, and compiler-specific Rust ABIs do not
cross the boundary. Variable-size owned values cross as COM interfaces; bounded
borrowed buffers use explicit pointer/count lowering.

This is not Windows COM. It has no registry activation, apartments, `HRESULT`,
or operating-system marshalling.

## 2. Interface identity and calls

Every generated interface has:

- an `#[repr(C)]` header containing a vtable pointer and opaque object pointer;
- an `#[repr(C)]` vtable whose prefix is `IUnknownVTable`;
- an interface GUID;
- a precomputed declaration signature scoped to its exact typed IDL module revision;
- descriptive method metadata for diagnostics and inspection.

The IDL resolver computes the module fingerprint before target Rust compilation. It
covers declaration names and versions, record fields, enum representations and
variants, method order/names/receivers/parameters/returns, inheritance,
threading, referenced declaration identities, and exact dependency revisions.
It then derives each declaration signature from that fingerprint and the
declaration ID. Distinct aggregates therefore remain distinct in typed
descriptors, while any module revision change still changes every declaration
signature. Generated target bindings contain the typed layouts, vtables,
descriptors, and completed 32 bytes; they do not reconstruct or hash the IDL
graph at runtime. The IDL compiler computes those hashes during the Cargo build.

`QueryInterface` matches both GUID and ABI signature. A GUID collision or a
stale binding therefore cannot silently produce a typed pointer. A successful
query returns one owned reference. `ComPtr` calls `AddRef` on clone and
`Release` on drop; `ComRef` is borrowed and changes no count.

`Result<(), NcStatus>` lowers to a status-only call. Other successful results
use an output pointer that remains unwritten on failure. Expected failures must
be negative `NcStatus` values. Panics must not cross an `extern "C"` boundary.

Mutable IDL methods lower from `&mut self`. Generated object storage enforces an
exclusive call gate and reports conflicting/reentrant access as `NC_E_BUSY`.

## 3. Threading capabilities

The ABI exposes thread-local and thread-safe object capabilities. The raw
pointer representation is identical; the typed pointer marker retains the
producer's promise after the concrete implementation type is erased.

- `ComPtr<T, ThreadLocal>` and `ComRef<T, ThreadLocal>` are neither `Send` nor
  `Sync`.
- `ComPtr<T, ThreadSafe>` and `ComRef<T, ThreadSafe>` are `Send + Sync`.
- weakening thread-safe to thread-local is safe; upgrading is not.

Generated thread-safe implementations require their private Rust value to be
`Send + Sync`. Atomic reference counting protects lifetime but does not by
itself prove safe concurrent method calls or destruction.

## 4. Contract declarations

`.ncidl` supports copied `struct`, `enum`, and `bitenum` values plus callable
`interface`, `class`, and `factory` roles. Concrete output structs/enums use `#[message]`. Every declaration/member is public; private fields belong to
implementation objects. `#[version(N)]` derives an IID; `#[id(N, "UUID")]` pins it.

The first required interface is the physical base; later requirements are
sibling projections. The required projection graph must be acyclic. A factory
may serve as the runtime's zero-argument construction root. Ordinary generic
interfaces are specialized before ABI generation. `IEventStream<E>` and
`IMessageSink<E>` therefore cross COM as concrete attach/post vtables with exact
identities and signatures. A class has at most one resolved event stream;
`EventPort` enforces one lifetime attachment and terminal closure. Optional COM
inputs use passing tag `NC_ABI_PASSING_OPTIONAL_COM_REF` (`0x08`): null means
absence, and a present pointer must satisfy the same exact interface identity
and borrowing contract as an ordinary `ComRef`. Each receiver has one `post` operation
accepting `NcMessage<T>`. Passing tag `NC_ABI_PASSING_MESSAGE` (`0x07`) describes a
borrowed pointer to the exact message C record/union, identified by its aggregate
signature. Decoding checks the active tag and acquires owned fields before mapping.
Strings/slices use pointer/count pairs; objects use checked concrete COM pointers.
The mapper and consumer FIFO remain entirely in the receiving Rust binary.

## 5. Compile-time modules

A module's build script resolves its transitive source graph and emits Rust
bindings with fixed declaration signatures. Contracts have no native libraries,
module-description getters, runtime registration tables, or builtin seeding.
Cargo `links`/`DEP_*` graph metadata remains a build-time dependency protocol.

The signature covers the complete transitive module revision, preserving exact
compatibility across independently built hosts and plugins. Different revisions
may coexist while disconnected; factories and COM pointer transfers reject an
incompatible connection. Registration no longer enforces one module revision
per process.

## 6. Factory plugin descriptions

A plugin shared library must export this discovery symbol:

```text
nc_get_factory_plugin_description_v1
```

The getter lends an immutable `NcFactoryPluginDescription` while the library is
mapped:

```rust,ignore
#[repr(C)]
pub struct NcFactoryPluginDescription {
    pub description_abi_version: u32,
    pub core_abi_version: u32,
    pub plugin_id: NcGuid,
    pub qualified_name: NcStaticUtf8,
    pub plugin_version: NcStaticUtf8,
    pub dependency_count: u32,
    pub dependencies: *const NcPluginDependencyDescriptor,
    pub root_factory: NcInterfaceId,
    pub root_factory_threading: u32,
    pub activate: Option<NcActivatePluginFactory>,
}
```

The descriptor ABI is independently versioned as **1**, matching
`NC_FACTORY_PLUGIN_ABI_VERSION` and the getter suffix. These pre-alpha versions
have not been incremented for every layout change, so matching version numbers
alone do not establish compatibility with older checkouts. Rebuild the runtime,
host, and plugins together. Additional symbols are permitted; this is the one
plugin discovery entrypoint the loader requests.

The activation function has one fixed shape:

```rust,ignore
unsafe extern "C" fn(out: *mut *mut IUnknown) -> NcStatus
```

It takes no erased arguments and returns one owned root reference on success.
The bootstrap clears its output to null before fallible work; failure transfers
no reference. This differs from ordinary generated method payload outputs,
which remain unwritten on failure.
The runtime checks the requested generated factory ID/signature and threading
capability before invoking bootstrap, then validates the returned projection.
A signature mismatch returns `NC_E_ABI_VERSION` without constructing a root. All later construction happens by
calling typed factory methods.

Plugin dependency records contain hard plugin IDs used for ordering. They never
perform file discovery or recursive loading.

## 7. Registration and loading

Factory-plugin registration validates the descriptor ABI, full root signature,
threading, plugin identity/name mapping, and hard dependencies. It publishes the
plugin atomically and does not invoke bootstrap. Re-registering the same static
descriptor is idempotent. No contract graph is traversed or registered.

Dynamic loading is for trusted code. Mapping a library may execute platform
loader initialization before metadata can be validated. Required live-load
failure is process-fatal after diagnostics; successful and post-open failed
modules remain pinned so no borrowed code/data pointer can outlive its library.
Hot unload/reload is outside this ABI.

## 8. Runtime ABI v1

Hosts first call `nc_com_runtime_abi_version` and require
`NC_COM_RUNTIME_ABI_VERSION == 1`. They then obtain the immutable size-prefixed
`NcRuntimeApiV1`, `NcPluginRegistryV1`, `NcInspectionApiV1`, `NcDebugApiV1`, and
`NcPersistenceApiV1`
tables through the corresponding `_v1` getters. There is no contract-registry
table. Plugin descriptor ABI also remains v1; core object ABI remains v7.
The plugin and runtime version numbers stay at one during pre-alpha and do not
promise compatibility between checkout revisions. Rebuild hosts, runtimes, and
plugins together.

`NcRuntimeApiV1` contains `register_factory_plugin_description`,
`load_plugin_library_utf8`, and `activate_factory_plugin`. Other tables enumerate
plugins and loaded modules, inspect plugin metadata, and manage diagnostic
callbacks. Count/fill operations report required length for retryable growth.
There is no contract-module enumeration or intrinsic initialization step.

`NcPersistenceApiV1` creates/destroys caller-owned, thread-local recipe contexts,
registers borrowed `IPersistenceFactory` objects with one retained reference,
snapshots `IPersistable` trees, and restores one owned `IUnknown` root. Its fixed
`NcPersistenceLimitsV1` policy explicitly carries binary and object-tree budgets.
Context mutation/destruction requires exclusive access; restore takes a shared
borrow. Contexts never participate in the global plugin catalog.

Successful snapshot capture calls `NcPersistenceWriteBytesV1` exactly once with
a borrowed complete document. The callback may copy it, must not unwind, and
cannot retain the byte pointer. Failed capture never calls the sink. The runtime
retains ownership of temporary storage, and no Rust allocation crosses the ABI.
The persistence consolidation preserved the existing interface identities and
NCB-v1 record schema; both remain experimental contracts.

The runtime tracks at most 1,024 live-loaded module records and 64 simultaneous debug
messengers. The public Rust facade additionally bounds copied enumeration data
and inspection text. Offline inspection pins opened libraries without adding
module catalog records, so the live-load record limit does not bound those
inspection handles.

## 9. ABI signatures

Every IDL-generated ABI signature is a SHA-256 digest of its canonical typed
stream. This representation is unconditional across hosts, plugins, inspection,
and monolithic builds. Runtime calls compare identities and exact signatures.
Declaration names and versions participate in the build-time identity and
fingerprint inputs. A plugin's qualified name is also validated against its
derived plugin ID; its release-version text is diagnostic and does not select
a compatible implementation.

## 10. Ownership and trust

Description pointers and static UTF-8 views are borrowed from retained native
libraries or runtime storage. Inspection callbacks must copy values they need
after the call. Successful activation transfers one COM reference to the host.

`NcStaticUtf8` stores `(*const u8, u32)`. Its
`unsafe as_str<'a>(self) -> Result<&'a str, NcStatus>` validates UTF-8 without
copying; `'a` is supplied by the caller and is not tied to a descriptor borrow.
`NcUtf8View` stores `(*const u8, usize)` and currently has no conversion methods.
Both are raw views that do not own storage or retain libraries. See the
[conversion and lifetime table](../core/README.md#text-bytes-and-borrowed-storage).

Structural validation protects against mistakes by trusted producers. It is
not memory isolation and cannot make arbitrary pointers or untrusted native code
safe. Installation, signature verification, allowlists, path policy, and
sandboxing belong to the application or deployment system.
