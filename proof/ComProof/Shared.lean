import Std

/-!
# Shared ownership

This model describes one allocation identity at the linearization points of its
strong and weak operations. It splits a promotion into acquisition and lookup so
ordinary owners may release while the lookup is pending. The caller's borrowed
weak token stays live through that operation. The model tracks aggregate counts
under lawful handle use. Atomic ordering, pointer validity, and execution of
provider code are outside this model.
-/

namespace ComProof.Shared

/-- The last strong release starts destruction. Returning from the destructor
releases the implicit storage reference. -/
inductive Phase where
  | live
  | destroying
  | destroyed
  deriving DecidableEq, Repr

/-- `owners` counts published strong owners and `pending` counts strong owners
already acquired by weak promotions that have yet to negotiate an interface.
`weakOwners` excludes the implicit storage reference. -/
structure State where
  owners : Nat
  pending : Nat
  weakOwners : Nat
  phase : Phase
  deriving DecidableEq, Repr

namespace State

/-- Strong count represented in the shared allocation header. -/
def strong (s : State) : Nat := s.owners + s.pending

/-- The implicit weak reference protects storage through the destructor. -/
def implicitWeak (s : State) : Nat :=
  match s.phase with
  | .live | .destroying => 1
  | .destroyed => 0

/-- Weak count represented in the shared allocation header. Zero ends all lawful
access to storage and hands reclamation to the allocating provider. -/
def weak (s : State) : Nat := s.weakOwners + s.implicitWeak

/-- Initialization publishes one strong owner and the implicit weak reference. -/
def initial : State := ⟨1, 0, 0, .live⟩

/-- A strong release enters destruction exactly when its decrement leaves zero. -/
def closeIfLast (s : State) : State :=
  { s with phase := if s.strong = 0 then .destroying else .live }

@[simp] theorem strong_closeIfLast (s : State) : s.closeIfLast.strong = s.strong := rfl

@[simp] theorem weak_closeIfLast (s : State) : s.closeIfLast.weak = s.weakOwners + 1 := by
  unfold closeIfLast weak implicitWeak
  split <;> rfl

@[simp] theorem closeIfLast_live (s : State) :
    s.closeIfLast.phase = .live ↔ 0 < s.strong := by
  unfold closeIfLast
  split <;> simp_all <;> omega

end State

/-- The externally relevant events and the promotion and destructor boundaries. -/
inductive Action where
  | retainStrong
  | releaseStrong
  | retainWeak
  | releaseWeak
  | acquireForUpgrade
  | lookupSucceeded
  | lookupFailed
  | finishDestruction
  deriving DecidableEq, Repr

/-- A successful transition. The configured limit represents the maximum usable
header count. Exhaustion has no transition because it terminates the process.
Expiration has no upgrade acquisition. Only an acquired pending owner can
negotiate. -/
inductive Step (limit : Nat) : State → Action → State → Prop where
  | retainStrong {s : State} (owner : 0 < s.owners) (room : s.strong < limit) :
      Step limit s .retainStrong { s with owners := s.owners + 1 }
  | releaseStrong {s : State} (owner : 0 < s.owners) :
      Step limit s .releaseStrong ({ s with owners := s.owners - 1 } : State).closeIfLast
  | retainWeak {s : State} (anchor : 0 < s.owners ∨ 0 < s.weakOwners) (room : s.weak < limit) :
      Step limit s .retainWeak { s with weakOwners := s.weakOwners + 1 }
  -- Aggregate accounting keeps a weak anchor through every outstanding lookup.
  | releaseWeak {s : State} (token : 0 < s.weakOwners)
      (keepsAnchor : s.pending = 0 ∨ 1 < s.weakOwners) :
      Step limit s .releaseWeak { s with weakOwners := s.weakOwners - 1 }
  | acquireForUpgrade {s : State} (token : 0 < s.weakOwners) (live : 0 < s.strong)
      (room : s.strong < limit) :
      Step limit s .acquireForUpgrade { s with pending := s.pending + 1 }
  | lookupSucceeded {s : State} (acquired : 0 < s.pending) :
      Step limit s .lookupSucceeded { s with owners := s.owners + 1, pending := s.pending - 1 }
  | lookupFailed {s : State} (acquired : 0 < s.pending) :
      Step limit s .lookupFailed ({ s with pending := s.pending - 1 } : State).closeIfLast
  | finishDestruction {s : State} (destroying : s.phase = .destroying) :
      Step limit s .finishDestruction { s with phase := .destroyed }

/-- A finite sequence of successful events for the same allocation identity. -/
inductive Trace (limit : Nat) : State → List Action → State → Prop where
  | nil (s : State) : Trace limit s [] s
  | cons {s t u : State} {action : Action} {rest : List Action}
      (first : Step limit s action t) (later : Trace limit t rest u) :
      Trace limit s (action :: rest) u

/-- Header counts fit the configured bound. Zero strong count ends value access,
and an in-progress promotion requires the caller's weak anchor. -/
structure Valid (limit : Nat) (s : State) : Prop where
  strongBound : s.strong ≤ limit
  weakBound : s.weak ≤ limit
  liveIffStrong : s.phase = .live ↔ 0 < s.strong
  pendingAnchored : 0 < s.pending → 0 < s.weakOwners

theorem initial_valid {limit : Nat} (capacity : 0 < limit) : Valid limit State.initial := by
  refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [State.initial, State.strong, State.weak, State.implicitWeak] <;> omega

theorem step_valid {limit : Nat} {s t : State} {action : Action}
    (transition : Step limit s action t) (before : Valid limit s) : Valid limit t := by
  rcases before with ⟨strongBound, weakBound, live, pendingAnchored⟩
  cases transition with
  | retainStrong owner room =>
      refine ⟨?_, weakBound, ?_, pendingAnchored⟩
      · simp [State.strong] at room ⊢
        omega
      · have wasLive := live.mpr (by change 0 < _ + _; omega)
        change s.phase = .live ↔ 0 < s.owners + 1 + s.pending
        simp [wasLive]
        omega
  | releaseStrong owner =>
      refine ⟨?_, ?_, State.closeIfLast_live _, pendingAnchored⟩
      · rw [State.strong_closeIfLast]
        change s.owners - 1 + s.pending ≤ limit
        unfold State.strong at strongBound
        omega
      · rw [State.weak_closeIfLast]
        have wasLive := live.mpr (by change 0 < _ + _; omega)
        simpa [State.weak, State.implicitWeak, wasLive] using weakBound
  | retainWeak anchor room =>
      refine ⟨strongBound, ?_, live, ?_⟩
      · change s.weakOwners + 1 + s.implicitWeak ≤ limit
        unfold State.weak at room
        omega
      · simp
  | releaseWeak token keepsAnchor =>
      refine ⟨strongBound, ?_, live, ?_⟩
      · change s.weakOwners - 1 + s.implicitWeak ≤ limit
        unfold State.weak at weakBound
        omega
      · change 0 < s.pending → 0 < s.weakOwners - 1
        omega
  | acquireForUpgrade token positive room =>
      refine ⟨?_, weakBound, ?_, fun _ => token⟩
      · simp [State.strong] at room ⊢
        omega
      · have wasLive := live.mpr positive
        change s.phase = .live ↔ 0 < s.owners + (s.pending + 1)
        simp [wasLive]
        omega
  | lookupSucceeded acquired =>
      have count : s.owners + 1 + (s.pending - 1) = s.strong := by
        unfold State.strong
        omega
      refine ⟨?_, weakBound, ?_, fun _ => pendingAnchored acquired⟩
      · change s.owners + 1 + (s.pending - 1) ≤ limit
        rwa [count]
      · change s.phase = .live ↔ 0 < s.owners + 1 + (s.pending - 1)
        rwa [count]
  | lookupFailed acquired =>
      refine ⟨?_, ?_, State.closeIfLast_live _, fun _ => pendingAnchored acquired⟩
      · rw [State.strong_closeIfLast]
        change s.owners + (s.pending - 1) ≤ limit
        unfold State.strong at strongBound
        omega
      · rw [State.weak_closeIfLast]
        have wasLive := live.mpr (by change 0 < _ + _; omega)
        simpa [State.weak, State.implicitWeak, wasLive] using weakBound
  | finishDestruction destroying =>
      refine ⟨strongBound, ?_, ?_, pendingAnchored⟩
      · simp [State.weak, State.implicitWeak, destroying] at weakBound ⊢
        omega
      · simp [destroying] at live
        simpa [State.strong] using live

/-- Configured counter bounds and lifecycle invariants hold on entire traces. -/
theorem trace_valid {limit : Nat} {s t : State} {actions : List Action}
    (trace : Trace limit s actions t) (before : Valid limit s) : Valid limit t := by
  induction trace with
  | nil => exact before
  | cons first _ later => exact later (step_valid first before)

/-- A published allocation never exceeds either header counter bound. -/
theorem published_bounds {limit : Nat} {s : State} {actions : List Action}
    (capacity : 0 < limit) (trace : Trace limit State.initial actions s) :
    s.strong ≤ limit ∧ s.weak ≤ limit := by
  have valid := trace_valid trace (initial_valid capacity)
  exact ⟨valid.strongBound, valid.weakBound⟩

theorem step_terminal_zero {limit : Nat} {s t : State} {action : Action}
    (transition : Step limit s action t) (zero : s.strong = 0) : t.strong = 0 := by
  cases transition <;> simp [State.strong] at * <;> omega

/-- A final release forbids resurrection, even when weak tokens retain storage. -/
theorem terminal_zero {limit : Nat} {s t : State} {actions : List Action}
    (trace : Trace limit s actions t) (zero : s.strong = 0) : t.strong = 0 := by
  induction trace with
  | nil => exact zero
  | cons first _ later => exact later (step_terminal_zero first zero)

theorem upgrade_expires_before_lookup {limit : Nat} {s t : State} (zero : s.strong = 0) :
    ¬ Step limit s .acquireForUpgrade t := by
  intro transition
  cases transition with
  | acquireForUpgrade _ live _ => omega

theorem upgrade_at_capacity {limit : Nat} {s t : State} (full : s.strong = limit) :
    ¬ Step limit s .acquireForUpgrade t := by
  intro transition
  cases transition with
  | acquireForUpgrade _ _ room => omega

theorem no_lookup_without_acquisition {limit : Nat} {s t : State} (none : s.pending = 0) :
    (¬ Step limit s .lookupSucceeded t) ∧ (¬ Step limit s .lookupFailed t) := by
  constructor <;> intro transition <;> cases transition <;> omega

/-- Successful acquisition increases the header count before lookup publishes
any new owner. -/
theorem acquisition_adds_pending_owner {limit : Nat} {s t : State}
    (transition : Step limit s .acquireForUpgrade t) :
    t.pending = s.pending + 1 ∧ t.strong = s.strong + 1 ∧ t.owners = s.owners := by
  cases transition
  simp [State.strong, Nat.add_assoc]

/-- Pending negotiation itself pins a live value, even when another owner has
released the last externally published handle. -/
theorem pending_keeps_value_live {limit : Nat} {s : State} (valid : Valid limit s)
    (acquired : 0 < s.pending) : s.phase = .live := by
  apply valid.liveIffStrong.mpr
  simp [State.strong]
  omega

theorem successful_lookup_transfers_acquired_owner {limit : Nat} {s t : State}
    (transition : Step limit s .lookupSucceeded t) :
    t.owners = s.owners + 1 ∧ t.strong = s.strong := by
  cases transition
  simp [State.strong]
  omega

/-- An unsuccessful lookup consumes the pending owner. If it was the last
strong owner, rollback starts the normal destructor path. -/
theorem failed_lookup_releases_acquired_owner {limit : Nat} {s t : State}
    (transition : Step limit s .lookupFailed t) :
    t.owners = s.owners ∧ t.strong + 1 = s.strong ∧
      (t.strong = 0 → t.phase = .destroying) := by
  cases transition
  constructor
  · rfl
  constructor
  · rw [State.strong_closeIfLast]
    change s.owners + (s.pending - 1) + 1 = s.owners + s.pending
    omega
  · intro last
    simpa [State.closeIfLast] using last

/-- The last ordinary strong release starts destruction as soon as the count
reaches zero. -/
theorem final_strong_release_starts_destruction {limit : Nat} {s t : State}
    (transition : Step limit s .releaseStrong t) (last : s.strong = 1) :
    t.strong = 0 ∧ t.phase = .destroying := by
  cases transition with
  | releaseStrong owner =>
      have noRemaining : s.owners - 1 + s.pending = 0 := by
        unfold State.strong at last
        omega
      simp [State.closeIfLast, State.strong, noRemaining]

/-- Storage remains retained until the destructor has returned. -/
theorem destructor_retains_storage {s : State} (destroying : s.phase = .destroying) :
    0 < s.weak := by
  simp [State.weak, State.implicitWeak, destroying]

theorem finish_destruction_releases_implicit_weak {limit : Nat} {s t : State}
    (transition : Step limit s .finishDestruction t) :
    t.phase = .destroyed ∧ t.weak + 1 = s.weak := by
  cases transition with
  | finishDestruction destroying => simp [State.weak, State.implicitWeak, destroying]

/-- Once the destructor has returned, remaining weak operations cannot run it
again. -/
theorem destroyed_stays_destroyed {limit : Nat} {s t : State} {action : Action}
    (transition : Step limit s action t) (before : Valid limit s)
    (destroyed : s.phase = .destroyed) : t.phase = .destroyed := by
  have zero : s.strong = 0 := by
    have live := before.liveIffStrong
    simp [destroyed] at live
    omega
  cases transition <;> simp_all [State.strong] <;> omega

/-- Once no storage anchors remain, neither kind of token can be recovered. -/
theorem reclaimed_has_no_successful_step {limit : Nat} {s t : State} {action : Action}
    (valid : Valid limit s) (reclaimed : s.weak = 0) : ¬ Step limit s action t := by
  have destroyed : s.phase = .destroyed := by
    cases h : s.phase <;> simp_all [State.weak, State.implicitWeak]
  have zero : s.strong = 0 := by
    have live := valid.liveIffStrong
    simp [destroyed] at live
    omega
  intro transition
  cases transition <;> simp_all [State.strong, State.weak, State.implicitWeak] <;> omega

/-! These traces use the same transitions as the general theorems. In the
failure trace, a competing owner releases before negotiation. Rollback then
starts destruction itself, and the external weak owner is the final anchor. -/

example : Trace 2 State.initial [.retainWeak, .acquireForUpgrade, .lookupSucceeded]
    ⟨2, 0, 1, .live⟩ := by
  exact .cons (.retainWeak (by decide) (by decide))
    (.cons (.acquireForUpgrade (by decide) (by decide) (by decide))
      (.cons (.lookupSucceeded (by decide)) (.nil _)))

example : Trace 2 State.initial
    [.retainWeak, .acquireForUpgrade, .releaseStrong, .lookupFailed, .finishDestruction, .releaseWeak]
    ⟨0, 0, 0, .destroyed⟩ := by
  exact .cons (.retainWeak (by decide) (by decide))
    (.cons (.acquireForUpgrade (by decide) (by decide) (by decide))
      (.cons (.releaseStrong (by decide))
        (.cons (.lookupFailed (by decide))
          (.cons (.finishDestruction (by decide))
            (.cons (.releaseWeak (by decide) (by decide)) (.nil _))))))

/-- Dropping the last explicit weak token during value destruction does not free
storage before the implicit weak reference is released. -/
example : Trace 2 State.initial [.retainWeak, .releaseStrong, .releaseWeak]
    ⟨0, 0, 0, .destroying⟩ := by
  exact .cons (.retainWeak (by decide) (by decide))
    (.cons (.releaseStrong (by decide)) (.cons (.releaseWeak (by decide) (by decide)) (.nil _)))

example {s : State} : ¬ Step 2 ⟨0, 0, 1, .destroyed⟩ .acquireForUpgrade s :=
  upgrade_expires_before_lookup (by decide)

example {s : State} : ¬ Step 2 ⟨1, 0, 1, .live⟩ .retainWeak s := by
  intro transition
  cases transition with
  | retainWeak _ room => exact (by decide : ¬ 2 < 2) room

end ComProof.Shared
