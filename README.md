<!-- readme-v2:begin -->
# HyperIntel

[![CI](https://github.com/Epistetechnician/HyperIntel/actions/workflows/ci.yml/badge.svg)](https://github.com/Epistetechnician/HyperIntel/actions/workflows/ci.yml) [![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE) ![claims: bounded](https://img.shields.io/badge/claims-bounded%20local-orange)

> HyperIntel is the inference-native foundation for a policy-compliant, SLA-backed, evidence-carrying inference marketplace.

## Start here

**Question.** Can a policy-compliant, evidence-carrying inference marketplace be built so that routing, admission, execution, metering and settlement intent are deterministic, replayable and fail closed, before any network, wallet or real money exists?

**Headline result.** A 13-crate Rust workspace whose gate passes 115 tests with 0 failures and 2 ignored. Receipts are checkable offline against a separate trust registry, replay protection uses a durable compare-and-swap, and one routing property is machine-checked with Z3 4.15.0 and bound to the SHA-256 of the routing source.

**What it does not show.** No live market, no real settlement, and no confidentiality or no-retention claim: the replay store is a local unencrypted file. The Z3 check covers a two-candidate abstraction of the router, not the whole system.

**Reproduce.**

```sh
tools/pinned-cargo test --workspace -- --skip contract_policy_non_widening
```

## Contents

- [Start here](#start-here)
- [At a glance](#at-a-glance)
- [Key findings](#key-findings)
- [Verification and CI/CD acceptance](#verification-and-cicd-acceptance)
- [Quick start](#quick-start)
- [Detailed documentation](#detailed-documentation)
- [Governance, claims, and non-claims](#governance-claims-and-non-claims)
- [Security, contributing, license](#security-contributing-license)

## At a glance

| | |
|---|---|
| Stack | Rust 1.93.1 (pinned) |
| Release stage | Public research release |
| License | Apache-2.0 (see [LICENSE](LICENSE), [NOTICE](NOTICE)) |
| Research report | [RESEARCH.md](RESEARCH.md) |
| Last full acceptance run | 2026-10-14 |
| Evidence ceiling | Bounded local replay. Nothing here is independent, benchmark, or production evidence. |

## Key findings

This is a summary of [RESEARCH.md](RESEARCH.md). Every result is a bounded local or synthetic observation, and negative results are retained rather than retuned away.

### Question

Can a policy-compliant, SLA-backed, evidence-carrying inference marketplace be
built so that routing, admission, execution, metering and settlement intent are
all deterministic, replayable, and fail closed, before any network, wallet, or
real-money component exists?

### What was built and tested

A Rust workspace of 13 crates covering: canonical JSON and digests; signed
capacity offers; typed reliability artifacts with exact count-to-PPM arithmetic;
eligibility and expected-successful-cost routing with portable full route
evaluations; authenticated buyer admission; a durable SQLite replay store with
serialized compare-and-swap and exact-retry idempotency; a text-only streaming
execution harness with deadlines and cancellation; a metadata-only execution
journal with crash classification; identity-only reconciliation; strict runtime
configuration; tenant IAM grants; an operator-secret provider; provider-agent
manifests and readiness; and an offline CLI verifier for V1-V4 evidence bundles.

Local gate: `115 passed, 0 failed, 2 ignored` across the workspace on the
pinned Rust 1.93.1 toolchain, plus clean `clippy -D warnings` and `fmt`.

### Findings

1. **A complete route-to-settlement-intent loop can be fully deterministic.**
   Route, usage, settlement and evidence identities are derived, so any tampering
   with route, metering, settlement or evidence is rejected offline against a
   separately supplied trust registry.
2. **Replay protection needs a durable compare-and-swap, not just a nonce.** The
   SQLite store registers an exact initial state, advances it atomically, makes
   exact retries idempotent and rejects a competing writer as stale.
3. **Crash states must be classified, not resumed.** A consumed admission with no
   terminal record fails closed and requires a new authenticated nonce or
   operator reconciliation. The journal and replay store are separate databases,
   so cross-database atomicity is explicitly not claimed.
4. **Routing optimises buyer expected successful cost, not platform revenue.**
   Provider price and platform fee are separate signed fields; hidden spread is
   prohibited (`docs/economics.md`).
5. **One property is machine-checked.** With Z3 4.15.0, a two-candidate
   abstraction of the router cannot select an ineligible offer (`unsat`), and a
   deliberately weakened selector can (`sat`). The check is bound to the SHA-256
   of `routing.rs`.

### Known open items

- Two tests in `hyperintel-execution-journal` (`contract_concurrent_exact_wrapper_retry_invokes_provider_agent_once` and `contract_provider_agent_seam_derives_completed_incomplete_and_failed_terminals`) were failing at the release-staging snapshot and are marked `#[ignore]` with that reason. They cover the provider-agent journal wrapper under concurrency and crash recovery, which should be treated as unverified until they pass.
- Windows and Linux builds are untested; the pinned toolchain and Z3 gate were exercised on macOS only.

### What this does not establish

No HTTP gateway, provider networking, GPU scheduling, wallet signing, EVM
contracts, mainnet value movement, production authentication, rollback-resistant
or remotely replicated replay enforcement, or proof of model or output
correctness. The Z3 result is a handwritten abstraction, not Rust-source
correspondence. Reliability verification does not show that source observations
are true or predictive. Nothing here is a security guarantee or independent
verification.

## Verification and CI/CD acceptance

GitHub Actions workflow `ci.yml` runs on every push and pull request with read-only repository permissions (`contents: read`).

| Job | Runner | Gate |
|---|---|---|
| test | ubuntu-latest | `cargo fmt --all -- --check` |
|  |  | `cargo clippy --workspace --all-targets -- -D warnings` |
|  |  | `cargo test --workspace -- --skip contract_policy_non_widening` |
| formal (advisory, `continue-on-error`) | ubuntu-latest | `pip install z3-solver==4.15.0.0` then `cargo test -p hyperintel-market --test formal_policy` |

**Gate intent:** Format, lint, test; Z3 formal-policy job is advisory.

**Local acceptance (2026-10-14):** fmt clean; clippy clean; 114 tests passed, 0 failed, 2 ignored (known open items, tracked in the research report).

Run the same gate locally:

```sh
tools/pinned-cargo fmt --all -- --check
tools/pinned-cargo clippy --workspace --all-targets -- -D warnings
tools/pinned-cargo test --workspace -- --skip contract_policy_non_widening
```

### Acceptance criteria

- The gate above exits 0 on a clean checkout of the default branch.
- No claim in this repository exceeds the evidence ceiling stated in [At a glance](#at-a-glance).
- A failing, skipped, or ignored check is reported as such; it is never reclassified as a pass.
- Commits to this repository are authored by the maintainer only.

## Quick start

```sh
git clone https://github.com/Epistetechnician/HyperIntel.git
cd HyperIntel
tools/pinned-cargo fmt --all -- --check
tools/pinned-cargo clippy --workspace --all-targets -- -D warnings
tools/pinned-cargo test --workspace -- --skip contract_policy_non_widening
```

## Detailed documentation

The sections below are the repository's original design and implementation documentation, preserved in full.

<!-- readme-v2:detail -->

HyperIntel is the inference-native foundation for a policy-compliant,
SLA-backed, evidence-carrying inference marketplace.

The current repository implements a **local hermetic foundation only**:

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
- caller-trusted buyer-policy authentication, non-authoritative selected-route
  admission, provider-receipt-key authentication of completed execution, and
  metering reconstruction from the signed receipt;
- local SQLite replay-state initialization and authenticated admission
  consumption using a serialized, exact state-digest compare-and-swap,
  immutable consumption records, and exact-retry idempotency;
- a text-only, in-process Chat Completions request and streaming-chunk subset
  that invokes only the selected injected provider after a new durable replay
  consume, rejects exact retry without re-execution, enforces deterministic
  deadline and cancellation boundaries, verifies an injected provider receipt
  signature, reconstructs metering, and derives a non-authoritative settlement
  intent;
- an authenticated reroute lineage that accepts only a zero-output
  `UNAVAILABLE` attempt and a caller-supplied, buyer-signed next-nonce
  admission for the same buyer, service, and prompt but a different selected
  provider, then executes that admission through the same local harness;
- a separate metadata-only SQLite execution journal that records canonical
  starts and terminals, queries the durable replay store during recovery,
  rejects implicit resume, and classifies non-retriable consumed-without-
  terminal crash states without retaining prompts or responses;
- deterministic identity-only reconciliation across admission, durable replay
  consumption, journal lineage, completed or incomplete execution, stream,
  provider receipt, metering, and non-authoritative settlement intent;
- a bounded, strict local-hermetic runtime configuration document with
  canonical identity, explicit distinct SQLite paths, bounded pool sizes, and
  opaque immutable-version secret references that never resolve secret
  material;
- pure-data tenant authorization separation with fixed roles, one-action
  short-lived signed grants, exact caller-owned resource and operation
  bindings, purpose-separated tenant IAM keys, and signed rollback-pinned
  revocation evidence;
- atomic local operator-secret authorization and injected resolution with
  dedicated trust, exact runtime/provider/reference/purpose bindings, one
  resolver call, and bounded redacted zeroize-on-drop material;
- purpose-separated signed provider-agent manifests, readiness assertions,
  rollback/fork checkpoints, and atomic preflight before the existing local
  harness consumes replay or invokes an injected provider;
- an offline-verifiable V1–V4 fixture corpus exercised through the real CLI
  process; and
- a source-hash-bound, two-candidate Z3 policy-non-widening check.

The unsigned-offer V1 and signed-offer V2 bundles remain available for schema
compatibility. V3 also remains byte- and semantic-compatible. Each version
retains its narrower nonclaims and historical collapsed assurance-rejection
profile. Frozen rejection/fallback report and bundle commitments cover all
three legacy versions. New local integrations should use V4's additive causal
rejection profile and supply reliability artifacts separately from both the
bundle and caller-controlled trust registry.

It does not contain an HTTP gateway, provider networking, external prompt
transport, ambient provider credentials, GPU scheduling, wallet signing, EVM
contracts, HyperCore actions, mainnet value movement, production
authentication, rollback-resistant or remotely replicated replay enforcement,
or proof of model or output correctness. V4's transition alone advances only
the caller-supplied state carried by the bundle. The separate local SQLite
store verifies an authenticated admission and atomically consumes that exact
transition. The local execution harness can then release the request only to
the selected injected in-process provider; it grants no external request or
provider authority and does not prevent an operator from restoring an older
database artifact. Reliability verification does not prove that source
observations are true, representative, independently reproduced, calibrated,
stationary, or predictive of future provider performance.
The execution journal and replay store are separate SQLite databases: their
individual writes are durable and validated, but no cross-database atomicity is
claimed. An interrupted consumed admission without a matching terminal fails
closed and requires a new authenticated nonce or explicit operator
reconciliation.
The reconciliation report checks local record consistency only. It does not
establish independent metering, observation authenticity, payment, settlement
finality, or production authority.

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
  pure-data replay transitions, authenticated but non-authoritative execution
  admissions, provider-signed completed-execution receipts, metering,
  deterministic settlement intent, and portable V1/V2/V3/V4 market evidence
  bundles.
- `hyperintel-persistence`: local SQLite schema checksum, exact initial replay
  state registration, authenticated admission verification, serialized atomic
  compare-and-swap, immutable replay-consumption records, validated reads, and
  restart coverage. It opens no remote database or provider connection.
- `hyperintel-execution`: text-only OpenAI-shaped request and stream records,
  exact prompt commitment, one-shot durable admission consumption,
  selected-provider binding, deterministic local deadlines and cancellation,
  injected provider and receipt-signer ports, completed receipt verification,
  metering reconstruction, and non-authoritative settlement-intent derivation.
  It opens no socket and reads no credential or environment secret.
- `hyperintel-execution-journal`: metadata-only local SQLite starts and
  terminals, serialized exact-retry idempotency, replay-store-bound restart
  classification, tamper detection, and a fail-closed wrapper around the
  hermetic harness. It stores no prompt, response, credential, key, arbitrary
  error text, or settlement authority.
- `hyperintel-reconciliation`: pure-data completed and incomplete receipt-graph
  reconstruction, stable identity-only reports, fail-closed stream/time/status
  checks, and adversarial tamper coverage. It performs no database mutation or
  external effect.
- `hyperintel-runtime-config`: bounded strict JSON validation, canonical
  configuration identity, conservative local SQLite path validation, bounded
  pool settings, and opaque immutable-version secret references. It reads only
  an explicit caller-selected file and opens no database, socket, environment
  source, or secret provider.
- `hyperintel-tenant-iam`: caller-supplied tenant policy, dedicated
  tenant-namespaced Ed25519 trust, fixed roles and actions, exact resource
  ownership, 300-second grants, and signed freshness-bounded revocation
  snapshots. It issues no token and provides no runtime tenant isolation.
- `hyperintel-secret-provider`: signed short-lived operator-secret
  authorization, dedicated trust and rollback-pinned revocation, exact
  runtime/consumer/provider/reference/purpose binding, and one injected local
  resolution call returning bounded redacted zeroize-on-drop material. It has
  no ambient credential source, network adapter, cache, retry, or external
  effect.
- `hyperintel-provider-agent`: signed service-manifest, readiness, revocation,
  and checkpoint verification bound to the selected provider, service, offer,
  runtime, artifacts, capabilities, receipt key, interface, and secret-
  reference identity before one call into the existing hermetic harness. Its
  assertions are not observed health and it adds no provider transport.
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

## Research report and project policy

See [RESEARCH.md](RESEARCH.md) for findings and known open items,
[SECURITY.md](SECURITY.md) to report issues, and [CONTRIBUTING.md](CONTRIBUTING.md).
Licensed under [Apache-2.0](LICENSE).

---
<!-- readme-v2:footer -->
## Governance, claims, and non-claims

Work proceeds under named state slices recorded in [AGENTS.md](AGENTS.md). The claim ceiling stated above is a hard limit: a local pass is not independent verification, a benchmark result, a proof of the whole system, or a statement about production readiness. See "What this does not establish" in [RESEARCH.md](RESEARCH.md).

## Security, contributing, license

- Report vulnerabilities per [SECURITY.md](SECURITY.md).
- Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.
- Licensed under [Apache-2.0](LICENSE); see [NOTICE](NOTICE).
