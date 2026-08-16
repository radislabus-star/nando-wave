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
        }
    }

    const fn expects_directory(self) -> bool {
        matches!(
            self,
            Self::Output | Self::Workspace | Self::Source | Self::Evidence
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct K2UncertaintyConfirmDataMountV1<'a> {
    pub host_path: &'a Path,
    pub target: K2UncertaintyConfirmMountTargetV1,
    pub writable: bool,
}

pub fn run_self_formed_confirm_sandbox_v1(
    role: K2UncertaintyConfirmGuestExecutableV1,
    executable: &Path,
    expected_executable_sha256: &str,
    mounts: &[K2UncertaintyConfirmDataMountV1<'_>],
    input: &[u8],
    cpu_seconds: u64,
) -> K2CompositionResultV1<Vec<u8>> {
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
    execute_child_v1(command, input, Duration::from_secs(cpu_seconds + 10))
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
        let metadata = fs::symlink_metadata(mount.host_path)
            .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_confirm_sandbox_mount"))?;
        if metadata.file_type().is_symlink()
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

fn execute_child_v1(
    mut command: Command,
    input: &[u8],
    timeout: Duration,
) -> K2CompositionResultV1<Vec<u8>> {
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
    if !status.success() {
        if std::env::var_os("NANDO_K2_CONFIRM_SANDBOX_DIAGNOSTICS").is_some() {
            eprintln!(
                "self_formed_confirm_sandbox_child_diagnostic:{}",
                String::from_utf8_lossy(&stderr)
            );
        }
        return Err(if stderr.is_empty() {
            K2CompositionErrorV1::Invalid("self_formed_confirm_sandbox_child_failed")
        } else {
            K2CompositionErrorV1::Invalid("self_formed_confirm_sandbox_child_reported_error")
        });
    }
    Ok(stdout)
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
