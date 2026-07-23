use super::*;

pub(super) struct ClassifiedSemanticEvidence {
    pub(super) frame: crate::RelationFrame,
    pub(super) outcome: SemanticEvidenceOutcome,
    pub(super) reason: &'static str,
}

pub(super) struct ClassifiedCohortPool {
    pub(super) pool: TeacherPoolSnapshot,
    pub(super) evidence: Vec<ClassifiedSemanticEvidence>,
}

pub(super) fn semantic_outcome_precedence(outcome: SemanticEvidenceOutcome) -> u8 {
    match outcome {
        SemanticEvidenceOutcome::CensoredUnknown => 0,
        SemanticEvidenceOutcome::ApplicabilityNegative => 1,
        SemanticEvidenceOutcome::VerifiedEquivalent => 2,
        SemanticEvidenceOutcome::HardContradiction => 3,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FutureFilterDiagnostic {
    pub(super) matching_rows: usize,
    pub(super) matching_sessions: usize,
    pub(super) post_repair_rows: usize,
    pub(super) post_repair_sessions: usize,
    pub(super) live_rows: usize,
    pub(super) after_watermark_rows: usize,
    pub(super) support_frame_rejects: usize,
    pub(super) support_session_rejects: usize,
    pub(super) support_intent_rejects: usize,
    pub(super) support_event_rejects: usize,
    pub(super) independent_rows: usize,
    pub(super) program_mismatch_rejects: usize,
    pub(super) consistent_rows: usize,
    pub(super) route_mismatch_rejects: usize,
    pub(super) routed_rows: usize,
}

#[derive(Clone, Debug)]
pub(super) struct DerivedWinnerCohort {
    pub(super) winner: CegisWinner,
    pub(super) members: Vec<CegisWinner>,
    pub(super) member_signatures: BTreeSet<String>,
    pub(super) physical_adapter_count: usize,
    pub(super) law_signature_sha256: String,
}

pub(super) fn semantic_member_action_matches(
    members: &[CegisWinner],
    law_signature_sha256: &str,
    frame_signature: Option<&str>,
    frame: &crate::RelationFrame,
) -> bool {
    // Physical teacher signatures may span several effect laws. The broader
    // action contract can join adapters only after semantic-law ownership.
    let same_effect_law =
        crate::teacher_semantic_law_signature(frame).as_deref() == Some(law_signature_sha256);
    same_effect_law
        && members.iter().any(|member| {
            (frame_signature == Some(member.teacher_signature_sha256.as_str())
                && crate::synthesis::program_is_consistent(&member.program, frame)
                && crate::cegis::winner_routes_frame(member, frame))
                || crate::frame_matches_program_action_contract(&member.program, frame)
        })
}

pub(super) fn semantic_program_matches_runtime_parity(
    program: &crate::ResponseProgram,
    parity: &crate::RuntimeParityCase,
) -> bool {
    let execution =
        crate::execute_response(program, &parity.request_text, &parity.provider_payload);
    execution.status == crate::ResponseExecutionStatus::Executed
        && execution.response.as_deref().is_some_and(|actual| {
            actual == parity.expected_response
                || crate::online_admission::responses_match_after_execution_budget_normalization(
                    actual,
                    &parity.expected_response,
                )
        })
}

pub(super) fn semantic_program_covers_all_runtime_parity(
    program: &crate::ResponseProgram,
    parity_cases: &[&crate::RuntimeParityCase],
) -> bool {
    // A semantic winner owns every member receipt; a support threshold cannot
    // authorize a partial union that silently drops one physical adapter.
    !parity_cases.is_empty()
        && parity_cases
            .iter()
            .all(|parity| semantic_program_matches_runtime_parity(program, parity))
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct GenerationParityReceipts {
    pub(super) support: BTreeMap<String, crate::RuntimeParityCase>,
    pub(super) future: BTreeMap<String, crate::RuntimeParityCase>,
}

pub(super) fn rekey_parity_to_canonical_frames(
    cases: &mut BTreeMap<String, crate::RuntimeParityCase>,
    frames: &mut BTreeMap<String, crate::RelationFrame>,
    canonical_frames: &BTreeMap<(String, String, String), crate::RelationFrame>,
) {
    let old_cases = std::mem::take(cases);
    let old_frames = std::mem::take(frames);
    for (old_frame_id, mut parity) in old_cases {
        let Some(old_frame) = old_frames.get(&old_frame_id) else {
            cases.insert(old_frame_id, parity);
            continue;
        };
        let key = (
            old_frame.evidence_ref_sha256.clone(),
            old_frame.event_id_sha256.clone(),
            old_frame.session_id_sha256.clone(),
        );
        let canonical = canonical_frames.get(&key).unwrap_or(old_frame);
        let canonical_id = canonical.frame_id_sha256.clone();
        parity.evidence_ref_sha256.clone_from(&canonical_id);
        cases.insert(canonical_id.clone(), parity);
        frames.insert(canonical_id, canonical.clone());
    }
}

pub(super) fn generation_evidence_improves(
    current: &FrozenGeneration,
    next: &FrozenGeneration,
) -> bool {
    let current_support = current
        .support
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let next_support = next
        .support
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    current_support == next_support
        && (next.future.len() > current.future.len()
            || next.future_sessions > current.future_sessions
            || next.surfaces > current.surfaces
            || next.wrong_future_rows > current.wrong_future_rows
            || (current.blocker.is_some() && next.blocker.is_none()))
}

pub(super) fn generation_support_parity_complete(
    generation: &FrozenGeneration,
    receipts: Option<&GenerationParityReceipts>,
    policy: RolloverPolicy,
) -> bool {
    let Some(receipts) = receipts else {
        return false;
    };
    let receipt_backed = generation
        .support
        .iter()
        .filter(|frame| receipts.support.contains_key(&frame.frame_id_sha256))
        .count();
    support_partition_complete(receipt_backed, policy)
}
