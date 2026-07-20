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
use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LIVE_MINER_REPORT_URL: &str = "http://127.0.0.1:18789/v2/miner/report";
const LIVE_MINER_REPORT_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone)]
struct AppState {
    config: Arc<ControlConfig>,
}

#[derive(Deserialize)]
struct ModeForm {
    mode: String,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FlowState {
    Live,
    Pass,
    Wait,
    Block,
    Locked,
}

impl FlowState {
    fn class(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Pass => "pass",
            Self::Wait => "wait",
            Self::Block => "block",
            Self::Locked => "locked",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Live => "LIVE",
            Self::Pass => "PASS",
            Self::Wait => "WAIT",
            Self::Block => "BLOCK",
            Self::Locked => "LOCKED",
        }
    }
}

struct SignalArchitectureView<'a> {
    partition: u64,
    generation: u64,
    transitions: u64,
    support: u64,
    matching: u64,
    matching_sessions: u64,
    after_watermark: u64,
    independent: u64,
    consistent: u64,
    routed: u64,
    future: u64,
    lost: u64,
    blocker: &'a str,
    online_emitted: u64,
    online_blocked: u64,
    response_active: u64,
    cpu_share: &'a str,
    online_ready: bool,
}

struct SignalStage<'a> {
    id: &'a str,
    step: &'a str,
    title: &'a str,
    logic: &'a str,
    metric: String,
    metric_label: &'a str,
    module: &'a str,
    owner: &'a str,
    state: FlowState,
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
    let online_miner = read_json(&state.config.response_online_miner_report_path);
    let live_miner = read_live_miner_report().await;
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
                if service.active { "RUNNING" } else { "STOPPED" }
            )
        })
        .collect::<String>();
    let active_profiles = metric_u64(runtime_admission, "active_profile_count");
    let avoided_calls = metric_u64(&economics, "avoided_calls");
    let tokens_saved = metric_u64(&economics, "avoided_input_tokens");
    let total_requests = metric_u64(&economics, "dedupe_eligible_client_intents");
    let cpu_share = format_ratio_milli(metric_u64(&economics, "call_saving_share_milli"));
    let token_share = format_ratio_milli(metric_u64(&economics, "input_token_saving_share_milli"));
    let verification_coverage =
        format_ratio_milli(metric_u64(&economics, "verification_coverage_milli"));
    let economics_age = unix_now().saturating_sub(metric_u64(&economics, "generated_at_unix"));
    let economics_status = match economics.get("schema").and_then(Value::as_str) {
        Some(schema) if schema.starts_with("nando.economics-snapshot.v") => {
            if economics_age <= 120 {
                "FRESH"
            } else {
                "STALE"
            }
        }
        _ => "MISSING",
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
    let eligible_intents = metric_u64(&economics, "dedupe_eligible_client_intents");
    let excluded_intents = metric_u64(&economics, "dedupe_ineligible_client_intents");
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
    let strongest_generation = self_training
        .get("generations")
        .and_then(Value::as_array)
        .and_then(|generations| strongest_signal_generation(generations))
        .unwrap_or(&Value::Null);
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
    let signal_support = metric_u64(strongest_generation, "support_runtime_parity_rows");
    let signal_matching = metric_u64(strongest_generation, "matching_runtime_parity_rows");
    let signal_matching_sessions =
        metric_u64(strongest_generation, "matching_runtime_parity_sessions");
    let signal_after_watermark = metric_u64(strongest_generation, "after_future_watermark_rows");
    let signal_independent = metric_u64(strongest_generation, "independent_future_rows");
    let signal_consistent = metric_u64(strongest_generation, "program_consistent_future_rows");
    let signal_routed = metric_u64(strongest_generation, "routed_future_rows");
    let signal_future = metric_u64(strongest_generation, "future_rows");
    let signal_lost = signal_routed.saturating_sub(signal_future);
    let signal_blocker = metric_str(strongest_generation, "blocker", "нет");
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
    let signal_architecture = signal_architecture_html(
        &SignalArchitectureView {
            partition: signal_partition,
            generation: signal_generation,
            transitions: online_transitions,
            support: signal_support,
            matching: signal_matching,
            matching_sessions: signal_matching_sessions,
            after_watermark: signal_after_watermark,
            independent: signal_independent,
            consistent: signal_consistent,
            routed: signal_routed,
            future: signal_future,
            lost: signal_lost,
            blocker: signal_blocker,
            online_emitted,
            online_blocked,
            response_active,
            cpu_share: &cpu_share,
            online_ready: online_status == "READY",
        },
        &build_manifest,
    );
    let body = format!(
        r#"<!doctype html>
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="refresh" content="10">
<title>Nando Gateway</title>
<style>
:root {{ color-scheme:dark; font-family:"DejaVu Sans Mono","Liberation Mono",ui-monospace,monospace; background:#111315; color:#d8dde2; }}
* {{ box-sizing:border-box; }}
body {{ margin:0; background:#111315; }}
header {{ background:#080a0b; color:#eef1f3; padding:11px 20px; border-bottom:1px solid #4a5055; }}
.header-inner {{ width:min(1180px,100%); margin:0 auto; display:flex; justify-content:space-between; align-items:center; gap:20px; }}
.brand {{ display:flex; align-items:baseline; gap:12px; min-width:0; }}
h1 {{ margin:0; font-size:15px; letter-spacing:0; text-transform:uppercase; }}
.build {{ color:#7f8991; font-size:10px; overflow-wrap:anywhere; }}
.mode-wrap {{ display:flex; align-items:center; gap:10px; }}
.mode-label {{ color:#7f8991; font-size:10px; text-transform:uppercase; }}
.mode {{ color:#66d98b; font-size:12px; font-weight:700; }}
main {{ width:min(1180px,100%); margin:0 auto; padding:14px 20px 28px; }}
.controls {{ display:flex; flex-wrap:wrap; gap:12px; margin:0 0 12px; }}
.controls form {{ margin:0; }}
button {{ min-height:0; padding:2px 0; border:0; border-radius:0; background:transparent; color:#aeb7bf; font:700 11px inherit; cursor:pointer; }}
button::before {{ content:"["; color:#59636b; }}
button::after {{ content:"]"; color:#59636b; }}
button:hover {{ color:#fff; background:transparent; }}
.bypass,.bypass:hover {{ color:#f46d65; background:transparent; }}
button:disabled {{ color:#535a60; cursor:not-allowed; }}
.metric-grid {{ display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); column-gap:32px; }}
.band {{ border-top:1px solid #353a3e; padding:12px 0; min-width:0; }}
h2 {{ margin:0 0 10px; color:#dfe5e9; font-size:12px; letter-spacing:0; text-transform:uppercase; }}
.architecture {{ padding-top:4px; }}
.architecture-head {{ display:flex; justify-content:space-between; align-items:flex-start; gap:24px; margin-bottom:8px; }}
.architecture-title h2 {{ margin-bottom:3px; color:#8ee6a8; font-size:13px; }}
.architecture-title p {{ margin:0; color:#707a82; font-size:10px; line-height:1.4; }}
.architecture-state {{ flex:0 0 auto; display:flex; align-items:center; gap:8px; }}
.state-chip {{ color:#8b949b; font-size:10px; font-weight:700; white-space:nowrap; }}
.state-chip::before {{ content:"["; }} .state-chip::after {{ content:"]"; }}
.state-chip.live,.state-chip.pass {{ color:#66d98b; }}
.state-chip.wait {{ color:#e0b35a; }}
.state-chip.block {{ color:#ff6b63; }}
.state-chip.locked {{ color:#70777d; }}
.architecture-meta {{ color:#707a82; font-size:10px; font-weight:700; }}
.flow-tree {{ padding:12px 14px; border:1px solid #3f464b; background:#080a0b; font-size:11px; line-height:1.35; overflow-x:auto; }}
.terminal-stage {{ min-width:620px; }}
.terminal-line {{ display:grid; grid-template-columns:22px 42px minmax(240px,1fr) minmax(70px,auto) 74px; align-items:baseline; gap:8px; min-height:19px; }}
.tree-glyph {{ color:#566068; white-space:pre; }}
.stage-index {{ color:#737e86; }}
.stage-title {{ color:#dce2e6; font-weight:700; }}
.stage-metric {{ color:#e5bd63; text-align:right; white-space:nowrap; }}
.terminal-stage.locked .stage-title,.terminal-stage.locked .stage-metric {{ color:#666e74; }}
.terminal-detail,.terminal-edge {{ min-width:620px; padding-left:72px; color:#586168; font-size:9px; white-space:nowrap; }}
.terminal-detail .tree-glyph,.terminal-edge .tree-glyph {{ display:inline-block; width:22px; margin-left:-72px; margin-right:50px; }}
.terminal-failure {{ min-width:620px; margin:3px 0; grid-template-columns:22px 180px minmax(260px,1fr) auto; color:#ff6b63; }}
.terminal-failure strong {{ color:#ff6b63; margin-right:8px; }}
.terminal-failure span {{ color:#ef938e; }}
.terminal-failure code {{ color:#b87f7b; margin-left:10px; }}
.terminal-rule {{ padding:7px 2px 0; color:#626c73; font-size:9px; }}
.compact td {{ padding:5px 0; }}
details {{ border-top:1px solid #353a3e; margin-top:4px; }}
summary {{ padding:11px 0; color:#9ba4ab; cursor:pointer; font-size:11px; font-weight:700; }}
.advanced {{ padding-bottom:8px; }}
table {{ width:100%; border-collapse:collapse; font-size:10px; }}
td {{ padding:5px 0; border-bottom:1px dotted #3a4044; overflow-wrap:anywhere; vertical-align:top; }}
td:first-child {{ padding-right:16px; }}
td:last-child {{ text-align:right; font-weight:700; }}
.ok {{ color:#66d98b; }} .off {{ color:#ff6b63; }}
.note {{ margin:0; color:#7f8991; font-size:10px; line-height:1.5; overflow-wrap:anywhere; }}
@media (max-width:680px) {{
  .header-inner,.brand,.architecture-head {{ align-items:flex-start; flex-direction:column; }}
  .header-inner,.brand {{ gap:6px; }}
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
}}
</style>
</head>
<body>
<header><div class="header-inner">
<div class="brand"><h1>Nando Gateway</h1><span class="build">build {build_id} · {build_commit_short}</span></div>
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
<tr><td>ACTIVE / QUARANTINE</td><td>{response_active} / {response_quarantine}</td></tr>
</table></section>
<section class="band"><h2>Независимый допуск</h2><table>
<tr><td>Composite gate</td><td>{verdict}</td></tr>
<tr><td>False accepts</td><td>{false_accepts}</td></tr>
<tr><td>Parity mismatches</td><td>{parity_mismatches}</td></tr>
<tr><td>Admission emitted / blocked</td><td>{online_emitted} / {online_blocked}</td></tr>
</table></section>
</div>
<details><summary>Технические детали</summary><div class="advanced">
<section class="band"><h2>Сервисы</h2><table>{service_rows}</table></section>
	<section class="band"><h2>Версии сборок и модулей</h2><table>{module_version_rows}</table></section>
	<section class="band"><h2>CPU admission</h2><table>
<tr><td>Composite gate</td><td>{verdict}</td></tr>
<tr><td>Future eligibility</td><td>{eligible}</td></tr>
<tr><td>Evidence fresh</td><td>{fresh}</td></tr>
<tr><td>Runtime route ready</td><td>{route_ready}</td></tr>
	</table></section>
	<div class="metric-grid">
	<section class="band"><h2>CPU факты</h2><table>
	<tr><td>Активные transition-профили</td><td>{active_profiles}</td></tr>
	<tr><td>LLM-вызовов предотвращено</td><td>{avoided_calls}</td></tr>
	<tr><td>Входных токенов сэкономлено</td><td>{tokens_saved}</td></tr>
	<tr><td>Наблюдаемых provider-запросов</td><td>{total_requests}</td></tr>
	<tr><td>Доля обычного трафика на CPU</td><td>{cpu_share}</td></tr>
	<tr><td>Экономия входных токенов</td><td>{token_share}</td></tr>
	<tr><td>Покрытие независимой проверкой</td><td>{verification_coverage}</td></tr>
	<tr><td>Economics snapshot</td><td>{economics_status}</td></tr>
	<tr><td>False accepts</td><td>{false_accepts}</td></tr>
	<tr><td>Runtime parity mismatches</td><td>{parity_mismatches}</td></tr>
	<tr><td>Promotion policy</td><td>{policy_version}</td></tr>
	</table></section>
	<section class="band"><h2>Коммерческий denominator</h2><table>
	<tr><td>Учитываемые / исключённые intents</td><td>{eligible_intents} / {excluded_intents}</td></tr>
	<tr><td>Входные токены denominator</td><td>{global_input_tokens}</td></tr>
	<tr><td>Local / independently verified</td><td>{actual_local_accepts} / {verified_local_accepts}</td></tr>
	<tr><td>Неразрешённые исходы / отсутствующие receipts</td><td>{unresolved_local} / {missing_receipts}</td></tr>
	<tr><td>Economics hard gate</td><td>{hard_gate}</td></tr>
	<tr><td>Product M1</td><td>{product_m1}</td></tr>
	<tr><td>До M1: intents / avoided calls</td><td>{m1_intent_gap} / {m1_avoided_gap}</td></tr>
	<tr><td>Возраст economics snapshot</td><td>{economics_age_text}</td></tr>
	</table></section>
	</div>
	<div class="metric-grid">
	<section class="band"><h2>Response actors</h2><table>
	<tr><td>ACTIVE / QUARANTINE</td><td>{response_active} / {response_quarantine}</td></tr>
	<tr><td>ACTIVE package</td><td>{active_response_id}</td></tr>
	<tr><td>ACTIVE program</td><td>{active_response_program}</td></tr>
	<tr><td>Доказательство ACTIVE</td><td>{active_response_progress}</td></tr>
	<tr><td>Execution p99</td><td>{execution_p99}</td></tr>
	<tr><td>Размер transition packages</td><td>{package_bytes} Б</td></tr>
	<tr><td>Чистые active future rows</td><td>{active_future_rows}</td></tr>
	<tr><td>Shadow executions</td><td>{shadow_executions}</td></tr>
	</table></section>
	<section class="band"><h2>Поточный Rust-майнер</h2><table>
	<tr><td>Состояние / возраст snapshot</td><td>{online_status} / {online_age} с</td></tr>
	<tr><td>Очередь / ёмкость</td><td>{worker_queue}</td></tr>
	<tr><td>Обработано / ошибок worker</td><td>{worker_processed}</td></tr>
	<tr><td>Задержка synthesis</td><td>{worker_synthesis_latency}</td></tr>
	<tr><td>Checkpoint policy</td><td>{worker_checkpoint_policy}</td></tr>
	<tr><td>Последний checkpoint / всего</td><td>{worker_checkpoint_latency}</td></tr>
	<tr><td>Teacher transitions / pools</td><td>{online_transitions} / {online_teacher_pools}</td></tr>
	<tr><td>Teacher programs</td><td>{online_teacher_programs}</td></tr>
	<tr><td>CEGIS cohorts / winners / exact checks</td><td>{online_cegis_cohorts} / {online_cegis_winners} / {online_exact_checks}</td></tr>
	<tr><td>Candidates / admission-ready</td><td>{online_candidates} / {online_admission_ready}</td></tr>
	<tr><td>Admission emitted / blocked / accounting</td><td>{online_emitted} / {online_blocked} / {online_admission_accounting}</td></tr>
	<tr><td>Точный admission blocker</td><td>{online_admission_blockers}</td></tr>
	<tr><td>Лучшее frozen generation</td><td>{online_generation}</td></tr>
	<tr><td>Горячее состояние</td><td>{online_warm_bytes} Б</td></tr>
	<tr><td>False accepts / parity failures</td><td>{online_false_accepts} / {online_parity_failures}</td></tr>
	<tr><td>Текущее verified-окно</td><td>{online_product_window}</td></tr>
	</table></section>
	</div>
	<div class="metric-grid">
	<section class="band"><h2>Collection synthesis</h2><table>
	<tr><td>Наблюдения</td><td>{collection_observations}</td></tr>
	<tr><td>Exact / semantic / accounted executable</td><td>{collection_exact} / {collection_semantic} / {collection_executable}</td></tr>
	<tr><td>Ambiguous / irreducible</td><td>{collection_ambiguous} / {collection_irreducible}</td></tr>
	<tr><td>Accounting identity</td><td>{collection_accounting}</td></tr>
	<tr><td>Frozen / QUARANTINE / future receipts</td><td>{collection_frozen} / {collection_quarantine} / {collection_future_receipts}</td></tr>
	<tr><td>Candidate emitted / blocked / accounting</td><td>{collection_candidate_progress}</td></tr>
	<tr><td>Точные blockers</td><td>{collection_blockers}</td></tr>
	<tr><td>Отозванные / ошибочные гипотезы</td><td>{collection_revoked} / {collection_rejected_wrong_candidates}</td></tr>
	</table></section>
	<section class="band"><h2>Верхняя граница CPU</h2><table>
	<tr><td>Authoritative окно</td><td>{opportunity_intents} intents / {opportunity_tokens} tokens</td></tr>
	<tr><td>Optimistic executable upper bound</td><td>{upper_bound_tokens} tokens / {upper_bound_share}</td></tr>
	<tr><td>Irreducible / unresolved tokens</td><td>{irreducible_tokens} / {unresolved_tokens}</td></tr>
	<tr><td>Upper-bound accounting</td><td>{upper_bound_accounting}</td></tr>
	<tr><td>M3 достижим в этом окне</td><td>{m3_upper_bound_reachable}</td></tr>
	</table></section>
	</div>
</div></details>
<section class="band"><h2>Граница</h2><p class="note">{reason}. Кнопка BYPASS останавливает Nando-наблюдение и майнер, но Nginx продолжает передавать Codex-трафик в OpenAI.</p></section>
</main>
</body>
</html>"#,
        mode = current.mode,
        service_rows = service_rows,
        build_id = html_escape(build_id),
        build_commit_short = html_escape(build_commit_short),
        key = html_escape(&key),
        cpu_disabled = cpu_disabled,
        signal_architecture = signal_architecture,
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
        ,"response_miner_status": read_json(&state.config.response_miner_status_path)
        ,"response_online_miner": read_json(&state.config.response_online_miner_report_path)
        ,"build_manifest": read_json(&state.config.build_manifest_path)
    }))
    .into_response()
}

async fn read_live_miner_report() -> Value {
    let Ok(uri) = LIVE_MINER_REPORT_URL.parse::<Uri>() else {
        return Value::Null;
    };
    let request = match Request::get(uri).body(Empty::<Bytes>::new()) {
        Ok(request) => request,
        Err(_) => return Value::Null,
    };
    let client: Client<HttpConnector, Empty<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();
    let Ok(Ok(response)) =
        tokio::time::timeout(LIVE_MINER_REPORT_TIMEOUT, client.request(request)).await
    else {
        return Value::Null;
    };
    if !response.status().is_success() {
        return Value::Null;
    }
    let Ok(Ok(body)) =
        tokio::time::timeout(LIVE_MINER_REPORT_TIMEOUT, response.into_body().collect()).await
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

fn signal_architecture_html(view: &SignalArchitectureView<'_>, manifest: &Value) -> String {
    let future_state = if view.lost > 0 {
        FlowState::Block
    } else if view.future >= 32 {
        FlowState::Pass
    } else {
        FlowState::Wait
    };
    let admission_state = if view.future < 32 {
        FlowState::Locked
    } else if view.online_emitted > 0 {
        FlowState::Pass
    } else {
        FlowState::Wait
    };
    let cpu_state = if view.response_active > 0 {
        FlowState::Live
    } else {
        FlowState::Locked
    };
    let overall_state = if view.lost > 0 {
        FlowState::Block
    } else if view.response_active > 0 {
        FlowState::Pass
    } else {
        FlowState::Wait
    };
    let stages = [
        SignalStage {
            id: "capture",
            step: "00",
            title: "Захват завершённых переходов",
            logic: "Формирует state_before / action / state_after и receipt для проверенного teacher-перехода.",
            metric: view.transitions.to_string(),
            metric_label: "teacher transitions",
            module: "Streaming worker",
            owner: "nando-transition-serving::session_stream",
            state: if view.online_ready {
                FlowState::Live
            } else {
                FlowState::Wait
            },
        },
        SignalStage {
            id: "support",
            step: "01",
            title: "Замороженный support",
            logic: "Закрепляет receipt-backed support и immutable root текущего operator circuit.",
            metric: format!("{} / 32", view.support),
            metric_label: "support receipts",
            module: "Teacher/student miner",
            owner: "nando-response-actor::online_state",
            state: if view.support >= 32 {
                FlowState::Pass
            } else {
                FlowState::Wait
            },
        },
        SignalStage {
            id: "law-match",
            step: "02",
            title: "Совпадение с relation-law",
            logic: "Сравнивает новые runtime parity frames с teacher signature и выбранным cohort.",
            metric: view.matching.to_string(),
            metric_label: "frames, sessions shown below",
            module: "Teacher/student miner",
            owner: "online_state::parity_diagnostics",
            state: if view.matching > 0 {
                FlowState::Pass
            } else {
                FlowState::Wait
            },
        },
        SignalStage {
            id: "watermark",
            step: "03",
            title: "Event-time watermark",
            logic: "Оставляет только события новее max(support watermark, repair watermark).",
            metric: view.after_watermark.to_string(),
            metric_label: "post-watermark frames",
            module: "Frozen future",
            owner: "online_state::future_watermark",
            state: if view.after_watermark > 0 {
                FlowState::Pass
            } else {
                FlowState::Wait
            },
        },
        SignalStage {
            id: "independence",
            step: "04",
            title: "Независимость от support",
            logic: "Исключает повтор frame, session, intent и event из замороженного support.",
            metric: view.independent.to_string(),
            metric_label: "independent frames",
            module: "Frozen future",
            owner: "online_state::independence_filter",
            state: if view.independent > 0 {
                FlowState::Pass
            } else {
                FlowState::Wait
            },
        },
        SignalStage {
            id: "typed-parity",
            step: "05",
            title: "Typed program parity",
            logic: "Проверяет, что winner program воспроизводит ожидаемую структуру; mismatch даёт ABSTAIN.",
            metric: view.consistent.to_string(),
            metric_label: "program-consistent frames",
            module: "Typed DSL + verifier",
            owner: "synthesis::program_is_consistent",
            state: if view.consistent > 0 {
                FlowState::Pass
            } else {
                FlowState::Wait
            },
        },
        SignalStage {
            id: "future-route",
            step: "06",
            title: "Маршрутизация в generation",
            logic: "Winner должен направить frame в тот же cohort и подтверждённый physical adapter.",
            metric: view.routed.to_string(),
            metric_label: "routed frames",
            module: "Teacher/student miner",
            owner: "cegis::winner_routes_frame",
            state: if view.routed > 0 {
                FlowState::Pass
            } else {
                FlowState::Wait
            },
        },
        SignalStage {
            id: "future-store",
            step: "07",
            title: "Generation-owned future storage",
            logic: "Должен атомарно записать frame и parity receipt в immutable future поколения g+1.",
            metric: format!("{} / 32", view.future),
            metric_label: "durable future receipts",
            module: "Frozen future",
            owner: "generation.future + parity_receipts.future",
            state: future_state,
        },
        SignalStage {
            id: "admission",
            step: "08",
            title: "External admission",
            logic: "Пересобирает proof, проверяет 32 future rows, zero wrong/parity и только затем выдаёт package.",
            metric: format!("{} / {}", view.online_emitted, view.online_blocked),
            metric_label: "emitted / blocked",
            module: "Admission",
            owner: "nando-response-actor::online_admission",
            state: admission_state,
        },
        SignalStage {
            id: "hot-cpu",
            step: "09",
            title: "ACTIVE CPU execution",
            logic: "Role grounding и actor исполняют package; независимый verifier всё ещё может вернуть ABSTAIN.",
            metric: view.cpu_share.to_owned(),
            metric_label: "ordinary traffic on CPU",
            module: "Rust serving",
            owner: "runtime::execute_response",
            state: cpu_state,
        },
    ];
    let edge_labels = [
        "verified transition receipt".to_owned(),
        "support root + winner cohort".to_owned(),
        format!("{} matching sessions", view.matching_sessions),
        "event-time disjointness".to_owned(),
        "typed structural replay".to_owned(),
        "winner route predicate".to_owned(),
        "generation-owned write".to_owned(),
        "proof-carrying candidate".to_owned(),
        "ACTIVE authority lease".to_owned(),
    ];
    let mut tree = String::new();
    for (index, stage) in stages.iter().enumerate() {
        tree.push_str(&signal_stage_html(
            stage,
            manifest,
            index + 1 == stages.len(),
        ));
        if index + 1 == stages.len() {
            continue;
        }
        if index == 6 && view.lost > 0 {
            tree.push_str(&format!(
                "<div class=\"terminal-line terminal-failure\" data-edge=\"route-to-future\"><span class=\"tree-glyph\">├─</span><strong>BLOCK НА ЭТОМ РЕБРЕ</strong><span>{} routed -> {} записано; потеряно {}</span><code>{}</code></div>",
                view.routed,
                view.future,
                view.lost,
                html_escape(view.blocker)
            ));
        } else if let Some(label) = edge_labels.get(index) {
            tree.push_str(&format!(
                "<div class=\"terminal-edge\"><span class=\"tree-glyph\">│</span>{}</div>",
                html_escape(label)
            ));
        }
    }
    format!(
        r#"<section class="architecture" data-signal-status="{}">
<div class="architecture-head">
<div class="architecture-title"><h2>NANDO SIGNAL PATH</h2><p>live trace -&gt; frozen future -&gt; admission -&gt; CPU</p></div>
<div class="architecture-state"><span class="state-chip {}">{}</span><span class="architecture-meta">partition v{} · generation {}</span></div>
</div>
<div class="flow-tree">{}</div>
<div class="terminal-rule">support != future | verifier = authority | missing proof = ABSTAIN</div>
</section>"#,
        overall_state.class(),
        overall_state.class(),
        overall_state.label(),
        view.partition,
        view.generation,
        tree
    )
}

fn signal_stage_html(stage: &SignalStage<'_>, manifest: &Value, last: bool) -> String {
    let branch = if last { "└─" } else { "├─" };
    format!(
        r#"<div class="terminal-stage {}" data-stage="{}" title="{}">
<div class="terminal-line"><span class="tree-glyph">{}</span><span class="stage-index">[{}]</span><strong class="stage-title">{}</strong><span class="stage-metric">{}</span><span class="state-chip {}">{}</span></div>
<div class="terminal-detail"><span class="tree-glyph">│</span>{} · {} · {}</div>
</div>"#,
        stage.state.class(),
        html_escape(stage.id),
        html_escape(stage.logic),
        branch,
        html_escape(stage.step),
        html_escape(stage.title),
        html_escape(&stage.metric),
        stage.state.class(),
        stage.state.label(),
        module_identity_text(manifest, stage.module),
        html_escape(stage.owner),
        html_escape(stage.metric_label)
    )
}

fn module_identity_text(manifest: &Value, module_name: &str) -> String {
    let module = manifest
        .get("modules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|module| module.get("name").and_then(Value::as_str) == Some(module_name));
    let version = module
        .and_then(|module| module.get("version"))
        .and_then(Value::as_str)
        .unwrap_or("MISSING");
    let contract = module
        .and_then(|module| module.get("contract"))
        .and_then(Value::as_str)
        .unwrap_or("MISSING");
    format!(
        "{} {} {}",
        html_escape(module_name),
        html_escape(version),
        html_escape(&compact_identity(contract))
    )
}

fn compact_identity(value: &str) -> String {
    if value.len() <= 24 {
        value.to_owned()
    } else {
        format!("{}...", value.chars().take(12).collect::<String>())
    }
}

fn metric_u64(metrics: &Value, key: &str) -> u64 {
    metrics.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn strongest_signal_generation(generations: &[Value]) -> Option<&Value> {
    // A zero-future generation can still be the active signal path. Rank by
    // proven support and routed evidence first so the dashboard exposes the
    // actual loss point instead of selecting an unrelated empty generation.
    generations.iter().max_by_key(|generation| {
        (
            metric_u64(generation, "support_runtime_parity_rows"),
            metric_u64(generation, "routed_future_rows"),
            metric_u64(generation, "future_rows"),
            metric_u64(generation, "matching_runtime_parity_rows"),
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
        return "<tr><td>Manifest</td><td class=\"off\">MISSING</td></tr>".to_owned();
    };
    modules
        .iter()
        .map(|module| {
            let name = metric_str(module, "name", "unknown");
            let version = metric_str(module, "version", "MISSING");
            let contract = metric_str(module, "contract", "MISSING");
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
    if value { "YES" } else { "NO" }
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
                "support_runtime_parity_rows": 0,
                "routed_future_rows": 0,
                "future_rows": 0
            }),
            json!({
                "generation": 4,
                "support_runtime_parity_rows": 32,
                "matching_runtime_parity_rows": 51,
                "routed_future_rows": 15,
                "future_rows": 0
            }),
        ];

        let selected = strongest_signal_generation(&generations).expect("generation");
        assert_eq!(metric_u64(selected, "generation"), 4);
        assert_eq!(metric_u64(selected, "routed_future_rows"), 15);
    }

    #[test]
    fn architecture_tree_places_storage_gap_on_route_edge() {
        let manifest = json!({
            "modules": [
                {"name":"Streaming worker","version":"event-driven.v2","contract":"0-events-or-60s","sha256":"aaaaaaaaaaaa1111"},
                {"name":"Teacher/student miner","version":"strategy.v3","contract":"state.v3","sha256":"bbbbbbbbbbbb2222"},
                {"name":"Frozen future","version":"partition.v14","contract":"event-time-disjoint","sha256":"cccccccccccc3333"},
                {"name":"Typed DSL + verifier","version":"registry.v6","contract":"typed-v6","sha256":"dddddddddddd4444"},
                {"name":"Admission","version":"gate.v2","contract":"proof-carrying","sha256":"eeeeeeeeeeee5555"},
                {"name":"Rust serving","version":"0.1.0","contract":"runtime-v6","sha256":"ffffffffffff6666"}
            ]
        });
        let html = signal_architecture_html(
            &SignalArchitectureView {
                partition: 14,
                generation: 4,
                transitions: 14_878,
                support: 32,
                matching: 55,
                matching_sessions: 7,
                after_watermark: 22,
                independent: 18,
                consistent: 18,
                routed: 18,
                future: 0,
                lost: 18,
                blocker: "future_rows_below_32",
                online_emitted: 0,
                online_blocked: 10,
                response_active: 0,
                cpu_share: "0.7%",
                online_ready: true,
            },
            &manifest,
        );

        let route = html.find("data-stage=\"future-route\"").expect("route");
        let failure = html
            .find("data-edge=\"route-to-future\"")
            .expect("failure edge");
        let storage = html.find("data-stage=\"future-store\"").expect("storage");
        assert!(route < failure && failure < storage);
        assert!(html.contains("18 routed -> 0 записано; потеряно 18"));
        assert!(html.contains("future_rows_below_32"));
        assert!(html.contains("partition.v14"));
        assert!(html.contains("data-stage=\"admission\""));
        assert!(html.contains("state-chip locked\">LOCKED"));
    }

    #[test]
    fn architecture_tree_does_not_invent_failure_when_route_is_persisted() {
        let html = signal_architecture_html(
            &SignalArchitectureView {
                partition: 14,
                generation: 5,
                transitions: 20_000,
                support: 32,
                matching: 40,
                matching_sessions: 8,
                after_watermark: 32,
                independent: 32,
                consistent: 32,
                routed: 32,
                future: 32,
                lost: 0,
                blocker: "none",
                online_emitted: 1,
                online_blocked: 0,
                response_active: 1,
                cpu_share: "50.0%",
                online_ready: true,
            },
            &Value::Null,
        );

        assert!(!html.contains("route-to-future"));
        assert!(!html.contains("BLOCK НА ЭТОМ РЕБРЕ"));
        assert!(html.contains("data-signal-status=\"pass\""));
        assert!(html.contains("MISSING"));
    }

    #[test]
    fn watchdog_auto_promotes_only_eligible_shadow_mode() {
        assert!(should_auto_promote(GatewayMode::Shadow, true));
        assert!(!should_auto_promote(GatewayMode::Shadow, false));
        assert!(!should_auto_promote(GatewayMode::Bypass, true));
        assert!(!should_auto_promote(GatewayMode::Cpu, true));
    }
}
