# COM core ABI v17: one-way copied output

Core generation 17 replaces generic copied writable parameters with output-only
wire storage and a separately published initialized-prefix count. Versions 16
and earlier are rejected by bootstrap negotiation. Every output-bearing IDL
interface also receives a revised canonical signature: the output category uses
tag 14, distinct from the retired tag 8. Nominal type identities are unchanged.

The fixed 32-byte signature views, two-word object/interface handles, atomic
lifetime header, immediate exclusive Drop, weak reference lifetime, and returned
owner handshake follow [generation 16](abi-v16.md). Its *bidirectional generic
buffer* rules are superseded here; generation 16 is historical documentation.
Independently compiled modules remain supported with different compiler versions
and flags when they agree on core generation, exact signatures, target C layout,
and atomic pointer width. Rust native enum or container layout never becomes ABI.

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
Expired and unpublished shared tokens remain valid: checking reads immutable
class metadata under weak ownership and never promotes or reads the value.
Raw callers must supply a retained valid header, readable class metadata and
provider code lifetime; checks cannot make an arbitrary pointer safe.

A weak field in a borrowed value uses the same `NcWeakRaw` as a borrowed token.
The source handle keeps its header alive throughout encoding and acquisition.
Decoding validates before retaining an independent weak reference. Partial
aggregate failure drops already acquired fields. It never creates a strong owner
and is not a copied inline value or an implicit persistence representation.

This adds a passing category to generation 17 without changing existing layouts
or signatures. IDL assigns the new ownership spelling a distinct fingerprint tag;
older metadata readers reject the unfamiliar passing category. `WeakLocalCom<T>`
is construction-proven Rust access layered on this existing shared lifetime
protocol; its concrete T and native wrapper do not cross the ABI.

## Fixed byte-copy primitives

`INcBytes::copy_to` and `INcString::copy_utf8_to` retain their exact-range
`offset, length, pointer` slots and `NC_ABI_PASSING_COPY_TO_BUFFER_OFFSET` (`0x04`).
These built-in contracts copy a prescribed number of unrestricted bytes or fail.
They have never interpreted old bytes as input, need no enum conversion, and do
not represent the removed generic mutable-value path. IDL and `com_interface`
output parameters always use the new four-slot protocol.

See [directional output](../../../docs/COM_OUTPUT_BUFFERS.md),
[ownership completion](../../../docs/COM_OUTPUT_COMPLETION.md), and
measurements for examples and evidence.
