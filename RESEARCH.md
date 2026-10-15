# Research report and findings

HyperIntel is an engineering foundation rather than an experiment series, so
its "findings" are what a hermetic build established about the design, plus the
limits it recorded. Governing documents: `docs/product-contract.md`,
`docs/economics.md`, `docs/security-and-custody.md`, `docs/release-gates.md`,
`docs/claim-boundary.md`.

## Question

Can a policy-compliant, SLA-backed, evidence-carrying inference marketplace be
built so that routing, admission, execution, metering and settlement intent are
all deterministic, replayable, and fail closed, before any network, wallet, or
real-money component exists?

## What was built and tested

A Rust workspace of ten crates covering: canonical JSON and digests; signed
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

## Findings

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

## Known open items

- Two tests in `hyperintel-execution-journal` (`contract_concurrent_exact_wrapper_retry_invokes_provider_agent_once` and `contract_provider_agent_seam_derives_completed_incomplete_and_failed_terminals`) were failing at the release-staging snapshot and are marked `#[ignore]` with that reason. They cover the provider-agent journal wrapper under concurrency and crash recovery, which should be treated as unverified until they pass.
- Windows and Linux builds are untested; the pinned toolchain and Z3 gate were exercised on macOS only.

## What this does not establish

No HTTP gateway, provider networking, GPU scheduling, wallet signing, EVM
contracts, mainnet value movement, production authentication, rollback-resistant
or remotely replicated replay enforcement, or proof of model or output
correctness. The Z3 result is a handwritten abstraction, not Rust-source
correspondence. Reliability verification does not show that source observations
are true or predictive. Nothing here is a security guarantee or independent
verification.

## Reproduce

```sh
rustup toolchain install 1.93.1
pnpm run verify:full       # or: tools/pinned-cargo test --workspace
```

The formal test needs Z3 4.15.0 (set `HYPERINTEL_Z3` or put `z3` on `PATH`).
