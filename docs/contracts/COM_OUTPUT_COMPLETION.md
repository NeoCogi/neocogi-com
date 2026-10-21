# Prepared output completion for consuming COM calls

A generated call accepting `ComBox` inputs must finish resource preparation
**before the implementation accepts those owners**. An ordinary failure returns
all original owners; success consumes them. Generated completion cannot retrieve
an owner already stored in an arbitrary application field.

[ABI v17](../../crates/com/docs/abi-v17.md) retains that transaction boundary and
replaces the old bidirectional writable buffer with fresh output. IDL and
IDL method/value plans share one typed backend and checked value codecs.
Persistence serialization is a separate protocol, not part of these getters.

## Example

```rust,ignore
#[version(1)] interface IContainer {
    fn accept(&mut self, item: ComBox<IItem>, flags: &mut [bool]) -> Result<()>;
}
```

The Rust provider implements:

```rust,ignore
fn accept(
    &mut self,
    item: ComBox<IItemAbi>,
    mut flags: NcOutputWriter<'_, bool>,
) -> Result<(), (NcStatus, ComBox<IItemAbi>)> {
    if let Err(status) = self.prepare_insertion() {
        return Err((status, item));
    }
    // COM has already admitted every slot and reserved caller completion storage.
    for _ in 0..flags.capacity() {
        flags.push(true).expect("valid bool fits prepaid inline output");
    }
    self.item = Some(item);
    Ok(())
}
```

The caller owns `NcOutputBuffer<bool>` and passes `flags.output(0, count)`.
Acquiring that output view immediately discards old buffer results, retaining
native and wire capacity. Abandoning the view, failing an earlier argument or
unwinding before preparation still leaves every acquired output fresh. Old
values are never encoded as input. Application rollback needs a separate snapshot.
The provider must still prepare its own fallible work before ownership commit.

Each raw input owner slot stays unchanged through preparation, becomes null on
adoption, and is restored if the implementation rejects the owner. Multiple
inputs transfer together or all original identities return in parameter order.
An output adds its own `written: *mut u64` slot after offset, capacity, and raw
wire pointer; it does not use the method's ordinary return value as its count.

## Ordering and publication

```mermaid
sequenceDiagram
    participant C as Generated caller
    participant T as Generated provider thunk
    participant P as Implementation
    C->>C: Acquire output views; discard every previous result
    C->>C: Admit maximum output cost; reserve native and wire capacity
    C->>T: Lend owner slots and fresh raw output storage
    T->>T: Check every argument and output; no scratch allocation
    T->>T: Admit receiver, then adopt owners
    T->>P: Invoke with output writers
    P->>P: Prepare application work; encode fresh values directly into slots
    P->>P: Commit ownership
    P-->>T: Success
    T->>T: Encode ordinary result; publish initialized-prefix counts
    T-->>C: Success with null owner slots
    C->>C: Check counts and acquire every output without allocating
    C->>C: Publish all results together
```

Local caller completion follows `Unacquired -> Acquired -> Published`. A guard
that drops before publication clears native results or the stack/forwarded count.
A malformed later output therefore discards an earlier successful acquisition.
Only the reported prefix is read; unused capacity can remain uninitialized.
The provider may have modified raw bytes on failure, but neither safe clients nor
raw callers may interpret those bytes as results after a failed status.

A reusable converted buffer retains two local allocations: native result capacity
and raw wire capacity. Providers retain **no** conversion vectors. Identity-wire
numeric outputs use only native storage, preserving bulk copy.
`NcOutput::from_slots` permits allocation-free identity-only stack storage;
`NcOutputWriter::forward` permits allocation-free synchronous provider delegation
through the same raw slots. Both publish a count only after complete acquisition.

## Maximum output cost

A valid implementation can select the most expensive enum alternative:

```rust,ignore
struct Details { flags: [bool; 2] }
enum Choice { Idle, Filled(Details) }
```

`Idle` visits one value; `Filled` visits five. With the default 1,000,000-value
limit, a converted output capacity of 200,001 is rejected before dispatch even
if the implementation would have returned only cheap `Idle` elements. There
are no incoming output elements whose actual contents could justify admission.

| Position | Preparation rule |
| --- | --- |
| Copied result of a consuming call | Allocation-free `ComInlineValue::MAX_COST`; maximum count/depth must fit before invocation. |
| Converted output, consuming or ordinary | Finite allocation-free element maximum; capacity times maximum logical cost, depth, native bytes, and wire bytes must fit before invocation. |
| Identity-wire output | Recursive `BORROW_WIRE` proof, representable region and pre-reserved native capacity; no conversion scratch. |
| Readable input | Existing checked decoding and actual-input resource policy. |
| Ordinary non-consuming copied return | Existing actual-output budget; recoverable output errors remain allowed. |

Declaration-time assertions reject consuming copied/output codecs with no finite
maximum. Both safe caller and raw provider also perform runtime admission before
owner adoption. Foreign callers cannot bypass it. Native layout may differ
between compilers; each side checks its own sizes. Existing bounds remain 8 MiB
of element storage per converted buffer, 1,000,000 visited values, depth 64.
They are not a cumulative allowance across all buffers in a method. Checked
multiplication rejects overflow. Empty output capacities consume no element work.

`MAX_COST` is an unsafe codec promise. Prechecked conversion remains recursive
and must validate restricted fields without allocating: booleans must be 0/1
and tags must select a valid alternative. The promise permits prepayment; it
does not permit skipping content validation. Native valid inline output covered
by its prepaid maximum cannot fail due to later allocation or budget exhaustion.

## Errors and ownership

- Preparation allocation failure: `NC_E_OUT_OF_MEMORY`, all original owners return,
  implementation never enters.
- Preparation bounds failure: `NC_E_OUT_OF_RANGE`, same ownership behavior.
- Implementation rejection: restores its returned owners; no output is published.
- Invalid successful consuming output: provider contract violation; generated
  clients panic rather than fabricate returned owners. An extern-C provider panic
  aborts. Owner commit cannot be rolled back from arbitrary implementation state.
- Invalid ordinary output: a checked status error; every output result is discarded.

This guarantees binding resource ordering and native publication, not application
state rollback. Provider implementations still control their own transactions.

## Why the previous guards were replaced

`a3de006b`/`272976f8` fixed unsafe restricted native slices. `5970db11` established
returning original owners on rejected consuming calls. `d586d06f`/`1ca86a32`
prepared output completion before adoption, fixing post-commit allocation failure.
Their composition still treated initialized mutable output as *input/output*:
encode dummy values, decode on provider, overwrite, encode back, decode on caller.

Directional output removes that redundant direction, the native mutable slice,
and provider scratch. It preserves checked C-layout wire values and the ownership
transaction instead of passing native Rust enums or allocator ownership across
independently built plugins. The removed generic wrapper and output guards have
no compatibility path in the current backend.

## Regression evidence

[Output primitive tests](../../crates/com/api/integration-tests/output_buffers.rs)
cover uninitialized/short prefixes, bad tags and nested booleans, count/pointer
shape, resource admission, allocation failure, reuse, and grouped publication.
[Generated-call tests](../../crates/com/api/integration-tests/output_completion.rs)
run the [shared fixture](../../crates/com/api/integration-tests/support/output_completion_fixture.rs)
through two independently [compiled IDL paths](../../crates/com/idl/tests/output_completion.rs).
They seed previous successful results, deny every preparation allocation independently, verify exact owner identity
and destruction, deny all allocation after implementation entry, reject excessive
maximum alternatives before entry, and exercise malformed successful output.

[Native C](../../crates/com/api/integration-tests/fixtures/native-c-abi/native/caller.c)
uses independent output signatures and raw storage. Readable-input regressions
still reject malformed bits before implementation entry; transformations now
supply explicit input plus separate output. See
[output ownership regressions](../../crates/com/api/integration-tests/output_completion.rs)
for executed behavior checks.
