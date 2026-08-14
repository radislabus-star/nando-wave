use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_kernel::canonical_json_sha256;

use super::*;
use crate::{
    LawLabProbeDomainV1, LawLabSandboxOperationV1, LawLabSandboxPurposeV1,
    LawLabSandboxRequestInputV1, LawLabSandboxRequestV1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

fn fixture_action(
    environment_root_sha256: &str,
    label: &str,
    operations: &[LawLabSandboxOperationV1],
    predicted_consequence_root_sha256: String,
) -> K2K1ActionRefV1 {
    fixture_action_with_effect(
        environment_root_sha256,
        label,
        operations,
        predicted_consequence_root_sha256,
        root(&format!("fixture-effect:{label}")),
    )
}

fn fixture_action_with_effect(
    environment_root_sha256: &str,
    label: &str,
    operations: &[LawLabSandboxOperationV1],
    predicted_consequence_root_sha256: String,
    fixture_effect_root_sha256: String,
) -> K2K1ActionRefV1 {
    K2K1ActionRefV1::seal(K2K1ActionRefInputV1 {
        provenance: K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        applicability_environment_root_sha256: environment_root_sha256.to_owned(),
        applicability_receipt_root_sha256: root(&format!("applicability:{label}")),
        operation_plan_root_sha256: canonical_json_sha256(&operations).expect("operations root"),
        predicted_consequence_root_sha256,
        fixture_effect_root_sha256: Some(fixture_effect_root_sha256),
        law_certificate_root_sha256: None,
        epistemic_registry_member_root_sha256: None,
        bundle_v4_root_sha256: None,
        execution_certificate_root_sha256: None,
        applicability_guard_root_sha256: None,
        effect_contract_root_sha256: None,
        semantic_class_root_sha256: None,
        role_topology_root_sha256: None,
    })
    .expect("fixture action")
}

fn certificate_action(
    environment_root_sha256: &str,
    label: &str,
    predicted_consequence_root_sha256: String,
) -> K2K1ActionRefV1 {
    K2K1ActionRefV1::seal(K2K1ActionRefInputV1 {
        provenance: K2EvidenceProvenanceV1::CertificateBoundK1,
        applicability_environment_root_sha256: environment_root_sha256.to_owned(),
        applicability_receipt_root_sha256: root(&format!("applicability:{label}")),
        operation_plan_root_sha256: root(&format!("operation:{label}")),
        predicted_consequence_root_sha256,
        fixture_effect_root_sha256: None,
        law_certificate_root_sha256: Some(root(&format!("law:{label}"))),
        epistemic_registry_member_root_sha256: Some(root(&format!("member:{label}"))),
        bundle_v4_root_sha256: Some(root(&format!("bundle:{label}"))),
        execution_certificate_root_sha256: Some(root(&format!("execution:{label}"))),
        applicability_guard_root_sha256: Some(root(&format!("guard:{label}"))),
        effect_contract_root_sha256: Some(root(&format!("effect:{label}"))),
        semantic_class_root_sha256: Some(root(&format!("semantic:{label}"))),
        role_topology_root_sha256: Some(root(&format!("topology:{label}"))),
    })
    .expect("certificate action")
}

struct FixtureCaseV1 {
    goal: K2GoalEnvelopeV1,
    vocabulary: K2K1VocabularySnapshotV1,
    alternatives: K2AlternativeSetV1,
    freeze: K2DecisionFreezeV1,
    predictions: K2AlternativePredictionSetV1,
    selection: K2PreparedSelectionReceiptV1,
    request: LawLabSandboxRequestV1,
}

fn fixture_case(goal_matches_one: bool) -> FixtureCaseV1 {
    let environment = root("fixture-environment");
    let selected_operations = vec![LawLabSandboxOperationV1::CopySourceFile {
        source_path: "input.txt".to_owned(),
        work_path: "selected.txt".to_owned(),
    }];
    let alternate_operations = vec![LawLabSandboxOperationV1::RemoveWorkPath {
        work_path: "input.txt".to_owned(),
    }];
    let selected_consequence = root("selected-terminal-tree");
    let alternate_consequence = root("alternate-terminal-tree");
    let oracle = K2ExactOracleManifestV1::seal(root("oracle-executable")).expect("oracle");
    let goal = K2GoalEnvelopeV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        environment.clone(),
        if goal_matches_one {
            selected_consequence.clone()
        } else {
            root("unmatched-terminal-tree")
        },
        root("goal-store-snapshot"),
        root("goal-constraints"),
        oracle.manifest_root_sha256.clone(),
        1_786_500_000_000,
    )
    .expect("goal");
    let selected_action = fixture_action(
        &environment,
        "selected",
        &selected_operations,
        selected_consequence,
    );
    let alternate_action = fixture_action(
        &environment,
        "alternate",
        &alternate_operations,
        alternate_consequence,
    );
    let vocabulary = K2K1VocabularySnapshotV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        None,
        None,
        vec![selected_action, alternate_action],
        1_786_500_000_001,
    )
    .expect("vocabulary");
    let alternatives = K2AlternativeSetV1::seal(&vocabulary, environment).expect("alternatives");
    let budget = K2GoalEnvironmentBudgetV1::preregistered_v1();
    let freeze = K2DecisionFreezeV1::seal(K2DecisionFreezeInputV1 {
        episode_id_sha256: root("episode"),
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        budget,
        selector_contract_root_sha256: root("selector-contract"),
        selector_executable_sha256: root("selector-executable"),
        oracle_manifest: &oracle,
        sandbox_worker_sha256: root("sandbox-worker"),
        deterministic_seed_sha256: root("deterministic-seed"),
        observed_registry_revision: None,
        observed_registry_root_sha256: None,
        frozen_at_unix_ms: 1_786_500_000_002,
    })
    .expect("freeze");
    let predictions = K2AlternativePredictionSetV1::prepared_capability_v1(
        &freeze,
        &goal,
        &vocabulary,
        &alternatives,
        &budget,
        &oracle,
    )
    .expect("predictions");
    let selection = K2PreparedSelectionReceiptV1::select(&freeze, &predictions)
        .unwrap_or_else(|error| panic!("selection: {error}"));
    let selected = alternatives
        .alternative(&selection.selected_action_root_sha256)
        .expect("selected alternative");
    let selected_operations = if selected.operation_plan_root_sha256
        == canonical_json_sha256(&selected_operations).expect("selected operations root")
    {
        selected_operations
    } else {
        alternate_operations
    };
    let request = LawLabSandboxRequestV1::seal(LawLabSandboxRequestInputV1 {
        executor_manifest_root_sha256: root("executor-manifest"),
        worker_sha256: freeze.sandbox_worker_sha256.clone(),
        candidate_root_sha256: freeze.episode_id_sha256.clone(),
        version_space_root_sha256: alternatives.alternative_set_root_sha256.clone(),
        durable_prediction_ledger_root_sha256: predictions.prediction_set_root_sha256.clone(),
        probe_root_sha256: selection.selection_root_sha256.clone(),
        source_tree_root_sha256: freeze.initial_environment_root_sha256.clone(),
        deterministic_seed_sha256: freeze.deterministic_seed_sha256.clone(),
        domain: LawLabProbeDomainV1::Filesystem,
        purpose: LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest,
        surviving_hypothesis_count: alternatives.alternatives.len() as u64,
        precommitted_prediction_count: predictions.predictions.len() as u64,
        operations: selected_operations,
    })
    .expect("request");
    FixtureCaseV1 {
        goal,
        vocabulary,
        alternatives,
        freeze,
        predictions,
        selection,
        request,
    }
}

#[test]
fn goal_and_capability_choice_are_pre_action_canonical_and_authority_free() {
    let case = fixture_case(true);
    case.goal.validate().expect("goal valid");
    assert!(!case.predictions.learned);
    assert_eq!(
        case.predictions
            .predictions
            .iter()
            .filter(|prediction| prediction.predicted_goal_satisfied)
            .count(),
        1
    );
    case.freeze.authority.validate().expect("authority false");
    case.selection
        .authority
        .validate()
        .expect("selection authority false");
    let first = case.goal.canonical_bytes().expect("goal bytes");
    let second = case.goal.canonical_bytes().expect("goal bytes");
    assert_eq!(first, second);

    let mut post_action_tamper = case.goal.clone();
    post_action_tamper.expected_terminal_tree_root_sha256 = root("post-action-rewrite");
    assert!(post_action_tamper.validate().is_err());
}

#[test]
fn meaningful_alternatives_reject_fixture_effect_aliases() {
    let environment = root("alias-environment");
    let operations_a = vec![LawLabSandboxOperationV1::RemoveWorkPath {
        work_path: "a".to_owned(),
    }];
    let operations_b = vec![LawLabSandboxOperationV1::RemoveWorkPath {
        work_path: "b".to_owned(),
    }];
    let shared_effect = root("shared-fixture-effect");
    let first = fixture_action_with_effect(
        &environment,
        "a",
        &operations_a,
        root("outcome-a"),
        shared_effect.clone(),
    );
    let second = fixture_action_with_effect(
        &environment,
        "b",
        &operations_b,
        root("outcome-b"),
        shared_effect,
    );
    assert!(
        K2K1VocabularySnapshotV1::seal(
            K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
            None,
            None,
            vec![first, second],
            1,
        )
        .is_err()
    );
}

#[test]
fn certificate_snapshot_is_rechecked_atomically_and_runtime_stays_closed() {
    let environment = root("certificate-environment");
    let oracle = K2ExactOracleManifestV1::seal(root("certificate-oracle")).expect("oracle");
    let goal = K2GoalEnvelopeV1::seal(
        K2EvidenceProvenanceV1::CertificateBoundK1,
        environment.clone(),
        root("certificate-goal"),
        root("certificate-goal-store"),
        root("certificate-constraints"),
        oracle.manifest_root_sha256.clone(),
        1,
    )
    .expect("goal");
    let registry_root = root("registry");
    let vocabulary = K2K1VocabularySnapshotV1::seal(
        K2EvidenceProvenanceV1::CertificateBoundK1,
        Some(7),
        Some(registry_root.clone()),
        vec![
            certificate_action(&environment, "a", root("certificate-outcome-a")),
            certificate_action(&environment, "b", root("certificate-outcome-b")),
        ],
        2,
    )
    .expect("vocabulary");
    let alternatives = K2AlternativeSetV1::seal(&vocabulary, environment).expect("alternatives");
    let result = K2DecisionFreezeV1::seal(K2DecisionFreezeInputV1 {
        episode_id_sha256: root("certificate-episode"),
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        budget: K2GoalEnvironmentBudgetV1::preregistered_v1(),
        selector_contract_root_sha256: root("certificate-selector-contract"),
        selector_executable_sha256: root("certificate-selector"),
        oracle_manifest: &oracle,
        sandbox_worker_sha256: root("certificate-worker"),
        deterministic_seed_sha256: root("certificate-seed"),
        observed_registry_revision: Some(8),
        observed_registry_root_sha256: Some(registry_root),
        frozen_at_unix_ms: 3,
    });
    assert_eq!(
        result,
        Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_registry_stale_before_freeze"
        ))
    );
    assert_eq!(
        k2_certificate_bound_runtime_status_v1(1),
        K2CertificateBoundRuntimeStatusV1::InsufficientK1Vocabulary
    );
    assert_eq!(
        k2_certificate_bound_runtime_status_v1(2),
        K2CertificateBoundRuntimeStatusV1::CertificateBoundRuntimeClosed
    );
}

#[test]
fn prepared_selector_executes_nothing_without_one_satisfier() {
    let environment = root("no-selection-environment");
    let operations_a = vec![LawLabSandboxOperationV1::RemoveWorkPath {
        work_path: "a".to_owned(),
    }];
    let operations_b = vec![LawLabSandboxOperationV1::RemoveWorkPath {
        work_path: "b".to_owned(),
    }];
    let oracle = K2ExactOracleManifestV1::seal(root("no-selection-oracle")).expect("oracle");
    let goal = K2GoalEnvelopeV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        environment.clone(),
        root("unmatched-goal"),
        root("no-selection-store"),
        root("no-selection-constraints"),
        oracle.manifest_root_sha256.clone(),
        1,
    )
    .expect("goal");
    let vocabulary = K2K1VocabularySnapshotV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        None,
        None,
        vec![
            fixture_action(&environment, "a", &operations_a, root("outcome-a")),
            fixture_action(&environment, "b", &operations_b, root("outcome-b")),
        ],
        2,
    )
    .expect("vocabulary");
    let alternatives = K2AlternativeSetV1::seal(&vocabulary, environment).expect("alternatives");
    let budget = K2GoalEnvironmentBudgetV1::preregistered_v1();
    let freeze = K2DecisionFreezeV1::seal(K2DecisionFreezeInputV1 {
        episode_id_sha256: root("no-selection-episode"),
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        budget,
        selector_contract_root_sha256: root("no-selection-contract"),
        selector_executable_sha256: root("no-selection-selector"),
        oracle_manifest: &oracle,
        sandbox_worker_sha256: root("no-selection-worker"),
        deterministic_seed_sha256: root("no-selection-seed"),
        observed_registry_revision: None,
        observed_registry_root_sha256: None,
        frozen_at_unix_ms: 3,
    })
    .expect("freeze");
    let predictions = K2AlternativePredictionSetV1::prepared_capability_v1(
        &freeze,
        &goal,
        &vocabulary,
        &alternatives,
        &budget,
        &oracle,
    )
    .expect("predictions");
    assert_eq!(
        K2PreparedSelectionReceiptV1::select(&freeze, &predictions),
        Err(K2GoalEnvironmentErrorV1::Invalid("k2_no_unique_selection"))
    );
}

#[test]
fn k2_binding_rejects_a_valid_request_from_another_episode() {
    let case = fixture_case(true);
    let binding = K2LawLabBindingV1::seal(K2LawLabBindingInputV1 {
        freeze: &case.freeze,
        goal: &case.goal,
        vocabulary: &case.vocabulary,
        alternatives: &case.alternatives,
        predictions: &case.predictions,
        selection: &case.selection,
        request: &case.request,
    })
    .expect("binding");
    binding
        .validate(K2LawLabBindingInputV1 {
            freeze: &case.freeze,
            goal: &case.goal,
            vocabulary: &case.vocabulary,
            alternatives: &case.alternatives,
            predictions: &case.predictions,
            selection: &case.selection,
            request: &case.request,
        })
        .expect("valid binding");

    let replay = LawLabSandboxRequestV1::seal(LawLabSandboxRequestInputV1 {
        executor_manifest_root_sha256: case.request.executor_manifest_root_sha256.clone(),
        worker_sha256: case.request.worker_sha256.clone(),
        candidate_root_sha256: root("other-episode"),
        version_space_root_sha256: case.request.version_space_root_sha256.clone(),
        durable_prediction_ledger_root_sha256: case
            .request
            .durable_prediction_ledger_root_sha256
            .clone(),
        probe_root_sha256: case.request.probe_root_sha256.clone(),
        source_tree_root_sha256: case.request.source_tree_root_sha256.clone(),
        deterministic_seed_sha256: case.request.deterministic_seed_sha256.clone(),
        domain: case.request.domain,
        purpose: case.request.purpose,
        surviving_hypothesis_count: case.request.surviving_hypothesis_count,
        precommitted_prediction_count: case.request.precommitted_prediction_count,
        operations: case.request.operations.clone(),
    })
    .expect("replay request");
    assert!(
        K2LawLabBindingV1::seal(K2LawLabBindingInputV1 {
            freeze: &case.freeze,
            goal: &case.goal,
            vocabulary: &case.vocabulary,
            alternatives: &case.alternatives,
            predictions: &case.predictions,
            selection: &case.selection,
            request: &replay,
        })
        .is_err()
    );
}

#[test]
fn journal_enforces_order_and_restart_marks_dispatched_as_non_retryable() {
    let case = fixture_case(true);
    let binding = K2LawLabBindingV1::seal(K2LawLabBindingInputV1 {
        freeze: &case.freeze,
        goal: &case.goal,
        vocabulary: &case.vocabulary,
        alternatives: &case.alternatives,
        predictions: &case.predictions,
        selection: &case.selection,
        request: &case.request,
    })
    .expect("binding");
    let scratch = TestDirectoryV1::new("dispatch-restart");
    let mut journal =
        K2EpisodeJournalV1::create(scratch.path(), case.freeze.episode_id_sha256.clone())
            .expect("journal");
    journal
        .append(K2EpisodeEventKindV1::ContractFrozen, &case.freeze, 1)
        .expect("contract event");
    journal
        .append(
            K2EpisodeEventKindV1::PredictionsPrecommitted,
            &case.predictions,
            2,
        )
        .expect("prediction event");
    journal
        .append(K2EpisodeEventKindV1::ProbePlanned, &binding, 3)
        .expect("plan event");
    assert!(journal.projection().same_identity_execution_allowed);
    journal
        .append(
            K2EpisodeEventKindV1::ProbeDispatched,
            &binding.binding_root_sha256,
            4,
        )
        .expect("dispatch event");
    assert!(!journal.projection().same_identity_execution_allowed);

    let reopened = K2EpisodeJournalV1::open_existing(scratch.path(), case.freeze.episode_id_sha256)
        .expect("reopened");
    assert_eq!(
        reopened.projection().state,
        K2EpisodeStateV1::ProbeDispatched
    );
    assert!(reopened.projection().indeterminate_after_crash);
    assert!(!reopened.projection().same_identity_execution_allowed);
}

#[test]
fn journal_rejects_valid_predictions_rebound_from_another_episode() {
    let case = fixture_case(true);
    let oracle = K2ExactOracleManifestV1::seal(root("oracle-executable")).expect("oracle");
    let budget = K2GoalEnvironmentBudgetV1::preregistered_v1();
    let foreign_freeze = K2DecisionFreezeV1::seal(K2DecisionFreezeInputV1 {
        episode_id_sha256: root("foreign-episode"),
        goal: &case.goal,
        vocabulary: &case.vocabulary,
        alternatives: &case.alternatives,
        budget,
        selector_contract_root_sha256: root("selector-contract"),
        selector_executable_sha256: root("selector-executable"),
        oracle_manifest: &oracle,
        sandbox_worker_sha256: root("sandbox-worker"),
        deterministic_seed_sha256: root("deterministic-seed"),
        observed_registry_revision: None,
        observed_registry_root_sha256: None,
        frozen_at_unix_ms: 1_786_500_000_003,
    })
    .expect("foreign freeze");
    let foreign_predictions = K2AlternativePredictionSetV1::prepared_capability_v1(
        &foreign_freeze,
        &case.goal,
        &case.vocabulary,
        &case.alternatives,
        &budget,
        &oracle,
    )
    .expect("foreign predictions");
    assert_ne!(
        foreign_predictions.decision_freeze_root_sha256,
        case.freeze.decision_freeze_root_sha256
    );

    let scratch = TestDirectoryV1::new("cross-episode-predictions");
    let mut journal =
        K2EpisodeJournalV1::create(scratch.path(), case.freeze.episode_id_sha256.clone())
            .expect("journal");
    journal
        .append(K2EpisodeEventKindV1::ContractFrozen, &case.freeze, 1)
        .expect("contract event");
    assert!(
        journal
            .append(
                K2EpisodeEventKindV1::PredictionsPrecommitted,
                &foreign_predictions,
                2,
            )
            .is_err()
    );
    assert_eq!(journal.projection().event_count, 1);
    let reopened = K2EpisodeJournalV1::open_existing(scratch.path(), case.freeze.episode_id_sha256)
        .expect("valid prefix");
    assert_eq!(reopened.projection().event_count, 1);
}

#[test]
fn journal_faults_never_turn_partial_publication_into_a_valid_episode() {
    let case = fixture_case(true);
    let clean = TestDirectoryV1::new("temp-fault");
    let mut journal =
        K2EpisodeJournalV1::create(clean.path(), case.freeze.episode_id_sha256.clone())
            .expect("journal");
    assert!(
        journal
            .append_with_fault_v1(
                K2EpisodeEventKindV1::ContractFrozen,
                &case.freeze,
                1,
                K2JournalFaultPointV1::AfterTempSync,
            )
            .is_err()
    );
    let reopened =
        K2EpisodeJournalV1::open_existing(clean.path(), case.freeze.episode_id_sha256.clone())
            .expect("clean restart");
    assert_eq!(reopened.projection().state, K2EpisodeStateV1::Empty);

    let torn = TestDirectoryV1::new("publish-fault");
    let mut journal =
        K2EpisodeJournalV1::create(torn.path(), case.freeze.episode_id_sha256.clone())
            .expect("journal");
    assert!(
        journal
            .append_with_fault_v1(
                K2EpisodeEventKindV1::ContractFrozen,
                &case.freeze,
                1,
                K2JournalFaultPointV1::AfterPublishBeforeDirectorySync,
            )
            .is_err()
    );
    assert!(K2EpisodeJournalV1::open_existing(torn.path(), case.freeze.episode_id_sha256).is_err());
}

#[test]
fn journal_rejects_illegal_order_and_oversized_payload_without_publication() {
    let case = fixture_case(true);
    let scratch = TestDirectoryV1::new("invalid-events");
    let mut journal =
        K2EpisodeJournalV1::create(scratch.path(), case.freeze.episode_id_sha256.clone())
            .expect("journal");
    assert!(
        journal
            .append(
                K2EpisodeEventKindV1::PredictionsPrecommitted,
                &case.predictions,
                1,
            )
            .is_err()
    );
    assert_eq!(journal.projection().event_count, 0);
    let oversized = "x".repeat(K2_MAX_EVENT_BYTES_V1 as usize + 1);
    assert!(
        journal
            .append(K2EpisodeEventKindV1::ContractFrozen, &oversized, 2)
            .is_err()
    );
    assert_eq!(journal.projection().event_count, 0);
}

#[test]
fn terminal_seal_has_a_stable_one_way_root() {
    let seal = K2DecisionEpisodeSealV1::derive(
        root("seal-episode"),
        root("seal-outcome"),
        root("seal-terminal-event"),
        root("seal-projection"),
    )
    .expect("seal");
    seal.validate().expect("valid seal");
    let repeated = K2DecisionEpisodeSealV1::derive(
        seal.episode_id_sha256.clone(),
        seal.outcome_root_sha256.clone(),
        seal.terminal_event_root_sha256.clone(),
        seal.final_projection_root_sha256.clone(),
    )
    .expect("repeated seal");
    assert_eq!(seal, repeated);
    assert_ne!(seal.seal_root_sha256, seal.terminal_event_root_sha256);
}

struct TestDirectoryV1 {
    path: PathBuf,
}

impl TestDirectoryV1 {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "nando-k2-goal-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("test directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectoryV1 {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
