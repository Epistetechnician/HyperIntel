use std::collections::{BTreeMap, BTreeSet};

use hyperintel_assurance::{
    AssuranceAcceptancePolicyV1, AssuranceClaimV1, AssuranceEvidenceSetV1, VerificationKeyRegistry,
    conjoin_envelopes_v1, evaluate_assurance_v1, verify_assurance_evidence_set_v1,
};
use hyperintel_core::{
    AssuranceProperty, AtomicAmount, CanonicalizationError, CapacityOfferV1, ExactArithmeticError,
    InferenceRequestPolicyV1, ProviderIdentityV1, ProviderStatus, ServiceSpecV1, Sha256Digest,
    UtcTimestamp, domain_separated_digest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    MARKET_STATE_SLICE_V1, SignedCapacityOfferV1, SignedReliabilityArtifactV1,
    verify_offer_reliability_v1, verify_signed_capacity_offer_v1,
};

/// Complete pure-data routing inputs.
pub struct RoutingContextV1<'a> {
    /// Explicit evaluation time. No ambient clock is read.
    pub evaluated_at: &'a UtcTimestamp,
    /// Required service semantics.
    pub service: &'a ServiceSpecV1,
    /// Provider registry snapshot keyed by provider ID.
    pub providers: &'a BTreeMap<hyperintel_core::ProtocolId, ProviderIdentityV1>,
    /// Candidate signed declarations. Cryptographic transport is external.
    pub offers: &'a [CapacityOfferV1],
    /// Buyer policy and economic authorization.
    pub policy: &'a InferenceRequestPolicyV1,
    /// Generic assurance evidence sets keyed by evidence-set digest.
    pub assurance_sets: &'a BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    /// Content-addressed evidence bytes.
    pub assurance_blobs: &'a BTreeMap<Sha256Digest, Vec<u8>>,
    /// Caller-owned verification-key registry.
    pub verification_keys: &'a VerificationKeyRegistry,
}

/// Complete pure-data routing inputs whose offers must authenticate before admission.
pub struct SignedRoutingContextV1<'a> {
    /// Explicit evaluation time. No ambient clock is read.
    pub evaluated_at: &'a UtcTimestamp,
    /// Required service semantics.
    pub service: &'a ServiceSpecV1,
    /// Provider registry snapshot keyed by provider ID.
    pub providers: &'a BTreeMap<hyperintel_core::ProtocolId, ProviderIdentityV1>,
    /// Candidate offers with provider signatures over their exact canonical digests.
    pub offers: &'a [SignedCapacityOfferV1],
    /// Buyer policy and economic authorization.
    pub policy: &'a InferenceRequestPolicyV1,
    /// Generic assurance evidence sets keyed by evidence-set digest.
    pub assurance_sets: &'a BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    /// Content-addressed evidence bytes.
    pub assurance_blobs: &'a BTreeMap<Sha256Digest, Vec<u8>>,
    /// Caller-owned verification-key registry.
    pub verification_keys: &'a VerificationKeyRegistry,
}

/// Signed-offer routing inputs with caller-supplied typed reliability artifacts.
pub struct ReliabilityArtifactVerifiedRoutingContextV1<'a> {
    /// Explicit evaluation time. No ambient clock is read.
    pub evaluated_at: &'a UtcTimestamp,
    /// Required service semantics.
    pub service: &'a ServiceSpecV1,
    /// Provider registry snapshot keyed by provider ID.
    pub providers: &'a BTreeMap<hyperintel_core::ProtocolId, ProviderIdentityV1>,
    /// Candidate offers with provider signatures over their exact canonical digests.
    pub offers: &'a [SignedCapacityOfferV1],
    /// Buyer policy and economic authorization.
    pub policy: &'a InferenceRequestPolicyV1,
    /// Generic assurance evidence sets keyed by evidence-set digest.
    pub assurance_sets: &'a BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    /// Content-addressed assurance evidence bytes.
    pub assurance_blobs: &'a BTreeMap<Sha256Digest, Vec<u8>>,
    /// Typed reliability artifacts keyed by their canonical artifact digests.
    pub reliability_artifacts: &'a BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
    /// Caller-owned verification-key registry for assurance, offer, and reliability keys.
    pub verification_keys: &'a VerificationKeyRegistry,
}

/// Closed deterministic offer rejection taxonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EligibilityFailureV1 {
    /// Input schema is unsupported.
    UnsupportedSchema,
    /// A required digest is zero or inconsistent.
    InvalidDigest,
    /// Provider is absent from the supplied registry snapshot.
    ProviderUnknown,
    /// Provider is not active.
    ProviderInactive,
    /// Provider identity is not valid at evaluation time.
    ProviderOutsideValidity,
    /// Provider is not on the buyer allow-list.
    ProviderNotAllowed,
    /// Provider is explicitly denied.
    ProviderDenied,
    /// Provider does not declare authorization for the service.
    ProviderServiceUnauthorized,
    /// Offer does not bind the provider identity.
    ProviderBindingMismatch,
    /// Offer does not bind the required service.
    ServiceBindingMismatch,
    /// Provider authentication of the exact offer is absent or invalid.
    OfferSignatureInvalid,
    /// Offer is not valid at evaluation time.
    OfferOutsideValidity,
    /// Offer has no capacity.
    OfferHasNoCapacity,
    /// Settlement asset differs.
    SettlementAssetMismatch,
    /// Region differs or is not allowed.
    RegionMismatch,
    /// Service privacy is below the buyer minimum.
    PrivacyModeInsufficient,
    /// A required inference capability is absent.
    MissingCapability,
    /// Input bound cannot be served.
    InputTokenLimitMismatch,
    /// Output bound cannot be served.
    OutputTokenLimitMismatch,
    /// Success probability is zero or below the buyer floor.
    SuccessProbabilityBelowMinimum,
    /// Latency exceeds the buyer ceiling.
    LatencyAboveMaximum,
    /// Assurance set is absent or invalid.
    AssuranceEvidenceInvalid,
    /// Typed reliability artifact is absent, stale, untrusted, tampered, or inconsistent.
    ReliabilityEvidenceInvalid,
    /// Assurance semantics fail the buyer policy.
    AssurancePolicyRejected,
    /// Maximum quote exceeds buyer authorization.
    QuoteExceedsBuyerMaximum,
    /// Exact arithmetic failed.
    ArithmeticFailure,
}

/// Maximum and risk-adjusted route cost in atomic settlement units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostQuoteV1 {
    /// Provider maximum input charge.
    pub input_charge_max: AtomicAmount,
    /// Provider maximum output charge.
    pub output_charge_max: AtomicAmount,
    /// Fixed plus input plus output provider quote.
    pub provider_quote_max: AtomicAmount,
    /// Transparent protocol fee.
    pub protocol_fee_max: AtomicAmount,
    /// Maximum buyer debit for this route.
    pub buyer_quote_max: AtomicAmount,
    /// `ceil(buyer_quote_max / success_probability)`.
    pub expected_successful_cost: AtomicAmount,
}

/// Complete eligibility result for one offer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EligibilityReportV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Service specification digest.
    pub service_spec_digest: Sha256Digest,
    /// Provider identity digest when the provider exists.
    pub provider_identity_digest: Option<Sha256Digest>,
    /// Offer digest.
    pub offer_digest: Sha256Digest,
    /// Request-policy digest.
    pub request_policy_digest: Sha256Digest,
    /// Buyer replay snapshot commitment.
    pub replay_snapshot_digest: Sha256Digest,
    /// Explicit evaluation time.
    pub evaluated_at: UtcTimestamp,
    /// True only when the failure set is empty.
    pub eligible: bool,
    /// Deterministically ordered failures.
    pub failures: Vec<EligibilityFailureV1>,
    /// Cost available only for otherwise well-formed arithmetic.
    pub cost: Option<CostQuoteV1>,
    /// Digest of every preceding field.
    pub report_digest: Sha256Digest,
}

#[derive(Serialize)]
struct EligibilityIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    service_spec_digest: Sha256Digest,
    provider_identity_digest: Option<Sha256Digest>,
    offer_digest: Sha256Digest,
    request_policy_digest: Sha256Digest,
    replay_snapshot_digest: Sha256Digest,
    evaluated_at: &'a UtcTimestamp,
    eligible: bool,
    failures: &'a [EligibilityFailureV1],
    cost: Option<CostQuoteV1>,
}

/// Route candidate record retained for reconstruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfferRouteRecordV1 {
    /// Offer digest.
    pub offer_digest: Sha256Digest,
    /// Eligibility report digest.
    pub eligibility_report_digest: Sha256Digest,
    /// Provider identity.
    pub provider_id: hyperintel_core::ProtocolId,
    /// Whether the offer was eligible.
    pub eligible: bool,
    /// Cost decomposition when available.
    pub cost: Option<CostQuoteV1>,
}

/// Deterministic selection receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteDecisionV1 {
    /// Fixed schema identifier.
    pub schema_version: String,
    /// Named state slice.
    pub state_slice: String,
    /// Buyer request policy.
    pub request_policy_digest: Sha256Digest,
    /// Required service specification.
    pub service_spec_digest: Sha256Digest,
    /// Replay-state commitment.
    pub replay_snapshot_digest: Sha256Digest,
    /// Candidates sorted by offer digest.
    pub candidates: Vec<OfferRouteRecordV1>,
    /// Chosen provider.
    pub selected_provider_id: hyperintel_core::ProtocolId,
    /// Chosen offer digest.
    pub selected_offer_digest: Sha256Digest,
    /// Chosen provider identity digest.
    pub selected_provider_identity_digest: Sha256Digest,
    /// Exact cost decomposition.
    pub selected_cost: CostQuoteV1,
    /// Frozen routing algorithm identifier.
    pub routing_algorithm: String,
    /// Explicit evaluation time.
    pub evaluated_at: UtcTimestamp,
    /// Pure-data records grant no execution authority.
    pub authority_granted: bool,
    /// Digest of every preceding field.
    pub route_digest: Sha256Digest,
}

#[derive(Serialize)]
struct RouteIdentity<'a> {
    schema_version: &'a str,
    state_slice: &'a str,
    request_policy_digest: Sha256Digest,
    service_spec_digest: Sha256Digest,
    replay_snapshot_digest: Sha256Digest,
    candidates: &'a [OfferRouteRecordV1],
    selected_provider_id: &'a hyperintel_core::ProtocolId,
    selected_offer_digest: Sha256Digest,
    selected_provider_identity_digest: Sha256Digest,
    selected_cost: CostQuoteV1,
    routing_algorithm: &'a str,
    evaluated_at: &'a UtcTimestamp,
    authority_granted: bool,
}

/// Routing and reconstruction failures.
#[derive(Debug, Error)]
pub enum RoutingError {
    /// No offer passed every hard gate.
    #[error("no eligible capacity offer")]
    NoEligibleOffer,
    /// Duplicate canonical offer identity.
    #[error("duplicate offer identity")]
    DuplicateOffer,
    /// A verified provider key signed distinct offer digests at one logical sequence.
    #[error("verified capacity-offer signing sequence equivocation")]
    VerifiedOfferSigningSequenceEquivocation,
    /// A verified reliability key signed distinct artifact digests at one logical sequence.
    #[error("verified reliability-artifact signing sequence equivocation")]
    VerifiedReliabilitySigningSequenceEquivocation,
    /// Required provider is absent.
    #[error("selected provider is absent")]
    ProviderUnknown,
    /// Exact arithmetic failed.
    #[error("route arithmetic failed: {0}")]
    Arithmetic(#[from] ExactArithmeticError),
    /// Canonicalization failed.
    #[error("route canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Select the lowest expected-successful-cost eligible offer.
///
/// Tie-breaking is buyer quote, p95 latency, highest success probability, then
/// canonical offer digest. Candidate input order cannot change the result.
///
/// # Errors
///
/// Returns on duplicate identity, canonicalization, arithmetic, or no eligible
/// offer. Ineligible offers remain visible in the decision candidate set.
pub fn select_route_v1(context: &RoutingContextV1<'_>) -> Result<RouteDecisionV1, RoutingError> {
    select_route_internal(context, None, None)
}

/// Select among offers only after provider-key authentication of each exact offer digest.
///
/// Invalid signatures become non-compensable eligibility failures. A cheaper unsigned,
/// tampered, cross-provider, wrong-key, expired, revoked, or assurance-unbound offer cannot
/// win. This remains a pure-data local decision and grants no execution authority.
///
/// # Errors
///
/// Returns on duplicate offer identity, canonicalization, arithmetic, or no eligible
/// signed offer.
pub fn select_signed_route_v1(
    context: &SignedRoutingContextV1<'_>,
) -> Result<RouteDecisionV1, RoutingError> {
    let mut offers = Vec::with_capacity(context.offers.len());
    let mut signed_by_digest = BTreeMap::new();
    for signed_offer in context.offers {
        let offer_digest = signed_offer.offer.digest()?;
        if signed_by_digest
            .insert(offer_digest, signed_offer)
            .is_some()
        {
            return Err(RoutingError::DuplicateOffer);
        }
        offers.push(signed_offer.offer.clone());
    }
    let unsigned_context = RoutingContextV1 {
        evaluated_at: context.evaluated_at,
        service: context.service,
        providers: context.providers,
        offers: &offers,
        policy: context.policy,
        assurance_sets: context.assurance_sets,
        assurance_blobs: context.assurance_blobs,
        verification_keys: context.verification_keys,
    };
    select_route_internal(&unsigned_context, Some(&signed_by_digest), None)
}

/// Select among provider-authenticated offers whose typed reliability artifact also verifies.
///
/// Missing, stale, untrusted, tampered, cross-provider, cross-service, or arithmetically
/// inconsistent reliability artifacts become non-compensable candidate failures. They cannot be
/// offset by lower price, lower latency, higher declared capacity, or a valid offer signature.
///
/// # Errors
///
/// Returns on duplicate offer identity, verified signing-sequence equivocation,
/// canonicalization, arithmetic, or when no fully eligible offer remains.
pub fn select_reliability_artifact_verified_route_v1(
    context: &ReliabilityArtifactVerifiedRoutingContextV1<'_>,
) -> Result<RouteDecisionV1, RoutingError> {
    let mut offers = Vec::with_capacity(context.offers.len());
    let mut signed_by_digest = BTreeMap::new();
    for signed_offer in context.offers {
        let offer_digest = signed_offer.offer.digest()?;
        if signed_by_digest
            .insert(offer_digest, signed_offer)
            .is_some()
        {
            return Err(RoutingError::DuplicateOffer);
        }
        offers.push(signed_offer.offer.clone());
    }
    let unsigned_context = RoutingContextV1 {
        evaluated_at: context.evaluated_at,
        service: context.service,
        providers: context.providers,
        offers: &offers,
        policy: context.policy,
        assurance_sets: context.assurance_sets,
        assurance_blobs: context.assurance_blobs,
        verification_keys: context.verification_keys,
    };
    select_route_internal(
        &unsigned_context,
        Some(&signed_by_digest),
        Some(context.reliability_artifacts),
    )
}

fn select_route_internal(
    context: &RoutingContextV1<'_>,
    signed_offers: Option<&BTreeMap<Sha256Digest, &SignedCapacityOfferV1>>,
    reliability_artifacts: Option<&BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>>,
) -> Result<RouteDecisionV1, RoutingError> {
    let service_digest = context.service.digest()?;
    let policy_digest = context.policy.digest()?;
    let mut reports = Vec::with_capacity(context.offers.len());
    let mut seen = BTreeSet::new();
    for offer in context.offers {
        let offer_digest = offer.digest()?;
        if !seen.insert(offer_digest) {
            return Err(RoutingError::DuplicateOffer);
        }
        reports.push(evaluate_offer(
            context,
            offer,
            service_digest,
            policy_digest,
            signed_offers,
            reliability_artifacts,
        )?);
    }
    reports.sort_by_key(|report| report.offer_digest);
    if let Some(signed_offers) = signed_offers {
        reject_verified_signing_sequence_equivocation(&reports, signed_offers)?;
    }
    if let Some(reliability_artifacts) = reliability_artifacts {
        reject_verified_reliability_sequence_equivocation(context, reliability_artifacts)?;
    }

    let chosen = reports
        .iter()
        .filter_map(|report| {
            report
                .cost
                .filter(|_| report.eligible)
                .map(|cost| (report, cost))
        })
        .min_by_key(|(report, cost)| {
            let offer = context
                .offers
                .iter()
                .find(|offer| offer.digest().ok() == Some(report.offer_digest));
            (
                cost.expected_successful_cost,
                cost.buyer_quote_max,
                offer.map_or(u64::MAX, |value| value.p95_latency_ms),
                std::cmp::Reverse(offer.map_or(0, |value| value.success_probability.get())),
                report.offer_digest,
            )
        })
        .ok_or(RoutingError::NoEligibleOffer)?;
    let selected_offer = context
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(chosen.0.offer_digest))
        .ok_or(RoutingError::NoEligibleOffer)?;
    let selected_provider = context
        .providers
        .get(&selected_offer.provider_id)
        .ok_or(RoutingError::ProviderUnknown)?;
    let candidates = reports
        .iter()
        .map(|report| OfferRouteRecordV1 {
            offer_digest: report.offer_digest,
            eligibility_report_digest: report.report_digest,
            provider_id: context
                .offers
                .iter()
                .find(|offer| offer.digest().ok() == Some(report.offer_digest))
                .map_or_else(
                    || selected_offer.provider_id.clone(),
                    |offer| offer.provider_id.clone(),
                ),
            eligible: report.eligible,
            cost: report.cost,
        })
        .collect::<Vec<_>>();
    let mut decision = RouteDecisionV1 {
        schema_version: "hyperintel.route-decision.v1".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        request_policy_digest: policy_digest,
        service_spec_digest: service_digest,
        replay_snapshot_digest: context.policy.replay_snapshot_digest,
        candidates,
        selected_provider_id: selected_offer.provider_id.clone(),
        selected_offer_digest: chosen.0.offer_digest,
        selected_provider_identity_digest: selected_provider.digest()?,
        selected_cost: chosen.1,
        routing_algorithm: if reliability_artifacts.is_some() {
            "expected-successful-cost-reliability-artifact-v1".to_owned()
        } else {
            "expected-successful-cost-v1".to_owned()
        },
        evaluated_at: context.evaluated_at.clone(),
        authority_granted: false,
        route_digest: Sha256Digest::from_bytes([0; 32]),
    };
    decision.route_digest = route_digest(&decision)?;
    Ok(decision)
}

fn evaluate_offer(
    context: &RoutingContextV1<'_>,
    offer: &CapacityOfferV1,
    service_digest: Sha256Digest,
    policy_digest: Sha256Digest,
    signed_offers: Option<&BTreeMap<Sha256Digest, &SignedCapacityOfferV1>>,
    reliability_artifacts: Option<&BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>>,
) -> Result<EligibilityReportV1, RoutingError> {
    let offer_digest = offer.digest()?;
    let mut failures = BTreeSet::new();
    evaluate_product_constraints(
        context,
        offer,
        service_digest,
        policy_digest,
        offer_digest,
        &mut failures,
    );
    if let Some(reliability_artifacts) = reliability_artifacts
        && verify_offer_reliability_v1(
            reliability_artifacts,
            offer,
            context.evaluated_at,
            context.verification_keys,
        )
        .is_err()
    {
        failures.insert(EligibilityFailureV1::ReliabilityEvidenceInvalid);
    }

    let provider = context.providers.get(&offer.provider_id);
    let provider_identity_digest = provider.and_then(|value| value.digest().ok());
    evaluate_provider_constraints(
        context,
        offer,
        offer_digest,
        service_digest,
        provider,
        signed_offers,
        &mut failures,
    );

    let cost = if let Ok(cost) = quote_max_v1(offer, context.policy) {
        if cost.buyer_quote_max > context.policy.max_total_cost {
            failures.insert(EligibilityFailureV1::QuoteExceedsBuyerMaximum);
        }
        Some(cost)
    } else {
        failures.insert(EligibilityFailureV1::ArithmeticFailure);
        None
    };
    let failures = failures.into_iter().collect::<Vec<_>>();
    let eligible = failures.is_empty();
    let mut report = EligibilityReportV1 {
        schema_version: "hyperintel.eligibility-report.v1".to_owned(),
        state_slice: MARKET_STATE_SLICE_V1.to_owned(),
        service_spec_digest: service_digest,
        provider_identity_digest,
        offer_digest,
        request_policy_digest: policy_digest,
        replay_snapshot_digest: context.policy.replay_snapshot_digest,
        evaluated_at: context.evaluated_at.clone(),
        eligible,
        failures,
        cost,
        report_digest: Sha256Digest::from_bytes([0; 32]),
    };
    report.report_digest = eligibility_digest(&report)?;
    Ok(report)
}

fn evaluate_product_constraints(
    context: &RoutingContextV1<'_>,
    offer: &CapacityOfferV1,
    service_digest: Sha256Digest,
    policy_digest: Sha256Digest,
    offer_digest: Sha256Digest,
    failures: &mut BTreeSet<EligibilityFailureV1>,
) {
    if context.service.schema_version != "hyperintel.service-spec.v1"
        || context.policy.schema_version != "hyperintel.request-policy.v1"
        || offer.schema_version != "hyperintel.capacity-offer.v1"
    {
        failures.insert(EligibilityFailureV1::UnsupportedSchema);
    }
    if service_digest.is_zero()
        || policy_digest.is_zero()
        || offer_digest.is_zero()
        || context.policy.prompt_digest.is_zero()
        || context.policy.replay_snapshot_digest.is_zero()
        || offer.reliability_evidence_digest.is_zero()
    {
        failures.insert(EligibilityFailureV1::InvalidDigest);
    }
    if context.policy.required_service_spec_digest != service_digest
        || offer.service_spec_digest != service_digest
    {
        failures.insert(EligibilityFailureV1::ServiceBindingMismatch);
    }
    if context.policy.valid_from > *context.evaluated_at
        || context.policy.valid_until < *context.evaluated_at
        || offer.valid_from > *context.evaluated_at
        || offer.valid_until < *context.evaluated_at
    {
        failures.insert(EligibilityFailureV1::OfferOutsideValidity);
    }
    if offer.available_request_units == 0 {
        failures.insert(EligibilityFailureV1::OfferHasNoCapacity);
    }
    if offer.settlement_asset != context.policy.settlement_asset {
        failures.insert(EligibilityFailureV1::SettlementAssetMismatch);
    }
    if offer.region != context.service.region
        || !context.policy.allowed_regions.contains(&offer.region)
    {
        failures.insert(EligibilityFailureV1::RegionMismatch);
    }
    if context.service.privacy_mode < context.policy.minimum_privacy_mode {
        failures.insert(EligibilityFailureV1::PrivacyModeInsufficient);
    }
    if !context
        .policy
        .required_capabilities
        .is_subset(&context.service.capabilities)
    {
        failures.insert(EligibilityFailureV1::MissingCapability);
    }
    if context.policy.max_input_tokens > context.service.max_input_tokens
        || context.policy.max_input_tokens > offer.max_input_tokens
    {
        failures.insert(EligibilityFailureV1::InputTokenLimitMismatch);
    }
    if context.policy.max_output_tokens > context.service.max_output_tokens
        || context.policy.max_output_tokens > offer.max_output_tokens
    {
        failures.insert(EligibilityFailureV1::OutputTokenLimitMismatch);
    }
    if offer.success_probability.get() == 0
        || offer.success_probability < context.policy.minimum_success_probability
    {
        failures.insert(EligibilityFailureV1::SuccessProbabilityBelowMinimum);
    }
    if offer.p95_latency_ms > context.policy.max_p95_latency_ms {
        failures.insert(EligibilityFailureV1::LatencyAboveMaximum);
    }
}

fn evaluate_provider_constraints(
    context: &RoutingContextV1<'_>,
    offer: &CapacityOfferV1,
    offer_digest: Sha256Digest,
    service_digest: Sha256Digest,
    provider: Option<&ProviderIdentityV1>,
    signed_offers: Option<&BTreeMap<Sha256Digest, &SignedCapacityOfferV1>>,
    failures: &mut BTreeSet<EligibilityFailureV1>,
) {
    if let Some(provider) = provider {
        if provider.schema_version != "hyperintel.provider-identity.v1" {
            failures.insert(EligibilityFailureV1::UnsupportedSchema);
        }
        if provider.status != ProviderStatus::Active {
            failures.insert(EligibilityFailureV1::ProviderInactive);
        }
        if provider.valid_from > *context.evaluated_at
            || provider.valid_until < *context.evaluated_at
        {
            failures.insert(EligibilityFailureV1::ProviderOutsideValidity);
        }
        if provider.provider_id != offer.provider_id {
            failures.insert(EligibilityFailureV1::ProviderBindingMismatch);
        }
        if !provider
            .authorized_service_digests
            .contains(&service_digest)
        {
            failures.insert(EligibilityFailureV1::ProviderServiceUnauthorized);
        }
        if !context.policy.allowed_providers.is_empty()
            && !context
                .policy
                .allowed_providers
                .contains(&provider.provider_id)
        {
            failures.insert(EligibilityFailureV1::ProviderNotAllowed);
        }
        if context
            .policy
            .denied_providers
            .contains(&provider.provider_id)
        {
            failures.insert(EligibilityFailureV1::ProviderDenied);
        }
        if let Ok(provider_envelope_digest) =
            verify_offer_assurance(context, provider, service_digest)
        {
            if let Some(signed_offers) = signed_offers {
                let signature_is_valid =
                    signed_offers
                        .get(&offer_digest)
                        .is_some_and(|signed_offer| {
                            verify_signed_capacity_offer_v1(
                                signed_offer,
                                provider,
                                provider_envelope_digest,
                                context.verification_keys,
                                context.evaluated_at,
                            )
                            .is_ok()
                        });
                if !signature_is_valid {
                    failures.insert(EligibilityFailureV1::OfferSignatureInvalid);
                }
            }
        } else {
            failures.insert(EligibilityFailureV1::AssurancePolicyRejected);
            if signed_offers.is_some() {
                failures.insert(EligibilityFailureV1::OfferSignatureInvalid);
            }
        }
    } else {
        failures.insert(EligibilityFailureV1::ProviderUnknown);
        if signed_offers.is_some() {
            failures.insert(EligibilityFailureV1::OfferSignatureInvalid);
        }
    }
}

fn reject_verified_signing_sequence_equivocation(
    reports: &[EligibilityReportV1],
    signed_offers: &BTreeMap<Sha256Digest, &SignedCapacityOfferV1>,
) -> Result<(), RoutingError> {
    let mut verified_sequences = BTreeSet::new();
    for report in reports {
        if report
            .failures
            .contains(&EligibilityFailureV1::OfferSignatureInvalid)
        {
            continue;
        }
        let Some(signed_offer) = signed_offers.get(&report.offer_digest) else {
            continue;
        };
        let sequence_identity = (
            signed_offer.statement.signer_id.clone(),
            signed_offer.statement.key_id.clone(),
            signed_offer.statement.sequence,
        );
        if !verified_sequences.insert(sequence_identity) {
            return Err(RoutingError::VerifiedOfferSigningSequenceEquivocation);
        }
    }
    Ok(())
}

fn reject_verified_reliability_sequence_equivocation(
    context: &RoutingContextV1<'_>,
    reliability_artifacts: &BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
) -> Result<(), RoutingError> {
    let mut verified_sequences = BTreeMap::new();
    for offer in context.offers {
        let Ok(artifact) = verify_offer_reliability_v1(
            reliability_artifacts,
            offer,
            context.evaluated_at,
            context.verification_keys,
        ) else {
            continue;
        };
        let sequence_identity = (
            artifact.statement.signer_id.clone(),
            artifact.statement.key_id.clone(),
            artifact.statement.sequence,
        );
        if verified_sequences
            .insert(sequence_identity, artifact.artifact.artifact_digest)
            .is_some_and(|existing| existing != artifact.artifact.artifact_digest)
        {
            return Err(RoutingError::VerifiedReliabilitySigningSequenceEquivocation);
        }
    }
    Ok(())
}

fn verify_offer_assurance(
    context: &RoutingContextV1<'_>,
    provider: &ProviderIdentityV1,
    service_digest: Sha256Digest,
) -> Result<Sha256Digest, ()> {
    let provider_digest = provider.digest().map_err(|_| ())?;
    let provider_claim = AssuranceClaimV1 {
        subject_digest: provider_digest,
        property: AssuranceProperty::ProviderAuthorization,
    };
    let service_claim = AssuranceClaimV1 {
        subject_digest: service_digest,
        property: AssuranceProperty::ServiceManifestBinding,
    };
    let provider_set = context
        .assurance_sets
        .values()
        .find(|set| set.envelope.guarantees.contains(&provider_claim))
        .ok_or(())?;
    let offer_set = context
        .assurance_sets
        .values()
        .find(|set| set.envelope.guarantees.contains(&service_claim))
        .ok_or(())?;
    verify_assurance_evidence_set_v1(
        provider_set,
        context.assurance_blobs,
        context.verification_keys,
        context.evaluated_at,
    )
    .map_err(|_| ())?;
    verify_assurance_evidence_set_v1(
        offer_set,
        context.assurance_blobs,
        context.verification_keys,
        context.evaluated_at,
    )
    .map_err(|_| ())?;
    let provider_statement_is_bound = provider_set.signed_statements.iter().any(|statement| {
        statement.subject_kind.as_str() == "provider-identity"
            && statement.subject_digest == provider_digest
            && statement.signer_id == provider.provider_id
            && statement.key_id == provider.offer_key_id
            && context
                .verification_keys
                .trust_root_class(&statement.signer_id, &statement.key_id)
                == Some(hyperintel_assurance::AssuranceRootClassV1::ProviderKey)
    });
    if !provider_statement_is_bound {
        return Err(());
    }
    let service_statement_is_bound = offer_set.signed_statements.iter().any(|statement| {
        statement.subject_kind.as_str() == "service-spec"
            && statement.subject_digest == service_digest
            && context
                .verification_keys
                .trust_root_class(&statement.signer_id, &statement.key_id)
                == Some(hyperintel_assurance::AssuranceRootClassV1::VerifierKey)
    });
    if !service_statement_is_bound {
        return Err(());
    }
    let combined =
        conjoin_envelopes_v1(&provider_set.envelope, &offer_set.envelope).map_err(|_| ())?;
    let mut required = BTreeSet::from([provider_claim, service_claim]);
    for property in &context.policy.required_assurance_properties {
        let subject_digest = match property {
            AssuranceProperty::ProviderAuthorization => provider_digest,
            AssuranceProperty::ServiceManifestBinding
            | AssuranceProperty::RuntimeMeasurement
            | AssuranceProperty::RegionConfinement
            | AssuranceProperty::NoContentRetentionDeclared
            | AssuranceProperty::MeteringIntegrity => service_digest,
            AssuranceProperty::PolicyCompliance => return Err(()),
        };
        required.insert(AssuranceClaimV1 {
            subject_digest,
            property: property.clone(),
        });
    }
    evaluate_assurance_v1(
        &combined,
        &AssuranceAcceptancePolicyV1 {
            required_guarantees: required,
            minimum_maturity: context.policy.minimum_assurance_maturity,
            require_closed_assumptions: true,
            forbidden_root_classes: BTreeSet::new(),
            evaluated_at: context.evaluated_at.clone(),
        },
    )
    .map_err(|_| ())?;
    provider_set.envelope.digest().map_err(|_| ())
}

pub(crate) fn quote_max_v1(
    offer: &CapacityOfferV1,
    policy: &InferenceRequestPolicyV1,
) -> Result<CostQuoteV1, ExactArithmeticError> {
    let input_charge_max = offer
        .input_price_per_million
        .checked_ceil_mul_div(policy.max_input_tokens, 1_000_000)?;
    let output_charge_max = offer
        .output_price_per_million
        .checked_ceil_mul_div(policy.max_output_tokens, 1_000_000)?;
    let provider_quote_max = offer
        .fixed_request_fee
        .checked_add(input_charge_max)?
        .checked_add(output_charge_max)?;
    let protocol_fee_max =
        provider_quote_max.checked_ceil_mul_div(u64::from(policy.protocol_fee.get()), 10_000)?;
    let buyer_quote_max = provider_quote_max.checked_add(protocol_fee_max)?;
    let expected_successful_cost = buyer_quote_max
        .checked_ceil_mul_div(1_000_000, u64::from(offer.success_probability.get()))?;
    Ok(CostQuoteV1 {
        input_charge_max,
        output_charge_max,
        provider_quote_max,
        protocol_fee_max,
        buyer_quote_max,
        expected_successful_cost,
    })
}

fn eligibility_digest(report: &EligibilityReportV1) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.eligibility-report.v1",
        &EligibilityIdentity {
            schema_version: &report.schema_version,
            state_slice: &report.state_slice,
            service_spec_digest: report.service_spec_digest,
            provider_identity_digest: report.provider_identity_digest,
            offer_digest: report.offer_digest,
            request_policy_digest: report.request_policy_digest,
            replay_snapshot_digest: report.replay_snapshot_digest,
            evaluated_at: &report.evaluated_at,
            eligible: report.eligible,
            failures: &report.failures,
            cost: report.cost,
        },
    )
}

pub(crate) fn route_digest(
    decision: &RouteDecisionV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(
        "hyperintel.route-decision.v1",
        &RouteIdentity {
            schema_version: &decision.schema_version,
            state_slice: &decision.state_slice,
            request_policy_digest: decision.request_policy_digest,
            service_spec_digest: decision.service_spec_digest,
            replay_snapshot_digest: decision.replay_snapshot_digest,
            candidates: &decision.candidates,
            selected_provider_id: &decision.selected_provider_id,
            selected_offer_digest: decision.selected_offer_digest,
            selected_provider_identity_digest: decision.selected_provider_identity_digest,
            selected_cost: decision.selected_cost,
            routing_algorithm: &decision.routing_algorithm,
            evaluated_at: &decision.evaluated_at,
            authority_granted: decision.authority_granted,
        },
    )
}
