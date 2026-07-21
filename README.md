# HyperIntel

HyperIntel is the inference-native foundation for a policy-compliant,
SLA-backed, evidence-carrying inference marketplace.

The current repository implements a **local pure-data foundation only**:

- canonical service, provider, offer, policy, route, metering, settlement, and
  evidence objects;
- fail-closed eligibility;
- expected-successful-cost routing among economically eligible offers;
- assurance envelopes with explicit assumptions, exclusions, maturity, and
  trust roots;
- deterministic route, usage, settlement, and evidence identities;
- an offline-verifiable end-to-end fixture.

It does not contain provider networking, prompt transport, credential custody,
GPU scheduling, wallet signing, EVM contracts, HyperCore actions, mainnet value
movement, production authentication, or proof of model or output correctness.

## Repository lineage

HyperIntel generalizes selected committed primitives from:

- `hyperAI` at `dcf41e31aa1f98438a6f8ec6cb253801e26dc0aa`;
- `composed-zk-benchmark-os` at
  `26ac4eed8e9e698d45596167a279c92f19972e62`.

Exact source mappings and exclusions are recorded in
[`docs/provenance.md`](docs/provenance.md).

## Workspace

- `hyperintel-core`: canonical JSON, digests, timestamps, identifiers, and
  inference-domain records.
- `hyperintel-assurance`: claim-envelope algebra, acceptance policy, signatures,
  and generic assurance evidence sets.
- `hyperintel-market`: eligibility, routing, receipts, metering, and deterministic
  settlement intent, plus the complete portable market evidence bundle.
- `hyperintel-cli`: offline JSON bundle verifier; its only external effect is
  reading caller-selected evidence and trusted-key files and writing the verdict
  to stdout. Bundle-disclosed keys receive no authority unless they exactly
  match the separate caller-controlled trust registry.

## Validation

```text
pnpm run lint:fast
pnpm run test:focused
pnpm run verify:contracts
pnpm run lint
pnpm run verify:full
```

The scripts call `tools/pinned-cargo`, which resolves both Cargo and Rustc from
the Rust 1.93.1 rustup toolchain. This prevents a system Rust installation from
silently compiling the workspace with a different compiler.

## Governing documents

- [`docs/product-plan.md`](docs/product-plan.md)
- [`docs/product-contract.md`](docs/product-contract.md)
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/economics.md`](docs/economics.md)
- [`docs/hyperliquid-evm.md`](docs/hyperliquid-evm.md)
- [`docs/security-and-custody.md`](docs/security-and-custody.md)
- [`docs/release-gates.md`](docs/release-gates.md)
- [`docs/state-slices.md`](docs/state-slices.md)
- [`docs/provenance.md`](docs/provenance.md)
- [`docs/claim-boundary.md`](docs/claim-boundary.md)
- [`docs/roadmap.md`](docs/roadmap.md)
