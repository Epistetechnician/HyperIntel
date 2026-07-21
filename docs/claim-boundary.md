# Claim boundary

## Current supported claim

HyperIntel contains a local, hermetic, pure-data implementation of one scoped
inference-market decision path. Its tests can show that the implemented fixture
obeys the named validation, routing, arithmetic, digest, evidence-signature,
signed-capacity-offer, typed reliability, and deterministic reconstruction
rules. The V2 path checks offer attribution against caller-supplied trust. The
additive V3 path also checks that a caller-trusted verifier signed the exact
typed reliability artifact and that its provider, service, windows, counts, and
floor-PPM result match the offer. Neither path grants external execution or
provider authority.

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
- consumed-nonce or replay-state enforcement beyond committing the buyer's
  supplied snapshot digest;
- truth, completeness, representative sampling, independence, or provenance of
  the source observations committed by a V3 reliability artifact;
- calibration, stationarity, causal validity, or future predictive accuracy of
  the artifact's historical success fraction;
- buyer-policy compliance assurance; R0 rejects this property because it has no
  policy-digest-bound evidence lane;
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
