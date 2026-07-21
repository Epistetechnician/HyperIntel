# Delivery roadmap

## R0: local market kernel

Implemented local target: canonical domain objects, assurance envelopes,
provider-key signed-offer admission, routing, receipts, settlement intent,
portable V1/V2 evidence, verifier-signed typed reliability artifacts and V3
evidence, CLI verification, and hermetic tests. Remaining R0 work includes a
hermetic replay-state transition, fuller route rejection records, and direct
CLI-process coverage.

## R1: private execution alpha

Future state slices: provider agent, OpenAI-compatible gateway, streaming,
cancellation, passive health, direct-provider mode, durable persistence, and
production IAM. No chain dependency.

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
