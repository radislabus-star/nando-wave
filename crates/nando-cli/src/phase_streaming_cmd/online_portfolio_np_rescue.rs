use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use nando_core::{
    PhaseCenterEvalTask, PhaseCenterHotRouteTable, PhaseCenterHotRuntime, PhaseCenterHotScratch,
    PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime, PhaseCenterPreparedHotRequest,
};
use serde::Serialize;
use serde_json::Value;

use super::{
    margin_to_micro, parse_phase_atom_binary_event_for_action, phase_atom_action_families,
    phase_atom_binary_event_vector_for_task, phase_atom_state_action_bucket_key,
    phase_atom_string_vec,
};

const DEFAULT_NP_RESCUE_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-np-rescue-v1.report.json";
const DEFAULT_NP_RESCUE_RUNTIME_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-np-rescue-runtime-replay-v1.report.json";
const DEFAULT_SELECTOR_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-selector-v1.report.json";
const DEFAULT_MAX_SELECTED_SUBCENTERS: usize = 64;
const DEFAULT_SAFETY_GAP_MICRO: i64 = 1;
const MIN_POLICY_EVENTS: usize = 1;
const MIN_FUTURE_EVENTS: usize = 1;

#[derive(Clone, Debug)]
struct DecisionSample {
    request_fingerprint: String,
    exact_cache_key: String,
    external_provider_correlation_keys: Vec<String>,
    bucket_key: String,
    task_name: String,
    action_family_atom: String,
    exact_cache_hit: bool,
    verified_safe_accept: bool,
    margin_micro: i64,
    reference_runtime_parity_mismatch: bool,
    total_tokens: usize,
    total_cost_microusd: u64,
    ordinal_in_bucket: usize,
    denominator_row_index: usize,
    package_fingerprint64: u64,
    atoms: Vec<String>,
}

#[derive(Clone, Debug)]
struct BucketSelection {
    bucket_key: String,
    threshold_micro: i64,
    runtime_replay_start_event_ordinal: usize,
    false_accepts: usize,
}

#[derive(Clone, Debug)]
struct RescueSubcenterCandidate {
    source_bucket_key: String,
    source_task_name: String,
    action_family_atom: String,
    subcenter_key: String,
    split_atom: String,
    threshold_micro: i64,
    policy_events: usize,
    future_events: usize,
    policy_false_accepts: usize,
    future_false_accepts: usize,
    future_runtime_parity_mismatches: usize,
    accepted_fingerprints: BTreeMap<String, AcceptedEvent>,
    accepted_tokens: usize,
    rejected_reason: Option<&'static str>,
}

#[derive(Clone, Debug)]
struct AcceptedEvent {
    request_fingerprint: String,
    exact_cache_key: String,
    external_provider_correlation_keys: Vec<String>,
    source_bucket_key: String,
    source_task_name: String,
    denominator_row_index: usize,
    package_fingerprint64: u64,
    margin_micro: i64,
    threshold_micro: i64,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Clone, Debug)]
struct SelectedNpSubcenter {
    rank: usize,
    source_bucket_key: String,
    source_task_name: String,
    action_family_atom: String,
    split_atom: String,
    threshold_micro: i64,
    expected_marginal_accepts: usize,
    expected_marginal_tokens: usize,
    expected_marginal_cost_microusd: u64,
}

#[derive(Clone, Debug)]
struct NpReplayEvent {
    event: super::PhaseAtomBinaryEvent,
}

struct NpReplayRuntimeEntry {
    runtime: PhaseCenterOffloadRuntime,
    hot_runtime: PhaseCenterHotRuntime,
    hot_routes: PhaseCenterHotRouteTable,
    hot_scratch: PhaseCenterHotScratch,
}

#[derive(Clone, Debug, Default)]
struct BillingRequestRows {
    rows: Vec<Value>,
    request_rows_with_exact_cache_key: usize,
    external_provider_correlation_key_rows: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Clone, Debug, Serialize)]
struct SelectedSubcenterReport {
    rank: usize,
    source_bucket_key: String,
    source_task_name: String,
    action_family_atom: String,
    subcenter_key: String,
    split_atom: String,
    threshold_micro: i64,
    policy_events: usize,
    future_events: usize,
    marginal_unique_accepts_over_exact_cache: usize,
    marginal_tokens_saved: usize,
    marginal_cost_saved_microusd: u64,
    overlap_with_baseline_or_prior: usize,
    future_false_accepts: usize,
    future_runtime_parity_mismatches: usize,
}

#[derive(Clone, Debug, Serialize)]
struct RejectedSubcenterReport {
    source_bucket_key: String,
    subcenter_key: String,
    split_atom: String,
    policy_events: usize,
    future_events: usize,
    future_accepts_over_exact_cache: usize,
    future_tokens: usize,
    policy_false_accepts: usize,
    future_false_accepts: usize,
    future_runtime_parity_mismatches: usize,
    reason: &'static str,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_np_rescue_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NP_RESCUE_REPORT));
    let selector_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTOR_REPORT));
    let explicit_decision_log_path = args.next().map(PathBuf::from);
    let max_selected_subcenters = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid max_selected_subcenters '{value}' for NP rescue: {error}")
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_MAX_SELECTED_SUBCENTERS);
    if max_selected_subcenters == 0 {
        return Err("max_selected_subcenters must be > 0".to_owned());
    }

    let selector = read_json_value(&selector_report_path)?;
    let online_miner_report_path = json_string(&selector, &["online_miner_report_path"])
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "selector report '{}' missing online_miner_report_path",
                selector_report_path.display()
            )
        })?;
    let online_report = read_json_value(&online_miner_report_path)?;
    let decision_log_path = explicit_decision_log_path
        .or_else(|| json_string(&selector, &["decision_log_path"]).map(PathBuf::from))
        .or_else(|| json_string(&online_report, &["decision_log_path"]).map(PathBuf::from))
        .ok_or_else(|| {
            format!(
                "selector/online reports missing decision_log_path: '{}' '{}'",
                selector_report_path.display(),
                online_miner_report_path.display()
            )
        })?;

    let extra_trace_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    let trace_paths = if extra_trace_paths.is_empty() {
        json_string_array(&online_report, &["trace_paths"])
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>()
    } else {
        extra_trace_paths
    };
    let trace_atom_index = read_trace_atom_index(&trace_paths)?;
    let samples_by_bucket = read_decision_samples(&decision_log_path, &trace_atom_index)?;

    let constrained_selected = read_bucket_selections(&selector, &["selected_buckets"])?;
    let fixed_selected = read_bucket_selections(&selector, &["fixed_greedy_selected_buckets"])?;
    let constrained_bucket_keys = constrained_selected
        .iter()
        .map(|selection| selection.bucket_key.clone())
        .collect::<BTreeSet<_>>();
    let constrained_baseline =
        evaluate_bucket_selections(&samples_by_bucket, &constrained_selected);
    let fixed_ceiling = evaluate_bucket_selections(&samples_by_bucket, &fixed_selected);
    let unsafe_fixed = fixed_selected
        .iter()
        .filter(|selection| selection.false_accepts > 0)
        .cloned()
        .collect::<Vec<_>>();
    let rescue_sources = fixed_selected
        .iter()
        .filter(|selection| {
            selection.false_accepts > 0 || !constrained_bucket_keys.contains(&selection.bucket_key)
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut candidates = Vec::<RescueSubcenterCandidate>::new();
    for selection in &rescue_sources {
        if let Some(samples) = samples_by_bucket.get(&selection.bucket_key) {
            candidates.extend(build_subcenter_candidates(samples, selection));
        }
    }
    let selected = select_rescued_subcenters(
        &candidates,
        &constrained_baseline.accepted_fingerprints,
        max_selected_subcenters,
    );
    let rejected_samples = rejected_subcenter_reports(&candidates, 32);

    let rescued_safe_accepts = selected
        .last()
        .map(|report| {
            selected
                .iter()
                .map(|item| item.marginal_unique_accepts_over_exact_cache)
                .sum::<usize>()
                .max(report.marginal_unique_accepts_over_exact_cache)
        })
        .unwrap_or(0);
    let rescued_safe_tokens = selected
        .iter()
        .map(|item| item.marginal_tokens_saved)
        .sum::<usize>();
    let rescued_safe_cost_microusd = selected
        .iter()
        .map(|item| item.marginal_cost_saved_microusd)
        .sum::<u64>();
    let recovered_total_accepts = constrained_baseline
        .unique_accepts_over_exact_cache
        .saturating_add(rescued_safe_accepts);
    let recovered_total_tokens = constrained_baseline
        .tokens_saved
        .saturating_add(rescued_safe_tokens);
    let recovered_total_cost = constrained_baseline
        .cost_saved_microusd
        .saturating_add(rescued_safe_cost_microusd);
    let false_accepts_after_rescue = selected
        .iter()
        .map(|item| item.future_false_accepts)
        .sum::<usize>();
    let parity_mismatches_after_rescue = selected
        .iter()
        .map(|item| item.future_runtime_parity_mismatches)
        .sum::<usize>();
    let discarded_risk_accepts = fixed_ceiling
        .unique_accepts_over_exact_cache
        .saturating_sub(recovered_total_accepts);
    let discarded_risk_tokens = fixed_ceiling
        .tokens_saved
        .saturating_sub(recovered_total_tokens);

    let first_rung_target_passed = recovered_total_accepts >= 450
        && recovered_total_tokens >= 800_000
        && false_accepts_after_rescue == 0
        && parity_mismatches_after_rescue == 0;
    let recovered_fixed_token_permille =
        permille(recovered_total_tokens, fixed_ceiling.tokens_saved);
    let second_rung_target_passed = recovered_fixed_token_permille >= 700
        && false_accepts_after_rescue == 0
        && parity_mismatches_after_rescue == 0;
    let shadow_rescue_passed = rescued_safe_accepts > 0
        && false_accepts_after_rescue == 0
        && parity_mismatches_after_rescue == 0;
    let verdict = if shadow_rescue_passed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_NP_RESCUE_V1_PASS_SHADOW_SAFE_RESCUE"
    } else if candidates.is_empty() {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_NP_RESCUE_V1_WATCH_NO_UNSAFE_SUBCENTERS"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_NP_RESCUE_V1_WATCH_NO_SAFE_RESCUE"
    };

    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_np_rescue_v1"),
    );
    report.insert(
        "selector_report_path".to_owned(),
        serde_json::json!(selector_report_path),
    );
    report.insert(
        "online_miner_report_path".to_owned(),
        serde_json::json!(online_miner_report_path),
    );
    report.insert(
        "decision_log_path".to_owned(),
        serde_json::json!(decision_log_path),
    );
    report.insert(
        "trace_paths".to_owned(),
        serde_json::json!(
            trace_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
        ),
    );
    report.insert(
        "max_selected_subcenters".to_owned(),
        serde_json::json!(max_selected_subcenters),
    );
    report.insert(
        "safety_gap_micro".to_owned(),
        serde_json::json!(DEFAULT_SAFETY_GAP_MICRO),
    );
    report.insert(
        "fixed_greedy_ceiling_accepts".to_owned(),
        serde_json::json!(fixed_ceiling.unique_accepts_over_exact_cache),
    );
    report.insert(
        "fixed_greedy_ceiling_tokens".to_owned(),
        serde_json::json!(fixed_ceiling.tokens_saved),
    );
    report.insert(
        "fixed_greedy_ceiling_cost_microusd".to_owned(),
        serde_json::json!(fixed_ceiling.cost_saved_microusd),
    );
    report.insert(
        "fixed_greedy_ceiling_false_accepts".to_owned(),
        serde_json::json!(fixed_ceiling.false_accepts),
    );
    report.insert(
        "constrained_baseline_accepts".to_owned(),
        serde_json::json!(constrained_baseline.unique_accepts_over_exact_cache),
    );
    report.insert(
        "constrained_baseline_tokens".to_owned(),
        serde_json::json!(constrained_baseline.tokens_saved),
    );
    report.insert(
        "constrained_baseline_cost_microusd".to_owned(),
        serde_json::json!(constrained_baseline.cost_saved_microusd),
    );
    report.insert(
        "constrained_baseline_false_accepts".to_owned(),
        serde_json::json!(constrained_baseline.false_accepts),
    );
    report.insert(
        "unsafe_fixed_bucket_count".to_owned(),
        serde_json::json!(unsafe_fixed.len()),
    );
    report.insert(
        "rescue_source_bucket_count".to_owned(),
        serde_json::json!(rescue_sources.len()),
    );
    report.insert(
        "candidate_subcenter_count".to_owned(),
        serde_json::json!(candidates.len()),
    );
    report.insert(
        "selected_subcenter_count".to_owned(),
        serde_json::json!(selected.len()),
    );
    report.insert(
        "rescued_safe_accepts".to_owned(),
        serde_json::json!(rescued_safe_accepts),
    );
    report.insert(
        "rescued_safe_tokens".to_owned(),
        serde_json::json!(rescued_safe_tokens),
    );
    report.insert(
        "rescued_safe_cost_microusd".to_owned(),
        serde_json::json!(rescued_safe_cost_microusd),
    );
    report.insert(
        "recovered_total_accepts".to_owned(),
        serde_json::json!(recovered_total_accepts),
    );
    report.insert(
        "recovered_total_tokens".to_owned(),
        serde_json::json!(recovered_total_tokens),
    );
    report.insert(
        "recovered_total_cost_microusd".to_owned(),
        serde_json::json!(recovered_total_cost),
    );
    report.insert(
        "discarded_risk_accepts".to_owned(),
        serde_json::json!(discarded_risk_accepts),
    );
    report.insert(
        "discarded_risk_tokens".to_owned(),
        serde_json::json!(discarded_risk_tokens),
    );
    report.insert(
        "false_accepts_after_rescue".to_owned(),
        serde_json::json!(false_accepts_after_rescue),
    );
    report.insert(
        "parity_mismatches_after_rescue".to_owned(),
        serde_json::json!(parity_mismatches_after_rescue),
    );
    report.insert(
        "exact_cache_overlap_removed".to_owned(),
        serde_json::json!(true),
    );
    report.insert(
        "recovered_fixed_token_permille".to_owned(),
        serde_json::json!(recovered_fixed_token_permille),
    );
    report.insert(
        "first_rung_target".to_owned(),
        serde_json::json!({
            "accepts": 450,
            "tokens": 800000,
            "false_accepts": 0,
            "passed": first_rung_target_passed
        }),
    );
    report.insert(
        "second_rung_target".to_owned(),
        serde_json::json!({
            "recover_fixed_tokens_permille": 700,
            "false_accepts": 0,
            "passed": second_rung_target_passed
        }),
    );
    report.insert(
        "selected_subcenters".to_owned(),
        serde_json::to_value(&selected)
            .map_err(|error| format!("failed to serialize selected subcenters: {error}"))?,
    );
    report.insert(
        "rejected_subcenter_samples".to_owned(),
        serde_json::to_value(&rejected_samples)
            .map_err(|error| format!("failed to serialize rejected subcenters: {error}"))?,
    );
    report.insert(
        "policy".to_owned(),
        serde_json::json!({
            "name": "aggressive_neyman_pearson_l4_rescue",
            "source": "fixed_greedy unsafe buckets plus safe fixed buckets missed by constrained baseline",
            "split_key": "source-neutral trace atoms plus margin/token/boundary bands",
            "threshold_rule": "threshold = max(policy false margin) + safety_gap, floored by bucket threshold",
            "future_gate": "selected only if future false_accepts = 0 and runtime parity mismatches = 0",
            "portfolio_objective": "marginal unique accepts over exact cache, then tokens/cost"
        }),
    );
    report.insert(
        "discovery_mode".to_owned(),
        serde_json::json!({
            "manual_class_list_used": false,
            "static_topn_seed_used": false,
            "online_discovery_used": true,
            "aggressive_np_rescue_used": true,
            "selector_learning_shadow_only": true,
            "product_dynamic_discovery_claim_allowed": false
        }),
    );
    report.insert(
        "forbidden_flags".to_owned(),
        serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        }),
    );
    report.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report.insert(
        "product_promotion_allowed".to_owned(),
        serde_json::json!(false),
    );
    report.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
    );
    report.insert(
        "boundary".to_owned(),
        serde_json::json!("shadow/report only: rescues the fixed-greedy online-miner frontier by source-neutral subcenter splits and NP thresholds; does not compile new packages, promote, serve, enable local_accept, claim market money, or use legacy nwrb"),
    );
    report.insert("verdict".to_owned(), serde_json::json!(verdict));
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_np_rescue_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  fixed_greedy_ceiling: accepts={} tokens={} false_accepts={}",
        fixed_ceiling.unique_accepts_over_exact_cache,
        fixed_ceiling.tokens_saved,
        fixed_ceiling.false_accepts
    );
    println!(
        "  constrained_baseline: accepts={} tokens={} false_accepts={}",
        constrained_baseline.unique_accepts_over_exact_cache,
        constrained_baseline.tokens_saved,
        constrained_baseline.false_accepts
    );
    println!("  unsafe_fixed_bucket_count: {}", unsafe_fixed.len());
    println!("  rescue_source_bucket_count: {}", rescue_sources.len());
    println!("  candidate_subcenter_count: {}", candidates.len());
    println!("  selected_subcenter_count: {}", selected.len());
    println!("  rescued_safe_accepts: {rescued_safe_accepts}");
    println!("  rescued_safe_tokens: {rescued_safe_tokens}");
    println!("  recovered_total_accepts: {recovered_total_accepts}");
    println!("  recovered_total_tokens: {recovered_total_tokens}");
    println!("  false_accepts_after_rescue: {false_accepts_after_rescue}");
    println!("  parity_mismatches_after_rescue: {parity_mismatches_after_rescue}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_portfolio_np_rescue_runtime_replay_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NP_RESCUE_RUNTIME_REPLAY_REPORT));
    let np_rescue_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_NP_RESCUE_REPORT));
    let np_rescue = read_json_value(&np_rescue_report_path)?;
    let billing_request_jsonl_path = np_runtime_replay_billing_request_path(&report_path);
    let selector_report_path = json_string(&np_rescue, &["selector_report_path"])
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTOR_REPORT));
    let selector = read_json_value(&selector_report_path)?;
    let online_miner_report_path = json_string(&np_rescue, &["online_miner_report_path"])
        .or_else(|| json_string(&selector, &["online_miner_report_path"]))
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "np rescue/selector reports missing online_miner_report_path: '{}' '{}'",
                np_rescue_report_path.display(),
                selector_report_path.display()
            )
        })?;
    let online_report = read_json_value(&online_miner_report_path)?;
    let decision_log_path = json_string(&np_rescue, &["decision_log_path"])
        .or_else(|| json_string(&selector, &["decision_log_path"]))
        .or_else(|| json_string(&online_report, &["decision_log_path"]))
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "np rescue/selector/online reports missing decision_log_path: '{}' '{}' '{}'",
                np_rescue_report_path.display(),
                selector_report_path.display(),
                online_miner_report_path.display()
            )
        })?;
    let trace_paths = json_string_array(&np_rescue, &["trace_paths"])
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if trace_paths.is_empty() {
        return Err(format!(
            "np rescue report '{}' has no trace_paths",
            np_rescue_report_path.display()
        ));
    }

    let selected_subcenters = selected_np_subcenters_from_report(&np_rescue)?;
    if selected_subcenters.is_empty() {
        return Err("np rescue report has no selected_subcenters".to_owned());
    }
    let trace_atom_index = read_trace_atom_index(&trace_paths)?;
    let samples_by_bucket = read_decision_samples(&decision_log_path, &trace_atom_index)?;
    let constrained_selected = read_bucket_selections(&selector, &["selected_buckets"])?;
    let fixed_selected = read_bucket_selections(&selector, &["fixed_greedy_selected_buckets"])?;
    let fixed_selection_by_bucket = fixed_selected
        .iter()
        .map(|selection| (selection.bucket_key.clone(), selection.clone()))
        .collect::<BTreeMap<_, _>>();
    let baseline = evaluate_bucket_selections(&samples_by_bucket, &constrained_selected);
    let package_paths = package_paths_by_fingerprint(&online_report)?;
    let selected_bucket_tasks = selected_subcenters
        .iter()
        .map(|subcenter| {
            (
                subcenter.source_bucket_key.clone(),
                (
                    subcenter.source_task_name.clone(),
                    subcenter.action_family_atom.clone(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let replay_events =
        np_replay_events_from_traces(&trace_paths, &selected_bucket_tasks, &samples_by_bucket)?;
    let mut runtime_cache = BTreeMap::<(u64, i64), NpReplayRuntimeEntry>::new();
    let mut used_fingerprints = baseline
        .accepted_fingerprints
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut replay_rows = 0usize;
    let mut score_candidate_rows = 0usize;
    let mut verifier_bound_score_candidate_rows = 0usize;
    let mut verifier_rejected_score_candidate_rows = 0usize;
    let mut false_accepts = 0usize;
    let mut missing_event_rows = 0usize;
    let mut missing_package_rows = 0usize;
    let mut hot_margin_parity_mismatches = 0usize;
    let mut hot_decision_parity_mismatches = 0usize;
    let mut decision_log_margin_mismatches = 0usize;
    let mut unique_accepts = 0usize;
    let mut tokens_saved = 0usize;
    let mut cost_saved_microusd = 0u64;
    let mut billing_requests = BillingRequestRows::default();
    for (index, event) in baseline.accepted_fingerprints.values().enumerate() {
        push_np_billing_request_row(
            &mut billing_requests,
            "constrained_baseline",
            format!(
                "np-rescue-baseline-cpu-accept-{}-{}",
                event.denominator_row_index,
                index + 1
            ),
            event,
            None,
            None,
        );
    }
    let mut subcenter_reports = Vec::<Value>::new();

    for selected in &selected_subcenters {
        let Some(selection) = fixed_selection_by_bucket.get(&selected.source_bucket_key) else {
            subcenter_reports.push(serde_json::json!({
                "rank": selected.rank,
                "source_bucket_key": selected.source_bucket_key,
                "split_atom": selected.split_atom,
                "replay_rows": 0,
                "missing_selection": true
            }));
            continue;
        };
        let samples = selected_samples_for_subcenter(
            samples_by_bucket
                .get(&selected.source_bucket_key)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            selection.runtime_replay_start_event_ordinal,
            &selected.split_atom,
        );
        let policy_len = np_policy_len(samples.len());
        let future_samples = samples.get(policy_len..).unwrap_or(&[]);
        let mut sub_replay_rows = 0usize;
        let mut sub_score_candidates = 0usize;
        let mut sub_false_accepts = 0usize;
        let mut sub_unique_accepts = 0usize;
        let mut sub_tokens = 0usize;
        let mut sub_cost = 0u64;
        let mut sub_missing_events = 0usize;
        let mut sub_missing_packages = 0usize;
        let mut sub_margin_mismatches = 0usize;
        let mut sub_decision_mismatches = 0usize;
        let mut sub_decision_log_mismatches = 0usize;

        for sample in future_samples {
            let Some(replay_event) = replay_events.get(&(
                sample.denominator_row_index,
                sample.bucket_key.clone(),
                sample.request_fingerprint.clone(),
            )) else {
                missing_event_rows += 1;
                sub_missing_events += 1;
                continue;
            };
            let Some(package_path) = package_paths.get(&sample.package_fingerprint64) else {
                missing_package_rows += 1;
                sub_missing_packages += 1;
                continue;
            };
            let cache_key = (sample.package_fingerprint64, selected.threshold_micro);
            if !runtime_cache.contains_key(&cache_key) {
                let entry = load_np_replay_runtime_entry(
                    package_path,
                    selected.threshold_micro,
                    sample.package_fingerprint64,
                )?;
                runtime_cache.insert(cache_key, entry);
            }
            let runtime_entry = runtime_cache
                .get_mut(&cache_key)
                .expect("runtime entry inserted before NP replay");
            let safe_accept_vec = phase_atom_binary_event_vector_for_task(
                &replay_event.event,
                true,
                runtime_entry.runtime.cells(),
                &selected.source_task_name,
            );
            let zero = vec![nando_core::PhaseCenterCell::default(); runtime_entry.runtime.cells()];
            let task = PhaseCenterEvalTask {
                center_index: 0,
                correct_vec: safe_accept_vec.clone().into_boxed_slice(),
                wrong_vec: zero.into_boxed_slice(),
            };
            let flat_margin_micro = margin_to_micro(
                runtime_entry
                    .runtime
                    .runtime()
                    .margin(&task)
                    .map_err(|error| format!("NP replay flat margin error: {error:?}"))?,
            )?;
            let candidates = runtime_entry
                .hot_runtime
                .score_prepared_hot_request_candidates(
                    &runtime_entry.hot_routes,
                    PhaseCenterPreparedHotRequest::new(0, &safe_accept_vec),
                    &mut runtime_entry.hot_scratch,
                )
                .map_err(|error| format!("NP replay hot runtime error: {error:?}"))?;
            let Some(hot_decision) = candidates.first() else {
                return Err("NP replay hot runtime returned no candidates".to_owned());
            };
            replay_rows += 1;
            sub_replay_rows += 1;
            if hot_decision.margin_micro != flat_margin_micro {
                hot_margin_parity_mismatches += 1;
                sub_margin_mismatches += 1;
            }
            let flat_score_candidate = flat_margin_micro >= selected.threshold_micro;
            if hot_decision.score_candidate != flat_score_candidate {
                hot_decision_parity_mismatches += 1;
                sub_decision_mismatches += 1;
            }
            if sample.margin_micro != flat_margin_micro {
                decision_log_margin_mismatches += 1;
                sub_decision_log_mismatches += 1;
            }
            if hot_decision.score_candidate {
                score_candidate_rows += 1;
                sub_score_candidates += 1;
                if sample.verified_safe_accept {
                    verifier_bound_score_candidate_rows += 1;
                    if !sample.exact_cache_hit
                        && used_fingerprints.insert(sample.request_fingerprint.clone())
                    {
                        unique_accepts += 1;
                        sub_unique_accepts += 1;
                        tokens_saved = tokens_saved.saturating_add(sample.total_tokens);
                        sub_tokens = sub_tokens.saturating_add(sample.total_tokens);
                        cost_saved_microusd =
                            cost_saved_microusd.saturating_add(sample.total_cost_microusd);
                        sub_cost = sub_cost.saturating_add(sample.total_cost_microusd);
                        let mut event =
                            accepted_event_from_sample(sample, selected.threshold_micro);
                        if !replay_event.event.exact_cache_key.is_empty() {
                            event.exact_cache_key = replay_event.event.exact_cache_key.clone();
                        }
                        if !replay_event
                            .event
                            .external_provider_correlation_keys
                            .is_empty()
                        {
                            event.external_provider_correlation_keys = replay_event
                                .event
                                .external_provider_correlation_keys
                                .clone();
                        }
                        event.margin_micro = flat_margin_micro;
                        push_np_billing_request_row(
                            &mut billing_requests,
                            "np_rescue_marginal",
                            format!(
                                "np-rescue-marginal-cpu-accept-{}-{}-{}",
                                sample.denominator_row_index, selected.rank, unique_accepts
                            ),
                            &event,
                            Some(&selected.split_atom),
                            Some(selected.rank),
                        );
                    }
                } else {
                    false_accepts += 1;
                    sub_false_accepts += 1;
                    verifier_rejected_score_candidate_rows += 1;
                }
            }
        }
        subcenter_reports.push(serde_json::json!({
            "rank": selected.rank,
            "source_bucket_key": selected.source_bucket_key,
            "split_atom": selected.split_atom,
            "threshold_micro": selected.threshold_micro,
            "expected_marginal_accepts": selected.expected_marginal_accepts,
            "expected_marginal_tokens": selected.expected_marginal_tokens,
            "expected_marginal_cost_microusd": selected.expected_marginal_cost_microusd,
            "replay_rows": sub_replay_rows,
            "score_candidate_rows": sub_score_candidates,
            "marginal_unique_accepts_over_exact_cache": sub_unique_accepts,
            "marginal_tokens_saved": sub_tokens,
            "marginal_cost_saved_microusd": sub_cost,
            "false_accepts": sub_false_accepts,
            "missing_event_rows": sub_missing_events,
            "missing_package_rows": sub_missing_packages,
            "hot_margin_parity_mismatches": sub_margin_mismatches,
            "hot_decision_parity_mismatches": sub_decision_mismatches,
            "decision_log_margin_mismatches": sub_decision_log_mismatches,
        }));
    }

    let expected_accepts = json_usize(&np_rescue, &["rescued_safe_accepts"]).unwrap_or(0);
    let expected_tokens = json_usize(&np_rescue, &["rescued_safe_tokens"]).unwrap_or(0);
    let expected_cost = json_u64(&np_rescue, &["rescued_safe_cost_microusd"]).unwrap_or(0);
    let expected_recovered_accepts =
        json_usize(&np_rescue, &["recovered_total_accepts"]).unwrap_or(0);
    let expected_recovered_tokens =
        json_usize(&np_rescue, &["recovered_total_tokens"]).unwrap_or(0);
    let recovered_total_accepts = baseline
        .unique_accepts_over_exact_cache
        .saturating_add(unique_accepts);
    let recovered_total_tokens = baseline.tokens_saved.saturating_add(tokens_saved);
    let replay_accept_parity =
        unique_accepts == expected_accepts && recovered_total_accepts == expected_recovered_accepts;
    let replay_token_parity =
        tokens_saved == expected_tokens && recovered_total_tokens == expected_recovered_tokens;
    let replay_cost_parity = cost_saved_microusd == expected_cost;
    let recovered_billing_request_parity = billing_requests.rows.len() == recovered_total_accepts
        && billing_requests.total_tokens == recovered_total_tokens;
    write_jsonl_value_file(&billing_request_jsonl_path, &billing_requests.rows)?;
    let runtime_replay_passed = replay_rows > 0
        && missing_event_rows == 0
        && missing_package_rows == 0
        && hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && false_accepts == 0
        && replay_accept_parity
        && replay_token_parity
        && recovered_billing_request_parity;
    let verdict = if runtime_replay_passed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_NP_RESCUE_RUNTIME_REPLAY_V1_PASS_REVIEW_ONLY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_NP_RESCUE_RUNTIME_REPLAY_V1_WATCH"
    };

    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_np_rescue_runtime_replay_v1"),
    );
    report.insert(
        "np_rescue_report_path".to_owned(),
        serde_json::json!(np_rescue_report_path),
    );
    report.insert(
        "selector_report_path".to_owned(),
        serde_json::json!(selector_report_path),
    );
    report.insert(
        "online_miner_report_path".to_owned(),
        serde_json::json!(online_miner_report_path),
    );
    report.insert(
        "decision_log_path".to_owned(),
        serde_json::json!(decision_log_path),
    );
    report.insert(
        "trace_paths".to_owned(),
        serde_json::json!(
            trace_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
        ),
    );
    report.insert(
        "billing_request_jsonl_path".to_owned(),
        serde_json::json!(billing_request_jsonl_path),
    );
    report.insert(
        "selected_subcenter_count".to_owned(),
        serde_json::json!(selected_subcenters.len()),
    );
    report.insert("replay_rows".to_owned(), serde_json::json!(replay_rows));
    report.insert(
        "score_candidate_rows".to_owned(),
        serde_json::json!(score_candidate_rows),
    );
    report.insert(
        "verifier_bound_score_candidate_rows".to_owned(),
        serde_json::json!(verifier_bound_score_candidate_rows),
    );
    report.insert(
        "verifier_rejected_score_candidate_rows".to_owned(),
        serde_json::json!(verifier_rejected_score_candidate_rows),
    );
    report.insert(
        "unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(unique_accepts),
    );
    report.insert(
        "marginal_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(unique_accepts),
    );
    report.insert(
        "billing_request_rows".to_owned(),
        serde_json::json!(billing_requests.rows.len()),
    );
    report.insert(
        "marginal_billing_request_rows".to_owned(),
        serde_json::json!(unique_accepts),
    );
    report.insert(
        "request_rows_with_exact_cache_key".to_owned(),
        serde_json::json!(billing_requests.request_rows_with_exact_cache_key),
    );
    report.insert(
        "external_provider_correlation_key_rows".to_owned(),
        serde_json::json!(billing_requests.external_provider_correlation_key_rows),
    );
    report.insert(
        "external_provider_correlation_missing_rows".to_owned(),
        serde_json::json!(
            billing_requests
                .rows
                .len()
                .saturating_sub(billing_requests.external_provider_correlation_key_rows)
        ),
    );
    report.insert("tokens_saved".to_owned(), serde_json::json!(tokens_saved));
    report.insert(
        "marginal_tokens_saved".to_owned(),
        serde_json::json!(tokens_saved),
    );
    report.insert(
        "total_tokens_requiring_billing".to_owned(),
        serde_json::json!(billing_requests.total_tokens),
    );
    report.insert(
        "cost_saved_microusd".to_owned(),
        serde_json::json!(cost_saved_microusd),
    );
    report.insert(
        "marginal_cost_saved_microusd".to_owned(),
        serde_json::json!(cost_saved_microusd),
    );
    report.insert(
        "current_known_cost_microusd".to_owned(),
        serde_json::json!(billing_requests.total_cost_microusd),
    );
    report.insert(
        "baseline_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(baseline.unique_accepts_over_exact_cache),
    );
    report.insert(
        "baseline_tokens_saved".to_owned(),
        serde_json::json!(baseline.tokens_saved),
    );
    report.insert(
        "recovered_total_accepts".to_owned(),
        serde_json::json!(recovered_total_accepts),
    );
    report.insert(
        "recovered_total_tokens".to_owned(),
        serde_json::json!(recovered_total_tokens),
    );
    report.insert(
        "recovered_total_cost_microusd".to_owned(),
        serde_json::json!(
            baseline
                .cost_saved_microusd
                .saturating_add(cost_saved_microusd)
        ),
    );
    report.insert(
        "portfolio_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(recovered_total_accepts),
    );
    report.insert(
        "portfolio_tokens_saved".to_owned(),
        serde_json::json!(recovered_total_tokens),
    );
    report.insert(
        "portfolio_cost_saved_microusd".to_owned(),
        serde_json::json!(billing_requests.total_cost_microusd),
    );
    report.insert(
        "expected_rescued_safe_accepts".to_owned(),
        serde_json::json!(expected_accepts),
    );
    report.insert(
        "expected_rescued_safe_tokens".to_owned(),
        serde_json::json!(expected_tokens),
    );
    report.insert(
        "expected_rescued_safe_cost_microusd".to_owned(),
        serde_json::json!(expected_cost),
    );
    report.insert(
        "expected_recovered_total_accepts".to_owned(),
        serde_json::json!(expected_recovered_accepts),
    );
    report.insert(
        "expected_recovered_total_tokens".to_owned(),
        serde_json::json!(expected_recovered_tokens),
    );
    report.insert("false_accepts".to_owned(), serde_json::json!(false_accepts));
    report.insert(
        "missing_event_rows".to_owned(),
        serde_json::json!(missing_event_rows),
    );
    report.insert(
        "missing_package_rows".to_owned(),
        serde_json::json!(missing_package_rows),
    );
    report.insert(
        "hot_margin_parity_mismatches".to_owned(),
        serde_json::json!(hot_margin_parity_mismatches),
    );
    report.insert(
        "hot_decision_parity_mismatches".to_owned(),
        serde_json::json!(hot_decision_parity_mismatches),
    );
    report.insert(
        "decision_log_margin_mismatches".to_owned(),
        serde_json::json!(decision_log_margin_mismatches),
    );
    report.insert(
        "replay_accept_parity".to_owned(),
        serde_json::json!(replay_accept_parity),
    );
    report.insert(
        "replay_token_parity".to_owned(),
        serde_json::json!(replay_token_parity),
    );
    report.insert(
        "replay_cost_parity".to_owned(),
        serde_json::json!(replay_cost_parity),
    );
    report.insert(
        "recovered_billing_request_parity".to_owned(),
        serde_json::json!(recovered_billing_request_parity),
    );
    report.insert(
        "runtime_replay_passed".to_owned(),
        serde_json::json!(runtime_replay_passed),
    );
    report.insert(
        "subcenter_reports".to_owned(),
        serde_json::json!(subcenter_reports),
    );
    report.insert(
        "discovery_mode".to_owned(),
        serde_json::json!({
            "manual_class_list_used": false,
            "static_topn_seed_used": false,
            "online_discovery_used": true,
            "aggressive_np_rescue_used": true,
            "runtime_replay_passed": runtime_replay_passed,
            "product_dynamic_discovery_claim_allowed": false
        }),
    );
    report.insert(
        "billing_gate".to_owned(),
        serde_json::json!({
            "provider_billing_request_only": true,
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "policy": "NP-rescue runtime replay emits exact match keys for external provider billing evidence; this artifact is not evidence that money was saved"
        }),
    );
    report.insert(
        "forbidden_flags".to_owned(),
        serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        }),
    );
    report.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report.insert(
        "product_promotion_allowed".to_owned(),
        serde_json::json!(false),
    );
    report.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
    );
    report.insert("verdict".to_owned(), serde_json::json!(verdict));
    report.insert(
        "boundary".to_owned(),
        serde_json::json!("runtime replay/reporting only: reloads .nwpc packages for selected NP-rescue subcenters, rebuilds phase vectors from source traces, checks prepared-hot vs flat margins, and does not promote, serve, enable local_accept, claim market money, or revive legacy nwrb"),
    );
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_np_rescue_runtime_replay_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  selected_subcenter_count: {}", selected_subcenters.len());
    println!("  replay_rows: {replay_rows}");
    println!("  unique_cpu_accepts_over_exact_cache: {unique_accepts}");
    println!("  tokens_saved: {tokens_saved}");
    println!("  false_accepts: {false_accepts}");
    println!("  hot_margin_parity_mismatches: {hot_margin_parity_mismatches}");
    println!("  hot_decision_parity_mismatches: {hot_decision_parity_mismatches}");
    println!("  replay_accept_parity: {replay_accept_parity}");
    println!("  replay_token_parity: {replay_token_parity}");
    println!("  runtime_replay_passed: {runtime_replay_passed}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

#[derive(Clone, Debug, Default)]
struct PortfolioOutcome {
    accepted_fingerprints: BTreeMap<String, AcceptedEvent>,
    unique_accepts_over_exact_cache: usize,
    tokens_saved: usize,
    cost_saved_microusd: u64,
    false_accepts: usize,
    runtime_parity_mismatches: usize,
}

fn evaluate_bucket_selections(
    samples_by_bucket: &BTreeMap<String, Vec<DecisionSample>>,
    selections: &[BucketSelection],
) -> PortfolioOutcome {
    let mut outcome = PortfolioOutcome::default();
    for selection in selections {
        let Some(samples) = samples_by_bucket.get(&selection.bucket_key) else {
            continue;
        };
        for sample in samples.iter().filter(|sample| {
            sample.ordinal_in_bucket >= selection.runtime_replay_start_event_ordinal
        }) {
            outcome.runtime_parity_mismatches +=
                usize::from(sample.reference_runtime_parity_mismatch);
            if sample.margin_micro < selection.threshold_micro {
                continue;
            }
            if sample.verified_safe_accept {
                if !sample.exact_cache_hit {
                    outcome
                        .accepted_fingerprints
                        .entry(sample.request_fingerprint.clone())
                        .or_insert_with(|| {
                            accepted_event_from_sample(sample, selection.threshold_micro)
                        });
                }
            } else {
                outcome.false_accepts += 1;
            }
        }
    }
    outcome.unique_accepts_over_exact_cache = outcome.accepted_fingerprints.len();
    outcome.tokens_saved = outcome
        .accepted_fingerprints
        .values()
        .map(|event| event.total_tokens)
        .sum();
    outcome.cost_saved_microusd = outcome
        .accepted_fingerprints
        .values()
        .map(|event| event.total_cost_microusd)
        .sum();
    outcome
}

fn accepted_event_from_sample(sample: &DecisionSample, threshold_micro: i64) -> AcceptedEvent {
    AcceptedEvent {
        request_fingerprint: sample.request_fingerprint.clone(),
        exact_cache_key: sample.exact_cache_key.clone(),
        external_provider_correlation_keys: sample.external_provider_correlation_keys.clone(),
        source_bucket_key: sample.bucket_key.clone(),
        source_task_name: sample.task_name.clone(),
        denominator_row_index: sample.denominator_row_index,
        package_fingerprint64: sample.package_fingerprint64,
        margin_micro: sample.margin_micro,
        threshold_micro,
        total_tokens: sample.total_tokens,
        total_cost_microusd: sample.total_cost_microusd,
    }
}

fn push_np_billing_request_row(
    billing_requests: &mut BillingRequestRows,
    portfolio_component: &'static str,
    billing_request_id: String,
    event: &AcceptedEvent,
    split_atom: Option<&str>,
    subcenter_rank: Option<usize>,
) {
    let mut match_keys = vec![format!("request_fingerprint:{}", event.request_fingerprint)];
    if !event.exact_cache_key.is_empty() {
        billing_requests.request_rows_with_exact_cache_key += 1;
        match_keys.push(format!("exact_cache_key:{}", event.exact_cache_key));
    }
    if !event.external_provider_correlation_keys.is_empty() {
        billing_requests.external_provider_correlation_key_rows += 1;
        match_keys.extend(event.external_provider_correlation_keys.iter().cloned());
    }
    match_keys.sort();
    match_keys.dedup();
    billing_requests.total_tokens = billing_requests
        .total_tokens
        .saturating_add(event.total_tokens);
    billing_requests.total_cost_microusd = billing_requests
        .total_cost_microusd
        .saturating_add(event.total_cost_microusd);
    billing_requests.rows.push(serde_json::json!({
        "schema_version": "phase_stream_online_miner_portfolio_np_rescue_billing_request_v1",
        "billing_request_id": billing_request_id,
        "portfolio_component": portfolio_component,
        "request_fingerprint": event.request_fingerprint.clone(),
        "exact_cache_key": event.exact_cache_key.clone(),
        "external_provider_correlation_keys": event.external_provider_correlation_keys.clone(),
        "provider_correlation_ready": !event.external_provider_correlation_keys.is_empty(),
        "match_keys": match_keys,
        "source_bucket_key": event.source_bucket_key.clone(),
        "source_task_name": event.source_task_name.clone(),
        "split_atom": split_atom,
        "subcenter_rank": subcenter_rank,
        "package_fingerprint64": event.package_fingerprint64,
        "denominator_row_index": event.denominator_row_index,
        "margin_micro": event.margin_micro,
        "threshold_micro": event.threshold_micro,
        "estimated_total_tokens": event.total_tokens,
        "current_total_cost_microusd": event.total_cost_microusd,
        "unique_cpu_accept_over_exact_cache": true,
        "verified_safe_accept": true,
        "false_accept": false,
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "boundary": "selected NP-rescue recovered portfolio shadow accept billing request only: asks external provider billing evidence to attach real cost; does not estimate missing money, promote, serve, or enable local_accept"
    }));
}

fn build_subcenter_candidates(
    samples: &[DecisionSample],
    selection: &BucketSelection,
) -> Vec<RescueSubcenterCandidate> {
    let mut grouped = BTreeMap::<String, (String, Vec<DecisionSample>)>::new();
    for sample in samples {
        if sample.ordinal_in_bucket < selection.runtime_replay_start_event_ordinal {
            continue;
        }
        for split_atom in split_atoms_for_sample(sample) {
            let subcenter_key = format!("{}::{}", sample.bucket_key, split_atom);
            grouped
                .entry(subcenter_key)
                .or_insert_with(|| (split_atom, Vec::new()))
                .1
                .push(sample.clone());
        }
    }
    grouped
        .into_iter()
        .map(|(subcenter_key, (split_atom, samples))| {
            subcenter_candidate_from_samples(subcenter_key, split_atom, &samples, selection)
        })
        .collect()
}

fn subcenter_candidate_from_samples(
    subcenter_key: String,
    split_atom: String,
    samples: &[DecisionSample],
    selection: &BucketSelection,
) -> RescueSubcenterCandidate {
    let policy_len = np_policy_len(samples.len());
    let (policy_samples, future_samples) = samples.split_at(policy_len);
    let max_false_margin = policy_samples
        .iter()
        .filter(|sample| !sample.verified_safe_accept)
        .map(|sample| sample.margin_micro)
        .max();
    let threshold_micro = max_false_margin.map_or(selection.threshold_micro, |margin| {
        selection
            .threshold_micro
            .max(margin.saturating_add(DEFAULT_SAFETY_GAP_MICRO))
    });
    let mut policy_false_accepts = 0usize;
    for sample in policy_samples {
        if sample.margin_micro >= threshold_micro && !sample.verified_safe_accept {
            policy_false_accepts += 1;
        }
    }
    let mut accepted_fingerprints = BTreeMap::<String, AcceptedEvent>::new();
    let mut future_false_accepts = 0usize;
    let mut future_runtime_parity_mismatches = 0usize;
    for sample in future_samples {
        future_runtime_parity_mismatches += usize::from(sample.reference_runtime_parity_mismatch);
        if sample.margin_micro < threshold_micro {
            continue;
        }
        if sample.verified_safe_accept {
            if !sample.exact_cache_hit {
                accepted_fingerprints
                    .entry(sample.request_fingerprint.clone())
                    .or_insert_with(|| accepted_event_from_sample(sample, threshold_micro));
            }
        } else {
            future_false_accepts += 1;
        }
    }
    let accepted_tokens = accepted_fingerprints
        .values()
        .map(|event| event.total_tokens)
        .sum();
    let rejected_reason = if policy_samples.len() < MIN_POLICY_EVENTS {
        Some("too_few_policy_events")
    } else if future_samples.len() < MIN_FUTURE_EVENTS {
        Some("too_few_future_events")
    } else if policy_false_accepts > 0 {
        Some("policy_false_accepts_after_np_threshold")
    } else if future_false_accepts > 0 {
        Some("future_false_accepts_after_np_threshold")
    } else if future_runtime_parity_mismatches > 0 {
        Some("future_runtime_parity_mismatches")
    } else if accepted_fingerprints.is_empty() {
        Some("no_future_unique_accepts_over_exact_cache")
    } else {
        None
    };
    let first = samples.first();
    RescueSubcenterCandidate {
        source_bucket_key: first
            .map(|sample| sample.bucket_key.clone())
            .unwrap_or_else(|| selection.bucket_key.clone()),
        source_task_name: first
            .map(|sample| sample.task_name.clone())
            .unwrap_or_default(),
        action_family_atom: first
            .map(|sample| sample.action_family_atom.clone())
            .unwrap_or_default(),
        subcenter_key,
        split_atom,
        threshold_micro,
        policy_events: policy_samples.len(),
        future_events: future_samples.len(),
        policy_false_accepts,
        future_false_accepts,
        future_runtime_parity_mismatches,
        accepted_fingerprints,
        accepted_tokens,
        rejected_reason,
    }
}

fn select_rescued_subcenters(
    candidates: &[RescueSubcenterCandidate],
    baseline_fingerprints: &BTreeMap<String, AcceptedEvent>,
    max_selected: usize,
) -> Vec<SelectedSubcenterReport> {
    let mut used_fingerprints = baseline_fingerprints
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut selected_indexes = BTreeSet::<usize>::new();
    let mut reports = Vec::<SelectedSubcenterReport>::new();
    while reports.len() < max_selected {
        let mut best: Option<(usize, usize, u64, usize, usize)> = None;
        for (index, candidate) in candidates.iter().enumerate() {
            if selected_indexes.contains(&index) || candidate.rejected_reason.is_some() {
                continue;
            }
            let (delta_accepts, delta_tokens, delta_cost, overlap) =
                candidate_delta(candidate, &used_fingerprints);
            if delta_accepts == 0 {
                continue;
            }
            let score = (delta_tokens, delta_cost, delta_accepts, overlap);
            if best.is_none_or(|(_, best_tokens, best_cost, best_accepts, best_overlap)| {
                score > (best_tokens, best_cost, best_accepts, best_overlap)
            }) {
                best = Some((index, delta_tokens, delta_cost, delta_accepts, overlap));
            }
        }
        let Some((index, delta_tokens, delta_cost, delta_accepts, overlap)) = best else {
            break;
        };
        selected_indexes.insert(index);
        let candidate = &candidates[index];
        for fingerprint in candidate.accepted_fingerprints.keys() {
            used_fingerprints.insert(fingerprint.clone());
        }
        reports.push(SelectedSubcenterReport {
            rank: reports.len() + 1,
            source_bucket_key: candidate.source_bucket_key.clone(),
            source_task_name: candidate.source_task_name.clone(),
            action_family_atom: candidate.action_family_atom.clone(),
            subcenter_key: candidate.subcenter_key.clone(),
            split_atom: candidate.split_atom.clone(),
            threshold_micro: candidate.threshold_micro,
            policy_events: candidate.policy_events,
            future_events: candidate.future_events,
            marginal_unique_accepts_over_exact_cache: delta_accepts,
            marginal_tokens_saved: delta_tokens,
            marginal_cost_saved_microusd: delta_cost,
            overlap_with_baseline_or_prior: overlap,
            future_false_accepts: candidate.future_false_accepts,
            future_runtime_parity_mismatches: candidate.future_runtime_parity_mismatches,
        });
    }
    reports
}

fn rejected_subcenter_reports(
    candidates: &[RescueSubcenterCandidate],
    limit: usize,
) -> Vec<RejectedSubcenterReport> {
    candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .rejected_reason
                .map(|reason| RejectedSubcenterReport {
                    source_bucket_key: candidate.source_bucket_key.clone(),
                    subcenter_key: candidate.subcenter_key.clone(),
                    split_atom: candidate.split_atom.clone(),
                    policy_events: candidate.policy_events,
                    future_events: candidate.future_events,
                    future_accepts_over_exact_cache: candidate.accepted_fingerprints.len(),
                    future_tokens: candidate.accepted_tokens,
                    policy_false_accepts: candidate.policy_false_accepts,
                    future_false_accepts: candidate.future_false_accepts,
                    future_runtime_parity_mismatches: candidate.future_runtime_parity_mismatches,
                    reason,
                })
        })
        .take(limit)
        .collect()
}

fn candidate_delta(
    candidate: &RescueSubcenterCandidate,
    used_fingerprints: &BTreeSet<String>,
) -> (usize, usize, u64, usize) {
    let mut accepts = 0usize;
    let mut tokens = 0usize;
    let mut cost = 0u64;
    let mut overlap = 0usize;
    for (fingerprint, event) in &candidate.accepted_fingerprints {
        if used_fingerprints.contains(fingerprint) {
            overlap += 1;
        } else {
            accepts += 1;
            tokens = tokens.saturating_add(event.total_tokens);
            cost = cost.saturating_add(event.total_cost_microusd);
        }
    }
    (accepts, tokens, cost, overlap)
}

fn split_atoms_for_sample(sample: &DecisionSample) -> Vec<String> {
    let mut atoms = BTreeSet::<String>::new();
    let margin = margin_band(sample.margin_micro);
    let tokens = token_band(sample.total_tokens);
    atoms.insert(format!("margin_band:{margin}"));
    atoms.insert(format!("token_band:{tokens}"));
    atoms.insert(format!("margin_token:{margin}|{tokens}"));
    atoms.insert(format!("provider_ready:{}", !sample.atoms.is_empty()));
    let mut split_atoms = Vec::<String>::new();
    for atom in &sample.atoms {
        if source_neutral_split_atom(atom) {
            atoms.insert(format!("atom:{atom}"));
            atoms.insert(format!("atom_margin:{atom}|{margin}"));
            atoms.insert(format!("atom_token:{atom}|{tokens}"));
            split_atoms.push(atom.clone());
        }
    }
    split_atoms.sort();
    split_atoms.dedup();
    split_atoms.truncate(16);
    for left_index in 0..split_atoms.len() {
        for right in split_atoms.iter().skip(left_index + 1) {
            let left = &split_atoms[left_index];
            atoms.insert(format!("atom_pair:{left}&{right}"));
            atoms.insert(format!("atom_pair_margin:{left}&{right}|{margin}"));
        }
    }
    atoms.into_iter().collect()
}

fn source_neutral_split_atom(atom: &str) -> bool {
    let allowed_prefixes = [
        "request_command_kind:",
        "request_command_arg_band:",
        "request_cwd_kind:",
        "request_char_band:",
        "request_line_count_band:",
        "request_word_count_band:",
        "request_has_code_fence:",
        "request_has_json_shape:",
        "request_has_path:",
        "request_has_question:",
        "state_cwd_kind:",
        "state_followup_marker:",
        "state_stop_marker:",
        "state_session_turn_band:",
        "tool_command_kind:",
        "tool_command_shell_family:",
        "tool_check_kind:",
        "route_hint:",
        "result_kind:",
        "verifier_signal:",
        "exit_code_band:",
        "evidence:",
        "domain_family:",
        "topic:",
    ];
    allowed_prefixes
        .iter()
        .any(|prefix| atom.starts_with(prefix))
        && !atom.starts_with("profile_id:")
        && !atom.starts_with("proof_rule_id:")
        && !atom.starts_with("target_id:")
        && !atom.starts_with("output_hash")
        && !atom.contains("provider_request_id")
}

fn margin_band(margin: i64) -> &'static str {
    match margin {
        i64::MIN..=-500_001 => "neg_gt_500k",
        -500_000..=-200_001 => "neg_200k_500k",
        -200_000..=-50_001 => "neg_50k_200k",
        -50_000..=-1 => "neg_lt_50k",
        0..=49_999 => "pos_lt_50k",
        50_000..=199_999 => "pos_50k_200k",
        200_000..=499_999 => "pos_200k_500k",
        _ => "pos_gt_500k",
    }
}

fn token_band(tokens: usize) -> &'static str {
    match tokens {
        0 => "0",
        1..=63 => "1_63",
        64..=255 => "64_255",
        256..=1023 => "256_1023",
        1024..=4095 => "1024_4095",
        _ => "4096_plus",
    }
}

fn np_policy_len(sample_count: usize) -> usize {
    if sample_count < 2 {
        return sample_count;
    }
    (sample_count / 2).max(1).min(sample_count - 1)
}

fn read_bucket_selections(value: &Value, path: &[&str]) -> Result<Vec<BucketSelection>, String> {
    let rows = json_at(value, path)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut selections = Vec::<BucketSelection>::new();
    for row in rows {
        let Some(bucket_key) = json_string(&row, &["bucket_key"]) else {
            continue;
        };
        selections.push(BucketSelection {
            bucket_key,
            threshold_micro: json_i64(&row, &["threshold_micro"]).unwrap_or(0),
            runtime_replay_start_event_ordinal: json_usize(
                &row,
                &["runtime_replay_start_event_ordinal"],
            )
            .unwrap_or(0),
            false_accepts: json_usize(&row, &["false_accepts"]).unwrap_or(0),
        });
    }
    Ok(selections)
}

fn read_decision_samples(
    path: &Path,
    trace_atom_index: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<String, Vec<DecisionSample>>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read decision log '{}': {error}", path.display()))?;
    let mut by_bucket = BTreeMap::<String, Vec<DecisionSample>>::new();
    let mut ordinals = BTreeMap::<String, usize>::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse decision log '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if !json_bool(&row, &["future_only_shadow_scoring"]).unwrap_or(false) {
            continue;
        }
        let Some(bucket_key) = json_string(&row, &["bucket_key"]) else {
            continue;
        };
        let ordinal = ordinals.entry(bucket_key.clone()).or_insert(0);
        let request_fingerprint = json_string(&row, &["request_fingerprint"])
            .unwrap_or_else(|| format!("decision-row:{}", line_index + 1));
        let exact_cache_key = json_string(&row, &["exact_cache_key"]).unwrap_or_default();
        let external_provider_correlation_keys =
            super::phase_atom_external_provider_correlation_keys(&row);
        let token_cost = row.get("token_cost").unwrap_or(&Value::Null);
        let sample = DecisionSample {
            request_fingerprint: request_fingerprint.clone(),
            exact_cache_key,
            external_provider_correlation_keys,
            bucket_key: bucket_key.clone(),
            task_name: json_string(&row, &["task_name"]).unwrap_or_default(),
            action_family_atom: json_string(&row, &["action_family_atom"]).unwrap_or_default(),
            exact_cache_hit: json_bool(&row, &["exact_cache_hit"]).unwrap_or(false),
            verified_safe_accept: json_bool(&row, &["verified_safe_accept"]).unwrap_or(false),
            margin_micro: json_i64(&row, &["margin_micro"]).unwrap_or(0),
            reference_runtime_parity_mismatch: json_bool(
                &row,
                &["reference_runtime_parity_mismatch"],
            )
            .unwrap_or(false),
            total_tokens: json_usize(token_cost, &["total_tokens"]).unwrap_or(0),
            total_cost_microusd: json_u64(token_cost, &["total_cost_microusd"]).unwrap_or(0),
            ordinal_in_bucket: *ordinal,
            denominator_row_index: json_usize(&row, &["denominator_row_index"])
                .unwrap_or(line_index + 1),
            package_fingerprint64: json_u64(&row, &["package_fingerprint64"]).unwrap_or(0),
            atoms: trace_atom_index
                .get(&request_fingerprint)
                .cloned()
                .unwrap_or_default(),
        };
        *ordinal += 1;
        by_bucket.entry(bucket_key).or_default().push(sample);
    }
    Ok(by_bucket)
}

fn read_trace_atom_index(paths: &[PathBuf]) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut index = BTreeMap::<String, Vec<String>>::new();
    for path in paths {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read trace '{}': {error}", path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse trace '{}' line {}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
            let Some(request_fingerprint) = json_string(&row, &["request_fingerprint"]) else {
                continue;
            };
            let mut atoms = BTreeSet::<String>::new();
            for key in [
                "request_atoms",
                "state_atoms",
                "tool_atoms",
                "result_atoms",
                "route_hint_atoms",
                "action_atoms",
            ] {
                for atom in json_string_array(&row, &[key]) {
                    atoms.insert(atom);
                }
            }
            if let Some(groups) = row.get("atom_groups").and_then(Value::as_object) {
                for value in groups.values() {
                    for atom in json_string_vec(value) {
                        atoms.insert(atom);
                    }
                }
            }
            index.insert(request_fingerprint, atoms.into_iter().collect());
        }
    }
    Ok(index)
}

fn selected_np_subcenters_from_report(report: &Value) -> Result<Vec<SelectedNpSubcenter>, String> {
    let rows = report
        .get("selected_subcenters")
        .and_then(Value::as_array)
        .ok_or_else(|| "np rescue report missing selected_subcenters".to_owned())?;
    let mut subcenters = Vec::new();
    for row in rows {
        let source_bucket_key = json_string(row, &["source_bucket_key"])
            .ok_or_else(|| "selected NP subcenter missing source_bucket_key".to_owned())?;
        let split_atom = json_string(row, &["split_atom"])
            .ok_or_else(|| "selected NP subcenter missing split_atom".to_owned())?;
        subcenters.push(SelectedNpSubcenter {
            rank: json_usize(row, &["rank"]).unwrap_or(0),
            source_bucket_key,
            source_task_name: json_string(row, &["source_task_name"]).unwrap_or_default(),
            action_family_atom: json_string(row, &["action_family_atom"]).unwrap_or_default(),
            split_atom,
            threshold_micro: json_i64(row, &["threshold_micro"]).unwrap_or(1).max(1),
            expected_marginal_accepts: json_usize(
                row,
                &["marginal_unique_accepts_over_exact_cache"],
            )
            .unwrap_or(0),
            expected_marginal_tokens: json_usize(row, &["marginal_tokens_saved"]).unwrap_or(0),
            expected_marginal_cost_microusd: json_u64(row, &["marginal_cost_saved_microusd"])
                .unwrap_or(0),
        });
    }
    subcenters.sort_by_key(|subcenter| subcenter.rank);
    Ok(subcenters)
}

fn selected_samples_for_subcenter<'a>(
    samples: &'a [DecisionSample],
    runtime_replay_start_event_ordinal: usize,
    split_atom: &str,
) -> Vec<&'a DecisionSample> {
    samples
        .iter()
        .filter(|sample| sample.ordinal_in_bucket >= runtime_replay_start_event_ordinal)
        .filter(|sample| {
            split_atoms_for_sample(sample)
                .iter()
                .any(|atom| atom == split_atom)
        })
        .collect()
}

fn package_paths_by_fingerprint(online_miner: &Value) -> Result<BTreeMap<u64, PathBuf>, String> {
    let mut paths = BTreeMap::new();
    for checkpoint in online_miner
        .get("checkpoints")
        .and_then(Value::as_array)
        .ok_or_else(|| "online miner report missing checkpoints".to_owned())?
    {
        let fingerprint = json_u64(checkpoint, &["package_fingerprint64"]).unwrap_or(0);
        if fingerprint == 0 {
            continue;
        }
        let Some(path) = json_string(checkpoint, &["package_path"]) else {
            continue;
        };
        paths.insert(fingerprint, PathBuf::from(path));
    }
    Ok(paths)
}

fn np_replay_events_from_traces(
    trace_paths: &[PathBuf],
    selected_bucket_tasks: &BTreeMap<String, (String, String)>,
    samples_by_bucket: &BTreeMap<String, Vec<DecisionSample>>,
) -> Result<BTreeMap<(usize, String, String), NpReplayEvent>, String> {
    let mut events = BTreeMap::new();
    let mut parsed_events = 0usize;
    let mut denominator_row_index = 0usize;
    let mut wanted = BTreeMap::<(usize, String), Vec<(String, String, String)>>::new();
    for (bucket_key, samples) in samples_by_bucket {
        let Some((task_name, action_family_atom)) = selected_bucket_tasks.get(bucket_key) else {
            continue;
        };
        for sample in samples {
            wanted
                .entry((
                    sample.denominator_row_index,
                    sample.request_fingerprint.clone(),
                ))
                .or_default()
                .push((
                    bucket_key.clone(),
                    task_name.clone(),
                    action_family_atom.clone(),
                ));
        }
    }
    for trace_path in trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read NP replay trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            denominator_row_index += 1;
            let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse NP replay trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if row
                .get("verified_safe_accept")
                .and_then(Value::as_bool)
                .is_none()
            {
                continue;
            }
            let request_fingerprint = json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("np-replay-row:{denominator_row_index}"));
            if let Some(wanted_rows) = wanted.get(&(denominator_row_index, request_fingerprint)) {
                for (bucket_key, task_name, action_family) in wanted_rows {
                    if !selected_bucket_tasks.contains_key(bucket_key) {
                        continue;
                    }
                    let Some(event) = parse_phase_atom_binary_event_for_action(
                        &row,
                        parsed_events,
                        action_family,
                        task_name,
                    ) else {
                        continue;
                    };
                    parsed_events += 1;
                    events.insert(
                        (
                            denominator_row_index,
                            bucket_key.clone(),
                            event.request_fingerprint.clone(),
                        ),
                        NpReplayEvent { event },
                    );
                }
            } else {
                let action_atoms = phase_atom_string_vec(&row, "action_atoms");
                let action_families = phase_atom_action_families(&action_atoms);
                if action_families.is_empty() {
                    continue;
                }
                let request_atoms = phase_atom_string_vec(&row, "request_atoms");
                let state_atoms = phase_atom_string_vec(&row, "state_atoms");
                let tool_atoms = phase_atom_string_vec(&row, "tool_atoms");
                let route_hint_atoms = phase_atom_string_vec(&row, "route_hint_atoms");
                if request_atoms.is_empty()
                    && state_atoms.is_empty()
                    && tool_atoms.is_empty()
                    && route_hint_atoms.is_empty()
                {
                    continue;
                }
                for action_family in action_families {
                    let bucket_key = phase_atom_state_action_bucket_key(
                        &action_family,
                        &request_atoms,
                        &state_atoms,
                        &tool_atoms,
                        &route_hint_atoms,
                    );
                    let Some((task_name, _)) = selected_bucket_tasks.get(&bucket_key) else {
                        continue;
                    };
                    let Some(event) = parse_phase_atom_binary_event_for_action(
                        &row,
                        parsed_events,
                        &action_family,
                        task_name,
                    ) else {
                        continue;
                    };
                    parsed_events += 1;
                    events.insert(
                        (
                            denominator_row_index,
                            bucket_key.clone(),
                            event.request_fingerprint.clone(),
                        ),
                        NpReplayEvent { event },
                    );
                }
            }
        }
    }
    Ok(events)
}

fn load_np_replay_runtime_entry(
    package_path: &Path,
    threshold_micro: i64,
    expected_fingerprint64: u64,
) -> Result<NpReplayRuntimeEntry, String> {
    let package_bytes = std::fs::read(package_path).map_err(|error| {
        format!(
            "failed to read NP replay package '{}': {error}",
            package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).map_err(|error| {
            format!(
                "failed to inspect NP replay package '{}': {error:?}",
                package_path.display()
            )
        })?;
    if expected_fingerprint64 != 0 && package_info.fingerprint64 != expected_fingerprint64 {
        return Err(format!(
            "NP replay package fingerprint mismatch '{}': expected {}, got {}",
            package_path.display(),
            expected_fingerprint64,
            package_info.fingerprint64
        ));
    }
    let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        PhaseCenterOffloadPolicy::new(threshold_micro)
            .map_err(|error| format!("invalid NP replay policy: {error:?}"))?,
    )
    .map_err(|error| format!("failed to load NP replay runtime: {error:?}"))?;
    let profile_ids = (0..runtime.record_count() as u32).collect::<Vec<_>>();
    let thresholds = vec![threshold_micro; runtime.record_count()];
    let hot_runtime =
        PhaseCenterHotRuntime::from_flat_runtime(runtime.runtime(), &profile_ids, &thresholds)
            .map_err(|error| format!("failed to build NP replay hot runtime: {error:?}"))?;
    let route_plan = hot_runtime
        .route_plan_from_profile_ids(0, [0u32])
        .map_err(|error| format!("failed to build NP replay route plan: {error:?}"))?
        .ok_or_else(|| "NP replay route plan has no profiles".to_owned())?;
    let hot_routes = PhaseCenterHotRouteTable::from_plans([route_plan])
        .map_err(|error| format!("failed to build NP replay route table: {error:?}"))?;
    let hot_scratch = PhaseCenterHotScratch::new(runtime.cells(), runtime.record_count())
        .map_err(|error| format!("failed to build NP replay scratch: {error:?}"))?;
    Ok(NpReplayRuntimeEntry {
        runtime,
        hot_runtime,
        hot_routes,
        hot_scratch,
    })
}

fn permille(value: usize, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    value.saturating_mul(1000) / total
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSON '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON '{}': {error}", path.display()))
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create report directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize report '{}': {error}", path.display()))?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write report '{}': {error}", path.display()))
}

fn np_runtime_replay_billing_request_path(report_path: &Path) -> PathBuf {
    let file_name = report_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("phase-stream-online-miner-portfolio-np-rescue-runtime-replay-v1.report.json");
    let output_name = if let Some(stem) = file_name.strip_suffix(".report.json") {
        format!("{stem}.billing-request.jsonl")
    } else if let Some(stem) = file_name.strip_suffix(".json") {
        format!("{stem}.billing-request.jsonl")
    } else {
        format!("{file_name}.billing-request.jsonl")
    };
    report_path.with_file_name(output_name)
}

fn write_jsonl_value_file(path: &Path, rows: &[Value]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create JSONL directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(path)
        .map_err(|error| format!("failed to create JSONL '{}': {error}", path.display()))?;
    let mut writer = BufWriter::new(file);
    for row in rows {
        serde_json::to_writer(&mut writer, row)
            .map_err(|error| format!("failed to serialize JSONL '{}': {error}", path.display()))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("failed to write JSONL '{}': {error}", path.display()))?;
    }
    writer
        .flush()
        .map_err(|error| format!("failed to flush JSONL '{}': {error}", path.display()))
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)?.as_str().map(ToOwned::to_owned)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path)?.as_bool()
}

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    json_at(value, path)?.as_i64()
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|raw| usize::try_from(raw).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path)?.as_u64()
}

fn json_string_array(value: &Value, path: &[&str]) -> Vec<String> {
    json_at(value, path)
        .map(json_string_vec)
        .unwrap_or_default()
}

fn json_string_vec(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
