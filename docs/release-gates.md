# Release and evidence gates

## Evidence vocabulary

- **Implemented:** source exists in the named state slice.
- **Locally checked:** declared hermetic checks passed at the recorded commit.
- **Independently reproduced:** a separate operator replayed the exact bundle.
- **Testnet exercised:** configured contracts and services produced retained
  testnet artifacts.
- **Audited:** a named external review covers an exact commit and deployment.
- **Production authorized:** technical, operational, financial, and legal gates
  for the named scope are all approved.

No lower state implies a higher one.

## Gate matrix

| Release | Required technical evidence | Required operating evidence | Prohibited claim |
|---|---|---|---|
| R0 local kernel | Full local gate, tamper fixtures, deterministic reconstruction | Source provenance and claim boundary | Network, funds, provider, chain, or production readiness |
| R1 private alpha | End-to-end transport tests, durable replay, cancellation, tenant isolation | Two authorized providers, incident runbook, synthetic/no-funds soak | Confidential computing or external settlement |
| R2 testnet | Contract invariants, deployment provenance, indexer reconstruction, escape tests | Key ceremonies, RPC failover, testnet accounting reconciliation | Mainnet safety or custody authorization |
| R3 paid beta | External security review, bounded production telemetry, rollback | Legal approval, exposure caps, reserve policy, support and incident drills | General availability or insured SLA |
| R4 GA | Reproducible releases, independent checker, repeated tenant isolation and recovery tests | Published support matrix, SLOs, audits, provider concentration controls | Universal model identity or correctness |
| R5 index | Reproducible methodology and observations, manipulation monitors | Governance, revision, incident and licensing policy | Tradable benchmark readiness |
| R6 hedge | Oracle and market-operation controls, liquidity and failure simulations | HIP-3 stake, legal, treasury, slashing and 24/7 operations approval | Capacity delivery or portfolio-margin equivalence |

## R0 acceptance checklist

- All public identities are canonical and domain separated.
- Floats and ambiguous timestamps are rejected.
- Arithmetic is checked and settlement conserves value.
- Ineligible supply cannot win regardless of price.
- Route selection is permutation invariant with explicit tie-breaking.
- The V2 path verifies each exact capacity-offer digest against the configured
  provider offer key, caller trust class, provider-assurance binding, validity,
  and revocation state. V1 remains an explicitly unsigned compatibility schema.
- Verified same-key sequence equivocation blocks the signed batch; unverified
  spoofed sequence fields cannot poison fallback, and rejected claimed keys do
  not become caller-trusted disclosures.
- Route identity commits to the buyer-supplied replay snapshot and declared
  reliability inputs. R0 does not enforce consumed nonces or verify that the
  reliability artifact establishes the declared probability.
- Evidence signatures, content bytes, validity, and policy are verified offline
  against trust keys supplied separately from the bundle.
- Route, metering, settlement, and evidence tampering fails.
- No source imports an HTTP, wallet, EVM, database, or secret-handling runtime.
- `pnpm run verify:full` passes on the pinned toolchain.

## Change control

Every mutation names a state slice. Schema changes require compatibility and
golden-vector tests. Routing or fee changes require before/after fixture replay.
Settlement changes require conservation, authorization, replay, and destination
tests. Assurance changes require explicit claim-ceiling review. Contract changes
require a new deployment identity and migration/escape analysis.
