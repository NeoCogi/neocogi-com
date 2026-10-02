# COM core ABI v16: fixed-width signature views

Version 16 replaces the signature pointer/count pair with one nullable pointer
to exactly 32 immutable SHA-256 bytes. Every containing interface/descriptor record
uses this new layout. Core ABI 15 and earlier are rejected before reading their
changed record fields or invoking a plugin bootstrap. Discovery's exported symbol
and initial two-u32 version prefix remain unchanged; the core generation selects
the rest of the descriptor layout.

Independently compiled hosts/plugins remain supported across compiler versions,
optimization flags and features when they agree on this core generation, exact
interface signatures, target C layout and atomic pointer width. The digest
algorithm, canonical IDL encoding and digest contents are unchanged by this
storage migration. The ownership and consuming-call rules below carry forward
from version 15.

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

The view occupies one pointer word: 8 bytes on the tested 64-bit target instead
of the old 16-byte pointer/count/padding view, and 4 bytes on conventional 32-bit
C targets instead of 8. `NcInterfaceId` correspondingly occupies 24/20 bytes.
`NcInterfaceDescriptor` occupies 80/56 bytes on those targets. Digest backing
storage remains 32 bytes and can be shared among descriptors. These are record
sizes, not a claim of equivalent executable-file savings. Inline arrays would
copy 32 bytes into every occurrence and still require an absence representation.

History explains the choice: `6060bac4` needed variable mangled strings;
`f5e15ec4` supported multiple representations; `4c016417` made SHA-256 mandatory;
`57cd9101` precomputed declarations in IDL. The variable-width view outlived its
purpose. Fixed borrowed storage removes that flexibility without duplicating
static digests or changing compatibility semantics.

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
a two-word provider-local lease: a value reference and one selected guard.

A reader atomically checks owner presence, excludes the writer and increments the
reader count. A writer atomically checks owner presence and zero borrowers before
setting its bit. Conflicts and reader-count exhaustion return `NC_E_BUSY`;
expired or unpublished objects return `NC_E_DROPPED`. These checks precede creation
of any Rust reference. Acquire admission pairs with release of prior borrows and
constructor publication. Reader release subtracts one; writer release clears only
its bit, preserving concurrent owner removal. No borrower destroys the value.

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

Owner Drop clears owner presence with an acquire/release operation, rejecting all
new calls. It waits for existing readers/writers using acquire observations, invokes
the provider's destructor exactly once, and releases its implicit weak storage
reference. Drop returns only after value destruction, including ordinary owned
child fields. The wait uses spin hints; std-enabled builds also yield to the
scheduler. Completion requires admitted calls to terminate: this is not wait-free.

The implicit weak reference lasts through the destructor, protecting storage when
T drops a weak-to-self field. The last external weak reference frees allocation
storage through the original provider and never drops T again. Provider code and
metadata must remain loaded until that final weak release.

A weak handle's `as_com_ref` borrows retained interface storage, not T. Each method
still acquires its own checked access, so keeping an ABI argument view cannot
prolong the value's lifetime. `is_alive` and `is_idle` are snapshots, never access
permission. Cached interface queries can inspect retained class metadata after
value destruction. No weak reference upgrades to an exclusive owner.

An executing method must not destroy its own owner, directly or by removing an
ancestor; waiting would deadlock. Nor may the dropping thread hold resources
needed for an admitted call to finish. These are ordinary blocking synchronization
constraints, not permission to defer destruction. Thread-local UI removal checks
the existing subtree borrow state before committing and returns Busy on recursive
removal. Widget behavior posts ordinary typed close intent to its parent, whose
handler decides removal after the child returns; no COM lifecycle queue is involved.
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

```rust,ignore
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

Generated adapters also finish their own resource preparation before adoption.
Converted writable buffers reserve input and completion vectors on each side.
For consuming calls, copied results and converted output elements must provide
an allocation-free `ComValue::MAX_COST`; both sides admit the maximum output
count/depth and buffer bytes before invocation. Unavailable maxima reject the
Rust declaration; unaffordable known maxima return `NC_E_OUT_OF_RANGE` with the
original owners. Completion reuses reserved storage and validates all converted
outputs before publishing any. Direct buffers retain in-place semantics, and
non-consuming calls retain actual-output budget checks. See
[prepared output completion](../../../docs/COM_OUTPUT_COMPLETION.md) for examples,
error ordering, limits, and allocator-injection regressions.

The caller decides whether to retry or destroy returned inputs. In particular,
rejecting ancestor A offered to descendant B returns A through every forwarding
call before application code can drop it. This requires no deferred-destruction
queue. Destroying one's own active owner remains forbidden by the synchronous
Drop contract; ownership-taking providers must validate before accepting it.

`ComBox::query` and `LocalBox::into_interface` likewise return the original owner
on failed projection. UI facade calls preserve the original typed widget or
compiled component. Private builders that create fresh, unpublished candidates
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

Validation covers actual generated calls, reader/writer exclusion, concurrent Drop,
expired borrowed views, cyclic construction rollback, consuming-input rejection,
and UI tree destruction. These tests establish exercised behavior; they are not a
formal verification of arbitrary foreign providers or application deadlock freedom.

## Verification

The migration was checked on native x86_64 Linux with fresh COM facade/core/
macros/runtime tests, the complete IDL suite, UI design/document tests,
`neocogi-inspect` unit and standalone-plugin tests, and `cargo check --workspace`.
The core also passes with no default features. Regression coverage includes:

- Compile-fail construction from empty and 17-byte arrays; absence versus a
  present all-zero digest; one-word storage and containing-record offsets.
- Equal bytes at separate addresses compare and hash equally; changing each of
  the 32 bytes rejects equality, including the last byte.
- An independently compiled native C caller copies a generated digest into its
  own stack array, successfully queries the Rust object, and releases the acquired
  owner. Every single-byte mutation and an absent digest reject the query and
  clear both output words. The borrowed stack identity never escapes the call.
- Factory activation rejects absent/different signatures before the bootstrap
  counter changes; rejected outputs are cleared. Previous generations are rejected
  using a record consisting solely of the two version words.
- Actual plugin inspection still reports the exact generated factory digest.

The independent-build driver also executed both `cross-version` cases using
stable Rust 1.98.1 (`48a229cea`) and nightly 1.101.0 (`c1070d693`):

| Host and runtime | Plugin | Result |
| --- | --- | --- |
| Stable dev, unwind, no LTO | Nightly release, abort, fat LTO, randomized private layout | Passed |
| Nightly runtime dev; host release, abort, thin LTO, randomized private layout | Stable dev, unwind, no LTO | Passed |

Each artifact was built independently, copied into a relocated directory and
executed after cleaning its build directory. The host exercised factory calls,
event identity, persistence and release; this is execution evidence rather than
successful compilation alone. The driver invocation was
`python3 crates/com/scripts/compatibility.py --suite cross-version --report /tmp/fixed-signature-compatibility.json`.

Local limitations: Rust 1.85 compilation is blocked by two pre-existing
`unsigned_is_multiple_of` uses in `core/src/value.rs`, not by this signature API.
The attempted 32-bit C compilation lacks system libc headers; 32-bit constants
were updated but have not been executed here. No whole-bundle size or query-speed
benchmark was performed.
