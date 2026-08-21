use super::*;

pub fn run_process_v1<T: serde::Serialize>(path: &Path, request: &T) -> Output {
    let input = nando_operator_learning::uncertainty_bytes_v1(request).expect("encode R8B process request");
    run_process_input_v2(path, &input)
}

pub fn run_process_recorded_v2<T: serde::Serialize>(path: &Path, request: &T) -> Output {
    let input = uncertainty_bytes_v1(request).expect("encode recorded R8B process request");
    let nested = begin_suite_nested_child_v2(path, request, &input);
    let output = run_process_input_v2(path, &input);
    assert!(output.status.success(), "recorded suite child must succeed");
    finish_suite_nested_child_v2(nested, &output);
    output
}

fn run_process_input_v2(path: &Path, input: &[u8]) -> Output {
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn R8B process");
    child
        .stdin
        .take()
        .expect("R8B process stdin")
        .write_all(input)
        .expect("write R8B process request");
    child.wait_with_output().expect("wait for R8B process")
}

pub fn begin_suite_nested_child_v2<T: serde::Serialize>(
    path: &Path,
    request: &T,
    input: &[u8],
) -> Option<(K2UncertaintyR8BLedgerWriterV2, K2UncertaintyR8BProcessEventV2)> {
    let suite = SUITE_REQUEST_V2.get()?;
    let canonical = fs::canonicalize(path).expect("canonical suite child executable");
    let child_role = role_for_executable_v2(&canonical)?;
    let metadata = fs::metadata(&canonical).expect("suite child metadata");
    let current = fs::canonicalize(std::env::current_exe().expect("suite writer executable")).expect("canonical suite writer");
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
            child_role,
            None,
            None,
            child_role,
            &canonical,
            uncertainty_root_v1(request).expect("suite nested request root"),
            composition_sha256_bytes_v1(input),
            monotonic_ns_v2(),
        )
        .expect("append suite nested ChildStarted");
    Some((writer, started))
}

pub fn finish_suite_nested_child_v2(nested: Option<(K2UncertaintyR8BLedgerWriterV2, K2UncertaintyR8BProcessEventV2)>, output: &Output) {
    let Some((writer, started)) = nested else {
        return;
    };
    assert!(output.status.success(), "recorded suite child must succeed");
    let (schema, root) = typed_json_identity_v2(&output.stdout).expect("typed canonical recorded suite child output");
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
        _ => None,
    }
}

pub fn run_process_success_v1<I, O>(path: &Path, request: &I) -> O
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
{
    let output = run_process_recorded_v2(path, request);
    assert!(output.status.success(), "R8B process failed: {}", String::from_utf8_lossy(&output.stderr));
    let decoded: O = nando_operator_learning::uncertainty_decode_v1(&output.stdout).expect("decode R8B process output");
    assert_eq!(
        output.stdout,
        nando_operator_learning::uncertainty_bytes_v1(&decoded).expect("re-encode R8B process output")
    );
    decoded
}

pub struct DurableProcessLedgerV2 {
    writer: K2UncertaintyR8BLedgerWriterV2,
    root: PathBuf,
    route_id_sha256: String,
}

impl DurableProcessLedgerV2 {
    pub fn create(root: &Path, route_id_sha256: String, writer: &BinaryV2, allowed_children: &[&BinaryV2]) -> Self {
        create_private_directory_v1(root);
        assert!(fs::read_dir(root).expect("new ledger directory").next().is_none());
        let writer = K2UncertaintyR8BLedgerWriterV2::new(
            root.to_path_buf(),
            route_id_sha256.clone(),
            writer.role,
            writer.sha256.clone(),
            allowed_children.iter().map(|child| child.identity()).collect(),
        )
        .expect("create shared process ledger writer");
        Self {
            writer,
            root: root.to_path_buf(),
            route_id_sha256,
        }
    }

    pub fn open(root: &Path, route_id_sha256: String, writer: &BinaryV2, allowed_children: &[&BinaryV2]) -> Self {
        let root = fs::canonicalize(root).expect("canonical existing ledger root");
        let writer = K2UncertaintyR8BLedgerWriterV2::new(
            root.clone(),
            route_id_sha256.clone(),
            writer.role,
            writer.sha256.clone(),
            allowed_children.iter().map(|child| child.identity()).collect(),
        )
        .expect("open shared process ledger writer");
        Self { writer, root, route_id_sha256 }
    }

    pub fn start(
        &mut self,
        stage_id: &str,
        case_id_sha256: Option<String>,
        probe_ordinal: Option<u64>,
        binary: &BinaryV2,
        request_root_sha256: String,
        stdin_sha256: String,
    ) -> K2UncertaintyR8BProcessEventV2 {
        self.writer
            .child_started(
                stage_id,
                case_id_sha256,
                probe_ordinal,
                binary.role,
                &binary.path,
                request_root_sha256,
                stdin_sha256,
                monotonic_ns_v2(),
            )
            .expect("append ChildStarted")
    }

    pub fn finish(
        &mut self,
        start: &K2UncertaintyR8BProcessEventV2,
        output: &Output,
        receipt_schema: String,
        decoded_receipt_root_sha256: String,
    ) -> K2UncertaintyR8BProcessEventV2 {
        assert!(output.status.success(), "process must succeed before ChildFinished");
        self.writer
            .child_finished(
                start,
                &output.stdout,
                &output.stderr,
                vec![K2UncertaintyR8BProducedReceiptV2 {
                    relative_path: K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2.to_owned(),
                    byte_len: output.stdout.len() as u64,
                    unix_mode: 0,
                    content_sha256: composition_sha256_bytes_v1(&output.stdout),
                    receipt_schema,
                    semantic_root_sha256: decoded_receipt_root_sha256,
                }],
                monotonic_ns_v2(),
            )
            .expect("append ChildFinished")
    }

    pub fn freeze(self) -> K2UncertaintyR8BProcessLedgerV2 {
        self.writer.complete_ledger().expect("freeze complete process ledger")
    }

    pub fn environment(&self) -> [(String, String); 2] {
        [
            (K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2.to_owned(), self.root.to_string_lossy().into_owned()),
            (K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2.to_owned(), self.route_id_sha256.clone()),
        ]
    }

    pub fn finish_closed(
        &self,
        start: &K2UncertaintyR8BProcessEventV2,
        output: &Output,
        receipts: Vec<K2UncertaintyR8BProducedReceiptV2>,
    ) -> K2UncertaintyR8BProcessEventV2 {
        assert!(output.status.success(), "closed producer must exit successfully");
        self.writer
            .child_finished(start, &output.stdout, &output.stderr, receipts, monotonic_ns_v2())
            .expect("append closed-channel ChildFinished")
    }
}

pub struct RecordedProcessV2<T> {
    pub value: T,
    pub output: Output,
    pub finished: K2UncertaintyR8BProcessEventV2,
}

pub struct ClosedProducerV2 {
    pub request: K2UncertaintyR8BProducerRequestV2,
    pub output: Output,
    pub descriptors: Vec<K2UncertaintyR8BProducedReceiptV2>,
    pub finished: K2UncertaintyR8BProcessEventV2,
}

pub fn run_closed_suite_producer_v2(
    ledger: &mut DurableProcessLedgerV2,
    binary: &BinaryV2,
    selector: &str,
    output_root: &Path,
    allowed_paths: Vec<String>,
) -> ClosedProducerV2 {
    create_private_directory_v1(output_root);
    assert!(fs::read_dir(output_root).expect("fresh suite output").next().is_none());
    let mut request = K2UncertaintyR8BProducerRequestV2 {
        schema: String::new(),
        route_id_sha256: ledger.route_id_sha256.clone(),
        producer_role: binary.role.to_owned(),
        producer_executable_sha256: binary.sha256.clone(),
        test_selector: selector.to_owned(),
        allowed_relative_paths: allowed_paths,
        exclusive_output_directory: output_root.to_string_lossy().into_owned(),
        request_root_sha256: String::new(),
    };
    request.reseal().expect("seal suite producer request");
    let input = uncertainty_bytes_v1(&request).expect("suite producer stdin");
    let started = ledger.start(
        binary.role,
        None,
        None,
        binary,
        request.request_root_sha256.clone(),
        composition_sha256_bytes_v1(&input),
    );
    let mut command = Command::new(&binary.path);
    command
        .args(["--ignored", "--exact", selector, "--nocapture"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in ledger.environment() {
        command.env(key, value);
    }
    let output = run_command_bytes_v2(command, &input, 1_200);
    assert!(output.status.success(), "suite producer failed: {}", String::from_utf8_lossy(&output.stderr));
    let descriptors = reopen_closed_measured_receipts_v2(&request);
    let finished = ledger.finish_closed(&started, &output, descriptors.clone());
    ClosedProducerV2 {
        request,
        output,
        descriptors,
        finished,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_recorded_process_v2<I, O, F>(
    ledger: &mut DurableProcessLedgerV2,
    binary: &BinaryV2,
    stage_id: &str,
    case_id_sha256: Option<String>,
    probe_ordinal: Option<u64>,
    request_root_sha256: String,
    request: &I,
    receipt_root: F,
) -> RecordedProcessV2<O>
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned + serde::Serialize,
    F: FnOnce(&O) -> (&str, &str),
{
    let input = uncertainty_bytes_v1(request).expect("recorded process input");
    let start = ledger.start(
        stage_id,
        case_id_sha256,
        probe_ordinal,
        binary,
        request_root_sha256,
        composition_sha256_bytes_v1(&input),
    );
    let output = run_process_bytes_v2(&binary.path, &input, 60);
    let value: O = uncertainty_decode_v1(&output.stdout)
        .unwrap_or_else(|error| panic!("{} output invalid: {error}; stderr={}", binary.role, String::from_utf8_lossy(&output.stderr)));
    let (schema, root) = receipt_root(&value);
    let finished = ledger.finish(&start, &output, schema.to_owned(), root.to_owned());
    assert!(output.status.success(), "{} process failed", binary.role);
    RecordedProcessV2 { value, output, finished }
}

pub struct PrivateDescriptorV2 {
    file: File,
    pub inode: u64,
    pub byte_len: u64,
    pub unix_mode: u32,
}

impl PrivateDescriptorV2 {
    pub fn open(path: &Path, expected_byte_len: u64, expected_unix_mode: u32) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW)
            .open(path)
            .expect("open O_PATH private descriptor");
        let metadata = file.metadata().expect("private descriptor metadata");
        assert!(metadata.is_file());
        assert_eq!(metadata.nlink(), 1);
        assert_eq!(metadata.len(), expected_byte_len);
        assert_eq!(metadata.permissions().mode() & 0o7777, expected_unix_mode);
        let mut flags = rustix::io::fcntl_getfd(&file).expect("read private descriptor flags");
        flags.remove(rustix::io::FdFlags::CLOEXEC);
        rustix::io::fcntl_setfd(&file, flags).expect("make private descriptor inheritable by the immediate bwrap child");
        Self {
            file,
            inode: metadata.ino(),
            byte_len: metadata.len(),
            unix_mode: metadata.permissions().mode() & 0o7777,
        }
    }

    pub fn proc_path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.file.as_raw_fd()))
    }
}

pub fn write_new_read_only_v2(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(path)
        .expect("create immutable evidence file");
    file.write_all(bytes).expect("write immutable evidence file");
    file.sync_all().expect("fsync immutable evidence file");
}

pub fn freeze_directory_tree_v2(root: &Path) {
    let mut directories = vec![root.to_path_buf()];
    let mut index = 0;
    while index < directories.len() {
        for entry in fs::read_dir(&directories[index]).expect("read evidence tree") {
            let path = entry.expect("evidence tree entry").path();
            if path.is_dir() {
                directories.push(path);
            } else {
                fs::set_permissions(path, fs::Permissions::from_mode(0o400)).expect("freeze evidence file");
            }
        }
        index += 1;
    }
    for directory in directories.into_iter().rev() {
        File::open(&directory)
            .expect("open evidence directory")
            .sync_all()
            .expect("fsync evidence directory");
        fs::set_permissions(directory, fs::Permissions::from_mode(0o500)).expect("freeze evidence directory");
    }
}

fn run_process_bytes_v2(path: &Path, input: &[u8], timeout_seconds: u64) -> Output {
    let mut command = Command::new(path);
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    run_command_bytes_v2(command, input, timeout_seconds)
}

fn run_command_bytes_v2(mut command: Command, input: &[u8], timeout_seconds: u64) -> Output {
    let mut child = command.spawn().expect("spawn recorded process");
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
        stdout.by_ref().take(1_048_577).read_to_end(&mut bytes).expect("read recorded stdout");
        bytes
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.by_ref().take(65_537).read_to_end(&mut bytes).expect("read recorded stderr");
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
    Output {
        status,
        stdout: stdout_thread.join().expect("join recorded stdout"),
        stderr: stderr_thread.join().expect("join recorded stderr"),
    }
}

fn monotonic_ns_v2() -> u64 {
    static ORIGIN: OnceLock<std::time::Instant> = OnceLock::new();
    ORIGIN.get_or_init(std::time::Instant::now).elapsed().as_nanos() as u64
}
