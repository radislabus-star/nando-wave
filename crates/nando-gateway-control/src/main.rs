use axum::extract::{Form, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
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
    let economics = read_json(&state.config.economics_path);
    let response_registry = read_json(&state.config.response_registry_path);
    let online_miner = read_json(&state.config.response_online_miner_report_path);
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
    let cpu_disabled = if admission.cpu_allowed {
        ""
    } else {
        " disabled"
    };
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
    let online_generated_at = online_miner
        .get("generated_at_unix_ms")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        / 1_000;
    let online_age = unix_now().saturating_sub(online_generated_at);
    let online_status = if online_miner.get("schema").and_then(Value::as_str)
        == Some("nando.embedded-response-online-miner.v1")
    {
        "READY"
    } else {
        "MISSING"
    };
    let self_training = online_miner
        .pointer("/miner/self_training_v2")
        .unwrap_or(&Value::Null);
    let online_discovery = self_training.get("discovery").unwrap_or(&Value::Null);
    let online_cegis = self_training.get("cegis").unwrap_or(&Value::Null);
    let online_opportunity = self_training.get("opportunity").unwrap_or(&Value::Null);
    let strongest_generation = self_training
        .get("generations")
        .and_then(Value::as_array)
        .and_then(|generations| {
            generations
                .iter()
                .max_by_key(|generation| metric_u64(generation, "future_rows"))
        })
        .unwrap_or(&Value::Null);
    let online_generation = format!(
        "support {} / future {} / sessions {} / wrong {} / parity {}",
        metric_u64(strongest_generation, "support_rows"),
        metric_u64(strongest_generation, "future_rows"),
        metric_u64(strongest_generation, "future_sessions"),
        metric_u64(strongest_generation, "wrong_future_rows"),
        metric_u64(strongest_generation, "runtime_parity_rows")
    );
    let online_teacher_programs = teacher_programs_text(online_discovery);
    let online_candidates = online_miner
        .pointer("/miner/candidate_bucket_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let online_admission_ready = metric_u64(self_training, "admission_ready_cohorts");
    let online_warm_bytes = online_miner
        .pointer("/miner/warm_bytes_estimate")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let online_product_window = format!(
        "{} intents / {} tokens / {}",
        metric_u64(online_opportunity, "verified_intents"),
        metric_u64(online_opportunity, "verified_tokens"),
        format_ratio_milli(metric_u64(online_opportunity, "verified_token_share_milli"))
    );
    let false_accepts = metric_u64(runtime_admission, "global_false_accepts");
    let parity_mismatches = metric_u64(runtime_admission, "global_runtime_parity_mismatches");
    let policy_version = metric_str(runtime_admission, "policy_version", "MISSING");
    let body = format!(
        r#"<!doctype html>
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="refresh" content="10">
<title>Nando Gateway</title>
<style>
:root {{ color-scheme: light; font-family: Inter, system-ui, sans-serif; background:#f4f5f7; color:#17191c; }}
* {{ box-sizing:border-box; }}
body {{ margin:0; }}
header {{ background:#101214; color:#fff; padding:18px 24px; display:flex; justify-content:space-between; align-items:baseline; gap:16px; }}
h1 {{ margin:0; font-size:20px; letter-spacing:0; }}
.mode {{ font:700 14px ui-monospace,monospace; color:#76e39d; }}
main {{ width:min(1120px,100%); margin:0 auto; padding:18px 24px; }}
.controls {{ display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:10px; margin:0 0 12px; }}
button {{ width:100%; min-height:48px; border:1px solid #aeb4bc; border-radius:6px; background:#fff; color:#17191c; font-weight:750; cursor:pointer; }}
button:hover {{ border-color:#101214; }}
.bypass {{ background:#b42318; border-color:#b42318; color:#fff; }}
button:disabled {{ opacity:.45; cursor:not-allowed; }}
.metric-grid {{ display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); column-gap:32px; }}
.band {{ border-top:1px solid #d7dbe0; padding:12px 0; min-width:0; }}
h2 {{ margin:0 0 12px; font-size:15px; letter-spacing:0; }}
table {{ width:100%; border-collapse:collapse; font-size:13px; }}
td {{ padding:6px 0; border-bottom:1px solid #e1e4e8; overflow-wrap:anywhere; vertical-align:top; }}
td:first-child {{ padding-right:16px; }}
td:last-child {{ text-align:right; font-family:ui-monospace,monospace; font-weight:700; }}
.ok {{ color:#067647; }} .off {{ color:#b42318; }}
.note {{ margin:0; color:#535862; line-height:1.5; overflow-wrap:anywhere; }}
@media (max-width:760px) {{ .controls,.metric-grid {{ grid-template-columns:1fr; }} header {{ align-items:flex-start; flex-direction:column; }} main {{ padding:14px 18px; }} }}
</style>
</head>
<body>
<header><h1>Nando Gateway</h1><div class="mode">{mode}</div></header>
<main>
<div class="controls">
<form method="post" action="/control/{key}/mode"><input type="hidden" name="mode" value="BYPASS"><button class="bypass">ОБХОД NANDO</button></form>
<form method="post" action="/control/{key}/mode"><input type="hidden" name="mode" value="SHADOW"><button>SHADOW</button></form>
<form method="post" action="/control/{key}/mode"><input type="hidden" name="mode" value="CPU"><button{cpu_disabled}>CPU</button></form>
</div>
<section class="band"><h2>Сервисы</h2><table>{service_rows}</table></section>
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
	<tr><td>Teacher transitions / pools</td><td>{online_transitions} / {online_teacher_pools}</td></tr>
	<tr><td>Teacher programs</td><td>{online_teacher_programs}</td></tr>
	<tr><td>CEGIS cohorts / winners / exact checks</td><td>{online_cegis_cohorts} / {online_cegis_winners} / {online_exact_checks}</td></tr>
	<tr><td>Candidates / admission-ready</td><td>{online_candidates} / {online_admission_ready}</td></tr>
	<tr><td>Лучшее frozen generation</td><td>{online_generation}</td></tr>
	<tr><td>Горячее состояние</td><td>{online_warm_bytes} Б</td></tr>
	<tr><td>False accepts / parity failures</td><td>{online_false_accepts} / {online_parity_failures}</td></tr>
	<tr><td>Текущее verified-окно</td><td>{online_product_window}</td></tr>
	</table></section>
	</div>
<section class="band"><h2>Граница</h2><p class="note">{reason}. Кнопка BYPASS останавливает Nando-наблюдение и майнер, но Nginx продолжает передавать Codex-трафик в OpenAI.</p></section>
</main>
</body>
</html>"#,
        mode = current.mode,
        key = html_escape(&key),
        cpu_disabled = cpu_disabled,
        service_rows = service_rows,
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
        online_transitions = metric_u64(self_training, "transitions_seen"),
        online_teacher_pools = metric_u64(online_discovery, "teacher_pool_count"),
        online_teacher_programs = html_escape(&online_teacher_programs),
        online_cegis_cohorts = metric_u64(online_cegis, "cohorts"),
        online_cegis_winners = metric_u64(online_cegis, "winners"),
        online_exact_checks = metric_u64(online_cegis, "exact_checks"),
        online_candidates = online_candidates,
        online_admission_ready = online_admission_ready,
        online_generation = html_escape(&online_generation),
        online_warm_bytes = online_warm_bytes,
        online_false_accepts = metric_u64(online_opportunity, "false_accepts"),
        online_parity_failures = metric_u64(online_opportunity, "parity_failures"),
        online_product_window = html_escape(&online_product_window),
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
    }))
    .into_response()
}

fn read_json(path: &std::path::Path) -> Value {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null)
}

fn metric_u64(metrics: &Value, key: &str) -> u64 {
    metrics.get(key).and_then(Value::as_u64).unwrap_or(0)
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
    fn watchdog_auto_promotes_only_eligible_shadow_mode() {
        assert!(should_auto_promote(GatewayMode::Shadow, true));
        assert!(!should_auto_promote(GatewayMode::Shadow, false));
        assert!(!should_auto_promote(GatewayMode::Bypass, true));
        assert!(!should_auto_promote(GatewayMode::Cpu, true));
    }

    #[test]
    fn package_generations_do_not_mix_grounded_and_legacy() {
        let packages = vec![
            json!({"package_id":"raw-phase-grounded-r4-a", "state":"quarantine"}),
            json!({"package_id":"raw-phase-wait-v1", "state":"active"}),
        ];
        assert_eq!(response_package_generation_counts(&packages), (0, 1, 1, 0));
    }

    #[test]
    fn empty_future_receipts_are_not_evaluated() {
        let status = json!({
            "verifier_coverage": {
                "state": "NOT_EVALUATED",
                "required": 0,
                "emitted": 0,
                "accepted": 0,
                "missing": 0
            }
        });
        assert!(verifier_coverage_text(&status).starts_with("NOT_EVALUATED:"));
    }

    #[test]
    fn one_of_routing_predicate_lists_all_learned_bands() {
        let routing_split = json!({
            "selected_predicates": [{
                "role": "turn_message_count_band",
                "comparison": "one_of",
                "threshold": 0,
                "allowed_counts": [0, 1, 2, 4, 16]
            }]
        });

        assert_eq!(
            routing_predicates_text(&routing_split),
            "turn_message_count_band in [0,1,2,4,16]"
        );
    }

    #[test]
    fn future_eligibility_explains_every_post_freeze_stage() {
        let status = json!({
            "future_eligibility": {
                "post_freeze_rows": 12,
                "support_session_reject_rows": 2,
                "support_intent_reject_rows": 1,
                "independent_post_freeze_rows": 9,
                "reserved_session_rows": 7,
                "new_session_rows": 2,
                "route_mismatch_rows": 3,
                "routed_rows": 6,
                "verifier_accepted_rows": 5,
                "verifier_rejected_rows": 1
            }
        });

        let text = future_eligibility_text(&status);
        assert!(text.contains("post-freeze 12 -> independent 9"));
        assert!(text.contains("routed 6 / route-mismatch 3"));
        assert!(text.contains("verifier accept 5 / reject 1"));
    }
}
