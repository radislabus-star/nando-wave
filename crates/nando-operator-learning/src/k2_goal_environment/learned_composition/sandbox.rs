use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::model::{
    K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1, K2_COMPOSITION_SANDBOX_OUTCOME_SCHEMA_V1,
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionLearnedEffectV1,
    K2CompositionOperationResultV1, K2CompositionResultV1, K2CompositionSandboxOutcomeV1,
    K2CompositionSandboxRequestV1, K2CompositionTreeManifestV1, composition_bytes_v1,
    composition_decode_v1, composition_root_v1, composition_sha256_file_v1,
    valid_composition_path_v1,
};

const BWRAP_PATH_V1: &str = "/usr/bin/bwrap";
const PRLIMIT_PATH_V1: &str = "/usr/bin/prlimit";
const GUEST_WORKER_PATH_V1: &str = "/nando/bin/sequential-worker";
const PROCESS_STDERR_LIMIT_V1: usize = 8 * 1024;
static WORKSPACE_SEQUENCE_V1: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct K2CompositionSandboxAdapterV1 {
    worker_path: PathBuf,
    worker_executable_sha256: String,
    workspace_store_root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionSandboxExecutionV1 {
    pub outcome: K2CompositionSandboxOutcomeV1,
    pub adapter_observed_post: K2CompositionTreeManifestV1,
    pub source_integrity_preserved: bool,
    pub workspace_removed: bool,
    pub cleanup_root_sha256: String,
}

struct K2CompositionAdapterReplayV1 {
    files: BTreeMap<String, Vec<u8>>,
    operation_results: Vec<K2CompositionOperationResultV1>,
    success: bool,
    failed_step: Option<u64>,
}

impl K2CompositionSandboxAdapterV1 {
    pub fn new(
        worker_path: PathBuf,
        worker_executable_sha256: String,
        workspace_store_root: PathBuf,
    ) -> K2CompositionResultV1<Self> {
        if composition_sha256_file_v1(&worker_path)? != worker_executable_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "sandbox_worker_hash_mismatch",
            ));
        }
        fs::create_dir_all(&workspace_store_root)
            .map_err(|_| K2CompositionErrorV1::Io("create_workspace_store"))?;
        Ok(Self {
            worker_path,
            worker_executable_sha256,
            workspace_store_root,
        })
    }

    pub fn execute(
        &self,
        request: &K2CompositionSandboxRequestV1,
        initial_files: &BTreeMap<String, Vec<u8>>,
    ) -> K2CompositionResultV1<K2CompositionSandboxExecutionV1> {
        request.validate()?;
        if request.worker_executable_sha256 != self.worker_executable_sha256
            || K2CompositionTreeManifestV1::from_files(initial_files)? != request.initial_manifest
        {
            return Err(K2CompositionErrorV1::Invalid(
                "sandbox_input_binding_invalid",
            ));
        }
        let mut workspace = SandboxWorkspaceGuardV1::create(
            &self.workspace_store_root,
            &request.request_root_sha256,
        )?;
        materialize_files_v1(workspace.source_root(), initial_files)?;
        materialize_files_v1(workspace.work_root(), initial_files)?;
        let source_before = K2CompositionTreeManifestV1::scan(workspace.source_root())?;
        let command = self.command_v1(workspace.source_root(), workspace.work_root());
        let output = run_bounded_command_v1(
            &command,
            &composition_bytes_v1(request)?,
            Duration::from_secs(3),
        )?;
        let outcome: K2CompositionSandboxOutcomeV1 = composition_decode_v1(&output)?;
        validate_worker_outcome_v1(request, &outcome)?;
        let source_after = K2CompositionTreeManifestV1::scan(workspace.source_root())?;
        let adapter_observed_post = K2CompositionTreeManifestV1::scan(workspace.work_root())?;
        if source_after != source_before || adapter_observed_post != outcome.post_manifest {
            return Err(K2CompositionErrorV1::Invalid(
                "adapter_filesystem_parity_failure",
            ));
        }
        let expected = adapter_replay_v1(initial_files, &request.operations)?;
        if K2CompositionTreeManifestV1::from_files(&expected.files)? != adapter_observed_post
            || expected.operation_results != outcome.operation_results
            || expected.success != outcome.success
            || expected.failed_step != outcome.failed_step
        {
            return Err(K2CompositionErrorV1::Invalid(
                "adapter_operation_parity_failure",
            ));
        }
        let workspace_id = workspace.workspace_id().to_owned();
        workspace.cleanup()?;
        let workspace_removed = !workspace.root().exists();
        if !workspace_removed {
            return Err(K2CompositionErrorV1::Invalid("sandbox_cleanup_failed"));
        }
        let cleanup_root_sha256 = composition_root_v1(&(
            "nando.k2-composition-sandbox-cleanup.v1",
            &workspace_id,
            &request.request_root_sha256,
            true,
        ))?;
        Ok(K2CompositionSandboxExecutionV1 {
            outcome,
            adapter_observed_post,
            source_integrity_preserved: true,
            workspace_removed,
            cleanup_root_sha256,
        })
    }

    fn command_v1(&self, source_root: &Path, work_root: &Path) -> CommandSpecV1 {
        let mut args = vec![
            OsString::from("--unshare-all"),
            OsString::from("--die-with-parent"),
            OsString::from("--new-session"),
            OsString::from("--cap-drop"),
            OsString::from("ALL"),
            OsString::from("--clearenv"),
        ];
        for path in ["/usr", "/lib", "/lib64"] {
            if Path::new(path).exists() {
                args.extend([
                    OsString::from("--ro-bind"),
                    OsString::from(path),
                    OsString::from(path),
                ]);
            }
        }
        args.extend([
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
            self.worker_path.as_os_str().to_owned(),
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
            OsString::from("--setenv"),
            OsString::from("TZ"),
            OsString::from("UTC"),
            OsString::from("--"),
            OsString::from(PRLIMIT_PATH_V1),
            OsString::from("--cpu=2:2"),
            OsString::from("--as=268435456:268435456"),
            OsString::from("--nproc=16:16"),
            OsString::from("--fsize=1048576:1048576"),
            OsString::from("--"),
            OsString::from(GUEST_WORKER_PATH_V1),
        ]);
        CommandSpecV1 {
            program: PathBuf::from(BWRAP_PATH_V1),
            args,
        }
    }
}

pub fn run_composition_sequential_worker_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_worker_stdin"))?;
    let request: K2CompositionSandboxRequestV1 = composition_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_worker_executable"))?;
    if composition_sha256_file_v1(&executable)? != request.worker_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid("worker_executable_mismatch"));
    }
    let work_root = Path::new("/work");
    let pre_manifest = K2CompositionTreeManifestV1::scan(work_root)?;
    if pre_manifest != request.initial_manifest {
        return Err(K2CompositionErrorV1::Invalid(
            "worker_pre_manifest_mismatch",
        ));
    }
    let mut operation_results = Vec::with_capacity(request.operations.len());
    let mut success = true;
    let mut failed_step = None;
    for (step, operation) in request.operations.iter().enumerate() {
        let result = worker_apply_operation_v1(work_root, operation);
        match result {
            Ok(()) => operation_results.push(K2CompositionOperationResultV1 {
                step: step as u64,
                applied: true,
                reason: "applied".to_owned(),
            }),
            Err(reason) => {
                operation_results.push(K2CompositionOperationResultV1 {
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
    let authority = K2CompositionAuthorityBoundaryV1::denied();
    let mut outcome = K2CompositionSandboxOutcomeV1 {
        schema: K2_COMPOSITION_SANDBOX_OUTCOME_SCHEMA_V1.to_owned(),
        request_root_sha256: request.request_root_sha256,
        worker_executable_sha256: request.worker_executable_sha256,
        pre_manifest,
        post_manifest,
        operation_results,
        success,
        failed_step,
        authority,
        outcome_root_sha256: String::new(),
    };
    outcome.reseal()?;
    std::io::stdout()
        .write_all(&composition_bytes_v1(&outcome)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_worker_stdout"))
}

fn worker_apply_operation_v1(
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

fn adapter_replay_v1(
    initial: &BTreeMap<String, Vec<u8>>,
    operations: &[K2CompositionLearnedEffectV1],
) -> K2CompositionResultV1<K2CompositionAdapterReplayV1> {
    let mut files = initial.clone();
    let mut results = Vec::with_capacity(operations.len());
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
        if applied.is_some() {
            results.push(K2CompositionOperationResultV1 {
                step: step as u64,
                applied: true,
                reason: "applied".to_owned(),
            });
        } else {
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
    Ok(K2CompositionAdapterReplayV1 {
        files,
        operation_results: results,
        success,
        failed_step,
    })
}

fn validate_worker_outcome_v1(
    request: &K2CompositionSandboxRequestV1,
    outcome: &K2CompositionSandboxOutcomeV1,
) -> K2CompositionResultV1<()> {
    outcome.pre_manifest.validate()?;
    outcome.post_manifest.validate()?;
    outcome.authority.validate()?;
    let supplied_root = outcome.outcome_root_sha256.clone();
    let mut resealed = outcome.clone();
    resealed.reseal()?;
    if outcome.schema != K2_COMPOSITION_SANDBOX_OUTCOME_SCHEMA_V1
        || outcome.request_root_sha256 != request.request_root_sha256
        || outcome.worker_executable_sha256 != request.worker_executable_sha256
        || outcome.pre_manifest != request.initial_manifest
        || resealed.outcome_root_sha256 != supplied_root
    {
        return Err(K2CompositionErrorV1::Invalid("worker_outcome_invalid"));
    }
    Ok(())
}

fn materialize_files_v1(
    root: &Path,
    files: &BTreeMap<String, Vec<u8>>,
) -> K2CompositionResultV1<()> {
    for (relative, bytes) in files {
        if !valid_composition_path_v1(relative) {
            return Err(K2CompositionErrorV1::Invalid("materialize_path_invalid"));
        }
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| K2CompositionErrorV1::Io("create_fixture_parent"))?;
        }
        fs::write(path, bytes).map_err(|_| K2CompositionErrorV1::Io("write_fixture_file"))?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct CommandSpecV1 {
    program: PathBuf,
    args: Vec<OsString>,
}

fn run_bounded_command_v1(
    spec: &CommandSpecV1,
    input: &[u8],
    deadline: Duration,
) -> K2CompositionResultV1<Vec<u8>> {
    let mut child = Command::new(&spec.program)
        .args(&spec.args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| K2CompositionErrorV1::Process("spawn_bwrap_failed"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or(K2CompositionErrorV1::Process("worker_stdin_missing"))?;
    stdin
        .write_all(input)
        .map_err(|_| K2CompositionErrorV1::Io("write_worker_stdin"))?;
    drop(stdin);
    let mut stdout = child
        .stdout
        .take()
        .ok_or(K2CompositionErrorV1::Process("worker_stdout_missing"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or(K2CompositionErrorV1::Process("worker_stderr_missing"))?;
    let stdout_reader =
        thread::spawn(move || read_limited_v1(&mut stdout, K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1));
    let stderr_reader =
        thread::spawn(move || read_limited_v1(&mut stderr, PROCESS_STDERR_LIMIT_V1));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| K2CompositionErrorV1::Process("worker_wait_failed"))?
        {
            break status;
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(K2CompositionErrorV1::Process("worker_timed_out"));
        }
        thread::sleep(Duration::from_millis(2));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| K2CompositionErrorV1::Process("stdout_reader_panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| K2CompositionErrorV1::Process("stderr_reader_panicked"))??;
    if !status.success() {
        if std::env::var_os("NANDO_K2_COMPOSITION_DEBUG_STDERR").is_some() {
            eprintln!("{}", String::from_utf8_lossy(&stderr));
        }
        return Err(K2CompositionErrorV1::Process("worker_failed"));
    }
    Ok(stdout)
}

fn read_limited_v1(reader: &mut impl Read, maximum: usize) -> K2CompositionResultV1<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take((maximum + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| K2CompositionErrorV1::Io("read_process_pipe"))?;
    if bytes.len() > maximum {
        return Err(K2CompositionErrorV1::Process("process_output_too_large"));
    }
    Ok(bytes)
}

struct SandboxWorkspaceGuardV1 {
    root: PathBuf,
    source_root: PathBuf,
    work_root: PathBuf,
    workspace_id: String,
    cleaned: bool,
}

impl SandboxWorkspaceGuardV1 {
    fn create(store: &Path, request_root: &str) -> K2CompositionResultV1<Self> {
        let sequence = WORKSPACE_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| K2CompositionErrorV1::Io("workspace_clock"))?
            .as_nanos();
        let workspace_id = composition_root_v1(&(
            "nando.k2-composition-workspace.v1",
            request_root,
            std::process::id(),
            now,
            sequence,
        ))?;
        let root = store.join(&workspace_id);
        let source_root = root.join("source");
        let work_root = root.join("work");
        fs::create_dir(&root).map_err(|_| K2CompositionErrorV1::Io("create_workspace"))?;
        fs::create_dir(&source_root)
            .map_err(|_| K2CompositionErrorV1::Io("create_workspace_source"))?;
        fs::create_dir(&work_root)
            .map_err(|_| K2CompositionErrorV1::Io("create_workspace_work"))?;
        Ok(Self {
            root,
            source_root,
            work_root,
            workspace_id,
            cleaned: false,
        })
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn source_root(&self) -> &Path {
        &self.source_root
    }

    fn work_root(&self) -> &Path {
        &self.work_root
    }

    fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    fn cleanup(&mut self) -> K2CompositionResultV1<()> {
        fs::remove_dir_all(&self.root).map_err(|_| K2CompositionErrorV1::Io("remove_workspace"))?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for SandboxWorkspaceGuardV1 {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
