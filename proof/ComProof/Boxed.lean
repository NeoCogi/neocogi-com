import ComProof.Borrow

/-!
# Unique boxed ownership

One machine follows a single allocation identity. The owner has one holding
thread, and all value access depends on its Rust borrow tree. Retained raw bits
and a previous owner cannot add transitions. In particular there is no callable
boxed weak token.

The destroy step represents one logical destruction and reclamation. The
allocating provider still has to implement both effects with valid pointers.
-/

namespace ComProof.Boxed

open Borrow

/-- One live owner with its loans, or a terminal allocation identity. -/
inductive State where
  | owned (holder : Nat) (loans : Tree)
  | destroyed
  deriving DecidableEq, Repr

/-- Transfer moves the unique owner without changing the allocation identity. -/
inductive Action where
  | borrow (event : Event)
  | transfer (destination : Nat)
  | destroy
  deriving DecidableEq, Repr

/-- Moves and drops change the holder or consume ownership, unlike an event
inside the borrow tree. -/
def Action.changesOwner : Action → Bool
  | .borrow _ => false
  | .transfer _ | .destroy => true

/-- Ownership cannot be transferred or consumed while a loan is outstanding.
A move across threads requires the same evidence as moving a Rust box. -/
inductive Step (threading : Place → Threading) (root : Place) :
    State → Action → State → Prop where
  | borrow : Borrow.Step threading .exclusive root before event after →
      Step threading root (.owned holder before) (.borrow event) (.owned holder after)
  | transfer : (holder = destination ∨ (threading root).send = true) →
      Step threading root (.owned holder .idle) (.transfer destination) (.owned destination .idle)
  | destroy : Step threading root (.owned holder .idle) .destroy .destroyed

/-- Moves and drops require that all owner-dependent loans end, including
read-only loans of separately allocated child fields. -/
theorem owner_action_requires_idle (step : Step threading root (.owned holder loans) action next)
    (ownerAction : action.changesOwner = true) : loans = .idle := by
  cases step with
  | borrow => cases ownerAction
  | transfer => rfl
  | destroy => rfl

/-- A freed identity cannot be reused or destroyed again. Any new allocation
that happens to occupy the same address needs a separate identity. -/
theorem destroyed_is_terminal : ¬ Step threading root .destroyed action next := by
  intro step
  cases step

/-- Execution starts from a freshly initialized, uniquely owned value. -/
inductive Reachable (threading : Place → Threading) (root : Place) (creator : Nat) :
    State → Prop where
  | publish : Reachable threading root creator (.owned creator .idle)
  | next : Reachable threading root creator before → Step threading root before action after →
      Reachable threading root creator after

/-- Every reachable live allocation retains a well-formed borrow tree. -/
theorem reachable_loans_valid
    (reach : Reachable threading root creator state) :
    match state with
    | .destroyed => True
    | .owned _ loans => Valid .exclusive root loans := by
  induction reach with
  | publish => exact .idle
  | next _ step ih =>
      cases step with
      | borrow change => exact change.preserves ih
      | transfer => exact .idle
      | destroy => trivial

/-- Every reachable whole-value writer excludes all other usable claims. -/
theorem reachable_writer_is_alone
    (reach : Reachable threading root creator (.owned holder loans))
    (writer : (.exclusive, target) ∈ claims .exclusive root loans) :
    claims .exclusive root loans = [(.exclusive, target)] :=
  writer_is_only_claim (reachable_loans_valid reach) writer

end ComProof.Boxed
