use super::evaluation;
use super::model::{OperatorSummary, SignalPathInputs};
use crate::{
    AppState, COLD_LEARNING_HEALTH_URL, HOT_SERVING_HEALTH_URL, HOT_SERVING_RUNTIME_HEALTH_URL,
    authorized, exact_miner_observed_tokens, exact_token_totals, metric_str, metric_u64, not_found,
    read_json, read_live_json, read_live_miner_report, unix_now,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use nando_gateway_control::{admission_status, read_state, service_statuses};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) async fn control_signal_path(
    Path(key): Path<String>,
    State(state): State<AppState>,
) -> Response {
    if !authorized(&state, &key) {
        return not_found().await;
    }

    let current = read_state(&state.config.state_path);
    let admission = admission_status(&state.config);
    let fallback_economics = read_json(&state.config.economics_path);
    let persisted_miner = read_json(&state.config.response_online_miner_report_path);
    let registry = read_json(&state.config.response_registry_path);
    let controller = read_json(&state.config.response_admission_controller_report_path);
    let services = service_statuses();
    let windows = crate::client_connections::snapshot();
    let (live, hot_health, runtime_health, cold_health) = tokio::join!(
        read_live_miner_report(),
        read_live_json(HOT_SERVING_HEALTH_URL),
        read_live_json(HOT_SERVING_RUNTIME_HEALTH_URL),
        read_live_json(COLD_LEARNING_HEALTH_URL),
    );
    let economics = live
        .get("economics")
        .filter(|value| value.is_object())
        .unwrap_or(&fallback_economics);
    let token_totals = exact_token_totals(economics);
    let bridge = crate::live_dashboard::bridge_view(&hot_health, &cold_health);
    let now_seconds = unix_now();
    let now_millis = unix_now_ms();
    let economics_age_seconds = age_seconds(economics, "generated_at_unix", now_seconds);
    let controller_age_seconds = age_seconds(&controller, "generated_at_unix", now_seconds);
    let serving_sample_age_seconds = hot_health
        .get("sampled_at_unix_ms")
        .and_then(Value::as_u64)
        .map(|sampled_at| now_millis.saturating_sub(sampled_at) / 1_000);
    let serving_service_active = services
        .iter()
        .any(|service| service.unit == "nando-transition-serving.service" && service.active);
    let false_accepts = metric_u64(economics, "false_accepts").max(bridge.false_accepts);
    let runtime_parity_failures =
        metric_u64(economics, "runtime_parity_mismatches").max(bridge.parity_mismatches);

    let snapshot = evaluation::build(SignalPathInputs {
        generated_at_unix_ms: now_millis,
        windows,
        gateway_mode: current.mode.to_string(),
        gateway_route_ready: admission.route_ready,
        kill_switch_present: admission.kill_switch_present,
        kill_switch_check_error: admission.kill_switch_check_error.clone(),
        serving_service_active,
        serving_health_ok: hot_health.get("ok").and_then(Value::as_bool) == Some(true),
        serving_instance_id_sha256: hot_health
            .pointer("/process/instance_id_sha256")
            .and_then(Value::as_str)
            .map(str::to_owned),
        serving_sample_age_seconds,
        serving_response_executor_ready: runtime_health
            .get("response_executor_cache_ready")
            .and_then(Value::as_bool)
            == Some(true),
        serving_response_local_accept_enabled: runtime_health
            .get("response_effective_local_accept_enabled")
            .and_then(Value::as_bool)
            == Some(true),
        serving_response_active_packages: metric_u64(&runtime_health, "response_active_profiles"),
        serving_response_admission_seconds_remaining: runtime_health
            .get("response_admission_seconds_remaining")
            .and_then(Value::as_u64),
        admission_cpu_allowed: admission.cpu_allowed,
        admission_eligible: admission.eligible_for_local_accept,
        admission_fresh: admission.fresh,
        controller_verdict: metric_str(&controller, "verdict", "MISSING").to_owned(),
        controller_diagnostic: metric_str(&controller, "blocker", "none").to_owned(),
        controller_active_packages: metric_u64(&controller, "active_packages"),
        controller_age_seconds,
        controller_max_age_seconds: state.config.response_controller_report_max_age_seconds,
        operators: active_operator_summaries(&registry),
        total_input_tokens: token_totals.map(|(total, _)| total),
        miner_input_tokens: exact_miner_observed_tokens(&live, &persisted_miner),
        cpu_input_tokens: token_totals.map(|(_, cpu)| cpu),
        verified_local_accepts: metric_u64(economics, "actual_local_accepts")
            .max(metric_u64(economics, "verified_local_accepts")),
        economics_age_seconds,
        false_accepts,
        runtime_parity_failures,
        bridge_failures: bridge.failures,
    });

    ([(header::CACHE_CONTROL, "no-store")], Json(snapshot)).into_response()
}

fn active_operator_summaries(registry: &Value) -> Vec<OperatorSummary> {
    let mut operators = registry
        .get("packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|package| package.get("state").and_then(Value::as_str) == Some("active"))
        .filter_map(|package| {
            let package_id = package.get("package_id")?.as_str()?.to_owned();
            let function_name = package
                .pointer("/program/operation/function_name")?
                .as_str()?
                .to_owned();
            let proof = package.get("proof").unwrap_or(&Value::Null);
            Some(OperatorSummary {
                package_id,
                function_name,
                origin: metric_str(package, "origin", "unknown").to_owned(),
                state: "active".to_owned(),
                support_rows: metric_u64(proof, "support_rows"),
                future_rows: metric_u64(proof, "future_rows"),
                distinct_sessions: metric_u64(proof, "distinct_sessions"),
                wrong_accepts: metric_u64(proof, "wrong_accepts"),
                runtime_parity_failures: metric_u64(proof, "runtime_parity_failures"),
            })
        })
        .collect::<Vec<_>>();
    operators.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    operators
}

fn age_seconds(value: &Value, field: &str, now: u64) -> Option<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .map(|generated_at| now.saturating_sub(generated_at))
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}
