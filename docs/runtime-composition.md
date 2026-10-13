# Hermetic runtime composition V1

State slice: `hyperintel-hermetic-runtime-composition-v1`.

This slice composes existing validated local contracts into one deterministic attempt. It does
not add a transport, provider SDK, ambient credential source, secret backend, route selection,
reroute, remote database, crash resumption, settlement submission, custody, deployment, or
production authority.

The operation identity is domain-separated and binds the validated configuration and runtime,
tenant, principal, audience, authenticated admission, request and prompt commitments, selected
route, provider and offer, complete provider-agent binding and agent instance, configured receipt
key, immutable secret reference, provider-API-authentication purpose, cancellation signal, and the
explicit evaluation time. Evaluation time must equal replay-consumption time. In V1 the tenant
principal must equal the authenticated buyer because no separate signed delegation exists. The
same identity must be present in the tenant `SUBMIT_INFERENCE` grant and signed operator-secret
authorization.

The journal owns start and recovery inspection, one exact provider-agent activation attempt,
actual replay lookup, and terminalization. It derives terminal metadata only from the opaque local
outcome and actual durable replay record. It never retries, resumes, or creates a terminal for a
preexisting or concurrently observed consumption that this attempt did not create.

Provider-agent activation occurs only after current signed control, authenticated admission,
consumption-time, request, prompt, route, provider, signer, and cancellation preflight succeeds.
Activation verifies one signed operator-secret authorization, invokes one injected synthetic
resolver, and moves the bounded resolved material into a private nonserializable provider adapter.
No reusable verified-control token or secret material appears in the composition report.
The private adapter moves the resolved secret into a one-use slot, lends bytes to the injected
provider call only for that call's dynamic extent, and drops the zeroizing owner when the call
returns. A second provider invocation fails closed. An injected caller implementation could still
copy or retain the borrowed byte slice; non-retention by caller code remains an explicit trust
limitation.

A positive report means the composed attempt closed consistently, not necessarily that inference
completed. Its closed reconciliation status distinguishes completed execution from cancellation,
deadline, provider-failure, and receipt-signing incomplete terminals. Completed reports reconcile
receipt, metering, and settlement-intent identities; incomplete reports require those fields to be
absent. The report retains the exact IAM, operator-secret, provider-agent, replay, journal, and
reconciliation identity lineage. It is opaque, nonserializable, identity-only, redacted under
`Debug`, and always grants no authority. Local fixtures, signatures, digests, SQLite records, and
tests do not establish confidential execution, tenant isolation, provider availability, external
settlement, value movement, or production readiness.

The validated configuration commits to replay and journal paths, but V1 receives already-opened
stores and cannot introspect their connection origins. Exact store-to-path correspondence remains
a caller-trust limitation; the composition report proves the configuration identity supplied to
authorization records, not that an injected open handle was created from that path.
