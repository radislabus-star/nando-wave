use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::atomic::{Ordering, compiler_fence};
use std::thread;
use std::time::{Duration, Instant};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1,
    composition_sha256_file_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2UncertaintyAuthorizationSlotLedgerV1,
    K2UncertaintyConfirmAttemptEventKindV1, K2UncertaintyConfirmAttemptJournalV1,
    K2UncertaintyConfirmAttemptModeV1, K2UncertaintyConfirmGeneratorRequestV1,
    K2UncertaintyConfirmGeneratorResponseV1, K2UncertaintyConfirmOwnerReceiptV1,
    K2UncertaintyConfirmOwnerRequestV1, K2UncertaintyConfirmPipeReceiptV1,
    K2UncertaintyGeneratorResponseV1, load_confirm_generator_split_receipt_v1,
    persist_retained_confirm_nonce_v1, publish_confirm_generator_split_v1,
    retained_confirm_nonce_observed_root_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
    uncertainty_root_v1,
};

const BWRAP_PATH_V1: &str = "/usr/bin/bwrap";
const PRLIMIT_PATH_V1: &str = "/usr/bin/prlimit";
const GUEST_GENERATOR_PATH_V1: &str = "/nando/bin/generator";
const GENERATOR_CPU_SECONDS_V1: u64 = 30;
const GENERATOR_TIMEOUT_SECONDS_V1: u64 = 40;
const MAX_GENERATOR_STDERR_BYTES_V1: usize = 65_536;

pub fn execute_self_formed_confirm_owner_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    owner_executable: &Path,
) -> K2CompositionResultV1<K2UncertaintyConfirmOwnerReceiptV1> {
    request.validate()?;
    let owner_executable_sha256 = composition_sha256_file_v1(owner_executable)?;
    let generator_executable = PathBuf::from(&request.generator_executable_path);
    let generator_executable_sha256 = composition_sha256_file_v1(&generator_executable)?;
    if owner_executable_sha256 != request.descriptor.confirm_owner_executable_sha256
        || generator_executable_sha256 != request.descriptor.generator_executable_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_owner_executable_mismatch",
        ));
    }

    let lab_root = canonical_private_lab_root_v1(&request.lab_root)?;
    validate_authority_binding_v1(request, &lab_root)?;
    let attempt_root = lab_root.join(&request.attempt_relative_path);
    if attempt_root.exists() {
        return project_existing_attempt_without_replay_v1(
            request,
            &attempt_root,
            owner_executable_sha256,
        );
    }
    let mut journal = K2UncertaintyConfirmAttemptJournalV1::create_exclusive(
        &attempt_root,
        request.descriptor.clone(),
    )?;
    let artifacts_root = uncertainty_root_v1(&(
        "nando.k2-self-formed-confirm-frozen-artifacts.v1",
        &request.request_root_sha256,
        &request.descriptor.executable_manifest_root_sha256,
    ))?;
    journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
        owner_executable_sha256.clone(),
        request.request_root_sha256.clone(),
        artifacts_root,
    )?;

    match request.descriptor.mode {
        K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal => {
            execute_development_rehearsal_v1(
                request,
                &generator_executable,
                owner_executable_sha256,
                &mut journal,
            )
        }
        K2UncertaintyConfirmAttemptModeV1::Confirm => execute_confirm_v1(
            request,
            &attempt_root,
            &generator_executable,
            owner_executable_sha256,
            &mut journal,
        ),
    }
}

fn project_existing_attempt_without_replay_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    attempt_root: &Path,
    owner_executable_sha256: String,
) -> K2CompositionResultV1<K2UncertaintyConfirmOwnerReceiptV1> {
    let mut journal = K2UncertaintyConfirmAttemptJournalV1::open_existing(attempt_root)?;
    if journal.descriptor() != &request.descriptor {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_existing_attempt_descriptor_mismatch",
        ));
    }
    let retained_nonce_root = retained_confirm_nonce_observed_root_v1(attempt_root)?;
    let complete_split_root =
        if request.descriptor.mode == K2UncertaintyConfirmAttemptModeV1::Confirm {
            load_confirm_generator_split_receipt_v1(&attempt_root.join("generated"))
                .ok()
                .map(|receipt| receipt.split_receipt_root_sha256)
        } else {
            None
        };
    journal.recover_after_restart(
        owner_executable_sha256,
        request.request_root_sha256.clone(),
        retained_nonce_root,
        complete_split_root,
    )?;
    Err(K2CompositionErrorV1::Invalid(
        "self_formed_confirm_existing_attempt_recovered_without_replay",
    ))
}

fn execute_development_rehearsal_v1(
    owner_request: &K2UncertaintyConfirmOwnerRequestV1,
    generator_executable: &Path,
    owner_executable_sha256: String,
    journal: &mut K2UncertaintyConfirmAttemptJournalV1,
) -> K2CompositionResultV1<K2UncertaintyConfirmOwnerReceiptV1> {
    let generator_request = owner_request.development_generator_request.as_ref().ok_or(
        K2CompositionErrorV1::Invalid("self_formed_development_owner_generator_request_missing"),
    )?;
    generator_request.validate()?;
    let request_root = generator_request.request_root_sha256.clone();
    let mut request_bytes = uncertainty_bytes_v1(generator_request)?;
    let dispatch_root = dispatch_binding_root_v1(
        &request_root,
        &owner_request.descriptor.generator_executable_sha256,
    )?;
    journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched,
        owner_executable_sha256,
        owner_request.request_root_sha256.clone(),
        dispatch_root,
    )?;
    let (response_bytes, pipe_receipt) = dispatch_self_formed_generator_once_v1(
        generator_executable,
        &owner_request.descriptor.generator_executable_sha256,
        &request_root,
        &mut request_bytes,
        None,
    )?;
    let response: K2UncertaintyGeneratorResponseV1 = uncertainty_decode_v1(&response_bytes)?;
    response.validate()?;
    if response.generator_request_root_sha256 != request_root
        || response.public.experiment_id_sha256 != owner_request.descriptor.experiment_id_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_owner_response_binding_invalid",
        ));
    }
    let event = journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated,
        owner_request
            .descriptor
            .confirm_owner_executable_sha256
            .clone(),
        owner_request.request_root_sha256.clone(),
        response.response_root_sha256.clone(),
    )?;
    K2UncertaintyConfirmOwnerReceiptV1::seal(
        owner_request,
        response.response_root_sha256,
        response.public.public_batch_root_sha256,
        response.private.private_batch_root_sha256,
        request_root,
        None,
        None,
        pipe_receipt,
        event.event_root_sha256,
        journal.projection().generator_dispatch_count,
    )
}

fn execute_confirm_v1(
    owner_request: &K2UncertaintyConfirmOwnerRequestV1,
    attempt_root: &Path,
    generator_executable: &Path,
    owner_executable_sha256: String,
    journal: &mut K2UncertaintyConfirmAttemptJournalV1,
) -> K2CompositionResultV1<K2UncertaintyConfirmOwnerReceiptV1> {
    let mut nonce = SecretArray32V1([0; 32]);
    File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut nonce.0))
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_confirm_csprng"))?;
    let nonce_receipt = persist_retained_confirm_nonce_v1(attempt_root, &nonce.0)?;
    journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::NonceCreated,
        owner_executable_sha256.clone(),
        owner_request.request_root_sha256.clone(),
        nonce_receipt.receipt_root_sha256.clone(),
    )?;
    journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::NonceCommitted,
        owner_executable_sha256.clone(),
        owner_request.request_root_sha256.clone(),
        nonce_receipt.nonce_commitment_sha256.clone(),
    )?;

    let authorization_root = owner_request
        .authorization_receipt
        .as_ref()
        .map(|receipt| receipt.receipt_root_sha256.clone())
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_owner_authorization_missing",
        ))?;
    let mut generator_request =
        SecretConfirmRequestV1(K2UncertaintyConfirmGeneratorRequestV1::seal(
            nonce.0.to_vec(),
            owner_request
                .descriptor
                .successor_freeze_root_sha256
                .clone(),
            authorization_root,
            owner_request.descriptor.generator_executable_sha256.clone(),
        )?);
    let request_root = generator_request.0.request_root_sha256.clone();
    let mut request_bytes = uncertainty_bytes_v1(&generator_request.0)?;
    let dispatch_root = dispatch_binding_root_v1(
        &request_root,
        &owner_request.descriptor.generator_executable_sha256,
    )?;
    journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched,
        owner_executable_sha256,
        owner_request.request_root_sha256.clone(),
        dispatch_root,
    )?;
    let (response_bytes, pipe_receipt) = dispatch_self_formed_generator_once_v1(
        generator_executable,
        &owner_request.descriptor.generator_executable_sha256,
        &request_root,
        &mut request_bytes,
        Some(&nonce.0),
    )?;
    let response: K2UncertaintyConfirmGeneratorResponseV1 = uncertainty_decode_v1(&response_bytes)?;
    response.validate()?;
    if response.generator_request_root_sha256 != request_root {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_owner_response_binding_invalid",
        ));
    }
    let split = publish_confirm_generator_split_v1(
        &attempt_root.join("generated"),
        &generator_request.0,
        &response,
    )?;
    wipe_bytes_v1(&mut generator_request.0.nonce_bytes);
    let event = journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated,
        owner_request
            .descriptor
            .confirm_owner_executable_sha256
            .clone(),
        owner_request.request_root_sha256.clone(),
        split.split_receipt_root_sha256.clone(),
    )?;
    K2UncertaintyConfirmOwnerReceiptV1::seal(
        owner_request,
        response.response_root_sha256,
        response.public.public_batch_root_sha256,
        response.private.private_batch_root_sha256,
        request_root,
        Some(split.split_receipt_root_sha256),
        Some(nonce_receipt.nonce_commitment_sha256),
        pipe_receipt,
        event.event_root_sha256,
        journal.projection().generator_dispatch_count,
    )
}

pub fn dispatch_self_formed_generator_once_v1(
    generator_executable: &Path,
    expected_generator_sha256: &str,
    generator_request_root_sha256: &str,
    request_bytes: &mut [u8],
    forbidden_secret: Option<&[u8]>,
) -> K2CompositionResultV1<(Vec<u8>, K2UncertaintyConfirmPipeReceiptV1)> {
    let actual_generator_sha256 = composition_sha256_file_v1(generator_executable)?;
    if actual_generator_sha256 != expected_generator_sha256 {
        wipe_bytes_v1(request_bytes);
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_pipe_generator_mismatch",
        ));
    }
    let request_len = request_bytes.len() as u64;
    let mut command = generator_sandbox_command_v1(generator_executable);
    if let Some(secret) = forbidden_secret
        && let Err(error) =
            require_secret_absent_from_command_v1(&command, generator_executable, secret)
    {
        wipe_bytes_v1(request_bytes);
        return Err(error);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            wipe_bytes_v1(request_bytes);
            return Err(K2CompositionErrorV1::Process(
                "spawn_self_formed_confirm_generator",
            ));
        }
    };
    let write_result = child
        .stdin
        .take()
        .ok_or(K2CompositionErrorV1::Process(
            "open_self_formed_confirm_generator_stdin",
        ))
        .and_then(|mut stdin| {
            stdin
                .write_all(request_bytes)
                .map_err(|_| K2CompositionErrorV1::Process("write_self_formed_confirm_generator"))
        });
    wipe_bytes_v1(request_bytes);
    if let Err(error) = write_result {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            terminate_child_v1(&mut child);
            return Err(K2CompositionErrorV1::Process(
                "open_self_formed_confirm_generator_stdout",
            ));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            terminate_child_v1(&mut child);
            return Err(K2CompositionErrorV1::Process(
                "open_self_formed_confirm_generator_stderr",
            ));
        }
    };
    let stdout_reader = thread::spawn(move || {
        read_bounded_stdout_v1(stdout, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1)
    });
    let stderr_reader =
        thread::spawn(move || read_bounded_stderr_v1(stderr, MAX_GENERATOR_STDERR_BYTES_V1 + 1));
    let deadline = Instant::now() + Duration::from_secs(GENERATOR_TIMEOUT_SECONDS_V1);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(_) => {
                terminate_child_v1(&mut child);
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(K2CompositionErrorV1::Process(
                    "poll_self_formed_confirm_generator",
                ));
            }
        }
        if Instant::now() >= deadline {
            terminate_child_v1(&mut child);
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(K2CompositionErrorV1::Process(
                "self_formed_confirm_generator_timeout",
            ));
        }
        thread::sleep(Duration::from_millis(5));
    };
    let response_bytes = stdout_reader
        .join()
        .map_err(|_| K2CompositionErrorV1::Process("join_self_formed_confirm_stdout"))??;
    let stderr_bytes = stderr_reader
        .join()
        .map_err(|_| K2CompositionErrorV1::Process("join_self_formed_confirm_stderr"))??;
    if forbidden_secret.is_some_and(|secret| {
        contains_subslice_v1(&response_bytes, secret) || contains_subslice_v1(&stderr_bytes, secret)
    }) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_generator_output_leaked_nonce",
        ));
    }
    if !status.success()
        || response_bytes.is_empty()
        || response_bytes.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
        || stderr_bytes.len() > MAX_GENERATOR_STDERR_BYTES_V1
    {
        return Err(K2CompositionErrorV1::Process(
            "self_formed_confirm_generator_failed",
        ));
    }
    let receipt = K2UncertaintyConfirmPipeReceiptV1::seal(
        actual_generator_sha256,
        generator_request_root_sha256.to_owned(),
        request_len,
        response_bytes.len() as u64,
        composition_sha256_bytes_v1(&response_bytes),
    )?;
    Ok((response_bytes, receipt))
}

pub fn run_self_formed_confirm_owner_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_confirm_owner_stdin"))?;
    let request: K2UncertaintyConfirmOwnerRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_confirm_owner"))?;
    let receipt = execute_self_formed_confirm_owner_v1(&request, &executable)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_confirm_owner_stdout"))
}

fn validate_authority_binding_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    lab_root: &Path,
) -> K2CompositionResultV1<()> {
    if request.descriptor.mode == K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal {
        return Ok(());
    }
    let authorization =
        request
            .authorization_receipt
            .as_ref()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_owner_authorization_missing",
            ))?;
    let supplied_claim =
        request
            .authorization_slot_claim
            .as_ref()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_owner_claim_missing",
            ))?;
    let ledger_relative =
        request
            .slot_ledger_relative_path
            .as_deref()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_confirm_owner_ledger_missing",
            ))?;
    let ledger =
        K2UncertaintyAuthorizationSlotLedgerV1::open_existing(&lab_root.join(ledger_relative))?;
    let durable_claim = ledger.read_slot_claim(&authorization.slot_key()?)?;
    if &durable_claim != supplied_claim {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_owner_durable_claim_mismatch",
        ));
    }
    Ok(())
}

fn canonical_private_lab_root_v1(value: &str) -> K2CompositionResultV1<PathBuf> {
    let requested = PathBuf::from(value);
    let link_metadata = fs::symlink_metadata(&requested)
        .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_confirm_lab_root"))?;
    let root = fs::canonicalize(value)
        .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_confirm_lab_root"))?;
    let metadata = fs::metadata(&root)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_confirm_lab_root"))?;
    if link_metadata.file_type().is_symlink()
        || requested != root
        || !metadata.is_dir()
        || metadata.permissions().mode() & 0o777 != 0o700
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_lab_root_mode_invalid",
        ));
    }
    Ok(root)
}

fn dispatch_binding_root_v1(
    request_root_sha256: &str,
    generator_executable_sha256: &str,
) -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&(
        "nando.k2-self-formed-confirm-generator-dispatch.v1",
        request_root_sha256,
        generator_executable_sha256,
    ))
}

fn generator_sandbox_command_v1(generator_executable: &Path) -> Command {
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
        generator_executable.as_os_str().to_owned(),
        OsString::from(GUEST_GENERATOR_PATH_V1),
        OsString::from("--setenv"),
        OsString::from("HOME"),
        OsString::from("/tmp"),
        OsString::from("--setenv"),
        OsString::from("LANG"),
        OsString::from("C"),
        OsString::from("--"),
        OsString::from(PRLIMIT_PATH_V1),
        OsString::from(format!(
            "--cpu={GENERATOR_CPU_SECONDS_V1}:{GENERATOR_CPU_SECONDS_V1}"
        )),
        OsString::from("--as=536870912:536870912"),
        OsString::from("--nproc=32:32"),
        OsString::from("--fsize=33554432:33554432"),
        OsString::from("--"),
        OsString::from(GUEST_GENERATOR_PATH_V1),
    ];
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn require_secret_absent_from_command_v1(
    command: &Command,
    generator_executable: &Path,
    secret: &[u8],
) -> K2CompositionResultV1<()> {
    let argv_match = contains_subslice_v1(command.get_program().as_bytes(), secret)
        || command
            .get_args()
            .any(|argument| contains_subslice_v1(argument.as_bytes(), secret));
    let environment_match = command.get_envs().any(|(key, value)| {
        contains_subslice_v1(key.as_bytes(), secret)
            || value.is_some_and(|value| contains_subslice_v1(value.as_bytes(), secret))
    });
    let path_match = contains_subslice_v1(generator_executable.as_os_str().as_bytes(), secret)
        || contains_subslice_v1(BWRAP_PATH_V1.as_bytes(), secret)
        || contains_subslice_v1(GUEST_GENERATOR_PATH_V1.as_bytes(), secret);
    if argv_match || environment_match || path_match {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_confirm_nonce_in_forbidden_process_channel",
        ));
    }
    Ok(())
}

fn read_bounded_stdout_v1(stdout: ChildStdout, limit: usize) -> K2CompositionResultV1<Vec<u8>> {
    read_bounded_v1(stdout, limit, "read_self_formed_confirm_generator_stdout")
}

fn read_bounded_stderr_v1(stderr: ChildStderr, limit: usize) -> K2CompositionResultV1<Vec<u8>> {
    read_bounded_v1(stderr, limit, "read_self_formed_confirm_generator_stderr")
}

fn read_bounded_v1<R: Read>(
    reader: R,
    limit: usize,
    reason: &'static str,
) -> K2CompositionResultV1<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| K2CompositionErrorV1::Process(reason))?;
    Ok(bytes)
}

fn terminate_child_v1(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn contains_subslice_v1(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

fn wipe_bytes_v1(bytes: &mut [u8]) {
    bytes.fill(0);
    std::hint::black_box(&mut *bytes);
    compiler_fence(Ordering::SeqCst);
}

struct SecretArray32V1([u8; 32]);

impl Drop for SecretArray32V1 {
    fn drop(&mut self) {
        wipe_bytes_v1(&mut self.0);
    }
}

struct SecretConfirmRequestV1(K2UncertaintyConfirmGeneratorRequestV1);

impl Drop for SecretConfirmRequestV1 {
    fn drop(&mut self) {
        wipe_bytes_v1(&mut self.0.nonce_bytes);
    }
}
