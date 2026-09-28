# neocogi-com core ABI v10

This document specifies the current experimental Rust-to-Rust component ABI.
The authoritative layouts are the public `#[repr(C)]` definitions in
`neocogi-com-core`; this guide explains how those layouts compose.

Core ABI v10 is pre-alpha and not yet a cross-release compatibility promise.
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
cross the boundary. Recursive application values cross as checked raw records,
integer tags, and call-scoped pointer/count storage. The receiver acquires any
contents it retains. Object identity crosses as explicit COM references.

This is not Windows COM. It has no registry activation, apartments, `HRESULT`,
or operating-system marshalling.

## 2. Interface identity and calls

Every generated interface has a Rust marker, a `#[repr(C)]` method table,
an exact GUID/signature identity, and separate inspection descriptions. The
marker has no instances in object storage. Raw references are explicit C records:

```c
typedef struct NcRawInterface {
    void *object;
    const IUnknownVTable *vtable; /* typed tables share this prefix */
} NcRawInterface;

struct IUnknownVTable {
    const NcInterfaceId *interface_id;
    NcStatus (*query_interface)(void *object, const NcInterfaceId *id, NcRawInterface *out);
    uint32_t (*add_ref)(void *object);
    uint32_t (*release)(void *object);
};
```

Rust names the corresponding generic record `ComRaw<T>`. The object pointer is
the allocation's identity and stays unchanged across queries. The table selects
methods for that exact concrete implementation. Ordinary method receivers take
`void *object`; borrowed interface arguments pass pairs by value and owned
interface results write pairs into caller-provided output records. Tables
contain no owner offsets, and objects contain no embedded interface headers.

A present pair has two non-null pointers. Absence has **both** pointers null.
A half-null pair is malformed and checked adoption returns `NC_E_POINTER`
before reading either pointer. Nullable raw records are distinct from safe
`Option<ComPtr<T>>`; the latter's Rust niche is not the foreign representation.
Safe owners, borrows, and their optional forms occupy two pointer words.
`ComRaw` copying never acquires ownership by itself.

Tables have the common prefix above. A derived table can embed its first base's
full method table at offset zero. All nested prefixes advertise the actual
outer interface's identity. A borrowed inherited method view keeps the original
pair and uses compatible prefix slots. It is not an owned base handle; acquiring
one requires QueryInterface and its exact table. This design preserves static
inherited slots; it does not claim to deduplicate their entries.

A successful query returns the same object address and the exact requested
table, with one acquired reference. Failure clears both output words and
transfers no reference. Typed wrappers validate the returned contract and reject
object substitution. Comparing `object` tests allocation identity; comparing both
fields tests the selected view. Table addresses alone are not contract identities:
GUID and signature are authoritative. Implementations may share or duplicate
identical static tables without changing those contracts.

Core generation 10 is a breaking change from generation 9's one-pointer handles
and embedded headers. Runtime/plugin discovery rejects other core generations
before dispatch. Interface signatures and the separately versioned discovery
records do not replace this global layout check. See the
[storage decision](../../../docs/COM_INTERFACE_STORAGE.md) and `ComPtr` rustdoc
for why the larger handle is intentional and how its costs must be measured.

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

`.ncidl` supports `struct`, `enum`, and `bitenum` values plus callable
`interface`, `class`, and `factory` roles. All records/enums use one recursive
value model; there is no `#[message]` category. Every declaration/member is
public; private fields belong to implementation objects. `#[version(N)]` derives
an IID; `#[id(N, "UUID")]` pins it.

`ComValue` defines an application type's separate raw `Wire` and concrete
producer `Storage`. Booleans lower to bytes checked for 0 or 1. Enum decoding
matches an integer tag before reading only its selected recursively lowered
payload. Records, optional fields, arrays, and sequence elements follow the
same conversion. `ComInlineValue` additionally proves self-contained copied
wire storage; generated copied slots and buffers require that proof. Merely
implementing `AbiType`, `AbiPlainValue`, or having `repr(C)` is insufficient.

Borrowed named values use `NcValueRef<'_, T>`. Passing tag
`NC_ABI_PASSING_BORROWED_VALUE` (`0x07`) identifies a pointer to the exact raw
record/union by its aggregate signature. `NcValueRef::new(&value)?` retains typed
producer storage; `to_owned()` recursively validates and acquires contents.
Sequence fields own `Vec<T>` in application Rust. Inline cycles, including
`Option<Self>`, fail IDL resolution; explicit sequence indirection permits finite
recursive trees. Raw memory validity remains the caller's obligation.

One shared conversion budget bounds temporary/acquired storage to 8 MiB, visited
values to one million, and depth to 64. Allocation is fallible. Converted buffers
use the same checked codecs; numeric buffers may remain direct borrows only with
an explicit proof of recursively identical layout and unrestricted contents.
Direct borrows allocate and traverse no elements, so these conversion budgets do
not cap their length. Operation-specific limits still apply. The complete
[recursive value contract](../../../docs/RECURSIVE_VALUE_CONTRACT.md) specifies
supported use sites, budget accounting, raw storage obligations, and failure
semantics. `ComStruct::ABI_STRUCT_LAYOUT` describes the application C-layout
record; the method's actual raw type is `ComValue::Wire`, which can differ.

The first required interface is the physical base; later requirements are
sibling projections. The required projection graph must be acyclic. A factory
may serve as the runtime's zero-argument construction root. Ordinary generic
interfaces are specialized before ABI generation. `IMessageSink<E>` crosses COM
as a concrete post vtable with exact identity and signature. `EventBinding<E>`
is an ordinary enum passed into construction; its observed case owns that sink.
The product retains one lifetime receiver before publication, and `EventPort`
provides terminal closure. No attachment interface is added to the product.
The receiver acquires the event before mapping it to the behavioral parent's
queue; later application dispatch uses mutable parent access. Visual widget
routing is outside this contract.

Hand-authored Rust COM contracts can use nullable `Option<ComRef<I>>` inputs,
identified by `NC_ABI_PASSING_OPTIONAL_COM_REF` (`0x08`). Current IDL does not
emit that shortcut: `&Option<ComPtr<I>>` borrows an ordinary tagged library enum.
Its active COM payload uses the standard checked object reference codec. See the
[generic value contract](../../../docs/IDL_GENERIC_VALUES.md).

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
table. Plugin descriptor ABI also remains v1; core object ABI remains v8.
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
stream, currently encoding revision 3. Encoding revision 3 distinguishes checked recursive wire lowering from native
aggregate slots. Core version 10 additionally selects the object/table handle
and receiver convention; both checks apply. This representation is unconditional across hosts, plugins, inspection,
and monolithic builds. Runtime calls compare identities and exact signatures.
Declaration names and versions participate in the build-time identity and
fingerprint inputs. A plugin's qualified name is also validated against its
derived plugin ID; its release-version text is diagnostic and does not select
a compatible implementation.

## 10. Ownership and trust

Description pointers and static UTF-8 views are borrowed from retained native
libraries or runtime storage. Inspection callbacks must copy values they need
after the call. Successful activation transfers one COM reference to the host. The bootstrap's
`ComRaw<IUnknown>` is an erased **factory** pair; erasure preserves its actual
factory table. The runtime returns the validated factory projection. A typed
factory client adopts that pair directly; a caller needing canonical IUnknown
queries it explicitly. Casting the erased pair to a safe IUnknown owner would
incorrectly certify its table identity.

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
