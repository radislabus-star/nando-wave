use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use nando_operator_learning::{
    K2UncertaintyConfirmAttemptEventKindV1, K2UncertaintyConfirmAttemptJournalV1,
    K2UncertaintyConfirmAttemptPhaseV1, K2UncertaintyConfirmOwnerRequestV1,
    K2UncertaintyDevelopmentRehearsalOwnerReceiptV1, K2UncertaintyGeneratorResponseV1,
    composition_root_v1, dispatch_self_formed_generator_once_v1,
    load_development_rehearsal_owner_metadata_v1, publish_development_rehearsal_split_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1,
};

#[rustfmt::skip]
#[path = "k2_self_formed_uncertainty_confirm_r8b_support/mod.rs"]
mod support;

use support::*;

const S02_SELECTOR_V2: &str = "r8b_v7_s02_restart_aggregate";

#[test]
#[ignore = "requires explicit R8B V7 execution authorization"]
fn r8b_v7_s02_restart_aggregate() {
    begin_suite_request_from_stdin_v2("S02_RESTART", S02_SELECTOR_V2);
    let strace = Path::new("/usr/bin/strace");
    let metadata = fs::metadata(strace).expect("pinned strace dependency");
    assert!(metadata.is_file() && metadata.permissions().mode() & 0o111 != 0);
    let strace_sha =
        nando_operator_learning::composition_sha256_file_v1(strace).expect("strace SHA-256");
    p01_incomplete_attempt_initialization_is_indeterminate_without_dispatch();
    p02_artifacts_frozen_performs_one_first_dispatch();
    p03_dispatched_without_split_never_redispatches();
    p04_complete_split_appends_cases_and_publishes_owner();
    p05_cases_generated_without_owner_publishes_exact_owner();
    p06_durable_owner_replays_stdout_without_mutation();
    p07_ptrace_stopped_real_owner_excludes_real_contender();
    publish_suite_measurements_v2(vec![SuiteMeasurementV2 {
        relative_path: "suites/s02/process-restart.json",
        kind: nando_operator_learning::K2UncertaintyR8BEvidenceKindV2::ProcessRestart,
        source_roots_sha256: vec![strace_sha],
        observed: 7,
        metrics: [
            ("strace_byte_len".to_owned(), metadata.len()),
            (
                "strace_unix_mode".to_owned(),
                (metadata.permissions().mode() & 0o7777) as u64,
            ),
        ]
        .into_iter()
        .collect(),
    }]);
}

#[test]
fn p01_incomplete_attempt_initialization_is_indeterminate_without_dispatch() {
    let case = RestartCaseV1::new("p01", "attempt");
    fs::create_dir(case.attempt_root()).expect("create incomplete attempt root");
    fs::set_permissions(
        case.attempt_root(),
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .expect("chmod incomplete attempt root");
    let before = tree_snapshot_v1(&case.lab);
    let output = case.run_owner();
    assert!(!output.status.success());
    assert_eq!(tree_snapshot_v1(&case.lab), before);
    assert_eq!(case.dispatch_count(), 0);
}

#[test]
fn p02_artifacts_frozen_performs_one_first_dispatch() {
    let case = RestartCaseV1::new("p02", "attempt");
    case.prepare_artifacts_frozen();
    let output = case.run_owner_success();
    let receipt = decode_owner_success_v1(&output);
    assert_eq!(receipt.generator_dispatch_count, 1);
    assert_eq!(case.dispatch_count(), 1);
    assert_eq!(
        case.phase(),
        K2UncertaintyConfirmAttemptPhaseV1::CasesGenerated
    );
}

#[test]
fn p03_dispatched_without_split_never_redispatches() {
    let case = RestartCaseV1::new("p03", "attempt");
    case.prepare_generator_dispatched();
    let output = case.run_owner();
    assert!(!output.status.success());
    assert_eq!(case.dispatch_count(), 1);
    assert_eq!(
        case.phase(),
        K2UncertaintyConfirmAttemptPhaseV1::GeneratorResultIndeterminate
    );
    let after = case.run_owner();
    assert!(!after.status.success());
    assert_eq!(case.dispatch_count(), 1);
}

#[test]
fn p04_complete_split_appends_cases_and_publishes_owner() {
    let case = RestartCaseV1::new("p04", "attempt");
    case.prepare_complete_split(false);
    let output = case.run_owner_success();
    let receipt = decode_owner_success_v1(&output);
    assert_eq!(receipt.generator_dispatch_count, 1);
    assert_eq!(case.dispatch_count(), 1);
    assert_eq!(
        case.phase(),
        K2UncertaintyConfirmAttemptPhaseV1::CasesGenerated
    );
}

#[test]
fn p05_cases_generated_without_owner_publishes_exact_owner() {
    let case = RestartCaseV1::new("p05", "attempt");
    case.prepare_complete_split(true);
    assert!(
        !case
            .attempt_root()
            .join("development-owner-receipt.json")
            .exists()
    );
    let output = case.run_owner_success();
    let receipt = decode_owner_success_v1(&output);
    let metadata = load_development_rehearsal_owner_metadata_v1(case.attempt_root(), &case.request)
        .expect("P05 durable owner metadata");
    assert_eq!(metadata.owner, receipt);
    assert_eq!(case.dispatch_count(), 1);
}

#[test]
fn p06_durable_owner_replays_stdout_without_mutation() {
    let case = RestartCaseV1::new("p06", "attempt");
    let first = case.run_owner_success();
    let first_receipt = decode_owner_success_v1(&first);
    let before = tree_snapshot_v1(case.attempt_root());
    let second = case.run_owner_success();
    let second_receipt = decode_owner_success_v1(&second);
    assert_eq!(second.stdout, first.stdout);
    assert_eq!(second_receipt, first_receipt);
    assert_eq!(tree_snapshot_v1(case.attempt_root()), before);
    assert_eq!(case.dispatch_count(), 1);
}

#[test]
fn p07_ptrace_stopped_real_owner_excludes_real_contender() {
    let case = RestartCaseV1::new("p07", "attempt");
    let trace = case.environment.root.join("p07-strace.log");
    let (tracer, nested) = spawn_traced_owner_v1(&case, &trace);
    let owner_pid = wait_for_tracee_v1(tracer.id(), Duration::from_secs(10));
    let lab_inode = fs::metadata(&case.lab).expect("P07 lab metadata").ino();
    wait_for_stopped_lock_v1(owner_pid, lab_inode, Duration::from_secs(10));

    let before = tree_snapshot_v1(&case.lab);
    let contender = case.run_owner();
    assert!(!contender.status.success());
    assert!(String::from_utf8_lossy(&contender.stderr).contains("development_attempt_owner_busy"));
    assert_eq!(tree_snapshot_v1(&case.lab), before);

    rustix::process::kill_process(
        rustix::process::Pid::from_raw(owner_pid as i32).expect("positive traced owner PID"),
        rustix::process::Signal::CONT,
    )
    .expect("resume traced owner");
    let first = tracer.wait_with_output().expect("wait traced owner");
    finish_suite_nested_child_v2(nested, &first);
    let receipt = decode_owner_success_v1(&first);
    assert_eq!(receipt.generator_dispatch_count, 1);
    let trace_bytes = fs::read(&trace).expect("read P07 ptrace log");
    let trace_text = String::from_utf8_lossy(&trace_bytes);
    assert!(trace_text.contains("flock(") && trace_text.contains("= 0"));
    let generator_path = case.generator.to_string_lossy();
    assert_eq!(
        trace_text
            .lines()
            .filter(|line| line.contains("execve(") && line.contains(generator_path.as_ref()))
            .count(),
        1
    );
}

struct RestartCaseV1 {
    environment: TestEnvironmentV1,
    lab: PathBuf,
    attempt_root: PathBuf,
    owner: PathBuf,
    generator: PathBuf,
    request: K2UncertaintyConfirmOwnerRequestV1,
}

impl RestartCaseV1 {
    fn new(label: &str, attempt: &str) -> Self {
        let environment = TestEnvironmentV1::new(label);
        let lab = environment.private_child("lab");
        let owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-confirm-owner"));
        let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
        let request = development_owner_request_v1(&lab, attempt, &owner, &generator);
        let attempt_root = lab.join(attempt);
        Self {
            environment,
            lab,
            attempt_root,
            owner,
            generator,
            request,
        }
    }

    fn attempt_root(&self) -> &Path {
        &self.attempt_root
    }

    fn run_owner(&self) -> Output {
        run_process_v1(&self.owner, &self.request)
    }

    fn run_owner_success(&self) -> Output {
        run_process_recorded_v2(&self.owner, &self.request)
    }

    fn prepare_artifacts_frozen(&self) {
        let mut journal = K2UncertaintyConfirmAttemptJournalV1::create_exclusive(
            self.attempt_root(),
            self.request.descriptor.clone(),
        )
        .expect("create P02 journal");
        journal
            .append(
                K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
                self.request
                    .descriptor
                    .confirm_owner_executable_sha256
                    .clone(),
                self.request.request_root_sha256.clone(),
                frozen_artifacts_root_v1(&self.request),
            )
            .expect("append ArtifactsFrozen");
    }

    fn prepare_generator_dispatched(&self) {
        self.prepare_artifacts_frozen();
        let mut journal = K2UncertaintyConfirmAttemptJournalV1::open_existing(self.attempt_root())
            .expect("open P03 journal");
        let generator_request = self
            .request
            .development_generator_request
            .as_ref()
            .expect("Development generator request");
        journal
            .append(
                K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched,
                self.request
                    .descriptor
                    .confirm_owner_executable_sha256
                    .clone(),
                self.request.request_root_sha256.clone(),
                dispatch_root_v1(
                    &generator_request.request_root_sha256,
                    &self.request.descriptor.generator_executable_sha256,
                ),
            )
            .expect("append GeneratorDispatched");
    }

    fn prepare_complete_split(&self, append_cases: bool) {
        self.prepare_generator_dispatched();
        let generator_request = self
            .request
            .development_generator_request
            .as_ref()
            .expect("Development generator request");
        let mut input = uncertainty_bytes_v1(generator_request).expect("generator request bytes");
        let (response_bytes, pipe) = dispatch_self_formed_generator_once_v1(
            &self.generator,
            &self.request.descriptor.generator_executable_sha256,
            &generator_request.request_root_sha256,
            &mut input,
            None,
        )
        .expect("prepare complete generator split");
        let response: K2UncertaintyGeneratorResponseV1 =
            uncertainty_decode_v1(&response_bytes).expect("decode prepared generator response");
        let split = publish_development_rehearsal_split_v1(
            &self.attempt_root().join("generated"),
            &self.request,
            self.request
                .descriptor
                .confirm_owner_executable_sha256
                .clone(),
            &response,
            &response_bytes,
            pipe,
        )
        .expect("publish prepared complete split");
        if append_cases {
            let mut journal =
                K2UncertaintyConfirmAttemptJournalV1::open_existing(self.attempt_root())
                    .expect("open P05 journal");
            journal
                .append(
                    K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated,
                    self.request
                        .descriptor
                        .confirm_owner_executable_sha256
                        .clone(),
                    self.request.request_root_sha256.clone(),
                    split.split_receipt_root_sha256,
                )
                .expect("append CasesGenerated");
        }
    }

    fn dispatch_count(&self) -> u64 {
        K2UncertaintyConfirmAttemptJournalV1::open_existing(self.attempt_root())
            .map(|journal| journal.projection().generator_dispatch_count)
            .unwrap_or(0)
    }

    fn phase(&self) -> K2UncertaintyConfirmAttemptPhaseV1 {
        K2UncertaintyConfirmAttemptJournalV1::open_existing(self.attempt_root())
            .expect("open restart projection")
            .projection()
            .phase
    }
}

fn spawn_traced_owner_v1(
    case: &RestartCaseV1,
    trace: &Path,
) -> (Child, Option<SuiteNestedStartV3>) {
    let input = uncertainty_bytes_v1(&case.request).expect("traced owner request");
    let nested = begin_suite_nested_child_v2(&case.owner, &case.request, &input);
    let mut child = Command::new("/usr/bin/strace")
        .args(["-f", "-qq", "-s", "4096", "-e", "trace=flock,execve"])
        .arg("-e")
        .arg("inject=flock:signal=SIGSTOP:when=1")
        .arg("-o")
        .arg(trace)
        .arg(&case.owner)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ptrace owner");
    child
        .stdin
        .take()
        .expect("traced owner stdin")
        .write_all(&input)
        .expect("write traced owner request");
    (child, nested)
}

fn wait_for_tracee_v1(tracer_pid: u32, timeout: Duration) -> u32 {
    let deadline = Instant::now() + timeout;
    loop {
        let path = format!("/proc/{tracer_pid}/task/{tracer_pid}/children");
        if let Ok(children) = fs::read_to_string(path)
            && let Some(pid) = children
                .split_whitespace()
                .find_map(|value| value.parse::<u32>().ok())
        {
            return pid;
        }
        assert!(Instant::now() < deadline, "P07 tracee did not appear");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn wait_for_stopped_lock_v1(pid: u32, inode: u64, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let stopped = fs::read_to_string(format!("/proc/{pid}/status")).is_ok_and(|value| {
            value
                .lines()
                .any(|line| line.starts_with("State:\tT") || line.starts_with("State:\tt"))
        });
        let locked = fs::read_to_string("/proc/locks").is_ok_and(|value| {
            value.lines().any(|line| {
                line.contains("FLOCK")
                    && line
                        .split_whitespace()
                        .any(|field| field == pid.to_string())
                    && line.contains(&format!(":{inode}"))
            })
        });
        if stopped && locked {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "P07 stopped lock identity unavailable"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn decode_owner_success_v1(output: &Output) -> K2UncertaintyDevelopmentRehearsalOwnerReceiptV1 {
    assert!(
        output.status.success(),
        "owner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("decode Development owner receipt")
}

fn frozen_artifacts_root_v1(request: &K2UncertaintyConfirmOwnerRequestV1) -> String {
    composition_root_v1(&(
        "nando.k2-self-formed-confirm-frozen-artifacts.v1",
        &request.request_root_sha256,
        &request.descriptor.executable_manifest_root_sha256,
    ))
    .expect("frozen artifacts root")
}

fn dispatch_root_v1(request_root: &str, generator_sha256: &str) -> String {
    composition_root_v1(&(
        "nando.k2-self-formed-confirm-generator-dispatch.v1",
        request_root,
        generator_sha256,
    ))
    .expect("generator dispatch root")
}
