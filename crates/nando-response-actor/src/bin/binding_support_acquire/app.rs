use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use nando_response_actor::{
    BindingCompletionStateV1, BindingSupportCaptureBatchV1, BindingSupportCaptureRowV1,
    CaptureCommitmentIndex, CaptureEvidenceReceipt, CaptureRecordCommitment,
    EVIDENCE_LEDGER_SCHEMA_V1, EvidenceIngestOutcome, EvidenceLedgerRecord, EvidencePolicyV1,
    PreActionBindingContextV1, PreActionBindingSurfaceV1, RawEvidenceEnvelope,
    canonical_json_sha256, canonicalize_evidence_envelope,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SUPPORT_ROWS: usize = 12;

pub(super) fn main() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let capture_index_path = args.next().ok_or_else(usage)?;
    let support_batch_path = args.next().ok_or_else(usage)?;
    let acquisition_report_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage());
    }

    let mut previous_record_sha256 = "0".repeat(64);
    let mut records = Vec::with_capacity(SUPPORT_ROWS);
    let mut rows = Vec::with_capacity(SUPPORT_ROWS);
    for row_index in 0..SUPPORT_ROWS {
        let intervention = row_index % 6 + 1;
        let replicate = row_index / 6;
        let session = row_index / 3;
        let payload = intervention_payload(intervention, replicate);
        let record = capture_record(row_index, session, &payload, &previous_record_sha256)?;
        previous_record_sha256 = record.record_sha256.clone();
        let commitment = CaptureRecordCommitment {
            sequence: record.sequence,
            record_sha256: record.record_sha256.clone(),
        };
        let context = context_from_payload(&payload)?;
        let frozen_graph = PreActionBindingSurfaceV1::capture(
            sha256_bytes(format!("b1b-support-row-{row_index}").as_bytes()),
            sha256_bytes(format!("b1b-support-evidence-{row_index}").as_bytes()),
            "continue active execution",
            &payload,
            context,
            Default::default(),
        )
        .map_err(|error| format!("pre_action_capture:{error:?}"))?
        .candidate_relation_graph(Default::default())
        .map_err(|error| format!("candidate_relation_graph:{error:?}"))?
        .freeze()
        .map_err(|error| format!("candidate_relation_graph_freeze:{error:?}"))?;
        let row = BindingSupportCaptureRowV1::new(
            frozen_graph,
            CaptureEvidenceReceipt::new(vec![commitment.clone()]).map_err(str::to_owned)?,
            record.clone(),
            format!("I{intervention}"),
        )
        .map_err(|error| format!("support_capture_row:{error:?}"))?;
        records.push(commitment);
        rows.push(row);
    }

    let capture_index = CaptureCommitmentIndex::new(records).map_err(str::to_owned)?;
    let support_batch = BindingSupportCaptureBatchV1::new(rows)
        .map_err(|error| format!("support_batch:{error:?}"))?;
    support_batch
        .validate()
        .map_err(|error| format!("support_batch_validate:{error:?}"))?;
    let capture_index_bytes = serde_cbor::to_vec(&capture_index)
        .map_err(|error| format!("capture_index_encode:{error}"))?;
    let support_batch_bytes = serde_json::to_vec(&support_batch)
        .map_err(|error| format!("support_batch_encode:{error}"))?;
    let acquisition_report = json!({
        "schema": "nando.binding-support-acquisition-report.v1",
        "stage": "B1B-S",
        "source": "controlled_label_blind_causal_interventions_v1",
        "support_rows": SUPPORT_ROWS,
        "session_lineages": 4,
        "intervention_rows": {"I1": 2, "I2": 2, "I3": 2, "I4": 2, "I5": 2, "I6": 2},
        "planned_applicable_slots": 6,
        "planned_not_applicable_or_ambiguous_slots": 6,
        "expected_labels_joined": false,
        "h0_status": "UNPROVEN",
        "h1_status": "UNPROVEN",
        "future_status": "NOT_OPENED",
        "capture_index_file_sha256": sha256_bytes(&capture_index_bytes),
        "support_batch_file_sha256": sha256_bytes(&support_batch_bytes),
        "execution_authority": false,
    });
    let acquisition_report_bytes = serde_json::to_vec_pretty(&acquisition_report)
        .map_err(|error| format!("acquisition_report_encode:{error}"))?;

    write_new_sync(&capture_index_path, &capture_index_bytes)?;
    write_new_sync(&support_batch_path, &support_batch_bytes)?;
    write_new_sync(&acquisition_report_path, &acquisition_report_bytes)?;
    Ok(())
}

fn intervention_payload(intervention: usize, replicate: usize) -> Value {
    let left = format!("opaque-capability-{replicate}-left");
    let right = format!("opaque-capability-{replicate}-right");
    let parent_left = format!("opaque-parent-{replicate}-left");
    let parent_right = format!("opaque-parent-{replicate}-right");
    let mut parents = vec![json!({
        "anchor": parent_left,
        "capability": left,
        "state": "active",
        "distance": 1
    })];
    let mut candidates = vec![left.clone()];
    let relation = match intervention {
        1 => {
            candidates.push(right.clone());
            if replicate == 1 {
                candidates.reverse();
            }
            json!({"source": parent_left, "capability": left})
        }
        2 => {
            parents.push(json!({
                "anchor": parent_right,
                "capability": right,
                "state": "active",
                "distance": 1
            }));
            candidates.push(right.clone());
            if replicate == 0 {
                json!({"source": parent_left, "capability": left})
            } else {
                json!({"source": parent_right, "capability": right})
            }
        }
        3 => {
            candidates.extend([right.clone(), format!("opaque-decoy-{replicate}")]);
            json!({"source": parent_left, "capability": left})
        }
        4 => {
            parents[0]["state"] = Value::String("completed".to_owned());
            json!({"source": parent_left, "capability": left})
        }
        5 => {
            parents.push(json!({
                "anchor": parent_right,
                "capability": right,
                "state": "active",
                "distance": 1
            }));
            candidates.push(right);
            json!({"source": [parent_left, parent_right], "capability": null})
        }
        6 => {
            candidates.push(right);
            json!({"source": format!("opaque-missing-parent-{replicate}"), "capability": null})
        }
        _ => unreachable!("bounded intervention id"),
    };
    if replicate == 0 {
        json!({
            "history": parents,
            "available": candidates,
            "request_relation": relation
        })
    } else {
        json!({
            "transport": {
                "items": parents,
                "choices": candidates,
                "relation": relation
            }
        })
    }
}

fn context_from_payload(payload: &Value) -> Result<PreActionBindingContextV1, String> {
    let (parents, candidates) = if let Some(transport) = payload.get("transport") {
        (
            transport
                .get("items")
                .and_then(Value::as_array)
                .ok_or("transport_items_missing")?,
            transport
                .get("choices")
                .and_then(Value::as_array)
                .ok_or("transport_choices_missing")?,
        )
    } else {
        (
            payload
                .get("history")
                .and_then(Value::as_array)
                .ok_or("history_missing")?,
            payload
                .get("available")
                .and_then(Value::as_array)
                .ok_or("available_missing")?,
        )
    };
    let active = parents
        .iter()
        .filter(|parent| parent.get("state").and_then(Value::as_str) == Some("active"))
        .count();
    let completed = parents.len().saturating_sub(active);
    let completion_state = if active > 0 {
        BindingCompletionStateV1::Unresolved
    } else if completed > 0 {
        BindingCompletionStateV1::Completed
    } else {
        BindingCompletionStateV1::Unknown
    };
    let topology_neighborhood_root_sha256 = canonical_json_sha256(&json!({
        "parents": parents.len(),
        "active": active,
        "completed": completed,
        "candidates": candidates.len(),
        "relation_present": true
    }))
    .map_err(|error| format!("topology_digest:{error}"))?;
    Ok(PreActionBindingContextV1 {
        call_shape_count: u16::try_from(parents.len()).map_err(|_| "parent_count_overflow")?,
        capability_count: u16::try_from(candidates.len())
            .map_err(|_| "candidate_count_overflow")?,
        completion_state,
        temporal_relation_count: u16::try_from(parents.len())
            .map_err(|_| "temporal_count_overflow")?,
        cardinality_relation_count: 1,
        topology_neighborhood_root_sha256,
    })
}

fn capture_record(
    row_index: usize,
    session: usize,
    payload: &Value,
    previous_record_sha256: &str,
) -> Result<EvidenceLedgerRecord, String> {
    let envelope = RawEvidenceEnvelope {
        source_stream_id: "nando-b1b-support-acquisition-v1".to_owned(),
        source_offset: row_index as u64,
        event_id: format!("b1b-support-event-{row_index}"),
        session_id: format!("b1b-support-session-{session}"),
        client_intent_id: Some(format!("b1b-support-intent-{row_index}")),
        call_id: Some(format!("b1b-support-call-{row_index}")),
        output_ordinal: Some(row_index as u32),
        event_time_unix_nanos: Some(10_000_000 + row_index as u64),
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
    let sequence = row_index as u64;
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

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn usage() -> String {
    "usage: nando-binding-support-acquire <capture-index.cbor> <support-batch.json> <acquisition-report.json>".to_owned()
}
