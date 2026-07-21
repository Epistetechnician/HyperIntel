# Migration provenance

HyperIntel is a new inference-native repository. It does not preserve either
source repository's outcome-market or benchmark-specific domain model.

## Source A: HyperAI

- Repository: `/Users/shaanp/Documents/GitHub/hyperAI`
- Commit: `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`
- Working tree at migration: clean.

Generalized foundations:

- `crates/outcomes-domain/src/canonical.rs` -> canonical JSON validation and
  domain-separated digesting.
- `crates/outcomes-domain/src/identifiers.rs` -> strict SHA-256 representation.
- `crates/outcomes-domain/src/timestamp.rs` -> canonical UTC whole-second time.
- `crates/outcomes-evidence/src/lib.rs` -> offline bundle verification,
  activation/revocation-aware signature patterns, and tamper rejection.
  HyperIntel's signed-capacity-offer V2 path reuses only this already
  generalized caller-owned registry and canonical Ed25519 verification pattern;
  it adds inference-native offer/provider/envelope binding without copying an
  outcome schema or development signer.
- `crates/outcomes-settlement/src/lib.rs` -> deterministic intent identity,
  canonical payloads, idempotency, and fail-closed preflight patterns.

Explicitly excluded:

- `ClaimSpecV1`, `ClaimTruth`, resolution aggregation, disputes over binary
  truth, `MarketBindingV1`, external sides, and all HIP-4 code;
- outcome-specific API, persistence, worker, and settlement schemas;
- development authentication and development-only signing authority.

## Source B: composed-zk-benchmark-os

- Repository: `/Users/shaanp/Documents/GitHub/composed-zk-benchmark-os`
- Commit: `26ac4eed8e9e698d45596167a279c92f19972e62`
- Working tree at migration: dirty; only committed Git objects were consulted.

Generalized foundations:

- `crates/hsai-claim-envelope/src/lib.rs` -> non-compensable guarantees,
  assumptions, exclusions, maturity, trust-root classes, validity, and
  acceptance-policy algebra.
- `crates/statebook-core/**` -> exact integer/rational discipline, canonical
  identities, semantic separation, and fail-closed validation patterns.
- `crates/statebook-settlement/src/completeness.rs` -> independent completeness
  dimensions plus bounded, closed, fail-closed validation patterns consulted
  for the inference-native typed reliability artifact. No Statebook subject,
  capital, settlement, recovery, or payoff type was copied.
- `crates/hsai-gateway-digest-checker/**` -> implementation-diverse checking as
  a future verification pattern.

Explicitly excluded:

- benchmark DSL, scoring, evidence promotion, external execution, economy,
  membrane, agent identity, payoff/residual market semantics, live attestation
  provider integrations, and all uncommitted admission/P01B work;
- any claim that local replay constitutes accepted evidence or production
  authority.

## Licensing boundary

Neither source checkout exposed a root license file during the migration audit.
HyperIntel therefore records provenance but does not assert a public
redistribution license. A license must be selected by the repository owner
before external distribution.
