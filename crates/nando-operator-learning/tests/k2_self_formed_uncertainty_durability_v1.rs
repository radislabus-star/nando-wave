use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_learning::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2UncertaintyArtifactFaultV1,
    K2UncertaintyBatchJournalEventKindV1, K2UncertaintyBatchJournalFaultV1,
    K2UncertaintyBatchJournalV1, K2UncertaintyGeneratorRequestV1, K2UncertaintyLearnerRequestV1,
    K2UncertaintyProbeRequestV1, composition_root_v1, enumerate_self_formed_probe_frontier_v1,
    generate_self_formed_development_batch_v1, learn_self_formed_uncertainty_v1,
    publish_self_formed_probe_output_v1, publish_self_formed_probe_output_with_fault_v1,
    reopen_self_formed_probe_output_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn r6_paged_artifacts_and_batch_journal_are_atomic_and_restart_exact() {
    let environment = TestEnvironment::new();
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R6 test");
    let generated = generate_self_formed_development_batch_v1(
        &K2UncertaintyGeneratorRequestV1::development(
            fs::read(seed_path).expect("read development seed"),
            root("generator"),
        )
        .expect("generator request"),
    )
    .expect("development batch");
    let public_case = &generated.public.cases[0];
    let learned = learn_self_formed_uncertainty_v1(
        &K2UncertaintyLearnerRequestV1::seal(
            public_case.vocabulary.clone(),
            public_case.support.clone(),
            root("learner"),
        )
        .expect("learner request"),
    )
    .expect("learner response");
    let output = enumerate_self_formed_probe_frontier_v1(
        &K2UncertaintyProbeRequestV1::seal(
            public_case.clone(),
            learned,
            generated.public.split_commitment_root_sha256.clone(),
            root("probe"),
        )
        .expect("probe request"),
    )
    .expect("probe output");

    let artifact_root = environment.root.join("artifacts");
    let receipt = publish_self_formed_probe_output_v1(&artifact_root, &output)
        .expect("publish paged probe artifacts");
    receipt.validate().expect("valid artifact receipt");
    assert!(
        receipt
            .entries
            .iter()
            .all(|entry| entry.byte_len <= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64)
    );
    for entry in &receipt.entries {
        assert_eq!(
            fs::metadata(artifact_root.join(&entry.relative_path))
                .expect("artifact metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    let reopened = reopen_self_formed_probe_output_v1(&artifact_root, &receipt)
        .expect("reopen paged probe output");
    assert_eq!(reopened, output);

    let fault_root = environment.root.join("artifact-fault");
    assert!(
        publish_self_formed_probe_output_with_fault_v1(
            &fault_root,
            &output,
            K2UncertaintyArtifactFaultV1::BeforeRename(1),
        )
        .is_err()
    );
    assert!(fault_root.join("state-universe.json").is_file());
    assert!(!fault_root.join("frontier.json").exists());
    assert!(!fault_root.join(".frontier.json.tmp").exists());

    let execution_order = generated
        .public
        .cases
        .iter()
        .map(|case| case.vocabulary.case_id_sha256.clone())
        .collect::<Vec<_>>();
    let experiment = generated.public.experiment_id_sha256.clone();
    let journal_root = environment.root.join("journal");
    let mut journal = K2UncertaintyBatchJournalV1::create(
        &journal_root,
        experiment.clone(),
        execution_order.clone(),
    )
    .expect("create batch journal");
    for (sequence, kind) in [
        K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
        K2UncertaintyBatchJournalEventKindV1::CasesGenerated,
        K2UncertaintyBatchJournalEventKindV1::ModelSetsFrozen,
        K2UncertaintyBatchJournalEventKindV1::ProbeSetsFrozen,
        K2UncertaintyBatchJournalEventKindV1::SelectionsFrozen,
        K2UncertaintyBatchJournalEventKindV1::AllCasesPrecommitted,
    ]
    .into_iter()
    .enumerate()
    {
        journal
            .append(
                kind,
                None,
                root("journal-owner"),
                root(&format!("request-{sequence}")),
                root(&format!("payload-{sequence}")),
            )
            .expect("append batch prefix");
        let reopened =
            K2UncertaintyBatchJournalV1::open_existing(&journal_root, experiment.clone())
                .expect("reopen batch prefix");
        assert_eq!(reopened.projection(), journal.projection());
    }
    assert!(journal.projection().all_cases_precommitted);
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ProbeDispatched,
            Some(execution_order[0].clone()),
            root("dispatch-owner"),
            root("dispatch-request"),
            root("dispatch-payload"),
        )
        .expect("durable dispatch");
    assert_eq!(
        journal.projection().indeterminate_dispatch_case_id_sha256,
        Some(execution_order[0].clone())
    );
    assert!(
        journal
            .append(
                K2UncertaintyBatchJournalEventKindV1::ProbeDispatched,
                Some(execution_order[0].clone()),
                root("dispatch-owner"),
                root("duplicate-request"),
                root("duplicate-payload"),
            )
            .is_err()
    );
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ProbeObserved,
            Some(execution_order[0].clone()),
            root("observer-owner"),
            root("observer-request"),
            root("observer-payload"),
        )
        .expect("durable observation");
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ModelsUpdated,
            Some(execution_order[0].clone()),
            root("verifier-owner"),
            root("verifier-request"),
            root("verifier-payload"),
        )
        .expect("durable update");
    assert_eq!(journal.projection().completed_cases, 1);
    assert_eq!(
        K2UncertaintyBatchJournalV1::open_existing(&journal_root, experiment)
            .expect("final restart")
            .projection(),
        journal.projection()
    );

    assert_journal_fault_boundaries(&environment.root, &execution_order);
}

fn assert_journal_fault_boundaries(root_path: &std::path::Path, cases: &[String]) {
    let before_root = root_path.join("journal-before");
    let before_id = root("before-experiment");
    let mut before =
        K2UncertaintyBatchJournalV1::create(&before_root, before_id.clone(), cases.to_vec())
            .expect("before journal");
    assert!(
        before
            .append_with_fault(
                K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
                None,
                root("owner"),
                root("request"),
                root("payload"),
                K2UncertaintyBatchJournalFaultV1::BeforeRename,
            )
            .is_err()
    );
    assert_eq!(
        K2UncertaintyBatchJournalV1::open_existing(&before_root, before_id)
            .expect("reopen before fault")
            .projection()
            .event_count,
        0
    );

    let after_root = root_path.join("journal-after");
    let after_id = root("after-experiment");
    let mut after =
        K2UncertaintyBatchJournalV1::create(&after_root, after_id.clone(), cases.to_vec())
            .expect("after journal");
    assert!(
        after
            .append_with_fault(
                K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
                None,
                root("owner"),
                root("request"),
                root("payload"),
                K2UncertaintyBatchJournalFaultV1::AfterRename,
            )
            .is_err()
    );
    assert_eq!(
        K2UncertaintyBatchJournalV1::open_existing(&after_root, after_id)
            .expect("reopen after fault")
            .projection()
            .event_count,
        1
    );
}

struct TestEnvironment {
    root: PathBuf,
}

impl TestEnvironment {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-self-formed-r6-durability-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create test root");
        Self { root }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r6-durability-test.v1", label)).expect("test root")
}
