# HyperIntel

HyperIntel is the inference-native foundation for a policy-compliant,
SLA-backed, evidence-carrying inference marketplace.

The current repository implements a **local pure-data foundation only**:

- canonical service, provider, offer, policy, route, metering, settlement, and
  evidence objects;
- fail-closed eligibility, including a V2 signed-offer path that verifies the
  exact capacity-offer digest against the provider's configured offer key;
- an additive V3 path that also verifies caller-supplied, verifier-signed typed
  reliability artifacts, exact count-to-PPM arithmetic, bindings, and validity;
- an additive V4 path that retains every typed eligibility report for either a
  selected route or a deterministic all-rejected result, then binds that result
  to a pure-data buyer replay-state transition;
- expected-successful-cost routing among economically eligible offers;
- assurance envelopes with explicit assumptions, exclusions, maturity, and
  trust roots;
- deterministic route, usage, settlement, and evidence identities;
- an offline-verifiable V1–V4 fixture corpus exercised through the real CLI
  process; and
- a source-hash-bound, two-candidate Z3 policy-non-widening check.

The unsigned-offer V1 and signed-offer V2 bundles remain available for schema
compatibility. V3 also remains byte- and semantic-compatible. Each version
retains its narrower nonclaims. New local integrations should use V4 and supply
reliability artifacts separately from both the bundle and caller-controlled
trust registry.

It does not contain provider networking, prompt transport, credential custody,
GPU scheduling, wallet signing, EVM contracts, HyperCore actions, mainnet value
movement, production authentication, durable or atomic replay enforcement, or
proof of model or output correctness. V4's transition advances only the
caller-supplied state carried by the bundle. It does not prove that state is
current, persisted, rollback-resistant, buyer-authenticated, or consumed by an
external system. Reliability verification does not prove that source
observations are true, representative, independently reproduced, calibrated,
stationary, or predictive of future provider performance.

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
- `hyperintel-market`: signed-offer admission, eligibility, routing, receipts,
  typed reliability-artifact verification, portable full route evaluations,
  pure-data replay transitions, metering, deterministic settlement intent, and
  portable V1/V2/V3/V4 market evidence bundles.
- `hyperintel-cli`: offline JSON bundle verifier; its only external effect is
  reading caller-selected evidence and trusted-key files and writing the verdict
  to stdout. It dispatches only explicit V1/V2/V3/V4 bundle schema identifiers.
  Bundle-disclosed keys receive no authority unless they exactly match the
  separate caller-controlled trust registry. Committed V1–V4 fixtures exercise
  every supported success path plus missing trust, unsupported schema,
  argument-count, and missing-file behavior through the compiled CLI process.

The direct process corpus is
`crates/hyperintel-cli/tests/fixtures/evidence-v1.json` through
`evidence-v4.json`, the version-appropriate trusted-key files,
`empty-trusted-keys.json`, and `unsupported-v5.json`. Evidence and caller trust
remain separate inputs, and the market contract test regenerates the canonical
documents to detect fixture drift.

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

The formal contract test also requires local `Z3 version 4.15.0 - 64 bit`.
`tools/pinned-z3` resolves the operator-installed executable and rejects any
other reported version. The test checks a handwritten two-candidate routing
abstraction, binds the check to the SHA-256 digest of `routing.rs`, and expects
`unsat` for the non-widening query and `sat` for the deliberately weakened
counterexample. This is scoped environment-pinned local evidence, not a proof
of solver-binary provenance or Rust-source correspondence. See
[`docs/formal-verification.md`](docs/formal-verification.md).

## Governing documents

- [`docs/e2e.md`](docs/e2e.md)
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
- [`docs/formal-verification.md`](docs/formal-verification.md)
- [`docs/roadmap.md`](docs/roadmap.md)
