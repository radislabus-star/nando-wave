use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, K2CompositionTreeManifestV1, composition_bytes_v1,
    composition_decode_v1, composition_root_v1, composition_sha256_file_v1,
    valid_composition_path_v1,
};
use super::model::{
    K2_INQUIRY_MAX_PROTOCOL_BYTES_V1, K2_INQUIRY_OBSERVATION_SCHEMA_V1,
    K2_INQUIRY_WORKER_OUTCOME_SCHEMA_V1, K2InquiryObservationReceiptV1, K2InquiryObserverRequestV1,
    K2InquiryWorkerOutcomeV1, K2InquiryWorkerRequestV1,
};

const BWRAP_PATH_V1: &str = "/usr/bin/bwrap";
const PRLIMIT_PATH_V1: &str = "/usr/bin/prlimit";
const GUEST_WORKER_PATH_V1: &str = "/nando/bin/inquiry-worker";
const GUEST_OBSERVER_PATH_V1: &str = "/nando/bin/inquiry-observer";
static WORKSPACE_SEQUENCE_V1: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2InquirySandboxExecutionV1 {
    pub worker_outcome: K2InquiryWorkerOutcomeV1,
    pub observation: K2InquiryObservationReceiptV1,
    pub source_integrity_preserved: bool,
    pub workspace_removed: bool,
    pub cleanup_root_sha256: String,
}

#[derive(Clone, Debug)]
pub struct K2InquirySandboxAdapterV1 {
    worker_path: PathBuf,
    worker_executable_sha256: String,
    observer_path: PathBuf,
    observer_executable_sha256: String,
    workspace_store_root: PathBuf,
}

impl K2InquirySandboxAdapterV1 {
    pub fn new(
        worker_path: PathBuf,
        worker_executable_sha256: String,
        observer_path: PathBuf,
        observer_executable_sha256: String,
        workspace_store_root: PathBuf,
    ) -> K2CompositionResultV1<Self> {
        if composition_sha256_file_v1(&worker_path)? != worker_executable_sha256
            || composition_sha256_file_v1(&observer_path)? != observer_executable_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_sandbox_executable_hash_mismatch",
            ));
        }
        fs::create_dir_all(&workspace_store_root)
            .map_err(|_| K2CompositionErrorV1::Io("create_inquiry_workspace_store"))?;
        Ok(Self {
            worker_path,
            worker_executable_sha256,
            observer_path,
            observer_executable_sha256,
            workspace_store_root,
        })
    }

    pub fn execute(
        &self,
        worker_request: &K2InquiryWorkerRequestV1,
        observer_request: &K2InquiryObserverRequestV1,
        initial_files: &BTreeMap<String, Vec<u8>>,
    ) -> K2CompositionResultV1<K2InquirySandboxExecutionV1> {
        worker_request.validate()?;
        observer_request.validate()?;
        if worker_request.worker_executable_sha256 != self.worker_executable_sha256
            || observer_request.observer_executable_sha256 != self.observer_executable_sha256
            || worker_request.experiment_id_sha256 != observer_request.experiment_id_sha256
            || worker_request.selected_probe_root_sha256
                != observer_request.selected_probe_root_sha256
            || K2CompositionTreeManifestV1::from_files(initial_files)?
                != worker_request.initial_manifest
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_sandbox_binding_invalid",
            ));
        }

        let sequence = WORKSPACE_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed);
        let workspace_root = self.workspace_store_root.join(format!(
            "{}-{sequence}",
            &worker_request.request_root_sha256[..16]
        ));
        if workspace_root.exists() {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_workspace_identity_exists",
            ));
        }
        let source_root = workspace_root.join("source");
        let work_root = workspace_root.join("work");
        fs::create_dir_all(&source_root)
            .and_then(|_| fs::create_dir_all(&work_root))
            .map_err(|_| K2CompositionErrorV1::Io("create_inquiry_workspace"))?;
        materialize_files_v1(&source_root, initial_files)?;
        materialize_files_v1(&work_root, initial_files)?;
        let source_before = K2CompositionTreeManifestV1::scan(&source_root)?;

        let worker_bytes = run_isolated_v1(
            &self.worker_path,
            GUEST_WORKER_PATH_V1,
            &source_root,
            &work_root,
            true,
            &composition_bytes_v1(worker_request)?,
        )?;
        let worker_outcome: K2InquiryWorkerOutcomeV1 = composition_decode_v1(&worker_bytes)?;
        validate_worker_outcome_v1(worker_request, &worker_outcome)?;

        let observer_bytes = run_isolated_v1(
            &self.observer_path,
            GUEST_OBSERVER_PATH_V1,
            &source_root,
            &work_root,
            false,
            &composition_bytes_v1(observer_request)?,
        )?;
        let observation: K2InquiryObservationReceiptV1 = composition_decode_v1(&observer_bytes)?;
        validate_observation_v1(observer_request, &observation)?;

        let source_after = K2CompositionTreeManifestV1::scan(&source_root)?;
        if source_before != source_after
            || worker_outcome.post_manifest != observation.post_manifest
            || worker_outcome.selected_probe_root_sha256 != observation.selected_probe_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_sandbox_observation_parity_failure",
            ));
        }

        fs::remove_dir_all(&workspace_root)
            .map_err(|_| K2CompositionErrorV1::Io("remove_inquiry_workspace"))?;
        let workspace_removed = !workspace_root.exists();
        let cleanup_root_sha256 = composition_root_v1(&(
            "nando.k2-inquiry-sandbox-cleanup.v1",
            &worker_request.request_root_sha256,
            &observer_request.request_root_sha256,
            sequence,
            workspace_removed,
        ))?;
        Ok(K2InquirySandboxExecutionV1 {
            worker_outcome,
            observation,
            source_integrity_preserved: true,
            workspace_removed,
            cleanup_root_sha256,
        })
    }
}

pub fn run_inquiry_worker_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_INQUIRY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_inquiry_worker_stdin"))?;
    let request: K2InquiryWorkerRequestV1 = composition_decode_v1(&input)?;
    request.validate()?;
    let executable =
        std::env::current_exe().map_err(|_| K2CompositionErrorV1::Io("resolve_inquiry_worker"))?;
    if composition_sha256_file_v1(&executable)? != request.worker_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_worker_executable_mismatch",
        ));
    }
    let work_root = Path::new("/work");
    let pre_manifest = K2CompositionTreeManifestV1::scan(work_root)?;
    if pre_manifest != request.initial_manifest {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_worker_pre_manifest_mismatch",
        ));
    }
    let (transition_applied, transition_reason) =
        apply_worker_effect_v1(work_root, &request.resolved_effect);
    let post_manifest = K2CompositionTreeManifestV1::scan(work_root)?;
    let mut outcome = K2InquiryWorkerOutcomeV1 {
        schema: K2_INQUIRY_WORKER_OUTCOME_SCHEMA_V1.to_owned(),
        request_root_sha256: request.request_root_sha256,
        worker_executable_sha256: request.worker_executable_sha256,
        selected_probe_root_sha256: request.selected_probe_root_sha256,
        pre_manifest,
        post_manifest,
        transition_applied,
        transition_reason: transition_reason.to_owned(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        outcome_root_sha256: String::new(),
    };
    outcome.reseal()?;
    std::io::stdout()
        .write_all(&composition_bytes_v1(&outcome)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_inquiry_worker_stdout"))
}

pub fn run_inquiry_observer_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_INQUIRY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_inquiry_observer_stdin"))?;
    let request: K2InquiryObserverRequestV1 = composition_decode_v1(&input)?;
    request.validate()?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_inquiry_observer"))?;
    if composition_sha256_file_v1(&executable)? != request.observer_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_observer_executable_mismatch",
        ));
    }
    let post_manifest = K2CompositionTreeManifestV1::scan(Path::new("/work"))?;
    let mut receipt = K2InquiryObservationReceiptV1 {
        schema: K2_INQUIRY_OBSERVATION_SCHEMA_V1.to_owned(),
        observer_request_root_sha256: request.request_root_sha256,
        observer_executable_sha256: request.observer_executable_sha256,
        selected_probe_root_sha256: request.selected_probe_root_sha256,
        post_manifest,
        observable_outcome_root_sha256: String::new(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    std::io::stdout()
        .write_all(&composition_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_inquiry_observer_stdout"))
}

fn apply_worker_effect_v1(
    work_root: &Path,
    effect: &K2CompositionLearnedEffectV1,
) -> (bool, &'static str) {
    match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => {
            if !valid_composition_path_v1(source_path) || !valid_composition_path_v1(target_path) {
                return (false, "path_invalid");
            }
            let source = work_root.join(source_path);
            if !source.is_file() {
                return (false, "copy_source_missing");
            }
            let target = work_root.join(target_path);
            if target
                .parent()
                .is_none_or(|parent| fs::create_dir_all(parent).is_err())
            {
                return (false, "create_target_parent_failed");
            }
            match fs::copy(source, target) {
                Ok(_) => (true, "applied"),
                Err(_) => (false, "copy_failed"),
            }
        }
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if !valid_composition_path_v1(path) {
                return (false, "path_invalid");
            }
            let target = work_root.join(path);
            if !target.is_file() {
                return (false, "remove_path_missing");
            }
            match fs::remove_file(target) {
                Ok(()) => (true, "applied"),
                Err(_) => (false, "remove_failed"),
            }
        }
    }
}

fn validate_worker_outcome_v1(
    request: &K2InquiryWorkerRequestV1,
    outcome: &K2InquiryWorkerOutcomeV1,
) -> K2CompositionResultV1<()> {
    outcome.pre_manifest.validate()?;
    outcome.post_manifest.validate()?;
    outcome.authority.validate()?;
    let mut expected = outcome.clone();
    expected.reseal()?;
    if outcome.schema != K2_INQUIRY_WORKER_OUTCOME_SCHEMA_V1
        || outcome.request_root_sha256 != request.request_root_sha256
        || outcome.worker_executable_sha256 != request.worker_executable_sha256
        || outcome.selected_probe_root_sha256 != request.selected_probe_root_sha256
        || outcome.pre_manifest != request.initial_manifest
        || expected != *outcome
    {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_worker_outcome_invalid",
        ));
    }
    Ok(())
}

fn validate_observation_v1(
    request: &K2InquiryObserverRequestV1,
    observation: &K2InquiryObservationReceiptV1,
) -> K2CompositionResultV1<()> {
    observation.post_manifest.validate()?;
    observation.authority.validate()?;
    let mut expected = observation.clone();
    expected.reseal()?;
    if observation.schema != K2_INQUIRY_OBSERVATION_SCHEMA_V1
        || observation.observer_request_root_sha256 != request.request_root_sha256
        || observation.observer_executable_sha256 != request.observer_executable_sha256
        || observation.selected_probe_root_sha256 != request.selected_probe_root_sha256
        || expected != *observation
    {
        return Err(K2CompositionErrorV1::Invalid("inquiry_observation_invalid"));
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
                "inquiry_materialize_path_invalid",
            ));
        }
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| K2CompositionErrorV1::Io("create_inquiry_parent"))?;
        }
        fs::write(target, bytes).map_err(|_| K2CompositionErrorV1::Io("write_inquiry_fixture"))?;
    }
    Ok(())
}

fn run_isolated_v1(
    executable: &Path,
    guest_executable: &str,
    source_root: &Path,
    work_root: &Path,
    writable_work: bool,
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
    let work_bind = if writable_work { "--bind" } else { "--ro-bind" };
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
        executable.as_os_str().to_owned(),
        OsString::from(guest_executable),
        OsString::from("--ro-bind"),
        source_root.as_os_str().to_owned(),
        OsString::from("/source"),
        OsString::from(work_bind),
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
        OsString::from(guest_executable),
    ];
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|_| K2CompositionErrorV1::Io("spawn_inquiry_process"))?;
    child
        .stdin
        .take()
        .ok_or(K2CompositionErrorV1::Io("inquiry_process_stdin"))?
        .write_all(input)
        .map_err(|_| K2CompositionErrorV1::Io("write_inquiry_process_stdin"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or(K2CompositionErrorV1::Io("inquiry_process_stdout"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or(K2CompositionErrorV1::Io("inquiry_process_stderr"))?;
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
            .map_err(|_| K2CompositionErrorV1::Io("poll_inquiry_process"))?
        {
            break status;
        }
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            return Err(K2CompositionErrorV1::Invalid("inquiry_process_timeout"));
        }
        thread::sleep(Duration::from_millis(2));
    };
    let stdout = stdout_thread
        .join()
        .map_err(|_| K2CompositionErrorV1::Io("join_inquiry_stdout"))?;
    let stderr = stderr_thread
        .join()
        .map_err(|_| K2CompositionErrorV1::Io("join_inquiry_stderr"))?;
    if !status.success() {
        let _ = String::from_utf8_lossy(&stderr);
        return Err(K2CompositionErrorV1::Process(
            "inquiry_isolated_process_failed",
        ));
    }
    Ok(stdout)
}
