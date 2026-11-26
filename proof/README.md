# NeoCogi COM proofs

This independent Lean 4 project checks the proposed Box-like COM ownership
contract. The runtime in the current checkout still uses ABI v5 WIP. The
[current ABI specification](../docs/abi.md) describes that implementation.

From this directory, run `lake build`. Elan selects the Lean version in
`lean-toolchain`. The project requires only Lean's standard library and treats
proof warnings as errors.

[Borrow.lean](ComProof/Borrow.lean) defines the reusable scoped borrow calculus.
[Boxed.lean](ComProof/Boxed.lean) models one linear owner and the effects of
moving or destroying it. [BorrowExamples.lean](ComProof/BorrowExamples.lean)
checks witness traces for ordinary returned loans, nested worker lifetimes,
and competing readers. Recorded trace events distinguish an ordinary native
return from ending a loan, finishing a visitor, and joining a worker.

The model reserves the source borrow while a child is live, even if that child
only grants read access or points to another allocation. It proves claims about
legal transitions, assuming every path to a boxed value is derived from the
owner or a valid reborrow. A copied raw pointer has no authority in the model.
Generated Rust lifetimes, native pointer provenance, layout, and progress of a
scoped worker require separate implementation evidence.
