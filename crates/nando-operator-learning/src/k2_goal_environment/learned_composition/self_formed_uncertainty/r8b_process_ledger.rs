use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::r8b_process_authorizer::validate_self_formed_r8b_process_event_v3;
use super::{
    K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1, K2_UNCERTAINTY_R8B_LEDGER_HEADER_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_LEDGER_SEAL_SCHEMA_V3, K2UncertaintyImmutablePublicationFaultV1,
    K2UncertaintyR8BCompletionKindV3, K2UncertaintyR8BExecutableIdentityV2,
    K2UncertaintyR8BInputRoleV3, K2UncertaintyR8BInvocationPlanV3, K2UncertaintyR8BLedgerHeaderV3,
    K2UncertaintyR8BLedgerSealV3, K2UncertaintyR8BLedgerSummaryV3,
    K2UncertaintyR8BOutputContractV3, K2UncertaintyR8BProcessEventKindV2,
    K2UncertaintyR8BProcessEventV2, K2UncertaintyR8BProcessEventV3,
    K2UncertaintyR8BProcessLedgerV2, K2UncertaintyR8BProducedReceiptV2,
    K2UncertaintyR8BProducerRequestV3, K2UncertaintyR8BScheduleAuthorityV3,
    K2UncertaintyR8BValidatedFactV3, K2UncertaintyR8BValidatedOutputV3, append_canonical_jsonl_v1,
    canonical_jsonl_line_v1, create_exclusive_file_v1, decode_canonical_json_v1,
    open_nofollow_file_v1, publish_immutable_file_v1, read_bounded_jsonl_line_v1,
    read_immutable_file_v1, recover_renamed_file_v1, rename_noreplace_same_device_v1,
    require_private_directory_v1, seal_self_formed_r8b_process_event_v3, sync_directory_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1, validate_regular_file_v1,
    validate_self_formed_r8b_schedule_authority_v3,
};

pub const K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2: &str = "NANDO_R8B_LEDGER_ROOT_V2";
pub const K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2: &str = "NANDO_R8B_ROUTE_ID_V2";
pub const K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_ENV_V3: &str = "NANDO_R8B_PRODUCER_REQUEST_V3";
pub const K2_UNCERTAINTY_R8B_MAX_LEDGER_EVENTS_V3: u64 = 33_336;
pub const K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3: u64 = 134_217_728;
const MAX_LEDGER_LINE_BYTES_V3: usize = 4_096;
const OPEN_LEDGER_V3: &str = "process-ledger.open.jsonl";
const LEDGER_LOCK_V3: &str = "process-ledger.lock";
pub fn seal_self_formed_r8b_ledger_header_v3(
    route_id_sha256: String,
    expected_projection_root_sha256: String,
    schedule_authority: K2UncertaintyR8BScheduleAuthorityV3,
) -> K2CompositionResultV1<K2UncertaintyR8BLedgerHeaderV3> {
    let mut header = K2UncertaintyR8BLedgerHeaderV3 {
        schema: K2_UNCERTAINTY_R8B_LEDGER_HEADER_SCHEMA_V3.to_owned(),
        route_id_sha256,
        expected_projection_root_sha256,
        schedule_authority,
        header_root_sha256: String::new(),
    };
    header.header_root_sha256 = uncertainty_root_v1(&header)?;
    validate_self_formed_r8b_ledger_header_v3(&header)?;
    Ok(header)
}

pub fn seal_self_formed_r8b_ledger_seal_v3(
    route_id_sha256: String,
    event_count: u64,
    final_event_root_sha256: String,
) -> K2CompositionResultV1<K2UncertaintyR8BLedgerSealV3> {
    let mut seal = K2UncertaintyR8BLedgerSealV3 {
        schema: K2_UNCERTAINTY_R8B_LEDGER_SEAL_SCHEMA_V3.to_owned(),
        route_id_sha256,
        event_count,
        final_event_root_sha256,
        seal_root_sha256: String::new(),
    };
    seal.seal_root_sha256 = uncertainty_root_v1(&seal)?;
    validate_self_formed_r8b_ledger_seal_v3(&seal)?;
    Ok(seal)
}

pub fn validate_self_formed_r8b_ledger_header_v3(
    header: &K2UncertaintyR8BLedgerHeaderV3,
) -> K2CompositionResultV1<()> {
    require_composition_root_v1(&header.route_id_sha256)?;
    require_composition_root_v1(&header.expected_projection_root_sha256)?;
    validate_self_formed_r8b_schedule_authority_v3(&header.schedule_authority)?;
    let mut canonical = header.clone();
    canonical.header_root_sha256.clear();
    if header.schema != K2_UNCERTAINTY_R8B_LEDGER_HEADER_SCHEMA_V3
        || header.header_root_sha256 != uncertainty_root_v1(&canonical)?
    {
        return Err(invalid_v1("self_formed_r8b_v3_ledger_header_invalid"));
    }
    Ok(())
}

pub fn validate_self_formed_r8b_ledger_seal_v3(
    seal: &K2UncertaintyR8BLedgerSealV3,
) -> K2CompositionResultV1<()> {
    require_composition_root_v1(&seal.route_id_sha256)?;
    require_composition_root_v1(&seal.final_event_root_sha256)?;
    let mut canonical = seal.clone();
    canonical.seal_root_sha256.clear();
    if seal.schema != K2_UNCERTAINTY_R8B_LEDGER_SEAL_SCHEMA_V3
        || seal.event_count > K2_UNCERTAINTY_R8B_MAX_LEDGER_EVENTS_V3
        || seal.seal_root_sha256 != uncertainty_root_v1(&canonical)?
    {
        return Err(invalid_v1("self_formed_r8b_v3_ledger_seal_invalid"));
    }
    Ok(())
}

pub fn validate_self_formed_r8b_ledger_stream_v3<R: BufRead>(
    reader: R,
    require_seal: bool,
) -> K2CompositionResultV1<K2UncertaintyR8BLedgerSummaryV3> {
    validate_self_formed_r8b_ledger_stream_attested_v3(reader, require_seal)
        .map(|(summary, _, _)| summary)
}

pub fn validate_self_formed_r8b_ledger_stream_attested_v3<R: BufRead>(
    mut reader: R,
    require_seal: bool,
) -> K2CompositionResultV1<(K2UncertaintyR8BLedgerSummaryV3, u64, String)> {
    let mut total_bytes = 0_u64;
    let mut stream_sha256 = Sha256::new();
    let header_line = read_bounded_jsonl_line_v1(
        &mut reader,
        &mut total_bytes,
        MAX_LEDGER_LINE_BYTES_V3,
        K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3,
    )?
    .ok_or_else(|| invalid_v1("self_formed_r8b_v3_ledger_header_missing"))?;
    stream_sha256.update(&header_line);
    stream_sha256.update(b"\n");
    let header: K2UncertaintyR8BLedgerHeaderV3 = decode_canonical_json_v1(&header_line)?;
    validate_self_formed_r8b_ledger_header_v3(&header)?;
    let mut previous = header.header_root_sha256.clone();
    let mut sequence = 0_u64;
    let mut seen = BTreeSet::new();
    let mut open = BTreeMap::<String, K2UncertaintyR8BProcessEventV3>::new();
    let mut m16_events = BTreeSet::new();
    let mut m16_receipts = BTreeSet::new();
    let mut m17_events = BTreeSet::new();
    let mut m17_receipts = BTreeSet::new();
    let mut invocations = Vec::new();
    let mut request_roots = BTreeMap::new();
    let mut representative_counts = BTreeMap::new();
    let mut authority_outputs = Vec::new();
    let mut seal = None;
    let mut fail_stopped = false;
    while let Some(line) = read_bounded_jsonl_line_v1(
        &mut reader,
        &mut total_bytes,
        MAX_LEDGER_LINE_BYTES_V3,
        K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3,
    )? {
        stream_sha256.update(&line);
        stream_sha256.update(b"\n");
        let value: serde_json::Value = uncertainty_decode_v1(&line)?;
        let schema = value
            .get("schema")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_v1("self_formed_r8b_v3_ledger_schema_missing"))?;
        if schema == K2_UNCERTAINTY_R8B_LEDGER_SEAL_SCHEMA_V3 {
            let candidate: K2UncertaintyR8BLedgerSealV3 = decode_canonical_json_v1(&line)?;
            validate_self_formed_r8b_ledger_seal_v3(&candidate)?;
            if seal.is_some()
                || candidate.route_id_sha256 != header.route_id_sha256
                || candidate.event_count != sequence
                || candidate.final_event_root_sha256 != previous
                || !open.is_empty()
            {
                return Err(invalid_v1("self_formed_r8b_v3_ledger_seal_invalid"));
            }
            seal = Some(candidate);
            continue;
        }
        if seal.is_some() || fail_stopped {
            return Err(invalid_v1("self_formed_r8b_v3_ledger_after_terminal"));
        }
        let event: K2UncertaintyR8BProcessEventV3 = decode_canonical_json_v1(&line)?;
        validate_self_formed_r8b_process_event_v3(&event)?;
        if event.sequence != sequence
            || event.previous_event_root_sha256 != previous
            || event.route_id_sha256 != header.route_id_sha256
        {
            return Err(invalid_v1("self_formed_r8b_v3_ledger_chain_invalid"));
        }
        let id = event.invocation.invocation_id_sha256.clone();
        if event.completion.is_none() {
            if !seen.insert(id.clone()) || open.insert(id.clone(), event.clone()).is_some() {
                return Err(invalid_v1("self_formed_r8b_v3_ledger_request_duplicate"));
            }
            invocations.push(event.invocation.clone());
            request_roots.insert(id, event.request_root_sha256.clone());
        } else {
            let started = open
                .remove(&id)
                .ok_or_else(|| invalid_v1("self_formed_r8b_v3_ledger_request_missing"))?;
            if event.started_event_root_sha256.as_ref() != Some(&started.event_root_sha256)
                || event.invocation != started.invocation
                || event.request_root_sha256 != started.request_root_sha256
                || event.stdin_sha256 != started.stdin_sha256
                || event.monotonic_ns < started.monotonic_ns
            {
                return Err(invalid_v1("self_formed_r8b_v3_ledger_completion_mismatch"));
            }
            if event.completion == Some(K2UncertaintyR8BCompletionKindV3::AuthoritySuccess) {
                let receipt = event
                    .validated_output
                    .as_ref()
                    .ok_or_else(|| invalid_v1("self_formed_r8b_v3_ledger_output_missing"))?;
                match event.invocation.target_role.as_str() {
                    "M04_PROBE" => {
                        let case = event
                            .invocation
                            .case_id_sha256
                            .as_ref()
                            .ok_or_else(|| invalid_v1("self_formed_r8b_v3_m04_case_missing"))?;
                        let count = match receipt.fact {
                            super::K2UncertaintyR8BValidatedFactV3::RepresentativeCount {
                                count,
                            } => count,
                            _ => return Err(invalid_v1("self_formed_r8b_v3_m04_fact_missing")),
                        };
                        if representative_counts.insert(case.clone(), count).is_some() {
                            return Err(invalid_v1("self_formed_r8b_v3_m04_fact_duplicate"));
                        }
                    }
                    "M16_ORACLE" if event.invocation.request_owner_role == "M24_LINKED_RUNNER" => {
                        if !m16_events.insert(event.event_root_sha256.clone())
                            || !m16_receipts.insert(receipt.semantic_root_sha256.clone())
                        {
                            return Err(invalid_v1("self_formed_r8b_v3_m16_root_duplicate"));
                        }
                    }
                    "M17_CONTROL_EVALUATOR"
                        if event.invocation.request_owner_role == "M24_LINKED_RUNNER" =>
                    {
                        if !m17_events.insert(event.event_root_sha256.clone())
                            || !m17_receipts.insert(receipt.semantic_root_sha256.clone())
                        {
                            return Err(invalid_v1("self_formed_r8b_v3_m17_root_duplicate"));
                        }
                    }
                    _ => {}
                }
                authority_outputs.extend(
                    receipt
                        .authority_outputs
                        .iter()
                        .cloned()
                        .map(|output| (event.event_root_sha256.clone(), output)),
                );
            }
            fail_stopped = matches!(
                event.completion,
                Some(
                    K2UncertaintyR8BCompletionKindV3::UnexpectedFailure
                        | K2UncertaintyR8BCompletionKindV3::LaunchFailure
                )
            );
        }
        previous = event.event_root_sha256;
        sequence += 1;
        if sequence > K2_UNCERTAINTY_R8B_MAX_LEDGER_EVENTS_V3 {
            return Err(invalid_v1("self_formed_r8b_v3_ledger_event_limit"));
        }
    }
    if require_seal != seal.is_some() {
        return Err(invalid_v1(
            "self_formed_r8b_v3_ledger_terminal_state_invalid",
        ));
    }
    let summary = K2UncertaintyR8BLedgerSummaryV3 {
        route_id_sha256: header.route_id_sha256,
        expected_projection_root_sha256: header.expected_projection_root_sha256,
        schedule_authority: header.schedule_authority,
        event_count: sequence,
        final_event_root_sha256: previous,
        seal_root_sha256: seal.map(|value| value.seal_root_sha256),
        invocations,
        request_roots_sha256: request_roots,
        representative_counts,
        authority_outputs,
        open_invocations: open.len() as u64,
        fail_stopped,
        m16_event_roots_sha256: m16_events,
        m16_receipt_roots_sha256: m16_receipts,
        m17_event_roots_sha256: m17_events,
        m17_receipt_roots_sha256: m17_receipts,
    };
    Ok((
        summary,
        total_bytes,
        format!("{:x}", stream_sha256.finalize()),
    ))
}

#[derive(Debug)]
pub struct K2UncertaintyR8BLedgerWriterV3 {
    staging_root: PathBuf,
    route_id_sha256: String,
    expected_projection_root_sha256: String,
    schedule_authority: K2UncertaintyR8BScheduleAuthorityV3,
}

impl K2UncertaintyR8BLedgerWriterV3 {
    pub fn create(
        staging_root: &Path,
        route_id_sha256: String,
        expected_projection_root_sha256: String,
        schedule_authority: K2UncertaintyR8BScheduleAuthorityV3,
    ) -> K2CompositionResultV1<Self> {
        require_private_directory_v1(staging_root)?;
        let writer = Self {
            staging_root: fs::canonicalize(staging_root)
                .map_err(|_| invalid_v1("self_formed_r8b_v3_staging_root_invalid"))?,
            route_id_sha256,
            expected_projection_root_sha256,
            schedule_authority,
        };
        let header = seal_self_formed_r8b_ledger_header_v3(
            writer.route_id_sha256.clone(),
            writer.expected_projection_root_sha256.clone(),
            writer.schedule_authority.clone(),
        )?;
        create_exclusive_file_v1(&writer.path(LEDGER_LOCK_V3), &[], 0o600)?;
        create_exclusive_file_v1(
            &writer.path(OPEN_LEDGER_V3),
            &canonical_jsonl_line_v1(&header, MAX_LEDGER_LINE_BYTES_V3)?,
            0o600,
        )?;
        sync_directory_v1(&writer.staging_root)?;
        Ok(writer)
    }

    pub fn attach(staging_root: &Path) -> K2CompositionResultV1<Self> {
        require_private_directory_v1(staging_root)?;
        let staging_root = fs::canonicalize(staging_root)
            .map_err(|_| invalid_v1("self_formed_r8b_v3_staging_root_invalid"))?;
        let ledger = open_nofollow_file_v1(&staging_root.join(OPEN_LEDGER_V3), true, true)?;
        validate_regular_file_v1(&ledger, 0o600, K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3)?;
        let summary = validate_self_formed_r8b_ledger_stream_v3(BufReader::new(ledger), false)?;
        Ok(Self {
            staging_root,
            route_id_sha256: summary.route_id_sha256,
            expected_projection_root_sha256: summary.expected_projection_root_sha256,
            schedule_authority: summary.schedule_authority,
        })
    }

    pub fn attach_request(
        request: &K2UncertaintyR8BProducerRequestV3,
    ) -> K2CompositionResultV1<Self> {
        let ledger = request
            .inputs
            .iter()
            .find(|input| input.role == K2UncertaintyR8BInputRoleV3::ProcessLedger)
            .ok_or_else(|| invalid_v1("self_formed_r8b_v3_ledger_input_missing"))?;
        let writer = Self::attach(
            Path::new(&ledger.canonical_path)
                .parent()
                .ok_or_else(|| invalid_v1("self_formed_r8b_v3_ledger_input_invalid"))?,
        )?;
        if writer.route_id_sha256 != request.route_id_sha256 {
            return Err(invalid_v1(
                "self_formed_r8b_v3_ledger_request_route_invalid",
            ));
        }
        Ok(writer)
    }

    pub fn summary(&self) -> K2CompositionResultV1<K2UncertaintyR8BLedgerSummaryV3> {
        let ledger = open_nofollow_file_v1(&self.path(OPEN_LEDGER_V3), true, true)?;
        validate_regular_file_v1(&ledger, 0o600, K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3)?;
        validate_self_formed_r8b_ledger_stream_v3(BufReader::new(ledger), false)
    }

    pub fn request(
        &self,
        invocation: K2UncertaintyR8BInvocationPlanV3,
        request_root_sha256: String,
        stdin_sha256: String,
        monotonic_ns: u64,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV3> {
        self.append(K2UncertaintyR8BProcessEventV3 {
            schema: String::new(),
            sequence: 0,
            previous_event_root_sha256: String::new(),
            route_id_sha256: String::new(),
            invocation,
            request_root_sha256,
            stdin_sha256,
            started_event_root_sha256: None,
            completion: None,
            exit_code: None,
            stdout_byte_len: None,
            stdout_sha256: None,
            stderr_byte_len: None,
            stderr_sha256: None,
            validated_output: None,
            monotonic_ns,
            event_root_sha256: String::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn success(
        &self,
        started: &K2UncertaintyR8BProcessEventV3,
        stdout: &[u8],
        stderr: &[u8],
        receipt_schema: String,
        semantic_root_sha256: String,
        fact: K2UncertaintyR8BValidatedFactV3,
        authority_outputs: Vec<K2UncertaintyR8BOutputContractV3>,
        monotonic_ns: u64,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV3> {
        let stdout_sha256 = composition_sha256_bytes_v1(stdout);
        self.append(K2UncertaintyR8BProcessEventV3 {
            schema: String::new(),
            sequence: 0,
            previous_event_root_sha256: String::new(),
            route_id_sha256: String::new(),
            invocation: started.invocation.clone(),
            request_root_sha256: started.request_root_sha256.clone(),
            stdin_sha256: started.stdin_sha256.clone(),
            started_event_root_sha256: Some(started.event_root_sha256.clone()),
            completion: Some(K2UncertaintyR8BCompletionKindV3::AuthoritySuccess),
            exit_code: Some(0),
            stdout_byte_len: Some(stdout.len() as u64),
            stdout_sha256: Some(stdout_sha256.clone()),
            stderr_byte_len: Some(stderr.len() as u64),
            stderr_sha256: Some(composition_sha256_bytes_v1(stderr)),
            validated_output: Some(K2UncertaintyR8BValidatedOutputV3 {
                stdout_byte_len: stdout.len() as u64,
                stdout_sha256,
                receipt_schema,
                semantic_root_sha256,
                validator: started.invocation.validator,
                validator_executable_sha256: started.invocation.target_executable_sha256.clone(),
                fact,
                authority_outputs,
            }),
            monotonic_ns,
            event_root_sha256: String::new(),
        })
    }

    pub fn failure(
        &self,
        started: &K2UncertaintyR8BProcessEventV3,
        completion: K2UncertaintyR8BCompletionKindV3,
        exit_code: i32,
        stdout: &[u8],
        stderr: &[u8],
        monotonic_ns: u64,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV3> {
        if completion == K2UncertaintyR8BCompletionKindV3::AuthoritySuccess {
            return Err(invalid_v1("self_formed_r8b_v3_failure_kind_invalid"));
        }
        self.append(K2UncertaintyR8BProcessEventV3 {
            schema: String::new(),
            sequence: 0,
            previous_event_root_sha256: String::new(),
            route_id_sha256: String::new(),
            invocation: started.invocation.clone(),
            request_root_sha256: started.request_root_sha256.clone(),
            stdin_sha256: started.stdin_sha256.clone(),
            started_event_root_sha256: Some(started.event_root_sha256.clone()),
            completion: Some(completion),
            exit_code: Some(exit_code),
            stdout_byte_len: Some(stdout.len() as u64),
            stdout_sha256: Some(composition_sha256_bytes_v1(stdout)),
            stderr_byte_len: Some(stderr.len() as u64),
            stderr_sha256: Some(composition_sha256_bytes_v1(stderr)),
            validated_output: None,
            monotonic_ns,
            event_root_sha256: String::new(),
        })
    }

    pub fn append(
        &self,
        mut event: K2UncertaintyR8BProcessEventV3,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV3> {
        let lock = open_nofollow_file_v1(&self.path(LEDGER_LOCK_V3), true, false)?;
        validate_regular_file_v1(&lock, 0o600, 0)?;
        lock.lock()
            .map_err(|_| invalid_v1("self_formed_r8b_v3_ledger_lock_failed"))?;
        let result = (|| {
            let mut ledger = open_nofollow_file_v1(&self.path(OPEN_LEDGER_V3), true, true)?;
            validate_regular_file_v1(&ledger, 0o600, K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3)?;
            let summary =
                validate_self_formed_r8b_ledger_stream_v3(
                    BufReader::new(ledger.try_clone().map_err(|_| {
                        K2CompositionErrorV1::Io("clone_self_formed_r8b_v3_ledger")
                    })?),
                    false,
                )?;
            if summary.route_id_sha256 != self.route_id_sha256
                || summary.expected_projection_root_sha256 != self.expected_projection_root_sha256
                || summary.schedule_authority != self.schedule_authority
                || summary.fail_stopped
            {
                return Err(invalid_v1("self_formed_r8b_v3_ledger_prefix_foreign"));
            }
            event.sequence = summary.event_count;
            event.previous_event_root_sha256 = summary.final_event_root_sha256;
            event.route_id_sha256 = self.route_id_sha256.clone();
            event = seal_self_formed_r8b_process_event_v3(event)?;
            append_canonical_jsonl_v1(&mut ledger, &event, MAX_LEDGER_LINE_BYTES_V3)?;
            sync_directory_v1(&self.staging_root)?;
            Ok(event)
        })();
        let _ = lock.unlock();
        result
    }

    pub fn freeze(
        &self,
        destination: &Path,
    ) -> K2CompositionResultV1<K2UncertaintyR8BLedgerSummaryV3> {
        let parent = destination
            .parent()
            .ok_or_else(|| invalid_v1("self_formed_r8b_v3_ledger_destination_invalid"))?;
        let lock = open_nofollow_file_v1(&self.path(LEDGER_LOCK_V3), true, false)?;
        validate_regular_file_v1(&lock, 0o600, 0)?;
        lock.lock()
            .map_err(|_| invalid_v1("self_formed_r8b_v3_ledger_lock_failed"))?;
        let result = (|| {
            match fs::symlink_metadata(self.path(OPEN_LEDGER_V3)) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return self.recover_renamed_ledger(destination, parent);
                }
                Err(_) => {
                    return Err(K2CompositionErrorV1::Io(
                        "stat_self_formed_r8b_v3_open_ledger",
                    ));
                }
            }
            let mut ledger = open_nofollow_file_v1(&self.path(OPEN_LEDGER_V3), true, true)?;
            validate_regular_file_v1(&ledger, 0o600, K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3)?;
            let (summary, already_sealed) = self.validate_freezable_open_ledger()?;
            if summary.route_id_sha256 != self.route_id_sha256
                || summary.expected_projection_root_sha256 != self.expected_projection_root_sha256
                || summary.schedule_authority != self.schedule_authority
                || summary.open_invocations != 0
                || summary.fail_stopped
            {
                return Err(invalid_v1("self_formed_r8b_v3_ledger_not_closable"));
            }
            if !already_sealed {
                let seal = seal_self_formed_r8b_ledger_seal_v3(
                    self.route_id_sha256.clone(),
                    summary.event_count,
                    summary.final_event_root_sha256,
                )?;
                append_canonical_jsonl_v1(&mut ledger, &seal, MAX_LEDGER_LINE_BYTES_V3)?;
            }
            rename_noreplace_same_device_v1(&self.path(OPEN_LEDGER_V3), destination)?;
            fs::set_permissions(destination, fs::Permissions::from_mode(0o400))
                .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_r8b_v3_ledger"))?;
            sync_directory_v1(parent)?;
            let frozen = open_nofollow_file_v1(destination, false, false)?;
            validate_regular_file_v1(&frozen, 0o400, K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3)?;
            validate_self_formed_r8b_ledger_stream_v3(BufReader::new(frozen), true)
        })();
        let _ = lock.unlock();
        result
    }

    fn validate_freezable_open_ledger(
        &self,
    ) -> K2CompositionResultV1<(K2UncertaintyR8BLedgerSummaryV3, bool)> {
        let open = || -> K2CompositionResultV1<File> {
            let file = open_nofollow_file_v1(&self.path(OPEN_LEDGER_V3), true, true)?;
            validate_regular_file_v1(&file, 0o600, K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3)?;
            Ok(file)
        };
        if let Ok(summary) =
            validate_self_formed_r8b_ledger_stream_v3(BufReader::new(open()?), false)
        {
            return Ok((summary, false));
        }
        validate_self_formed_r8b_ledger_stream_v3(BufReader::new(open()?), true)
            .map(|summary| (summary, true))
    }

    fn recover_renamed_ledger(
        &self,
        destination: &Path,
        parent: &Path,
    ) -> K2CompositionResultV1<K2UncertaintyR8BLedgerSummaryV3> {
        recover_renamed_file_v1(
            destination,
            parent,
            K2_UNCERTAINTY_R8B_MAX_LEDGER_BYTES_V3,
            |file| {
                let summary =
                    validate_self_formed_r8b_ledger_stream_v3(BufReader::new(file), true)?;
                if summary.route_id_sha256 != self.route_id_sha256
                    || summary.expected_projection_root_sha256
                        != self.expected_projection_root_sha256
                    || summary.schedule_authority != self.schedule_authority
                {
                    return Err(invalid_v1("self_formed_r8b_v3_renamed_ledger_foreign"));
                }
                Ok(summary)
            },
        )
    }

    fn path(&self, name: &str) -> PathBuf {
        self.staging_root.join(name)
    }
}
#[derive(Clone, Debug)]
pub struct K2UncertaintyR8BLedgerWriterV2 {
    root: PathBuf,
    route_id_sha256: String,
    writer_role: String,
    writer_executable_sha256: String,
    allowed_children: BTreeMap<String, K2UncertaintyR8BExecutableIdentityV2>,
}

impl K2UncertaintyR8BLedgerWriterV2 {
    pub fn from_environment(
        writer_role: &str,
        writer_executable_sha256: String,
        allowed_children: Vec<K2UncertaintyR8BExecutableIdentityV2>,
    ) -> K2CompositionResultV1<Option<Self>> {
        let Some(root) = std::env::var_os(K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2) else {
            if std::env::var_os(K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2).is_some() {
                return Err(invalid_v1("self_formed_r8b_ledger_environment_partial"));
            }
            return Ok(None);
        };
        let route = std::env::var(K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2)
            .map_err(|_| invalid_v1("self_formed_r8b_ledger_environment_partial"))?;
        Self::new(
            PathBuf::from(root),
            route,
            writer_role,
            writer_executable_sha256,
            allowed_children,
        )
        .map(Some)
    }

    pub fn new(
        root: PathBuf,
        route_id_sha256: String,
        writer_role: &str,
        writer_executable_sha256: String,
        allowed_children: Vec<K2UncertaintyR8BExecutableIdentityV2>,
    ) -> K2CompositionResultV1<Self> {
        require_composition_root_v1(&route_id_sha256)?;
        require_composition_root_v1(&writer_executable_sha256)?;
        require_private_directory_v1(&root)?;
        if root
            != fs::canonicalize(&root)
                .map_err(|_| invalid_v1("self_formed_r8b_ledger_root_invalid"))?
            || writer_role.is_empty()
            || composition_sha256_file_v1(
                &std::env::current_exe()
                    .map_err(|_| invalid_v1("self_formed_r8b_writer_executable_missing"))?,
            )? != writer_executable_sha256
        {
            return Err(invalid_v1("self_formed_r8b_ledger_writer_invalid"));
        }
        let mut children = BTreeMap::new();
        for child in allowed_children {
            child.validate()?;
            if children.insert(child.role.clone(), child).is_some() {
                return Err(invalid_v1("self_formed_r8b_ledger_allowlist_invalid"));
            }
        }
        if children.is_empty() {
            return Err(invalid_v1("self_formed_r8b_ledger_allowlist_invalid"));
        }
        Ok(Self {
            root,
            route_id_sha256,
            writer_role: writer_role.to_owned(),
            writer_executable_sha256,
            allowed_children: children,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn child_started(
        &self,
        stage_id: &str,
        case_id_sha256: Option<String>,
        probe_ordinal: Option<u64>,
        child_role: &str,
        child_executable: &Path,
        request_root_sha256: String,
        stdin_sha256: String,
        started_monotonic_ns: u64,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV2> {
        let child = self.require_child(child_role, child_executable)?;
        self.append_with_lock(|prefix| {
            let mut event = K2UncertaintyR8BProcessEventV2 {
                schema: String::new(),
                sequence: prefix.events.len() as u64,
                previous_event_root_sha256: prefix
                    .events
                    .last()
                    .map(|value| value.event_root_sha256.clone()),
                kind: K2UncertaintyR8BProcessEventKindV2::ChildStarted,
                route_id_sha256: self.route_id_sha256.clone(),
                stage_id: stage_id.to_owned(),
                case_id_sha256,
                probe_ordinal,
                writer_role: self.writer_role.clone(),
                writer_executable_sha256: self.writer_executable_sha256.clone(),
                role: child.role.clone(),
                executable_sha256: child.sha256.clone(),
                request_root_sha256,
                stdin_sha256,
                started_event_root_sha256: None,
                normal_exit: None,
                exit_code: None,
                stdout_byte_len: None,
                stdout_sha256: None,
                produced_receipts: Vec::new(),
                stderr_byte_len: None,
                stderr_sha256: None,
                started_monotonic_ns,
                finished_monotonic_ns: None,
                event_root_sha256: String::new(),
            };
            event.reseal()?;
            Ok(event)
        })
    }

    pub fn child_finished(
        &self,
        start: &K2UncertaintyR8BProcessEventV2,
        stdout: &[u8],
        stderr: &[u8],
        produced_receipts: Vec<K2UncertaintyR8BProducedReceiptV2>,
        finished_monotonic_ns: u64,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV2> {
        self.append_with_lock(|prefix| {
            let observed_start = prefix
                .events
                .iter()
                .find(|event| event.event_root_sha256 == start.event_root_sha256);
            if observed_start != Some(start)
                || prefix.events.iter().any(|event| {
                    event.started_event_root_sha256.as_ref() == Some(&start.event_root_sha256)
                })
            {
                return Err(invalid_v1("self_formed_r8b_ledger_finish_start_invalid"));
            }
            let mut event = start.clone();
            event.sequence = prefix.events.len() as u64;
            event.previous_event_root_sha256 = prefix
                .events
                .last()
                .map(|value| value.event_root_sha256.clone());
            event.kind = K2UncertaintyR8BProcessEventKindV2::ChildFinished;
            event.started_event_root_sha256 = Some(start.event_root_sha256.clone());
            event.normal_exit = Some(true);
            event.exit_code = Some(0);
            event.stdout_byte_len = Some(stdout.len() as u64);
            event.stdout_sha256 = Some(composition_sha256_bytes_v1(stdout));
            event.produced_receipts = produced_receipts;
            event.stderr_byte_len = Some(stderr.len() as u64);
            event.stderr_sha256 = Some(composition_sha256_bytes_v1(stderr));
            event.finished_monotonic_ns = Some(finished_monotonic_ns);
            event.event_root_sha256.clear();
            event.reseal()?;
            Ok(event)
        })
    }

    pub fn complete_ledger(&self) -> K2CompositionResultV1<K2UncertaintyR8BProcessLedgerV2> {
        let directory =
            File::open(&self.root).map_err(|_| invalid_v1("self_formed_r8b_ledger_lock_open"))?;
        directory
            .lock()
            .map_err(|_| invalid_v1("self_formed_r8b_ledger_lock"))?;
        let result = read_ledger_prefix_v2(&self.root, &self.route_id_sha256).and_then(|prefix| {
            K2UncertaintyR8BProcessLedgerV2::seal(prefix.route_id_sha256, prefix.events)
        });
        let _ = directory.unlock();
        result
    }

    fn require_child<'a>(
        &'a self,
        role: &str,
        path: &Path,
    ) -> K2CompositionResultV1<&'a K2UncertaintyR8BExecutableIdentityV2> {
        let child = self
            .allowed_children
            .get(role)
            .ok_or_else(|| invalid_v1("self_formed_r8b_ledger_child_not_allowed"))?;
        let metadata = fs::symlink_metadata(path)
            .map_err(|_| invalid_v1("self_formed_r8b_ledger_child_missing"))?;
        if metadata.file_type().is_symlink()
            || path != Path::new(&child.canonical_path)
            || metadata.len() != child.byte_len
            || metadata.permissions().mode() & 0o7777 != child.unix_mode
            || composition_sha256_file_v1(path)? != child.sha256
        {
            return Err(invalid_v1("self_formed_r8b_ledger_child_identity_invalid"));
        }
        Ok(child)
    }

    fn append_with_lock(
        &self,
        build: impl FnOnce(
            &K2UncertaintyR8BProcessLedgerV2,
        ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV2>,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV2> {
        let directory =
            File::open(&self.root).map_err(|_| invalid_v1("self_formed_r8b_ledger_lock_open"))?;
        directory
            .lock()
            .map_err(|_| invalid_v1("self_formed_r8b_ledger_lock"))?;
        let result = (|| {
            let prefix = read_ledger_prefix_v2(&self.root, &self.route_id_sha256)?;
            let event = build(&prefix)?;
            let path = self.root.join(format!("{:08}.json", event.sequence));
            publish_immutable_file_v1(
                &self.root,
                path.file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| invalid_v1("self_formed_r8b_ledger_event_path_invalid"))?,
                &uncertainty_bytes_v1(&event)?,
                0o400,
                event.sequence,
                K2UncertaintyImmutablePublicationFaultV1::None,
            )?;
            sync_directory_v1(&self.root)?;
            Ok(event)
        })();
        let _ = directory.unlock();
        result
    }
}

fn read_ledger_prefix_v2(
    root: &Path,
    route_id_sha256: &str,
) -> K2CompositionResultV1<K2UncertaintyR8BProcessLedgerV2> {
    let mut paths = fs::read_dir(root)
        .map_err(|_| invalid_v1("self_formed_r8b_ledger_read"))?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid_v1("self_formed_r8b_ledger_entry_read"))?;
    paths.sort();
    let mut events = Vec::with_capacity(paths.len());
    for (sequence, path) in paths.into_iter().enumerate() {
        let expected = format!("{sequence:08}.json");
        if path.file_name().and_then(|value| value.to_str()) != Some(expected.as_str()) {
            return Err(invalid_v1("self_formed_r8b_ledger_natural_prefix_invalid"));
        }
        let file = read_immutable_file_v1(
            root,
            &expected,
            0o400,
            K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1,
        )?;
        let event: K2UncertaintyR8BProcessEventV2 = uncertainty_decode_v1(&file.bytes)?;
        if uncertainty_bytes_v1(&event)? != file.bytes || event.sequence != sequence as u64 {
            return Err(invalid_v1("self_formed_r8b_ledger_event_bytes_invalid"));
        }
        events.push(event);
    }
    K2UncertaintyR8BProcessLedgerV2::seal_natural_prefix(route_id_sha256.to_owned(), events)
}

fn invalid_v1(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
