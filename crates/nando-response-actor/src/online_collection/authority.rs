//! Independent receipts, phase routing, consensus, and candidate-authority proof.
//!
//! These checks do not activate packages; they produce proof material for admission.

use super::*;

pub(super) fn validate_observation(
    observation: &OnlineCollectionObservation,
) -> Result<(), String> {
    if !is_sha256(&observation.evidence_graph_sha256)
        || !is_sha256(&observation.client_intent_id_sha256)
        || !is_sha256(&observation.session_id_sha256)
    {
        return Err("online_collection_observation_identity_invalid".to_owned());
    }
    Ok(())
}

pub(super) fn receipt(
    observation: &OnlineCollectionObservation,
    verifier_pass: bool,
) -> Result<OnlineCollectionReceipt, String> {
    Ok(OnlineCollectionReceipt {
        evidence_graph_sha256: observation.evidence_graph_sha256.clone(),
        client_intent_id_sha256: observation.client_intent_id_sha256.clone(),
        session_id_sha256: observation.session_id_sha256.clone(),
        event_time_unix_nanos: observation.event_time_unix_nanos,
        layout_sha256: structural_layout_sha256(&observation.example.provider_payload)?,
        estimated_input_tokens: observation.estimated_input_tokens,
        verifier_pass,
        request_atom_ids: observation_request_atom_ids(observation)
            .into_iter()
            .collect(),
        matched_program_sha256: Vec::new(),
        witness_class_commitment_sha256: None,
        witness_round: None,
        witness_candidates_before: None,
        witness_candidates_after: None,
    })
}

pub(super) fn receipt_with_program_atoms(
    observation: &OnlineCollectionObservation,
    verifier_pass: bool,
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<OnlineCollectionReceipt, String> {
    let mut value = receipt(observation, verifier_pass)?;
    value
        .request_atom_ids
        .extend(common_program_atom_ids(programs));
    value.request_atom_ids.sort_unstable();
    value.request_atom_ids.dedup();
    value.matched_program_sha256 = programs
        .iter()
        .filter(|(_, program)| independently_verified_teacher_match(program, &observation.example))
        .map(|(digest, _)| digest.clone())
        .collect();
    Ok(value)
}

pub(super) fn common_program_atom_ids(
    programs: &BTreeMap<String, ResponseProgram>,
) -> BTreeSet<u64> {
    let mut programs = programs.values();
    let Some(first) = programs.next() else {
        return BTreeSet::new();
    };
    let mut common = response_program_required_routing_atom_ids(first)
        .into_iter()
        .collect::<BTreeSet<_>>();
    for program in programs {
        let atoms = response_program_required_routing_atom_ids(program)
            .into_iter()
            .collect::<BTreeSet<_>>();
        common.retain(|atom| atoms.contains(atom));
    }
    common
}

pub(super) fn bucket_program_atom_ids(bucket: &OnlineCollectionBucket) -> BTreeSet<u64> {
    common_program_atom_ids(&bucket.programs)
}

pub(super) fn durable_pre_action_atom_ids(
    bucket: &OnlineCollectionBucket,
    receipt: &OnlineCollectionReceipt,
) -> BTreeSet<u64> {
    // Receipts are durable but may also contain routing atoms contributed by
    // the support-side program pool. Remove the union of every known program
    // atom so a teacher-derived program cannot become runtime evidence. A hash
    // collision can only remove a real pre-action atom and reduce recall.
    let program_atoms = bucket
        .programs
        .values()
        .flat_map(response_program_required_routing_atom_ids)
        .collect::<BTreeSet<_>>();
    receipt
        .request_atom_ids
        .iter()
        .copied()
        .filter(|atom| !program_atoms.contains(atom))
        .collect()
}

pub(super) fn bucket_phase_center_atom_ids(bucket: &OnlineCollectionBucket) -> Vec<u64> {
    let program_atoms = bucket_program_atom_ids(bucket);
    let mut atoms = program_atoms.into_iter().collect::<Vec<_>>();
    atoms.extend(bucket.common_request_atom_ids.iter().copied());
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

pub(super) fn distinct_receipt_sessions(receipts: &[OnlineCollectionReceipt]) -> usize {
    receipts
        .iter()
        .map(|receipt| &receipt.session_id_sha256)
        .collect::<BTreeSet<_>>()
        .len()
}

pub(super) fn distinct_receipt_layouts(receipts: &[OnlineCollectionReceipt]) -> usize {
    receipts
        .iter()
        .map(|receipt| &receipt.layout_sha256)
        .collect::<BTreeSet<_>>()
        .len()
}

pub(super) fn learned_wave_margin_micro(
    bucket: &OnlineCollectionBucket,
    phase_centers: &[u64],
    anti_centers: &[u64],
) -> i64 {
    let positive = phase_vector_from_atom_ids(phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(anti_centers.iter().copied(), 16);
    bucket
        .support
        .iter()
        .filter_map(|receipt| {
            let query = phase_vector_from_atom_ids(receipt.request_atom_ids.iter().copied(), 16);
            phase_margin_to_micro(
                phase_coherence(&query, &positive) - phase_coherence(&query, &negative),
            )
            .ok()
        })
        .min()
        .map(|minimum| minimum.saturating_mul(9).saturating_div(10).max(1))
        .unwrap_or(1)
}

pub(super) fn receipt_routes_phase(
    receipt: &OnlineCollectionReceipt,
    phase_centers: &[u64],
    anti_centers: &[u64],
    threshold: i64,
) -> bool {
    let query = phase_vector_from_atom_ids(receipt.request_atom_ids.iter().copied(), 16);
    let positive = phase_vector_from_atom_ids(phase_centers.iter().copied(), 16);
    let negative = phase_vector_from_atom_ids(anti_centers.iter().copied(), 16);
    phase_margin_to_micro(phase_coherence(&query, &positive) - phase_coherence(&query, &negative))
        .is_ok_and(|margin| margin >= threshold)
}

pub(super) fn update_applicability_negative_sessions(
    evidence: &mut BTreeMap<u64, BTreeSet<String>>,
    candidates: BTreeSet<u64>,
    session_id_sha256: &str,
) -> BTreeSet<u64> {
    for atom in candidates
        .into_iter()
        .take(MAX_APPLICABILITY_NEGATIVE_ATOMS_PER_BUCKET)
    {
        evidence
            .entry(atom)
            .or_default()
            .insert(session_id_sha256.to_owned());
    }
    while evidence.len() > MAX_APPLICABILITY_NEGATIVE_ATOMS_PER_BUCKET {
        let Some(atom) = evidence.keys().next_back().copied() else {
            break;
        };
        evidence.remove(&atom);
    }
    evidence
        .iter()
        .filter_map(|(atom, sessions)| {
            (sessions.len() >= MIN_APPLICABILITY_NEGATIVE_SESSIONS).then_some(*atom)
        })
        .collect()
}

pub(super) fn structural_layout_sha256(value: &Value) -> Result<String, String> {
    canonical_json_sha256(&structural_layout(value)).map_err(str::to_owned)
}

pub(super) fn independently_verified_authority_response(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> Option<String> {
    independently_verified_authority_response_result(program, example).ok()
}

pub(super) fn independently_verified_teacher_match(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> bool {
    // Discovery may retain structurally aligned laws, but a durable proof link
    // is exact only when the independently verified output equals the teacher.
    independently_verified_authority_response(program, example).as_deref()
        == Some(example.expected_response.as_str())
}

pub(super) fn independently_verified_authority_response_result(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> Result<String, &'static str> {
    if !response_program_authority_matches_example(program, example) {
        return Err("authority_mismatch");
    }
    let execution = execute_response(program, "", &example.provider_payload);
    if execution.status != ResponseExecutionStatus::Executed {
        return Err("actor_abstain");
    }
    let response = execution.response.ok_or("actor_response_missing")?;
    let verifier =
        source_neutral_verifier_for_program(program).map_err(|_| "verifier_build_failed")?;
    verify_response_independently(&verifier, &example.provider_payload, &response)
        .map_err(|_| "verifier_rejected")?;
    Ok(response)
}

pub(super) fn authority_rejection_reason(
    result: &Result<String, &'static str>,
) -> Option<&'static str> {
    match result {
        Err(reason) => Some(*reason),
        Ok(_) => None,
    }
}

pub(super) fn is_hard_teacher_counterexample(reason: &str) -> bool {
    matches!(
        reason,
        "verifier_build_failed"
            | "verifier_rejected"
            | "actor_response_missing"
            | "teacher_response_mismatch"
    )
}

pub(super) enum SupportConsensusCandidate {
    Ready(ResponseProgram),
    Blocked(&'static str),
}

pub(super) fn best_adapter<'a>(
    adapters: impl Iterator<Item = (&'a String, &'a ResponseProgram)>,
) -> Option<(&'a String, &'a ResponseProgram)> {
    adapters.min_by(|(left_digest, left), (right_digest, right)| {
        u8::from(!is_source_neutral_response_program(left))
            .cmp(&u8::from(!is_source_neutral_response_program(right)))
            .then_with(|| {
                serde_json::to_vec(left)
                    .map_or(usize::MAX, |bytes| bytes.len())
                    .cmp(&serde_json::to_vec(right).map_or(usize::MAX, |bytes| bytes.len()))
            })
            .then_with(|| left_digest.cmp(right_digest))
    })
}

pub(super) fn request_atoms_for_example(
    example: &CollectionSynthesisExample,
) -> Option<BTreeSet<u64>> {
    let text = example
        .provider_payload
        .get("input")?
        .as_array()?
        .iter()
        .filter(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .filter_map(|item| item.get("content"))
        .filter_map(request_content_text)
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then(|| request_phase_atom_ids(&text).into_iter().collect())
}

pub(super) fn phase_ranked_semantic_adapters(
    bucket: &OnlineCollectionBucket,
) -> Option<ResponseProgram> {
    const CELLS: usize = 16;
    const MAX_VARIANTS: usize = crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS;
    let rows = bucket.support.iter().collect::<Vec<_>>();
    if rows.len() < 4 {
        return None;
    }
    let programs = concrete_adapter_program_classes(bucket);
    if !(2..=MAX_VARIANTS).contains(&programs.len()) {
        return None;
    }
    let globally_proven = programs.iter().filter(|(_, (_, source_digests))| {
        rows.iter().all(|receipt| {
            receipt
                .matched_program_sha256
                .iter()
                .any(|matched| source_digests.contains(matched))
        })
    });
    if let Some((_, program)) =
        best_adapter(globally_proven.map(|(digest, (program, _))| (digest, program)))
    {
        return Some(program.clone());
    }
    let mut variants = Vec::new();
    let mut routes = Vec::new();
    for (_, (program, source_digests)) in programs {
        let row_atoms = rows
            .iter()
            .map(|receipt| durable_adapter_atoms(bucket, receipt, &source_digests))
            .collect::<Option<Vec<_>>>();
        let Some(row_atoms) = row_atoms else {
            continue;
        };
        let mut positives = Vec::new();
        let mut negatives = Vec::new();
        for (receipt, atoms) in rows.iter().zip(row_atoms) {
            if receipt
                .matched_program_sha256
                .iter()
                .any(|matched| source_digests.contains(matched))
            {
                positives.push(atoms.to_vec());
            } else {
                negatives.push(atoms.to_vec());
            }
        }
        if positives.is_empty() || negatives.is_empty() {
            continue;
        }
        let Some(route) = fit_adapter_wave_route(&positives, &negatives, CELLS) else {
            continue;
        };
        variants.push(ResponseConsensusVariant {
            program,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        });
        routes.push(route);
    }
    if variants.len() < 2 {
        return None;
    }
    let candidate = ResponseProgram::unique_consensus(variants).with_adapter_wave(
        ResponseAdapterWaveConsensus {
            exact_budget: u16::try_from(routes.len().min(16)).ok()?,
            routes,
        },
    );
    candidate.validate().ok()?;
    candidate_authority_verified_on_support(bucket, &candidate).then_some(candidate)
}

pub(super) fn concrete_adapter_program_classes(
    bucket: &OnlineCollectionBucket,
) -> BTreeMap<String, (ResponseProgram, BTreeSet<String>)> {
    let mut classes = BTreeMap::<String, (ResponseProgram, BTreeSet<String>)>::new();
    for (source_digest, program) in &bucket.programs {
        let Ok(class_digest) = canonical_json_sha256(program) else {
            continue;
        };
        classes
            .entry(class_digest)
            .or_insert_with(|| (program.clone(), BTreeSet::new()))
            .1
            .insert(source_digest.clone());
    }
    classes
}

pub(super) fn durable_adapter_atoms<'a>(
    bucket: &'a OnlineCollectionBucket,
    receipt: &OnlineCollectionReceipt,
    source_digests: &BTreeSet<String>,
) -> Option<&'a [u64]> {
    let atoms_by_program = bucket
        .durable_adapter_phase_atoms
        .get(&receipt.evidence_graph_sha256)?;
    source_digests
        .iter()
        .find_map(|digest| atoms_by_program.get(digest))
        .map(Vec::as_slice)
}

pub(super) fn phase_guarded_layout_adapters(
    bucket: &OnlineCollectionBucket,
    rows: &[&OnlineCollectionReceipt],
) -> Option<Vec<(String, ResponseProgram, Vec<u64>)>> {
    type GuardedAdapter = (String, ResponseProgram, Vec<u64>, BTreeSet<usize>);
    let row_atoms = rows
        .iter()
        .map(|receipt| durable_pre_action_atom_ids(bucket, receipt))
        .collect::<Vec<_>>();
    if row_atoms.iter().any(BTreeSet::is_empty) {
        return None;
    }
    let mut safe = Vec::<GuardedAdapter>::new();
    for (digest, program) in &bucket.programs {
        let positives = rows
            .iter()
            .enumerate()
            .filter(|(_, receipt)| {
                receipt.verifier_pass
                    && receipt
                        .matched_program_sha256
                        .iter()
                        .any(|matched| matched == digest)
            })
            .map(|(index, _)| index)
            .collect::<BTreeSet<_>>();
        if positives.is_empty() {
            continue;
        }
        let mut common = positives
            .iter()
            .next()
            .map(|index| row_atoms[*index].clone())?;
        for index in positives.iter().skip(1) {
            common.retain(|atom| row_atoms[*index].contains(atom));
        }
        let mut remaining_negatives = (0..rows.len())
            .filter(|index| !positives.contains(index))
            .collect::<BTreeSet<_>>();
        let mut guard = Vec::<u64>::new();
        while !remaining_negatives.is_empty() && guard.len() < 8 {
            let next = common
                .iter()
                .filter(|atom| !guard.contains(atom))
                .map(|atom| {
                    let excluded = remaining_negatives
                        .iter()
                        .filter(|index| !row_atoms[**index].contains(atom))
                        .count();
                    (*atom, excluded)
                })
                .max_by(|(left_atom, left), (right_atom, right)| {
                    left.cmp(right).then_with(|| right_atom.cmp(left_atom))
                });
            let Some((atom, excluded)) = next else {
                break;
            };
            if excluded == 0 {
                break;
            }
            guard.push(atom);
            remaining_negatives.retain(|index| row_atoms[*index].contains(&atom));
        }
        if remaining_negatives.is_empty() {
            guard.sort_unstable();
            safe.push((digest.clone(), program.clone(), guard, positives));
        }
    }
    let mut uncovered = (0..rows.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::new();
    while !uncovered.is_empty() {
        let candidate = safe
            .iter()
            .filter(|(digest, _, guard, _)| {
                !selected.iter().any(|(selected_digest, _, selected_guard)| {
                    selected_digest == digest && selected_guard == guard
                })
            })
            .map(|(digest, program, guard, covered)| {
                let gain = covered.intersection(&uncovered).count();
                (gain, digest, program, guard, covered)
            })
            .filter(|(gain, _, _, _, _)| *gain > 0)
            .max_by(|left, right| {
                left.0
                    .cmp(&right.0)
                    .then_with(|| right.3.len().cmp(&left.3.len()))
                    .then_with(|| {
                        is_source_neutral_response_program(left.2)
                            .cmp(&is_source_neutral_response_program(right.2))
                    })
                    .then_with(|| right.1.cmp(left.1))
            });
        let (_, digest, program, guard, covered) = candidate?;
        selected.push((digest.clone(), program.clone(), guard.clone()));
        for index in covered {
            uncovered.remove(index);
        }
    }
    Some(selected)
}

pub(super) fn response_selector_family(program: &ResponseProgram) -> &'static str {
    let selector = match &program.operation {
        crate::ResponseOperation::ProjectSelectedValue { selector, .. }
        | crate::ResponseOperation::ProjectStatus { selector, .. } => selector,
        crate::ResponseOperation::ComposeCollection { .. } => return "collection",
        _ => return "other",
    };
    match selector {
        crate::ResponseValueSelector::ContinuationHandle { .. } => "continuation_handle",
        crate::ResponseValueSelector::UniqueScalar { .. } => "unique_scalar",
        crate::ResponseValueSelector::UniqueTurnScalar { .. } => "unique_turn_scalar",
        crate::ResponseValueSelector::ContentLinePrefix { .. } => "content_line_prefix",
        crate::ResponseValueSelector::JsonField { .. } => "json_field",
        crate::ResponseValueSelector::JsonScalarOrdinal { .. } => "json_scalar_ordinal",
        crate::ResponseValueSelector::UniqueTurnJsonField { .. } => "unique_turn_json_field",
        crate::ResponseValueSelector::UniqueActiveTurnJsonField { .. } => {
            "unique_active_turn_json_field"
        }
        crate::ResponseValueSelector::RequestReferencedJsonField { .. } => {
            "request_referenced_json_field"
        }
        crate::ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. } => {
            "request_referenced_json_field_ordinal"
        }
        crate::ResponseValueSelector::TurnOutputLine { .. } => "turn_output_line",
        crate::ResponseValueSelector::TurnOutputScalarOrdinal { .. } => {
            "turn_output_scalar_ordinal"
        }
        crate::ResponseValueSelector::LatestTurnOutputLine { .. } => "latest_turn_output_line",
        crate::ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. } => {
            "latest_turn_output_scalar_ordinal"
        }
        crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. } => {
            "latest_turn_output_scalar_from_end"
        }
        crate::ResponseValueSelector::CommandOutputBody => "command_output_body",
        crate::ResponseValueSelector::RequestLastToken => "request_last_token",
        crate::ResponseValueSelector::RequestUniqueLiteral => "request_unique_literal",
    }
}

#[derive(Default)]
pub(super) struct AdapterWaveDiagnostic {
    programs_considered: usize,
    programs_with_positive_and_negative: usize,
    routes_fitted: usize,
    candidate_valid: bool,
    authority_pass: bool,
    authority_rejection_counts: BTreeMap<String, usize>,
    first_rejected_evidence_sha256: String,
    blocker: String,
}

pub(super) fn adapter_wave_diagnostic(bucket: &OnlineCollectionBucket) -> AdapterWaveDiagnostic {
    const CELLS: usize = 16;
    let mut diagnostic = AdapterWaveDiagnostic::default();
    let rows = bucket.support.iter().collect::<Vec<_>>();
    let programs = concrete_adapter_program_classes(bucket);
    diagnostic.programs_considered = programs.len();
    if !(2..=crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS).contains(&programs.len()) {
        diagnostic.blocker = "adapter_wave_variant_count".to_owned();
        return diagnostic;
    }
    let mut variants = Vec::new();
    let mut routes = Vec::new();
    for (_, (program, source_digests)) in programs {
        let mut positives = Vec::new();
        let mut negatives = Vec::new();
        for receipt in &rows {
            let Some(atoms) = durable_adapter_atoms(bucket, receipt, &source_digests) else {
                diagnostic.blocker = "adapter_wave_missing_durable_phase_atoms".to_owned();
                return diagnostic;
            };
            if receipt
                .matched_program_sha256
                .iter()
                .any(|matched| source_digests.contains(matched))
            {
                positives.push(atoms.to_vec());
            } else {
                negatives.push(atoms.to_vec());
            }
        }
        if positives.is_empty() || negatives.is_empty() {
            continue;
        }
        diagnostic.programs_with_positive_and_negative = diagnostic
            .programs_with_positive_and_negative
            .saturating_add(1);
        let Some(route) = fit_adapter_wave_route(&positives, &negatives, CELLS) else {
            continue;
        };
        diagnostic.routes_fitted = diagnostic.routes_fitted.saturating_add(1);
        variants.push(ResponseConsensusVariant {
            program,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        });
        routes.push(route);
    }
    if variants.len() < 2 {
        diagnostic.blocker = "adapter_wave_routes_below_two".to_owned();
        return diagnostic;
    }
    let candidate = ResponseProgram::unique_consensus(variants).with_adapter_wave(
        ResponseAdapterWaveConsensus {
            exact_budget: u16::try_from(routes.len().min(16)).unwrap_or(16),
            routes,
        },
    );
    diagnostic.candidate_valid = candidate.validate().is_ok();
    if !diagnostic.candidate_valid {
        diagnostic.blocker = "adapter_wave_candidate_invalid".to_owned();
        return diagnostic;
    }
    for receipt in &bucket.support {
        if durable_adapter_wave_proves_candidate(bucket, receipt, &candidate) {
            continue;
        }
        let reason = "adapter_wave_durable_authority_unproven".to_owned();
        if diagnostic.first_rejected_evidence_sha256.is_empty() {
            diagnostic.first_rejected_evidence_sha256 = receipt.evidence_graph_sha256.clone();
        }
        *diagnostic
            .authority_rejection_counts
            .entry(reason)
            .or_default() += 1;
    }
    diagnostic.authority_pass = diagnostic.authority_rejection_counts.is_empty();
    diagnostic.blocker = if diagnostic.authority_pass {
        String::new()
    } else {
        "adapter_wave_support_authority_failed".to_owned()
    };
    diagnostic
}

pub(super) fn consensus_diagnostic(
    bucket: &OnlineCollectionBucket,
    required_support_rows: usize,
    max_receipts_per_bucket: usize,
) -> OnlineCollectionConsensusDiagnostic {
    let adapter_wave = adapter_wave_diagnostic(bucket);
    let law_subcenters =
        support_law_subcenters(bucket, required_support_rows, max_receipts_per_bucket)
            .unwrap_or_default();
    let best_law_subcenter = law_subcenters.first();
    let best_law_subcenter_consensus =
        best_law_subcenter.map_or_else(String::new, |subcenter| match support_consensus_candidate(
            subcenter,
        ) {
            Ok(SupportConsensusCandidate::Ready(_)) => "READY".to_owned(),
            Ok(SupportConsensusCandidate::Blocked(reason)) => reason.to_owned(),
            Err(error) => format!("ERROR:{error}"),
        });
    let best_law_subcenter_freeze_blocker = best_law_subcenter
        .and_then(|subcenter| support_freeze_blocker(subcenter, required_support_rows))
        .unwrap_or_default();
    let mut canonical = BTreeMap::<String, ResponseProgram>::new();
    for program in bucket.programs.values() {
        let Ok(direct) = canonical_direct_response_program(program) else {
            continue;
        };
        if !is_source_neutral_response_program(&direct) {
            continue;
        }
        if let Ok(digest) = canonical_json_sha256(&direct) {
            canonical.entry(digest).or_insert(direct);
        }
    }
    let mut selector_families = BTreeMap::<String, usize>::new();
    for program in canonical.values() {
        *selector_families
            .entry(response_selector_family(program).to_owned())
            .or_default() += 1;
    }
    let rows = bucket
        .support
        .iter()
        .filter_map(|receipt| bucket.runtime_examples.get(&receipt.evidence_graph_sha256))
        .collect::<Vec<_>>();
    let targets = rows
        .iter()
        .map(|example| {
            canonical
                .values()
                .filter_map(|program| independently_verified_authority_response(program, example))
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    let unique_target_rows = targets.iter().filter(|values| values.len() == 1).count();
    let missing_target_rows = targets.iter().filter(|values| values.is_empty()).count();
    let ambiguous_target_rows = targets.iter().filter(|values| values.len() > 1).count();
    let mut safe_programs = 0_usize;
    let mut unsafe_disagreement_programs = 0_usize;
    let mut safely_covered = BTreeSet::<usize>::new();
    let mut max_safe_program_coverage = 0_usize;
    for program in canonical.values() {
        let mut coverage = BTreeSet::new();
        let mut disagrees = false;
        for (index, (example, targets)) in rows.iter().zip(&targets).enumerate() {
            let Some(target) = (targets.len() == 1)
                .then(|| targets.iter().next())
                .flatten()
            else {
                continue;
            };
            let execution = execute_response(program, "", &example.provider_payload);
            if execution.status != ResponseExecutionStatus::Executed {
                continue;
            }
            if execution.response.as_deref() != Some(target.as_str()) {
                disagrees = true;
                break;
            }
            if independently_verified_authority_response(program, example).as_deref()
                == Some(target.as_str())
            {
                coverage.insert(index);
            }
        }
        if disagrees {
            unsafe_disagreement_programs = unsafe_disagreement_programs.saturating_add(1);
        } else if !coverage.is_empty() {
            safe_programs = safe_programs.saturating_add(1);
            max_safe_program_coverage = max_safe_program_coverage.max(coverage.len());
            safely_covered.extend(coverage);
        }
    }
    OnlineCollectionConsensusDiagnostic {
        bucket_id: bucket.bucket_id.clone(),
        support_rows: bucket.support.len(),
        replayable_rows: rows.len(),
        canonical_programs: canonical.len(),
        unique_target_rows,
        missing_target_rows,
        ambiguous_target_rows,
        safe_programs,
        unsafe_disagreement_programs,
        safely_coverable_rows: safely_covered.len(),
        max_safe_program_coverage,
        selector_families,
        candidate_present: unguarded_unique_consensus_candidate(bucket).is_some(),
        adapter_wave_programs_considered: adapter_wave.programs_considered,
        adapter_wave_programs_with_positive_and_negative: adapter_wave
            .programs_with_positive_and_negative,
        adapter_wave_routes_fitted: adapter_wave.routes_fitted,
        adapter_wave_candidate_valid: adapter_wave.candidate_valid,
        adapter_wave_authority_pass: adapter_wave.authority_pass,
        adapter_wave_authority_rejection_counts: adapter_wave.authority_rejection_counts,
        adapter_wave_first_rejected_evidence_sha256: adapter_wave.first_rejected_evidence_sha256,
        adapter_wave_blocker: adapter_wave.blocker,
        law_subcenters_total: law_subcenters.len(),
        best_law_subcenter_support_rows: best_law_subcenter.map_or(0, |value| value.support.len()),
        best_law_subcenter_programs: best_law_subcenter.map_or(0, |value| value.programs.len()),
        best_law_subcenter_consensus,
        best_law_subcenter_freeze_blocker,
    }
}

pub(super) fn unguarded_unique_consensus_candidate(
    bucket: &OnlineCollectionBucket,
) -> Option<ResponseProgram> {
    let mut canonical = BTreeMap::<String, ResponseProgram>::new();
    for program in bucket.programs.values() {
        let Ok(direct) = canonical_direct_response_program(program) else {
            continue;
        };
        if !is_source_neutral_response_program(&direct) {
            continue;
        }
        let digest = canonical_json_sha256(&direct).ok()?;
        canonical.entry(digest).or_insert(direct);
    }
    if !(2..=crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS).contains(&canonical.len()) {
        return None;
    }
    let rows = bucket
        .support
        .iter()
        .map(|receipt| bucket.runtime_examples.get(&receipt.evidence_graph_sha256))
        .collect::<Option<Vec<_>>>()?;
    let targets = rows
        .iter()
        .map(|example| {
            let responses = canonical
                .values()
                .filter_map(|program| independently_verified_authority_response(program, example))
                .collect::<BTreeSet<_>>();
            (responses.len() == 1).then(|| responses.into_iter().next())?
        })
        .collect::<Option<Vec<_>>>()?;
    let mut safe = Vec::<(String, ResponseProgram, BTreeSet<usize>)>::new();
    for (digest, program) in canonical {
        let mut covered = BTreeSet::new();
        let mut disagrees = false;
        for (index, (example, target)) in rows.iter().zip(&targets).enumerate() {
            let execution = execute_response(&program, "", &example.provider_payload);
            if execution.status != ResponseExecutionStatus::Executed {
                continue;
            }
            if execution.response.as_deref() != Some(target.as_str()) {
                disagrees = true;
                break;
            }
            if independently_verified_authority_response(&program, example).as_deref()
                == Some(target.as_str())
            {
                covered.insert(index);
            }
        }
        if !disagrees && !covered.is_empty() {
            safe.push((digest, program, covered));
        }
    }
    let mut uncovered = (0..rows.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::<ResponseProgram>::new();
    while !uncovered.is_empty() {
        let candidate = safe
            .iter()
            .filter(|(_, program, _)| !selected.contains(program))
            .map(|(digest, program, covered)| {
                (
                    covered.intersection(&uncovered).count(),
                    digest,
                    program,
                    covered,
                )
            })
            .filter(|(gain, _, _, _)| *gain > 0)
            .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(left.1)))?;
        selected.push(candidate.2.clone());
        for index in candidate.3 {
            uncovered.remove(index);
        }
    }
    let candidate = if selected.len() == 1 {
        selected.pop()?
    } else {
        ResponseProgram::unique_consensus(
            selected
                .into_iter()
                .map(|program| ResponseConsensusVariant {
                    program,
                    allowed_layout_sha256: Vec::new(),
                    required_request_atom_ids: Vec::new(),
                })
                .collect(),
        )
    };
    candidate.validate().ok()?;
    candidate_authority_verified_on_support(bucket, &candidate).then_some(candidate)
}

pub(super) fn support_consensus_candidate(
    bucket: &OnlineCollectionBucket,
) -> Result<SupportConsensusCandidate, String> {
    let globally_proven = bucket
        .programs
        .iter()
        .filter(|(digest, _)| {
            bucket.support.iter().all(|receipt| {
                receipt.verifier_pass
                    && receipt
                        .matched_program_sha256
                        .iter()
                        .any(|matched| matched == *digest)
            })
        })
        .map(|(digest, program)| (digest.clone(), program.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut variants = if let Some((_, program)) = best_adapter(globally_proven.iter()) {
        vec![ResponseConsensusVariant {
            program: program.clone(),
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        }]
    } else if let Some(candidate) = phase_ranked_semantic_adapters(bucket) {
        return Ok(SupportConsensusCandidate::Ready(candidate));
    } else if let Some(candidate) = unguarded_unique_consensus_candidate(bucket) {
        return Ok(SupportConsensusCandidate::Ready(candidate));
    } else {
        let mut by_adapter = BTreeMap::<(String, Vec<u64>), (ResponseProgram, Vec<String>)>::new();
        let layouts = bucket
            .support
            .iter()
            .map(|receipt| receipt.layout_sha256.clone())
            .collect::<BTreeSet<_>>();
        for layout in layouts {
            let rows = bucket
                .support
                .iter()
                .filter(|receipt| receipt.layout_sha256 == layout)
                .collect::<Vec<_>>();
            let common = bucket.programs.iter().filter(|(digest, _)| {
                rows.iter().all(|receipt| {
                    receipt.verifier_pass
                        && receipt
                            .matched_program_sha256
                            .iter()
                            .any(|matched| matched == *digest)
                })
            });
            if let Some((digest, program)) = best_adapter(common) {
                by_adapter
                    .entry((digest.clone(), Vec::new()))
                    .or_insert_with(|| (program.clone(), Vec::new()))
                    .1
                    .push(layout);
            } else {
                let Some(adapters) = phase_guarded_layout_adapters(bucket, &rows) else {
                    return Ok(SupportConsensusCandidate::Blocked(
                        "support_phase_adapter_unproven",
                    ));
                };
                for (digest, program, guard) in adapters {
                    by_adapter
                        .entry((digest, guard))
                        .or_insert_with(|| (program, Vec::new()))
                        .1
                        .push(layout.clone());
                }
            }
        }
        by_adapter
            .into_iter()
            .flat_map(|((_, required_request_atom_ids), (program, layouts))| {
                layouts
                    .chunks(16)
                    .map(|layout_chunk| ResponseConsensusVariant {
                        program: program.clone(),
                        allowed_layout_sha256: layout_chunk.to_vec(),
                        required_request_atom_ids: required_request_atom_ids.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    variants.sort_by(|left, right| {
        canonical_json_sha256(&left.program)
            .unwrap_or_default()
            .cmp(&canonical_json_sha256(&right.program).unwrap_or_default())
            .then_with(|| left.allowed_layout_sha256.cmp(&right.allowed_layout_sha256))
    });
    if variants.len() > crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS {
        return Ok(SupportConsensusCandidate::Blocked(
            "support_consensus_variant_budget_exceeded",
        ));
    }
    let candidate = if variants.len() == 1 && variants[0].allowed_layout_sha256.is_empty() {
        variants.into_iter().next().expect("one variant").program
    } else {
        ResponseProgram::unique_consensus(variants)
    };
    candidate.validate().map_err(str::to_owned)?;
    if !candidate_authority_verified_on_support(bucket, &candidate) {
        return Ok(SupportConsensusCandidate::Blocked(
            "support_consensus_authority_unproven",
        ));
    }
    Ok(SupportConsensusCandidate::Ready(candidate))
}

pub(super) fn candidate_authority_verified_on_support(
    bucket: &OnlineCollectionBucket,
    candidate: &ResponseProgram,
) -> bool {
    bucket.support.iter().all(|receipt| {
        if let Some(example) = bucket.runtime_examples.get(&receipt.evidence_graph_sha256) {
            // Structural teacher alignment may train a canonical law, but a
            // frozen CPU package must reproduce the complete teacher response.
            return independently_verified_teacher_match(candidate, example);
        }
        durable_adapter_wave_proves_candidate(bucket, receipt, candidate)
            || receipt_proves_candidate_authority(receipt, candidate)
    })
}

pub(super) fn durable_adapter_wave_proves_candidate(
    bucket: &OnlineCollectionBucket,
    receipt: &OnlineCollectionReceipt,
    candidate: &ResponseProgram,
) -> bool {
    if !receipt.verifier_pass {
        return false;
    }
    let crate::ResponseOperation::UniqueConsensus {
        variants,
        adapter_wave: Some(wave),
    } = &candidate.operation
    else {
        return false;
    };
    if variants.len() != wave.routes.len() || variants.is_empty() {
        return false;
    }

    // The compact checkpoint can prove a unique phase winner without retaining
    // raw payload. Equal-margin routes remain unknown because output parity
    // cannot be reconstructed from phase atoms alone.
    let classes = concrete_adapter_program_classes(bucket);
    let mut ranked = Vec::with_capacity(variants.len());
    for (index, (variant, route)) in variants.iter().zip(&wave.routes).enumerate() {
        let Ok(class_digest) = canonical_json_sha256(&variant.program) else {
            return false;
        };
        let Some((_, source_digests)) = classes.get(&class_digest) else {
            return false;
        };
        let Some(atoms) = durable_adapter_atoms(bucket, receipt, source_digests) else {
            return false;
        };
        if let Some(margin) = crate::runtime::adapter_wave_margin_from_atoms(atoms, route) {
            ranked.push((margin, index, source_digests));
        }
    }
    ranked.sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    let Some((best_margin, _, winner_digests)) = ranked.first() else {
        return false;
    };
    if ranked
        .iter()
        .filter(|entry| entry.0 == *best_margin)
        .count()
        != 1
    {
        return false;
    }
    receipt
        .matched_program_sha256
        .iter()
        .any(|matched| winner_digests.contains(matched))
}

pub(super) fn receipt_proves_candidate_authority(
    receipt: &OnlineCollectionReceipt,
    candidate: &ResponseProgram,
) -> bool {
    if !receipt.verifier_pass {
        return false;
    }
    if canonical_json_sha256(candidate).is_ok_and(|digest| {
        receipt
            .matched_program_sha256
            .iter()
            .any(|matched| matched == &digest)
    }) {
        return true;
    }
    let crate::ResponseOperation::UniqueConsensus { variants, .. } = &candidate.operation else {
        return false;
    };
    let mut applicable = false;
    for variant in variants {
        if !variant.allowed_layout_sha256.is_empty()
            && !variant
                .allowed_layout_sha256
                .iter()
                .any(|layout| layout == &receipt.layout_sha256)
        {
            continue;
        }
        if variant
            .required_request_atom_ids
            .iter()
            .any(|atom| receipt.request_atom_ids.binary_search(atom).is_err())
        {
            continue;
        }
        applicable = true;
        let Ok(digest) = canonical_json_sha256(&variant.program) else {
            return false;
        };
        if !receipt
            .matched_program_sha256
            .iter()
            .any(|matched| matched == &digest)
        {
            return false;
        }
    }
    applicable
}

pub(super) fn collection_support_manifest_digest(
    bucket: &OnlineCollectionBucket,
) -> Result<String, String> {
    let program_sha256 = bucket
        .frozen_program_sha256
        .as_deref()
        .ok_or_else(|| "online_collection_support_program_missing".to_owned())?;
    let watermark_event_time_unix_nanos = bucket
        .support_watermark_event_time_unix_nanos
        .ok_or_else(|| "online_collection_support_watermark_missing".to_owned())?;
    canonical_json_sha256(&CollectionSupportManifestMaterial {
        schema: "nando.collection-support-manifest.v1",
        bucket_id: &bucket.bucket_id,
        program_sha256,
        watermark_event_time_unix_nanos,
        receipts: &bucket.support,
    })
    .map_err(str::to_owned)
}

pub(super) fn collection_future_manifest_digest(
    bucket: &OnlineCollectionBucket,
) -> Result<String, String> {
    let support_manifest_sha256 = bucket
        .support_manifest_sha256
        .as_deref()
        .ok_or_else(|| "online_collection_support_manifest_missing".to_owned())?;
    canonical_json_sha256(&CollectionFutureManifestMaterial {
        schema: "nando.collection-future-manifest.v1",
        support_manifest_sha256,
        receipts: &bucket.future,
    })
    .map_err(str::to_owned)
}
