# Delivery roadmap

## R0: local market kernel

Complete local implementation target: canonical domain objects, assurance
envelopes, provider-key signed-offer admission, reliability-qualified routing,
receipts, settlement intent, compatible V1/V2/V3 evidence, and additive V4
evidence. V4 retains every typed eligibility report for selected and
all-rejected evaluations, binds the result to a deterministic caller-supplied
replay-state transition, and is exercised by committed fixtures through the
compiled CLI process for every V1–V4 schema. A source-hash-bound Z3 4.15.0 test
checks one bounded two-candidate policy-non-widening property and a weakened
counterexample.

R0 completion is a local implementation claim only. The replay transition does
not authenticate the buyer or provide current, durable, atomic,
rollback-resistant, concurrent, or externally consumed state. The formal model
is handwritten and does not prove Rust-source correspondence. No R0 artifact
authorizes inference execution, networking, persistence, custody, chain action,
funds, or production use.

## R1: private execution alpha

Future state slices: provider agent, OpenAI-compatible gateway, streaming,
cancellation, passive health, direct-provider mode, durable persistence, and
production IAM. No chain dependency.

The next MVP tranche is authenticated buyer admission, durable atomic replay,
two provider agents, request deadlines, streaming and cancellation, health and
failover, signed execution and metering receipts, tenant isolation, receipt
reconciliation, fault injection, and a synthetic no-funds soak.

## R2: HyperEVM testnet

Future contracts: provider registry, buyer session escrow, cumulative vouchers,
bond manager, settlement hub, receipt commitments, objective disputes, multisig,
timelock, pause, and escape path.

The testnet exit gate includes measured fast/slow-block gas behavior, nonce
queue handling, indexed event reconstruction, RPC rate-limit and outage tests,
and a non-single-RPC production design. Current documented parameters such as
block cadence, gas limits, accepted future nonces, and official API rate limits
are deployment inputs to re-query, not protocol constants embedded in code.

## R3: capped mainnet beta

Requires external audits, verified settlement asset, exposure caps, practiced
incident recovery, legal approval, paying design partners, and a sustained
testnet soak.

## R4: reserved-capacity GA

Adds availability windows, reservations, enterprise tenancy, on-prem verifier,
release provenance, and repeated proof-to-enforcement customer runs.

## R5: reference index

Publishes a non-tradable executed-fill index only after diversity,
concentration, manipulation, staleness, and reproducibility gates pass.

## R6: optional HIP-3 hedge

Requires a separate legal, oracle, operational, liquidity, stake, slashing, and
basis-risk decision. It is never a dependency of inference authorization or
physical settlement.
