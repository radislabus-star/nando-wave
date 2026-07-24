//! Status materialization, structural layout, freeze blockers, and bounded IO helpers.

use super::*;

pub(super) fn observation_request_atom_ids(
    observation: &OnlineCollectionObservation,
) -> BTreeSet<u64> {
    let mut atoms: BTreeSet<u64> = observation
        .example
        .provider_payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .rev()
        .find(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .and_then(|item| item.get("content"))
        .and_then(request_content_text)
        .map(|text| request_phase_atom_ids(&text).into_iter().collect())
        .unwrap_or_default();
    atoms.extend(response_pre_action_context_atom_ids(
        &observation.example.provider_payload,
    ));
    atoms
}

pub(super) fn request_content_text(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return (!text.is_empty()).then(|| text.to_owned());
    }
    let text = content
        .as_array()?
        .iter()
        .filter(|part| {
            matches!(
                part.get("type").and_then(Value::as_str),
                Some("text" | "input_text" | "output_text")
            )
        })
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty()).then_some(text)
}

pub(super) fn structural_layout(value: &Value) -> Value {
    match value {
        Value::Null => Value::String("null".to_owned()),
        Value::Bool(_) => Value::String("bool".to_owned()),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Value::String("integer".to_owned())
        }
        Value::Number(_) => Value::String("number".to_owned()),
        Value::String(value) => serde_json::from_str::<Value>(value)
            .ok()
            .filter(|parsed| !matches!(parsed, Value::String(_)))
            .map_or_else(
                || Value::String("string".to_owned()),
                |parsed| structural_layout(&parsed),
            ),
        Value::Array(values) => Value::Array(values.iter().map(structural_layout).collect()),
        Value::Object(values) => {
            let mut shapes = values
                .iter()
                .map(|(key, value)| {
                    Value::Array(vec![
                        Value::String(sha256_bytes(key.as_bytes())),
                        structural_layout(value),
                    ])
                })
                .collect::<Vec<_>>();
            shapes.sort_by_cached_key(|shape| serde_json::to_vec(shape).unwrap_or_default());
            Value::Array(shapes)
        }
    }
}

pub(super) fn support_freeze_blocker(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
) -> Option<String> {
    if bucket.frozen_program_sha256.is_some() {
        return None;
    }
    if bucket.support.len() < required_support_rows {
        return Some(format!("support_rows_below_{required_support_rows}"));
    }
    if bucket.support.iter().any(|receipt| !receipt.verifier_pass) {
        return Some("support_verifier_incomplete".to_owned());
    }
    match support_consensus_candidate(bucket) {
        Ok(SupportConsensusCandidate::Blocked(reason)) => return Some(reason.to_owned()),
        Err(_) => return Some("support_consensus_invalid".to_owned()),
        Ok(SupportConsensusCandidate::Ready(_)) => {}
    }
    if bucket_program_atom_ids(bucket).is_empty() {
        return Some("support_program_atoms_empty".to_owned());
    }
    if bucket
        .support
        .iter()
        .any(|receipt| receipt.event_time_unix_nanos.is_none())
    {
        return Some("support_event_time_missing".to_owned());
    }
    Some("support_freeze_ready_not_applied".to_owned())
}

pub(super) fn bucket_status(
    bucket: &OnlineCollectionBucket,
    proof_mode: OnlineCollectionProofMode,
    required_support_rows: usize,
) -> OnlineCollectionBucketStatus {
    let retained_runtime_examples = bucket.runtime_examples.len();
    let support_rows_with_runtime_examples = bucket
        .support
        .iter()
        .filter(|receipt| {
            bucket
                .runtime_examples
                .contains_key(&receipt.evidence_graph_sha256)
        })
        .count();
    let digest_law_keys = bucket
        .programs
        .iter()
        .filter_map(|(digest, program)| {
            response_law_key(program)
                .ok()
                .map(|law_key| (digest.as_str(), law_key))
        })
        .collect::<BTreeMap<_, _>>();
    let mut abstract_law_support = BTreeMap::<Vec<u8>, usize>::new();
    let mut abstract_law_replayable_support = BTreeMap::<Vec<u8>, usize>::new();
    let mut abstract_law_sessions = BTreeMap::<Vec<u8>, BTreeSet<String>>::new();
    for receipt in &bucket.support {
        let receipt_laws = receipt
            .matched_program_sha256
            .iter()
            .filter_map(|digest| digest_law_keys.get(digest.as_str()).cloned())
            .collect::<BTreeSet<_>>();
        for law_key in receipt_laws {
            *abstract_law_support.entry(law_key.clone()).or_default() += 1;
            abstract_law_sessions
                .entry(law_key.clone())
                .or_default()
                .insert(receipt.session_id_sha256.clone());
            if bucket
                .runtime_examples
                .contains_key(&receipt.evidence_graph_sha256)
            {
                *abstract_law_replayable_support.entry(law_key).or_default() += 1;
            }
        }
    }
    let abstract_law_groups = abstract_law_support.len();
    let best_law_key = abstract_law_support
        .iter()
        .max_by(|(left_key, left_rows), (right_key, right_rows)| {
            left_rows
                .cmp(right_rows)
                .then_with(|| right_key.cmp(left_key))
        })
        .map(|(law_key, _)| law_key.clone());
    let best_abstract_law_support_rows =
        abstract_law_support.into_values().max().unwrap_or_default();
    let best_abstract_law_replayable_support_rows = abstract_law_replayable_support
        .into_values()
        .max()
        .unwrap_or_default();
    let best_abstract_law_missing_replay_hints = best_law_key
        .as_ref()
        .map(|law_key| {
            bucket
                .support
                .iter()
                .filter(|receipt| {
                    !bucket
                        .runtime_examples
                        .contains_key(&receipt.evidence_graph_sha256)
                        && receipt.matched_program_sha256.iter().any(|digest| {
                            digest_law_keys
                                .get(digest.as_str())
                                .is_some_and(|candidate| candidate == law_key)
                        })
                })
                .take(MAX_TARGETED_REHYDRATION_HINTS)
                .map(|receipt| OnlineCollectionRehydrationHint {
                    evidence_graph_sha256: receipt.evidence_graph_sha256.clone(),
                    session_id_sha256: receipt.session_id_sha256.clone(),
                    event_time_unix_nanos: receipt.event_time_unix_nanos,
                    estimated_input_tokens: receipt.estimated_input_tokens,
                })
                .collect()
        })
        .unwrap_or_default();
    let best_abstract_law_session_ids_sha256 = best_law_key
        .clone()
        .and_then(|law_key| abstract_law_sessions.remove(&law_key))
        .unwrap_or_default()
        .into_iter()
        .take(MAX_TARGETED_REHYDRATION_HINTS)
        .collect();
    // Matched digests are durable exact teacher proofs. Runtime examples are
    // tracked separately because they are optional synthesis working memory.
    let best_verified_law_support_rows = best_abstract_law_support_rows;
    let future_sessions = distinct_receipt_sessions(&bucket.future);
    let future_layouts = distinct_receipt_layouts(&bucket.future);
    let runtime_parity_cases = bucket
        .future
        .iter()
        .filter(|receipt| {
            bucket
                .runtime_examples
                .contains_key(&receipt.evidence_graph_sha256)
                || bucket
                    .durable_runtime_parity_receipts
                    .contains_key(&receipt.evidence_graph_sha256)
        })
        .count();
    let adaptive = bucket.adaptive_candidate_freeze.is_some();
    let all_sessions = bucket
        .support
        .iter()
        .chain(&bucket.future)
        .map(|receipt| receipt.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let all_surfaces = bucket
        .support
        .iter()
        .chain(&bucket.future)
        .map(|receipt| receipt.evidence_graph_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let admission_blocker = if bucket.frozen_program_sha256.is_none() {
        status_support_freeze_blocker(
            bucket,
            proof_mode,
            required_support_rows,
            best_verified_law_support_rows,
        )
    } else if adaptive && bucket.future.is_empty() {
        Some("adaptive_future_missing".to_owned())
    } else if adaptive && all_sessions < 2 {
        Some("adaptive_independent_session_missing".to_owned())
    } else if adaptive && all_surfaces < 2 {
        Some("adaptive_surface_missing".to_owned())
    } else if adaptive && runtime_parity_cases < bucket.future.len() {
        Some("adaptive_runtime_parity_incomplete".to_owned())
    } else if !adaptive && bucket.future.len() < 32 {
        Some("future_rows_below_32".to_owned())
    } else if !adaptive && future_sessions < 3 {
        Some("future_sessions_below_3".to_owned())
    } else if !adaptive && future_layouts < 2 {
        Some("future_layouts_below_2".to_owned())
    } else if bucket.wrong_accepts > 0 {
        Some("wrong_accepts_nonzero".to_owned())
    } else if !adaptive && runtime_parity_cases < 32 {
        Some("runtime_parity_cases_below_32".to_owned())
    } else {
        None
    };
    OnlineCollectionBucketStatus {
        bucket_id: bucket.bucket_id.clone(),
        version_space_size: bucket.programs.len(),
        support_rows: bucket.support.len(),
        retained_runtime_examples,
        support_rows_with_runtime_examples,
        abstract_law_groups,
        best_abstract_law_support_rows,
        best_abstract_law_replayable_support_rows,
        best_abstract_law_session_ids_sha256,
        best_abstract_law_missing_replay_hints,
        best_verified_law_support_rows,
        future_rows: bucket.future.len(),
        future_sessions,
        future_layouts,
        wrong_accepts: bucket.wrong_accepts,
        frozen: bucket.frozen_program_sha256.is_some(),
        candidate_program_sha256: bucket.frozen_program_sha256.clone(),
        candidate_program_kind: bucket
            .frozen_program_sha256
            .as_ref()
            .and_then(|digest| bucket.programs.get(digest))
            .map(response_program_kind_code)
            .map(str::to_owned),
        program_kinds: bucket
            .programs
            .values()
            .map(response_program_kind_code)
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        rejected_programs: bucket.rejected_program_sha256.len(),
        learned_anti_atoms: bucket.learned_anti_atom_ids.len(),
        common_request_atoms: bucket.common_request_atom_ids.len(),
        support_tokens: bucket
            .support
            .iter()
            .map(|receipt| receipt.estimated_input_tokens)
            .sum(),
        future_tokens: bucket
            .future
            .iter()
            .map(|receipt| receipt.estimated_input_tokens)
            .sum(),
        support_watermark_event_time_unix_nanos: bucket.support_watermark_event_time_unix_nanos,
        support_manifest_sha256: bucket.support_manifest_sha256.clone(),
        future_manifest_sha256: bucket
            .frozen_program_sha256
            .as_ref()
            .and_then(|_| collection_future_manifest_digest(bucket).ok()),
        runtime_parity_cases,
        admission_blocker,
    }
}

pub(super) fn status_support_freeze_blocker(
    bucket: &OnlineCollectionBucket,
    proof_mode: OnlineCollectionProofMode,
    required_support_rows: usize,
    verified_law_support_rows: usize,
) -> Option<String> {
    if bucket.wrong_accepts > 0 {
        return Some("support_wrong_accepts_nonzero".to_owned());
    }
    if proof_mode == OnlineCollectionProofMode::AdaptiveVersionSpace {
        if bucket.support.is_empty() {
            return Some("adaptive_support_missing".to_owned());
        }
        if bucket
            .support
            .iter()
            .any(|receipt| !receipt.verifier_pass || receipt.matched_program_sha256.is_empty())
        {
            return Some("adaptive_support_verifier_incomplete".to_owned());
        }
        return match identify_collection_bucket(bucket) {
            Ok(Some(_)) => Some("adaptive_freeze_ready_not_applied".to_owned()),
            Ok(None) => Some(format!(
                "adaptive_version_space_ambiguous_{}",
                bucket.programs.len()
            )),
            Err(_) => Some("adaptive_identification_invalid".to_owned()),
        };
    }
    if bucket.support.len() < required_support_rows {
        // The blocker is part of the operator-facing accounting contract, so
        // it must report the threshold used by this bucket rather than the
        // production default. Tests and migrations intentionally use smaller
        // thresholds while preserving the same admission logic.
        return Some(format!("support_rows_below_{required_support_rows}"));
    }
    if bucket
        .support
        .iter()
        .any(|receipt| !receipt.verifier_pass || receipt.matched_program_sha256.is_empty())
    {
        return Some("support_verifier_incomplete".to_owned());
    }
    if verified_law_support_rows < required_support_rows {
        return Some("support_consensus_authority_unproven".to_owned());
    }
    if bucket_program_atom_ids(bucket).is_empty() {
        return Some("support_program_atoms_empty".to_owned());
    }
    if bucket
        .support
        .iter()
        .any(|receipt| receipt.event_time_unix_nanos.is_none())
    {
        return Some("support_event_time_missing".to_owned());
    }
    Some("support_phase_adapter_unproven".to_owned())
}

pub(super) fn response_program_kind_code(program: &ResponseProgram) -> &'static str {
    match response_program_kind(program) {
        AstProgramKind::PlanAdvance => "plan_advance",
        AstProgramKind::FunctionCall => "function_call",
        AstProgramKind::CustomToolCall => "custom_tool_call",
        AstProgramKind::Project => "project",
        AstProgramKind::Status => "status",
        AstProgramKind::Collection => "collection",
        AstProgramKind::Legacy => "legacy",
    }
}

pub(super) fn merge_receipts(
    target: &mut Vec<OnlineCollectionReceipt>,
    source: Vec<OnlineCollectionReceipt>,
    max: usize,
) {
    let mut by_evidence = BTreeMap::<String, OnlineCollectionReceipt>::new();
    for mut receipt in target.drain(..).chain(source) {
        let evidence = receipt.evidence_graph_sha256.clone();
        if let Some(existing) = by_evidence.get_mut(&evidence) {
            existing
                .request_atom_ids
                .append(&mut receipt.request_atom_ids);
            existing.request_atom_ids.sort_unstable();
            existing.request_atom_ids.dedup();
            existing
                .matched_program_sha256
                .append(&mut receipt.matched_program_sha256);
            existing.matched_program_sha256.sort();
            existing.matched_program_sha256.dedup();
            if existing.witness_class_commitment_sha256.is_none() {
                existing.witness_class_commitment_sha256 = receipt.witness_class_commitment_sha256;
                existing.witness_round = receipt.witness_round;
                existing.witness_candidates_before = receipt.witness_candidates_before;
                existing.witness_candidates_after = receipt.witness_candidates_after;
            }
        } else {
            receipt.matched_program_sha256.sort();
            receipt.matched_program_sha256.dedup();
            by_evidence.insert(evidence, receipt);
        }
    }
    let mut receipts = by_evidence.into_values().collect::<Vec<_>>();
    receipts.sort_by(|left, right| {
        left.event_time_unix_nanos
            .cmp(&right.event_time_unix_nanos)
            .then_with(|| left.evidence_graph_sha256.cmp(&right.evidence_graph_sha256))
    });
    if receipts.len() > max {
        receipts.drain(..receipts.len().saturating_sub(max));
    }
    *target = receipts;
}

pub(super) fn push_bounded<T>(values: &mut Vec<T>, value: T, max: usize) {
    if values.len() == max {
        values.remove(0);
    }
    values.push(value);
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn valid_witness_receipt_metadata(receipt: &OnlineCollectionReceipt) -> bool {
    match (
        receipt.witness_class_commitment_sha256.as_deref(),
        receipt.witness_round,
        receipt.witness_candidates_before,
        receipt.witness_candidates_after,
    ) {
        (None, None, None, None) => true,
        (Some(commitment), Some(round), Some(before), Some(after)) => {
            is_sha256(commitment)
                && (1..=MAX_ACTIVE_WITNESS_ROUNDS).contains(&round)
                && after > 0
                && after < before
        }
        _ => false,
    }
}

pub(super) fn sync_parent(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("online_collection_checkpoint_parent_sync:{error}"))
}
