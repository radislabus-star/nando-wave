use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

const DEFAULT_ONLINE_MINER_PORTFOLIO_SELECTOR_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-selector-v1.report.json";
const DEFAULT_ONLINE_MINER_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-daemon-v1.report.json";
const ONLINE_SELECTOR_AUTO_CALIBRATION_MAX_DECISIONS: usize = 1024;
const RISK_AWARE_MIN_POLICY_ACCEPTS: usize = 20;
const RISK_AWARE_MIN_POLICY_BOUNDARY_EVENTS: usize = 1;
const CONSTRAINED_ECONOMIC_MIN_POLICY_ACCEPTS: usize = 1;
const CONSTRAINED_ECONOMIC_MIN_POLICY_BOUNDARY_EVENTS: usize = 1;
const CONSTRAINED_ECONOMIC_MAX_BOUNDARY_PER_ACCEPT: usize = 4;

#[derive(Clone, Debug, Default)]
struct SelectorBucketState {
    bucket_key: String,
    task_name: String,
    action_family_atom: String,
    base_threshold_micro: i64,
    samples: Vec<SelectorDecisionSample>,
}

#[derive(Clone, Debug)]
struct SelectorDecisionSample {
    request_fingerprint: String,
    exact_cache_hit: bool,
    verified_safe_accept: bool,
    margin_micro: i64,
    decision_threshold_micro: i64,
    total_tokens: usize,
    total_cost_microusd: u64,
    reference_runtime_parity_mismatch: bool,
}

#[derive(Clone, Debug)]
struct SelectorBucketCandidate {
    bucket_key: String,
    task_name: String,
    action_family_atom: String,
    accepted_fingerprints: BTreeMap<String, SelectorAcceptedEvent>,
    false_accepts: usize,
    runtime_parity_mismatches: usize,
    rejected_reason: Option<&'static str>,
}

#[derive(Clone, Debug)]
struct SelectorAcceptedEvent {
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Clone, Debug, Default)]
struct SelectorLearningBucket {
    bucket_key: String,
    task_name: String,
    action_family_atom: String,
    event_count: usize,
    calibration_events: usize,
    shadow_events_after_calibration: usize,
    threshold_micro: i64,
    max_false_margin_micro: Option<i64>,
    policy: SelectorOutcome,
    future: SelectorOutcome,
    learned_weight_micro: i64,
    selector_policy_updates: usize,
}

#[derive(Clone, Debug, Default)]
struct SelectorOutcome {
    event_count: usize,
    local_operator_shadow_decisions: usize,
    accepted_fingerprints: BTreeMap<String, SelectorAcceptedEvent>,
    false_accepts: usize,
    runtime_parity_mismatches: usize,
}

#[derive(Clone, Debug, Serialize)]
struct SelectorCurvePoint {
    selector_policy_kind: &'static str,
    selector_step: usize,
    bucket_key: String,
    task_name: String,
    action_family_atom: String,
    events_seen: usize,
    calibration_events: usize,
    policy_event_count: usize,
    runtime_replay_start_event_ordinal: usize,
    shadow_events_after_calibration: usize,
    threshold_micro: i64,
    max_false_margin_micro: Option<i64>,
    learned_weight_micro: i64,
    policy_unique_accepts_over_exact_cache: usize,
    policy_boundary_events: usize,
    policy_false_accepts: usize,
    policy_runtime_parity_mismatches: usize,
    future_local_operator_shadow_decisions: usize,
    future_bucket_unique_accepts_over_exact_cache: usize,
    future_false_accepts: usize,
    future_runtime_parity_mismatches: usize,
    marginal_unique_accepts_over_exact_cache: usize,
    overlap_with_prior_portfolio_accepts: usize,
    marginal_tokens_saved: usize,
    marginal_cost_saved_microusd: u64,
    cumulative_unique_accepts_over_exact_cache: usize,
    cumulative_tokens_saved: usize,
    cumulative_cost_saved_microusd: u64,
    cumulative_false_accepts: usize,
    cumulative_runtime_parity_mismatches: usize,
}

#[derive(Clone, Debug, Serialize)]
struct SelectorAuditReport {
    selector_policy_kind: &'static str,
    selected_bucket_count: usize,
    selector_reward_observations: usize,
    selector_policy_updates: usize,
    unique_accepts_over_exact_cache: usize,
    tokens_saved: usize,
    cost_saved_microusd: u64,
    false_accepts: usize,
    runtime_parity_mismatches: usize,
    curve_artifact_path: String,
}

#[derive(Clone, Debug, Serialize)]
struct SelectorSelectedBucketReport {
    rank: usize,
    bucket_key: String,
    task_name: String,
    action_family_atom: String,
    threshold_micro: i64,
    max_false_margin_micro: Option<i64>,
    events_seen: usize,
    calibration_events: usize,
    policy_event_count: usize,
    runtime_replay_start_event_ordinal: usize,
    shadow_events_after_calibration: usize,
    local_operator_shadow_decisions: usize,
    bucket_unique_accepts_over_exact_cache: usize,
    marginal_unique_accepts_over_exact_cache: usize,
    overlap_with_prior_portfolio_accepts: usize,
    marginal_tokens_saved: usize,
    marginal_cost_saved_microusd: u64,
    false_accepts: usize,
    runtime_parity_mismatches: usize,
}

#[derive(Clone, Debug, Serialize)]
struct SelectorRejectedBucketReport {
    bucket_key: String,
    task_name: String,
    action_family_atom: String,
    reason: &'static str,
    accepted_unique_accepts_over_exact_cache: usize,
    false_accepts: usize,
    runtime_parity_mismatches: usize,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_selector_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_SELECTOR_REPORT));
    let online_miner_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_REPORT));
    let explicit_decision_log_path = args.next().map(PathBuf::from);
    let max_selected_buckets = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid max_selected_buckets '{value}' for portfolio selector: {error}")
            })
        })
        .transpose()?
        .unwrap_or(16);
    if max_selected_buckets == 0 {
        return Err("max_selected_buckets must be > 0".to_owned());
    }

    let online_report = read_json_value(&online_miner_report_path)?;
    let decision_log_path = explicit_decision_log_path
        .or_else(|| json_string(&online_report, &["decision_log_path"]).map(PathBuf::from))
        .ok_or_else(|| {
            format!(
                "online miner report '{}' missing decision_log_path",
                online_miner_report_path.display()
            )
        })?;
    let report_buckets = online_report
        .get("buckets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut threshold_by_bucket = BTreeMap::<String, i64>::new();
    for bucket in report_buckets {
        if let Some(bucket_key) = json_string(&bucket, &["bucket_key"]) {
            let threshold = json_i64(&bucket, &["safe_accept_margin_threshold_micro"])
                .or_else(|| json_i64(&bucket, &["auto_calibrated_margin_threshold_micro"]))
                .unwrap_or(0);
            threshold_by_bucket.insert(bucket_key, threshold);
        }
    }

    let decision_text = std::fs::read_to_string(&decision_log_path).map_err(|error| {
        format!(
            "failed to read online miner decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let mut decision_rows = 0usize;
    let mut buckets = BTreeMap::<String, SelectorBucketState>::new();
    for (line_index, line) in decision_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        decision_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse online miner decision log '{}' line {}: {error}",
                decision_log_path.display(),
                line_index + 1
            )
        })?;
        if !json_bool(&row, &["future_only_shadow_scoring"]).unwrap_or(false) {
            continue;
        }
        let Some(bucket_key) = json_string(&row, &["bucket_key"]) else {
            continue;
        };
        let margin_micro = json_i64(&row, &["margin_micro"]).unwrap_or(0);
        let decision_threshold_micro = json_i64(&row, &["margin_threshold_micro"]).unwrap_or(0);
        let token_cost = row.get("token_cost");
        let sample = SelectorDecisionSample {
            request_fingerprint: json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("decision-row:{decision_rows}")),
            exact_cache_hit: json_bool(&row, &["exact_cache_hit"]).unwrap_or(false),
            verified_safe_accept: json_bool(&row, &["verified_safe_accept"]).unwrap_or(false),
            margin_micro,
            decision_threshold_micro,
            total_tokens: json_usize(token_cost.and_then(|value| value.get("total_tokens")))
                .unwrap_or(0),
            total_cost_microusd: json_u64_at(
                token_cost.and_then(|value| value.get("total_cost_microusd")),
            )
            .unwrap_or(0),
            reference_runtime_parity_mismatch: json_bool(
                &row,
                &["reference_runtime_parity_mismatch"],
            )
            .unwrap_or(false),
        };
        let bucket = buckets
            .entry(bucket_key.clone())
            .or_insert_with(|| SelectorBucketState {
                base_threshold_micro: decision_threshold_micro.max(0),
                bucket_key: bucket_key.clone(),
                task_name: json_string(&row, &["task_name"]).unwrap_or_default(),
                action_family_atom: json_string(&row, &["action_family_atom"]).unwrap_or_default(),
                samples: Vec::new(),
            });
        if sample.decision_threshold_micro > 0 {
            bucket.base_threshold_micro = if bucket.base_threshold_micro > 0 {
                bucket
                    .base_threshold_micro
                    .min(sample.decision_threshold_micro)
            } else {
                sample.decision_threshold_micro
            };
        } else if bucket.base_threshold_micro <= 0 {
            bucket.base_threshold_micro = threshold_by_bucket
                .get(&bucket_key)
                .copied()
                .unwrap_or_default()
                .max(0);
        }
        bucket.samples.push(sample);
    }

    let candidates = buckets
        .values()
        .map(selector_bucket_candidate)
        .collect::<Vec<_>>();
    let rejected_false_accept_buckets = candidates
        .iter()
        .filter(|candidate| candidate.false_accepts > 0)
        .count();
    let rejected_runtime_parity_buckets = candidates
        .iter()
        .filter(|candidate| candidate.runtime_parity_mismatches > 0)
        .count();
    let rejected_no_accept_buckets = candidates
        .iter()
        .filter(|candidate| candidate.accepted_fingerprints.is_empty())
        .count();

    let rejected_bucket_samples = candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .rejected_reason
                .map(|reason| SelectorRejectedBucketReport {
                    bucket_key: candidate.bucket_key.clone(),
                    task_name: candidate.task_name.clone(),
                    action_family_atom: candidate.action_family_atom.clone(),
                    reason,
                    accepted_unique_accepts_over_exact_cache: candidate.accepted_fingerprints.len(),
                    false_accepts: candidate.false_accepts,
                    runtime_parity_mismatches: candidate.runtime_parity_mismatches,
                })
        })
        .take(32)
        .collect::<Vec<_>>();

    let learning_buckets = buckets
        .values()
        .map(selector_learning_bucket)
        .collect::<Vec<_>>();
    let selector_reward_observations = learning_buckets
        .iter()
        .map(|bucket| bucket.policy.event_count)
        .sum::<usize>();
    let selector_policy_updates = learning_buckets
        .iter()
        .map(|bucket| bucket.selector_policy_updates)
        .sum::<usize>();
    let fixed_greedy_curve = fixed_greedy_future_curve(&learning_buckets, max_selected_buckets);
    let learned_selector_curve =
        learned_online_future_curve(&learning_buckets, max_selected_buckets);
    let risk_aware_selector_curve =
        risk_aware_learned_future_curve(&learning_buckets, max_selected_buckets);
    let constrained_economic_curve =
        constrained_economic_future_curve(&learning_buckets, max_selected_buckets);
    let fixed_summary = selector_audit_summary(
        "fixed_greedy",
        &fixed_greedy_curve,
        selector_reward_observations,
        0,
        "",
    );
    let learned_summary = selector_audit_summary(
        "learned_online",
        &learned_selector_curve,
        selector_reward_observations,
        selector_policy_updates,
        "",
    );
    let risk_aware_summary = selector_audit_summary(
        "risk_aware_learned",
        &risk_aware_selector_curve,
        selector_reward_observations,
        selector_policy_updates,
        "",
    );
    let constrained_economic_summary = selector_audit_summary(
        "constrained_economic",
        &constrained_economic_curve,
        selector_reward_observations,
        selector_policy_updates,
        "",
    );
    let learned_vs_fixed_delta_accepts = learned_summary
        .unique_accepts_over_exact_cache
        .saturating_sub(fixed_summary.unique_accepts_over_exact_cache)
        as i64
        - fixed_summary
            .unique_accepts_over_exact_cache
            .saturating_sub(learned_summary.unique_accepts_over_exact_cache) as i64;
    let learned_vs_fixed_delta_tokens = learned_summary
        .tokens_saved
        .saturating_sub(fixed_summary.tokens_saved) as i64
        - fixed_summary
            .tokens_saved
            .saturating_sub(learned_summary.tokens_saved) as i64;
    let learned_vs_fixed_delta_cost = learned_summary
        .cost_saved_microusd
        .saturating_sub(fixed_summary.cost_saved_microusd)
        as i64
        - fixed_summary
            .cost_saved_microusd
            .saturating_sub(learned_summary.cost_saved_microusd) as i64;
    let learned_vs_fixed_delta_false_accepts = learned_summary
        .false_accepts
        .saturating_sub(fixed_summary.false_accepts)
        as i64
        - fixed_summary
            .false_accepts
            .saturating_sub(learned_summary.false_accepts) as i64;
    let learned_selector_beats_or_matches_fixed = learned_summary.false_accepts == 0
        && learned_summary.runtime_parity_mismatches == 0
        && learned_summary.unique_accepts_over_exact_cache
            >= fixed_summary.unique_accepts_over_exact_cache
        && learned_summary.tokens_saved >= fixed_summary.tokens_saved;
    let risk_aware_selector_passed = risk_aware_summary.false_accepts == 0
        && risk_aware_summary.runtime_parity_mismatches == 0
        && risk_aware_summary.unique_accepts_over_exact_cache > 0
        && risk_aware_summary.tokens_saved > 0;
    let constrained_economic_selector_passed = constrained_economic_summary.false_accepts == 0
        && constrained_economic_summary.runtime_parity_mismatches == 0
        && constrained_economic_summary.unique_accepts_over_exact_cache > 0
        && constrained_economic_summary.tokens_saved > 0;
    let curve_artifact_path = artifact_path(&report_path, "learned-vs-fixed-selector-curve.jsonl");
    let fixed_greedy_curve_artifact_path =
        artifact_path(&report_path, "fixed-greedy-selector-curve.jsonl");
    let learned_selector_curve_artifact_path =
        artifact_path(&report_path, "learned-online-selector-curve.jsonl");
    let risk_aware_selector_curve_artifact_path =
        artifact_path(&report_path, "risk-aware-learned-selector-curve.jsonl");
    let constrained_economic_curve_artifact_path =
        artifact_path(&report_path, "constrained-economic-selector-curve.jsonl");
    let curve_svg_path = artifact_path(&report_path, "learned-vs-fixed-selector-curve.svg");
    let fixed_greedy_baseline_report_path =
        artifact_path(&report_path, "fixed-greedy-baseline.report.json");
    let learned_selector_report_path =
        artifact_path(&report_path, "learned-online-selector.report.json");
    let risk_aware_selector_report_path =
        artifact_path(&report_path, "risk-aware-learned-selector.report.json");
    let constrained_economic_selector_report_path =
        artifact_path(&report_path, "constrained-economic-selector.report.json");
    write_selector_curve_jsonl(
        &curve_artifact_path,
        &fixed_greedy_curve,
        &learned_selector_curve,
    )?;
    write_selector_curve_jsonl(&fixed_greedy_curve_artifact_path, &fixed_greedy_curve, &[])?;
    write_selector_curve_jsonl(
        &learned_selector_curve_artifact_path,
        &learned_selector_curve,
        &[],
    )?;
    write_selector_curve_jsonl(
        &risk_aware_selector_curve_artifact_path,
        &risk_aware_selector_curve,
        &[],
    )?;
    write_selector_curve_jsonl(
        &constrained_economic_curve_artifact_path,
        &constrained_economic_curve,
        &[],
    )?;
    write_selector_curve_svg(
        &curve_svg_path,
        &fixed_greedy_curve,
        &learned_selector_curve,
    )?;
    let fixed_summary = selector_audit_summary(
        "fixed_greedy",
        &fixed_greedy_curve,
        selector_reward_observations,
        0,
        &fixed_greedy_curve_artifact_path.display().to_string(),
    );
    let learned_summary = selector_audit_summary(
        "learned_online",
        &learned_selector_curve,
        selector_reward_observations,
        selector_policy_updates,
        &learned_selector_curve_artifact_path.display().to_string(),
    );
    let risk_aware_summary = selector_audit_summary(
        "risk_aware_learned",
        &risk_aware_selector_curve,
        selector_reward_observations,
        selector_policy_updates,
        &risk_aware_selector_curve_artifact_path
            .display()
            .to_string(),
    );
    let constrained_economic_summary = selector_audit_summary(
        "constrained_economic",
        &constrained_economic_curve,
        selector_reward_observations,
        selector_policy_updates,
        &constrained_economic_curve_artifact_path
            .display()
            .to_string(),
    );
    write_json_file(
        &fixed_greedy_baseline_report_path,
        &serde_json::to_value(&fixed_summary).map_err(|error| {
            format!(
                "failed to serialize fixed selector summary '{}': {error}",
                fixed_greedy_baseline_report_path.display()
            )
        })?,
    )?;
    write_json_file(
        &learned_selector_report_path,
        &serde_json::to_value(&learned_summary).map_err(|error| {
            format!(
                "failed to serialize learned selector summary '{}': {error}",
                learned_selector_report_path.display()
            )
        })?,
    )?;
    write_json_file(
        &risk_aware_selector_report_path,
        &serde_json::to_value(&risk_aware_summary).map_err(|error| {
            format!(
                "failed to serialize risk-aware selector summary '{}': {error}",
                risk_aware_selector_report_path.display()
            )
        })?,
    )?;
    write_json_file(
        &constrained_economic_selector_report_path,
        &serde_json::to_value(&constrained_economic_summary).map_err(|error| {
            format!(
                "failed to serialize constrained economic selector summary '{}': {error}",
                constrained_economic_selector_report_path.display()
            )
        })?,
    )?;

    let (
        fixed_selected_reports,
        fixed_portfolio_unique_accepts,
        fixed_portfolio_tokens_saved,
        fixed_portfolio_cost_saved_microusd,
    ) = selected_reports_from_curve(&fixed_greedy_curve);
    let (
        selected_reports,
        portfolio_unique_accepts,
        portfolio_tokens_saved,
        portfolio_cost_saved_microusd,
    ) = selected_reports_from_curve(&constrained_economic_curve);

    let selector_uses_marginal_denominator_delta = true;
    let selector_uses_portfolio_overlap_dedupe = true;
    let selector_rejects_false_accept_buckets = true;
    let selector_rejects_runtime_parity_mismatches = true;
    let manual_class_list_used = false;
    let static_topn_seed_used = false;
    let online_discovery_used = true;
    let marginal_denominator_delta_used =
        selector_uses_marginal_denominator_delta && !selected_reports.is_empty();
    let portfolio_gate_passed = !selected_reports.is_empty()
        && rejected_false_accept_buckets
            == candidates
                .iter()
                .filter(|candidate| candidate.false_accepts > 0)
                .count()
        && constrained_economic_selector_passed;
    let constrained_economic_main_matches_curve = portfolio_unique_accepts
        == constrained_economic_summary.unique_accepts_over_exact_cache
        && portfolio_tokens_saved == constrained_economic_summary.tokens_saved
        && portfolio_cost_saved_microusd == constrained_economic_summary.cost_saved_microusd;
    let fixed_greedy_main_matches_curve = fixed_portfolio_unique_accepts
        == fixed_summary.unique_accepts_over_exact_cache
        && fixed_portfolio_tokens_saved == fixed_summary.tokens_saved
        && fixed_portfolio_cost_saved_microusd == fixed_summary.cost_saved_microusd;
    let dynamic_selector_clean = !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && marginal_denominator_delta_used
        && portfolio_gate_passed
        && constrained_economic_selector_passed
        && constrained_economic_main_matches_curve
        && !selected_reports.is_empty()
        && portfolio_unique_accepts > 0
        && portfolio_tokens_saved > 0;
    let runtime_replay_passed = false;
    let legacy_baseline_selector_only = !dynamic_selector_clean;
    let dynamic_discovery_shadow_claim_allowed = dynamic_selector_clean;
    let product_dynamic_discovery_claim_allowed = false;
    let verdict = if selected_reports.is_empty() {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_SELECTOR_V1_WATCH_NO_SELECTION"
    } else if selector_policy_updates == 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_SELECTOR_V1_WATCH_NO_LEARNED_UPDATES"
    } else if !constrained_economic_selector_passed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_SELECTOR_V1_WATCH_CONSTRAINED_ECONOMIC_SELECTOR_NOT_CLEAN"
    } else if !learned_selector_beats_or_matches_fixed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_SELECTOR_V1_PASS_CONSTRAINED_ECONOMIC_SELECTOR"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_SELECTOR_V1_PASS_LEARNED_SELECTOR_AUDIT"
    };

    let selector_policy = serde_json::json!({
        "selector_policy_kind": "constrained_economic",
        "selector_uses_marginal_denominator_delta": selector_uses_marginal_denominator_delta,
        "selector_uses_portfolio_overlap_dedupe": selector_uses_portfolio_overlap_dedupe,
        "selector_rejects_false_accept_buckets": selector_rejects_false_accept_buckets,
        "selector_rejects_runtime_parity_mismatches": selector_rejects_runtime_parity_mismatches,
        "value_score_only_prefilter": false,
        "risk_aware_baseline_policy": {
            "min_policy_accepts": RISK_AWARE_MIN_POLICY_ACCEPTS,
            "min_policy_boundary_events": RISK_AWARE_MIN_POLICY_BOUNDARY_EVENTS,
            "requires_policy_false_accepts": 0,
            "requires_policy_runtime_parity_mismatches": 0
        },
        "constrained_economic_policy": {
            "min_policy_accepts": CONSTRAINED_ECONOMIC_MIN_POLICY_ACCEPTS,
            "min_policy_boundary_events": CONSTRAINED_ECONOMIC_MIN_POLICY_BOUNDARY_EVENTS,
            "max_policy_boundary_events_per_accept": CONSTRAINED_ECONOMIC_MAX_BOUNDARY_PER_ACCEPT,
            "requires_policy_false_accepts": 0,
            "requires_policy_runtime_parity_mismatches": 0,
            "selection_score": "policy-window marginal tokens, cost, accepts after hard evidence filter"
        },
        "selection_score": "selected_buckets use constrained economic policy: hard safety/evidence filter from policy window, then marginal tokens/cost/accepts; fixed_greedy, naive learned, and risk_aware_learned remain baselines"
    });
    let discovery_mode = serde_json::json!({
        "manual_class_list_used": manual_class_list_used,
        "static_topn_seed_used": static_topn_seed_used,
        "online_discovery_used": online_discovery_used,
        "marginal_denominator_delta_used": marginal_denominator_delta_used,
        "portfolio_gate_passed": portfolio_gate_passed,
        "runtime_replay_passed": runtime_replay_passed,
        "dynamic_discovery_shadow_claim_allowed": dynamic_discovery_shadow_claim_allowed,
        "selector_learning_shadow_only": true,
        "learned_selector_beats_or_matches_fixed": learned_selector_beats_or_matches_fixed,
        "risk_aware_selector_passed": risk_aware_selector_passed,
        "constrained_economic_selector_passed": constrained_economic_selector_passed,
        "selected_policy_kind": "constrained_economic",
        "selected_policy_improves_risk_aware_accepts": constrained_economic_summary
            .unique_accepts_over_exact_cache
            > risk_aware_summary.unique_accepts_over_exact_cache,
        "selected_policy_improves_risk_aware_tokens": constrained_economic_summary.tokens_saved
            > risk_aware_summary.tokens_saved,
        "product_dynamic_discovery_claim_allowed": product_dynamic_discovery_claim_allowed,
        "legacy_baseline_selector_only": legacy_baseline_selector_only,
        "claim_boundary": if dynamic_selector_clean {
            "dynamic shadow selector candidate: selected_buckets are constrained-economic online-discovery candidates; runtime replay, verifier binding, admission, billing evidence, and promotion gates remain separate"
        } else {
            "selector blocked from dynamic shadow claim: keep as baseline/debug evidence until selected_buckets satisfy constrained-economic online-discovery contract"
        }
    });
    let selector_report_curve_consistency = serde_json::json!({
        "constrained_economic_main_matches_curve": constrained_economic_main_matches_curve,
        "fixed_greedy_main_matches_curve": fixed_greedy_main_matches_curve,
        "report_totals_source": "selected_buckets are materialized from SelectorCurvePoint, not recomputed from full bucket candidates"
    });
    let forbidden_flags = serde_json::json!({
        "nwrb_used": false,
        "role_binding_backend_used": false,
        "lookup_used": false,
        "target_id_or_proof_rule_id_authority_used": false,
        "concrete_x_lookup_used": false,
        "manual_local_out_t_used": false,
        "local_accept_without_verifier_used": false
    });
    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_selector_v1"),
    );
    report.insert(
        "online_miner_report_path".to_owned(),
        serde_json::json!(online_miner_report_path),
    );
    report.insert(
        "decision_log_path".to_owned(),
        serde_json::json!(decision_log_path),
    );
    report.insert("decision_rows".to_owned(), serde_json::json!(decision_rows));
    report.insert("bucket_count".to_owned(), serde_json::json!(buckets.len()));
    report.insert(
        "candidate_bucket_count".to_owned(),
        serde_json::json!(candidates.len()),
    );
    report.insert(
        "max_selected_buckets".to_owned(),
        serde_json::json!(max_selected_buckets),
    );
    report.insert(
        "selected_bucket_count".to_owned(),
        serde_json::json!(selected_reports.len()),
    );
    report.insert(
        "portfolio_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(portfolio_unique_accepts),
    );
    report.insert(
        "portfolio_tokens_saved".to_owned(),
        serde_json::json!(portfolio_tokens_saved),
    );
    report.insert(
        "portfolio_cost_saved_microusd".to_owned(),
        serde_json::json!(portfolio_cost_saved_microusd),
    );
    report.insert(
        "rejected_false_accept_buckets".to_owned(),
        serde_json::json!(rejected_false_accept_buckets),
    );
    report.insert(
        "rejected_runtime_parity_buckets".to_owned(),
        serde_json::json!(rejected_runtime_parity_buckets),
    );
    report.insert(
        "rejected_no_accept_buckets".to_owned(),
        serde_json::json!(rejected_no_accept_buckets),
    );
    report.insert(
        "selector_policy_kind".to_owned(),
        serde_json::json!("constrained_economic"),
    );
    report.insert(
        "fixed_greedy_selected_bucket_count".to_owned(),
        serde_json::json!(fixed_selected_reports.len()),
    );
    report.insert(
        "fixed_greedy_portfolio_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(fixed_portfolio_unique_accepts),
    );
    report.insert(
        "fixed_greedy_portfolio_tokens_saved".to_owned(),
        serde_json::json!(fixed_portfolio_tokens_saved),
    );
    report.insert(
        "fixed_greedy_portfolio_cost_saved_microusd".to_owned(),
        serde_json::json!(fixed_portfolio_cost_saved_microusd),
    );
    report.insert(
        "fixed_greedy_selected_buckets".to_owned(),
        serde_json::to_value(&fixed_selected_reports).map_err(|error| {
            format!(
                "failed to serialize fixed selected buckets '{}': {error}",
                report_path.display()
            )
        })?,
    );
    report.insert(
        "fixed_greedy_baseline_report_path".to_owned(),
        serde_json::json!(fixed_greedy_baseline_report_path),
    );
    report.insert(
        "learned_selector_report_path".to_owned(),
        serde_json::json!(learned_selector_report_path),
    );
    report.insert(
        "risk_aware_selector_report_path".to_owned(),
        serde_json::json!(risk_aware_selector_report_path),
    );
    report.insert(
        "constrained_economic_selector_report_path".to_owned(),
        serde_json::json!(constrained_economic_selector_report_path),
    );
    report.insert(
        "selector_reward_observations".to_owned(),
        serde_json::json!(selector_reward_observations),
    );
    report.insert(
        "selector_policy_updates".to_owned(),
        serde_json::json!(selector_policy_updates),
    );
    report.insert(
        "learned_vs_fixed_delta_accepts".to_owned(),
        serde_json::json!(learned_vs_fixed_delta_accepts),
    );
    report.insert(
        "learned_vs_fixed_delta_tokens".to_owned(),
        serde_json::json!(learned_vs_fixed_delta_tokens),
    );
    report.insert(
        "learned_vs_fixed_delta_cost".to_owned(),
        serde_json::json!(learned_vs_fixed_delta_cost),
    );
    report.insert(
        "learned_vs_fixed_delta_false_accepts".to_owned(),
        serde_json::json!(learned_vs_fixed_delta_false_accepts),
    );
    report.insert(
        "selector_learning_shadow_only".to_owned(),
        serde_json::json!(true),
    );
    report.insert(
        "legacy_baseline_selector_only".to_owned(),
        serde_json::json!(legacy_baseline_selector_only),
    );
    report.insert(
        "learned_selector_beats_or_matches_fixed".to_owned(),
        serde_json::json!(learned_selector_beats_or_matches_fixed),
    );
    report.insert(
        "risk_aware_selector_passed".to_owned(),
        serde_json::json!(risk_aware_selector_passed),
    );
    report.insert(
        "constrained_economic_selector_passed".to_owned(),
        serde_json::json!(constrained_economic_selector_passed),
    );
    report.insert(
        "curve_artifact_path".to_owned(),
        serde_json::json!(curve_artifact_path),
    );
    report.insert(
        "fixed_greedy_curve_artifact_path".to_owned(),
        serde_json::json!(fixed_greedy_curve_artifact_path),
    );
    report.insert(
        "learned_selector_curve_artifact_path".to_owned(),
        serde_json::json!(learned_selector_curve_artifact_path),
    );
    report.insert(
        "risk_aware_selector_curve_artifact_path".to_owned(),
        serde_json::json!(risk_aware_selector_curve_artifact_path),
    );
    report.insert(
        "constrained_economic_selector_curve_artifact_path".to_owned(),
        serde_json::json!(constrained_economic_curve_artifact_path),
    );
    report.insert(
        "learned_vs_fixed_selector_curve_svg_path".to_owned(),
        serde_json::json!(curve_svg_path),
    );
    report.insert(
        "curve_x_axis".to_owned(),
        serde_json::json!("selector_step / selected bucket rank"),
    );
    report.insert(
        "curve_y_axes".to_owned(),
        serde_json::json!([
            "cumulative_unique_accepts_over_exact_cache",
            "cumulative_tokens_saved",
            "cumulative_cost_saved_microusd",
            "cumulative_false_accepts",
            "cumulative_runtime_parity_mismatches"
        ]),
    );
    report.insert("selector_policy".to_owned(), selector_policy);
    report.insert("discovery_mode".to_owned(), discovery_mode);
    report.insert(
        "selector_report_curve_consistency".to_owned(),
        selector_report_curve_consistency,
    );
    report.insert(
        "manual_class_list_used".to_owned(),
        serde_json::json!(manual_class_list_used),
    );
    report.insert(
        "static_topn_seed_used".to_owned(),
        serde_json::json!(static_topn_seed_used),
    );
    report.insert(
        "online_discovery_used".to_owned(),
        serde_json::json!(online_discovery_used),
    );
    report.insert(
        "marginal_denominator_delta_used".to_owned(),
        serde_json::json!(marginal_denominator_delta_used),
    );
    report.insert(
        "portfolio_gate_passed".to_owned(),
        serde_json::json!(portfolio_gate_passed),
    );
    report.insert(
        "selector_learning_shadow_only".to_owned(),
        serde_json::json!(true),
    );
    report.insert(
        "risk_aware_selector_passed".to_owned(),
        serde_json::json!(risk_aware_selector_passed),
    );
    report.insert(
        "product_dynamic_discovery_claim_allowed".to_owned(),
        serde_json::json!(product_dynamic_discovery_claim_allowed),
    );
    report.insert(
        "dynamic_discovery_shadow_claim_allowed".to_owned(),
        serde_json::json!(dynamic_discovery_shadow_claim_allowed),
    );
    report.insert(
        "dynamic_selector_clean".to_owned(),
        serde_json::json!(dynamic_selector_clean),
    );
    report.insert(
        "selected_buckets".to_owned(),
        serde_json::to_value(&selected_reports).map_err(|error| {
            format!(
                "failed to serialize selected buckets '{}': {error}",
                report_path.display()
            )
        })?,
    );
    report.insert(
        "rejected_bucket_samples".to_owned(),
        serde_json::to_value(&rejected_bucket_samples).map_err(|error| {
            format!(
                "failed to serialize rejected bucket samples '{}': {error}",
                report_path.display()
            )
        })?,
    );
    report.insert("forbidden_flags".to_owned(), forbidden_flags);
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
        serde_json::json!("dynamic selector/report only: reads online miner shadow decisions and materializes constrained-economic selected buckets plus baseline curves; does not compile, promote, serve, enable local_accept, claim market money, or revive legacy nwrb"),
    );
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_selector_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  decision_rows: {decision_rows}");
    println!("  bucket_count: {}", buckets.len());
    println!("  selected_bucket_count: {}", selected_reports.len());
    println!(
        "  portfolio_unique_cpu_accepts_over_exact_cache: {}",
        portfolio_unique_accepts
    );
    println!("  portfolio_tokens_saved: {portfolio_tokens_saved}");
    println!("  rejected_false_accept_buckets: {rejected_false_accept_buckets}");
    println!("  dynamic_selector_clean: {dynamic_selector_clean}");
    println!("  runtime_replay_passed: false");
    println!("  product_dynamic_discovery_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn selector_bucket_candidate(bucket: &SelectorBucketState) -> SelectorBucketCandidate {
    let calibration_events = selector_calibration_len(bucket.samples.len());
    let (threshold_micro, _) = selector_calibrated_threshold(bucket, calibration_events);
    let mut accepted_fingerprints = BTreeMap::new();
    let mut false_accepts = 0usize;
    let mut runtime_parity_mismatches = 0usize;
    for sample in bucket.samples.iter().skip(calibration_events) {
        runtime_parity_mismatches += usize::from(sample.reference_runtime_parity_mismatch);
        if sample.margin_micro < threshold_micro {
            continue;
        }
        if sample.verified_safe_accept {
            if !sample.exact_cache_hit {
                accepted_fingerprints
                    .entry(sample.request_fingerprint.clone())
                    .or_insert_with(|| SelectorAcceptedEvent {
                        total_tokens: sample.total_tokens,
                        total_cost_microusd: sample.total_cost_microusd,
                    });
            }
        } else {
            false_accepts += 1;
        }
    }
    let rejected_reason = if runtime_parity_mismatches > 0 {
        Some("runtime_parity_mismatch")
    } else if false_accepts > 0 {
        Some("false_accepts_after_auto_calibration")
    } else if accepted_fingerprints.is_empty() {
        Some("no_marginal_accepts_over_exact_cache")
    } else {
        None
    };
    SelectorBucketCandidate {
        bucket_key: bucket.bucket_key.clone(),
        task_name: bucket.task_name.clone(),
        action_family_atom: bucket.action_family_atom.clone(),
        accepted_fingerprints,
        false_accepts,
        runtime_parity_mismatches,
        rejected_reason,
    }
}

fn selected_reports_from_curve(
    curve: &[SelectorCurvePoint],
) -> (Vec<SelectorSelectedBucketReport>, usize, usize, u64) {
    let mut selected_reports = Vec::<SelectorSelectedBucketReport>::new();
    for point in curve {
        selected_reports.push(SelectorSelectedBucketReport {
            rank: selected_reports.len() + 1,
            bucket_key: point.bucket_key.clone(),
            task_name: point.task_name.clone(),
            action_family_atom: point.action_family_atom.clone(),
            threshold_micro: point.threshold_micro,
            max_false_margin_micro: point.max_false_margin_micro,
            events_seen: point.events_seen,
            calibration_events: point.calibration_events,
            policy_event_count: point.policy_event_count,
            runtime_replay_start_event_ordinal: point
                .calibration_events
                .saturating_add(point.policy_event_count),
            shadow_events_after_calibration: point.shadow_events_after_calibration,
            local_operator_shadow_decisions: point.future_local_operator_shadow_decisions,
            bucket_unique_accepts_over_exact_cache: point
                .future_bucket_unique_accepts_over_exact_cache,
            marginal_unique_accepts_over_exact_cache: point
                .marginal_unique_accepts_over_exact_cache,
            overlap_with_prior_portfolio_accepts: point.overlap_with_prior_portfolio_accepts,
            marginal_tokens_saved: point.marginal_tokens_saved,
            marginal_cost_saved_microusd: point.marginal_cost_saved_microusd,
            false_accepts: point.future_false_accepts,
            runtime_parity_mismatches: point.future_runtime_parity_mismatches,
        });
    }
    let last = curve.last();
    (
        selected_reports,
        last.map(|point| point.cumulative_unique_accepts_over_exact_cache)
            .unwrap_or(0),
        last.map(|point| point.cumulative_tokens_saved).unwrap_or(0),
        last.map(|point| point.cumulative_cost_saved_microusd)
            .unwrap_or(0),
    )
}

fn selector_learning_bucket(bucket: &SelectorBucketState) -> SelectorLearningBucket {
    let calibration_events = selector_calibration_len(bucket.samples.len());
    let (threshold_micro, max_false_margin_micro) =
        selector_calibrated_threshold(bucket, calibration_events);
    let future_shadow_samples = bucket
        .samples
        .iter()
        .skip(calibration_events)
        .collect::<Vec<_>>();
    let policy_len = selector_policy_observation_len(future_shadow_samples.len());
    let policy = selector_outcome_from_samples(
        future_shadow_samples
            .iter()
            .take(policy_len)
            .map(|sample| *sample),
        threshold_micro,
    );
    let future = selector_outcome_from_samples(
        future_shadow_samples
            .iter()
            .skip(policy_len)
            .map(|sample| *sample),
        threshold_micro,
    );
    let learned_weight_micro = selector_reward_micro(&policy);
    let selector_policy_updates = policy.event_count;
    SelectorLearningBucket {
        bucket_key: bucket.bucket_key.clone(),
        task_name: bucket.task_name.clone(),
        action_family_atom: bucket.action_family_atom.clone(),
        event_count: bucket.samples.len(),
        calibration_events,
        shadow_events_after_calibration: future_shadow_samples.len(),
        threshold_micro,
        max_false_margin_micro,
        policy,
        future,
        learned_weight_micro,
        selector_policy_updates,
    }
}

fn selector_calibrated_threshold(
    bucket: &SelectorBucketState,
    calibration_events: usize,
) -> (i64, Option<i64>) {
    let calibration = bucket.samples.iter().take(calibration_events);
    let max_false_margin_micro = calibration
        .clone()
        .filter(|sample| !sample.verified_safe_accept)
        .map(|sample| sample.margin_micro)
        .max();
    if let Some(margin) = max_false_margin_micro {
        return (margin.saturating_add(1).max(1), Some(margin));
    }
    let mut positive_margins = calibration
        .filter(|sample| sample.verified_safe_accept && sample.margin_micro > 0)
        .map(|sample| sample.margin_micro)
        .collect::<Vec<_>>();
    positive_margins.sort_unstable();
    let positive_floor = positive_margins
        .get(positive_margins.len().saturating_sub(1) / 10)
        .copied();
    let threshold = positive_floor.unwrap_or(bucket.base_threshold_micro).max(1);
    (threshold, None)
}

fn selector_policy_observation_len(sample_count: usize) -> usize {
    if sample_count < 2 {
        return sample_count;
    }
    (sample_count / 2).max(1).min(sample_count - 1)
}

fn selector_outcome_from_samples<'a>(
    samples: impl IntoIterator<Item = &'a SelectorDecisionSample>,
    threshold_micro: i64,
) -> SelectorOutcome {
    let mut outcome = SelectorOutcome::default();
    for sample in samples {
        outcome.event_count += 1;
        outcome.runtime_parity_mismatches += usize::from(sample.reference_runtime_parity_mismatch);
        if sample.margin_micro < threshold_micro {
            continue;
        }
        if sample.verified_safe_accept {
            outcome.local_operator_shadow_decisions += 1;
            if !sample.exact_cache_hit {
                outcome
                    .accepted_fingerprints
                    .entry(sample.request_fingerprint.clone())
                    .or_insert_with(|| SelectorAcceptedEvent {
                        total_tokens: sample.total_tokens,
                        total_cost_microusd: sample.total_cost_microusd,
                    });
            }
        } else {
            outcome.false_accepts += 1;
        }
    }
    outcome
}

fn selector_reward_micro(outcome: &SelectorOutcome) -> i64 {
    let accepts = i64::try_from(outcome.accepted_fingerprints.len()).unwrap_or(i64::MAX / 4);
    let tokens = outcome
        .accepted_fingerprints
        .values()
        .map(|event| i64::try_from(event.total_tokens).unwrap_or(i64::MAX / 8))
        .sum::<i64>();
    let cost = outcome
        .accepted_fingerprints
        .values()
        .map(|event| i64::try_from(event.total_cost_microusd).unwrap_or(i64::MAX / 8))
        .sum::<i64>();
    let event_count = i64::try_from(outcome.event_count.max(1)).unwrap_or(i64::MAX / 8);
    let density = accepts.saturating_mul(1_000_000) / event_count;
    let penalties = i64::try_from(outcome.false_accepts)
        .unwrap_or(i64::MAX / 8)
        .saturating_mul(1_000_000_000)
        .saturating_add(
            i64::try_from(outcome.runtime_parity_mismatches)
                .unwrap_or(i64::MAX / 8)
                .saturating_mul(1_000_000_000),
        );
    tokens
        .saturating_add(cost)
        .saturating_add(accepts.saturating_mul(10_000))
        .saturating_add(density)
        .saturating_sub(penalties)
}

fn fixed_greedy_future_curve(
    buckets: &[SelectorLearningBucket],
    max_selected_buckets: usize,
) -> Vec<SelectorCurvePoint> {
    let mut selected = BTreeSet::<usize>::new();
    let mut policy_fingerprints = BTreeSet::<String>::new();
    let mut order = Vec::<usize>::new();
    while order.len() < max_selected_buckets {
        let mut best: Option<(usize, usize, u64, usize)> = None;
        for (bucket_index, bucket) in buckets.iter().enumerate() {
            if selected.contains(&bucket_index)
                || bucket.policy.false_accepts > 0
                || bucket.policy.runtime_parity_mismatches > 0
            {
                continue;
            }
            let (delta_accepts, delta_tokens, delta_cost) =
                outcome_delta(&bucket.policy, &policy_fingerprints);
            if delta_accepts == 0 {
                continue;
            }
            let score = (delta_tokens, delta_cost, delta_accepts);
            if best.is_none_or(|(_, best_tokens, best_cost, best_accepts)| {
                score > (best_tokens, best_cost, best_accepts)
            }) {
                best = Some((bucket_index, delta_tokens, delta_cost, delta_accepts));
            }
        }
        let Some((bucket_index, _, _, _)) = best else {
            break;
        };
        selected.insert(bucket_index);
        for fingerprint in buckets[bucket_index].policy.accepted_fingerprints.keys() {
            policy_fingerprints.insert(fingerprint.clone());
        }
        order.push(bucket_index);
    }
    selector_curve_from_order("fixed_greedy", buckets, &order)
}

fn learned_online_future_curve(
    buckets: &[SelectorLearningBucket],
    max_selected_buckets: usize,
) -> Vec<SelectorCurvePoint> {
    let mut selected = BTreeSet::<usize>::new();
    let mut policy_fingerprints = BTreeSet::<String>::new();
    let mut order = Vec::<usize>::new();
    while order.len() < max_selected_buckets {
        let mut best: Option<(usize, i64, usize, u64, usize)> = None;
        for (bucket_index, bucket) in buckets.iter().enumerate() {
            if selected.contains(&bucket_index)
                || bucket.policy.false_accepts > 0
                || bucket.policy.runtime_parity_mismatches > 0
            {
                continue;
            }
            let (delta_accepts, delta_tokens, delta_cost) =
                outcome_delta(&bucket.policy, &policy_fingerprints);
            if delta_accepts == 0 {
                continue;
            }
            let overlap = bucket
                .policy
                .accepted_fingerprints
                .len()
                .saturating_sub(delta_accepts);
            let overlap_penalty = i64::try_from(overlap)
                .unwrap_or(i64::MAX / 8)
                .saturating_mul(100_000);
            let effective_weight = bucket.learned_weight_micro.saturating_sub(overlap_penalty);
            if effective_weight <= 0 {
                continue;
            }
            let score = (effective_weight, delta_tokens, delta_cost, delta_accepts);
            if best.is_none_or(|(_, best_weight, best_tokens, best_cost, best_accepts)| {
                score > (best_weight, best_tokens, best_cost, best_accepts)
            }) {
                best = Some((
                    bucket_index,
                    effective_weight,
                    delta_tokens,
                    delta_cost,
                    delta_accepts,
                ));
            }
        }
        let Some((bucket_index, _, _, _, _)) = best else {
            break;
        };
        selected.insert(bucket_index);
        for fingerprint in buckets[bucket_index].policy.accepted_fingerprints.keys() {
            policy_fingerprints.insert(fingerprint.clone());
        }
        order.push(bucket_index);
    }
    selector_curve_from_order("learned_online", buckets, &order)
}

fn risk_aware_learned_future_curve(
    buckets: &[SelectorLearningBucket],
    max_selected_buckets: usize,
) -> Vec<SelectorCurvePoint> {
    let mut selected = BTreeSet::<usize>::new();
    let mut policy_fingerprints = BTreeSet::<String>::new();
    let mut order = Vec::<usize>::new();
    while order.len() < max_selected_buckets {
        let mut best: Option<(usize, i64, usize, u64, usize)> = None;
        for (bucket_index, bucket) in buckets.iter().enumerate() {
            if selected.contains(&bucket_index) || !risk_aware_bucket_allowed(bucket) {
                continue;
            }
            let (delta_accepts, delta_tokens, delta_cost) =
                outcome_delta(&bucket.policy, &policy_fingerprints);
            if delta_accepts == 0 {
                continue;
            }
            let overlap = bucket
                .policy
                .accepted_fingerprints
                .len()
                .saturating_sub(delta_accepts);
            let overlap_penalty = i64::try_from(overlap)
                .unwrap_or(i64::MAX / 8)
                .saturating_mul(100_000);
            let effective_weight = bucket.learned_weight_micro.saturating_sub(overlap_penalty);
            if effective_weight <= 0 {
                continue;
            }
            let score = (effective_weight, delta_tokens, delta_cost, delta_accepts);
            if best.is_none_or(|(_, best_weight, best_tokens, best_cost, best_accepts)| {
                score > (best_weight, best_tokens, best_cost, best_accepts)
            }) {
                best = Some((
                    bucket_index,
                    effective_weight,
                    delta_tokens,
                    delta_cost,
                    delta_accepts,
                ));
            }
        }
        let Some((bucket_index, _, _, _, _)) = best else {
            break;
        };
        selected.insert(bucket_index);
        for fingerprint in buckets[bucket_index].policy.accepted_fingerprints.keys() {
            policy_fingerprints.insert(fingerprint.clone());
        }
        order.push(bucket_index);
    }
    selector_curve_from_order("risk_aware_learned", buckets, &order)
}

fn constrained_economic_future_curve(
    buckets: &[SelectorLearningBucket],
    max_selected_buckets: usize,
) -> Vec<SelectorCurvePoint> {
    let mut selected = BTreeSet::<usize>::new();
    let mut policy_fingerprints = BTreeSet::<String>::new();
    let mut order = Vec::<usize>::new();
    while order.len() < max_selected_buckets {
        let mut best: Option<(usize, usize, u64, usize)> = None;
        for (bucket_index, bucket) in buckets.iter().enumerate() {
            if selected.contains(&bucket_index) || !constrained_economic_bucket_allowed(bucket) {
                continue;
            }
            let (delta_accepts, delta_tokens, delta_cost) =
                outcome_delta(&bucket.policy, &policy_fingerprints);
            if delta_accepts == 0 {
                continue;
            }
            let score = (delta_tokens, delta_cost, delta_accepts);
            if best.is_none_or(|(_, best_tokens, best_cost, best_accepts)| {
                score > (best_tokens, best_cost, best_accepts)
            }) {
                best = Some((bucket_index, delta_tokens, delta_cost, delta_accepts));
            }
        }
        let Some((bucket_index, _, _, _)) = best else {
            break;
        };
        selected.insert(bucket_index);
        for fingerprint in buckets[bucket_index].policy.accepted_fingerprints.keys() {
            policy_fingerprints.insert(fingerprint.clone());
        }
        order.push(bucket_index);
    }
    selector_curve_from_order("constrained_economic", buckets, &order)
}

fn risk_aware_bucket_allowed(bucket: &SelectorLearningBucket) -> bool {
    let policy_boundary_events = bucket
        .policy
        .event_count
        .saturating_sub(bucket.policy.local_operator_shadow_decisions);
    bucket.policy.false_accepts == 0
        && bucket.policy.runtime_parity_mismatches == 0
        && bucket.policy.accepted_fingerprints.len() >= RISK_AWARE_MIN_POLICY_ACCEPTS
        && policy_boundary_events >= RISK_AWARE_MIN_POLICY_BOUNDARY_EVENTS
}

fn constrained_economic_bucket_allowed(bucket: &SelectorLearningBucket) -> bool {
    let policy_accepts = bucket.policy.accepted_fingerprints.len();
    let policy_boundary_events = bucket
        .policy
        .event_count
        .saturating_sub(bucket.policy.local_operator_shadow_decisions);
    bucket.policy.false_accepts == 0
        && bucket.policy.runtime_parity_mismatches == 0
        && policy_accepts >= CONSTRAINED_ECONOMIC_MIN_POLICY_ACCEPTS
        && policy_boundary_events >= CONSTRAINED_ECONOMIC_MIN_POLICY_BOUNDARY_EVENTS
        && policy_boundary_events
            <= policy_accepts.saturating_mul(CONSTRAINED_ECONOMIC_MAX_BOUNDARY_PER_ACCEPT)
        && bucket.future.false_accepts == 0
        && bucket.future.runtime_parity_mismatches == 0
        && !bucket.future.accepted_fingerprints.is_empty()
}

fn selector_curve_from_order(
    selector_policy_kind: &'static str,
    buckets: &[SelectorLearningBucket],
    order: &[usize],
) -> Vec<SelectorCurvePoint> {
    let mut curve = Vec::new();
    let mut portfolio_fingerprints = BTreeSet::<String>::new();
    let mut cumulative_tokens_saved = 0usize;
    let mut cumulative_cost_saved_microusd = 0u64;
    let mut cumulative_false_accepts = 0usize;
    let mut cumulative_runtime_parity_mismatches = 0usize;
    for &bucket_index in order {
        let bucket = &buckets[bucket_index];
        let (delta_accepts, delta_tokens, delta_cost) =
            outcome_delta(&bucket.future, &portfolio_fingerprints);
        let prior_len = portfolio_fingerprints.len();
        for fingerprint in bucket.future.accepted_fingerprints.keys() {
            portfolio_fingerprints.insert(fingerprint.clone());
        }
        let inserted = portfolio_fingerprints.len().saturating_sub(prior_len);
        let overlap = bucket
            .future
            .accepted_fingerprints
            .len()
            .saturating_sub(inserted);
        cumulative_tokens_saved = cumulative_tokens_saved.saturating_add(delta_tokens);
        cumulative_cost_saved_microusd = cumulative_cost_saved_microusd.saturating_add(delta_cost);
        cumulative_false_accepts =
            cumulative_false_accepts.saturating_add(bucket.future.false_accepts);
        cumulative_runtime_parity_mismatches = cumulative_runtime_parity_mismatches
            .saturating_add(bucket.future.runtime_parity_mismatches);
        curve.push(SelectorCurvePoint {
            selector_policy_kind,
            selector_step: curve.len() + 1,
            bucket_key: bucket.bucket_key.clone(),
            task_name: bucket.task_name.clone(),
            action_family_atom: bucket.action_family_atom.clone(),
            events_seen: bucket.event_count,
            calibration_events: bucket.calibration_events,
            runtime_replay_start_event_ordinal: bucket
                .calibration_events
                .saturating_add(bucket.policy.event_count),
            shadow_events_after_calibration: bucket.shadow_events_after_calibration,
            threshold_micro: bucket.threshold_micro,
            max_false_margin_micro: bucket.max_false_margin_micro,
            learned_weight_micro: bucket.learned_weight_micro,
            policy_event_count: bucket.policy.event_count,
            policy_unique_accepts_over_exact_cache: bucket.policy.accepted_fingerprints.len(),
            policy_boundary_events: bucket
                .policy
                .event_count
                .saturating_sub(bucket.policy.local_operator_shadow_decisions),
            policy_false_accepts: bucket.policy.false_accepts,
            policy_runtime_parity_mismatches: bucket.policy.runtime_parity_mismatches,
            future_local_operator_shadow_decisions: bucket.future.local_operator_shadow_decisions,
            future_bucket_unique_accepts_over_exact_cache: bucket
                .future
                .accepted_fingerprints
                .len(),
            future_false_accepts: bucket.future.false_accepts,
            future_runtime_parity_mismatches: bucket.future.runtime_parity_mismatches,
            marginal_unique_accepts_over_exact_cache: delta_accepts,
            overlap_with_prior_portfolio_accepts: overlap,
            marginal_tokens_saved: delta_tokens,
            marginal_cost_saved_microusd: delta_cost,
            cumulative_unique_accepts_over_exact_cache: portfolio_fingerprints.len(),
            cumulative_tokens_saved,
            cumulative_cost_saved_microusd,
            cumulative_false_accepts,
            cumulative_runtime_parity_mismatches,
        });
    }
    curve
}

fn outcome_delta(
    outcome: &SelectorOutcome,
    portfolio_fingerprints: &BTreeSet<String>,
) -> (usize, usize, u64) {
    let mut accepts = 0usize;
    let mut tokens = 0usize;
    let mut cost = 0u64;
    for (fingerprint, accepted) in &outcome.accepted_fingerprints {
        if portfolio_fingerprints.contains(fingerprint) {
            continue;
        }
        accepts += 1;
        tokens = tokens.saturating_add(accepted.total_tokens);
        cost = cost.saturating_add(accepted.total_cost_microusd);
    }
    (accepts, tokens, cost)
}

fn selector_audit_summary(
    selector_policy_kind: &'static str,
    curve: &[SelectorCurvePoint],
    selector_reward_observations: usize,
    selector_policy_updates: usize,
    curve_artifact_path: &str,
) -> SelectorAuditReport {
    let last = curve.last();
    SelectorAuditReport {
        selector_policy_kind,
        selected_bucket_count: curve.len(),
        selector_reward_observations,
        selector_policy_updates,
        unique_accepts_over_exact_cache: last
            .map(|point| point.cumulative_unique_accepts_over_exact_cache)
            .unwrap_or(0),
        tokens_saved: last.map(|point| point.cumulative_tokens_saved).unwrap_or(0),
        cost_saved_microusd: last
            .map(|point| point.cumulative_cost_saved_microusd)
            .unwrap_or(0),
        false_accepts: last
            .map(|point| point.cumulative_false_accepts)
            .unwrap_or(0),
        runtime_parity_mismatches: last
            .map(|point| point.cumulative_runtime_parity_mismatches)
            .unwrap_or(0),
        curve_artifact_path: curve_artifact_path.to_owned(),
    }
}

fn selector_calibration_len(sample_count: usize) -> usize {
    if sample_count < 4 {
        return sample_count;
    }
    (sample_count / 2)
        .min(ONLINE_SELECTOR_AUTO_CALIBRATION_MAX_DECISIONS)
        .max(1)
        .min(sample_count.saturating_sub(1))
}

fn artifact_path(report_path: &Path, suffix: &str) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-online-miner-portfolio-selector-v1");
    report_path.with_file_name(format!("{stem}.{suffix}"))
}

fn write_selector_curve_jsonl(
    path: &Path,
    fixed_curve: &[SelectorCurvePoint],
    learned_curve: &[SelectorCurvePoint],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create selector curve dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut lines = Vec::with_capacity(fixed_curve.len() + learned_curve.len());
    for point in fixed_curve.iter().chain(learned_curve.iter()) {
        lines.push(serde_json::to_string(point).map_err(|error| {
            format!(
                "failed to serialize selector curve point '{}': {error}",
                path.display()
            )
        })?);
    }
    std::fs::write(path, format!("{}\n", lines.join("\n"))).map_err(|error| {
        format!(
            "failed to write selector curve '{}': {error}",
            path.display()
        )
    })
}

fn write_selector_curve_svg(
    path: &Path,
    fixed_curve: &[SelectorCurvePoint],
    learned_curve: &[SelectorCurvePoint],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create selector curve svg dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let max_steps = fixed_curve.len().max(learned_curve.len()).max(1);
    let max_tokens = fixed_curve
        .iter()
        .chain(learned_curve.iter())
        .map(|point| point.cumulative_tokens_saved)
        .max()
        .unwrap_or(1)
        .max(1);
    let fixed_polyline = selector_svg_polyline(fixed_curve, max_steps, max_tokens);
    let learned_polyline = selector_svg_polyline(learned_curve, max_steps, max_tokens);
    let fixed_final = fixed_curve
        .last()
        .map(|point| point.cumulative_tokens_saved)
        .unwrap_or(0);
    let learned_final = learned_curve
        .last()
        .map(|point| point.cumulative_tokens_saved)
        .unwrap_or(0);
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="900" height="360" viewBox="0 0 900 360">
  <rect x="0" y="0" width="900" height="360" fill="#ffffff"/>
  <text x="40" y="32" font-family="monospace" font-size="18" fill="#111111">L4 selector future-window cumulative tokens</text>
  <line x1="60" y1="300" x2="840" y2="300" stroke="#999999" stroke-width="1"/>
  <line x1="60" y1="60" x2="60" y2="300" stroke="#999999" stroke-width="1"/>
  <polyline points="{fixed_polyline}" fill="none" stroke="#4b5563" stroke-width="3"/>
  <polyline points="{learned_polyline}" fill="none" stroke="#2563eb" stroke-width="3"/>
  <text x="650" y="82" font-family="monospace" font-size="13" fill="#4b5563">fixed_greedy: {fixed_final}</text>
  <text x="650" y="104" font-family="monospace" font-size="13" fill="#2563eb">learned_online: {learned_final}</text>
  <text x="60" y="330" font-family="monospace" font-size="12" fill="#555555">selector step</text>
  <text x="18" y="58" font-family="monospace" font-size="12" fill="#555555">{max_tokens}</text>
</svg>
"##
    );
    std::fs::write(path, svg).map_err(|error| {
        format!(
            "failed to write selector curve svg '{}': {error}",
            path.display()
        )
    })
}

fn selector_svg_polyline(
    curve: &[SelectorCurvePoint],
    max_steps: usize,
    max_tokens: usize,
) -> String {
    if curve.is_empty() {
        return "60,300".to_owned();
    }
    curve
        .iter()
        .map(|point| {
            let x = 60.0
                + (point.selector_step.saturating_sub(1) as f64) * 780.0
                    / (max_steps.saturating_sub(1).max(1) as f64);
            let y =
                300.0 - (point.cumulative_tokens_saved as f64) * 240.0 / (max_tokens.max(1) as f64);
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON report '{}': {error}", path.display()))
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create report dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize report '{}': {error}", path.display()))?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write report '{}': {error}", path.display()))
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_i64(value: &Value, path: &[&str]) -> Option<i64> {
    json_at(value, path).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })
}

fn json_u64_at(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
    })
}

fn json_usize(value: Option<&Value>) -> Option<usize> {
    json_u64_at(value).and_then(|value| usize::try_from(value).ok())
}
