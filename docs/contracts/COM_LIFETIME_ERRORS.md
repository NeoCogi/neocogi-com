# Dropped objects and closed services

COM uses separate statuses for object ownership loss and logical service closure.
Both are ordinary failures; a caller must never infer receiver lifetime solely
from a method's returned status. A live receiver can propagate either error from
another object or service it used.

| Status | Contract | Examples |
| --- | --- | --- |
| `NC_E_DROPPED` (`-14`) | No live owner is available; new access or promotion is rejected. | A shared weak reference has no strong owner or a required weak dependency cannot upgrade. |
| `NC_E_CLOSED` (`-13`) | A service, session or channel has closed; its COM capability can still exist. | A disconnected transport or explicitly closed application service whose COM handle remains owned. |

Logical closure is application-defined state. A closed object can retain its
owner and still be promoted through a weak handle; its methods report closure.
Empty optional slots are not evidence of dropped ownership.

## Ownership and concurrency

Objects are allocated from fully initialized Rust values. Shared factories obtain
weak identities afterward and finish any private wiring before exposing a
capability. A zero shared owner count is terminal. Weak tokens keep the control
allocation available for promotion and accounting. The [construction contract](COM_OBJECT_CONSTRUCTION.md)
defines when factories may publish their interfaces.

`ComBox` is the sole boxed owner. Rust requires its loans and their scoped
workers to end before owner destruction. Native callers must uphold the same
rule. Dropping the owner synchronously destroys the value and its owned
descendants. A raw pair saved past the end of its loan is outside the ABI
contract.

Shared weak promotion is an internal typed Rust operation returning `Ok(None)`
when no strong owner remains, before interface negotiation. Binding-owned C
adapters map that result to `NC_E_DROPPED` and leave their output empty. A logically closed but strongly owned object still upgrades to
`Some`; its methods can then return `NC_E_CLOSED`. Interface-negotiation and other
failures remain errors. Neither status changes handle or vtable layout.

`WeakLocalCom<T>` preserves the concrete implementation established at local
construction. Its `upgrade` follows the same `Result<Option<_>, NcStatus>`
contract and acquires a `LocalCom<T>` strong owner before any value access.
Interface owners keep local promotion possible. The local owner gives shared
access to its concrete value through `get()`. Providers can use a field guard
and report `NC_E_BUSY` when the operation needs conflicting access. Weak clones
and formatting operate on the control allocation. Concrete Rust type evidence
comes from local construction.

## Error propagation

A live receiver may return `NC_E_DROPPED` from an expired dependency or
`NC_E_CLOSED` from a service it calls. Neither result proves the receiver itself
was destroyed. A strong owner or an active loan supplies live access. Promote
a weak handle before using the target, and propagate method errors independently.

## Verification boundaries

Real generated exclusive calls exercise owner destruction, unchanged rejected
owning inputs, and joined workers using owner-dependent loans.
Shared-object tests check successful promotion of a logically closed receiver and
failed promotion after final destruction. The [native C fixture](../../tests/com/integration-tests/native_c_abi.rs) checks
raw weak promotion and its status through the ABI. The
[weak-ownership tests](../../tests/com/integration-tests/weak_com.rs) verify that
a logically closed object remains promotable while strongly owned and expires
after its final strong owner is released.
