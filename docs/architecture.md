# Architecture

HyperIntel separates five planes:

1. **Execution**: future provider discovery, encrypted transport, streaming,
   cancellation, metering, and failover.
2. **Market**: signed offers, hard eligibility, equivalent-product comparison,
   and expected-successful-cost routing.
3. **Assurance**: canonical manifests, signatures, claim envelopes, evidence,
   revocation, and customer-runnable verification.
4. **Settlement**: deterministic preauthorization, payout, fee, credit, refund,
   and later HyperEVM effects.
5. **Financial risk**: a future executed-price index and separately gated HIP-3
   hedge, never inference delivery.

The current implementation covers only pure-data portions of planes 2–4.

## First vertical slice

```text
ServiceSpecV1 + ProviderIdentityV1 + CapacityOfferV1
  + SignedCapacityOfferV1
  + SignedReliabilityArtifactV1
  + InferenceRequestPolicyV1 + AssuranceAcceptancePolicyV1
  -> provider-key offer authentication
  -> verifier-key reliability identity, binding, validity, and arithmetic checks
  -> fail-closed eligibility
  -> expected-successful-cost selection
  -> RouteEvaluationRecordV1 with every typed candidate report
  -> selected RouteDecisionV1 or deterministic NoEligibleOffer
  -> ReplayStateTransitionV1 over caller-supplied buyer state
  -> optional MeteringReceiptV1 + SettlementIntentV1 for selected routes only
  -> portable ReplayProtectedMarketEvidenceBundleV4
  -> offline verification
```

`SignedCapacityOfferV1` binds the canonical offer digest to the provider's
configured offer key, a caller-classified `ProviderKey`, the verified provider
authorization envelope, and contained validity windows. Invalid offers remain
ineligible regardless of price. `SignedMarketEvidenceBundleV2` reconstructs
this check. `MarketEvidenceBundleV1` remains a compatibility schema and keeps
its unsigned-offer nonclaim.

Two distinct offers that both authenticate under one provider key and logical
sequence are treated as verified equivocation and block the signed batch.
Unverified spoofed sequence fields remain ordinary ineligible offers and cannot
poison fallback to valid supply. Rejected attacker-claimed keys are retained in
the signed wrapper for reconstruction but are never promoted into caller trust.

`SignedReliabilityArtifactV1` binds exact observed-success and total counts,
their floor-PPM result, provider, service, source and observation-policy
commitments, observation window, and validity to a caller-classified
`VerifierKey`. Missing, stale, untrusted, tampered, or inconsistent artifacts
are candidate-scoped hard failures. `ReliabilityArtifactVerifiedMarketEvidenceBundleV3`
reconstructs this path while artifacts and trust remain separate inputs. V1 and
V2 remain compatible under their narrower explicit nonclaims.

`RouteEvaluationRecordV1` retains one canonically ordered
`EligibilityReportV1` per candidate, including typed failures and available
cost decomposition. Its outcome is either a selected `RouteDecisionV1` or
`NoEligibleOffer`. V4 therefore reconstructs both fills and complete denials;
metering and settlement records must both be present for a selection and both
absent for a denial.

`BuyerReplayStateV1` and `ReplayStateTransitionV1` bind the buyer, exact next
nonce, policy, evaluation digest, evaluation time, and before/after state. The
transition is a deterministic pure-data result for selected and denied
evaluations. It does not authenticate the buyer or establish currentness,
durable storage, atomic consumption, concurrency control, rollback resistance,
or external enforcement.

V1, V2, and V3 schemas and reconstruction behavior remain available unchanged.

Assurance requirements are hard gates. They are never converted into a scalar
that a cheaper offer can offset.

## Initial trusted computing base

- Pinned Rust toolchain and dependencies.
- Canonical JSON implementation.
- SHA-256 implementation.
- Ed25519 implementation.
- HyperIntel validation and routing code.
- Caller-controlled provider and verifier key registry, supplied separately
  from the evidence bundle. Bundle-disclosed keys must match it exactly.
- Key-validity and revocation-aware Ed25519 verification for domain-specific
  canonical messages. The assurance crate checks cryptography and caller trust;
  the owning protocol remains responsible for its message schema and bindings.
- Test fixture declarations.
- Z3 4.15.0 for the separately scoped local abstract routing-property gate. Z3
  is resolved through `tools/pinned-z3`; its reported version is pinned, but
  its installed binary provenance is an environment assumption. It is not a
  runtime dependency or an authority source.

No blockchain, provider, attestation service, or network is in the current
trusted computing base because none is called.

## Dependency direction

```text
hyperintel-core <- hyperintel-assurance <- hyperintel-market <- hyperintel-cli
```

`AssuranceEvidenceSetV1` is generic and contains no market-derived records.
Market evidence bundles belong to `hyperintel-market`, where reliability,
route, metering, and settlement objects can be reconstructed without a
dependency cycle.

## Formal property boundary

The local formal gate models two candidates and proves that the abstract
selector cannot select an ineligible candidate. A deliberately weakened
selector is satisfiable under a fixed counterexample. The test pins the exact
Z3 version and the SHA-256 digest of `routing.rs`; source drift therefore fails
the gate before the SMT result is accepted. The SMT model remains handwritten
and does not establish semantic correspondence to Rust, serialization,
concurrency, networking, or any external runtime. Exact inputs and claims are
recorded in [`formal-verification.md`](formal-verification.md).
