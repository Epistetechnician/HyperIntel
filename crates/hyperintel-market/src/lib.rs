//! Pure-data inference eligibility, routing, metering, settlement, and evidence.
//!
//! Every derived record carries `authority_granted = false`. This crate decides
//! what a future operator should do; it cannot execute inference or settlement.

mod evidence;
mod metering;
mod offer_admission;
mod reliability;
mod routing;
mod settlement;

pub use evidence::{
    MarketEvidenceBundleV1, MarketEvidenceError, ReliabilityArtifactVerifiedMarketEvidenceBundleV3,
    SignedMarketEvidenceBundleV2, build_market_evidence_bundle_v1,
    build_reliability_artifact_verified_market_evidence_bundle_v3,
    build_signed_market_evidence_bundle_v2, verify_market_evidence_bundle_v1,
    verify_reliability_artifact_verified_market_evidence_bundle_v3,
    verify_signed_market_evidence_bundle_v2,
};
pub use metering::{
    ExecutionStatusV1, MeteringError, MeteringReceiptDraftV1, MeteringReceiptV1,
    build_metering_receipt_v1, validate_metering_receipt_v1,
};
pub use offer_admission::{
    OfferAdmissionError, SignedCapacityOfferV1, verify_signed_capacity_offer_v1,
};
pub use reliability::{
    ReliabilityArtifactDraftV1, ReliabilityArtifactError, ReliabilityArtifactV1,
    ReliabilityProbabilityMethodV1, ReliabilityVerifierStatementV1, SignedReliabilityArtifactV1,
    build_reliability_artifact_v1, derive_observed_success_probability_v1,
    verify_offer_reliability_v1, verify_signed_reliability_artifact_v1,
};
pub use routing::{
    CostQuoteV1, EligibilityFailureV1, EligibilityReportV1, OfferRouteRecordV1,
    ReliabilityArtifactVerifiedRoutingContextV1, RouteDecisionV1, RoutingContextV1, RoutingError,
    SignedRoutingContextV1, select_reliability_artifact_verified_route_v1, select_route_v1,
    select_signed_route_v1,
};
pub use settlement::{
    SettlementError, SettlementIntentV1, derive_settlement_intent_v1, validate_settlement_intent_v1,
};

/// State slice emitted by every v1 market-derived record.
pub const MARKET_STATE_SLICE_V1: &str = "hyperintel-route-receipt-vertical-slice-v1";
