import ComProof.Boxed

/-!
# Borrow witness traces

These examples instantiate the generic transitions with actual lender and child
paths. They fail to elaborate if a positive step is accidentally removed or a
negative example stops following from the rules. A native endpoint must still
implement the trace with valid generated bindings and pointer provenance.
-/

namespace ComProof.BorrowExamples

open Borrow

/-- This owner can be transferred, but its shared references stay on-thread. -/
private def mode (_ : Place) : Threading := ⟨true, false⟩

/-- Root place for the boxed parent. -/
private def root : Place := []

/-- Leaf place held in a field of the parent. -/
private def field : Place := [0]

/-- The field is reachable only from this parent in the example. -/
private theorem field_within : Within root field := ⟨[0], rfl⟩

/-- Ordinary native output comes from an exclusive receiver and is read-only.
The parent can resume only once the new borrower releases the result. -/
private def readResult : Tree := .writing field .shared .ordinary .idle

example : Boxed.Reachable mode root 0 (.owned 0 readResult) :=
  .next .publish (.borrow (.writeIdle field_within trivial))

example : Borrow.Step mode .exclusive root readResult .nativeReturn readResult := .returnWrite

example : lenderAccess .exclusive readResult = none ∧
    claims .exclusive root readResult = [(.shared, field)] := ⟨rfl, rfl⟩

example : ¬ Boxed.Step mode root (.owned 0 readResult) .destroy .destroyed := by
  intro destroy
  have idle := Boxed.owner_action_requires_idle destroy (by decide)
  contradiction

example : ¬ Boxed.Step mode root (.owned 0 readResult) (.transfer 1) (.owned 1 .idle) := by
  intro transfer
  have idle := Boxed.owner_action_requires_idle transfer (by decide)
  contradiction

example : Boxed.Reachable mode root 0 .destroyed :=
  .next (.next (.next .publish (.borrow (.writeIdle (returned := .shared) (scope := .ordinary) field_within trivial)))
    (.borrow (Borrow.Step.endWrite (threading := mode) (place := root)
      (target := field) (returned := .shared) (scope := .ordinary)))) .destroy

/-- The guarded visitor first opens a loan of its own. Within that loan it can
transfer an exclusive reborrow to a worker, even with Send-only evidence. -/
private def visitor : Tree := .writing field .exclusive .visitor .idle

/-- The worker's edge remains installed until its scope is joined. -/
private def worker : Tree := .writing field .exclusive .worker .idle

/-- The enclosing visitor cannot be released while its worker is live. -/
private def running : Tree := .writing field .exclusive .visitor worker

example : Borrow.Step mode .exclusive root .idle (.issue .visitor) visitor :=
  .writeIdle field_within trivial

example : Borrow.Step mode .exclusive root visitor (.issue .worker) running :=
  .inWriter (.writeIdle (within_self field) rfl)

example : ¬ Borrow.Step mode .exclusive root running (.finish .visitor) .idle := by
  intro endVisit
  have idle := (ending_exclusive endVisit).1
  contradiction

-- An ordinary borrow-end event cannot stand in for completion of the visitor.
example : ¬ Borrow.Step mode .exclusive root visitor (.finish .ordinary) .idle := by
  intro releaseLoan
  have wrongScope := (ending_exclusive releaseLoan).2
  contradiction

example : Trace mode .exclusive root running [.finish .worker, .finish .visitor] .idle :=
  .next (.next .refl (.inWriter .endWrite)) .endWrite

/-- Two read borrowers remain independently usable. No writer can start through
the parent until the last reader releases its loan. -/
private def reader : Tree := .reading field .ordinary .idle .idle
private def readers : Tree := .reading field .ordinary .idle reader

example : Trace mode .exclusive root .idle [.issue .ordinary, .issue .ordinary] readers :=
  .next (.next .refl (.readIdle (scope := .ordinary) field_within trivial))
    (.readMore (scope := .ordinary) field_within trivial)

example : claims .exclusive root readers =
    [(.shared, field), (.shared, field), (.shared, root)] := rfl

example : ¬ Borrow.Step mode .exclusive root readers (.issue .ordinary) (.writing root .exclusive .ordinary .idle) := by
  intro conflict
  cases conflict

example : Trace mode .exclusive root readers [.finish .ordinary, .finish .ordinary] .idle :=
  .next (.next .refl .endRead) .endRead

end ComProof.BorrowExamples
