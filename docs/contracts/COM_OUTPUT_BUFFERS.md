# Directional copied COM output

An output getter produces fresh values. Existing destination contents are not
inputs, and no native mutable slice is reconstructed from old foreign contents.
An actual transformation takes a readable input and a separate output. Persistence
serialization remains independent of these live COM wire records.

The provider receives `NcOutputWriter<'_, T>`; generated clients take
`NcOutput<'_, T>`. Both are local Rust views. The raw ABI passes source offset,
wire pointer, capacity, and an initialized-prefix count pointer. `T` must implement
`ComInlineValue`: its copied wire is self-contained, with no borrowed pointees or
transferred capabilities. Dynamic strings, vectors, and COM ownership transfers
retain their separately defined boundary contracts.

## Provider writes

`push` encodes a native value directly into caller-owned wire storage. For enums,
the existing codec writes the explicit C-layout tag and selected wire payload.
Booleans remain bytes. `extend_from_slice` uses direct bulk copying only when the
recursive `BORROW_WIRE` proof establishes identical layout and unrestricted bits;
otherwise it uses the same checked typed encoding as individual writes.

The writer admits capacity and validates pointer shape before implementation
entry. It never reads incoming output contents, allocates scratch vectors, or
retains the caller's region. Partial production on failure is discarded. Count
publication belongs to the thunk after successful implementation completion.

## Caller storage and publication

`NcOutputBuffer<T>` owns reusable native results and, only for converted elements,
separate raw capacity. `output(offset, capacity)` immediately discards previous
results and creates a borrowed fresh-output argument without allocating. This
also applies when the view is abandoned, an earlier argument rejects, or the
caller unwinds before preparation reaches this output. Keep a separate snapshot
for rollback. Generated preparation admits the complete possible shape and
reserves capacity before provider entry or ownership transfer. Reservations are
fallible. Dropping the buffer frees both allocations locally.

Allocation-free numeric callers can instead use `NcOutput::from_slots` with
`MaybeUninit<T>` slots and a result count. This requires the identity-wire proof
and preserves clients/providers without the `alloc` feature. Its count remains
zero until the complete output group succeeds.

After success, acquisition checks the reported count against requested capacity,
then validates every active tag, boolean byte, and nested field. Only the reported
prefix is read. Native results remain private through the exclusive call borrow.
Generated clients acquire every output before publishing any. Rejecting,
malformed, or unwinding calls discard every new result, including an earlier
buffer acquired before a later buffer failed. There is no old-value rollback:
an application needing it retains the old state separately until the new result
is complete.

The local completion state is `Unacquired -> Acquired -> Published`. An error or
Drop before publication clears results/count. Publication before complete
acquisition is a binding invariant violation. All allocation and conservative
resource admission precede any accepted owning input; completion cannot report
ordinary allocation failure after an owner has committed. Malformed output from
a successful consuming provider remains a provider violation, not fabricated
recovery of already consumed owners.

## Foreign memory obligations

Identity element admission checks a representable destination region without a
conversion count/depth budget. Both `push` and `extend_from_slice` subsequently
copy under that same identity proof; an individual write never re-enters ordinary
inline conversion. Converted elements retain their prepaid maximum and restricted
field validators. The distinction belongs to output admission/completion and does
not relax shared recursive value-tree limits.

Raw callers supply aligned, exclusively writable, live capacity and a separate
live count destination. Raw successful providers initialize all active fields in
the reported prefix and never write beyond capacity. Padding and inactive union
payloads need not be initialized. Content validation rejects malformed restricted
bits, but cannot prove arbitrary address readability or a lying provider's
initialization promise. Failed calls make partially written slots unavailable.

## Why this direction

The old generic writable buffer represented initialized input/output values. A
getter therefore encoded dummy caller values, decoded them on the provider,
overwrote them, and converted them back. Preparing completion before owner
acceptance fixed a real failure but retained two vectors per side. Directional
output preserves checked wire encoding, early admission, and grouped publication
while removing the input direction and provider scratch. It does not pass Rust
enums, vectors, allocator ownership, or writer layout across a plugin boundary.

The output migration includes real generated thunks, owner failure injection, malformed
outputs, native C, and independently compiled host/plugin checks. Core generation
17 and revised output-bearing fingerprints prevent mixing retired signatures.

`NcOutputWriter::forward` lends untouched raw storage through a synchronous
provider-to-provider call, with no intermediate vector. Partial writers cannot
be forwarded. The nested client validates every reported element before its
count reaches the outer thunk.

The built-in `INcBytes` and `INcString` exact-range byte-copy methods retain
their three-argument pointer protocol and descriptor category. They never read
old bytes, contain no restricted native elements, and are independent of the
removed generic mutable buffer. See [ABI v1](../abi-v1.md).
