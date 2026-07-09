use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

use nando_core::{
    PhaseCenterEvalTask, PhaseCenterHotRouteTable, PhaseCenterHotRuntime, PhaseCenterHotScratch,
    PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime, PhaseCenterPreparedHotRequest,
};
use serde_json::Value;

use super::{
    margin_to_micro, online_miner_daemon::online_miner_event_bucket_specs,
    parse_phase_atom_binary_event_for_action, phase_atom_action_families,
    phase_atom_binary_event_vector_for_task, phase_atom_state_action_bucket_key,
    phase_atom_string_vec, stable_fingerprint,
};

const DEFAULT_ONLINE_MINER_PORTFOLIO_RUNTIME_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-runtime-replay-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_SELECTOR_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-selector-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-future-tail-replay-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_TRACE: &str =
    "target/nando-wave/streaming/agent-continue-command-result-followup-pack-v25.jsonl";
const DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-future-tail-billing-request-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-future-tail-billing-request-v1.jsonl";
const DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-live-tail-score-only-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_DECISIONS: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-live-tail-score-only-v1.decisions.jsonl";
const DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_TRACE: &str =
    "target/nando-wave/streaming/live-agent-phase-atom-append-v1.jsonl";
const DEFAULT_ONLINE_MINER_PRODUCT_HOT_REGISTRY: &str = "target/nando-wave/streaming/online-miner-daemon-v1-command-followup-v25-first5000-output-shape-basis-cap8-c1000-release-hotparity/product-hot-promotion-registry.shadow.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-live-tail-billing-request-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-live-tail-billing-request-v1.jsonl";

#[derive(Clone, Debug)]
struct SelectedBucket {
    bucket_key: String,
    action_family_atom: String,
    task_name: String,
    threshold_micro: i64,
    calibration_events: usize,
    runtime_replay_start_event_ordinal: usize,
}

#[derive(Clone, Debug)]
struct SelectedDecision {
    bucket_key: String,
    request_fingerprint: String,
    external_provider_correlation_keys: Vec<String>,
    exact_cache_hit: bool,
    verified_safe_accept: bool,
    denominator_row_index: usize,
    margin_micro: i64,
    package_fingerprint64: u64,
}

#[derive(Clone, Debug)]
struct ReplayEvent {
    event: super::PhaseAtomBinaryEvent,
}

#[derive(Clone, Debug, Default)]
struct ReplayTraceDenominator {
    total_rows: usize,
    token_rows: usize,
    total_tokens: usize,
    estimated_total_cost_microusd: u64,
    exact_cache_hits: usize,
    exact_cache_tokens: usize,
    exact_cache_estimated_cost_microusd: u64,
}

#[derive(Clone, Debug, Default)]
struct ReplayTraceSet {
    events: BTreeMap<(usize, String, String), ReplayEvent>,
    denominator: ReplayTraceDenominator,
}

struct ReplayRuntimeEntry {
    runtime: PhaseCenterOffloadRuntime,
    hot_runtime: PhaseCenterHotRuntime,
    hot_routes: PhaseCenterHotRouteTable,
    hot_scratch: PhaseCenterHotScratch,
}

#[derive(Clone, Debug)]
struct ProductHotRegistryEntry {
    selected_bucket: SelectedBucket,
    package_path: PathBuf,
    package_fingerprint64: u64,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_runtime_replay_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_RUNTIME_REPLAY_REPORT));
    let selector_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_SELECTOR_REPORT));
    let selector = read_json_value(&selector_report_path)?;
    let online_miner_report_path = PathBuf::from(
        json_string(&selector, &["online_miner_report_path"]).ok_or_else(|| {
            format!(
                "selector report '{}' missing online_miner_report_path",
                selector_report_path.display()
            )
        })?,
    );
    let online_miner = read_json_value(&online_miner_report_path)?;
    let decision_log_path = PathBuf::from(
        json_string(&selector, &["decision_log_path"])
            .or_else(|| json_string(&online_miner, &["decision_log_path"]))
            .ok_or_else(|| {
                format!(
                    "selector/online reports missing decision_log_path: '{}' / '{}'",
                    selector_report_path.display(),
                    online_miner_report_path.display()
                )
            })?,
    );

    let selected_buckets = selected_buckets_from_report(&selector)?;
    if selected_buckets.is_empty() {
        return Err("selector report has no selected_buckets".to_owned());
    }
    let selected_bucket_keys = selected_buckets
        .keys()
        .cloned()
        .collect::<BTreeSet<String>>();
    let package_paths = package_paths_by_fingerprint(&online_miner)?;
    let trace_paths = online_miner
        .get("trace_paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if trace_paths.is_empty() {
        return Err("online miner report has no trace_paths".to_owned());
    }

    let selected_decisions =
        selected_decisions_from_log(&decision_log_path, &selected_bucket_keys)?;
    let replay_trace_set =
        replay_events_from_traces(&trace_paths, &selected_buckets, &selected_decisions)?;
    let replay_events = &replay_trace_set.events;
    let mut runtime_cache = BTreeMap::<u64, ReplayRuntimeEntry>::new();

    let mut replay_rows = 0usize;
    let mut missing_event_rows = 0usize;
    let mut missing_package_rows = 0usize;
    let mut hot_margin_parity_mismatches = 0usize;
    let mut hot_decision_parity_mismatches = 0usize;
    let mut decision_log_margin_mismatches = 0usize;
    let mut false_accepts = 0usize;
    let mut wrong_wins = 0usize;
    let mut replay_verified_safe_accept_rows = 0usize;
    let mut replay_unverified_rows = 0usize;
    let mut post_calibration_score_candidate_rows = 0usize;
    let mut verifier_bound_score_candidate_rows = 0usize;
    let mut verifier_bound_exact_cache_score_candidate_rows = 0usize;
    let mut verifier_rejected_score_candidate_rows = 0usize;
    let mut external_provider_correlation_key_rows = 0usize;
    let mut external_provider_correlation_missing_rows = 0usize;
    let mut external_provider_correlation_key_count = 0usize;
    let mut selected_bucket_ordinals = BTreeMap::<String, usize>::new();
    let mut accepted_fingerprints = BTreeMap::<String, usize>::new();
    let mut portfolio_tokens_saved = 0usize;
    let mut portfolio_cost_saved_microusd = 0u64;
    let mut bucket_reports = BTreeMap::<String, serde_json::Value>::new();

    for decision in &selected_decisions {
        let Some(selected_bucket) = selected_buckets.get(&decision.bucket_key) else {
            continue;
        };
        let ordinal = selected_bucket_ordinals
            .entry(decision.bucket_key.clone())
            .or_insert(0usize);
        let in_selected_future_window =
            *ordinal >= selected_bucket.runtime_replay_start_event_ordinal;
        *ordinal += 1;

        let Some(replay_event) = replay_events.get(&(
            decision.denominator_row_index,
            decision.bucket_key.clone(),
            decision.request_fingerprint.clone(),
        )) else {
            missing_event_rows += 1;
            continue;
        };
        let Some(package_path) = package_paths.get(&decision.package_fingerprint64) else {
            missing_package_rows += 1;
            continue;
        };
        if !runtime_cache.contains_key(&decision.package_fingerprint64) {
            let entry = load_runtime_entry(
                package_path,
                selected_bucket.threshold_micro,
                decision.package_fingerprint64,
            )?;
            runtime_cache.insert(decision.package_fingerprint64, entry);
        }
        let runtime_entry = runtime_cache
            .get_mut(&decision.package_fingerprint64)
            .expect("runtime entry inserted before replay");
        let safe_accept_vec = phase_atom_binary_event_vector_for_task(
            &replay_event.event,
            true,
            runtime_entry.runtime.cells(),
            &selected_bucket.task_name,
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
                .map_err(|error| format!("portfolio replay flat margin error: {error:?}"))?,
        )?;
        let candidates = runtime_entry
            .hot_runtime
            .score_prepared_hot_request_candidates(
                &runtime_entry.hot_routes,
                PhaseCenterPreparedHotRequest::new(0, &safe_accept_vec),
                &mut runtime_entry.hot_scratch,
            )
            .map_err(|error| format!("portfolio replay hot runtime error: {error:?}"))?;
        let Some(hot_decision) = candidates.first() else {
            return Err("portfolio replay hot runtime returned no candidates".to_owned());
        };
        replay_rows += 1;
        if hot_decision.margin_micro != flat_margin_micro {
            hot_margin_parity_mismatches += 1;
        }
        let flat_local_operator = flat_margin_micro >= selected_bucket.threshold_micro;
        if hot_decision.score_candidate != flat_local_operator {
            hot_decision_parity_mismatches += 1;
        }
        if decision.margin_micro != flat_margin_micro {
            decision_log_margin_mismatches += 1;
        }
        if decision.verified_safe_accept {
            replay_verified_safe_accept_rows += 1;
        } else {
            replay_unverified_rows += 1;
        }
        if in_selected_future_window && hot_decision.score_candidate {
            post_calibration_score_candidate_rows += 1;
            if decision.verified_safe_accept {
                verifier_bound_score_candidate_rows += 1;
                let unique_accept_over_exact_cache = !decision.exact_cache_hit
                    && accepted_fingerprints
                        .insert(decision.request_fingerprint.clone(), 1)
                        .is_none();
                if unique_accept_over_exact_cache {
                    portfolio_tokens_saved = portfolio_tokens_saved
                        .saturating_add(replay_event.event.token_cost.total_tokens);
                    portfolio_cost_saved_microusd = portfolio_cost_saved_microusd
                        .saturating_add(replay_event.event.token_cost.total_cost_microusd);
                    if decision.external_provider_correlation_keys.is_empty() {
                        external_provider_correlation_missing_rows =
                            external_provider_correlation_missing_rows.saturating_add(1);
                    } else {
                        external_provider_correlation_key_rows =
                            external_provider_correlation_key_rows.saturating_add(1);
                        external_provider_correlation_key_count =
                            external_provider_correlation_key_count
                                .saturating_add(decision.external_provider_correlation_keys.len());
                    }
                }
                if decision.exact_cache_hit {
                    verifier_bound_exact_cache_score_candidate_rows += 1;
                }
            } else {
                false_accepts += 1;
                verifier_rejected_score_candidate_rows += 1;
            }
        }
        if (decision.verified_safe_accept && hot_decision.margin_micro <= 0)
            || (!decision.verified_safe_accept && hot_decision.margin_micro >= 0)
        {
            wrong_wins += 1;
        }

        let entry = bucket_reports
            .entry(decision.bucket_key.clone())
            .or_insert_with(|| {
                serde_json::json!({
                    "bucket_key": selected_bucket.bucket_key,
                    "task_name": selected_bucket.task_name,
                    "threshold_micro": selected_bucket.threshold_micro,
                    "calibration_events": selected_bucket.calibration_events,
                    "runtime_replay_start_event_ordinal": selected_bucket.runtime_replay_start_event_ordinal,
                    "replay_rows": 0usize,
                    "false_accepts": 0usize,
                    "post_calibration_score_candidate_rows": 0usize,
                    "verifier_bound_score_candidate_rows": 0usize,
                    "verifier_bound_exact_cache_score_candidate_rows": 0usize,
                    "verifier_rejected_score_candidate_rows": 0usize,
                    "hot_margin_parity_mismatches": 0usize,
                })
            });
        increment_json_usize(entry, "replay_rows", 1)?;
        increment_json_usize(
            entry,
            "false_accepts",
            usize::from(
                in_selected_future_window
                    && hot_decision.score_candidate
                    && !decision.verified_safe_accept,
            ),
        )?;
        increment_json_usize(
            entry,
            "post_calibration_score_candidate_rows",
            usize::from(in_selected_future_window && hot_decision.score_candidate),
        )?;
        increment_json_usize(
            entry,
            "verifier_bound_score_candidate_rows",
            usize::from(
                in_selected_future_window
                    && hot_decision.score_candidate
                    && decision.verified_safe_accept,
            ),
        )?;
        increment_json_usize(
            entry,
            "verifier_bound_exact_cache_score_candidate_rows",
            usize::from(
                in_selected_future_window
                    && hot_decision.score_candidate
                    && decision.verified_safe_accept
                    && decision.exact_cache_hit,
            ),
        )?;
        increment_json_usize(
            entry,
            "verifier_rejected_score_candidate_rows",
            usize::from(
                in_selected_future_window
                    && hot_decision.score_candidate
                    && !decision.verified_safe_accept,
            ),
        )?;
        increment_json_usize(
            entry,
            "hot_margin_parity_mismatches",
            usize::from(hot_decision.margin_micro != flat_margin_micro),
        )?;
    }

    let selector_portfolio_accepts = json_usize(json_at(
        &selector,
        &["portfolio_unique_cpu_accepts_over_exact_cache"],
    ))
    .unwrap_or(0);
    let selector_portfolio_tokens =
        json_usize(json_at(&selector, &["portfolio_tokens_saved"])).unwrap_or(0);
    let portfolio_accepts = accepted_fingerprints.len();
    let verifier_bound_unique_accepts_over_exact_cache = portfolio_accepts;
    let provider_correlation_complete_for_shadow_accepts = portfolio_accepts > 0
        && external_provider_correlation_key_rows >= portfolio_accepts
        && external_provider_correlation_missing_rows == 0;
    let all_score_candidates_verifier_bound =
        post_calibration_score_candidate_rows > 0 && verifier_rejected_score_candidate_rows == 0;
    let verifier_binding_bound = all_score_candidates_verifier_bound
        && false_accepts == 0
        && post_calibration_score_candidate_rows > 0;
    let trace_denominator = &replay_trace_set.denominator;
    let calls_saved_over_exact_cache_milli =
        ratio_milli(portfolio_accepts, trace_denominator.total_rows);
    let tokens_saved_over_exact_cache_milli =
        ratio_milli(portfolio_tokens_saved, trace_denominator.total_tokens);
    let estimated_cost_saved_over_exact_cache_milli = ratio_milli_u64(
        portfolio_cost_saved_microusd,
        trace_denominator.estimated_total_cost_microusd,
    );
    let exact_cache_calls_saved_milli = ratio_milli(
        trace_denominator.exact_cache_hits,
        trace_denominator.total_rows,
    );
    let exact_cache_tokens_saved_milli = ratio_milli(
        trace_denominator.exact_cache_tokens,
        trace_denominator.total_tokens,
    );
    let exact_cache_estimated_cost_saved_milli = ratio_milli_u64(
        trace_denominator.exact_cache_estimated_cost_microusd,
        trace_denominator.estimated_total_cost_microusd,
    );
    let combined_calls_saved_milli = ratio_milli(
        trace_denominator
            .exact_cache_hits
            .saturating_add(portfolio_accepts),
        trace_denominator.total_rows,
    );
    let combined_tokens_saved_milli = ratio_milli(
        trace_denominator
            .exact_cache_tokens
            .saturating_add(portfolio_tokens_saved),
        trace_denominator.total_tokens,
    );
    let combined_estimated_cost_saved_milli = ratio_milli_u64(
        trace_denominator
            .exact_cache_estimated_cost_microusd
            .saturating_add(portfolio_cost_saved_microusd),
        trace_denominator.estimated_total_cost_microusd,
    );
    let selector_accept_parity = portfolio_accepts == selector_portfolio_accepts;
    let selector_token_parity = portfolio_tokens_saved == selector_portfolio_tokens;
    let runtime_replay_passed = replay_rows > 0
        && missing_event_rows == 0
        && missing_package_rows == 0
        && hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && false_accepts == 0
        && selector_accept_parity
        && selector_token_parity;

    let manual_class_list_used =
        json_bool(&selector, &["discovery_mode", "manual_class_list_used"]).unwrap_or(false);
    let static_topn_seed_used =
        json_bool(&selector, &["discovery_mode", "static_topn_seed_used"]).unwrap_or(false);
    let online_discovery_used =
        json_bool(&selector, &["discovery_mode", "online_discovery_used"]).unwrap_or(false);
    let marginal_denominator_delta_used = json_bool(
        &selector,
        &["discovery_mode", "marginal_denominator_delta_used"],
    )
    .unwrap_or(false);
    let portfolio_gate_passed =
        json_bool(&selector, &["discovery_mode", "portfolio_gate_passed"]).unwrap_or(false);
    let selector_dynamic_discovery_shadow_claim_allowed = json_bool(
        &selector,
        &["discovery_mode", "dynamic_discovery_shadow_claim_allowed"],
    )
    .unwrap_or(false);
    let dynamic_discovery_shadow_claim_allowed = runtime_replay_passed
        && !manual_class_list_used
        && !static_topn_seed_used
        && online_discovery_used
        && marginal_denominator_delta_used
        && portfolio_gate_passed
        && selector_dynamic_discovery_shadow_claim_allowed;
    let product_dynamic_discovery_claim_allowed = false;
    let verdict = if runtime_replay_passed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_RUNTIME_REPLAY_V1_PASS_REVIEW_ONLY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_RUNTIME_REPLAY_V1_WATCH"
    };

    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_runtime_replay_v1"),
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
    report.insert("trace_paths".to_owned(), serde_json::json!(trace_paths));
    report.insert(
        "selected_bucket_count".to_owned(),
        serde_json::json!(selected_buckets.len()),
    );
    report.insert(
        "selected_decision_rows".to_owned(),
        serde_json::json!(selected_decisions.len()),
    );
    report.insert("replay_rows".to_owned(), serde_json::json!(replay_rows));
    report.insert(
        "runtime_package_count".to_owned(),
        serde_json::json!(runtime_cache.len()),
    );
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
        "replay_verified_safe_accept_rows".to_owned(),
        serde_json::json!(replay_verified_safe_accept_rows),
    );
    report.insert(
        "replay_unverified_rows".to_owned(),
        serde_json::json!(replay_unverified_rows),
    );
    report.insert(
        "post_calibration_score_candidate_rows".to_owned(),
        serde_json::json!(post_calibration_score_candidate_rows),
    );
    report.insert(
        "verifier_bound_score_candidate_rows".to_owned(),
        serde_json::json!(verifier_bound_score_candidate_rows),
    );
    report.insert(
        "verifier_bound_exact_cache_score_candidate_rows".to_owned(),
        serde_json::json!(verifier_bound_exact_cache_score_candidate_rows),
    );
    report.insert(
        "verifier_rejected_score_candidate_rows".to_owned(),
        serde_json::json!(verifier_rejected_score_candidate_rows),
    );
    report.insert(
        "verifier_bound_unique_accepts_over_exact_cache".to_owned(),
        serde_json::json!(verifier_bound_unique_accepts_over_exact_cache),
    );
    report.insert(
        "external_provider_correlation_key_rows".to_owned(),
        serde_json::json!(external_provider_correlation_key_rows),
    );
    report.insert(
        "external_provider_correlation_missing_rows".to_owned(),
        serde_json::json!(external_provider_correlation_missing_rows),
    );
    report.insert(
        "external_provider_correlation_key_count".to_owned(),
        serde_json::json!(external_provider_correlation_key_count),
    );
    report.insert(
        "provider_correlation_complete_for_shadow_accepts".to_owned(),
        serde_json::json!(provider_correlation_complete_for_shadow_accepts),
    );
    report.insert(
        "provider_correlation_policy".to_owned(),
        serde_json::json!("counts only unique verifier-bound CPU accepts over exact cache; provider keys prove correlation route only, not provider billing money"),
    );
    report.insert(
        "all_score_candidates_verifier_bound".to_owned(),
        serde_json::json!(all_score_candidates_verifier_bound),
    );
    report.insert(
        "verifier_binding_bound".to_owned(),
        serde_json::json!(verifier_binding_bound),
    );
    report.insert(
        "verifier_binding".to_owned(),
        online_portfolio_verifier_binding_report(),
    );
    report.insert(
        "verifier_binding_policy".to_owned(),
        serde_json::json!("selected .nwpc candidates are treated as verifier-bound only when prepared-hot score_candidate is true after calibration, every accepted candidate row has verified_safe_accept=true, false_accepts=0, and exact-cache hits are excluded from unique CPU accepts over exact cache"),
    );
    report.insert("false_accepts".to_owned(), serde_json::json!(false_accepts));
    report.insert("wrong_wins".to_owned(), serde_json::json!(wrong_wins));
    report.insert(
        "portfolio_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(portfolio_accepts),
    );
    report.insert(
        "selector_portfolio_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(selector_portfolio_accepts),
    );
    report.insert(
        "selector_accept_parity".to_owned(),
        serde_json::json!(selector_accept_parity),
    );
    report.insert(
        "portfolio_tokens_saved".to_owned(),
        serde_json::json!(portfolio_tokens_saved),
    );
    report.insert(
        "selector_portfolio_tokens_saved".to_owned(),
        serde_json::json!(selector_portfolio_tokens),
    );
    report.insert(
        "selector_token_parity".to_owned(),
        serde_json::json!(selector_token_parity),
    );
    report.insert(
        "portfolio_cost_saved_microusd".to_owned(),
        serde_json::json!(portfolio_cost_saved_microusd),
    );
    report.insert(
        "portfolio_estimated_cost_saved_microusd".to_owned(),
        serde_json::json!(portfolio_cost_saved_microusd),
    );
    report.insert(
        "trace_denominator".to_owned(),
        serde_json::json!({
            "trace_rows": trace_denominator.total_rows,
            "token_rows": trace_denominator.token_rows,
            "total_tokens": trace_denominator.total_tokens,
            "estimated_total_cost_microusd": trace_denominator.estimated_total_cost_microusd,
            "exact_cache_hits": trace_denominator.exact_cache_hits,
            "exact_cache_tokens": trace_denominator.exact_cache_tokens,
            "exact_cache_estimated_cost_microusd": trace_denominator.exact_cache_estimated_cost_microusd,
            "cost_source": "trace token_cost estimate; market money claim still requires provider billing evidence"
        }),
    );
    report.insert(
        "savings_over_exact_cache".to_owned(),
        serde_json::json!({
            "calls_saved": portfolio_accepts,
            "calls_saved_milli": calls_saved_over_exact_cache_milli,
            "tokens_saved": portfolio_tokens_saved,
            "tokens_saved_milli": tokens_saved_over_exact_cache_milli,
            "estimated_cost_saved_microusd": portfolio_cost_saved_microusd,
            "estimated_cost_saved_milli": estimated_cost_saved_over_exact_cache_milli,
            "market_money_claim_allowed": false
        }),
    );
    report.insert(
        "exact_cache_baseline".to_owned(),
        serde_json::json!({
            "calls_saved": trace_denominator.exact_cache_hits,
            "calls_saved_milli": exact_cache_calls_saved_milli,
            "tokens_saved": trace_denominator.exact_cache_tokens,
            "tokens_saved_milli": exact_cache_tokens_saved_milli,
            "estimated_cost_saved_microusd": trace_denominator.exact_cache_estimated_cost_microusd,
            "estimated_cost_saved_milli": exact_cache_estimated_cost_saved_milli
        }),
    );
    report.insert(
        "combined_exact_cache_plus_nando_shadow".to_owned(),
        serde_json::json!({
            "calls_saved": trace_denominator.exact_cache_hits.saturating_add(portfolio_accepts),
            "calls_saved_milli": combined_calls_saved_milli,
            "tokens_saved": trace_denominator.exact_cache_tokens.saturating_add(portfolio_tokens_saved),
            "tokens_saved_milli": combined_tokens_saved_milli,
            "estimated_cost_saved_microusd": trace_denominator.exact_cache_estimated_cost_microusd.saturating_add(portfolio_cost_saved_microusd),
            "estimated_cost_saved_milli": combined_estimated_cost_saved_milli,
            "market_money_claim_allowed": false
        }),
    );
    report.insert(
        "bucket_reports".to_owned(),
        serde_json::json!(bucket_reports.values().cloned().collect::<Vec<_>>()),
    );
    report.insert(
        "discovery_mode".to_owned(),
        serde_json::json!({
            "manual_class_list_used": manual_class_list_used,
            "static_topn_seed_used": static_topn_seed_used,
            "online_discovery_used": online_discovery_used,
            "marginal_denominator_delta_used": marginal_denominator_delta_used,
            "portfolio_gate_passed": portfolio_gate_passed,
            "runtime_replay_passed": runtime_replay_passed,
            "selector_dynamic_discovery_shadow_claim_allowed": selector_dynamic_discovery_shadow_claim_allowed,
            "dynamic_discovery_shadow_claim_allowed": dynamic_discovery_shadow_claim_allowed,
            "product_dynamic_discovery_claim_allowed": product_dynamic_discovery_claim_allowed,
            "claim_boundary": "prepared-hot runtime replay of selected online-miner portfolio only; product claim still needs admission/promotion policy and money evidence"
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
        serde_json::json!("runtime replay/reporting only: reloads selected .nwpc packages, rebuilds phase vectors from source traces, checks prepared-hot vs flat margins, and does not promote, serve, enable local_accept, claim market money, or revive legacy nwrb"),
    );
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_runtime_replay_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  selected_bucket_count: {}", selected_buckets.len());
    println!("  selected_decision_rows: {}", selected_decisions.len());
    println!("  replay_rows: {replay_rows}");
    println!("  hot_margin_parity_mismatches: {hot_margin_parity_mismatches}");
    println!("  false_accepts: {false_accepts}");
    println!("  portfolio_unique_cpu_accepts_over_exact_cache: {portfolio_accepts}");
    println!("  selector_accept_parity: {selector_accept_parity}");
    println!("  runtime_replay_passed: {runtime_replay_passed}");
    println!("  product_dynamic_discovery_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_portfolio_future_tail_replay_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_REPLAY_REPORT));
    let selector_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_SELECTOR_REPORT));
    let future_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_TRACE));
    let min_future_row_index = args
        .next()
        .as_deref()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|error| format!("invalid min_future_row_index: {error}"))?
        .unwrap_or(5000);

    let selector = read_json_value(&selector_report_path)?;
    let online_miner_report_path = PathBuf::from(
        json_string(&selector, &["online_miner_report_path"]).ok_or_else(|| {
            format!(
                "selector report '{}' missing online_miner_report_path",
                selector_report_path.display()
            )
        })?,
    );
    let online_miner = read_json_value(&online_miner_report_path)?;
    let decision_log_path = PathBuf::from(
        json_string(&selector, &["decision_log_path"])
            .or_else(|| json_string(&online_miner, &["decision_log_path"]))
            .ok_or_else(|| {
                format!(
                    "selector/online reports missing decision_log_path: '{}' / '{}'",
                    selector_report_path.display(),
                    online_miner_report_path.display()
                )
            })?,
    );

    let selected_buckets = selected_buckets_from_report(&selector)?;
    let selected_bucket_keys = selected_buckets
        .keys()
        .cloned()
        .collect::<BTreeSet<String>>();
    let selected_decisions =
        selected_decisions_from_log(&decision_log_path, &selected_bucket_keys)?;
    let package_paths = package_paths_by_fingerprint(&online_miner)?;
    let mut latest_package_by_bucket = BTreeMap::<String, SelectedDecision>::new();
    for decision in selected_decisions {
        if decision.package_fingerprint64 == 0 {
            continue;
        }
        latest_package_by_bucket
            .entry(decision.bucket_key.clone())
            .and_modify(|current| {
                if decision.denominator_row_index >= current.denominator_row_index {
                    *current = decision.clone();
                }
            })
            .or_insert(decision);
    }

    let future_text = std::fs::read_to_string(&future_trace_path).map_err(|error| {
        format!(
            "failed to read future tail trace '{}': {error}",
            future_trace_path.display()
        )
    })?;
    let mut runtime_cache = BTreeMap::<u64, ReplayRuntimeEntry>::new();
    let mut future_rows = 0usize;
    let mut future_token_rows = 0usize;
    let mut future_total_tokens = 0usize;
    let mut future_total_cost_microusd = 0u64;
    let mut future_exact_cache_hits = 0usize;
    let mut future_exact_cache_tokens = 0usize;
    let mut future_exact_cache_cost_microusd = 0u64;
    let mut future_matching_bucket_events = 0usize;
    let mut future_score_candidate_rows = 0usize;
    let mut verifier_bound_score_candidate_rows = 0usize;
    let mut verifier_rejected_score_candidate_rows = 0usize;
    let mut future_false_accepts = 0usize;
    let mut hot_margin_parity_mismatches = 0usize;
    let mut hot_decision_parity_mismatches = 0usize;
    let mut missing_package_rows = 0usize;
    let mut external_provider_correlation_key_rows = 0usize;
    let mut external_provider_correlation_missing_rows = 0usize;
    let mut external_provider_correlation_key_count = 0usize;
    let mut accepted_fingerprints = BTreeMap::<String, usize>::new();
    let mut accepted_by_bucket = BTreeMap::<String, BTreeMap<String, (usize, u64)>>::new();
    let mut false_accepts_by_bucket = BTreeMap::<String, usize>::new();
    let mut future_tokens_saved = 0usize;
    let mut future_cost_saved_microusd = 0u64;
    let mut bucket_reports = BTreeMap::<String, serde_json::Value>::new();
    let mut parsed_events = 0usize;

    for (line_index, line) in future_text.lines().enumerate() {
        let row_number = line_index + 1;
        if row_number <= min_future_row_index {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        future_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse future tail trace '{}' line {}: {error}",
                future_trace_path.display(),
                row_number
            )
        })?;
        let total_tokens = json_usize(json_at(&row, &["token_cost", "total_tokens"])).unwrap_or(0);
        let total_cost_microusd =
            json_u64(&row, &["token_cost", "total_cost_microusd"]).unwrap_or(0);
        if total_tokens > 0 {
            future_token_rows += 1;
            future_total_tokens = future_total_tokens.saturating_add(total_tokens);
        }
        future_total_cost_microusd = future_total_cost_microusd.saturating_add(total_cost_microusd);
        let exact_cache_hit = json_bool(&row, &["exact_cache_hit"]).unwrap_or(false);
        if exact_cache_hit {
            future_exact_cache_hits += 1;
            future_exact_cache_tokens = future_exact_cache_tokens.saturating_add(total_tokens);
            future_exact_cache_cost_microusd =
                future_exact_cache_cost_microusd.saturating_add(total_cost_microusd);
        }
        if row
            .get("verified_safe_accept")
            .and_then(Value::as_bool)
            .is_none()
        {
            continue;
        }
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
            let bucket_specs = online_miner_event_bucket_specs(
                &action_family,
                &request_atoms,
                &state_atoms,
                &tool_atoms,
                &route_hint_atoms,
                &[],
            );
            let bucket_keys = online_portfolio_matching_selected_bucket_keys(
                &action_family,
                &request_atoms,
                &state_atoms,
                &tool_atoms,
                &route_hint_atoms,
                &bucket_specs,
                &selected_buckets,
            );
            for bucket_key in bucket_keys {
                let Some(selected_bucket) = selected_buckets.get(&bucket_key) else {
                    continue;
                };
                future_matching_bucket_events += 1;
                let Some(package_decision) = latest_package_by_bucket.get(&bucket_key) else {
                    missing_package_rows += 1;
                    continue;
                };
                let Some(package_path) = package_paths.get(&package_decision.package_fingerprint64)
                else {
                    missing_package_rows += 1;
                    continue;
                };
                if !runtime_cache.contains_key(&package_decision.package_fingerprint64) {
                    let entry = load_runtime_entry(
                        package_path,
                        selected_bucket.threshold_micro,
                        package_decision.package_fingerprint64,
                    )?;
                    runtime_cache.insert(package_decision.package_fingerprint64, entry);
                }
                let Some(event) = parse_phase_atom_binary_event_for_action(
                    &row,
                    parsed_events,
                    &action_family,
                    &selected_bucket.task_name,
                ) else {
                    continue;
                };
                parsed_events += 1;
                let runtime_entry = runtime_cache
                    .get_mut(&package_decision.package_fingerprint64)
                    .expect("future tail runtime entry inserted before replay");
                let safe_accept_vec = phase_atom_binary_event_vector_for_task(
                    &event,
                    true,
                    runtime_entry.runtime.cells(),
                    &selected_bucket.task_name,
                );
                let zero =
                    vec![nando_core::PhaseCenterCell::default(); runtime_entry.runtime.cells()];
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
                        .map_err(|error| format!("future tail flat margin error: {error:?}"))?,
                )?;
                let candidates = runtime_entry
                    .hot_runtime
                    .score_prepared_hot_request_candidates(
                        &runtime_entry.hot_routes,
                        PhaseCenterPreparedHotRequest::new(0, &safe_accept_vec),
                        &mut runtime_entry.hot_scratch,
                    )
                    .map_err(|error| format!("future tail hot runtime error: {error:?}"))?;
                let Some(hot_decision) = candidates.first() else {
                    return Err("future tail hot runtime returned no candidates".to_owned());
                };
                if hot_decision.margin_micro != flat_margin_micro {
                    hot_margin_parity_mismatches += 1;
                }
                let flat_score_candidate = flat_margin_micro >= selected_bucket.threshold_micro;
                if hot_decision.score_candidate != flat_score_candidate {
                    hot_decision_parity_mismatches += 1;
                }
                if hot_decision.score_candidate {
                    future_score_candidate_rows += 1;
                    if event.verified_safe_accept {
                        verifier_bound_score_candidate_rows += 1;
                        if !exact_cache_hit
                            && accepted_fingerprints
                                .insert(event.request_fingerprint.clone(), 1)
                                .is_none()
                        {
                            future_tokens_saved =
                                future_tokens_saved.saturating_add(event.token_cost.total_tokens);
                            future_cost_saved_microusd = future_cost_saved_microusd
                                .saturating_add(event.token_cost.total_cost_microusd);
                            if event.external_provider_correlation_keys.is_empty() {
                                external_provider_correlation_missing_rows =
                                    external_provider_correlation_missing_rows.saturating_add(1);
                            } else {
                                external_provider_correlation_key_rows =
                                    external_provider_correlation_key_rows.saturating_add(1);
                                external_provider_correlation_key_count =
                                    external_provider_correlation_key_count.saturating_add(
                                        event.external_provider_correlation_keys.len(),
                                    );
                            }
                        }
                        if !exact_cache_hit {
                            accepted_by_bucket
                                .entry(bucket_key.clone())
                                .or_default()
                                .entry(event.request_fingerprint.clone())
                                .or_insert((
                                    event.token_cost.total_tokens,
                                    event.token_cost.total_cost_microusd,
                                ));
                        }
                    } else {
                        future_false_accepts += 1;
                        *false_accepts_by_bucket
                            .entry(bucket_key.clone())
                            .or_default() += 1;
                        verifier_rejected_score_candidate_rows += 1;
                    }
                }
                let entry = bucket_reports.entry(bucket_key.clone()).or_insert_with(|| {
                    serde_json::json!({
                        "bucket_key": selected_bucket.bucket_key,
                        "task_name": selected_bucket.task_name,
                        "threshold_micro": selected_bucket.threshold_micro,
                        "package_fingerprint64": package_decision.package_fingerprint64,
                        "future_matching_bucket_events": 0usize,
                        "future_score_candidate_rows": 0usize,
                        "verifier_bound_score_candidate_rows": 0usize,
                        "verifier_rejected_score_candidate_rows": 0usize,
                        "future_false_accepts": 0usize,
                        "hot_margin_parity_mismatches": 0usize,
                    })
                });
                increment_json_usize(entry, "future_matching_bucket_events", 1)?;
                increment_json_usize(
                    entry,
                    "future_score_candidate_rows",
                    usize::from(hot_decision.score_candidate),
                )?;
                increment_json_usize(
                    entry,
                    "verifier_bound_score_candidate_rows",
                    usize::from(hot_decision.score_candidate && event.verified_safe_accept),
                )?;
                increment_json_usize(
                    entry,
                    "verifier_rejected_score_candidate_rows",
                    usize::from(hot_decision.score_candidate && !event.verified_safe_accept),
                )?;
                increment_json_usize(
                    entry,
                    "future_false_accepts",
                    usize::from(hot_decision.score_candidate && !event.verified_safe_accept),
                )?;
                increment_json_usize(
                    entry,
                    "hot_margin_parity_mismatches",
                    usize::from(hot_decision.margin_micro != flat_margin_micro),
                )?;
            }
        }
    }

    let future_unique_accepts_over_exact_cache = accepted_fingerprints.len();
    let future_rejected_false_accept_buckets = false_accepts_by_bucket
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(bucket_key, count)| {
            serde_json::json!({
                "bucket_key": bucket_key,
                "future_false_accepts": count
            })
        })
        .collect::<Vec<_>>();
    let future_clean_bucket_keys = selected_bucket_keys
        .iter()
        .filter(|bucket_key| {
            false_accepts_by_bucket
                .get(*bucket_key)
                .copied()
                .unwrap_or(0)
                == 0
        })
        .filter(|bucket_key| accepted_by_bucket.contains_key(*bucket_key))
        .cloned()
        .collect::<Vec<_>>();
    let mut clean_accepted_fingerprints = BTreeMap::<String, (usize, u64)>::new();
    for bucket_key in &future_clean_bucket_keys {
        if let Some(bucket_accepts) = accepted_by_bucket.get(bucket_key) {
            for (request_fingerprint, token_cost) in bucket_accepts {
                clean_accepted_fingerprints
                    .entry(request_fingerprint.clone())
                    .or_insert(*token_cost);
            }
        }
    }
    let future_clean_unique_accepts_over_exact_cache = clean_accepted_fingerprints.len();
    let future_clean_tokens_saved = clean_accepted_fingerprints
        .values()
        .map(|(tokens, _)| *tokens)
        .sum::<usize>();
    let future_clean_cost_saved_microusd = clean_accepted_fingerprints
        .values()
        .map(|(_, cost)| *cost)
        .sum::<u64>();
    for (bucket_key, report_row) in &mut bucket_reports {
        let bucket_accepts = accepted_by_bucket.get(bucket_key);
        let bucket_unique_accepts = bucket_accepts.map(BTreeMap::len).unwrap_or(0);
        let bucket_tokens_saved = bucket_accepts
            .map(|accepts| accepts.values().map(|(tokens, _)| *tokens).sum::<usize>())
            .unwrap_or(0);
        let bucket_cost_saved_microusd = bucket_accepts
            .map(|accepts| accepts.values().map(|(_, cost)| *cost).sum::<u64>())
            .unwrap_or(0);
        let bucket_false_accepts = false_accepts_by_bucket
            .get(bucket_key)
            .copied()
            .unwrap_or(0);
        if let Some(object) = report_row.as_object_mut() {
            object.insert(
                "future_unique_accepts_over_exact_cache".to_owned(),
                serde_json::json!(bucket_unique_accepts),
            );
            object.insert(
                "future_tokens_saved".to_owned(),
                serde_json::json!(bucket_tokens_saved),
            );
            object.insert(
                "future_cost_saved_microusd".to_owned(),
                serde_json::json!(bucket_cost_saved_microusd),
            );
            object.insert(
                "future_clean_candidate".to_owned(),
                serde_json::json!(bucket_unique_accepts > 0 && bucket_false_accepts == 0),
            );
        }
    }
    let provider_correlation_complete_for_future_accepts = future_unique_accepts_over_exact_cache
        > 0
        && external_provider_correlation_key_rows >= future_unique_accepts_over_exact_cache
        && external_provider_correlation_missing_rows == 0;
    let future_runtime_replay_passed = future_rows > 0
        && future_matching_bucket_events > 0
        && missing_package_rows == 0
        && hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && future_false_accepts == 0
        && future_unique_accepts_over_exact_cache > 0;
    let future_clean_portfolio_passed = future_rows > 0
        && future_matching_bucket_events > 0
        && missing_package_rows == 0
        && hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && future_clean_unique_accepts_over_exact_cache > 0
        && !future_clean_bucket_keys.is_empty();
    let calls_saved_milli = ratio_milli(future_unique_accepts_over_exact_cache, future_rows);
    let tokens_saved_milli = ratio_milli(future_tokens_saved, future_total_tokens);
    let cost_saved_milli = ratio_milli_u64(future_cost_saved_microusd, future_total_cost_microusd);
    let verdict = if future_runtime_replay_passed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_REPLAY_V1_PASS_REVIEW_ONLY"
    } else if future_clean_portfolio_passed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_REPLAY_V1_WATCH_CLEAN_SUBSET_AVAILABLE"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_REPLAY_V1_WATCH"
    };
    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_future_tail_replay_v1"),
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
        "future_trace_path".to_owned(),
        serde_json::json!(future_trace_path),
    );
    report.insert(
        "min_future_row_index".to_owned(),
        serde_json::json!(min_future_row_index),
    );
    report.insert("future_rows".to_owned(), serde_json::json!(future_rows));
    report.insert(
        "future_token_rows".to_owned(),
        serde_json::json!(future_token_rows),
    );
    report.insert(
        "future_total_tokens".to_owned(),
        serde_json::json!(future_total_tokens),
    );
    report.insert(
        "future_total_cost_microusd".to_owned(),
        serde_json::json!(future_total_cost_microusd),
    );
    report.insert(
        "future_exact_cache_hits".to_owned(),
        serde_json::json!(future_exact_cache_hits),
    );
    report.insert(
        "future_exact_cache_tokens".to_owned(),
        serde_json::json!(future_exact_cache_tokens),
    );
    report.insert(
        "future_exact_cache_cost_microusd".to_owned(),
        serde_json::json!(future_exact_cache_cost_microusd),
    );
    report.insert(
        "selected_bucket_count".to_owned(),
        serde_json::json!(selected_buckets.len()),
    );
    report.insert(
        "selected_bucket_package_count".to_owned(),
        serde_json::json!(latest_package_by_bucket.len()),
    );
    report.insert(
        "future_matching_bucket_events".to_owned(),
        serde_json::json!(future_matching_bucket_events),
    );
    report.insert(
        "future_score_candidate_rows".to_owned(),
        serde_json::json!(future_score_candidate_rows),
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
        "future_unique_accepts_over_exact_cache".to_owned(),
        serde_json::json!(future_unique_accepts_over_exact_cache),
    );
    report.insert(
        "future_tokens_saved".to_owned(),
        serde_json::json!(future_tokens_saved),
    );
    report.insert(
        "future_cost_saved_microusd".to_owned(),
        serde_json::json!(future_cost_saved_microusd),
    );
    report.insert(
        "future_false_accepts".to_owned(),
        serde_json::json!(future_false_accepts),
    );
    report.insert(
        "future_clean_bucket_count".to_owned(),
        serde_json::json!(future_clean_bucket_keys.len()),
    );
    report.insert(
        "future_clean_bucket_keys".to_owned(),
        serde_json::json!(future_clean_bucket_keys),
    );
    report.insert(
        "future_rejected_false_accept_buckets".to_owned(),
        serde_json::json!(future_rejected_false_accept_buckets),
    );
    report.insert(
        "future_clean_unique_accepts_over_exact_cache".to_owned(),
        serde_json::json!(future_clean_unique_accepts_over_exact_cache),
    );
    report.insert(
        "future_clean_tokens_saved".to_owned(),
        serde_json::json!(future_clean_tokens_saved),
    );
    report.insert(
        "future_clean_cost_saved_microusd".to_owned(),
        serde_json::json!(future_clean_cost_saved_microusd),
    );
    report.insert(
        "future_clean_portfolio_passed".to_owned(),
        serde_json::json!(future_clean_portfolio_passed),
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
        "external_provider_correlation_key_rows".to_owned(),
        serde_json::json!(external_provider_correlation_key_rows),
    );
    report.insert(
        "external_provider_correlation_missing_rows".to_owned(),
        serde_json::json!(external_provider_correlation_missing_rows),
    );
    report.insert(
        "external_provider_correlation_key_count".to_owned(),
        serde_json::json!(external_provider_correlation_key_count),
    );
    report.insert(
        "provider_correlation_complete_for_future_accepts".to_owned(),
        serde_json::json!(provider_correlation_complete_for_future_accepts),
    );
    report.insert(
        "savings_over_exact_cache".to_owned(),
        serde_json::json!({
            "calls_saved": future_unique_accepts_over_exact_cache,
            "calls_saved_milli": calls_saved_milli,
            "tokens_saved": future_tokens_saved,
            "tokens_saved_milli": tokens_saved_milli,
            "estimated_cost_saved_microusd": future_cost_saved_microusd,
            "estimated_cost_saved_milli": cost_saved_milli,
            "market_money_claim_allowed": false
        }),
    );
    report.insert(
        "exact_cache_baseline".to_owned(),
        serde_json::json!({
            "calls_saved": future_exact_cache_hits,
            "calls_saved_milli": ratio_milli(future_exact_cache_hits, future_rows),
            "tokens_saved": future_exact_cache_tokens,
            "tokens_saved_milli": ratio_milli(future_exact_cache_tokens, future_total_tokens),
            "estimated_cost_saved_microusd": future_exact_cache_cost_microusd,
            "estimated_cost_saved_milli": ratio_milli_u64(future_exact_cache_cost_microusd, future_total_cost_microusd)
        }),
    );
    report.insert(
        "bucket_reports".to_owned(),
        serde_json::json!(bucket_reports.values().cloned().collect::<Vec<_>>()),
    );
    report.insert(
        "discovery_mode".to_owned(),
        serde_json::json!({
            "manual_class_list_used": json_bool(&selector, &["discovery_mode", "manual_class_list_used"]).unwrap_or(false),
            "static_topn_seed_used": json_bool(&selector, &["discovery_mode", "static_topn_seed_used"]).unwrap_or(false),
            "online_discovery_used": json_bool(&selector, &["discovery_mode", "online_discovery_used"]).unwrap_or(false),
            "marginal_denominator_delta_used": json_bool(&selector, &["discovery_mode", "marginal_denominator_delta_used"]).unwrap_or(false),
            "portfolio_gate_passed": json_bool(&selector, &["discovery_mode", "portfolio_gate_passed"]).unwrap_or(false),
            "runtime_replay_passed": future_runtime_replay_passed,
            "claim_boundary": "future-tail replay only: selected .nwpc packages learned before the tail are scored on later trace rows without recompiling or changing thresholds"
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
        "future_runtime_replay_passed".to_owned(),
        serde_json::json!(future_runtime_replay_passed),
    );
    report.insert("verdict".to_owned(), serde_json::json!(verdict));
    report.insert(
        "boundary".to_owned(),
        serde_json::json!("future-tail replay/reporting only: loads already selected .nwpc packages and scores later trace rows; does not mine, compile, promote, serve, enable local_accept, claim market money, or revive legacy nwrb"),
    );
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_future_tail_replay_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  future_rows: {future_rows}");
    println!("  future_matching_bucket_events: {future_matching_bucket_events}");
    println!("  future_unique_accepts_over_exact_cache: {future_unique_accepts_over_exact_cache}");
    println!("  future_tokens_saved: {future_tokens_saved}");
    println!("  future_false_accepts: {future_false_accepts}");
    println!("  future_runtime_replay_passed: {future_runtime_replay_passed}");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_portfolio_live_tail_score_only_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_REPORT)
    });
    let score_decision_log_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_DECISIONS)
    });
    let registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PRODUCT_HOT_REGISTRY));
    let append_tail_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_TRACE));
    let idle_sleep_ms = args
        .next()
        .as_deref()
        .map(str::parse::<u64>)
        .transpose()
        .map_err(|error| format!("invalid idle_sleep_ms: {error}"))?
        .unwrap_or(50);
    let max_idle_ms = args
        .next()
        .as_deref()
        .map(str::parse::<u64>)
        .transpose()
        .map_err(|error| format!("invalid max_idle_ms: {error}"))?
        .unwrap_or(5000);
    let max_append_events = args
        .next()
        .as_deref()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|error| format!("invalid max_append_events: {error}"))?
        .unwrap_or(0);

    let registry = read_json_value(&registry_path)?;
    let registry_entries = product_hot_registry_entries_from_report(&registry)?;
    let selected_buckets = registry_entries
        .iter()
        .map(|(bucket_key, entry)| (bucket_key.clone(), entry.selected_bucket.clone()))
        .collect::<BTreeMap<_, _>>();
    if selected_buckets.is_empty() {
        return Err(format!(
            "product-hot registry '{}' has no clean candidates",
            registry_path.display()
        ));
    }

    if let Some(parent) = append_tail_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create live-tail dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    if !append_tail_path.exists() {
        std::fs::File::create(&append_tail_path).map_err(|error| {
            format!(
                "failed to create live-tail append file '{}': {error}",
                append_tail_path.display()
            )
        })?;
    }
    if let Some(parent) = score_decision_log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create live-tail decision dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let decision_file = std::fs::File::create(&score_decision_log_path).map_err(|error| {
        format!(
            "failed to create live-tail score decision log '{}': {error}",
            score_decision_log_path.display()
        )
    })?;
    let mut decision_writer = BufWriter::new(decision_file);

    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(&append_tail_path)
        .map_err(|error| {
            format!(
                "failed to open live-tail append file '{}': {error}",
                append_tail_path.display()
            )
        })?;
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::End(0)).map_err(|error| {
        format!(
            "failed to seek live-tail append file '{}': {error}",
            append_tail_path.display()
        )
    })?;

    let mut runtime_cache = BTreeMap::<u64, ReplayRuntimeEntry>::new();
    let mut accepted_fingerprints = BTreeMap::<String, usize>::new();
    let mut accepted_by_bucket = BTreeMap::<String, BTreeMap<String, (usize, u64)>>::new();
    let mut false_accepts_by_bucket = BTreeMap::<String, usize>::new();
    let mut bucket_reports = BTreeMap::<String, serde_json::Value>::new();
    let mut score_latencies_ns = Vec::<u64>::new();

    let mut append_rows = 0usize;
    let mut parsed_events = 0usize;
    let mut events_with_verifier_label = 0usize;
    let mut append_token_rows = 0usize;
    let mut append_total_tokens = 0usize;
    let mut append_total_cost_microusd = 0u64;
    let mut append_exact_cache_hits = 0usize;
    let mut append_exact_cache_tokens = 0usize;
    let mut append_exact_cache_cost_microusd = 0u64;
    let mut append_matching_bucket_events = 0usize;
    let mut append_score_candidate_rows = 0usize;
    let mut verifier_bound_score_candidate_rows = 0usize;
    let mut verifier_rejected_score_candidate_rows = 0usize;
    let mut append_false_accepts = 0usize;
    let mut hot_margin_parity_mismatches = 0usize;
    let mut hot_decision_parity_mismatches = 0usize;
    let mut missing_package_rows = 0usize;
    let mut append_tokens_saved = 0usize;
    let mut append_cost_saved_microusd = 0u64;
    let mut idle_since = Instant::now();
    let mut last_snapshot_write = Instant::now();

    write_live_tail_score_only_snapshot(
        &report_path,
        &registry_path,
        &append_tail_path,
        &score_decision_log_path,
        &registry,
        selected_buckets.len(),
        registry_entries.len(),
        runtime_cache.len(),
        append_rows,
        events_with_verifier_label,
        append_token_rows,
        append_total_tokens,
        append_total_cost_microusd,
        append_exact_cache_hits,
        append_exact_cache_tokens,
        append_exact_cache_cost_microusd,
        append_matching_bucket_events,
        append_score_candidate_rows,
        verifier_bound_score_candidate_rows,
        verifier_rejected_score_candidate_rows,
        append_false_accepts,
        accepted_fingerprints.len(),
        append_tokens_saved,
        append_cost_saved_microusd,
        missing_package_rows,
        hot_margin_parity_mismatches,
        hot_decision_parity_mismatches,
        &score_latencies_ns,
        true,
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_V1_RUNNING",
    )?;

    loop {
        let mut line = String::new();
        let read_bytes = reader.read_line(&mut line).map_err(|error| {
            format!(
                "failed to read live-tail append file '{}': {error}",
                append_tail_path.display()
            )
        })?;
        if read_bytes == 0 {
            if last_snapshot_write.elapsed() >= Duration::from_secs(5) {
                write_live_tail_score_only_snapshot(
                    &report_path,
                    &registry_path,
                    &append_tail_path,
                    &score_decision_log_path,
                    &registry,
                    selected_buckets.len(),
                    registry_entries.len(),
                    runtime_cache.len(),
                    append_rows,
                    events_with_verifier_label,
                    append_token_rows,
                    append_total_tokens,
                    append_total_cost_microusd,
                    append_exact_cache_hits,
                    append_exact_cache_tokens,
                    append_exact_cache_cost_microusd,
                    append_matching_bucket_events,
                    append_score_candidate_rows,
                    verifier_bound_score_candidate_rows,
                    verifier_rejected_score_candidate_rows,
                    append_false_accepts,
                    accepted_fingerprints.len(),
                    append_tokens_saved,
                    append_cost_saved_microusd,
                    missing_package_rows,
                    hot_margin_parity_mismatches,
                    hot_decision_parity_mismatches,
                    &score_latencies_ns,
                    true,
                    "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_V1_RUNNING",
                )?;
                last_snapshot_write = Instant::now();
            }
            if max_idle_ms > 0 && idle_since.elapsed() >= Duration::from_millis(max_idle_ms) {
                break;
            }
            sleep(Duration::from_millis(idle_sleep_ms.max(1)));
            continue;
        }
        idle_since = Instant::now();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        append_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse live-tail append row {} from '{}': {error}",
                append_rows,
                append_tail_path.display()
            )
        })?;
        let total_tokens = json_usize(json_at(&row, &["token_cost", "total_tokens"])).unwrap_or(0);
        let total_cost_microusd =
            json_u64(&row, &["token_cost", "total_cost_microusd"]).unwrap_or(0);
        if total_tokens > 0 {
            append_token_rows += 1;
            append_total_tokens = append_total_tokens.saturating_add(total_tokens);
        }
        append_total_cost_microusd = append_total_cost_microusd.saturating_add(total_cost_microusd);
        let exact_cache_hit = json_bool(&row, &["exact_cache_hit"]).unwrap_or(false);
        if exact_cache_hit {
            append_exact_cache_hits += 1;
            append_exact_cache_tokens = append_exact_cache_tokens.saturating_add(total_tokens);
            append_exact_cache_cost_microusd =
                append_exact_cache_cost_microusd.saturating_add(total_cost_microusd);
        }
        if row
            .get("verified_safe_accept")
            .and_then(Value::as_bool)
            .is_none()
        {
            if max_append_events > 0 && append_rows >= max_append_events {
                break;
            }
            continue;
        }
        events_with_verifier_label += 1;
        let action_atoms = phase_atom_string_vec(&row, "action_atoms");
        let action_families = phase_atom_action_families(&action_atoms);
        if action_families.is_empty() {
            if max_append_events > 0 && append_rows >= max_append_events {
                break;
            }
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
            if max_append_events > 0 && append_rows >= max_append_events {
                break;
            }
            continue;
        }
        for action_family in action_families {
            let bucket_specs = online_miner_event_bucket_specs(
                &action_family,
                &request_atoms,
                &state_atoms,
                &tool_atoms,
                &route_hint_atoms,
                &[],
            );
            let bucket_keys = online_portfolio_matching_selected_bucket_keys(
                &action_family,
                &request_atoms,
                &state_atoms,
                &tool_atoms,
                &route_hint_atoms,
                &bucket_specs,
                &selected_buckets,
            );
            for bucket_key in bucket_keys {
                let Some(registry_entry) = registry_entries.get(&bucket_key) else {
                    continue;
                };
                append_matching_bucket_events += 1;
                if !runtime_cache.contains_key(&registry_entry.package_fingerprint64) {
                    if !registry_entry.package_path.exists() {
                        missing_package_rows += 1;
                        continue;
                    }
                    let entry = load_runtime_entry(
                        &registry_entry.package_path,
                        registry_entry.selected_bucket.threshold_micro,
                        registry_entry.package_fingerprint64,
                    )?;
                    runtime_cache.insert(registry_entry.package_fingerprint64, entry);
                }
                let Some(event) = parse_phase_atom_binary_event_for_action(
                    &row,
                    parsed_events,
                    &action_family,
                    &registry_entry.selected_bucket.task_name,
                ) else {
                    continue;
                };
                parsed_events += 1;
                let runtime_entry = runtime_cache
                    .get_mut(&registry_entry.package_fingerprint64)
                    .expect("live-tail runtime entry inserted before score");
                let safe_accept_vec = phase_atom_binary_event_vector_for_task(
                    &event,
                    true,
                    runtime_entry.runtime.cells(),
                    &registry_entry.selected_bucket.task_name,
                );
                let zero =
                    vec![nando_core::PhaseCenterCell::default(); runtime_entry.runtime.cells()];
                let task = PhaseCenterEvalTask {
                    center_index: 0,
                    correct_vec: safe_accept_vec.clone().into_boxed_slice(),
                    wrong_vec: zero.into_boxed_slice(),
                };
                let flat_margin_micro =
                    margin_to_micro(runtime_entry.runtime.runtime().margin(&task).map_err(
                        |error| format!("live-tail score-only flat margin error: {error:?}"),
                    )?)?;
                let score_started = Instant::now();
                let candidates = runtime_entry
                    .hot_runtime
                    .score_prepared_hot_request_candidates(
                        &runtime_entry.hot_routes,
                        PhaseCenterPreparedHotRequest::new(0, &safe_accept_vec),
                        &mut runtime_entry.hot_scratch,
                    )
                    .map_err(|error| {
                        format!("live-tail score-only hot runtime error: {error:?}")
                    })?;
                score_latencies_ns.push(duration_nanos_u64(score_started.elapsed()));
                let Some(hot_decision) = candidates.first() else {
                    return Err(
                        "live-tail score-only hot runtime returned no candidates".to_owned()
                    );
                };
                if hot_decision.margin_micro != flat_margin_micro {
                    hot_margin_parity_mismatches += 1;
                }
                let flat_score_candidate =
                    flat_margin_micro >= registry_entry.selected_bucket.threshold_micro;
                if hot_decision.score_candidate != flat_score_candidate {
                    hot_decision_parity_mismatches += 1;
                }
                let mut unique_accept_over_exact_cache = false;
                if hot_decision.score_candidate {
                    append_score_candidate_rows += 1;
                    if event.verified_safe_accept {
                        verifier_bound_score_candidate_rows += 1;
                        if !exact_cache_hit
                            && accepted_fingerprints
                                .insert(event.request_fingerprint.clone(), 1)
                                .is_none()
                        {
                            unique_accept_over_exact_cache = true;
                            append_tokens_saved =
                                append_tokens_saved.saturating_add(event.token_cost.total_tokens);
                            append_cost_saved_microusd = append_cost_saved_microusd
                                .saturating_add(event.token_cost.total_cost_microusd);
                            accepted_by_bucket
                                .entry(bucket_key.clone())
                                .or_default()
                                .insert(
                                    event.request_fingerprint.clone(),
                                    (
                                        event.token_cost.total_tokens,
                                        event.token_cost.total_cost_microusd,
                                    ),
                                );
                        }
                    } else {
                        append_false_accepts += 1;
                        verifier_rejected_score_candidate_rows += 1;
                        *false_accepts_by_bucket
                            .entry(bucket_key.clone())
                            .or_default() += 1;
                    }
                }
                let mut match_keys =
                    vec![format!("request_fingerprint:{}", event.request_fingerprint)];
                if !event.exact_cache_key.is_empty() {
                    match_keys.push(format!("exact_cache_key:{}", event.exact_cache_key));
                }
                match_keys.extend(event.external_provider_correlation_keys.iter().cloned());
                match_keys.sort();
                match_keys.dedup();
                let decision = serde_json::json!({
                    "schema_version": "phase_stream_online_miner_portfolio_live_tail_score_only_decision_v1",
                    "append_row_index": append_rows,
                    "input_trace_path": json_string(&row, &["input_trace_path"]),
                    "event_timestamp": json_string(&row, &["event_timestamp"]).or_else(|| json_string(&row, &["timestamp"])),
                    "trace_id": json_string(&row, &["trace_id"]),
                    "bucket_key": registry_entry.selected_bucket.bucket_key,
                    "task_name": registry_entry.selected_bucket.task_name,
                    "package_fingerprint64": registry_entry.package_fingerprint64,
                    "request_fingerprint": event.request_fingerprint,
                    "exact_cache_key": event.exact_cache_key,
                    "external_provider_correlation_keys": event.external_provider_correlation_keys,
                    "provider_correlation_ready": !event.external_provider_correlation_keys.is_empty(),
                    "match_keys": match_keys,
                    "token_cost": {
                        "total_tokens": event.token_cost.total_tokens,
                        "total_cost_microusd": event.token_cost.total_cost_microusd,
                        "token_evidence_missing": event.token_cost.token_evidence_missing,
                        "cost_evidence_missing": event.token_cost.cost_evidence_missing,
                        "evidence_missing": event.token_cost.evidence_missing
                    },
                    "exact_cache_hit": exact_cache_hit,
                    "verified_safe_accept": event.verified_safe_accept,
                    "margin_micro": hot_decision.margin_micro,
                    "flat_margin_micro": flat_margin_micro,
                    "threshold_micro": registry_entry.selected_bucket.threshold_micro,
                    "score_candidate": hot_decision.score_candidate,
                    "false_accept": hot_decision.score_candidate && !event.verified_safe_accept,
                    "unique_cpu_accept_over_exact_cache": unique_accept_over_exact_cache,
                    "local_accept_enabled": false,
                    "market_money_claim_allowed": false
                });
                serde_json::to_writer(&mut decision_writer, &decision).map_err(|error| {
                    format!(
                        "failed to serialize live-tail score decision '{}': {error}",
                        score_decision_log_path.display()
                    )
                })?;
                decision_writer.write_all(b"\n").map_err(|error| {
                    format!(
                        "failed to write live-tail score decision '{}': {error}",
                        score_decision_log_path.display()
                    )
                })?;
                decision_writer.flush().map_err(|error| {
                    format!(
                        "failed to flush live-tail score decision '{}': {error}",
                        score_decision_log_path.display()
                    )
                })?;
                let entry = bucket_reports.entry(bucket_key.clone()).or_insert_with(|| {
                    serde_json::json!({
                        "bucket_key": registry_entry.selected_bucket.bucket_key,
                        "task_name": registry_entry.selected_bucket.task_name,
                        "threshold_micro": registry_entry.selected_bucket.threshold_micro,
                        "package_fingerprint64": registry_entry.package_fingerprint64,
                        "append_matching_bucket_events": 0usize,
                        "append_score_candidate_rows": 0usize,
                        "verifier_bound_score_candidate_rows": 0usize,
                        "verifier_rejected_score_candidate_rows": 0usize,
                        "append_false_accepts": 0usize,
                        "hot_margin_parity_mismatches": 0usize,
                    })
                });
                increment_json_usize(entry, "append_matching_bucket_events", 1)?;
                increment_json_usize(
                    entry,
                    "append_score_candidate_rows",
                    usize::from(hot_decision.score_candidate),
                )?;
                increment_json_usize(
                    entry,
                    "verifier_bound_score_candidate_rows",
                    usize::from(hot_decision.score_candidate && event.verified_safe_accept),
                )?;
                increment_json_usize(
                    entry,
                    "verifier_rejected_score_candidate_rows",
                    usize::from(hot_decision.score_candidate && !event.verified_safe_accept),
                )?;
                increment_json_usize(
                    entry,
                    "append_false_accepts",
                    usize::from(hot_decision.score_candidate && !event.verified_safe_accept),
                )?;
                increment_json_usize(
                    entry,
                    "hot_margin_parity_mismatches",
                    usize::from(hot_decision.margin_micro != flat_margin_micro),
                )?;
            }
        }
        if max_append_events > 0 && append_rows >= max_append_events {
            break;
        }
        if last_snapshot_write.elapsed() >= Duration::from_secs(5) {
            write_live_tail_score_only_snapshot(
                &report_path,
                &registry_path,
                &append_tail_path,
                &score_decision_log_path,
                &registry,
                selected_buckets.len(),
                registry_entries.len(),
                runtime_cache.len(),
                append_rows,
                events_with_verifier_label,
                append_token_rows,
                append_total_tokens,
                append_total_cost_microusd,
                append_exact_cache_hits,
                append_exact_cache_tokens,
                append_exact_cache_cost_microusd,
                append_matching_bucket_events,
                append_score_candidate_rows,
                verifier_bound_score_candidate_rows,
                verifier_rejected_score_candidate_rows,
                append_false_accepts,
                accepted_fingerprints.len(),
                append_tokens_saved,
                append_cost_saved_microusd,
                missing_package_rows,
                hot_margin_parity_mismatches,
                hot_decision_parity_mismatches,
                &score_latencies_ns,
                true,
                "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_V1_RUNNING",
            )?;
            last_snapshot_write = Instant::now();
        }
    }
    decision_writer.flush().map_err(|error| {
        format!(
            "failed to flush live-tail score decision '{}': {error}",
            score_decision_log_path.display()
        )
    })?;

    score_latencies_ns.sort_unstable();
    let append_unique_accepts_over_exact_cache = accepted_fingerprints.len();
    let append_calls_saved_milli = ratio_milli(append_unique_accepts_over_exact_cache, append_rows);
    let append_tokens_saved_milli = ratio_milli(append_tokens_saved, append_total_tokens);
    let append_cost_saved_milli =
        ratio_milli_u64(append_cost_saved_microusd, append_total_cost_microusd);
    let exact_cache_calls_saved_milli = ratio_milli(append_exact_cache_hits, append_rows);
    let exact_cache_tokens_saved_milli =
        ratio_milli(append_exact_cache_tokens, append_total_tokens);
    let exact_cache_cost_saved_milli =
        ratio_milli_u64(append_exact_cache_cost_microusd, append_total_cost_microusd);
    let clean_runtime_score_only_passed = append_rows > 0
        && append_matching_bucket_events > 0
        && append_score_candidate_rows > 0
        && append_false_accepts == 0
        && missing_package_rows == 0
        && hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && append_unique_accepts_over_exact_cache > 0;
    let verdict = if clean_runtime_score_only_passed {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_V1_PASS_SHADOW_ONLY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_V1_WATCH"
    };

    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_live_tail_score_only_v1"),
    );
    report.insert(
        "mode".to_owned(),
        serde_json::json!("product_hot_score_only_live_tail_shadow"),
    );
    report.insert("registry_path".to_owned(), serde_json::json!(registry_path));
    report.insert(
        "append_tail_path".to_owned(),
        serde_json::json!(append_tail_path),
    );
    report.insert(
        "score_decision_log_path".to_owned(),
        serde_json::json!(score_decision_log_path),
    );
    report.insert(
        "registry_kind".to_owned(),
        serde_json::json!(json_string(&registry, &["registry_kind"]).unwrap_or_default()),
    );
    report.insert(
        "source_report_path".to_owned(),
        serde_json::json!(json_string(&registry, &["source_report_path"]).unwrap_or_default()),
    );
    report.insert(
        "selected_bucket_count".to_owned(),
        serde_json::json!(selected_buckets.len()),
    );
    report.insert(
        "selected_bucket_package_count".to_owned(),
        serde_json::json!(registry_entries.len()),
    );
    report.insert(
        "runtime_package_count".to_owned(),
        serde_json::json!(runtime_cache.len()),
    );
    report.insert("append_rows".to_owned(), serde_json::json!(append_rows));
    report.insert(
        "events_with_verifier_label".to_owned(),
        serde_json::json!(events_with_verifier_label),
    );
    report.insert(
        "append_token_rows".to_owned(),
        serde_json::json!(append_token_rows),
    );
    report.insert(
        "append_total_tokens".to_owned(),
        serde_json::json!(append_total_tokens),
    );
    report.insert(
        "append_total_cost_microusd".to_owned(),
        serde_json::json!(append_total_cost_microusd),
    );
    report.insert(
        "append_exact_cache_hits".to_owned(),
        serde_json::json!(append_exact_cache_hits),
    );
    report.insert(
        "append_exact_cache_tokens".to_owned(),
        serde_json::json!(append_exact_cache_tokens),
    );
    report.insert(
        "append_exact_cache_cost_microusd".to_owned(),
        serde_json::json!(append_exact_cache_cost_microusd),
    );
    report.insert(
        "append_matching_bucket_events".to_owned(),
        serde_json::json!(append_matching_bucket_events),
    );
    report.insert(
        "append_score_candidate_rows".to_owned(),
        serde_json::json!(append_score_candidate_rows),
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
        "append_false_accepts".to_owned(),
        serde_json::json!(append_false_accepts),
    );
    report.insert(
        "append_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(append_unique_accepts_over_exact_cache),
    );
    report.insert(
        "append_tokens_saved".to_owned(),
        serde_json::json!(append_tokens_saved),
    );
    report.insert(
        "append_cost_saved_microusd".to_owned(),
        serde_json::json!(append_cost_saved_microusd),
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
        "score_latency_p50_ns".to_owned(),
        serde_json::json!(percentile_u64(&score_latencies_ns, 50)),
    );
    report.insert(
        "score_latency_p90_ns".to_owned(),
        serde_json::json!(percentile_u64(&score_latencies_ns, 90)),
    );
    report.insert(
        "score_latency_p99_ns".to_owned(),
        serde_json::json!(percentile_u64(&score_latencies_ns, 99)),
    );
    report.insert(
        "score_latency_max_ns".to_owned(),
        serde_json::json!(score_latencies_ns.last().copied().unwrap_or(0)),
    );
    report.insert(
        "savings_over_exact_cache".to_owned(),
        serde_json::json!({
            "calls_saved": append_unique_accepts_over_exact_cache,
            "calls_saved_milli": append_calls_saved_milli,
            "tokens_saved": append_tokens_saved,
            "tokens_saved_milli": append_tokens_saved_milli,
            "estimated_cost_saved_microusd": append_cost_saved_microusd,
            "estimated_cost_saved_milli": append_cost_saved_milli,
            "market_money_claim_allowed": false
        }),
    );
    report.insert(
        "exact_cache_baseline".to_owned(),
        serde_json::json!({
            "calls_saved": append_exact_cache_hits,
            "calls_saved_milli": exact_cache_calls_saved_milli,
            "tokens_saved": append_exact_cache_tokens,
            "tokens_saved_milli": exact_cache_tokens_saved_milli,
            "estimated_cost_saved_microusd": append_exact_cache_cost_microusd,
            "estimated_cost_saved_milli": exact_cache_cost_saved_milli
        }),
    );
    report.insert(
        "bucket_reports".to_owned(),
        serde_json::json!(bucket_reports.values().cloned().collect::<Vec<_>>()),
    );
    report.insert(
        "accepted_bucket_count".to_owned(),
        serde_json::json!(accepted_by_bucket.len()),
    );
    report.insert(
        "false_accept_bucket_count".to_owned(),
        serde_json::json!(false_accepts_by_bucket.len()),
    );
    report.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
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
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        }),
    );
    report.insert(
        "clean_runtime_score_only_passed".to_owned(),
        serde_json::json!(clean_runtime_score_only_passed),
    );
    report.insert("verdict".to_owned(), serde_json::json!(verdict));
    report.insert(
        "boundary".to_owned(),
        serde_json::json!("score-only product-hot live tail: starts at EOF, loads already promoted quarantine .nwpc packages, scores appended verifier-labeled events, does not mine, compile, change thresholds, promote, enable local_accept, claim market money, use lookup, or revive legacy nwrb"),
    );
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_live_tail_score_only_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  registry_path: {}", registry_path.display());
    println!("  append_tail_path: {}", append_tail_path.display());
    println!("  selected_bucket_count: {}", selected_buckets.len());
    println!("  append_rows: {append_rows}");
    println!("  append_matching_bucket_events: {append_matching_bucket_events}");
    println!("  append_score_candidate_rows: {append_score_candidate_rows}");
    println!(
        "  append_unique_cpu_accepts_over_exact_cache: {append_unique_accepts_over_exact_cache}"
    );
    println!("  append_false_accepts: {append_false_accepts}");
    println!(
        "  score_latency_p99_ns: {}",
        percentile_u64(&score_latencies_ns, 99)
    );
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_portfolio_future_tail_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_BILLING_REQUEST_REPORT)
    });
    let request_jsonl_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_BILLING_REQUEST_JSONL)
    });
    let future_tail_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_REPLAY_REPORT));

    let future_tail = read_json_value(&future_tail_report_path)?;
    let source_clean = json_bool(&future_tail, &["future_runtime_replay_passed"]).unwrap_or(false)
        && json_usize(json_at(&future_tail, &["future_false_accepts"])).unwrap_or(usize::MAX) == 0
        && json_usize(json_at(
            &future_tail,
            &["future_unique_accepts_over_exact_cache"],
        ))
        .unwrap_or(0)
            > 0;
    let selector_report_path = PathBuf::from(
        json_string(&future_tail, &["selector_report_path"]).ok_or_else(|| {
            format!(
                "future-tail report '{}' missing selector_report_path",
                future_tail_report_path.display()
            )
        })?,
    );
    let online_miner_report_path = PathBuf::from(
        json_string(&future_tail, &["online_miner_report_path"]).ok_or_else(|| {
            format!(
                "future-tail report '{}' missing online_miner_report_path",
                future_tail_report_path.display()
            )
        })?,
    );
    let decision_log_path = PathBuf::from(
        json_string(&future_tail, &["decision_log_path"]).ok_or_else(|| {
            format!(
                "future-tail report '{}' missing decision_log_path",
                future_tail_report_path.display()
            )
        })?,
    );
    let future_trace_path = PathBuf::from(
        json_string(&future_tail, &["future_trace_path"]).ok_or_else(|| {
            format!(
                "future-tail report '{}' missing future_trace_path",
                future_tail_report_path.display()
            )
        })?,
    );
    let min_future_row_index =
        json_usize(json_at(&future_tail, &["min_future_row_index"])).unwrap_or(5000);

    let selector = read_json_value(&selector_report_path)?;
    let online_miner = read_json_value(&online_miner_report_path)?;
    let selected_buckets = selected_buckets_from_report(&selector)?;
    let selected_bucket_keys = selected_buckets
        .keys()
        .cloned()
        .collect::<BTreeSet<String>>();
    let selected_decisions =
        selected_decisions_from_log(&decision_log_path, &selected_bucket_keys)?;
    let package_paths = package_paths_by_fingerprint(&online_miner)?;
    let mut latest_package_by_bucket = BTreeMap::<String, SelectedDecision>::new();
    for decision in selected_decisions {
        if decision.package_fingerprint64 == 0 {
            continue;
        }
        latest_package_by_bucket
            .entry(decision.bucket_key.clone())
            .and_modify(|current| {
                if decision.denominator_row_index >= current.denominator_row_index {
                    *current = decision.clone();
                }
            })
            .or_insert(decision);
    }

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create future-tail billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let request_file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create future-tail billing request JSONL '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(request_file);

    let future_text = std::fs::read_to_string(&future_trace_path).map_err(|error| {
        format!(
            "failed to read future-tail trace '{}': {error}",
            future_trace_path.display()
        )
    })?;
    let mut runtime_cache = BTreeMap::<u64, ReplayRuntimeEntry>::new();
    let mut emitted_request_fingerprints = BTreeSet::<String>::new();
    let mut future_rows = 0usize;
    let mut future_matching_bucket_events = 0usize;
    let mut future_score_candidate_rows = 0usize;
    let mut verifier_bound_score_candidate_rows = 0usize;
    let mut verifier_rejected_score_candidate_rows = 0usize;
    let mut skipped_source_not_clean = 0usize;
    let mut skipped_exact_cache_hit = 0usize;
    let mut skipped_duplicate_request_fingerprint = 0usize;
    let mut missing_package_rows = 0usize;
    let mut hot_margin_parity_mismatches = 0usize;
    let mut hot_decision_parity_mismatches = 0usize;
    let mut request_rows = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut external_provider_correlation_key_rows = 0usize;
    let mut external_provider_correlation_missing_rows = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;
    let mut parsed_events = 0usize;

    for (line_index, line) in future_text.lines().enumerate() {
        let row_number = line_index + 1;
        if row_number <= min_future_row_index {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        future_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse future-tail trace '{}' line {}: {error}",
                future_trace_path.display(),
                row_number
            )
        })?;
        if !source_clean {
            skipped_source_not_clean += 1;
            continue;
        }
        if row
            .get("verified_safe_accept")
            .and_then(Value::as_bool)
            .is_none()
        {
            continue;
        }
        let exact_cache_hit = json_bool(&row, &["exact_cache_hit"]).unwrap_or(false);
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
            let bucket_specs = online_miner_event_bucket_specs(
                &action_family,
                &request_atoms,
                &state_atoms,
                &tool_atoms,
                &route_hint_atoms,
                &[],
            );
            let bucket_keys = online_portfolio_matching_selected_bucket_keys(
                &action_family,
                &request_atoms,
                &state_atoms,
                &tool_atoms,
                &route_hint_atoms,
                &bucket_specs,
                &selected_buckets,
            );
            for bucket_key in bucket_keys {
                let Some(selected_bucket) = selected_buckets.get(&bucket_key) else {
                    continue;
                };
                future_matching_bucket_events += 1;
                let Some(package_decision) = latest_package_by_bucket.get(&bucket_key) else {
                    missing_package_rows += 1;
                    continue;
                };
                let Some(package_path) = package_paths.get(&package_decision.package_fingerprint64)
                else {
                    missing_package_rows += 1;
                    continue;
                };
                if !runtime_cache.contains_key(&package_decision.package_fingerprint64) {
                    let entry = load_runtime_entry(
                        package_path,
                        selected_bucket.threshold_micro,
                        package_decision.package_fingerprint64,
                    )?;
                    runtime_cache.insert(package_decision.package_fingerprint64, entry);
                }
                let Some(event) = parse_phase_atom_binary_event_for_action(
                    &row,
                    parsed_events,
                    &action_family,
                    &selected_bucket.task_name,
                ) else {
                    continue;
                };
                parsed_events += 1;
                let runtime_entry = runtime_cache
                    .get_mut(&package_decision.package_fingerprint64)
                    .expect("future-tail billing runtime entry inserted before replay");
                let safe_accept_vec = phase_atom_binary_event_vector_for_task(
                    &event,
                    true,
                    runtime_entry.runtime.cells(),
                    &selected_bucket.task_name,
                );
                let zero =
                    vec![nando_core::PhaseCenterCell::default(); runtime_entry.runtime.cells()];
                let task = PhaseCenterEvalTask {
                    center_index: 0,
                    correct_vec: safe_accept_vec.clone().into_boxed_slice(),
                    wrong_vec: zero.into_boxed_slice(),
                };
                let flat_margin_micro =
                    margin_to_micro(runtime_entry.runtime.runtime().margin(&task).map_err(
                        |error| format!("future-tail billing flat margin error: {error:?}"),
                    )?)?;
                let candidates = runtime_entry
                    .hot_runtime
                    .score_prepared_hot_request_candidates(
                        &runtime_entry.hot_routes,
                        PhaseCenterPreparedHotRequest::new(0, &safe_accept_vec),
                        &mut runtime_entry.hot_scratch,
                    )
                    .map_err(|error| format!("future-tail billing hot runtime error: {error:?}"))?;
                let Some(hot_decision) = candidates.first() else {
                    return Err("future-tail billing hot runtime returned no candidates".to_owned());
                };
                if hot_decision.margin_micro != flat_margin_micro {
                    hot_margin_parity_mismatches += 1;
                }
                let flat_score_candidate = flat_margin_micro >= selected_bucket.threshold_micro;
                if hot_decision.score_candidate != flat_score_candidate {
                    hot_decision_parity_mismatches += 1;
                }
                if !hot_decision.score_candidate {
                    continue;
                }
                future_score_candidate_rows += 1;
                if !event.verified_safe_accept {
                    verifier_rejected_score_candidate_rows += 1;
                    continue;
                }
                verifier_bound_score_candidate_rows += 1;
                if exact_cache_hit {
                    skipped_exact_cache_hit += 1;
                    continue;
                }
                if !emitted_request_fingerprints.insert(event.request_fingerprint.clone()) {
                    skipped_duplicate_request_fingerprint += 1;
                    continue;
                }
                let mut match_keys =
                    vec![format!("request_fingerprint:{}", event.request_fingerprint)];
                if !event.exact_cache_key.is_empty() {
                    request_rows_with_exact_cache_key += 1;
                    match_keys.push(format!("exact_cache_key:{}", event.exact_cache_key));
                }
                let provider_correlation_ready =
                    !event.external_provider_correlation_keys.is_empty();
                if provider_correlation_ready {
                    external_provider_correlation_key_rows += 1;
                    match_keys.extend(event.external_provider_correlation_keys.iter().cloned());
                } else {
                    external_provider_correlation_missing_rows += 1;
                }
                match_keys.sort();
                match_keys.dedup();
                total_tokens_requiring_billing =
                    total_tokens_requiring_billing.saturating_add(event.token_cost.total_tokens);
                current_known_cost_microusd = current_known_cost_microusd
                    .saturating_add(event.token_cost.total_cost_microusd);
                request_rows += 1;
                let request = serde_json::json!({
                    "schema_version": "phase_stream_online_miner_portfolio_future_tail_billing_request_v1",
                    "billing_request_id": format!(
                        "online-portfolio-future-tail-cpu-accept-{}-{}",
                        row_number,
                        request_rows
                    ),
                    "request_fingerprint": event.request_fingerprint,
                    "exact_cache_key": event.exact_cache_key,
                    "external_provider_correlation_keys": event.external_provider_correlation_keys,
                    "provider_correlation_ready": provider_correlation_ready,
                    "match_keys": match_keys,
                    "bucket_key": selected_bucket.bucket_key,
                    "task_name": selected_bucket.task_name,
                    "package_fingerprint64": package_decision.package_fingerprint64,
                    "denominator_row_index": row_number,
                    "margin_micro": hot_decision.margin_micro,
                    "threshold_micro": selected_bucket.threshold_micro,
                    "estimated_total_tokens": event.token_cost.total_tokens,
                    "current_total_cost_microusd": event.token_cost.total_cost_microusd,
                    "token_evidence_missing": event.token_cost.token_evidence_missing,
                    "cost_evidence_missing": event.token_cost.cost_evidence_missing,
                    "unique_cpu_accept_over_exact_cache": true,
                    "verified_safe_accept": true,
                    "false_accept": false,
                    "local_accept_enabled": false,
                    "market_money_claim_allowed": false,
                    "boundary": "future-tail billing request export only: verified clean .nwpc shadow accepts are converted to provider billing match keys; no serving, local_accept, promotion, money claim, lookup, or legacy nwrb"
                });
                serde_json::to_writer(&mut writer, &request).map_err(|error| {
                    format!(
                        "failed to serialize future-tail billing request '{}': {error}",
                        request_jsonl_path.display()
                    )
                })?;
                writer.write_all(b"\n").map_err(|error| {
                    format!(
                        "failed to write future-tail billing request '{}': {error}",
                        request_jsonl_path.display()
                    )
                })?;
            }
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush future-tail billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let runtime_accepts = json_usize(json_at(
        &future_tail,
        &["future_unique_accepts_over_exact_cache"],
    ))
    .unwrap_or(0);
    let runtime_tokens = json_usize(json_at(&future_tail, &["future_tokens_saved"])).unwrap_or(0);
    let accept_parity = request_rows == runtime_accepts;
    let token_parity = total_tokens_requiring_billing == runtime_tokens;
    let provider_correlation_parity =
        request_rows > 0 && external_provider_correlation_key_rows == request_rows;
    let request_ready = request_rows > 0
        && source_clean
        && accept_parity
        && token_parity
        && missing_package_rows == 0
        && hot_margin_parity_mismatches == 0
        && hot_decision_parity_mismatches == 0
        && verifier_rejected_score_candidate_rows == 0;
    let ready_for_external_provider_evidence = request_ready && provider_correlation_parity;
    let verdict = if ready_for_external_provider_evidence {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_BILLING_REQUEST_V1_READY_FOR_EXTERNAL_EVIDENCE"
    } else if request_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_BILLING_REQUEST_V1_WATCH_PROVIDER_CORRELATION_MISSING"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_FUTURE_TAIL_BILLING_REQUEST_V1_BLOCKED"
    };

    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_future_tail_billing_request_v1"),
    );
    report.insert(
        "future_tail_report_path".to_owned(),
        serde_json::json!(future_tail_report_path),
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
        "future_trace_path".to_owned(),
        serde_json::json!(future_trace_path),
    );
    report.insert(
        "billing_request_jsonl_path".to_owned(),
        serde_json::json!(request_jsonl_path),
    );
    report.insert(
        "min_future_row_index".to_owned(),
        serde_json::json!(min_future_row_index),
    );
    report.insert(
        "source_future_tail_clean".to_owned(),
        serde_json::json!(source_clean),
    );
    report.insert(
        "selected_bucket_count".to_owned(),
        serde_json::json!(selected_buckets.len()),
    );
    report.insert("future_rows".to_owned(), serde_json::json!(future_rows));
    report.insert(
        "future_matching_bucket_events".to_owned(),
        serde_json::json!(future_matching_bucket_events),
    );
    report.insert(
        "future_score_candidate_rows".to_owned(),
        serde_json::json!(future_score_candidate_rows),
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
        "billing_request_rows".to_owned(),
        serde_json::json!(request_rows),
    );
    report.insert(
        "request_rows_with_exact_cache_key".to_owned(),
        serde_json::json!(request_rows_with_exact_cache_key),
    );
    report.insert(
        "external_provider_correlation_key_rows".to_owned(),
        serde_json::json!(external_provider_correlation_key_rows),
    );
    report.insert(
        "external_provider_correlation_missing_rows".to_owned(),
        serde_json::json!(external_provider_correlation_missing_rows),
    );
    report.insert(
        "provider_correlation_parity".to_owned(),
        serde_json::json!(provider_correlation_parity),
    );
    report.insert(
        "ready_for_external_provider_evidence".to_owned(),
        serde_json::json!(ready_for_external_provider_evidence),
    );
    report.insert(
        "runtime_future_unique_accepts_over_exact_cache".to_owned(),
        serde_json::json!(runtime_accepts),
    );
    report.insert("accept_parity".to_owned(), serde_json::json!(accept_parity));
    report.insert(
        "total_tokens_requiring_billing".to_owned(),
        serde_json::json!(total_tokens_requiring_billing),
    );
    report.insert(
        "runtime_future_tokens_saved".to_owned(),
        serde_json::json!(runtime_tokens),
    );
    report.insert("token_parity".to_owned(), serde_json::json!(token_parity));
    report.insert(
        "current_known_cost_microusd".to_owned(),
        serde_json::json!(current_known_cost_microusd),
    );
    report.insert(
        "skipped_source_not_clean".to_owned(),
        serde_json::json!(skipped_source_not_clean),
    );
    report.insert(
        "skipped_exact_cache_hit".to_owned(),
        serde_json::json!(skipped_exact_cache_hit),
    );
    report.insert(
        "skipped_duplicate_request_fingerprint".to_owned(),
        serde_json::json!(skipped_duplicate_request_fingerprint),
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
        "billing_gate".to_owned(),
        serde_json::json!({
            "provider_billing_request_only": true,
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "policy": "future-tail request rows are exact keys for external provider billing evidence; this artifact is not evidence that money was saved"
        }),
    );
    report.insert(
        "provider_correlation_gate".to_owned(),
        serde_json::json!({
            "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
            "external_provider_correlation_missing_rows": external_provider_correlation_missing_rows,
            "provider_correlation_parity": provider_correlation_parity,
            "ready_for_external_provider_evidence": ready_for_external_provider_evidence,
            "policy": "request rows can be joined only if the external provider export carries request_fingerprint, exact_cache_key, or external provider correlation keys; internal match keys alone are not provider billing evidence"
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
        serde_json::json!("future-tail billing request export only: clean future-tail .nwpc accepts are converted to provider billing match keys; no serving, local_accept, promotion, money claim, lookup, or legacy nwrb"),
    );
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_future_tail_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  accept_parity: {accept_parity}");
    println!("  token_parity: {token_parity}");
    println!("  ready_for_external_provider_evidence: {ready_for_external_provider_evidence}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_online_miner_portfolio_live_tail_billing_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_BILLING_REQUEST_REPORT)
    });
    let request_jsonl_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_BILLING_REQUEST_JSONL)
    });
    let live_score_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_REPORT)
    });
    let live_score = read_json_value(&live_score_report_path)?;
    let decision_log_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        json_string(&live_score, &["score_decision_log_path"])
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_SCORE_ONLY_DECISIONS)
            })
    });

    if let Some(parent) = request_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create live-tail billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let request_file = std::fs::File::create(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to create live-tail billing request JSONL '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(request_file);
    let decision_text = std::fs::read_to_string(&decision_log_path).map_err(|error| {
        format!(
            "failed to read live-tail score decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;

    let source_accepts = json_usize(json_at(
        &live_score,
        &["append_unique_cpu_accepts_over_exact_cache"],
    ))
    .unwrap_or(0);
    let source_tokens = json_usize(json_at(&live_score, &["append_tokens_saved"])).unwrap_or(0);
    let source_cost = json_u64(&live_score, &["append_cost_saved_microusd"]).unwrap_or(0);
    let source_false_accepts =
        json_usize(json_at(&live_score, &["append_false_accepts"])).unwrap_or(usize::MAX);
    let source_clean = source_accepts > 0
        && source_false_accepts == 0
        && !json_bool(&live_score, &["local_accept_enabled"]).unwrap_or(true)
        && !json_bool(&live_score, &["market_money_claim_allowed"]).unwrap_or(true);

    let mut decision_rows = 0usize;
    let mut candidate_rows = 0usize;
    let mut verifier_bound_rows = 0usize;
    let mut rejected_rows = 0usize;
    let mut skipped_source_not_clean = 0usize;
    let mut skipped_non_candidate = 0usize;
    let mut skipped_not_unique_over_exact_cache = 0usize;
    let mut skipped_false_accept = 0usize;
    let mut request_rows = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut external_provider_correlation_key_rows = 0usize;
    let mut external_provider_correlation_missing_rows = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;
    let mut emitted_request_fingerprints = BTreeSet::<String>::new();

    for (line_index, line) in decision_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        decision_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse live-tail score decision '{}' line {}: {error}",
                decision_log_path.display(),
                line_index + 1
            )
        })?;
        if !source_clean {
            skipped_source_not_clean += 1;
            continue;
        }
        let score_candidate = json_bool(&row, &["score_candidate"]).unwrap_or(false);
        if !score_candidate {
            skipped_non_candidate += 1;
            continue;
        }
        candidate_rows += 1;
        let verified_safe_accept = json_bool(&row, &["verified_safe_accept"]).unwrap_or(false);
        if verified_safe_accept {
            verifier_bound_rows += 1;
        } else {
            rejected_rows += 1;
        }
        let false_accept = json_bool(&row, &["false_accept"]).unwrap_or(false);
        if false_accept || !verified_safe_accept {
            skipped_false_accept += 1;
            continue;
        }
        let unique_over_exact =
            json_bool(&row, &["unique_cpu_accept_over_exact_cache"]).unwrap_or(false);
        if !unique_over_exact {
            skipped_not_unique_over_exact_cache += 1;
            continue;
        }
        let request_fingerprint = json_string(&row, &["request_fingerprint"])
            .unwrap_or_else(|| format!("live-tail-score-decision:{}", line_index + 1));
        if !emitted_request_fingerprints.insert(request_fingerprint.clone()) {
            skipped_not_unique_over_exact_cache += 1;
            continue;
        }
        let exact_cache_key = json_string(&row, &["exact_cache_key"]).unwrap_or_default();
        let external_provider_correlation_keys =
            local_json_string_vec(json_at(&row, &["external_provider_correlation_keys"]));
        let mut match_keys = local_json_string_vec(json_at(&row, &["match_keys"]));
        if match_keys.is_empty() {
            match_keys.push(format!("request_fingerprint:{request_fingerprint}"));
            if !exact_cache_key.is_empty() {
                match_keys.push(format!("exact_cache_key:{exact_cache_key}"));
            }
            match_keys.extend(external_provider_correlation_keys.iter().cloned());
        }
        if !exact_cache_key.is_empty() {
            request_rows_with_exact_cache_key += 1;
        }
        let input_trace_path = json_string(&row, &["input_trace_path"]);
        let event_timestamp =
            json_string(&row, &["event_timestamp"]).or_else(|| json_string(&row, &["timestamp"]));
        let trace_id = json_string(&row, &["trace_id"]);
        if let Some(value) = trace_id.as_deref().filter(|value| !value.is_empty()) {
            match_keys.push(format!("trace_id:{value}"));
        }
        match_keys.sort();
        match_keys.dedup();
        let provider_correlation_ready = !external_provider_correlation_keys.is_empty();
        if provider_correlation_ready {
            external_provider_correlation_key_rows += 1;
        } else {
            external_provider_correlation_missing_rows += 1;
        }
        let estimated_total_tokens =
            json_usize(json_at(&row, &["token_cost", "total_tokens"])).unwrap_or(0);
        let current_total_cost_microusd =
            json_u64(&row, &["token_cost", "total_cost_microusd"]).unwrap_or(0);
        total_tokens_requiring_billing =
            total_tokens_requiring_billing.saturating_add(estimated_total_tokens);
        current_known_cost_microusd =
            current_known_cost_microusd.saturating_add(current_total_cost_microusd);
        request_rows += 1;
        let request = serde_json::json!({
            "schema_version": "phase_stream_online_miner_portfolio_live_tail_billing_request_v1",
            "billing_request_id": format!(
                "online-portfolio-live-tail-cpu-accept-{}",
                request_rows
            ),
            "request_fingerprint": request_fingerprint,
            "exact_cache_key": exact_cache_key,
            "input_trace_path": input_trace_path,
            "event_timestamp": event_timestamp,
            "trace_id": trace_id,
            "external_provider_correlation_keys": external_provider_correlation_keys,
            "provider_correlation_ready": provider_correlation_ready,
            "match_keys": match_keys,
            "bucket_key": json_string(&row, &["bucket_key"]).unwrap_or_default(),
            "task_name": json_string(&row, &["task_name"]).unwrap_or_default(),
            "package_fingerprint64": json_u64(&row, &["package_fingerprint64"]).unwrap_or(0),
            "append_row_index": json_usize(json_at(&row, &["append_row_index"])).unwrap_or(0),
            "margin_micro": json_i64(&row, &["margin_micro"]).unwrap_or(0),
            "threshold_micro": json_i64(&row, &["threshold_micro"]).unwrap_or(0),
            "estimated_total_tokens": estimated_total_tokens,
            "current_total_cost_microusd": current_total_cost_microusd,
            "token_evidence_missing": json_bool(&row, &["token_cost", "token_evidence_missing"]).unwrap_or(estimated_total_tokens == 0),
            "cost_evidence_missing": json_bool(&row, &["token_cost", "cost_evidence_missing"]).unwrap_or(current_total_cost_microusd == 0),
            "unique_cpu_accept_over_exact_cache": true,
            "verified_safe_accept": true,
            "false_accept": false,
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "live-tail billing request export only: verified clean .nwpc shadow accepts are converted to provider billing match keys; no serving, local_accept, promotion, money claim, lookup, or legacy nwrb"
        });
        serde_json::to_writer(&mut writer, &request).map_err(|error| {
            format!(
                "failed to serialize live-tail billing request '{}': {error}",
                request_jsonl_path.display()
            )
        })?;
        writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write live-tail billing request '{}': {error}",
                request_jsonl_path.display()
            )
        })?;
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush live-tail billing request '{}': {error}",
            request_jsonl_path.display()
        )
    })?;

    let accept_parity = request_rows == source_accepts;
    let token_parity = total_tokens_requiring_billing == source_tokens;
    let cost_parity = current_known_cost_microusd == source_cost;
    let provider_correlation_parity =
        request_rows > 0 && external_provider_correlation_key_rows == request_rows;
    let request_ready = request_rows > 0
        && source_clean
        && accept_parity
        && token_parity
        && cost_parity
        && skipped_false_accept == 0
        && rejected_rows == 0;
    let ready_for_external_provider_evidence = request_ready && provider_correlation_parity;
    let verdict = if ready_for_external_provider_evidence {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_BILLING_REQUEST_V1_READY_FOR_EXTERNAL_EVIDENCE"
    } else if request_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_BILLING_REQUEST_V1_WATCH_PROVIDER_CORRELATION_MISSING"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_LIVE_TAIL_BILLING_REQUEST_V1_BLOCKED"
    };
    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_live_tail_billing_request_v1",
        "live_score_report_path": live_score_report_path,
        "decision_log_path": decision_log_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "source_clean": source_clean,
        "decision_rows": decision_rows,
        "candidate_rows": candidate_rows,
        "verifier_bound_rows": verifier_bound_rows,
        "rejected_rows": rejected_rows,
        "billing_request_rows": request_rows,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
        "external_provider_correlation_missing_rows": external_provider_correlation_missing_rows,
        "provider_correlation_parity": provider_correlation_parity,
        "ready_for_external_provider_evidence": ready_for_external_provider_evidence,
        "source_unique_cpu_accepts_over_exact_cache": source_accepts,
        "accept_parity": accept_parity,
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "source_tokens_saved": source_tokens,
        "token_parity": token_parity,
        "current_known_cost_microusd": current_known_cost_microusd,
        "source_cost_saved_microusd": source_cost,
        "cost_parity": cost_parity,
        "skipped_source_not_clean": skipped_source_not_clean,
        "skipped_non_candidate": skipped_non_candidate,
        "skipped_not_unique_over_exact_cache": skipped_not_unique_over_exact_cache,
        "skipped_false_accept": skipped_false_accept,
        "billing_gate": {
            "provider_billing_request_only": true,
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "policy": "live-tail request rows are exact keys for external provider billing evidence; this artifact is not evidence that money was saved"
        },
        "provider_correlation_gate": {
            "external_provider_correlation_key_rows": external_provider_correlation_key_rows,
            "external_provider_correlation_missing_rows": external_provider_correlation_missing_rows,
            "provider_correlation_parity": provider_correlation_parity,
            "ready_for_external_provider_evidence": ready_for_external_provider_evidence,
            "policy": "request rows can be joined only if the external provider export carries request_fingerprint, exact_cache_key, or external provider correlation keys; internal match keys alone are not provider billing evidence"
        },
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        },
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": false,
        "verdict": verdict,
        "boundary": "live-tail billing request export only: clean live .nwpc score-only accepts are converted to provider billing match keys; no serving, local_accept, promotion, money claim, lookup, or legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_live_tail_billing_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  billing_request_jsonl_path: {}",
        request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  accept_parity: {accept_parity}");
    println!("  token_parity: {token_parity}");
    println!("  ready_for_external_provider_evidence: {ready_for_external_provider_evidence}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn online_portfolio_verifier_binding_report() -> serde_json::Value {
    serde_json::json!({
        "verifier_id": verifier_binding_id("phase_stream_online_miner_verified_safe_accept_v1"),
        "verifier_version": 1u32,
        "verifier_input_kind_id": verifier_binding_id("phase_atom_binary_event"),
        "verifier_evidence_source_id": verifier_binding_id("real_traffic_phase_atom_trace_jsonl_verified_safe_accept"),
        "false_accept_threshold": 0usize,
        "policy": "binding metadata around selected quarantine .nwpc packages; package bytes stay phase-center runtime, promotion still requires admission, billing evidence, and local_accept=false"
    })
}

fn verifier_binding_id(label: &str) -> u32 {
    let id = stable_fingerprint([label]) as u32;
    if id == 0 { 1 } else { id }
}

#[allow(clippy::too_many_arguments)]
fn write_live_tail_score_only_snapshot(
    report_path: &Path,
    registry_path: &Path,
    append_tail_path: &Path,
    score_decision_log_path: &Path,
    registry: &Value,
    selected_bucket_count: usize,
    selected_bucket_package_count: usize,
    runtime_package_count: usize,
    append_rows: usize,
    events_with_verifier_label: usize,
    append_token_rows: usize,
    append_total_tokens: usize,
    append_total_cost_microusd: u64,
    append_exact_cache_hits: usize,
    append_exact_cache_tokens: usize,
    append_exact_cache_cost_microusd: u64,
    append_matching_bucket_events: usize,
    append_score_candidate_rows: usize,
    verifier_bound_score_candidate_rows: usize,
    verifier_rejected_score_candidate_rows: usize,
    append_false_accepts: usize,
    append_unique_cpu_accepts_over_exact_cache: usize,
    append_tokens_saved: usize,
    append_cost_saved_microusd: u64,
    missing_package_rows: usize,
    hot_margin_parity_mismatches: usize,
    hot_decision_parity_mismatches: usize,
    score_latencies_ns: &[u64],
    snapshot_in_progress: bool,
    verdict: &str,
) -> Result<(), String> {
    let mut sorted_latencies = score_latencies_ns.to_vec();
    sorted_latencies.sort_unstable();
    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_live_tail_score_only_v1"),
    );
    report.insert(
        "mode".to_owned(),
        serde_json::json!("product_hot_score_only_live_tail_shadow"),
    );
    report.insert(
        "snapshot_in_progress".to_owned(),
        serde_json::json!(snapshot_in_progress),
    );
    report.insert("registry_path".to_owned(), serde_json::json!(registry_path));
    report.insert(
        "append_tail_path".to_owned(),
        serde_json::json!(append_tail_path),
    );
    report.insert(
        "score_decision_log_path".to_owned(),
        serde_json::json!(score_decision_log_path),
    );
    report.insert(
        "registry_kind".to_owned(),
        serde_json::json!(json_string(registry, &["registry_kind"]).unwrap_or_default()),
    );
    report.insert(
        "source_report_path".to_owned(),
        serde_json::json!(json_string(registry, &["source_report_path"]).unwrap_or_default()),
    );
    report.insert(
        "selected_bucket_count".to_owned(),
        serde_json::json!(selected_bucket_count),
    );
    report.insert(
        "selected_bucket_package_count".to_owned(),
        serde_json::json!(selected_bucket_package_count),
    );
    report.insert(
        "runtime_package_count".to_owned(),
        serde_json::json!(runtime_package_count),
    );
    report.insert("append_rows".to_owned(), serde_json::json!(append_rows));
    report.insert(
        "events_with_verifier_label".to_owned(),
        serde_json::json!(events_with_verifier_label),
    );
    report.insert(
        "append_token_rows".to_owned(),
        serde_json::json!(append_token_rows),
    );
    report.insert(
        "append_total_tokens".to_owned(),
        serde_json::json!(append_total_tokens),
    );
    report.insert(
        "append_total_cost_microusd".to_owned(),
        serde_json::json!(append_total_cost_microusd),
    );
    report.insert(
        "append_exact_cache_hits".to_owned(),
        serde_json::json!(append_exact_cache_hits),
    );
    report.insert(
        "append_exact_cache_tokens".to_owned(),
        serde_json::json!(append_exact_cache_tokens),
    );
    report.insert(
        "append_exact_cache_cost_microusd".to_owned(),
        serde_json::json!(append_exact_cache_cost_microusd),
    );
    report.insert(
        "append_matching_bucket_events".to_owned(),
        serde_json::json!(append_matching_bucket_events),
    );
    report.insert(
        "append_score_candidate_rows".to_owned(),
        serde_json::json!(append_score_candidate_rows),
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
        "append_false_accepts".to_owned(),
        serde_json::json!(append_false_accepts),
    );
    report.insert(
        "append_unique_cpu_accepts_over_exact_cache".to_owned(),
        serde_json::json!(append_unique_cpu_accepts_over_exact_cache),
    );
    report.insert(
        "append_tokens_saved".to_owned(),
        serde_json::json!(append_tokens_saved),
    );
    report.insert(
        "append_cost_saved_microusd".to_owned(),
        serde_json::json!(append_cost_saved_microusd),
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
        "score_latency_p50_ns".to_owned(),
        serde_json::json!(percentile_u64(&sorted_latencies, 50)),
    );
    report.insert(
        "score_latency_p90_ns".to_owned(),
        serde_json::json!(percentile_u64(&sorted_latencies, 90)),
    );
    report.insert(
        "score_latency_p99_ns".to_owned(),
        serde_json::json!(percentile_u64(&sorted_latencies, 99)),
    );
    report.insert(
        "score_latency_max_ns".to_owned(),
        serde_json::json!(sorted_latencies.last().copied().unwrap_or(0)),
    );
    report.insert(
        "savings_over_exact_cache".to_owned(),
        serde_json::json!({
            "calls_saved": append_unique_cpu_accepts_over_exact_cache,
            "calls_saved_milli": ratio_milli(append_unique_cpu_accepts_over_exact_cache, append_rows),
            "tokens_saved": append_tokens_saved,
            "tokens_saved_milli": ratio_milli(append_tokens_saved, append_total_tokens),
            "estimated_cost_saved_microusd": append_cost_saved_microusd,
            "estimated_cost_saved_milli": ratio_milli_u64(append_cost_saved_microusd, append_total_cost_microusd),
            "market_money_claim_allowed": false
        }),
    );
    report.insert(
        "exact_cache_baseline".to_owned(),
        serde_json::json!({
            "calls_saved": append_exact_cache_hits,
            "calls_saved_milli": ratio_milli(append_exact_cache_hits, append_rows),
            "tokens_saved": append_exact_cache_tokens,
            "tokens_saved_milli": ratio_milli(append_exact_cache_tokens, append_total_tokens),
            "estimated_cost_saved_microusd": append_exact_cache_cost_microusd,
            "estimated_cost_saved_milli": ratio_milli_u64(append_exact_cache_cost_microusd, append_total_cost_microusd)
        }),
    );
    report.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
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
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        }),
    );
    report.insert("verdict".to_owned(), serde_json::json!(verdict));
    report.insert(
        "boundary".to_owned(),
        serde_json::json!("score-only product-hot live tail snapshot: starts at EOF, loads already promoted quarantine .nwpc packages, scores appended verifier-labeled events, does not mine, compile, change thresholds, promote, enable local_accept, claim market money, use lookup, or revive legacy nwrb"),
    );
    let report = Value::Object(report);
    write_json_file(report_path, &report)
}

fn product_hot_registry_entries_from_report(
    registry: &Value,
) -> Result<BTreeMap<String, ProductHotRegistryEntry>, String> {
    let mut entries = BTreeMap::new();
    for candidate in registry
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "product-hot registry missing candidates".to_owned())?
    {
        let bucket_key = json_string(candidate, &["bucket_key"])
            .ok_or_else(|| "product-hot candidate missing bucket_key".to_owned())?;
        let promotion_gate_passed =
            json_bool(candidate, &["promotion_gate_passed"]).unwrap_or(false);
        let false_accepts =
            json_usize(json_at(candidate, &["false_accepts"])).unwrap_or(usize::MAX);
        let verifier_bound = json_bool(candidate, &["verifier_bound"]).unwrap_or(false);
        let quarantine_nwpc = json_bool(candidate, &["quarantine_nwpc"]).unwrap_or(false);
        let local_accept_enabled = json_bool(candidate, &["local_accept_enabled"]).unwrap_or(true);
        if !promotion_gate_passed
            || false_accepts != 0
            || !verifier_bound
            || !quarantine_nwpc
            || local_accept_enabled
        {
            continue;
        }
        let package_path =
            PathBuf::from(json_string(candidate, &["package_path"]).ok_or_else(|| {
                format!("product-hot candidate '{bucket_key}' missing package_path")
            })?);
        let package_fingerprint64 =
            json_u64(candidate, &["package_fingerprint64"]).ok_or_else(|| {
                format!("product-hot candidate '{bucket_key}' missing package_fingerprint64")
            })?;
        let threshold_micro = json_i64(candidate, &["auto_calibrated_margin_threshold_micro"])
            .or_else(|| json_i64(candidate, &["safe_accept_margin_threshold_micro"]))
            .unwrap_or(1)
            .max(1);
        let action_family_atom =
            json_string(candidate, &["action_family_atom"]).unwrap_or_else(|| {
                bucket_key
                    .split("::")
                    .next()
                    .unwrap_or("action_family:unknown")
                    .to_owned()
            });
        let task_name = json_string(candidate, &["task_name"]).unwrap_or_default();
        let calibration_events = json_usize(json_at(candidate, &["auto_calibration_events"]))
            .or_else(|| json_usize(json_at(candidate, &["calibration_events"])))
            .unwrap_or(0);
        let selected_bucket = SelectedBucket {
            bucket_key: bucket_key.clone(),
            action_family_atom,
            task_name,
            threshold_micro,
            calibration_events,
            runtime_replay_start_event_ordinal: 0,
        };
        entries.insert(
            bucket_key,
            ProductHotRegistryEntry {
                selected_bucket,
                package_path,
                package_fingerprint64,
            },
        );
    }
    Ok(entries)
}

fn selected_buckets_from_report(
    selector: &Value,
) -> Result<BTreeMap<String, SelectedBucket>, String> {
    let mut buckets = BTreeMap::new();
    for row in selector
        .get("selected_buckets")
        .and_then(Value::as_array)
        .ok_or_else(|| "selector report missing selected_buckets".to_owned())?
    {
        let bucket_key = json_string(row, &["bucket_key"])
            .ok_or_else(|| "selected bucket missing bucket_key".to_owned())?;
        buckets.insert(
            bucket_key.clone(),
            SelectedBucket {
                action_family_atom: json_string(row, &["action_family_atom"]).unwrap_or_else(
                    || {
                        bucket_key
                            .split("::")
                            .next()
                            .unwrap_or("action_family:unknown")
                            .to_owned()
                    },
                ),
                bucket_key,
                task_name: json_string(row, &["task_name"]).unwrap_or_default(),
                threshold_micro: json_i64(row, &["threshold_micro"]).unwrap_or(1).max(1),
                calibration_events: json_usize(json_at(row, &["calibration_events"])).unwrap_or(0),
                runtime_replay_start_event_ordinal: json_usize(json_at(
                    row,
                    &["runtime_replay_start_event_ordinal"],
                ))
                .unwrap_or_else(|| {
                    let calibration_events =
                        json_usize(json_at(row, &["calibration_events"])).unwrap_or(0);
                    let policy_event_count =
                        json_usize(json_at(row, &["policy_event_count"])).unwrap_or(0);
                    calibration_events.saturating_add(policy_event_count)
                }),
            },
        );
    }
    Ok(buckets)
}

fn online_portfolio_matching_selected_bucket_keys(
    action_family: &str,
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    route_hint_atoms: &[String],
    bucket_specs: &[(&'static str, String)],
    selected_buckets: &BTreeMap<String, SelectedBucket>,
) -> Vec<String> {
    let mut matched = bucket_specs
        .iter()
        .filter_map(|(_, bucket_key)| {
            selected_buckets
                .contains_key(bucket_key)
                .then(|| bucket_key.clone())
        })
        .collect::<BTreeSet<_>>();
    for selected_bucket in selected_buckets.values() {
        if online_portfolio_selected_bucket_matches_row(
            selected_bucket,
            action_family,
            request_atoms,
            state_atoms,
            tool_atoms,
            route_hint_atoms,
        ) {
            matched.insert(selected_bucket.bucket_key.clone());
        }
    }
    matched.into_iter().collect()
}

fn online_portfolio_selected_bucket_matches_row(
    selected_bucket: &SelectedBucket,
    action_family: &str,
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    route_hint_atoms: &[String],
) -> bool {
    if selected_bucket.action_family_atom != action_family {
        return false;
    }
    let Some(suffix) = selected_bucket
        .bucket_key
        .strip_prefix(&format!("{action_family}::"))
    else {
        return false;
    };
    if suffix == "broad_action" {
        return true;
    }
    if selected_bucket.bucket_key
        == phase_atom_state_action_bucket_key(
            action_family,
            request_atoms,
            state_atoms,
            tool_atoms,
            route_hint_atoms,
        )
    {
        return true;
    }
    suffix
        .strip_prefix("auto_subcenter:")
        .or_else(|| suffix.strip_prefix("learned_auto_subcenter:"))
        .is_some_and(|split_atom| {
            online_portfolio_split_atom_matches_row(
                split_atom,
                request_atoms,
                state_atoms,
                tool_atoms,
                route_hint_atoms,
            )
        })
}

fn online_portfolio_split_atom_matches_row(
    split_atom: &str,
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    route_hint_atoms: &[String],
) -> bool {
    let atoms = request_atoms
        .iter()
        .chain(state_atoms)
        .chain(tool_atoms)
        .chain(route_hint_atoms)
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if atoms.contains(split_atom) {
        return true;
    }
    let Some((prefix, rest)) = split_atom.split_once(':') else {
        return false;
    };
    if !matches!(prefix, "multi2" | "multi3" | "multi4") {
        return false;
    }
    let expected_parts = match prefix {
        "multi2" => 2,
        "multi3" => 3,
        "multi4" => 4,
        _ => 0,
    };
    let parts = rest.split('|').collect::<Vec<_>>();
    parts.len() == expected_parts && parts.iter().all(|part| atoms.contains(part))
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

fn selected_decisions_from_log(
    decision_log_path: &Path,
    selected_bucket_keys: &BTreeSet<String>,
) -> Result<Vec<SelectedDecision>, String> {
    let text = std::fs::read_to_string(decision_log_path).map_err(|error| {
        format!(
            "failed to read selected portfolio decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let mut decisions = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse selected portfolio decision log '{}' line {}: {error}",
                decision_log_path.display(),
                line_index + 1
            )
        })?;
        let Some(bucket_key) = json_string(&row, &["bucket_key"]) else {
            continue;
        };
        if !selected_bucket_keys.contains(&bucket_key) {
            continue;
        }
        decisions.push(SelectedDecision {
            bucket_key,
            request_fingerprint: json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("decision-line:{}", line_index + 1)),
            external_provider_correlation_keys: external_provider_correlation_keys(&row),
            exact_cache_hit: json_bool(&row, &["exact_cache_hit"]).unwrap_or(false),
            verified_safe_accept: json_bool(&row, &["verified_safe_accept"]).unwrap_or(false),
            denominator_row_index: json_usize(json_at(&row, &["denominator_row_index"]))
                .unwrap_or(line_index + 1),
            margin_micro: json_i64(&row, &["margin_micro"]).unwrap_or(0),
            package_fingerprint64: json_u64(&row, &["package_fingerprint64"]).unwrap_or(0),
        });
    }
    Ok(decisions)
}

fn replay_events_from_traces(
    trace_paths: &[PathBuf],
    selected_buckets: &BTreeMap<String, SelectedBucket>,
    selected_decisions: &[SelectedDecision],
) -> Result<ReplayTraceSet, String> {
    let mut events = BTreeMap::new();
    let mut denominator = ReplayTraceDenominator::default();
    let mut parsed_events = 0usize;
    let mut wanted = BTreeMap::<(usize, String), Vec<(String, String, String)>>::new();
    for decision in selected_decisions {
        if let Some(bucket) = selected_buckets.get(&decision.bucket_key) {
            wanted
                .entry((
                    decision.denominator_row_index,
                    decision.request_fingerprint.clone(),
                ))
                .or_default()
                .push((
                    decision.bucket_key.clone(),
                    bucket.action_family_atom.clone(),
                    bucket.task_name.clone(),
                ));
        }
    }
    for trace_path in trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read portfolio replay trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            denominator.total_rows += 1;
            let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse portfolio replay trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            let total_tokens =
                json_usize(json_at(&row, &["token_cost", "total_tokens"])).unwrap_or(0);
            let estimated_cost =
                json_u64(&row, &["token_cost", "total_cost_microusd"]).unwrap_or(0);
            if total_tokens > 0 {
                denominator.token_rows += 1;
                denominator.total_tokens = denominator.total_tokens.saturating_add(total_tokens);
            }
            denominator.estimated_total_cost_microusd = denominator
                .estimated_total_cost_microusd
                .saturating_add(estimated_cost);
            if json_bool(&row, &["exact_cache_hit"]).unwrap_or(false) {
                denominator.exact_cache_hits += 1;
                denominator.exact_cache_tokens =
                    denominator.exact_cache_tokens.saturating_add(total_tokens);
                denominator.exact_cache_estimated_cost_microusd = denominator
                    .exact_cache_estimated_cost_microusd
                    .saturating_add(estimated_cost);
            }
            if row
                .get("verified_safe_accept")
                .and_then(Value::as_bool)
                .is_none()
            {
                continue;
            }
            let request_fingerprint = json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("portfolio-replay-row:{}", denominator.total_rows));
            if let Some(wanted_rows) =
                wanted.get(&(denominator.total_rows, request_fingerprint.clone()))
            {
                for (bucket_key, action_family, task_name) in wanted_rows {
                    let Some(_selected_bucket) = selected_buckets.get(bucket_key) else {
                        continue;
                    };
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
                            denominator.total_rows,
                            bucket_key.clone(),
                            event.request_fingerprint.clone(),
                        ),
                        ReplayEvent { event },
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
                    let Some(selected_bucket) = selected_buckets.get(&bucket_key) else {
                        continue;
                    };
                    let Some(event) = parse_phase_atom_binary_event_for_action(
                        &row,
                        parsed_events,
                        &action_family,
                        &selected_bucket.task_name,
                    ) else {
                        continue;
                    };
                    parsed_events += 1;
                    events.insert(
                        (
                            denominator.total_rows,
                            selected_bucket.bucket_key.clone(),
                            event.request_fingerprint.clone(),
                        ),
                        ReplayEvent { event },
                    );
                }
            }
        }
    }
    Ok(ReplayTraceSet {
        events,
        denominator,
    })
}

fn load_runtime_entry(
    package_path: &Path,
    threshold_micro: i64,
    expected_fingerprint64: u64,
) -> Result<ReplayRuntimeEntry, String> {
    let package_bytes = std::fs::read(package_path).map_err(|error| {
        format!(
            "failed to read portfolio replay package '{}': {error}",
            package_path.display()
        )
    })?;
    let package_info =
        PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).map_err(|error| {
            format!(
                "failed to inspect portfolio replay package '{}': {error:?}",
                package_path.display()
            )
        })?;
    if expected_fingerprint64 != 0 && package_info.fingerprint64 != expected_fingerprint64 {
        return Err(format!(
            "portfolio replay package fingerprint mismatch '{}': expected {}, got {}",
            package_path.display(),
            expected_fingerprint64,
            package_info.fingerprint64
        ));
    }
    let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        PhaseCenterOffloadPolicy::new(threshold_micro)
            .map_err(|error| format!("invalid replay policy: {error:?}"))?,
    )
    .map_err(|error| format!("failed to load replay runtime: {error:?}"))?;
    let profile_ids = (0..runtime.record_count() as u32).collect::<Vec<_>>();
    let thresholds = vec![threshold_micro; runtime.record_count()];
    let hot_runtime =
        PhaseCenterHotRuntime::from_flat_runtime(runtime.runtime(), &profile_ids, &thresholds)
            .map_err(|error| format!("failed to build replay hot runtime: {error:?}"))?;
    let route_plan = hot_runtime
        .route_plan_from_profile_ids(0, [0u32])
        .map_err(|error| format!("failed to build replay route plan: {error:?}"))?
        .ok_or_else(|| "replay route plan has no profiles".to_owned())?;
    let hot_routes = PhaseCenterHotRouteTable::from_plans([route_plan])
        .map_err(|error| format!("failed to build replay route table: {error:?}"))?;
    let hot_scratch = PhaseCenterHotScratch::new(runtime.cells(), runtime.record_count())
        .map_err(|error| format!("failed to build replay scratch: {error:?}"))?;
    Ok(ReplayRuntimeEntry {
        runtime,
        hot_runtime,
        hot_routes,
        hot_scratch,
    })
}

fn increment_json_usize(row: &mut Value, key: &str, by: usize) -> Result<(), String> {
    let Some(object) = row.as_object_mut() else {
        return Err("cannot increment non-object JSON row".to_owned());
    };
    let current = object
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0);
    object.insert(
        key.to_owned(),
        serde_json::json!(current.saturating_add(by)),
    );
    Ok(())
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

fn local_json_string_vec(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn external_provider_correlation_keys(row: &Value) -> Vec<String> {
    let mut keys = super::phase_atom_external_provider_correlation_keys(row);
    keys.sort();
    keys.dedup();
    keys
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

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
    })
}

fn json_usize(value: Option<&Value>) -> Option<usize> {
    value
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
        })
        .and_then(|value| usize::try_from(value).ok())
}

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn ratio_milli_u64(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn duration_nanos_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn percentile_u64(sorted_values: &[u64], percentile: usize) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    let index = sorted_values
        .len()
        .saturating_mul(percentile)
        .checked_div(100)
        .unwrap_or(0)
        .min(sorted_values.len().saturating_sub(1));
    sorted_values[index]
}
