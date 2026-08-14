use super::future_authority::*;
use super::journal::*;
use super::projection::exact_attempt_index_for;
use super::projection::projection_for;
use super::selection_authority::validate_queue_derivation;
use super::*;
use nando_operator_learning::multi_source::IdentifierResultV1;
use std::path::Path;

const K1_EXACT_SCHEDULER_POLICY_SCHEMA_V1: &str = "nando.k1-exact-scheduler-policy.v1";
const K1_EXACT_MAX_NEW_FREEZES_PER_WAKE_V1: u64 = 1;
const K1_EXACT_MIN_FREEZE_INTERVAL_SECONDS_V1: u64 = 300;
const K1_EXACT_MAX_TRAILING_24H_FREEZES_V1: u64 = 48;
const K1_EXACT_MAX_READINESS_ROWS_PER_WAKE_V1: u64 = 256;
const K1_EXACT_TRAILING_WINDOW_SECONDS_V1: u64 = 86_400;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct K1ExactSchedulerPolicyV1 {
    schema: String,
    policy_root_sha256: String,
    writer_enabled: bool,
    minimum_queue_schema: String,
    minimum_freeze_schema: String,
    minimum_wire_schema: String,
    scheduler_schema: String,
    maximum_new_freezes_per_wake: u64,
    minimum_freeze_interval_seconds: u64,
    maximum_trailing_24h_freezes: u64,
    maximum_readiness_rows_per_wake: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct K1ExactWriterPolicyHealthV1 {
    pub(crate) state: &'static str,
    pub(crate) policy_root_sha256: Option<String>,
    pub(crate) minimum_queue_schema: Option<String>,
    pub(crate) minimum_freeze_schema: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct K1ExactResearchBudgetStateV1 {
    pub trailing_24h_freezes: u64,
    pub next_eligible_at_unix: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct K1ExactWakeAuthorityResultV1 {
    pub(super) status: K1ExactWakeStatusV1,
    pub(super) projection: K1SchedulerProjectionV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum K1ExactWakeSelectionV1 {
    WaitingForEvidence,
    WaitingForNovelEvidence,
    ResearchBudgetCooldown(u64),
    CandidateReady,
}

#[derive(Serialize)]
struct AuthorityBindingManifestMaterialV1<'a> {
    schema: &'static str,
    scheduler_revision: u64,
    scheduler_ledger_root_sha256: &'a str,
    registry_revision: u64,
    registry_root_sha256: &'a str,
    deficit_snapshot_root_sha256: &'a str,
    fixture_exclusion_root_sha256: &'a str,
    catalog_root_sha256: &'a str,
    queue_root_sha256: &'a str,
    candidate_root_sha256: &'a str,
    active_protocol_mode_set_root_sha256: &'a str,
    minimum_queue_schema: &'a str,
    minimum_freeze_schema: &'a str,
    minimum_wire_schema: &'a str,
    evidence_source_snapshot_root_sha256: &'a str,
    collection_checkpoint_root_sha256: &'a str,
    policy_root_sha256: &'a str,
}

const K1_SCHEDULER_AUTHORITY_READ_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) fn handle_authority_line(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    line: &str,
) -> Option<String> {
    let value: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return None,
    };
    let schema = value.get("schema").and_then(Value::as_str)?;
    if schema == K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1 {
        let result = serde_json::from_value::<K1ExactWakeAuthorityRequestV1>(value)
            .map_err(|error| format!("k1_exact_wake_request_decode:{error}"))
            .and_then(|request| exact_wake_authoritative(config, signing_key, request));
        let response = match result {
            Ok(result) => K1ExactWakeAuthorityResponseV1 {
                schema: K1_EXACT_WAKE_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
                status: Some(result.status),
                projection: Some(result.projection),
                error: String::new(),
            },
            Err(error) => K1ExactWakeAuthorityResponseV1 {
                schema: K1_EXACT_WAKE_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
                status: None,
                projection: None,
                error,
            },
        };
        return serde_json::to_string(&response).ok();
    }
    let result = match schema {
        K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1 => {
            serde_json::from_value::<K1CandidateFreezeAuthorityRequestV1>(value)
                .map_err(|error| format!("k1_candidate_freeze_request_decode:{error}"))
                .and_then(|request| {
                    append_candidate_freeze_authoritative(config, signing_key, request, None)
                })
        }
        K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V2 => {
            serde_json::from_value::<bounded_wire::K1CandidateFreezeAuthorityRequestV2>(value)
                .map_err(|error| format!("k1_candidate_freeze_v2_request_decode:{error}"))
                .and_then(bounded_wire::decode_candidate_freeze_v2)
                .and_then(|(request, scheduler_cas)| {
                    append_candidate_freeze_authoritative(
                        config,
                        signing_key,
                        request,
                        Some(&scheduler_cas),
                    )
                })
        }
        K1_SCHEDULER_APPEND_AUTHORITY_REQUEST_SCHEMA_V1 => {
            serde_json::from_value::<K1SchedulerAppendAuthorityRequestV1>(value)
                .map_err(|error| format!("k1_scheduler_append_request_decode:{error}"))
                .and_then(|request| append_payload_authoritative(config, signing_key, request))
        }
        K1_TRANSFER_SETTLEMENT_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1TransferSettlementAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_transfer_settlement_request_decode:{error}"))
        .and_then(|request| append_transfer_settlement_authoritative(config, signing_key, request)),
        K1_FUTURE_CONTRACT_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1FutureContractAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_future_contract_request_decode:{error}"))
        .and_then(|request| append_future_contract_authoritative(config, signing_key, request)),
        K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1FuturePredictionAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_future_prediction_request_decode:{error}"))
        .and_then(|request| append_future_prediction_authoritative(config, signing_key, request)),
        K1_FUTURE_PREDICTION_CENSOR_AUTHORITY_REQUEST_SCHEMA_V1 => {
            serde_json::from_value::<K1FuturePredictionCensorAuthorityRequestV1>(value)
                .map_err(|error| format!("k1_future_prediction_censor_request_decode:{error}"))
                .and_then(|request| {
                    append_future_prediction_censor_authoritative(config, signing_key, request)
                })
        }
        K1_PRE_ACTION_EVIDENCE_AUTHORITY_REQUEST_SCHEMA_V1 => {
            serde_json::from_value::<K1PreActionEvidenceAuthorityRequestV1>(value)
                .map_err(|error| format!("k1_pre_action_evidence_request_decode:{error}"))
                .and_then(|request| archive_pre_action_evidence_authoritative(config, request))
        }
        K1_FUTURE_OUTCOME_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1FutureOutcomeAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_future_outcome_request_decode:{error}"))
        .and_then(|request| append_future_outcome_authoritative(config, signing_key, request)),
        K1_EXACT_TERMINAL_AUTHORITY_REQUEST_SCHEMA_V1 => {
            serde_json::from_value::<K1ExactTerminalAuthorityRequestV1>(value)
                .map_err(|error| format!("k1_exact_terminal_request_decode:{error}"))
                .and_then(|request| exact_terminal_authoritative(config, signing_key, request))
        }
        _ => return None,
    };
    let response = match result {
        Ok(projection) => K1SchedulerAuthorityResponseV1 {
            schema: K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
            projection: Some(projection),
            error: String::new(),
        },
        Err(error) => K1SchedulerAuthorityResponseV1 {
            schema: K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
            projection: None,
            error,
        },
    };
    Some(serde_json::to_string(&response).unwrap_or_else(|error| {
        format!(
            "{{\"schema\":\"{K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1}\",\"projection\":null,\"error\":\"k1_scheduler_response_encode:{error}\"}}"
        )
    }))
}

fn exact_terminal_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1ExactTerminalAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_EXACT_TERMINAL_AUTHORITY_REQUEST_SCHEMA_V1
        || request.lane != K1SchedulerLaneV1::Epistemic
    {
        return Err("k1_exact_terminal_request_invalid".to_owned());
    }
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let projection = projection_for(&scheduler)?;
    if projection.active_candidate_freeze.is_none() {
        if projection
            .latest_terminal_verdict
            .as_ref()
            .is_some_and(|verdict| {
                verdict.candidate_freeze_root_sha256 == request.candidate_freeze_root_sha256
            })
        {
            return Ok(projection);
        }
        return Err("k1_exact_terminal_active_freeze_missing".to_owned());
    }
    let freeze = projection
        .active_candidate_freeze
        .as_ref()
        .filter(|freeze| {
            freeze.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8
                && freeze.freeze_root_sha256 == request.candidate_freeze_root_sha256
        })
        .cloned()
        .ok_or_else(|| "k1_exact_terminal_freeze_mismatch".to_owned())?;
    if projection.identification_freeze.is_some() {
        return Err("k1_exact_terminal_not_pre_future".to_owned());
    }
    let inputs =
        crate::k1_natural_scheduler_runtime::restore_exact_identifier_inputs_v1(config, &freeze)?;
    let evaluation = crate::k1_natural_scheduler_runtime::evaluate_exact_initial_identifier_v1(
        &inputs, &freeze,
    )?;
    let blocker = evaluation
        .report
        .blocker
        .as_deref()
        .ok_or_else(|| "k1_exact_terminal_identifier_not_terminal".to_owned())?;
    if !deterministic_initial_blocker_v1(blocker) {
        return Err("k1_exact_terminal_operational_or_future".to_owned());
    }
    let causal = freeze
        .identifier_causal_input_manifest
        .as_deref()
        .ok_or_else(|| "k1_exact_identifier_causal_manifest_missing".to_owned())?;
    let result = IdentifierResultV1::seal(
        causal.opportunity_root_sha256.clone(),
        &evaluation.dispositions,
        &evaluation.report,
    )
    .map_err(str::to_owned)?;
    let mut existing_diagnostic = None;
    for event in scheduler.events.iter().rev() {
        match &event.payload {
            K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(value) => {
                existing_diagnostic = Some(value.as_ref());
                break;
            }
            K1SchedulerEventPayloadV1::CandidateFreeze(_)
            | K1SchedulerEventPayloadV1::TerminalVerdict(_) => break,
            _ => {}
        }
    }
    let terminal_at_unix =
        existing_diagnostic.map_or_else(crate::unix_now, |value| value.terminal_at_unix);
    let diagnostic = TerminalDiagnosticV1::seal(
        freeze.freeze_root_sha256.clone(),
        &result,
        inputs.support.manifest_root_sha256.clone(),
        u64::try_from(inputs.support.rows.len())
            .map_err(|_| "k1_exact_terminal_support_count".to_owned())?,
        inputs.projection.projection_root_sha256.clone(),
        u64::try_from(inputs.artifacts.len())
            .map_err(|_| "k1_exact_terminal_artifact_count".to_owned())?,
        &evaluation.dispositions,
        &evaluation.report.remaining_semantic_class_roots_sha256,
        evaluation.report.state,
        blocker.to_owned(),
        terminal_at_unix,
    )
    .map_err(str::to_owned)?;
    if diagnostic.terminal_disposition != TerminalDispositionV1::DeterministicPreFuture {
        return Err("k1_exact_terminal_operational_or_future".to_owned());
    }
    complete_exact_terminal_transaction(
        config,
        signing_key,
        request.lane,
        &mut scheduler,
        &freeze,
        &evaluation.report.report_root_sha256,
        &result.identifier_result_root_sha256,
        diagnostic,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn complete_exact_terminal_transaction(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    lane: K1SchedulerLaneV1,
    scheduler: &mut K1SchedulerLedgerV1,
    freeze: &K1NaturalCandidateFreezeV1,
    identifier_report_root_sha256: &str,
    identifier_result_root_sha256: &str,
    diagnostic: TerminalDiagnosticV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if lane != K1SchedulerLaneV1::Epistemic
        || restore_anchored_scheduler_for(config, lane)? != *scheduler
    {
        return Err("k1_exact_terminal_transaction_cas_invalid".to_owned());
    }
    freeze.validate().map_err(str::to_owned)?;
    diagnostic.validate().map_err(str::to_owned)?;
    let causal = freeze
        .identifier_causal_input_manifest
        .as_deref()
        .ok_or_else(|| "k1_exact_identifier_causal_manifest_missing".to_owned())?;
    if diagnostic.terminal_disposition != TerminalDispositionV1::DeterministicPreFuture
        || diagnostic.candidate_freeze_root_sha256 != freeze.freeze_root_sha256
        || diagnostic.opportunity_root_sha256 != causal.opportunity_root_sha256
        || diagnostic.identifier_report_root_sha256 != identifier_report_root_sha256
        || diagnostic.identifier_result_root_sha256 != identifier_result_root_sha256
    {
        return Err("k1_exact_terminal_transaction_binding_invalid".to_owned());
    }
    let projection = projection_for(scheduler)?;
    if projection.active_candidate_freeze.is_none()
        && projection
            .latest_terminal_verdict
            .as_ref()
            .is_some_and(|verdict| {
                verdict.candidate_freeze_root_sha256 == freeze.freeze_root_sha256
                    && verdict
                        .evidence_roots_sha256
                        .contains(&diagnostic.terminal_diagnostic_root_sha256)
                    && verdict.blocker == diagnostic.exact_result_blocker
                    && verdict.terminal_at_unix == diagnostic.terminal_at_unix
            })
    {
        return Ok(projection);
    }
    let mut existing_diagnostic = None;
    for event in scheduler.events.iter().rev() {
        match &event.payload {
            K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(value) => {
                existing_diagnostic = Some(value.as_ref());
                break;
            }
            K1SchedulerEventPayloadV1::CandidateFreeze(_)
            | K1SchedulerEventPayloadV1::TerminalVerdict(_) => break,
            _ => {}
        }
    }
    match existing_diagnostic {
        Some(existing) if existing != &diagnostic => {
            return Err("k1_exact_terminal_diagnostic_conflict".to_owned());
        }
        Some(_) => {}
        None => {
            append_and_persist(
                config,
                lane,
                signing_key,
                scheduler,
                K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(Box::new(diagnostic.clone())),
            )?;
        }
    }
    let verdict = K1GenerationTerminalVerdictV1::seal(
        freeze.freeze_root_sha256.clone(),
        None,
        Vec::new(),
        vec![
            freeze.freeze_root_sha256.clone(),
            identifier_report_root_sha256.to_owned(),
            identifier_result_root_sha256.to_owned(),
            diagnostic.terminal_diagnostic_root_sha256.clone(),
        ],
        K1GenerationVerdictClassV1::AcquisitionFail,
        diagnostic.exact_result_blocker.clone(),
        diagnostic.terminal_at_unix,
        None,
    )
    .map_err(str::to_owned)?;
    append_and_persist(
        config,
        lane,
        signing_key,
        scheduler,
        K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
    )
}

pub(super) fn exact_wake_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1ExactWakeAuthorityRequestV1,
) -> Result<K1ExactWakeAuthorityResultV1, String> {
    if request.schema != K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1
        || request.lane != K1SchedulerLaneV1::Epistemic
    {
        return Err("k1_exact_wake_request_invalid".to_owned());
    }
    let sources = config
        .k1_exact_sources
        .as_ref()
        .ok_or_else(|| "k1_exact_authority_sources_not_configured".to_owned())?;
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let now_unix = crate::unix_now();
    let policy = match read_exact_scheduler_policy(&sources.scheduler_policy_path) {
        Ok(policy) => {
            validate_rollback_reader_schema(&scheduler, &policy.minimum_freeze_schema)?;
            policy
        }
        Err(error) if error == "k1_exact_writer_inactive" => {
            let policy = read_exact_scheduler_policy_document(&sources.scheduler_policy_path)?;
            validate_rollback_reader_schema(&scheduler, &policy.minimum_freeze_schema)?;
            let budget = exact_research_budget_state_v1(&scheduler, &policy, now_unix)?;
            return exact_wake_result(
                &scheduler,
                K1ExactWakeDecisionV1::WriterInactive,
                error,
                None,
                budget.trailing_24h_freezes,
                budget.next_eligible_at_unix,
            );
        }
        Err(error) => return Err(error),
    };
    let budget = exact_research_budget_state_v1(&scheduler, &policy, now_unix)?;
    let scheduler_projection = projection_for(&scheduler)?;
    if scheduler.active_candidate_freeze().is_some()
        || scheduler_projection.pending_terminal_transfer.is_some()
    {
        let blocker = if scheduler_projection.pending_terminal_transfer.is_some() {
            "terminal_transfer_pending"
        } else {
            "active_generation_immutable"
        };
        return exact_wake_result(
            &scheduler,
            K1ExactWakeDecisionV1::ActiveGeneration,
            blocker,
            None,
            budget.trailing_24h_freezes,
            None,
        );
    }
    let registry = restore_anchored_ledger(config)?;
    let deficit = current_deficit_snapshot(config)?;
    validate_registry_cas(&registry, &deficit)?;
    if deficit.k1_open {
        return exact_wake_result(
            &scheduler,
            K1ExactWakeDecisionV1::K1VocabularyOpen,
            "k1_vocabulary_open",
            None,
            budget.trailing_24h_freezes,
            None,
        );
    }
    let active_protocols = crate::multi_source_live::known_epistemic_protocol_mode_roots(
        &config.response_registry_path,
        &registry,
    )?;
    let attempt_index = exact_attempt_index_for(&scheduler)?;
    let source_heads =
        crate::k1_natural_scheduler_runtime::restore_exact_durable_source_heads_v1(config)?;
    if source_heads.topology_rows == 0 || source_heads.frame_rows == 0 {
        return exact_wake_result(
            &scheduler,
            K1ExactWakeDecisionV1::WaitingForEvidence,
            "no_durable_evidence_rows",
            Some((0, 0, 0, attempt_index.legacy_unbound_terminals)),
            budget.trailing_24h_freezes,
            None,
        );
    }
    let exact = crate::k1_natural_scheduler_runtime::restore_exact_opportunity_v1(
        config,
        &deficit,
        &attempt_index,
        &active_protocols,
    )?;
    let readiness_pass_rows = exact
        .queue
        .rows
        .iter()
        .filter(|row| row.score.readiness_rank == 1)
        .count() as u64;
    if readiness_pass_rows > policy.maximum_readiness_rows_per_wake {
        return Err("k1_exact_readiness_budget_exceeded".to_owned());
    }
    let exact_counts = Some((
        readiness_pass_rows,
        exact.queue.exact_unseen_opportunities,
        exact.queue.exact_attempted_deterministic_roots,
        exact.queue.legacy_unbound_terminals,
    ));
    if exact.active_protocol_mode_set_root_sha256
        != crate::k1_natural_scheduler::duplicate_cohorts::known_epistemic_protocol_mode_set_root(
            &active_protocols,
        )?
    {
        return Err("STALE_BEFORE_FREEZE".to_owned());
    }
    match exact_wake_selection_v1(
        readiness_pass_rows,
        exact.queue.exact_unseen_opportunities,
        budget,
    ) {
        K1ExactWakeSelectionV1::WaitingForEvidence => {
            return exact_wake_result(
                &scheduler,
                K1ExactWakeDecisionV1::WaitingForEvidence,
                "no_readiness_pass_candidate",
                exact_counts,
                budget.trailing_24h_freezes,
                None,
            );
        }
        K1ExactWakeSelectionV1::WaitingForNovelEvidence => {
            return exact_wake_result(
                &scheduler,
                K1ExactWakeDecisionV1::WaitingForNovelEvidence,
                "all_readiness_pass_opportunities_attempted_deterministic",
                exact_counts,
                budget.trailing_24h_freezes,
                None,
            );
        }
        K1ExactWakeSelectionV1::ResearchBudgetCooldown(next_eligible_at_unix) => {
            return exact_wake_result(
                &scheduler,
                K1ExactWakeDecisionV1::ResearchBudgetCooldown,
                "research_budget_cooldown",
                exact_counts,
                budget.trailing_24h_freezes,
                Some(next_eligible_at_unix),
            );
        }
        K1ExactWakeSelectionV1::CandidateReady => {}
    }
    let queue_row = exact
        .queue
        .first_readiness_pass()
        .ok_or_else(|| "k1_exact_ready_candidate_missing".to_owned())?;
    let selected_at_unix = now_unix;
    let candidate = exact
        .catalog
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_root_sha256 == queue_row.candidate_root_sha256)
        .cloned()
        .ok_or_else(|| "k1_exact_queue_candidate_missing".to_owned())?;
    let support = exact
        .support_manifests_by_candidate
        .get(&candidate.candidate_root_sha256)
        .ok_or_else(|| "k1_exact_support_manifest_missing".to_owned())?;
    let artifacts = exact
        .artifact_projections_by_candidate
        .get(&candidate.candidate_root_sha256)
        .ok_or_else(|| "k1_exact_artifact_projection_missing".to_owned())?;
    let causal = exact
        .causal_manifests_by_candidate
        .get(&candidate.candidate_root_sha256)
        .cloned()
        .ok_or_else(|| "k1_exact_causal_manifest_missing".to_owned())?;
    let archive_manifest_root_sha256 = publish_exact_artifacts(
        &sources.artifact_archive_path,
        exact.source_snapshot.clone(),
        support.clone(),
        artifacts.clone(),
        &exact.artifacts,
        causal.clone(),
        exact.active_protocols.clone(),
    )?;
    validate_exact_wake_cas(
        config,
        &scheduler,
        &registry,
        &exact.active_protocol_mode_set_root_sha256,
        &policy,
        &exact.source_heads,
    )?;
    let authority_binding_manifest_root_sha256 =
        canonical_json_sha256(&AuthorityBindingManifestMaterialV1 {
            schema: "nando.k1-authority-binding-manifest.v1",
            scheduler_revision: scheduler.revision,
            scheduler_ledger_root_sha256: &scheduler.ledger_root_sha256,
            registry_revision: registry.revision,
            registry_root_sha256: &registry.ledger_root_sha256,
            deficit_snapshot_root_sha256: &deficit.snapshot_root_sha256,
            fixture_exclusion_root_sha256: &exact.catalog.fixture_exclusion_root_sha256,
            catalog_root_sha256: &exact.catalog.catalog_root_sha256,
            queue_root_sha256: &exact.queue.queue_root_sha256,
            candidate_root_sha256: &candidate.candidate_root_sha256,
            active_protocol_mode_set_root_sha256: &exact.active_protocol_mode_set_root_sha256,
            minimum_queue_schema: &policy.minimum_queue_schema,
            minimum_freeze_schema: &policy.minimum_freeze_schema,
            minimum_wire_schema: &policy.minimum_wire_schema,
            evidence_source_snapshot_root_sha256: &exact.source_snapshot.snapshot_root_sha256,
            collection_checkpoint_root_sha256: &exact
                .source_heads
                .collection_checkpoint_root_sha256,
            policy_root_sha256: &policy.policy_root_sha256,
        })
        .map_err(str::to_owned)?;
    let freeze = K1NaturalCandidateFreezeV1::seal_exact_v8(
        projection_for(&scheduler)?.next_generation_sequence,
        exact.catalog.as_ref(),
        &deficit,
        &exact.queue,
        &candidate,
        queue_row.score.clone(),
        policy.scheduler_schema.clone(),
        natural_t1_discovery_basis_root_v4().map_err(str::to_owned)?,
        crate::k1_natural_scheduler_runtime::exact_generation_budget_v1(),
        candidate.last_capture_sequence,
        exact.contract_watermark,
        selected_at_unix,
        causal,
        exact.source_snapshot.snapshot_root_sha256,
        archive_manifest_root_sha256,
        attempt_index.index_root_sha256,
        authority_binding_manifest_root_sha256,
    )
    .map_err(str::to_owned)?;
    validate_exact_wake_cas(
        config,
        &scheduler,
        &registry,
        &exact.active_protocol_mode_set_root_sha256,
        &policy,
        &exact.source_heads,
    )?;
    let projection = append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::CandidateFreeze(freeze),
    )?;
    let status = K1ExactWakeStatusV1::seal(
        K1ExactWakeDecisionV1::CandidateFrozen,
        "candidate_frozen",
        exact_counts.map(|value| value.0),
        exact_counts.map(|value| value.1),
        exact_counts.map(|value| value.2),
        exact_counts.map(|value| value.3),
        budget.trailing_24h_freezes.saturating_add(1),
        None,
    )?;
    Ok(K1ExactWakeAuthorityResultV1 { status, projection })
}

pub(super) fn exact_wake_selection_v1(
    readiness_pass_rows: u64,
    exact_unseen_opportunities: u64,
    budget: K1ExactResearchBudgetStateV1,
) -> K1ExactWakeSelectionV1 {
    if readiness_pass_rows == 0 {
        K1ExactWakeSelectionV1::WaitingForEvidence
    } else if exact_unseen_opportunities == 0 {
        K1ExactWakeSelectionV1::WaitingForNovelEvidence
    } else if let Some(next_eligible_at_unix) = budget.next_eligible_at_unix {
        K1ExactWakeSelectionV1::ResearchBudgetCooldown(next_eligible_at_unix)
    } else {
        K1ExactWakeSelectionV1::CandidateReady
    }
}

fn exact_wake_result(
    scheduler: &K1SchedulerLedgerV1,
    decision: K1ExactWakeDecisionV1,
    blocker: impl Into<String>,
    exact_counts: Option<(u64, u64, u64, u64)>,
    trailing_24h_freezes: u64,
    next_eligible_at_unix: Option<u64>,
) -> Result<K1ExactWakeAuthorityResultV1, String> {
    let status = K1ExactWakeStatusV1::seal(
        decision,
        blocker,
        exact_counts.map(|value| value.0),
        exact_counts.map(|value| value.1),
        exact_counts.map(|value| value.2),
        exact_counts.map(|value| value.3),
        trailing_24h_freezes,
        next_eligible_at_unix,
    )?;
    Ok(K1ExactWakeAuthorityResultV1 {
        status,
        projection: projection_for(scheduler)?,
    })
}

pub(super) fn exact_research_budget_state_v1(
    scheduler: &K1SchedulerLedgerV1,
    policy: &K1ExactSchedulerPolicyV1,
    now_unix: u64,
) -> Result<K1ExactResearchBudgetStateV1, String> {
    scheduler.validate().map_err(str::to_owned)?;
    if now_unix == 0 || policy.maximum_new_freezes_per_wake != 1 {
        return Err("k1_exact_research_budget_input_invalid".to_owned());
    }
    let window_start = now_unix.saturating_sub(K1_EXACT_TRAILING_WINDOW_SECONDS_V1);
    let mut freeze_times = scheduler
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            K1SchedulerEventPayloadV1::CandidateFreeze(freeze)
                if freeze.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8 =>
            {
                Some(freeze.selected_at_unix)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    freeze_times.sort_unstable();
    let trailing = freeze_times
        .iter()
        .filter(|time| **time > window_start && **time <= now_unix)
        .copied()
        .collect::<Vec<_>>();
    let interval_next = freeze_times
        .last()
        .copied()
        .map(|last| last.saturating_add(policy.minimum_freeze_interval_seconds))
        .filter(|next| *next > now_unix);
    let daily_next = (trailing.len() as u64 >= policy.maximum_trailing_24h_freezes).then(|| {
        trailing
            .first()
            .copied()
            .unwrap_or(now_unix)
            .saturating_add(K1_EXACT_TRAILING_WINDOW_SECONDS_V1)
            .saturating_add(1)
    });
    Ok(K1ExactResearchBudgetStateV1 {
        trailing_24h_freezes: trailing.len() as u64,
        next_eligible_at_unix: interval_next.into_iter().chain(daily_next).max(),
    })
}

pub(super) fn read_exact_scheduler_policy(path: &Path) -> Result<K1ExactSchedulerPolicyV1, String> {
    let policy = read_exact_scheduler_policy_document(path)?;
    if !policy.writer_enabled {
        return Err("k1_exact_writer_inactive".to_owned());
    }
    Ok(policy)
}

pub(crate) fn exact_writer_policy_health(path: &Path) -> K1ExactWriterPolicyHealthV1 {
    match fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => K1ExactWriterPolicyHealthV1 {
            state: "NOT_INSTALLED",
            policy_root_sha256: None,
            minimum_queue_schema: None,
            minimum_freeze_schema: None,
        },
        Err(_) => K1ExactWriterPolicyHealthV1 {
            state: "INVALID",
            policy_root_sha256: None,
            minimum_queue_schema: None,
            minimum_freeze_schema: None,
        },
        Ok(_) => match read_exact_scheduler_policy_document(path) {
            Ok(policy) => K1ExactWriterPolicyHealthV1 {
                state: if policy.writer_enabled { "ON" } else { "OFF" },
                policy_root_sha256: Some(policy.policy_root_sha256),
                minimum_queue_schema: Some(policy.minimum_queue_schema),
                minimum_freeze_schema: Some(policy.minimum_freeze_schema),
            },
            Err(_) => K1ExactWriterPolicyHealthV1 {
                state: "INVALID",
                policy_root_sha256: None,
                minimum_queue_schema: None,
                minimum_freeze_schema: None,
            },
        },
    }
}

fn read_exact_scheduler_policy_document(path: &Path) -> Result<K1ExactSchedulerPolicyV1, String> {
    let bytes = fs::read(path).map_err(|error| format!("k1_exact_policy_read:{error}"))?;
    let policy: K1ExactSchedulerPolicyV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("k1_exact_policy_decode:{error}"))?;
    let expected_root = canonical_json_sha256(&(
        K1_EXACT_SCHEDULER_POLICY_SCHEMA_V1,
        policy.writer_enabled,
        policy.minimum_queue_schema.as_str(),
        policy.minimum_freeze_schema.as_str(),
        policy.minimum_wire_schema.as_str(),
        policy.scheduler_schema.as_str(),
        policy.maximum_new_freezes_per_wake,
        policy.minimum_freeze_interval_seconds,
        policy.maximum_trailing_24h_freezes,
        policy.maximum_readiness_rows_per_wake,
    ))
    .map_err(str::to_owned)?;
    if policy.schema != K1_EXACT_SCHEDULER_POLICY_SCHEMA_V1
        || policy.policy_root_sha256 != expected_root
        || policy.minimum_queue_schema != "nando.k1-natural-candidate-queue.v4"
        || policy.minimum_freeze_schema != K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8
        || policy.minimum_wire_schema != K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1
        || policy.scheduler_schema != "nando.k1-operator-blind-scheduler.v4"
        || policy.maximum_new_freezes_per_wake != K1_EXACT_MAX_NEW_FREEZES_PER_WAKE_V1
        || policy.minimum_freeze_interval_seconds < K1_EXACT_MIN_FREEZE_INTERVAL_SECONDS_V1
        || policy.maximum_trailing_24h_freezes == 0
        || policy.maximum_trailing_24h_freezes > K1_EXACT_MAX_TRAILING_24H_FREEZES_V1
        || policy.maximum_readiness_rows_per_wake == 0
        || policy.maximum_readiness_rows_per_wake > K1_EXACT_MAX_READINESS_ROWS_PER_WAKE_V1
    {
        return Err("K1_AUTHORITY_SCHEMA_DOWNGRADE".to_owned());
    }
    Ok(policy)
}

fn publish_exact_artifacts(
    root: &Path,
    source_snapshot: nando_operator_learning::multi_source::EvidenceSourceSnapshotV1,
    support: nando_operator_learning::multi_source::IdentifierSupportManifestV1,
    artifacts: nando_operator_learning::multi_source::RelevantIdentifierArtifactProjectionV1,
    all_artifacts: &[nando_operator_learning::multi_source::NaturalT1ProgramArtifactV1],
    causal: nando_operator_learning::multi_source::IdentifierCausalInputManifestV1,
    active_protocols: BTreeSet<String>,
) -> Result<String, String> {
    let (manifest, object_bytes) =
        crate::k1_natural_scheduler_runtime::build_exact_identifier_archive_v1(
            source_snapshot,
            support,
            artifacts,
            all_artifacts,
            causal,
            active_protocols,
        )?;
    let object_path = root
        .join("objects")
        .join(format!("{}.cbor", manifest.object_root_sha256()));
    write_once_exact(&object_path, &object_bytes)?;
    let manifest_bytes =
        nando_operator_kernel::canonical_json_bytes(&manifest).map_err(str::to_owned)?;
    let manifest_root_sha256 = manifest.manifest_root_sha256().to_owned();
    write_once_exact(
        &root
            .join("manifests")
            .join(format!("{manifest_root_sha256}.json")),
        &manifest_bytes,
    )?;
    Ok(manifest_root_sha256)
}

fn write_once_exact(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Ok(existing) = fs::read(path) {
        return if existing == bytes {
            Ok(())
        } else {
            Err("k1_exact_artifact_rebind".to_owned())
        };
    }
    let parent = path
        .parent()
        .ok_or_else(|| "k1_exact_artifact_parent_missing".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| format!("k1_exact_artifact_parent:{error}"))?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("artifact"),
        std::process::id()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("k1_exact_artifact_create:{error}"))?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|error| format!("k1_exact_artifact_write:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("k1_exact_artifact_sync:{error}"))?;
        fs::hard_link(&temporary, path)
            .map_err(|error| format!("k1_exact_artifact_publish:{error}"))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("k1_exact_artifact_dir_sync:{error}"))
    })();
    let _ = fs::remove_file(&temporary);
    result
}

pub(super) fn validate_exact_wake_cas(
    config: &CertificationAuthorityConfigV1,
    scheduler: &K1SchedulerLedgerV1,
    registry: &nando_operator_admission::OperatorCertificationLedgerV1,
    active_protocol_mode_set_root_sha256: &str,
    policy: &K1ExactSchedulerPolicyV1,
    source_heads: &crate::k1_natural_scheduler_runtime::ExactDurableSourceHeadsV1,
) -> Result<(), String> {
    let current_registry = restore_anchored_ledger(config)?;
    let current_active_protocols = crate::multi_source_live::known_epistemic_protocol_mode_roots(
        &config.response_registry_path,
        &current_registry,
    )?;
    let current_active_protocol_mode_set_root_sha256 =
        duplicate_cohorts::known_epistemic_protocol_mode_set_root(&current_active_protocols)?;
    let sources = config
        .k1_exact_sources
        .as_ref()
        .ok_or_else(|| "k1_exact_authority_sources_not_configured".to_owned())?;
    if restore_anchored_scheduler_for(config, K1SchedulerLaneV1::Epistemic)? != *scheduler
        || current_registry != *registry
        || current_active_protocol_mode_set_root_sha256 != active_protocol_mode_set_root_sha256
        || read_exact_scheduler_policy_document(&sources.scheduler_policy_path)? != *policy
        || crate::k1_natural_scheduler_runtime::restore_exact_durable_source_heads_v1(config)?
            != *source_heads
    {
        return Err("STALE_BEFORE_FREEZE".to_owned());
    }
    Ok(())
}

pub(crate) fn recover_authority(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
) -> Result<(), String> {
    recover_lane(config, signing_key, K1SchedulerLaneV1::Mechanism)?;
    let mechanism = restore_projection_for(config, K1SchedulerLaneV1::Mechanism)?;
    if mechanism.active_candidate_freeze.is_some() && mechanism.identification_freeze.is_some() {
        fork::ensure_epistemic_lane(config, signing_key)?;
    }
    recover_lane(config, signing_key, K1SchedulerLaneV1::Epistemic)?;
    Ok(())
}

fn recover_lane(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    lane: K1SchedulerLaneV1,
) -> Result<(), String> {
    let (ledger, last_event_root) = restore_signed_scheduler_journal_for(config, lane)?;
    let anchor_path = scheduler_anchor_path_for(config, lane)?;
    if !anchor_path.exists() {
        if ledger.revision != 0 {
            return Err("k1_scheduler_anchor_missing_for_nonempty_journal".to_owned());
        }
        return persist_scheduler_anchor_for(config, lane, signing_key, &ledger, &last_event_root);
    }
    let anchor = restore_scheduler_anchor_for(config, lane)?;
    if anchor.revision == ledger.revision
        && anchor.journal_event_root_sha256 == last_event_root
        && anchor.ledger_root_sha256 == ledger.ledger_root_sha256
    {
        return Ok(());
    }
    if anchor.revision >= ledger.revision {
        return Err("k1_scheduler_rollback_detected".to_owned());
    }
    let (prefix, prefix_event_root) =
        restore_scheduler_journal_prefix_for(config, lane, anchor.revision)?;
    if prefix.ledger_root_sha256 != anchor.ledger_root_sha256
        || prefix_event_root != anchor.journal_event_root_sha256
    {
        return Err("k1_scheduler_rollback_detected".to_owned());
    }
    persist_scheduler_anchor_for(config, lane, signing_key, &ledger, &last_event_root)
}

fn append_candidate_freeze_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1CandidateFreezeAuthorityRequestV1,
    scheduler_cas: Option<&bounded_wire::K1SchedulerCasV2>,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1 {
        return Err("k1_candidate_freeze_request_schema_invalid".to_owned());
    }
    request.catalog.validate().map_err(str::to_owned)?;
    request.deficit_snapshot.validate().map_err(str::to_owned)?;
    request.queue.validate().map_err(str::to_owned)?;
    request.candidate.validate().map_err(str::to_owned)?;
    request.freeze.validate().map_err(str::to_owned)?;
    validate_active_protocol_mode_cas(config, &request.active_protocol_mode_set_root_sha256)?;
    let discovery_basis_root_sha256 = validate_discovery_basis_cas(&request.freeze)?;

    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let completed_candidate_roots_sha256 = candidate_exclusions_for_ledger(
        config,
        request.lane,
        &scheduler,
        &request.catalog,
        &request.active_protocol_mode_set_root_sha256,
        &request.freeze.schema,
        &discovery_basis_root_sha256,
    )?;
    validate_queue_derivation(
        &scheduler,
        &request.catalog,
        &request.deficit_snapshot,
        &completed_candidate_roots_sha256,
        &discovery_basis_root_sha256,
        request.freeze.contract_watermark,
        &request.queue,
    )?;
    if scheduler
        .active_candidate_freeze()
        .is_some_and(|freeze| freeze == &request.freeze)
    {
        return projection_for(&scheduler);
    }
    if let Some(cas) = scheduler_cas {
        validate_scheduler_cas(&scheduler, cas)?;
    }
    let registry = restore_anchored_ledger(config)?;
    validate_registry_cas(&registry, &request.deficit_snapshot)?;
    let expected = K1NaturalCandidateFreezeV1::seal(
        request.freeze.generation_sequence,
        &request.catalog,
        &request.deficit_snapshot,
        &request.queue,
        &request.candidate,
        request.freeze.scoring_tuple.clone(),
        request.freeze.scheduler_schema.clone(),
        discovery_basis_root_sha256,
        request.freeze.budget,
        request.freeze.support_watermark,
        request.freeze.contract_watermark,
        request.freeze.selected_at_unix,
    )
    .map_err(str::to_owned)?;
    if expected != request.freeze {
        return Err("k1_candidate_freeze_reseal_mismatch".to_owned());
    }
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::CandidateFreeze(request.freeze),
    )
}

pub(super) fn validate_scheduler_cas(
    scheduler: &K1SchedulerLedgerV1,
    claimed: &bounded_wire::K1SchedulerCasV2,
) -> Result<(), String> {
    let projection = projection_for(scheduler)?;
    if claimed.ledger_revision != projection.ledger_revision
        || claimed.ledger_root_sha256 != projection.ledger_root_sha256
        || claimed.projection_root_sha256 != projection.projection_root_sha256
    {
        return Err("k1_candidate_freeze_scheduler_cas_failed".to_owned());
    }
    Ok(())
}

pub(super) fn validate_discovery_basis_cas(
    freeze: &K1NaturalCandidateFreezeV1,
) -> Result<String, String> {
    let current = match freeze.schema.as_str() {
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3 => {
            natural_t1_discovery_basis_root_v1().map_err(str::to_owned)?
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4 => {
            natural_t1_discovery_basis_root_v2().map_err(str::to_owned)?
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5 => {
            natural_t1_discovery_basis_root_v3().map_err(str::to_owned)?
        }
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6
        | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7
        | K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8 => {
            natural_t1_discovery_basis_root_v4().map_err(str::to_owned)?
        }
        _ => return Err("k1_candidate_freeze_discovery_basis_cas_failed".to_owned()),
    };
    if freeze.discovery_basis_root_sha256 != current {
        return Err("k1_candidate_freeze_discovery_basis_cas_failed".to_owned());
    }
    Ok(current)
}

pub(super) fn validate_rollback_reader_schema(
    ledger: &K1SchedulerLedgerV1,
    minimum_freeze_schema: &str,
) -> Result<(), String> {
    ledger.validate().map_err(str::to_owned)?;
    let v8_suffix_exists = ledger.events.iter().any(|event| {
        matches!(
            &event.payload,
            K1SchedulerEventPayloadV1::CandidateFreeze(freeze)
                if freeze.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8
        )
    });
    if v8_suffix_exists && minimum_freeze_schema != K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8 {
        return Err("k1_post_v8_rollback_reader_forbidden".to_owned());
    }
    Ok(())
}

pub(super) fn validate_active_protocol_mode_cas(
    config: &CertificationAuthorityConfigV1,
    claimed_root_sha256: &str,
) -> Result<(), String> {
    let certification = restore_anchored_ledger(config)?;
    let current_known_modes = crate::multi_source_live::known_epistemic_protocol_mode_roots(
        &config.response_registry_path,
        &certification,
    )?;
    let current_root =
        duplicate_cohorts::known_epistemic_protocol_mode_set_root(&current_known_modes)?;
    if claimed_root_sha256 != current_root {
        return Err("k1_candidate_freeze_active_protocol_mode_cas_failed".to_owned());
    }
    Ok(())
}

pub(super) fn append_payload_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1SchedulerAppendAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_SCHEDULER_APPEND_AUTHORITY_REQUEST_SCHEMA_V1
        || matches!(
            request.payload,
            K1SchedulerEventPayloadV1::CandidateFreeze(_)
                | K1SchedulerEventPayloadV1::TransferSettlement(_)
                | K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(_)
                | K1SchedulerEventPayloadV1::FuturePredictionContract(_)
                | K1SchedulerEventPayloadV1::FuturePrediction(_)
                | K1SchedulerEventPayloadV1::FuturePredictionCensored(_)
                | K1SchedulerEventPayloadV1::FutureOutcome(_)
        )
    {
        return Err("k1_scheduler_append_request_invalid".to_owned());
    }
    request.payload.validate().map_err(str::to_owned)?;
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    if matches!(
        &request.payload,
        K1SchedulerEventPayloadV1::TerminalVerdict(verdict)
            if request.lane == K1SchedulerLaneV1::Epistemic
                && verdict.verdict == K1GenerationVerdictClassV1::AcquisitionFail
                && scheduler.active_candidate_freeze().is_some_and(|freeze| {
                    freeze.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8
                        && deterministic_initial_blocker_v1(&verdict.blocker)
                })
    ) {
        return Err("k1_exact_terminal_requires_authority".to_owned());
    }
    let terminal_root = match &request.payload {
        K1SchedulerEventPayloadV1::TerminalVerdict(verdict)
            if request.lane == K1SchedulerLaneV1::Epistemic =>
        {
            Some(verdict.verdict_root_sha256.clone())
        }
        _ => None,
    };
    if scheduler
        .latest_event()
        .is_some_and(|event| payload_root(&event.payload) == payload_root(&request.payload))
    {
        if let Some(terminal_root) = terminal_root {
            super::pre_action_evidence_retention::prune_after_terminal_verdict(
                config,
                &terminal_root,
            )?;
        }
        return projection_for(&scheduler);
    }
    let projection = append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        request.payload,
    )?;
    if let Some(terminal_root) = terminal_root {
        super::pre_action_evidence_retention::prune_after_terminal_verdict(config, &terminal_root)?;
    }
    Ok(projection)
}

fn append_transfer_settlement_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1TransferSettlementAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_TRANSFER_SETTLEMENT_AUTHORITY_REQUEST_SCHEMA_V1 {
        return Err("k1_transfer_settlement_request_schema_invalid".to_owned());
    }
    request.settlement.validate().map_err(str::to_owned)?;
    let certification = restore_anchored_ledger(config)?;
    if certification.ledger_root_sha256 != request.settlement.certification_ledger_root_sha256 {
        return Err("k1_transfer_settlement_certification_cas_failed".to_owned());
    }
    let entry = certification
        .latest_entries()
        .into_iter()
        .find(|entry| entry.package_id == request.settlement.package_id)
        .ok_or_else(|| "k1_transfer_settlement_certification_missing".to_owned())?;
    if !certification_authorizes_settlement(entry, &request.settlement) {
        return Err("k1_transfer_settlement_certification_invalid".to_owned());
    }
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    if scheduler.latest_event().is_some_and(|event| {
        matches!(
            &event.payload,
            K1SchedulerEventPayloadV1::TransferSettlement(existing)
                if existing == &request.settlement
        )
    }) {
        return projection_for(&scheduler);
    }
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::TransferSettlement(request.settlement),
    )
}

pub(super) fn certification_authorizes_settlement(
    entry: &nando_operator_admission::OperatorCertificationEntryV1,
    settlement: &K1TransferSettlementV1,
) -> bool {
    entry.package_id == settlement.package_id
        && entry.entry_root_sha256 == settlement.certification_entry_root_sha256
        && entry.law.certificate_root_sha256 == settlement.law_certificate_root_sha256
        && entry.law.status == nando_operator_admission::LawCertificateStatusV1::Pass
        && entry.product_registry_member
        && entry.epistemic_registry_member
        && entry.k1_unit_eligible
        && entry.false_bad_apply == 0
        && entry
            .law
            .evidence_roots_sha256
            .contains(&settlement.package_candidate_root_sha256)
        && entry
            .law
            .evidence_roots_sha256
            .contains(&settlement.terminal_verdict_root_sha256)
        && entry
            .law
            .evidence_roots_sha256
            .contains(&settlement.identification_report_root_sha256)
}

pub(super) fn append_and_persist(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    signing_key: &SigningKey,
    ledger: &mut K1SchedulerLedgerV1,
    payload: K1SchedulerEventPayloadV1,
) -> Result<K1SchedulerProjectionV1, String> {
    let event = ledger.append(payload).map_err(str::to_owned)?.clone();
    let signed =
        SignedSchedulerEventV1::seal(event, ledger.ledger_root_sha256.clone(), signing_key)?;
    persist_scheduler_event_for(config, lane, &signed)?;
    persist_scheduler_anchor_for(
        config,
        lane,
        signing_key,
        ledger,
        &signed.event.event_root_sha256,
    )?;
    persist_scheduler_cache_for(config, lane, ledger)?;
    let restored = restore_anchored_scheduler_for(config, lane)?;
    if &restored != ledger {
        return Err("k1_scheduler_restart_parity_failed".to_owned());
    }
    projection_for(ledger)
}

pub(super) fn validate_registry_cas(
    ledger: &nando_operator_admission::OperatorCertificationLedgerV1,
    snapshot: &K1DeficitSnapshotV1,
) -> Result<(), String> {
    let gate = ledger.k1_vocabulary_gate().map_err(str::to_owned)?;
    let eligible = ledger
        .latest_entries()
        .into_iter()
        .filter(|entry| entry.k1_unit_eligible)
        .collect::<Vec<_>>();
    let semantic_roots = eligible
        .iter()
        .map(|entry| entry.semantic_law_id_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let topology_roots = eligible
        .iter()
        .map(|entry| entry.role_topology_id_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if snapshot.epistemic_registry_revision != ledger.revision
        || snapshot.epistemic_registry_root_sha256 != ledger.ledger_root_sha256
        || snapshot.k1_gate_root_sha256 != gate.gate_root_sha256
        || snapshot.law_certificates != gate.law_certificates
        || snapshot.semantic_laws != gate.semantic_laws
        || snapshot.role_topologies != gate.role_topologies
        || snapshot.cleanup_receipts != gate.cleanup_receipts
        || snapshot.false_bad_apply != gate.false_bad_apply
        || snapshot.minimum_law_certificates != gate.min_law_certificates
        || snapshot.minimum_semantic_laws != gate.min_semantic_laws
        || snapshot.minimum_role_topologies != gate.min_role_topologies
        || snapshot.eligible_semantic_law_roots_sha256 != semantic_roots
        || snapshot.eligible_role_topology_roots_sha256 != topology_roots
        || snapshot.k1_open != gate.open
    {
        return Err("k1_candidate_freeze_registry_cas_failed".to_owned());
    }
    Ok(())
}

pub(super) fn send_authority_request<T: Serialize>(
    config: &CertificationAuthorityConfigV1,
    request: &T,
) -> Result<K1SchedulerProjectionV1, String> {
    #[cfg(not(unix))]
    {
        let _ = (config, request);
        return Err("k1_scheduler_authority_requires_unix".to_owned());
    }
    #[cfg(unix)]
    {
        let bytes = serde_json::to_vec(request)
            .map_err(|error| format!("k1_scheduler_authority_encode:{error}"))?;
        send_authority_bytes(config, bytes)
    }
}

#[cfg(unix)]
pub(super) fn send_exact_wake_bytes(
    config: &CertificationAuthorityConfigV1,
    bytes: Vec<u8>,
) -> Result<(K1ExactWakeStatusV1, K1SchedulerProjectionV1), String> {
    if bytes.len() > K1_SCHEDULER_MAX_REQUEST_BYTES {
        return Err("k1_scheduler_authority_request_budget".to_owned());
    }
    let mut stream = UnixStream::connect(&config.authority_socket_path)
        .map_err(|error| format!("k1_scheduler_authority_connect:{error}"))?;
    stream
        .set_read_timeout(Some(K1_SCHEDULER_AUTHORITY_READ_TIMEOUT))
        .map_err(|error| format!("k1_scheduler_authority_read_timeout:{error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("k1_scheduler_authority_write_timeout:{error}"))?;
    stream
        .write_all(&bytes)
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| format!("k1_scheduler_authority_write:{error}"))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| format!("k1_scheduler_authority_shutdown:{error}"))?;
    let response: K1ExactWakeAuthorityResponseV1 = serde_json::from_reader(&mut stream)
        .map_err(|error| format!("k1_exact_wake_decode:{error}"))?;
    if response.schema != K1_EXACT_WAKE_AUTHORITY_RESPONSE_SCHEMA_V1 || !response.error.is_empty() {
        return Err(if response.error.is_empty() {
            "k1_exact_wake_response_invalid".to_owned()
        } else {
            response.error
        });
    }
    let status = response
        .status
        .ok_or_else(|| "k1_exact_wake_status_missing".to_owned())?;
    status.validate()?;
    let projection = response
        .projection
        .ok_or_else(|| "k1_exact_wake_projection_missing".to_owned())?;
    projection.validate()?;
    Ok((status, projection))
}

#[cfg(not(unix))]
pub(super) fn send_exact_wake_bytes(
    _config: &CertificationAuthorityConfigV1,
    _bytes: Vec<u8>,
) -> Result<(K1ExactWakeStatusV1, K1SchedulerProjectionV1), String> {
    Err("k1_scheduler_authority_requires_unix".to_owned())
}

#[cfg(unix)]
pub(super) fn send_authority_bytes(
    config: &CertificationAuthorityConfigV1,
    bytes: Vec<u8>,
) -> Result<K1SchedulerProjectionV1, String> {
    if bytes.len() > K1_SCHEDULER_MAX_REQUEST_BYTES {
        return Err("k1_scheduler_authority_request_budget".to_owned());
    }
    let mut stream = UnixStream::connect(&config.authority_socket_path)
        .map_err(|error| format!("k1_scheduler_authority_connect:{error}"))?;
    stream
        .set_read_timeout(Some(K1_SCHEDULER_AUTHORITY_READ_TIMEOUT))
        .map_err(|error| format!("k1_scheduler_authority_read_timeout:{error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("k1_scheduler_authority_write_timeout:{error}"))?;
    stream
        .write_all(&bytes)
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| format!("k1_scheduler_authority_write:{error}"))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| format!("k1_scheduler_authority_shutdown:{error}"))?;
    let response: K1SchedulerAuthorityResponseV1 = serde_json::from_reader(&mut stream)
        .map_err(|error| format!("k1_scheduler_authority_decode:{error}"))?;
    if response.schema != K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1 || !response.error.is_empty() {
        return Err(if response.error.is_empty() {
            "k1_scheduler_authority_response_invalid".to_owned()
        } else {
            response.error
        });
    }
    let projection = response
        .projection
        .ok_or_else(|| "k1_scheduler_authority_projection_missing".to_owned())?;
    projection.validate()?;
    Ok(projection)
}
