# Dropped objects and closed services

COM uses separate statuses for object ownership loss and logical service closure.
Both are ordinary failures; a caller must never infer receiver lifetime solely
from a method's returned status. A live receiver can propagate either error from
another object or service it used.

| Status | Contract | Examples |
| --- | --- | --- |
| `NC_E_DROPPED` (`-14`) | No live owner is available; new access or promotion is rejected. | An exclusive object's `ComBox` was dropped; a shared weak reference has no strong owner; a required weak dependency cannot upgrade. |
| `NC_E_CLOSED` (`-13`) | A service, session or channel has closed; its COM capability can still exist. | A disconnected transport or explicitly closed application service whose COM handle remains owned. |

Logical closure is application-defined state. A closed object can retain its
owner and still be promoted through a weak handle; its methods report closure.
Empty optional slots are not evidence of dropped ownership.

## Ownership and concurrency

Objects are allocated from fully initialized Rust values. Factories obtain weak
identities afterward and finish any private wiring before exposing a capability.
There is no cyclic-construction exception: a zero shared owner count or cleared
exclusive owner bit is terminal, and promotion cannot resurrect the value. See
[the construction contract](COM_OBJECT_CONSTRUCTION.md).

The common exclusive borrow-admission check returns `NC_E_DROPPED` as soon as
owner Drop clears the owner bit. Already admitted cross-thread calls can finish
while the dropping thread waits. The status rejects new calls; it does not claim
that an in-progress destructor has completed. Returning from owner Drop still
proves synchronous value/descendant destruction. Weak tokens retain bookkeeping,
not object values, and do not postpone that destruction.

Shared `nc_weak_upgrade` returns `NC_E_DROPPED` when no strong owner remains and
leaves its output empty. The Rust `WeakComPtr::upgrade` maps that result to
`Ok(None)`. A logically closed but strongly owned object still upgrades to
`Some`; its methods can then return `NC_E_CLOSED`. Interface-negotiation and other
failures remain errors. Neither status changes handle or vtable layout.

`WeakLocalCom<T>` preserves the concrete implementation established at local
construction. Its `upgrade` follows the same `Result<Option<_>, NcStatus>`
contract and acquires a `LocalCom<T>` strong owner before any value access.
Promotion can succeed while another call holds the object gate; conflicting
`try_borrow` or `try_borrow_mut` access then returns `NC_E_BUSY`. Interface owners
also keep local promotion possible. Weak clones and formatting never retain the
value, and neither a foreign strong nor weak interface proves a concrete Rust T.

## Error propagation

A live receiver may return `NC_E_DROPPED` from an expired dependency or
`NC_E_CLOSED` from a service it calls. Neither result proves the receiver itself
was destroyed. Use the relevant weak handle's promotion or checked call admission
to establish liveness, and propagate method errors independently.

## Verification boundaries

Real generated exclusive calls exercise owner destruction, unchanged rejected
owning inputs, local weak access, and concurrent admitted calls versus Drop.
Shared-object tests check successful promotion of a logically closed receiver and
failed promotion after final destruction. The [native C fixture](../../tests/com/integration-tests/native_c_abi.rs) checks
raw weak promotion and its status through the ABI. The
[weak-ownership tests](../../tests/com/integration-tests/weak_com.rs) verify that
a logically closed object remains promotable while strongly owned and expires
after its final strong owner is released.
