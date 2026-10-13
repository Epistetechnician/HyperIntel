//! Hermetic composition of existing validated, authorized, and replay-protected local seams.
//!
//! This crate adds no transport, ambient credential source, external provider,
//! rerouting, settlement submission, custody, deployment, or production authority.

use std::fmt;

use hyperintel_core::{
    CanonicalizationError, ProtocolId, Sha256Digest, UtcTimestamp, domain_separated_digest,
};
use hyperintel_execution::{
    ChatCompletionRequestV1, LocalProviderAgentV1, ProviderAgentErrorV1, ProviderExecutionPlanV1,
    ProviderReceiptSignerV1, chat_completion_request_digest_v1,
};
use hyperintel_execution_journal::{
    ExecutionJournalError, SqliteExecutionJournalV1,
    execute_journaled_provider_agent_with_activation_v1,
};
use hyperintel_market::AuthenticatedExecutionAdmissionV1;
use hyperintel_provider_agent::{ProviderAgentError, ProviderAgentExecutionContextV1};
use hyperintel_reconciliation::{
    ReceiptReconciliationError, ReceiptReconciliationInputV1, ReceiptReconciliationStatusV1,
    reconcile_receipts_v1,
};
use hyperintel_runtime_config::{RuntimeProfileV1, ValidatedRuntimeConfigurationV1};
use hyperintel_secret_provider::{
    InjectedSecretResolverV1, OperatorSecretAuthorizationVerificationContextV1, ResolvedSecretV1,
    SecretProviderError, SecretPurposeV1, SignedOperatorSecretAuthorizationV1,
    resolve_authorized_secret_v1, secret_reference_digest_v1,
};
use hyperintel_tenant_iam::{
    SignedTenantAccessGrantV1, TenantAccessVerificationContextV1, TenantIamActionV1,
    TenantIamError, VerifiedTenantAccessV1, verify_tenant_access_grant_v1,
};
use serde::Serialize;
use thiserror::Error;

/// Authorized repository state slice.
pub const RUNTIME_COMPOSITION_STATE_SLICE_V1: &str = "hyperintel-hermetic-runtime-composition-v1";
/// Exact opaque composition-report schema.
pub const RUNTIME_COMPOSITION_REPORT_SCHEMA_V1: &str =
    "hyperintel.hermetic-runtime-composition-report.v1";

const OPERATION_BINDING_DOMAIN_V1: &str = "hyperintel.hermetic-operation-binding.v1";
const COMPOSITION_REPORT_DOMAIN_V1: &str = "hyperintel.hermetic-runtime-composition-report.v1";

/// Exact identity-only binding for one composed local operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HermeticOperationBindingV1 {
    /// Validated runtime-configuration identity.
    pub configuration_digest: Sha256Digest,
    /// Exact local runtime instance.
    pub runtime_id: ProtocolId,
    /// Authorized tenant.
    pub tenant_id: ProtocolId,
    /// Authorized principal and authenticated buyer.
    pub principal_id: ProtocolId,
    /// Exact tenant-grant audience.
    pub audience: ProtocolId,
    /// Authenticated admission identity.
    pub admission_digest: Sha256Digest,
    /// Exact request identity.
    pub request_digest: Sha256Digest,
    /// Admission prompt commitment.
    pub prompt_digest: Sha256Digest,
    /// Selected route identity.
    pub route_decision_digest: Sha256Digest,
    /// Selected provider.
    pub provider_id: ProtocolId,
    /// Selected signed offer.
    pub offer_digest: Sha256Digest,
    /// Complete provider-agent operational binding.
    pub provider_agent_binding_digest: Sha256Digest,
    /// Exact provider-agent instance.
    pub agent_instance_id: ProtocolId,
    /// Configured receipt key.
    pub receipt_key_id: ProtocolId,
    /// Immutable secret-reference identity.
    pub secret_reference_digest: Sha256Digest,
    /// Closed secret use.
    pub secret_purpose: SecretPurposeV1,
    /// Explicit cancellation signal identity.
    pub cancellation_requested_at: Option<UtcTimestamp>,
    /// Exact evaluation and consumption time.
    pub evaluated_at: UtcTimestamp,
}

/// Compute the exact domain-separated operation-context identity used by IAM and secret trust.
///
/// # Errors
///
/// Returns when canonical identity construction fails.
pub fn hermetic_operation_context_digest_v1(
    binding: &HermeticOperationBindingV1,
) -> Result<Sha256Digest, CanonicalizationError> {
    domain_separated_digest(OPERATION_BINDING_DOMAIN_V1, binding)
}

/// Injected provider port that receives only a scoped borrow of synthetic secret bytes.
pub trait SecretUsingProviderV1 {
    /// Provider identity implemented by this injected port.
    fn provider_id(&self) -> &ProtocolId;

    /// Execute once with a scoped borrow of the authorization-bound secret.
    ///
    /// # Errors
    ///
    /// Returns only the existing closed provider-port failure.
    fn execute_with_secret(
        &mut self,
        secret: &[u8],
        request: &ChatCompletionRequestV1,
        admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionPlanV1, ProviderAgentErrorV1>;
}

struct SecretBoundProviderV1<'a, P> {
    inner: &'a mut P,
    secret: Option<ResolvedSecretV1>,
}

#[derive(Clone, Copy)]
struct SecretVerificationLineageV1 {
    authorization: Sha256Digest,
    signing_key: Sha256Digest,
    revocation_state: Sha256Digest,
    revocation_checkpoint: Sha256Digest,
}

impl From<&ResolvedSecretV1> for SecretVerificationLineageV1 {
    fn from(secret: &ResolvedSecretV1) -> Self {
        Self {
            authorization: secret.authorization_digest(),
            signing_key: secret.signing_key_record_digest(),
            revocation_state: secret.revocation_state_digest(),
            revocation_checkpoint: secret.revocation_checkpoint_digest(),
        }
    }
}

impl<P: SecretUsingProviderV1> LocalProviderAgentV1 for SecretBoundProviderV1<'_, P> {
    fn provider_id(&self) -> &ProtocolId {
        self.inner.provider_id()
    }

    fn execute(
        &mut self,
        request: &ChatCompletionRequestV1,
        admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionPlanV1, ProviderAgentErrorV1> {
        let secret = self
            .secret
            .take()
            .ok_or(ProviderAgentErrorV1::Unavailable)?;
        self.inner
            .execute_with_secret(secret.expose_secret_bytes(), request, admission)
    }
}

/// Borrowed inputs for exactly one hermetic composed attempt.
pub struct HermeticRuntimeCompositionContextV1<'a> {
    /// Validated local-hermetic runtime configuration.
    pub runtime_configuration: &'a ValidatedRuntimeConfigurationV1,
    /// Exact signed tenant access grant.
    pub tenant_access_grant: &'a SignedTenantAccessGrantV1,
    /// Current tenant policy, trust, revocation, expectation, and time.
    pub tenant_access_context: TenantAccessVerificationContextV1<'a>,
    /// Exact signed operator-secret authorization.
    pub secret_authorization: &'a SignedOperatorSecretAuthorizationV1,
    /// Current operator-secret trust, revocation, expectation, and time.
    pub secret_authorization_context: OperatorSecretAuthorizationVerificationContextV1<'a>,
    /// Current provider-agent signed control and existing harness inputs.
    pub provider_agent_context: ProviderAgentExecutionContextV1<'a>,
    /// Metadata-only execution journal.
    pub journal: &'a SqliteExecutionJournalV1,
}

/// Opaque identity-only positive composition report.
pub struct HermeticRuntimeCompositionReportV1 {
    schema_version: &'static str,
    state_slice: &'static str,
    configuration_digest: Sha256Digest,
    operation_context_digest: Sha256Digest,
    request_digest: Sha256Digest,
    prompt_digest: Sha256Digest,
    admission_digest: Sha256Digest,
    route_decision_digest: Sha256Digest,
    selected_offer_digest: Sha256Digest,
    provider_agent_binding_digest: Sha256Digest,
    secret_reference_digest: Sha256Digest,
    tenant_grant_digest: Sha256Digest,
    tenant_iam_snapshot_digest: Sha256Digest,
    tenant_revocation_snapshot_digest: Sha256Digest,
    secret_authorization_digest: Sha256Digest,
    secret_signing_key_digest: Sha256Digest,
    secret_revocation_state_digest: Sha256Digest,
    secret_revocation_checkpoint_digest: Sha256Digest,
    provider_manifest_digest: Sha256Digest,
    provider_readiness_digest: Sha256Digest,
    provider_revocation_snapshot_digest: Sha256Digest,
    provider_manifest_key_digest: Sha256Digest,
    provider_readiness_key_digest: Sha256Digest,
    provider_revocation_key_digest: Sha256Digest,
    provider_checkpoint_digest: Sha256Digest,
    replay_consumption_digest: Sha256Digest,
    execution_outcome_digest: Sha256Digest,
    reconciliation_status: ReceiptReconciliationStatusV1,
    journal_start_digest: Sha256Digest,
    journal_terminal_digest: Sha256Digest,
    reconciliation_report_digest: Sha256Digest,
    report_digest: Sha256Digest,
}

impl fmt::Debug for HermeticRuntimeCompositionReportV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HermeticRuntimeCompositionReportV1")
            .field("identities", &"[REDACTED]")
            .field("authority_granted", &false)
            .finish_non_exhaustive()
    }
}

impl HermeticRuntimeCompositionReportV1 {
    /// Exact report schema.
    #[must_use]
    pub const fn schema_version(&self) -> &'static str {
        self.schema_version
    }
    /// Authorized state slice that produced this report.
    #[must_use]
    pub const fn state_slice(&self) -> &'static str {
        self.state_slice
    }
    /// Validated runtime-configuration identity.
    #[must_use]
    pub const fn configuration_digest(&self) -> Sha256Digest {
        self.configuration_digest
    }
    /// Exact composed operation identity.
    #[must_use]
    pub const fn operation_context_digest(&self) -> Sha256Digest {
        self.operation_context_digest
    }
    /// Exact request identity.
    #[must_use]
    pub const fn request_digest(&self) -> Sha256Digest {
        self.request_digest
    }
    /// Admission prompt commitment.
    #[must_use]
    pub const fn prompt_digest(&self) -> Sha256Digest {
        self.prompt_digest
    }
    /// Authenticated admission identity.
    #[must_use]
    pub const fn admission_digest(&self) -> Sha256Digest {
        self.admission_digest
    }
    /// Reconstructed selected-route identity.
    #[must_use]
    pub const fn route_decision_digest(&self) -> Sha256Digest {
        self.route_decision_digest
    }
    /// Selected signed-offer identity.
    #[must_use]
    pub const fn selected_offer_digest(&self) -> Sha256Digest {
        self.selected_offer_digest
    }
    /// Complete provider-agent operational-binding identity.
    #[must_use]
    pub const fn provider_agent_binding_digest(&self) -> Sha256Digest {
        self.provider_agent_binding_digest
    }
    /// Immutable secret-reference identity.
    #[must_use]
    pub const fn secret_reference_digest(&self) -> Sha256Digest {
        self.secret_reference_digest
    }
    /// Complete signed tenant-grant identity.
    #[must_use]
    pub const fn tenant_grant_digest(&self) -> Sha256Digest {
        self.tenant_grant_digest
    }
    /// Caller-owned tenant-IAM snapshot identity.
    #[must_use]
    pub const fn tenant_iam_snapshot_digest(&self) -> Sha256Digest {
        self.tenant_iam_snapshot_digest
    }
    /// Authenticated tenant-revocation snapshot identity.
    #[must_use]
    pub const fn tenant_revocation_snapshot_digest(&self) -> Sha256Digest {
        self.tenant_revocation_snapshot_digest
    }
    /// Complete signed secret-authorization identity.
    #[must_use]
    pub const fn secret_authorization_digest(&self) -> Sha256Digest {
        self.secret_authorization_digest
    }
    /// Trusted operator-secret signing-key record identity.
    #[must_use]
    pub const fn secret_signing_key_digest(&self) -> Sha256Digest {
        self.secret_signing_key_digest
    }
    /// Caller-owned operator-secret revocation-state identity.
    #[must_use]
    pub const fn secret_revocation_state_digest(&self) -> Sha256Digest {
        self.secret_revocation_state_digest
    }
    /// Caller-pinned operator-secret checkpoint identity.
    #[must_use]
    pub const fn secret_revocation_checkpoint_digest(&self) -> Sha256Digest {
        self.secret_revocation_checkpoint_digest
    }
    /// Signed provider-agent manifest identity.
    #[must_use]
    pub const fn provider_agent_manifest_digest(&self) -> Sha256Digest {
        self.provider_manifest_digest
    }
    /// Signed provider-agent readiness identity.
    #[must_use]
    pub const fn provider_agent_readiness_digest(&self) -> Sha256Digest {
        self.provider_readiness_digest
    }
    /// Signed provider-agent revocation snapshot identity.
    #[must_use]
    pub const fn provider_agent_revocation_snapshot_digest(&self) -> Sha256Digest {
        self.provider_revocation_snapshot_digest
    }
    /// Trusted provider-agent manifest-key record identity.
    #[must_use]
    pub const fn provider_agent_manifest_key_digest(&self) -> Sha256Digest {
        self.provider_manifest_key_digest
    }
    /// Trusted provider-agent readiness-key record identity.
    #[must_use]
    pub const fn provider_agent_readiness_key_digest(&self) -> Sha256Digest {
        self.provider_readiness_key_digest
    }
    /// Trusted provider-agent revocation-key record identity.
    #[must_use]
    pub const fn provider_agent_revocation_key_digest(&self) -> Sha256Digest {
        self.provider_revocation_key_digest
    }
    /// Caller-pinned provider-agent checkpoint identity.
    #[must_use]
    pub const fn provider_agent_checkpoint_digest(&self) -> Sha256Digest {
        self.provider_checkpoint_digest
    }
    /// Exact replay-consumption identity.
    #[must_use]
    pub const fn replay_consumption_digest(&self) -> Sha256Digest {
        self.replay_consumption_digest
    }
    /// Completed or incomplete local execution identity.
    #[must_use]
    pub const fn execution_outcome_digest(&self) -> Sha256Digest {
        self.execution_outcome_digest
    }
    /// Closed completed or incomplete reconciliation status.
    #[must_use]
    pub const fn reconciliation_status(&self) -> ReceiptReconciliationStatusV1 {
        self.reconciliation_status
    }
    /// Metadata-only journal-start identity.
    #[must_use]
    pub const fn journal_start_digest(&self) -> Sha256Digest {
        self.journal_start_digest
    }
    /// Metadata-only journal-terminal identity.
    #[must_use]
    pub const fn journal_terminal_digest(&self) -> Sha256Digest {
        self.journal_terminal_digest
    }
    /// Positive reconciliation-report identity.
    #[must_use]
    pub const fn reconciliation_report_digest(&self) -> Sha256Digest {
        self.reconciliation_report_digest
    }
    /// Complete identity-only composition-report digest.
    #[must_use]
    pub const fn report_digest(&self) -> Sha256Digest {
        self.report_digest
    }
    /// Recompute every retained identity and verify the report commitment.
    ///
    /// # Errors
    ///
    /// Returns when canonical identity construction fails.
    pub fn verify_digest(&self) -> Result<bool, CanonicalizationError> {
        Ok(domain_separated_digest(
            COMPOSITION_REPORT_DOMAIN_V1,
            &CompositionReportIdentityV1::from(self),
        )? == self.report_digest)
    }
    /// Local composition evidence grants no external authority.
    #[must_use]
    pub const fn authority_granted(&self) -> bool {
        false
    }
}

/// Closed composition failure.
#[derive(Debug, Error)]
pub enum HermeticRuntimeCompositionErrorV1 {
    /// Runtime, tenant, operation, route, secret, or provider identity disagreed.
    #[error("hermetic runtime composition identity mismatch")]
    IdentityMismatch,
    /// Tenant authorization failed.
    #[error("hermetic runtime composition tenant authorization failed: {0}")]
    Tenant(#[from] TenantIamError),
    /// Operator-secret authorization or resolution failed.
    #[error("hermetic runtime composition secret activation failed: {0}")]
    Secret(#[from] SecretProviderError),
    /// Provider-agent control, preflight, activation, or harness failed.
    #[error("hermetic runtime composition provider-agent failed: {0}")]
    ProviderAgent(#[from] ProviderAgentError),
    /// Journal or replay lineage failed.
    #[error("hermetic runtime composition journal failed: {0}")]
    Journal(#[from] ExecutionJournalError),
    /// Terminal evidence reconciliation failed.
    #[error("hermetic runtime composition reconciliation failed: {0}")]
    Reconciliation(#[from] ReceiptReconciliationError),
    /// Canonical identity construction failed.
    #[error("hermetic runtime composition canonicalization failed: {0}")]
    Canonicalization(#[from] CanonicalizationError),
}

/// Execute one exact hermetic composition with no retry, fallback, or resumption.
///
/// # Errors
///
/// Returns on any identity, authorization, activation, replay, journal, execution, or
/// reconciliation failure. No positive report is returned for a failed composition.
pub async fn execute_hermetic_runtime_composition_v1<P, S, R>(
    context: HermeticRuntimeCompositionContextV1<'_>,
    provider: &mut P,
    signer: &S,
    resolver: &mut R,
) -> Result<HermeticRuntimeCompositionReportV1, HermeticRuntimeCompositionErrorV1>
where
    P: SecretUsingProviderV1,
    S: ProviderReceiptSignerV1,
    R: InjectedSecretResolverV1,
{
    let access =
        verify_tenant_access_grant_v1(context.tenant_access_grant, &context.tenant_access_context)?;
    let (operation_context_digest, operation_binding) =
        validate_operation_binding(&context, &access)?;
    let configuration_digest = context.runtime_configuration.configuration_digest();
    let secret_authorization_digest = context.secret_authorization.digest()?;
    let admission = context.provider_agent_context.local_execution.admission;
    let authenticated = context.provider_agent_context.local_execution.authenticated;
    let verification_keys = context
        .provider_agent_context
        .local_execution
        .verification_keys;
    let mut activation_error = None;
    let mut secret_lineage = None;
    let mut secret_bound_provider = SecretBoundProviderV1 {
        inner: provider,
        secret: None,
    };
    let attempt = execute_journaled_provider_agent_with_activation_v1(
        context.journal,
        context.provider_agent_context,
        &mut secret_bound_provider,
        signer,
        |provider| {
            let secret = match resolve_authorized_secret_v1(
                resolver,
                context.secret_authorization,
                &context.secret_authorization_context,
            ) {
                Ok(value) => value,
                Err(error) => {
                    activation_error = Some(HermeticRuntimeCompositionErrorV1::Secret(error));
                    return Err(ProviderAgentError::ActivationDenied);
                }
            };
            if secret.operation_context_digest() != operation_context_digest {
                activation_error = Some(HermeticRuntimeCompositionErrorV1::IdentityMismatch);
                return Err(ProviderAgentError::ActivationDenied);
            }
            let lineage = SecretVerificationLineageV1::from(&secret);
            if lineage.authorization != secret_authorization_digest {
                activation_error = Some(HermeticRuntimeCompositionErrorV1::IdentityMismatch);
                return Err(ProviderAgentError::ActivationDenied);
            }
            secret_lineage = Some(lineage);
            provider.secret = Some(secret);
            Ok(())
        },
    )
    .await;
    let journaled = match attempt {
        Ok(value) => value,
        Err(ExecutionJournalError::ProviderAgent(ProviderAgentError::ActivationDenied)) => {
            return Err(activation_error.unwrap_or_else(|| {
                HermeticRuntimeCompositionErrorV1::ProviderAgent(
                    ProviderAgentError::ActivationDenied,
                )
            }));
        }
        Err(error) => return Err(error.into()),
    };
    let secret_lineage =
        secret_lineage.ok_or(HermeticRuntimeCompositionErrorV1::IdentityMismatch)?;
    let reconciliation = reconcile_receipts_v1(&ReceiptReconciliationInputV1 {
        admission,
        authenticated,
        verification_keys,
        journal_start: &journaled.start,
        journal_terminal: &journaled.terminal,
        execution: journaled.execution.outcome(),
    })?;
    build_report(&ReportInputsV1 {
        configuration: configuration_digest,
        operation_binding: &operation_binding,
        tenant_grant: access.grant_digest(),
        tenant_iam_snapshot: access.iam_snapshot_digest(),
        tenant_revocation_snapshot: access.revocation_snapshot_digest(),
        operation: operation_context_digest,
        secret: secret_lineage,
        manifest: journaled.execution.manifest_digest(),
        readiness: journaled.execution.readiness_digest(),
        provider_revocation: journaled.execution.revocation_snapshot_digest(),
        manifest_key: journaled.execution.manifest_key_digest(),
        readiness_key: journaled.execution.readiness_key_digest(),
        revocation_key: journaled.execution.revocation_key_digest(),
        provider_checkpoint: journaled.execution.checkpoint_digest(),
        replay_consumption: reconciliation.replay_consumption_digest,
        execution_outcome: reconciliation.execution_outcome_digest,
        reconciliation_status: reconciliation.status,
        journal_start: journaled.start.start_digest,
        journal_terminal: journaled.terminal.terminal_digest,
        reconciliation: reconciliation.report_digest,
    })
}

fn validate_operation_binding(
    context: &HermeticRuntimeCompositionContextV1<'_>,
    access: &VerifiedTenantAccessV1,
) -> Result<(Sha256Digest, HermeticOperationBindingV1), HermeticRuntimeCompositionErrorV1> {
    let local = &context.provider_agent_context.local_execution;
    let admission = local.admission;
    let expected = context.provider_agent_context.expected_binding;
    let secret_expectation = context.secret_authorization_context.expectation;
    let configuration_digest = context.runtime_configuration.configuration_digest();
    let request_digest = chat_completion_request_digest_v1(local.request)?;
    let reference_digest = secret_reference_digest_v1(secret_expectation.secret_reference)?;
    let binding = HermeticOperationBindingV1 {
        configuration_digest,
        runtime_id: context.runtime_configuration.document().runtime_id.clone(),
        tenant_id: access.tenant_id().clone(),
        principal_id: access.principal_id().clone(),
        audience: access.audience().clone(),
        admission_digest: admission.admission_digest,
        request_digest,
        prompt_digest: admission.prompt_digest,
        route_decision_digest: admission.route_decision_digest,
        provider_id: admission.provider_id.clone(),
        offer_digest: admission.offer_digest,
        provider_agent_binding_digest: expected.digest()?,
        agent_instance_id: expected.agent_instance_id.clone(),
        receipt_key_id: expected.receipt_key_id.clone(),
        secret_reference_digest: reference_digest,
        secret_purpose: SecretPurposeV1::ProviderApiAuthentication,
        cancellation_requested_at: local.cancellation.requested_at.clone(),
        evaluated_at: context.provider_agent_context.evaluated_at.clone(),
    };
    let operation_digest = hermetic_operation_context_digest_v1(&binding)?;
    if context.runtime_configuration.document().profile != RuntimeProfileV1::LocalHermetic
        || access.action() != TenantIamActionV1::SubmitInference
        || access.principal_id() != &admission.buyer_id
        || access.runtime_configuration_digest() != configuration_digest
        || access.resource_digest() != admission.admission_digest
        || access.operation_context_digest() != operation_digest
        || access.evaluated_at() != local.consumed_at
        || context.tenant_access_context.evaluated_at != local.consumed_at
        || context.secret_authorization_context.evaluated_at != local.consumed_at
        || expected.runtime_configuration_digest != configuration_digest
        || secret_expectation.runtime_configuration_digest != configuration_digest
        || secret_expectation.consumer_id != &expected.agent_instance_id
        || secret_expectation.selected_provider_id != &admission.provider_id
        || secret_expectation.purpose != SecretPurposeV1::ProviderApiAuthentication
        || secret_expectation.operation_context_digest != operation_digest
        || reference_digest != expected.secret_reference_digest
        || request_digest != admission.prompt_digest
        || expected.selected_offer_digest != admission.offer_digest
        || expected.provider_id != admission.provider_id
    {
        return Err(HermeticRuntimeCompositionErrorV1::IdentityMismatch);
    }
    Ok((operation_digest, binding))
}

struct ReportInputsV1<'a> {
    configuration: Sha256Digest,
    operation_binding: &'a HermeticOperationBindingV1,
    tenant_grant: Sha256Digest,
    tenant_iam_snapshot: Sha256Digest,
    tenant_revocation_snapshot: Sha256Digest,
    operation: Sha256Digest,
    secret: SecretVerificationLineageV1,
    manifest: Sha256Digest,
    readiness: Sha256Digest,
    provider_revocation: Sha256Digest,
    manifest_key: Sha256Digest,
    readiness_key: Sha256Digest,
    revocation_key: Sha256Digest,
    provider_checkpoint: Sha256Digest,
    replay_consumption: Sha256Digest,
    execution_outcome: Sha256Digest,
    reconciliation_status: ReceiptReconciliationStatusV1,
    journal_start: Sha256Digest,
    journal_terminal: Sha256Digest,
    reconciliation: Sha256Digest,
}

fn build_report(
    input: &ReportInputsV1,
) -> Result<HermeticRuntimeCompositionReportV1, HermeticRuntimeCompositionErrorV1> {
    let mut report = HermeticRuntimeCompositionReportV1 {
        schema_version: RUNTIME_COMPOSITION_REPORT_SCHEMA_V1,
        state_slice: RUNTIME_COMPOSITION_STATE_SLICE_V1,
        configuration_digest: input.configuration,
        operation_context_digest: input.operation,
        request_digest: input.operation_binding.request_digest,
        prompt_digest: input.operation_binding.prompt_digest,
        admission_digest: input.operation_binding.admission_digest,
        route_decision_digest: input.operation_binding.route_decision_digest,
        selected_offer_digest: input.operation_binding.offer_digest,
        provider_agent_binding_digest: input.operation_binding.provider_agent_binding_digest,
        secret_reference_digest: input.operation_binding.secret_reference_digest,
        tenant_grant_digest: input.tenant_grant,
        tenant_iam_snapshot_digest: input.tenant_iam_snapshot,
        tenant_revocation_snapshot_digest: input.tenant_revocation_snapshot,
        secret_authorization_digest: input.secret.authorization,
        secret_signing_key_digest: input.secret.signing_key,
        secret_revocation_state_digest: input.secret.revocation_state,
        secret_revocation_checkpoint_digest: input.secret.revocation_checkpoint,
        provider_manifest_digest: input.manifest,
        provider_readiness_digest: input.readiness,
        provider_revocation_snapshot_digest: input.provider_revocation,
        provider_manifest_key_digest: input.manifest_key,
        provider_readiness_key_digest: input.readiness_key,
        provider_revocation_key_digest: input.revocation_key,
        provider_checkpoint_digest: input.provider_checkpoint,
        replay_consumption_digest: input.replay_consumption,
        execution_outcome_digest: input.execution_outcome,
        reconciliation_status: input.reconciliation_status,
        journal_start_digest: input.journal_start,
        journal_terminal_digest: input.journal_terminal,
        reconciliation_report_digest: input.reconciliation,
        report_digest: Sha256Digest::from_bytes([0; 32]),
    };
    report.report_digest = domain_separated_digest(
        COMPOSITION_REPORT_DOMAIN_V1,
        &CompositionReportIdentityV1::from(&report),
    )?;
    Ok(report)
}

#[derive(Serialize)]
struct CompositionReportIdentityV1 {
    schema_version: &'static str,
    state_slice: &'static str,
    configuration: Sha256Digest,
    operation: Sha256Digest,
    request: Sha256Digest,
    prompt: Sha256Digest,
    admission: Sha256Digest,
    route_decision: Sha256Digest,
    selected_offer: Sha256Digest,
    provider_agent_binding: Sha256Digest,
    secret_reference: Sha256Digest,
    tenant_grant: Sha256Digest,
    tenant_iam_snapshot: Sha256Digest,
    tenant_revocation_snapshot: Sha256Digest,
    secret_authorization: Sha256Digest,
    secret_signing_key: Sha256Digest,
    secret_revocation_state: Sha256Digest,
    secret_revocation_checkpoint: Sha256Digest,
    manifest: Sha256Digest,
    readiness: Sha256Digest,
    provider_revocation: Sha256Digest,
    manifest_key: Sha256Digest,
    readiness_key: Sha256Digest,
    revocation_key: Sha256Digest,
    provider_checkpoint: Sha256Digest,
    replay_consumption: Sha256Digest,
    execution_outcome: Sha256Digest,
    reconciliation_status: ReceiptReconciliationStatusV1,
    journal_start: Sha256Digest,
    journal_terminal: Sha256Digest,
    reconciliation: Sha256Digest,
    authority_granted: bool,
}

impl From<&HermeticRuntimeCompositionReportV1> for CompositionReportIdentityV1 {
    fn from(value: &HermeticRuntimeCompositionReportV1) -> Self {
        Self {
            schema_version: value.schema_version,
            state_slice: value.state_slice,
            configuration: value.configuration_digest,
            operation: value.operation_context_digest,
            request: value.request_digest,
            prompt: value.prompt_digest,
            admission: value.admission_digest,
            route_decision: value.route_decision_digest,
            selected_offer: value.selected_offer_digest,
            provider_agent_binding: value.provider_agent_binding_digest,
            secret_reference: value.secret_reference_digest,
            tenant_grant: value.tenant_grant_digest,
            tenant_iam_snapshot: value.tenant_iam_snapshot_digest,
            tenant_revocation_snapshot: value.tenant_revocation_snapshot_digest,
            secret_authorization: value.secret_authorization_digest,
            secret_signing_key: value.secret_signing_key_digest,
            secret_revocation_state: value.secret_revocation_state_digest,
            secret_revocation_checkpoint: value.secret_revocation_checkpoint_digest,
            manifest: value.provider_manifest_digest,
            readiness: value.provider_readiness_digest,
            provider_revocation: value.provider_revocation_snapshot_digest,
            manifest_key: value.provider_manifest_key_digest,
            readiness_key: value.provider_readiness_key_digest,
            revocation_key: value.provider_revocation_key_digest,
            provider_checkpoint: value.provider_checkpoint_digest,
            replay_consumption: value.replay_consumption_digest,
            execution_outcome: value.execution_outcome_digest,
            reconciliation_status: value.reconciliation_status,
            journal_start: value.journal_start_digest,
            journal_terminal: value.journal_terminal_digest,
            reconciliation: value.reconciliation_report_digest,
            authority_granted: false,
        }
    }
}
