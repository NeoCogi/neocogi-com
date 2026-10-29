# COM ABI v1

This is the single current COM binary contract. The authoritative layouts are
the public `#[repr(C)]` definitions in [core](../crates/core/src/lib.rs); this document
describes how their calling, ownership, runtime and deployment rules compose.

COM is pre-release. Core, runtime-table and plugin-descriptor discovery all
currently report version **1**. Layouts and behavior may change in place before
ratification: rebuild the runtime, host, plugins and generated contracts from
the same checkout. Version 1 is not a promise of compatibility with an older
checkout that also reported 1. There are no compatibility adapters for retired
experimental layouts.

| Identity or version | Current authority and purpose |
| --- | --- |
| Core ABI | `NC_COM_CORE_ABI_VERSION == 1`; object/lifetime/descriptor layout and behavior |
| Runtime ABI | `NC_COM_RUNTIME_ABI_VERSION == 1`; versioned C table discovery |
| Plugin descriptor ABI | `NC_PLUGIN_DESCRIPTOR_ABI_VERSION == 1`; native getter and descriptor prefix |
| Interface contract | IID plus exact 32-byte ABI signature, checked at connection boundaries |
| IDL fingerprint encoding | `neocogi_idl::MODULE_ABI_ENCODING_DOMAIN`; compiler-owned canonical bytes, currently encoding v13 |
| NCB format | Version 1 positional persistence encoding, separate from live COM wire values |

The IDL encoding revision is not a COM ABI generation. Naming this contract v1
does not reassign existing IIDs, built-in signature bytes, IDL fingerprints or
application catalog schemas. Ordinary IDL signatures continue to describe exact
transitive contracts; core discovery checks the enclosing binary contract.

Shared `.ncidl` files stay under their owning subsystem. Each consumer may
compile them into its own `OUT_DIR` with `cargo::SharedSources`, adding explicit
subsystem roots with `include_root`. No dedicated contract package is required.
Independent Rust definitions retain the same logical IDs and canonical ABI;
ordinary Rust helper calls still use the helper library's concrete Rust types.
This source/build ownership change adds no registry or runtime and changes no
ABI version. The host supplies access to its one runtime registry to all plugins.

## Scope

The boundary uses fixed-width values, explicit C-layout records and `extern "C"`
function pointers. `NcStatus` is the result protocol; GUID/signature pairs select
interfaces. Rust trait objects, native enum/container layout, slices, arbitrary
references, allocator-owned `String`/`Vec`, implementation types and unwinding
do not cross the boundary. Recursive application values use checked raw records,
integer tags and call-scoped pointer/count storage. A receiver acquires any
contents it retains.

This is a component ABI with its own registry and lifetime protocol; it supplies
no Windows COM activation, apartments or operating-system marshalling. The one
global native plugin registry belongs to the executable-owned `neocogi_com_host`. Object storage and
caller-owned persistence contexts retain their separate responsibilities.


## Interface identity and calls

Every generated interface has a Rust marker, a `#[repr(C)]` method table,
an exact GUID/signature identity, and separate inspection descriptions. The
marker has no instances in object storage. Raw references are explicit C records:

```c
typedef struct NcWeakRaw NcWeakRaw;
typedef struct IUnknownVTable IUnknownVTable;

typedef struct NcRawInterface {
    void *object;
    const IUnknownVTable *vtable; /* typed tables share this prefix */
} NcRawInterface;

struct IUnknownVTable {
    const NcInterfaceId *interface_id;
    NcStatus (*query_interface)(void *object, const NcInterfaceId *id, NcRawInterface *out);
    uint32_t (*add_ref)(void *object);
    uint32_t (*release)(void *object);
    NcStatus (*downgrade)(void *object, NcWeakRaw *out); /* nullable capability */
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

Discovery checks core ABI v1 before dispatch. Exact interface signatures do
not replace that global layout/behavior check. See the
[storage rationale](contracts/COM_INTERFACE_STORAGE.md) for the
object/table design and its measurement boundaries.

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

When generation selects an external Rust definition, it emits a compile-time
check of that actual type's identity against the resolved declaration. Module
constants cannot establish which types an ordinary facade reexports. Inherited
clients and physical table prefixes come from the checked ABI marker's associated
types, so a separately reexported companion cannot change their representation.
These checks add no runtime discovery or registry state; the
[IDL generation contract](contracts/IDL_MODULES.md#shared-idl-sources)
describes their scope.

`QueryInterface` matches both GUID and ABI signature. A GUID collision or a
stale binding therefore cannot silently produce a typed pointer. A successful
query returns one owned reference. `ComPtr` calls `AddRef` on clone and
`Release` on drop; `ComRef` is borrowed and changes no count.

`Result<(), NcStatus>` lowers to a status-only call. Other successful results
use an output pointer that remains unwritten on failure. Expected failures must
be negative `NcStatus` values. Panics must not cross an `extern "C"` boundary.

Mutable IDL methods lower from `&mut self`. Generated object storage enforces an
exclusive call gate and reports conflicting/reentrant access as `NC_E_BUSY`.

## Signature storage and absence

```c
typedef struct NcAbiSignature {
    const uint8_t (*digest)[32];
} NcAbiSignature;
```

Rust uses `#[repr(C)] struct NcAbiSignature { digest: *const [u8; 32] }`.
This is a thin pointer, not a Rust slice pointer. Null explicitly means no
contract where the enclosing record permits absence. A root factory must have a
present signature; absence returns `NC_E_INVALID_ARG`. An all-zero digest remains
a present digest and must never be interpreted as absent or unchecked.

Generated signatures point to immutable provider-local static arrays. Modules
stay pinned while metadata/objects can refer to them. Synchronous interface
queries may borrow a caller's `[u8; 32]` for the call only; no provider may retain
that requested identity pointer. Contents determine equality and hashing, never
addresses. Separate libraries can store the same digest at different locations.

The public static constructor accepts only `&'static [u8; 32]`; incorrect widths
fail compilation. `EMPTY` supplies explicit absence. The unsafe `as_digest`
accessor returns `Option<&[u8; 32]>`; it neither reads a foreign length nor invents
an empty byte slice. Foreign producers must still guarantee readable immutable
storage for all 32 bytes. Pointer shape does not validate arbitrary native memory.

The signature view occupies one pointer word, while backing storage remains
32 bytes. It is not an inline digest or an owning pointer. The checked-in
built-in digests and compiler-emitted declaration digests are fixed contract
identities; comparing their pointer addresses would incorrectly distinguish
equal contracts stored in different libraries.

The public static constructor makes digest width a compile-time obligation:

```rust
use neocogi_com::{NcAbiSignature, NC_COM_CORE_ABI_VERSION};
assert_eq!(NC_COM_CORE_ABI_VERSION, 1);
static FIRST: [u8; 32] = [7; 32];
static SECOND: [u8; 32] = [7; 32];
let a = NcAbiSignature::from_static(&FIRST);
let b = NcAbiSignature::from_static(&SECOND);
assert_eq!(a, b); // Content equality across separate backing declarations.
assert!(!a.is_empty());
assert!(NcAbiSignature::EMPTY.is_empty());
```

```compile_fail
use neocogi_com::NcAbiSignature;
static WRONG_WIDTH: [u8; 17] = [0; 17];
let _ = NcAbiSignature::from_static(&WRONG_WIDTH);
```

## Weak lifetime references

`IUnknownVTable.downgrade` is optional. Absence reports `NC_E_NOINTERFACE`;
success transfers one retained shared weak token at the canonical allocation
address. Failure clears its output. Handwritten providers opting in implement
the common header and lifetime protocol, not an opaque Rust Weak token.

```c
typedef struct NcWeakControl NcWeakControl;
typedef struct NcWeakRaw { NcWeakControl *control; } NcWeakRaw;
```

The [common lifetime engine](../crates/core/src/lifetime.rs) defines `NcWeakControl`
in this field order: `state: AtomicUsize`, `weak: AtomicUsize`, and
`class: *const NcClassLifetime`. Its immutable class metadata contains provider-local
`destroy_value` and `deallocate` C callbacks followed by the interface-table
pointer array and its count. Foreign implementations match target atomic
width/alignment and synchronization; no Rust Arc representation is inspected.
There is no per-handle weak operation table. `nc_weak_retain`, `nc_weak_release`
and `nc_weak_upgrade` are C-callable common operations, without a promise of
unmangled dynamic exports. The native C fixture receives their addresses explicitly.

A shared `WeakComPtr<T>` owns one weak reference and occupies one pointer word.
Cloning changes only weak ownership. Promotion acquires a strong owner before
selecting the exact requested interface, and never resurrects a destroyed value.
Failure clears both strong output words; contract negotiation errors remain errors.
An expired value reports `NC_E_DROPPED`, mapped by Rust `upgrade()` to `Ok(None)`.
Canonical control addresses remain unique while weak owners prevent storage reuse.

Objects are constructed from completely initialized values. Generated construction
publishes one shared strong owner or the exclusive owner bit, plus an implicit
weak storage reference. There is no cyclic initializer, unpublished callable
object, or rollback state exposed by construction. Obtain weak identities after
construction while an owner is available.

Logical closure is separate from destruction. A closed service retained by a
strong owner can still be promoted; its methods enforce closure. Owned queued
facts remain readable after their weak source expires.

## Allocation and borrowing

`NcWeakControl` is the common header, at offset zero of one provider allocation.
Its first atomic word counts strong references for shared objects. Exclusive
objects instead use the highest bit for exclusive mode, the next bit for owner
presence, the next bit for mutable borrowing, and the remaining bits for readers.
Exclusive mode remains set after destruction. The other header words are the weak
count and a pointer to immutable class metadata. That metadata is shared by all
instances of a class and supplies provider-local destruction and allocation release.

Generated exclusive storage places `UnsafeCell<MaybeUninit<T>>` after the header.
It contains no separate call gate. Generated shared storage retains its separate
borrow gate because its header word counts strong owners. Both policies produce
a two-word provider-local lease: a typed `UnsafeCell<MaybeUninit<T>>` slot
reference and one selected guard. The lease creates `&T`/`&mut T` only for
short guarded reborrows. It must not store a T reference across admission release:
passing a reference-bearing lease into a drop function can protect that reference
until after the guard unlocks. A newly admitted writer or waiting exclusive-owner
destructor may then access T immediately. Retained slot storage outlives the
complete lease/drop call; `MaybeUninit` permits T to be destroyed within that
interval once its last value loan and admission have ended. Threading capability
still requires the corresponding T, guard and owner-mode proofs.
Shared entry requires the caller to retain a strong owner throughout the call;
the shared borrow gate does not acquire a replacement ownership reference.

Exclusive objects have one `ComBox` owner and cached `WeakComBox` observers.
Their shared `AddRef` slot returns zero without retaining; shared query and
shared weak downgrade reject with `NC_E_INVALID_ARG`. Borrowed references cannot
silently grant another owner: use fallible `ComRef::try_add_ref` when ownership
policy is not already proven shared. Consuming exclusive queries preserve the
one owner; borrowed `query_ref` and weak observation do not duplicate it.

A reader atomically checks owner presence, excludes the writer and increments the
reader count. A writer atomically checks owner presence and zero borrowers before
setting its bit. Conflicts and reader-count exhaustion return `NC_E_BUSY`;
expired objects return `NC_E_DROPPED`. These checks precede creation
of any Rust reference. Acquire admission pairs with release of prior borrows and
construction publication. Reader release subtracts one; writer release clears only
its bit, preserving concurrent owner removal. Exclusive borrow release never
destroys the value; the owning thread completes synchronous destruction.

## Drop and weak storage

Every successful shared retain records exactly one additional owner. This applies
to generated objects and to strong-only bytes, strings and persistence child
lists. An exhausted counter terminates the process through the non-unwinding
`extern "C"` entrypoint before returning an owner, in every build profile.
`Clone`/`AddRef` have no recoverable exhaustion status. A zero private count also
cannot be retained or resurrected; raw callers must keep valid storage owned
throughout the operation.

Generated shared counters use the common pointer-width header; strong-only
providers keep their existing `AtomicU32` and have no weak-control allocation.
The last available count is usable and may later be decremented normally.
Saturating the **actual** counter while accepting more owners would undercount
later releases and permit premature destruction. A capped diagnostic `u32`
return is allowed only when the wider private counter still records the retain.
This policy does not change any handle, method table or core ABI version.

Exclusive owner Drop clears owner presence with an acquire/release operation, rejecting all
new calls. It waits for existing readers/writers using acquire observations, invokes
the provider's destructor exactly once, and releases its implicit weak storage
reference. Drop returns only after value destruction, including ordinary owned
child fields. The wait uses spin hints; std-enabled builds also yield to the
scheduler. Completion requires admitted calls to terminate: this is not wait-free.

The implicit weak reference lasts through the destructor, protecting storage when
T drops a weak-to-self field. The last external weak reference frees allocation
storage through the original provider and never drops T again. Provider code and
metadata must remain loaded until that final weak release.

An exclusive weak handle's `as_com_ref` borrows retained interface storage,
not T. Each method still acquires its own checked access, so keeping an ABI
argument view cannot prolong the value's lifetime. Liveness and idle observations
are snapshots, never access permission. Cached interface queries can inspect
retained class metadata after value destruction. No weak reference upgrades to
an exclusive owner. Shared weak handles instead use `upgrade()` to acquire a
shared strong reference.

An executing method must not destroy its own owner, directly or by removing an
ancestor; waiting would deadlock. Nor may the dropping thread hold resources
needed for an admitted call to finish. These are ordinary blocking synchronization
constraints, not permission to defer destruction.
Explicit ThreadSafe objects additionally require Send + Sync
implementations and permit destruction on the dropping thread.

## Consuming arguments

The wire argument is `*mut ComRaw<I>`: writable call-scoped storage containing
one exclusive owner. On successful return the slot is null. On **every** negative
status it contains exactly its original object/table pair. For multiple owning
inputs, all transfer together or all return, in declaration order. Raw callers
must provide distinct, properly aligned slots and valid unique owners; forged
pointers and duplicate ownership tokens are not made safe by slot validation.

The Rust provider and client use ordinary `ComBox` values:

```text
fn insert(&mut self, child: ComBox<IWidgetAbi>)
    -> Result<(), (NcStatus, ComBox<IWidgetAbi>)>;
```

IDL still spells this `fn insert(&mut self, child: ComBox<IWidget>) -> Result<()>`.
It generates the error tuple from the owning parameters. For two inputs, the Rust
error is `(NcStatus, ComBox<A>, ComBox<B>)`. No `ComBoxInput` or transfer-state enum
is exposed. The macro rejects incomplete or reordered error tuple signatures.

The generated thunk validates every argument and admits the receiver before it
adopts any input. It clears the raw slots, invokes the provider, releases the
receiver borrow, then restores returned owners on failure. A forwarding provider
propagates the complete error; it must not silently discard or substitute owners.
Safe clients check the success/failure handshake before recovering Rust ownership.

Providers finish fallible validation and resource preparation before ownership
commit. Failure leaves input values and destination topology unchanged. After
commit they return a valid successful value. Invalid output after a successful
commit is a **provider contract violation**, not a recoverable rejection: those
input owners are no longer available to return. Generated code panics on this
violation (a panic inside an extern-C provider aborts). Non-owning methods retain
ordinary checked-output status errors. This is not an exception allowing a provider
to lose an owner on an ordinary application or resource error.

Generated adapters finish fallible preparation before adoption. Copied consuming
results require a finite allocation-free `ComValue::MAX_COST`. Converted output
buffers prepay their complete finite capacity before invocation; conversion uses
reserved storage and publishes only after every output succeeds. Input-only
values charge actual work. Unavailable required maxima reject the declaration;
known unaffordable work returns `NC_E_OUT_OF_RANGE` with original owners intact.
See [prepared completion](contracts/COM_OUTPUT_COMPLETION.md)
for exact ordering and resource accounting.

The caller decides whether to retry or destroy returned inputs. In particular,
rejecting ancestor A offered to descendant B returns A through every forwarding
call before application code can drop it. This requires no deferred-destruction
queue. Destroying one's own active owner remains forbidden by the synchronous
Drop contract; ownership-taking providers must validate before accepting it.

`ComBox::query` and `LocalBox::into_interface` likewise return the original owner
on failed projection. Private builders that create fresh, unpublished candidates
may explicitly discard a returned owner when abandoning their own construction;
that is a caller decision, never automatic error conversion in COM.

A construction-only helper can select `IWidget` once and keep that projection
through insertion and disposal. Restoring the original concrete interface just
before Drop is unnecessary. A consuming `ComBox::query` of the same exact
interface reuses its already-validated table; matching includes the ABI signature,
not just the GUID. Different revisions and interfaces still take checked lookup.
This changes neither the wire protocol nor ownership/borrow state.

## Construction access

`LocalBox` borrows are tied to its unique owner, so Rust prevents moving that owner
while its local guard lives. `WeakLocalBox::with` and `with_mut` provide scoped
provider-local construction access without exporting implementation references or
guards. Their closures must obey the same non-recursive-destruction rule as methods.
No implementation pointer, Any payload, or foreign downcast is exposed by safe APIs.
Shared `LocalCom<T>` and its weak local counterpart likewise preserve typed
construction proof in Rust. Foreign COM projections cannot recover a concrete
implementation or fabricate that proof.


## Threading capabilities

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

## Contract declarations and checked values

Unit data has the existing Aggregate descriptor tag and the SHA-256 digest of
the exact sixteen bytes `neocogi.unit:v1\0` (hex
`6e656f636f67692e756e69743a763100`):
`88895ec8e34f7f3c989d6c940d69ec3951658e689401ce5dfa796cd06ffe0891`.
This core-owned intrinsic differs from U8 and Void. Its checked wire is one byte,
valid only as zero; its native Rust layout is zero-sized and grants no direct
wire borrowing. Arrays include this element identity even at length zero.
Named unit-containing results have output storage; root `Result<()>` remains
status-only. Canonical IDL Unit tags, closed-type identity encoding and structural
Unit metadata retain their existing encodings.

IDL supports public `struct`, `enum`, `bitenum` and ordinary `interface`
declarations. Construction and observation are ordinary interface methods;
concrete classes and plugin acquisition are Rust implementation concerns.
The [IDL reference](contracts/IDL_MODULES.md) owns accepted syntax,
identity and canonical encoding. Special class/factory/event IDL roles are absent.

`ComValue` supplies separate raw `Wire` and concrete producer `Storage` types.
Boolean wire fields are bytes checked for 0 or 1. Payload enums decode an integer
tag before reading only its selected lowered payload. Records, options, arrays
and sequences use the same recursive conversion. `ComInlineValue` additionally
proves self-contained copied wire storage; native `repr(C)` alone is insufficient.
No application-memory layout table or full source reflection graph is published
by physical metadata. Binary-local `idl_metadata` records instead carry static
Rust names/digests and bound method/value facts; a domain owns any transported catalog.

Named borrowed inputs use `NcValueRef<'_, T>`; passing tag
`NC_ABI_PASSING_BORROWED_VALUE` (`0x07`) means a pointer to the exact wire aggregate.
`NcValueRef::new(&value)?` retains producer storage for the call, and `to_owned()`
validates/acquires retained contents. The receiver's Rust declaration uses the
anonymous call lifetime for borrowed COM/value/buffer wrappers. Static application
data does not make temporary encoded arrays static. Retain acquired owners,
never the borrowed pointers. Optional COM borrowing has its own raw passing tag;
ordinary IDL optional capabilities use their tagged value representation.

One shared conversion budget bounds temporary/acquired storage to 8 MiB,
one million visited values and depth 64. Allocation is fallible. Recursive
identity-wire proofs permit unrestricted numeric/direct buffers without per-element
conversion or allocation; their lengths are not capped by hypothetical conversion
work. Operation-specific bounds still apply. Inline cycles fail resolution;
sequence indirection permits finite recursive values. See the
[recursive-value contract](contracts/RECURSIVE_VALUE_CONTRACT.md).

Built-in data interfaces exercise shared ownership and checked native calls:

```rust
use neocogi_com::{INcBytesAbi, INcBytesComPtrExt, INcStringComPtrExt, IUnknown, NcString};
let text = NcString::from_utf8_str("ABI v1");
let another = text.clone();
let bytes = text.query::<INcBytesAbi>().unwrap();
assert_eq!(bytes.to_vec().unwrap(), b"ABI v1");
assert_eq!(another.to_string().unwrap(), "ABI v1");
let identity = text.query::<IUnknown>().unwrap();
assert!(identity.as_com_ref().same_object(bytes.as_com_ref()));
drop(text); // Other strong owners still retain the immutable value.
assert_eq!(another.to_string().unwrap(), "ABI v1");
```


## Integer metadata tags and Rust enums

Physical metadata uses fixed-width integer fields: `NcAbiTypeDescriptor.tag` is
a `u32`, as are the passing and threading encodings in `NcAbiPassingDescriptor`.
The `NC_ABI_TYPE_*`, `NC_ABI_PASSING_*` and related constants name their accepted
encodings. `NcAbiType`, `NcAbiPassing`, `NcAbiThreadingMode` and `NcAbiReceiver`
are local Rust descriptions converted explicitly to or from those raw values;
their native enum layouts are not shared-library metadata contracts.

Integer storage allows a reader to inspect and reject an unsupported code.
For example, type tag `0x01` denotes `bool`, while reserved tag `0x10` must be
rejected by a reader interpreting the type. Both are representable as initialized
`u32` values. The signature formatter matches the raw codes and returns
`NC_E_INVALID_ARG` for an unsupported tag.

A Rust enum may contain only a declared discriminant. `#[repr(u32)]` fixes its
representation but does not make other integers valid enum values. Receiving
an invalid enum and then trying to reject it in an ordinary match is already
undefined behavior; see the [Rust validity rules](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).
Read the raw integer first and use checked interpretation or conversion, never
transmutation of an unchecked tag.

Flat constants are an API organization choice. Known encodings could instead
have namespaced enum variants with checked integer conversion; the physical
record would still need storage that permits raw-value validation. This rationale
does not require backward compatibility or acceptance of unfamiliar tags.

## Raw output parameter

A logical output `T` becomes four raw arguments in this order:

```c
uint64_t offset,
uint64_t capacity,
TWire *elements,
uint64_t *written
```

`TWire` is the existing generated C-layout copied wire record, not necessarily
native `T`. Boolean fields are `uint8_t`; payload enums have an explicit tag and
selected wire payload. The logical descriptor passing tag is
`NC_ABI_PASSING_OUTPUT_BUFFER` (`0x0c`). A method's ordinary result slot, if any,
is separate. For example, a snapshot getter can return its *required total size*
while `written` is zero because supplied capacity was insufficient.

The raw caller supplies valid, aligned, exclusively writable capacity plus a
separate count slot. Regions and different output arguments must not overlap.
Zero capacity permits a null element pointer but still requires a count pointer.
Old element contents need not be initialized; the provider must never read them.
On success, `written <= capacity`, and every active field in that prefix is
initialized. Padding, inactive payloads, and the tail are never inspected.
On error no prefix is published; raw bytes may have been modified and must not
be treated as results. Pointer checks cannot prove arbitrary memory validity.

## Typed views and ordering

Provider declarations use `NcOutputWriter<'_, T>` for self-contained
`ComInlineValue` elements. `push` encodes one native value into its raw slot;
`extend_from_slice` preserves bulk copy for the recursive identity-wire proof.
Writers cannot read old values, allocate conversion vectors, or retain storage.
`forward` lends an untouched writer to another synchronous generated client
using the same slots; the nested result is validated before its count propagates.

Clients use `NcOutput<'_, T>` from `NcOutputBuffer<T>::output(offset, capacity)`.
Acquiring the view immediately discards previous results, even if preparation
never reaches it or the view is abandoned. Retained capacity is unchanged.
Converted results retain reusable native and wire allocations locally. Identity
results use only native allocation; `NcOutput::from_slots` also permits
allocation-free `MaybeUninit<T>` storage with the identity proof. No Rust writer,
Vec, allocator ownership, or native enum representation crosses the boundary.

Every output is admitted and all needed caller capacity is reserved before any
owning argument is transferred. Provider thunks validate writable storage and
admit all output shapes before adopting owners; they require no output scratch.
Converted outputs prepay the whole capacity's finite maximum byte/value/depth
cost, including non-consuming calls. Input-only values retain actual-input
validation. Consuming copied returns additionally require finite maximum cost.
Thus valid successful output cannot exhaust conversion allocation or budget.

Thunks publish counts only after implementation success and result encoding.
Clients check counts, validate restricted fields into their reserved native
storage, acquire *every* output, then publish the complete group. Any failed call
leaves no new native results, even if an earlier output had already validated.
This does not undo provider application mutations or restore previous snapshots.
Successful malformed output after owner acceptance remains a contract violation;
it cannot return owners the implementation has already stored.

Readable input buffers remain `NcCopyFromBufferOffset<'_, T>`. A transformation
must take explicit input and separate output. There is no generic compatibility
adapter retaining initialized input/output semantics. The obsolete wrapper and
its encoded/decoded output guards have been removed.

## Shared weak result and value transport

`WeakComPtr<I>` results transfer one `NcWeakRaw` through a writable output slot.
The raw record is the existing single common-header pointer, without a cached
interface table. `NC_ABI_PASSING_WEAK_COM_PTR_OUT` (`0x0d`) identifies this passing
mode; the return type descriptor supplies the exact interface identity and the
passing descriptor preserves the threading capability. It is distinct from the
two-word exclusive `WeakComBox` output (`0x0b`).

A successful provider transfers one retained weak token. The client rejects null
tokens, exclusive ownership, missing interfaces and wrong interface revisions,
releasing a rejected transferred token exactly once. Errors transfer no result.
Expired shared tokens remain valid: checking reads immutable
interface metadata under weak ownership and never promotes or reads the value.
Raw callers must supply a retained valid header, readable class metadata and
provider code lifetime; checks cannot make an arbitrary pointer safe.

A weak field in a borrowed value uses the same `NcWeakRaw` as a borrowed token.
The source handle keeps its header alive throughout encoding and acquisition.
Decoding validates before retaining an independent weak reference. Partial
aggregate failure drops already acquired fields. It never creates a strong owner
and is not a copied inline value or an implicit persistence representation.

IDL assigns shared weak ownership its own fingerprint tag. Metadata readers
reject unsupported passing categories. `WeakLocalCom<T>`
is construction-proven Rust access layered on this existing shared lifetime
protocol; its concrete T and native wrapper do not cross the ABI.

## Fixed byte-copy primitives

`INcBytes::copy_to` and `INcString::copy_to` retain their exact-range
`offset, length, pointer` slots and `NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET` (`0x04`).
These built-in contracts copy a prescribed number of unrestricted bytes or fail.
They have never interpreted old bytes as input, need no enum conversion, and do
not represent the removed generic mutable-value path. IDL and its typed backend
output parameters always use the new four-slot protocol.

See [directional output](contracts/COM_OUTPUT_BUFFERS.md),
[ownership completion](contracts/COM_OUTPUT_COMPLETION.md) for examples
and regression requirements.

## Compile-time modules

A module's build script resolves its transitive source graph and emits Rust
bindings with fixed declaration signatures. Contracts have no native libraries,
module-description getters, runtime registration tables, or builtin seeding.
Cargo `links`/`DEP_*` graph metadata remains a build-time dependency protocol.

The signature covers the complete transitive module revision, preserving exact
compatibility across independently built hosts and plugins. Different revisions
may coexist while disconnected; factories and COM pointer transfers reject an
incompatible connection. Registration no longer enforces one module revision
per process.

## Runtime and global registry

Native Rust loading and owned inspection reject an interior NUL before entering
the platform loader. Windows checks the original native UTF-16 path before resolving it;
Unix checks its byte path before dlopen. Both report the existing OpenLibrary
phase. The facade's UTF-8 path adapter retains its invalid-argument rejection.
Successfully opened native mappings remain process-pinned.

On Windows, owned inspection, callback inspection and required loading share one
opener. It rejects empty paths, qualifies the supplied filesystem path once with
`std::path::absolute`, and calls `LoadLibraryExW` with
`LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32`. There is no
permissive retry or process-global search-directory change. Bare and relative
paths select files relative to Windows' native path resolution; a bare name is
not a request for PATH discovery. Ordinary imports for this open search the
selected DLL's directory and System32, subject to normal Windows module reuse
and redirection. Deploy private import dependencies beside the selected DLL.
Application, user-added, current-working and PATH directories are excluded from
this per-open search. The original caller spelling remains the diagnostic path
and first-registration provenance; the temporary absolute path is not catalog state.

Qualification does not canonicalize links, authenticate files or bind a checked
file handle to an image. Later provider-issued `LoadLibrary*` calls and delay
loads do not inherit these flags. Providers and their native dependencies remain
trusted code. The low-level inspector accepts native `OsStr` paths; SDK UTF-8
adapters retain their documented encoding restriction. Missing or unsupported
native loads report `OpenLibrary` without broadening the search.

### One host, supplied access

One authority owns plugin GUIDs, dependency validation, registration order, first
provenance and pinned native code. The executable links `neocogi-com-host`
(the Rust library `neocogi_com_host`). Its normal `#[com_main]` entry initializes that
implementation once and passes a copyable `Runtime` to application code.

Every provider bootstrap receives access to those same immutable host tables.
A context-free callback in a table executes in the host and reaches its private
singleton. A nested acquisition from plugin A to B forwards the canonical host
bundle again. Handles own no registry and select no implementation; reusable
libraries take a handle from their caller. There is no global/default facade,
per-plugin cache, fallback, runtime DSO or named service getter.

The supported application has one ordinary startup. Additional independent
initialization requires an explicit path outside that composition; mere linkage
does not initialize state. No owner counter or Cargo/linker singleton detector
is part of the contract. Separate processes/Wasm instances have separate owners.

Plugin operations use one table; diagnostics and persistence keep independent
tables because their callback and caller-owned context lifecycles differ.

## Provider descriptor and registered information

`NcPluginDescriptor` is the provider-authored input: plugin identity and release
version, hard dependency records, the exact root factory interface/threading
contract, and its bootstrap function. `#[plugin]` derives the root
contract from the typed Rust acquisition return and creates this immutable descriptor.
The root is any closed ordinary interface: a finished service or a provider with
typed construction methods. Existing wire field names such as `root_factory`
describe that root and impose no IDL construction role.
The accessor and native export return metadata without registration or acquisition.

| Provider boundary | Current contract |
| --- | --- |
| Metadata input | `NcPluginDescriptor`; no separate plugin-kind qualifier. |
| Generated Rust accessor | `plugin_descriptor()` borrows assembly metadata. |
| Native getter | `NcGetPluginDescriptor`, exported as `nc_get_plugin_descriptor_v1` by `#[plugin]` under `dynamic-plugin`. |
| Validation | `validate_plugin_descriptor` checks metadata without acquiring a factory or checking whether providers are registered. |
| Static registration | `Runtime::register_plugin_descriptor` validates and publishes the descriptor after requiring registered hard providers. |
| Unregistered library inspection | `Runtime::with_plugin_library_descriptor` lends the validated descriptor to a synchronous visitor. |
| Registered inspection output | `NcPluginInfoV1` / owned `PluginInfo`, including first registration origin. |
| Factory acquisition | `Runtime::acquire_plugin_root` checks the exact contract and returns one COM owner. |

The descriptor's `description_abi_version` field remains the v1 discovery prefix,
matching `NC_PLUGIN_DESCRIPTOR_ABI_VERSION`. Its next field is
`core_abi_version`, currently 1. Both are checked before the remaining layout is
read. The two-word prefix must be checked before borrowing the complete descriptor.
Matching version numbers alone do not make different pre-release checkouts compatible.

The provider owns the descriptor's storage and code. Static registration requires
the entire graph to remain immutable for the process lifetime, including referenced
text, dependency tables and signature bytes. A static root reference alone does
not prove these raw-pointer lifetimes. Dynamic registration pins the exporting
library. Descriptor copies own neither those allocations nor a library mapping.
The native getter clears a usable output slot before validation, leaves it null
on failure, and returns `NC_E_POINTER` for null output.

`NcPluginInfoV1` is an output projected from a registered catalog record; it has
provenance but no bootstrap pointer. The facade's `PluginInfo` owns copies of text,
paths and dependency GUIDs. Neither output can be used to register a provider, and
neither holds a live factory reference. The catalog below remains the single
authority that combines the provider's descriptor with runtime registration state.

## One authoritative record

The catalog has one append-only record per registered plugin GUID and one
GUID-to-record index. A record retains one validated process-lived descriptor and
its first registration origin: static storage or an owned dynamic library path.
The descriptor supplies the ID, dependency table, name, release version and exact
single factory contract. No independent ID field or dependency projection Vec is
stored beside it. The actual origin is supplied before validation/publication;
dynamic loading never constructs a provisional static registration.

Lookup and enumeration use the same allocation-free projection of these records.
There are no parallel ID vectors, cached metadata summaries or module-to-plugin
joins. Enumeration returns complete records, removing the need for repeated
lookups to assemble plugin, factory and dependency information. Registration and inspection share this one authoritative record.

| `NcPluginInfoV1` field | Meaning and owner |
| --- | --- |
| `plugin_id` | Stable GUID derived from the qualified plugin name; acquisition and lookup key. |
| `qualified_name` | Borrowed immutable descriptor text; copied to `PluginInfo.qualified_name`. |
| `version` | Plugin release text, distinct from interface ABI signatures. |
| `library_path` | First successful dynamic caller path; empty denotes static origin. Provenance, without reopening a file. |
| `factory.interface` | Exact root factory IID and full ABI signature retained in static or pinned storage. |
| `factory.threading` | Valid raw `NcAbiThreadingMode`; the Rust facade returns the typed enum. |
| `dependencies` / `dependency_count` | Original immutable aligned `NcPluginDependencyDescriptor` array in declared order, borrowed directly from the validated descriptor. |

`NcFactoryInfoV1` contains only interface and threading information: the enclosing
plugin already supplies identity. There is one factory per plugin, so a separate
interface GUID or enumeration is unnecessary. Neither record contains a live COM
reference. Reading metadata never invokes the factory bootstrap.

All views returned by registered-plugin lookup and enumeration remain readable
for the process lifetime. Descriptor and dependency storage is static or pinned;
path buffers are immutable heap allocations. Catalog vector growth moves the
records, not that backing storage. The projection never points into a
movable record, stack temporary or temporary metadata copy. The caller may read
views after the catalog read guard is released.

The facade copies text, paths and GUID arrays into owned `PluginInfo` values.
Changing those values cannot change registration state. `FactoryInfo.interface`
retains the existing process-lived signature view rather than copying a contract
or acquiring a native handle. The raw ABI trusts valid aligned/readable pointer
storage; scalar shape checks are not arbitrary-pointer validation.

## Unified C table

The implementation also exposes `neocogi_com_host::initialize_process()` as an ordinary
Rust host entry. It initializes its existing singleton without provider calls
and returns an immutable `NcHostApiV1`: byte size, runtime/core versions (both 1),
and pointers to the runtime, debug and persistence tables. The complete service
graph and callback code remain process/instance-lived, including after main
returns. Copying this description does not create a registry.

`validate_host_api` checks required pointers, advertised byte sizes and matching
versions without invoking callbacks. It reads prefixes before complete layouts;
the unsafe caller still guarantees readable backing, valid callable tables and
coherent ownership. Validation retains nothing. Adopting references for process
lifetime additionally requires that lifetime for every reachable table/code
allocation; a static outer description alone proves no child lifetime. This is
part of pre-release ABI-v1, with no new ABI generation or identity changes.

The supplied record replaces named runtime symbols. All three tables are required:
`NcRuntimeApiV1` for plugin operations, `NcDebugApiV1` for diagnostics, and
`NcPersistenceApiV1` for caller-owned persistence contexts. The safe handle stores
private references only after size/version checks and under the unsafe caller's
complete process-lived graph guarantee. Unsupported versions and short records
return `NC_E_ABI_VERSION`; required null pointers return `NC_E_POINTER`.
Handle copies can cross threads. Unsafe adoption therefore also guarantees
concurrently callable host entrypoints; caller-owned persistence contexts keep
their own thread-local and exclusive-borrow restrictions.
The pre-release v1 layouts remain revision-specific: rebuild participants together.

| Runtime operation | Result and purpose |
| --- | --- |
| `register_plugin_descriptor` | Validate and atomically register a static descriptor. No factory acquisition. |
| `load_plugin_library_utf8` | Open a mandatory trusted plugin library, pin its code and register it; output its GUID. |
| `acquire_factory_plugin` | Check exact interface/signature/threading, call bootstrap and transfer one owned factory reference. |
| `plugin_info` | Write one complete borrowed record, or return `NC_E_NOT_FOUND`. |
| `plugin_count` | Snapshot number of all registered plugins, including static ones. |
| `plugin_infos` | Fill complete records in successful registration order under one read guard. |
| `inspect_plugin_library_utf8` | Open and validate an unregistered library, lending its descriptor to a synchronous callback. |

An unknown GUID is distinguishable from a registered plugin with no dependencies:
lookup returns `NC_E_NOT_FOUND` for the former and a complete zero-dependency
record for the latter. Lookup checks a null output first (`NC_E_POINTER`) and
writes nothing on failure.

## Enumeration protocol

The raw count is a snapshot, not a reservation. Registration can grow the catalog
between count and fill. Each fill acquires one read guard, fixes the ordered set
of records, projects every field and releases the guard before returning.

1. `out_len` must point to writable, aligned storage for one `usize`. Null returns
   `NC_E_POINTER` without writing any record.
2. Write the current required length to `out_len`.
3. If capacity is insufficient, return `NC_E_INSUFFICIENT_BUFFER`. No record is
   written, including when `out` is null. The caller may allocate and retry.
4. If the required length is zero, return `NC_OK`; null `out` is accepted.
5. Otherwise `out` must provide aligned writable storage for the advertised
   capacity. Null returns `NC_E_POINTER`. Data and length outputs must not overlap.
6. Write exactly the required prefix and return `NC_OK`. Spare slots are untouched.

A successful fill is one coherent metadata snapshot; later registration does not
change it. It does not snapshot live factory objects or their reference counts.
The Rust `plugins()` wrapper retains the existing bounded count/fill reader:
65,536 elements, at most eight strictly growing retries after the initial fill,
and fallible capacity reservation. Dependency copies use the same element bound;
individual text copies allow at most 1 MiB. Invalid raw scalar/count shapes return
statuses before allocation or dereferencing those arrays. These are facade
resource limits, separate from the dynamic-provider limit.

## Rust application API

Registered metadata is available directly on `Runtime`; no intermediate
`Inspection` or `PluginRegistry` context is needed:

The executable explicitly selects the host implementation in Cargo and uses:

```rust
#[neocogi_com::com_main]
fn main(runtime: neocogi_com::Runtime) {
    for plugin in runtime.plugins().unwrap() {
        println!("{} {}", plugin.qualified_name, plugin.version);
    }
}
```

Its manifest enables `neocogi-com`'s `runtime` clients and directly depends on
`neocogi-com-host`. The handle itself is available without `std` or `runtime`;
only richer clients require that feature. `com_main` supports safe synchronous
nongeneric `fn main(runtime: Runtime)`, preserving normal `()`/`ExitCode`/`Result`
returns, `?`, cfg and lint scopes. Explicit `neocogi_com = path, neocogi_com_host = path`
arguments support renamed dependencies. It adds no executor.
A mismatched selected implementation is a fatal startup configuration error.

The [factory walkthrough](COM.md) executes generated typed registration,
acquisition and ordinary construction. Tests may initialize their explicitly
selected host with `neocogi_com_host::initialize_process()` and unsafe checked adoption;
ordinary reusable libraries accept the resulting handle rather than initialize.

`plugin_info` returns `None` only for an unknown GUID. `plugins` returns owned
complete records in registration order; clients no longer enumerate IDs and then
perform three extra metadata queries per plugin. Applications can use the factory
contract to explain what the plugin supplies without creating a factory object.
The generated factory example in the COM/IDL documentation tests exercises actual
typed acquisition; catalog tests exercise these direct methods against C tables.

A generic host that does not know the factory's Rust type calls
`acquire_registered_plugin_root(plugin_id)`. The facade reads the registered exact
interface without allocating owned strings or dependency lists, acquires that
interface with a thread-local request and queries canonical `IUnknown`. An erased
raw factory table has the common lifecycle prefix but is not itself canonical
`IUnknown`; the query is required before typed adoption. The original bootstrap
reference is released on every path. The returned handle stays on the caller's
thread even if the provider supports shared threading.

## Acquisition and ownership

Acquisition means obtaining one owned root factory COM reference. The descriptor
stores `acquire: Option<NcAcquirePluginFactory>`; generated bootstrap code adapts
the implementation's typed construction policy to that fixed supplied-host ABI.
The raw bootstrap is
`unsafe extern "C" fn(*const NcHostApiV1, *mut ComRaw<IUnknown>) -> NcStatus`.
The authored `#[plugin]` function accepts one `Runtime` and returns its existing
exact typed `Result<ComPtr<FactoryAbi>, NcStatus>`. Its adapter clears output
before admitting host access; invalid access enters no provider code and
transfers no owner. `write_plugin_root` invokes an unboxed `FnOnce` only after
output preflight. Metadata discovery needs no host or acquisition.

A provider may create a fresh factory or retain an existing instance. The runtime
promises no singleton and no identity reuse between successful calls.

The runtime compares the requested IID, full signature and threading capability
before invoking bootstrap. It releases the catalog guard before plugin code runs,
then validates the returned object/table pair and its declared contract. Rejected
returned references are released. The raw output starts null and only success
transfers one owner. Typed facade adoption preserves the requested IDL factory
contract and threading model. Product construction occurs through factory methods.

Loading and metadata queries never acquire factories. Registration does not need
an application factory type and does not traverse the IDL contract graph. Exact
compatibility belongs to factory acquisition and subsequent interface transfers.

## Publication, provenance and repeated loading

Native open, symbol lookup, descriptor entry and validation run outside the catalog
lock. Registration validates the immutable descriptor, then takes one write guard to require
all hard providers, check identity/capacity and publish descriptor and origin
together. Failure leaves the catalog unchanged. Plugin code and user callbacks
never run while a catalog guard is held.

The same GUID and descriptor address reuse the first registration and consume no
extra capacity. A different descriptor claiming that GUID returns
`NC_E_INVALID_ARG`; matching name or version text alone does not make providers
interchangeable. Repeated static registration is idempotent. A later dynamic open
of the same already-static descriptor preserves its original static origin.

Native mappings remain pinned until process exit, including inspection-only opens
and failures after opening. Pinning belongs to the platform loader's never-unload
policy; dropping the symbol-resolution wrapper does not close a mapping. Catalog
provenance holds no optional native handle or handle-free test substitute.
Repeated loads still open and validate the library;
catalog idempotency does not suppress native loader calls or initialization.
Once registered, metadata and factory acquisition do not reopen the origin path,
so the plugin remains callable after its file is removed.

`NC_COM_RUNTIME_MAX_DYNAMIC_PLUGINS` bounds distinct dynamic registrations at
1,024. Static registrations, repeats and inspection-only opens are excluded. The
count derives from authoritative records. Inspection-only pins therefore remain
outside that bound; build tooling inspects in a short-lived process. There is no
unloading, path discovery, recursive dependency loading or an independently initialized runtime.

## Unregistered inspection and errors

`Runtime::with_plugin_library_descriptor(path, visitor)` remains unsafe: the
path selects trusted native code, and opening can execute platform loader code.
It validates a descriptor and invokes the visitor synchronously exactly once,
without registering the plugin, acquiring a factory or following dependencies.
The descriptor graph is borrowed only during the callback; copy needed metadata.
Do not retain descriptor pointers or panic across the C callback boundary. Once
opened, the mapping is pinned even if validation or the visitor later fails.

| Operation | Failure policy |
| --- | --- |
| Static registration | Ordinary validation/dependency/identity status; no partial publication. |
| Required live loading | Native loading, description, dependency, identity or capacity failures diagnose and abort. |
| Raw live-load arguments | Invalid pointers/path shape return statuses before loading; the application facade treats these as fatal too. GUID output is cleared to nil first. |
| Factory acquisition | Ordinary unknown-plugin, interface/signature/threading or bootstrap status. |
| Registered metadata | `None` for missing Rust lookup; raw `NC_E_NOT_FOUND`; copy/allocation errors remain statuses. |
| Unregistered inspection | Recoverable loader/validation/visitor status. Callback status is returned unchanged by C; Rust visitor errors are normalized. |

## Verification boundaries

The regression suites check complete lookup/enumeration agreement, dependency
ordering, failed publication, duplicate registration, owned copies, borrowed
views across catalog growth, null/empty/undersized buffers and untouched spare
slots. They also verify that metadata does not invoke bootstrap, that ABI remains
v1, and that independent C layouts match the combined table and plugin record.
Native-library tests cover provenance, identity conflicts, post-unlink metadata
and retained factory code. Both independently loaded and statically linked providers exercise
the same contracts; generated factories still undergo exact identity checks.

## Persistence and diagnostics

`NcPersistenceApiV1` creates/destroys caller-owned, thread-local contexts, registers
borrowed `IPersistenceFactory` objects with one retained owner, snapshots
`IPersistable` trees, and restores an owned canonical `IUnknown`. Contexts do not
populate or replace the global plugin catalog. Mutation/destruction requires
exclusive access; restore takes a shared borrow. `NcPersistenceLimitsV1` carries
binary and object-tree budgets explicitly.

Capture calls `NcPersistenceWriteBytesV1` exactly once on success with the complete
borrowed document; failure never calls it. The callback may copy bytes, must not
unwind, and cannot retain the pointer. Temporary storage is destroyed in the
runtime, so no Rust allocator ownership crosses the ABI. Snapshot identity retains
the canonical allocation along active ancestors; repeated siblings are legal.
Logical interface equality is not allocation identity. Validate the whole recipe
before constructing objects and release partial restored owners on failure.
NCB v1, live-call wire validation and application reconstruction remain distinct
contracts; see [persistence](../crates/host/PERSISTENCE.md).

`NcDebugApiV1` separately owns callback messenger creation/destruction. At most
64 messengers are live; callbacks execute outside runtime catalog locks. Final
destruction prevents new entry and waits for admitted callbacks to finish.
Callbacks may run concurrently on any thread and must not destroy a messenger.
Callback data is call-scoped. Its lifecycle does not belong in the
plugin operation table or a persistence context; see [debug ABI](../crates/core/src/runtime_abi/debug.rs).

## Borrowed metadata storage

`NcStaticUtf8` is `(*const u8, u32)`; `NcUtf8View` is `(*const u8, usize)`.
Neither owns storage or retains a library. `NcStaticUtf8::as_str` is unsafe:
UTF-8 is checked, but readable backing storage and its returned lifetime remain
caller obligations. Static/pinned descriptor graphs must retain names, signature
bytes, dependency arrays and code; a static outer reference does not prove raw
children are static. Copy callback metadata needed after a callback. See the
[text/lifetime table](../crates/core/README.md#text-bytes-and-borrowed-storage).

## Native and static composition

The facade provides typed clients and owns no implementation dependency, including
with all features. The final executable links the ordinary `neocogi_com_host` rlib. Native
providers link contracts/facade only and export their version-one descriptor.
Static composition registers a linked provider descriptor; dynamic composition
loads a provider library. Both use the same host-owned registry and C service graph.

No COM shared image, import library, bridge crate, runtime environment variable or
runtime-specific search path is deployed. Generic native library/resource staging
remains responsible for providers and assets. Provider mappings stay process-pinned
through final object/weak release. Host state, tables and callback code also stay
process/Wasm-instance-lived after main returns, allowing browser callbacks.
See the [native guide](guides/COM_NATIVE.md).

## Independent-build compatibility

The interoperability contract is a matching target C ABI, current core/runtime
ABI generation, and exact generated contract signatures. Independently selected
Rust compilers, optimization, LTO and panic settings must not leak private Rust
layout or allocator ownership across that boundary. This does not promise that
old contract revisions or different architectures are interchangeable.

The unpublished Rust [`xtask`](../tools/xtask/README.md) owns the independent
build and execution policy using installed Rustup toolchains and the platform's
native compiler/linker. An unpublished final test fixture owns its host
implementation; no additional production registry is introduced. From the repository root, run:

```sh
cargo xtask compatibility --suite cross-version --report com-cross.json
cargo xtask compatibility --suite build-flags --report com-flags.json
cargo xtask compatibility --suite linked --report com-linked.json
cargo test -p xtask --locked
cargo xtask workspace
```

The matrix is deliberately explicit, rather than a claim that every possible
compiler flag is interchangeable:

| Case | Host and its runtime | Providers |
| --- | --- | --- |
| linked | stable, debug | stable, debug |
| stable-host | stable, debug | nightly, release, abort, fat LTO, randomized private layout |
| nightly-host | nightly, release, abort, thin LTO, randomized private layout | stable, debug |
| stable-release | stable, release, unwind, thin LTO | stable, release, abort, fat LTO |

Unspecified panic/LTO settings are unwind/off. Randomization uses seed 107.
`cross-version` executes the two mixed-compiler rows; `build-flags` adds
`stable-release`. `linked` executes ordinary C-layout, checked-value,
recursive-interface, output-completion and lifetime regressions, then the complete
factory/persistence suites through the independent
[`host-runtime` final fixture](../tests/com/integration-tests/fixtures/host-runtime/Cargo.toml).
The same case files are reused by the ordinary test roots. Each final executable
explicitly owns its implementation; pure ABI tests enable no unused service clients.

On Windows, `linked` also requires the exact
`windows_native_paths_and_dependency_search_are_explicit` case and then the full
`windows_plugin_paths` target. One fixture build per parent invocation supplies
the genuinely imported helper and same-name alternative. Fresh bounded children
test search layouts, native Unicode/NUL/empty/missing paths, callback inspection,
and mandatory loading. Initializer marker positive/failure controls distinguish
rejection from already-executed code; mandatory failure is compared with an
isolated abort from the same test binary. Native Windows execution of this Rust
loader suite is required for publication; cross-checking and Wine probes only
provide supporting evidence.

Exact names, nonempty execution and actual Cargo artifacts are checked. Installing
nightly or finding a successfully compiled binary never passes a compatibility case.

Stable/nightly are rolling selectors, independent of the COM crates' declared
Rust 1.85 minimum. The Linux SDK job runs the floor suite sequentially:

```sh
rustup toolchain install 1.85.0 --profile minimal
cargo xtask msrv
```

That suite checks all COM/IDL/inspector targets and features, library tests,
normative ABI snippets, inspector binary tests, and the facade/core feature
variants. It also discovers and executes exact required semantic cases and then
the complete client-view and provider-threading targets, capability admission,
checked values, output buffers/completion and generated unit values. Parser
qualification additionally requires exact receiver-lifetime, unsupported-annotation,
type/integer syntax rejection and multipart source-diagnostic cases before running
the complete parser target. Snapshot diagnostics remain in stable qualification.
The cross-compiler native-loader cases run in the Linux safety job. The workflow
contains four jobs total, with one Cargo build job and one test thread per job.

Reports record `rustc -vV` for every selected compiler plus each role's exact
profile, panic and LTO policy. A writable report changes to `running` before
source preparation and to `failed` on preparation, execution or cleanup errors,
retaining completed case evidence. `passed` and the human success message are
published only after source cleanup finishes. Interrupted work can remain
`running`; report-write errors fail the command. Relative report paths belong to
the caller's working directory. The platform must permit native execution with
matching host triples. Linux, macOS
and Windows provider artifact names are handled. No COM runtime import library
is needed. Platform qualification
comes from executing its CI row, not from another platform's successful build.

The Windows and macOS jobs retain their linked-suite JSON as Actions artifacts
named for the native OS, architecture, checkout SHA and run attempt. Reports live
outside the temporary build targets and upload on failure as well as success;
missing reports fail retention. A receipt closes native qualification only when
its status is `passed` and its matching run/revision context shows complete native
execution. A cancelled run or detached JSON is insufficient evidence.

## What one case does

1. Validate installed compilers and matching native C ABI; remove ambient build
   and loader overrides.
2. Build provider A, independent peer B, and the host in separate targets with
   explicit compiler/profile policies. The host owns its runtime; providers do not.
3. Require each exact nonempty artifact, copy it and clean its build target.
   The relocated sample contains exactly one executable and two providers.
4. In linked/instrumented rows, build the maintained host-runtime test fixture.
   Its committed lockfile and actual Cargo manifest/source/kind fields identify
   both exact test executables, whether fresh or cached. Copy and clean them.
5. Discover exact sentinels and execute both unfiltered suites single-threaded
   after build outputs are gone. Check unchanged executable hashes. Counts alone
   cannot establish that the maintained bodies executed.
6. Run the [sample host](../examples/com-host/src/main.rs) without COM
   loader paths. B publishes a marker visible to the host and acquires A through
   its supplied handle. The host checks conflict rejection preserves its catalog.
7. Require successful exit and the marker emitted after all behavioral assertions.
   Reports distinguish ordinary tests, host-runtime cases and independent providers.

The independent sample exercises typed factory construction, inherited interface
calls, exclusive mutation, domain notifications delivered into the host's concrete queue,
canonical weak-origin identity, fresh copied output (nested enum/boolean and short
prefix) through ABI v1, persistence through provider-owned byte buffers,
restoration through a host-implemented factory adapter, promotion while live,
and failed promotion after final object release. All assertions execute in
release/abort builds. Normal completion crosses calls in both directions; panic
settings are build policies, not permission to unwind over a COM call.

Native C layout and malformed copied/nested boolean values are separate explicit
checks in `linked`, as are the runtime-free unique-owner and weak-lifetime test
targets. The relocated sample establishes cross-provider authority; the final fixture
executes the complete factory and persistence cases with host-owned services.
The monolith remains a separate integration test because it runs without any
native NeoCogi provider library.

## Entry points, safety checks and CI

`cargo xtask compatibility --suite NAME` selects the same Rust-owned policy on
Linux, macOS and Windows. `--report PATH` retains the invocation's evidence.
The Python driver and shell/PowerShell entry points have been removed; tasks
invoke subprocess argument vectors directly. Temporary sources, nested test
scratch and targets live below the checkout's `target/xtask-work` directory and
are explicitly retired before success. The runner forces one build job and one
test thread in all children, including compiler-spawning tests.
The checkout, copied source and archive workspaces exclude their `target` and
`tmp` subtrees from Cargo workspace membership. `TMPDIR`, `TMP` and `TEMP` point
to `tmp`; generated Cargo projects such as Miri's sysroot builder therefore
remain independent of every enclosing workspace during Cargo's ancestor search,
even when the interpreter cache is empty.

`cargo xtask compatibility --suite safety` requires nightly with `miri` and
`rust-src` installed. It discovers and interprets the COM core, runtime,
and facade library tests, plus current `local_com`, `com_main`, `borrowed_values`, `client_views`,
`checked_values`, `output_buffers`, `output_completion`, `exclusive_com` and
`weak_com` targets.
Four explicit large-buffer stress cases remain native-only: the two checked-value
budget traversals and the output-completion maximum-variant/provider-publication
stress tests. Their exact names live in the driver. They run in linked, ASan and
TSan suites. The handle-method inventory audit also runs natively: its `syn`
parsing of Rust source checks compiler naming policy, not COM memory operations.
Miri omits that parser workload and runs all remaining cases, including allocation
injection, preserved owners and malformed outputs. Each run records its actual
discovery/execution counts; historical counts are not an acceptance threshold.
Final runtime test executables explicitly select their implementation because Miri interprets pure Rust and
cannot load native plugins.
That is a different check from executing the real shared-library boundary.

`cargo xtask compatibility --suite sanitizers` executes address and then
thread instrumentation and failing on either error. The `address` and `thread`
suites also run individually. These require nightly `rust-src` and
`x86_64-unknown-linux-gnu`. Each rebuilds the standard library, runtime, host,
plugin and test harnesses under the same sanitizer. Instrumented executables
export sanitizer hooks before linker section collection: a later-loaded plugin
may need a hook that the host itself never called. Without that export,
ThreadSanitizer tests could pass before plugin acquisition failed on an undefined
`__tsan_atomic64_exchange`. The seven runtime-free Rust targets and three intentionally
embedded library suites retain their ordinary mode under instrumentation. The
two independent factory/persistence targets own their instrumented host runtime
and run after build cleanup without any COM runtime library. The
relocated example then executes without its build targets. Native-C layout
checks remain in `linked`, since this driver does not instrument the C compiler.
Leak reporting is disabled for sanitizer execution because successful plugin
libraries and process runtime registrations intentionally remain pinned. Address
and data-race checks remain enabled. Miri receives no suppression flags.

```sh
rustup toolchain install nightly --component miri --component rust-src
cargo xtask compatibility --suite safety --report com-miri.json
cargo xtask compatibility --suite address --report com-asan.json
cargo xtask compatibility --suite thread --report com-tsan.json
```

Missing components, unsupported sanitizer targets, discovery mismatches and
instrumentation errors fail the invocation; they are never silently skipped.
The combined sanitizer report records both cases and is successful only after
both complete. Temporary targets are deleted even when verification fails;
passing native roles also run explicit `cargo clean` after copying. Miri's task
owns its private temporary target, not the user's shared interpreter sysroot
cache; cleanup leaves that shared compiler cache intact.

[CI](../.github/workflows/rust.yml) uses four jobs without matrix expansion.
The Linux SDK job runs quality, workspace/release tests, Rust 1.85, archive
qualification and the linked row. A second Linux job runs the three mixed-build
rows, Miri, AddressSanitizer and ThreadSanitizer sequentially. macOS and Windows
have one linked job each. Grouped Linux checks still run after an earlier check
fails, unless the run is cancelled; each failure keeps the job unsuccessful.
A platform job is not evidence until it actually runs successfully; local Linux
validation does not imply macOS/Windows execution.

The ordinary COM job also runs facade/runtime library tests in release mode.
Strong-only retain exhaustion formerly returned uncounted owners only when debug
assertions were disabled, so debug tests alone cannot prevent its recurrence.
Both profiles exercise injected counter limits and native subprocess termination
through authentic typed byte/string `Clone`, the actual provider C
`QueryInterface` slots, and the child-list entrypoints. Byte queries cover IUnknown
and INcBytes; string queries cover IUnknown, INcString and INcBytes. Each success
branch must independently retain before publishing its projection. Same-contract
`ComPtr::query` legitimately uses AddRef and is not evidence that a provider query
slot was exercised. Portable ordinary and last-available-count controls check the
returned contract/object, exactly one retain, balanced query release, a usable
original and final release after restoring synthetic counts. Miri skips the
process-abort cases and interprets those count/final-destruction tests,
including child-list deallocation through its original owning pointer.

## Published archive validation

Run `cargo xtask packages` with Rust 1.85 installed. It creates
real Cargo archives for the five SDK packages and inspector, extracts them into a
fresh directory, and patches dependencies **only to those extracted archives**.
The temporary workspace contains only extracted packages and downstream consumers;
it inherits no package metadata from the checkout. Rust 1.85 checks all shipped
targets and runs shipped executable tests and runnable documentation examples.
An ordinary runtime build must produce its host
rlib; no COM runtime shared library is deployed. One temporary shared target
cache avoids retaining one copy per package and is removed after verification. The gate also builds a fresh consumer
with `build.rs`, generates an interface and executes its checked COM call. Runtime
and descriptor discovery are asserted to remain v1. No-default and alloc-only
facade checks cover the portable feature boundary. CI runs this same gate.

The unpublished `neocogi-com-tests` package at `tests/com` owns integration
fixtures, their generation and five executable Cargo examples. The facade has
no build script or compiler build dependencies. A standalone no-default facade
consumer's normal/build dependency graph excludes the IDL compiler and its
syntax/printing dependencies. Default macro consumers still compile the backend
for macro expansion. Run a checkout example with
`cargo run -p neocogi-com-tests --example calculator`.

Archive qualification copies the authoritative `strings_and_bytes` source and
schema from that test owner into a temporary downstream consumer. It runs against
extracted SDK archives, without the unpublished package, checking the string
prefix, borrowed-byte call, complete 8/8/7-byte stream and terminal empty chunk.

IDL, the test assembly and inspector repository integration suites inspect sibling packages,
current guide examples or nested native fixture workspaces. Their source files
stay in checkout CI under the unpublished `neocogi-tooling-tests` owner. The IDL
and inspector packages disable integration-test discovery and exclude those
sources from registry archives; packaging therefore omits no discovered targets.
The tooling owner explicitly registers the same sources without moving fixtures. Package-local unit tests remain
included. Compiler backend printer fixtures belong to the IDL archive; the core handle
source audit belongs to core. The small subprocess helper is deliberately local
to each owning package rather than requiring a published test-support crate.

IDL owns the shared `codegen` module used for both generated contracts and the
thin procedural macro crate. There is no separately published backend package.
The dependency order remains acyclic: core → IDL → macros → facade. Generation
and macro expansion run on the build machine; runtime peers receive compiled
bindings and access to the host's one registry.

The IDL package's backend executable tests create an ordinary matching-version facade
consumer. Cargo metadata supplies the selected core/facade source locations;
there is no `../core` or `../api` convention. In a checkout it reuses Cargo's
workspace package. Archive validation supplies all SDK archives through normal
patch configuration; after publication the same consumer resolves the published
version. These checks require that matching SDK and its registry dependencies
be available offline. They do not introduce a production IDL-to-facade dependency
or a cyclic publication order.

### Evidence limits

The commands above define acceptance; their presence is not a passing result.
Record actual source, compiler/profile/features, platform, executed/ignored cases
and limitations with each run. Native C layout, relocated shared ownership,
Miri, sanitizers, compiler floor and extracted archives are distinct scopes.
Dated runs and old specifications remain retrievable in Git, not as competing
normative files. Validation owns procedures;
review history owns recorded dispositions.
No historical test counts are promoted to acceptance for this ABI-v1 consolidation.

## Related contracts


- [COM overview and executed factory example](COM.md): contract authoring,
  implementations, plugin export and bootstrap.
- [IDL language and generation](contracts/IDL_MODULES.md): accepted syntax,
  identities, revisions, value shapes and Cargo integration.
- [Component responsibilities](architecture/COM_DESIGN.md): build tools,
  providers, runtime and application ownership.
- [Object/table representation](contracts/COM_INTERFACE_STORAGE.md): handle
  layout and the reason diamond inheritance does not require object headers.
- [Lifetime errors](contracts/COM_LIFETIME_ERRORS.md): dropped objects versus
  closed services and error propagation from live receivers.
- [Prepared output completion](contracts/COM_OUTPUT_COMPLETION.md): fallible
  preparation and the ownership boundary of consuming ABI calls.
- [Recursive values](contracts/RECURSIVE_VALUE_CONTRACT.md): checked wire
  acquisition, recursion, budgets and metadata.
- [Local NCB persistence](../crates/core/README.md#why-persistence-is-ncb-only): concrete
  bounded cursors, ordered construction, codec history and independent-plugin checks.
- [Independent-build compatibility](#independent-build-compatibility): compiler/flag
  matrix, instrumentation and platform limits.
