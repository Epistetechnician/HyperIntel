//! Contract tests for the hermetic route-to-settlement evidence slice.

use std::collections::{BTreeMap, BTreeSet};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperintel_assurance::{
    AssuranceClaimV1, AssuranceEnvelopeV1, AssuranceEvidenceSetV1, AssuranceRootClassV1,
    AssuranceRootV1, EvidenceClassification, RegisteredVerificationKeyV1, SignedStatementV1,
    ValidityWindowV1, VerificationKeyRegistry, build_assurance_evidence_set_v1,
    evidence_blob_ref_v1,
};
use hyperintel_core::{
    AssuranceMaturity, AssuranceProperty, AtomicAmount, BasisPoints, CapacityOfferV1,
    InferenceCapability, InferenceRequestPolicyV1, MeteringMethod, PartsPerMillion, PrivacyMode,
    ProtocolId, ProviderIdentityV1, ProviderStatus, RetentionMode, ServiceSpecV1, Sha256Digest,
    UtcTimestamp,
};
use hyperintel_market::{
    ExecutionStatusV1, MarketEvidenceBundleV1, MeteringReceiptDraftV1, RouteDecisionV1,
    RoutingContextV1, RoutingError, SignedCapacityOfferV1, SignedMarketEvidenceBundleV2,
    SignedRoutingContextV1, build_market_evidence_bundle_v1, build_metering_receipt_v1,
    build_signed_market_evidence_bundle_v2, derive_settlement_intent_v1, select_route_v1,
    select_signed_route_v1, verify_market_evidence_bundle_v1,
    verify_signed_market_evidence_bundle_v2,
};

struct Fixture {
    evaluated_at: UtcTimestamp,
    service: ServiceSpecV1,
    providers: BTreeMap<ProtocolId, ProviderIdentityV1>,
    offers: Vec<CapacityOfferV1>,
    signed_offers: Vec<SignedCapacityOfferV1>,
    policy: InferenceRequestPolicyV1,
    assurance_sets: BTreeMap<Sha256Digest, AssuranceEvidenceSetV1>,
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    registry: VerificationKeyRegistry,
    verification_keys: Vec<RegisteredVerificationKeyV1>,
}

#[test]
fn contract_vertical_slice_reconstructs_route_metering_settlement_and_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let route = route(&fixture)?;
    assert_eq!(route.selected_provider_id.as_str(), "provider:beta");
    assert!(!route.authority_granted);

    let provider = fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected provider missing")?;
    let offer = fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected offer missing")?;
    let metering_draft = MeteringReceiptDraftV1 {
        started_at: timestamp("2026-07-20T12:00:01Z")?,
        first_token_at: timestamp("2026-07-20T12:00:02Z")?,
        completed_at: timestamp("2026-07-20T12:00:04Z")?,
        input_tokens: 900,
        output_tokens: 400,
        execution_status: ExecutionStatusV1::Completed,
        prompt_digest: fixture.policy.prompt_digest,
        output_digest: Sha256Digest::from_bytes([31; 32]),
        provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
        buyer_observation_digest: Some(Sha256Digest::from_bytes([33; 32])),
    };
    let metering = build_metering_receipt_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        metering_draft.clone(),
    )?;
    assert!(metering.actual_provider_charge < route.selected_cost.provider_quote_max);
    assert!(!metering.authority_granted);

    assert_authorization_windows(
        &fixture,
        provider,
        offer,
        &route,
        &metering_draft,
        &metering,
    )?;
    let settlement = derive_settlement_intent_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    assert_eq!(
        settlement
            .provider_payout_amount
            .checked_add(settlement.protocol_fee_amount)?
            .checked_add(settlement.buyer_refund_amount)?,
        settlement.escrow_amount
    );
    assert!(!settlement.authority_granted);

    let bundle = build_market_evidence_bundle_v1(
        fixture.evaluated_at.clone(),
        fixture.service.clone(),
        fixture.providers.clone(),
        fixture.offers.clone(),
        fixture.policy.clone(),
        fixture.assurance_sets.clone(),
        fixture.verification_keys.clone(),
        route,
        metering,
        settlement,
        vec![
            "provider authorization is evaluated only from supplied signed evidence".to_owned(),
            "success probability is an evidence-qualified declared input".to_owned(),
        ],
    )?;
    verify_market_evidence_bundle_v1(&bundle, &fixture.assurance_blobs, &fixture.registry)?;
    let mut undisclosed_keys = bundle.clone();
    undisclosed_keys.verification_keys.clear();
    assert!(
        verify_market_evidence_bundle_v1(
            &undisclosed_keys,
            &fixture.assurance_blobs,
            &fixture.registry
        )
        .is_err()
    );
    assert!(
        verify_market_evidence_bundle_v1(
            &bundle,
            &fixture.assurance_blobs,
            &VerificationKeyRegistry::default()
        )
        .is_err()
    );

    let encoded = serde_json::to_vec(&bundle)?;
    let decoded: MarketEvidenceBundleV1 = serde_json::from_slice(&encoded)?;
    assert_eq!(decoded, bundle);
    Ok(())
}

#[test]
fn contract_signed_offer_v2_reconstructs_and_has_a_stable_golden_digest()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let bundle = signed_bundle(&fixture)?;
    verify_signed_market_evidence_bundle_v2(&bundle, &fixture.assurance_blobs, &fixture.registry)?;
    assert!(
        !bundle
            .explicit_nonclaims
            .iter()
            .any(|claim| claim == "no-capacity-offer-signature-verification")
    );
    assert_eq!(
        bundle.bundle_digest.to_hex(),
        "781fab0921e649922f53ceb67f9037ddfd0f039922d3663ef54b29f7bd042cab"
    );
    let encoded = serde_json::to_vec(&bundle)?;
    let decoded: SignedMarketEvidenceBundleV2 = serde_json::from_slice(&encoded)?;
    assert_eq!(decoded, bundle);

    let mut reversed = fixture.signed_offers.clone();
    reversed.reverse();
    assert_eq!(
        signed_route(&fixture)?,
        signed_route_with(&fixture, &reversed, &fixture.registry)?
    );
    Ok(())
}

#[test]
fn contract_invalid_or_tampered_capacity_offer_signature_cannot_win()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    assert_eq!(
        signed_route(&fixture)?.selected_provider_id.as_str(),
        "provider:beta"
    );

    let beta_index = fixture
        .signed_offers
        .iter()
        .position(|signed_offer| signed_offer.offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta signed offer missing")?;

    let mut cases = Vec::new();

    let mut subject_tamper = fixture.signed_offers.clone();
    subject_tamper[beta_index].statement.subject_digest = Sha256Digest::from_bytes([99; 32]);
    cases.push(subject_tamper);

    let mut wrong_signer = fixture.signed_offers.clone();
    wrong_signer[beta_index].statement.signer_id = "provider:alpha".parse()?;
    cases.push(wrong_signer);

    let mut wrong_key = fixture.signed_offers.clone();
    wrong_key[beta_index].statement.key_id = "key:alpha-offer".parse()?;
    cases.push(wrong_key);

    let mut expired = fixture.signed_offers.clone();
    expired[beta_index].statement.expires_at = timestamp("2026-07-20T11:59:59Z")?;
    cases.push(expired);

    let mut offer_tamper = fixture.signed_offers.clone();
    offer_tamper[beta_index].offer.fixed_request_fee = AtomicAmount(0);
    cases.push(offer_tamper);

    for invalid_offers in cases {
        let decision = signed_route_with(&fixture, &invalid_offers, &fixture.registry)?;
        assert_eq!(decision.selected_provider_id.as_str(), "provider:alpha");
    }

    let mut missing_statement = serde_json::to_value(&fixture.signed_offers[beta_index])?;
    missing_statement
        .as_object_mut()
        .ok_or("signed offer must serialize as an object")?
        .remove("statement");
    assert!(serde_json::from_value::<SignedCapacityOfferV1>(missing_statement).is_err());

    let mut revoked_registry = VerificationKeyRegistry::default();
    for mut key in fixture.verification_keys.clone() {
        if key.signer_id.as_str() == "provider:beta" && key.key_id.as_str() == "key:beta-offer" {
            key.revoked_at = Some(fixture.evaluated_at.clone());
        }
        revoked_registry.register(key)?;
    }
    let revoked_route = signed_route_with(&fixture, &fixture.signed_offers, &revoked_registry)?;
    assert_eq!(
        revoked_route.selected_provider_id.as_str(),
        "provider:alpha"
    );
    Ok(())
}

#[test]
fn contract_only_verified_offer_sequence_equivocation_blocks_the_batch()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let alpha_index = fixture
        .signed_offers
        .iter()
        .position(|signed_offer| signed_offer.offer.provider_id.as_str() == "provider:alpha")
        .ok_or("alpha signed offer missing")?;
    let beta_index = fixture
        .signed_offers
        .iter()
        .position(|signed_offer| signed_offer.offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta signed offer missing")?;
    let alpha = &fixture.signed_offers[alpha_index];

    let mut second_alpha_offer = alpha.offer.clone();
    second_alpha_offer.offer_id = "offer:alpha-equivocation".parse()?;
    second_alpha_offer.fixed_request_fee = second_alpha_offer
        .fixed_request_fee
        .checked_add(AtomicAmount(1))?;
    let second_alpha = signed_capacity_offer(
        second_alpha_offer,
        50,
        alpha.statement.assurance_envelope_digest,
        alpha.statement.sequence,
    )?;
    let mut verified_equivocation = fixture.signed_offers.clone();
    verified_equivocation.push(second_alpha);
    assert!(matches!(
        signed_route_with(
            &fixture,
            &verified_equivocation,
            &fixture.registry
        ),
        Err(error)
            if error
                .downcast_ref::<RoutingError>()
                .is_some_and(|routing_error| matches!(
                    routing_error,
                    RoutingError::VerifiedOfferSigningSequenceEquivocation
                ))
    ));

    let mut spoofed_duplicate = fixture.signed_offers.clone();
    spoofed_duplicate[beta_index].statement.signer_id = alpha.statement.signer_id.clone();
    spoofed_duplicate[beta_index].statement.key_id = alpha.statement.key_id.clone();
    spoofed_duplicate[beta_index].statement.sequence = alpha.statement.sequence;
    let fallback = signed_route_with(&fixture, &spoofed_duplicate, &fixture.registry)?;
    assert_eq!(fallback.selected_provider_id.as_str(), "provider:alpha");
    Ok(())
}

#[test]
fn contract_signed_bundle_rejects_offer_signature_and_offer_tamper()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let bundle = signed_bundle(&fixture)?;

    let mut signature_tamper = bundle.clone();
    let replacement = if signature_tamper.offers[0]
        .statement
        .signature_hex
        .starts_with("00")
    {
        "01"
    } else {
        "00"
    };
    signature_tamper.offers[0]
        .statement
        .signature_hex
        .replace_range(0..2, replacement);
    assert!(
        verify_signed_market_evidence_bundle_v2(
            &signature_tamper,
            &fixture.assurance_blobs,
            &fixture.registry
        )
        .is_err()
    );

    let mut offer_tamper = bundle;
    offer_tamper.offers[0].offer.available_request_units = offer_tamper.offers[0]
        .offer
        .available_request_units
        .checked_add(1)
        .ok_or("fixture capacity overflow")?;
    assert!(
        verify_signed_market_evidence_bundle_v2(
            &offer_tamper,
            &fixture.assurance_blobs,
            &fixture.registry
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn contract_v2_bundle_reconstructs_rejected_unknown_key_without_trusting_it()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let mut offers = fixture.signed_offers.clone();
    let beta = offers
        .iter_mut()
        .find(|signed_offer| signed_offer.offer.provider_id.as_str() == "provider:beta")
        .ok_or("beta signed offer missing")?;
    beta.statement.key_id = "key:attacker-claimed".parse()?;

    let bundle = signed_bundle_with(&fixture, &offers)?;
    assert_eq!(bundle.route.selected_provider_id.as_str(), "provider:alpha");
    assert!(
        !bundle
            .verification_keys
            .iter()
            .any(|key| key.key_id.as_str() == "key:attacker-claimed")
    );
    verify_signed_market_evidence_bundle_v2(&bundle, &fixture.assurance_blobs, &fixture.registry)?;
    Ok(())
}

#[test]
fn contract_provider_authorization_is_bound_to_declared_provider_key()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    let alpha = fixture
        .providers
        .get(&"provider:alpha".parse()?)
        .ok_or("alpha provider missing")?
        .clone();
    let alpha_claim = AssuranceClaimV1 {
        subject_digest: alpha.digest()?,
        property: AssuranceProperty::ProviderAuthorization,
    };
    let original_set_digest = fixture
        .assurance_sets
        .iter()
        .find(|(_, set)| set.envelope.guarantees.contains(&alpha_claim))
        .map(|(digest, _)| *digest)
        .ok_or("alpha assurance set missing")?;
    fixture.assurance_sets.remove(&original_set_digest);

    let cross_signed = signed_evidence_set(
        51,
        "provider:beta".parse()?,
        "key:beta-offer".parse()?,
        "provider-identity".parse()?,
        &alpha_claim,
        AssuranceRootClassV1::ProviderKey,
        b"cross-signed alpha authorization".to_vec(),
    )?;
    fixture
        .assurance_blobs
        .insert(cross_signed.1.0, cross_signed.1.1);
    fixture
        .assurance_sets
        .insert(cross_signed.0.evidence_set_digest, cross_signed.0);

    let decision = route(&fixture)?;
    let alpha_candidate = decision
        .candidates
        .iter()
        .find(|candidate| candidate.provider_id == alpha.provider_id)
        .ok_or("alpha route candidate missing")?;
    assert!(!alpha_candidate.eligible);
    Ok(())
}

#[test]
fn contract_policy_compliance_assurance_is_unsupported_in_r0()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    fixture
        .policy
        .required_assurance_properties
        .insert(AssuranceProperty::PolicyCompliance);
    assert!(route(&fixture).is_err());
    Ok(())
}

#[test]
fn contract_route_is_permutation_invariant_and_ineligible_offer_cannot_win()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture()?;
    let baseline = route(&fixture)?;
    fixture.offers.reverse();
    let reversed = route(&fixture)?;
    assert_eq!(baseline, reversed);

    let mut ineligible = fixture.offers[0].clone();
    ineligible.offer_id = "offer:ineligible".parse()?;
    ineligible.region = "region:forbidden".parse()?;
    ineligible.fixed_request_fee = AtomicAmount(0);
    ineligible.input_price_per_million = AtomicAmount(0);
    ineligible.output_price_per_million = AtomicAmount(0);
    fixture.offers.push(ineligible);
    let with_ineligible = route(&fixture)?;
    assert_eq!(
        baseline.selected_offer_digest,
        with_ineligible.selected_offer_digest
    );
    assert_eq!(
        baseline.selected_provider_id,
        with_ineligible.selected_provider_id
    );
    Ok(())
}

#[test]
fn contract_bundle_rejects_semantic_tamper_even_with_original_bundle_digest()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture()?;
    let route = route(&fixture)?;
    let provider = fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected provider missing")?;
    let offer = fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected offer missing")?;
    let metering = build_metering_receipt_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        MeteringReceiptDraftV1 {
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            first_token_at: timestamp("2026-07-20T12:00:02Z")?,
            completed_at: timestamp("2026-07-20T12:00:04Z")?,
            input_tokens: 900,
            output_tokens: 400,
            execution_status: ExecutionStatusV1::Completed,
            prompt_digest: fixture.policy.prompt_digest,
            output_digest: Sha256Digest::from_bytes([31; 32]),
            provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
            buyer_observation_digest: None,
        },
    )?;
    let settlement = derive_settlement_intent_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    let mut bundle = build_market_evidence_bundle_v1(
        fixture.evaluated_at.clone(),
        fixture.service.clone(),
        fixture.providers.clone(),
        fixture.offers.clone(),
        fixture.policy.clone(),
        fixture.assurance_sets.clone(),
        fixture.verification_keys.clone(),
        route,
        metering,
        settlement,
        Vec::new(),
    )?;
    bundle.settlement.buyer_refund_amount = AtomicAmount(
        bundle
            .settlement
            .buyer_refund_amount
            .0
            .checked_add(1)
            .ok_or("fixture overflow")?,
    );
    assert!(
        verify_market_evidence_bundle_v1(&bundle, &fixture.assurance_blobs, &fixture.registry)
            .is_err()
    );
    Ok(())
}

fn route(fixture: &Fixture) -> Result<RouteDecisionV1, Box<dyn std::error::Error>> {
    Ok(select_route_v1(&RoutingContextV1 {
        evaluated_at: &fixture.evaluated_at,
        service: &fixture.service,
        providers: &fixture.providers,
        offers: &fixture.offers,
        policy: &fixture.policy,
        assurance_sets: &fixture.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        verification_keys: &fixture.registry,
    })?)
}

fn signed_route(fixture: &Fixture) -> Result<RouteDecisionV1, Box<dyn std::error::Error>> {
    signed_route_with(fixture, &fixture.signed_offers, &fixture.registry)
}

fn signed_route_with(
    fixture: &Fixture,
    offers: &[SignedCapacityOfferV1],
    registry: &VerificationKeyRegistry,
) -> Result<RouteDecisionV1, Box<dyn std::error::Error>> {
    Ok(select_signed_route_v1(&SignedRoutingContextV1 {
        evaluated_at: &fixture.evaluated_at,
        service: &fixture.service,
        providers: &fixture.providers,
        offers,
        policy: &fixture.policy,
        assurance_sets: &fixture.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        verification_keys: registry,
    })?)
}

fn signed_bundle(
    fixture: &Fixture,
) -> Result<SignedMarketEvidenceBundleV2, Box<dyn std::error::Error>> {
    signed_bundle_with(fixture, &fixture.signed_offers)
}

fn signed_bundle_with(
    fixture: &Fixture,
    offers: &[SignedCapacityOfferV1],
) -> Result<SignedMarketEvidenceBundleV2, Box<dyn std::error::Error>> {
    let route = signed_route_with(fixture, offers, &fixture.registry)?;
    let provider = fixture
        .providers
        .get(&route.selected_provider_id)
        .ok_or("selected provider missing")?;
    let offer = fixture
        .offers
        .iter()
        .find(|offer| offer.digest().ok() == Some(route.selected_offer_digest))
        .ok_or("selected offer missing")?;
    let metering = build_metering_receipt_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        MeteringReceiptDraftV1 {
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            first_token_at: timestamp("2026-07-20T12:00:02Z")?,
            completed_at: timestamp("2026-07-20T12:00:04Z")?,
            input_tokens: 900,
            output_tokens: 400,
            execution_status: ExecutionStatusV1::Completed,
            prompt_digest: fixture.policy.prompt_digest,
            output_digest: Sha256Digest::from_bytes([31; 32]),
            provider_metering_evidence_digest: Sha256Digest::from_bytes([32; 32]),
            buyer_observation_digest: Some(Sha256Digest::from_bytes([33; 32])),
        },
    )?;
    let settlement = derive_settlement_intent_v1(
        &fixture.service,
        provider,
        offer,
        &fixture.policy,
        &route,
        &metering,
        timestamp("2026-07-20T13:00:00Z")?,
    )?;
    Ok(build_signed_market_evidence_bundle_v2(
        fixture.evaluated_at.clone(),
        fixture.service.clone(),
        fixture.providers.clone(),
        offers.to_vec(),
        fixture.policy.clone(),
        fixture.assurance_sets.clone(),
        fixture.verification_keys.clone(),
        route,
        metering,
        settlement,
        vec![
            "capacity-offer authenticity is checked only against caller-supplied keys".to_owned(),
            "success probability remains an unverified declared input".to_owned(),
        ],
    )?)
}

fn assert_authorization_windows(
    fixture: &Fixture,
    provider: &ProviderIdentityV1,
    offer: &CapacityOfferV1,
    route: &RouteDecisionV1,
    draft: &MeteringReceiptDraftV1,
    metering: &hyperintel_market::MeteringReceiptV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut late_draft = draft.clone();
    late_draft.completed_at = timestamp("2026-07-20T14:00:00Z")?;
    assert!(
        build_metering_receipt_v1(
            &fixture.service,
            provider,
            offer,
            &fixture.policy,
            route,
            late_draft
        )
        .is_err()
    );
    assert!(
        derive_settlement_intent_v1(
            &fixture.service,
            provider,
            offer,
            &fixture.policy,
            route,
            metering,
            metering.completed_at.clone()
        )
        .is_err()
    );
    let mut fabricated_metering = metering.clone();
    fabricated_metering.actual_provider_charge = AtomicAmount(
        fabricated_metering
            .actual_provider_charge
            .0
            .checked_add(1)
            .ok_or("fabricated charge overflow")?,
    );
    assert!(
        derive_settlement_intent_v1(
            &fixture.service,
            provider,
            offer,
            &fixture.policy,
            route,
            &fabricated_metering,
            timestamp("2026-07-20T13:00:00Z")?
        )
        .is_err()
    );
    Ok(())
}

#[allow(clippy::too_many_lines)] // A complete cross-crate fixture is intentionally explicit.
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let evaluated_at = timestamp("2026-07-20T12:00:00Z")?;
    let service = ServiceSpecV1 {
        schema_version: "hyperintel.service-spec.v1".to_owned(),
        service_id: "service:open-70b-v1".parse()?,
        model_family: "model:open-70b".parse()?,
        model_version: "version:1".parse()?,
        model_artifact_digest: Sha256Digest::from_bytes([1; 32]),
        tokenizer_digest: Sha256Digest::from_bytes([2; 32]),
        runtime_image_digest: Sha256Digest::from_bytes([3; 32]),
        region: "region:us-east".parse()?,
        privacy_mode: PrivacyMode::DirectSealed,
        retention_mode: RetentionMode::NoContentRetentionDeclared,
        capabilities: [
            InferenceCapability::Streaming,
            InferenceCapability::ToolCalls,
        ]
        .into_iter()
        .collect(),
        max_input_tokens: 4_096,
        max_output_tokens: 1_024,
        metering_method: MeteringMethod::TokenizerDigest,
    };
    let service_digest = service.digest()?;
    let mut providers = BTreeMap::new();
    for name in ["alpha", "beta"] {
        let provider_id: ProtocolId = format!("provider:{name}").parse()?;
        providers.insert(
            provider_id.clone(),
            ProviderIdentityV1 {
                schema_version: "hyperintel.provider-identity.v1".to_owned(),
                provider_id,
                status: ProviderStatus::Active,
                valid_from: timestamp("2026-07-20T00:00:00Z")?,
                valid_until: timestamp("2026-07-21T00:00:00Z")?,
                offer_key_id: format!("key:{name}-offer").parse()?,
                receipt_key_id: format!("key:{name}-receipt").parse()?,
                payout_account: format!("account:{name}").parse()?,
                authorized_service_digests: [service_digest].into_iter().collect(),
            },
        );
    }
    let offers = vec![
        CapacityOfferV1 {
            schema_version: "hyperintel.capacity-offer.v1".to_owned(),
            offer_id: "offer:alpha".parse()?,
            provider_id: "provider:alpha".parse()?,
            service_spec_digest: service_digest,
            valid_from: timestamp("2026-07-20T00:00:00Z")?,
            valid_until: timestamp("2026-07-21T00:00:00Z")?,
            available_request_units: 10,
            max_input_tokens: 4_096,
            max_output_tokens: 1_024,
            fixed_request_fee: AtomicAmount(1_000),
            input_price_per_million: AtomicAmount(10_000),
            output_price_per_million: AtomicAmount(20_000),
            success_probability: PartsPerMillion::new(800_000)?,
            p95_latency_ms: 5_000,
            settlement_asset: "asset:test-usdc".parse()?,
            region: "region:us-east".parse()?,
            reliability_evidence_digest: Sha256Digest::from_bytes([21; 32]),
        },
        CapacityOfferV1 {
            schema_version: "hyperintel.capacity-offer.v1".to_owned(),
            offer_id: "offer:beta".parse()?,
            provider_id: "provider:beta".parse()?,
            service_spec_digest: service_digest,
            valid_from: timestamp("2026-07-20T00:00:00Z")?,
            valid_until: timestamp("2026-07-21T00:00:00Z")?,
            available_request_units: 10,
            max_input_tokens: 4_096,
            max_output_tokens: 1_024,
            fixed_request_fee: AtomicAmount(1_100),
            input_price_per_million: AtomicAmount(10_000),
            output_price_per_million: AtomicAmount(20_000),
            success_probability: PartsPerMillion::new(990_000)?,
            p95_latency_ms: 3_000,
            settlement_asset: "asset:test-usdc".parse()?,
            region: "region:us-east".parse()?,
            reliability_evidence_digest: Sha256Digest::from_bytes([22; 32]),
        },
    ];
    let policy = InferenceRequestPolicyV1 {
        schema_version: "hyperintel.request-policy.v1".to_owned(),
        request_id: "request:demo-1".parse()?,
        buyer_id: "buyer:demo".parse()?,
        buyer_refund_account: "account:buyer-demo".parse()?,
        nonce: 1,
        valid_from: timestamp("2026-07-20T11:00:00Z")?,
        valid_until: timestamp("2026-07-20T13:00:00Z")?,
        required_service_spec_digest: service_digest,
        max_input_tokens: 1_000,
        max_output_tokens: 500,
        max_total_cost: AtomicAmount(2_000),
        settlement_asset: "asset:test-usdc".parse()?,
        allowed_regions: ["region:us-east".parse()?].into_iter().collect(),
        allowed_providers: BTreeSet::new(),
        denied_providers: BTreeSet::new(),
        required_capabilities: [InferenceCapability::Streaming].into_iter().collect(),
        minimum_privacy_mode: PrivacyMode::DirectSealed,
        minimum_assurance_maturity: AssuranceMaturity::Declared,
        required_assurance_properties: [
            AssuranceProperty::ProviderAuthorization,
            AssuranceProperty::ServiceManifestBinding,
        ]
        .into_iter()
        .collect(),
        minimum_success_probability: PartsPerMillion::new(750_000)?,
        max_p95_latency_ms: 6_000,
        protocol_fee: BasisPoints::new(400)?,
        protocol_fee_recipient: "account:hyperintel-fees".parse()?,
        prompt_digest: Sha256Digest::from_bytes([11; 32]),
        replay_snapshot_digest: Sha256Digest::from_bytes([12; 32]),
    };

    let mut assurance_sets = BTreeMap::new();
    let mut assurance_blobs = BTreeMap::new();
    let mut registry = VerificationKeyRegistry::default();
    let mut verification_keys = Vec::new();
    let mut provider_signing_material = BTreeMap::new();
    for (index, provider) in providers.values().enumerate() {
        let provider_digest = provider.digest()?;
        let key_byte = 50_u8
            .checked_add(u8::try_from(index)?)
            .ok_or("key overflow")?;
        let built = signed_evidence_set(
            key_byte,
            provider.provider_id.clone(),
            provider.offer_key_id.clone(),
            "provider-identity".parse()?,
            &AssuranceClaimV1 {
                subject_digest: provider_digest,
                property: AssuranceProperty::ProviderAuthorization,
            },
            AssuranceRootClassV1::ProviderKey,
            format!("provider authorization {index}").into_bytes(),
        )?;
        provider_signing_material.insert(
            provider.provider_id.clone(),
            (key_byte, built.0.envelope.digest()?),
        );
        registry.register(built.2.clone())?;
        verification_keys.push(built.2);
        assurance_blobs.insert(built.1.0, built.1.1);
        assurance_sets.insert(built.0.evidence_set_digest, built.0);
    }
    let service_evidence = signed_evidence_set(
        60,
        "verifier:service".parse()?,
        "key:service-1".parse()?,
        "service-spec".parse()?,
        &AssuranceClaimV1 {
            subject_digest: service_digest,
            property: AssuranceProperty::ServiceManifestBinding,
        },
        AssuranceRootClassV1::VerifierKey,
        b"service manifest binding".to_vec(),
    )?;
    registry.register(service_evidence.2.clone())?;
    verification_keys.push(service_evidence.2);
    assurance_blobs.insert(service_evidence.1.0, service_evidence.1.1);
    assurance_sets.insert(service_evidence.0.evidence_set_digest, service_evidence.0);

    let signed_offers = offers
        .iter()
        .enumerate()
        .map(|(index, offer)| {
            let (key_byte, envelope_digest) = provider_signing_material
                .get(&offer.provider_id)
                .copied()
                .ok_or("provider signing material missing")?;
            signed_capacity_offer(
                offer.clone(),
                key_byte,
                envelope_digest,
                u64::try_from(index)?
                    .checked_add(1)
                    .ok_or("sequence overflow")?,
            )
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

    Ok(Fixture {
        evaluated_at,
        service,
        providers,
        offers,
        signed_offers,
        policy,
        assurance_sets,
        assurance_blobs,
        registry,
        verification_keys,
    })
}

type BuiltEvidence = (
    AssuranceEvidenceSetV1,
    (Sha256Digest, Vec<u8>),
    RegisteredVerificationKeyV1,
);

fn signed_evidence_set(
    key_byte: u8,
    signer_id: ProtocolId,
    key_id: ProtocolId,
    subject_kind: ProtocolId,
    claim: &AssuranceClaimV1,
    root_class: AssuranceRootClassV1,
    evidence_bytes: Vec<u8>,
) -> Result<BuiltEvidence, Box<dyn std::error::Error>> {
    let evidence_ref = evidence_blob_ref_v1(
        &evidence_bytes,
        "application/json",
        EvidenceClassification::Public,
    )?;
    let envelope = AssuranceEnvelopeV1 {
        schema_version: "hyperintel.assurance-envelope.v1".to_owned(),
        guarantees: [claim.clone()].into_iter().collect(),
        assumptions: BTreeSet::new(),
        exclusions: BTreeSet::new(),
        maturity: match root_class {
            AssuranceRootClassV1::VerifierKey => AssuranceMaturity::LocallyChecked,
            AssuranceRootClassV1::ProviderKey | AssuranceRootClassV1::BuyerPolicy => {
                AssuranceMaturity::Declared
            }
            AssuranceRootClassV1::HardwareVendor => AssuranceMaturity::Attested,
        },
        trust_roots: [AssuranceRootV1 {
            class: root_class,
            root_id: key_id.clone(),
        }]
        .into_iter()
        .collect(),
        valid: ValidityWindowV1 {
            start: timestamp("2026-07-20T00:00:00Z")?,
            end: timestamp("2026-07-21T00:00:00Z")?,
        },
        source_digests: [evidence_ref.digest].into_iter().collect(),
    };
    let signing_key = SigningKey::from_bytes(&[key_byte; 32]);
    let registered = RegisteredVerificationKeyV1 {
        signer_id: signer_id.clone(),
        key_id: key_id.clone(),
        trust_root_class: root_class,
        public_key_hex: HEXLOWER.encode(signing_key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T00:00:00Z")?,
        valid_until: timestamp("2026-07-22T00:00:00Z")?,
        revoked_at: None,
    };
    let mut statement = SignedStatementV1 {
        schema_version: "hyperintel.signed-statement.v1".to_owned(),
        subject_kind,
        subject_digest: claim.subject_digest,
        assurance_envelope_digest: envelope.digest()?,
        issued_at: timestamp("2026-07-20T10:00:00Z")?,
        expires_at: timestamp("2026-07-21T00:00:00Z")?,
        sequence: 1,
        signer_id,
        key_id,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    let set = build_assurance_evidence_set_v1(
        envelope,
        [evidence_ref.clone()].into_iter().collect(),
        vec![statement],
        Sha256Digest::from_bytes([90; 32]),
    )?;
    Ok((set, (evidence_ref.digest, evidence_bytes), registered))
}

fn signed_capacity_offer(
    offer: CapacityOfferV1,
    key_byte: u8,
    provider_assurance_envelope_digest: Sha256Digest,
    sequence: u64,
) -> Result<SignedCapacityOfferV1, Box<dyn std::error::Error>> {
    let signing_key = SigningKey::from_bytes(&[key_byte; 32]);
    let mut statement = SignedStatementV1 {
        schema_version: "hyperintel.signed-statement.v1".to_owned(),
        subject_kind: "capacity-offer".parse()?,
        subject_digest: offer.digest()?,
        assurance_envelope_digest: provider_assurance_envelope_digest,
        issued_at: timestamp("2026-07-20T10:00:00Z")?,
        expires_at: timestamp("2026-07-20T23:59:59Z")?,
        sequence,
        signer_id: offer.provider_id.clone(),
        key_id: format!(
            "key:{}-offer",
            offer
                .provider_id
                .as_str()
                .strip_prefix("provider:")
                .ok_or("fixture provider prefix missing")?
        )
        .parse()?,
        algorithm: "ed25519".to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex =
        HEXLOWER.encode(&signing_key.sign(&statement.signing_message()?).to_bytes());
    Ok(SignedCapacityOfferV1 {
        schema_version: "hyperintel.signed-capacity-offer.v1".to_owned(),
        offer,
        statement,
    })
}

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}
