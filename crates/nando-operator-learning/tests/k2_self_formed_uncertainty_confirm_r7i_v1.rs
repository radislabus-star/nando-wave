use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_learning::{
    K2CompositionTreeManifestV1, K2InquiryObservationReceiptV1, K2InquiryWorkerOutcomeV1,
    K2UncertaintyCaseJournalFaultV2, K2UncertaintyCaseJournalV2, K2UncertaintyConfirmDataMountV1,
    K2UncertaintyConfirmFinalVerifierReceiptV1, K2UncertaintyConfirmFinalVerifierRequestV1,
    K2UncertaintyConfirmGeneratorRequestV1, K2UncertaintyConfirmGeneratorResponseV1,
    K2UncertaintyConfirmGuestExecutableV1, K2UncertaintyConfirmMountTargetV1,
    K2UncertaintyConfirmPlanSafetyBindingV1, K2UncertaintyConfirmPrivateSplitReceiptV1,
    K2UncertaintyConfirmPublicDenominatorReceiptV1, K2UncertaintyConfirmSafetyReceiptV1,
    K2UncertaintyConfirmSafetyRequestV1, K2UncertaintyConfirmStoredArtifactKindV1,
    K2UncertaintyObservationVectorV2, K2UncertaintyPrivateResolverReceiptV1,
    K2UncertaintyPrivateResolverRequestV1, K2UncertaintyProbeExecutionEvidenceV2,
    K2UncertaintyPublicBatchV1, K2UncertaintyPublicCoordinatorRequestV1,
    K2UncertaintyPublicOwnerRoleV1, K2UncertaintyPublicOwnerSetV1, K2UncertaintyPublicOwnerV1,
    K2UncertaintyPublicPrecommitReceiptV1, K2UncertaintyWorkspaceIdentityV2, composition_root_v1,
    composition_sha256_file_v1, load_confirm_generator_split_receipt_v1,
    load_self_formed_public_case_v1, load_self_formed_public_precommit_v1,
    materialize_self_formed_probe_files_v1, prepare_self_formed_confirm_plan_dispatch_v1,
    publish_confirm_generator_split_v1, publish_self_formed_final_verifier_material_v2,
    run_self_formed_confirm_sandbox_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn r7i_public_barrier_precedes_bounded_private_execution_for_all_cases() {
    let environment = TestEnvironment::new("route");
    let binaries = ProcessBinaries::from_cargo();
    binaries.assert_pairwise_distinct();

    let reuse = std::env::var_os("NANDO_K2_R7I_REUSE_FIXTURE_ROOT").map(PathBuf::from);
    let split_root = reuse
        .as_ref()
        .map_or_else(|| environment.root.join("split"), |root| root.join("split"));
    let public_root = reuse.as_ref().map_or_else(
        || environment.root.join("public-precommit"),
        |root| root.join("public-precommit"),
    );
    let (split, public_batch, public_receipt) = if reuse.is_some() {
        (
            load_confirm_generator_split_receipt_v1(&split_root).expect("reopen Confirm split"),
            uncertainty_decode_v1::<K2UncertaintyPublicBatchV1>(
                &fs::read(split_root.join("public/public-batch.json")).expect("read public batch"),
            )
            .expect("decode public batch"),
            load_self_formed_public_precommit_v1(&public_root).expect("reuse public precommit"),
        )
    } else {
        let generator_request = K2UncertaintyConfirmGeneratorRequestV1::seal(
            vec![0xa5; 32],
            root("successor-freeze"),
            root("rehearsal-authorization"),
            binaries.generator.sha256.clone(),
        )
        .expect("confirm rehearsal generator request");
        let generated: K2UncertaintyConfirmGeneratorResponseV1 =
            run_process(&binaries.generator.path, &generator_request);
        let split = publish_confirm_generator_split_v1(&split_root, &generator_request, &generated)
            .expect("publish Confirm rehearsal split");
        let denominator: K2UncertaintyConfirmPublicDenominatorReceiptV1 = uncertainty_decode_v1(
            &fs::read(split_root.join("public/denominator-receipt.json"))
                .expect("read public denominator"),
        )
        .expect("decode public denominator");
        let public_request = K2UncertaintyPublicCoordinatorRequestV1::seal(
            generated.public.clone(),
            denominator,
            binaries.public_owner_set(),
            public_root.to_string_lossy().into_owned(),
            binaries.public_coordinator.sha256.clone(),
        )
        .expect("public coordinator request");
        let receipt: K2UncertaintyPublicPrecommitReceiptV1 =
            run_process(&binaries.public_coordinator.path, &public_request);
        (split, generated.public, receipt)
    };
    let private: K2UncertaintyConfirmPrivateSplitReceiptV1 = uncertainty_decode_v1(
        &fs::read(split_root.join("private/private-split-receipt.json"))
            .expect("read private split receipt"),
    )
    .expect("decode private split receipt");

    assert_eq!(
        load_self_formed_public_precommit_v1(&public_root).expect("reopen public precommit"),
        public_receipt
    );
    assert_eq!(public_receipt.case_artifacts.len(), 16);
    assert_eq!(public_receipt.private_mount_count, 0);
    assert!(public_receipt.all_cases_precommitted);
    assert!(!public_root.join("private").exists());
    for artifact in &public_receipt.case_artifacts {
        let material_root = public_root.join(format!(
            "probes/case-{:02}/final-v2",
            artifact.case_sequence
        ));
        let material_count = fs::read_dir(&material_root)
            .expect("final verifier material precedes private execution")
            .count();
        assert_eq!(material_count, 2);
    }
    assert_eq!(
        split.private_split_root_sha256,
        private.private_split_root_sha256
    );

    let mut plan_lengths = Vec::new();
    for artifact in &public_receipt.case_artifacts {
        let prepared = load_self_formed_public_case_v1(&public_root, artifact)
            .expect("reopen prepared public case");
        let case_id = prepared
            .probe_request
            .public_case
            .vocabulary
            .case_id_sha256
            .clone();
        let resolver_artifact = private
            .artifacts
            .iter()
            .find(|entry| {
                entry.kind == K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable
                    && entry.case_id_sha256.as_deref() == Some(case_id.as_str())
            })
            .expect("case resolver artifact");
        let truth_artifact = private
            .artifacts
            .iter()
            .find(|entry| {
                entry.kind == K2UncertaintyConfirmStoredArtifactKindV1::FinalTruth
                    && entry.case_id_sha256.as_deref() == Some(case_id.as_str())
            })
            .expect("case truth artifact");
        let plan = prepared
            .preverification
            .closure_plan
            .as_ref()
            .expect("publicly frozen closure plan");
        plan_lengths.push(plan.plan_length);
        let planner = &prepared
            .preverification
            .closure_verification_request
            .planner_request;
        let mut bindings = Vec::with_capacity(plan.plan_length as usize);
        for (ordinal, probe_root) in plan.ordered_probe_roots_sha256.iter().enumerate() {
            let selected_probe = planner
                .representatives
                .iter()
                .find(|candidate| &candidate.probe.probe_root_sha256 == probe_root)
                .map(|candidate| candidate.probe.clone())
                .expect("planned representative");
            let resolver_request = K2UncertaintyPrivateResolverRequestV1::seal(
                public_batch.experiment_id_sha256.clone(),
                public_batch.public_batch_root_sha256.clone(),
                public_receipt.batch_precommit.batch_root_sha256.clone(),
                prepared.preverification.receipt_root_sha256.clone(),
                prepared
                    .probe_request
                    .public_case
                    .public_case_root_sha256
                    .clone(),
                plan.clone(),
                ordinal as u64,
                selected_probe,
                resolver_artifact.semantic_root_sha256.clone(),
                binaries.private_resolver.sha256.clone(),
            )
            .expect("private resolver request");
            let resolver_path = split_root.join(&resolver_artifact.relative_path);
            let resolver_mount = [K2UncertaintyConfirmDataMountV1 {
                host_path: &resolver_path,
                target: K2UncertaintyConfirmMountTargetV1::ResolverTable,
                writable: false,
            }];
            eprintln!(
                "R7I resolver case {} ordinal {}",
                artifact.case_sequence, ordinal
            );
            let resolver_receipt: K2UncertaintyPrivateResolverReceiptV1 = run_sandbox(
                K2UncertaintyConfirmGuestExecutableV1::PrivateResolver,
                &binaries.private_resolver,
                &resolver_mount,
                &resolver_request,
                20,
            );
            assert_eq!(resolver_receipt.exposed_effect_count, 1);
            let workspace_identity = K2UncertaintyWorkspaceIdentityV2::seal(
                case_id.clone(),
                plan.plan_root_sha256.clone(),
                ordinal as u64,
            )
            .expect("workspace identity");
            let safety_request = K2UncertaintyConfirmSafetyRequestV1::seal(
                resolver_request.clone(),
                resolver_receipt.clone(),
                prepared.probe_request.public_case.vocabulary.clone(),
                workspace_identity,
                binaries.safety.sha256.clone(),
            )
            .expect("Confirm safety request");
            eprintln!(
                "R7I safety case {} ordinal {}",
                artifact.case_sequence, ordinal
            );
            let safety_receipt: K2UncertaintyConfirmSafetyReceiptV1 = run_sandbox(
                K2UncertaintyConfirmGuestExecutableV1::Safety,
                &binaries.safety,
                &[],
                &safety_request,
                20,
            );
            bindings.push(K2UncertaintyConfirmPlanSafetyBindingV1 {
                resolver_request,
                resolver_receipt,
                safety_request,
                safety_receipt,
            });
        }
        let dispatch = prepare_self_formed_confirm_plan_dispatch_v1(
            &public_batch.public_batch_root_sha256,
            &public_receipt.batch_precommit,
            &prepared,
            bindings,
            &binaries.worker.sha256,
            &binaries.observer.sha256,
        )
        .expect("bounded Confirm dispatch");
        let case_journal_root = environment
            .root
            .join(format!("journal-{}", artifact.case_sequence));
        let mut journal = K2UncertaintyCaseJournalV2::create(&case_journal_root, dispatch.clone())
            .expect("create case journal");
        journal
            .record_plan_dispatch(
                binaries.public_coordinator.sha256.clone(),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("freeze dispatch before worker");
        let mut executions = Vec::with_capacity(dispatch.items.len());
        for item in &dispatch.items {
            let source = environment.root.join(format!(
                "source-{}",
                item.workspace_identity.identity_root_sha256
            ));
            let workspace = environment.root.join(format!(
                "work-{}",
                item.workspace_identity.identity_root_sha256
            ));
            let files = materialize_self_formed_probe_files_v1(
                &prepared.probe_request.public_case,
                &item.selected_probe,
            )
            .expect("materialize selected public state");
            materialize(&source, &files);
            materialize(&workspace, &files);
            let source_before = K2CompositionTreeManifestV1::scan(&source).expect("source before");
            let permit = journal
                .begin_probe_execution(item.probe_ordinal, K2UncertaintyCaseJournalFaultV2::None)
                .expect("begin exact ordinal");
            let worker_mounts = [
                K2UncertaintyConfirmDataMountV1 {
                    host_path: &source,
                    target: K2UncertaintyConfirmMountTargetV1::Source,
                    writable: false,
                },
                K2UncertaintyConfirmDataMountV1 {
                    host_path: &workspace,
                    target: K2UncertaintyConfirmMountTargetV1::Workspace,
                    writable: true,
                },
            ];
            let worker: K2InquiryWorkerOutcomeV1 = run_sandbox(
                K2UncertaintyConfirmGuestExecutableV1::Worker,
                &binaries.worker,
                &worker_mounts,
                &item.worker_request,
                20,
            );
            let observer_mounts = [
                K2UncertaintyConfirmDataMountV1 {
                    host_path: &source,
                    target: K2UncertaintyConfirmMountTargetV1::Source,
                    writable: false,
                },
                K2UncertaintyConfirmDataMountV1 {
                    host_path: &workspace,
                    target: K2UncertaintyConfirmMountTargetV1::Workspace,
                    writable: false,
                },
            ];
            eprintln!(
                "R7I observer case {} ordinal {}",
                artifact.case_sequence, item.probe_ordinal
            );
            let observation: K2InquiryObservationReceiptV1 = run_sandbox(
                K2UncertaintyConfirmGuestExecutableV1::Observer,
                &binaries.observer,
                &observer_mounts,
                &item.observer_request,
                20,
            );
            assert_eq!(worker.post_manifest, observation.post_manifest);
            assert_eq!(
                source_before,
                K2CompositionTreeManifestV1::scan(&source).expect("source after")
            );
            let evidence = K2UncertaintyProbeExecutionEvidenceV2::seal(
                dispatch.dispatch_root_sha256.clone(),
                item,
                worker,
                observation,
            )
            .expect("execution evidence");
            journal
                .record_probe_observation(
                    permit,
                    evidence.observation.receipt_root_sha256.clone(),
                    K2UncertaintyCaseJournalFaultV2::None,
                )
                .expect("freeze observation");
            executions.push(evidence);
        }
        let observation_vector = K2UncertaintyObservationVectorV2::seal(&dispatch, executions)
            .expect("ordered observation vector");
        let vector_request_root = root(&format!("vector-{}", artifact.case_sequence));
        journal
            .freeze_observation_vector(
                binaries.public_coordinator.sha256.clone(),
                vector_request_root,
                observation_vector.vector_root_sha256.clone(),
                K2UncertaintyCaseJournalFaultV2::None,
            )
            .expect("freeze complete observation vector");
        let evidence_root = public_root.join(format!("probes/case-{:02}", artifact.case_sequence));
        let material = publish_self_formed_final_verifier_material_v2(
            &evidence_root,
            &public_receipt.batch_precommit,
            &prepared.preverification,
        )
        .expect("publish final verifier material");
        let final_request = K2UncertaintyConfirmFinalVerifierRequestV1::seal(
            binaries.final_verifier.sha256.clone(),
            material,
            prepared.probe_request.clone(),
            prepared.probe_artifacts.clone(),
            dispatch,
            observation_vector,
            journal.state().clone(),
            truth_artifact.semantic_root_sha256.clone(),
        )
        .expect("Confirm final verifier request");
        let truth_path = split_root.join(&truth_artifact.relative_path);
        let final_mounts = [
            K2UncertaintyConfirmDataMountV1 {
                host_path: &truth_path,
                target: K2UncertaintyConfirmMountTargetV1::FinalTruth,
                writable: false,
            },
            K2UncertaintyConfirmDataMountV1 {
                host_path: &evidence_root,
                target: K2UncertaintyConfirmMountTargetV1::Evidence,
                writable: false,
            },
        ];
        eprintln!("R7I final verifier case {}", artifact.case_sequence);
        let final_receipt: K2UncertaintyConfirmFinalVerifierReceiptV1 = run_sandbox(
            K2UncertaintyConfirmGuestExecutableV1::FinalVerifier,
            &binaries.final_verifier,
            &final_mounts,
            &final_request,
            120,
        );
        final_receipt.validate().expect("Confirm final receipt");
        assert_eq!(
            final_receipt.final_truth_root_sha256,
            truth_artifact.semantic_root_sha256
        );
        assert!(final_receipt.verification.private_true_class_match);
        assert_eq!(final_receipt.verification.false_accepts, 0);
    }
    assert_eq!(
        plan_lengths.iter().filter(|length| **length == 1).count(),
        8
    );
    assert_eq!(
        plan_lengths.iter().filter(|length| **length == 2).count(),
        8
    );
}

#[test]
fn r7i_sandbox_rejects_private_mount_on_public_owner() {
    let environment = TestEnvironment::new("mount-veto");
    let binary = ProcessBinary::new(PathBuf::from(env!(
        "CARGO_BIN_EXE_nando-k2-self-formed-learner"
    )));
    let private = environment.root.join("private.json");
    fs::write(&private, b"{}").expect("private fixture");
    let mounts = [K2UncertaintyConfirmDataMountV1 {
        host_path: &private,
        target: K2UncertaintyConfirmMountTargetV1::ResolverTable,
        writable: false,
    }];
    let error = run_self_formed_confirm_sandbox_v1(
        K2UncertaintyConfirmGuestExecutableV1::Learner,
        &binary.path,
        &binary.sha256,
        &mounts,
        b"{}",
        1,
    )
    .expect_err("public owner must reject private mount");
    assert!(error.to_string().contains("role_mount_matrix_invalid"));
}

fn run_sandbox<I, O>(
    role: K2UncertaintyConfirmGuestExecutableV1,
    binary: &ProcessBinary,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    input: &I,
    cpu_seconds: u64,
) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let bytes = run_self_formed_confirm_sandbox_v1(
        role,
        &binary.path,
        &binary.sha256,
        mounts,
        &uncertainty_bytes_v1(input).expect("sandbox input"),
        cpu_seconds,
    )
    .expect("sandbox process");
    uncertainty_decode_v1(&bytes).expect("sandbox output")
}

fn run_process<I, O>(path: &Path, input: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let mut child = Command::new(path)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn process");
    child
        .stdin
        .take()
        .expect("process stdin")
        .write_all(&uncertainty_bytes_v1(input).expect("process input"))
        .expect("write process input");
    let output = child.wait_with_output().expect("wait process");
    assert!(
        output.status.success(),
        "process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("process output")
}

fn materialize(root: &Path, files: &BTreeMap<String, Vec<u8>>) {
    fs::create_dir_all(root).expect("create materialization root");
    for (relative, bytes) in files {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("materialized parent"))
            .expect("create materialized parent");
        fs::write(path, bytes).expect("write materialized file");
    }
}

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r7i-test.v1", label)).expect("test root")
}

#[derive(Clone)]
struct ProcessBinary {
    path: PathBuf,
    sha256: String,
}

impl ProcessBinary {
    fn new(path: PathBuf) -> Self {
        let sha256 = composition_sha256_file_v1(&path).expect("binary SHA-256");
        Self { path, sha256 }
    }
}

struct ProcessBinaries {
    generator: ProcessBinary,
    learner: ProcessBinary,
    probe: ProcessBinary,
    selector: ProcessBinary,
    baseline: ProcessBinary,
    selection_preverifier: ProcessBinary,
    closure_planner: ProcessBinary,
    closure_verifier: ProcessBinary,
    public_coordinator: ProcessBinary,
    private_resolver: ProcessBinary,
    safety: ProcessBinary,
    worker: ProcessBinary,
    observer: ProcessBinary,
    final_verifier: ProcessBinary,
}

impl ProcessBinaries {
    fn from_cargo() -> Self {
        Self {
            generator: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator")),
            learner: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-learner")),
            probe: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-probe")),
            selector: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-selector")),
            baseline: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-baseline")),
            selection_preverifier: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-verifier")),
            closure_planner: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-planner")),
            closure_verifier: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-verifier")),
            public_coordinator: binary(env!(
                "CARGO_BIN_EXE_nando-k2-self-formed-public-coordinator"
            )),
            private_resolver: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-private-resolver")),
            safety: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-safety")),
            worker: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-worker")),
            observer: binary(env!("CARGO_BIN_EXE_nando-k2-inquiry-observer")),
            final_verifier: binary(env!("CARGO_BIN_EXE_nando-k2-self-formed-final-verifier-v2")),
        }
    }

    fn public_owner_set(&self) -> K2UncertaintyPublicOwnerSetV1 {
        K2UncertaintyPublicOwnerSetV1::seal(vec![
            owner(K2UncertaintyPublicOwnerRoleV1::Learner, &self.learner),
            owner(K2UncertaintyPublicOwnerRoleV1::Probe, &self.probe),
            owner(K2UncertaintyPublicOwnerRoleV1::Selector, &self.selector),
            owner(K2UncertaintyPublicOwnerRoleV1::Baseline, &self.baseline),
            owner(
                K2UncertaintyPublicOwnerRoleV1::SelectionPreverifier,
                &self.selection_preverifier,
            ),
            owner(
                K2UncertaintyPublicOwnerRoleV1::ClosurePlanner,
                &self.closure_planner,
            ),
            owner(
                K2UncertaintyPublicOwnerRoleV1::ClosureVerifier,
                &self.closure_verifier,
            ),
        ])
        .expect("public owner set")
    }

    fn assert_pairwise_distinct(&self) {
        let roots = [
            &self.generator.sha256,
            &self.learner.sha256,
            &self.probe.sha256,
            &self.selector.sha256,
            &self.baseline.sha256,
            &self.selection_preverifier.sha256,
            &self.closure_planner.sha256,
            &self.closure_verifier.sha256,
            &self.public_coordinator.sha256,
            &self.private_resolver.sha256,
            &self.safety.sha256,
            &self.worker.sha256,
            &self.observer.sha256,
            &self.final_verifier.sha256,
        ];
        assert_eq!(
            roots.iter().copied().collect::<BTreeSet<_>>().len(),
            roots.len()
        );
    }
}

fn binary(path: &str) -> ProcessBinary {
    ProcessBinary::new(PathBuf::from(path))
}

fn owner(
    role: K2UncertaintyPublicOwnerRoleV1,
    binary: &ProcessBinary,
) -> K2UncertaintyPublicOwnerV1 {
    K2UncertaintyPublicOwnerV1 {
        role,
        executable_path: binary.path.to_string_lossy().into_owned(),
        executable_sha256: binary.sha256.clone(),
    }
}

struct TestEnvironment {
    root: PathBuf,
}

impl TestEnvironment {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-self-formed-r7i-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create test root");
        Self { root }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("R7I failed fixture retained at {}", self.root.display());
            return;
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
