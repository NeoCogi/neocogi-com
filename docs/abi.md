# COM ABI v4 (WIP)

**SDK 0.5.0-wip-alpha.1 uses ABI v4 while the ABI is WORK IN PROGRESS (WIP).**

One native ABI version covers the shared layouts, callback signatures and
ownership contracts. The native version stays at 4 during WIP, including through
incompatible development changes. A version change or release freeze requires
explicit maintainer direction. Rebuild hosts, providers and generated consumers
from the same source revision. Hosts and plugins require a matching header and
version.

Published SDK 0.4.0 uses [ABI v3/core 21](https://github.com/NeoCogi/neocogi-com/blob/v0.4.0/docs/abi-v3.md).
Published SDK 0.3.0 uses [ABI v2/core 19](https://github.com/NeoCogi/neocogi-com/blob/v0.3.0/docs/abi-v2.md).
The [ABI v1 specification](https://github.com/NeoCogi/neocogi-com/blob/v0.1.0/docs/abi-v1.md)
is available at its release tag. Current admission rejects earlier
ABI v4/core 23 and ABI 24 drafts, along with drafts that used the retired
`NCOM` marker.

The persistence service borrows shared-owned roots and factories. Construction
and declared ownership establish the matching lifetime header. Serialization
session failures carry the originating status.

## ABI identities and versions

The public C-shaped definitions in [core](../crates/core/src/lib.rs) own the
layouts. Admission reads only supported prefixes before borrowing complete
records or invoking callbacks; it does not authenticate trusted native code.
Opening an image may still execute its initializers and descriptor getter.

| Identity or version | Current authority and purpose |
| --- | --- |
| Native ABI | NC_COM_ABI_VERSION == 4; checked in the plugin descriptor and host bundle |
| Native getter | `nc_get_plugin_descriptor`; fixed C signature and descriptor header |
| Interface contract | IID plus exact 32-byte ABI signature; checked for a particular interface |
| IDL fingerprint encoding | MODULE_ABI_ENCODING_DOMAIN; canonical encoding v16 with directional loans and threading |
| NCB format | Version 1; independent of live object ownership and lifetime layout |

Live method/vtable dispatch, shared-query ownership transfer, checked recursive
value wires, caller-owned persistence contexts and the separate service tables
retain their existing roles. Interface GUIDs remain nominal. V16 signs actual borrow permissions, result lifetime form, and threading evidence. NCB-v1 documents
and explicit serialization-provider selection keep their contracts. No Rust trait object, container layout, application value
or unwinding crosses a C boundary. The application links one host owner and
supplies its tables through nested plugins.

The [provider migration guide](guides/ABI_MIGRATION.md) maps earlier headers,
metadata, raw tokens and bootstrap names to this development contract.

## Interface identity and calls

The canonical `IUnknown` IID is reserved. Every owned COM object supplies that
projection, and IDL rejects authored callable interfaces that claim its IID.

Every generated interface has a Rust marker, a `#[repr(C)]` method table,
an exact GUID/signature identity, and separate inspection descriptions. The
marker has no instances in object storage. Raw references are explicit C records:

```c
typedef struct NcSharedWeakRaw NcSharedWeakRaw;
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
    NcStatus (*downgrade)(void *object, NcSharedWeakRaw *out); /* nullable capability */
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
The implementation's private `NonNullComRaw<T>` checks only presence of the pair;
its name grants no live-value, ownership, interface-identity or threading proof.
Those obligations remain with handle construction and provider call admission.

### Directional interface slots

A successful required shared-owner result transfers one reference in the
caller's out-pair. An optional shared-owner result uses the same layout and
transfers a reference only when both words are present. The all-null pair
denotes successful absence. The caller validates shape and identity before
adopting the transfer. A negative status leaves the out slot unread and
transfers no reference. A present but mismatched shared transfer releases its
reference using the supplied common table.

A shared or mutable input lends a live object/table pair for the call. A
mutable pair grants exclusive access to the target across all its projections.
An optional input permits the both-null pair. The receiver and all arguments
must jointly obey their declared borrowing rules.

An ordinary borrowed result writes a pair on success. Its lender is the
receiver borrow and remains live after native return. A read result from a
mutable receiver still retains that mutable receiver borrow. Required results
reject null. Optional results use both-null for absence. Borrowed results
transfer no ownership and rejected pairs require no ownership cleanup.

An explicitly scoped result replaces the out slot with a caller context and
`void (*visit)(void *context, NcRawInterface loan)`. The provider retains its
actual lender, such as a field guard, until delivery finishes. Success calls
the continuation once on the invoking thread, including for absence. Failure
calls it zero times. The continuation must not unwind or use a present pair
after it returns. The Rust wrapper checks pair shape, exact identity and
cardinality. It runs the visitor only for a valid present loan. Any workers
borrowing that loan must finish before the visit ends.

Unit uses the status-only slot. Direct and checked Rust projections of the
same native slot share its ABI signature. Direct unit discards checked status.
Existing direct optional owned and scoped outputs map checked failure to
absence. Direct ordinary borrowed and copied results panic on unexpected
checked dispatch or decoding failure. Their optional absence is a successful
output value. A checked declaration exposes the error status.

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

Rust method lookup uses `ComRustInterface::Wrapper<M>`: generated
`IFooWrapper<M>` borrows an opaque `ComInterfacePointer<IFooAbi, M>` from
its existing handle. These Rust-only wrappers retain zero-length slice metadata;
they do not cross the C ABI or change the two-word stored handle. Inherited
lookup borrows the physical table prefix without retaining or querying; an
owned shared base projection requires checked `QueryInterface`. A boxed
projection changes the interface through consuming lookup or through a borrow
of the live owner.

A scoped getter's declaring ABI marker supplies a Rust helper accepting the
same physical first-base inheritance path from a live shared or exclusive loan. A hidden unsafe
Rust witness certifies the offset-zero prefix and exact declaring marker. The
helper casts the borrowed table pointer for dispatch, preserves its object
address and threading marker, and uses the existing synchronous guarded visitor.
Its native slot, ownership, status and lifetime rules follow the declaration.

The former `ComClientInterface`, `NcClientPointer`, associated `Client<M>` and
`IFooClient` names have been replaced without compatibility aliases. This is a
Rust source/generated-name change. C layouts, vtable slots, exact IID/signatures,
canonical IDL encoding and NCB-v1 bytes were unchanged by that rename.
Regenerate Rust bindings with the matching SDK.

Admission checks the native ABI before dispatch. Interface signatures also
check each particular callable contract. See the
[storage rationale](contracts/COM_INTERFACE_STORAGE.md) for the
object/table design and its measurement boundaries.

Ordinary IDL declarations derive nominal identity from their exact qualified
names. `#[id("UUID")]` overrides that identity; declaration versions are absent.
Exact SHA-256 signatures detect typed contract changes automatically. Express
semantic distinctions outside the IDL's structural model with distinct named
types or interfaces. The migration removes `#[version(N)]` and the numeric
argument of `#[id(N, "UUID")]` from contract data. Both deprecated spellings
are accepted with source-located `WIDL001` warnings and ignore their integers.
Remove them to clear the warnings; regenerate bindings and rebuild hosts/providers
together. Old and new generated identities/signatures are incompatible.

The IDL resolver computes the module fingerprint before target Rust compilation. It
covers declaration names and identities, record fields, enum representations and
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
wrappers and physical table prefixes come from the checked ABI marker's associated
types, so a separately reexported companion cannot change their representation.
These checks add no runtime discovery or registry state; the
[IDL generation contract](contracts/IDL_MODULES.md#shared-idl-sources)
describes their scope.

`QueryInterface` matches both GUID and ABI signature. A GUID collision or a
stale binding therefore cannot silently produce a typed pointer. A successful
query returns one owned reference. `ComPtr` calls `AddRef` on clone and
`Release` on drop. A live lender supplies `ComRef` for shared methods or
`ComMut` from a mutable lender borrow. Both loans leave reference counts
unchanged. A mutable loan can be moved and may lend a shared reborrow.

A unit method uses a status-only slot. A scoped getter uses a synchronous
continuation. Other successful results use an output pointer. Expected failures
return negative `NcStatus` values and leave any output pointer unwritten.
Panics must not cross an `extern "C"` boundary.

Mutable IDL methods lower from `&mut self` and require exclusive access
to the target. That access comes from the live boxed owner or its
reborrow. Locks protecting shared provider fields and scoped stack
adapters can still reject reentry with `NC_E_BUSY`.

## Signature storage and absence

```c
typedef struct NcAbiDigest {
    const uint8_t (*digest)[32];
} NcAbiDigest;
```

Rust uses `#[repr(C)] struct NcAbiDigest { digest: *const [u8; 32] }`.
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
use neocogi_com_sdk::{NcAbiDigest, NC_COM_ABI_VERSION};
assert_eq!(NC_COM_ABI_VERSION, 4);
static FIRST: [u8; 32] = [7; 32];
static SECOND: [u8; 32] = [7; 32];
let a = NcAbiDigest::from_static(&FIRST);
let b = NcAbiDigest::from_static(&SECOND);
assert_eq!(a, b); // Content equality across separate backing declarations.
assert!(!a.is_empty());
assert!(NcAbiDigest::EMPTY.is_empty());
```

```compile_fail
use neocogi_com_sdk::NcAbiDigest;
static WRONG_WIDTH: [u8; 17] = [0; 17];
let _ = NcAbiDigest::from_static(&WRONG_WIDTH);
```

## Weak lifetime references

The callback in `IUnknownVTable.downgrade` is optional. When it is absent,
`ComPtr::downgrade` reports `NC_E_NOINTERFACE`. A successful callback transfers
one retained shared weak token at the canonical allocation address. A failed
callback clears its output. Handwritten providers opting in implement
the common header and lifetime protocol, not an opaque Rust Weak token.

The raw shared weak token contains one opaque allocation address:

```c
typedef struct NcSharedWeakRaw { struct NcSharedLifetimeState *control; } NcSharedWeakRaw;
```

The [lifetime engine](../crates/core/src/lifetime.rs) has a three-word
`NcSharedLifetimeState` with type information, a strong count and a weak count.
A box has a one-word metadata prefix. Both point to the four-word
`NcObjectTypeInfo` containing provider-local destruction/deallocation callbacks,
an immutable table array and its count. Ownership follows construction and
the declared slot. There is no erased ownership tag or mixed-policy cleanup.

Checked adoption validates presence and exact interface identity. An owner
rejected after transfer is released through its declared policy. Shared weak
tokens retain their own shared storage and code until their final release. The
preconditions for each boundary are in the
[lifetime contract](contracts/COM_LIFETIME_ADMISSION.md).

A shared `WeakComPtr<T>` owns one weak reference and occupies one pointer word.
Cloning changes only weak ownership. Promotion acquires a strong owner before
selecting the exact requested interface, and never resurrects a destroyed value.
Rust `upgrade()` returns `Ok(None)` when the strong count is zero, before exact
interface negotiation. Negotiation errors remain errors and release the acquired
strong owner, which can itself be the final release. Binding-owned C adapters
translate this result to their status/output protocol and clear failed outputs.
Canonical control addresses remain unique while weak owners prevent storage reuse.

Objects are constructed from completely initialized values. Generated construction
publishes one shared strong owner with an implicit weak storage reference, or one exclusive owner. There is no cyclic initializer, unpublished callable
object, or rollback state exposed by construction. Obtain weak identities after
construction while an owner is available.

Logical closure is separate from destruction. A closed service retained by a
strong owner can still be promoted; its methods enforce closure. Owned queued
facts remain readable after their weak source expires.

## Ownership operations and C callbacks

Generated objects select SHARED_UNKNOWN_VTABLE or EXCLUSIVE_UNKNOWN_VTABLE through
ComObject::UNKNOWN_VTABLE at construction/code-generation time. Every interface
prefix copies that implementation's selected callbacks and changes only its exact
interface identity. Shared classes reuse one shared callback family; exclusive
classes reuse another. There are two canonical root tables, not one per class.

Shared `AddRef` and `Release` operate on counted strong owners. Exclusive
`AddRef` refuses retention. Shared query or downgrade on boxed storage clears
the output and rejects. A boxed caller can release the sole owner via its
matching release slot. An interface loan grants use of the value for the
declared duration and supplies no ownership callback authority. Exact boxed
lookup uses the provider's immutable metadata and canonical table.

Only declared slots and provider-local cleanup use the C convention. Shared
weak promotion and counter bookkeeping use typed Rust support. Allocations
are freed in their allocating binary.

## Allocation and borrowing

Generated shared-backed implementations expose only actual shared references.
Rust providers that mutate state through shared receivers synchronize their
fields explicitly. Generated shared storage cannot implement an interface
whose authored or inherited methods need whole-value exclusive access.

Boxed storage contains a fully initialized provider value and starts with the
typed metadata prefix. All access derives from its sole owner or a live target
loan. Exclusive lending suspends the owner's conflicting use for the loan's
whole lifetime. Once the last loan ends, the owner can move or be destroyed.
The ordinary boxed path requires no runtime borrow counter. Native callers
must uphold the same liveness, aliasing and provenance contract. Stack format
adapters retain their separate admission guard.

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
This policy does not change any handle, method table or native ABI version.

Exclusive Drop consumes the owner. It invokes the provider's value destructor
and then deallocates in the allocating provider. Its valid call begins only
after all dependent borrows end. No expired boxed observer remains. Shared
storage retains its implicit weak reference through destruction so weak-to-self
fields can release safely. Final weak release frees storage exactly once.

## Consuming boxed arguments

The wire argument is `*mut ComRaw<I>`: writable call-scoped storage containing
one exclusive owner. On successful return the slot is null. On **every** negative
status it contains exactly its original object/table pair. For multiple owning
inputs, all transfer together or all return, in declaration order. Raw callers
must provide distinct, properly aligned slots and valid unique owners; forged
pointers and duplicate ownership tokens are not made safe by slot validation.

The Rust provider and caller use ordinary `ComBox` values:

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
Safe callers check the success/failure handshake before recovering Rust ownership.

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

The caller decides whether to retry or destroy returned inputs. Safe Rust
prevents moving an ancestor while the receiver borrows it. Native calls must
enforce the same ownership rule before supplying the consuming token.

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

`LocalBox<T>` derives concrete shared or mutable references from the unique
owner and can borrow exact interface projections. `LocalCom<T>` derives shared
concrete access from its strong owner. A foreign interface pair never supplies
construction proof for a private Rust type.

## Threading capabilities

The ABI signs four provider modes. Local grants neither cross-thread transfer
nor sharing. Shared grants both. Send grants transfer, including boxed
destruction on the destination thread. Sync grants cross-thread shared access.

| Capability | Send requires | Sync requires |
| --- | --- | --- |
| `ComBox` and `ComMut` | Send evidence | Sync evidence |
| `ComRef` | Sync evidence | Sync evidence |
| `ComPtr` and `WeakComPtr` | Both | Both |

Borrowed workers obey the loan lifetime. Generated Rust providers enforce
the declared target trait bounds. A handwritten provider promises the
corresponding access and destruction behavior.

A handle may use `into_threading::<Mode>()` to discard guarantees. For example,
a boxed handle from a shared-threading provider can enter a slot declared
as send-only. The conversion keeps the object, table and any loan lifetime
and does not acquire a reference. The Rust bound permits Shared to weaken to
Send or Sync, and permits every mode to weaken to Local.

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

`ComValue` supplies a raw `AbiRepr` and a `KeepAlive` for producer-owned
temporaries. The source value and `KeepAlive` retain the regions addressed by
`AbiRepr` through the call.
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
use neocogi_com_sdk::{INcBytesAbi, ComBytesExt, ComStringExt, IUnknown, NcString};
let text = NcString::from_utf8_str("ABI v4");
let another = text.clone();
let bytes = text.query::<INcBytesAbi>().unwrap();
assert_eq!(bytes.to_vec().unwrap(), b"ABI v4");
assert_eq!(another.to_string().unwrap(), "ABI v4");
let identity = text.query::<IUnknown>().unwrap();
assert!(identity.as_com_ref().same_object(bytes.as_com_ref()));
drop(text); // Other strong owners still retain the immutable value.
assert_eq!(another.to_string().unwrap(), "ABI v4");
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
`forward` lends an untouched writer to another synchronous generated wrapper
using the same slots; the nested result is validated before its count propagates.

Callers use `NcOutput<'_, T>` from `NcOutputBuffer<T>::output(offset, capacity)`.
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
Callers check counts, validate restricted fields into their reserved native
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

`WeakComPtr<I>` results transfer one `NcSharedWeakRaw` through a writable output slot.
The raw record is the existing single common-header pointer, without a cached
interface table. `NC_ABI_PASSING_WEAK_COM_PTR_OUT` (`0x0d`) identifies this passing
mode; the return type descriptor supplies the exact interface identity and the
passing descriptor preserves the threading capability. ABI v4 reserves
the retired boxed observer code `0x0b`.

A successful provider transfers one retained genuine shared weak token. The caller
rejects null tokens, missing interfaces and wrong revisions, releasing a rejected
present token exactly once through shared release. The producer guarantees shared
backing; boxed storage is outside this ABI result contract. Errors transfer no result.
Expired shared tokens remain valid: checking reads immutable
interface metadata under weak ownership and never promotes or reads the value.
Raw callers must supply a retained valid header, readable class metadata and
provider code lifetime; checks cannot make an arbitrary pointer safe.

A weak field in a borrowed value uses the same `NcSharedWeakRaw` as a borrowed token.
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

Before invoking the stable descriptor getter, native loading and inspection
require the selected image to export it. An ordinary import of a library that
exports the getter is insufficient and fails in `FindSymbol`. Windows PE export
forwarders and Apple Mach-O reexports are explicit exports of the selected image.
Windows also pins the image supplying the getter address. On glibc Linux, the
getter must resolve within the selected ELF link map. A local getter can call an
ordinary native dependency. Solaris and illumos restrict lookup to the selected
object with `RTLD_FIRST`. Other Unix loaders fail in `FindSymbol` when selected
exports cannot be verified. The caller's original path remains the diagnostic
spelling and, after registration succeeds, records the first selected exporting
image, including an export facade.

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
this per-open search. Export forward targets resolve under the native Windows
loader policy. The original caller spelling remains the diagnostic path and
first-registration provenance; the temporary absolute path is not catalog state.

Qualification does not canonicalize links, authenticate files or bind a checked
file handle to an image. Later provider-issued `LoadLibrary*` calls and delay
loads do not inherit these flags. Providers and their native dependencies remain
trusted code. The low-level inspector accepts native `OsStr` paths; SDK UTF-8
adapters retain their documented encoding restriction. Missing or unsupported
native loads report `OpenLibrary` without broadening the search.

### One host, supplied access

One authority owns plugin GUIDs, dependency validation, registration order, first
provenance and pinned native code. The application links the host rlib
`neocogi_com_host`. Ordinary `#[com_main]` initializes that owner and passes a
copyable `RuntimeHandle` to application code.

Every provider bootstrap receives access to those same immutable host tables.
A context-free callback in a table executes in the host and reaches its private
singleton in the linked host. A nested acquisition from plugin A to B
forwards that canonical host bundle again. Handles own no registry and select no
implementation; reusable libraries take a handle from their caller. There is no
global/default facade, per-plugin cache or fallback host discovery.

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
| Native getter | `NcGetPluginDescriptor`, exported as `nc_get_plugin_descriptor` by `#[plugin]` under `dynamic-plugin`. |
| Validation | `validate_plugin_descriptor` checks metadata without acquiring a factory or checking whether providers are registered. |
| Static registration | `RuntimeHandle::register_plugin_descriptor` validates and publishes the descriptor after requiring registered hard providers. |
| Unregistered library inspection | `RuntimeHandle::with_plugin_library_descriptor` lends the validated descriptor to a synchronous visitor. |
| Registered inspection output | `NcPluginInfo` / owned `PluginInfo`, including first registration origin. |
| Factory acquisition | `RuntimeHandle::acquire_plugin_root` checks the exact contract and returns one COM owner. |

The stable getter takes a pointer to one descriptor-pointer output slot and
returns `NcStatus`. The descriptor starts with C fields `uint32_t abi_magic`
and `uint32_t abi_version` at byte offsets 0 and 4. The consumer reads the
version only after the first word matches `NC_COM_ABI_MAGIC` (`0x4e434142`).
The version must equal `NC_COM_ABI_VERSION` before the consumer reads the
complete immutable descriptor. Static registration checks the same header.
Interface IID/signature and threading checks additionally select the exact
callable contract.

The fixed `NCAB` marker identifies this header family. It distinguishes
unmarked layouts and earlier drafts that used the `NCOM` marker with version
`4`. The getter name and both field offsets remain readable from the prefix.

The provider owns the descriptor's storage and code. Static registration requires
the entire graph to remain immutable for the process lifetime, including referenced
text, dependency tables and signature bytes. A static root reference alone does
not prove these raw-pointer lifetimes. Dynamic registration pins the exporting
library. Descriptor copies own neither those allocations nor a library mapping.
The native getter clears a usable output slot before validation, leaves it null
on failure, and returns `NC_E_POINTER` for null output.

`NcPluginInfo` is an output projected from a registered catalog record; it has
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

| `NcPluginInfo` field | Meaning and owner |
| --- | --- |
| `plugin_id` | Stable GUID derived from the qualified plugin name; acquisition and lookup key. |
| `qualified_name` | Borrowed immutable descriptor text; copied to `PluginInfo.qualified_name`. |
| `version` | Plugin release text, distinct from interface ABI signatures. |
| `library_path` | First successful dynamic caller path; empty denotes static origin. Provenance, without reopening a file. |
| `factory.interface` | Exact root factory IID and full ABI signature retained in static or pinned storage. |
| `factory.threading` | Valid raw `NcAbiThreadingMode`; the Rust facade returns the typed enum. |
| `dependencies` / `dependency_count` | Original immutable aligned `NcPluginDependencyDescriptor` array in declared order, borrowed directly from the validated descriptor. |

`NcFactoryInfo` contains only interface and threading information: the enclosing
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

The implementation exposes `neocogi_com_host::initialize_runtime()` as its safe
Rust entry, returning an admitted `RuntimeHandle` from its existing singleton without
provider calls. It owns unsafe adoption of the complete static graph; safe
startup never promotes a raw record returned by an arbitrary module. The separate
`initialize_process()` lends the immutable C description for ABI bootstrap:
`NcHostApi` starts with the byte size (`size_t`), followed by the fixed
`abi_magic` and native `abi_version` (`uint32_t` each). The following three
pointers select the runtime, debug and persistence tables. The complete graph and callback code
remain process/instance-lived, including after main returns. Copying the
description copies access to the same registry.

`validate_host_api` checks the exact host size, then the marker, then the
current ABI version before reading child pointers. The three child sizes must
match their respective table layouts. Each child belongs to the same immutable host graph
and carries its byte size as its first field. An unrecognized marker, unsupported
version, or wrong size returns `NC_E_ABI_VERSION`. A required null pointer returns
`NC_E_POINTER`. The checks invoke no callbacks and retain nothing. The unsafe
caller guarantees readable backing and callable code. Adoption additionally
requires those guarantees for the entire process lifetime.

The three tables are `NcPluginApi` for plugin operations, `NcDebugApi` for
diagnostics, and `NcPersistenceApi` for caller-owned persistence contexts. The
safe handle stores private references after admission. Handle copies can cross
threads. Host callbacks must support concurrent calls, and caller-owned
persistence contexts keep their thread-local and exclusive-borrow restrictions.

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

Registered metadata is available directly on `RuntimeHandle`; no intermediate
`Inspection` or `PluginRegistry` context is needed:

The executable explicitly selects the host implementation in Cargo and uses:

```rust
#[neocogi_com_sdk::com_main]
fn main(runtime: neocogi_com_sdk::RuntimeHandle) {
    for plugin in runtime.plugins().unwrap() {
        println!("{} {}", plugin.qualified_name, plugin.version);
    }
}
```

Its manifest enables `neocogi-com-sdk`'s `runtime` API wrappers and directly depends on
`neocogi-com-host`. The handle itself is available without `std` or `runtime`;
only richer API wrappers require that feature. `com_main` supports safe synchronous
nongeneric `fn main(runtime: RuntimeHandle)`, preserving normal `()`/`ExitCode`/`Result`
returns, `?`, cfg and lint scopes. Explicit `neocogi_com = path, neocogi_com_host = path`
arguments support renamed dependencies. It adds no executor.
The selected path must expose `initialize_runtime() -> RuntimeHandle`; safe forwarding
modules can return existing admitted access. A raw record is a compile-time type
error. An inconsistent canonical graph is an internal host error at initialization.

The [factory walkthrough](COM.md) executes generated typed registration,
acquisition and ordinary construction. Tests may initialize their explicitly
selected host with the safe `neocogi_com_host::initialize_runtime()`;
ordinary reusable libraries accept the resulting handle rather than initialize.

`plugin_info` returns `None` only for an unknown GUID. `plugins` returns owned
complete records in registration order; callers no longer enumerate IDs and then
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
`unsafe extern "C" fn(*const NcHostApi, *mut ComRaw<IUnknown>) -> NcStatus`.
The authored `#[plugin]` function accepts one `RuntimeHandle` and returns its existing
exact typed `Result<ComPtr<FactoryAbi>, NcStatus>`. Its adapter clears output
before admitting host access; invalid access enters no provider code and
transfers no owner. `write_plugin_root` invokes an unboxed `FnOnce` only after
output preflight. Metadata discovery needs no host or acquisition.

A provider may create a fresh factory or retain an existing instance. The runtime
promises no singleton and no identity reuse between successful calls.

The runtime compares the requested IID, full signature and threading capability
before invoking bootstrap. It releases the catalog guard before plugin code runs.
The successful bootstrap pair itself must advertise the declared root IID and
full signature. Acquisition validates it without calling `QueryInterface` or
retaining another reference, and transfers that original pair. A wrong IID returns
`NC_E_NOINTERFACE`, a wrong signature returns `NC_E_ABI_VERSION`, and a null,
half-null, or missing identity returns `NC_E_POINTER`. Rejection consumes one present successful
transfer through its original table. A failed bootstrap transfers nothing and its
output is never dereferenced or released.
The raw output starts null and only success transfers one owner. Typed facade
adoption preserves the requested IDL factory contract and threading model. Product
construction occurs through factory methods.

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

`RuntimeHandle::with_plugin_library_descriptor(path, visitor)` remains unsafe: the
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

A required native loading failure delivers its typed plugin error event
synchronously before attempting the console diagnostic. Console writes are best
effort and the process explicitly aborts even if stderr refuses output. The SDK
applies the same console and abort policy to local path conversion and failed
raw argument statuses. The host takes its stderr lock after the debug callback.

## Verification boundaries

The regression suites check complete lookup/enumeration agreement, dependency
ordering, failed publication, duplicate registration, owned copies, borrowed
views across catalog growth, null/empty/undersized buffers and untouched spare
slots. They also verify that metadata does not invoke bootstrap, that admission
checks the native ABI before borrowing layouts, and that independent
C layouts match the combined table and plugin record.
Native-library tests cover getter provenance across native imports, identity
conflicts, post-unlink metadata and retained factory code. Both independently
loaded and statically linked providers exercise the same contracts. Generated
factories still undergo exact identity checks.

## Serialization format objects

Value serialization uses the COM-owned [format interfaces](contracts/SERIALIZATION_FORMATS.md).
A shared `ISerializationFormat` factory returns exclusive serializer and source
objects. Typed local values use borrowed `Serializer`/`Deserializer` facades;
primitive values, names and counted container operations cross their C vtables.
Input is copied before an owning source returns, and committed output is copied
into fresh caller storage. Fixed-buffer NCB conveniences lend scoped,
nonretainable objects. Format/schema failures make a session terminal, and finish
requires exactly one complete document. ABI rejections do not enter the provider.

NCB remains format version 1 and preserves existing bytes. Names are available
for other formats without adding names to NCB. This supersedes the concrete
NCB-cursor restriction from 0.1.0 and changes the Rust value-trait API. Discovery
family 2 is used by the 0.3.0 release.

The reusable serialization rlib owns unconditional/default NCB and an optional custom JSON
codec. A separate thin dylib shim exports its static plugin descriptor.
Failed owning sessions expose `ISerializationDiagnostics`; convenience operations
copy the first failure message, logical path and available source coordinates
into a caller-owned error before releasing the session.

## Persistence and diagnostics

`NcPersistenceApi` creates/destroys caller-owned, thread-local contexts, registers
borrowed shared-owned `IPersistenceFactory` objects with one retained owner,
saves shared-owned `IPersistable` trees with `save_object_tree`, and returns an
owned canonical `IUnknown` from `restore_object_tree`. Contexts do not
populate or replace the global plugin catalog. Mutation/destruction requires
exclusive access; restore takes a shared borrow. `NcPersistenceLimits` carries
binary and object-tree budgets explicitly.

Table presence and successful host admission establish supported operation
layouts, not an available serialization provider. The caller still owes the
host API's complete backing and lifetime guarantees. Host initialization, empty
context creation and recipe registration do not require a codec or populate the catalog.
The final application must explicitly register `nc.serialization` or load its
library before saving or restoring. Each operation acquires that provider through
this host's own catalog and selects named `ncb`; `default_format()` is never used,
even when an external provider defaults to JSON. The shipped serialization
provider still defaults to NCB; JSON-default tests challenge persistence's
independence from default selection. There is no implicit registration, loading
or embedded fallback. The provider root and selected format remain retained
through codec use and restoration's recipe callbacks. Restore counts the root
and charges each admitted child count during decoding. The host rejects an
impossible object count or depth before reading the children or reserving their
record storage. Child-record storage grows as children complete. Saving produces
an independent host-owned byte vector; its later delivery callback does not
retain the codec objects. Provider and delivery callbacks run outside catalog locks.

A missing provider returns `NC_E_NOT_FOUND`. Root acquisition and named-format
selection errors propagate. `save_object_tree` validates the sink and exact shared
root, charges the root object budget, then retains it and opens the writer.
All live-object getters run after writer creation. The application supplies a
stable view of the complete tree through the synchronous call, including
reentrant calls. The state buffer keeps the same length and contents through
all chunk copies.

The saver writes each node via the ordinary serializer as four positional
values in NCB-v1 order: recipe UUID bytes, schema version, initialization byte
sequence, and ordered children. It charges all declared child object slots
before reading their parent's state or fetching children. The format enforces
document bytes, element count, value count, and wire depth. A sequence with
nonzero count requires room for its immediate element. The saver returns the
first failing operation's status, including codec failures.

`restore_object_tree` clears its output and checks the input byte bound before
codec selection. The object's count and ownership depth limits return
`NC_E_OUT_OF_RANGE`. Refused host storage for decoded state, child records,
constructed child owners, or the immutable child list returns
`NC_E_OUT_OF_MEMORY`. Restore maps codec failures to `NC_E_INVALID_ARG`,
including failures originating in the codec's own allocation.
Recipe validation begins after accepting the complete document and finishes
before constructing any object. The same live context can retry after a failure.

Saving opens the writer before reading object state, copies the state in
bounded chunks, and preserves the originating save status. The saved NCB-v1
representation uses the same count and value order.

After successful encoding and output copy, `save_object_tree` calls
`NcPersistenceWriteBytes` once with borrowed complete bytes. The callback may
copy its input, must not unwind, and cannot retain that pointer. A pre-delivery
failure makes no call. The sink's own error propagates after the attempt.
While saving, the host retains the active ancestor allocations. It tests
identity by allocation address. Repeated siblings can occur independently in
the encoded tree. See [persistence](../crates/host/PERSISTENCE.md) for format
limits, asset offsets, and cross-document validation boundaries.

`NcDebugApi` separately owns callback messenger creation/destruction. At most
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

The SDK provides typed wrappers over the selected host's C service graph.
Native and browser applications link the `neocogi_com_host` rlib and can
register linked provider descriptors. Native applications can also load
provider libraries exporting `nc_get_plugin_descriptor`. Both operations
use the linked host's registry. Native provider mappings stay process-pinned
through final object and weak release. Linked host state, tables and callback
code outlive main in the process or Wasm instance.
See the [native guide](guides/COM_NATIVE.md).

## Independent-build compatibility

The interoperability contract is a matching target C ABI, native ABI version,
and exact generated contract signatures. Independently selected
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
explicitly owns its implementation; pure ABI tests enable no unused runtime API wrappers.

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
the complete interface-wrapper and provider-threading targets, capability admission,
checked values, output buffers/completion and generated unit values. Parser
qualification additionally requires exact receiver-lifetime, unsupported-annotation,
type/integer syntax rejection and multipart source-diagnostic cases before running
the complete parser target. Snapshot diagnostics remain in stable qualification.
Logical module-name/span parser cases are also required, and the existing full
library phase must execute the named independent SharedSources consumer. Its
real calls cover imported and qualified types in legal weak/underscore namespaces.
Archives require the same shipped consumer alongside the older imported-provider
operation; neither can be ignored or absent behind a broad passing test count.
The cross-compiler native-loader cases run in the Linux safety job. Qualification
uses one test thread while Cargo selects build parallelism. The workspace, linked,
and 32-bit gates require named success for both native C protocol directions and
the portable fixed-array wire case within the complete fixture run. The 32-bit gate queries the selected compiler's
target configuration before building.

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
calls, shared-provider state changes, domain notifications delivered into the host's concrete queue,
canonical weak-origin identity, fresh copied output (nested enum/boolean and short
prefix) through ABI v4, persistence through provider-owned byte buffers,
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
invoke subprocess argument vectors directly. Temporary sources and their nested
temporary directories use a canonical system temporary parent outside the checkout.
Archive sources reuse that parent. Build outputs and task scratch use the checkout's
`target/xtask-work` directory. Both owners are explicitly retired before success.
The runner forces one test thread in children, including compiler-spawning
tests. Cargo uses its normal build
parallelism and honors a caller's build-job configuration; no one-job cap is imposed.
The checkout, copied source and archive workspaces exclude their `target` and
`tmp` subtrees from Cargo workspace membership. In child processes, `TMPDIR`, `TMP`
and `TEMP` point to that source owner's `tmp`. Generated Cargo projects such as
Miri's sysroot builder therefore remain independent of every enclosing workspace
during Cargo's ancestor search, even when the interpreter cache is empty.

`cargo xtask compatibility --suite safety` requires nightly with `miri` and
`rust-src` installed. It discovers and interprets the COM core, runtime,
and facade library tests, plus current `local_com`, `com_main`, `borrowed_values`, `interface_wrappers`,
`checked_values`, `array_construction`, `output_buffers`, `output_completion`, `exclusive_com` and
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
have one linked job each. All three native platforms run the checkout inspector
integration cases for relocated plugins, Unicode paths, dependency plans and
exact root mismatches. Linux also compares the inspector's min-size diagnostics
and exercises native paths with invalid UTF-8 bytes. Grouped Linux checks still run after an earlier check
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
real Cargo archives for the [published SDK inventory](../tools/xtask/src/packages.rs), extracts them into a
fresh directory, and patches dependencies **only to those extracted archives**.
The temporary workspace contains only extracted packages and downstream consumers;
it inherits no package metadata from the checkout. Rust 1.85 checks all shipped
targets and runs shipped executable tests and runnable documentation examples.
An ordinary runtime build must produce its host
rlib; no COM runtime shared library is deployed. One temporary shared target
cache avoids retaining one copy per package and is removed after verification. The gate also builds a fresh consumer
with `build.rs`, generates an interface and executes its checked COM call. Runtime
and native admission follow the [current version table](#abi-identities-and-versions).
No-default and alloc-only facade checks cover the portable feature boundary. CI runs this same gate.

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
No historical test counts are promoted to acceptance for the current ABI candidate.

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
- [Value serialization formats](contracts/SERIALIZATION_FORMATS.md): COM format
  objects, typed construction, bounded sessions and independent-plugin checks.
- [Independent-build compatibility](#independent-build-compatibility): compiler/flag
  matrix, instrumentation and platform limits.
