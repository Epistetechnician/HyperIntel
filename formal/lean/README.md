# Lean proof: N-candidate router abstraction

Kernel-checked (Lean 4.30.0, Mathlib v4.30.0) properties of a handwritten abstraction of the router in `crates/hyperintel-market/src/routing.rs`, matching `../../crates/hyperintel-market/formal/policy_non_widening_v1.smt2`:

- `route_selected_eligible`: for a candidate list of **any length**, a selected candidate is eligible and has a cost (the Z3 file checks this for two candidates).
- `route_min`: the selected candidate has the minimal `rank` among all admissible candidates.
- `route_none_iff`: the router returns nothing exactly when no candidate is admissible.
- `route_two_selected_zero` / `route_two_selected_one`: on two candidates the model equals the SMT file's `selected_0` / `selected_1` definitions (tie to index 0 on `<=`, later candidate wins only on strict `<`).

Axioms used (`#print axioms`): only `propext` and `Quot.sound`. No `sorry`.

## What this does not show

It is an abstraction, not a proof about the Rust source: it does not show `routing.rs` implements this model. Rank abstracts the whole routing tuple into a total order, and no claim is made about the correctness of eligibility or cost inputs. Reviewing that the formal statements faithfully capture the intended property is a human step.

## Check

```sh
cd formal/lean && lake exe cache get && lake build
```
