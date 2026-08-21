use super::*;

const SYSTEMD_RUN_V3: &str = "/usr/bin/systemd-run";
const SYSTEMCTL_V3: &str = "/usr/bin/systemctl";
const SYSTEMD_MANAGER_V3: &str = "/usr/lib/systemd/systemd";
const SUDO_V3: &str = "/usr/lib/cargo/bin/sudo";
const SHA256SUM_V3: &str = "/usr/lib/cargo/bin/coreutils/sha256sum";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelegatedChildOwnerV3 {
    UserSystemdManager,
    M24Direct,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegatedLaunchContractV3 {
    pub route_id_sha256: String,
    pub unit: String,
    pub request_owner_role: String,
    pub child_owner: DelegatedChildOwnerV3,
    pub systemd_run_sha256: String,
    pub child_executable: PathBuf,
    pub child_executable_sha256: String,
    pub credential_path: PathBuf,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
    pub selector: String,
    pub normalized_argv: Vec<String>,
}

#[derive(Default)]
pub struct MeasurementsV2 {
    pub maximum_protocol_bytes: u64,
}

impl MeasurementsV2 {
    pub fn observe_bytes(&mut self, bytes: &[u8]) {
        self.maximum_protocol_bytes = self.maximum_protocol_bytes.max(bytes.len() as u64);
    }
}

pub fn route_unit_v3(route_id_sha256: &str) -> String {
    format!("nando-r8b-{}.service", &route_id_sha256[..16])
}

pub fn delegated_launch_argv_v3(contract: &DelegatedLaunchContractV3) -> Vec<String> {
    vec![
        SYSTEMD_RUN_V3.to_owned(),
        "--user".to_owned(),
        "--no-ask-password".to_owned(),
        "--expand-environment=no".to_owned(),
        format!("--unit={}", contract.unit),
        "--service-type=exec".to_owned(),
        "--remain-after-exit".to_owned(),
        "--property=MemoryMax=536870912".to_owned(),
        "--property=MemorySwapMax=0".to_owned(),
        "--property=TasksMax=256".to_owned(),
        "--property=RuntimeMaxSec=1200".to_owned(),
        "--property=KillMode=control-group".to_owned(),
        "--property=PrivateNetwork=yes".to_owned(),
        "--property=RestrictAddressFamilies=AF_UNIX".to_owned(),
        format!(
            "--property=LoadCredential=r8b-producer-request:{}",
            contract.credential_path.display()
        ),
        format!(
            "--property=StandardOutput=file:{}",
            contract.stdout_path.display()
        ),
        format!(
            "--property=StandardError=file:{}",
            contract.stderr_path.display()
        ),
        contract.child_executable.to_string_lossy().into_owned(),
        "--ignored".to_owned(),
        "--exact".to_owned(),
        contract.selector.clone(),
        "--nocapture".to_owned(),
    ]
}

pub fn validate_delegated_launch_v3(
    contract: &DelegatedLaunchContractV3,
) -> Result<(), &'static str> {
    if nando_operator_learning::require_composition_root_v1(&contract.route_id_sha256).is_err()
        || nando_operator_learning::require_composition_root_v1(&contract.systemd_run_sha256)
            .is_err()
        || nando_operator_learning::require_composition_root_v1(&contract.child_executable_sha256)
            .is_err()
        || contract.unit != route_unit_v3(&contract.route_id_sha256)
        || contract.request_owner_role != "M24_LINKED_RUNNER"
        || contract.child_owner != DelegatedChildOwnerV3::UserSystemdManager
        || contract.selector != "r8b_v8_m24_linked_child"
        || !bounded_absolute_v3(&contract.child_executable)
        || !bounded_absolute_v3(&contract.credential_path)
        || !bounded_absolute_v3(&contract.stdout_path)
        || !bounded_absolute_v3(&contract.stderr_path)
        || contract.normalized_argv != delegated_launch_argv_v3(contract)
    {
        return Err("r8b_v8_delegated_launch_invalid");
    }
    Ok(())
}

pub fn privileged_probe_argv_v3(manager_pid: u32) -> Vec<String> {
    vec![
        SUDO_V3.to_owned(),
        "--non-interactive".to_owned(),
        "--user=root".to_owned(),
        "--".to_owned(),
        SHA256SUM_V3.to_owned(),
        "--binary".to_owned(),
        "--zero".to_owned(),
        format!("/proc/{manager_pid}/exe"),
    ]
}

fn bounded_absolute_v3(path: &Path) -> bool {
    path.is_absolute() && path.as_os_str().as_encoded_bytes().len() <= 240
}

#[allow(clippy::too_many_arguments)]
pub fn run_recorded_sandbox_v2<I, O, F>(
    ledger: &mut DurableProcessLedgerV2,
    binary: &BinaryV2,
    role: K2UncertaintyConfirmGuestExecutableV1,
    stage: &str,
    case_id_sha256: &str,
    probe_ordinal: Option<u64>,
    request_root_sha256: &str,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    request: &I,
    measurements: &mut MeasurementsV2,
    receipt_root: F,
) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
    F: FnOnce(&O) -> (&str, &str),
{
    let input = uncertainty_bytes_v1(request).expect("M24 sandbox request bytes");
    let started = ledger.start_bound(
        stage,
        Some(case_id_sha256.to_owned()),
        probe_ordinal,
        binary,
        request_root_sha256.to_owned(),
        composition_sha256_bytes_v1(&input),
    );
    let outcome = run_self_formed_confirm_sandbox_measured_v1(
        role,
        &binary.path,
        &binary.sha256,
        mounts,
        &input,
        60,
    )
    .unwrap_or_else(|error| {
        let failure = CommandFailureV3::launch(error);
        ledger.fail_bound(&started, &failure);
        panic!("{} sandbox launch failed: {}", binary.role, failure.message);
    });
    let output = sandbox_output_v2(outcome);
    if !output.status.success() {
        let failure =
            CommandFailureV3::terminal(&output, &format!("{} sandbox failed", binary.role));
        ledger.fail_bound(&started, &failure);
        panic!(
            "{}: {}",
            failure.message,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let value: O = uncertainty_decode_v1(&output.stdout).unwrap_or_else(|error| {
        let failure = CommandFailureV3::terminal(
            &output,
            &format!("{} sandbox output invalid: {error}", binary.role),
        );
        ledger.fail_bound(&started, &failure);
        panic!("{}", failure.message);
    });
    assert_eq!(
        output.stdout,
        uncertainty_bytes_v1(&value).expect("canonical M24 sandbox receipt")
    );
    let (schema, root) = receipt_root(&value);
    ledger.finish_bound(
        &started,
        &output,
        schema.to_owned(),
        root.to_owned(),
        K2UncertaintyR8BValidatedFactV3::None,
        Vec::new(),
    );
    measurements.observe_bytes(&input);
    measurements.observe_bytes(&output.stdout);
    value
}

pub fn sandbox_output_v2(
    outcome: nando_operator_learning::K2UncertaintySandboxProcessOutcomeV1,
) -> Output {
    use std::os::unix::process::ExitStatusExt;
    Output {
        status: std::process::ExitStatus::from_raw(outcome.exit_code << 8),
        stdout: outcome.stdout,
        stderr: outcome.stderr,
    }
}

pub fn private_artifact_v2<'a>(
    metadata: &'a K2UncertaintyDevelopmentRehearsalMetadataV1,
    case_id_sha256: &str,
    kind: K2UncertaintyDevelopmentRehearsalStoredArtifactKindV1,
) -> &'a K2UncertaintyDevelopmentRehearsalStoredArtifactV1 {
    metadata
        .split
        .artifacts
        .iter()
        .find(|artifact| {
            artifact.kind == kind && artifact.case_id_sha256.as_deref() == Some(case_id_sha256)
        })
        .expect("M24 private artifact metadata")
}

pub fn materialize_v2(root: &Path, files: &BTreeMap<String, Vec<u8>>) {
    create_private_directory_v1(root);
    for (relative, bytes) in files {
        let path = root.join(relative);
        create_private_directory_v1(path.parent().expect("M24 materialized parent"));
        fs::write(path, bytes).expect("write M24 materialized file");
    }
}

#[allow(clippy::too_many_arguments)]
pub fn publish_oracle_case_evidence_v2(
    root: &Path,
    public_bindings: &K2UncertaintyOraclePublicBindingsV1,
    prepared: &K2UncertaintyPublicPreparedCaseV1,
    probe_output: &nando_operator_learning::K2UncertaintyProbeOutputV1,
    observation_vector: &K2UncertaintyObservationVectorV2,
    final_receipt: &K2UncertaintyConfirmFinalVerifierReceiptV1,
    truth_artifact: &K2UncertaintyDevelopmentRehearsalStoredArtifactV1,
) -> K2UncertaintyOracleCaseEvidenceManifestV1 {
    create_private_directory_v1(root);
    let model_set = &public_bindings.probe_request.learner_response.model_set;
    let closure_plan = prepared
        .preverification
        .closure_plan
        .as_ref()
        .expect("M24 frozen closure plan");
    let mut entries = vec![
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::PublicBindings,
            "public-bindings.json",
            &public_bindings.bindings_root_sha256,
            public_bindings,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::ModelSet,
            "model-set.json",
            &model_set.model_set_root_sha256,
            model_set,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::FrontierCensus,
            "frontier-census.json",
            &probe_output.frontier.frontier_root_sha256,
            &probe_output.frontier,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::ClosurePlan,
            "closure-plan.json",
            &closure_plan.plan_root_sha256,
            closure_plan,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::ClosurePreverification,
            "closure-preverification.json",
            &prepared.preverification.receipt_root_sha256,
            &prepared.preverification,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::BaselineSummary,
            "baseline-summary.json",
            &prepared
                .selection_preverification
                .baseline_summary
                .summary_root_sha256,
            &prepared.selection_preverification.baseline_summary,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::ObservationVector,
            "observation-vector.json",
            &observation_vector.vector_root_sha256,
            observation_vector,
        ),
        write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::FinalVerifierReceipt,
            "final-verifier-receipt.json",
            &final_receipt.receipt_root_sha256,
            final_receipt,
        ),
        K2UncertaintyOracleEvidenceEntryV1::seal(
            K2UncertaintyOracleEvidenceKindV1::PrivateTruth,
            "private-truth.json".to_owned(),
            truth_artifact.byte_len,
            truth_artifact.unix_mode,
            truth_artifact.content_sha256.clone(),
            truth_artifact.semantic_root_sha256.clone(),
        )
        .expect("seal descriptor-only M24 private truth entry"),
    ];
    for page in &probe_output.pages {
        entries.push(write_oracle_entry_v2(
            root,
            K2UncertaintyOracleEvidenceKindV1::FrontierPage,
            &format!("frontier-pages/page-{:04}.json", page.page_sequence),
            &page.page_root_sha256,
            page,
        ));
    }
    let manifest = K2UncertaintyOracleCaseEvidenceManifestV1::seal(
        prepared
            .probe_request
            .public_case
            .vocabulary
            .case_id_sha256
            .clone(),
        entries,
    )
    .expect("seal M24 Oracle evidence manifest");
    write_new_read_only_v2(
        &root.join(K2_UNCERTAINTY_ORACLE_MANIFEST_FILE_V1),
        &uncertainty_bytes_v1(&manifest).expect("M24 Oracle manifest bytes"),
    );
    freeze_directory_tree_v2(root);
    assert!(!root.join("private-truth.json").exists());
    manifest
}

fn write_oracle_entry_v2<T: serde::Serialize>(
    root: &Path,
    kind: K2UncertaintyOracleEvidenceKindV1,
    relative: &str,
    semantic_root_sha256: &str,
    value: &T,
) -> K2UncertaintyOracleEvidenceEntryV1 {
    let bytes = uncertainty_bytes_v1(value).expect("M24 Oracle evidence bytes");
    let path = root.join(relative);
    create_private_directory_v1(path.parent().expect("M24 Oracle evidence parent"));
    write_new_read_only_v2(&path, &bytes);
    K2UncertaintyOracleEvidenceEntryV1::seal(
        kind,
        relative.to_owned(),
        bytes.len() as u64,
        0o400,
        composition_sha256_bytes_v1(&bytes),
        semantic_root_sha256.to_owned(),
    )
    .expect("seal M24 Oracle evidence entry")
}

pub fn evaluation_routes_v2(
    binaries: &LinkedBinariesV2,
    case_execution_count: u64,
) -> Vec<K2UncertaintyEvaluationRouteReceiptV1> {
    [
        (
            "public_precommit",
            "M10_PUBLIC_COORDINATOR",
            "M16_ORACLE",
            16,
        ),
        (
            "case_execution",
            "M13_WORKER",
            "M15_FINAL_VERIFIER",
            case_execution_count,
        ),
        ("final_verification", "M15_FINAL_VERIFIER", "M16_ORACLE", 16),
        (
            "oracle_evaluation",
            "M16_ORACLE",
            "M18_TERMINAL_EVALUATOR",
            16,
        ),
        (
            "control_evaluation",
            "M17_CONTROL_EVALUATOR",
            "M18_TERMINAL_EVALUATOR",
            64,
        ),
    ]
    .into_iter()
    .map(|(route, producer, consumer, events)| {
        K2UncertaintyEvaluationRouteReceiptV1::seal(
            route.to_owned(),
            binaries.get(producer).sha256.clone(),
            binaries.get(consumer).sha256.clone(),
            events,
            events,
        )
        .expect("seal M24 evaluation route")
    })
    .collect()
}

pub fn run_process_v1<T: serde::Serialize>(path: &Path, request: &T) -> Output {
    let input =
        nando_operator_learning::uncertainty_bytes_v1(request).expect("encode R8B process request");
    run_process_input_v2(path, &input)
}

pub fn run_process_recorded_v2<T: serde::Serialize>(path: &Path, request: &T) -> Output {
    let input = uncertainty_bytes_v1(request).expect("encode recorded R8B process request");
    let nested = begin_suite_nested_child_v2(path, request, &input);
    let output = match try_run_process_input_v3(path, &input) {
        Ok(output) => output,
        Err(failure) => fail_suite_nested_child_v3(nested, failure),
    };
    if !output.status.success() {
        fail_suite_nested_child_v3(
            nested,
            CommandFailureV3::terminal(&output, "recorded suite child failed"),
        );
    }
    finish_suite_nested_child_v2(nested, &output);
    output
}

fn run_process_input_v2(path: &Path, input: &[u8]) -> Output {
    try_run_process_input_v3(path, input)
        .unwrap_or_else(|failure| panic!("R8B process failed before output: {}", failure.message))
}

fn try_run_process_input_v3(path: &Path, input: &[u8]) -> Result<Output, CommandFailureV3> {
    let mut command = Command::new(path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    try_run_command_bytes_v3(command, input, 1_200)
}

pub type SuiteNestedStartV3 = (
    K2UncertaintyR8BLedgerWriterV2,
    K2UncertaintyR8BProcessEventV2,
    Option<(
        K2UncertaintyR8BLedgerWriterV3,
        K2UncertaintyR8BProcessEventV3,
    )>,
);

pub fn begin_suite_nested_child_v2<T: serde::Serialize>(
    path: &Path,
    request: &T,
    input: &[u8],
) -> Option<SuiteNestedStartV3> {
    let suite = SUITE_REQUEST_V2.get()?;
    let canonical = fs::canonicalize(path).expect("canonical suite child executable");
    let child_role = role_for_executable_v2(&canonical)?;
    let metadata = fs::metadata(&canonical).expect("suite child metadata");
    let current = fs::canonicalize(std::env::current_exe().expect("suite writer executable"))
        .expect("canonical suite writer");
    let writer = K2UncertaintyR8BLedgerWriterV2::from_environment(
        &suite.producer_role,
        composition_sha256_file_v1(&current).expect("suite writer SHA-256"),
        vec![K2UncertaintyR8BExecutableIdentityV2 {
            role: child_role.to_owned(),
            canonical_path: canonical.to_string_lossy().into_owned(),
            byte_len: metadata.len(),
            unix_mode: metadata.permissions().mode() & 0o7777,
            sha256: composition_sha256_file_v1(&canonical).expect("suite child SHA-256"),
        }],
    )
    .expect("open suite nested ledger")?;
    let request_root = uncertainty_root_v1(request).expect("suite nested request root");
    let stdin_sha256 = composition_sha256_bytes_v1(input);
    let monotonic = monotonic_ns_v2();
    let v3 = active_producer_request_v3().map(|(request, _)| {
        append_frozen_request_v3(
            request,
            &suite.producer_role,
            child_role,
            child_role,
            None,
            None,
            request_root.clone(),
            stdin_sha256.clone(),
            monotonic,
        )
    });
    let started = writer
        .child_started(
            child_role,
            None,
            None,
            child_role,
            &canonical,
            request_root,
            stdin_sha256,
            monotonic,
        )
        .expect("append suite nested ChildStarted");
    Some((writer, started, v3))
}

pub fn finish_suite_nested_child_v2(nested: Option<SuiteNestedStartV3>, output: &Output) {
    let Some((writer, started, v3)) = nested else {
        return;
    };
    assert!(output.status.success(), "recorded suite child must succeed");
    let (schema, root) = typed_json_identity_v2(&output.stdout)
        .expect("typed canonical recorded suite child output");
    if let Some((writer, event)) = v3 {
        writer
            .success(
                &event,
                &output.stdout,
                &output.stderr,
                schema.clone(),
                root.clone(),
                K2UncertaintyR8BValidatedFactV3::None,
                Vec::new(),
                monotonic_ns_v2(),
            )
            .expect("append suite nested V3 completion");
    }
    writer
        .child_finished(
            &started,
            &output.stdout,
            &output.stderr,
            vec![K2UncertaintyR8BProducedReceiptV2 {
                relative_path: K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2.to_owned(),
                byte_len: output.stdout.len() as u64,
                unix_mode: 0,
                content_sha256: composition_sha256_bytes_v1(&output.stdout),
                receipt_schema: schema,
                semantic_root_sha256: root,
            }],
            monotonic_ns_v2(),
        )
        .expect("append suite nested ChildFinished");
}

fn fail_suite_nested_child_v3(nested: Option<SuiteNestedStartV3>, failure: CommandFailureV3) -> ! {
    if let Some((_, _, Some((writer, started)))) = nested {
        writer
            .failure(
                &started,
                failure.kind,
                failure.exit_code,
                &failure.stdout,
                &failure.stderr,
                monotonic_ns_v2(),
            )
            .expect("append suite nested V3 failure");
    }
    panic!("{}", failure.message)
}

#[allow(clippy::too_many_arguments)]
fn append_frozen_request_v3(
    request: &K2UncertaintyR8BProducerRequestV3,
    owner: &str,
    target: &str,
    stage: &str,
    case_id_sha256: Option<String>,
    probe_ordinal: Option<u64>,
    request_root_sha256: String,
    stdin_sha256: String,
    monotonic_ns: u64,
) -> (
    K2UncertaintyR8BLedgerWriterV3,
    K2UncertaintyR8BProcessEventV3,
) {
    let writer =
        K2UncertaintyR8BLedgerWriterV3::attach_request(request).expect("attach frozen V3 ledger");
    let observed = writer
        .summary()
        .expect("read frozen V3 prefix")
        .invocations
        .into_iter()
        .map(|row| row.invocation_id_sha256)
        .collect::<BTreeSet<_>>();
    let plan = request
        .invocation_plan
        .iter()
        .find(|row| {
            row.request_owner_role == owner
                && row.target_role == target
                && row.stage == stage
                && row.case_id_sha256 == case_id_sha256
                && row.probe_ordinal == probe_ordinal
                && !observed.contains(&row.invocation_id_sha256)
        })
        .cloned()
        .expect("unused frozen V3 invocation");
    let event = writer
        .request(plan, request_root_sha256, stdin_sha256, monotonic_ns)
        .expect("append frozen V3 InvocationRequested");
    (writer, event)
}

fn role_for_executable_v2(path: &Path) -> Option<&'static str> {
    match path.file_name()?.to_str()? {
        "nando-k2-self-formed-confirm-owner" => Some("M01_DEVELOPMENT_OWNER"),
        "nando-k2-self-formed-public-coordinator" => Some("M10_PUBLIC_COORDINATOR"),
        "nando-k2-self-formed-control-evaluator" => Some("M17_CONTROL_EVALUATOR"),
        "nando-k2-self-formed-terminal-evaluator" => Some("M18_TERMINAL_EVALUATOR"),
        "nando-k2-self-formed-cleanup-authorizer" => Some("M20_CLEANUP_AUTHORIZER"),
        "nando-k2-self-formed-cleanup-owner" => Some("M21_CLEANUP_OWNER"),
        "nando-k2-self-formed-cleanup-verifier" => Some("M22_CLEANUP_VERIFIER"),
        "nando-k2-self-formed-result-publisher" => Some("M23_DEVELOPMENT_RESULT_PUBLISHER"),
        _ => None,
    }
}

pub fn run_process_success_v1<I, O>(path: &Path, request: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let output = run_process_recorded_v2(path, request);
    assert!(
        output.status.success(),
        "R8B process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decoded: O = nando_operator_learning::uncertainty_decode_v1(&output.stdout)
        .expect("decode R8B process output");
    assert_eq!(
        output.stdout,
        nando_operator_learning::uncertainty_bytes_v1(&decoded)
            .expect("re-encode R8B process output")
    );
    decoded
}

pub struct DurableProcessLedgerV2 {
    writer: K2UncertaintyR8BLedgerWriterV2,
    root: PathBuf,
    route_id_sha256: String,
    writer_role: &'static str,
    v3: Option<V3LedgerBinding>,
}

#[derive(Clone)]
struct V3LedgerBinding {
    request: K2UncertaintyR8BProducerRequestV3,
    path: PathBuf,
}

pub struct BoundProcessStartV3 {
    legacy: K2UncertaintyR8BProcessEventV2,
    v3: Option<(
        K2UncertaintyR8BLedgerWriterV3,
        K2UncertaintyR8BProcessEventV3,
    )>,
}

impl DurableProcessLedgerV2 {
    pub fn create(
        root: &Path,
        route_id_sha256: String,
        writer: &BinaryV2,
        allowed_children: &[&BinaryV2],
    ) -> Self {
        let writer_role = writer.role;
        create_private_directory_v1(root);
        assert!(
            fs::read_dir(root)
                .expect("new ledger directory")
                .next()
                .is_none()
        );
        let writer = K2UncertaintyR8BLedgerWriterV2::new(
            root.to_path_buf(),
            route_id_sha256.clone(),
            writer.role,
            writer.sha256.clone(),
            allowed_children
                .iter()
                .map(|child| child.identity())
                .collect(),
        )
        .expect("create shared process ledger writer");
        Self {
            writer,
            root: root.to_path_buf(),
            route_id_sha256,
            writer_role,
            v3: None,
        }
    }

    pub fn open(
        root: &Path,
        route_id_sha256: String,
        writer: &BinaryV2,
        allowed_children: &[&BinaryV2],
    ) -> Self {
        let writer_role = writer.role;
        let root = fs::canonicalize(root).expect("canonical existing ledger root");
        let writer = K2UncertaintyR8BLedgerWriterV2::new(
            root.clone(),
            route_id_sha256.clone(),
            writer.role,
            writer.sha256.clone(),
            allowed_children
                .iter()
                .map(|child| child.identity())
                .collect(),
        )
        .expect("open shared process ledger writer");
        let v3 = active_producer_request_v3().map(|(request, path)| V3LedgerBinding {
            request: request.clone(),
            path: path.to_path_buf(),
        });
        Self {
            writer,
            root,
            route_id_sha256,
            writer_role,
            v3,
        }
    }

    pub fn bind_v3(&mut self, request: K2UncertaintyR8BProducerRequestV3, path: PathBuf) {
        validate_self_formed_r8b_producer_request_v3(&request)
            .expect("validate bound V3 producer request");
        assert_eq!(request.route_id_sha256, self.route_id_sha256);
        assert_eq!(
            fs::read(&path).expect("read bound V3 producer request"),
            uncertainty_bytes_v1(&request).expect("V3 producer bytes")
        );
        self.v3 = Some(V3LedgerBinding { request, path });
    }

    pub fn start(
        &mut self,
        stage_id: &str,
        case_id_sha256: Option<String>,
        probe_ordinal: Option<u64>,
        binary: &BinaryV2,
        request_root_sha256: String,
        stdin_sha256: String,
    ) -> K2UncertaintyR8BProcessEventV2 {
        self.writer
            .child_started(
                stage_id,
                case_id_sha256,
                probe_ordinal,
                binary.role,
                &binary.path,
                request_root_sha256,
                stdin_sha256,
                monotonic_ns_v2(),
            )
            .expect("append ChildStarted")
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_bound(
        &mut self,
        stage_id: &str,
        case_id_sha256: Option<String>,
        probe_ordinal: Option<u64>,
        binary: &BinaryV2,
        request_root_sha256: String,
        stdin_sha256: String,
    ) -> BoundProcessStartV3 {
        let monotonic = monotonic_ns_v2();
        let v3 = self.v3.as_ref().map(|binding| {
            append_frozen_request_v3(
                &binding.request,
                self.writer_role,
                binary.role,
                stage_id,
                case_id_sha256.clone(),
                probe_ordinal,
                request_root_sha256.clone(),
                stdin_sha256.clone(),
                monotonic,
            )
        });
        let legacy = self.start(
            stage_id,
            case_id_sha256,
            probe_ordinal,
            binary,
            request_root_sha256,
            stdin_sha256,
        );
        BoundProcessStartV3 { legacy, v3 }
    }

    pub fn finish(
        &mut self,
        start: &K2UncertaintyR8BProcessEventV2,
        output: &Output,
        receipt_schema: String,
        decoded_receipt_root_sha256: String,
    ) -> K2UncertaintyR8BProcessEventV2 {
        assert!(
            output.status.success(),
            "process must succeed before ChildFinished"
        );
        self.writer
            .child_finished(
                start,
                &output.stdout,
                &output.stderr,
                vec![K2UncertaintyR8BProducedReceiptV2 {
                    relative_path: K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2.to_owned(),
                    byte_len: output.stdout.len() as u64,
                    unix_mode: 0,
                    content_sha256: composition_sha256_bytes_v1(&output.stdout),
                    receipt_schema,
                    semantic_root_sha256: decoded_receipt_root_sha256,
                }],
                monotonic_ns_v2(),
            )
            .expect("append ChildFinished")
    }

    pub fn finish_bound(
        &mut self,
        start: &BoundProcessStartV3,
        output: &Output,
        receipt_schema: String,
        decoded_receipt_root_sha256: String,
        fact: K2UncertaintyR8BValidatedFactV3,
        authority_outputs: Vec<K2UncertaintyR8BOutputContractV3>,
    ) -> K2UncertaintyR8BProcessEventV2 {
        assert!(
            output.status.success(),
            "process must succeed before typed completion"
        );
        if let Some((writer, started)) = &start.v3 {
            writer
                .success(
                    started,
                    &output.stdout,
                    &output.stderr,
                    receipt_schema.clone(),
                    decoded_receipt_root_sha256.clone(),
                    fact,
                    authority_outputs,
                    monotonic_ns_v2(),
                )
                .expect("append V3 typed completion");
        }
        self.finish(
            &start.legacy,
            output,
            receipt_schema,
            decoded_receipt_root_sha256,
        )
    }

    fn fail_bound(&mut self, start: &BoundProcessStartV3, failure: &CommandFailureV3) {
        if let Some((writer, started)) = &start.v3 {
            writer
                .failure(
                    started,
                    failure.kind,
                    failure.exit_code,
                    &failure.stdout,
                    &failure.stderr,
                    monotonic_ns_v2(),
                )
                .expect("append V3 process failure");
        }
    }

    pub fn fail_launch_bound(
        &mut self,
        start: &BoundProcessStartV3,
        message: impl std::fmt::Display,
    ) -> ! {
        let failure = CommandFailureV3::launch(message);
        self.fail_bound(start, &failure);
        panic!("{}", failure.message)
    }

    pub fn fail_unexpected_bound(
        &mut self,
        start: &BoundProcessStartV3,
        output: &Output,
        message: impl Into<String>,
    ) -> ! {
        let failure = CommandFailureV3::terminal(output, &message.into());
        self.fail_bound(start, &failure);
        panic!("{}", failure.message)
    }

    pub fn run_started_process_v3(
        &mut self,
        start: &BoundProcessStartV3,
        path: &Path,
        input: &[u8],
        timeout_seconds: u64,
    ) -> Output {
        match try_run_process_bytes_v3(path, input, timeout_seconds) {
            Ok(output) if output.status.success() => output,
            Ok(output) => self.fail_unexpected_bound(start, &output, "recorded process failed"),
            Err(failure) => {
                self.fail_bound(start, &failure);
                panic!("{}", failure.message)
            }
        }
    }

    pub fn freeze(self) -> K2UncertaintyR8BProcessLedgerV2 {
        self.writer
            .complete_ledger()
            .expect("freeze complete process ledger")
    }

    pub fn environment(&self) -> Vec<(String, String)> {
        let mut values = vec![
            (
                K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2.to_owned(),
                self.root.to_string_lossy().into_owned(),
            ),
            (
                K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2.to_owned(),
                self.route_id_sha256.clone(),
            ),
        ];
        if let Some(binding) = &self.v3 {
            values.push((
                K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_ENV_V3.to_owned(),
                binding.path.to_string_lossy().into_owned(),
            ));
        }
        values
    }

    pub fn finish_closed(
        &self,
        start: &K2UncertaintyR8BProcessEventV2,
        output: &Output,
        receipts: Vec<K2UncertaintyR8BProducedReceiptV2>,
    ) -> K2UncertaintyR8BProcessEventV2 {
        assert!(
            output.status.success(),
            "closed producer must exit successfully"
        );
        self.writer
            .child_finished(
                start,
                &output.stdout,
                &output.stderr,
                receipts,
                monotonic_ns_v2(),
            )
            .expect("append closed-channel ChildFinished")
    }
}

pub struct RecordedProcessV2<T> {
    pub value: T,
    pub output: Output,
    pub finished: K2UncertaintyR8BProcessEventV2,
}

pub struct ClosedProducerV2 {
    pub request: K2UncertaintyR8BProducerRequestV2,
    pub output: Output,
    pub descriptors: Vec<K2UncertaintyR8BProducedReceiptV2>,
    pub finished: K2UncertaintyR8BProcessEventV2,
}

pub fn run_closed_suite_producer_v2(
    ledger: &mut DurableProcessLedgerV2,
    binary: &BinaryV2,
    selector: &str,
    output_root: &Path,
    allowed_paths: Vec<String>,
) -> ClosedProducerV2 {
    create_private_directory_v1(output_root);
    assert!(
        fs::read_dir(output_root)
            .expect("fresh suite output")
            .next()
            .is_none()
    );
    let mut request = K2UncertaintyR8BProducerRequestV2 {
        schema: String::new(),
        route_id_sha256: ledger.route_id_sha256.clone(),
        producer_role: binary.role.to_owned(),
        producer_executable_sha256: binary.sha256.clone(),
        test_selector: selector.to_owned(),
        allowed_relative_paths: allowed_paths,
        exclusive_output_directory: output_root.to_string_lossy().into_owned(),
        request_root_sha256: String::new(),
    };
    request.reseal().expect("seal suite producer request");
    let input = uncertainty_bytes_v1(&request).expect("suite producer stdin");
    let started = ledger.start(
        binary.role,
        None,
        None,
        binary,
        request.request_root_sha256.clone(),
        composition_sha256_bytes_v1(&input),
    );
    let mut command = Command::new(&binary.path);
    command
        .args(["--ignored", "--exact", selector, "--nocapture"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in ledger.environment() {
        command.env(key, value);
    }
    let output = run_command_bytes_v2(command, &input, 1_200);
    assert!(
        output.status.success(),
        "suite producer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let descriptors = reopen_closed_measured_receipts_v2(&request);
    let finished = ledger.finish_closed(&started, &output, descriptors.clone());
    ClosedProducerV2 {
        request,
        output,
        descriptors,
        finished,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_recorded_process_v2<I, O, F>(
    ledger: &mut DurableProcessLedgerV2,
    binary: &BinaryV2,
    stage_id: &str,
    case_id_sha256: Option<String>,
    probe_ordinal: Option<u64>,
    request_root_sha256: String,
    request: &I,
    receipt_root: F,
) -> RecordedProcessV2<O>
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
    F: FnOnce(&O) -> (&str, &str),
{
    let input = uncertainty_bytes_v1(request).expect("recorded process input");
    let start = ledger.start_bound(
        stage_id,
        case_id_sha256,
        probe_ordinal,
        binary,
        request_root_sha256,
        composition_sha256_bytes_v1(&input),
    );
    let mut command = Command::new(&binary.path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in ledger.environment() {
        command.env(key, value);
    }
    let output = match try_run_command_bytes_v3(command, &input, 60) {
        Ok(output) => output,
        Err(failure) => {
            ledger.fail_bound(&start, &failure);
            panic!("{} launch failed: {}", binary.role, failure.message);
        }
    };
    if !output.status.success() {
        let failure =
            CommandFailureV3::terminal(&output, &format!("{} process failed", binary.role));
        ledger.fail_bound(&start, &failure);
        panic!("{}", failure.message);
    }
    let value: O = uncertainty_decode_v1(&output.stdout).unwrap_or_else(|error| {
        let failure = CommandFailureV3::terminal(
            &output,
            &format!("{} output invalid: {error}", binary.role),
        );
        ledger.fail_bound(&start, &failure);
        panic!(
            "{}; stderr={}",
            failure.message,
            String::from_utf8_lossy(&output.stderr)
        );
    });
    let (schema, root) = receipt_root(&value);
    let finished = ledger.finish_bound(
        &start,
        &output,
        schema.to_owned(),
        root.to_owned(),
        K2UncertaintyR8BValidatedFactV3::None,
        Vec::new(),
    );
    RecordedProcessV2 {
        value,
        output,
        finished,
    }
}

pub struct PrivateDescriptorV2 {
    file: File,
    pub inode: u64,
    pub byte_len: u64,
    pub unix_mode: u32,
}

impl PrivateDescriptorV2 {
    pub fn open(path: &Path, expected_byte_len: u64, expected_unix_mode: u32) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW)
            .open(path)
            .expect("open O_PATH private descriptor");
        let metadata = file.metadata().expect("private descriptor metadata");
        assert!(metadata.is_file());
        assert_eq!(metadata.nlink(), 1);
        assert_eq!(metadata.len(), expected_byte_len);
        assert_eq!(metadata.permissions().mode() & 0o7777, expected_unix_mode);
        let mut flags = rustix::io::fcntl_getfd(&file).expect("read private descriptor flags");
        flags.remove(rustix::io::FdFlags::CLOEXEC);
        rustix::io::fcntl_setfd(&file, flags)
            .expect("make private descriptor inheritable by the immediate bwrap child");
        Self {
            file,
            inode: metadata.ino(),
            byte_len: metadata.len(),
            unix_mode: metadata.permissions().mode() & 0o7777,
        }
    }

    pub fn proc_path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.file.as_raw_fd()))
    }
}

pub fn write_new_read_only_v2(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(path)
        .expect("create immutable evidence file");
    file.write_all(bytes)
        .expect("write immutable evidence file");
    file.sync_all().expect("fsync immutable evidence file");
}

pub fn freeze_directory_tree_v2(root: &Path) {
    let mut directories = vec![root.to_path_buf()];
    let mut index = 0;
    while index < directories.len() {
        for entry in fs::read_dir(&directories[index]).expect("read evidence tree") {
            let path = entry.expect("evidence tree entry").path();
            if path.is_dir() {
                directories.push(path);
            } else {
                fs::set_permissions(path, fs::Permissions::from_mode(0o400))
                    .expect("freeze evidence file");
            }
        }
        index += 1;
    }
    for directory in directories.into_iter().rev() {
        File::open(&directory)
            .expect("open evidence directory")
            .sync_all()
            .expect("fsync evidence directory");
        fs::set_permissions(directory, fs::Permissions::from_mode(0o500))
            .expect("freeze evidence directory");
    }
}

fn run_process_bytes_v2(path: &Path, input: &[u8], timeout_seconds: u64) -> Output {
    try_run_process_bytes_v3(path, input, timeout_seconds)
        .unwrap_or_else(|failure| panic!("recorded process failed: {}", failure.message))
}

fn try_run_process_bytes_v3(
    path: &Path,
    input: &[u8],
    timeout_seconds: u64,
) -> Result<Output, CommandFailureV3> {
    let mut command = Command::new(path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    try_run_command_bytes_v3(command, input, timeout_seconds)
}

fn run_command_bytes_v2(command: Command, input: &[u8], timeout_seconds: u64) -> Output {
    try_run_command_bytes_v3(command, input, timeout_seconds)
        .unwrap_or_else(|failure| panic!("recorded process failed: {}", failure.message))
}

#[derive(Debug)]
struct CommandFailureV3 {
    kind: K2UncertaintyR8BCompletionKindV3,
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    message: String,
}

impl CommandFailureV3 {
    fn launch(error: impl std::fmt::Display) -> Self {
        let message = format!("spawn recorded process: {error}");
        Self {
            kind: K2UncertaintyR8BCompletionKindV3::LaunchFailure,
            exit_code: -1,
            stdout: Vec::new(),
            stderr: message.as_bytes().to_vec(),
            message,
        }
    }

    fn terminal(output: &Output, message: &str) -> Self {
        Self {
            kind: K2UncertaintyR8BCompletionKindV3::UnexpectedFailure,
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout.clone(),
            stderr: output.stderr.clone(),
            message: message.to_owned(),
        }
    }
}

fn try_run_command_bytes_v3(
    mut command: Command,
    input: &[u8],
    timeout_seconds: u64,
) -> Result<Output, CommandFailureV3> {
    let mut child = command.spawn().map_err(CommandFailureV3::launch)?;
    child
        .stdin
        .take()
        .expect("recorded process stdin")
        .write_all(input)
        .expect("write recorded process stdin");
    let mut stdout = child.stdout.take().expect("recorded stdout");
    let mut stderr = child.stderr.take().expect("recorded stderr");
    let stdout_thread = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .by_ref()
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .expect("read recorded stdout");
        bytes
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .by_ref()
            .take(65_537)
            .read_to_end(&mut bytes)
            .expect("read recorded stderr");
        bytes
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_seconds);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll recorded process") {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().expect("kill timed out recorded process");
            break child.wait().expect("reap timed out recorded process");
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    };
    Ok(Output {
        status,
        stdout: stdout_thread.join().expect("join recorded stdout"),
        stderr: stderr_thread.join().expect("join recorded stderr"),
    })
}

fn monotonic_ns_v2() -> u64 {
    static ORIGIN: OnceLock<std::time::Instant> = OnceLock::new();
    ORIGIN
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_nanos() as u64
}
