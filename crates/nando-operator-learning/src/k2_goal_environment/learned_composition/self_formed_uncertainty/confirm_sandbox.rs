use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_file_v1,
    require_composition_root_v1,
};
use super::K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1;

const BWRAP_PATH_V1: &str = "/usr/bin/bwrap";
const PRLIMIT_PATH_V1: &str = "/usr/bin/prlimit";
const PRIMARY_GUEST_EXECUTABLE_V1: &str = "/nando/bin/process";
const MAX_STDERR_BYTES_V1: usize = 65_536;
const ADDRESS_SPACE_BYTES_V1: u64 = 536_870_912;
const MAX_PROCESSES_V1: u64 = 32;
const MAX_FILE_BYTES_V1: u64 = 33_554_432;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum K2UncertaintyConfirmGuestExecutableV1 {
    Learner,
    Probe,
    Selector,
    Baseline,
    SelectionPreverifier,
    ClosurePlanner,
    ClosureVerifier,
    PrivateResolver,
    Safety,
    Worker,
    Observer,
    FinalVerifier,
    Oracle,
    R8BAggregateAuthorizer,
}

impl K2UncertaintyConfirmGuestExecutableV1 {
    pub const fn path(self) -> &'static str {
        match self {
            Self::Learner => "/nando/bin/learner",
            Self::Probe => "/nando/bin/probe",
            Self::Selector => "/nando/bin/selector",
            Self::Baseline => "/nando/bin/baseline",
            Self::SelectionPreverifier => "/nando/bin/selection-preverifier",
            Self::ClosurePlanner => "/nando/bin/closure-planner",
            Self::ClosureVerifier => "/nando/bin/closure-verifier",
            Self::PrivateResolver => "/nando/bin/private-resolver",
            Self::Safety => "/nando/bin/safety",
            Self::Worker => "/nando/bin/worker",
            Self::Observer => "/nando/bin/observer",
            Self::FinalVerifier => "/nando/bin/final-verifier",
            Self::Oracle => "/nando/bin/oracle",
            Self::R8BAggregateAuthorizer => "/nando/bin/r8b-authorizer",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyConfirmMountTargetV1 {
    Output,
    ResolverTable,
    Workspace,
    Source,
    Evidence,
    FinalTruth,
    OracleEvidence,
    OraclePrivateTruth,
    AggregateEvidence,
}

impl K2UncertaintyConfirmMountTargetV1 {
    const fn path(self) -> &'static str {
        match self {
            Self::Output => "/out",
            Self::ResolverTable => "/private/resolver.json",
            Self::Workspace => "/work",
            Self::Source => "/source",
            Self::Evidence => "/evidence",
            Self::FinalTruth => "/private/final-truth.json",
            Self::OracleEvidence => "/oracle",
            Self::OraclePrivateTruth => "/oracle/private-truth.json",
            Self::AggregateEvidence => "/evidence",
        }
    }

    const fn expects_directory(self) -> bool {
        matches!(
            self,
            Self::Output
                | Self::Workspace
                | Self::Source
                | Self::Evidence
                | Self::OracleEvidence
                | Self::AggregateEvidence
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct K2UncertaintyConfirmDataMountV1<'a> {
    pub host_path: &'a Path,
    pub target: K2UncertaintyConfirmMountTargetV1,
    pub writable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintySandboxProcessOutcomeV1 {
    pub normal_exit: bool,
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyR7kCleanupGuestV1 {
    Authorizer,
    Owner,
    Verifier,
    ResultPublisher,
}

impl K2UncertaintyR7kCleanupGuestV1 {
    const fn governed_access(self) -> Option<bool> {
        match self {
            Self::Authorizer | Self::ResultPublisher => None,
            Self::Owner => Some(true),
            Self::Verifier => Some(false),
        }
    }
}

pub fn run_self_formed_r7k_cleanup_sandbox_v1(
    role: K2UncertaintyR7kCleanupGuestV1,
    executable: &Path,
    expected_executable_sha256: &str,
    governed_root: Option<&Path>,
    control_root: &Path,
    input: &[u8],
    cpu_seconds: u64,
) -> K2CompositionResultV1<Vec<u8>> {
    successful_stdout_v1(run_self_formed_r7k_cleanup_sandbox_measured_v1(
        role,
        executable,
        expected_executable_sha256,
        governed_root,
        control_root,
        input,
        cpu_seconds,
    )?)
}

pub fn run_self_formed_r7k_cleanup_sandbox_measured_v1(
    role: K2UncertaintyR7kCleanupGuestV1,
    executable: &Path,
    expected_executable_sha256: &str,
    governed_root: Option<&Path>,
    control_root: &Path,
    input: &[u8],
    cpu_seconds: u64,
) -> K2CompositionResultV1<K2UncertaintySandboxProcessOutcomeV1> {
    require_composition_root_v1(expected_executable_sha256)?;
    if input.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 || !(1..=600).contains(&cpu_seconds) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_cleanup_sandbox_budget_invalid",
        ));
    }
    validate_executable_v1(executable, expected_executable_sha256)?;
    validate_r7k_cleanup_root_v1(control_root)?;
    if role.governed_access().is_some() != governed_root.is_some() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_cleanup_sandbox_mount_matrix_invalid",
        ));
    }
    if let Some(governed_root) = governed_root {
        validate_r7k_cleanup_root_v1(governed_root)?;
        let governed = fs::canonicalize(governed_root)
            .map_err(|_| K2CompositionErrorV1::Io("canonicalize_r7k_governed_root"))?;
        let control = fs::canonicalize(control_root)
            .map_err(|_| K2CompositionErrorV1::Io("canonicalize_r7k_control_root"))?;
        if governed == control || governed.starts_with(&control) || control.starts_with(&governed) {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_r7k_cleanup_sandbox_roots_not_siblings",
            ));
        }
    }

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
    command.args([
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--dir",
        "/nando",
        "--dir",
        "/nando/bin",
        "--dir",
        "/private",
        "--ro-bind",
    ]);
    command.arg(executable).arg(PRIMARY_GUEST_EXECUTABLE_V1);
    if let (Some(governed_root), Some(writable)) = (governed_root, role.governed_access()) {
        command
            .args(["--dir", "/governed"])
            .arg(if writable { "--bind" } else { "--ro-bind" })
            .arg(governed_root)
            .arg("/governed");
    }
    command
        .args(["--dir", "/control", "--bind"])
        .arg(control_root)
        .arg("/control")
        .args(["--setenv", "HOME", "/tmp", "--setenv", "LANG", "C"])
        .args(["--", PRLIMIT_PATH_V1])
        .arg(format!("--cpu={cpu_seconds}:{cpu_seconds}"))
        .arg(format!(
            "--as={ADDRESS_SPACE_BYTES_V1}:{ADDRESS_SPACE_BYTES_V1}"
        ))
        .arg(format!("--nproc={MAX_PROCESSES_V1}:{MAX_PROCESSES_V1}"))
        .arg(format!("--fsize={MAX_FILE_BYTES_V1}:{MAX_FILE_BYTES_V1}"))
        .args(["--", PRIMARY_GUEST_EXECUTABLE_V1])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    execute_child_measured_v1(command, input, Duration::from_secs(cpu_seconds + 10))
}

fn validate_r7k_cleanup_root_v1(root: &Path) -> K2CompositionResultV1<()> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r7k_cleanup_root"))?;
    if !root.is_absolute()
        || metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o7777 != 0o700
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_cleanup_sandbox_root_invalid",
        ));
    }
    Ok(())
}

pub fn run_self_formed_r7k_control_sandbox_v1(
    executable: &Path,
    expected_executable_sha256: &str,
    scratch_root: &Path,
    input: &[u8],
    argv: &[String],
    environment: &[(String, String)],
    cpu_seconds: u64,
) -> K2CompositionResultV1<K2UncertaintySandboxProcessOutcomeV1> {
    require_composition_root_v1(expected_executable_sha256)?;
    if input.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 || !(1..=600).contains(&cpu_seconds) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_control_sandbox_budget_invalid",
        ));
    }
    validate_executable_v1(executable, expected_executable_sha256)?;
    let scratch_metadata = fs::symlink_metadata(scratch_root)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r7k_control_scratch"))?;
    if !scratch_root.is_absolute()
        || scratch_metadata.file_type().is_symlink()
        || !scratch_metadata.is_dir()
        || environment
            .iter()
            .any(|(key, _)| key.is_empty() || key.contains('='))
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_control_sandbox_input_invalid",
        ));
    }

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
    command.args([
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--dir",
        "/nando",
        "--dir",
        "/nando/bin",
        "--dir",
        "/private",
        "--dir",
        "/scratch",
        "--ro-bind",
    ]);
    command
        .arg(executable)
        .arg(PRIMARY_GUEST_EXECUTABLE_V1)
        .args(["--bind"])
        .arg(scratch_root)
        .arg("/scratch")
        .args(["--setenv", "HOME", "/tmp", "--setenv", "LANG", "C"]);
    for (key, value) in environment {
        command.args(["--setenv", key, value]);
    }
    command
        .args(["--", PRLIMIT_PATH_V1])
        .arg(format!("--cpu={cpu_seconds}:{cpu_seconds}"))
        .arg(format!(
            "--as={ADDRESS_SPACE_BYTES_V1}:{ADDRESS_SPACE_BYTES_V1}"
        ))
        .arg(format!("--nproc={MAX_PROCESSES_V1}:{MAX_PROCESSES_V1}"))
        .arg(format!("--fsize={MAX_FILE_BYTES_V1}:{MAX_FILE_BYTES_V1}"))
        .args(["--", PRIMARY_GUEST_EXECUTABLE_V1])
        .args(argv)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    execute_child_measured_v1(command, input, Duration::from_secs(cpu_seconds + 10))
}

pub fn run_self_formed_confirm_sandbox_v1(
    role: K2UncertaintyConfirmGuestExecutableV1,
    executable: &Path,
    expected_executable_sha256: &str,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    input: &[u8],
    cpu_seconds: u64,
) -> K2CompositionResultV1<Vec<u8>> {
    successful_stdout_v1(run_self_formed_confirm_sandbox_measured_v1(
        role,
        executable,
        expected_executable_sha256,
        mounts,
        input,
        cpu_seconds,
    )?)
}

pub fn run_self_formed_confirm_sandbox_measured_v1(
    role: K2UncertaintyConfirmGuestExecutableV1,
    executable: &Path,
    expected_executable_sha256: &str,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    input: &[u8],
    cpu_seconds: u64,
) -> K2CompositionResultV1<K2UncertaintySandboxProcessOutcomeV1> {
    require_composition_root_v1(expected_executable_sha256)?;
    if input.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 || !(1..=600).contains(&cpu_seconds) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_sandbox_budget_invalid",
        ));
    }
    validate_executable_v1(executable, expected_executable_sha256)?;
    validate_data_mounts_v1(role, mounts)?;

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
    command.args([
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--dir",
        "/nando",
        "--dir",
        "/nando/bin",
        "--dir",
        "/private",
        "--ro-bind",
    ]);
    command.arg(executable).arg(PRIMARY_GUEST_EXECUTABLE_V1);
    for mount in mounts {
        if mount.target.expects_directory() {
            command.arg("--dir").arg(mount.target.path());
        }
        command
            .arg(if mount.writable {
                "--bind"
            } else {
                "--ro-bind"
            })
            .arg(mount.host_path)
            .arg(mount.target.path());
    }
    match role {
        K2UncertaintyConfirmGuestExecutableV1::Oracle => {
            command.args(["--chdir", "/oracle"]);
        }
        K2UncertaintyConfirmGuestExecutableV1::R8BAggregateAuthorizer => {
            command.args(["--chdir", "/evidence"]);
        }
        _ => {}
    }
    command
        .args(["--setenv", "HOME", "/tmp", "--setenv", "LANG", "C"])
        .args(["--", PRLIMIT_PATH_V1])
        .arg(format!("--cpu={cpu_seconds}:{cpu_seconds}"))
        .arg(format!(
            "--as={ADDRESS_SPACE_BYTES_V1}:{ADDRESS_SPACE_BYTES_V1}"
        ))
        .arg(format!("--nproc={MAX_PROCESSES_V1}:{MAX_PROCESSES_V1}"))
        .arg(format!("--fsize={MAX_FILE_BYTES_V1}:{MAX_FILE_BYTES_V1}"))
        .args(["--", PRIMARY_GUEST_EXECUTABLE_V1])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    execute_child_measured_v1(command, input, Duration::from_secs(cpu_seconds + 10))
}

fn validate_executable_v1(path: &Path, expected_sha256: &str) -> K2CompositionResultV1<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_confirm_sandbox_executable"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || !path.is_absolute()
        || composition_sha256_file_v1(path)? != expected_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_sandbox_executable_invalid",
        ));
    }
    Ok(())
}

fn validate_data_mounts_v1(
    role: K2UncertaintyConfirmGuestExecutableV1,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
) -> K2CompositionResultV1<()> {
    let mut targets = std::collections::BTreeSet::new();
    for mount in mounts {
        let link_metadata = fs::symlink_metadata(mount.host_path)
            .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_confirm_sandbox_mount"))?;
        let descriptor_overlay = matches!(
            mount.target,
            K2UncertaintyConfirmMountTargetV1::ResolverTable
                | K2UncertaintyConfirmMountTargetV1::FinalTruth
                | K2UncertaintyConfirmMountTargetV1::OraclePrivateTruth
        ) && inherited_descriptor_path_v1(mount.host_path);
        let metadata = if descriptor_overlay {
            fs::metadata(mount.host_path).map_err(|_| {
                K2CompositionErrorV1::Io("stat_self_formed_confirm_sandbox_descriptor_mount")
            })?
        } else {
            link_metadata.clone()
        };
        if (link_metadata.file_type().is_symlink() && !descriptor_overlay)
            || !mount.host_path.is_absolute()
            || metadata.is_dir() != mount.target.expects_directory()
            || !targets.insert(mount.target.path())
            || (mount.writable
                && !matches!(
                    mount.target,
                    K2UncertaintyConfirmMountTargetV1::Output
                        | K2UncertaintyConfirmMountTargetV1::Workspace
                ))
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_sandbox_mount_invalid",
            ));
        }
    }
    let expected: &[(K2UncertaintyConfirmMountTargetV1, bool)] = match role {
        K2UncertaintyConfirmGuestExecutableV1::Probe => {
            &[(K2UncertaintyConfirmMountTargetV1::Output, true)]
        }
        K2UncertaintyConfirmGuestExecutableV1::PrivateResolver => {
            &[(K2UncertaintyConfirmMountTargetV1::ResolverTable, false)]
        }
        K2UncertaintyConfirmGuestExecutableV1::Worker => &[
            (K2UncertaintyConfirmMountTargetV1::Source, false),
            (K2UncertaintyConfirmMountTargetV1::Workspace, true),
        ],
        K2UncertaintyConfirmGuestExecutableV1::Observer => &[
            (K2UncertaintyConfirmMountTargetV1::Source, false),
            (K2UncertaintyConfirmMountTargetV1::Workspace, false),
        ],
        K2UncertaintyConfirmGuestExecutableV1::FinalVerifier => &[
            (K2UncertaintyConfirmMountTargetV1::FinalTruth, false),
            (K2UncertaintyConfirmMountTargetV1::Evidence, false),
        ],
        K2UncertaintyConfirmGuestExecutableV1::Oracle => &[
            (K2UncertaintyConfirmMountTargetV1::OracleEvidence, false),
            (K2UncertaintyConfirmMountTargetV1::OraclePrivateTruth, false),
        ],
        K2UncertaintyConfirmGuestExecutableV1::R8BAggregateAuthorizer => {
            &[(K2UncertaintyConfirmMountTargetV1::AggregateEvidence, false)]
        }
        K2UncertaintyConfirmGuestExecutableV1::Learner
        | K2UncertaintyConfirmGuestExecutableV1::Selector
        | K2UncertaintyConfirmGuestExecutableV1::Baseline
        | K2UncertaintyConfirmGuestExecutableV1::SelectionPreverifier
        | K2UncertaintyConfirmGuestExecutableV1::ClosurePlanner
        | K2UncertaintyConfirmGuestExecutableV1::ClosureVerifier
        | K2UncertaintyConfirmGuestExecutableV1::Safety => &[],
    };
    if mounts.len() != expected.len()
        || !expected.iter().all(|expected_mount| {
            mounts
                .iter()
                .any(|mount| mount.target == expected_mount.0 && mount.writable == expected_mount.1)
        })
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_sandbox_role_mount_matrix_invalid",
        ));
    }
    Ok(())
}

fn inherited_descriptor_path_v1(path: &Path) -> bool {
    path.parent() == Some(Path::new("/proc/self/fd"))
        && path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
            })
}

fn successful_stdout_v1(
    outcome: K2UncertaintySandboxProcessOutcomeV1,
) -> K2CompositionResultV1<Vec<u8>> {
    if !outcome.normal_exit || outcome.exit_code != 0 {
        if std::env::var_os("NANDO_K2_CONFIRM_SANDBOX_DIAGNOSTICS").is_some() {
            eprintln!(
                "self_formed_confirm_sandbox_child_diagnostic:{}",
                String::from_utf8_lossy(&outcome.stderr)
            );
        }
        return Err(if outcome.stderr.is_empty() {
            K2CompositionErrorV1::Invalid("self_formed_confirm_sandbox_child_failed")
        } else {
            K2CompositionErrorV1::Invalid("self_formed_confirm_sandbox_child_reported_error")
        });
    }
    Ok(outcome.stdout)
}

fn execute_child_measured_v1(
    mut command: Command,
    input: &[u8],
    timeout: Duration,
) -> K2CompositionResultV1<K2UncertaintySandboxProcessOutcomeV1> {
    let mut child = command
        .spawn()
        .map_err(|_| K2CompositionErrorV1::Io("spawn_self_formed_confirm_sandbox"))?;
    child
        .stdin
        .take()
        .ok_or(K2CompositionErrorV1::Io(
            "open_self_formed_confirm_sandbox_stdin",
        ))?
        .write_all(input)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_confirm_sandbox_stdin"))?;
    let stdout = child.stdout.take().ok_or(K2CompositionErrorV1::Io(
        "open_self_formed_confirm_sandbox_stdout",
    ))?;
    let stderr = child.stderr.take().ok_or(K2CompositionErrorV1::Io(
        "open_self_formed_confirm_sandbox_stderr",
    ))?;
    let stdout_thread =
        thread::spawn(move || read_bounded_v1(stdout, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1));
    let stderr_thread = thread::spawn(move || read_bounded_v1(stderr, MAX_STDERR_BYTES_V1));
    let status = wait_child_v1(&mut child, timeout)?;
    let stdout = stdout_thread
        .join()
        .map_err(|_| K2CompositionErrorV1::Io("join_self_formed_confirm_sandbox_stdout"))??;
    let stderr = stderr_thread
        .join()
        .map_err(|_| K2CompositionErrorV1::Io("join_self_formed_confirm_sandbox_stderr"))??;
    Ok(K2UncertaintySandboxProcessOutcomeV1 {
        normal_exit: status.code().is_some(),
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        timed_out: false,
    })
}

fn wait_child_v1(
    child: &mut Child,
    timeout: Duration,
) -> K2CompositionResultV1<std::process::ExitStatus> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| K2CompositionErrorV1::Io("poll_self_formed_confirm_sandbox"))?
        {
            return Ok(status);
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_sandbox_timeout",
            ));
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn read_bounded_v1<R: Read>(mut reader: R, limit: usize) -> K2CompositionResultV1<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_confirm_sandbox_output"))?;
    if bytes.len() > limit {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_sandbox_output_too_large",
        ));
    }
    Ok(bytes)
}
