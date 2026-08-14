use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_kernel::canonical_json_sha256;
use nando_operator_learning::{
    K2AlternativePredictionSetV1, K2AlternativeSetV1, K2AuthorityBoundaryV1,
    K2DecisionFreezeInputV1, K2DecisionFreezeV1, K2DecisionOutcomeReceiptV1, K2EpisodeEventKindV1,
    K2EpisodeJournalV1, K2EpisodeStateV1, K2EvidenceProvenanceV1, K2ExactGoalEvaluationInputV1,
    K2ExactGoalReceiptV1, K2ExactOracleManifestV1, K2ExactOracleOutcomeV1, K2ExactOracleRequestV1,
    K2GoalEnvelopeV1, K2GoalEnvironmentBudgetV1, K2K1ActionRefInputV1, K2K1ActionRefV1,
    K2K1VocabularySnapshotV1, K2LawLabBindingInputV1, K2LawLabBindingV1,
    K2PreparedSelectionReceiptV1, LAW_LAB_MAX_INPUT_BYTES_V1, LawLabProbeDomainV1,
    LawLabSandboxAdapterV1, LawLabSandboxConfigV1, LawLabSandboxOperationV1,
    LawLabSandboxPurposeV1, LawLabSandboxRequestInputV1, LawLabSandboxRequestV1,
    LawLabTreeManifestV1, law_lab_sha256_file_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
#[ignore = "requires Linux bwrap and both Law Lab binaries on the mini-PC"]
fn exact_goal_capability_episode_is_isolated_durable_and_authority_free() {
    let fixture = CapabilityFixtureV1::new();
    let worker_path = PathBuf::from(env!("CARGO_BIN_EXE_nando-law-lab-sandbox-worker"));
    let oracle_path = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-exact-goal-oracle"));
    let selector_path = std::env::current_exe().expect("selector test executable");
    let worker_sha256 = law_lab_sha256_file_v1(&worker_path).expect("worker sha");
    let oracle_sha256 = law_lab_sha256_file_v1(&oracle_path).expect("oracle sha");
    let selector_sha256 = law_lab_sha256_file_v1(&selector_path).expect("selector sha");
    assert_ne!(oracle_sha256, worker_sha256);
    assert_ne!(oracle_sha256, selector_sha256);
    assert_ne!(worker_sha256, selector_sha256);

    let adapter =
        LawLabSandboxAdapterV1::new(LawLabSandboxConfigV1::generated_capability_self_test_v1(
            worker_path,
            worker_sha256.clone(),
            fixture.source_store.clone(),
            fixture.workspace_store.clone(),
        ));
    let executor = adapter.executor_manifest().expect("executor manifest");
    assert_eq!(executor.worker_sha256, worker_sha256);

    let oracle_manifest = K2ExactOracleManifestV1::seal(oracle_sha256).expect("oracle manifest");
    let goal = K2GoalEnvelopeV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        fixture.source_manifest.tree_root_sha256.clone(),
        fixture.expected_manifest.tree_root_sha256.clone(),
        canonical_json_sha256(&fixture.expected_manifest).expect("goal store snapshot"),
        root("capability-constraints"),
        oracle_manifest.manifest_root_sha256.clone(),
        1_786_500_100_000,
    )
    .expect("goal");

    let selected_operations = vec![LawLabSandboxOperationV1::CopySourceFile {
        source_path: "input.txt".to_owned(),
        work_path: "selected.txt".to_owned(),
    }];
    let alternate_operations = vec![LawLabSandboxOperationV1::RemoveWorkPath {
        work_path: "input.txt".to_owned(),
    }];
    let selected_action = fixture_action(
        &fixture.source_manifest.tree_root_sha256,
        "copy-selected",
        &selected_operations,
        fixture.expected_manifest.tree_root_sha256.clone(),
    );
    let alternate_action = fixture_action(
        &fixture.source_manifest.tree_root_sha256,
        "remove-input",
        &alternate_operations,
        root("alternate-terminal-tree"),
    );
    let vocabulary = K2K1VocabularySnapshotV1::seal(
        K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        None,
        None,
        vec![selected_action, alternate_action],
        1_786_500_100_001,
    )
    .expect("vocabulary");
    let alternatives = K2AlternativeSetV1::seal(
        &vocabulary,
        fixture.source_manifest.tree_root_sha256.clone(),
    )
    .expect("alternatives");
    let budget = K2GoalEnvironmentBudgetV1::preregistered_v1();
    let freeze = K2DecisionFreezeV1::seal(K2DecisionFreezeInputV1 {
        episode_id_sha256: root("capability-episode"),
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        budget,
        selector_contract_root_sha256: root("prepared-capability-selector-contract"),
        selector_executable_sha256: selector_sha256,
        oracle_manifest: &oracle_manifest,
        sandbox_worker_sha256: worker_sha256,
        deterministic_seed_sha256: root("capability-deterministic-seed"),
        observed_registry_revision: None,
        observed_registry_root_sha256: None,
        frozen_at_unix_ms: 1_786_500_100_002,
    })
    .expect("decision freeze");
    let predictions = K2AlternativePredictionSetV1::prepared_capability_v1(
        &freeze,
        &goal,
        &vocabulary,
        &alternatives,
        &budget,
        &oracle_manifest,
    )
    .expect("prediction precommit");
    let selection = K2PreparedSelectionReceiptV1::select(&freeze, &predictions).expect("selection");
    let selected = alternatives
        .alternative(&selection.selected_action_root_sha256)
        .expect("selected alternative");
    assert_eq!(
        selected.operation_plan_root_sha256,
        canonical_json_sha256(&selected_operations).expect("selected operation root")
    );

    let request = LawLabSandboxRequestV1::seal(LawLabSandboxRequestInputV1 {
        executor_manifest_root_sha256: executor.manifest_root_sha256,
        worker_sha256: executor.worker_sha256,
        candidate_root_sha256: freeze.episode_id_sha256.clone(),
        version_space_root_sha256: alternatives.alternative_set_root_sha256.clone(),
        durable_prediction_ledger_root_sha256: predictions.prediction_set_root_sha256.clone(),
        probe_root_sha256: selection.selection_root_sha256.clone(),
        source_tree_root_sha256: fixture.source_manifest.tree_root_sha256.clone(),
        deterministic_seed_sha256: freeze.deterministic_seed_sha256.clone(),
        domain: LawLabProbeDomainV1::Filesystem,
        purpose: LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest,
        surviving_hypothesis_count: alternatives.alternatives.len() as u64,
        precommitted_prediction_count: predictions.predictions.len() as u64,
        operations: selected_operations,
    })
    .expect("sandbox request");
    let binding = K2LawLabBindingV1::seal(K2LawLabBindingInputV1 {
        freeze: &freeze,
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        predictions: &predictions,
        selection: &selection,
        request: &request,
    })
    .expect("K2 sandbox binding");

    let mut journal =
        K2EpisodeJournalV1::create(&fixture.journal_store, freeze.episode_id_sha256.clone())
            .expect("episode journal");
    journal
        .append(K2EpisodeEventKindV1::ContractFrozen, &freeze, 1)
        .expect("contract durable");
    journal
        .append(
            K2EpisodeEventKindV1::PredictionsPrecommitted,
            &predictions,
            2,
        )
        .expect("predictions durable");
    journal
        .append(K2EpisodeEventKindV1::ProbePlanned, &binding, 3)
        .expect("plan durable");
    assert!(journal.projection().same_identity_execution_allowed);
    journal
        .append(
            K2EpisodeEventKindV1::ProbeDispatched,
            &binding.binding_root_sha256,
            4,
        )
        .expect("dispatch durable");
    assert!(!journal.projection().same_identity_execution_allowed);

    let execution = adapter
        .execute(&request)
        .expect("isolated sandbox execution");
    journal
        .append(K2EpisodeEventKindV1::ProbeExecuted, &execution.receipt, 5)
        .expect("execution durable");
    let oracle_request =
        K2ExactOracleRequestV1::seal(&goal, &freeze, &binding, &execution, &oracle_manifest)
            .expect("oracle request");
    let oracle_outcome = execute_exact_oracle(&oracle_path, &oracle_request);
    let exact_goal = K2ExactGoalReceiptV1::evaluate(K2ExactGoalEvaluationInputV1 {
        freeze: &freeze,
        goal: &goal,
        vocabulary: &vocabulary,
        alternatives: &alternatives,
        predictions: &predictions,
        selection: &selection,
        binding: &binding,
        request: &request,
        execution: &execution,
        oracle_manifest: &oracle_manifest,
        oracle_request: &oracle_request,
        oracle_outcome: &oracle_outcome,
    })
    .expect("exact goal receipt");
    assert!(exact_goal.goal_satisfied);
    journal
        .append(K2EpisodeEventKindV1::OutcomeVerified, &exact_goal, 6)
        .expect("oracle durable");
    let outcome = K2DecisionOutcomeReceiptV1::capability_pass(
        &freeze,
        &predictions,
        &binding,
        &execution,
        &exact_goal,
    )
    .expect("capability outcome");
    let terminal_event = journal
        .append(K2EpisodeEventKindV1::Terminal, &outcome, 7)
        .expect("terminal durable");
    let seal = journal
        .derive_terminal_seal(&outcome)
        .expect("episode seal");

    assert_eq!(journal.projection().state, K2EpisodeStateV1::Terminal);
    assert_eq!(journal.projection().event_count, 7);
    assert_ne!(
        terminal_event.event_payload_root_sha256,
        outcome.outcome_root_sha256
    );
    assert_ne!(seal.seal_root_sha256, terminal_event.entry_root_sha256);
    assert_authority_free(&outcome.authority);
    assert_authority_free(&seal.authority);
    assert_eq!(
        execution
            .worker_outcome
            .isolation
            .ipv4_non_loopback_route_entries,
        0
    );
    assert_eq!(
        execution
            .worker_outcome
            .isolation
            .ipv6_non_loopback_route_entries,
        0
    );
    assert!(execution.worker_outcome.isolation.source_write_blocked);
    assert!(execution.receipt.cleanup.verified_absent);
    assert!(workspace_is_empty(&fixture.workspace_store));

    let reopened =
        K2EpisodeJournalV1::open_existing(&fixture.journal_store, freeze.episode_id_sha256)
            .expect("restart projection");
    assert_eq!(reopened.projection(), journal.projection());
    assert_eq!(
        reopened
            .derive_terminal_seal(&outcome)
            .expect("restart seal"),
        seal
    );
    eprintln!(
        "K2_GOAL_ENVIRONMENT_CAPABILITY_PASS outcome={} seal={} events=7 authority=false",
        outcome.outcome_root_sha256, seal.seal_root_sha256
    );
}

fn fixture_action(
    environment_root_sha256: &str,
    label: &str,
    operations: &[LawLabSandboxOperationV1],
    predicted_consequence_root_sha256: String,
) -> K2K1ActionRefV1 {
    K2K1ActionRefV1::seal(K2K1ActionRefInputV1 {
        provenance: K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
        applicability_environment_root_sha256: environment_root_sha256.to_owned(),
        applicability_receipt_root_sha256: root(&format!("applicability:{label}")),
        operation_plan_root_sha256: canonical_json_sha256(&operations).expect("operation root"),
        predicted_consequence_root_sha256,
        fixture_effect_root_sha256: Some(root(&format!("fixture-effect:{label}"))),
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

fn execute_exact_oracle(
    oracle_path: &Path,
    request: &K2ExactOracleRequestV1,
) -> K2ExactOracleOutcomeV1 {
    let mut child = Command::new(oracle_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn exact oracle");
    child
        .stdin
        .take()
        .expect("oracle stdin")
        .write_all(&request.canonical_bytes().expect("oracle request bytes"))
        .expect("write oracle request");
    let output = child.wait_with_output().expect("wait exact oracle");
    assert!(
        output.status.success(),
        "oracle stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    K2ExactOracleOutcomeV1::from_canonical_bytes(&output.stdout, request)
        .expect("canonical oracle outcome")
}

fn assert_authority_free(authority: &K2AuthorityBoundaryV1) {
    authority.validate().expect("authority false");
    assert!(!authority.law_certificate_issued);
    assert!(!authority.execution_authority_granted);
    assert!(!authority.k1_registry_mutated);
    assert!(!authority.k2_claim_granted);
    assert!(!authority.phase_memory_mutated);
    assert!(!authority.natural_holdout_satisfied);
}

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

fn workspace_is_empty(path: &Path) -> bool {
    fs::read_dir(path)
        .expect("workspace store")
        .next()
        .is_none()
}

struct CapabilityFixtureV1 {
    root: PathBuf,
    source_store: PathBuf,
    workspace_store: PathBuf,
    journal_store: PathBuf,
    source_manifest: LawLabTreeManifestV1,
    expected_manifest: LawLabTreeManifestV1,
}

impl CapabilityFixtureV1 {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let parent = std::env::var_os("NANDO_K2_GOAL_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("current directory")
                    .join("target/k2-goal-environment-tests")
            });
        fs::create_dir_all(&parent).expect("test parent");
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("parent mode");
        let root = parent.join(format!("{}-{sequence}", std::process::id()));
        fs::create_dir(&root).expect("fixture root");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).expect("root mode");
        let source_store = root.join("sources");
        let workspace_store = root.join("workspaces");
        let journal_store = root.join("journals");
        let source_staging = root.join("source-staging");
        let expected_store = root.join("expected-goal-store");
        for path in [
            &source_store,
            &workspace_store,
            &journal_store,
            &source_staging,
            &expected_store,
        ] {
            fs::create_dir(path).expect("fixture directory");
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("directory mode");
        }
        write_file(
            &source_staging.join("input.txt"),
            b"grounded capability input",
        );
        let source_manifest =
            LawLabTreeManifestV1::scan(&source_staging, LAW_LAB_MAX_INPUT_BYTES_V1)
                .expect("source manifest");
        fs::rename(
            &source_staging,
            source_store.join(&source_manifest.tree_root_sha256),
        )
        .expect("seal source");

        write_file(
            &expected_store.join("input.txt"),
            b"grounded capability input",
        );
        write_file(
            &expected_store.join("selected.txt"),
            b"grounded capability input",
        );
        let expected_manifest =
            LawLabTreeManifestV1::scan(&expected_store, LAW_LAB_MAX_INPUT_BYTES_V1)
                .expect("expected manifest");
        for entry in fs::read_dir(&expected_store).expect("expected entries") {
            fs::set_permissions(
                entry.expect("expected entry").path(),
                fs::Permissions::from_mode(0o400),
            )
            .expect("expected file read-only");
        }
        fs::set_permissions(&expected_store, fs::Permissions::from_mode(0o500))
            .expect("expected store read-only");
        Self {
            root,
            source_store,
            workspace_store,
            journal_store,
            source_manifest,
            expected_manifest,
        }
    }
}

impl Drop for CapabilityFixtureV1 {
    fn drop(&mut self) {
        let _ = fs::set_permissions(
            self.root.join("expected-goal-store"),
            fs::Permissions::from_mode(0o700),
        );
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_file(path: &Path, bytes: &[u8]) {
    let mut file = File::create(path).expect("fixture file");
    file.write_all(bytes).expect("fixture write");
    file.sync_all().expect("fixture sync");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("fixture file mode");
}
