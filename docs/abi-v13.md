# COM core ABI v13: exclusive owners and checked weak calls

Version 13 adds an exclusive lifetime policy to the common provider allocation.
It retains the object/table handle layout and the three-word lifetime header.
Plugins must agree on core ABI version, platform C ABI, pointer width, and
interface signatures; Rust compiler versions, optimization, and feature flags
must not redefine these records. Version 12 providers are rejected at plugin
validation, not adapted at runtime.

Shared references retain their existing policy. Exclusive objects have one
`ComBox` owner and any number of cached `WeakComBox` observers. The strong-state
word uses its two high bits for exclusive mode and owner presence; low bits count
admitted calls. Exclusive mode is permanent, including after value destruction.
New call acquisition and owner removal synchronize through the same atomic word.
The weak count and immutable class descriptor retain their previous roles.

All generated ABI entries obtain lifetime protection before accessing the Rust
value, then obtain the existing read/write call gate. They release the gate before
ending lifetime protection. Owner removal refuses new calls immediately, does
not wait, and destroys through the provider after the last admitted call ends.
Unpublished exclusive constructors have no owner bit and cannot be called.

On exclusive objects, the shared `AddRef` slot returns zero without retaining;
shared `QueryInterface` and shared weak downgrade return `NC_E_INVALID_ARG`.
Clients must never adopt an owning reference after rejection. Consuming exclusive
queries use the immutable class table without changing ownership. Weak exclusive
queries cache a table while retaining only a weak reference. Value codecs cannot
copy exclusive objects into `ComPtr` fields.

The existing `ComRef::add_ref` convenience method is shared-only and panics when
its precondition is violated. Boundary code that can receive either ownership
policy uses `try_add_ref` and propagates rejection. This distinction prevents a
borrow from silently granting another owner.

A weak exclusive call uses its cached table directly. Its provider must implement
the checked entry protocol; this is an unsafe provider obligation, not a promise
that arbitrary pointers can be validated. Foreign providers use equivalent C
layout and synchronization and retain code until the final weak token is gone.

The complete UI ownership rationale and migration status are recorded in
[Exclusive widget ownership](../../../docs/EXCLUSIVE_WIDGET_OWNERSHIP.md).
