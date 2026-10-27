import Mathlib

/-!
# N-candidate abstraction of the HyperIntel router (policy non-widening)

Abstraction of `routing.rs` matching `crates/hyperintel-market/formal/policy_non_widening_v1.smt2`:
a candidate has `eligible`, `hasCost` and a nonnegative total-order `rank`; the router filters on
`eligible ∧ hasCost`, then minimizes `rank`, preferring the earlier candidate on ties (index 0
is kept on `≤`, a later one wins only on strict `<`).

This file proves the property the SMT file checks for two candidates for lists of any length, plus
minimality and totality, and shows that on two candidates the model coincides with the SMT
`selected_0` / `selected_1` definitions. It is a handwritten abstraction: it says nothing about
the Rust source.
-/

namespace EvidenceLean.Router

structure Cand where
  eligible : Bool
  hasCost : Bool
  rank : ℕ

def adm (c : Cand) : Bool := c.eligible && c.hasCost

/-- One fold step: keep the incumbent unless the new admissible candidate is strictly cheaper. -/
def step (acc : Option (ℕ × Cand)) (x : ℕ × Cand) : Option (ℕ × Cand) :=
  if adm x.2 then
    match acc with
    | none => some x
    | some a => if x.2.rank < a.2.rank then some x else some a
  else acc

/-- Scan from index `i` with incumbent `acc`. -/
def go : ℕ → Option (ℕ × Cand) → List Cand → Option (ℕ × Cand)
  | _, acc, [] => acc
  | i, acc, c :: cs => go (i + 1) (step acc (i, c)) cs

/-- The router: index and candidate of the selection, if any. -/
def route (cs : List Cand) : Option (ℕ × Cand) := go 0 none cs

/-- Invariant: every incumbent is admissible. -/
theorem go_admissible :
    ∀ (cs : List Cand) (i : ℕ) (acc : Option (ℕ × Cand)),
      (∀ a, acc = some a → adm a.2 = true) →
      ∀ r, go i acc cs = some r → adm r.2 = true := by
  intro cs
  induction cs with
  | nil => intro i acc h r hr; exact h r hr
  | cons c cs ih =>
    intro i acc h r hr
    refine ih (i + 1) (step acc (i, c)) ?_ r hr
    intro a ha
    unfold step at ha
    split_ifs at ha with hadm
    · cases acc with
      | none => simp at ha; subst ha; simpa using hadm
      | some b =>
        simp only at ha
        split_ifs at ha with hlt
        · simp at ha; subst ha; simpa using hadm
        · simp at ha; subst ha; exact h b rfl
    · exact h a ha

/-- Policy non-widening for any number of candidates: the selected candidate is eligible
(and has a cost). -/
theorem route_selected_eligible (cs : List Cand) (r : ℕ × Cand) (h : route cs = some r) :
    r.2.eligible = true ∧ r.2.hasCost = true := by
  have hadm := go_admissible cs 0 none (by intro a ha; simp at ha) r h
  simpa [adm, Bool.and_eq_true] using hadm


/-- Once an incumbent exists the scan never loses it, and its rank never increases. -/
theorem step_some_le (a : ℕ × Cand) (x : ℕ × Cand) :
    ∃ a', step (some a) x = some a' ∧ a'.2.rank ≤ a.2.rank := by
  unfold step
  by_cases hadm : adm x.2 = true
  · by_cases hlt : x.2.rank < a.2.rank
    · exact ⟨x, by simp [hadm, hlt], le_of_lt hlt⟩
    · exact ⟨a, by simp [hadm, hlt], le_refl _⟩
  · exact ⟨a, by simp [hadm], le_refl _⟩

/-- An admissible new candidate leaves an incumbent whose rank is at most its own. -/
theorem step_admissible_le (acc : Option (ℕ × Cand)) (x : ℕ × Cand) (hadm : adm x.2 = true) :
    ∃ a', step acc x = some a' ∧ a'.2.rank ≤ x.2.rank := by
  unfold step
  cases acc with
  | none => exact ⟨x, by simp [hadm], le_refl _⟩
  | some a =>
    by_cases hlt : x.2.rank < a.2.rank
    · exact ⟨x, by simp [hadm, hlt], le_refl _⟩
    · exact ⟨a, by simp [hadm, hlt], by omega⟩

theorem go_some_ne_none :
    ∀ (cs : List Cand) (i : ℕ) (a : ℕ × Cand), go i (some a) cs ≠ none := by
  intro cs
  induction cs with
  | nil => intro i a; simp [go]
  | cons c cs ih =>
    intro i a
    obtain ⟨a', ha', _⟩ := step_some_le a (i, c)
    simp only [go, ha']
    exact ih _ _

/-- Minimality: the selection's rank is at most that of every admissible candidate scanned. -/
theorem go_min :
    ∀ (cs : List Cand) (i : ℕ) (acc : Option (ℕ × Cand)) (r : ℕ × Cand),
      go i acc cs = some r →
      (∀ a, acc = some a → r.2.rank ≤ a.2.rank) ∧
      (∀ c ∈ cs, adm c = true → r.2.rank ≤ c.rank) := by
  intro cs
  induction cs with
  | nil =>
    intro i acc r h
    refine ⟨?_, by intro c hc; simp at hc⟩
    intro a ha
    simp [go, ha] at h
    subst h
    exact le_refl _
  | cons c cs ih =>
    intro i acc r h
    obtain ⟨ih1, ih2⟩ := ih (i + 1) (step acc (i, c)) r h
    constructor
    · intro a ha
      subst ha
      obtain ⟨a', ha', hle⟩ := step_some_le a (i, c)
      exact le_trans (ih1 a' ha') hle
    · intro c' hc' hadm'
      rcases List.mem_cons.mp hc' with rfl | hmem
      · obtain ⟨a', ha', hle⟩ := step_admissible_le acc (i, c') hadm'
        exact le_trans (ih1 a' ha') hle
      · exact ih2 c' hmem hadm'

theorem route_min (cs : List Cand) (r : ℕ × Cand) (h : route cs = some r) :
    ∀ c ∈ cs, adm c = true → r.2.rank ≤ c.rank :=
  (go_min cs 0 none r h).2

/-- Totality: the router returns nothing exactly when no candidate is admissible. -/
theorem go_none_iff :
    ∀ (cs : List Cand) (i : ℕ),
      go i none cs = none ↔ ∀ c ∈ cs, adm c = false := by
  intro cs
  induction cs with
  | nil => intro i; simp [go]
  | cons c cs ih =>
    intro i
    by_cases hadm : adm c = true
    · have hs : step none (i, c) = some (i, c) := by simp [step, hadm]
      simp only [go, hs]
      constructor
      · intro h; exact absurd h (go_some_ne_none cs (i + 1) (i, c))
      · intro h; exact absurd (h c (by simp)) (by simp [hadm])
    · have hf : adm c = false := by simpa using hadm
      have hs : step none (i, c) = none := by simp [step, hf]
      simp only [go, hs]
      rw [ih (i + 1)]
      constructor
      · intro h c' hc'
        rcases List.mem_cons.mp hc' with rfl | hm
        · exact hf
        · exact h c' hm
      · intro h c' hc'
        exact h c' (List.mem_cons_of_mem _ hc')

theorem route_none_iff (cs : List Cand) :
    route cs = none ↔ ∀ c ∈ cs, adm c = false :=
  go_none_iff cs 0

/-- The model on two candidates is exactly the SMT abstraction's `selected_0` / `selected_1`. -/
theorem route_two_selected_zero (c0 c1 : Cand) :
    route [c0, c1] = some (0, c0) ↔
      (adm c0 = true ∧ (adm c1 = false ∨ c0.rank ≤ c1.rank)) := by
  cases h0 : adm c0 <;> cases h1 : adm c1 <;> by_cases hl : c1.rank < c0.rank <;>
    simp [route, go, step, h0, h1, hl] <;> omega

theorem route_two_selected_one (c0 c1 : Cand) :
    route [c0, c1] = some (1, c1) ↔
      (adm c1 = true ∧ (adm c0 = false ∨ c1.rank < c0.rank)) := by
  cases h0 : adm c0 <;> cases h1 : adm c1 <;> by_cases hl : c1.rank < c0.rank <;>
    simp [route, go, step, h0, h1, hl]

end EvidenceLean.Router

#print axioms EvidenceLean.Router.route_selected_eligible
#print axioms EvidenceLean.Router.route_min
#print axioms EvidenceLean.Router.route_none_iff
#print axioms EvidenceLean.Router.route_two_selected_zero
#print axioms EvidenceLean.Router.route_two_selected_one
