# Typed lifetime contracts

ABI v4 uses the three-word shared header, one-word boxed metadata prefix and
four-word `NcObjectTypeInfo`. Rebuild hosts and providers using the same ABI
version. [Current ABI authorities](../abi.md#abi-identities-and-versions)

## Ownership is a precondition

Shared state counts strong and weak owners. Boxed state contains immutable
provider metadata. Ownership comes from construction and the declared ABI
position. Neither header carries a runtime ownership tag.

Generated construction selects storage, callbacks and root prefix together.
An ABI method/value position declares shared versus boxed ownership. Its producer
must supply that matching backing, even when a caller later rejects the interface.
The unsafe raw constructor contract makes the same obligation explicit. A pointer
`.cast()` changes a Rust pointer type; the surrounding contract proves its backing.

| Operation | Required proof | Checks and ownership |
| --- | --- | --- |
| Shared raw weak | `NcSharedWeakRaw.control: *mut NcSharedLifetimeState` points to retained shared backing | Check presence/exact interface; failed owned adoption releases one shared weak |
| Boxed owner raw pair | ABI producer supplies genuine boxed backing and a matching provider table | Check shape and incoming/canonical interface identity; reject through boxed owner cleanup |
| Borrowed weak wire | Declared shared backing, metadata and source reference retained throughout | Validate interfaces before retaining one independent weak; failure changes no source count |
| Typed handle or matching callback | Construction, a valid owner or the unsafe matched-provider contract | Use its own state directly; no ownership classification |
| Boxed method entry | Live owner or a loan derived from it | Form the actual shared or exclusive reference required by the slot |
| Generic strong/scoped interface | Matching lifecycle table and retained lender/owner | Use its actual vtable; no common allocation header is implied |

Wrong-policy tokens are ABI producer contract violations. There is no SDK
promise to accept, classify, reject and clean up a shared token as boxed or the
reverse. Earlier provisional mixed-policy contracts and their tests are
superseded. Do not reintroduce that promise through a helper, a metadata tag,
function-address comparison, or a second checked/trusted wrapper family.

Null/half-null boxed owner shapes transfer nothing and return `NC_E_POINTER`
without dereferencing either pointer. Present tokens require valid declared
backing. These checks do not authenticate arbitrary native pointers or providers.

## Interface validation is independent

A checked boxed owner's supplied table identity and the allocation's canonical
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
required by canonical table caching.

## Destruction and weak retention

Boxed release consumes the sole owner after its loans end. The allocating
provider destroys the value and frees its storage. Native code must respect
owner uniqueness, the exact pair, and the joint access of the receiver,
arguments and returned loans. A raw address grants no independent access.

Shared promotion acquires a nonzero strong count before exact interface
lookup. Zero means absence before negotiation. Failure releases the
provisional strong owner. Its rollback can perform the final release. The
shared implicit weak reference keeps storage live through destruction.
When the final weak leaves, the allocating provider frees storage. Counters
retain their bounds and fail-stop exhaustion policy.

Destruction and reclamation use raw storage pointers. They must carry no
Rust reference protecting a value across its destruction or storage release.
The [Lean models](../../proof/README.md) cover the contractual transitions.
