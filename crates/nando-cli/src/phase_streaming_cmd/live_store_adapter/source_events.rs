use std::collections::{BTreeMap, BTreeSet};

use nando_core::{
    PhaseCenterHotRequestEvidence, PhaseCenterLiveOperatorAtomEvent, PhaseCenterOnlineDecision,
};

pub(super) const LIVE_STORE_MAX_BUCKET_REFINEMENT_DEPTH: usize = 3;

#[derive(Clone, Debug, Default)]
pub(super) struct LiveStoreAdaptiveBucketPolicy {
    route_depths: BTreeMap<String, usize>,
    pub(super) refinement_count: usize,
}

#[derive(Clone, Debug)]
pub(super) struct LiveStoreParsedAtomEvent {
    pub(super) route_key: String,
    pub(super) bucket_key: String,
    pub(super) route_id: u32,
    pub(super) bucket_id: u32,
    pub(super) atom_ids: Vec<u64>,
    pub(super) safe_atom_count: usize,
    pub(super) bucket_selector_candidate_atoms: Vec<String>,
    pub(super) selected_bucket_atoms: Vec<String>,
    pub(super) auto_subcenter_atoms: Vec<String>,
    pub(super) auto_subcenter_bucket_ids: Vec<u32>,
    pub(super) bucket_refinement_depth: usize,
    pub(super) verified_safe_accept: bool,
    pub(super) exact_cache_hit: bool,
    pub(super) request_fingerprint: Option<String>,
    pub(super) exact_cache_key: Option<String>,
    pub(super) trace_id: Option<String>,
    pub(super) input_trace_path: Option<String>,
    pub(super) event_timestamp: Option<String>,
    pub(super) tokens: u64,
    pub(super) cost_microusd: u64,
    pub(super) cost_estimate_used: bool,
}

#[derive(Clone, Debug)]
pub(super) struct LiveStoreParsedAtomEventWithAtoms {
    pub(super) event: LiveStoreParsedAtomEvent,
    pub(super) safe_atoms: Vec<String>,
}

impl LiveStoreParsedAtomEvent {
    pub(super) fn hot_request_evidence(&self) -> PhaseCenterHotRequestEvidence {
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: self.verified_safe_accept,
            exact_cache_hit: self.exact_cache_hit,
            tokens: self.tokens,
            cost_microusd: self.cost_microusd,
        }
    }

    pub(super) fn to_live_operator_atom_event(&self) -> PhaseCenterLiveOperatorAtomEvent<'_> {
        self.to_live_operator_atom_event_for_bucket(self.bucket_id)
    }

    pub(super) fn to_live_operator_atom_event_for_bucket(
        &self,
        bucket_id: u32,
    ) -> PhaseCenterLiveOperatorAtomEvent<'_> {
        PhaseCenterLiveOperatorAtomEvent::new(
            self.route_id,
            bucket_id,
            &self.atom_ids,
            self.hot_request_evidence(),
        )
    }
}

pub(super) fn live_store_apply_tail_cost_estimate(
    event: &mut LiveStoreParsedAtomEvent,
    price_config: &super::super::ModelPriceConfig,
) -> u64 {
    if event.cost_microusd > 0 || event.tokens == 0 {
        return 0;
    }
    let estimated_cost =
        super::super::token_floor_cost_microusd(event.tokens as usize, price_config);
    if estimated_cost == 0 {
        return 0;
    }
    event.cost_microusd = estimated_cost;
    event.cost_estimate_used = true;
    estimated_cost
}

pub(super) fn live_store_safe_atoms(row: &serde_json::Value) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    for atom in super::super::phase_atom_string_vec(row, "request_atoms")
        .into_iter()
        .chain(super::super::phase_atom_string_vec(row, "state_atoms"))
        .chain(super::super::phase_atom_string_vec(row, "action_atoms"))
        .chain(super::super::phase_atom_string_vec(row, "tool_atoms"))
        .chain(super::super::phase_atom_string_vec(row, "route_hint_atoms"))
        .chain(super::super::phase_atom_string_vec(
            row,
            "shadow_payload_atoms",
        ))
    {
        if !live_store_forbidden_atom(&atom) {
            atoms.insert(atom);
        }
    }
    atoms.into_iter().collect()
}

pub(super) fn live_store_atom_event_from_row(
    row: &serde_json::Value,
    verified_safe_accept: bool,
    bucket_policy: &LiveStoreAdaptiveBucketPolicy,
    exact_cache_keys_seen: &mut BTreeSet<String>,
) -> Option<LiveStoreParsedAtomEvent> {
    let safe_atoms = live_store_safe_atoms(row);
    if safe_atoms.is_empty() {
        return None;
    }
    let route_key = live_store_route_key(row);
    let bucket_refinement_depth = bucket_policy.route_depth(&route_key);
    let bucket_selector_candidate_atoms =
        live_store_bucket_selector_candidate_atoms(&safe_atoms, bucket_refinement_depth);
    let selected_bucket_atoms =
        live_store_selected_bucket_atoms_from_candidates(&bucket_selector_candidate_atoms);
    let bucket_key = live_store_bucket_key(&route_key, &selected_bucket_atoms);
    let auto_subcenter_atoms =
        super::hidden_state::live_store_auto_subcenter_atoms_from_safe_atoms(&safe_atoms);
    let atom_ids = safe_atoms
        .iter()
        .map(|atom| super::super::stable_fingerprint(["live_store_atom", atom.as_str()]))
        .collect::<Vec<_>>();
    let row_exact_cache_key = super::super::json_string(row, &["exact_cache_key"]);
    let request_fingerprint = super::super::json_string(row, &["request_fingerprint"]);
    let exact_cache_key = row_exact_cache_key
        .clone()
        .or_else(|| request_fingerprint.clone())
        .unwrap_or_else(|| {
            format!(
                "live-store-row:{:016x}",
                super::super::stable_fingerprint(safe_atoms.iter().map(String::as_str))
            )
        });
    let duplicate_exact_cache_key = !exact_cache_keys_seen.insert(exact_cache_key);
    let exact_cache_hit = row
        .get("exact_cache_hit")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(duplicate_exact_cache_key);
    let token_cost = super::super::generic_token_cost_from_row(row);
    let route_id = live_store_hash_id(["live_store_route", route_key.as_str()]);
    let bucket_id = live_store_hash_id(["live_store_bucket", bucket_key.as_str()]);
    let auto_subcenter_bucket_ids = super::hidden_state::live_store_auto_subcenter_bucket_ids(
        &route_key,
        bucket_id,
        &auto_subcenter_atoms,
    );
    Some(LiveStoreParsedAtomEvent {
        route_key,
        bucket_key,
        route_id,
        bucket_id,
        atom_ids,
        safe_atom_count: safe_atoms.len(),
        bucket_selector_candidate_atoms,
        selected_bucket_atoms,
        auto_subcenter_atoms,
        auto_subcenter_bucket_ids,
        bucket_refinement_depth,
        verified_safe_accept,
        exact_cache_hit,
        request_fingerprint,
        exact_cache_key: row_exact_cache_key,
        trace_id: super::super::json_string(row, &["trace_id"]),
        input_trace_path: super::super::json_string(row, &["input_trace_path"]),
        event_timestamp: super::super::json_string(row, &["event_timestamp"])
            .or_else(|| super::super::json_string(row, &["time_ms"])),
        tokens: token_cost.total_tokens as u64,
        cost_microusd: token_cost.total_cost_microusd,
        cost_estimate_used: false,
    })
}

pub(super) fn live_store_forbidden_atom(atom: &str) -> bool {
    [
        "output_hash64:",
        "verifier_label:",
        "verified_safe_accept:",
        "candidate_verified_safe_accept:",
        "candidate_result_label:",
        "exact_cache_key:",
        "request_fingerprint:",
        "trace_id:",
        "source_trace_id:",
        "target_id:",
        "proof_rule_id:",
        "concrete_x_lookup:",
        "manual_local_out_t:",
    ]
    .iter()
    .any(|prefix| atom.starts_with(prefix))
}

impl LiveStoreAdaptiveBucketPolicy {
    pub(super) fn route_depth(&self, route_key: &str) -> usize {
        self.route_depths.get(route_key).copied().unwrap_or(0)
    }

    pub(super) fn max_depth(&self) -> usize {
        self.route_depths.values().copied().max().unwrap_or(0)
    }

    pub(super) fn observe_decision(
        &mut self,
        event: &LiveStoreParsedAtomEvent,
        decision: PhaseCenterOnlineDecision,
    ) {
        if decision.false_accept || self.should_refine_on_learning_pressure(event, decision) {
            self.observe_rejected_bucket(event);
        }
    }

    pub(super) fn observe_rejected_bucket(&mut self, event: &LiveStoreParsedAtomEvent) {
        let depth = self
            .route_depths
            .entry(event.route_key.clone())
            .or_default();
        let next_depth = (*depth + 1).min(LIVE_STORE_MAX_BUCKET_REFINEMENT_DEPTH);
        if next_depth != *depth {
            *depth = next_depth;
            self.refinement_count += 1;
        }
    }

    fn should_refine_on_learning_pressure(
        &self,
        event: &LiveStoreParsedAtomEvent,
        decision: PhaseCenterOnlineDecision,
    ) -> bool {
        if event.bucket_refinement_depth >= LIVE_STORE_MAX_BUCKET_REFINEMENT_DEPTH {
            return false;
        }
        if !decision.active_before_update || decision.calibration_event {
            return false;
        }
        if decision.local_operator_shadow_decision || decision.unique_cpu_accept_over_exact_cache {
            return false;
        }
        if event.verified_safe_accept {
            return false;
        }
        true
    }
}

pub(super) fn live_store_route_key(row: &serde_json::Value) -> String {
    super::super::phase_atom_string_vec(row, "route_hint_atoms")
        .into_iter()
        .next()
        .or_else(|| {
            super::super::phase_atom_action_families(&super::super::phase_atom_string_vec(
                row,
                "action_atoms",
            ))
            .into_iter()
            .next()
        })
        .or_else(|| super::super::json_string(row, &["traffic_source"]))
        .unwrap_or_else(|| "unknown_route".to_owned())
}

pub(super) fn live_store_action_family_route_id_from_row(row: &serde_json::Value) -> Option<u32> {
    super::super::phase_atom_action_families(&super::super::phase_atom_string_vec(
        row,
        "action_atoms",
    ))
    .into_iter()
    .next()
    .map(|route_key| live_store_hash_id(["live_store_route", route_key.as_str()]))
}

pub(super) fn live_store_bucket_selector_candidate_atoms(
    safe_atoms: &[String],
    refinement_depth: usize,
) -> Vec<String> {
    let mut candidates = safe_atoms
        .iter()
        .filter(|atom| live_store_bucket_selector(atom, refinement_depth))
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    candidates.truncate(12);
    candidates
}

pub(super) fn live_store_selected_bucket_atoms_from_candidates(
    candidates: &[String],
) -> Vec<String> {
    let mut selected = candidates
        .iter()
        .filter(|atom| live_store_false_accept_split_atom_refinement_blocker(atom) == "none")
        .cloned()
        .collect::<Vec<_>>();
    selected.sort();
    selected.dedup();
    selected.truncate(12);
    selected
}

pub(super) fn live_store_auto_refinement_candidate_atoms_from_row(
    row: &serde_json::Value,
    selected_bucket_atoms: &[String],
) -> Vec<String> {
    let selected_bucket_atoms = selected_bucket_atoms
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut candidates = live_store_safe_atoms(row)
        .into_iter()
        .filter(|atom| !selected_bucket_atoms.contains(atom))
        .filter(|atom| live_store_bucket_selector(atom, LIVE_STORE_MAX_BUCKET_REFINEMENT_DEPTH))
        .filter(|atom| live_store_false_accept_split_atom_refinement_blocker(atom) == "none")
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    candidates.truncate(12);
    candidates
}

pub(super) fn live_store_bucket_selector(atom: &str, refinement_depth: usize) -> bool {
    if refinement_depth == 0 {
        return false;
    }
    let depth_one = [
        "request_route_family:",
        "action_family:",
        "route_operator:",
        "tool_call_fingerprint_count_band:",
        "tool_call_fingerprint_present:",
    ]
    .iter()
    .any(|prefix| atom.starts_with(prefix));
    if refinement_depth == 1 {
        return depth_one;
    }
    let depth_two = depth_one
        || [
            "request_route_family:",
            "request_token_band:",
            "action_family:",
            "route_operator:",
            "shadow_active_fringe_len_band:",
            "shadow_slot_count_band:",
            "tool_call_fingerprint_count_band:",
            "tool_call_fingerprint_present:",
        ]
        .iter()
        .any(|prefix| atom.starts_with(prefix));
    if refinement_depth == 2 {
        return depth_two;
    }
    depth_two
        || [
            "domain_family:",
            "request_command_kind:",
            "tool_command_kind:",
            "tool_command_shell_family:",
            "tool_check_kind:",
            "state_exit_code_band:",
        ]
        .iter()
        .any(|prefix| atom.starts_with(prefix))
}

pub(super) fn live_store_bucket_key(route_key: &str, selected: &[String]) -> String {
    if selected.is_empty() {
        route_key.to_owned()
    } else {
        format!("{route_key}::{}", selected.join("|"))
    }
}

pub(super) fn live_store_false_accept_split_atom_refinement_blocker(atom: &str) -> &'static str {
    if atom.starts_with("state_source:") {
        return "source_identity_not_operator_refinement";
    }
    if atom.starts_with("state_session_bucket:") {
        return "session_identity_not_operator_refinement";
    }
    if atom.starts_with("request_route_family:") || atom.starts_with("route_hint:") {
        return "route_family_too_broad_for_operator_refinement";
    }
    if atom.starts_with("route_operator:") || atom.starts_with("profile_id:") {
        return "route_identity_too_broad_for_operator_refinement";
    }
    if atom.starts_with("token_band:")
        || atom.starts_with("cost_band:")
        || atom.starts_with("request_token_band:")
        || atom.starts_with("request_cost_band:")
        || atom.starts_with("state_token_band:")
        || atom.starts_with("state_cost_band:")
    {
        return "token_or_cost_band_not_operator_refinement";
    }
    if atom.starts_with("shadow_") {
        return "shadow_payload_shape_not_operator_refinement";
    }
    if atom.starts_with("tool_call_fingerprint_count_band:")
        || atom.starts_with("tool_call_fingerprint_present:")
    {
        return "tool_count_shape_not_operator_refinement";
    }
    if atom.contains("_cwd_kind:") {
        return "cwd_identity_not_operator_refinement";
    }
    if atom == "request_command_kind:other"
        || atom == "tool_command_kind:other"
        || atom == "tool_command_shell_family:other"
    {
        return "generic_other_command_family_too_broad";
    }
    if atom.starts_with("request_command_arg_band:") {
        return "command_length_band_shape_only";
    }
    if atom == "state_tool_status_evidence:other" {
        return "generic_command_evidence_not_status_refinement";
    }
    if atom.starts_with("state_output_char_band:")
        || atom.starts_with("state_output_line_band:")
        || atom.starts_with("state_output_has_warning_marker:")
        || atom.starts_with("state_output_has_error_marker:")
        || atom.starts_with("state_output_marker:")
    {
        return "output_marker_or_size_band_needs_stronger_verifier_atoms";
    }
    if atom.starts_with("action:") {
        return "action_or_domain_family_too_broad";
    }
    "none"
}

pub(super) fn live_store_hash_id<'a, I>(parts: I) -> u32
where
    I: IntoIterator<Item = &'a str>,
{
    (super::super::stable_fingerprint(parts) & 0xffff_ffff) as u32
}
