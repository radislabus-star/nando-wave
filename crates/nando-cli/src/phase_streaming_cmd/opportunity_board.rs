use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::{
    json_bool, json_string, phase_atom_action_families, phase_atom_binary_token_cost,
    phase_atom_external_provider_correlation_keys, phase_atom_state_action_bucket_key,
    phase_atom_string_vec, write_json_file,
};

const DEFAULT_OPPORTUNITY_BOARD_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-opportunity-board-v1.report.json";
const OPPORTUNITY_BOARD_TOP_CLASS_LIMIT: usize = 96;

#[derive(Default)]
struct OpportunityBoardState {
    total_rows: usize,
    rows_with_action_family: usize,
    rows_without_action_family: usize,
    rows_with_verifier_label: usize,
    rows_without_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    rows_with_shadow_request: usize,
    rows_with_provider_correlation: usize,
    rows_with_positive_tokens: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    exact_cache_hits: usize,
    seen_exact_cache_keys: BTreeSet<String>,
    classes: BTreeMap<String, OpportunityClassState>,
}

#[derive(Default)]
struct OpportunityClassState {
    bucket_key: String,
    action_family_atom: String,
    class_rows: usize,
    verifier_ready_rows: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    rows_missing_verifier: usize,
    mining_ready_rows: usize,
    score_ready_rows: usize,
    shadow_request_rows: usize,
    provider_correlation_rows: usize,
    exact_cache_hits: usize,
    non_exact_rows: usize,
    non_exact_verifier_true_rows: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    positive_token_rows: usize,
    token_evidence_missing_rows: usize,
    cost_evidence_missing_rows: usize,
    result_atom_rows: usize,
    route_keys: BTreeMap<String, usize>,
    request_route_families: BTreeMap<String, usize>,
    traffic_sources: BTreeMap<String, usize>,
    samples: Vec<String>,
}

#[derive(Serialize)]
struct OpportunityClassReport {
    rank: usize,
    bucket_key: String,
    action_family_atom: String,
    class_rows: usize,
    class_share_milli: usize,
    verifier_ready_rows: usize,
    verifier_coverage_milli: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    rows_missing_verifier: usize,
    mining_ready_rows: usize,
    score_ready_rows: usize,
    shadow_request_rows: usize,
    provider_correlation_rows: usize,
    exact_cache_hits: usize,
    exact_cache_overlap_milli: usize,
    non_exact_rows: usize,
    non_exact_verifier_true_rows: usize,
    total_tokens: usize,
    token_share_milli: usize,
    total_cost_microusd: u64,
    positive_token_rows: usize,
    token_evidence_missing_rows: usize,
    cost_evidence_missing_rows: usize,
    result_atom_rows: usize,
    opportunity_score: u128,
    status: &'static str,
    next_action: &'static str,
    blockers: Vec<&'static str>,
    top_route_keys: Vec<CountReport>,
    top_request_route_families: Vec<CountReport>,
    top_traffic_sources: Vec<CountReport>,
    samples: Vec<String>,
}

#[derive(Serialize)]
struct CountReport {
    key: String,
    count: usize,
}

pub(crate) fn run_phase_stream_opportunity_board_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OPPORTUNITY_BOARD_REPORT));
    let input_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let mut state = OpportunityBoardState::default();
    for path in &input_paths {
        scan_opportunity_path(path, &mut state)?;
    }

    let mut class_reports = state
        .classes
        .values()
        .map(|class| opportunity_class_report(class, &state))
        .collect::<Vec<_>>();
    class_reports.sort_by(|left, right| {
        right
            .opportunity_score
            .cmp(&left.opportunity_score)
            .then_with(|| right.total_tokens.cmp(&left.total_tokens))
            .then_with(|| right.class_rows.cmp(&left.class_rows))
            .then_with(|| left.bucket_key.cmp(&right.bucket_key))
    });
    for (index, row) in class_reports.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    class_reports.truncate(OPPORTUNITY_BOARD_TOP_CLASS_LIMIT);

    let candidate_classes = class_reports
        .iter()
        .filter(|row| row.status == "CANDIDATE")
        .count();
    let verifier_bottleneck_classes = class_reports
        .iter()
        .filter(|row| row.status == "NEEDS_VERIFIER_COVERAGE")
        .count();
    let fat_false_risk_classes = class_reports
        .iter()
        .filter(|row| row.status == "NEEDS_SEPARATOR_OR_VERIFIER_SPLIT")
        .count();
    let verdict = if candidate_classes > 0 {
        "PHASE_STREAM_OPPORTUNITY_BOARD_V1_PASS_CANDIDATES_FOUND"
    } else if fat_false_risk_classes > 0 || verifier_bottleneck_classes > 0 {
        "PHASE_STREAM_OPPORTUNITY_BOARD_V1_WATCH_BOTTLENECKS_FOUND"
    } else if state.total_rows > 0 {
        "PHASE_STREAM_OPPORTUNITY_BOARD_V1_WATCH_NO_CANDIDATES"
    } else {
        "PHASE_STREAM_OPPORTUNITY_BOARD_V1_WATCH_EMPTY_INPUT"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_opportunity_board_v1",
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
        "rows_ready_for_route_family_mining": state.rows_ready_for_route_family_mining,
        "rows_ready_for_existing_shadow_scoring": state.rows_ready_for_existing_shadow_scoring,
        "rows_with_shadow_request": state.rows_with_shadow_request,
        "rows_with_provider_correlation": state.rows_with_provider_correlation,
        "rows_with_positive_tokens": state.rows_with_positive_tokens,
        "total_tokens": state.total_tokens,
        "total_cost_microusd": state.total_cost_microusd,
        "exact_cache_hits": state.exact_cache_hits,
        "class_count": state.classes.len(),
        "candidate_classes": candidate_classes,
        "verifier_bottleneck_classes": verifier_bottleneck_classes,
        "fat_false_risk_classes": fat_false_risk_classes,
        "classes": class_reports,
        "board_policy": {
            "purpose": "cold opportunity board for automatic phase-center discovery; ranks repeated real stream classes by traffic, verifier coverage, exact-cache overlap and token weight",
            "not_authority_for_accept": true,
            "next_stage": "feed high-value CANDIDATE classes to online phase-center miner; improve adapters/verifiers for NEEDS_VERIFIER_COVERAGE or NEEDS_SEPARATOR_OR_VERIFIER_SPLIT"
        },
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "local_accept_without_verifier_used": false
        },
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": false,
        "verdict": verdict,
        "boundary": "opportunity board only: does not mine, compile, score, promote, serve, enable local_accept, claim market money, or use legacy nwrb/role-binding paths"
    });
    write_json_file(&report_path, &report)?;

    println!("phase_stream_opportunity_board_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {}", state.total_rows);
    println!("  class_count: {}", state.classes.len());
    println!("  candidate_classes: {candidate_classes}");
    println!("  verifier_bottleneck_classes: {verifier_bottleneck_classes}");
    println!("  fat_false_risk_classes: {fat_false_risk_classes}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn scan_opportunity_path(path: &Path, state: &mut OpportunityBoardState) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read phase opportunity input '{}': {error}",
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
                "failed to parse phase opportunity input '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if row.is_object() {
            scan_opportunity_row(&row, state);
        }
    }
    Ok(())
}

fn scan_opportunity_row(row: &Value, state: &mut OpportunityBoardState) {
    state.total_rows += 1;
    let action_atoms = phase_atom_string_vec(row, "action_atoms");
    let action_families = phase_atom_action_families(&action_atoms);
    if action_families.is_empty() {
        state.rows_without_action_family += 1;
        return;
    }
    state.rows_with_action_family += 1;

    let request_atoms = phase_atom_string_vec(row, "request_atoms");
    let state_atoms = phase_atom_string_vec(row, "state_atoms");
    let tool_atoms = phase_atom_string_vec(row, "tool_atoms");
    let route_hint_atoms = phase_atom_string_vec(row, "route_hint_atoms");
    let result_atoms = phase_atom_string_vec(row, "result_atoms");
    let token_cost = phase_atom_binary_token_cost(row);
    state.total_tokens = state.total_tokens.saturating_add(token_cost.total_tokens);
    state.total_cost_microusd = state
        .total_cost_microusd
        .saturating_add(token_cost.total_cost_microusd);
    state.rows_with_positive_tokens += usize::from(token_cost.total_tokens > 0);

    let exact_cache_key = json_string(row, &["exact_cache_key"])
        .or_else(|| json_string(row, &["request_fingerprint"]))
        .unwrap_or_else(|| format!("phase-opportunity-row:{}", state.total_rows));
    let exact_cache_hit = !state.seen_exact_cache_keys.insert(exact_cache_key);
    state.exact_cache_hits += usize::from(exact_cache_hit);

    let verifier_label = json_bool(row, &["verified_safe_accept"]);
    state.rows_with_verifier_label += usize::from(verifier_label.is_some());
    state.rows_without_verifier_label += usize::from(verifier_label.is_none());
    state.verifier_true_rows += usize::from(verifier_label == Some(true));
    state.verifier_false_rows += usize::from(verifier_label == Some(false));

    let has_shadow_request = json_bool(row, &["has_shadow_request"]).unwrap_or(false)
        || row
            .get("nando_shadow_request")
            .is_some_and(serde_json::Value::is_object);
    let mining_ready = json_bool(row, &["ready_for_route_family_mining"]).unwrap_or(false);
    let score_ready = json_bool(row, &["ready_for_existing_shadow_scoring"]).unwrap_or(false);
    let has_provider_correlation = !phase_atom_external_provider_correlation_keys(row).is_empty();
    state.rows_with_shadow_request += usize::from(has_shadow_request);
    state.rows_ready_for_route_family_mining += usize::from(mining_ready);
    state.rows_ready_for_existing_shadow_scoring += usize::from(score_ready);
    state.rows_with_provider_correlation += usize::from(has_provider_correlation);

    for action_family in action_families {
        let bucket_key = phase_atom_state_action_bucket_key(
            &action_family,
            &request_atoms,
            &state_atoms,
            &tool_atoms,
            &route_hint_atoms,
        );
        let class =
            state
                .classes
                .entry(bucket_key.clone())
                .or_insert_with(|| OpportunityClassState {
                    bucket_key,
                    action_family_atom: action_family.clone(),
                    ..OpportunityClassState::default()
                });
        class.class_rows += 1;
        class.verifier_ready_rows += usize::from(verifier_label.is_some());
        class.verifier_true_rows += usize::from(verifier_label == Some(true));
        class.verifier_false_rows += usize::from(verifier_label == Some(false));
        class.rows_missing_verifier += usize::from(verifier_label.is_none());
        class.mining_ready_rows += usize::from(mining_ready);
        class.score_ready_rows += usize::from(score_ready);
        class.shadow_request_rows += usize::from(has_shadow_request);
        class.provider_correlation_rows += usize::from(has_provider_correlation);
        class.exact_cache_hits += usize::from(exact_cache_hit);
        class.non_exact_rows += usize::from(!exact_cache_hit);
        class.non_exact_verifier_true_rows +=
            usize::from(!exact_cache_hit && verifier_label == Some(true));
        class.total_tokens = class.total_tokens.saturating_add(token_cost.total_tokens);
        class.total_cost_microusd = class
            .total_cost_microusd
            .saturating_add(token_cost.total_cost_microusd);
        class.positive_token_rows += usize::from(token_cost.total_tokens > 0);
        class.token_evidence_missing_rows += usize::from(token_cost.token_evidence_missing);
        class.cost_evidence_missing_rows += usize::from(token_cost.cost_evidence_missing);
        class.result_atom_rows += usize::from(!result_atoms.is_empty());
        for atom in &route_hint_atoms {
            if atom.starts_with("route_key:") {
                *class.route_keys.entry(atom.clone()).or_default() += 1;
            }
        }
        for atom in &request_atoms {
            if atom.starts_with("request_route_family:") {
                *class
                    .request_route_families
                    .entry(atom.clone())
                    .or_default() += 1;
            }
        }
        if let Some(traffic_source) = json_string(row, &["traffic_source"]) {
            *class.traffic_sources.entry(traffic_source).or_default() += 1;
        }
        if class.samples.len() < 3 {
            let trace_id = json_string(row, &["trace_id"])
                .or_else(|| json_string(row, &["request_fingerprint"]))
                .unwrap_or_else(|| format!("row:{}", state.total_rows));
            class.samples.push(trace_id);
        }
    }
}

fn opportunity_class_report(
    class: &OpportunityClassState,
    state: &OpportunityBoardState,
) -> OpportunityClassReport {
    let class_share_milli = milli(class.class_rows, state.total_rows);
    let verifier_coverage_milli = milli(class.verifier_ready_rows, class.class_rows);
    let exact_cache_overlap_milli = milli(class.exact_cache_hits, class.class_rows);
    let token_share_milli = milli(class.total_tokens, state.total_tokens);
    let mut blockers = Vec::new();
    if class.verifier_ready_rows == 0 {
        blockers.push("no_verifier_ready_rows");
    }
    if class.verifier_true_rows == 0 {
        blockers.push("no_verified_true_rows");
    }
    if class.verifier_false_rows == 0 {
        blockers.push("no_verified_false_rows");
    }
    if class.non_exact_verifier_true_rows == 0 {
        blockers.push("no_non_exact_verified_true_rows");
    }
    if class.total_tokens == 0 {
        blockers.push("no_token_denominator");
    }
    if class.verifier_ready_rows < 8 {
        blockers.push("low_verifier_support");
    }
    if class.rows_missing_verifier > class.verifier_ready_rows {
        blockers.push("verifier_coverage_bottleneck");
    }

    let status = if class.verifier_ready_rows >= 8
        && class.verifier_true_rows > 0
        && class.verifier_false_rows > 0
        && class.non_exact_verifier_true_rows > 0
        && class.total_tokens > 0
    {
        "CANDIDATE"
    } else if class.total_tokens >= 10_000
        && class.verifier_true_rows > 0
        && class.verifier_false_rows > 0
        && class.non_exact_verifier_true_rows > 0
    {
        "NEEDS_SEPARATOR_OR_VERIFIER_SPLIT"
    } else if class.class_rows >= 20 && class.rows_missing_verifier > class.verifier_ready_rows {
        "NEEDS_VERIFIER_COVERAGE"
    } else {
        "WATCH"
    };
    let next_action = match status {
        "CANDIDATE" => "run_online_phase_center_miner_then_selector_on_this_bucket_family",
        "NEEDS_SEPARATOR_OR_VERIFIER_SPLIT" => {
            "add_non_target_state_result_atoms_or_verifier_split_before_promoting"
        }
        "NEEDS_VERIFIER_COVERAGE" => {
            "extend_l4_adapter_to_emit_verifier_labels_for_this_repeated_class"
        }
        _ => "keep_collecting_stream_or_ignore_until_support_grows",
    };
    let opportunity_score = (class.non_exact_verifier_true_rows as u128)
        .saturating_mul(class.total_tokens as u128)
        .saturating_add((class.verifier_ready_rows as u128).saturating_mul(1_000))
        .saturating_add(class.class_rows as u128);
    OpportunityClassReport {
        rank: 0,
        bucket_key: class.bucket_key.clone(),
        action_family_atom: class.action_family_atom.clone(),
        class_rows: class.class_rows,
        class_share_milli,
        verifier_ready_rows: class.verifier_ready_rows,
        verifier_coverage_milli,
        verifier_true_rows: class.verifier_true_rows,
        verifier_false_rows: class.verifier_false_rows,
        rows_missing_verifier: class.rows_missing_verifier,
        mining_ready_rows: class.mining_ready_rows,
        score_ready_rows: class.score_ready_rows,
        shadow_request_rows: class.shadow_request_rows,
        provider_correlation_rows: class.provider_correlation_rows,
        exact_cache_hits: class.exact_cache_hits,
        exact_cache_overlap_milli,
        non_exact_rows: class.non_exact_rows,
        non_exact_verifier_true_rows: class.non_exact_verifier_true_rows,
        total_tokens: class.total_tokens,
        token_share_milli,
        total_cost_microusd: class.total_cost_microusd,
        positive_token_rows: class.positive_token_rows,
        token_evidence_missing_rows: class.token_evidence_missing_rows,
        cost_evidence_missing_rows: class.cost_evidence_missing_rows,
        result_atom_rows: class.result_atom_rows,
        opportunity_score,
        status,
        next_action,
        blockers,
        top_route_keys: top_counts(&class.route_keys, 4),
        top_request_route_families: top_counts(&class.request_route_families, 4),
        top_traffic_sources: top_counts(&class.traffic_sources, 4),
        samples: class.samples.clone(),
    }
}

fn milli(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn top_counts(counts: &BTreeMap<String, usize>, limit: usize) -> Vec<CountReport> {
    let mut rows = counts
        .iter()
        .map(|(key, count)| CountReport {
            key: key.clone(),
            count: *count,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    rows.truncate(limit);
    rows
}
