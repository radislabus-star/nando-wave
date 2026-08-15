use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nando_operator_learning::{
    K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1, K2CompositionAuthorityBoundaryV1,
    K2InquiryBaselineRequestV1, K2InquiryBaselinesV1, K2InquiryObservationReceiptV1,
    K2InquiryVerifierCommandV1, K2InquiryVerifierReceiptV1, K2InquiryWorkerOutcomeV1,
    K2UncertaintyBatchJournalEventKindV1, K2UncertaintyBatchJournalV1,
    K2UncertaintyBatchPrecommitV1, K2UncertaintyCasePreverificationV1,
    K2UncertaintyCaseVerificationReceiptV1, K2UncertaintyFinalVerifierRequestV1,
    K2UncertaintyGeneratorRequestV1, K2UncertaintyGeneratorResponseV1,
    K2UncertaintyLearnerRequestV1, K2UncertaintyLearnerResponseV1,
    K2UncertaintyPrivateSafetyDispositionV1, K2UncertaintyProbeArtifactsV1,
    K2UncertaintyProbeOutputV1, K2UncertaintyProbeRequestV1, K2UncertaintySafetyReceiptV1,
    K2UncertaintySafetyRequestV1, K2UncertaintyTournamentArtifactsV1, composition_root_v1,
    composition_sha256_file_v1, enumerate_self_formed_probe_frontier_v1,
    materialize_self_formed_probe_files_v1, prepare_self_formed_dispatch_v1,
    preverify_self_formed_case_with_owner_v1, publish_self_formed_probe_output_v1,
    reopen_self_formed_probe_output_v1, run_self_formed_tournament_with_owners_v1,
    self_formed_grammar_root_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn r6_real_owners_precommit_before_isolated_dispatch_and_observation() {
    let environment = TestEnvironment::new();
    let binaries = ProcessBinaries::from_cargo();
    binaries.assert_pairwise_distinct();
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R6 process test");
    let generated: K2UncertaintyGeneratorResponseV1 = run_process(
        &binaries.generator,
        &K2UncertaintyGeneratorRequestV1::development(
            fs::read(seed_path).expect("read development seed"),
            binaries.generator_sha256.clone(),
        )
        .expect("generator request"),
    );

    let mut prepared = Vec::new();
    for (case_sequence, public_case) in generated.public.cases.iter().enumerate() {
        eprintln!("R6 preverify case {}/16", case_sequence + 1);
        let learned: K2UncertaintyLearnerResponseV1 = run_process(
            &binaries.learner,
            &K2UncertaintyLearnerRequestV1::seal(
                public_case.vocabulary.clone(),
                public_case.support.clone(),
                binaries.learner_sha256.clone(),
            )
            .expect("learner request"),
        );
        let probe_request = K2UncertaintyProbeRequestV1::seal(
            public_case.clone(),
            learned.clone(),
            generated.public.split_commitment_root_sha256.clone(),
            binaries.probe_sha256.clone(),
        )
        .expect("probe request");
        let artifact_root = environment
            .root
            .join(format!("frontier-{case_sequence:02}"));
        let (probe_artifacts, probe_output) = if case_sequence == 0 {
            fs::create_dir_all(&artifact_root).expect("create probe output root");
            let receipt: K2UncertaintyProbeArtifactsV1 = run_isolated(
                &binaries.probe,
                &probe_request,
                &[Mount::ReadWrite(&artifact_root, "/out")],
                120,
            );
            let output = reopen_self_formed_probe_output_v1(&artifact_root, &receipt)
                .expect("reopen process-owned frontier");
            (receipt, output)
        } else {
            let output =
                enumerate_self_formed_probe_frontier_v1(&probe_request).expect("complete frontier");
            let receipt = publish_self_formed_probe_output_v1(&artifact_root, &output)
                .expect("publish complete frontier");
            (receipt, output)
        };

        let tournament = run_self_formed_tournament_with_owners_v1(
            public_case,
            &learned,
            &probe_output,
            &generated.public.split_commitment_root_sha256,
            K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
            &binaries.selector_sha256,
            &binaries.baseline_sha256,
            &mut |request| Ok(run_process(&binaries.selector, request)),
            &mut |request: &K2InquiryBaselineRequestV1| {
                Ok(run_process::<_, K2InquiryBaselinesV1>(
                    &binaries.baseline,
                    request,
                ))
            },
        )
        .expect("process-owned tournament");
        let preverification = preverify_self_formed_case_with_owner_v1(
            &tournament,
            &probe_artifacts,
            &binaries.baseline_sha256,
            &binaries.preverifier_sha256,
            &mut |command: &K2InquiryVerifierCommandV1| {
                Ok(run_process::<_, K2InquiryVerifierReceiptV1>(
                    &binaries.preverifier,
                    command,
                ))
            },
        )
        .expect("process-owned preverification");
        prepared.push(PreparedCase {
            probe_request,
            probe_output,
            probe_artifacts,
            tournament,
            preverification,
        });
    }
    eprintln!("R6 all-case batch precommit complete");

    let execution_order = generated
        .public
        .cases
        .iter()
        .map(|case| case.vocabulary.case_id_sha256.clone())
        .collect::<Vec<_>>();
    let batch = K2UncertaintyBatchPrecommitV1::seal(
        generated.public.experiment_id_sha256.clone(),
        generated
            .private
            .expected_denominator_commitment_sha256
            .clone(),
        prepared
            .iter()
            .map(|case| case.preverification.clone())
            .collect(),
        execution_order.clone(),
    )
    .expect("all-case batch precommit");
    let journal_root = environment.root.join("journal");
    let mut journal = K2UncertaintyBatchJournalV1::create(
        &journal_root,
        generated.public.experiment_id_sha256.clone(),
        execution_order,
    )
    .expect("create batch journal");
    let model_sets_root = composition_root_v1(
        &prepared
            .iter()
            .map(|case| case.preverification.tournament.case_id_sha256.as_str())
            .collect::<Vec<_>>(),
    )
    .expect("model sets payload root");
    let probe_sets_root = composition_root_v1(
        &prepared
            .iter()
            .map(|case| case.probe_artifacts.artifacts_root_sha256.as_str())
            .collect::<Vec<_>>(),
    )
    .expect("probe sets payload root");
    let selections_root = composition_root_v1(
        &prepared
            .iter()
            .map(|case| case.tournament.tournament.tournament_root_sha256.as_str())
            .collect::<Vec<_>>(),
    )
    .expect("selections payload root");
    for (kind, owner, request, payload) in [
        (
            K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
            binaries.coordinator_sha256.clone(),
            root("batch-freeze-request"),
            root("batch-freeze-payload"),
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::CasesGenerated,
            binaries.generator_sha256.clone(),
            generated.generator_request_root_sha256.clone(),
            generated.response_root_sha256.clone(),
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::ModelSetsFrozen,
            binaries.learner_sha256.clone(),
            root("model-sets-request"),
            model_sets_root,
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::ProbeSetsFrozen,
            binaries.probe_sha256.clone(),
            root("probe-sets-request"),
            probe_sets_root,
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::SelectionsFrozen,
            binaries.selector_sha256.clone(),
            root("selections-request"),
            selections_root,
        ),
        (
            K2UncertaintyBatchJournalEventKindV1::AllCasesPrecommitted,
            binaries.preverifier_sha256.clone(),
            root("all-cases-precommit-request"),
            batch.batch_root_sha256.clone(),
        ),
    ] {
        journal
            .append(kind, None, owner, request, payload)
            .expect("append batch barrier event");
    }
    let reopened = K2UncertaintyBatchJournalV1::open_existing(
        &journal_root,
        generated.public.experiment_id_sha256.clone(),
    )
    .expect("reopen all-case barrier");
    assert_eq!(reopened.projection(), journal.projection());

    let mut final_receipts = Vec::new();
    for case_sequence in 0..generated.public.cases.len() {
        eprintln!("R7 execute case {}/16", case_sequence + 1);
        final_receipts.push(execute_precommitted_case(
            &environment,
            &binaries,
            &generated,
            &batch,
            &prepared[case_sequence],
            &mut journal,
            case_sequence,
        ));
    }
    assert_eq!(final_receipts.len(), 16);
    assert_eq!(journal.projection().completed_cases, 16);
    assert_eq!(
        K2UncertaintyBatchJournalV1::open_existing(
            &journal_root,
            generated.public.experiment_id_sha256.clone(),
        )
        .expect("restart after all observations")
        .projection(),
        journal.projection()
    );
}

#[allow(clippy::too_many_arguments)]
fn execute_precommitted_case(
    environment: &TestEnvironment,
    binaries: &ProcessBinaries,
    generated: &K2UncertaintyGeneratorResponseV1,
    batch: &K2UncertaintyBatchPrecommitV1,
    prepared_case: &PreparedCase,
    journal: &mut K2UncertaintyBatchJournalV1,
    case_sequence: usize,
) -> K2UncertaintyCaseVerificationReceiptV1 {
    let public_case = &generated.public.cases[case_sequence];
    let private_case = generated
        .private
        .cases
        .iter()
        .find(|case| case.case_id_sha256 == public_case.vocabulary.case_id_sha256)
        .expect("private case");
    let selected_probe = prepared_case
        .probe_output
        .pages
        .iter()
        .flat_map(|page| &page.dispositions)
        .find(|disposition| {
            disposition.probe.probe_root_sha256
                == prepared_case
                    .preverification
                    .tournament
                    .tournament_winner_probe_root_sha256
        })
        .map(|disposition| disposition.probe.clone())
        .expect("selected complete-frontier probe");
    let resolved_effect = private_case
        .mapping
        .iter()
        .find(|entry| entry.opaque_action_root_sha256 == selected_probe.action_id_sha256)
        .map(|entry| entry.effect.clone())
        .expect("private selected effect");
    let workspace = environment
        .root
        .join(format!("sandbox-work-{case_sequence:02}"));
    fs::create_dir_all(&workspace).expect("create sandbox workspace");
    let safety_request = K2UncertaintySafetyRequestV1::seal(
        prepared_case.preverification.receipt_root_sha256.clone(),
        selected_probe.clone(),
        resolved_effect,
        public_case.vocabulary.clone(),
        self_formed_grammar_root_v1(&public_case.vocabulary).expect("grammar root"),
        composition_root_v1(&workspace.to_string_lossy().as_ref()).expect("sandbox root"),
        binaries.safety_sha256.clone(),
    )
    .expect("private safety request");
    let safety_receipt: K2UncertaintySafetyReceiptV1 =
        run_isolated(&binaries.safety, &safety_request, &[], 20);
    assert_eq!(
        safety_receipt.disposition,
        K2UncertaintyPrivateSafetyDispositionV1::Pass
    );
    let dispatch = prepare_self_formed_dispatch_v1(
        batch,
        &prepared_case.preverification,
        &journal.projection(),
        public_case,
        private_case,
        &selected_probe,
        &safety_request,
        &safety_receipt,
        &binaries.worker_sha256,
        &binaries.observer_sha256,
    )
    .expect("fail-closed dispatch preparation");
    for (relative_path, bytes) in
        materialize_self_formed_probe_files_v1(public_case, &selected_probe)
            .expect("materialize selected state")
    {
        let path = workspace.join(relative_path);
        fs::create_dir_all(path.parent().expect("workspace file parent"))
            .expect("create workspace parent");
        fs::write(path, bytes).expect("write workspace input");
    }
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ProbeDispatched,
            Some(private_case.case_id_sha256.clone()),
            binaries.coordinator_sha256.clone(),
            dispatch.worker_request.request_root_sha256.clone(),
            dispatch.receipt.receipt_root_sha256.clone(),
        )
        .expect("durable dispatch");
    let worker_outcome: K2InquiryWorkerOutcomeV1 = run_isolated(
        &binaries.worker,
        &dispatch.worker_request,
        &[Mount::ReadWrite(&workspace, "/work")],
        20,
    );
    let observation: K2InquiryObservationReceiptV1 = run_isolated(
        &binaries.observer,
        &dispatch.observer_request,
        &[Mount::ReadOnly(&workspace, "/work")],
        20,
    );
    let mut resealed_worker = worker_outcome.clone();
    resealed_worker.reseal().expect("reseal worker outcome");
    assert_eq!(resealed_worker, worker_outcome);
    let mut resealed_observation = observation.clone();
    resealed_observation.reseal().expect("reseal observation");
    assert_eq!(resealed_observation, observation);
    assert_eq!(worker_outcome.post_manifest, observation.post_manifest);
    assert_eq!(
        worker_outcome.selected_probe_root_sha256,
        observation.selected_probe_root_sha256
    );
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ProbeObserved,
            Some(private_case.case_id_sha256.clone()),
            binaries.observer_sha256.clone(),
            dispatch.observer_request.request_root_sha256.clone(),
            observation.receipt_root_sha256.clone(),
        )
        .expect("durable observation");
    let final_request = K2UncertaintyFinalVerifierRequestV1::seal(
        binaries.final_verifier_sha256.clone(),
        prepared_case.probe_request.clone(),
        prepared_case.probe_artifacts.clone(),
        prepared_case.preverification.clone(),
        private_case.clone(),
        safety_request,
        safety_receipt,
        dispatch.receipt,
        dispatch.worker_request,
        dispatch.observer_request,
        worker_outcome,
        observation,
    )
    .expect("final verifier request");
    let final_request_bytes = uncertainty_bytes_v1(&final_request).expect("final request bytes");
    assert!(final_request_bytes.len() <= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1);
    eprintln!(
        "R7 final verifier request bytes case {}: {}",
        case_sequence + 1,
        final_request_bytes.len()
    );
    let artifact_root = environment
        .root
        .join(format!("frontier-{case_sequence:02}"));
    let final_receipt: K2UncertaintyCaseVerificationReceiptV1 = run_isolated(
        &binaries.final_verifier,
        &final_request,
        &[Mount::ReadOnly(&artifact_root, "/evidence")],
        120,
    );
    let mut expected_final_receipt = K2UncertaintyCaseVerificationReceiptV1 {
        schema: K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V1.to_owned(),
        verifier_executable_sha256: binaries.final_verifier_sha256.clone(),
        verifier_request_root_sha256: final_request.request_root_sha256.clone(),
        case_id_sha256: private_case.case_id_sha256.clone(),
        consistency_dispositions: 336,
        materialized_models: 4,
        semantic_signature_outcomes: 7_168,
        raw_probe_dispositions: 1_792,
        raw_predictions: 7_168,
        representative_count: prepared_case
            .preverification
            .tournament
            .representative_count,
        tournament_requests: prepared_case.preverification.tournament.request_count,
        adapted_predictions: prepared_case
            .preverification
            .tournament
            .adapted_prediction_count,
        safety_verified: true,
        worker_observer_match: true,
        surviving_semantic_classes: 1,
        private_true_class_match: true,
        selected_outcome_precommitted: true,
        false_accepts: 0,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    expected_final_receipt
        .reseal()
        .expect("expected final verifier receipt");
    assert_eq!(final_receipt, expected_final_receipt);
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ModelsUpdated,
            Some(private_case.case_id_sha256.clone()),
            binaries.final_verifier_sha256.clone(),
            final_request.request_root_sha256,
            final_receipt.receipt_root_sha256.clone(),
        )
        .expect("durable model update");
    fs::remove_dir_all(&workspace).expect("remove disposable workspace");
    assert!(!workspace.exists());
    final_receipt
}

struct PreparedCase {
    probe_request: K2UncertaintyProbeRequestV1,
    probe_output: K2UncertaintyProbeOutputV1,
    probe_artifacts: K2UncertaintyProbeArtifactsV1,
    tournament: K2UncertaintyTournamentArtifactsV1,
    preverification: K2UncertaintyCasePreverificationV1,
}

struct ProcessBinaries {
    generator: PathBuf,
    learner: PathBuf,
    probe: PathBuf,
    selector: PathBuf,
    baseline: PathBuf,
    preverifier: PathBuf,
    safety: PathBuf,
    worker: PathBuf,
    observer: PathBuf,
    final_verifier: PathBuf,
    generator_sha256: String,
    learner_sha256: String,
    probe_sha256: String,
    selector_sha256: String,
    baseline_sha256: String,
    preverifier_sha256: String,
    safety_sha256: String,
    worker_sha256: String,
    observer_sha256: String,
    final_verifier_sha256: String,
    coordinator_sha256: String,
}

impl ProcessBinaries {
    fn from_cargo() -> Self {
        let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
        let learner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-learner"));
        let probe = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-probe"));
        let selector = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-selector"));
        let baseline = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-baseline"));
        let preverifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-verifier"));
        let safety = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-safety"));
        let worker = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-worker"));
        let observer = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-observer"));
        let final_verifier =
            PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-final-verifier"));
        let coordinator = std::env::current_exe().expect("coordinator executable");
        Self {
            generator_sha256: sha(&generator),
            learner_sha256: sha(&learner),
            probe_sha256: sha(&probe),
            selector_sha256: sha(&selector),
            baseline_sha256: sha(&baseline),
            preverifier_sha256: sha(&preverifier),
            safety_sha256: sha(&safety),
            worker_sha256: sha(&worker),
            observer_sha256: sha(&observer),
            final_verifier_sha256: sha(&final_verifier),
            coordinator_sha256: sha(&coordinator),
            generator,
            learner,
            probe,
            selector,
            baseline,
            preverifier,
            safety,
            worker,
            observer,
            final_verifier,
        }
    }

    fn assert_pairwise_distinct(&self) {
        let roots = [
            &self.generator_sha256,
            &self.learner_sha256,
            &self.probe_sha256,
            &self.selector_sha256,
            &self.baseline_sha256,
            &self.preverifier_sha256,
            &self.safety_sha256,
            &self.worker_sha256,
            &self.observer_sha256,
            &self.final_verifier_sha256,
            &self.coordinator_sha256,
        ];
        assert_eq!(
            roots.iter().copied().collect::<BTreeSet<_>>().len(),
            roots.len()
        );
    }
}

enum Mount<'a> {
    ReadOnly(&'a Path, &'static str),
    ReadWrite(&'a Path, &'static str),
}

fn run_isolated<I, O>(executable: &Path, input: &I, mounts: &[Mount<'_>], cpu_seconds: u64) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let guest = "/nando/bin/process";
    let mut command = Command::new("/usr/bin/bwrap");
    command.args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--cap-drop",
        "ALL",
        "--clearenv",
    ]);
    for path in ["/usr", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            command.args(["--ro-bind", path, path]);
        }
    }
    command
        .args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"])
        .args(["--dir", "/nando", "--dir", "/nando/bin"])
        .arg("--ro-bind")
        .arg(executable)
        .arg(guest);
    for mount in mounts {
        match mount {
            Mount::ReadOnly(host, guest) => {
                command.arg("--ro-bind").arg(host).arg(guest);
            }
            Mount::ReadWrite(host, guest) => {
                command.arg("--bind").arg(host).arg(guest);
            }
        }
    }
    command
        .args(["--setenv", "HOME", "/tmp", "--setenv", "LANG", "C"])
        .args(["--", "/usr/bin/prlimit"])
        .arg(format!("--cpu={cpu_seconds}:{cpu_seconds}"))
        .args(["--as=536870912:536870912", "--nproc=32:32"])
        .args(["--fsize=33554432:33554432", "--", guest])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_child(command, input, Duration::from_secs(cpu_seconds + 10))
}

fn run_process<I, O>(executable: &Path, input: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run_child(command, input, Duration::from_secs(120))
}

fn run_child<I, O>(mut command: Command, input: &I, timeout: Duration) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut child = command.spawn().expect("spawn owner process");
    child
        .stdin
        .take()
        .expect("owner stdin")
        .write_all(&uncertainty_bytes_v1(input).expect("owner request bytes"))
        .expect("write owner request");
    let mut stdout = child.stdout.take().expect("owner stdout");
    let mut stderr = child.stderr.take().expect("owner stderr");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).expect("read owner stdout");
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).expect("read owner stderr");
        bytes
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll owner process") {
            break status;
        }
        assert!(Instant::now() < deadline, "owner process timed out");
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = stdout_reader.join().expect("join owner stdout");
    let stderr = stderr_reader.join().expect("join owner stderr");
    assert!(
        status.success(),
        "owner process failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    uncertainty_decode_v1(&stdout).expect("canonical owner response")
}

struct TestEnvironment {
    root: PathBuf,
}

impl TestEnvironment {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-self-formed-r6-process-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create R6 process root");
        Self { root }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn sha(path: &Path) -> String {
    composition_sha256_file_v1(path).expect("executable hash")
}

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r6-process-test.v1", label)).expect("test root")
}
