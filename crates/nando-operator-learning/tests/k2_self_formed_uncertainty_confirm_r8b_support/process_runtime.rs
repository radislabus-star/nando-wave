use super::*;

#[path = "process_runtime/command.rs"]
mod command;
#[path = "process_runtime/ledger.rs"]
mod ledger;

pub use command::*;
use command::{
    CommandFailureV3, append_frozen_request_v3, monotonic_ns_v2, run_command_bytes_v2,
    try_run_command_bytes_v3, try_run_process_bytes_v3,
};
pub use ledger::*;

const SYSTEMD_RUN_V3: &str = "/usr/bin/systemd-run";
const SYSTEMCTL_V3: &str = "/usr/bin/systemctl";
pub const SYSTEMD_MANAGER_V3: &str = "/usr/lib/systemd/systemd";
pub const SUDO_V3: &str = "/usr/lib/cargo/bin/sudo";
pub const SHA256SUM_V3: &str = "/usr/lib/cargo/bin/coreutils/sha256sum";

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
