use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::{
    generic_count_band, json_bool, json_string, per_thousand, phase_atom_action_families,
    phase_atom_binary_token_cost, phase_atom_string_vec, write_json_file,
};

const DEFAULT_CONSTRAINED_SPLIT_MINER_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-constrained-split-miner-v1.report.json";
const MIN_POLICY_SUPPORT: usize = 4;
const MIN_FUTURE_ROWS: usize = 1;
const MAX_SELECTED_CHILDREN_PER_CLASS: usize = 16;
const MAX_REJECTED_CHILDREN_PER_CLASS: usize = 16;
const MAX_CLASS_REPORTS: usize = 96;
const MAX_ATOMS_PER_ROW_FOR_PAIRS: usize = 48;

#[derive(Clone)]
struct SplitSourceRow {
    stream_index: usize,
    broad_class_id: String,
    request_fingerprint: String,
    exact_cache_hit: bool,
    verified_safe_accept: Option<bool>,
    total_tokens: usize,
    total_cost_microusd: u64,
    atoms: Vec<String>,
}

#[derive(Default)]
struct SplitMinerState {
    total_rows: usize,
    rows_with_action_family: usize,
    rows_without_action_family: usize,
    rows_with_verifier_label: usize,
    rows_without_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    exact_cache_hits: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    source_rows: Vec<SplitSourceRow>,
    seen_exact_cache_keys: BTreeSet<String>,
}

#[derive(Default)]
struct BroadClassInput {
    broad_class_id: String,
    rows: Vec<SplitSourceRow>,
}

#[derive(Default)]
struct SplitCandidateCounts {
    policy_rows: usize,
    policy_true: usize,
    policy_false: usize,
    policy_non_exact_true: usize,
    policy_tokens_over_exact_cache: usize,
    policy_cost_microusd_over_exact_cache: u64,
    future_rows: usize,
    future_true: usize,
    future_false: usize,
    future_non_exact_true: usize,
    future_tokens_over_exact_cache: usize,
    future_cost_microusd_over_exact_cache: u64,
    future_exact_cache_hits: usize,
    future_fingerprints: BTreeSet<String>,
}

#[derive(Clone, Serialize)]
struct SelectedSplitReport {
    split_rule: String,
    support: usize,
    future_rows: usize,
    marginal_accepts_over_exact_cache: usize,
    tokens_saved: usize,
    cost_saved_microusd: u64,
    false_accepts: usize,
    information_gain: usize,
    mdl_penalty: usize,
    net_gain: i64,
    verifier_ready_rows: usize,
    cache_overlap: usize,
}

#[derive(Clone, Serialize)]
struct RejectedSplitReport {
    split_rule: String,
    reason: &'static str,
    support: usize,
    future_rows: usize,
    marginal_accepts_over_exact_cache: usize,
    false_accepts: usize,
    information_gain: usize,
    net_gain: i64,
}

#[derive(Serialize)]
struct BroadClassReport {
    broad_class_id: String,
    broad_class_rows: usize,
    verifier_ready_rows: usize,
    rows_missing_verifier: usize,
    broad_class_true_rows: usize,
    broad_class_false_accepts: usize,
    candidate_split_count: usize,
    selected_split_count: usize,
    rejected_split_count: usize,
    policy_rows: usize,
    future_rows: usize,
    selected_children: Vec<SelectedSplitReport>,
    rejected_children: Vec<RejectedSplitReport>,
    selected_future_unique_accepts_over_exact_cache: usize,
    selected_future_tokens_saved: usize,
    selected_future_cost_saved_microusd: u64,
    selected_future_false_accepts: usize,
    exact_cache_overlap_milli: usize,
}

#[derive(Default, Serialize)]
struct GlobalDeltaReport {
    before_accepts: usize,
    after_accepts: usize,
    before_tokens: usize,
    after_tokens: usize,
    before_cost_microusd: u64,
    after_cost_microusd: u64,
    false_accepts: usize,
}

pub(crate) fn run_phase_stream_constrained_split_miner_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONSTRAINED_SPLIT_MINER_REPORT));
    let input_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let mut state = SplitMinerState::default();
    for path in &input_paths {
        scan_split_miner_path(path, &mut state)?;
    }

    let mut by_class = BTreeMap::<String, BroadClassInput>::new();
    for row in state.source_rows {
        by_class
            .entry(row.broad_class_id.clone())
            .or_insert_with(|| BroadClassInput {
                broad_class_id: row.broad_class_id.clone(),
                rows: Vec::new(),
            })
            .rows
            .push(row);
    }

    let mut global_seen = BTreeSet::<String>::new();
    let mut reports = by_class
        .into_values()
        .map(|class| broad_class_report(class, &mut global_seen))
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| {
        right
            .selected_future_tokens_saved
            .cmp(&left.selected_future_tokens_saved)
            .then_with(|| {
                right
                    .selected_future_unique_accepts_over_exact_cache
                    .cmp(&left.selected_future_unique_accepts_over_exact_cache)
            })
            .then_with(|| right.candidate_split_count.cmp(&left.candidate_split_count))
            .then_with(|| left.broad_class_id.cmp(&right.broad_class_id))
    });
    let full_class_count = reports.len();
    let selected_class_count = reports
        .iter()
        .filter(|class| class.selected_split_count > 0)
        .count();
    let candidate_split_count = reports
        .iter()
        .map(|class| class.candidate_split_count)
        .sum::<usize>();
    let selected_split_count = reports
        .iter()
        .map(|class| class.selected_split_count)
        .sum::<usize>();
    let rejected_split_count = reports
        .iter()
        .map(|class| class.rejected_split_count)
        .sum::<usize>();
    let global_delta = GlobalDeltaReport {
        before_accepts: 0,
        after_accepts: reports
            .iter()
            .map(|class| class.selected_future_unique_accepts_over_exact_cache)
            .sum(),
        before_tokens: 0,
        after_tokens: reports
            .iter()
            .map(|class| class.selected_future_tokens_saved)
            .sum(),
        before_cost_microusd: 0,
        after_cost_microusd: reports
            .iter()
            .map(|class| class.selected_future_cost_saved_microusd)
            .sum(),
        false_accepts: reports
            .iter()
            .map(|class| class.selected_future_false_accepts)
            .sum(),
    };
    let verdict = if global_delta.false_accepts > 0 {
        "PHASE_STREAM_CONSTRAINED_SPLIT_MINER_V1_FAIL_FALSE_ACCEPTS"
    } else if selected_split_count == 0 || global_delta.after_accepts == 0 {
        "PHASE_STREAM_CONSTRAINED_SPLIT_MINER_V1_WATCH_NO_SAFE_GLOBAL_DELTA"
    } else if global_delta.after_accepts < 10 || global_delta.after_tokens < 1000 {
        "PHASE_STREAM_CONSTRAINED_SPLIT_MINER_V1_WATCH_TINY_SAFE_DELTA"
    } else {
        "PHASE_STREAM_CONSTRAINED_SPLIT_MINER_V1_PASS_SAFE_AUTOMATIC_DELTA"
    };

    reports.truncate(MAX_CLASS_REPORTS);
    let report = serde_json::json!({
        "report_kind": "phase_stream_constrained_split_miner_v1",
        "input_paths": input_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>(),
        "total_rows": state.total_rows,
        "rows_with_action_family": state.rows_with_action_family,
        "rows_without_action_family": state.rows_without_action_family,
        "rows_with_verifier_label": state.rows_with_verifier_label,
        "rows_without_verifier_label": state.rows_without_verifier_label,
        "verifier_true_rows": state.verifier_true_rows,
        "verifier_false_rows": state.verifier_false_rows,
        "exact_cache_hits": state.exact_cache_hits,
        "total_tokens": state.total_tokens,
        "total_cost_microusd": state.total_cost_microusd,
        "broad_class_count": full_class_count,
        "selected_broad_class_count": selected_class_count,
        "candidate_split_count": candidate_split_count,
        "selected_split_count": selected_split_count,
        "rejected_split_count": rejected_split_count,
        "global_delta": global_delta,
        "classes": reports,
        "selection_policy": {
            "candidate_generator": "automatic observable atom splits from action/state/result/route/context/token-cost bands",
            "policy_window": "first half of verifier-ready rows per broad action class",
            "future_window": "second half of verifier-ready rows per broad action class",
            "constraint": "select child only when policy_false_accepts = 0 and future_false_accepts = 0",
            "objective": "maximize future unique accepts/tokens/cost over exact cache minus cache-overlap and MDL penalties",
            "minimum_policy_support": MIN_POLICY_SUPPORT,
            "minimum_future_rows": MIN_FUTURE_ROWS
        },
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": false,
        "verdict": verdict,
        "boundary": "cold constrained split miner only: generates automatic split candidates from real stream evidence and measures safe future denominator delta; does not compile .nwpc, mutate runtime, promote, serve, enable local_accept, claim market money, or use legacy nwrb/role-binding paths"
    });
    write_json_file(&report_path, &report)?;

    println!("phase_stream_constrained_split_miner_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {}", state.total_rows);
    println!("  broad_class_count: {full_class_count}");
    println!("  candidate_split_count: {candidate_split_count}");
    println!("  selected_split_count: {selected_split_count}");
    println!(
        "  safe_future_accepts_over_exact_cache: {}",
        global_delta.after_accepts
    );
    println!("  safe_future_tokens_saved: {}", global_delta.after_tokens);
    println!("  false_accepts: {}", global_delta.false_accepts);
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn scan_split_miner_path(path: &Path, state: &mut SplitMinerState) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read phase constrained split input '{}': {error}",
            path.display()
        )
    })?;
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse phase constrained split input '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if row.is_object() {
            scan_split_miner_row(&row, path, line_index, state);
        }
    }
    Ok(())
}

fn scan_split_miner_row(row: &Value, path: &Path, line_index: usize, state: &mut SplitMinerState) {
    state.total_rows += 1;
    let action_atoms = phase_atom_string_vec(row, "action_atoms");
    let action_families = phase_atom_action_families(&action_atoms);
    if action_families.is_empty() {
        state.rows_without_action_family += 1;
        return;
    }
    state.rows_with_action_family += 1;

    let verified_safe_accept = json_bool(row, &["verified_safe_accept"]);
    state.rows_with_verifier_label += usize::from(verified_safe_accept.is_some());
    state.rows_without_verifier_label += usize::from(verified_safe_accept.is_none());
    state.verifier_true_rows += usize::from(verified_safe_accept == Some(true));
    state.verifier_false_rows += usize::from(verified_safe_accept == Some(false));

    let exact_cache_key = json_string(row, &["exact_cache_key"])
        .or_else(|| json_string(row, &["request_fingerprint"]))
        .unwrap_or_else(|| format!("{}:{}", path.display(), line_index + 1));
    let exact_cache_hit = !state.seen_exact_cache_keys.insert(exact_cache_key);
    state.exact_cache_hits += usize::from(exact_cache_hit);

    let request_fingerprint = json_string(row, &["request_fingerprint"])
        .unwrap_or_else(|| format!("phase-constrained-split-row:{}", state.total_rows));
    let token_cost = phase_atom_binary_token_cost(row);
    state.total_tokens = state.total_tokens.saturating_add(token_cost.total_tokens);
    state.total_cost_microusd = state
        .total_cost_microusd
        .saturating_add(token_cost.total_cost_microusd);
    let atoms =
        constrained_split_atoms(row, token_cost.total_tokens, token_cost.total_cost_microusd);
    if atoms.is_empty() {
        return;
    }

    for action_family in action_families {
        state.source_rows.push(SplitSourceRow {
            stream_index: state.total_rows,
            broad_class_id: action_family,
            request_fingerprint: request_fingerprint.clone(),
            exact_cache_hit,
            verified_safe_accept,
            total_tokens: token_cost.total_tokens,
            total_cost_microusd: token_cost.total_cost_microusd,
            atoms: atoms.clone(),
        });
    }
}

fn constrained_split_atoms(
    row: &Value,
    total_tokens: usize,
    total_cost_microusd: u64,
) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    for (group, key) in [
        ("request", "request_atoms"),
        ("state", "state_atoms"),
        ("action", "action_atoms"),
        ("tool", "tool_atoms"),
        ("result", "result_atoms"),
        ("route", "route_hint_atoms"),
    ] {
        for atom in phase_atom_string_vec(row, key) {
            if constrained_split_atom_allowed(&atom) {
                atoms.insert(format!("{group}::{atom}"));
            }
        }
    }
    let has_shadow_request = json_bool(row, &["has_shadow_request"]).unwrap_or(false)
        || row
            .get("nando_shadow_request")
            .is_some_and(serde_json::Value::is_object);
    atoms.insert(format!(
        "context::request_has_shadow_request:{has_shadow_request}"
    ));
    atoms.insert(format!(
        "token::token_band:{}",
        generic_count_band(total_tokens)
    ));
    atoms.insert(format!(
        "cost::cost_band:{}",
        generic_count_band(total_cost_microusd as usize)
    ));
    atoms.into_iter().collect()
}

fn constrained_split_atom_allowed(atom: &str) -> bool {
    if atom.is_empty() {
        return false;
    }
    let lower = atom.to_ascii_lowercase();
    !atom.starts_with("output_hash64:")
        && !atom.starts_with("verifier_label:")
        && !atom.starts_with("verified_safe_accept:")
        && !atom.starts_with("request_fingerprint:")
        && !atom.starts_with("exact_cache_key:")
        && !atom.starts_with("trace_id:")
        && !atom.starts_with("source_trace_id:")
        && !atom.starts_with("state_session_bucket:")
        && !atom.starts_with("action_family:")
        && !lower.contains("target_id")
        && !lower.contains("proof_rule")
        && !lower.contains("local_out_t")
        && !lower.contains("concrete_x")
        && !lower.contains("nwrb")
        && !lower.contains("role_binding")
}

fn broad_class_report(
    mut class: BroadClassInput,
    global_seen: &mut BTreeSet<String>,
) -> BroadClassReport {
    class
        .rows
        .sort_by(|left, right| left.stream_index.cmp(&right.stream_index));
    let broad_class_rows = class.rows.len();
    let verifier_rows = class
        .rows
        .iter()
        .filter(|row| row.verified_safe_accept.is_some())
        .cloned()
        .collect::<Vec<_>>();
    let verifier_ready_rows = verifier_rows.len();
    let rows_missing_verifier = broad_class_rows.saturating_sub(verifier_ready_rows);
    let broad_class_true_rows = verifier_rows
        .iter()
        .filter(|row| row.verified_safe_accept == Some(true))
        .count();
    let broad_class_false_accepts = verifier_rows
        .iter()
        .filter(|row| row.verified_safe_accept == Some(false))
        .count();
    let exact_cache_hits = class.rows.iter().filter(|row| row.exact_cache_hit).count();
    let (policy_rows, future_rows) = split_policy_future(verifier_rows);
    let candidate_map = split_candidate_counts(&policy_rows, &future_rows);
    let candidate_split_count = candidate_map.len();

    let parent_policy_true = policy_rows
        .iter()
        .filter(|row| row.verified_safe_accept == Some(true))
        .count();
    let parent_policy_false = policy_rows.len().saturating_sub(parent_policy_true);
    let mut selected = Vec::new();
    let mut rejected = Vec::new();
    let mut candidates = candidate_map
        .into_iter()
        .map(|(split_rule, counts)| {
            let information_gain =
                information_gain_milli(parent_policy_true, parent_policy_false, &counts);
            let mdl_penalty = split_rule.len().div_ceil(16).max(1);
            let cache_overlap_penalty = counts.future_exact_cache_hits.saturating_mul(100);
            let false_accepts = counts.policy_false.saturating_add(counts.future_false);
            let net_gain = counts
                .future_tokens_over_exact_cache
                .saturating_add(counts.future_non_exact_true.saturating_mul(1000))
                .saturating_add(information_gain)
                .saturating_sub(mdl_penalty)
                .saturating_sub(cache_overlap_penalty) as i64;
            (
                split_rule,
                counts,
                information_gain,
                mdl_penalty,
                net_gain,
                false_accepts,
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .4
            .cmp(&left.4)
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.0.cmp(&right.0))
    });

    for (split_rule, counts, information_gain, mdl_penalty, net_gain, false_accepts) in candidates {
        let support = counts.policy_rows;
        let future_rows_count = counts.future_rows;
        let reject_reason = split_reject_reason(&counts, false_accepts);
        if reject_reason.is_none() {
            let mut marginal_accepts = 0usize;
            let mut marginal_tokens = 0usize;
            let mut marginal_cost = 0u64;
            for fingerprint in &counts.future_fingerprints {
                if global_seen.insert(fingerprint.clone()) {
                    marginal_accepts += 1;
                }
            }
            if marginal_accepts > 0 {
                marginal_tokens = counts.future_tokens_over_exact_cache;
                marginal_cost = counts.future_cost_microusd_over_exact_cache;
            }
            if marginal_accepts > 0 && selected.len() < MAX_SELECTED_CHILDREN_PER_CLASS {
                selected.push(SelectedSplitReport {
                    split_rule,
                    support,
                    future_rows: future_rows_count,
                    marginal_accepts_over_exact_cache: marginal_accepts,
                    tokens_saved: marginal_tokens,
                    cost_saved_microusd: marginal_cost,
                    false_accepts,
                    information_gain,
                    mdl_penalty,
                    net_gain,
                    verifier_ready_rows: counts.policy_rows.saturating_add(counts.future_rows),
                    cache_overlap: per_thousand(counts.future_exact_cache_hits, counts.future_rows),
                });
            }
        } else if rejected.len() < MAX_REJECTED_CHILDREN_PER_CLASS {
            rejected.push(RejectedSplitReport {
                split_rule,
                reason: reject_reason.unwrap_or("unknown"),
                support,
                future_rows: future_rows_count,
                marginal_accepts_over_exact_cache: counts.future_non_exact_true,
                false_accepts,
                information_gain,
                net_gain,
            });
        }
    }
    let selected_future_unique_accepts_over_exact_cache = selected
        .iter()
        .map(|child| child.marginal_accepts_over_exact_cache)
        .sum();
    let selected_future_tokens_saved = selected.iter().map(|child| child.tokens_saved).sum();
    let selected_future_cost_saved_microusd =
        selected.iter().map(|child| child.cost_saved_microusd).sum();
    let selected_future_false_accepts = selected.iter().map(|child| child.false_accepts).sum();

    BroadClassReport {
        broad_class_id: class.broad_class_id,
        broad_class_rows,
        verifier_ready_rows,
        rows_missing_verifier,
        broad_class_true_rows,
        broad_class_false_accepts,
        candidate_split_count,
        selected_split_count: selected.len(),
        rejected_split_count: candidate_split_count.saturating_sub(selected.len()),
        policy_rows: policy_rows.len(),
        future_rows: future_rows.len(),
        selected_children: selected,
        rejected_children: rejected,
        selected_future_unique_accepts_over_exact_cache,
        selected_future_tokens_saved,
        selected_future_cost_saved_microusd,
        selected_future_false_accepts,
        exact_cache_overlap_milli: per_thousand(exact_cache_hits, broad_class_rows),
    }
}

fn split_policy_future(
    mut rows: Vec<SplitSourceRow>,
) -> (Vec<SplitSourceRow>, Vec<SplitSourceRow>) {
    if rows.len() < 2 {
        return (rows, Vec::new());
    }
    rows.sort_by(|left, right| left.stream_index.cmp(&right.stream_index));
    let policy_len = (rows.len() / 2).clamp(1, rows.len().saturating_sub(1));
    let future = rows.split_off(policy_len);
    (rows, future)
}

fn split_candidate_counts(
    policy_rows: &[SplitSourceRow],
    future_rows: &[SplitSourceRow],
) -> BTreeMap<String, SplitCandidateCounts> {
    let mut candidates = BTreeMap::<String, SplitCandidateCounts>::new();
    for row in policy_rows {
        for rule in row_split_rules(row) {
            let counts = candidates.entry(rule).or_default();
            update_candidate_counts(counts, row, false);
        }
    }
    for row in future_rows {
        for rule in row_split_rules(row) {
            let counts = candidates.entry(rule).or_default();
            update_candidate_counts(counts, row, true);
        }
    }
    candidates
}

fn row_split_rules(row: &SplitSourceRow) -> Vec<String> {
    let mut atoms = row.atoms.clone();
    atoms.sort();
    atoms.dedup();
    let pair_atoms = atoms
        .iter()
        .filter(|atom| pair_split_atom_allowed(atom))
        .take(MAX_ATOMS_PER_ROW_FOR_PAIRS)
        .cloned()
        .collect::<Vec<_>>();
    let mut rules = atoms;
    for left_index in 0..pair_atoms.len() {
        for right in pair_atoms.iter().skip(left_index + 1) {
            rules.push(format!("pair::{} && {}", pair_atoms[left_index], right));
        }
    }
    rules
}

fn pair_split_atom_allowed(atom: &str) -> bool {
    !atom.starts_with("token::")
        && !atom.starts_with("cost::")
        && !atom.starts_with("context::request_has_shadow_request:")
}

fn update_candidate_counts(counts: &mut SplitCandidateCounts, row: &SplitSourceRow, future: bool) {
    let positive = row.verified_safe_accept == Some(true);
    let negative = row.verified_safe_accept == Some(false);
    if future {
        counts.future_rows += 1;
        counts.future_true += usize::from(positive);
        counts.future_false += usize::from(negative);
        counts.future_exact_cache_hits += usize::from(row.exact_cache_hit);
        if positive && !row.exact_cache_hit {
            counts.future_non_exact_true += 1;
            counts.future_tokens_over_exact_cache = counts
                .future_tokens_over_exact_cache
                .saturating_add(row.total_tokens);
            counts.future_cost_microusd_over_exact_cache = counts
                .future_cost_microusd_over_exact_cache
                .saturating_add(row.total_cost_microusd);
            counts
                .future_fingerprints
                .insert(row.request_fingerprint.clone());
        }
    } else {
        counts.policy_rows += 1;
        counts.policy_true += usize::from(positive);
        counts.policy_false += usize::from(negative);
        if positive && !row.exact_cache_hit {
            counts.policy_non_exact_true += 1;
            counts.policy_tokens_over_exact_cache = counts
                .policy_tokens_over_exact_cache
                .saturating_add(row.total_tokens);
            counts.policy_cost_microusd_over_exact_cache = counts
                .policy_cost_microusd_over_exact_cache
                .saturating_add(row.total_cost_microusd);
        }
    }
}

fn split_reject_reason(
    counts: &SplitCandidateCounts,
    false_accepts: usize,
) -> Option<&'static str> {
    if counts.policy_rows < MIN_POLICY_SUPPORT {
        return Some("below_min_policy_support");
    }
    if counts.future_rows < MIN_FUTURE_ROWS {
        return Some("missing_future_window");
    }
    if counts.policy_true == 0 {
        return Some("no_policy_safe_evidence");
    }
    if counts.future_non_exact_true == 0 {
        return Some("no_future_accepts_over_exact_cache");
    }
    if false_accepts > 0 {
        return Some("false_accept_constraint_failed");
    }
    None
}

fn information_gain_milli(
    parent_policy_true: usize,
    parent_policy_false: usize,
    counts: &SplitCandidateCounts,
) -> usize {
    let parent_total = parent_policy_true.saturating_add(parent_policy_false);
    if parent_total == 0 || counts.policy_rows == 0 || counts.policy_rows >= parent_total {
        return 0;
    }
    let child_true = counts.policy_true;
    let child_false = counts.policy_false;
    let other_true = parent_policy_true.saturating_sub(child_true);
    let other_false = parent_policy_false.saturating_sub(child_false);
    let parent_gini = gini_milli(parent_policy_true, parent_policy_false);
    let child_total = child_true.saturating_add(child_false);
    let other_total = other_true.saturating_add(other_false);
    let weighted_child = gini_milli(child_true, child_false).saturating_mul(child_total);
    let weighted_other = gini_milli(other_true, other_false).saturating_mul(other_total);
    let weighted = weighted_child.saturating_add(weighted_other) / parent_total;
    parent_gini.saturating_sub(weighted)
}

fn gini_milli(positive: usize, negative: usize) -> usize {
    let total = positive.saturating_add(negative);
    if total == 0 {
        return 0;
    }
    let total_sq = total.saturating_mul(total);
    let pure_sq = positive
        .saturating_mul(positive)
        .saturating_add(negative.saturating_mul(negative));
    1000usize.saturating_sub(pure_sq.saturating_mul(1000) / total_sq)
}
