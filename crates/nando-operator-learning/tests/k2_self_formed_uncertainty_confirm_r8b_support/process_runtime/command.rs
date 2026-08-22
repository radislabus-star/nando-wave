use super::*;

pub fn run_process_v1<T: serde::Serialize>(path: &Path, request: &T) -> Output {
    let input =
        nando_operator_learning::uncertainty_bytes_v1(request).expect("encode R8B process request");
    run_process_input_v2(path, &input)
}

pub fn run_process_recorded_v2<T: serde::Serialize>(path: &Path, request: &T) -> Output {
    run_suite_command_recorded_v3(path, request, Command::new(path), None)
}

pub fn try_run_parent_process_v3<T: serde::Serialize>(
    path: &Path,
    working_directory: Option<&Path>,
    request: &T,
    timeout_seconds: u64,
) -> Result<Output, &'static str> {
    let input =
        uncertainty_bytes_v1(request).map_err(|_| "r8b_v8_parent_process_request_encode_failed")?;
    let mut command = Command::new(path);
    if let Some(directory) = working_directory {
        command.current_dir(directory);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    try_run_command_bytes_v3(command, &input, timeout_seconds)
        .map_err(|_| "r8b_v8_parent_process_transport_failed")
}

pub fn run_process_expected_failure_recorded_v3<T: serde::Serialize>(
    path: &Path,
    request: &T,
    exact_exit_code: i32,
) -> Output {
    run_suite_command_recorded_v3(path, request, Command::new(path), Some(exact_exit_code))
}

pub fn run_suite_command_recorded_v3<T: serde::Serialize>(
    path: &Path,
    request: &T,
    mut command: Command,
    expected_exit_code: Option<i32>,
) -> Output {
    let input = uncertainty_bytes_v1(request).expect("encode recorded R8B process request");
    let nested = begin_suite_nested_child_v2(path, request, &input);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = match try_run_command_bytes_v3(command, &input, 1_200) {
        Ok(output) => output,
        Err(failure) => fail_suite_nested_child_v3(nested, failure),
    };
    if let Some(expected) = expected_exit_code {
        if output.status.code() != Some(expected) {
            fail_suite_nested_child_v3(
                nested,
                CommandFailureV3::terminal(&output, "recorded suite child exit predicate mismatch"),
            );
        }
        finish_suite_nested_expected_failure_v3(nested, &output, expected);
        return output;
    }
    if !output.status.success() {
        fail_suite_nested_child_v3(
            nested,
            CommandFailureV3::terminal(&output, "recorded suite child failed"),
        );
    }
    finish_suite_nested_child_v2(nested, &output);
    output
}

fn run_process_input_v2(path: &Path, input: &[u8]) -> Output {
    try_run_process_input_v3(path, input)
        .unwrap_or_else(|failure| panic!("R8B process failed before output: {}", failure.message))
}

fn try_run_process_input_v3(path: &Path, input: &[u8]) -> Result<Output, CommandFailureV3> {
    let mut command = Command::new(path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    try_run_command_bytes_v3(command, input, 1_200)
}

pub struct SuiteNestedStartV3 {
    legacy: Option<(
        K2UncertaintyR8BLedgerWriterV2,
        K2UncertaintyR8BProcessEventV2,
    )>,
    v3: Option<(
        K2UncertaintyR8BLedgerWriterV3,
        K2UncertaintyR8BProcessEventV3,
    )>,
}

pub fn begin_suite_nested_child_v2<T: serde::Serialize>(
    path: &Path,
    request: &T,
    input: &[u8],
) -> Option<SuiteNestedStartV3> {
    let suite = SUITE_REQUEST_V2.get()?;
    let canonical = fs::canonicalize(path).expect("canonical suite child executable");
    let child_role = role_for_executable_v2(&canonical)?;
    let stage = suite_stage_v3(&suite.producer_role).unwrap_or(child_role);
    let request_root = uncertainty_root_v1(request).expect("suite nested request root");
    let stdin_sha256 = composition_sha256_bytes_v1(input);
    let monotonic = monotonic_ns_v2();
    let v3 = active_producer_request_v3().map(|(request, _)| {
        let bound = append_frozen_request_v3(
            request,
            &suite.producer_role,
            child_role,
            stage,
            None,
            None,
            request_root.clone(),
            stdin_sha256.clone(),
            monotonic,
        );
        assert_eq!(
            bound.1.invocation.target_executable_sha256,
            composition_sha256_file_v1(&canonical).expect("suite child SHA-256")
        );
        bound
    });
    let legacy = if v3.is_none() {
        let metadata = fs::metadata(&canonical).expect("suite child metadata");
        let current = fs::canonicalize(std::env::current_exe().expect("suite writer executable"))
            .expect("canonical suite writer");
        let writer = K2UncertaintyR8BLedgerWriterV2::from_environment(
            &suite.producer_role,
            composition_sha256_file_v1(&current).expect("suite writer SHA-256"),
            vec![K2UncertaintyR8BExecutableIdentityV2 {
                role: child_role.to_owned(),
                canonical_path: canonical.to_string_lossy().into_owned(),
                byte_len: metadata.len(),
                unix_mode: metadata.permissions().mode() & 0o7777,
                sha256: composition_sha256_file_v1(&canonical).expect("suite child SHA-256"),
            }],
        )
        .expect("open suite nested ledger")?;
        let started = writer
            .child_started(
                stage,
                None,
                None,
                child_role,
                &canonical,
                request_root,
                stdin_sha256,
                monotonic,
            )
            .expect("append suite nested ChildStarted");
        Some((writer, started))
    } else {
        None
    };
    Some(SuiteNestedStartV3 { legacy, v3 })
}

pub fn finish_suite_nested_child_v2(nested: Option<SuiteNestedStartV3>, output: &Output) {
    let Some(nested) = nested else {
        return;
    };
    assert!(output.status.success(), "recorded suite child must succeed");
    let (schema, root) = typed_json_identity_v2(&output.stdout)
        .expect("typed canonical recorded suite child output");
    if let Some((writer, event)) = nested.v3 {
        writer
            .success(
                &event,
                &output.stdout,
                &output.stderr,
                schema.clone(),
                root.clone(),
                K2UncertaintyR8BValidatedFactV3::None,
                Vec::new(),
                monotonic_ns_v2(),
            )
            .expect("append suite nested V3 completion");
    }
    if let Some((writer, started)) = nested.legacy {
        writer
            .child_finished(
                &started,
                &output.stdout,
                &output.stderr,
                vec![K2UncertaintyR8BProducedReceiptV2 {
                    relative_path: K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2.to_owned(),
                    byte_len: output.stdout.len() as u64,
                    unix_mode: 0,
                    content_sha256: composition_sha256_bytes_v1(&output.stdout),
                    receipt_schema: schema,
                    semantic_root_sha256: root,
                }],
                monotonic_ns_v2(),
            )
            .expect("append suite nested ChildFinished");
    }
}

fn finish_suite_nested_expected_failure_v3(
    nested: Option<SuiteNestedStartV3>,
    output: &Output,
    expected_exit_code: i32,
) {
    let Some(nested) = nested else { return };
    assert!(
        nested.legacy.is_none(),
        "expected failures require the V3 ledger"
    );
    let (writer, started) = nested.v3.expect("expected failure V3 start");
    let completed = writer
        .failure(
            &started,
            K2UncertaintyR8BCompletionKindV3::UnexpectedFailure,
            output.status.code().unwrap_or(-1),
            &output.stdout,
            &output.stderr,
            monotonic_ns_v2(),
        )
        .expect("append suite nested expected failure");
    assert_eq!(output.status.code(), Some(expected_exit_code));
    assert_eq!(
        completed.completion,
        Some(K2UncertaintyR8BCompletionKindV3::DiagnosticExpectedFailure)
    );
}

fn fail_suite_nested_child_v3(nested: Option<SuiteNestedStartV3>, failure: CommandFailureV3) -> ! {
    if let Some(SuiteNestedStartV3 {
        v3: Some((writer, started)),
        ..
    }) = nested
    {
        writer
            .failure(
                &started,
                failure.kind,
                failure.exit_code,
                &failure.stdout,
                &failure.stderr,
                monotonic_ns_v2(),
            )
            .expect("append suite nested V3 failure");
    }
    panic!("{}", failure.message)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_frozen_request_v3(
    request: &K2UncertaintyR8BProducerRequestV3,
    owner: &str,
    target: &str,
    stage: &str,
    case_id_sha256: Option<String>,
    probe_ordinal: Option<u64>,
    request_root_sha256: String,
    stdin_sha256: String,
    monotonic_ns: u64,
) -> (
    K2UncertaintyR8BLedgerWriterV3,
    K2UncertaintyR8BProcessEventV3,
) {
    let writer =
        K2UncertaintyR8BLedgerWriterV3::attach_request(request).expect("attach frozen V3 ledger");
    let observed = writer
        .summary()
        .expect("read frozen V3 prefix")
        .invocations
        .into_iter()
        .map(|row| row.invocation_id_sha256)
        .collect::<BTreeSet<_>>();
    let plan = request
        .invocation_plan
        .iter()
        .find(|row| {
            row.request_owner_role == owner
                && row.target_role == target
                && row.stage == stage
                && row.case_id_sha256 == case_id_sha256
                && row.probe_ordinal == probe_ordinal
                && !observed.contains(&row.invocation_id_sha256)
        })
        .cloned()
        .expect("unused frozen V3 invocation");
    let event = writer
        .request(plan, request_root_sha256, stdin_sha256, monotonic_ns)
        .expect("append frozen V3 InvocationRequested");
    (writer, event)
}

fn role_for_executable_v2(path: &Path) -> Option<&'static str> {
    match path.file_name()?.to_str()? {
        "nando-k2-self-formed-confirm-owner" => Some("M01_DEVELOPMENT_OWNER"),
        "nando-k2-self-formed-public-coordinator" => Some("M10_PUBLIC_COORDINATOR"),
        "nando-k2-self-formed-control-evaluator" => Some("M17_CONTROL_EVALUATOR"),
        "nando-k2-self-formed-terminal-evaluator" => Some("M18_TERMINAL_EVALUATOR"),
        "nando-k2-self-formed-cleanup-authorizer" => Some("M20_CLEANUP_AUTHORIZER"),
        "nando-k2-self-formed-cleanup-owner" => Some("M21_CLEANUP_OWNER"),
        "nando-k2-self-formed-cleanup-verifier" => Some("M22_CLEANUP_VERIFIER"),
        "nando-k2-self-formed-result-publisher" => Some("M23_DEVELOPMENT_RESULT_PUBLISHER"),
        "nando-k2-self-formed-r8b-evidence-publisher" => Some("M26_R8B_PUBLISHER"),
        _ => None,
    }
}

fn suite_stage_v3(role: &str) -> Option<&'static str> {
    match role {
        "S02_RESTART" => Some("C02"),
        "S03_MODE_MATRIX" => Some("C03"),
        "S04_CLEANUP_NEGATIVE" => Some("C04"),
        "S05_AUTHORITY_PUBLICATION" => Some("C05"),
        _ => None,
    }
}

pub fn run_process_success_v1<I, O>(path: &Path, request: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let output = run_process_recorded_v2(path, request);
    assert!(
        output.status.success(),
        "R8B process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decoded: O = nando_operator_learning::uncertainty_decode_v1(&output.stdout)
        .expect("decode R8B process output");
    assert_eq!(
        output.stdout,
        nando_operator_learning::uncertainty_bytes_v1(&decoded)
            .expect("re-encode R8B process output")
    );
    decoded
}

fn run_process_bytes_v2(path: &Path, input: &[u8], timeout_seconds: u64) -> Output {
    try_run_process_bytes_v3(path, input, timeout_seconds)
        .unwrap_or_else(|failure| panic!("recorded process failed: {}", failure.message))
}

pub(super) fn try_run_process_bytes_v3(
    path: &Path,
    input: &[u8],
    timeout_seconds: u64,
) -> Result<Output, CommandFailureV3> {
    let mut command = Command::new(path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    try_run_command_bytes_v3(command, input, timeout_seconds)
}

pub(super) fn run_command_bytes_v2(command: Command, input: &[u8], timeout_seconds: u64) -> Output {
    try_run_command_bytes_v3(command, input, timeout_seconds)
        .unwrap_or_else(|failure| panic!("recorded process failed: {}", failure.message))
}

#[derive(Debug)]
pub(super) struct CommandFailureV3 {
    pub(super) kind: K2UncertaintyR8BCompletionKindV3,
    pub(super) exit_code: i32,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
    pub(super) message: String,
}

impl CommandFailureV3 {
    pub(super) fn launch(error: impl std::fmt::Display) -> Self {
        let message = format!("spawn recorded process: {error}");
        Self {
            kind: K2UncertaintyR8BCompletionKindV3::LaunchFailure,
            exit_code: -1,
            stdout: Vec::new(),
            stderr: message.as_bytes().to_vec(),
            message,
        }
    }

    pub(super) fn terminal(output: &Output, message: &str) -> Self {
        Self {
            kind: K2UncertaintyR8BCompletionKindV3::UnexpectedFailure,
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout.clone(),
            stderr: output.stderr.clone(),
            message: message.to_owned(),
        }
    }
}

pub(super) fn try_run_command_bytes_v3(
    mut command: Command,
    input: &[u8],
    timeout_seconds: u64,
) -> Result<Output, CommandFailureV3> {
    let mut child = command.spawn().map_err(CommandFailureV3::launch)?;
    child
        .stdin
        .take()
        .expect("recorded process stdin")
        .write_all(input)
        .expect("write recorded process stdin");
    let mut stdout = child.stdout.take().expect("recorded stdout");
    let mut stderr = child.stderr.take().expect("recorded stderr");
    let stdout_thread = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .by_ref()
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .expect("read recorded stdout");
        bytes
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .by_ref()
            .take(65_537)
            .read_to_end(&mut bytes)
            .expect("read recorded stderr");
        bytes
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_seconds);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll recorded process") {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().expect("kill timed out recorded process");
            break child.wait().expect("reap timed out recorded process");
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    };
    Ok(Output {
        status,
        stdout: stdout_thread.join().expect("join recorded stdout"),
        stderr: stderr_thread.join().expect("join recorded stderr"),
    })
}

pub(super) fn monotonic_ns_v2() -> u64 {
    static ORIGIN: OnceLock<std::time::Instant> = OnceLock::new();
    ORIGIN
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_nanos() as u64
}
