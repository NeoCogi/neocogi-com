# neocogi-com core ABI v7

This document specifies the current experimental Rust-to-Rust component ABI.
The authoritative layouts are the public `#[repr(C)]` definitions in
`neocogi-com-core`; this guide explains how those layouts compose.

Core ABI v7 is pre-alpha and not yet a cross-release compatibility promise.
Runtime discovery is separately versioned and currently uses runtime ABI v2.

## 1. Scope

The stable boundary is C-shaped:

- fixed-width values and explicit `#[repr(C)]` records;
- `extern "C"` function pointers;
- `NcStatus` return values;
- GUID plus exact ABI-signature identity;
- COM-style `QueryInterface`, `AddRef`, and `Release` ownership;
- immutable contract and factory-plugin descriptions.

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
- a canonical ABI signature covering method order, names, receiver modes,
  lowered parameters/returns, inherited interfaces, and referenced identities;
- descriptive method metadata for diagnostics and inspection.

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

## 4. IDL declarations

A `.ncidl` module may declare `struct`, `enum`, `trait`, `class`, and `factory`.
Every declaration/member is public. Its qualified name is the dotted module
name plus the identifier exactly as authored. `#[version(N)]` derives identity;
`#[id(N, "UUID")]` pins it explicitly while retaining the version.

`class` describes one public callable object surface and its required
interfaces. `factory` describes a callable construction surface. Neither
contains a runtime constructor record. Private fields and implementation
details exist only in implementation crates.

## 5. Contract modules

Generated Rust bindings compile the ABI layouts and typed wrappers used by both
host and plugin. Canonical descriptive data lives once in a separately linked,
fingerprint-qualified contract library.

The generated Rust crate exports an `NcContractModuleReference`:

```rust,ignore
#[repr(C)]
pub struct NcContractModuleReference {
    pub module_id: NcGuid,
    pub abi_fingerprint: NcContractFingerprint,
    pub get: Option<NcGetContractModuleDescription>,
}
```

Its getter returns an immutable `NcContractModuleDescription` owned by that
native contract library:

```rust,ignore
#[repr(C)]
pub struct NcContractModuleDescription {
    pub description_abi_version: u32,
    pub module_id: NcGuid,
    pub abi_fingerprint: NcContractFingerprint,
    pub module_name: NcStaticUtf8,
    pub dependency_count: u32,
    pub dependencies: *const NcContractDependencyDescription,
    pub declaration_count: u32,
    pub declarations: *const NcContractDeclarationDescription,
    pub source: NcStaticUtf8,
}
```

The description stores the canonical module name/source, module identity, full
SHA-256 ABI fingerprint, top-level declaration identity/kind/version/threading,
and exact fingerprinted dependency getters. It does not store Rust object
state, implementation code, vtables, constructors, factory activation code,
Cargo paths, or mutable registry state.

The module fingerprint covers canonical IDL and exact dependency revisions.
Native filenames and getter symbols include the full fingerprint. Thus two
incompatible revisions may both be mapped by the platform loader, after which
the shared runtime can diagnose one logical module ID paired with two
fingerprints as `NC_E_CONTRACT_MISMATCH`.

## 6. Factory plugin descriptions

A plugin shared library exports exactly one symbol:

```text
nc_get_factory_plugin_description
```

The getter lends an immutable `NcFactoryPluginDescription` while the library is
mapped:

```rust,ignore
#[repr(C)]
pub struct NcFactoryPluginDescription {
    pub core_abi_version: u32,
    pub validation_mode: NcAbiValidationMode,
    pub plugin_id: NcGuid,
    pub qualified_name: NcStaticUtf8,
    pub plugin_version: NcStaticUtf8,
    pub dependency_count: u32,
    pub dependencies: *const NcPluginDependencyDescriptor,
    pub contract_count: u32,
    pub contracts: *const NcContractModuleReference,
    pub root_factory_id: NcGuid,
    pub root_factory_threading: u32,
    pub activate: Option<NcActivatePluginFactory>,
}
```

The activation function has one fixed shape:

```rust,ignore
unsafe extern "C" fn(out: *mut *mut IUnknown) -> NcStatus
```

It takes no erased arguments and returns one owned root reference on success.
The runtime checks the requested generated factory ID/signature and threading
capability before exposing a typed pointer. All later construction happens by
calling typed factory methods.

Plugin dependency records contain hard plugin IDs used for ordering. They never
perform file discovery or recursive loading.

## 7. Registration and loading

Contract registration follows exact dependency getters depth-first. It validates
the complete graph before publishing it. Re-registering the same ID and
fingerprint is idempotent; a different fingerprint is a mismatch. Cycles,
duplicate identities, malformed pointer/count pairs, null getters, or invalid
UTF-8 descriptions are rejected.

Factory-plugin registration validates its full contract graph, plugin
dependencies, root factory declaration, validation mode, and identity mapping
before one atomic catalog publication.

Dynamic loading is for trusted code. Mapping a library may execute platform
loader initialization before metadata can be validated. Required live-load
failure is process-fatal after diagnostics; successful and post-open failed
modules remain pinned so no borrowed code/data pointer can outlive its library.
Hot unload/reload is outside this ABI.

## 8. Runtime ABI v2

Hosts first call `nc_com_runtime_abi_version` and require
`NC_COM_RUNTIME_ABI_VERSION == 2`. They then obtain immutable, size-prefixed
tables. Larger tables are compatible within v2; shorter tables cannot be used
for fields they do not contain.

`NcRuntimeApiV2` provides:

- `register_contract_module`;
- `register_factory_plugin_description`;
- `load_plugin_library_utf8`;
- `activate_factory_plugin`.

Separate v2 tables enumerate plugins/loaded modules, enumerate canonical
contract modules, inspect metadata and plugin dependencies, inspect an unopened
plugin without registration, and manage diagnostic callbacks. Count/fill
operations write the required length so callers can retry append-only growth.

The runtime retains at most 1,024 dynamic modules and 64 simultaneous debug
messengers. The public Rust facade additionally bounds copied enumeration data
and inspection text.

## 9. Validation modes

One assembly uses exactly one representation:

- `none` retains structural checks with an unchecked signature sentinel;
- `sha-256` stores a SHA-256 digest of the canonical typed stream;
- `fnv-1a-128` stores an FNV-1a-128 digest;
- `mangling` stores the collision-free canonical stream.

Plugins record the selected mode. Assembly planning and runtime registration
reject incompatible modes before root activation. Names and versions remain
diagnostic; ABI compatibility is determined by identities, fingerprints, and
signatures.

## 10. Ownership and trust

Description pointers and static UTF-8 views are borrowed from retained native
libraries or runtime storage. Inspection callbacks must copy values they need
after the call. Successful activation transfers one COM reference to the host.

Structural validation protects against mistakes by trusted producers. It is
not memory isolation and cannot make arbitrary pointers or untrusted native code
safe. Installation, signature verification, allowlists, path policy, and
sandboxing belong to the application or deployment system.
