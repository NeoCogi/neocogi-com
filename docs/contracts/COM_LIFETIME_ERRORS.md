# Dropped objects and closed services

COM uses separate statuses for object ownership loss and logical service closure.
Both are ordinary failures; a caller must never infer receiver lifetime solely
from a method's returned status. A live receiver can propagate either error from
another object or service it used.

| Status | Contract | Examples |
| --- | --- | --- |
| `NC_E_DROPPED` (`-14`) | No live owner is available; new access or promotion is rejected. | An exclusive widget's `ComBox` was dropped; a shared weak reference has no strong owner; a required weak runtime dependency cannot upgrade. |
| `NC_E_CLOSED` (`-13`) | A service, session or channel has closed; its COM capability can still exist. | Explicit runtime shutdown; a dismissed logical window; consumed menu-builder capability; a disconnected transport; explicitly closed shared-producer event port. |

`UI_E_CLOSED` aliases only the second meaning. Widget expiration is enforced by
COM before entering the implementation and returns `NC_E_DROPPED`. A detached
widget whose owner still exists may instead encounter `NC_E_CLOSED` from its
shut-down runtime. Empty optional slots are not evidence of dropped ownership.

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
not widget values, and do not postpone that destruction.

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

The shared-producer `EventPort` similarly distinguishes an expired weak origin
from its existing explicit terminal port state. Exclusive widget queues remain
ordinary owned fields. There is no widget `close()` phase to reinstate.

## Why these meanings were separated

`2cb90ec9` introduced weak widget dispatch snapshots and reused `NC_E_CLOSED` for
expired exclusive calls. It also suppressed that status in scheduling, treating
it as proof that the scheduled widget had disappeared. A live handler accessing
an expired child could return the same status, losing the error and allowing later
work to execute. The lifetime and scheduling policies had become coupled through
one ambiguous error name.

Receiver selection must use the existing weak observer's liveness. Once a live
receiver is called, the scheduler propagates every handler error. Merely adding
or renaming a status cannot establish which object failed. See the
dispatch contract.

## Verification boundaries

Real generated exclusive calls exercise owner destruction, unchanged rejected
owning inputs, local weak access, and concurrent admitted calls versus Drop.
Shared-object tests check successful promotion of a logically closed receiver and
failed promotion after final destruction. The native C fixture checks raw weak
promotion and its status through the ABI. A retained event-port test distinguishes
an expired origin from explicit closure and verifies neither failed call enqueues
a payload. UI tests distinguish destroyed widgets from retained closed windows,
child-store views, and detached widgets whose runtime has shut down.

Whole-context `Ui::close()` is terminal teardown, without
per-window lifecycle posting. It preflights busy widget trees before committing;
retained services then report `NC_E_CLOSED`, while destroyed widget observers
report `NC_E_DROPPED`. Ordinary individual-window removal still reports delivery
errors after its committed transition.
