# Claim boundary

## Current supported claim

HyperIntel contains a local, pure-data implementation of one scoped
inference-market decision path. Its protocol tests and fixtures are hermetic and
can show that the implemented fixture obeys the named validation, routing,
arithmetic, digest, evidence-signature, signed-capacity-offer, typed
reliability, replay-transition, and deterministic reconstruction rules. The V2
path checks offer attribution against caller-supplied trust. The additive V3
path also checks that a caller-trusted verifier signed the exact typed
reliability artifact and that its provider, service, windows, counts, and
floor-PPM result match the offer.

The additive V4 path retains the full typed eligibility report for every
candidate and reconstructs either a selected route or a deterministic
all-rejected outcome. It binds both outcomes to a pure-data transition from the
exact caller-supplied buyer replay state to its deterministic successor.
Selected outcomes require metering and settlement records; denied outcomes
prohibit them. V4's evaluation profile distinguishes invalid assurance
evidence, policy rejection, and invalid offer authentication. V1, V2, and V3
remain compatible under their existing nonclaims and historical collapsed
assurance-rejection profile.

The formal contract test is an offline, environment-pinned local gate. With the
exact prerequisite `Z3 version 4.15.0 - 64 bit`, `tools/pinned-z3` checks the
reported solver version before running a handwritten two-candidate abstraction
bound to the recorded `routing.rs` source hash. Its supported result is `unsat`
for ineligible selection and `sat` for a deliberately weakened counterexample.
The resolver does not prove solver-binary provenance, and the model is not a
machine-checked Rust-source proof. No path grants external execution, replay,
provider, or settlement authority.

## Unsupported claims

The repository does not establish:

- model identity beyond supplied evidence;
- model or output correctness, truthfulness, safety, or alignment;
- confidential execution;
- provider authorization in the external world;
- capacity-offer signature verification in the V1 compatibility bundle, which
  retains its explicit unsigned-offer nonclaim;
- production offer-signing authority, key custody, or external provider
  authorization;
- buyer authentication or current, durable, atomic, rollback-resistant,
  concurrent, or externally consumed replay state; V4 advances only the exact
  caller-supplied pure-data cursor;
- truth, completeness, representative sampling, independence, or provenance of
  the source observations committed by a V3 reliability artifact;
- calibration, stationarity, causal validity, or future predictive accuracy of
  the artifact's historical success fraction;
- `AssuranceProperty::PolicyCompliance`; R0 rejects this assurance property
  because it has no policy-digest-bound assurance-evidence lane. Routing still
  evaluates the supplied policy's hard gates, but does not authenticate the
  buyer or release an external request;
- production authentication, availability, durability, or tenant isolation;
- independent reproduction or machine-checked Rust-source correspondence;
- HyperEVM contract correctness or deployment;
- HyperCore market availability;
- settlement finality, custody, payment, or value movement;
- legal, regulatory, or insurance status;
- production readiness or state of the art.

Digests prove byte commitments, not semantic truth. Signatures prove possession
of a signing key under the verifier's registry assumptions, not the truth of the
signed statement. Attestation, when added, must remain distinct from proof.
