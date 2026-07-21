# Claim boundary

## Current supported claim

HyperIntel contains a local, hermetic, pure-data implementation of one scoped
inference-market decision path. Its tests can show that the implemented fixture
obeys the named validation, routing, arithmetic, digest, signature, and replay
rules.

## Unsupported claims

The repository does not establish:

- model identity beyond supplied evidence;
- model or output correctness, truthfulness, safety, or alignment;
- confidential execution;
- provider authorization in the external world;
- cryptographic verification of capacity-offer signatures;
- consumed-nonce or replay-state enforcement beyond committing the buyer's
  supplied snapshot digest;
- verification that an offer's declared success probability is established by
  its reliability-evidence digest;
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
