import ComProof.Borrow

/-!
# Completion of COM calls

This model starts with abstract raw words, an abstract lookup of a readable
table's interface identity, and a native success or error status. It checks
which capability, if any, the receiver can adopt and which cleanup obligation
follows a rejected result. The lawful lender, ownership policy of a transferred
token, readable backing, and foreign code lifetime are premises. No memory
layout, pointer provenance, provider implementation, or actual deallocation is
proved by these definitions.
-/

namespace ComProof.Wire

open Borrow (Access)

/-- Exact identity comprises the nominal IID and the complete signature digest. -/
structure Identity where
  iid : Nat
  digest : Nat
  deriving DecidableEq, Repr

/-- An absent word models null. A present word is a symbolic address. -/
structure RawPair where
  objectWord : Option Nat
  tableWord : Option Nat
  deriving DecidableEq, Repr

def RawPair.null : RawPair := ⟨none, none⟩

/-- A shape-checked pair has both words. Identity is checked separately. -/
structure PresentPair where
  objectWord : Nat
  tableWord : Nat
  deriving DecidableEq, Repr

def PresentPair.toRaw (pair : PresentPair) : RawPair :=
  ⟨some pair.objectWord, some pair.tableWord⟩

inductive PairCheck where
  | absent
  | malformed
  | exact (pair : PresentPair)
  | wrongIdentity (pair : PresentPair)
  deriving DecidableEq, Repr

/-- Check both words before looking up table metadata. The lookup abstracts the
    trusted readability premise and has no authority over ownership or access. -/
def inspectPair (identityAt : Nat → Identity) (expected : Identity) : RawPair → PairCheck
  | ⟨none, none⟩ => .absent
  | ⟨some objectWord, some tableWord⟩ =>
      let pair : PresentPair := ⟨objectWord, tableWord⟩
      if identityAt tableWord = expected then .exact pair else .wrongIdentity pair
  | _ => .malformed

theorem exact_pair_preserves_words_and_identity
    {identityAt : Nat → Identity} {expected : Identity} {raw : RawPair} {pair : PresentPair}
    (checked : inspectPair identityAt expected raw = .exact pair) :
    raw = pair.toRaw ∧ identityAt pair.tableWord = expected := by
  rcases raw with ⟨objectWord, tableWord⟩
  cases objectWord with
  | none => cases tableWord <;> simp [inspectPair] at checked
  | some objectWord =>
      cases tableWord with
      | none => simp [inspectPair] at checked
      | some tableWord =>
          by_cases exactIdentity : identityAt tableWord = expected
          · simp [inspectPair, exactIdentity] at checked
            subst pair
            exact ⟨rfl, exactIdentity⟩
          · simp [inspectPair, exactIdentity] at checked

inductive NativeStatus where
  | success
  | failure (code : Nat)
  deriving DecidableEq, Repr

inductive Error where
  | native (code : Nat)
  | malformedPair
  | wrongIdentity
  | requiredPairAbsent
  | ownerHandshake
  | visitorCardinality
  deriving DecidableEq, Repr

inductive OwnerPolicy where
  | shared
  | boxed
  deriving DecidableEq, Repr

/-- The declaration selects ownership policy or borrow permission. The raw pair
    does not classify a pointer's backing. -/
inductive ResultKind where
  | borrowed (access : Access)
  | owned (policy : OwnerPolicy)
  deriving DecidableEq, Repr

structure OwnerToken where
  policy : OwnerPolicy
  pair : PresentPair
  deriving DecidableEq, Repr

inductive Capability where
  | borrowed (access : Access) (pair : PresentPair)
  | owned (token : OwnerToken)
  deriving DecidableEq, Repr

structure ResultCompletion where
  result : Except Error (Option Capability)
  /-- At most one rejected transfer. It must be released through its declared policy. -/
  rejectedOwner : Option OwnerToken

/-- Readable identity metadata is an external premise. A known boxed allocation
also has its own canonical catalog. A successful canonical lookup returns the
table from that catalog for the exact requested identity. -/
structure TableLookup where
  /-- The identity advertised by the incoming readable table. -/
  suppliedIdentity : Nat → Identity
  /-- Lookup in the immutable catalog of an already-established boxed backing. -/
  boxedCanonical : Nat → Identity → Except Error Nat

/-- Native failure supplies no result. On success, only a complete but mismatched
    owned pair carries a rejected transfer. A borrowed result never acquires one. -/
def completeResult (tables : TableLookup) (expected : Identity)
    (optional : Bool) (kind : ResultKind) (status : NativeStatus) (raw : RawPair) :
    ResultCompletion :=
  match status with
  | .failure code => ⟨.error (.native code), none⟩
  | .success =>
      match inspectPair tables.suppliedIdentity expected raw with
      | .absent =>
          if optional then ⟨.ok none, none⟩ else ⟨.error .requiredPairAbsent, none⟩
      | .malformed => ⟨.error .malformedPair, none⟩
      | .exact pair =>
          match kind with
          | .borrowed access => ⟨.ok (some (.borrowed access pair)), none⟩
          | .owned .shared => ⟨.ok (some (.owned ⟨.shared, pair⟩)), none⟩
          | .owned .boxed =>
              match tables.boxedCanonical pair.objectWord expected with
              | .ok tableWord => ⟨.ok (some (.owned ⟨.boxed, { pair with tableWord }⟩)), none⟩
              | .error error => ⟨.error error, some ⟨.boxed, pair⟩⟩
      | .wrongIdentity pair =>
          match kind with
          | .borrowed _ => ⟨.error .wrongIdentity, none⟩
          | .owned policy => ⟨.error .wrongIdentity, some ⟨policy, pair⟩⟩

theorem failed_result_ignores_output (tables : TableLookup) (expected : Identity)
    (optional : Bool) (kind : ResultKind) (code : Nat) (raw : RawPair) :
    completeResult tables expected optional kind (.failure code) raw =
      ⟨.error (.native code), none⟩ := rfl

theorem optional_absence_is_success (tables : TableLookup) (expected : Identity)
    (kind : ResultKind) :
    (completeResult tables expected true kind .success RawPair.null).result =
      .ok none := rfl

theorem required_absence_is_rejected (tables : TableLookup) (expected : Identity)
    (kind : ResultKind) :
    (completeResult tables expected false kind .success RawPair.null).result =
      .error .requiredPairAbsent := rfl

theorem checked_absence_iff_successful_optional_null (tables : TableLookup)
    (expected : Identity) (optional : Bool) (kind : ResultKind) (status : NativeStatus)
    (raw : RawPair) :
    (completeResult tables expected optional kind status raw).result = .ok none ↔
      status = .success ∧ optional = true ∧ raw = RawPair.null := by
  cases status with
  | failure _ => simp [completeResult]
  | success =>
      rcases raw with ⟨objectWord, tableWord⟩
      cases objectWord with
      | none => cases tableWord <;> cases optional <;>
          simp [completeResult, inspectPair, RawPair.null]
      | some objectWord =>
          cases tableWord with
          | none => simp [completeResult, inspectPair, RawPair.null]
          | some tableWord =>
              by_cases exactIdentity : tables.suppliedIdentity tableWord = expected <;>
                cases kind with
                | borrowed _ => simp [completeResult, inspectPair, exactIdentity, RawPair.null]
                | owned policy =>
                    cases policy with
                    | shared => simp [completeResult, inspectPair, exactIdentity, RawPair.null]
                    | boxed =>
                        cases canonical : tables.boxedCanonical objectWord expected <;>
                          simp [completeResult, inspectPair, exactIdentity, canonical, RawPair.null]

theorem borrowed_result_has_no_owned_effect (tables : TableLookup)
    (expected : Identity) (optional : Bool) (access : Access) (status : NativeStatus)
    (raw : RawPair) :
    (completeResult tables expected optional (.borrowed access) status raw).rejectedOwner =
      none ∧ ∀ token,
        (completeResult tables expected optional (.borrowed access) status raw).result ≠
          .ok (some (.owned token)) := by
  cases status with
  | failure _ => simp [completeResult]
  | success =>
      cases pairCheck : inspectPair tables.suppliedIdentity expected raw <;>
        cases optional <;> simp [completeResult, pairCheck]

theorem owned_identity_rejection_keeps_declared_policy
    {tables : TableLookup} {expected : Identity} {raw : RawPair} {pair : PresentPair}
    (wrong : inspectPair tables.suppliedIdentity expected raw = .wrongIdentity pair)
    (optional : Bool) (policy : OwnerPolicy) :
    completeResult tables expected optional (.owned policy) .success raw =
      ⟨.error .wrongIdentity, some ⟨policy, pair⟩⟩ := by
  simp [completeResult, wrong]

/-- The canonical table can differ from the supplied address. The adoption
keeps the object word and transfers the existing owner without a second retain. -/
theorem boxed_adoption_uses_canonical_table
    (checked : inspectPair tables.suppliedIdentity expected raw = .exact pair)
    (canonical : tables.boxedCanonical pair.objectWord expected = .ok tableWord)
    (optional : Bool) :
    completeResult tables expected optional (.owned .boxed) .success raw =
      ⟨.ok (some (.owned ⟨.boxed, { pair with tableWord }⟩)), none⟩ := by
  simp [completeResult, checked, canonical]

/-- Even an incoming table with the correct advertised identity can fail the
canonical lookup. The rejection carries the one declared boxed cleanup. -/
theorem canonical_rejection_keeps_boxed_cleanup
    (checked : inspectPair tables.suppliedIdentity expected raw = .exact pair)
    (canonical : tables.boxedCanonical pair.objectWord expected = .error error)
    (optional : Bool) :
    completeResult tables expected optional (.owned .boxed) .success raw =
      ⟨.error error, some ⟨.boxed, pair⟩⟩ := by
  simp [completeResult, checked, canonical]

inductive OwnerCompletion where
  | accepted
  | returned (code : Nat) (originals : List PresentPair)
  deriving DecidableEq, Repr

/-- The caller keeps the pre-call words. Every success slot must be null. On
    failure every slot must match both original words, in declaration order.
    The model assumes the original slots carried distinct lawful owners. -/
def completeOwners (originals : List PresentPair) (status : NativeStatus)
    (slots : List RawPair) : Except Error OwnerCompletion :=
  match status with
  | .success =>
      if slots = originals.map (fun _ => RawPair.null) then .ok .accepted
      else .error .ownerHandshake
  | .failure code =>
      if slots = originals.map PresentPair.toRaw then .ok (.returned code originals)
      else .error .ownerHandshake

theorem owner_success_iff_all_slots_cleared (originals : List PresentPair)
    (slots : List RawPair) :
    completeOwners originals .success slots = .ok .accepted ↔
      slots = originals.map (fun _ => RawPair.null) := by
  by_cases cleared : slots = originals.map (fun _ => RawPair.null) <;>
    simp [completeOwners, cleared]

theorem owner_failure_iff_all_original_pairs_returned (originals returned : List PresentPair)
    (code returnedCode : Nat) (slots : List RawPair) :
    completeOwners originals (.failure code) slots = .ok (.returned returnedCode returned) ↔
      slots = originals.map PresentPair.toRaw ∧ returnedCode = code ∧ returned = originals := by
  by_cases restored : slots = originals.map PresentPair.toRaw
  · simp [completeOwners, restored, eq_comm]
  · simp [completeOwners, restored]

/-- A callback records only whether it observed absence or visited a valid loan.
The callable loan belongs to the callback's live region, not this record. -/
inductive Delivery where
  | absent
  | visited
  deriving DecidableEq, Repr

/-- Screen the pair while the native callback's lender and backing are still
live. On successful presence the real callback runs the visitor in that scope.
This model records its completion rather than carrying out its user effects. -/
def recordDelivery (identityAt : Nat → Identity) (expected : Identity)
    (optional : Bool) (raw : RawPair) : Except Error Delivery :=
  match inspectPair identityAt expected raw with
  | .absent => if optional then .ok .absent else .error .requiredPairAbsent
  | .exact _ => .ok .visited
  | .malformed => .error .malformedPair
  | .wrongIdentity _ => .error .wrongIdentity

/-- After native return check the records, without reading the expired pair or
reviving callable authority. A detected protocol error cannot undo effects of
a visitor already invoked by a misbehaving producer. -/
def completeVisitor (status : NativeStatus) (deliveries : List (Except Error Delivery)) :
    Except Error Delivery :=
  match status, deliveries with
  | .success, [delivery] => delivery
  | .failure code, [] => .error (.native code)
  | _, _ => .error .visitorCardinality

/-- Success needs exactly one valid record from the callback's live region. -/
theorem visitor_success_iff_one_checked_delivery (status : NativeStatus)
    (deliveries : List (Except Error Delivery)) (result : Delivery) :
    completeVisitor status deliveries = .ok result ↔
      status = .success ∧ deliveries = [.ok result] := by
  cases status with
  | failure _ => cases deliveries <;> simp [completeVisitor]
  | success =>
      cases deliveries with
      | nil => simp [completeVisitor]
      | cons delivery rest => cases rest <;> simp [completeVisitor]

/-- A producer error is lawful only when it made zero deliveries. -/
theorem visitor_failure_iff_zero_deliveries (code : Nat)
    (deliveries : List (Except Error Delivery)) :
    completeVisitor (.failure code) deliveries = .error (.native code) ↔ deliveries = [] := by
  cases deliveries <;> simp [completeVisitor]

section Examples

private def expected : Identity := ⟨7, 19⟩

private def identityAt (tableWord : Nat) : Identity :=
  if tableWord = 30 then ⟨7, 20⟩
  else if tableWord = 31 then ⟨8, 19⟩
  else expected

-- An equivalent supplied table can have a different address. Only object 11
-- has the requested canonical slot in this example.
private def tables : TableLookup :=
  ⟨identityAt, fun objectWord _ =>
    if objectWord = 11 then .ok 22 else .error .wrongIdentity⟩

private def firstOwner : PresentPair := ⟨11, 20⟩
private def secondOwner : PresentPair := ⟨12, 21⟩

-- The same two words give different capabilities under different declarations.
example : (completeResult tables expected false (.owned .boxed) .success firstOwner.toRaw).result =
    .ok (some (.owned ⟨.boxed, ⟨11, 22⟩⟩)) := rfl

-- The incoming identity alone cannot manufacture a canonical projection.
example : completeResult tables expected false (.owned .boxed) .success secondOwner.toRaw =
    ⟨.error .wrongIdentity, some ⟨.boxed, secondOwner⟩⟩ := rfl

example : (completeResult tables expected false (.borrowed .exclusive) .success firstOwner.toRaw).result =
    .ok (some (.borrowed .exclusive firstOwner)) := rfl

example : (completeResult tables expected true (.borrowed .shared) (.failure 2) RawPair.null).result =
    .error (.native 2) := rfl

example : (completeResult tables expected true (.owned .boxed) .success ⟨some 11, none⟩).result =
    .error .malformedPair := rfl

example : (completeResult tables expected true (.borrowed .shared) .success ⟨none, some 30⟩).result =
    .error .malformedPair := rfl

example : (completeResult tables expected false (.owned .boxed) .success
    ⟨some 11, some 30⟩).rejectedOwner = some ⟨.boxed, ⟨11, 30⟩⟩ := rfl

example : (completeResult tables expected false (.borrowed .shared) .success
    ⟨some 11, some 31⟩).result = .error .wrongIdentity := rfl

example : completeOwners [firstOwner, secondOwner] .success [RawPair.null, RawPair.null] =
    .ok .accepted := rfl

example : completeOwners [firstOwner, secondOwner] .success [RawPair.null, secondOwner.toRaw] =
    .error .ownerHandshake := rfl

example : completeOwners [firstOwner, secondOwner] (.failure 3) [firstOwner.toRaw, secondOwner.toRaw] =
    .ok (.returned 3 [firstOwner, secondOwner]) := rfl

-- A different table word can advertise the same contract. That still violates
-- the failure rule, which restores the original words, not just their identity.
example : identityAt 22 = identityAt firstOwner.tableWord := rfl

example : completeOwners [firstOwner, secondOwner] (.failure 3)
    [⟨some 11, some 22⟩, secondOwner.toRaw] = .error .ownerHandshake := rfl

example : completeOwners [firstOwner, secondOwner] (.failure 3) [secondOwner.toRaw, firstOwner.toRaw] =
    .error .ownerHandshake := rfl

private def callback (optional : Bool) (raw : RawPair) : Except Error Delivery :=
  recordDelivery identityAt expected optional raw

example : completeVisitor .success [callback true RawPair.null] = .ok .absent := rfl

-- On return, only the delivery record survives. It cannot call the borrowed object.
example : completeVisitor .success [callback false firstOwner.toRaw] = .ok .visited := rfl

example : completeVisitor .success [] = .error .visitorCardinality := rfl

example : completeVisitor .success [callback true RawPair.null, callback true firstOwner.toRaw] =
    .error .visitorCardinality := rfl

example : completeVisitor .success [callback true ⟨some 11, none⟩] =
    .error .malformedPair := rfl

example : completeVisitor (.failure 3) [] = .error (.native 3) := rfl

example : completeVisitor (.failure 3) [callback true firstOwner.toRaw] =
    .error .visitorCardinality := rfl

end Examples

end ComProof.Wire
