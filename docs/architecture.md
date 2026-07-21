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
  + InferenceRequestPolicyV1 + AssuranceAcceptancePolicyV1
  -> provider-key offer authentication
  -> fail-closed eligibility
  -> expected-successful-cost selection
  -> RouteReceiptV1
  -> MeteringReceiptV1
  -> SettlementIntentV1
  -> portable MarketEvidenceBundleV1
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
- Test fixture declarations.

No blockchain, provider, attestation service, or network is in the current
trusted computing base because none is called.

## Dependency direction

```text
hyperintel-core <- hyperintel-assurance <- hyperintel-market <- hyperintel-cli
```

`AssuranceEvidenceSetV1` is generic and contains no market-derived records.
`MarketEvidenceBundleV1` belongs to `hyperintel-market`, where route, metering,
and settlement objects can be reconstructed without a dependency cycle.
