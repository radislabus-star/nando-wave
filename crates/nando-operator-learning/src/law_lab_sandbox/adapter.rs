use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256};

use super::manifest::{
    LawLabSandboxExecutorManifestInputV1, LawLabSandboxExecutorManifestV1, LawLabTreeEntryKindV1,
    LawLabTreeManifestV1, law_lab_sha256_file_v1,
};
use super::model::{
    LawLabSandboxCleanupProofV1, LawLabSandboxErrorV1, LawLabSandboxExecutionV1,
    LawLabSandboxOperationResultV1, LawLabSandboxOperationV1, LawLabSandboxPurposeV1,
    LawLabSandboxReceiptV1, LawLabSandboxRequestV1, LawLabSandboxWorkerOutcomeV1,
    deterministic_environment_v1,
};
use crate::{
    LAW_LAB_MAX_INPUT_BYTES_V1, LAW_LAB_MAX_OUTPUT_BYTES_V1, LAW_LAB_MAX_PROBE_CPU_MS_V1,
    LAW_LAB_MAX_PROBE_WALL_MS_V1,
};

const BWRAP_PATH_V1: &str = "/usr/bin/bwrap";
const PRLIMIT_PATH_V1: &str = "/usr/bin/prlimit";
const GUEST_WORKER_PATH_V1: &str = "/nando/bin/nando-law-lab-sandbox-worker";
const WORKSPACE_CREATE_ATTEMPTS_V1: u64 = 32;
const STDERR_LIMIT_V1: usize = 64 * 1024;
static WORKSPACE_SEQUENCE_V1: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LawLabSandboxConfigV1 {
    bwrap_path: PathBuf,
    prlimit_path: PathBuf,
    worker_path: PathBuf,
    expected_worker_sha256: String,
    source_store_root: PathBuf,
    workspace_store_root: PathBuf,
    root_owned_worker_required: bool,
    root_owned_source_snapshot_required: bool,
    content_addressed_worker_path_required: bool,
    generated_capability_fixture_only: bool,
    wall_timeout_ms: u64,
}

impl LawLabSandboxConfigV1 {
    #[must_use]
    pub fn strict_v1(
        worker_path: PathBuf,
        expected_worker_sha256: String,
        source_store_root: PathBuf,
        workspace_store_root: PathBuf,
    ) -> Self {
        Self {
            bwrap_path: PathBuf::from(BWRAP_PATH_V1),
            prlimit_path: PathBuf::from(PRLIMIT_PATH_V1),
            worker_path,
            expected_worker_sha256,
            source_store_root,
            workspace_store_root,
            root_owned_worker_required: true,
            root_owned_source_snapshot_required: true,
            content_addressed_worker_path_required: true,
            generated_capability_fixture_only: false,
            wall_timeout_ms: LAW_LAB_MAX_PROBE_WALL_MS_V1,
        }
    }

    #[must_use]
    pub fn generated_capability_self_test_v1(
        worker_path: PathBuf,
        expected_worker_sha256: String,
        source_store_root: PathBuf,
        workspace_store_root: PathBuf,
    ) -> Self {
        Self {
            bwrap_path: PathBuf::from(BWRAP_PATH_V1),
            prlimit_path: PathBuf::from(PRLIMIT_PATH_V1),
            worker_path,
            expected_worker_sha256,
            source_store_root,
            workspace_store_root,
            root_owned_worker_required: false,
            root_owned_source_snapshot_required: false,
            content_addressed_worker_path_required: false,
            generated_capability_fixture_only: true,
            wall_timeout_ms: LAW_LAB_MAX_PROBE_WALL_MS_V1,
        }
    }

    #[must_use]
    pub fn strict_generated_capability_self_test_v1(
        worker_path: PathBuf,
        expected_worker_sha256: String,
        source_store_root: PathBuf,
        workspace_store_root: PathBuf,
    ) -> Self {
        Self {
            bwrap_path: PathBuf::from(BWRAP_PATH_V1),
            prlimit_path: PathBuf::from(PRLIMIT_PATH_V1),
            worker_path,
            expected_worker_sha256,
            source_store_root,
            workspace_store_root,
            root_owned_worker_required: true,
            root_owned_source_snapshot_required: true,
            content_addressed_worker_path_required: true,
            generated_capability_fixture_only: true,
            wall_timeout_ms: LAW_LAB_MAX_PROBE_WALL_MS_V1,
        }
    }

    pub fn tighten_wall_timeout_for_self_test(
        &mut self,
        wall_timeout_ms: u64,
    ) -> Result<(), LawLabSandboxErrorV1> {
        if !self.generated_capability_fixture_only
            || wall_timeout_ms == 0
            || wall_timeout_ms > LAW_LAB_MAX_PROBE_WALL_MS_V1
        {
            return Err(LawLabSandboxErrorV1::ExecutorManifestInvalid);
        }
        self.wall_timeout_ms = wall_timeout_ms;
        Ok(())
    }

    #[must_use]
    pub fn source_store_root(&self) -> &Path {
        &self.source_store_root
    }

    #[must_use]
    pub fn workspace_store_root(&self) -> &Path {
        &self.workspace_store_root
    }
}

#[derive(Clone, Debug)]
pub struct LawLabSandboxAdapterV1 {
    config: LawLabSandboxConfigV1,
}

impl LawLabSandboxAdapterV1 {
    #[must_use]
    pub fn new(config: LawLabSandboxConfigV1) -> Self {
        Self { config }
    }

    pub fn executor_manifest(
        &self,
    ) -> Result<LawLabSandboxExecutorManifestV1, LawLabSandboxErrorV1> {
        self.validate_store_roots()?;
        validate_root_owned_executable_v1(&self.config.bwrap_path)?;
        validate_root_owned_executable_v1(&self.config.prlimit_path)?;
        let worker_metadata = validate_regular_executable_v1(&self.config.worker_path)?;
        if self.config.root_owned_worker_required
            && (worker_metadata.uid() != 0 || worker_metadata.mode() & 0o022 != 0)
        {
            return Err(LawLabSandboxErrorV1::ToolUntrusted);
        }
        if self.config.content_addressed_worker_path_required
            && self
                .config
                .worker_path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                != Some(self.config.expected_worker_sha256.as_str())
        {
            return Err(LawLabSandboxErrorV1::ToolUntrusted);
        }
        let worker_sha256 = law_lab_sha256_file_v1(&self.config.worker_path)?;
        if worker_sha256 != self.config.expected_worker_sha256 {
            return Err(LawLabSandboxErrorV1::WorkerHashMismatch);
        }
        let mut runtime_read_only_binds = ["/usr", "/lib", "/lib64"]
            .into_iter()
            .filter(|path| Path::new(path).exists())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        runtime_read_only_binds.sort();
        LawLabSandboxExecutorManifestV1::seal(LawLabSandboxExecutorManifestInputV1 {
            bwrap_host_path: path_to_string_v1(&self.config.bwrap_path)?,
            bwrap_sha256: law_lab_sha256_file_v1(&self.config.bwrap_path)?,
            prlimit_host_path: path_to_string_v1(&self.config.prlimit_path)?,
            prlimit_sha256: law_lab_sha256_file_v1(&self.config.prlimit_path)?,
            worker_host_path: path_to_string_v1(&self.config.worker_path)?,
            worker_sha256,
            source_store_host_path: path_to_string_v1(
                &self
                    .config
                    .source_store_root
                    .canonicalize()
                    .map_err(|_| LawLabSandboxErrorV1::Io)?,
            )?,
            workspace_store_host_path: path_to_string_v1(
                &self
                    .config
                    .workspace_store_root
                    .canonicalize()
                    .map_err(|_| LawLabSandboxErrorV1::Io)?,
            )?,
            root_owned_worker_required: self.config.root_owned_worker_required,
            root_owned_source_snapshot_required: self.config.root_owned_source_snapshot_required,
            content_addressed_worker_path_required: self
                .config
                .content_addressed_worker_path_required,
            generated_capability_fixture_only: self.config.generated_capability_fixture_only,
            runtime_read_only_binds,
            wall_ms: self.config.wall_timeout_ms,
        })
    }

    pub fn execute(
        &self,
        request: &LawLabSandboxRequestV1,
    ) -> Result<LawLabSandboxExecutionV1, LawLabSandboxErrorV1> {
        request.validate()?;
        if self.config.generated_capability_fixture_only
            != (request.purpose == LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest)
        {
            return Err(LawLabSandboxErrorV1::ToolUntrusted);
        }
        let executor_manifest = self.executor_manifest()?;
        if request.executor_manifest_root_sha256 != executor_manifest.manifest_root_sha256
            || request.worker_sha256 != executor_manifest.worker_sha256
        {
            return Err(LawLabSandboxErrorV1::ExecutorManifestMismatch);
        }
        let source_root = self.source_snapshot_path_v1(request)?;
        validate_source_snapshot_trust_v1(
            &source_root,
            self.config.root_owned_source_snapshot_required,
        )?;
        let source_manifest = LawLabTreeManifestV1::scan(&source_root, LAW_LAB_MAX_INPUT_BYTES_V1)?;
        if source_manifest.tree_root_sha256 != request.source_tree_root_sha256 {
            return Err(LawLabSandboxErrorV1::SourceManifestMismatch);
        }
        validate_operations_against_source_v1(request, &source_manifest)?;

        let mut workspace = WorkspaceGuardV1::create(
            &self.config.workspace_store_root,
            &request.request_root_sha256,
        )?;
        let execution_result = self.execute_in_workspace_v1(
            request,
            &executor_manifest,
            &source_root,
            &source_manifest,
            workspace.work_root(),
        );
        let source_integrity_result =
            LawLabTreeManifestV1::scan(&source_root, LAW_LAB_MAX_INPUT_BYTES_V1).and_then(
                |source_after| {
                    if source_after == source_manifest {
                        Ok(())
                    } else {
                        Err(LawLabSandboxErrorV1::SourceManifestMismatch)
                    }
                },
            );
        let execution_result = match (execution_result, source_integrity_result) {
            (_, Err(error)) => Err(error),
            (result, Ok(())) => result,
        };
        let cleanup_result = workspace.cleanup();
        match (execution_result, cleanup_result) {
            (Ok(outcome), Ok(cleanup)) => {
                let receipt = LawLabSandboxReceiptV1::seal(request, &outcome, cleanup)?;
                Ok(LawLabSandboxExecutionV1 {
                    receipt,
                    worker_outcome: outcome,
                })
            }
            (_, Err(error)) => Err(error),
            (Err(error), Ok(_)) => Err(error),
        }
    }

    fn execute_in_workspace_v1(
        &self,
        request: &LawLabSandboxRequestV1,
        executor_manifest: &LawLabSandboxExecutorManifestV1,
        source_root: &Path,
        source_manifest: &LawLabTreeManifestV1,
        work_root: &Path,
    ) -> Result<LawLabSandboxWorkerOutcomeV1, LawLabSandboxErrorV1> {
        let command = self.command_spec_v1(executor_manifest, source_root, work_root)?;
        let request_bytes = request.canonical_bytes()?;
        let process_output = run_command_v1(
            &command,
            &request_bytes,
            Duration::from_millis(executor_manifest.limits.wall_ms),
            LAW_LAB_MAX_OUTPUT_BYTES_V1 as usize,
        )?;
        let outcome: LawLabSandboxWorkerOutcomeV1 = serde_json::from_slice(&process_output)
            .map_err(|_| LawLabSandboxErrorV1::WorkerProtocolFailed)?;
        if canonical_json_bytes(&outcome).map_err(|_| LawLabSandboxErrorV1::Serialization)?
            != process_output
        {
            return Err(LawLabSandboxErrorV1::WorkerProtocolFailed);
        }
        outcome.validate(request)?;
        if outcome.worker_sha256 != executor_manifest.worker_sha256 {
            return Err(LawLabSandboxErrorV1::WorkerHashMismatch);
        }
        let source_after = LawLabTreeManifestV1::scan(source_root, LAW_LAB_MAX_INPUT_BYTES_V1)?;
        if &source_after != source_manifest {
            return Err(LawLabSandboxErrorV1::SourceManifestMismatch);
        }
        let post_work = LawLabTreeManifestV1::scan(
            work_root,
            LAW_LAB_MAX_INPUT_BYTES_V1 + LAW_LAB_MAX_OUTPUT_BYTES_V1,
        )?;
        if post_work != outcome.post_work_manifest {
            return Err(LawLabSandboxErrorV1::IndependentVerificationFailed);
        }
        verify_operations_independently_v1(
            request,
            source_root,
            work_root,
            source_manifest,
            &post_work,
            &outcome.operation_results,
        )?;
        Ok(outcome)
    }

    fn source_snapshot_path_v1(
        &self,
        request: &LawLabSandboxRequestV1,
    ) -> Result<PathBuf, LawLabSandboxErrorV1> {
        let source_store = self
            .config
            .source_store_root
            .canonicalize()
            .map_err(|_| LawLabSandboxErrorV1::Io)?;
        let source = source_store.join(&request.source_tree_root_sha256);
        if !source.try_exists().map_err(|_| LawLabSandboxErrorV1::Io)? {
            return Err(LawLabSandboxErrorV1::SourceSnapshotMissing);
        }
        let canonical_source = source
            .canonicalize()
            .map_err(|_| LawLabSandboxErrorV1::Io)?;
        if canonical_source.parent() != Some(source_store.as_path())
            || canonical_source.file_name().and_then(|name| name.to_str())
                != Some(request.source_tree_root_sha256.as_str())
        {
            return Err(LawLabSandboxErrorV1::SourceSnapshotMissing);
        }
        Ok(canonical_source)
    }

    fn validate_store_roots(&self) -> Result<(), LawLabSandboxErrorV1> {
        if self.config.bwrap_path != Path::new(BWRAP_PATH_V1)
            || self.config.prlimit_path != Path::new(PRLIMIT_PATH_V1)
        {
            return Err(LawLabSandboxErrorV1::ToolUntrusted);
        }
        let source = validate_store_root_v1(&self.config.source_store_root)?;
        let workspace = validate_store_root_v1(&self.config.workspace_store_root)?;
        if source == workspace || source.starts_with(&workspace) || workspace.starts_with(&source) {
            return Err(LawLabSandboxErrorV1::ToolUntrusted);
        }
        Ok(())
    }

    fn command_spec_v1(
        &self,
        executor_manifest: &LawLabSandboxExecutorManifestV1,
        source_root: &Path,
        work_root: &Path,
    ) -> Result<LawLabSandboxCommandSpecV1, LawLabSandboxErrorV1> {
        let cpu_seconds = LAW_LAB_MAX_PROBE_CPU_MS_V1.div_ceil(1_000);
        let mut args = vec![
            OsString::from("--unshare-all"),
            OsString::from("--die-with-parent"),
            OsString::from("--new-session"),
            OsString::from("--cap-drop"),
            OsString::from("ALL"),
            OsString::from("--clearenv"),
        ];
        for path in &executor_manifest.runtime_read_only_binds {
            args.push(OsString::from("--ro-bind"));
            args.push(OsString::from(path));
            args.push(OsString::from(path));
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
            self.config.worker_path.as_os_str().to_owned(),
            OsString::from(GUEST_WORKER_PATH_V1),
            OsString::from("--ro-bind"),
            source_root.as_os_str().to_owned(),
            OsString::from("/source"),
            OsString::from("--bind"),
            work_root.as_os_str().to_owned(),
            OsString::from("/work"),
            OsString::from("--chdir"),
            OsString::from("/work"),
        ]);
        for entry in deterministic_environment_v1() {
            args.push(OsString::from("--setenv"));
            args.push(OsString::from(entry.name));
            args.push(OsString::from(entry.value));
        }
        args.extend([
            OsString::from("--"),
            OsString::from(PRLIMIT_PATH_V1),
            OsString::from(format!("--cpu={cpu_seconds}:{cpu_seconds}")),
            OsString::from(format!(
                "--as={0}:{0}",
                executor_manifest.limits.address_space_bytes
            )),
            OsString::from(format!(
                "--nproc={0}:{0}",
                executor_manifest.limits.process_count
            )),
            OsString::from(format!(
                "--fsize={0}:{0}",
                executor_manifest.limits.output_bytes
            )),
            OsString::from("--"),
            OsString::from(GUEST_WORKER_PATH_V1),
        ]);
        Ok(LawLabSandboxCommandSpecV1 {
            program: self.config.bwrap_path.clone(),
            args,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LawLabSandboxCommandSpecV1 {
    program: PathBuf,
    args: Vec<OsString>,
}

struct WorkspaceGuardV1 {
    root: PathBuf,
    work_root: PathBuf,
    workspace_instance_sha256: String,
    cleaned: bool,
}

impl WorkspaceGuardV1 {
    fn create(
        workspace_store_root: &Path,
        request_root_sha256: &str,
    ) -> Result<Self, LawLabSandboxErrorV1> {
        let store = workspace_store_root
            .canonicalize()
            .map_err(|_| LawLabSandboxErrorV1::Io)?;
        for _ in 0..WORKSPACE_CREATE_ATTEMPTS_V1 {
            let sequence = WORKSPACE_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| LawLabSandboxErrorV1::Io)?
                .as_nanos();
            let workspace_instance_sha256 = canonical_json_sha256(&(
                "nando.law-lab-sandbox-workspace.v1",
                request_root_sha256,
                std::process::id(),
                timestamp,
                sequence,
            ))
            .map_err(|_| LawLabSandboxErrorV1::Serialization)?;
            let root = store.join(&workspace_instance_sha256);
            match fs::create_dir(&root) {
                Ok(()) => {
                    let work_root = root.join("work");
                    let setup = (|| {
                        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                            .map_err(|_| LawLabSandboxErrorV1::Io)?;
                        fs::create_dir(&work_root).map_err(|_| LawLabSandboxErrorV1::Io)?;
                        fs::set_permissions(&work_root, fs::Permissions::from_mode(0o700))
                            .map_err(|_| LawLabSandboxErrorV1::Io)
                    })();
                    if let Err(error) = setup {
                        if fs::remove_dir_all(&root).is_err() || root.try_exists().unwrap_or(true) {
                            return Err(LawLabSandboxErrorV1::CleanupVerificationFailed);
                        }
                        return Err(error);
                    }
                    return Ok(Self {
                        root,
                        work_root,
                        workspace_instance_sha256,
                        cleaned: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(LawLabSandboxErrorV1::Io),
            }
        }
        Err(LawLabSandboxErrorV1::Io)
    }

    fn work_root(&self) -> &Path {
        &self.work_root
    }

    fn cleanup(&mut self) -> Result<LawLabSandboxCleanupProofV1, LawLabSandboxErrorV1> {
        fs::remove_dir_all(&self.root).map_err(|_| LawLabSandboxErrorV1::Io)?;
        if self
            .root
            .try_exists()
            .map_err(|_| LawLabSandboxErrorV1::Io)?
        {
            return Err(LawLabSandboxErrorV1::CleanupVerificationFailed);
        }
        self.cleaned = true;
        LawLabSandboxCleanupProofV1::seal(self.workspace_instance_sha256.clone())
    }
}

impl Drop for WorkspaceGuardV1 {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn run_command_v1(
    spec: &LawLabSandboxCommandSpecV1,
    input: &[u8],
    deadline: Duration,
    maximum_stdout_bytes: usize,
) -> Result<Vec<u8>, LawLabSandboxErrorV1> {
    let mut child = Command::new(&spec.program)
        .args(&spec.args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| LawLabSandboxErrorV1::Io)?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or(LawLabSandboxErrorV1::WorkerProtocolFailed)?;
    stdin
        .write_all(input)
        .map_err(|_| LawLabSandboxErrorV1::Io)?;
    drop(stdin);
    let stdout = child
        .stdout
        .take()
        .ok_or(LawLabSandboxErrorV1::WorkerProtocolFailed)?;
    let stderr = child
        .stderr
        .take()
        .ok_or(LawLabSandboxErrorV1::WorkerProtocolFailed)?;
    let stdout_reader = thread::spawn(move || read_process_pipe_v1(stdout, maximum_stdout_bytes));
    let stderr_reader = thread::spawn(move || read_process_pipe_v1(stderr, STDERR_LIMIT_V1));

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| LawLabSandboxErrorV1::Io)? {
            break status;
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(LawLabSandboxErrorV1::TimedOut);
        }
        thread::sleep(Duration::from_millis(2));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| LawLabSandboxErrorV1::WorkerProtocolFailed)??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| LawLabSandboxErrorV1::WorkerProtocolFailed)??;
    if !status.success() && std::env::var_os("NANDO_LAW_LAB_DEBUG_STDERR").is_some() {
        eprintln!("{}", String::from_utf8_lossy(&stderr));
    }
    validate_exit_status_v1(status)?;
    Ok(stdout)
}

fn read_process_pipe_v1(
    mut pipe: impl Read,
    maximum_bytes: usize,
) -> Result<Vec<u8>, LawLabSandboxErrorV1> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 16 * 1024];
    let mut exceeded = false;
    loop {
        let read = pipe
            .read(&mut buffer)
            .map_err(|_| LawLabSandboxErrorV1::Io)?;
        if read == 0 {
            break;
        }
        if output.len().saturating_add(read) <= maximum_bytes {
            output.extend_from_slice(&buffer[..read]);
        } else {
            exceeded = true;
        }
    }
    if exceeded {
        return Err(LawLabSandboxErrorV1::WorkerOutputTooLarge);
    }
    Ok(output)
}

fn validate_exit_status_v1(status: ExitStatus) -> Result<(), LawLabSandboxErrorV1> {
    if status.success() {
        Ok(())
    } else {
        Err(LawLabSandboxErrorV1::ProcessFailed)
    }
}

fn validate_operations_against_source_v1(
    request: &LawLabSandboxRequestV1,
    source: &LawLabTreeManifestV1,
) -> Result<(), LawLabSandboxErrorV1> {
    let mut maximum_written = 0_u64;
    for operation in &request.operations {
        match operation {
            LawLabSandboxOperationV1::CopySourceFile {
                source_path,
                work_path,
            } => {
                let entry = source
                    .entry(source_path)
                    .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
                    .ok_or(LawLabSandboxErrorV1::InvalidTree)?;
                if source.entry(work_path).is_some() || parent_is_file_v1(source, work_path) {
                    return Err(LawLabSandboxErrorV1::OperationConflict);
                }
                maximum_written = maximum_written
                    .checked_add(entry.byte_length)
                    .ok_or(LawLabSandboxErrorV1::TreeBudgetExceeded)?;
            }
            LawLabSandboxOperationV1::RemoveWorkPath { work_path } => {
                if source.entry(work_path).is_none() {
                    return Err(LawLabSandboxErrorV1::InvalidTree);
                }
            }
            LawLabSandboxOperationV1::CanonicalizeJsonFile { work_path } => {
                let entry = source
                    .entry(work_path)
                    .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
                    .ok_or(LawLabSandboxErrorV1::InvalidTree)?;
                maximum_written = maximum_written
                    .checked_add(entry.byte_length)
                    .ok_or(LawLabSandboxErrorV1::TreeBudgetExceeded)?;
            }
        }
    }
    if maximum_written > LAW_LAB_MAX_OUTPUT_BYTES_V1 {
        return Err(LawLabSandboxErrorV1::TreeBudgetExceeded);
    }
    Ok(())
}

fn verify_operations_independently_v1(
    request: &LawLabSandboxRequestV1,
    source_root: &Path,
    work_root: &Path,
    source_manifest: &LawLabTreeManifestV1,
    post_manifest: &LawLabTreeManifestV1,
    results: &[LawLabSandboxOperationResultV1],
) -> Result<(), LawLabSandboxErrorV1> {
    if results.len() != request.operations.len() {
        return Err(LawLabSandboxErrorV1::IndependentVerificationFailed);
    }
    let mut total_written = 0_u64;
    for (ordinal, (operation, result)) in request.operations.iter().zip(results).enumerate() {
        let operation_root_sha256 =
            canonical_json_sha256(operation).map_err(|_| LawLabSandboxErrorV1::Serialization)?;
        if result.ordinal != ordinal as u64 || result.operation_root_sha256 != operation_root_sha256
        {
            return Err(LawLabSandboxErrorV1::IndependentVerificationFailed);
        }
        let expected_effect = match operation {
            LawLabSandboxOperationV1::CopySourceFile {
                source_path,
                work_path,
            } => {
                let source_entry = source_manifest
                    .entry(source_path)
                    .ok_or(LawLabSandboxErrorV1::IndependentVerificationFailed)?;
                let post_entry = post_manifest
                    .entry(work_path)
                    .ok_or(LawLabSandboxErrorV1::IndependentVerificationFailed)?;
                if source_entry.kind != LawLabTreeEntryKindV1::File
                    || post_entry.kind != LawLabTreeEntryKindV1::File
                    || source_entry.byte_length != post_entry.byte_length
                    || source_entry.content_sha256 != post_entry.content_sha256
                    || source_entry.executable != post_entry.executable
                    || result.bytes_written != source_entry.byte_length
                    || law_lab_sha256_file_v1(&source_root.join(source_path))?
                        != law_lab_sha256_file_v1(&work_root.join(work_path))?
                {
                    return Err(LawLabSandboxErrorV1::IndependentVerificationFailed);
                }
                canonical_json_sha256(&(
                    "nando.law-lab-sandbox-effect.v1",
                    ordinal as u64,
                    operation_root_sha256.as_str(),
                    work_path.as_str(),
                    post_entry,
                ))
            }
            LawLabSandboxOperationV1::RemoveWorkPath { work_path } => {
                if post_manifest.entries.iter().any(|entry| {
                    entry.relative_path == *work_path
                        || entry
                            .relative_path
                            .strip_prefix(work_path)
                            .is_some_and(|suffix| suffix.starts_with('/'))
                }) || work_root
                    .join(work_path)
                    .try_exists()
                    .map_err(|_| LawLabSandboxErrorV1::Io)?
                    || result.bytes_written != 0
                {
                    return Err(LawLabSandboxErrorV1::IndependentVerificationFailed);
                }
                canonical_json_sha256(&(
                    "nando.law-lab-sandbox-effect.v1",
                    ordinal as u64,
                    operation_root_sha256.as_str(),
                    work_path.as_str(),
                    "absent",
                ))
            }
            LawLabSandboxOperationV1::CanonicalizeJsonFile { work_path } => {
                let bytes =
                    fs::read(work_root.join(work_path)).map_err(|_| LawLabSandboxErrorV1::Io)?;
                let value: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|_| LawLabSandboxErrorV1::IndependentVerificationFailed)?;
                let canonical = canonical_json_bytes(&value)
                    .map_err(|_| LawLabSandboxErrorV1::IndependentVerificationFailed)?;
                let post_entry = post_manifest
                    .entry(work_path)
                    .ok_or(LawLabSandboxErrorV1::IndependentVerificationFailed)?;
                if bytes != canonical
                    || result.bytes_written != bytes.len() as u64
                    || post_entry.content_sha256.as_deref()
                        != Some(law_lab_sha256_file_v1(&work_root.join(work_path))?.as_str())
                {
                    return Err(LawLabSandboxErrorV1::IndependentVerificationFailed);
                }
                canonical_json_sha256(&(
                    "nando.law-lab-sandbox-effect.v1",
                    ordinal as u64,
                    operation_root_sha256.as_str(),
                    work_path.as_str(),
                    post_entry,
                ))
            }
        }
        .map_err(|_| LawLabSandboxErrorV1::Serialization)?;
        if result.effect_root_sha256 != expected_effect {
            return Err(LawLabSandboxErrorV1::IndependentVerificationFailed);
        }
        total_written = total_written
            .checked_add(result.bytes_written)
            .ok_or(LawLabSandboxErrorV1::TreeBudgetExceeded)?;
    }
    if total_written > LAW_LAB_MAX_OUTPUT_BYTES_V1 {
        return Err(LawLabSandboxErrorV1::TreeBudgetExceeded);
    }
    Ok(())
}

fn parent_is_file_v1(manifest: &LawLabTreeManifestV1, relative_path: &str) -> bool {
    let mut current = relative_path;
    while let Some((parent, _)) = current.rsplit_once('/') {
        if manifest
            .entry(parent)
            .is_some_and(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        {
            return true;
        }
        current = parent;
    }
    false
}

fn validate_store_root_v1(path: &Path) -> Result<PathBuf, LawLabSandboxErrorV1> {
    let metadata = fs::symlink_metadata(path).map_err(|_| LawLabSandboxErrorV1::Io)?;
    if !path.is_absolute()
        || !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.mode() & 0o022 != 0
    {
        return Err(LawLabSandboxErrorV1::ToolUntrusted);
    }
    path.canonicalize().map_err(|_| LawLabSandboxErrorV1::Io)
}

fn validate_source_snapshot_trust_v1(
    path: &Path,
    root_owned_required: bool,
) -> Result<(), LawLabSandboxErrorV1> {
    let mut pending = vec![path.to_path_buf()];
    while let Some(current) = pending.pop() {
        let metadata = fs::symlink_metadata(&current).map_err(|_| LawLabSandboxErrorV1::Io)?;
        if metadata.file_type().is_symlink()
            || metadata.mode() & 0o022 != 0
            || (root_owned_required && metadata.uid() != 0)
        {
            return Err(LawLabSandboxErrorV1::ToolUntrusted);
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(&current).map_err(|_| LawLabSandboxErrorV1::Io)? {
                pending.push(entry.map_err(|_| LawLabSandboxErrorV1::Io)?.path());
            }
        } else if !metadata.is_file() {
            return Err(LawLabSandboxErrorV1::InvalidTree);
        }
    }
    Ok(())
}

fn validate_root_owned_executable_v1(path: &Path) -> Result<(), LawLabSandboxErrorV1> {
    let metadata = validate_regular_executable_v1(path)?;
    if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
        return Err(LawLabSandboxErrorV1::ToolUntrusted);
    }
    Ok(())
}

fn validate_regular_executable_v1(path: &Path) -> Result<fs::Metadata, LawLabSandboxErrorV1> {
    let metadata = fs::symlink_metadata(path).map_err(|_| LawLabSandboxErrorV1::Io)?;
    if !path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.mode() & 0o111 == 0
    {
        return Err(LawLabSandboxErrorV1::ToolUntrusted);
    }
    Ok(metadata)
}

fn path_to_string_v1(path: &Path) -> Result<String, LawLabSandboxErrorV1> {
    path.to_str()
        .map(str::to_owned)
        .ok_or(LawLabSandboxErrorV1::ToolUntrusted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_has_namespaces_limits_direct_worker_and_no_shell() {
        let adapter =
            LawLabSandboxAdapterV1::new(LawLabSandboxConfigV1::generated_capability_self_test_v1(
                PathBuf::from("/proof/worker"),
                "1".repeat(64),
                PathBuf::from("/proof/sources"),
                PathBuf::from("/proof/workspaces"),
            ));
        let manifest =
            LawLabSandboxExecutorManifestV1::seal(LawLabSandboxExecutorManifestInputV1 {
                bwrap_host_path: BWRAP_PATH_V1.to_owned(),
                bwrap_sha256: "2".repeat(64),
                prlimit_host_path: PRLIMIT_PATH_V1.to_owned(),
                prlimit_sha256: "3".repeat(64),
                worker_host_path: "/proof/worker".to_owned(),
                worker_sha256: "1".repeat(64),
                source_store_host_path: "/proof/sources".to_owned(),
                workspace_store_host_path: "/proof/workspaces".to_owned(),
                root_owned_worker_required: false,
                root_owned_source_snapshot_required: false,
                content_addressed_worker_path_required: false,
                generated_capability_fixture_only: true,
                runtime_read_only_binds: vec![
                    "/lib".to_owned(),
                    "/lib64".to_owned(),
                    "/usr".to_owned(),
                ],
                wall_ms: LAW_LAB_MAX_PROBE_WALL_MS_V1,
            })
            .expect("manifest");
        let spec = adapter
            .command_spec_v1(
                &manifest,
                Path::new("/proof/sources/source"),
                Path::new("/proof/workspaces/work"),
            )
            .expect("command");
        let args = spec
            .args
            .iter()
            .map(|value| value.to_string_lossy())
            .collect::<Vec<_>>();
        assert_eq!(spec.program, Path::new(BWRAP_PATH_V1));
        for required in [
            "--unshare-all",
            "--die-with-parent",
            "--new-session",
            "--cap-drop",
            "--clearenv",
            "--proc",
            "--dev",
            "--tmpfs",
            PRLIMIT_PATH_V1,
            GUEST_WORKER_PATH_V1,
        ] {
            assert!(args.iter().any(|argument| argument == required));
        }
        assert!(!args.iter().any(|argument| {
            matches!(
                argument.as_ref(),
                "/bin/sh" | "/bin/bash" | "sh" | "bash" | "-c"
            )
        }));
    }
}
