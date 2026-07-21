# State slices

Every mutation names one slice. A slice authorizes only the files and behavior
listed here.

## `hyperintel-repository-foundation-v1`

Root Git, Cargo, pnpm, agent, documentation, lint, and validation surfaces.
No product runtime, network access, credentials, contracts, or value movement.

## `hyperintel-inference-domain-v1`

`crates/hyperintel-core/**`: canonicalization, strict identifiers and timestamps,
service specifications, provider identities, capacity offers, request policies,
and exact integer money/rate types.

## `hyperintel-assurance-evidence-v1`

`crates/hyperintel-assurance/**`: typed assurance guarantees, assumptions,
exclusions, trust roots, validity, acceptance policy, Ed25519 signed-statement
verification, content-addressed evidence references, and generic validated
assurance evidence sets. This crate knows no routing or settlement types.

## `hyperintel-route-receipt-vertical-slice-v1`

`crates/hyperintel-market/**` and `crates/hyperintel-cli/**`: fail-closed offer
eligibility, provider-key signed-capacity-offer admission,
expected-successful-cost routing, route receipts, metering-to-cost calculation,
settlement conservation, deterministic idempotency, compatible unsigned V1 and
signed V2 market-evidence assembly and reconstruction, offline verification,
and a hermetic local demonstration.

## Reserved future slices

The following names reserve no implementation authority:

- `hyperintel-provider-agent-v1`
- `hyperintel-openai-gateway-v1`
- `hyperintel-private-transport-v1`
- `hyperintel-hyperevm-contracts-v1`
- `hyperintel-operator-settlement-v1`
- `hyperintel-capacity-reservation-v1`
- `hyperintel-index-publication-v1`
- `hyperintel-hip3-hedge-v1`
