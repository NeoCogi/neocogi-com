# COM core ABI v14: synchronous exclusive destruction

Historical specification, superseded by [ABI v15](abi-v15.md).

Version 14 keeps the object/table handle and three-word common header, but
changes exclusive borrowing and owner destruction. Version 13 providers must be
rejected before activation: their separate admission counter and borrow gate
cannot participate in the new protocol. Independently compiled plugins may use
different Rust compiler versions, optimization settings and features when they
agree on this ABI, target C layout, atomic pointer width, and interface signatures.
Rust implementation layouts, guards, closures and transfer enums never cross COM.

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
expired or unpublished objects return `NC_E_CLOSED`. These checks precede creation
of any Rust reference. Acquire admission pairs with release of prior borrows and
constructor publication. Reader release subtracts one; writer release clears only
its bit, preserving concurrent owner removal. No borrower destroys the value.

## Drop and weak storage

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
removal. `IWindow::request_close` lets widget behavior request removal after its
call returns. Explicit ThreadSafe objects additionally require Send + Sync
implementations and permit destruction on the dropping thread.

## Consuming arguments

The wire argument remains a pointer to a writable object/table transfer slot.
The original caller owns that slot until a provider explicitly takes its contents.
Rust providers receive `ComBoxInput<'_, I>`: validate through `as_com_ref`, forward
the same input unchanged through nested calls, and call `take` only after validation
succeeds. Generated clients accept an ordinary `ComBox<I>` directly as well.

An unaccepted input is destroyed by the original client after the receiver returns,
including argument-decoding or receiver-admission failure. Merely entering a provider
must not adopt the input: rejecting an ancestor offered to its descendant otherwise
destroys the currently executing receiver. Consuming providers must not take an
ancestor/self owner or destroy an active receiver; container mounting proves the
acyclic relationship before adoption. There is no deferred-destructor queue.

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
