use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_kernel::{canonical_json_sha256, sha256_bytes};
use nando_operator_learning::{
    LawLabProbeDomainV1, LawLabSandboxAdapterV1, LawLabSandboxConfigV1, LawLabSandboxErrorV1,
    LawLabSandboxExecutorManifestV1, LawLabSandboxOperationV1, LawLabSandboxPurposeV1,
    LawLabSandboxRequestInputV1, LawLabSandboxRequestV1, LawLabTreeEntryKindV1,
    LawLabTreeManifestV1, law_lab_sha256_file_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

fn request(
    executor: &LawLabSandboxExecutorManifestV1,
    source: &LawLabTreeManifestV1,
    domain: LawLabProbeDomainV1,
    operations: Vec<LawLabSandboxOperationV1>,
) -> Result<LawLabSandboxRequestV1, LawLabSandboxErrorV1> {
    LawLabSandboxRequestV1::seal(LawLabSandboxRequestInputV1 {
        executor_manifest_root_sha256: executor.manifest_root_sha256.clone(),
        worker_sha256: executor.worker_sha256.clone(),
        candidate_root_sha256: root("natural-candidate"),
        version_space_root_sha256: root("frozen-version-space"),
        durable_prediction_ledger_root_sha256: root("external-durable-prediction-ledger"),
        probe_root_sha256: root("distinguishing-probe"),
        source_tree_root_sha256: source.tree_root_sha256.clone(),
        deterministic_seed_sha256: root("deterministic-seed"),
        domain,
        purpose: LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest,
        surviving_hypothesis_count: 2,
        precommitted_prediction_count: 2,
        operations,
    })
}

#[test]
fn request_rejects_zero_roots_path_escape_unsupported_domains_and_conflicts() {
    let base = valid_request_input();
    let mut zero = base.clone();
    zero.candidate_root_sha256 = "0".repeat(64);
    assert_eq!(
        LawLabSandboxRequestV1::seal(zero),
        Err(LawLabSandboxErrorV1::InvalidRequest)
    );

    let mut escape = base.clone();
    escape.operations = vec![LawLabSandboxOperationV1::RemoveWorkPath {
        work_path: "../production".to_owned(),
    }];
    assert_eq!(
        LawLabSandboxRequestV1::seal(escape),
        Err(LawLabSandboxErrorV1::UnsafePath)
    );

    let mut unsupported = base.clone();
    unsupported.domain = LawLabProbeDomainV1::Git;
    assert_eq!(
        LawLabSandboxRequestV1::seal(unsupported),
        Err(LawLabSandboxErrorV1::UnsupportedDomain)
    );

    let mut conflict = base;
    conflict.operations = vec![
        LawLabSandboxOperationV1::RemoveWorkPath {
            work_path: "records".to_owned(),
        },
        LawLabSandboxOperationV1::RemoveWorkPath {
            work_path: "records/one.json".to_owned(),
        },
    ];
    assert_eq!(
        LawLabSandboxRequestV1::seal(conflict),
        Err(LawLabSandboxErrorV1::OperationConflict)
    );
}

#[test]
fn tree_manifest_rejects_symlinks() {
    let fixture = TestFixtureV1::empty("symlink");
    let staging = fixture.base.join("staging");
    fs::create_dir(&staging).expect("staging");
    symlink("/etc/passwd", staging.join("escape")).expect("symlink");
    assert_eq!(
        LawLabTreeManifestV1::scan(&staging, 1024),
        Err(LawLabSandboxErrorV1::InvalidTree)
    );
}

#[test]
fn worker_sha_mismatch_fails_before_execution() {
    let fixture =
        TestFixtureV1::with_files("worker-mismatch", &[("value.txt", b"value".as_slice())]);
    let worker = worker_path();
    let config = LawLabSandboxConfigV1::generated_capability_self_test_v1(
        worker,
        sha256_bytes(b"wrong-worker"),
        fixture.source_store.clone(),
        fixture.workspace_store.clone(),
    );
    let adapter = LawLabSandboxAdapterV1::new(config);
    assert_eq!(
        adapter.executor_manifest(),
        Err(LawLabSandboxErrorV1::WorkerHashMismatch)
    );
}

#[test]
#[ignore = "requires Linux bwrap on the mini-PC"]
fn real_bwrap_filesystem_copy_delete_isolated_and_authority_free() {
    let fixture = TestFixtureV1::with_files(
        "filesystem",
        &[
            ("keep.txt", b"stable payload".as_slice()),
            ("remove.txt", b"remove me".as_slice()),
        ],
    );
    let adapter = fixture.adapter();
    let executor = adapter.executor_manifest().expect("executor manifest");
    let request = request(
        &executor,
        &fixture.source_manifest,
        LawLabProbeDomainV1::Filesystem,
        vec![
            LawLabSandboxOperationV1::CopySourceFile {
                source_path: "keep.txt".to_owned(),
                work_path: "copies/keep.txt".to_owned(),
            },
            LawLabSandboxOperationV1::RemoveWorkPath {
                work_path: "remove.txt".to_owned(),
            },
        ],
    )
    .expect("request");
    let execution = adapter.execute(&request).expect("sandbox execution");

    execution
        .receipt
        .validate(&request, &execution.worker_outcome)
        .expect("receipt");
    assert!(
        execution
            .worker_outcome
            .post_work_manifest
            .entry("remove.txt")
            .is_none()
    );
    let copied = execution
        .worker_outcome
        .post_work_manifest
        .entry("copies/keep.txt")
        .expect("copy");
    assert_eq!(copied.kind, LawLabTreeEntryKindV1::File);
    assert_eq!(
        copied.content_sha256.as_deref(),
        Some(root_of_bytes(b"stable payload").as_str())
    );
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
    assert!(execution.worker_outcome.isolation.forbidden_paths_absent);
    assert!(!execution.receipt.authority.prediction_commitments_written);
    assert!(!execution.receipt.authority.natural_holdout_satisfied);
    assert!(!execution.receipt.authority.law_certificate_issued);
    assert!(!execution.receipt.authority.execution_authority_granted);
    assert!(!execution.receipt.authority.k1_registry_mutated);
    assert!(workspace_is_empty(&fixture.workspace_store));

    let mut tampered = execution.receipt.clone();
    tampered.authority.k1_registry_mutated = true;
    assert_eq!(
        tampered.validate(&request, &execution.worker_outcome),
        Err(LawLabSandboxErrorV1::AuthorityBoundaryViolated)
    );
}

#[test]
#[ignore = "requires Linux bwrap on the mini-PC"]
fn real_bwrap_structured_json_is_canonical_and_exact() {
    let fixture = TestFixtureV1::with_files(
        "structured-json",
        &[(
            "record.json",
            br#"{ "z": 3, "a": {"b": 2, "a": 1} }"#.as_slice(),
        )],
    );
    let adapter = fixture.adapter();
    let executor = adapter.executor_manifest().expect("executor manifest");
    let request = request(
        &executor,
        &fixture.source_manifest,
        LawLabProbeDomainV1::StructuredData,
        vec![LawLabSandboxOperationV1::CanonicalizeJsonFile {
            work_path: "record.json".to_owned(),
        }],
    )
    .expect("request");
    let execution = adapter.execute(&request).expect("sandbox execution");
    let output = execution
        .worker_outcome
        .post_work_manifest
        .entry("record.json")
        .expect("record");
    let expected = br#"{"a":{"a":1,"b":2},"z":3}"#;
    assert_eq!(output.byte_length, expected.len() as u64);
    assert_eq!(
        output.content_sha256.as_deref(),
        Some(root_of_bytes(expected).as_str())
    );
    assert!(execution.receipt.cleanup.removed);
    assert!(execution.receipt.cleanup.verified_absent);
    assert!(workspace_is_empty(&fixture.workspace_store));
}

#[test]
#[ignore = "requires Linux bwrap on the mini-PC"]
fn real_bwrap_tight_deadline_times_out_and_workspace_is_cleaned() {
    let fixture =
        TestFixtureV1::with_files("timeout", &[("record.json", br#"{"value":1}"#.as_slice())]);
    let mut config = fixture.config();
    config
        .tighten_wall_timeout_for_self_test(1)
        .expect("tight timeout");
    let adapter = LawLabSandboxAdapterV1::new(config);
    let executor = adapter.executor_manifest().expect("executor manifest");
    let request = request(
        &executor,
        &fixture.source_manifest,
        LawLabProbeDomainV1::StructuredData,
        vec![LawLabSandboxOperationV1::CanonicalizeJsonFile {
            work_path: "record.json".to_owned(),
        }],
    )
    .expect("request");
    assert_eq!(
        adapter.execute(&request),
        Err(LawLabSandboxErrorV1::TimedOut)
    );
    assert!(workspace_is_empty(&fixture.workspace_store));
}

fn valid_request_input() -> LawLabSandboxRequestInputV1 {
    LawLabSandboxRequestInputV1 {
        executor_manifest_root_sha256: root("executor"),
        worker_sha256: root("worker"),
        candidate_root_sha256: root("candidate"),
        version_space_root_sha256: root("version-space"),
        durable_prediction_ledger_root_sha256: root("prediction-ledger"),
        probe_root_sha256: root("probe"),
        source_tree_root_sha256: root("source"),
        deterministic_seed_sha256: root("seed"),
        domain: LawLabProbeDomainV1::Filesystem,
        purpose: LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest,
        surviving_hypothesis_count: 2,
        precommitted_prediction_count: 2,
        operations: vec![LawLabSandboxOperationV1::RemoveWorkPath {
            work_path: "record.json".to_owned(),
        }],
    }
}

fn root_of_bytes(bytes: &[u8]) -> String {
    sha256_bytes(bytes)
}

fn worker_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nando-law-lab-sandbox-worker"))
}

fn workspace_is_empty(path: &Path) -> bool {
    fs::read_dir(path)
        .expect("workspace store")
        .next()
        .is_none()
}

struct TestFixtureV1 {
    base: PathBuf,
    source_store: PathBuf,
    workspace_store: PathBuf,
    source_manifest: LawLabTreeManifestV1,
}

impl TestFixtureV1 {
    fn empty(label: &str) -> Self {
        Self::with_files(label, &[])
    }

    fn with_files(label: &str, files: &[(&str, &[u8])]) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::var_os("NANDO_LAW_LAB_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .expect("current directory")
                    .join("target/law-lab-sandbox-tests")
            });
        fs::create_dir_all(&root).expect("test root");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).expect("test root mode");
        let base = root.join(format!("{label}-{}-{sequence}", std::process::id()));
        fs::create_dir(&base).expect("base");
        fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).expect("base mode");
        let source_store = base.join("sources");
        let workspace_store = base.join("workspaces");
        let staging = base.join("staging-source");
        for path in [&source_store, &workspace_store, &staging] {
            fs::create_dir(path).expect("fixture directory");
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .expect("fixture directory mode");
        }
        for (relative, bytes) in files {
            let path = staging.join(relative);
            if let Some(parent) = path.parent()
                && parent != staging
            {
                fs::create_dir_all(parent).expect("file parent");
            }
            let mut file = File::create(&path).expect("fixture file");
            file.write_all(bytes).expect("fixture write");
            file.sync_all().expect("fixture sync");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .expect("fixture file mode");
        }
        let source_manifest =
            LawLabTreeManifestV1::scan(&staging, 8 * 1024 * 1024).expect("source manifest");
        fs::rename(
            &staging,
            source_store.join(&source_manifest.tree_root_sha256),
        )
        .expect("seal source");
        Self {
            base,
            source_store,
            workspace_store,
            source_manifest,
        }
    }

    fn config(&self) -> LawLabSandboxConfigV1 {
        let worker = worker_path();
        let worker_sha256 = law_lab_sha256_file_v1(&worker).expect("worker sha");
        LawLabSandboxConfigV1::generated_capability_self_test_v1(
            worker,
            worker_sha256,
            self.source_store.clone(),
            self.workspace_store.clone(),
        )
    }

    fn adapter(&self) -> LawLabSandboxAdapterV1 {
        LawLabSandboxAdapterV1::new(self.config())
    }
}

impl Drop for TestFixtureV1 {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}
