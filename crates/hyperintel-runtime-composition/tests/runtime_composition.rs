//! Exact hermetic runtime-composition contracts built only from public APIs and fixture JSON.

mod contracts {
    use std::{cell::Cell, collections::BTreeMap};

    use data_encoding::HEXLOWER;
    use ed25519_dalek::{Signer as _, SigningKey};
    use hyperintel_assurance::{
        AssuranceRootClassV1, RegisteredVerificationKeyV1, VerificationKeyRegistry,
    };
    use hyperintel_core::{InferenceRequestPolicyV1, ProtocolId, Sha256Digest, UtcTimestamp};
    use hyperintel_execution::{
        CancellationSignalV1, ChatCompletionMessageV1, ChatCompletionRequestV1, ChatMessageRoleV1,
        LocalExecutionContextV1, ProviderAgentErrorV1, ProviderExecutionPlanV1,
        ProviderReceiptSignerErrorV1, ProviderReceiptSignerV1, ProviderStreamFrameV1,
        chat_completion_request_digest_v1,
    };

    use hyperintel_execution_journal::SqliteExecutionJournalV1;
    use hyperintel_market::{
        AuthenticatedExecutionAdmissionV1, AuthenticatedExecutionContextV1, BuyerPolicyStatementV1,
        ProviderExecutionReceiptV1, ProviderExecutionStatementV1,
        ReliabilityArtifactVerifiedRoutingContextV1, ReplayProtectedMarketEvidenceBundleV4,
        ReplayStateTransitionV1, SignedInferenceRequestPolicyV1, SignedReliabilityArtifactV1,
        build_authenticated_execution_admission_v1, derive_replay_state_transition_v1,
        evaluate_reliability_artifact_verified_route_v1,
        verify_replay_protected_market_evidence_bundle_v4,
    };
    use hyperintel_persistence::{InitializeReplayStateOutcomeV1, SqliteReplayStoreV1};
    use hyperintel_provider_agent::{
        ED25519_ALGORITHM_V1, PROVIDER_AGENT_KEY_SCHEMA_V1,
        PROVIDER_AGENT_READINESS_CLAIMS_SCHEMA_V1, PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1,
        PROVIDER_AGENT_SERVICE_MANIFEST_CLAIMS_SCHEMA_V1, ProviderAgentBindingV1,
        ProviderAgentExecutionContextV1, ProviderAgentKeyPurposeV1, ProviderAgentKeyRegistryV1,
        ProviderAgentReadinessClaimsV1, ProviderAgentReadinessStatusV1,
        ProviderAgentRevocationCheckpointV1, ProviderAgentRevocationSnapshotV1,
        ProviderAgentServiceManifestClaimsV1, ProviderTextStreamInterfaceV1,
        RegisteredProviderAgentKeyV1, SIGNED_PROVIDER_AGENT_READINESS_SCHEMA_V1,
        SIGNED_PROVIDER_AGENT_REVOCATION_SNAPSHOT_SCHEMA_V1,
        SIGNED_PROVIDER_AGENT_SERVICE_MANIFEST_SCHEMA_V1, SignedProviderAgentReadinessV1,
        SignedProviderAgentRevocationSnapshotV1, SignedProviderAgentServiceManifestV1,
    };
    use hyperintel_runtime_composition::{
        HermeticOperationBindingV1, HermeticRuntimeCompositionContextV1, SecretUsingProviderV1,
        execute_hermetic_runtime_composition_v1, hermetic_operation_context_digest_v1,
    };
    use hyperintel_runtime_config::{
        LocalPersistenceConfigurationV1, LocalSqliteConfigurationV1, RuntimeConfigurationV1,
        RuntimeProfileV1, SecretReferenceV1,
    };
    use hyperintel_secret_provider::{
        InjectedSecretResolverErrorV1, InjectedSecretResolverV1,
        OPERATOR_SECRET_AUTHORIZATION_CLAIMS_SCHEMA_V1, OPERATOR_SECRET_KEY_SCHEMA_V1,
        OPERATOR_SECRET_REVOCATION_STATE_SCHEMA_V1, OperatorSecretAuthorizationClaimsV1,
        OperatorSecretAuthorizationExpectationV1, OperatorSecretAuthorizationVerificationContextV1,
        OperatorSecretKeyPurposeV1, OperatorSecretKeyRegistryV1,
        OperatorSecretRevocationCheckpointV1, OperatorSecretRevocationStateV1,
        RegisteredOperatorSecretKeyV1, ResolvedSecretCandidateV1,
        SIGNED_OPERATOR_SECRET_AUTHORIZATION_SCHEMA_V1, SecretPurposeV1, SecretResolutionRequestV1,
        SignedOperatorSecretAuthorizationV1, operator_secret_revocation_state_digest_v1,
        secret_reference_digest_v1,
    };
    use hyperintel_tenant_iam::{
        ED25519_ALGORITHM_V1 as IAM_ED25519, RegisteredTenantIamKeyV1,
        SIGNED_TENANT_ACCESS_GRANT_SCHEMA_V1, SIGNED_TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1,
        SignedTenantAccessGrantV1, SignedTenantRevocationSnapshotV1,
        TENANT_ACCESS_GRANT_CLAIMS_SCHEMA_V1, TENANT_IAM_KEY_SCHEMA_V1,
        TENANT_IAM_SNAPSHOT_SCHEMA_V1, TENANT_RESOURCE_BINDING_SCHEMA_V1,
        TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1, TenantAccessExpectationV1, TenantAccessGrantClaimsV1,
        TenantAccessVerificationContextV1, TenantIamActionV1, TenantIamKeyPurposeV1,
        TenantIamKeyRegistryV1, TenantIamSnapshotV1, TenantKeyRevocationV1, TenantLifecycleV1,
        TenantPrincipalKindV1, TenantPrincipalV1, TenantResourceBindingV1, TenantResourceKindV1,
        TenantResourcePurposeV1, TenantRevocationCheckpointV1, TenantRevocationSnapshotV1,
        TenantRevocationVerificationContextV1, TenantRoleBindingV1, TenantRoleV1, TenantV1,
        ValidatedTenantIamSnapshotV1, ValidatedTenantRevocationSnapshotV1,
        verify_tenant_revocation_snapshot_v1,
    };
    use serde::Deserialize;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct V4Fixture {
        assurance_blobs: BTreeMap<Sha256Digest, Vec<u8>>,
        bundle: ReplayProtectedMarketEvidenceBundleV4,
        reliability_artifacts: BTreeMap<Sha256Digest, SignedReliabilityArtifactV1>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct TrustedKeysFixture {
        trusted_verification_keys: Vec<RegisteredVerificationKeyV1>,
    }

    struct ExecutionCase {
        policy: InferenceRequestPolicyV1,
        signed_policy: SignedInferenceRequestPolicyV1,
        replay_transition: ReplayStateTransitionV1,
        admission: AuthenticatedExecutionAdmissionV1,
    }

    struct ControlFixture {
        registry: ProviderAgentKeyRegistryV1,
        binding: ProviderAgentBindingV1,
        manifest: SignedProviderAgentServiceManifestV1,
        readiness: SignedProviderAgentReadinessV1,
        revocation: SignedProviderAgentRevocationSnapshotV1,
        checkpoint: ProviderAgentRevocationCheckpointV1,
    }

    struct ReceiptSigner {
        provider_id: ProtocolId,
        key_id: ProtocolId,
        key: SigningKey,
        calls: Cell<u64>,
    }

    impl ProviderReceiptSignerV1 for ReceiptSigner {
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
            self.calls.set(self.calls.get() + 1);
            sign_receipt(receipt, admission, self)
        }
    }
    const SYNTHETIC_SECRET: &[u8] = b"synthetic-composition-secret";

    struct TenantFixture {
        iam: ValidatedTenantIamSnapshotV1,
        keys: TenantIamKeyRegistryV1,
        revocations: ValidatedTenantRevocationSnapshotV1,
        checkpoint: TenantRevocationCheckpointV1,
        grant: SignedTenantAccessGrantV1,
        expectation: TenantAccessExpectationV1,
    }

    struct SecretFixture {
        reference: SecretReferenceV1,
        signed: SignedOperatorSecretAuthorizationV1,
        registry: OperatorSecretKeyRegistryV1,
        revocations: OperatorSecretRevocationStateV1,
        checkpoint: OperatorSecretRevocationCheckpointV1,
        operator_id: ProtocolId,
    }

    struct Resolver {
        reference: SecretReferenceV1,
        calls: u64,
    }

    impl InjectedSecretResolverV1 for Resolver {
        fn provider_id(&self) -> &ProtocolId {
            self.reference.provider_id()
        }
        fn resolve(
            &mut self,
            _request: &SecretResolutionRequestV1,
        ) -> Result<ResolvedSecretCandidateV1, InjectedSecretResolverErrorV1> {
            self.calls += 1;
            ResolvedSecretCandidateV1::new(
                self.reference.provider_id().clone(),
                self.reference.handle_id().clone(),
                self.reference.version_digest(),
                SecretPurposeV1::ProviderApiAuthentication,
                SYNTHETIC_SECRET.to_vec(),
            )
            .map_err(|_| InjectedSecretResolverErrorV1::Unavailable)
        }
    }

    struct SecretProvider {
        provider_id: ProtocolId,
        plan: Option<Result<ProviderExecutionPlanV1, ProviderAgentErrorV1>>,
        calls: u64,
        saw_secret: bool,
    }

    impl SecretUsingProviderV1 for SecretProvider {
        fn provider_id(&self) -> &ProtocolId {
            &self.provider_id
        }
        fn execute_with_secret(
            &mut self,
            secret: &[u8],
            _request: &ChatCompletionRequestV1,
            _admission: &AuthenticatedExecutionAdmissionV1,
        ) -> Result<ProviderExecutionPlanV1, ProviderAgentErrorV1> {
            self.calls += 1;
            self.saw_secret = secret == SYNTHETIC_SECRET;
            self.plan
                .take()
                .unwrap_or(Err(ProviderAgentErrorV1::ScriptExhausted))
        }
    }

    fn receipt_plan(
        provider_id: &str,
        fixture: &V4Fixture,
    ) -> Result<ProviderExecutionPlanV1, Box<dyn std::error::Error>> {
        Ok(ProviderExecutionPlanV1 {
            provider_id: provider_id.parse()?,
            started_at: timestamp("2026-07-20T12:00:01Z")?,
            frames: vec![ProviderStreamFrameV1 {
                emitted_at: timestamp("2026-07-20T12:00:02Z")?,
                content: "composition response".to_owned(),
                output_token_delta: 1,
            }],
            completed_at: timestamp("2026-07-20T12:00:03Z")?,
            input_tokens: 2,
            runtime_image_digest: fixture.bundle.service.runtime_image_digest,
        })
    }

    #[tokio::test]
    async fn contract_exact_composition_resolves_executes_journals_reconciles_and_redacts()
    -> TestResult {
        let (fixture, registry, buyer_key) = execution_environment()?;
        let request = request(&fixture);
        let case = execution_case(&fixture, &registry, &buyer_key, &request)?;
        let authenticated = authenticated_context(&fixture, &case, &registry);
        let store = initialized_store(&fixture).await?;
        let journal = SqliteExecutionJournalV1::in_memory().await?;
        journal.migrate().await?;
        let runtime = runtime_configuration()?.validate()?;
        let reference = SecretReferenceV1::new(
            "fixture-secret-backend".parse()?,
            "provider-api-handle".parse()?,
            Sha256Digest::from_bytes([44; 32]),
        )?;
        let mut control = control_fixture(&fixture, &case)?;
        rebind_control(
            &mut control,
            runtime.configuration_digest(),
            secret_reference_digest_v1(&reference)?,
        )?;
        let operation = operation_binding(&runtime, &case, &request, &control, &reference)?;
        let operation_digest = hermetic_operation_context_digest_v1(&operation)?;
        let tenant = tenant_fixture(&case, runtime.configuration_digest(), operation_digest)?;
        let secret = secret_fixture(
            &control,
            &reference,
            runtime.configuration_digest(),
            operation_digest,
        )?;
        let cancellation = CancellationSignalV1::default();
        let tenant_context = TenantAccessVerificationContextV1 {
            iam_snapshot: &tenant.iam,
            key_registry: &tenant.keys,
            revocation_snapshot: &tenant.revocations,
            revocation_checkpoint: &tenant.checkpoint,
            expectation: &tenant.expectation,
            evaluated_at: &case.admission.admitted_at,
        };
        let secret_expectation = OperatorSecretAuthorizationExpectationV1 {
            operator_id: &secret.operator_id,
            runtime_configuration_digest: runtime.configuration_digest(),
            consumer_id: &control.binding.agent_instance_id,
            selected_provider_id: &case.admission.provider_id,
            secret_reference: &secret.reference,
            purpose: SecretPurposeV1::ProviderApiAuthentication,
            operation_context_digest: operation_digest,
            maximum_secret_bytes: 64,
        };
        let secret_context = OperatorSecretAuthorizationVerificationContextV1 {
            key_registry: &secret.registry,
            revocation_state: &secret.revocations,
            revocation_checkpoint: &secret.checkpoint,
            expectation: &secret_expectation,
            evaluated_at: &case.admission.admitted_at,
        };
        let provider_context = provider_context(
            &store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        );
        let mut provider = SecretProvider {
            provider_id: "provider:alpha".parse()?,
            plan: Some(Ok(receipt_plan("provider:alpha", &fixture)?)),
            calls: 0,
            saw_secret: false,
        };
        let signer = receipt_signer()?;
        let mut resolver = Resolver {
            reference: reference.clone(),
            calls: 0,
        };
        let report = execute_hermetic_runtime_composition_v1(
            HermeticRuntimeCompositionContextV1 {
                runtime_configuration: &runtime,
                tenant_access_grant: &tenant.grant,
                tenant_access_context: tenant_context,
                secret_authorization: &secret.signed,
                secret_authorization_context: secret_context,
                provider_agent_context: provider_context,
                journal: &journal,
            },
            &mut provider,
            &signer,
            &mut resolver,
        )
        .await?;
        assert_eq!(resolver.calls, 1);
        assert_eq!(provider.calls, 1);
        assert!(provider.saw_secret);
        assert_eq!(signer.calls.get(), 1);
        assert_eq!(store.consumption_count().await?, 1);
        assert!(!report.authority_granted());
        assert!(report.verify_digest()?);
        let debug = format!("{report:?}");
        assert!(!debug.contains("synthetic-composition-secret"));
        assert!(debug.contains("REDACTED"));
        Ok(())
    }

    #[tokio::test]
    async fn contract_principal_and_operation_mismatch_fail_before_secret_and_economics()
    -> TestResult {
        let (fixture, registry, buyer_key) = execution_environment()?;
        let request = request(&fixture);
        let case = execution_case(&fixture, &registry, &buyer_key, &request)?;
        let authenticated = authenticated_context(&fixture, &case, &registry);
        let store = initialized_store(&fixture).await?;
        let journal = SqliteExecutionJournalV1::in_memory().await?;
        journal.migrate().await?;
        let runtime = runtime_configuration()?.validate()?;
        let reference = SecretReferenceV1::new(
            "fixture-secret-backend".parse()?,
            "provider-api-handle".parse()?,
            Sha256Digest::from_bytes([44; 32]),
        )?;
        let mut control = control_fixture(&fixture, &case)?;
        rebind_control(
            &mut control,
            runtime.configuration_digest(),
            secret_reference_digest_v1(&reference)?,
        )?;
        let operation = operation_binding(&runtime, &case, &request, &control, &reference)?;
        let operation_digest = hermetic_operation_context_digest_v1(&operation)?;
        let mut tenant = tenant_fixture(&case, runtime.configuration_digest(), operation_digest)?;
        tenant.expectation.operation_context_digest = Sha256Digest::from_bytes([99; 32]);
        let secret = secret_fixture(
            &control,
            &reference,
            runtime.configuration_digest(),
            operation_digest,
        )?;
        let cancellation = CancellationSignalV1::default();
        let tenant_context = TenantAccessVerificationContextV1 {
            iam_snapshot: &tenant.iam,
            key_registry: &tenant.keys,
            revocation_snapshot: &tenant.revocations,
            revocation_checkpoint: &tenant.checkpoint,
            expectation: &tenant.expectation,
            evaluated_at: &case.admission.admitted_at,
        };
        let secret_expectation = OperatorSecretAuthorizationExpectationV1 {
            operator_id: &secret.operator_id,
            runtime_configuration_digest: runtime.configuration_digest(),
            consumer_id: &control.binding.agent_instance_id,
            selected_provider_id: &case.admission.provider_id,
            secret_reference: &secret.reference,
            purpose: SecretPurposeV1::ProviderApiAuthentication,
            operation_context_digest: operation_digest,
            maximum_secret_bytes: 64,
        };
        let secret_context = OperatorSecretAuthorizationVerificationContextV1 {
            key_registry: &secret.registry,
            revocation_state: &secret.revocations,
            revocation_checkpoint: &secret.checkpoint,
            expectation: &secret_expectation,
            evaluated_at: &case.admission.admitted_at,
        };
        let provider_context = provider_context(
            &store,
            &case,
            &authenticated,
            &registry,
            &request,
            &cancellation,
            &control,
        );
        let mut provider = SecretProvider {
            provider_id: "provider:alpha".parse()?,
            plan: Some(Ok(receipt_plan("provider:alpha", &fixture)?)),
            calls: 0,
            saw_secret: false,
        };
        let signer = receipt_signer()?;
        let mut resolver = Resolver {
            reference: reference.clone(),
            calls: 0,
        };
        let denied = execute_hermetic_runtime_composition_v1(
            HermeticRuntimeCompositionContextV1 {
                runtime_configuration: &runtime,
                tenant_access_grant: &tenant.grant,
                tenant_access_context: tenant_context,
                secret_authorization: &secret.signed,
                secret_authorization_context: secret_context,
                provider_agent_context: provider_context,
                journal: &journal,
            },
            &mut provider,
            &signer,
            &mut resolver,
        )
        .await;
        assert!(denied.is_err());
        assert_eq!(resolver.calls, 0);
        assert_eq!(provider.calls, 0);
        assert_eq!(signer.calls.get(), 0);
        assert_eq!(store.consumption_count().await?, 0);
        Ok(())
    }

    fn runtime_configuration() -> Result<RuntimeConfigurationV1, Box<dyn std::error::Error>> {
        Ok(RuntimeConfigurationV1 {
            schema_version: "hyperintel.runtime-configuration.v1".to_owned(),
            runtime_id: "runtime:composition".parse()?,
            profile: RuntimeProfileV1::LocalHermetic,
            local_persistence: LocalPersistenceConfigurationV1 {
                replay_database: LocalSqliteConfigurationV1 {
                    path: "/tmp/hyperintel-composition-replay.sqlite".to_owned(),
                    max_connections: 1,
                },
                execution_journal_database: LocalSqliteConfigurationV1 {
                    path: "/tmp/hyperintel-composition-journal.sqlite".to_owned(),
                    max_connections: 1,
                },
            },
        })
    }

    fn operation_binding(
        runtime: &hyperintel_runtime_config::ValidatedRuntimeConfigurationV1,
        case: &ExecutionCase,
        request: &ChatCompletionRequestV1,
        control: &ControlFixture,
        reference: &SecretReferenceV1,
    ) -> Result<HermeticOperationBindingV1, Box<dyn std::error::Error>> {
        Ok(HermeticOperationBindingV1 {
            configuration_digest: runtime.configuration_digest(),
            runtime_id: runtime.document().runtime_id.clone(),
            tenant_id: "tenant:alpha".parse()?,
            principal_id: case.admission.buyer_id.clone(),
            audience: "gateway:local".parse()?,
            admission_digest: case.admission.admission_digest,
            request_digest: chat_completion_request_digest_v1(request)?,
            prompt_digest: case.admission.prompt_digest,
            route_decision_digest: case.admission.route_decision_digest,
            provider_id: case.admission.provider_id.clone(),
            offer_digest: case.admission.offer_digest,
            provider_agent_binding_digest: control.binding.digest()?,
            agent_instance_id: control.binding.agent_instance_id.clone(),
            receipt_key_id: control.binding.receipt_key_id.clone(),
            secret_reference_digest: secret_reference_digest_v1(reference)?,
            secret_purpose: SecretPurposeV1::ProviderApiAuthentication,
            cancellation_requested_at: None,
            evaluated_at: case.admission.admitted_at.clone(),
        })
    }

    fn rebind_control(
        control: &mut ControlFixture,
        runtime: Sha256Digest,
        reference: Sha256Digest,
    ) -> TestResult {
        control.binding.runtime_configuration_digest = runtime;
        control.binding.secret_reference_digest = reference;
        control.manifest.claims.binding = control.binding.clone();
        control.manifest.signature_hex = HEXLOWER.encode(
            &SigningKey::from_bytes(&[91; 32])
                .sign(&control.manifest.signing_message()?)
                .to_bytes(),
        );
        control.readiness.claims.binding = control.binding.clone();
        control.readiness.claims.service_manifest_digest = control.manifest.digest()?;
        resign_readiness(&mut control.readiness, &SigningKey::from_bytes(&[92; 32]))?;
        control.revocation.snapshot.runtime_configuration_digest = runtime;
        control.revocation.signature_hex = HEXLOWER.encode(
            &SigningKey::from_bytes(&[93; 32])
                .sign(&control.revocation.signing_message()?)
                .to_bytes(),
        );
        control.checkpoint.runtime_configuration_digest = runtime;
        control.checkpoint.binding_digest = control.binding.digest()?;
        control.checkpoint.manifest_digest_at_minimum_sequence = control.manifest.digest()?;
        control.checkpoint.readiness_digest_at_minimum_sequence = control.readiness.digest()?;
        control.checkpoint.revocation_digest_at_minimum_sequence = control.revocation.digest()?;
        Ok(())
    }

    fn tenant_fixture(
        case: &ExecutionCase,
        runtime: Sha256Digest,
        operation: Sha256Digest,
    ) -> Result<TenantFixture, Box<dyn std::error::Error>> {
        let access_key = SigningKey::from_bytes(&[7; 32]);
        let revocation_key = SigningKey::from_bytes(&[8; 32]);
        let iam = tenant_iam_snapshot(case)?;
        let mut keys = TenantIamKeyRegistryV1::default();
        keys.register(tenant_key(
            &access_key,
            TenantIamKeyPurposeV1::AccessGrant,
            "key:tenant-access",
        )?)?;
        keys.register(tenant_key(
            &revocation_key,
            TenantIamKeyPurposeV1::RevocationSnapshot,
            "key:tenant-revocation",
        )?)?;
        let (revocations, checkpoint) = tenant_revocations(case, runtime, &keys, &revocation_key)?;
        let resource = TenantResourceBindingV1 {
            schema_version: TENANT_RESOURCE_BINDING_SCHEMA_V1.to_owned(),
            tenant_id: "tenant:alpha".parse()?,
            resource_kind: TenantResourceKindV1::InferenceRequest,
            purpose: TenantResourcePurposeV1::Submit,
            resource_digest: case.admission.admission_digest,
            owner_principal_id: Some(case.admission.buyer_id.clone()),
        };
        let mut grant = SignedTenantAccessGrantV1 {
            schema_version: SIGNED_TENANT_ACCESS_GRANT_SCHEMA_V1.to_owned(),
            claims: TenantAccessGrantClaimsV1 {
                schema_version: TENANT_ACCESS_GRANT_CLAIMS_SCHEMA_V1.to_owned(),
                sequence: 1,
                tenant_id: "tenant:alpha".parse()?,
                principal_id: case.admission.buyer_id.clone(),
                issuer_id: "issuer:alpha".parse()?,
                audience: "gateway:local".parse()?,
                runtime_configuration_digest: runtime,
                iam_snapshot_digest: iam.snapshot_digest(),
                action: TenantIamActionV1::SubmitInference,
                resource: resource.clone(),
                operation_context_digest: operation,
                issued_at: timestamp("2026-07-20T11:58:00Z")?,
                expires_at: timestamp("2026-07-20T12:02:00Z")?,
            },
            signer_id: "issuer:alpha".parse()?,
            key_id: "key:tenant-access".parse()?,
            algorithm: IAM_ED25519.to_owned(),
            signature_hex: String::new(),
        };
        grant.signature_hex =
            HEXLOWER.encode(&access_key.sign(&grant.signing_message()?).to_bytes());
        let expectation = TenantAccessExpectationV1 {
            tenant_id: "tenant:alpha".parse()?,
            principal_id: case.admission.buyer_id.clone(),
            issuer_id: "issuer:alpha".parse()?,
            audience: "gateway:local".parse()?,
            runtime_configuration_digest: runtime,
            iam_snapshot_digest: iam.snapshot_digest(),
            action: TenantIamActionV1::SubmitInference,
            resource,
            operation_context_digest: operation,
        };
        Ok(TenantFixture {
            iam,
            keys,
            revocations,
            checkpoint,
            grant,
            expectation,
        })
    }

    fn tenant_iam_snapshot(
        case: &ExecutionCase,
    ) -> Result<ValidatedTenantIamSnapshotV1, Box<dyn std::error::Error>> {
        Ok(TenantIamSnapshotV1 {
            schema_version: TENANT_IAM_SNAPSHOT_SCHEMA_V1.to_owned(),
            tenant: TenantV1 {
                tenant_id: "tenant:alpha".parse()?,
                iam_issuer_id: "issuer:alpha".parse()?,
                lifecycle: TenantLifecycleV1::Active,
                valid_from: timestamp("2026-07-20T11:00:00Z")?,
                valid_until: timestamp("2026-07-20T13:00:00Z")?,
                revoked_at: None,
            },
            principals: vec![TenantPrincipalV1 {
                tenant_id: "tenant:alpha".parse()?,
                principal_id: case.admission.buyer_id.clone(),
                kind: TenantPrincipalKindV1::Workload,
                lifecycle: TenantLifecycleV1::Active,
                valid_from: timestamp("2026-07-20T11:00:00Z")?,
                valid_until: timestamp("2026-07-20T13:00:00Z")?,
                revoked_at: None,
            }],
            role_bindings: vec![TenantRoleBindingV1 {
                tenant_id: "tenant:alpha".parse()?,
                principal_id: case.admission.buyer_id.clone(),
                role: TenantRoleV1::InferenceCaller,
                valid_from: timestamp("2026-07-20T11:00:00Z")?,
                valid_until: timestamp("2026-07-20T13:00:00Z")?,
                revoked_at: None,
            }],
        }
        .validate()?)
    }

    fn tenant_revocations(
        case: &ExecutionCase,
        runtime: Sha256Digest,
        keys: &TenantIamKeyRegistryV1,
        signing_key: &SigningKey,
    ) -> Result<
        (
            ValidatedTenantRevocationSnapshotV1,
            TenantRevocationCheckpointV1,
        ),
        Box<dyn std::error::Error>,
    > {
        let mut signed = SignedTenantRevocationSnapshotV1 {
            schema_version: SIGNED_TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
            snapshot: TenantRevocationSnapshotV1 {
                schema_version: TENANT_REVOCATION_SNAPSHOT_SCHEMA_V1.to_owned(),
                tenant_id: "tenant:alpha".parse()?,
                issuer_id: "issuer:alpha".parse()?,
                audience: "gateway:local".parse()?,
                runtime_configuration_digest: runtime,
                sequence: 1,
                issued_at: timestamp("2026-07-20T11:58:00Z")?,
                effective_at: timestamp("2026-07-20T11:58:00Z")?,
                expires_at: timestamp("2026-07-20T12:02:00Z")?,
                grant_revocations: vec![],
                key_revocations: Vec::<TenantKeyRevocationV1>::new(),
            },
            signer_id: "issuer:alpha".parse()?,
            key_id: "key:tenant-revocation".parse()?,
            algorithm: IAM_ED25519.to_owned(),
            signature_hex: String::new(),
        };
        signed.signature_hex =
            HEXLOWER.encode(&signing_key.sign(&signed.signing_message()?).to_bytes());
        let checkpoint = TenantRevocationCheckpointV1 {
            tenant_id: "tenant:alpha".parse()?,
            issuer_id: "issuer:alpha".parse()?,
            audience: "gateway:local".parse()?,
            runtime_configuration_digest: runtime,
            minimum_sequence: 1,
            digest_at_minimum_sequence: signed.digest()?,
        };
        let validated = verify_tenant_revocation_snapshot_v1(
            &signed,
            &TenantRevocationVerificationContextV1 {
                key_registry: keys,
                checkpoint: &checkpoint,
                evaluated_at: &case.admission.admitted_at,
            },
        )?;
        Ok((validated, checkpoint))
    }

    fn tenant_key(
        key: &SigningKey,
        purpose: TenantIamKeyPurposeV1,
        key_id: &str,
    ) -> Result<RegisteredTenantIamKeyV1, Box<dyn std::error::Error>> {
        Ok(RegisteredTenantIamKeyV1 {
            schema_version: TENANT_IAM_KEY_SCHEMA_V1.to_owned(),
            tenant_id: "tenant:alpha".parse()?,
            issuer_id: "issuer:alpha".parse()?,
            key_id: key_id.parse()?,
            purpose,
            public_key_hex: HEXLOWER.encode(key.verifying_key().as_bytes()),
            valid_from: timestamp("2026-07-20T11:00:00Z")?,
            valid_until: timestamp("2026-07-20T13:00:00Z")?,
            revoked_at: None,
        })
    }

    fn secret_fixture(
        control: &ControlFixture,
        reference: &SecretReferenceV1,
        runtime: Sha256Digest,
        operation: Sha256Digest,
    ) -> Result<SecretFixture, Box<dyn std::error::Error>> {
        let key = SigningKey::from_bytes(&[65; 32]);
        let operator_id: ProtocolId = "operator:secrets".parse()?;
        let key_id: ProtocolId = "key:operator-secret".parse()?;
        let mut registry = OperatorSecretKeyRegistryV1::default();
        registry.register(RegisteredOperatorSecretKeyV1 {
            schema_version: OPERATOR_SECRET_KEY_SCHEMA_V1.to_owned(),
            operator_id: operator_id.clone(),
            key_id: key_id.clone(),
            purpose: OperatorSecretKeyPurposeV1::SecretResolutionAuthorization,
            public_key_hex: HEXLOWER.encode(key.verifying_key().as_bytes()),
            valid_from: timestamp("2026-07-20T11:00:00Z")?,
            valid_until: timestamp("2026-07-20T13:00:00Z")?,
            revoked_at: None,
        })?;
        let mut signed = SignedOperatorSecretAuthorizationV1 {
            schema_version: SIGNED_OPERATOR_SECRET_AUTHORIZATION_SCHEMA_V1.to_owned(),
            claims: OperatorSecretAuthorizationClaimsV1 {
                schema_version: OPERATOR_SECRET_AUTHORIZATION_CLAIMS_SCHEMA_V1.to_owned(),
                sequence: 1,
                operator_id: operator_id.clone(),
                runtime_configuration_digest: runtime,
                consumer_id: control.binding.agent_instance_id.clone(),
                selected_provider_id: control.binding.provider_id.clone(),
                secret_reference_digest: secret_reference_digest_v1(reference)?,
                purpose: SecretPurposeV1::ProviderApiAuthentication,
                operation_context_digest: operation,
                maximum_secret_bytes: 64,
                issued_at: timestamp("2026-07-20T11:58:00Z")?,
                expires_at: timestamp("2026-07-20T12:02:00Z")?,
            },
            signer_id: operator_id.clone(),
            key_id: key_id.clone(),
            algorithm: "ed25519".to_owned(),
            signature_hex: String::new(),
        };
        signed.signature_hex = HEXLOWER.encode(&key.sign(&signed.signing_message()?).to_bytes());
        let revocations = OperatorSecretRevocationStateV1 {
            schema_version: OPERATOR_SECRET_REVOCATION_STATE_SCHEMA_V1.to_owned(),
            operator_id: operator_id.clone(),
            runtime_configuration_digest: runtime,
            sequence: 1,
            issued_at: timestamp("2026-07-20T11:58:00Z")?,
            effective_at: timestamp("2026-07-20T11:58:00Z")?,
            expires_at: timestamp("2026-07-20T12:02:00Z")?,
            authorization_revocations: vec![],
            key_revocations: vec![],
        };
        let checkpoint = OperatorSecretRevocationCheckpointV1 {
            operator_id: operator_id.clone(),
            runtime_configuration_digest: runtime,
            minimum_sequence: 1,
            digest_at_minimum_sequence: operator_secret_revocation_state_digest_v1(&revocations)?,
        };
        Ok(SecretFixture {
            reference: reference.clone(),
            signed,
            registry,
            revocations,
            checkpoint,
            operator_id,
        })
    }

    fn execution_environment()
    -> Result<(V4Fixture, VerificationKeyRegistry, SigningKey), Box<dyn std::error::Error>> {
        let fixture: V4Fixture = serde_json::from_slice(include_bytes!(
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
        register_market_key(
            &mut registry,
            "buyer:demo",
            "key:buyer-policy-1",
            AssuranceRootClassV1::BuyerPolicy,
            &buyer_key,
        )?;
        register_market_key(
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

    fn execution_case(
        fixture: &V4Fixture,
        registry: &VerificationKeyRegistry,
        buyer_key: &SigningKey,
        request: &ChatCompletionRequestV1,
    ) -> Result<ExecutionCase, Box<dyn std::error::Error>> {
        let mut policy = fixture.bundle.policy.clone();
        policy.request_id = "request:runtime-composition-alpha".parse()?;
        policy.allowed_providers.clear();
        policy.allowed_providers.insert("provider:alpha".parse()?);
        policy.prompt_digest = chat_completion_request_digest_v1(request)?;
        let evaluation = evaluate_reliability_artifact_verified_route_v1(&routing_context(
            fixture, &policy, registry,
        ))?;
        let replay_transition = derive_replay_state_transition_v1(
            &fixture.bundle.replay_transition.before_state,
            &policy,
            evaluation.evaluation_digest,
            &fixture.bundle.evaluated_at,
        )?;
        let signed_policy = signed_buyer_policy(&policy, buyer_key)?;
        let authenticated = AuthenticatedExecutionContextV1 {
            routing: routing_context(fixture, &policy, registry),
            replay_transition: &replay_transition,
            signed_policy: &signed_policy,
        };
        let admission = build_authenticated_execution_admission_v1(
            &authenticated,
            registry,
            timestamp("2026-07-20T12:00:00Z")?,
            timestamp("2026-07-20T12:05:00Z")?,
        )?;
        Ok(ExecutionCase {
            policy,
            signed_policy,
            replay_transition,
            admission,
        })
    }

    fn authenticated_context<'a>(
        fixture: &'a V4Fixture,
        case: &'a ExecutionCase,
        registry: &'a VerificationKeyRegistry,
    ) -> AuthenticatedExecutionContextV1<'a> {
        AuthenticatedExecutionContextV1 {
            routing: routing_context(fixture, &case.policy, registry),
            replay_transition: &case.replay_transition,
            signed_policy: &case.signed_policy,
        }
    }

    fn routing_context<'a>(
        fixture: &'a V4Fixture,
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

    fn request(fixture: &V4Fixture) -> ChatCompletionRequestV1 {
        ChatCompletionRequestV1 {
            model: fixture.bundle.service.model_version.as_str().to_owned(),
            messages: vec![ChatCompletionMessageV1 {
                role: ChatMessageRoleV1::User,
                content: "execute composition fixture".to_owned(),
            }],
            stream: true,
        }
    }

    async fn initialized_store(
        fixture: &V4Fixture,
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

    fn receipt_signer() -> Result<ReceiptSigner, Box<dyn std::error::Error>> {
        Ok(ReceiptSigner {
            provider_id: "provider:alpha".parse()?,
            key_id: "key:alpha-receipt".parse()?,
            key: SigningKey::from_bytes(&[81; 32]),
            calls: Cell::new(0),
        })
    }

    fn provider_context<'a>(
        store: &'a SqliteReplayStoreV1,
        case: &'a ExecutionCase,
        authenticated: &'a AuthenticatedExecutionContextV1<'a>,
        registry: &'a VerificationKeyRegistry,
        request: &'a ChatCompletionRequestV1,
        cancellation: &'a CancellationSignalV1,
        control: &'a ControlFixture,
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
            evaluated_at: &case.admission.admitted_at,
        }
    }

    fn sign_receipt(
        receipt: &ProviderExecutionReceiptV1,
        admission: &AuthenticatedExecutionAdmissionV1,
        signer: &ReceiptSigner,
    ) -> Result<ProviderExecutionStatementV1, ProviderReceiptSignerErrorV1> {
        let mut statement = ProviderExecutionStatementV1 {
            schema_version: "hyperintel.provider-execution-statement.v1".to_owned(),
            receipt_digest: receipt.receipt_digest,
            admission_digest: admission.admission_digest,
            issued_at: receipt.completed_at.clone(),
            expires_at: admission.deadline_at.clone(),
            sequence: 1,
            signer_id: signer.provider_id.clone(),
            key_id: signer.key_id.clone(),
            algorithm: "ed25519".to_owned(),
            signature_hex: String::new(),
        };
        let message = statement
            .signing_message()
            .map_err(|_| ProviderReceiptSignerErrorV1::Unavailable)?;
        statement.signature_hex = HEXLOWER.encode(&signer.key.sign(&message).to_bytes());
        Ok(statement)
    }

    fn signed_buyer_policy(
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
            algorithm: "ed25519".to_owned(),
            signature_hex: String::new(),
        };
        statement.signature_hex =
            HEXLOWER.encode(&key.sign(&statement.signing_message()?).to_bytes());
        Ok(SignedInferenceRequestPolicyV1 {
            schema_version: "hyperintel.signed-inference-request-policy.v1".to_owned(),
            policy: policy.clone(),
            statement,
        })
    }

    fn register_market_key(
        registry: &mut VerificationKeyRegistry,
        signer_id: &str,
        key_id: &str,
        class: AssuranceRootClassV1,
        key: &SigningKey,
    ) -> TestResult {
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

    fn control_fixture(
        fixture: &V4Fixture,
        case: &ExecutionCase,
    ) -> Result<ControlFixture, Box<dyn std::error::Error>> {
        let manifest_key = SigningKey::from_bytes(&[91; 32]);
        let readiness_key = SigningKey::from_bytes(&[92; 32]);
        let revocation_key = SigningKey::from_bytes(&[93; 32]);
        let registry = provider_agent_registry(&manifest_key, &readiness_key, &revocation_key)?;
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
        let mut manifest = unsigned_manifest(&binding)?;
        manifest.signature_hex =
            HEXLOWER.encode(&manifest_key.sign(&manifest.signing_message()?).to_bytes());
        let mut readiness = unsigned_readiness(&binding, manifest.digest()?)?;
        resign_readiness(&mut readiness, &readiness_key)?;
        let mut revocation = unsigned_revocation(&binding)?;
        revocation.signature_hex = HEXLOWER.encode(
            &revocation_key
                .sign(&revocation.signing_message()?)
                .to_bytes(),
        );
        let checkpoint = provider_checkpoint(&binding, &manifest, &readiness, &revocation)?;
        Ok(ControlFixture {
            registry,
            binding,
            manifest,
            readiness,
            revocation,
            checkpoint,
        })
    }

    fn provider_agent_registry(
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

    fn unsigned_manifest(
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

    fn unsigned_readiness(
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

    fn unsigned_revocation(
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
                assertion_revocations: vec![],
                key_revocations: vec![],
            },
            signer_id: binding.provider_id.clone(),
            key_id: "key:pa-revocation".parse()?,
            algorithm: ED25519_ALGORITHM_V1.to_owned(),
            signature_hex: String::new(),
        })
    }

    fn resign_readiness(
        readiness: &mut SignedProviderAgentReadinessV1,
        key: &SigningKey,
    ) -> TestResult {
        readiness.signature_hex =
            HEXLOWER.encode(&key.sign(&readiness.signing_message()?).to_bytes());
        Ok(())
    }

    fn provider_checkpoint(
        binding: &ProviderAgentBindingV1,
        manifest: &SignedProviderAgentServiceManifestV1,
        readiness: &SignedProviderAgentReadinessV1,
        revocation: &SignedProviderAgentRevocationSnapshotV1,
    ) -> Result<ProviderAgentRevocationCheckpointV1, Box<dyn std::error::Error>> {
        Ok(ProviderAgentRevocationCheckpointV1 {
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
        })
    }

    fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
        Ok(value.parse()?)
    }
}
