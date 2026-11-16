# Typed lifetime contracts

ABI v3 remains WIP. Core generation **21** removes runtime allocation-policy
discrimination and shrinks `NcObjectTypeInfo` from five to four pointer-width
words. Core 20 is incompatible and rejected before storage/callback use. The
three-word lifetime headers, C interface slots, IID/full digests, IDL v14 and
NCB-v1 document encoding are unchanged. Rebuild hosts/providers together.

## Ownership is a precondition

Shared and boxed states are distinct concrete types. Shared `strong_count` is
numeric; boxed `owner_and_borrows` combines sole-owner presence and admitted
readers/writer. Each has its own weak count. Neither mutable state nor immutable
provider metadata carries an ownership tag. `NcObjectTypeInfo` holds only exact
interface tables/count and provider-local destroy/deallocate callbacks.

Generated construction selects storage, callbacks and root prefix together.
An ABI method/value position declares shared versus boxed ownership. Its producer
must supply that matching backing, even when a client later rejects the interface.
The unsafe raw constructor contract makes the same obligation explicit. A pointer
`.cast()` changes a Rust pointer type; the surrounding contract proves its backing.

| Operation | Required proof | Checks and ownership |
| --- | --- | --- |
| Shared raw weak | `NcWeakRaw.control: *mut NcSharedLifetimeState` points to retained shared backing | Check presence/exact interface; failed owned adoption releases one shared weak |
| Boxed owner/weak raw pair | ABI producer supplies genuine boxed backing and a matching provider table | Check shape and incoming/canonical interface identity; reject through boxed owner/weak cleanup |
| Borrowed weak wire | Declared shared/boxed backing, metadata and source reference retained throughout | Validate interfaces before retaining one independent weak; failure changes no source count |
| Typed handle or matching callback | Construction, a valid owner or the unsafe matched-provider contract | Use its own state directly; no ownership classification |
| Boxed method entry | Retained boxed backing through the complete guard/drop call | Atomically check owner presence and reader/writer conflicts before any value loan |
| Generic strong/scoped interface | Matching lifecycle table and retained lender/owner | Use its actual vtable; no common allocation header is implied |

Wrong-policy tokens are ABI producer contract violations. There is no SDK
promise to accept, classify, reject and clean up a shared token as boxed or the
reverse. Earlier provisional mixed-policy contracts and their tests are
superseded. Do not reintroduce that promise through a helper, a metadata tag,
function-address comparison, or a second checked/trusted wrapper family.

Null/half-null boxed weak shapes transfer nothing and return `NC_E_POINTER`
without dereferencing either pointer. Present tokens require valid declared
backing. These checks do not authenticate arbitrary native pointers or providers.

## Interface validation is independent

A checked boxed owner/weak pair's supplied table identity and the allocation's canonical
interface list are separate authorities. Validate the supplied IID/full digest,
then canonical membership and identity. Success retains the canonical table;
equivalent copied tables with equal identities are allowed. A stale incoming
identity cannot be replaced silently, and a correct incoming identity cannot
invent canonical membership. Policy is already established before either check.

### Canonical owner admission and unchanged rejected inputs

Checked boxed-owner results cache the resolved canonical table after both identity
checks; rejection consumes the transferred owner exactly once. Resolving an owner
input instead returns that table without changing its caller-owned slot or counts.
The generator saves the result, validates all other arguments and admits the
receiver before taking any owner. The implementation then receives a canonical
owner, and successful transfer leaves the caller's slot null.

On method rejection, each returned owner must still have the admitted allocation
and canonical table. The thunk verifies that pair before restoring the original
caller pair, including an equivalent copied table. Replacing either the admitted
allocation or its table violates the returned-owner contract and terminates at the
non-unwinding provider boundary before a mismatched pair can be published. Early
validation/admission rejection never changes the slots. Multiple owners follow
these rules together, preserving the destination on rejection.

`ComBox::from_raw_owned` is an unchecked transfer and preserves its caller-proven
pair. Canonical caching is the checked ABI-adoption rule; it does not replace the
unsafe constructor's backing, exact-identity, threading and provider/code proofs.
Regenerate WIP Rust consumers: `__nc_validate_raw_input` now returns the resolved
canonical table. No C record/slot, interface digest or core-generation change is
required; ABI v3/core 21 remains WIP.

## Consuming boxed weak erasure

`WeakComBox::into_unknown(self)` selects the exact canonical IUnknown table while
its input still retains boxed storage, then moves that existing weak token into
the result. It changes neither the weak count nor the owner/access word and
never promotes or borrows the application value. It is valid while the owner is
live and after value destruction, including at the weak counter limit. Object
identity, threading permission and provider/code retention are preserved.

This consuming operation must not delegate ownership to borrowed `query(&self)`:
that operation creates another independently retained weak observer. `Clone` and
borrowed weak query still retain and preserve non-unwinding overflow behavior.

## Destruction and weak retention

```mermaid
flowchart TD
    S[Shared ABI position] --> SR[Known shared state]
    B[Boxed ABI position] --> BR[Known boxed state]
    SR --> SC[Direct shared counters and exact lookup]
    BR --> BC[Direct boxed owner/access and weak operations]
    SC -->|final strong release| D[Destroy value once]
    BC -->|owner removal| W[Reject new calls and wait for admitted calls]
    W --> D
    D --> I[Release implicit weak after destructor returns]
    I --> E[Remaining weak observers retain storage and metadata]
    I -->|no observers| F[Provider-local deallocation]
    E -->|final weak release| F
```

No branch determines whether one allocation is shared or boxed. Each operation
starts with that fact. Runtime liveness, borrow conflicts and count bounds remain
real uncertainties and are checked. Box destruction is synchronous. The implicit
weak protects storage through destruction of weak-to-self fields; final weak
release deallocates through its allocating provider exactly once.

Releasing operations take raw pointers so no `&self` argument remains protected
across freeing its allocation. Value loans end before guard unlock; retained slot
storage survives the complete guard/drop call. Interior mutability remains for
the actual COM aliasing/reentrancy boundary, not for discovering ownership policy.

Shared promotion acquires a nonzero strong count before exact interface lookup,
returning `Result<Option<ComRaw<IUnknown>>, NcStatus>`. Zero returns None before
negotiation. Failure releases its acquired shared owner; that rollback can be the
final owner after another release. Count overflow remains non-unwinding fail-stop.
Binding-owned C adapters translate the Rust result to their status/output slots;
there is no SDK weak operation table or erased promotion handshake.
