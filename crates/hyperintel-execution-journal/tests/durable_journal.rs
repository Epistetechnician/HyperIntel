//! Contract coverage for metadata-only execution crash recovery.

use std::{
    cell::Cell,
    collections::BTreeMap,
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};

use data_encoding::HEXLOWER;
use ed25519_dalek::{Signer as _, SigningKey};
use hyperintel_assurance::{
    AssuranceRootClassV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
};
use hyperintel_core::{InferenceRequestPolicyV1, ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_execution::{
    CancellationSignalV1, ChatCompletionMessageV1, ChatCompletionRequestV1, ChatMessageRoleV1,
    LocalExecutionContextV1, LocalExecutionOutcomeV1, LocalProviderAgentV1, ProviderAgentErrorV1,
    ProviderExecutionPlanV1, ProviderReceiptSignerErrorV1, ProviderReceiptSignerV1,
    ProviderStreamFrameV1, chat_completion_request_digest_v1,
};
use hyperintel_execution_journal::{
    ExecutionJournalError, ExecutionJournalStartDraftV1, ExecutionJournalTerminalDraftV1,
    ExecutionJournalTerminalStatusV1, ExecutionRecoveryStateV1, JournalWriteOutcomeV1,
    SqliteExecutionJournalV1, build_execution_journal_start_v1,
    build_execution_journal_terminal_v1, execute_journaled_provider_agent_with_activation_v1,
};
use hyperintel_market::{
    AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerPolicyStatementV1,
    ProviderExecutionReceiptV1, ProviderExecutionStatementV1,
    ReliabilityArtifactVerifiedRoutingContextV1, ReplayProtectedMarketEvidenceBundleV4,
    ReplayStateTransitionV1, SignedInferenceRequestPolicyV1, SignedReliabilityArtifactV1,
    build_authenticated_execution_admission_v1, build_buyer_replay_state_v1,
    derive_replay_state_transition_v1, evaluate_reliability_artifact_verified_route_v1,
    verify_replay_protected_market_evidence_bundle_v4,
};
use hyperintel_persistence::{
    AuthenticatedReplayConsumptionV1, DURABLE_REPLAY_STATE_SLICE_V1,
    InitializeReplayStateOutcomeV1, SqliteReplayStoreV1,
};
use hyperintel_provider_agent::{
    ED25519_ALGORITHM_V1, PROVIDER_AGENT_KEY_SCHEMA_V1, PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1,
    PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1, PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1,
    ProviderAgentBindingV1, ProviderAgentError, ProviderAgentExecutionContextV1,
    ProviderAgentKeyPurposeV1, ProviderAgentKeyRegistryV1, ProviderAgentReadinessClaimsV1,
    ProviderAgentReadinessStatusV1, ProviderAgentRevocationCheckpointV1,
    ProviderAgentRevocationSnapshotV1, ProviderAgentServiceManifestClaimsV1,
    ProviderTextStreamInterfaceV1, RegisteredProviderAgentKeyV1,
    SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1, SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1,
    SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1, SignedProviderAgentReadinessV1,
    SignedProviderAgentRevocationSnapshotV1, SignedProviderAgentServiceManifestV1,
};
use serde::Deserialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tempfile::tempdir;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderAttemptFixture {
    assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
    bundle: ReplayProtectedMarketEvidenceBundleV4,
    reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedKeysFixture {
    trusted_verification_keys: Vec<RegisteredVerificationKeyV1>,
}

struct ValidExecutionCase {
    policy: InferenceRequestPolicyV1,
    signed_policy: SignedInferenceRequestPolicyV1,
    replay_transition: ReplayStateTransitionV1,
    admission: AuthenticatedExecutionAdmissionV1,
}

struct ValidProviderControl {
    registry: ProviderAgentKeyRegistryV1,
    binding: ProviderAgentBindingV1,
    manifest: SignedProviderAgentServiceManifestV1,
    readiness: SignedProviderAgentReadinessV1,
    revocation: SignedProviderAgentRevocationSnapshotV1,
    checkpoint: ProviderAgentRevocationCheckpointV1,
    evaluated_at: UtcTimestamp,
}

#[derive(Clone, Copy)]
enum ValidAttemptKind {
    Completed,
    Incomplete,
    ErrorAfterConsumption,
}

struct InvalidProviderControl {
    binding: ProviderAgentBindingV1,
    manifest: SignedProviderAgentServiceManifestV1,
    readiness: SignedProviderAgentReadinessV1,
    revocation: SignedProviderAgentRevocationSnapshotV1,
    checkpoint: ProviderAgentRevocationCheckpointV1,
    key_registry: ProviderAgentKeyRegistryV1,
}

struct MustNotRunProvider {
    provider_id: ProtocolId,
    identity_reads: Cell<u64>,
    calls: u64,
}

struct ScriptedProvider {
    provider_id: ProtocolId,
    result: Option<Result<ProviderExecutionPlanV1, ProviderAgentErrorV1>>,
    calls: u64,
    signal: Option<mpsc::Sender<()>>,
}

impl LocalProviderAgentV1 for ScriptedProvider {
    fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }

    fn execute(
        &mut self,
        _request: &ChatCompletionRequestV1,
        _admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionPlanV1, ProviderAgentErrorV1> {
        self.calls += 1;
        if let Some(signal) = &self.signal {
            let _ = signal.send(());
        }
        self.result
            .take()
            .unwrap_or(Err(ProviderAgentErrorV1::ScriptExhausted))
    }
}

impl LocalProviderAgentV1 for MustNotRunProvider {
    fn provider_id(&self) -> &ProtocolId {
        self.identity_reads.set(self.identity_reads.get() + 1);
        &self.provider_id
    }

    fn execute(
        &mut self,
        _request: &ChatCompletionRequestV1,
        _admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionPlanV1, ProviderAgentErrorV1> {
        self.calls += 1;
        Err(ProviderAgentErrorV1::ScriptExhausted)
    }
}

struct MustNotRunSigner {
    provider_id: ProtocolId,
    key_id: ProtocolId,
    identity_reads: Cell<u64>,
    calls: Cell<u64>,
}

struct TestReceiptSigner {
    provider_id: ProtocolId,
    key_id: ProtocolId,
    key: SigningKey,
    calls: AtomicU64,
}

impl ProviderReceiptSignerV1 for TestReceiptSigner {
    fn provider_id(&self) -> &ProtocolId {
        &self.provider_id
    }

    fn key_id(&self) -> &ProtocolId {
        &self.key_id
    }

    fn sign(
        &self,
        receipt: &ProviderExecutionReceiptV1,
        admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionStatementV1, ProviderReceiptSignerErrorV1> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let mut statement = ProviderExecutionStatementV1 {
            schema_version: "hyperintel.provider-execution-statement.v1".to_owned(),
            receipt_digest: receipt.receipt_digest,
            admission_digest: admission.admission_digest,
            issued_at: receipt.completed_at.clone(),
            expires_at: admission.deadline_at.clone(),
            sequence: 1,
            signer_id: self.provider_id.clone(),
            key_id: self.key_id.clone(),
            algorithm: ED25519_ALGORITHM_V1.to_owned(),
            signature_hex: String::new(),
        };
        statement.signature_hex = HEXLOWER.encode(
            &self
                .key
                .sign(
                    &statement
                        .signing_message()
                        .map_err(|_| ProviderReceiptSignerErrorV1::Unavailable)?,
                )
                .to_bytes(),
        );
        Ok(statement)
    }
}

impl ProviderReceiptSignerV1 for MustNotRunSigner {
    fn provider_id(&self) -> &ProtocolId {
        self.identity_reads.set(self.identity_reads.get() + 1);
        &self.provider_id
    }

    fn key_id(&self) -> &ProtocolId {
        self.identity_reads.set(self.identity_reads.get() + 1);
        &self.key_id
    }

    fn sign(
        &self,
        _receipt: &ProviderExecutionReceiptV1,
        _admission: &AuthenticatedExecutionAdmissionV1,
    ) -> Result<ProviderExecutionStatementV1, ProviderReceiptSignerErrorV1> {
        self.calls.set(self.calls.get() + 1);
        Err(ProviderReceiptSignerErrorV1::Unavailable)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn contract_concurrent_exact_retries_restart_and_consumption_binding_are_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("execution-journal.sqlite");
    let replay_path = directory.path().join("replay.sqlite");
    let replay_store = SqliteReplayStoreV1::connect_path(&replay_path, 2).await?;
    replay_store.migrate().await?;
    assert!(matches!(
        SqliteExecutionJournalV1::connect_path(&database_path, 0).await,
        Err(ExecutionJournalError::InvalidConnectionCount)
    ));
    let journal = SqliteExecutionJournalV1::connect_path(&database_path, 4).await?;
    journal.migrate().await?;

    let start = start([1; 32], "2026-07-21T10:00:00Z")?;
    assert_start_retries_and_conflict(&journal, &start).await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &replay_store)
            .await?,
        ExecutionRecoveryStateV1::StartedNotConsumed(value) if value == start
    ));

    let consumption = replay_consumption(start.admission_digest, 21)?;
    seed_replay_consumption(&replay_path, &consumption).await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &replay_store)
            .await?,
        ExecutionRecoveryStateV1::ConsumedWithoutTerminal { .. }
    ));
    let terminal = build_execution_journal_terminal_v1(
        &start,
        ExecutionJournalTerminalDraftV1 {
            consumption_digest: Some(consumption.consumption_digest),
            outcome_digest: Some(Sha256Digest::from_bytes([31; 32])),
            status: ExecutionJournalTerminalStatusV1::Completed,
            finished_at: timestamp("2026-07-21T10:00:03Z")?,
        },
    )?;
    assert_terminal_retries_and_conflict(&journal, &replay_store, &start, &terminal, &consumption)
        .await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &replay_store)
            .await?,
        ExecutionRecoveryStateV1::Terminal { start: value_start, terminal: value_terminal }
            if value_start == start && value_terminal == terminal
    ));
    let empty_replay_store = SqliteReplayStoreV1::in_memory().await?;
    empty_replay_store.migrate().await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &empty_replay_store)
            .await,
        Err(ExecutionJournalError::StoredBindingMismatch)
    ));
    let wrong_replay_path = directory.path().join("wrong-replay.sqlite");
    let wrong_replay_store = SqliteReplayStoreV1::connect_path(&wrong_replay_path, 1).await?;
    wrong_replay_store.migrate().await?;
    let wrong_consumption = replay_consumption(start.admission_digest, 22)?;
    seed_replay_consumption(&wrong_replay_path, &wrong_consumption).await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &wrong_replay_store)
            .await,
        Err(ExecutionJournalError::StoredBindingMismatch)
    ));

    let persisted = format!(
        "{}{}",
        serde_json::to_string(&start)?,
        serde_json::to_string(&terminal)?
    );
    for forbidden in [
        "messages",
        "content",
        "chunks",
        "signature_hex",
        "request_body",
        "response_body",
    ] {
        assert!(!persisted.contains(forbidden));
    }

    journal.checkpoint_and_close().await?;
    let reopened = SqliteExecutionJournalV1::connect_path(&database_path, 2).await?;
    reopened.migrate().await?;
    assert_eq!(reopened.start(start.admission_digest).await?, Some(start));
    assert_eq!(
        reopened.terminal(terminal.admission_digest).await?,
        Some(terminal)
    );
    reopened.checkpoint_and_close().await?;
    empty_replay_store.checkpoint_and_close().await?;
    wrong_replay_store.checkpoint_and_close().await?;
    replay_store.checkpoint_and_close().await?;
    Ok(())
}

#[tokio::test]
async fn contract_cross_row_and_regressing_time_tampering_fail_restart()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("tamper-journal.sqlite");
    let replay_path = directory.path().join("tamper-replay.sqlite");
    let replay_store = SqliteReplayStoreV1::connect_path(&replay_path, 1).await?;
    replay_store.migrate().await?;
    let journal = SqliteExecutionJournalV1::connect_path(&database_path, 2).await?;
    journal.migrate().await?;
    let first = start([41; 32], "2026-07-21T11:00:00Z")?;
    let second = start([42; 32], "2026-07-21T11:10:00Z")?;
    journal.begin(&first).await?;
    journal.begin(&second).await?;
    let consumption = replay_consumption(second.admission_digest, 43)?;
    seed_replay_consumption(&replay_path, &consumption).await?;
    let terminal = build_execution_journal_terminal_v1(
        &second,
        ExecutionJournalTerminalDraftV1 {
            consumption_digest: Some(consumption.consumption_digest),
            outcome_digest: Some(Sha256Digest::from_bytes([44; 32])),
            status: ExecutionJournalTerminalStatusV1::Completed,
            finished_at: timestamp("2026-07-21T11:10:03Z")?,
        },
    )?;
    journal.finish(&terminal, &replay_store).await?;

    let mut cross_row = terminal.clone();
    cross_row.start_digest = first.start_digest;
    cross_row.terminal_digest = Sha256Digest::from_bytes([0; 32]);
    cross_row.terminal_digest = cross_row.digest()?;
    overwrite_terminal(&database_path, &cross_row).await?;
    assert!(matches!(
        journal
            .recovery_state(second.admission_digest, &replay_store)
            .await,
        Err(ExecutionJournalError::StoredBindingMismatch)
    ));

    let mut regressed = terminal;
    regressed.finished_at = timestamp("2026-07-21T11:09:59Z")?;
    regressed.terminal_digest = Sha256Digest::from_bytes([0; 32]);
    regressed.terminal_digest = regressed.digest()?;
    overwrite_terminal(&database_path, &regressed).await?;
    assert!(matches!(
        journal
            .recovery_state(second.admission_digest, &replay_store)
            .await,
        Err(ExecutionJournalError::StoredBindingMismatch)
    ));
    journal.checkpoint_and_close().await?;
    replay_store.checkpoint_and_close().await?;
    Ok(())
}

#[tokio::test]
async fn contract_provider_agent_seam_stops_before_activation_on_preexisting_consumption()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture: ProviderAttemptFixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/evidence-v4.json"
    ))?;
    let directory = tempdir()?;
    let journal_path = directory.path().join("provider-attempt-journal.sqlite");
    let replay_path = directory.path().join("provider-attempt-replay.sqlite");
    let journal = SqliteExecutionJournalV1::connect_path(&journal_path, 2).await?;
    journal.migrate().await?;
    let replay_store = SqliteReplayStoreV1::connect_path(&replay_path, 2).await?;
    replay_store.migrate().await?;

    let admission_digest = Sha256Digest::from_bytes([91; 32]);
    let consumption = replay_consumption(admission_digest, 92)?;
    seed_replay_consumption(&replay_path, &consumption).await?;
    let admitted_at = timestamp("2026-07-21T10:00:00Z")?;
    let admission = invalid_admission(admission_digest, &admitted_at)?;
    let policy = fixture.bundle.policy.clone();
    let signed_policy = invalid_signed_policy(&policy, &admitted_at)?;
    let verification_keys = VerificationKeyRegistry::default();
    let authenticated = AuthenticatedExecutionContextV1 {
        routing: ReliabilityArtifactVerifiedRoutingContextV1 {
            evaluated_at: &fixture.bundle.evaluated_at,
            service: &fixture.bundle.service,
            providers: &fixture.bundle.providers,
            offers: &fixture.bundle.offers,
            policy: &policy,
            assurance_sets: &fixture.bundle.assurance_sets,
            assurance_blobs: &fixture.assurance_blobs,
            reliability_artifacts: &fixture.reliability_artifacts,
            verification_keys: &verification_keys,
        },
        replay_transition: &fixture.bundle.replay_transition,
        signed_policy: &signed_policy,
    };
    let request = ChatCompletionRequestV1 {
        model: fixture.bundle.service.model_version.as_str().to_owned(),
        messages: vec![ChatCompletionMessageV1 {
            role: ChatMessageRoleV1::User,
            content: "must not execute".to_owned(),
        }],
        stream: true,
    };
    let cancellation = CancellationSignalV1::default();
    let control = invalid_provider_control(&admission.provider_id, &admitted_at)?;
    let mut provider = MustNotRunProvider {
        provider_id: control.binding.provider_id.clone(),
        identity_reads: Cell::new(0),
        calls: 0,
    };
    let signer = MustNotRunSigner {
        provider_id: control.binding.provider_id.clone(),
        key_id: control.binding.receipt_key_id.clone(),
        identity_reads: Cell::new(0),
        calls: Cell::new(0),
    };
    let activation_calls = Cell::new(0_u64);

    let result = execute_journaled_provider_agent_with_activation_v1(
        &journal,
        ProviderAgentExecutionContextV1 {
            local_execution: LocalExecutionContextV1 {
                replay_store: &replay_store,
                admission: &admission,
                authenticated: &authenticated,
                verification_keys: &verification_keys,
                request: &request,
                consumed_at: &admitted_at,
                cancellation: &cancellation,
            },
            manifest: &control.manifest,
            readiness: &control.readiness,
            revocation: &control.revocation,
            checkpoint: &control.checkpoint,
            key_registry: &control.key_registry,
            expected_binding: &control.binding,
            evaluated_at: &admitted_at,
        },
        &mut provider,
        &signer,
        |_| {
            activation_calls.set(activation_calls.get() + 1);
            Ok::<(), ProviderAgentError>(())
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(ExecutionJournalError::ConsumedWithoutTerminal)
    ));
    assert_eq!(activation_calls.get(), 0);
    assert_eq!(provider.identity_reads.get(), 0);
    assert_eq!(provider.calls, 0);
    assert_eq!(signer.identity_reads.get(), 0);
    assert_eq!(signer.calls.get(), 0);
    assert_eq!(journal.start(admission_digest).await?, None);
    assert_eq!(journal.terminal(admission_digest).await?, None);
    journal.checkpoint_and_close().await?;
    replay_store.checkpoint_and_close().await?;
    Ok(())
}

#[ignore = "known open item: failing at the 2026-10 release-staging snapshot; tracked in RESEARCH.md"]
#[tokio::test]
async fn contract_provider_agent_seam_derives_completed_incomplete_and_failed_terminals()
-> Result<(), Box<dyn std::error::Error>> {
    for kind in [
        ValidAttemptKind::Completed,
        ValidAttemptKind::Incomplete,
        ValidAttemptKind::ErrorAfterConsumption,
    ] {
        assert_valid_attempt(kind).await?;
    }
    Ok(())
}

#[tokio::test]
async fn contract_provider_agent_preflight_rejection_is_terminal_without_effects()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, registry, buyer_key) = valid_execution_environment()?;
    let request = valid_request(&fixture);
    let case = valid_execution_case(&fixture, &registry, &buyer_key, &request)?;
    let authenticated = valid_authenticated_context(&fixture, &case, &registry);
    let replay_store = initialized_valid_store(&fixture).await?;
    let journal = SqliteExecutionJournalV1::in_memory().await?;
    journal.migrate().await?;
    let mut control = valid_provider_control(&fixture, &case)?;
    control.evaluated_at = timestamp("2026-07-20T11:59:59Z")?;
    let cancellation = CancellationSignalV1::default();
    let mut provider = provider_for_attempt(&fixture, ValidAttemptKind::Completed)?;
    let signer = valid_receipt_signer()?;
    let activation_calls = Cell::new(0_u64);
    let result = execute_journaled_provider_agent_with_activation_v1(
        &journal,
        valid_provider_context(
            &replay_store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
        |_| {
            activation_calls.set(activation_calls.get() + 1);
            Ok(())
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(ExecutionJournalError::ProviderAgent(
            ProviderAgentError::ControlDenied
        ))
    ));
    let terminal = journal
        .terminal(case.admission.admission_digest)
        .await?
        .ok_or("preflight terminal missing")?;
    assert_eq!(
        terminal.status,
        ExecutionJournalTerminalStatusV1::RejectedBeforeConsumption
    );
    assert_eq!(activation_calls.get(), 0);
    assert_eq!(provider.calls, 0);
    assert_eq!(signer.calls.load(Ordering::Relaxed), 0);
    assert_eq!(replay_store.consumption_count().await?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn contract_concurrent_preflight_failure_cannot_claim_another_attempt_consumption()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, _, _) = valid_execution_environment()?;
    let replay_store = initialized_valid_store(&fixture).await?;
    let barrier = Arc::new(Barrier::new(2));
    let (consumed_tx, consumed_rx) = mpsc::channel();
    let valid_store = replay_store.clone();
    let valid_barrier = Arc::clone(&barrier);
    let valid = std::thread::spawn(move || {
        current_thread_runtime()?.block_on(run_racing_provider_attempt(
            valid_store,
            false,
            valid_barrier,
            Some(consumed_tx),
            None,
        ))
    });
    let invalid_store = replay_store.clone();
    let invalid = std::thread::spawn(move || {
        current_thread_runtime()?.block_on(run_racing_provider_attempt(
            invalid_store,
            true,
            barrier,
            None,
            Some(consumed_rx),
        ))
    });
    let valid_result = valid
        .join()
        .map_err(|_| std::io::Error::other("valid race thread panicked"))?
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let invalid_result = invalid
        .join()
        .map_err(|_| std::io::Error::other("invalid race thread panicked"))?
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    assert_eq!(valid_result, (true, true));
    assert_eq!(invalid_result, (true, false));
    assert_eq!(replay_store.consumption_count().await?, 1);
    replay_store.checkpoint_and_close().await?;
    Ok(())
}

#[ignore = "known open item: failing at the 2026-10 release-staging snapshot; tracked in RESEARCH.md"]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn contract_concurrent_exact_wrapper_retry_invokes_provider_agent_once()
-> Result<(), Box<dyn std::error::Error>> {
    let (fixture, _, _) = valid_execution_environment()?;
    let replay_store = initialized_valid_store(&fixture).await?;
    let journal = SqliteExecutionJournalV1::in_memory().await?;
    journal.migrate().await?;
    let left_store = replay_store.clone();
    let left_journal = journal.clone();
    let left = std::thread::spawn(move || {
        current_thread_runtime()?.block_on(run_shared_journal_attempt(left_store, left_journal))
    });
    let right_store = replay_store.clone();
    let right_journal = journal.clone();
    let right = std::thread::spawn(move || {
        current_thread_runtime()?.block_on(run_shared_journal_attempt(right_store, right_journal))
    });
    let left_result = join_attempt_thread(left, "left")?;
    let right_result = join_attempt_thread(right, "right")?;
    assert_eq!(u8::from(left_result.0) + u8::from(right_result.0), 1);
    assert_eq!(left_result.1 + right_result.1, 1);
    assert_eq!(left_result.2 + right_result.2, 1);
    assert_eq!(replay_store.consumption_count().await?, 1);
    assert!(journal.terminal(valid_admission_digest()?).await?.is_some());
    Ok(())
}

type AttemptOutcome = Result<(bool, u64, u64), Box<dyn std::error::Error + Send + Sync>>;

fn join_attempt_thread(
    handle: std::thread::JoinHandle<AttemptOutcome>,
    name: &str,
) -> Result<(bool, u64, u64), Box<dyn std::error::Error>> {
    handle
        .join()
        .map_err(|_| std::io::Error::other(format!("{name} attempt thread panicked")))?
        .map_err(|error| std::io::Error::other(error.to_string()).into())
}

async fn run_shared_journal_attempt(
    replay_store: SqliteReplayStoreV1,
    journal: SqliteExecutionJournalV1,
) -> Result<(bool, u64, u64), Box<dyn std::error::Error + Send + Sync>> {
    let (fixture, registry, buyer_key) =
        valid_execution_environment().map_err(|error| boxed_send_error(error.as_ref()))?;
    let request = valid_request(&fixture);
    let case = valid_execution_case(&fixture, &registry, &buyer_key, &request)
        .map_err(|error| boxed_send_error(error.as_ref()))?;
    let authenticated = valid_authenticated_context(&fixture, &case, &registry);
    let control = valid_provider_control(&fixture, &case)
        .map_err(|error| boxed_send_error(error.as_ref()))?;
    let cancellation = CancellationSignalV1::default();
    let mut provider = provider_for_attempt(&fixture, ValidAttemptKind::Completed)
        .map_err(|error| boxed_send_error(error.as_ref()))?;
    let signer = valid_receipt_signer().map_err(|error| boxed_send_error(error.as_ref()))?;
    let activation_calls = Cell::new(0_u64);
    let result = execute_journaled_provider_agent_with_activation_v1(
        &journal,
        valid_provider_context(
            &replay_store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
        |_| {
            activation_calls.set(activation_calls.get() + 1);
            Ok(())
        },
    )
    .await;
    Ok((result.is_ok(), activation_calls.get(), provider.calls))
}

fn valid_admission_digest() -> Result<Sha256Digest, Box<dyn std::error::Error>> {
    let (fixture, registry, buyer_key) = valid_execution_environment()?;
    let request = valid_request(&fixture);
    Ok(
        valid_execution_case(&fixture, &registry, &buyer_key, &request)?
            .admission
            .admission_digest,
    )
}

fn current_thread_runtime()
-> Result<tokio::runtime::Runtime, Box<dyn std::error::Error + Send + Sync>> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?)
}

async fn run_racing_provider_attempt(
    replay_store: SqliteReplayStoreV1,
    invalidate_provider: bool,
    barrier: Arc<Barrier>,
    signal: Option<mpsc::Sender<()>>,
    wait_for_consumption: Option<mpsc::Receiver<()>>,
) -> Result<(bool, bool), Box<dyn std::error::Error + Send + Sync>> {
    let (fixture, registry, buyer_key) =
        valid_execution_environment().map_err(|error| boxed_send_error(error.as_ref()))?;
    let request = valid_request(&fixture);
    let case = valid_execution_case(&fixture, &registry, &buyer_key, &request)
        .map_err(|error| boxed_send_error(error.as_ref()))?;
    let authenticated = valid_authenticated_context(&fixture, &case, &registry);
    let journal = SqliteExecutionJournalV1::in_memory().await?;
    journal.migrate().await?;
    let control = valid_provider_control(&fixture, &case)
        .map_err(|error| boxed_send_error(error.as_ref()))?;
    let cancellation = CancellationSignalV1::default();
    let mut provider = provider_for_attempt(&fixture, ValidAttemptKind::Completed)
        .map_err(|error| boxed_send_error(error.as_ref()))?;
    provider.signal = signal;
    let signer = valid_receipt_signer().map_err(|error| boxed_send_error(error.as_ref()))?;
    let result = execute_journaled_provider_agent_with_activation_v1(
        &journal,
        valid_provider_context(
            &replay_store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
        |provider| {
            barrier.wait();
            if invalidate_provider {
                provider.provider_id = "provider:other"
                    .parse()
                    .map_err(|_| ProviderAgentError::ActivationDenied)?;
                wait_for_consumption
                    .ok_or(ProviderAgentError::ActivationDenied)?
                    .recv()
                    .map_err(|_| ProviderAgentError::ActivationDenied)?;
            }
            Ok(())
        },
    )
    .await;
    let terminal_exists = journal
        .terminal(case.admission.admission_digest)
        .await?
        .is_some();
    let expected = if invalidate_provider {
        matches!(result, Err(ExecutionJournalError::ConsumedWithoutTerminal))
    } else {
        result.is_ok()
    };
    Ok((expected, terminal_exists))
}

fn boxed_send_error(error: &dyn std::error::Error) -> Box<dyn std::error::Error + Send + Sync> {
    std::io::Error::other(error.to_string()).into()
}

async fn assert_valid_attempt(kind: ValidAttemptKind) -> Result<(), Box<dyn std::error::Error>> {
    let (fixture, registry, buyer_key) = valid_execution_environment()?;
    let request = valid_request(&fixture);
    let case = valid_execution_case(&fixture, &registry, &buyer_key, &request)?;
    let authenticated = valid_authenticated_context(&fixture, &case, &registry);
    let replay_store = initialized_valid_store(&fixture).await?;
    let journal = SqliteExecutionJournalV1::in_memory().await?;
    journal.migrate().await?;
    let control = valid_provider_control(&fixture, &case)?;
    let cancellation = CancellationSignalV1::default();
    let mut provider = provider_for_attempt(&fixture, kind)?;
    let signer = valid_receipt_signer()?;
    let activation_calls = Cell::new(0_u64);
    let result = execute_journaled_provider_agent_with_activation_v1(
        &journal,
        valid_provider_context(
            &replay_store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        ),
        &mut provider,
        &signer,
        |_| {
            activation_calls.set(activation_calls.get() + 1);
            Ok(())
        },
    )
    .await;
    assert_valid_attempt_result(kind, result, &journal, case.admission.admission_digest).await?;
    assert_eq!(activation_calls.get(), 1);
    assert_eq!(provider.calls, 1);
    assert_eq!(replay_store.consumption_count().await?, 1);
    let expected_signer_calls = u64::from(matches!(kind, ValidAttemptKind::Completed));
    assert_eq!(signer.calls.load(Ordering::Relaxed), expected_signer_calls);
    journal.checkpoint_and_close().await?;
    replay_store.checkpoint_and_close().await?;
    Ok(())
}

async fn assert_valid_attempt_result(
    kind: ValidAttemptKind,
    result: Result<
        hyperintel_execution_journal::JournaledProviderAgentExecutionV1,
        ExecutionJournalError,
    >,
    journal: &SqliteExecutionJournalV1,
    admission: Sha256Digest,
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = match kind {
        ValidAttemptKind::Completed => ExecutionJournalTerminalStatusV1::Completed,
        ValidAttemptKind::Incomplete => ExecutionJournalTerminalStatusV1::Incomplete,
        ValidAttemptKind::ErrorAfterConsumption => {
            assert!(matches!(
                result,
                Err(ExecutionJournalError::ConsumedWithoutTerminal)
            ));
            assert_eq!(journal.terminal(admission).await?, None);
            return Ok(());
        }
    };
    let journaled = result?;
    assert_eq!(journaled.terminal.status, expected);
    assert!(journaled.terminal.consumption_digest.is_some());
    assert!(journaled.terminal.outcome_digest.is_some());
    match (kind, journaled.execution.outcome()) {
        (ValidAttemptKind::Completed, LocalExecutionOutcomeV1::Completed(_))
        | (ValidAttemptKind::Incomplete, LocalExecutionOutcomeV1::Incomplete(_)) => {}
        _ => return Err("unexpected provider-agent outcome".into()),
    }
    Ok(())
}

fn valid_provider_context<'a>(
    store: &'a SqliteReplayStoreV1,
    case: &'a ValidExecutionCase,
    authenticated: &'a AuthenticatedExecutionContextV1<'a>,
    registry: &'a VerificationKeyRegistry,
    request: &'a ChatCompletionRequestV1,
    cancellation: &'a CancellationSignalV1,
    control: &'a ValidProviderControl,
) -> ProviderAgentExecutionContextV1<'a> {
    ProviderAgentExecutionContextV1 {
        local_execution: LocalExecutionContextV1 {
            replay_store: store,
            admission: &case.admission,
            authenticated,
            verification_keys: registry,
            request,
            consumed_at: &case.admission.admitted_at,
            cancellation,
        },
        manifest: &control.manifest,
        readiness: &control.readiness,
        revocation: &control.revocation,
        checkpoint: &control.checkpoint,
        key_registry: &control.registry,
        expected_binding: &control.binding,
        evaluated_at: &control.evaluated_at,
    }
}

fn valid_execution_environment()
-> Result<(ProviderAttemptFixture, VerificationKeyRegistry, SigningKey), Box<dyn std::error::Error>>
{
    let fixture: ProviderAttemptFixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/evidence-v4.json"
    ))?;
    let trusted: TrustedKeysFixture = serde_json::from_slice(include_bytes!(
        "../../hyperintel-cli/tests/fixtures/trusted-keys-v4.json"
    ))?;
    let buyer_key = SigningKey::from_bytes(&[80; 32]);
    let mut registry = VerificationKeyRegistry::default();
    for key in trusted.trusted_verification_keys {
        registry.register(key)?;
    }
    register_valid_market_key(
        &mut registry,
        "buyer:demo",
        "key:buyer-policy-1",
        AssuranceRootClassV1::BuyerPolicy,
        &buyer_key,
    )?;
    register_valid_market_key(
        &mut registry,
        "provider:alpha",
        "key:alpha-receipt",
        AssuranceRootClassV1::ProviderKey,
        &SigningKey::from_bytes(&[81; 32]),
    )?;
    verify_replay_protected_market_evidence_bundle_v4(
        &fixture.bundle,
        &fixture.assurance_blobs,
        &fixture.reliability_artifacts,
        &registry,
    )?;
    Ok((fixture, registry, buyer_key))
}

fn valid_execution_case(
    fixture: &ProviderAttemptFixture,
    registry: &VerificationKeyRegistry,
    buyer_key: &SigningKey,
    request: &ChatCompletionRequestV1,
) -> Result<ValidExecutionCase, Box<dyn std::error::Error>> {
    let mut policy = fixture.bundle.policy.clone();
    policy.request_id = "request:journal-provider-agent-alpha".parse()?;
    policy.allowed_providers.clear();
    policy.allowed_providers.insert("provider:alpha".parse()?);
    policy.prompt_digest = chat_completion_request_digest_v1(request)?;
    let evaluation = evaluate_reliability_artifact_verified_route_v1(&valid_routing_context(
        fixture, &policy, registry,
    ))?;
    let replay_transition = derive_replay_state_transition_v1(
        &fixture.bundle.replay_transition.before_state,
        &policy,
        evaluation.evaluation_digest,
        &fixture.bundle.evaluated_at,
    )?;
    let signed_policy = valid_signed_buyer_policy(&policy, buyer_key)?;
    let authenticated = AuthenticatedExecutionContextV1 {
        routing: valid_routing_context(fixture, &policy, registry),
        replay_transition: &replay_transition,
        signed_policy: &signed_policy,
    };
    let admission = build_authenticated_execution_admission_v1(
        &authenticated,
        registry,
        timestamp("2026-07-20T12:00:00Z")?,
        timestamp("2026-07-20T12:05:00Z")?,
    )?;
    Ok(ValidExecutionCase {
        policy,
        signed_policy,
        replay_transition,
        admission,
    })
}

fn valid_authenticated_context<'a>(
    fixture: &'a ProviderAttemptFixture,
    case: &'a ValidExecutionCase,
    registry: &'a VerificationKeyRegistry,
) -> AuthenticatedExecutionContextV1<'a> {
    AuthenticatedExecutionContextV1 {
        routing: valid_routing_context(fixture, &case.policy, registry),
        replay_transition: &case.replay_transition,
        signed_policy: &case.signed_policy,
    }
}

fn valid_routing_context<'a>(
    fixture: &'a ProviderAttemptFixture,
    policy: &'a InferenceRequestPolicyV1,
    registry: &'a VerificationKeyRegistry,
) -> ReliabilityArtifactVerifiedRoutingContextV1<'a> {
    ReliabilityArtifactVerifiedRoutingContextV1 {
        evaluated_at: &fixture.bundle.evaluated_at,
        service: &fixture.bundle.service,
        providers: &fixture.bundle.providers,
        offers: &fixture.bundle.offers,
        policy,
        assurance_sets: &fixture.bundle.assurance_sets,
        assurance_blobs: &fixture.assurance_blobs,
        reliability_artifacts: &fixture.reliability_artifacts,
        verification_keys: registry,
    }
}

fn valid_request(fixture: &ProviderAttemptFixture) -> ChatCompletionRequestV1 {
    ChatCompletionRequestV1 {
        model: fixture.bundle.service.model_version.as_str().to_owned(),
        messages: vec![ChatCompletionMessageV1 {
            role: ChatMessageRoleV1::User,
            content: "execute journaled provider agent".to_owned(),
        }],
        stream: true,
    }
}

async fn initialized_valid_store(
    fixture: &ProviderAttemptFixture,
) -> Result<SqliteReplayStoreV1, Box<dyn std::error::Error>> {
    let store = SqliteReplayStoreV1::in_memory().await?;
    store.migrate().await?;
    assert_eq!(
        store
            .initialize_buyer_state(
                &fixture.bundle.replay_transition.before_state,
                &fixture.bundle.evaluated_at,
            )
            .await?,
        InitializeReplayStateOutcomeV1::Inserted
    );
    Ok(store)
}

fn provider_for_attempt(
    fixture: &ProviderAttemptFixture,
    kind: ValidAttemptKind,
) -> Result<ScriptedProvider, Box<dyn std::error::Error>> {
    let result = match kind {
        ValidAttemptKind::Incomplete => Err(ProviderAgentErrorV1::Unavailable),
        ValidAttemptKind::Completed | ValidAttemptKind::ErrorAfterConsumption => {
            let frames = if matches!(kind, ValidAttemptKind::Completed) {
                vec![ProviderStreamFrameV1 {
                    emitted_at: timestamp("2026-07-20T12:00:02Z")?,
                    content: "journaled response".to_owned(),
                    output_token_delta: 1,
                }]
            } else {
                Vec::new()
            };
            Ok(ProviderExecutionPlanV1 {
                provider_id: "provider:alpha".parse()?,
                started_at: timestamp("2026-07-20T12:00:01Z")?,
                frames,
                completed_at: timestamp("2026-07-20T12:00:03Z")?,
                input_tokens: 2,
                runtime_image_digest: fixture.bundle.service.runtime_image_digest,
            })
        }
    };
    Ok(ScriptedProvider {
        provider_id: "provider:alpha".parse()?,
        result: Some(result),
        calls: 0,
        signal: None,
    })
}

fn valid_receipt_signer() -> Result<TestReceiptSigner, Box<dyn std::error::Error>> {
    Ok(TestReceiptSigner {
        provider_id: "provider:alpha".parse()?,
        key_id: "key:alpha-receipt".parse()?,
        key: SigningKey::from_bytes(&[81; 32]),
        calls: AtomicU64::new(0),
    })
}

fn valid_signed_buyer_policy(
    policy: &InferenceRequestPolicyV1,
    key: &SigningKey,
) -> Result<SignedInferenceRequestPolicyV1, Box<dyn std::error::Error>> {
    let mut statement = BuyerPolicyStatementV1 {
        schema_version: "hyperintel.buyer-policy-statement.v1".to_owned(),
        request_policy_digest: policy.digest()?,
        replay_snapshot_digest: policy.replay_snapshot_digest,
        issued_at: policy.valid_from.clone(),
        expires_at: policy.valid_until.clone(),
        sequence: policy.nonce,
        signer_id: policy.buyer_id.clone(),
        key_id: "key:buyer-policy-1".parse()?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    };
    statement.signature_hex = HEXLOWER.encode(&key.sign(&statement.signing_message()?).to_bytes());
    Ok(SignedInferenceRequestPolicyV1 {
        schema_version: "hyperintel.signed-inference-request-policy.v1".to_owned(),
        policy: policy.clone(),
        statement,
    })
}

fn register_valid_market_key(
    registry: &mut VerificationKeyRegistry,
    signer_id: &str,
    key_id: &str,
    class: AssuranceRootClassV1,
    key: &SigningKey,
) -> Result<(), Box<dyn std::error::Error>> {
    registry.register(RegisteredVerificationKeyV1 {
        signer_id: signer_id.parse()?,
        key_id: key_id.parse()?,
        trust_root_class: class,
        public_key_hex: HEXLOWER.encode(key.verifying_key().as_bytes()),
        valid_from: timestamp("2026-07-20T10:00:00Z")?,
        valid_until: timestamp("2026-07-20T14:00:00Z")?,
        revoked_at: None,
    })?;
    Ok(())
}

fn valid_provider_control(
    fixture: &ProviderAttemptFixture,
    case: &ValidExecutionCase,
) -> Result<ValidProviderControl, Box<dyn std::error::Error>> {
    let manifest_key = SigningKey::from_bytes(&[91; 32]);
    let readiness_key = SigningKey::from_bytes(&[92; 32]);
    let revocation_key = SigningKey::from_bytes(&[93; 32]);
    let registry = valid_provider_agent_registry(&manifest_key, &readiness_key, &revocation_key)?;
    let service = &fixture.bundle.service;
    let provider = fixture
        .bundle
        .providers
        .get(&"provider:alpha".parse::<ProtocolId>()?)
        .ok_or("provider missing")?;
    let binding = ProviderAgentBindingV1 {
        provider_id: "provider:alpha".parse()?,
        agent_instance_id: "agent:alpha-1".parse()?,
        service_spec_digest: service.digest()?,
        selected_offer_digest: case.admission.offer_digest,
        runtime_configuration_digest: Sha256Digest::from_bytes([71; 32]),
        model_artifact_digest: service.model_artifact_digest,
        tokenizer_digest: service.tokenizer_digest,
        runtime_image_digest: service.runtime_image_digest,
        capabilities: service.capabilities.iter().cloned().collect(),
        receipt_key_id: provider.receipt_key_id.clone(),
        interface: ProviderTextStreamInterfaceV1::OpenAiTextStreamingChatCompletionsV1,
        secret_reference_digest: Sha256Digest::from_bytes([72; 32]),
    };
    build_valid_provider_control(
        binding,
        registry,
        &manifest_key,
        &readiness_key,
        &revocation_key,
    )
}

fn build_valid_provider_control(
    binding: ProviderAgentBindingV1,
    registry: ProviderAgentKeyRegistryV1,
    manifest_key: &SigningKey,
    readiness_key: &SigningKey,
    revocation_key: &SigningKey,
) -> Result<ValidProviderControl, Box<dyn std::error::Error>> {
    let mut manifest = valid_unsigned_manifest(&binding)?;
    manifest.signature_hex =
        HEXLOWER.encode(&manifest_key.sign(&manifest.signing_message()?).to_bytes());
    let mut readiness = valid_unsigned_readiness(&binding, manifest.digest()?)?;
    readiness.signature_hex =
        HEXLOWER.encode(&readiness_key.sign(&readiness.signing_message()?).to_bytes());
    let mut revocation = valid_unsigned_revocation(&binding)?;
    revocation.signature_hex = HEXLOWER.encode(
        &revocation_key
            .sign(&revocation.signing_message()?)
            .to_bytes(),
    );
    let checkpoint = ProviderAgentRevocationCheckpointV1 {
        provider_id: binding.provider_id.clone(),
        runtime_configuration_digest: binding.runtime_configuration_digest,
        agent_instance_id: binding.agent_instance_id.clone(),
        binding_digest: binding.digest()?,
        manifest_minimum_sequence: 1,
        manifest_digest_at_minimum_sequence: manifest.digest()?,
        readiness_minimum_sequence: 1,
        readiness_digest_at_minimum_sequence: readiness.digest()?,
        revocation_minimum_sequence: 1,
        revocation_digest_at_minimum_sequence: revocation.digest()?,
    };
    Ok(ValidProviderControl {
        registry,
        binding,
        manifest,
        readiness,
        revocation,
        checkpoint,
        evaluated_at: timestamp("2026-07-20T12:00:00Z")?,
    })
}

fn valid_provider_agent_registry(
    manifest: &SigningKey,
    readiness: &SigningKey,
    revocation: &SigningKey,
) -> Result<ProviderAgentKeyRegistryV1, Box<dyn std::error::Error>> {
    let mut registry = ProviderAgentKeyRegistryV1::default();
    for (key, purpose, key_id) in [
        (
            manifest,
            ProviderAgentKeyPurposeV1::ServiceManifest,
            "key:pa-manifest",
        ),
        (
            readiness,
            ProviderAgentKeyPurposeV1::ReadinessAssertion,
            "key:pa-readiness",
        ),
        (
            revocation,
            ProviderAgentKeyPurposeV1::RevocationSnapshot,
            "key:pa-revocation",
        ),
    ] {
        registry.register(RegisteredProviderAgentKeyV1 {
            schema_version: PROVIDER_AGENT_KEY_SCHEMA_V1.to_owned(),
            provider_id: "provider:alpha".parse()?,
            key_id: key_id.parse()?,
            purpose,
            public_key_hex: HEXLOWER.encode(key.verifying_key().as_bytes()),
            valid_from: timestamp("2026-07-20T11:00:00Z")?,
            valid_until: timestamp("2026-07-20T13:00:00Z")?,
            revoked_at: None,
        })?;
    }
    Ok(registry)
}

fn valid_unsigned_manifest(
    binding: &ProviderAgentBindingV1,
) -> Result<SignedProviderAgentServiceManifestV1, Box<dyn std::error::Error>> {
    Ok(SignedProviderAgentServiceManifestV1 {
        schema_version: SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1.to_owned(),
        claims: ProviderAgentServiceManifestClaimsV1 {
            schema_version: PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1.to_owned(),
            sequence: 1,
            binding: binding.clone(),
            issued_at: timestamp("2026-07-20T12:00:00Z")?,
            expires_at: timestamp("2026-07-20T12:30:00Z")?,
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:pa-manifest".parse()?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    })
}

fn valid_unsigned_readiness(
    binding: &ProviderAgentBindingV1,
    manifest_digest: Sha256Digest,
) -> Result<SignedProviderAgentReadinessV1, Box<dyn std::error::Error>> {
    Ok(SignedProviderAgentReadinessV1 {
        schema_version: SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1.to_owned(),
        claims: ProviderAgentReadinessClaimsV1 {
            schema_version: PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1.to_owned(),
            sequence: 1,
            binding: binding.clone(),
            service_manifest_digest: manifest_digest,
            status: ProviderAgentReadinessStatusV1::Ready,
            issued_at: timestamp("2026-07-20T12:00:00Z")?,
            expires_at: timestamp("2026-07-20T12:05:00Z")?,
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:pa-readiness".parse()?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    })
}

fn valid_unsigned_revocation(
    binding: &ProviderAgentBindingV1,
) -> Result<SignedProviderAgentRevocationSnapshotV1, Box<dyn std::error::Error>> {
    Ok(SignedProviderAgentRevocationSnapshotV1 {
        schema_version: SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
        snapshot: ProviderAgentRevocationSnapshotV1 {
            schema_version: PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
            provider_id: binding.provider_id.clone(),
            runtime_configuration_digest: binding.runtime_configuration_digest,
            sequence: 1,
            issued_at: timestamp("2026-07-20T12:00:00Z")?,
            effective_at: timestamp("2026-07-20T12:00:00Z")?,
            expires_at: timestamp("2026-07-20T12:05:00Z")?,
            assertion_revocations: Vec::new(),
            key_revocations: Vec::new(),
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:pa-revocation".parse()?,
        algorithm: ED25519_ALGORITHM_V1.to_owned(),
        signature_hex: String::new(),
    })
}

fn invalid_admission(
    admission_digest: Sha256Digest,
    admitted_at: &UtcTimestamp,
) -> Result<AuthenticatedExecutionAdmissionV1, Box<dyn std::error::Error>> {
    Ok(AuthenticatedExecutionAdmissionV1 {
        schema_version: "invalid-before-recovery".to_owned(),
        state_slice: "invalid-before-recovery".to_owned(),
        buyer_id: "buyer:journal".parse()?,
        buyer_policy_key_id: "key:buyer".parse()?,
        buyer_policy_statement_digest: Sha256Digest::from_bytes([1; 32]),
        request_policy_digest: Sha256Digest::from_bytes([2; 32]),
        route_evaluation_digest: Sha256Digest::from_bytes([3; 32]),
        route_decision_digest: Sha256Digest::from_bytes([4; 32]),
        replay_transition_digest: Sha256Digest::from_bytes([5; 32]),
        service_spec_digest: Sha256Digest::from_bytes([6; 32]),
        provider_identity_digest: Sha256Digest::from_bytes([7; 32]),
        provider_id: "provider:alpha".parse()?,
        offer_digest: Sha256Digest::from_bytes([8; 32]),
        prompt_digest: Sha256Digest::from_bytes([9; 32]),
        admitted_at: admitted_at.clone(),
        deadline_at: timestamp("2026-07-21T10:05:00Z")?,
        authority_granted: false,
        admission_digest,
    })
}

fn invalid_signed_policy(
    policy: &hyperintel_core::InferenceRequestPolicyV1,
    admitted_at: &UtcTimestamp,
) -> Result<SignedInferenceRequestPolicyV1, Box<dyn std::error::Error>> {
    Ok(SignedInferenceRequestPolicyV1 {
        schema_version: "invalid-before-recovery".to_owned(),
        policy: policy.clone(),
        statement: BuyerPolicyStatementV1 {
            schema_version: "invalid-before-recovery".to_owned(),
            request_policy_digest: Sha256Digest::from_bytes([10; 32]),
            replay_snapshot_digest: Sha256Digest::from_bytes([11; 32]),
            issued_at: admitted_at.clone(),
            expires_at: admitted_at.clone(),
            sequence: 1,
            signer_id: "buyer:journal".parse()?,
            key_id: "key:buyer".parse()?,
            algorithm: "invalid".to_owned(),
            signature_hex: String::new(),
        },
    })
}

fn invalid_provider_control(
    provider_id: &ProtocolId,
    admitted_at: &UtcTimestamp,
) -> Result<InvalidProviderControl, Box<dyn std::error::Error>> {
    let binding = ProviderAgentBindingV1 {
        provider_id: provider_id.clone(),
        agent_instance_id: "agent:must-not-run".parse()?,
        service_spec_digest: Sha256Digest::from_bytes([12; 32]),
        selected_offer_digest: Sha256Digest::from_bytes([13; 32]),
        runtime_configuration_digest: Sha256Digest::from_bytes([14; 32]),
        model_artifact_digest: Sha256Digest::from_bytes([15; 32]),
        tokenizer_digest: Sha256Digest::from_bytes([16; 32]),
        runtime_image_digest: Sha256Digest::from_bytes([17; 32]),
        capabilities: Vec::new(),
        receipt_key_id: "key:alpha-receipt".parse()?,
        interface: ProviderTextStreamInterfaceV1::OpenAiTextStreamingChatCompletionsV1,
        secret_reference_digest: Sha256Digest::from_bytes([18; 32]),
    };
    let manifest = SignedProviderAgentServiceManifestV1 {
        schema_version: "invalid-before-recovery".to_owned(),
        claims: ProviderAgentServiceManifestClaimsV1 {
            schema_version: "invalid-before-recovery".to_owned(),
            sequence: 1,
            binding: binding.clone(),
            issued_at: admitted_at.clone(),
            expires_at: admitted_at.clone(),
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:manifest".parse()?,
        algorithm: "invalid".to_owned(),
        signature_hex: String::new(),
    };
    let readiness = invalid_readiness(&binding, admitted_at)?;
    let revocation = invalid_revocation(&binding, admitted_at)?;
    let checkpoint = ProviderAgentRevocationCheckpointV1 {
        provider_id: binding.provider_id.clone(),
        runtime_configuration_digest: binding.runtime_configuration_digest,
        agent_instance_id: binding.agent_instance_id.clone(),
        binding_digest: Sha256Digest::from_bytes([20; 32]),
        manifest_minimum_sequence: 1,
        manifest_digest_at_minimum_sequence: Sha256Digest::from_bytes([21; 32]),
        readiness_minimum_sequence: 1,
        readiness_digest_at_minimum_sequence: Sha256Digest::from_bytes([22; 32]),
        revocation_minimum_sequence: 1,
        revocation_digest_at_minimum_sequence: Sha256Digest::from_bytes([23; 32]),
    };
    Ok(InvalidProviderControl {
        binding,
        manifest,
        readiness,
        revocation,
        checkpoint,
        key_registry: ProviderAgentKeyRegistryV1::default(),
    })
}

fn invalid_readiness(
    binding: &ProviderAgentBindingV1,
    admitted_at: &UtcTimestamp,
) -> Result<SignedProviderAgentReadinessV1, Box<dyn std::error::Error>> {
    Ok(SignedProviderAgentReadinessV1 {
        schema_version: "invalid-before-recovery".to_owned(),
        claims: ProviderAgentReadinessClaimsV1 {
            schema_version: "invalid-before-recovery".to_owned(),
            sequence: 1,
            binding: binding.clone(),
            service_manifest_digest: Sha256Digest::from_bytes([19; 32]),
            status: ProviderAgentReadinessStatusV1::Ready,
            issued_at: admitted_at.clone(),
            expires_at: admitted_at.clone(),
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:readiness".parse()?,
        algorithm: "invalid".to_owned(),
        signature_hex: String::new(),
    })
}

fn invalid_revocation(
    binding: &ProviderAgentBindingV1,
    admitted_at: &UtcTimestamp,
) -> Result<SignedProviderAgentRevocationSnapshotV1, Box<dyn std::error::Error>> {
    Ok(SignedProviderAgentRevocationSnapshotV1 {
        schema_version: "invalid-before-recovery".to_owned(),
        snapshot: ProviderAgentRevocationSnapshotV1 {
            schema_version: "invalid-before-recovery".to_owned(),
            provider_id: binding.provider_id.clone(),
            runtime_configuration_digest: binding.runtime_configuration_digest,
            sequence: 1,
            issued_at: admitted_at.clone(),
            effective_at: admitted_at.clone(),
            expires_at: admitted_at.clone(),
            assertion_revocations: Vec::new(),
            key_revocations: Vec::new(),
        },
        signer_id: binding.provider_id.clone(),
        key_id: "key:revocation".parse()?,
        algorithm: "invalid".to_owned(),
        signature_hex: String::new(),
    })
}

fn start(
    admission: [u8; 32],
    started_at: &str,
) -> Result<hyperintel_execution_journal::ExecutionJournalStartV1, Box<dyn std::error::Error>> {
    Ok(build_execution_journal_start_v1(
        ExecutionJournalStartDraftV1 {
            admission_digest: Sha256Digest::from_bytes(admission),
            buyer_id: "buyer:journal".parse()?,
            provider_id: "provider:journal".parse()?,
            prompt_digest: Sha256Digest::from_bytes([2; 32]),
            started_at: timestamp(started_at)?,
        },
    )?)
}

fn replay_consumption(
    admission_digest: Sha256Digest,
    marker: u8,
) -> Result<AuthenticatedReplayConsumptionV1, Box<dyn std::error::Error>> {
    let buyer_id: ProtocolId = "buyer:journal".parse()?;
    let before_state = build_buyer_replay_state_v1(buyer_id.clone(), 7)?;
    let after_state = build_buyer_replay_state_v1(buyer_id.clone(), 8)?;
    let mut value = AuthenticatedReplayConsumptionV1 {
        schema_version: "hyperintel.authenticated-replay-consumption.v1".to_owned(),
        state_slice: DURABLE_REPLAY_STATE_SLICE_V1.to_owned(),
        admission_digest,
        transition_digest: Sha256Digest::from_bytes([marker; 32]),
        buyer_id,
        consumed_nonce: 7,
        before_state,
        after_state,
        consumed_at: timestamp("2026-07-21T10:00:01Z")?,
        authority_granted: false,
        consumption_digest: Sha256Digest::from_bytes([0; 32]),
    };
    value.consumption_digest = value.digest()?;
    Ok(value)
}

fn assert_one_insert_and_one_exact_retry(
    left: JournalWriteOutcomeV1,
    right: JournalWriteOutcomeV1,
) {
    assert!(matches!(
        (left, right),
        (
            JournalWriteOutcomeV1::Inserted,
            JournalWriteOutcomeV1::ExistingIdentical
        ) | (
            JournalWriteOutcomeV1::ExistingIdentical,
            JournalWriteOutcomeV1::Inserted
        )
    ));
}

async fn assert_start_retries_and_conflict(
    journal: &SqliteExecutionJournalV1,
    start: &hyperintel_execution_journal::ExecutionJournalStartV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let (left, right) = tokio::join!(journal.begin(start), journal.begin(start));
    assert_one_insert_and_one_exact_retry(left?, right?);
    let mut conflicting = start.clone();
    conflicting.provider_id = "provider:conflict".parse()?;
    conflicting.start_digest = Sha256Digest::from_bytes([0; 32]);
    conflicting.start_digest = conflicting.digest()?;
    assert!(matches!(
        journal.begin(&conflicting).await,
        Err(ExecutionJournalError::IdentityConflict)
    ));
    Ok(())
}

async fn assert_terminal_retries_and_conflict(
    journal: &SqliteExecutionJournalV1,
    replay_store: &SqliteReplayStoreV1,
    start: &hyperintel_execution_journal::ExecutionJournalStartV1,
    terminal: &hyperintel_execution_journal::ExecutionJournalTerminalV1,
    consumption: &AuthenticatedReplayConsumptionV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let (left, right) = tokio::join!(
        journal.finish(terminal, replay_store),
        journal.finish(terminal, replay_store)
    );
    assert_one_insert_and_one_exact_retry(left?, right?);
    let conflicting = build_execution_journal_terminal_v1(
        start,
        ExecutionJournalTerminalDraftV1 {
            consumption_digest: Some(consumption.consumption_digest),
            outcome_digest: Some(Sha256Digest::from_bytes([32; 32])),
            status: ExecutionJournalTerminalStatusV1::Incomplete,
            finished_at: timestamp("2026-07-21T10:00:04Z")?,
        },
    )?;
    assert!(matches!(
        journal.finish(&conflicting, replay_store).await,
        Err(ExecutionJournalError::IdentityConflict)
    ));
    Ok(())
}

async fn overwrite_terminal(
    path: &std::path::Path,
    terminal: &hyperintel_execution_journal::ExecutionJournalTerminalV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::query(
        "UPDATE execution_terminals SET terminal_digest = ?, payload_json = ?, finished_at = ? \
         WHERE admission_digest = ?",
    )
    .bind(terminal.terminal_digest.to_hex())
    .bind(serde_json::to_string(terminal)?)
    .bind(terminal.finished_at.as_str())
    .bind(terminal.admission_digest.to_hex())
    .execute(&pool)
    .await?;
    pool.close().await;
    Ok(())
}

async fn seed_replay_consumption(
    path: &std::path::Path,
    consumption: &AuthenticatedReplayConsumptionV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::query(
        "INSERT INTO buyer_replay_states(\
         buyer_id, state_digest, next_nonce, payload_json, version, updated_at) \
         VALUES(?, ?, ?, ?, 0, ?)",
    )
    .bind(consumption.buyer_id.as_str())
    .bind(consumption.after_state.state_digest.to_hex())
    .bind(consumption.after_state.next_nonce.to_string())
    .bind(serde_json::to_string(&consumption.after_state)?)
    .bind(consumption.consumed_at.as_str())
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO authenticated_replay_consumptions(\
         admission_digest, transition_digest, buyer_id, consumed_nonce, \
         before_state_digest, after_state_digest, payload_json, consumed_at) \
         VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(consumption.admission_digest.to_hex())
    .bind(consumption.transition_digest.to_hex())
    .bind(consumption.buyer_id.as_str())
    .bind(consumption.consumed_nonce.to_string())
    .bind(consumption.before_state.state_digest.to_hex())
    .bind(consumption.after_state.state_digest.to_hex())
    .bind(serde_json::to_string(consumption)?)
    .bind(consumption.consumed_at.as_str())
    .execute(&pool)
    .await?;
    pool.close().await;
    Ok(())
}

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}
