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
signed V2 market-evidence assembly and reconstruction, typed reliability-artifact
identity, caller-trusted verifier signatures, binding, validity, and exact-ratio
verification through an additive V3 market-evidence profile, offline verification,
portable full eligibility and rejection reports, pure-data buyer replay-state
transitions, selected-or-denied reconstruction through an additive V4
market-evidence profile, direct CLI-process verification, and an offline
environment-pinned local formal demonstration. Reliability-artifact and
replay-transition verification grant no authority and make no claim about
observation authenticity, representative sampling, calibration, stationarity,
future provider performance, durable state, state currentness, atomic
compare-and-swap, rollback resistance, buyer authentication, external nonce
consumption, or concurrent request serialization.

## `hyperintel-authenticated-execution-receipt-v1`

`crates/hyperintel-market/src/execution.rs`, the corresponding exports in
`crates/hyperintel-market/src/lib.rs`, and execution contract coverage in
`crates/hyperintel-market/tests/vertical_slice.rs`: caller-trusted
buyer-policy signatures, non-authoritative admission bound to a reconstructed
selected route from V4-grade signed-offer, assurance, reliability, and replay
inputs, provider-receipt-key signatures over completed execution and usage,
and reconstruction of the existing metering receipt from that verified
provider receipt. This slice is pure data. It authorizes no socket, provider
call, prompt transport, credential storage, persistent replay state,
cancellation runtime, retry, settlement submission, or value movement.

## `hyperintel-durable-replay-store-v1`

`crates/hyperintel-persistence/**`, its workspace membership and exact
dependency pins in `Cargo.toml` and `Cargo.lock`, and durable-replay contract
coverage: local SQLite schema initialization, exact initial buyer-state
registration, caller-trusted authenticated-admission verification, and an
atomic compare-and-swap from the admission's before-state to after-state with
immutable consumption records and exact-retry idempotency. The state slice may
open only an operator-selected local filesystem path or an in-memory test
database. It authorizes no remote database, socket, prompt release, provider
call, credential storage, request body, cancellation, retry, settlement
submission, custody, or value movement.

## `hyperintel-local-execution-harness-v1`

`crates/hyperintel-execution/**`, its workspace membership and exact dependency
pins in `Cargo.toml` and `Cargo.lock`, the focused-test command in
`package.json`, and direct execution-harness contract coverage: a text-only,
in-process OpenAI Chat Completions compatibility shape; an exact prompt
commitment; authenticated admission verification; one durable replay consume;
strict selected-provider binding; deterministic scripted streaming, deadline,
and cancellation semantics; a configured provider-receipt signer; completed
execution receipt verification; metering reconstruction; and derivation of the
existing non-authoritative settlement intent. Only a newly applied replay
consumption may invoke the selected provider. An exact retry fails closed and
does not execute again. Failed, cancelled, or late attempts cannot produce a
completed receipt, metering receipt, or settlement intent. This slice may use
only injected in-process provider and signer implementations. It authorizes no
socket, HTTP server, external provider call, ambient credential, environment
secret, tool call, image or audio request, live cancellation propagation,
automatic reroute, settlement submission, custody, funds, or value movement.

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
