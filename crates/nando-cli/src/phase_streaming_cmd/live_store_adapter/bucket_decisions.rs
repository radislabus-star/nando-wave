use super::source_events::LiveStoreParsedAtomEvent;

pub(super) fn live_store_exact_bucket_decisions(
    decisions: &[nando_core::PhaseCenterHotCandidateDecision],
    bucket_id: u32,
) -> Vec<nando_core::PhaseCenterHotCandidateDecision> {
    decisions
        .iter()
        .copied()
        .filter(|decision| decision.profile_id == bucket_id)
        .collect()
}

pub(super) fn live_store_union_score_candidate_decision(
    decisions: &[nando_core::PhaseCenterHotCandidateDecision],
) -> Vec<nando_core::PhaseCenterHotCandidateDecision> {
    decisions
        .iter()
        .copied()
        .filter(|decision| decision.score_candidate)
        .max_by_key(|decision| decision.margin_micro)
        .into_iter()
        .collect()
}

pub(super) fn live_store_relevant_online_bucket_ids(event: &LiveStoreParsedAtomEvent) -> Vec<u32> {
    let mut bucket_ids = Vec::with_capacity(event.auto_subcenter_bucket_ids.len() + 1);
    bucket_ids.push(event.bucket_id);
    bucket_ids.extend(event.auto_subcenter_bucket_ids.iter().copied());
    bucket_ids.sort_unstable();
    bucket_ids.dedup();
    bucket_ids
}

pub(super) fn live_store_relevant_bucket_decisions(
    decisions: &[nando_core::PhaseCenterHotCandidateDecision],
    relevant_bucket_ids: &[u32],
) -> Vec<nando_core::PhaseCenterHotCandidateDecision> {
    decisions
        .iter()
        .copied()
        .filter(|decision| {
            relevant_bucket_ids
                .binary_search(&decision.profile_id)
                .is_ok()
        })
        .collect()
}
