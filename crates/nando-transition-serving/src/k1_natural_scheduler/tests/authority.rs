use nando_operator_admission::{
    ExecutionCertificateStatusV1, ExecutionCertificateV1, LawCertificateStatusV1, LawCertificateV1,
    MechanismCertificateStatusV1, MechanismCertificateV1, OperatorCertificationEntryV1,
    OperatorCertificationLedgerV1, OperatorMechanismClassV1,
};

use super::*;
use crate::k1_natural_scheduler::authority::{
    K1ExactResearchBudgetStateV1, K1ExactWakeSelectionV1, append_and_persist,
    append_payload_authoritative, certification_authorizes_settlement,
    complete_exact_terminal_transaction, exact_research_budget_state_v1, exact_wake_authoritative,
    exact_wake_selection_v1, read_exact_scheduler_policy, validate_active_protocol_mode_cas,
    validate_discovery_basis_cas, validate_exact_wake_cas, validate_registry_cas,
    validate_rollback_reader_schema,
};
use crate::k1_natural_scheduler::journal::{
    persist_scheduler_event_for, restore_anchored_scheduler_for,
};
use crate::k1_natural_scheduler::projection::exact_attempt_index_for;
use crate::operator_certification::K1ExactAuthoritySourceConfigV1;
use nando_response_actor::{OnlineCollectionConfig, OnlineCollectionMiner};
use std::path::Path;

fn write_exact_policy(path: &Path, writer_enabled: bool, queue: &str) {
    let policy_root_sha256 = canonical_json_sha256(&(
        "nando.k1-exact-scheduler-policy.v1",
        writer_enabled,
        queue,
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8,
        K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1,
        "nando.k1-operator-blind-scheduler.v4",
        1_u64,
        300_u64,
        48_u64,
        256_u64,
    ))
    .expect("policy root");
    fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "schema": "nando.k1-exact-scheduler-policy.v1",
            "policy_root_sha256": policy_root_sha256,
            "writer_enabled": writer_enabled,
            "minimum_queue_schema": queue,
            "minimum_freeze_schema": K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8,
            "minimum_wire_schema": K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1,
            "scheduler_schema": "nando.k1-operator-blind-scheduler.v4",
            "maximum_new_freezes_per_wake": 1,
            "minimum_freeze_interval_seconds": 300,
            "maximum_trailing_24h_freezes": 48,
            "maximum_readiness_rows_per_wake": 256,
        }))
        .expect("policy bytes"),
    )
    .expect("policy write");
}

#[test]
fn authority_rejects_a_valid_freeze_bound_to_an_uninstalled_discovery_basis() {
    validate_discovery_basis_cas(&candidate_freeze()).expect("installed discovery basis");
    validate_discovery_basis_cas(&exact_candidate_freeze(1)).expect("installed V8 discovery basis");
    assert_eq!(
        validate_discovery_basis_cas(&candidate_freeze_with_basis(root(999))),
        Err("k1_candidate_freeze_discovery_basis_cas_failed".to_owned())
    );
}

#[test]
fn first_v8_suffix_permanently_fences_pre_phase_a_readers() {
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    validate_rollback_reader_schema(&ledger, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7)
        .expect("legacy-only prefix accepts legacy reader");

    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            exact_candidate_freeze(1),
        ))
        .expect("V8 suffix");
    assert_eq!(
        validate_rollback_reader_schema(&ledger, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7),
        Err("k1_post_v8_rollback_reader_forbidden".to_owned())
    );
    validate_rollback_reader_schema(&ledger, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8)
        .expect("Phase A reader remains valid");
}

#[test]
fn writer_off_wake_still_enforces_the_v8_reader_fence() {
    let (root_dir, mut config, signing_key) = test_context();
    let policy_path = root_dir.join("policy.json");
    write_exact_policy(&policy_path, false, "nando.k1-natural-candidate-queue.v4");
    config.k1_exact_sources = Some(K1ExactAuthoritySourceConfigV1 {
        topology_archive_path: root_dir.join("topology.cbor"),
        frame_archive_path: root_dir.join("frames.cbor"),
        collection_checkpoint_path: root_dir.join("collection.cbor"),
        artifact_archive_path: root_dir.join("artifacts"),
        scheduler_policy_path: policy_path.clone(),
    });
    recover_authority(&config, &signing_key).expect("genesis");
    let mut scheduler = K1SchedulerLedgerV1::empty().expect("scheduler");
    append_and_persist(
        &config,
        K1SchedulerLaneV1::Epistemic,
        &signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::CandidateFreeze(exact_candidate_freeze(1)),
    )
    .expect("V8 suffix");

    let mut policy: serde_json::Value =
        serde_json::from_slice(&fs::read(&policy_path).expect("policy bytes"))
            .expect("policy json");
    policy["minimum_freeze_schema"] =
        serde_json::Value::String(K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7.to_owned());
    policy["policy_root_sha256"] = serde_json::Value::String(
        canonical_json_sha256(&(
            "nando.k1-exact-scheduler-policy.v1",
            false,
            "nando.k1-natural-candidate-queue.v4",
            K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7,
            K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1,
            "nando.k1-operator-blind-scheduler.v4",
            1_u64,
            300_u64,
            48_u64,
            256_u64,
        ))
        .expect("downgrade policy root"),
    );
    fs::write(
        &policy_path,
        serde_json::to_vec(&policy).expect("policy encode"),
    )
    .expect("policy write");

    assert_eq!(
        exact_wake_authoritative(&config, &signing_key, exact_wake_request()),
        Err("K1_AUTHORITY_SCHEMA_DOWNGRADE".to_owned())
    );
    assert_eq!(
        restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
            .expect("preserved scheduler"),
        scheduler
    );
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn exact_policy_distinguishes_disabled_writer_from_schema_downgrade() {
    let (root_dir, _, _) = test_context();
    let path = root_dir.join("policy.json");
    write_exact_policy(&path, false, "nando.k1-natural-candidate-queue.v4");
    assert_eq!(
        read_exact_scheduler_policy(&path),
        Err("k1_exact_writer_inactive".to_owned())
    );
    write_exact_policy(&path, true, "nando.k1-natural-candidate-queue.v3");
    assert_eq!(
        read_exact_scheduler_policy(&path),
        Err("K1_AUTHORITY_SCHEMA_DOWNGRADE".to_owned())
    );
    std::fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn exact_policy_rejects_any_research_limit_increase() {
    let (root_dir, _, _) = test_context();
    let path = root_dir.join("policy.json");
    write_exact_policy(&path, true, "nando.k1-natural-candidate-queue.v4");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("policy bytes")).expect("policy json");
    value["maximum_trailing_24h_freezes"] = serde_json::json!(49);
    value["policy_root_sha256"] = serde_json::Value::String(
        canonical_json_sha256(&(
            "nando.k1-exact-scheduler-policy.v1",
            true,
            "nando.k1-natural-candidate-queue.v4",
            K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8,
            K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1,
            "nando.k1-operator-blind-scheduler.v4",
            1_u64,
            300_u64,
            49_u64,
            256_u64,
        ))
        .expect("raised policy root"),
    );
    fs::write(&path, serde_json::to_vec(&value).expect("policy encode")).expect("policy write");
    assert_eq!(
        read_exact_scheduler_policy(&path),
        Err("K1_AUTHORITY_SCHEMA_DOWNGRADE".to_owned())
    );
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn signed_v8_history_reconstructs_interval_and_daily_budget_after_restart() {
    let (root_dir, _, _) = test_context();
    let path = root_dir.join("policy.json");
    write_exact_policy(&path, true, "nando.k1-natural-candidate-queue.v4");
    let policy = read_exact_scheduler_policy(&path).expect("policy");
    let base = 1_800_000_000_u64;
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    for generation in 1..=48_u64 {
        let selected_at = base.saturating_add(generation.saturating_mul(1_000));
        let freeze = exact_candidate_freeze_at(generation, selected_at);
        let diagnostic = exact_terminal_diagnostic(&freeze);
        let verdict = K1GenerationTerminalVerdictV1::seal(
            freeze.freeze_root_sha256.clone(),
            None,
            Vec::new(),
            vec![
                freeze.freeze_root_sha256.clone(),
                diagnostic.identifier_report_root_sha256.clone(),
                diagnostic.identifier_result_root_sha256.clone(),
                diagnostic.terminal_diagnostic_root_sha256.clone(),
            ],
            K1GenerationVerdictClassV1::AcquisitionFail,
            diagnostic.exact_result_blocker.clone(),
            diagnostic.terminal_at_unix,
            None,
        )
        .expect("verdict");
        for payload in [
            K1SchedulerEventPayloadV1::CandidateFreeze(freeze),
            K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(Box::new(diagnostic)),
            K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
        ] {
            ledger.append(payload).expect("append history");
        }
    }
    let now = base.saturating_add(48_100);
    let budget = exact_research_budget_state_v1(&ledger, &policy, now).expect("daily budget");
    assert_eq!(budget.trailing_24h_freezes, 48);
    assert_eq!(
        budget.next_eligible_at_unix,
        Some(base + 1_000 + 86_400 + 1)
    );

    let encoded = serde_json::to_vec(&ledger).expect("ledger encode");
    let restored: K1SchedulerLedgerV1 = serde_json::from_slice(&encoded).expect("ledger restart");
    assert_eq!(
        exact_research_budget_state_v1(&restored, &policy, now).expect("restart budget"),
        budget
    );

    let interval_now = base.saturating_add(48_050);
    let interval =
        exact_research_budget_state_v1(&restored, &policy, interval_now).expect("interval budget");
    assert_eq!(
        interval.next_eligible_at_unix,
        Some(base + 1_000 + 86_400 + 1)
    );

    let mut single = K1SchedulerLedgerV1::empty().expect("single ledger");
    let freeze = exact_candidate_freeze_at(1, base + 48_000);
    single
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(freeze))
        .expect("single freeze");
    let interval =
        exact_research_budget_state_v1(&single, &policy, interval_now).expect("interval budget");
    assert_eq!(interval.trailing_24h_freezes, 1);
    assert_eq!(interval.next_eligible_at_unix, Some(base + 48_300));
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn exact_wake_state_matrix_orders_evidence_novelty_budget_and_freeze() {
    let open = K1ExactResearchBudgetStateV1 {
        trailing_24h_freezes: 0,
        next_eligible_at_unix: None,
    };
    let cooldown = K1ExactResearchBudgetStateV1 {
        trailing_24h_freezes: 1,
        next_eligible_at_unix: Some(1_800_000_300),
    };
    assert_eq!(
        exact_wake_selection_v1(0, 0, cooldown),
        K1ExactWakeSelectionV1::WaitingForEvidence
    );
    assert_eq!(
        exact_wake_selection_v1(8, 0, cooldown),
        K1ExactWakeSelectionV1::WaitingForNovelEvidence
    );
    assert_eq!(
        exact_wake_selection_v1(8, 1, cooldown),
        K1ExactWakeSelectionV1::ResearchBudgetCooldown(1_800_000_300)
    );
    assert_eq!(
        exact_wake_selection_v1(8, 1, open),
        K1ExactWakeSelectionV1::CandidateReady
    );
}

#[test]
fn exact_wake_status_allows_deadline_only_for_waiting_budget_states() {
    let vocabulary_open = K1ExactWakeStatusV1::seal(
        K1ExactWakeDecisionV1::K1VocabularyOpen,
        "k1_vocabulary_open",
        None,
        None,
        None,
        None,
        48,
        None,
    )
    .expect("vocabulary-open status");
    assert_eq!(vocabulary_open.next_eligible_at_unix, None);

    assert_eq!(
        K1ExactWakeStatusV1::seal(
            K1ExactWakeDecisionV1::K1VocabularyOpen,
            "k1_vocabulary_open",
            None,
            None,
            None,
            None,
            48,
            Some(1_800_000_300),
        )
        .expect_err("vocabulary-open deadline must be rejected"),
        "k1_exact_wake_status_invalid"
    );
}

fn exact_wake_request() -> K1ExactWakeAuthorityRequestV1 {
    K1ExactWakeAuthorityRequestV1 {
        schema: K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Epistemic,
    }
}

fn configure_empty_exact_sources(
    root_dir: &Path,
    config: &mut CertificationAuthorityConfigV1,
    writer_enabled: bool,
) {
    let topology_path = root_dir.join("topology");
    let frame_path = root_dir.join("frames");
    let collection_path = root_dir.join("collection.cbor");
    let artifact_path = root_dir.join("artifacts");
    let policy_path = root_dir.join("policy.json");
    fs::create_dir_all(&topology_path).expect("topology directory");
    fs::create_dir_all(&frame_path).expect("frame directory");
    let collection =
        OnlineCollectionMiner::open(&collection_path, OnlineCollectionConfig::default())
            .expect("empty collection");
    collection.flush().expect("durable empty collection");
    drop(collection);
    fs::write(
        &config.response_registry_path,
        serde_json::to_vec(&serde_json::json!({
            "schema": "nando.response-registry.v6",
            "revision": 0,
            "packages": []
        }))
        .expect("empty registry bytes"),
    )
    .expect("empty registry");
    write_exact_policy(
        &policy_path,
        writer_enabled,
        "nando.k1-natural-candidate-queue.v4",
    );
    config.k1_exact_sources = Some(K1ExactAuthoritySourceConfigV1 {
        topology_archive_path: topology_path,
        frame_archive_path: frame_path,
        collection_checkpoint_path: collection_path,
        artifact_archive_path: artifact_path,
        scheduler_policy_path: policy_path,
    });
}

#[test]
fn writer_inactive_and_waiting_wakes_append_no_event() {
    let (root_dir, mut config, signing_key) = test_context();
    configure_empty_exact_sources(&root_dir, &mut config, false);
    recover_authority(&config, &signing_key).expect("scheduler genesis");
    let before = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("initial scheduler");

    let inactive = exact_wake_authoritative(&config, &signing_key, exact_wake_request())
        .expect("inactive wake");
    assert_eq!(
        inactive.status.decision,
        K1ExactWakeDecisionV1::WriterInactive
    );
    assert_eq!(inactive.projection.ledger_revision, before.revision);
    assert_eq!(
        inactive.projection.ledger_root_sha256,
        before.ledger_root_sha256
    );

    let policy_path = &config
        .k1_exact_sources
        .as_ref()
        .expect("exact sources")
        .scheduler_policy_path;
    write_exact_policy(policy_path, true, "nando.k1-natural-candidate-queue.v4");
    let waiting = exact_wake_authoritative(&config, &signing_key, exact_wake_request())
        .expect("waiting wake");
    assert_eq!(
        waiting.status.decision,
        K1ExactWakeDecisionV1::WaitingForEvidence
    );
    assert_eq!(waiting.status.readiness_pass_rows, Some(0));
    assert_eq!(waiting.projection.ledger_revision, before.revision);
    assert_eq!(
        waiting.projection.ledger_root_sha256,
        before.ledger_root_sha256
    );
    let after = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("unchanged scheduler");
    assert_eq!(after, before);
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn active_generation_wake_is_read_only_and_preserves_exact_freeze() {
    let (root_dir, mut config, signing_key) = test_context();
    configure_empty_exact_sources(&root_dir, &mut config, true);
    recover_authority(&config, &signing_key).expect("scheduler genesis");
    let mut scheduler = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("initial scheduler");
    let freeze = exact_candidate_freeze(1);
    append_and_persist(
        &config,
        K1SchedulerLaneV1::Epistemic,
        &signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::CandidateFreeze(freeze.clone()),
    )
    .expect("active freeze");
    let before = scheduler.clone();

    let active =
        exact_wake_authoritative(&config, &signing_key, exact_wake_request()).expect("active wake");
    assert_eq!(
        active.status.decision,
        K1ExactWakeDecisionV1::ActiveGeneration
    );
    assert_eq!(
        active
            .projection
            .active_candidate_freeze
            .as_ref()
            .map(|value| value.freeze_root_sha256.as_str()),
        Some(freeze.freeze_root_sha256.as_str())
    );
    let after = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("unchanged active scheduler");
    assert_eq!(after, before);
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn policy_change_after_snapshot_fails_stale_without_scheduler_event() {
    let (root_dir, mut config, signing_key) = test_context();
    let topology_path = root_dir.join("topology");
    let frame_path = root_dir.join("frames");
    let collection_path = root_dir.join("collection.cbor");
    let artifact_path = root_dir.join("artifacts");
    let policy_path = root_dir.join("policy.json");
    fs::create_dir_all(&topology_path).expect("topology directory");
    fs::create_dir_all(&frame_path).expect("frame directory");
    let collection =
        OnlineCollectionMiner::open(&collection_path, OnlineCollectionConfig::default())
            .expect("empty collection");
    collection.flush().expect("durable empty collection");
    drop(collection);
    write_exact_policy(&policy_path, true, "nando.k1-natural-candidate-queue.v4");
    config.k1_exact_sources = Some(K1ExactAuthoritySourceConfigV1 {
        topology_archive_path: topology_path,
        frame_archive_path: frame_path,
        collection_checkpoint_path: collection_path,
        artifact_archive_path: artifact_path,
        scheduler_policy_path: policy_path.clone(),
    });
    recover_authority(&config, &signing_key).expect("scheduler genesis");
    let scheduler = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("epistemic scheduler");
    let registry =
        crate::operator_certification::restore_anchored_ledger(&config).expect("empty registry");
    let active_protocol_mode_set_root_sha256 =
        crate::k1_natural_scheduler::duplicate_cohorts::known_epistemic_protocol_mode_set_root(
            &BTreeSet::new(),
        )
        .expect("empty active modes");
    let policy = read_exact_scheduler_policy(&policy_path).expect("active policy");
    let source_heads =
        crate::k1_natural_scheduler_runtime::restore_exact_durable_source_heads_v1(&config)
            .expect("durable heads");
    validate_exact_wake_cas(
        &config,
        &scheduler,
        &registry,
        &active_protocol_mode_set_root_sha256,
        &policy,
        &source_heads,
    )
    .expect("unchanged snapshot");

    write_exact_policy(&policy_path, false, "nando.k1-natural-candidate-queue.v4");
    assert_eq!(
        validate_exact_wake_cas(
            &config,
            &scheduler,
            &registry,
            &active_protocol_mode_set_root_sha256,
            &policy,
            &source_heads,
        ),
        Err("STALE_BEFORE_FREEZE".to_owned())
    );
    let unchanged = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("unchanged scheduler");
    assert_eq!(unchanged.revision, 0);
    assert!(unchanged.events.is_empty());
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn exact_wake_wire_has_no_scientific_or_timestamp_fields() {
    let value = serde_json::to_value(K1ExactWakeAuthorityRequestV1 {
        schema: K1_EXACT_WAKE_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Epistemic,
    })
    .expect("wake");
    assert_eq!(value.as_object().expect("object").len(), 2);
    for forbidden in [
        "catalog",
        "queue",
        "attempt_index",
        "freeze",
        "selected_at_unix",
        "candidate",
        "source_snapshot",
    ] {
        assert!(value.get(forbidden).is_none(), "forbidden {forbidden}");
    }
}

#[test]
fn exact_terminal_wire_has_only_active_freeze_identity() {
    let value = serde_json::to_value(K1ExactTerminalAuthorityRequestV1 {
        schema: K1_EXACT_TERMINAL_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Epistemic,
        candidate_freeze_root_sha256: root(50_000),
    })
    .expect("terminal request");
    assert_eq!(value.as_object().expect("object").len(), 3);
    for forbidden in [
        "identifier_result",
        "diagnostic",
        "disposition",
        "blocker",
        "terminal_at_unix",
        "programs",
        "histogram",
    ] {
        assert!(value.get(forbidden).is_none(), "forbidden {forbidden}");
    }
}

#[test]
fn generic_authority_rejects_v8_diagnostic_and_deterministic_verdict() {
    let (root_dir, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("genesis");
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let mut scheduler = K1SchedulerLedgerV1::empty().expect("scheduler");
    append_and_persist(
        &config,
        K1SchedulerLaneV1::Epistemic,
        &signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::CandidateFreeze(freeze.clone()),
    )
    .expect("freeze");
    let diagnostic_error = append_payload_authoritative(
        &config,
        &signing_key,
        K1SchedulerAppendAuthorityRequestV1 {
            schema: K1_SCHEDULER_APPEND_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
            lane: K1SchedulerLaneV1::Epistemic,
            payload: K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(Box::new(
                diagnostic.clone(),
            )),
        },
    )
    .expect_err("diagnostic must use exact authority");
    assert_eq!(diagnostic_error, "k1_scheduler_append_request_invalid");
    let verdict = K1GenerationTerminalVerdictV1::seal(
        freeze.freeze_root_sha256.clone(),
        None,
        Vec::new(),
        vec![freeze.freeze_root_sha256],
        K1GenerationVerdictClassV1::AcquisitionFail,
        "motif_program_candidates_empty".to_owned(),
        1_700_000_100,
        None,
    )
    .expect("verdict");
    let verdict_error = append_payload_authoritative(
        &config,
        &signing_key,
        K1SchedulerAppendAuthorityRequestV1 {
            schema: K1_SCHEDULER_APPEND_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
            lane: K1SchedulerLaneV1::Epistemic,
            payload: K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
        },
    )
    .expect_err("verdict must use exact authority");
    assert_eq!(verdict_error, "k1_exact_terminal_requires_authority");
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn diagnostic_crash_retry_appends_only_one_matching_verdict() {
    let (root_dir, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("mechanism genesis");
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let mut scheduler = K1SchedulerLedgerV1::empty().expect("scheduler");
    for payload in [
        K1SchedulerEventPayloadV1::CandidateFreeze(freeze.clone()),
        K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(Box::new(diagnostic.clone())),
    ] {
        append_and_persist(
            &config,
            K1SchedulerLaneV1::Epistemic,
            &signing_key,
            &mut scheduler,
            payload,
        )
        .expect("durable pre-crash event");
    }

    let mut restarted = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("restart after diagnostic");
    complete_exact_terminal_transaction(
        &config,
        &signing_key,
        K1SchedulerLaneV1::Epistemic,
        &mut restarted,
        &freeze,
        &diagnostic.identifier_report_root_sha256,
        &diagnostic.identifier_result_root_sha256,
        diagnostic.clone(),
    )
    .expect("append missing verdict");

    let restored = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("restored completed transaction");
    assert_eq!(restored.revision, 3);
    assert_eq!(
        restored
            .events
            .iter()
            .filter(|event| matches!(
                &event.payload,
                K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(_)
            ))
            .count(),
        1
    );
    assert_eq!(
        restored
            .events
            .iter()
            .filter(|event| matches!(
                &event.payload,
                K1SchedulerEventPayloadV1::TerminalVerdict(_)
            ))
            .count(),
        1
    );
    assert_eq!(
        exact_attempt_index_for(&restored)
            .expect("attempt index")
            .deterministic_attempts
            .len(),
        1
    );
    let mut idempotent = restored.clone();
    let projection = complete_exact_terminal_transaction(
        &config,
        &signing_key,
        K1SchedulerLaneV1::Epistemic,
        &mut idempotent,
        &freeze,
        &diagnostic.identifier_report_root_sha256,
        &diagnostic.identifier_result_root_sha256,
        diagnostic.clone(),
    )
    .expect("idempotent completed retry");
    assert_eq!(projection.ledger_revision, 3);
    assert_eq!(idempotent.revision, 3);
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn exact_terminal_transaction_rejects_the_mechanism_lane() {
    let (root_dir, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("genesis");
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let mut mechanism = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Mechanism)
        .expect("mechanism lane");

    assert_eq!(
        complete_exact_terminal_transaction(
            &config,
            &signing_key,
            K1SchedulerLaneV1::Mechanism,
            &mut mechanism,
            &freeze,
            &diagnostic.identifier_report_root_sha256,
            &diagnostic.identifier_result_root_sha256,
            diagnostic.clone(),
        ),
        Err("k1_exact_terminal_transaction_cas_invalid".to_owned())
    );
    assert_eq!(mechanism.revision, 0);
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn lagging_anchor_after_verdict_recovers_one_exact_attempt() {
    let (root_dir, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("mechanism genesis");
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let mut scheduler = K1SchedulerLedgerV1::empty().expect("scheduler");
    for payload in [
        K1SchedulerEventPayloadV1::CandidateFreeze(freeze.clone()),
        K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(Box::new(diagnostic.clone())),
    ] {
        append_and_persist(
            &config,
            K1SchedulerLaneV1::Epistemic,
            &signing_key,
            &mut scheduler,
            payload,
        )
        .expect("anchored event");
    }
    let anchored_revision = scheduler.revision;
    let anchored_root = scheduler
        .latest_event()
        .expect("diagnostic event")
        .event_root_sha256
        .clone();
    let verdict = K1GenerationTerminalVerdictV1::seal(
        freeze.freeze_root_sha256.clone(),
        None,
        Vec::new(),
        vec![
            freeze.freeze_root_sha256.clone(),
            diagnostic.identifier_report_root_sha256.clone(),
            diagnostic.identifier_result_root_sha256.clone(),
            diagnostic.terminal_diagnostic_root_sha256.clone(),
        ],
        K1GenerationVerdictClassV1::AcquisitionFail,
        diagnostic.exact_result_blocker.clone(),
        diagnostic.terminal_at_unix,
        None,
    )
    .expect("verdict");
    let event = scheduler
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            verdict,
        )))
        .expect("append verdict")
        .clone();
    let signed =
        SignedSchedulerEventV1::seal(event, scheduler.ledger_root_sha256.clone(), &signing_key)
            .expect("signed verdict");
    persist_scheduler_event_for(&config, K1SchedulerLaneV1::Epistemic, &signed)
        .expect("durable verdict, stale anchor");
    assert_eq!(anchored_revision, 2);
    assert_eq!(
        restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic),
        Err("k1_scheduler_rollback_detected".to_owned())
    );

    recover_authority(&config, &signing_key).expect("recover epistemic signed tail");
    let restored = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("recovered ledger");
    assert_eq!(restored.revision, 3);
    assert_eq!(
        exact_attempt_index_for(&restored)
            .expect("attempt index")
            .deterministic_attempts
            .len(),
        1
    );
    assert_ne!(
        restored.latest_event().expect("verdict").event_root_sha256,
        anchored_root
    );
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn authority_rejects_stale_known_epistemic_protocol_mode_set_root() {
    let (root_dir, config, _) = test_context();
    std::fs::write(
        &config.response_registry_path,
        serde_json::to_vec(&nando_response_actor::ResponseRegistry {
            schema: "nando.response-registry.v6".to_owned(),
            revision: 0,
            packages: Vec::new(),
        })
        .expect("registry encode"),
    )
    .expect("registry write");
    let current =
        super::super::duplicate_cohorts::known_epistemic_protocol_mode_set_root(&BTreeSet::new())
            .expect("known set root");
    validate_active_protocol_mode_cas(&config, &current).expect("current root");
    assert_eq!(
        validate_active_protocol_mode_cas(&config, &root(999)),
        Err("k1_candidate_freeze_active_protocol_mode_cas_failed".to_owned())
    );
    std::fs::remove_dir_all(root_dir).expect("cleanup");
}
use crate::k1_natural_scheduler::selection_authority::validate_queue_derivation;

#[test]
fn stale_registry_snapshot_cannot_freeze_a_generation() {
    let ledger = OperatorCertificationLedgerV1::empty().expect("empty registry");
    let stale = K1DeficitSnapshotV1::seal(
        ledger.revision.saturating_add(1),
        root(600),
        root(601),
        0,
        0,
        0,
        0,
        0,
        3,
        3,
        2,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("valid stale snapshot");

    assert_eq!(
        validate_registry_cas(&ledger, &stale),
        Err("k1_candidate_freeze_registry_cas_failed".to_owned())
    );
}

#[test]
fn transfer_settlement_cannot_precede_law_certificate_pass() {
    let bundle = root(610);
    let package = "package-one";
    let package_candidate = root(611);
    let terminal = root(612);
    let identification = root(613);
    let entry = OperatorCertificationEntryV1::seal(
        &bundle,
        package,
        &root(614),
        &root(615),
        ExecutionCertificateV1::seal(
            &bundle,
            package,
            ExecutionCertificateStatusV1::Pass,
            vec![root(616)],
            "",
        )
        .expect("execution"),
        LawCertificateV1::seal(
            &bundle,
            package,
            LawCertificateStatusV1::Partial,
            vec![
                package_candidate.clone(),
                terminal.clone(),
                identification.clone(),
            ],
            None,
            "cleanup_receipt_pending",
        )
        .expect("partial law"),
        MechanismCertificateV1::seal(
            &bundle,
            package,
            MechanismCertificateStatusV1::Collecting,
            OperatorMechanismClassV1::Unresolved,
            vec![root(617)],
            "exact_wave_collecting",
        )
        .expect("mechanism"),
        0,
    )
    .expect("entry");
    let settlement = K1TransferSettlementV1 {
        schema: "test".to_owned(),
        settlement_root_sha256: root(618),
        terminal_verdict_root_sha256: terminal,
        candidate_freeze_root_sha256: root(619),
        identification_report_root_sha256: identification,
        package_id: package.to_owned(),
        package_candidate_root_sha256: package_candidate,
        certification_entry_root_sha256: entry.entry_root_sha256.clone(),
        certification_ledger_root_sha256: root(620),
        law_certificate_root_sha256: entry.law.certificate_root_sha256.clone(),
        settled_at_unix: 1_700_000_000,
        authority_ready: false,
        phase_mutation_allowed: false,
    };

    assert!(!certification_authorizes_settlement(&entry, &settlement));
}

#[test]
fn authority_rebuilds_queue_and_rejects_a_valid_omission() {
    let rows = (1..=8)
        .map(|index| {
            K1NaturalEvidenceRowV1::seal(
                root(700 + index),
                root(799),
                root(800),
                root(801),
                root(802),
                root(if index <= 4 { 803 } else { 804 }),
                K1ConsequenceTypeV1::Scalar,
                K1NaturalEvidenceClassV1::NaturalLive,
                index,
                100,
                1_000,
                true,
                index <= 2,
                false,
            )
            .expect("evidence")
        })
        .collect::<Vec<_>>();
    let catalog = build_k1_natural_cohort_catalog_v1(
        &rows,
        root(805),
        root(806),
        "nando.operator-blind-version-space-generator.v1".to_owned(),
    )
    .expect("catalog");
    let deficit = K1DeficitSnapshotV1::seal(
        0,
        root(807),
        root(808),
        0,
        0,
        0,
        0,
        0,
        3,
        3,
        2,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("deficit");
    let contract_watermark = catalog
        .candidates
        .iter()
        .map(|candidate| candidate.last_capture_sequence)
        .max()
        .expect("candidate watermark");
    let proposed =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, contract_watermark).expect("queue");
    let completed = BTreeSet::new();
    let ledger = K1SchedulerLedgerV1::empty().expect("empty scheduler ledger");
    validate_queue_derivation(
        &ledger,
        &catalog,
        &deficit,
        &completed,
        &natural_t1_discovery_basis_root_v3().expect("discovery basis"),
        contract_watermark,
        &proposed,
    )
    .expect("authoritative derivation");

    let omitted = proposed.rows[0].candidate_root_sha256.clone();
    let tampered =
        nando_operator_learning::multi_source::build_k1_natural_candidate_queue_with_exclusions_v1(
            &catalog,
            &deficit,
            &BTreeSet::from([omitted]),
            contract_watermark,
        )
        .expect("internally valid omitted queue");
    assert_eq!(
        validate_queue_derivation(
            &ledger,
            &catalog,
            &deficit,
            &completed,
            &natural_t1_discovery_basis_root_v3().expect("discovery basis"),
            contract_watermark,
            &tampered,
        ),
        Err("k1_candidate_queue_derivation_mismatch".to_owned())
    );
}
