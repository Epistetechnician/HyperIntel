# Product contract

HyperIntel will become an OpenAI-compatible market for policy-compliant,
SLA-backed, evidence-carrying inference.

The economic product is:

```text
service manifest x region x latency tier x privacy profile
x tool profile x assurance profile x availability window
```

Only equivalent and policy-eligible services compete directly.

## Buyer contract

The buyer declares permitted services, regions, privacy requirements, tool
requirements, assurance requirements, maximum cost, validity, and failover
rules. A missing or unsupported requirement denies routing.

## Provider contract

The provider declares authorization, service identity, capacity, price,
availability, operational risk inputs, settlement identity, and assurance
evidence. A signed offer is an attributable declaration, not proof that the
external service will perform.

The local V2 admission path verifies the exact offer digest, provider and offer
key identities, caller-assigned provider-key trust class, validity containment,
revocation state, and binding to verified provider-authorization evidence. It
does not supply, store, or operate a provider signing key.

Distinct, fully authenticated offers cannot reuse one provider-key logical
sequence. Sequence collisions are checked only after authentication so an
invalid wrapper cannot deny routing to unaffected valid providers.

## Router contract

The router first applies hard gates, then minimizes expected successful cost.
It emits the full eligible-offer-set commitment, typed rejections, scoring
version, selected offer, and maximum authorized cost.

## Settlement contract

The current pure-data settlement builder requires a successful route, fully
reconstructed metering, and buyer authorization. It enforces:

```text
authorized_max = provider_payout + platform_fee + buyer_refund
```

Buyer debit, provider/buyer co-signed receipts, bond remedies, and SLA credits
are future settlement-adapter behavior. When added, credits must be explicit
signed fields with their own conservation and collateral checks.

No external effect is authorized in the current repository.
