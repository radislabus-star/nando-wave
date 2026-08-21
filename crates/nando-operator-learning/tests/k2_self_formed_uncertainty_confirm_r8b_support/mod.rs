#![allow(dead_code)]

mod process_runtime;

pub use process_runtime::*;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_learning::{
    K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2, K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2, K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2,
    K2UncertaintyConfirmAttemptDescriptorV1, K2UncertaintyConfirmOwnerRequestV1, K2UncertaintyControlEvaluationReceiptV1,
    K2UncertaintyControlEvaluationRequestV1, K2UncertaintyControlProcessOutcomeV1, K2UncertaintyControlScopeV1, K2UncertaintyControlStdoutV1,
    K2UncertaintyEvaluationResourceMeasurementsV1, K2UncertaintyEvaluationRouteReceiptV1, K2UncertaintyGeneratorRequestV1,
    K2UncertaintyImmutablePublicationFaultV1, K2UncertaintyOracleBaselineBatchReceiptV1, K2UncertaintyR8BEvidenceKindV2, K2UncertaintyR8BExecutableIdentityV2,
    K2UncertaintyR8BExecutableManifestV2, K2UncertaintyR8BLedgerWriterV2, K2UncertaintyR8BManifestClassV2, K2UncertaintyR8BMeasuredReceiptV2,
    K2UncertaintyR8BProcessEventV2, K2UncertaintyR8BProcessLedgerV2, K2UncertaintyR8BProducedReceiptV2, K2UncertaintyR8BProducerRequestV2, composition_root_v1,
    composition_sha256_bytes_v1, composition_sha256_file_v1, expected_self_formed_control_v1, publish_immutable_file_v1, require_composition_root_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static SUITE_REQUEST_V2: OnceLock<K2UncertaintyR8BProducerRequestV2> = OnceLock::new();

pub struct TestEnvironmentV1 {
    pub root: PathBuf,
}

impl TestEnvironmentV1 {
    pub fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("nando-r8b-{label}-{}-{sequence}", std::process::id()));
        create_private_directory_v1(&path);
        Self {
            root: fs::canonicalize(path).expect("canonical R8B test root"),
        }
    }

    pub fn private_child(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        create_private_directory_v1(&path);
        fs::canonicalize(path).expect("canonical R8B child")
    }
}

impl Drop for TestEnvironmentV1 {
    fn drop(&mut self) {
        make_tree_writable_v1(&self.root);
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn development_owner_request_v1(lab_root: &Path, attempt: &str, owner: &Path, generator: &Path) -> K2UncertaintyConfirmOwnerRequestV1 {
    let owner_sha256 = composition_sha256_file_v1(owner).expect("owner SHA-256");
    let generator_sha256 = composition_sha256_file_v1(generator).expect("generator SHA-256");
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required");
    let seed = fs::read(seed_path).expect("read frozen Development seed");
    let generator_request = K2UncertaintyGeneratorRequestV1::development(seed, generator_sha256.clone()).expect("Development generator request");
    let generator_schema_root = composition_root_v1(&(
        "nando.k2-self-formed-deterministic-generator.v1",
        &generator_request.preregistration_v2_root_sha256,
        &generator_request.preregistration_v3_root_sha256,
    ))
    .expect("Development generator schema root");
    let experiment_id = composition_root_v1(&(
        "nando.k2-self-formed-development-experiment.v1",
        &generator_request.seed_commitment_sha256,
        &generator_schema_root,
    ))
    .expect("Development experiment ID");
    let descriptor = K2UncertaintyConfirmAttemptDescriptorV1::development_rehearsal(
        experiment_id,
        root_v1("r8b-successor-freeze"),
        root_v1("r8b-executable-manifest"),
        owner_sha256,
        generator_sha256,
    )
    .expect("Development descriptor");
    K2UncertaintyConfirmOwnerRequestV1::development_rehearsal(
        descriptor,
        lab_root.to_string_lossy().into_owned(),
        attempt.to_owned(),
        generator.to_string_lossy().into_owned(),
        generator_request,
    )
    .expect("Development owner request")
}

pub fn root_v1(label: &str) -> String {
    composition_root_v1(&("nando.r8b.test-root.v1", label)).expect("R8B test root")
}

pub fn r7j_terminal_evidence_v1() -> (
    K2UncertaintyOracleBaselineBatchReceiptV1,
    Vec<K2UncertaintyEvaluationRouteReceiptV1>,
    K2UncertaintyEvaluationResourceMeasurementsV1,
) {
    let root = std::env::var_os("NANDO_K2_R7K_FIXTURE_ROOT")
        .map(PathBuf::from)
        .expect("NANDO_K2_R7K_FIXTURE_ROOT is required for X18");
    let packet = root.join("fixture-packet");
    (
        decode_fixture_v1(&packet.join("oracle-batch.json")),
        decode_fixture_v1(&packet.join("routes.json")),
        decode_fixture_v1(&packet.join("resources.json")),
    )
}

pub fn control_receipt_v1(
    scope: K2UncertaintyControlScopeV1,
    experiment: &str,
    freeze: Option<&str>,
    attempt: Option<&str>,
) -> K2UncertaintyControlEvaluationReceiptV1 {
    let evaluator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-control-evaluator"));
    let evaluator_sha = composition_sha256_file_v1(&evaluator).expect("control evaluator SHA");
    let freeze = freeze.map(str::to_owned);
    let attempt = attempt.map(str::to_owned);
    let outcomes = (0..scope.expected_count())
        .map(|ordinal| {
            let (control_id, disposition) = expected_self_formed_control_v1(scope, ordinal).expect("expected control row");
            let stdout = uncertainty_bytes_v1(&K2UncertaintyControlStdoutV1 {
                control_id: control_id.clone(),
                disposition: disposition.clone(),
            })
            .expect("control stdout");
            K2UncertaintyControlProcessOutcomeV1::seal(
                scope,
                control_id,
                experiment.to_owned(),
                freeze.clone(),
                attempt.clone(),
                root_v1(&format!("fixture-runner-{scope:?}")),
                root_v1(&format!("fixture-test-{scope:?}-{ordinal}")),
                root_v1(&format!("fixture-request-{scope:?}-{ordinal}")),
                true,
                0,
                stdout,
                composition_sha256_bytes_v1(&[]),
                false,
                false,
                disposition,
                root_v1(&format!("fixture-source-{scope:?}-{ordinal}")),
                root_v1(&format!("fixture-log-{scope:?}-{ordinal}")),
            )
            .expect("fixture control outcome")
        })
        .collect();
    let request =
        K2UncertaintyControlEvaluationRequestV1::seal(scope, experiment.to_owned(), freeze, attempt, outcomes, evaluator_sha).expect("fixture control request");
    run_process_success_v1(&evaluator, &request)
}

fn decode_fixture_v1<T>(path: &Path) -> T
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    uncertainty_decode_v1(&fs::read(path).expect("read R7J terminal fixture")).expect("decode R7J terminal fixture")
}

pub fn create_private_directory_v1(path: &Path) {
    fs::create_dir_all(path).expect("create R8B private directory");
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("chmod R8B private directory");
}

pub fn tree_snapshot_v1(root: &Path) -> Vec<(String, u64, u32)> {
    let mut pending = vec![root.to_path_buf()];
    let mut values = Vec::new();
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(&path).expect("read snapshot directory") {
            let entry = entry.expect("snapshot entry");
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).expect("snapshot metadata");
            let relative = path.strip_prefix(root).expect("snapshot relative path").to_string_lossy().into_owned();
            values.push((relative, metadata.len(), metadata.permissions().mode() & 0o777));
            if metadata.is_dir() {
                pending.push(path);
            }
        }
    }
    values.sort();
    values
}

#[derive(Clone, Debug)]
pub struct BinaryV2 {
    pub role: &'static str,
    pub path: PathBuf,
    pub sha256: String,
}

impl BinaryV2 {
    fn new(role: &'static str, path: impl Into<PathBuf>) -> Self {
        let path = fs::canonicalize(path.into()).expect("canonical linked executable");
        let sha256 = composition_sha256_file_v1(&path).expect("linked executable SHA-256");
        Self { role, path, sha256 }
    }

    pub fn identity(&self) -> K2UncertaintyR8BExecutableIdentityV2 {
        let metadata = fs::metadata(&self.path).expect("linked executable metadata");
        K2UncertaintyR8BExecutableIdentityV2 {
            role: self.role.to_owned(),
            canonical_path: self.path.to_string_lossy().into_owned(),
            byte_len: metadata.len(),
            unix_mode: metadata.permissions().mode() & 0o7777,
            sha256: self.sha256.clone(),
        }
    }
}

pub struct LinkedBinariesV2 {
    pub values: Vec<BinaryV2>,
}

impl LinkedBinariesV2 {
    pub fn from_cargo() -> Self {
        let current = std::env::current_exe().expect("resolve linked M24 executable");
        Self {
            values: vec![
                BinaryV2::new("M01_DEVELOPMENT_OWNER", env!("CARGO_BIN_EXE_nando-k2-self-formed-confirm-owner")),
                BinaryV2::new("M02_GENERATOR", env!("CARGO_BIN_EXE_nando-k2-self-formed-generator")),
                BinaryV2::new("M03_LEARNER", env!("CARGO_BIN_EXE_nando-k2-self-formed-learner")),
                BinaryV2::new("M04_PROBE", env!("CARGO_BIN_EXE_nando-k2-self-formed-probe")),
                BinaryV2::new("M05_SELECTOR", env!("CARGO_BIN_EXE_nando-k2-inquiry-selector")),
                BinaryV2::new("M06_BASELINE", env!("CARGO_BIN_EXE_nando-k2-inquiry-baseline")),
                BinaryV2::new("M07_SELECTION_PREVERIFIER", env!("CARGO_BIN_EXE_nando-k2-inquiry-verifier")),
                BinaryV2::new("M08_CLOSURE_PLANNER", env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-planner")),
                BinaryV2::new("M09_CLOSURE_VERIFIER", env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-verifier")),
                BinaryV2::new("M10_PUBLIC_COORDINATOR", env!("CARGO_BIN_EXE_nando-k2-self-formed-public-coordinator")),
                BinaryV2::new("M11_PRIVATE_RESOLVER", env!("CARGO_BIN_EXE_nando-k2-self-formed-private-resolver")),
                BinaryV2::new("M12_SAFETY", env!("CARGO_BIN_EXE_nando-k2-self-formed-safety")),
                BinaryV2::new("M13_WORKER", env!("CARGO_BIN_EXE_nando-k2-inquiry-worker")),
                BinaryV2::new("M14_OBSERVER", env!("CARGO_BIN_EXE_nando-k2-inquiry-observer")),
                BinaryV2::new("M15_FINAL_VERIFIER", env!("CARGO_BIN_EXE_nando-k2-self-formed-final-verifier-v2")),
                BinaryV2::new("M16_ORACLE", env!("CARGO_BIN_EXE_nando-k2-self-formed-oracle-baseline")),
                BinaryV2::new("M17_CONTROL_EVALUATOR", env!("CARGO_BIN_EXE_nando-k2-self-formed-control-evaluator")),
                BinaryV2::new("M18_TERMINAL_EVALUATOR", env!("CARGO_BIN_EXE_nando-k2-self-formed-terminal-evaluator")),
                BinaryV2::new("M19_FRESH_CONTROL_CASE", env!("CARGO_BIN_EXE_nando-k2-self-formed-r7k-control-case")),
                BinaryV2::new("M20_CLEANUP_AUTHORIZER", env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-authorizer")),
                BinaryV2::new("M21_CLEANUP_OWNER", env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-owner")),
                BinaryV2::new("M22_CLEANUP_VERIFIER", env!("CARGO_BIN_EXE_nando-k2-self-formed-cleanup-verifier")),
                BinaryV2::new("M23_DEVELOPMENT_RESULT_PUBLISHER", env!("CARGO_BIN_EXE_nando-k2-self-formed-result-publisher")),
                BinaryV2::new("M24_LINKED_RUNNER", current),
                BinaryV2::new("M25_R8B_AUTHORIZER", env!("CARGO_BIN_EXE_nando-k2-self-formed-r8b-authorizer")),
                BinaryV2::new("M26_R8B_PUBLISHER", env!("CARGO_BIN_EXE_nando-k2-self-formed-r8b-evidence-publisher")),
            ],
        }
    }

    pub fn get(&self, role: &str) -> &BinaryV2 {
        self.values.iter().find(|binary| binary.role == role).expect("linked role present")
    }

    pub fn manifest(&self) -> K2UncertaintyR8BExecutableManifestV2 {
        K2UncertaintyR8BExecutableManifestV2::seal(K2UncertaintyR8BManifestClassV2::Linked, self.values.iter().map(BinaryV2::identity).collect())
            .expect("seal 26-identity linked manifest")
    }
}

pub struct SuiteBinariesV2 {
    pub values: Vec<BinaryV2>,
}

impl SuiteBinariesV2 {
    pub fn from_current_deps() -> Self {
        let current = fs::canonicalize(std::env::current_exe().expect("current test executable")).expect("canonical current test executable");
        let deps = current.parent().expect("Cargo deps directory");
        let specifications = [
            ("S01_CRATE_UNIT", "k2_self_formed_uncertainty_confirm_r7h_v1-"),
            ("S02_RESTART", "k2_self_formed_uncertainty_confirm_r8b_restart_v1-"),
            ("S03_MODE_MATRIX", "k2_self_formed_uncertainty_confirm_r8b_mode_matrix_v1-"),
            ("S04_CLEANUP_NEGATIVE", "k2_self_formed_uncertainty_confirm_r8b_cleanup_v1-"),
            ("S05_AUTHORITY_PUBLICATION", "k2_self_formed_uncertainty_confirm_r8b_authority_v1-"),
        ];
        let values = specifications
            .into_iter()
            .map(|(role, prefix)| {
                let mut candidates = fs::read_dir(deps)
                    .expect("read Cargo deps directory")
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with(prefix))
                            && fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
                    })
                    .collect::<Vec<_>>();
                candidates.sort_by_key(|path| {
                    fs::metadata(path)
                        .and_then(|metadata| metadata.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                });
                BinaryV2::new(role, candidates.pop().expect("built R8B suite executable"))
            })
            .collect();
        Self { values }
    }

    pub fn get(&self, role: &str) -> &BinaryV2 {
        self.values.iter().find(|binary| binary.role == role).expect("suite role present")
    }

    pub fn manifest(&self) -> K2UncertaintyR8BExecutableManifestV2 {
        K2UncertaintyR8BExecutableManifestV2::seal(K2UncertaintyR8BManifestClassV2::Suite, self.values.iter().map(BinaryV2::identity).collect())
            .expect("seal five-identity suite manifest")
    }
}

pub fn load_suite_manifest_v2() -> K2UncertaintyR8BExecutableManifestV2 {
    let path = PathBuf::from(std::env::var_os("NANDO_R8B_SUITE_MANIFEST_PATH").expect("NANDO_R8B_SUITE_MANIFEST_PATH is required"));
    let metadata = fs::symlink_metadata(&path).expect("suite manifest metadata");
    assert!(!metadata.file_type().is_symlink());
    assert_eq!(metadata.permissions().mode() & 0o7777, 0o400);
    let value: K2UncertaintyR8BExecutableManifestV2 = uncertainty_decode_v1(&fs::read(path).expect("read suite manifest")).expect("decode suite manifest");
    value.validate().expect("validate suite manifest");
    assert_eq!(value.class, K2UncertaintyR8BManifestClassV2::Suite);
    value
}

pub struct SuiteMeasurementV2 {
    pub relative_path: &'static str,
    pub kind: K2UncertaintyR8BEvidenceKindV2,
    pub source_roots_sha256: Vec<String>,
    pub observed: u64,
    pub metrics: BTreeMap<String, u64>,
}

pub fn publish_suite_measurements_from_stdin_v2(expected_role: &str, expected_selector: &str, measurements: Vec<SuiteMeasurementV2>) {
    begin_suite_request_from_stdin_v2(expected_role, expected_selector);
    publish_suite_measurements_v2(measurements);
}

pub fn begin_suite_request_from_stdin_v2(expected_role: &str, expected_selector: &str) {
    let mut input = Vec::new();
    std::io::stdin()
        .take(1_048_576)
        .read_to_end(&mut input)
        .expect("read canonical suite request stdin");
    let request: K2UncertaintyR8BProducerRequestV2 = uncertainty_decode_v1(&input).expect("decode canonical suite request");
    request.validate().expect("validate canonical suite request");
    let current = fs::canonicalize(std::env::current_exe().expect("suite current executable")).expect("canonical suite executable");
    assert_eq!(request.producer_role, expected_role);
    assert_eq!(request.test_selector, expected_selector);
    assert_eq!(
        composition_sha256_file_v1(&current).expect("suite executable SHA-256"),
        request.producer_executable_sha256
    );
    let root = PathBuf::from(&request.exclusive_output_directory);
    assert_eq!(fs::canonicalize(&root).expect("canonical suite output"), root);
    assert_eq!(fs::metadata(&root).expect("suite output metadata").permissions().mode() & 0o7777, 0o700);
    assert!(fs::read_dir(&root).expect("empty suite output").next().is_none());
    SUITE_REQUEST_V2.set(request).expect("set suite request once");
}

pub fn active_producer_request_v2() -> &'static K2UncertaintyR8BProducerRequestV2 {
    SUITE_REQUEST_V2.get().expect("producer request initialized")
}

pub fn publish_suite_measurements_v2(measurements: Vec<SuiteMeasurementV2>) {
    let request = SUITE_REQUEST_V2.get().expect("suite request initialized");
    let root = PathBuf::from(&request.exclusive_output_directory);
    let paths = measurements.iter().map(|value| value.relative_path.to_owned()).collect::<Vec<_>>();
    assert_eq!(paths, request.allowed_relative_paths);
    for (sequence, measurement) in measurements.into_iter().enumerate() {
        ensure_private_parent_v2(&root, measurement.relative_path);
        let receipt = K2UncertaintyR8BMeasuredReceiptV2::seal(
            measurement.kind,
            request.route_id_sha256.clone(),
            measurement.source_roots_sha256,
            measurement.observed,
            measurement.metrics,
            request.producer_executable_sha256.clone(),
        )
        .expect("seal suite measured receipt");
        publish_immutable_file_v1(
            &root,
            measurement.relative_path,
            &uncertainty_bytes_v1(&receipt).expect("suite receipt bytes"),
            0o400,
            sequence as u64,
            K2UncertaintyImmutablePublicationFaultV1::None,
        )
        .expect("publish suite measured receipt");
    }
    File::open(root).expect("open suite output").sync_all().expect("fsync suite output");
}

pub fn reopen_closed_measured_receipts_v2(request: &K2UncertaintyR8BProducerRequestV2) -> Vec<K2UncertaintyR8BProducedReceiptV2> {
    request.validate().expect("validate producer request before reopen");
    let root = PathBuf::from(&request.exclusive_output_directory);
    let mut observed = BTreeSet::new();
    let mut pending = vec![root.clone()];
    while let Some(directory) = pending.pop() {
        let metadata = fs::symlink_metadata(&directory).expect("closed directory metadata");
        assert!(!metadata.file_type().is_symlink() && metadata.is_dir());
        assert_eq!(metadata.permissions().mode() & 0o7777, 0o700);
        for entry in fs::read_dir(&directory).expect("read closed producer directory") {
            let path = entry.expect("closed producer entry").path();
            let metadata = fs::symlink_metadata(&path).expect("closed producer metadata");
            assert!(!metadata.file_type().is_symlink());
            if metadata.is_dir() {
                pending.push(path);
            } else {
                assert!(metadata.is_file());
                assert_eq!(metadata.nlink(), 1);
                assert_eq!(metadata.permissions().mode() & 0o7777, 0o400);
                observed.insert(path.strip_prefix(&root).expect("closed relative path").to_string_lossy().into_owned());
            }
        }
    }
    assert_eq!(observed, request.allowed_relative_paths.iter().cloned().collect());
    let descriptors = request
        .allowed_relative_paths
        .iter()
        .map(|relative| {
            let bytes = fs::read(root.join(relative)).expect("read closed receipt");
            let value: K2UncertaintyR8BMeasuredReceiptV2 = uncertainty_decode_v1(&bytes).expect("decode closed measured receipt");
            value.validate().expect("validate closed measured receipt");
            assert_eq!(value.route_id_sha256, request.route_id_sha256);
            assert_eq!(value.producer_executable_sha256, request.producer_executable_sha256);
            K2UncertaintyR8BProducedReceiptV2 {
                relative_path: relative.clone(),
                byte_len: bytes.len() as u64,
                unix_mode: 0o400,
                content_sha256: composition_sha256_bytes_v1(&bytes),
                receipt_schema: value.schema,
                semantic_root_sha256: value.receipt_root_sha256,
            }
        })
        .collect();
    freeze_directory_tree_v2(&root);
    descriptors
}

fn ensure_private_parent_v2(root: &Path, relative: &str) {
    let Some(parent) = Path::new(relative).parent() else {
        return;
    };
    let mut current = root.to_path_buf();
    for component in parent.components() {
        current.push(component);
        if !current.exists() {
            fs::create_dir(&current).expect("create suite receipt parent");
            fs::set_permissions(&current, fs::Permissions::from_mode(0o700)).expect("chmod suite receipt parent");
        }
    }
}

pub fn typed_json_identity_v2(bytes: &[u8]) -> Option<(String, String)> {
    let value: serde_json::Value = uncertainty_decode_v1(bytes).ok()?;
    if uncertainty_bytes_v1(&value).ok()? != bytes {
        return None;
    }
    fn visit(value: &serde_json::Value) -> Option<(String, String)> {
        let object = value.as_object()?;
        if let Some(schema) = object.get("schema").and_then(serde_json::Value::as_str) {
            for key in [
                "receipt_root_sha256",
                "response_root_sha256",
                "artifacts_root_sha256",
                "census_root_sha256",
                "precommit_root_sha256",
                "baselines_root_sha256",
                "manifest_root_sha256",
            ] {
                if let Some(root) = object.get(key).and_then(serde_json::Value::as_str)
                    && require_composition_root_v1(root).is_ok()
                {
                    return Some((schema.to_owned(), root.to_owned()));
                }
            }
        }
        object.values().find_map(visit)
    }
    visit(&value)
}

fn make_tree_writable_v1(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let child = entry.path();
                if child.is_dir() {
                    pending.push(child.clone());
                    let _ = fs::set_permissions(&child, fs::Permissions::from_mode(0o700));
                } else {
                    let _ = fs::set_permissions(&child, fs::Permissions::from_mode(0o600));
                }
            }
        }
    }
    let _ = fs::set_permissions(root, fs::Permissions::from_mode(0o700));
}
