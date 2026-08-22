use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;

use sha2::{Digest, Sha256};

use super::*;

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
                            K2UncertaintyR8BValidatedFactV3::RepresentativeCount { count } => count,
                            _ => return Err(invalid_v1("self_formed_r8b_v3_m04_fact_missing")),
                        };
                        if representative_counts.insert(case.clone(), count).is_some() {
                            return Err(invalid_v1("self_formed_r8b_v3_m04_fact_duplicate"));
                        }
                    }
                    "M16_ORACLE"
                        if event.invocation.request_owner_role == "M24_LINKED_RUNNER"
                            && (!m16_events.insert(event.event_root_sha256.clone())
                                || !m16_receipts.insert(receipt.semantic_root_sha256.clone())) =>
                    {
                        return Err(invalid_v1("self_formed_r8b_v3_m16_root_duplicate"));
                    }
                    "M17_CONTROL_EVALUATOR"
                        if event.invocation.request_owner_role == "M24_LINKED_RUNNER"
                            && (!m17_events.insert(event.event_root_sha256.clone())
                                || !m17_receipts.insert(receipt.semantic_root_sha256.clone())) =>
                    {
                        return Err(invalid_v1("self_formed_r8b_v3_m17_root_duplicate"));
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
