# NeoCogi COM proofs

This Lean 4 project checks the Box-like COM ownership model used by ABI v6 WIP.
The [ABI specification](../docs/abi.md) describes the native implementation.

## Run the checks

From this directory, run:

```sh
lake build
python3 tests/run.py
```

Elan selects the Lean release in [lean-toolchain](lean-toolchain). The Lake
project uses Lean's standard library and treats warnings as errors. The
[Rust probes](tests/README.md) use installed Rust stable and 1.85.0, with
artifacts in a temporary directory. They compile and run positive examples and
require each invalid example to produce its expected diagnostic.

## What Lean checks

| Module | Contract |
| --- | --- |
| [Borrow](ComProof/Borrow.lean) | The reusable borrow library records source permissions, projected places, and live descendants. Well-formedness is preserved by local transitions. An independently usable whole-value writer is the only active target claim in its ownership root. |
| [Boxed](ComProof/Boxed.lean) | One linear owner per modeled allocation. Transfer and destruction require every dependent loan to finish. Destruction ends that allocation identity. |
| [BorrowExamples](ComProof/BorrowExamples.lean) | Receiver-bound read results from exclusive sources, multiple readers, and nested workers. Trace events distinguish an ordinary native return, ending a loan, finishing a visitor, and joining a worker. |
| [Shared](ComProof/Shared.lean) | Bounded strong and weak counts, terminal strong zero, retaining storage through destruction, and shared weak promotion acquiring before lookup and releasing on rejection. |
| [Wire](ComProof/Wire.lean) | Pair shape and exact interface identity, canonical boxed adoption, checked optional absence, declared ownership and exact consuming-owner rollback. Visitor completion checks callback records without returning callable authority. |

A returned child may be read-only or live at a separate address. The model
still retains the source receiver's borrow until the child ends. Worker entry
checks transfer or sharing evidence for the actual target. An ordinary native
return leaves the borrow tree intact. A visitor can end only after the scopes
within it finish.

## Proof boundary

`Borrow.lean` is a standalone model of the scoped Rust borrowing rules this ABI
needs. Rustc still supplies lifetime inference. The model reserves the whole
source when lending and does not enumerate all of Rust's legal disjoint-place
or two-phase borrow patterns. [Aeneas](https://github.com/AeneasVerif/aeneas#targeted-subset-and-current-limitations)
uses Rust's borrow discipline for its safe sequential translation. RustBelt
provides a [lifetime logic in Iris/Coq](https://people.mpi-sws.org/~dreyer/papers/rustbelt/paper.pdf).
Neither supplies a drop-in Lean proof of this native ownership and worker
contract.

The boxed proof assumes every valid route to the value derives from its sole
owner or a genuine reborrow. The transition system tracks that contractual
authority. A copied native address carries no authority. Generated Rust
bindings and native implementations still have to establish readable storage,
valid pointer provenance, exact layout and callback threading. The shared
model checks transitions at their abstract linearization points. It leaves
Rust atomic ordering and allocator behavior to implementation validation.
Worker completion remains a premise of a join. The proof does not establish
scheduler progress or verify an arbitrary native plugin.
