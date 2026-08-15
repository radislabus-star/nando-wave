use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionLearnedEffectV1,
    K2CompositionOperationResultV1, K2CompositionResultV1, K2CompositionTreeManifestV1,
    composition_root_v1, composition_sha256_file_v1, valid_composition_path_v1,
};
use super::model::{
    K2_REPRESENTATION_MAX_DEPTH_V1, K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1,
    representation_bytes_v1, representation_decode_v1,
};

const REPRESENTATION_SANDBOX_REQUEST_SCHEMA_V1: &str = "nando.k2-representation-sandbox-request.v1";
const REPRESENTATION_SANDBOX_OUTCOME_SCHEMA_V1: &str = "nando.k2-representation-sandbox-outcome.v1";
const BWRAP_PATH_V1: &str = "/usr/bin/bwrap";
const PRLIMIT_PATH_V1: &str = "/usr/bin/prlimit";
const GUEST_WORKER_PATH_V1: &str = "/nando/bin/representation-worker";
static WORKSPACE_SEQUENCE_V1: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationSandboxRequestV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub worker_executable_sha256: String,
    pub initial_manifest: K2CompositionTreeManifestV1,
    pub operations: Vec<K2CompositionLearnedEffectV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2RepresentationSandboxRequestV1 {
    pub fn seal(
        experiment_id_sha256: String,
        worker_executable_sha256: String,
        initial_manifest: K2CompositionTreeManifestV1,
        operations: Vec<K2CompositionLearnedEffectV1>,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            REPRESENTATION_SANDBOX_REQUEST_SCHEMA_V1,
            &experiment_id_sha256,
            &worker_executable_sha256,
            &initial_manifest,
            &operations,
            &authority,
        ))?;
        let request = Self {
            schema: REPRESENTATION_SANDBOX_REQUEST_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            worker_executable_sha256,
            initial_manifest,
            operations,
            authority,
            request_root_sha256,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.initial_manifest.validate()?;
        self.authority.validate()?;
        if self.schema != REPRESENTATION_SANDBOX_REQUEST_SCHEMA_V1
            || self.operations.is_empty()
            || self.operations.len() > K2_REPRESENTATION_MAX_DEPTH_V1 as usize
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_sandbox_request_invalid",
            ));
        }
        for operation in &self.operations {
            operation.validate()?;
        }
        let expected = composition_root_v1(&(
            REPRESENTATION_SANDBOX_REQUEST_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.worker_executable_sha256,
            &self.initial_manifest,
            &self.operations,
            &self.authority,
        ))?;
        if expected != self.request_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_sandbox_request_root_mismatch",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationSandboxOutcomeV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub worker_executable_sha256: String,
    pub pre_manifest: K2CompositionTreeManifestV1,
    pub post_manifest: K2CompositionTreeManifestV1,
    pub operation_results: Vec<K2CompositionOperationResultV1>,
    pub success: bool,
    pub failed_step: Option<u64>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

impl K2RepresentationSandboxOutcomeV1 {
    fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.outcome_root_sha256 = composition_root_v1(&(
            REPRESENTATION_SANDBOX_OUTCOME_SCHEMA_V1,
            &self.request_root_sha256,
            &self.worker_executable_sha256,
            &self.pre_manifest,
            &self.post_manifest,
            &self.operation_results,
            self.success,
            self.failed_step,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2RepresentationSandboxExecutionV1 {
    pub outcome: K2RepresentationSandboxOutcomeV1,
    pub adapter_observed_post: K2CompositionTreeManifestV1,
    pub source_integrity_preserved: bool,
    pub workspace_removed: bool,
    pub cleanup_root_sha256: String,
}

#[derive(Clone, Debug)]
pub struct K2RepresentationSandboxAdapterV1 {
    worker_path: PathBuf,
    worker_executable_sha256: String,
    workspace_store_root: PathBuf,
}

impl K2RepresentationSandboxAdapterV1 {
    pub fn new(
        worker_path: PathBuf,
        worker_executable_sha256: String,
        workspace_store_root: PathBuf,
    ) -> K2CompositionResultV1<Self> {
        if composition_sha256_file_v1(&worker_path)? != worker_executable_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_sandbox_worker_hash_mismatch",
            ));
        }
        fs::create_dir_all(&workspace_store_root)
            .map_err(|_| K2CompositionErrorV1::Io("create_representation_workspace_store"))?;
        Ok(Self {
            worker_path,
            worker_executable_sha256,
            workspace_store_root,
        })
    }

    pub fn execute(
        &self,
        request: &K2RepresentationSandboxRequestV1,
        initial_files: &BTreeMap<String, Vec<u8>>,
    ) -> K2CompositionResultV1<K2RepresentationSandboxExecutionV1> {
        request.validate()?;
        if request.worker_executable_sha256 != self.worker_executable_sha256
            || K2CompositionTreeManifestV1::from_files(initial_files)? != request.initial_manifest
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_sandbox_input_binding_invalid",
            ));
        }
        let sequence = WORKSPACE_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed);
        let workspace_root = self
            .workspace_store_root
            .join(format!("{}-{sequence}", &request.request_root_sha256[..16]));
        let source_root = workspace_root.join("source");
        let work_root = workspace_root.join("work");
        fs::create_dir_all(&source_root)
            .and_then(|_| fs::create_dir_all(&work_root))
            .map_err(|_| K2CompositionErrorV1::Io("create_representation_workspace"))?;
        materialize_files_v1(&source_root, initial_files)?;
        materialize_files_v1(&work_root, initial_files)?;
        let source_before = K2CompositionTreeManifestV1::scan(&source_root)?;
        let output = run_worker_v1(
            &self.worker_path,
            &source_root,
            &work_root,
            &representation_bytes_v1(request)?,
        )?;
        let outcome: K2RepresentationSandboxOutcomeV1 = representation_decode_v1(&output)?;
        validate_worker_outcome_v1(request, &outcome)?;
        let source_after = K2CompositionTreeManifestV1::scan(&source_root)?;
        let adapter_observed_post = K2CompositionTreeManifestV1::scan(&work_root)?;
        if source_before != source_after || adapter_observed_post != outcome.post_manifest {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_adapter_filesystem_parity_failure",
            ));
        }
        let expected = replay_operations_v1(initial_files, &request.operations)?;
        if K2CompositionTreeManifestV1::from_files(&expected.files)? != adapter_observed_post
            || expected.results != outcome.operation_results
            || expected.success != outcome.success
            || expected.failed_step != outcome.failed_step
        {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_adapter_operation_parity_failure",
            ));
        }
        fs::remove_dir_all(&workspace_root)
            .map_err(|_| K2CompositionErrorV1::Io("remove_representation_workspace"))?;
        let workspace_removed = !workspace_root.exists();
        let cleanup_root_sha256 = composition_root_v1(&(
            "nando.k2-representation-sandbox-cleanup.v1",
            &request.request_root_sha256,
            sequence,
            workspace_removed,
        ))?;
        Ok(K2RepresentationSandboxExecutionV1 {
            outcome,
            adapter_observed_post,
            source_integrity_preserved: true,
            workspace_removed,
            cleanup_root_sha256,
        })
    }
}

pub fn run_representation_sandbox_worker_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_representation_worker_stdin"))?;
    let request: K2RepresentationSandboxRequestV1 = representation_decode_v1(&input)?;
    request.validate()?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_representation_worker"))?;
    if composition_sha256_file_v1(&executable)? != request.worker_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_worker_executable_mismatch",
        ));
    }
    let work_root = Path::new("/work");
    let pre_manifest = K2CompositionTreeManifestV1::scan(work_root)?;
    if pre_manifest != request.initial_manifest {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_worker_pre_manifest_mismatch",
        ));
    }
    let mut results = Vec::with_capacity(request.operations.len());
    let mut success = true;
    let mut failed_step = None;
    for (step, operation) in request.operations.iter().enumerate() {
        match worker_apply_v1(work_root, operation) {
            Ok(()) => results.push(K2CompositionOperationResultV1 {
                step: step as u64,
                applied: true,
                reason: "applied".to_owned(),
            }),
            Err(reason) => {
                results.push(K2CompositionOperationResultV1 {
                    step: step as u64,
                    applied: false,
                    reason: reason.to_owned(),
                });
                success = false;
                failed_step = Some(step as u64);
                break;
            }
        }
    }
    let post_manifest = K2CompositionTreeManifestV1::scan(work_root)?;
    let mut outcome = K2RepresentationSandboxOutcomeV1 {
        schema: REPRESENTATION_SANDBOX_OUTCOME_SCHEMA_V1.to_owned(),
        request_root_sha256: request.request_root_sha256,
        worker_executable_sha256: request.worker_executable_sha256,
        pre_manifest,
        post_manifest,
        operation_results: results,
        success,
        failed_step,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        outcome_root_sha256: String::new(),
    };
    outcome.reseal()?;
    std::io::stdout()
        .write_all(&representation_bytes_v1(&outcome)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_representation_worker_stdout"))
}

struct ReplayOutcomeV1 {
    files: BTreeMap<String, Vec<u8>>,
    results: Vec<K2CompositionOperationResultV1>,
    success: bool,
    failed_step: Option<u64>,
}

fn replay_operations_v1(
    initial: &BTreeMap<String, Vec<u8>>,
    operations: &[K2CompositionLearnedEffectV1],
) -> K2CompositionResultV1<ReplayOutcomeV1> {
    let mut files = initial.clone();
    let mut results = Vec::new();
    let mut success = true;
    let mut failed_step = None;
    for (step, operation) in operations.iter().enumerate() {
        let applied = match operation {
            K2CompositionLearnedEffectV1::CopyFile {
                source_path,
                target_path,
            } => files.get(source_path).cloned().map(|bytes| {
                files.insert(target_path.clone(), bytes);
            }),
            K2CompositionLearnedEffectV1::RemoveFile { path } => files.remove(path).map(drop),
        };
        match applied {
            Some(()) => results.push(K2CompositionOperationResultV1 {
                step: step as u64,
                applied: true,
                reason: "applied".to_owned(),
            }),
            None => {
                let reason = match operation {
                    K2CompositionLearnedEffectV1::CopyFile { .. } => "copy_source_missing",
                    K2CompositionLearnedEffectV1::RemoveFile { .. } => "remove_path_missing",
                };
                results.push(K2CompositionOperationResultV1 {
                    step: step as u64,
                    applied: false,
                    reason: reason.to_owned(),
                });
                success = false;
                failed_step = Some(step as u64);
                break;
            }
        }
    }
    Ok(ReplayOutcomeV1 {
        files,
        results,
        success,
        failed_step,
    })
}

fn worker_apply_v1(
    work_root: &Path,
    operation: &K2CompositionLearnedEffectV1,
) -> Result<(), &'static str> {
    match operation {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => {
            if !valid_composition_path_v1(source_path) || !valid_composition_path_v1(target_path) {
                return Err("path_invalid");
            }
            let source = work_root.join(source_path);
            if !source.is_file() {
                return Err("copy_source_missing");
            }
            let target = work_root.join(target_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|_| "create_target_parent_failed")?;
            }
            fs::copy(source, target).map_err(|_| "copy_failed")?;
            Ok(())
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if !valid_composition_path_v1(path) {
                return Err("path_invalid");
            }
            let target = work_root.join(path);
            if !target.is_file() {
                return Err("remove_path_missing");
            }
            fs::remove_file(target).map_err(|_| "remove_failed")
        }
    }
}

fn validate_worker_outcome_v1(
    request: &K2RepresentationSandboxRequestV1,
    outcome: &K2RepresentationSandboxOutcomeV1,
) -> K2CompositionResultV1<()> {
    outcome.pre_manifest.validate()?;
    outcome.post_manifest.validate()?;
    outcome.authority.validate()?;
    let mut resealed = outcome.clone();
    resealed.reseal()?;
    if outcome.schema != REPRESENTATION_SANDBOX_OUTCOME_SCHEMA_V1
        || outcome.request_root_sha256 != request.request_root_sha256
        || outcome.worker_executable_sha256 != request.worker_executable_sha256
        || outcome.pre_manifest != request.initial_manifest
        || resealed.outcome_root_sha256 != outcome.outcome_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "representation_worker_outcome_invalid",
        ));
    }
    Ok(())
}

fn materialize_files_v1(
    root: &Path,
    files: &BTreeMap<String, Vec<u8>>,
) -> K2CompositionResultV1<()> {
    for (path, bytes) in files {
        if !valid_composition_path_v1(path) {
            return Err(K2CompositionErrorV1::Invalid(
                "representation_materialize_path_invalid",
            ));
        }
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| K2CompositionErrorV1::Io("create_representation_parent"))?;
        }
        fs::write(target, bytes)
            .map_err(|_| K2CompositionErrorV1::Io("write_representation_fixture"))?;
    }
    Ok(())
}

fn run_worker_v1(
    worker: &Path,
    source_root: &Path,
    work_root: &Path,
    input: &[u8],
) -> K2CompositionResultV1<Vec<u8>> {
    let mut command = Command::new(BWRAP_PATH_V1);
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
    let args = [
        OsString::from("--proc"),
        OsString::from("/proc"),
        OsString::from("--dev"),
        OsString::from("/dev"),
        OsString::from("--tmpfs"),
        OsString::from("/tmp"),
        OsString::from("--dir"),
        OsString::from("/nando"),
        OsString::from("--dir"),
        OsString::from("/nando/bin"),
        OsString::from("--ro-bind"),
        worker.as_os_str().to_owned(),
        OsString::from(GUEST_WORKER_PATH_V1),
        OsString::from("--ro-bind"),
        source_root.as_os_str().to_owned(),
        OsString::from("/source"),
        OsString::from("--bind"),
        work_root.as_os_str().to_owned(),
        OsString::from("/work"),
        OsString::from("--chdir"),
        OsString::from("/work"),
        OsString::from("--setenv"),
        OsString::from("HOME"),
        OsString::from("/tmp"),
        OsString::from("--setenv"),
        OsString::from("LANG"),
        OsString::from("C"),
        OsString::from("--"),
        OsString::from(PRLIMIT_PATH_V1),
        OsString::from("--cpu=3:3"),
        OsString::from("--as=268435456:268435456"),
        OsString::from("--nproc=16:16"),
        OsString::from("--fsize=1048576:1048576"),
        OsString::from("--"),
        OsString::from(GUEST_WORKER_PATH_V1),
    ];
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|_| K2CompositionErrorV1::Io("spawn_representation_worker"))?;
    child
        .stdin
        .take()
        .ok_or(K2CompositionErrorV1::Io("representation_worker_stdin"))?
        .write_all(input)
        .map_err(|_| K2CompositionErrorV1::Io("write_representation_worker_stdin"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or(K2CompositionErrorV1::Io("representation_worker_stdout"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or(K2CompositionErrorV1::Io("representation_worker_stderr"))?;
    let stdout_thread = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let stderr_thread = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| K2CompositionErrorV1::Io("poll_representation_worker"))?
        {
            break status;
        }
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            return Err(K2CompositionErrorV1::Invalid(
                "representation_worker_timeout",
            ));
        }
        thread::sleep(Duration::from_millis(2));
    };
    let stdout = stdout_thread
        .join()
        .map_err(|_| K2CompositionErrorV1::Io("join_representation_stdout"))?;
    let stderr = stderr_thread
        .join()
        .map_err(|_| K2CompositionErrorV1::Io("join_representation_stderr"))?;
    if !status.success() || stdout.len() > K2_REPRESENTATION_MAX_PROTOCOL_BYTES_V1 {
        let _ = stderr;
        return Err(K2CompositionErrorV1::Invalid(
            "representation_worker_failed",
        ));
    }
    Ok(stdout)
}
