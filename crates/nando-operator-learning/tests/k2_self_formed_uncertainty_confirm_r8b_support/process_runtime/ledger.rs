use super::*;

pub struct DurableProcessLedgerV2 {
    writer: K2UncertaintyR8BLedgerWriterV2,
    root: PathBuf,
    route_id_sha256: String,
    writer_role: &'static str,
    v3: Option<V3LedgerBinding>,
}

#[derive(Clone)]
struct V3LedgerBinding {
    request: K2UncertaintyR8BProducerRequestV3,
    path: PathBuf,
}

pub struct BoundProcessStartV3 {
    legacy: K2UncertaintyR8BProcessEventV2,
    v3: Option<(
        K2UncertaintyR8BLedgerWriterV3,
        K2UncertaintyR8BProcessEventV3,
    )>,
}

impl DurableProcessLedgerV2 {
    pub fn create(
        root: &Path,
        route_id_sha256: String,
        writer: &BinaryV2,
        allowed_children: &[&BinaryV2],
    ) -> Self {
        let writer_role = writer.role;
        create_private_directory_v1(root);
        assert!(
            fs::read_dir(root)
                .expect("new ledger directory")
                .next()
                .is_none()
        );
        let writer = K2UncertaintyR8BLedgerWriterV2::new(
            root.to_path_buf(),
            route_id_sha256.clone(),
            writer.role,
            writer.sha256.clone(),
            allowed_children
                .iter()
                .map(|child| child.identity())
                .collect(),
        )
        .expect("create shared process ledger writer");
        Self {
            writer,
            root: root.to_path_buf(),
            route_id_sha256,
            writer_role,
            v3: None,
        }
    }

    pub fn open(
        root: &Path,
        route_id_sha256: String,
        writer: &BinaryV2,
        allowed_children: &[&BinaryV2],
    ) -> Self {
        let writer_role = writer.role;
        let root = fs::canonicalize(root).expect("canonical existing ledger root");
        let writer = K2UncertaintyR8BLedgerWriterV2::new(
            root.clone(),
            route_id_sha256.clone(),
            writer.role,
            writer.sha256.clone(),
            allowed_children
                .iter()
                .map(|child| child.identity())
                .collect(),
        )
        .expect("open shared process ledger writer");
        let v3 = active_producer_request_v3().map(|(request, path)| V3LedgerBinding {
            request: request.clone(),
            path: path.to_path_buf(),
        });
        Self {
            writer,
            root,
            route_id_sha256,
            writer_role,
            v3,
        }
    }

    pub fn bind_v3(&mut self, request: K2UncertaintyR8BProducerRequestV3, path: PathBuf) {
        validate_self_formed_r8b_producer_request_v3(&request)
            .expect("validate bound V3 producer request");
        assert_eq!(request.route_id_sha256, self.route_id_sha256);
        assert_eq!(
            fs::read(&path).expect("read bound V3 producer request"),
            uncertainty_bytes_v1(&request).expect("V3 producer bytes")
        );
        self.v3 = Some(V3LedgerBinding { request, path });
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

    #[allow(clippy::too_many_arguments)]
    pub fn start_bound(
        &mut self,
        stage_id: &str,
        case_id_sha256: Option<String>,
        probe_ordinal: Option<u64>,
        binary: &BinaryV2,
        request_root_sha256: String,
        stdin_sha256: String,
    ) -> BoundProcessStartV3 {
        let monotonic = monotonic_ns_v2();
        let v3 = self.v3.as_ref().map(|binding| {
            append_frozen_request_v3(
                &binding.request,
                self.writer_role,
                binary.role,
                stage_id,
                case_id_sha256.clone(),
                probe_ordinal,
                request_root_sha256.clone(),
                stdin_sha256.clone(),
                monotonic,
            )
        });
        let legacy = self.start(
            stage_id,
            case_id_sha256,
            probe_ordinal,
            binary,
            request_root_sha256,
            stdin_sha256,
        );
        BoundProcessStartV3 { legacy, v3 }
    }

    pub fn finish(
        &mut self,
        start: &K2UncertaintyR8BProcessEventV2,
        output: &Output,
        receipt_schema: String,
        decoded_receipt_root_sha256: String,
    ) -> K2UncertaintyR8BProcessEventV2 {
        assert!(
            output.status.success(),
            "process must succeed before ChildFinished"
        );
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

    pub fn finish_bound(
        &mut self,
        start: &BoundProcessStartV3,
        output: &Output,
        receipt_schema: String,
        decoded_receipt_root_sha256: String,
        fact: K2UncertaintyR8BValidatedFactV3,
        authority_outputs: Vec<K2UncertaintyR8BOutputContractV3>,
    ) -> K2UncertaintyR8BProcessEventV2 {
        assert!(
            output.status.success(),
            "process must succeed before typed completion"
        );
        if let Some((writer, started)) = &start.v3 {
            writer
                .success(
                    started,
                    &output.stdout,
                    &output.stderr,
                    receipt_schema.clone(),
                    decoded_receipt_root_sha256.clone(),
                    fact,
                    authority_outputs,
                    monotonic_ns_v2(),
                )
                .expect("append V3 typed completion");
        }
        self.finish(
            &start.legacy,
            output,
            receipt_schema,
            decoded_receipt_root_sha256,
        )
    }

    pub(super) fn fail_bound(&mut self, start: &BoundProcessStartV3, failure: &CommandFailureV3) {
        if let Some((writer, started)) = &start.v3 {
            writer
                .failure(
                    started,
                    failure.kind,
                    failure.exit_code,
                    &failure.stdout,
                    &failure.stderr,
                    monotonic_ns_v2(),
                )
                .expect("append V3 process failure");
        }
    }

    pub fn fail_launch_bound(
        &mut self,
        start: &BoundProcessStartV3,
        message: impl std::fmt::Display,
    ) -> ! {
        let failure = CommandFailureV3::launch(message);
        self.fail_bound(start, &failure);
        panic!("{}", failure.message)
    }

    pub fn fail_unexpected_bound(
        &mut self,
        start: &BoundProcessStartV3,
        output: &Output,
        message: impl Into<String>,
    ) -> ! {
        let failure = CommandFailureV3::terminal(output, &message.into());
        self.fail_bound(start, &failure);
        panic!("{}", failure.message)
    }

    pub fn run_started_process_v3(
        &mut self,
        start: &BoundProcessStartV3,
        path: &Path,
        input: &[u8],
        timeout_seconds: u64,
    ) -> Output {
        match try_run_process_bytes_v3(path, input, timeout_seconds) {
            Ok(output) if output.status.success() => output,
            Ok(output) => self.fail_unexpected_bound(start, &output, "recorded process failed"),
            Err(failure) => {
                self.fail_bound(start, &failure);
                panic!("{}", failure.message)
            }
        }
    }

    pub fn freeze(self) -> K2UncertaintyR8BProcessLedgerV2 {
        self.writer
            .complete_ledger()
            .expect("freeze complete process ledger")
    }

    pub fn environment(&self) -> Vec<(String, String)> {
        let mut values = vec![
            (
                K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2.to_owned(),
                self.root.to_string_lossy().into_owned(),
            ),
            (
                K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2.to_owned(),
                self.route_id_sha256.clone(),
            ),
        ];
        if let Some(binding) = &self.v3 {
            values.push((
                K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_ENV_V3.to_owned(),
                binding.path.to_string_lossy().into_owned(),
            ));
        }
        values
    }

    pub fn finish_closed(
        &self,
        start: &K2UncertaintyR8BProcessEventV2,
        output: &Output,
        receipts: Vec<K2UncertaintyR8BProducedReceiptV2>,
    ) -> K2UncertaintyR8BProcessEventV2 {
        assert!(
            output.status.success(),
            "closed producer must exit successfully"
        );
        self.writer
            .child_finished(
                start,
                &output.stdout,
                &output.stderr,
                receipts,
                monotonic_ns_v2(),
            )
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
    assert!(
        fs::read_dir(output_root)
            .expect("fresh suite output")
            .next()
            .is_none()
    );
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
    assert!(
        output.status.success(),
        "suite producer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
    let start = ledger.start_bound(
        stage_id,
        case_id_sha256,
        probe_ordinal,
        binary,
        request_root_sha256,
        composition_sha256_bytes_v1(&input),
    );
    let mut command = Command::new(&binary.path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in ledger.environment() {
        command.env(key, value);
    }
    let output = match try_run_command_bytes_v3(command, &input, 60) {
        Ok(output) => output,
        Err(failure) => {
            ledger.fail_bound(&start, &failure);
            panic!("{} launch failed: {}", binary.role, failure.message);
        }
    };
    if !output.status.success() {
        let failure =
            CommandFailureV3::terminal(&output, &format!("{} process failed", binary.role));
        ledger.fail_bound(&start, &failure);
        panic!("{}", failure.message);
    }
    let value: O = uncertainty_decode_v1(&output.stdout).unwrap_or_else(|error| {
        let failure = CommandFailureV3::terminal(
            &output,
            &format!("{} output invalid: {error}", binary.role),
        );
        ledger.fail_bound(&start, &failure);
        panic!(
            "{}; stderr={}",
            failure.message,
            String::from_utf8_lossy(&output.stderr)
        );
    });
    let (schema, root) = receipt_root(&value);
    let finished = ledger.finish_bound(
        &start,
        &output,
        schema.to_owned(),
        root.to_owned(),
        K2UncertaintyR8BValidatedFactV3::None,
        Vec::new(),
    );
    RecordedProcessV2 {
        value,
        output,
        finished,
    }
}
