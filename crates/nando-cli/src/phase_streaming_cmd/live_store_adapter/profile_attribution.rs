use std::collections::BTreeMap;

use super::hidden_state::live_store_auto_subcenter_bucket_key;
use super::source_events::{LiveStoreParsedAtomEvent, live_store_hash_id};

#[derive(Clone, Copy, Default)]
pub(super) struct LiveStoreProfileAttribution {
    pub(super) observable_score_candidate: bool,
    pub(super) hidden_state_score_candidate: bool,
    pub(super) unknown_score_candidate: bool,
}

pub(super) fn live_store_profile_attribution(
    event: &LiveStoreParsedAtomEvent,
    decisions: &[nando_core::PhaseCenterHotCandidateDecision],
    known_profile_kinds: &BTreeMap<u32, &'static str>,
) -> LiveStoreProfileAttribution {
    let mut attribution = LiveStoreProfileAttribution::default();
    for decision in decisions.iter().filter(|decision| decision.score_candidate) {
        match live_store_event_profile_kind(event, decision.profile_id, known_profile_kinds) {
            "hidden_state" => attribution.hidden_state_score_candidate = true,
            "observable_primary" | "observable_subcenter" => {
                attribution.observable_score_candidate = true;
            }
            _ => attribution.unknown_score_candidate = true,
        }
    }
    attribution
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_update_profile_attribution_counters(
    attribution: LiveStoreProfileAttribution,
    event: &LiveStoreParsedAtomEvent,
    observable_score_candidate_events: &mut usize,
    hidden_state_score_candidate_events: &mut usize,
    unknown_profile_score_candidate_events: &mut usize,
    observable_unique_cpu_accepts_over_exact_cache: &mut usize,
    hidden_state_unique_cpu_accepts_over_exact_cache: &mut usize,
    unknown_profile_unique_cpu_accepts_over_exact_cache: &mut usize,
    profile_attribution_overlap_accepts: &mut usize,
    observable_only_unique_cpu_accepts_over_exact_cache: &mut usize,
    hidden_state_only_unique_cpu_accepts_over_exact_cache: &mut usize,
    mixed_profile_unique_cpu_accepts_over_exact_cache: &mut usize,
    unknown_only_unique_cpu_accepts_over_exact_cache: &mut usize,
    observable_tokens_saved: &mut u64,
    hidden_state_tokens_saved: &mut u64,
    unknown_profile_tokens_saved: &mut u64,
    observable_only_tokens_saved: &mut u64,
    hidden_state_only_tokens_saved: &mut u64,
    mixed_profile_tokens_saved: &mut u64,
    unknown_only_tokens_saved: &mut u64,
    observable_only_cost_saved_microusd: &mut u64,
    hidden_state_only_cost_saved_microusd: &mut u64,
    mixed_profile_cost_saved_microusd: &mut u64,
    unknown_only_cost_saved_microusd: &mut u64,
) {
    if attribution.observable_score_candidate {
        *observable_score_candidate_events = observable_score_candidate_events.saturating_add(1);
    }
    if attribution.hidden_state_score_candidate {
        *hidden_state_score_candidate_events =
            hidden_state_score_candidate_events.saturating_add(1);
    }
    if attribution.unknown_score_candidate {
        *unknown_profile_score_candidate_events =
            unknown_profile_score_candidate_events.saturating_add(1);
    }
    if !event.verified_safe_accept || event.exact_cache_hit {
        return;
    }
    let mut accept_kinds = 0usize;
    if attribution.observable_score_candidate {
        accept_kinds = accept_kinds.saturating_add(1);
        *observable_unique_cpu_accepts_over_exact_cache =
            observable_unique_cpu_accepts_over_exact_cache.saturating_add(1);
        *observable_tokens_saved = observable_tokens_saved.saturating_add(event.tokens);
    }
    if attribution.hidden_state_score_candidate {
        accept_kinds = accept_kinds.saturating_add(1);
        *hidden_state_unique_cpu_accepts_over_exact_cache =
            hidden_state_unique_cpu_accepts_over_exact_cache.saturating_add(1);
        *hidden_state_tokens_saved = hidden_state_tokens_saved.saturating_add(event.tokens);
    }
    if attribution.unknown_score_candidate {
        accept_kinds = accept_kinds.saturating_add(1);
        *unknown_profile_unique_cpu_accepts_over_exact_cache =
            unknown_profile_unique_cpu_accepts_over_exact_cache.saturating_add(1);
        *unknown_profile_tokens_saved = unknown_profile_tokens_saved.saturating_add(event.tokens);
    }
    if accept_kinds > 1 {
        *profile_attribution_overlap_accepts =
            profile_attribution_overlap_accepts.saturating_add(1);
        *mixed_profile_unique_cpu_accepts_over_exact_cache =
            mixed_profile_unique_cpu_accepts_over_exact_cache.saturating_add(1);
        *mixed_profile_tokens_saved = mixed_profile_tokens_saved.saturating_add(event.tokens);
        *mixed_profile_cost_saved_microusd =
            mixed_profile_cost_saved_microusd.saturating_add(event.cost_microusd);
    } else if attribution.observable_score_candidate {
        *observable_only_unique_cpu_accepts_over_exact_cache =
            observable_only_unique_cpu_accepts_over_exact_cache.saturating_add(1);
        *observable_only_tokens_saved = observable_only_tokens_saved.saturating_add(event.tokens);
        *observable_only_cost_saved_microusd =
            observable_only_cost_saved_microusd.saturating_add(event.cost_microusd);
    } else if attribution.hidden_state_score_candidate {
        *hidden_state_only_unique_cpu_accepts_over_exact_cache =
            hidden_state_only_unique_cpu_accepts_over_exact_cache.saturating_add(1);
        *hidden_state_only_tokens_saved =
            hidden_state_only_tokens_saved.saturating_add(event.tokens);
        *hidden_state_only_cost_saved_microusd =
            hidden_state_only_cost_saved_microusd.saturating_add(event.cost_microusd);
    } else if attribution.unknown_score_candidate {
        *unknown_only_unique_cpu_accepts_over_exact_cache =
            unknown_only_unique_cpu_accepts_over_exact_cache.saturating_add(1);
        *unknown_only_tokens_saved = unknown_only_tokens_saved.saturating_add(event.tokens);
        *unknown_only_cost_saved_microusd =
            unknown_only_cost_saved_microusd.saturating_add(event.cost_microusd);
    }
}

pub(super) fn live_store_record_event_profile_kinds(
    event: &LiveStoreParsedAtomEvent,
    known_profile_kinds: &mut BTreeMap<u32, &'static str>,
) {
    known_profile_kinds
        .entry(event.bucket_id)
        .or_insert("observable_primary");
    for atom in &event.auto_subcenter_atoms {
        let bucket_key = live_store_auto_subcenter_bucket_key(&event.route_key, atom);
        let profile_id = live_store_hash_id(["live_store_bucket", bucket_key.as_str()]);
        known_profile_kinds
            .entry(profile_id)
            .or_insert_with(|| live_store_profile_kind_from_auto_subcenter_atom(atom));
    }
}

pub(super) fn live_store_known_profile_kind_counts(
    known_profile_kinds: &BTreeMap<u32, &'static str>,
) -> (usize, usize, usize) {
    let hidden_state = known_profile_kinds
        .values()
        .filter(|kind| **kind == "hidden_state")
        .count();
    let observable = known_profile_kinds.len().saturating_sub(hidden_state);
    (known_profile_kinds.len(), observable, hidden_state)
}

fn live_store_profile_kind_from_auto_subcenter_atom(atom: &str) -> &'static str {
    if atom.starts_with("hidden_state:") {
        "hidden_state"
    } else {
        "observable_subcenter"
    }
}

fn live_store_event_profile_kind(
    event: &LiveStoreParsedAtomEvent,
    profile_id: u32,
    known_profile_kinds: &BTreeMap<u32, &'static str>,
) -> &'static str {
    if profile_id == event.bucket_id {
        return "observable_primary";
    }
    for atom in &event.auto_subcenter_atoms {
        let bucket_key = live_store_auto_subcenter_bucket_key(&event.route_key, atom);
        let bucket_id = live_store_hash_id(["live_store_bucket", bucket_key.as_str()]);
        if bucket_id == profile_id {
            return live_store_profile_kind_from_auto_subcenter_atom(atom);
        }
    }
    if let Some(kind) = known_profile_kinds.get(&profile_id) {
        return *kind;
    }
    "unknown"
}
