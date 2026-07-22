use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};

use nando_response_actor::{
    BindingCompletionStateV1, BindingFutureAcquisitionProtocolV1, BindingFutureCaptureBatchV1,
    BindingFutureCaptureInputV1, BindingSupportFreezeV1, CaptureCommitmentIndex,
    CaptureEvidenceReceipt, CaptureRecordCommitment, EVIDENCE_LEDGER_SCHEMA_V1,
    EvidenceIngestOutcome, EvidenceLedgerRecord, EvidencePolicyV1, PreActionBindingContextV1,
    RawEvidenceEnvelope, canonical_json_sha256, canonicalize_evidence_envelope,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FUTURE_ROWS: usize = 12;

#[derive(Clone)]
struct ParentState {
    marker: String,
    capability: String,
    phase: &'static str,
    rank: u64,
}

pub(super) fn main() -> Result<(), String> {
    require_pipe("/proc/self/fd/1", "future_batch_stdout_must_be_a_pipe")?;
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let protocol_path = args.next().ok_or_else(usage)?;
    let support_freeze_path = args.next().ok_or_else(usage)?;
    let extended_index_path = args.next().ok_or_else(usage)?;
    let acquisition_report_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage());
    }

    let protocol_bytes = fs::read(&protocol_path)
        .map_err(|error| format!("protocol_read:{}:{error}", protocol_path.display()))?;
    let protocol = BindingFutureAcquisitionProtocolV1::from_canonical_bytes(&protocol_bytes)
        .map_err(|error| format!("protocol_decode:{error:?}"))?;
    let support_freeze_bytes = fs::read(&support_freeze_path).map_err(|error| {
        format!(
            "support_freeze_read:{}:{error}",
            support_freeze_path.display()
        )
    })?;
    if sha256_bytes(&support_freeze_bytes) != protocol.support_freeze_file_sha256 {
        return Err("support_freeze_not_pinned_by_protocol".to_owned());
    }
    let support_freeze = BindingSupportFreezeV1::from_canonical_bytes(&support_freeze_bytes)
        .map_err(|error| format!("support_freeze_decode:{error:?}"))?;
    if support_freeze.receipt_sha256() != protocol.support_freeze_receipt_sha256
        || support_freeze.capture_index().index_sha256 != protocol.support_capture_index_sha256
        || support_freeze.watermark_next_sequence() != protocol.support_watermark_next_sequence
    {
        return Err("support_boundary_mismatch".to_owned());
    }

    let mut commitments = support_freeze.capture_index().records.clone();
    let mut previous_record_sha256 = commitments
        .last()
        .ok_or("support_capture_index_empty")?
        .record_sha256
        .clone();
    let mut intervention_replicates = BTreeMap::<String, usize>::new();
    let mut rows = Vec::with_capacity(FUTURE_ROWS);
    for (row_index, slot) in protocol.source.slots.iter().enumerate() {
        let replicate = intervention_replicates
            .entry(slot.intervention_id.clone())
            .or_default();
        let intervention = slot
            .intervention_id
            .strip_prefix('I')
            .ok_or("future_intervention_prefix_invalid")?
            .parse::<usize>()
            .map_err(|_| "future_intervention_id_invalid")?;
        let (payload, context) = future_case(intervention, *replicate, row_index % 4)?;
        *replicate += 1;
        let record = capture_record(
            row_index,
            &slot.session_slot,
            &payload,
            &previous_record_sha256,
            protocol.support_watermark_next_sequence,
        )?;
        previous_record_sha256 = record.record_sha256.clone();
        let commitment = CaptureRecordCommitment {
            sequence: record.sequence,
            record_sha256: record.record_sha256.clone(),
        };
        commitments.push(commitment.clone());
        rows.push(BindingFutureCaptureInputV1 {
            slot_id: slot.slot_id.clone(),
            capture_receipt: CaptureEvidenceReceipt::new(vec![commitment])
                .map_err(str::to_owned)?,
            capture_record: record,
            provider_payload: payload,
            context,
        });
    }

    let extended_index = CaptureCommitmentIndex::new(commitments).map_err(str::to_owned)?;
    let batch = BindingFutureCaptureBatchV1::new(&protocol, rows)
        .map_err(|error| format!("future_batch:{error:?}"))?;
    let batch_bytes = batch
        .canonical_bytes()
        .map_err(|error| format!("future_batch_encode:{error:?}"))?;
    let extended_index_bytes =
        serde_cbor::to_vec(&extended_index).map_err(|error| format!("index_encode:{error}"))?;
    let acquisition_report = json!({
        "schema": "nando.binding-future-acquisition-report.v1",
        "stage": "B1B-F",
        "source": protocol.source.source_kind,
        "protocol_receipt_sha256": protocol.receipt_sha256,
        "support_freeze_file_sha256": sha256_bytes(&support_freeze_bytes),
        "support_capture_index_sha256": support_freeze.capture_index().index_sha256,
        "extended_capture_index_sha256": extended_index.index_sha256,
        "extended_capture_index_file_sha256": sha256_bytes(&extended_index_bytes),
        "future_batch_sha256": sha256_bytes(&batch_bytes),
        "future_batch_transport": "stdout_pipe_only",
        "future_rows": batch.rows().len(),
        "future_session_slots": protocol.source.planned_session_slots,
        "intervention_rows": protocol.source.rows_per_intervention,
        "raw_payload_persisted": false,
        "expected_labels_joined": false,
        "h0_status": "UNPROVEN",
        "h1_status": "UNPROVEN",
        "execution_authority": false,
    });
    let report_bytes = serde_json::to_vec_pretty(&acquisition_report)
        .map_err(|error| format!("report_encode:{error}"))?;

    write_new_sync(&extended_index_path, &extended_index_bytes)?;
    write_new_sync(&acquisition_report_path, &report_bytes)?;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&batch_bytes)
        .map_err(|error| format!("future_batch_pipe_write:{error}"))?;
    stdout
        .flush()
        .map_err(|error| format!("future_batch_pipe_flush:{error}"))?;
    Ok(())
}

fn future_case(
    intervention: usize,
    replicate: usize,
    shape: usize,
) -> Result<(Value, PreActionBindingContextV1), String> {
    if !(1..=6).contains(&intervention) || replicate > 1 {
        return Err("future_case_out_of_bounds".to_owned());
    }
    let base = format!("future-i{intervention}");
    let left = format!("{base}-capability-left");
    let right = format!("{base}-capability-right");
    let decoy = format!("{base}-capability-decoy");
    let parent_left = format!("{base}-parent-left");
    let parent_right = format!("{base}-parent-right");
    let missing_parent = format!("{base}-parent-missing");

    let mut parents = vec![ParentState {
        marker: parent_left.clone(),
        capability: left.clone(),
        phase: "active",
        rank: 1,
    }];
    let mut candidates = vec![left.clone(), right.clone()];
    let (relation_source, relation_target) = match intervention {
        1 => {
            if replicate == 1 {
                candidates.reverse();
            }
            (json!(parent_left), json!(left))
        }
        2 => {
            parents.push(ParentState {
                marker: parent_right.clone(),
                capability: right.clone(),
                phase: "active",
                rank: 1,
            });
            if replicate == 0 {
                (json!(parent_left), json!(left))
            } else {
                (json!(parent_right), json!(right))
            }
        }
        3 => {
            candidates.push(decoy);
            (json!(parent_left), json!(left))
        }
        4 => {
            parents[0].phase = "completed";
            (json!(parent_left), json!(left))
        }
        5 => {
            parents.push(ParentState {
                marker: parent_right.clone(),
                capability: right,
                phase: "active",
                rank: 1,
            });
            (json!([parent_left, parent_right]), Value::Null)
        }
        6 => (json!(missing_parent), Value::Null),
        _ => unreachable!("bounded intervention"),
    };
    let payload = render_future_shape(
        shape,
        &parents,
        &candidates,
        relation_source,
        relation_target,
    );
    let active = parents
        .iter()
        .filter(|parent| parent.phase == "active")
        .count();
    let completion_state = if active > 0 {
        BindingCompletionStateV1::Unresolved
    } else {
        BindingCompletionStateV1::Completed
    };
    let topology_neighborhood_root_sha256 = canonical_json_sha256(&json!({
        "parents": parents.len(),
        "active": active,
        "completed": parents.len().saturating_sub(active),
        "candidates": candidates.len(),
        "relation_sources": if intervention == 5 { 2 } else { 1 },
        "relation_target_present": intervention <= 4,
    }))
    .map_err(|error| format!("future_topology_digest:{error}"))?;
    let context = PreActionBindingContextV1 {
        call_shape_count: u16::try_from(parents.len()).map_err(|_| "parent_count_overflow")?,
        capability_count: u16::try_from(candidates.len())
            .map_err(|_| "candidate_count_overflow")?,
        completion_state,
        temporal_relation_count: u16::try_from(parents.len())
            .map_err(|_| "temporal_count_overflow")?,
        cardinality_relation_count: if intervention == 5 { 2 } else { 1 },
        topology_neighborhood_root_sha256,
    };
    Ok((payload, context))
}

fn render_future_shape(
    shape: usize,
    parents: &[ParentState],
    candidates: &[String],
    relation_source: Value,
    relation_target: Value,
) -> Value {
    match shape {
        0 => json!({
            "future_alpha_timeline": render_parents(parents, "alpha"),
            "future_alpha_options": candidates,
            "future_alpha_binding": {
                "future_alpha_origin": relation_source,
                "future_alpha_target": relation_target
            }
        }),
        1 => json!({
            "future_beta_packet": {
                "future_beta_records": render_parents(parents, "beta"),
                "future_beta_choices": candidates,
                "future_beta_link": {
                    "future_beta_from": relation_source,
                    "future_beta_to": relation_target
                }
            }
        }),
        2 => json!([{
            "future_gamma_records": render_parents(parents, "gamma"),
            "future_gamma_choices": candidates,
            "future_gamma_link": [relation_source, relation_target]
        }]),
        _ => json!({
            "future_delta_state": {
                "future_delta_records": render_parents(parents, "delta")
            },
            "future_delta_selection": {
                "future_delta_choices": candidates
            },
            "future_delta_relation": {
                "future_delta_from": relation_source,
                "future_delta_to": relation_target
            }
        }),
    }
}

fn render_parents(parents: &[ParentState], prefix: &str) -> Vec<Value> {
    parents
        .iter()
        .map(|parent| {
            json!({
                format!("future_{prefix}_marker"): parent.marker,
                format!("future_{prefix}_endpoint"): parent.capability,
                format!("future_{prefix}_phase"): parent.phase,
                format!("future_{prefix}_rank"): parent.rank,
            })
        })
        .collect()
}

fn capture_record(
    row_index: usize,
    session_slot: &str,
    payload: &Value,
    previous_record_sha256: &str,
    first_sequence: u64,
) -> Result<EvidenceLedgerRecord, String> {
    let envelope = RawEvidenceEnvelope {
        source_stream_id: "nando-b1b-future-acquisition-v1".to_owned(),
        source_offset: row_index as u64,
        event_id: format!("b1b-future-event-{row_index}"),
        session_id: format!("b1b-future-session-{session_slot}"),
        client_intent_id: Some(format!("b1b-future-intent-{row_index}")),
        call_id: Some(format!("b1b-future-call-{row_index}")),
        output_ordinal: Some(row_index as u32),
        event_time_unix_nanos: Some(20_000_000 + row_index as u64),
        schema_version: 1,
        payload: serde_json::to_vec(payload).map_err(|error| error.to_string())?,
    };
    let outcome = EvidenceIngestOutcome::Normalized {
        graph: canonicalize_evidence_envelope(&envelope, EvidencePolicyV1::streaming_bounded())
            .map_err(|error| format!("canonical_event:{error:?}"))?,
    };
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        sequence: u64,
        previous_record_sha256: &'a str,
        outcome: &'a EvidenceIngestOutcome,
    }
    let sequence = first_sequence + row_index as u64;
    let record_sha256 = canonical_json_sha256(&DigestFields {
        schema: EVIDENCE_LEDGER_SCHEMA_V1,
        sequence,
        previous_record_sha256,
        outcome: &outcome,
    })
    .map_err(|error| format!("capture_record_digest:{error}"))?;
    Ok(EvidenceLedgerRecord {
        schema: EVIDENCE_LEDGER_SCHEMA_V1.to_owned(),
        sequence,
        previous_record_sha256: previous_record_sha256.to_owned(),
        outcome,
        record_sha256,
    })
}

fn write_new_sync(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("output_dir_create:{}:{error}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("output_create_new:{}:{error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("output_write:{}:{error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("output_sync:{}:{error}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn require_pipe(fd_path: &str, error: &str) -> Result<(), String> {
    let file_type = fs::metadata(fd_path)
        .map_err(|source| format!("pipe_metadata:{fd_path}:{source}"))?
        .file_type();
    if !file_type.is_fifo() && !file_type.is_socket() {
        return Err(error.to_owned());
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_pipe(_fd_path: &str, _error: &str) -> Result<(), String> {
    Err("future_batch_pipe_type_check_unsupported".to_owned())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn usage() -> String {
    "usage: nando-binding-future-acquire <protocol.json> <support-freeze.json> <extended-index.cbor> <acquisition-report.json> | nando-binding-future-capture-owner ...".to_owned()
}
