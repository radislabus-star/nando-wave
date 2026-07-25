mod client_connections;
mod f5_runtime_status;
mod live_dashboard;
mod signal_map;
mod signal_path_api;

use axum::extract::{Form, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::{Request, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use nando_gateway_control::{
    ControlConfig, GatewayMode, admission_status, apply_mode, read_state, reconcile_startup,
    service_statuses,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LIVE_MINER_REPORT_URL: &str = "http://127.0.0.1:18789/v2/miner/report";
const HOT_SERVING_HEALTH_URL: &str = "http://127.0.0.1:18789/health/bridge";
const HOT_SERVING_RUNTIME_HEALTH_URL: &str = "http://127.0.0.1:18789/health";
const COLD_LEARNING_HEALTH_URL: &str = "http://127.0.0.1:18790/health/bridge";
const LIVE_STATUS_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone)]
struct AppState {
    config: Arc<ControlConfig>,
}

#[derive(Deserialize)]
struct ModeForm {
    mode: String,
}

#[tokio::main]
async fn main() {
    let config = match ControlConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("nando-gateway-control: {error}");
            std::process::exit(2);
        }
    };
    let bind = config.bind.clone();
    if let Err(error) = reconcile_startup(&config) {
        eprintln!("nando-gateway-control: startup reconciliation failed: {error}");
    }
    let state = AppState {
        config: Arc::new(config),
    };
    tokio::spawn(cpu_safety_watchdog(state.clone()));
    let app = Router::new()
        .route("/health", get(health))
        .route("/control/:key", get(control_page))
        .route(
            "/control/:key/api/v1/signal-path",
            get(signal_path_api::control_signal_path),
        )
        .route("/control/:key/tokens", get(control_token_stats))
        .route("/control/:key/connections", get(control_client_connections))
        .route("/control/:key/state", get(control_state))
        .route("/control/:key/mode", post(change_mode))
        .fallback(not_found)
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("nando-gateway-control: cannot bind {bind}: {error}");
            std::process::exit(2);
        }
    };
    if let Err(error) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("nando-gateway-control: server failed: {error}");
        std::process::exit(1);
    }
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mode = read_state(&state.config.state_path).mode;
    let admission = admission_status(&state.config);
    Json(json!({
        "ok": true,
        "service": "nando-gateway-control",
        "mode": mode,
        "cpu_allowed": admission.cpu_allowed,
        "transport_dependency": false
    }))
}

async fn control_page(Path(key): Path<String>, State(state): State<AppState>) -> Response {
    if !authorized(&state, &key) {
        return not_found().await;
    }
    let current = read_state(&state.config.state_path);
    let admission = admission_status(&state.config);
    let cpu_disabled = if admission.cpu_allowed {
        ""
    } else {
        " disabled"
    };
    let economics = read_json(&state.config.economics_path);
    let response_registry = read_json(&state.config.response_registry_path);
    let response_admission_controller =
        read_json(&state.config.response_admission_controller_report_path);
    let online_miner = read_json(&state.config.response_online_miner_report_path);
    let live_miner = read_live_miner_report().await;
    let response_runtime_health = read_live_json(HOT_SERVING_RUNTIME_HEALTH_URL).await;
    let live_economics = live_miner
        .get("economics")
        .filter(|value| value.is_object())
        .unwrap_or(&economics);
    let (visible_total_tokens, visible_cpu_tokens) =
        exact_token_totals(live_economics).unwrap_or((0, 0));
    let (current_epoch_total_tokens, current_epoch_cpu_tokens) =
        exact_current_epoch_token_totals(live_economics).unwrap_or((0, 0));
    let build_manifest = read_json(&state.config.build_manifest_path);
    let admission_receipt = read_json(&state.config.admission_path);
    let runtime_admission = admission_receipt
        .pointer("/sections/runtime_admission")
        .unwrap_or(&Value::Null);
    let services = service_statuses();
    let service_rows = services
        .iter()
        .map(|service| {
            format!(
                "<tr><td>{}</td><td class=\"{}\">{}</td></tr>",
                html_escape(service.unit),
                if service.active { "ok" } else { "off" },
                if service.active {
                    "РАБОТАЕТ"
                } else {
                    "ОСТАНОВЛЕН"
                }
            )
        })
        .collect::<String>();
    let active_profiles = metric_u64(runtime_admission, "active_profile_count");
    let avoided_calls = metric_u64(&economics, "avoided_calls");
    let tokens_saved = metric_u64(&economics, "avoided_input_tokens");
    let total_requests = metric_u64_any(
        &economics,
        &[
            "dedupe_eligible_request_events",
            "dedupe_eligible_client_intents",
        ],
    );
    let cpu_share = format_ratio_milli(metric_u64(&economics, "call_saving_share_milli"));
    let token_share = format_ratio_milli(metric_u64(&economics, "input_token_saving_share_milli"));
    let verification_coverage =
        format_ratio_milli(metric_u64(&economics, "verification_coverage_milli"));
    let economics_age = unix_now().saturating_sub(metric_u64(&economics, "generated_at_unix"));
    let economics_status = match economics.get("schema").and_then(Value::as_str) {
        Some(schema) if schema.starts_with("nando.economics-snapshot.v") => {
            if economics_age <= 120 {
                "СВЕЖИЙ"
            } else {
                "УСТАРЕЛ"
            }
        }
        _ => "ОТСУТСТВУЕТ",
    };
    let economics_age_text = format!("{} с", economics_age);
    let hard_gate = yes_no(
        economics
            .get("hard_gate_pass")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    let product_m1 = yes_no(
        economics
            .get("product_m1_pass")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    let eligible_intents = metric_u64_any(
        &economics,
        &[
            "dedupe_eligible_request_events",
            "dedupe_eligible_client_intents",
        ],
    );
    let excluded_intents = metric_u64_any(
        &economics,
        &[
            "dedupe_ineligible_request_events",
            "dedupe_ineligible_client_intents",
        ],
    );
    let global_input_tokens = metric_u64(&economics, "global_input_tokens");
    let actual_local_accepts = metric_u64(&economics, "actual_local_accepts");
    let verified_local_accepts = metric_u64(&economics, "verified_local_accepts");
    let unresolved_local = metric_u64(&economics, "unresolved_local_outcomes");
    let missing_receipts = metric_u64(&economics, "missing_evidence_receipts");
    let m1_intent_gap = 10_000_u64.saturating_sub(eligible_intents);
    let m1_avoided_gap = 100_u64.saturating_sub(avoided_calls);
    let response_packages = response_registry
        .get("packages")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let response_active = response_packages
        .iter()
        .filter(|package| package.get("state").and_then(Value::as_str) == Some("active"))
        .count() as u64;
    let active_response_package_ids = response_packages
        .iter()
        .filter(|package| package.get("state").and_then(Value::as_str) == Some("active"))
        .filter(|package| {
            package
                .pointer("/proof/adaptive_identification")
                .is_some_and(Value::is_object)
        })
        .filter_map(|package| package.get("package_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let response_natural_active = active_response_package_ids.len() as u64;
    let package_cpu_counters_valid = response_runtime_health
        .get("response_cpu_by_package_valid")
        .and_then(Value::as_bool)
        == Some(true);
    let (package_cpu_accepts, package_cpu_input_tokens) = response_runtime_health
        .get("response_cpu_by_package")
        .and_then(Value::as_object)
        .map(|counters| {
            active_response_package_ids
                .iter()
                .filter_map(|package_id| counters.get(*package_id))
                .fold((0_u64, 0_u64), |(accepts, tokens), package| {
                    (
                        accepts.saturating_add(metric_u64(package, "ordinary_accepts")),
                        tokens.saturating_add(metric_u64(package, "ordinary_input_tokens")),
                    )
                })
        })
        .unwrap_or((0, 0));
    let response_quarantine = response_packages
        .iter()
        .filter(|package| package.get("state").and_then(Value::as_str) == Some("quarantine"))
        .count() as u64;
    let active_response_package = response_packages
        .iter()
        .find(|package| package.get("state").and_then(Value::as_str) == Some("active"));
    let active_response_id = active_response_package
        .and_then(|package| package.get("package_id"))
        .and_then(Value::as_str)
        .unwrap_or("нет");
    let active_response_operation = active_response_package
        .and_then(|package| package.pointer("/program/operation/op"))
        .and_then(Value::as_str)
        .unwrap_or("нет");
    let active_response_function = active_response_package
        .and_then(|package| package.pointer("/program/operation/function_name"))
        .and_then(Value::as_str);
    let active_response_program = active_response_function.map_or_else(
        || active_response_operation.to_owned(),
        |function| format!("{active_response_operation}({function})"),
    );
    let active_response_proof = active_response_package
        .and_then(|package| package.get("proof"))
        .unwrap_or(&Value::Null);
    let active_response_progress = format!(
        "support {} / future {} / wrong {} / parity failures {}",
        metric_u64(active_response_proof, "support_rows"),
        metric_u64(active_response_proof, "future_rows"),
        metric_u64(active_response_proof, "wrong_accepts"),
        metric_u64(active_response_proof, "runtime_parity_failures")
    );
    let execution_p99 = format!(
        "{} мкс",
        metric_u64(runtime_admission, "active_execution_p99_ns") / 1_000
    );
    let package_bytes = metric_u64(runtime_admission, "package_bytes");
    let active_future_rows = metric_u64(runtime_admission, "active_future_rows");
    let shadow_executions = metric_u64(runtime_admission, "shadow_executions");
    let response_miner = live_miner
        .get("response")
        .filter(|value| value.is_object())
        .or_else(|| online_miner.get("miner"))
        .unwrap_or(&Value::Null);
    let collection_status = live_miner
        .pointer("/collection/status")
        .unwrap_or(&Value::Null);
    let worker_status = live_miner.get("worker").unwrap_or(&Value::Null);
    let provider_capture = live_miner.get("provider_capture").unwrap_or(&Value::Null);
    let operator_generation_shadow = live_miner
        .get("operator_generation_shadow")
        .unwrap_or(&Value::Null);
    let collection_outcomes = live_miner
        .pointer("/collection/candidate_outcomes")
        .unwrap_or(&Value::Null);
    let online_generated_at = live_miner
        .get("generated_at_unix")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            online_miner
                .get("generated_at_unix_ms")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                / 1_000
        });
    let online_age = unix_now().saturating_sub(online_generated_at);
    let online_status = if response_miner.is_object() {
        "READY"
    } else {
        "MISSING"
    };
    let self_training = response_miner
        .get("self_training_v2")
        .unwrap_or(&Value::Null);
    let online_discovery = self_training.get("discovery").unwrap_or(&Value::Null);
    let online_cegis = self_training.get("cegis").unwrap_or(&Value::Null);
    let online_opportunity = self_training.get("opportunity").unwrap_or(&Value::Null);
    let visible_miner_tokens = exact_miner_observed_tokens(&live_miner, &online_miner).unwrap_or(0);
    let strongest_generation = self_training
        .get("generations")
        .and_then(Value::as_array)
        .and_then(|generations| strongest_signal_generation(generations))
        .unwrap_or(&Value::Null);
    let live_scalar_shadow = response_miner
        .get("live_scalar_shadow")
        .unwrap_or(&Value::Null);
    let adaptive_identification_live = live_scalar_shadow.is_object()
        && metric_str(live_scalar_shadow, "identification_policy", "")
            == "adaptive_version_space_v1";
    let online_generation = format!(
        "support {} / future {} / sessions {} / wrong {} / parity {}",
        metric_u64(strongest_generation, "support_rows"),
        metric_u64(strongest_generation, "future_rows"),
        metric_u64(strongest_generation, "future_sessions"),
        metric_u64(strongest_generation, "wrong_future_rows"),
        metric_u64(strongest_generation, "runtime_parity_rows")
    );
    let signal_partition = metric_u64(strongest_generation, "partition_version");
    let signal_generation = metric_u64(strongest_generation, "generation");
    let signal_support = if adaptive_identification_live {
        metric_u64(live_scalar_shadow, "support_rows")
    } else {
        metric_u64(strongest_generation, "support_runtime_parity_rows")
    };
    let signal_physical_adapters = metric_u64(strongest_generation, "physical_adapter_count");
    let signal_matching = metric_u64(strongest_generation, "matching_runtime_parity_rows");
    let signal_matching_sessions =
        metric_u64(strongest_generation, "matching_runtime_parity_sessions");
    let signal_after_watermark = metric_u64(strongest_generation, "after_future_watermark_rows");
    let signal_independent = if adaptive_identification_live {
        metric_u64(live_scalar_shadow, "future_rows")
    } else {
        metric_u64(strongest_generation, "independent_future_rows")
    };
    let signal_consistent = metric_u64(strongest_generation, "program_consistent_future_rows");
    let signal_routed = metric_u64(strongest_generation, "routed_future_rows");
    let signal_future = if adaptive_identification_live {
        metric_u64(live_scalar_shadow, "future_rows")
    } else {
        metric_u64(strongest_generation, "future_rows")
    };
    let signal_blocker = if adaptive_identification_live {
        live_scalar_shadow
            .get("blockers")
            .and_then(Value::as_object)
            .and_then(|blockers| blockers.keys().next())
            .map(String::as_str)
            .unwrap_or("нет")
    } else {
        metric_str(strongest_generation, "blocker", "нет")
    };
    let signal_admission_verdict = metric_str(&response_admission_controller, "verdict", "MISSING");
    let signal_admission_blocker = metric_str(
        &response_admission_controller,
        "blocker",
        "controller_report_missing",
    );
    let signal_admission_blocker_stage = metric_str(
        &response_admission_controller,
        "blocker_stage",
        "controller_report",
    );
    let signal_admission_age_seconds = unix_now().saturating_sub(metric_u64(
        &response_admission_controller,
        "generated_at_unix",
    ));
    let online_transitions = metric_u64(self_training, "transitions_seen");
    let online_teacher_programs = teacher_programs_text(online_discovery);
    let online_candidates = metric_u64(response_miner, "candidate_bucket_count");
    let online_admission_ready = metric_u64(response_miner, "admission_ready_cohorts");
    let online_emitted = metric_u64(response_miner, "emitted_candidate_cohorts");
    let online_blocked = metric_u64(response_miner, "explicitly_blocked_cohorts");
    let online_admission_accounting = yes_no(
        response_miner
            .get("admission_accounting_complete")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    let online_admission_blockers = admission_blockers_text(response_miner);
    let online_warm_bytes = metric_u64(response_miner, "warm_bytes_estimate");
    let online_product_window = format!(
        "{} intents / {} tokens / {}",
        metric_u64(online_opportunity, "verified_intents"),
        metric_u64(online_opportunity, "verified_tokens"),
        format_ratio_milli(metric_u64(online_opportunity, "verified_token_share_milli"))
    );
    let worker_queue = format!(
        "{} / {}",
        metric_u64(worker_status, "queue_backlog_estimate"),
        metric_u64(worker_status, "queue_capacity")
    );
    let worker_processed = format!(
        "{} / {}",
        metric_u64(worker_status, "processed"),
        metric_u64(worker_status, "failed")
    );
    let worker_synthesis_latency = format!(
        "last {} / max {}",
        format_micros(metric_u64(worker_status, "synthesis_last_micros")),
        format_micros(metric_u64(worker_status, "synthesis_max_micros"))
    );
    let checkpoint_interval_seconds = match metric_u64(worker_status, "checkpoint_interval_seconds")
    {
        0 => 60,
        value => value,
    };
    let worker_checkpoint_policy = format!(
        "{} events / {} с",
        metric_u64(worker_status, "checkpoint_events"),
        checkpoint_interval_seconds
    );
    let worker_checkpoint_latency = format!(
        "{}; total {}",
        format_micros(metric_u64(worker_status, "checkpoint_last_micros")),
        metric_u64(worker_status, "checkpoints")
    );
    let collection_observations = metric_u64(collection_status, "observations_total");
    let collection_executable = metric_u64(collection_status, "accounted_executable_total");
    let collection_exact = metric_u64(collection_status, "exact_executable_observations_total");
    let collection_semantic =
        metric_u64(collection_status, "semantic_executable_observations_total");
    let collection_ambiguous = metric_u64(collection_status, "accounted_ambiguous_total");
    let collection_irreducible = metric_u64(collection_status, "accounted_irreducible_total");
    let collection_accounting = yes_no(
        collection_status
            .get("observation_accounting_complete")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    let collection_frozen = metric_u64(collection_status, "frozen_buckets_total");
    let collection_future_receipts = metric_u64(collection_status, "future_receipts_unique_total");
    let collection_rejected_wrong_candidates = metric_u64(collection_status, "wrong_accepts_total");
    let collection_revoked = metric_u64(collection_status, "revoked_candidates_total");
    let collection_quarantine = live_miner
        .pointer("/collection/quarantine_packages")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let collection_outcome_accounting = yes_no(
        collection_outcomes
            .get("outcome_identity_holds")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    let collection_candidate_progress = format!(
        "emitted {} / blocked {} / accounting {}",
        metric_u64(collection_outcomes, "emitted_candidates"),
        metric_u64(collection_outcomes, "explicitly_blocked_candidates"),
        collection_outcome_accounting
    );
    let collection_blockers = collection_blockers_text(collection_status);
    let opportunity_intents = metric_u64(online_opportunity, "ordinary_intents");
    let opportunity_tokens = metric_u64(online_opportunity, "ordinary_tokens");
    let upper_bound_tokens = metric_u64(
        online_opportunity,
        "optimistic_executable_upper_bound_tokens",
    );
    let upper_bound_share = if opportunity_tokens == 0 {
        "NOT EVALUATED".to_owned()
    } else {
        format_ratio_milli(metric_u64(
            online_opportunity,
            "optimistic_executable_upper_bound_share_milli",
        ))
    };
    let upper_bound_accounting = yes_no(
        online_opportunity
            .get("classification_identity_holds")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            && online_opportunity
                .get("upper_bound_identity_holds")
                .and_then(Value::as_bool)
                .unwrap_or(false),
    );
    let m3_upper_bound_reachable = if opportunity_tokens == 0 {
        "NOT EVALUATED"
    } else {
        yes_no(
            online_opportunity
                .get("m3_reachable_under_upper_bound")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )
    };
    let false_accepts = metric_u64(runtime_admission, "global_false_accepts");
    let parity_mismatches = metric_u64(runtime_admission, "global_runtime_parity_mismatches");
    let policy_version = metric_str(runtime_admission, "policy_version", "MISSING");
    let build_id = metric_str(&build_manifest, "build_id", "MISSING");
    let build_commit = metric_str(&build_manifest, "git_commit", "MISSING");
    let build_commit_short = build_commit.get(..12).unwrap_or(build_commit);
    let module_version_rows = module_version_rows(&build_manifest);
    let proof_summary = f5_runtime_status::proof_summary();
    let current_mode = current.mode.to_string();
    let signal_architecture = signal_map::render(
        &signal_map::LiveSignalView {
            mode: &current_mode,
            cpu_allowed: admission.cpu_allowed,
            partition: signal_partition,
            generation: signal_generation,
            transitions: online_transitions,
            support: signal_support,
            physical_adapters: signal_physical_adapters,
            matching: signal_matching,
            matching_sessions: signal_matching_sessions,
            after_watermark: signal_after_watermark,
            independent: signal_independent,
            consistent: signal_consistent,
            routed: signal_routed,
            future: signal_future,
            identification_policy: metric_str(
                live_scalar_shadow,
                "identification_policy",
                "legacy_fixed_control",
            ),
            candidate_freezes: metric_u64(live_scalar_shadow, "candidate_freezes"),
            transfer_proofs: metric_u64(live_scalar_shadow, "transfer_proofs"),
            support_frame_rejects: metric_u64(strongest_generation, "support_frame_rejects"),
            support_session_rejects: metric_u64(strongest_generation, "support_session_rejects"),
            support_intent_rejects: metric_u64(strongest_generation, "support_intent_rejects"),
            support_event_rejects: metric_u64(strongest_generation, "support_event_rejects"),
            program_mismatch_rejects: metric_u64(strongest_generation, "program_mismatch_rejects"),
            route_mismatch_rejects: metric_u64(strongest_generation, "route_mismatch_rejects"),
            blocker: signal_blocker,
            admission_verdict: signal_admission_verdict,
            admission_blocker: signal_admission_blocker,
            admission_blocker_stage: signal_admission_blocker_stage,
            admission_age_seconds: signal_admission_age_seconds,
            admission_relation_candidates: metric_u64(
                &response_admission_controller,
                "relation_candidates",
            ),
            admission_future_rows: metric_u64(
                &response_admission_controller,
                "relation_max_future_rows",
            ),
            admission_runtime_parity_cases: metric_u64(
                &response_admission_controller,
                "relation_max_runtime_parity_cases",
            ),
            active_packages: response_natural_active,
            package_cpu_counters_valid,
            package_cpu_accepts,
            package_cpu_input_tokens,
            active_transition_profiles: active_profiles,
            verified_local_accepts,
            call_saving_share_milli: metric_u64(&economics, "call_saving_share_milli"),
            input_token_saving_share_milli: metric_u64(
                &economics,
                "input_token_saving_share_milli",
            ),
            online_ready: online_status == "READY",
            capture_phase: metric_str(provider_capture, "phase", "missing"),
            capture_records: metric_u64(provider_capture, "records"),
            capture_captured: metric_u64(provider_capture, "captured"),
            capture_censored: metric_u64(provider_capture, "censored"),
            capture_publish_sequence: metric_u64(provider_capture, "publish_sequence"),
            capture_last_error: metric_str(provider_capture, "last_error", ""),
            shadow_phase: metric_str(operator_generation_shadow, "phase", "missing"),
            shadow_submitted: metric_u64(operator_generation_shadow, "submitted"),
            shadow_evaluated: metric_u64(operator_generation_shadow, "evaluated"),
            shadow_verified: metric_u64(operator_generation_shadow, "verified"),
            shadow_parity_mismatches: metric_u64(operator_generation_shadow, "parity_mismatches"),
        },
        &proof_summary,
        &build_manifest,
        &state.config.model_label,
    );
    let research_architecture = f5_runtime_status::panel_html();
    let live_dashboard = live_dashboard::render(live_dashboard::InitialMetrics {
        total_tokens: visible_total_tokens,
        miner_tokens: visible_miner_tokens,
        cpu_tokens: visible_cpu_tokens,
        current_epoch_total_tokens,
        current_epoch_cpu_tokens,
        verified_window_total_tokens: metric_u64(online_opportunity, "ordinary_tokens"),
        verified_window_cpu_tokens: metric_u64(online_opportunity, "verified_tokens"),
        optimistic_upper_bound_tokens: metric_u64(
            online_opportunity,
            "optimistic_executable_upper_bound_tokens",
        ),
        cpu_allowed: admission.cpu_allowed,
    });
    let body = format!(
        r#"<!doctype html>
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Nando Machine · Токены</title>
<style>
:root {{ color-scheme:dark; font-family:"DejaVu Sans Mono","Liberation Mono",ui-monospace,monospace; background:#111315; color:#d8dde2; }}
* {{ box-sizing:border-box; }}
body {{ margin:0; background:#111315; font-size:16px; line-height:1.5; }}
header {{ background:#080a0b; color:#eef1f3; padding:11px 20px; border-bottom:1px solid #4a5055; }}
.header-inner {{ width:min(1280px,100%); margin:0 auto; display:flex; justify-content:space-between; align-items:center; gap:20px; }}
.brand {{ display:flex; align-items:baseline; flex-wrap:wrap; gap:8px 14px; min-width:0; }}
h1 {{ margin:0; font-size:18px; letter-spacing:0; text-transform:uppercase; }}
.build {{ color:#7f8991; font-size:12px; overflow-wrap:anywhere; }}
.model-id {{ color:#8fd5ff; font-size:12px; font-weight:700; }}
.model-id::before {{ content:"МОДЕЛЬ "; color:#66727a; font-weight:400; }}
.mode-wrap {{ display:flex; align-items:center; gap:10px; }}
.mode-label {{ color:#7f8991; font-size:12px; text-transform:uppercase; }}
.mode {{ color:#66d98b; font-size:14px; font-weight:700; }}
main {{ width:min(1280px,100%); margin:0 auto; padding:14px 20px 28px; }}
.controls {{ display:flex; flex-wrap:wrap; gap:12px; margin:0 0 12px; }}
.controls form {{ margin:0; }}
button {{ min-height:0; padding:2px 0; border:0; border-radius:0; background:transparent; color:#aeb7bf; font:700 13px inherit; cursor:pointer; }}
button::before {{ content:"["; color:#59636b; }}
button::after {{ content:"]"; color:#59636b; }}
button:hover {{ color:#fff; background:transparent; }}
.bypass,.bypass:hover {{ color:#f46d65; background:transparent; }}
button:disabled {{ color:#535a60; cursor:not-allowed; }}
.metric-grid {{ display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); column-gap:32px; }}
.band {{ border-top:1px solid #353a3e; padding:12px 0; min-width:0; }}
h2 {{ margin:0 0 10px; color:#dfe5e9; font-size:14px; letter-spacing:0; text-transform:uppercase; }}
.architecture {{ padding-top:4px; }}
.architecture-head {{ display:flex; justify-content:space-between; align-items:flex-start; gap:24px; margin-bottom:8px; }}
.architecture-title h2 {{ margin-bottom:3px; color:#8ee6a8; font-size:15px; }}
.architecture-title p {{ margin:0; color:#707a82; font-size:12px; line-height:1.4; }}
.architecture-state {{ flex:0 0 auto; display:flex; align-items:center; gap:8px; }}
.signal-pipeline .architecture-state {{ flex-wrap:wrap; justify-content:flex-end; }}
.state-chip {{ color:#8b949b; font-size:12px; font-weight:700; white-space:nowrap; }}
.state-chip::before {{ content:"["; }} .state-chip::after {{ content:"]"; }}
.state-chip.live,.state-chip.work,.state-chip.pass {{ color:#66d98b; }}
.state-chip.proven {{ color:#82c7ff; }}
.state-chip.wait {{ color:#e0b35a; }}
.state-chip.block {{ color:#ff6b63; }}
.state-chip.locked {{ color:#70777d; }}
.architecture-meta {{ color:#707a82; font-size:12px; font-weight:700; }}
.flow-tree {{ padding:14px 16px; border:1px solid #3f464b; background:#080a0b; font-size:14px; line-height:1.45; overflow-x:auto; }}
.terminal-stage {{ min-width:620px; }}
.terminal-line {{ display:grid; grid-template-columns:22px 42px minmax(240px,1fr) minmax(70px,auto) minmax(74px,auto); align-items:baseline; gap:8px; min-height:19px; }}
.tree-glyph {{ color:#566068; white-space:pre; }}
.stage-index {{ color:#737e86; }}
.stage-title {{ color:#dce2e6; font-weight:700; }}
.stage-metric {{ color:#e5bd63; text-align:right; white-space:nowrap; }}
.terminal-stage.locked .stage-title,.terminal-stage.locked .stage-metric {{ color:#666e74; }}
.terminal-detail,.terminal-edge {{ min-width:620px; padding-left:72px; color:#647079; font-size:12px; white-space:nowrap; }}
.terminal-detail .tree-glyph,.terminal-edge .tree-glyph {{ display:inline-block; width:22px; margin-left:-72px; margin-right:50px; }}
.terminal-failure {{ min-width:620px; margin:3px 0; grid-template-columns:22px 180px minmax(260px,1fr) auto; color:#ff6b63; }}
.terminal-failure strong {{ color:#ff6b63; margin-right:8px; }}
.terminal-failure span {{ color:#ef938e; }}
.terminal-failure code {{ color:#b87f7b; margin-left:10px; }}
.terminal-rule {{ padding:8px 2px 0; color:#6f7980; font-size:12px; }}
.identity-line {{ display:flex; flex-wrap:wrap; gap:5px 22px; margin:0 0 10px; padding:8px 0; border-top:1px solid #293035; border-bottom:1px solid #293035; color:#8b969e; font-size:12px; }}
.identity-line b {{ color:#cbd4da; font-weight:700; }}
.current-blocker {{ display:grid; grid-template-columns:150px minmax(220px,auto) minmax(320px,1fr); gap:8px 16px; align-items:baseline; margin:0 0 10px; padding:10px 12px; border-left:3px solid #ff6b63; background:#151011; }}
.current-blocker .blocker-label {{ color:#ff6b63; font-size:12px; font-weight:700; }}
.current-blocker strong {{ color:#ffd1ce; font-size:14px; }}
.current-blocker p {{ margin:0; color:#b88d8a; font-size:12px; }}
.pipeline-legend {{ display:flex; flex-wrap:wrap; gap:5px 22px; margin-bottom:10px; color:#77828a; font-size:12px; line-height:1.45; }}
.pipeline-legend b {{ color:#cbd4da; }}
.pipeline-stack {{ border:1px solid #3f464b; background:#080a0b; }}
.pipeline-stage {{ position:relative; padding:13px 16px 12px; border-left:3px solid #4d565d; border-bottom:1px solid #272d31; }}
.pipeline-stage:last-child {{ border-bottom:0; }}
.pipeline-stage.live,.pipeline-stage.work {{ border-left-color:#58c87d; }}
.pipeline-stage.proven {{ border-left-color:#65aee7; }}
.pipeline-stage.wait {{ border-left-color:#d2a34b; }}
.pipeline-stage.block {{ border-left-color:#f05f58; background:#120d0e; }}
.pipeline-stage.locked {{ border-left-color:#565e64; background:#090b0c; }}
.pipeline-stage-head {{ display:grid; grid-template-columns:58px minmax(0,1fr) auto; align-items:start; gap:10px; }}
.pipeline-index {{ color:#8c979f; font-size:14px; font-weight:700; }}
.pipeline-stage h3 {{ margin:0; color:#dce3e7; font-size:15px; line-height:1.35; }}
.pipeline-stage.live h3,.pipeline-stage.work h3 {{ color:#a9e8bb; }}
.pipeline-stage.proven h3 {{ color:#add9f8; }}
.pipeline-stage.block h3 {{ color:#ffc0bc; }}
.pipeline-stage.locked h3 {{ color:#777f85; }}
.stage-owner {{ margin:3px 0 0; color:#7f8991; font-size:12px; overflow-wrap:anywhere; }}
.stage-owner b {{ color:#aeb8bf; }}
.pipeline-diagnostics {{ margin:10px 0 0 68px; font-size:13px; line-height:1.45; }}
.diagnostic-row {{ display:grid; grid-template-columns:62px minmax(0,1fr); gap:8px; padding:3px 0; border-top:1px dotted #252b2f; }}
.diagnostic-row:first-child {{ border-top:0; }}
.diagnostic-row dt {{ color:#667178; font-weight:700; }}
.diagnostic-row dd {{ min-width:0; margin:0; color:#aeb7bd; overflow-wrap:anywhere; }}
.diagnostic-row.live dt {{ color:#66d98b; }}
.diagnostic-row.proof dt {{ color:#82c7ff; }}
.diagnostic-row.diagnostic dt {{ color:#e0b35a; }}
.diagnostic-row.output dt {{ color:#b5c0c7; }}
.pipeline-stage.locked .diagnostic-row dd {{ color:#737b81; }}
.pipeline-handoff {{ display:grid; grid-template-columns:72px minmax(0,1fr); align-items:center; min-height:34px; padding:0 16px; color:#69747b; font-size:12px; }}
.pipeline-handoff .handoff-line {{ justify-self:center; width:1px; height:34px; background:#3b444a; }}
.pipeline-handoff.live {{ color:#68bd83; }}
.pipeline-handoff.live .handoff-line {{ background:#3d8052; }}
.pipeline-handoff.proof {{ color:#6faedb; }}
.pipeline-handoff.proof .handoff-line {{ background:#356787; }}
.pipeline-break {{ display:grid; grid-template-columns:210px minmax(0,1fr); gap:5px 16px; padding:11px 16px 11px 84px; border-top:1px solid #722f2b; border-bottom:1px solid #722f2b; background:#1a0f10; color:#c98d89; font-size:13px; line-height:1.45; }}
.pipeline-break strong {{ color:#ff6b63; }}
.pipeline-break span:last-child {{ grid-column:2; color:#76b9e7; }}
.authority-boundary {{ display:grid; grid-template-columns:210px minmax(0,1fr); gap:16px; padding:12px 16px 12px 84px; border-top:1px solid #5e2927; border-bottom:1px solid #5e2927; background:#120d0e; color:#b78582; font-size:13px; line-height:1.45; }}
.authority-boundary strong {{ color:#ff6b63; }}
.proof-console {{ padding:4px 0 14px; border-bottom:1px solid #24292d; }}
.proof-console .research-architecture {{ margin-top:0; }}
.research-architecture {{ margin-top:16px; }}
.research-architecture .architecture-title h2 {{ color:#89bff2; }}
.research-architecture .architecture-state {{ flex-wrap:wrap; justify-content:flex-end; }}
.research-facts {{ display:flex; flex-wrap:wrap; gap:5px 18px; min-width:620px; padding:8px 0 10px 72px; border-top:1px dotted #30363a; color:#849099; font-size:12px; }}
.research-facts span::before {{ content:"· "; color:#4f5960; }}
.research-boundary {{ display:grid; grid-template-columns:22px minmax(300px,1fr) auto; gap:8px; min-width:620px; margin:4px 0; padding:8px 0; border-top:1px solid #315b3d; border-bottom:1px solid #315b3d; color:#66d98b; }}
.research-boundary strong {{ color:#8ee6a8; }}
.research-boundary span:last-child {{ color:#78838a; font-size:12px; white-space:nowrap; }}
.terminal-stage.not-started .stage-title,.terminal-stage.not-started .stage-metric,.terminal-stage.not-started .state-chip {{ color:#ff6b63; }}
.compact td {{ padding:5px 0; }}
table {{ width:100%; border-collapse:collapse; font-size:13px; }}
td {{ padding:5px 0; border-bottom:1px dotted #3a4044; overflow-wrap:anywhere; vertical-align:top; }}
td:first-child {{ padding-right:16px; }}
td:last-child {{ text-align:right; font-weight:700; }}
.ok {{ color:#66d98b; }} .off {{ color:#ff6b63; }}
.note {{ margin:0; color:#89939a; font-size:12px; line-height:1.5; overflow-wrap:anywhere; }}
.technical-console {{ margin-top:14px; border:1px solid #3f464b; background:#080a0b; }}
.technical-console summary {{ display:flex; align-items:center; gap:9px; min-height:44px; padding:11px 14px; color:#c9d0d5; cursor:pointer; font-size:14px; font-weight:700; list-style:none; user-select:none; }}
.technical-console summary::-webkit-details-marker {{ display:none; }}
.technical-console summary:hover {{ background:#0d1012; color:#fff; }}
.technical-toggle {{ width:13px; color:#66d98b; text-align:center; }}
.technical-toggle::before {{ content:"+"; }}
.technical-console[open] .technical-toggle::before {{ content:"-"; }}
.technical-summary-title {{ color:#dfe5e9; }}
.technical-summary-meta {{ margin-left:auto; color:#707a82; font-size:11px; font-weight:400; white-space:nowrap; }}
.advanced {{ padding:0 13px 13px; border-top:1px solid #353a3e; }}
.console-toolbar {{ display:flex; justify-content:space-between; gap:16px; padding:9px 0; border-bottom:1px solid #24292d; color:#667078; font-size:11px; }}
.console-path {{ color:#78c990; }}
.technical-layout {{ display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); column-gap:30px; }}
.console-panel {{ min-width:0; padding:12px 0 14px; border-bottom:1px solid #24292d; }}
.console-panel.wide {{ grid-column:1 / -1; }}
.console-command {{ display:flex; align-items:baseline; gap:8px; margin:0 0 8px; color:#d5dce0; font-size:14px; font-weight:700; }}
.console-prompt {{ color:#66d98b; }}
.console-arg {{ color:#8a949b; font-size:11px; font-weight:400; }}
.console-table {{ table-layout:fixed; font-size:13px; }}
.console-table td {{ padding:5px 0; border-bottom:1px dotted #30363a; }}
.console-table tr:last-child td {{ border-bottom:0; }}
.console-table td:first-child {{ width:46%; color:#89939a; }}
.console-table td:last-child {{ color:#d8dde2; }}
.console-panel.wide .console-table td:last-child {{ text-align:left; }}
@media (max-width:680px) {{
  .header-inner,.brand,.architecture-head {{ align-items:flex-start; flex-direction:column; }}
  .header-inner,.brand {{ gap:6px; }}
  .signal-pipeline .architecture-state {{ justify-content:flex-start; }}
  main {{ padding:12px 10px 24px; }}
  .metric-grid {{ grid-template-columns:1fr; }}
  .flow-tree {{ padding:10px; }}
  .terminal-stage,.terminal-detail,.terminal-edge,.terminal-failure {{ min-width:0; }}
  .terminal-stage .terminal-line {{ grid-template-columns:18px 34px minmax(0,1fr) auto; gap:4px; }}
  .terminal-stage .terminal-line .state-chip {{ grid-column:3 / 5; grid-row:2; justify-self:start; }}
  .terminal-detail,.terminal-edge {{ padding-left:56px; white-space:normal; overflow-wrap:anywhere; }}
  .terminal-detail .tree-glyph,.terminal-edge .tree-glyph {{ width:18px; margin-left:-56px; margin-right:38px; }}
  .terminal-failure {{ grid-template-columns:18px minmax(0,1fr); gap:3px 4px; }}
  .terminal-failure strong,.terminal-failure span,.terminal-failure code {{ grid-column:2; margin:0; }}
  .research-architecture .architecture-state {{ justify-content:flex-start; }}
  .research-architecture .terminal-stage .terminal-line {{ grid-template-columns:18px 34px minmax(0,1fr); }}
  .research-architecture .terminal-stage .stage-metric {{ grid-column:3; grid-row:2; justify-self:start; text-align:left; white-space:normal; overflow-wrap:anywhere; }}
  .research-architecture .terminal-stage .terminal-line .state-chip {{ grid-column:3; grid-row:3; }}
  .research-facts {{ min-width:0; padding-left:56px; gap:3px 12px; }}
  .research-boundary {{ grid-template-columns:18px minmax(0,1fr); min-width:0; }}
  .research-boundary strong,.research-boundary span:last-child {{ grid-column:2; white-space:normal; }}
  .technical-console summary {{ align-items:flex-start; flex-wrap:wrap; }}
  .technical-summary-meta {{ width:100%; margin-left:22px; }}
  .technical-layout {{ grid-template-columns:1fr; }}
  .console-panel.wide {{ grid-column:auto; }}
  .console-panel.wide .console-table tr {{ display:grid; grid-template-columns:minmax(0,1fr); padding:5px 0; border-bottom:1px dotted #30363a; }}
  .console-panel.wide .console-table td {{ width:100%; padding:0; border:0; text-align:left; }}
  .console-panel.wide .console-table td:first-child {{ padding-bottom:2px; }}
  .console-toolbar {{ align-items:flex-start; flex-direction:column; gap:2px; }}
  .identity-line {{ flex-direction:column; gap:3px; }}
  .current-blocker {{ grid-template-columns:1fr; gap:3px; }}
  .pipeline-legend {{ flex-direction:column; gap:3px; }}
  .pipeline-stage {{ padding:12px 10px 11px; }}
  .pipeline-stage-head {{ grid-template-columns:38px minmax(0,1fr); gap:7px; }}
  .pipeline-stage-head .state-chip {{ grid-column:2; justify-self:start; margin-top:4px; }}
  .pipeline-diagnostics {{ margin-left:45px; font-size:12px; }}
  .diagnostic-row {{ grid-template-columns:52px minmax(0,1fr); gap:5px; }}
  .pipeline-handoff {{ grid-template-columns:38px minmax(0,1fr); padding:0 10px; }}
  .pipeline-break,.authority-boundary {{ grid-template-columns:1fr; gap:4px; padding:10px 12px; }}
  .pipeline-break span:last-child {{ grid-column:1; }}
}}
</style>
</head>
<body>
{live_dashboard}
<div class="legacy-dashboard" hidden aria-hidden="true">
<header><div class="header-inner">
<div class="brand"><h1>Nando Machine</h1><span class="model-id">{model_label}</span><span class="build">развёрнуто {build_id} · {build_commit_short}</span></div>
<div class="mode-wrap"><span class="mode-label">Режим</span><span class="mode">{mode}</span></div>
</div></header>
<main>
<div class="controls">
<form method="post" action="/control/{key}/mode"><input type="hidden" name="mode" value="BYPASS"><button class="bypass">ОБХОД NANDO</button></form>
<form method="post" action="/control/{key}/mode"><input type="hidden" name="mode" value="SHADOW"><button>SHADOW</button></form>
<form method="post" action="/control/{key}/mode"><input type="hidden" name="mode" value="CPU"><button{cpu_disabled}>CPU</button></form>
</div>
{signal_architecture}
<div class="metric-grid compact">
<section class="band"><h2>Продуктовый результат</h2><table>
<tr><td>Обычный трафик на CPU</td><td>{cpu_share}</td></tr>
<tr><td>Экономия входных токенов</td><td>{token_share}</td></tr>
<tr><td>Предотвращено LLM-вызовов</td><td>{avoided_calls}</td></tr>
<tr><td>Активные / карантин</td><td>{response_active} / {response_quarantine}</td></tr>
</table></section>
<section class="band"><h2>Независимый допуск</h2><table>
<tr><td>Составной gate</td><td>{verdict}</td></tr>
<tr><td>Ложные допуски</td><td>{false_accepts}</td></tr>
<tr><td>Расхождения parity</td><td>{parity_mismatches}</td></tr>
<tr><td>Допуск выдан / заблокирован</td><td>{online_emitted} / {online_blocked}</td></tr>
</table></section>
</div>
<details id="technical-details" class="technical-console"><summary><span class="technical-toggle" aria-hidden="true"></span><span class="technical-summary-title">ДИАГНОСТИКА И ДОКАЗАТЕЛЬСТВА</span><span class="technical-summary-meta">ЖИВОЙ СНИМОК · ОБНОВЛЕНИЕ 15 С</span></summary><div class="advanced">
<div class="console-toolbar"><span class="console-path">nando://control/signal-map</span><span>модель {model_label} · сборка {build_id} · {build_commit_short}</span></div>
<div class="proof-console">{research_architecture}</div>
<div class="technical-layout">
<section class="console-panel"><h2 class="console-command"><span class="console-prompt">$</span> службы Nando</h2><table class="console-table">{service_rows}</table></section>
	<section class="console-panel"><h2 class="console-command"><span class="console-prompt">$</span> независимый допуск <span class="console-arg">строгая проверка</span></h2><table class="console-table">
<tr><td>Составной gate</td><td>{verdict}</td></tr>
<tr><td>Допуск по future</td><td>{eligible}</td></tr>
<tr><td>Свежесть evidence</td><td>{fresh}</td></tr>
<tr><td>Готовность runtime-маршрута</td><td>{route_ready}</td></tr>
	</table></section>
	<section class="console-panel wide"><h2 class="console-command"><span class="console-prompt">$</span> состояние майнера <span class="console-arg">живой поток + доказательства</span></h2><table class="console-table">
	<tr><td>Состояние / возраст снимка</td><td>{online_status} / {online_age} с</td></tr>
	<tr><td>Очередь / ёмкость</td><td>{worker_queue}</td></tr>
	<tr><td>Обработано / ошибок worker-процесса</td><td>{worker_processed}</td></tr>
	<tr><td>Задержка синтеза</td><td>{worker_synthesis_latency}</td></tr>
	<tr><td>Политика checkpoint</td><td>{worker_checkpoint_policy}</td></tr>
	<tr><td>Последний checkpoint / всего</td><td>{worker_checkpoint_latency}</td></tr>
	<tr><td>Teacher-переходы / пулы</td><td>{online_transitions} / {online_teacher_pools}</td></tr>
	<tr><td>Teacher-программы</td><td>{online_teacher_programs}</td></tr>
	<tr><td>CEGIS: когорты / победители / точные проверки</td><td>{online_cegis_cohorts} / {online_cegis_winners} / {online_exact_checks}</td></tr>
	<tr><td>Кандидаты / готовы к допуску</td><td>{online_candidates} / {online_admission_ready}</td></tr>
	<tr><td>Допуск выдан / заблокирован / учтено</td><td>{online_emitted} / {online_blocked} / {online_admission_accounting}</td></tr>
	<tr><td>Точный блокер допуска</td><td>{online_admission_blockers}</td></tr>
	<tr><td>Лучшее замороженное поколение</td><td>{online_generation}</td></tr>
	<tr><td>Горячее состояние</td><td>{online_warm_bytes} Б</td></tr>
	<tr><td>Ложные допуски / ошибки parity</td><td>{online_false_accepts} / {online_parity_failures}</td></tr>
	<tr><td>Текущее проверенное окно</td><td>{online_product_window}</td></tr>
	</table></section>
	<section class="console-panel"><h2 class="console-command"><span class="console-prompt">$</span> реестр операторов</h2><table class="console-table">
	<tr><td>Активные / карантин</td><td>{response_active} / {response_quarantine}</td></tr>
	<tr><td>Активный пакет</td><td>{active_response_id}</td></tr>
	<tr><td>Активная программа</td><td>{active_response_program}</td></tr>
	<tr><td>Доказательство активного пакета</td><td>{active_response_progress}</td></tr>
	<tr><td>Исполнение p99</td><td>{execution_p99}</td></tr>
	<tr><td>Размер пакетов переходов</td><td>{package_bytes} Б</td></tr>
	<tr><td>Чистые активные future-строки</td><td>{active_future_rows}</td></tr>
	<tr><td>Исполнения в SHADOW</td><td>{shadow_executions}</td></tr>
	</table></section>
	<section class="console-panel"><h2 class="console-command"><span class="console-prompt">$</span> синтез коллекции</h2><table class="console-table">
	<tr><td>Наблюдения</td><td>{collection_observations}</td></tr>
	<tr><td>Точные / семантические / учтённые исполнимые</td><td>{collection_exact} / {collection_semantic} / {collection_executable}</td></tr>
	<tr><td>Неоднозначные / неразложимые</td><td>{collection_ambiguous} / {collection_irreducible}</td></tr>
	<tr><td>Тождество учёта</td><td>{collection_accounting}</td></tr>
	<tr><td>Замороженные / карантин / future-квитанции</td><td>{collection_frozen} / {collection_quarantine} / {collection_future_receipts}</td></tr>
	<tr><td>Кандидат выдан / заблокирован / учтён</td><td>{collection_candidate_progress}</td></tr>
	<tr><td>Точные блокеры</td><td>{collection_blockers}</td></tr>
	<tr><td>Отозванные / ошибочные гипотезы</td><td>{collection_revoked} / {collection_rejected_wrong_candidates}</td></tr>
	</table></section>
	<section class="console-panel"><h2 class="console-command"><span class="console-prompt">$</span> экономика <span class="console-arg">проверенные данные</span></h2><table class="console-table">
	<tr><td>Активные профили переходов</td><td>{active_profiles}</td></tr>
	<tr><td>LLM-вызовов предотвращено</td><td>{avoided_calls}</td></tr>
	<tr><td>Входных токенов сэкономлено</td><td>{tokens_saved}</td></tr>
	<tr><td>Наблюдаемых запросов провайдера</td><td>{total_requests}</td></tr>
	<tr><td>Доля обычного трафика на CPU</td><td>{cpu_share}</td></tr>
	<tr><td>Экономия входных токенов</td><td>{token_share}</td></tr>
	<tr><td>Покрытие независимой проверкой</td><td>{verification_coverage}</td></tr>
	<tr><td>Снимок экономики</td><td>{economics_status}</td></tr>
	<tr><td>Ложные допуски</td><td>{false_accepts}</td></tr>
	<tr><td>Расхождения runtime parity</td><td>{parity_mismatches}</td></tr>
	<tr><td>Политика продвижения</td><td>{policy_version}</td></tr>
	</table></section>
	<section class="console-panel"><h2 class="console-command"><span class="console-prompt">$</span> знаменатель экономики <span class="console-arg">M1</span></h2><table class="console-table">
	<tr><td>Учитываемые / исключённые запросы</td><td>{eligible_intents} / {excluded_intents}</td></tr>
	<tr><td>Входные токены знаменателя</td><td>{global_input_tokens}</td></tr>
	<tr><td>Локальные / независимо проверенные</td><td>{actual_local_accepts} / {verified_local_accepts}</td></tr>
	<tr><td>Неразрешённые исходы / отсутствующие квитанции</td><td>{unresolved_local} / {missing_receipts}</td></tr>
	<tr><td>Жёсткий gate экономики</td><td>{hard_gate}</td></tr>
	<tr><td>Продуктовая метрика M1</td><td>{product_m1}</td></tr>
	<tr><td>До M1: запросов / предотвращённых вызовов</td><td>{m1_intent_gap} / {m1_avoided_gap}</td></tr>
	<tr><td>Возраст снимка экономики</td><td>{economics_age_text}</td></tr>
	</table></section>
	<section class="console-panel"><h2 class="console-command"><span class="console-prompt">$</span> верхняя граница возможностей <span class="console-arg">M3</span></h2><table class="console-table">
	<tr><td>Авторитетное окно</td><td>{opportunity_intents} запросов / {opportunity_tokens} токенов</td></tr>
	<tr><td>Оптимистичная исполнимая граница</td><td>{upper_bound_tokens} токенов / {upper_bound_share}</td></tr>
	<tr><td>Неразложимые / неразрешённые токены</td><td>{irreducible_tokens} / {unresolved_tokens}</td></tr>
	<tr><td>Учёт верхней границы</td><td>{upper_bound_accounting}</td></tr>
	<tr><td>M3 достижим в этом окне</td><td>{m3_upper_bound_reachable}</td></tr>
	</table></section>
	<section class="console-panel wide manifest"><h2 class="console-command"><span class="console-prompt">$</span> манифест сборки <span class="console-arg">модули</span></h2><table class="console-table">{module_version_rows}</table></section>
</div>
</div></details>
<section class="band"><h2>Граница</h2><p class="note">{reason}. Кнопка BYPASS останавливает Nando-наблюдение и майнер, но Nginx продолжает передавать Codex-трафик в OpenAI.</p></section>
</main>
<script>
(() => {{
  const details = document.getElementById("technical-details");
  if (!details) return;
  const stateKey = "nando-control-view-v1";
  let saved = {{}};
  try {{ saved = JSON.parse(sessionStorage.getItem(stateKey) || "{{}}"); }} catch (_) {{}}
  if (typeof saved.technicalOpen === "boolean") details.open = saved.technicalOpen;
  if ("scrollRestoration" in history) history.scrollRestoration = "manual";
  const persist = () => {{
    try {{
      sessionStorage.setItem(stateKey, JSON.stringify({{
        technicalOpen: details.open,
        scrollY: window.scrollY
      }}));
    }} catch (_) {{}}
  }};
  details.addEventListener("toggle", persist);
  window.addEventListener("pagehide", persist);
  if (Number.isFinite(saved.scrollY)) {{
    requestAnimationFrame(() => window.scrollTo(0, saved.scrollY));
  }}
}})();
</script>
</div>
</body>
</html>"#,
        mode = current.mode,
        model_label = html_escape(&state.config.model_label),
        live_dashboard = live_dashboard,
        service_rows = service_rows,
        build_id = html_escape(build_id),
        build_commit_short = html_escape(build_commit_short),
        key = html_escape(&key),
        cpu_disabled = cpu_disabled,
        signal_architecture = signal_architecture,
        research_architecture = research_architecture,
        module_version_rows = module_version_rows,
        verdict = html_escape(&admission.verdict),
        eligible = yes_no(admission.eligible_for_local_accept),
        fresh = yes_no(admission.fresh),
        route_ready = yes_no(admission.route_ready),
        reason = html_escape(&admission.reason),
        active_profiles = active_profiles,
        avoided_calls = avoided_calls,
        tokens_saved = tokens_saved,
        total_requests = total_requests,
        cpu_share = cpu_share,
        token_share = token_share,
        verification_coverage = verification_coverage,
        economics_status = economics_status,
        economics_age_text = economics_age_text,
        hard_gate = hard_gate,
        product_m1 = product_m1,
        eligible_intents = eligible_intents,
        excluded_intents = excluded_intents,
        global_input_tokens = global_input_tokens,
        actual_local_accepts = actual_local_accepts,
        verified_local_accepts = verified_local_accepts,
        unresolved_local = unresolved_local,
        missing_receipts = missing_receipts,
        m1_intent_gap = m1_intent_gap,
        m1_avoided_gap = m1_avoided_gap,
        response_active = response_active,
        response_quarantine = response_quarantine,
        active_response_id = html_escape(active_response_id),
        active_response_program = html_escape(&active_response_program),
        active_response_progress = html_escape(&active_response_progress),
        execution_p99 = execution_p99,
        package_bytes = package_bytes,
        active_future_rows = active_future_rows,
        shadow_executions = shadow_executions,
        online_status = online_status,
        online_age = online_age,
        online_transitions = online_transitions,
        online_teacher_pools = metric_u64(online_discovery, "teacher_pool_count"),
        online_teacher_programs = html_escape(&online_teacher_programs),
        online_cegis_cohorts = metric_u64(online_cegis, "cohorts"),
        online_cegis_winners = metric_u64(online_cegis, "winners"),
        online_exact_checks = metric_u64(online_cegis, "exact_checks"),
        online_candidates = online_candidates,
        online_admission_ready = online_admission_ready,
        online_emitted = online_emitted,
        online_blocked = online_blocked,
        online_admission_accounting = online_admission_accounting,
        online_admission_blockers = html_escape(&online_admission_blockers),
        online_generation = html_escape(&online_generation),
        online_warm_bytes = online_warm_bytes,
        online_false_accepts = metric_u64(online_opportunity, "false_accepts"),
        online_parity_failures = metric_u64(online_opportunity, "parity_failures"),
        online_product_window = html_escape(&online_product_window),
        worker_queue = html_escape(&worker_queue),
        worker_processed = html_escape(&worker_processed),
        worker_synthesis_latency = html_escape(&worker_synthesis_latency),
        worker_checkpoint_policy = html_escape(&worker_checkpoint_policy),
        worker_checkpoint_latency = html_escape(&worker_checkpoint_latency),
        collection_observations = collection_observations,
        collection_exact = collection_exact,
        collection_semantic = collection_semantic,
        collection_executable = collection_executable,
        collection_ambiguous = collection_ambiguous,
        collection_irreducible = collection_irreducible,
        collection_accounting = collection_accounting,
        collection_frozen = collection_frozen,
        collection_quarantine = collection_quarantine,
        collection_future_receipts = collection_future_receipts,
        collection_candidate_progress = html_escape(&collection_candidate_progress),
        collection_blockers = html_escape(&collection_blockers),
        collection_revoked = collection_revoked,
        collection_rejected_wrong_candidates = collection_rejected_wrong_candidates,
        opportunity_intents = opportunity_intents,
        opportunity_tokens = opportunity_tokens,
        upper_bound_tokens = upper_bound_tokens,
        upper_bound_share = upper_bound_share,
        irreducible_tokens = metric_u64(online_opportunity, "proven_irreducible_tokens"),
        unresolved_tokens = metric_u64(online_opportunity, "unresolved_tokens"),
        upper_bound_accounting = upper_bound_accounting,
        m3_upper_bound_reachable = m3_upper_bound_reachable,
        false_accepts = false_accepts,
        parity_mismatches = parity_mismatches,
        policy_version = html_escape(policy_version),
    );
    ([(header::CACHE_CONTROL, "no-store")], Html(body)).into_response()
}

async fn control_token_stats(Path(key): Path<String>, State(state): State<AppState>) -> Response {
    if !authorized(&state, &key) {
        return not_found().await;
    }
    let fallback = read_json(&state.config.economics_path);
    let persisted_miner = read_json(&state.config.response_online_miner_report_path);
    let response_registry = read_json(&state.config.response_registry_path);
    let response_admission_controller =
        read_json(&state.config.response_admission_controller_report_path);
    let (live, hot_health, cold_health) = tokio::join!(
        read_live_miner_report(),
        read_live_json(HOT_SERVING_HEALTH_URL),
        read_live_json(COLD_LEARNING_HEALTH_URL),
    );
    let economics = live
        .get("economics")
        .filter(|value| value.is_object())
        .unwrap_or(&fallback);
    let Some((total_input_tokens, cpu_input_tokens)) = exact_token_totals(economics) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CACHE_CONTROL, "no-store")],
            Json(json!({"available":false})),
        )
            .into_response();
    };
    let (current_epoch_total_input_tokens, current_epoch_cpu_input_tokens) =
        exact_current_epoch_token_totals(economics).unwrap_or((0, 0));
    let Some(miner_input_tokens) = exact_miner_observed_tokens(&live, &persisted_miner) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CACHE_CONTROL, "no-store")],
            Json(json!({"available":false})),
        )
            .into_response();
    };
    let bridge = live_dashboard::bridge_view(&hot_health, &cold_health);
    let admission = admission_status(&state.config);
    let admission_ready_cohorts = persisted_miner
        .pointer("/miner/admission_ready_cohorts")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let online_opportunity = persisted_miner
        .pointer("/miner/self_training_v2/opportunity")
        .unwrap_or(&Value::Null);
    let response_package_count = response_registry
        .get("packages")
        .and_then(Value::as_array)
        .map_or(0, |packages| packages.len() as u64);
    let controller_relation_candidates =
        metric_u64(&response_admission_controller, "relation_candidates");
    let controller_collection_candidates =
        metric_u64(&response_admission_controller, "collection_candidates");
    let controller_crystallized_candidates =
        metric_u64(&response_admission_controller, "crystallized_candidates");
    let controller_crystallized_admissible_candidates = metric_u64(
        &response_admission_controller,
        "crystallized_admissible_candidates",
    );
    let controller_crystallized_held_candidates = metric_u64(
        &response_admission_controller,
        "crystallized_held_candidates",
    );
    let controller_crystallized_held_semantic_guard_candidates = metric_u64(
        &response_admission_controller,
        "crystallized_held_semantic_guard_candidates",
    );
    let controller_generation_delta_packages =
        metric_u64(&response_admission_controller, "generation_delta_packages");
    let controller_blocker = metric_str(
        &response_admission_controller,
        "blocker",
        "controller_report_missing",
    );
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "available": true,
            "total_input_tokens": total_input_tokens,
            "miner_input_tokens": miner_input_tokens,
            "cpu_input_tokens": cpu_input_tokens,
            "current_epoch_total_input_tokens": current_epoch_total_input_tokens,
            "current_epoch_cpu_input_tokens": current_epoch_cpu_input_tokens,
            "verified_window_total_input_tokens": metric_u64(online_opportunity, "ordinary_tokens"),
            "verified_window_cpu_input_tokens": metric_u64(online_opportunity, "verified_tokens"),
            "optimistic_upper_bound_tokens": metric_u64(online_opportunity, "optimistic_executable_upper_bound_tokens"),
            "bridge": bridge,
            "admission_ready_cohorts": admission_ready_cohorts,
            "controller_relation_candidates": controller_relation_candidates,
            "controller_collection_candidates": controller_collection_candidates,
            "controller_crystallized_candidates": controller_crystallized_candidates,
            "controller_crystallized_admissible_candidates": controller_crystallized_admissible_candidates,
            "controller_crystallized_held_candidates": controller_crystallized_held_candidates,
            "controller_crystallized_held_semantic_guard_candidates": controller_crystallized_held_semantic_guard_candidates,
            "controller_generation_delta_packages": controller_generation_delta_packages,
            "controller_blocker": controller_blocker,
            "response_package_count": response_package_count,
            "cpu_allowed": admission.cpu_allowed,
        })),
    )
        .into_response()
}

async fn control_client_connections(
    Path(key): Path<String>,
    State(state): State<AppState>,
) -> Response {
    if !authorized(&state, &key) {
        return not_found().await;
    }
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(client_connections::snapshot()),
    )
        .into_response()
}

async fn control_state(Path(key): Path<String>, State(state): State<AppState>) -> Response {
    if !authorized(&state, &key) {
        return not_found().await;
    }
    Json(json!({
        "state": read_state(&state.config.state_path),
        "admission": admission_status(&state.config),
        "services": service_statuses(),
        "metrics": read_json(&state.config.metrics_path),
        "economics": read_json(&state.config.economics_path)
        ,"response_registry": read_json(&state.config.response_registry_path)
        ,"response_admission_controller": read_json(&state.config.response_admission_controller_report_path)
        ,"response_miner_status": read_json(&state.config.response_miner_status_path)
        ,"response_online_miner": read_json(&state.config.response_online_miner_report_path)
        ,"build_manifest": read_json(&state.config.build_manifest_path)
    }))
    .into_response()
}

async fn read_live_miner_report() -> Value {
    read_live_json(LIVE_MINER_REPORT_URL).await
}

async fn read_live_json(url: &str) -> Value {
    let Ok(uri) = url.parse::<Uri>() else {
        return Value::Null;
    };
    let request = match Request::get(uri).body(Empty::<Bytes>::new()) {
        Ok(request) => request,
        Err(_) => return Value::Null,
    };
    let client: Client<HttpConnector, Empty<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();
    let Ok(Ok(response)) = tokio::time::timeout(LIVE_STATUS_TIMEOUT, client.request(request)).await
    else {
        return Value::Null;
    };
    if !response.status().is_success() {
        return Value::Null;
    }
    let Ok(Ok(body)) =
        tokio::time::timeout(LIVE_STATUS_TIMEOUT, response.into_body().collect()).await
    else {
        return Value::Null;
    };
    serde_json::from_slice(&body.to_bytes()).unwrap_or(Value::Null)
}

fn read_json(path: &std::path::Path) -> Value {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null)
}

fn admission_blockers_text(report: &Value) -> String {
    let blockers = report
        .get("admission_candidate_blockers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("blocker").and_then(Value::as_str))
        .take(3)
        .collect::<Vec<_>>();
    if blockers.is_empty() {
        "нет".to_owned()
    } else {
        blockers.join("; ")
    }
}

fn collection_blockers_text(status: &Value) -> String {
    let mut counts = BTreeMap::<String, u64>::new();
    for blocker in status
        .get("buckets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|bucket| bucket.get("admission_blocker").and_then(Value::as_str))
    {
        *counts.entry(blocker.to_owned()).or_default() += 1;
    }
    let mut blockers = counts.into_iter().collect::<Vec<_>>();
    blockers.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    if blockers.is_empty() {
        "нет".to_owned()
    } else {
        blockers
            .into_iter()
            .take(3)
            .map(|(blocker, count)| format!("{blocker} ×{count}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn metric_u64(metrics: &Value, key: &str) -> u64 {
    metrics.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn metric_u64_any(metrics: &Value, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|key| metrics.get(*key).and_then(Value::as_u64))
        .unwrap_or(0)
}

fn exact_token_totals(economics: &Value) -> Option<(u64, u64)> {
    let schema = economics.get("schema").and_then(Value::as_str)?;
    if !schema.starts_with("nando.economics-snapshot.v")
        || economics
            .get("input_token_accounting_exact")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return None;
    }
    let display_partitioned = economics
        .get("display_input_token_accounting_partitioned")
        .and_then(Value::as_bool)
        == Some(true);
    let total = if display_partitioned {
        economics.get("display_global_input_tokens")?.as_u64()?
    } else {
        economics.get("global_input_tokens")?.as_u64()?
    };
    let cpu = if display_partitioned {
        economics.get("display_avoided_input_tokens")?.as_u64()?
    } else {
        economics.get("avoided_input_tokens")?.as_u64()?
    };
    (cpu <= total).then_some((total, cpu))
}

fn exact_current_epoch_token_totals(economics: &Value) -> Option<(u64, u64)> {
    let schema = economics.get("schema").and_then(Value::as_str)?;
    if !schema.starts_with("nando.economics-snapshot.v")
        || economics
            .get("input_token_accounting_exact")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return None;
    }
    let total = economics.get("global_input_tokens")?.as_u64()?;
    let cpu = economics.get("avoided_input_tokens")?.as_u64()?;
    (cpu <= total).then_some((total, cpu))
}

fn exact_miner_observed_tokens(live: &Value, persisted: &Value) -> Option<u64> {
    let opportunity = live
        .pointer("/response/self_training_v2/opportunity")
        .filter(|value| value.is_object())
        .or_else(|| persisted.pointer("/miner/self_training_v2/opportunity"))?;
    if opportunity.get("schema").and_then(Value::as_str) != Some("nando.opportunity-board.v3")
        || opportunity
            .get("classification_identity_holds")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return None;
    }
    opportunity.get("ordinary_tokens")?.as_u64()
}

fn strongest_signal_generation(generations: &[Value]) -> Option<&Value> {
    // A persisted generation may outlive the current winner that created it.
    // Such an orphan has durable rows but no executable adapter, so it cannot
    // represent the live path toward admission.
    generations.iter().max_by_key(|generation| {
        (
            u64::from(metric_u64(generation, "physical_adapter_count") > 0),
            metric_u64(generation, "support_runtime_parity_rows"),
            metric_u64(generation, "routed_future_rows"),
            metric_u64(generation, "independent_future_rows"),
            metric_u64(generation, "after_future_watermark_rows"),
            metric_u64(generation, "matching_runtime_parity_rows"),
            metric_u64(generation, "future_rows"),
        )
    })
}

fn metric_str<'a>(metrics: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    metrics.get(key).and_then(Value::as_str).unwrap_or(fallback)
}

fn teacher_programs_text(discovery: &Value) -> String {
    let mut rows_by_action = BTreeMap::<String, u64>::new();
    for pool in discovery
        .get("teacher_pools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(action) = pool.get("action_symbol").and_then(Value::as_str) else {
            continue;
        };
        *rows_by_action.entry(action.to_owned()).or_default() += metric_u64(pool, "positive_rows");
    }
    let mut programs = rows_by_action.into_iter().collect::<Vec<_>>();
    programs.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    if programs.is_empty() {
        return "нет".to_owned();
    }
    programs
        .into_iter()
        .map(|(action, rows)| format!("{action}: {rows}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn module_version_rows(manifest: &Value) -> String {
    let Some(modules) = manifest.get("modules").and_then(Value::as_array) else {
        return "<tr><td>Манифест</td><td class=\"off\">ОТСУТСТВУЕТ</td></tr>".to_owned();
    };
    modules
        .iter()
        .map(|module| {
            let name = metric_str(module, "name", "неизвестно");
            let version = metric_str(module, "version", "ОТСУТСТВУЕТ");
            let contract = metric_str(module, "contract", "ОТСУТСТВУЕТ");
            let sha = metric_str(module, "sha256", "");
            let short_sha = sha.get(..12).unwrap_or(sha);
            let value = if short_sha.is_empty() {
                format!("{version} | {contract}")
            } else {
                format!("{version} | {contract} | {short_sha}")
            };
            format!(
                "<tr><td>{}</td><td>{}</td></tr>",
                html_escape(name),
                html_escape(&value)
            )
        })
        .collect()
}

fn format_micros(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.2} с", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1} мс", value as f64 / 1_000.0)
    } else {
        format!("{value} мкс")
    }
}

fn format_ratio_milli(value: u64) -> String {
    format!("{}.{:01}%", value / 10, value % 10)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

async fn change_mode(
    Path(key): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<ModeForm>,
) -> Response {
    if !authorized(&state, &key) {
        return not_found().await;
    }
    let Some(mode) = GatewayMode::parse(&form.mode) else {
        return (StatusCode::BAD_REQUEST, "invalid mode").into_response();
    };
    let config = Arc::clone(&state.config);
    let result =
        tokio::task::spawn_blocking(move || apply_mode(&config, mode, "manual_html")).await;
    match result {
        Ok(Ok(_)) => Redirect::to(&format!("/control/{key}")).into_response(),
        Ok(Err(error)) => (StatusCode::CONFLICT, error.to_string()).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("control task failed: {error}"),
        )
            .into_response(),
    }
}

async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": {"message": "not found", "type": "not_found"}})),
    )
        .into_response()
}

fn authorized(state: &AppState, key: &str) -> bool {
    key.as_bytes() == state.config.status_key.as_bytes()
}

fn yes_no(value: bool) -> &'static str {
    if value { "ДА" } else { "НЕТ" }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn cpu_safety_watchdog(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let mode = read_state(&state.config.state_path).mode;
        let admission = admission_status(&state.config);
        let services = service_statuses();
        if should_auto_promote(mode, admission.cpu_allowed) {
            let config = Arc::clone(&state.config);
            match tokio::task::spawn_blocking(move || {
                apply_mode(&config, GatewayMode::Cpu, "automatic_gate_promotion")
            })
            .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    eprintln!("nando-gateway-control: automatic promotion failed: {error}");
                }
                Err(error) => {
                    eprintln!("nando-gateway-control: promotion task failed: {error}");
                }
            }
            continue;
        }
        if mode != GatewayMode::Cpu {
            continue;
        }
        let service_failure = services.iter().any(|service| !service.active);
        if admission.cpu_allowed && !service_failure {
            continue;
        }

        let reason = if !admission.cpu_allowed {
            format!("automatic_cpu_revoke: {}", admission.reason)
        } else {
            "automatic_cpu_revoke: required service stopped".into()
        };
        let config = Arc::clone(&state.config);
        match tokio::task::spawn_blocking(move || apply_mode(&config, GatewayMode::Shadow, &reason))
            .await
        {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => eprintln!("nando-gateway-control: watchdog demotion failed: {error}"),
            Err(error) => eprintln!("nando-gateway-control: watchdog task failed: {error}"),
        }
    }
}

fn should_auto_promote(mode: GatewayMode, cpu_allowed: bool) -> bool {
    mode == GatewayMode::Shadow && cpu_allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_blocks_markup() {
        assert_eq!(html_escape("<x>&\""), "&lt;x&gt;&amp;&quot;");
    }

    #[test]
    fn bypass_label_is_present() {
        assert_eq!(GatewayMode::parse("BYPASS"), Some(GatewayMode::Bypass));
    }

    #[test]
    fn signal_tree_prefers_receipt_backed_routed_generation() {
        let generations = vec![
            json!({
                "generation": 0,
                "physical_adapter_count": 0,
                "support_runtime_parity_rows": 0,
                "routed_future_rows": 0,
                "future_rows": 0
            }),
            json!({
                "generation": 4,
                "physical_adapter_count": 1,
                "support_runtime_parity_rows": 32,
                "matching_runtime_parity_rows": 51,
                "routed_future_rows": 15,
                "future_rows": 0
            }),
            json!({
                "generation": 5,
                "physical_adapter_count": 1,
                "support_runtime_parity_rows": 32,
                "matching_runtime_parity_rows": 40,
                "routed_future_rows": 10,
                "future_rows": 18
            }),
        ];

        let selected = strongest_signal_generation(&generations).expect("generation");
        assert_eq!(metric_u64(selected, "generation"), 4);
        assert_eq!(metric_u64(selected, "routed_future_rows"), 15);
    }

    #[test]
    fn signal_tree_ignores_orphan_with_more_persisted_future() {
        let generations = vec![
            json!({
                "generation": 1,
                "physical_adapter_count": 0,
                "support_runtime_parity_rows": 32,
                "future_rows": 31
            }),
            json!({
                "generation": 2,
                "physical_adapter_count": 1,
                "support_runtime_parity_rows": 32,
                "matching_runtime_parity_rows": 64,
                "after_future_watermark_rows": 32,
                "future_rows": 0
            }),
        ];

        let selected = strongest_signal_generation(&generations).expect("generation");
        assert_eq!(metric_u64(selected, "generation"), 2);
    }

    #[test]
    fn watchdog_auto_promotes_only_eligible_shadow_mode() {
        assert!(should_auto_promote(GatewayMode::Shadow, true));
        assert!(!should_auto_promote(GatewayMode::Shadow, false));
        assert!(!should_auto_promote(GatewayMode::Bypass, true));
        assert!(!should_auto_promote(GatewayMode::Cpu, true));
    }

    #[test]
    fn token_dashboard_accepts_only_exact_consistent_accounting() {
        let exact = json!({
            "schema": "nando.economics-snapshot.v3",
            "input_token_accounting_exact": true,
            "global_input_tokens": 1_000,
            "avoided_input_tokens": 125,
        });
        assert_eq!(exact_token_totals(&exact), Some((1_000, 125)));
        assert_eq!(exact_current_epoch_token_totals(&exact), Some((1_000, 125)));

        let partitioned = json!({
            "schema": "nando.economics-snapshot.v4",
            "input_token_accounting_exact": true,
            "global_input_tokens": 100,
            "avoided_input_tokens": 5,
            "display_input_token_accounting_partitioned": true,
            "display_global_input_tokens": 1_100,
            "display_avoided_input_tokens": 130,
        });
        assert_eq!(exact_token_totals(&partitioned), Some((1_100, 130)));
        assert_eq!(
            exact_current_epoch_token_totals(&partitioned),
            Some((100, 5))
        );

        let estimated = json!({
            "schema": "nando.economics-snapshot.v3",
            "input_token_accounting_exact": false,
            "global_input_tokens": 1_000,
            "avoided_input_tokens": 125,
        });
        assert_eq!(exact_token_totals(&estimated), None);
        assert_eq!(exact_current_epoch_token_totals(&estimated), None);

        let impossible = json!({
            "schema": "nando.economics-snapshot.v3",
            "input_token_accounting_exact": true,
            "global_input_tokens": 100,
            "avoided_input_tokens": 101,
        });
        assert_eq!(exact_token_totals(&impossible), None);
        assert_eq!(exact_current_epoch_token_totals(&impossible), None);
    }

    #[test]
    fn token_dashboard_uses_only_accounted_miner_denominator() {
        let persisted = json!({
            "miner": {"self_training_v2": {"opportunity": {
                "schema": "nando.opportunity-board.v3",
                "classification_identity_holds": true,
                "ordinary_tokens": 497_237_835,
            }}}
        });
        assert_eq!(
            exact_miner_observed_tokens(&Value::Null, &persisted),
            Some(497_237_835)
        );

        let unaccounted = json!({
            "miner": {"self_training_v2": {"opportunity": {
                "schema": "nando.opportunity-board.v3",
                "classification_identity_holds": false,
                "ordinary_tokens": 497_237_835,
            }}}
        });
        assert_eq!(
            exact_miner_observed_tokens(&Value::Null, &unaccounted),
            None
        );
    }
}
