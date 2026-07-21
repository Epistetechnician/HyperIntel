# End-to-end product plan

## Executive decision

Build a verified inference-capacity marketplace, not a credential resale
exchange and not a prediction market.

> HyperIntel selects the lowest expected successful cost among offers that can
> satisfy the buyer's explicit service, privacy, assurance, and SLA policy.

Inference stays offchain. Evidence and settlement intent are deterministic and
portable. HyperEVM is the escrow, bond, registry, and settlement substrate.
HyperCore is reserved for a later, independently gated hedge over a mature
executed-price index. No outcome-market component belongs in the product.

## Initial customer

The first buyer is an agent platform, financial application, or regulated
software operator that needs all of the following:

- an OpenAI-compatible request path;
- price competition across authorized capacity;
- hard regional, privacy, model, tool, and assurance constraints;
- bounded maximum spend and machine-readable SLA remedies;
- evidence from which an independent checker can recompute routing and
  settlement without trusting the router's calculation. External provider,
  region, execution, model, and metering claims remain limited by their
  separately eligible evidence.

The first provider is an authorized self-hosted operator or official reseller
with a stable model build, declared service window, settlement identity, and a
bond. Buyer-owned provider keys may be supported as a separate direct mode.
Opaque credential resale is not accepted supply.

## Product surfaces

| Surface | Buyer result | Provider result | HyperIntel revenue |
|---|---|---|---|
| Immediate inference | Policy-eligible streamed response and receipts | Paid completed work | Execution fee |
| Reserved capacity | Contracted throughput window with measurable service levels and capped remedies | Forward demand and utilization | Reservation fee |
| Enterprise assurance | Private policies, on-prem checker, audit export | Higher-value compliant demand | Subscription |
| Verification services | Independent replay and evidence retention | Stronger trust tier | Verification fee |
| Future index licensing | Auditable executed-price benchmark | Market visibility | Data/API license |

## Non-negotiable product rules

1. Assurance requirements are hard eligibility gates, never compensable scores.
2. Routing minimizes expected successful cost only after economic equivalence.
3. Prompts, outputs, and provider credentials never enter public chain state.
4. Every charge binds to a route, offer, policy, service, usage, and payer cap.
5. Every settlement conserves value and has a full-payload idempotency key.
6. Missing, expired, revoked, unsupported, or ambiguous evidence fails closed.
7. Attestation can strengthen a declared boundary; it does not prove model
   semantics or output correctness.
8. A blockchain commitment proves inclusion and ordering, not external service
   delivery.
9. Inference delivery cannot depend on a derivatives market.
10. Mainnet, custody, or production claims require their own evidence gates.

## Target end-to-end service sequence

```text
buyer policy + maximum authorization
  -> equivalent-service discovery
  -> fail-closed evidence and availability checks
  -> expected-successful-cost route
  -> encrypted direct execution and streaming
  -> signed route + usage + provider + buyer receipts
  -> objective SLA evaluation
  -> deterministic settlement intent
  -> buyer refund + provider payout + platform fee + SLA credit
  -> portable evidence bundle and optional public commitment
```

The current R0 slice ends at a non-authoritative settlement intent. Its V2 path
verifies capacity-offer signatures against caller-supplied trust. Its additive
V3 path also verifies verifier-signed typed reliability identity, bindings,
validity, and exact historical count arithmetic. V1 and V2 remain compatible
under narrower explicit nonclaims. R0 does not verify the underlying
observation truth, provider/buyer execution-receipt signatures, live
consumed-nonce state, bonds, SLA credits, chain commitments, or value movement.

## System boundary

The marketplace is split so no component receives authority merely because it
can calculate a result:

- **Buyer SDK/gateway:** authentication, policy, maximum authorization,
  streaming, cancellation, and local receipt verification.
- **Provider agent:** signed offers, encrypted execution, metering, health,
  capacity accounting, and receipt signing.
- **Router:** eligibility and deterministic selection; never holds provider
  credentials and never settles funds by itself.
- **Assurance verifier:** validates manifests, signatures, validity, revocation,
  attestations, and evidence policy; replaceable and customer-runnable.
- **Settlement operator:** submits already-authorized intents and monitors
  confirmations; cannot change price, usage, or destination.
- **HyperEVM contracts:** provider registry, buyer session escrow, provider
  bonds, cumulative settlement, public commitments, pause, and escape path.
- **HyperCore integration:** future hedge only after index and listing gates.

## Initial market definition

Launch exactly one economically coherent testnet service:

- one pinned open-weight model build;
- one region and latency tier;
- one request/response schema and tokenizer version;
- two separately administered authorized providers, with distinct settlement
  keys, infrastructure failure domains, disclosed control, and disclosed
  beneficial ownership;
- one settlement asset selected only after verifying its exact address,
  bytecode and proxy/admin controls, decimals, transfer behavior, supply
  backing, and allowlist status; Core/EVM linkage is verified separately only
  if used;
- immediate requests plus explicit availability windows;
- deterministic token metering and maximum-cost authorization;
- provider bond against narrow objective non-performance;
- no transferable capacity token, leverage, HIP-3 market, or mainnet funds.

## Objective SLA vocabulary

V1 remedies may cover only values available from attributable receipts:

- admitted request not started before a deadline;
- stream terminated before a declared completion condition;
- metered units exceed the signed authorization;
- service, provider, or route binding differs from the authorization;
- region differs only when eligible provider or hardware evidence binds the
  observed execution region;
- provider failed to submit a required signed receipt;
- reserved availability below the declared window only when a declared
  observation protocol supplies attributable admission records, heartbeats, or
  challenge observations.

When the required evidence is absent or ambiguous, the result is `unsupported`,
`inconclusive`, or manual review—not automatic slashing.

Semantic quality, hallucination, truthfulness, usefulness, and subjective output
quality are excluded from automatic slashing.

## Twelve-month sequence

| Phase | Deliverable | Exit gate |
|---|---|---|
| 0 | Local market kernel | Portable bundle reconstructs route, metering, settlement, and tamper rejection |
| 1 | Private execution alpha | Two providers, encrypted streaming, cancellation, durable replay protection, no funds |
| 2 | HyperEVM testnet | Session escrow, bonds, cumulative vouchers, objective dispute fixtures, escape path |
| 3 | Paid design-partner beta | Bounded real settlement, production IAM, incident drills, independent security review |
| 4 | Reserved-capacity product | Availability windows, reservation accounting, SLA remedies, enterprise tenancy |
| 5 | Executed-price index | Published method, diversity and manipulation controls, reproducible observations |
| 6 | Optional HIP-3 hedge | Separate legal, oracle, liquidity, stake, operations, and basis-risk approval |

## Explicitly removed

HyperIntel contains no prediction-market resolution, binary claim truth,
outcome shares, HIP-4 dependency, general dispute voting, or oracle for
subjective inference quality. Disputes are limited to objective signed service
facts and contract state.
