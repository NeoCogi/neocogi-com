import Std

/-!
# A scoped Rust borrow calculus

One root represents a unique owner or an existing lawful reference. A place is a
path within that root. An owned child reached through its parent can occupy a
descendant path even when its storage is a separate allocation. The model
conservatively reserves the source place for lending. It does not infer Rust
lifetimes or implement rustc's treatment of disjoint places and two-phase borrows.

The tree records live loans, not object addresses or data. Paths and the movement
of a linear token stand for proven provenance. All independently callable routes
to a boxed value must belong to the same tree for the exclusion theorem to apply.
-/

namespace ComProof.Borrow

/-- Shared authority permits shared receiver methods, including interior mutation.
Exclusive authority also permits whole-value mutable receiver methods. -/
inductive Access where
  | shared
  | exclusive
  deriving DecidableEq, Repr

/-- A path from one ownership root through its fields or projections. -/
abbrev Place := List Nat

/-- A loan can stay on the same target or project a descendant of its lender. -/
def Within (outer inner : Place) : Prop :=
  ∃ fields : Place, inner = outer ++ fields

/-- Every lender may reborrow its own place. -/
theorem within_self (place : Place) : Within place place :=
  ⟨[], by simp⟩

/-- Repeated projections cannot escape their original place. -/
theorem within_trans {outer mid inner : Place}
    (h₁ : Within outer mid) (h₂ : Within mid inner) : Within outer inner := by
  obtain ⟨a, rfl⟩ := h₁
  obtain ⟨b, rfl⟩ := h₂
  exact ⟨a ++ b, by simp [List.append_assoc]⟩

/-- Independent evidence for exclusive transfer and shared use on another thread. -/
structure Threading where
  /-- The provider's value and applicable destruction can move to another thread. -/
  send : Bool
  /-- The provider permits a shared reference to be used from another thread. -/
  sync : Bool
  deriving DecidableEq, Repr

/-- Moving a read loan relies on Sync. Moving an exclusive loan relies on Send. -/
def canSend (mode : Threading) : Access → Bool
  | .shared => mode.sync
  | .exclusive => mode.send

/-- Distinguish ordinary regions, native visits, and workers that must join. -/
inductive Scope where
  | ordinary
  | visitor
  | worker
  deriving DecidableEq, Repr

/-- A worker's permission comes from the target it uses, not its enclosing lender. -/
def permits (mode : Threading) (access : Access) : Scope → Prop
  | .ordinary | .visitor => True
  | .worker => canSend mode access = true

/-- Outstanding descendants of one lender.

`reading` keeps one child and a sibling list represented by another tree at the
same place. The sibling tree has only shared authority, including the residual
read authority of the lender. `writing` suspends the lender completely. Its child
may have a read-only result because the *source* borrow and output permission
are independent. The mode label distinguishes ordinary end, callback end and join. -/
inductive Tree where
  | idle
  | reading (place : Place) (scope : Scope) (child older : Tree)
  | writing (place : Place) (returned : Access) (scope : Scope) (child : Tree)
  deriving DecidableEq, Repr

/-- Independently usable whole-value authority at one point in a trace. A shared
claim can still perform whatever interior mutation its normal Rust API permits. -/
abbrev Claim := Access × Place

/-- The active frontier of the loan tree. An exclusive source has no active claim
of its own while delegated, even when its child only has shared permission. -/
def claims (access : Access) (place : Place) : Tree → List Claim
  | .idle => [(access, place)]
  | .reading target _ child older =>
      claims .shared target child ++ claims .shared place older
  | .writing target returned _ child => claims returned target child

/-- Direct access left with the original lender. The returned child's access is
recorded separately, so a read result from an exclusive receiver still suspends
all direct use of that receiver. -/
def lenderAccess (access : Access) : Tree → Option Access
  | .idle => some access
  | .reading .. => some .shared
  | .writing .. => none

/-- Even a read-only result keeps its exclusive receiver suspended. -/
theorem exclusive_source_suspends_lender (place : Place) (returned : Access)
    (scope : Scope) (child : Tree) :
    lenderAccess .exclusive (.writing place returned scope child) = none := rfl

/-- A shared reborrow pauses only the original holder's exclusive access. -/
theorem shared_source_allows_lender_read (access : Access) (place : Place)
    (scope : Scope) (child older : Tree) :
    lenderAccess access (.reading place scope child older) = some .shared := rfl

/-- Well-formed provenance and authority. The forest of older shared loans cannot
contain a writer. Only exclusive source authority can create a writing edge. -/
inductive Valid : Access → Place → Tree → Prop where
  | idle : Valid access place .idle
  | reading : Within place target → Valid .shared target child →
      Valid .shared place older → Valid access place (.reading target scope child older)
  | writing : Within place target → Valid returned target child →
      Valid .exclusive place (.writing target returned scope child)

/-- Trace events distinguish releasing an ordinary borrow, completing a visitor,
and joining a worker. A native return with a live ordinary result is separate
from releasing that result. -/
inductive Event where
  | issue (scope : Scope)
  | finish (scope : Scope)
  | nativeReturn
  deriving DecidableEq, Repr

/-- Local borrow transitions. A new loan checks the lender's availability, its
projection, and any worker evidence. The matching finish event requires all
descendants to have ended. Congruence records nested events without unlocking
the enclosing lender or changing sibling regions. -/
inductive Step (threading : Place → Threading) : Access → Place → Tree → Event → Tree → Prop where
  | readIdle : Within place target → permits (threading target) .shared scope →
      Step threading access place .idle (.issue scope) (.reading target scope .idle .idle)
  | readMore : Within place target → permits (threading target) .shared scope →
      Step threading access place (.reading old scopeOld child older) (.issue scope)
        (.reading target scope .idle (.reading old scopeOld child older))
  | writeIdle : Within place target → permits (threading target) returned scope →
      Step threading .exclusive place .idle (.issue scope) (.writing target returned scope .idle)
  | endRead : Step threading access place (.reading target scope .idle older) (.finish scope) older
  | endWrite : Step threading .exclusive place (.writing target returned scope .idle)
      (.finish scope) .idle
  | returnRead : Step threading access place (.reading target .ordinary .idle older)
      .nativeReturn (.reading target .ordinary .idle older)
  | returnWrite : Step threading .exclusive place (.writing target returned .ordinary .idle)
      .nativeReturn (.writing target returned .ordinary .idle)
  | inReader : Step threading .shared target before event after →
      Step threading access place (.reading target scope before older) event
        (.reading target scope after older)
  | inOlder : Step threading .shared place before event after →
      Step threading access place (.reading target scope child before) event
        (.reading target scope child after)
  | inWriter : Step threading returned target before event after →
      Step threading .exclusive place (.writing target returned scope before) event
        (.writing target returned scope after)

/-- Using a tree with shared authority cannot yield a whole-value exclusive claim. -/
theorem shared_no_writer {place : Place} {tree : Tree} (valid : Valid .shared place tree)
    (target : Place) : (.exclusive, target) ∉ claims .shared place tree := by
  induction tree generalizing place with
  | idle => simp [claims]
  | reading childPlace scope child older childIH olderIH =>
      cases valid with
      | reading _ childValid olderValid =>
          simpa [claims] using And.intro (childIH childValid) (olderIH olderValid)
  | writing => cases valid

/-- A shared-only tree can be used under an exclusive source that has lent out
shared references. The caller gets the original authority back after it drains. -/
theorem shared_valid_under (valid : Valid .shared place tree) :
    Valid access place tree := by
  cases valid with
  | idle => exact .idle
  | reading nested child older => exact .reading nested child older

/-- The frontier of a writer is a singleton. This proves exclusion for independently
usable reads and writes without assuming pairwise exclusion as a transition guard. -/
theorem writer_is_only_claim {access : Access} {place : Place} {tree : Tree}
    (valid : Valid access place tree) {target : Place}
    (writer : (.exclusive, target) ∈ claims access place tree) :
    claims access place tree = [(.exclusive, target)] := by
  induction tree generalizing access place with
  | idle => simpa [claims] using (List.mem_singleton.mp writer).symm
  | reading childPlace scope child older _ _ =>
      cases valid with
      | reading _ childValid olderValid =>
          exact False.elim (by
            rcases List.mem_append.mp writer with inChild | inOlder
            · exact shared_no_writer childValid target inChild
            · exact shared_no_writer olderValid target inOlder)
  | writing childPlace returned scope child childIH =>
      cases valid with
      | writing _ childValid => exact childIH childValid writer

/-- All usable claims refer to the root or a descendant of the root. This theorem
also applies when a read-only child is obtained from an exclusive source. -/
theorem claim_within (valid : Valid access place tree)
    (member : claim ∈ claims access place tree) : Within place claim.2 := by
  induction valid with
  | idle =>
      have eq := List.mem_singleton.mp member
      cases eq
      exact within_self _
  | reading nested _ _ childIH olderIH =>
      rcases List.mem_append.mp member with inChild | inOlder
      · exact within_trans nested (childIH inChild)
      · exact olderIH inOlder
  | writing nested _ childIH => exact within_trans nested (childIH member)

/-- Each local rule preserves the structural invariant. Starting/ending shared
siblings changes the residual authority only when a child is actually removed. -/
theorem Step.preserves (step : Step threading access place before event after)
    (valid : Valid access place before) : Valid access place after := by
  induction step with
  | readIdle nested _ => exact .reading nested .idle .idle
  | readMore nested _ =>
      cases valid with
      | reading current child older =>
          exact .reading nested .idle (.reading current child older)
  | writeIdle nested _ => exact .writing nested .idle
  | endRead =>
      cases valid with
      | reading _ _ older => exact shared_valid_under older
  | endWrite => exact .idle
  | returnRead | returnWrite => exact valid
  | inReader _ ih =>
      cases valid with
      | reading nested child older => exact .reading nested (ih child) older
  | inOlder _ ih =>
      cases valid with
      | reading nested child older => exact .reading nested child (ih older)
  | inWriter _ ih =>
      cases valid with
      | writing nested child => exact .writing nested (ih child)

/-- Finite traces retain the order of loan issuance, nested activity and release. -/
inductive Trace (threading : Place → Threading) (access : Access) (place : Place) :
    Tree → List Event → Tree → Prop where
  | refl : Trace threading access place tree [] tree
  | next : Trace threading access place first events before →
      Step threading access place before event after →
      Trace threading access place first (events ++ [event]) after

/-- A tree reached from a valid starting configuration retains valid authority. -/
theorem Trace.preserves (trace : Trace threading access place first events last)
    (valid : Valid access place first) : Valid access place last := by
  induction trace with
  | refl => exact valid
  | next _ step ih => exact step.preserves ih

/-- A shared owner can create arbitrary shared reborrow traces but no whole-value writer. -/
theorem Trace.shared_never_writes
    (trace : Trace threading .shared place .idle events last) (target : Place) :
    (.exclusive, target) ∉ claims .shared place last :=
  shared_no_writer (trace.preserves .idle) target

/-- Returning the ordinary result preserves the entire borrow tree, whether the
return occurs at the root or inside an enclosing borrower. -/
theorem Step.native_return_preserves (step : Step threading access place before event after)
    (returned : event = .nativeReturn) : after = before := by
  induction step with
  | returnRead | returnWrite => rfl
  | inReader _ ih | inOlder _ ih | inWriter _ ih => rw [ih returned]
  | readIdle | readMore | writeIdle | endRead | endWrite => cases returned

/-- Releasing an exclusive edge requires its matching finish event and an idle
child. In particular a nested worker must join before a visitor can finish. -/
theorem ending_exclusive
    (step : Step threading .exclusive place (.writing target returned scope child) event .idle) :
    child = .idle ∧ event = .finish scope := by
  cases step
  exact ⟨rfl, rfl⟩

/-- Starting a read worker at an idle lender requires Sync evidence for the
target. The root owner may have different thread permissions. -/
theorem read_worker_requires_sync
    (step : Step threading access place .idle (.issue .worker)
      (.reading target .worker .idle .idle)) : (threading target).sync = true := by
  cases step with
  | readIdle _ allowed => exact allowed

/-- Starting a worker through an exclusive source checks the returned loan's
permission. An exclusive source returning a shared loan still needs Sync. -/
theorem writing_worker_checks_returned_permission
    (step : Step threading .exclusive place .idle (.issue .worker)
      (.writing target returned .worker .idle)) :
    canSend (threading target) returned = true := by
  cases step with
  | writeIdle _ allowed => exact allowed

/-- A send-only target can transfer an exclusive loan. Transferring a shared
loan would require Sync. An exclusive borrower on the worker can still make a
local shared reborrow, as with Rust Cell. -/
theorem send_only_worker :
    permits ⟨true, false⟩ .exclusive .worker ∧
      ¬ permits ⟨true, false⟩ .shared .worker := by simp [permits, canSend]

/-- A sync-only target can lend shared access while its owner remains pinned. -/
theorem sync_only_worker :
    permits ⟨false, true⟩ .shared .worker ∧
      ¬ permits ⟨false, true⟩ .exclusive .worker := by simp [permits, canSend]

end ComProof.Borrow
