use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use nando_core::{
    PhaseCenterCompiler, PhaseCenterOffloadAction, PhaseCenterOffloadDecision,
    PhaseCenterOffloadPolicy, PhaseCenterOffloadRuntime, PhaseCenterRuntimePackageInfo,
    phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

const DEFAULT_DAEMON_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-smoke.product-proof.json";
const DEFAULT_DAEMON_PACKAGE_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-package-smoke.product-proof.json";
const DEFAULT_DAEMON_HARDENING_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-hardening-smoke.product-proof.json";
const DEFAULT_DAEMON_AUTH_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-auth-smoke.product-proof.json";
const DEFAULT_DAEMON_REGISTRY_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-registry-smoke.product-proof.json";
const DEFAULT_DAEMON_REGISTRY_CONFIG: &str =
    "target/nando-wave/action-runtime-v1-daemon-registry.config.json";
const DEFAULT_DAEMON_REGISTRY_CONFIG_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-registry-config-smoke.product-proof.json";
const DEFAULT_DAEMON_RATE_LIMIT_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-rate-limit-smoke.product-proof.json";
const DEFAULT_DAEMON_OBSERVABILITY_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-observability-smoke.product-proof.json";
const DEFAULT_DAEMON_AUDIT_LOG_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.product-proof.json";
const DEFAULT_DAEMON_AUDIT_LOG_JSONL: &str =
    "target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.events.jsonl";
const DEFAULT_DAEMON_ERROR_TAXONOMY_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-error-taxonomy-smoke.product-proof.json";
const DEFAULT_DAEMON_CONFIG_VALIDATION_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-config-validation-smoke.product-proof.json";
const DEFAULT_DAEMON_PROOF_SUITE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-proof-suite.product-proof.json";
const DEFAULT_DAEMON_LIVE_PROOF_SUITE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-live-proof-suite.product-proof.json";
const DEFAULT_DAEMON_SYSTEMD_SERVICE: &str = "target/nando-wave/nando-wave-action-daemon.service";
const DEFAULT_DAEMON_SYSTEMD_ENV: &str = "target/nando-wave/nando-wave-action-daemon.env";
const DEFAULT_DAEMON_SYSTEMD_SMOKE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-systemd-smoke.product-proof.json";
const DEFAULT_DAEMON_SYSTEMD_AUDIT_LOG: &str =
    "target/nando-wave/action-runtime-v1-daemon-systemd.events.jsonl";
const DEFAULT_DAEMON_DEPLOYMENT_PACKAGE_REPORT: &str =
    "target/nando-wave/action-runtime-v1-daemon-deployment-package.product-proof.json";
const DEFAULT_GENERATED_PACKAGE: &str = "target/nando-wave/action-runtime-v1-generated-c32.nwpc";
const DEFAULT_GENERATED_MANIFEST: &str =
    "target/nando-wave/action-runtime-v1-generated-c32.nwpc.manifest.json";
const DEFAULT_GENERATED_CORPUS: &str =
    "data/rule_logic_operator_battery_v4/action_contract_v1/generated_action_contract_v1.jsonl";
const DEFAULT_DOMAIN_PACKAGE: &str =
    "target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc";
const DEFAULT_DOMAIN_MANIFEST: &str =
    "target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc.manifest.json";
const DEFAULT_DOMAIN_CORPUS: &str = "data/rule_logic_operator_battery_v4/action_contract_v1/generated_domain_action_contract_v1.jsonl";
const DEFAULT_COVERAGE_PACKAGE: &str =
    "target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc";
const DEFAULT_COVERAGE_MANIFEST: &str =
    "target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc.manifest.json";
const DEFAULT_COVERAGE_CORPUS: &str = "data/rule_logic_operator_battery_v5/action_contract_v1/generated_coverage_action_contract_v1.jsonl";
const DAEMON_SMOKE_CELLS: usize = 32;
const DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO: i64 = 300_000;
const DEFAULT_DAEMON_BIND_ADDR: &str = "127.0.0.1:8787";
const HTTP_READ_TIMEOUT_SECS: u64 = 5;
const HTTP_MAX_REQUEST_BYTES: usize = 64 * 1024;
const DAEMON_MAX_SCORE_ATOMS: usize = 1024;
const DAEMON_MAX_SCORE_ATOM_BYTES: usize = 256;
const DAEMON_AUTH_SMOKE_TOKEN: &str = "nando-wave-auth-smoke-token";
const DAEMON_RATE_LIMIT_SMOKE_MAX_SCORE_REQUESTS: usize = 1;

pub(crate) fn run_phase_action_daemon_serve_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_PACKAGE));
    let bind_addr = args
        .next()
        .unwrap_or_else(|| DEFAULT_DAEMON_BIND_ADDR.to_owned());
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    let auth_token = args.next();
    let max_score_requests = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max-score-requests '{value}': {error}"))
        })
        .transpose()?;
    let audit_log_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let runtime = load_offload_runtime_from_package(&package_path, margin_threshold_micro)?;
    let auth = PhaseActionDaemonAuth::from_optional_token(auth_token)?;
    let package_info = runtime.package_info();
    let registry = PhaseActionDaemonRuntimeRegistry::single("default", runtime)?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|error| format!("failed to bind {bind_addr}: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read listener addr: {error}"))?;
    println!("phase-action-daemon-serve-v1: serving package over HTTP");
    println!("  bind_addr: {addr}");
    println!("  package_path: {}", package_path.display());
    println!("  package_fingerprint64: {}", package_info.fingerprint64);
    println!("  package_cells: {}", package_info.cells);
    println!("  package_record_count: {}", package_info.record_count);
    println!("  margin_threshold_micro: {margin_threshold_micro}");
    println!("  endpoint: POST /score");
    println!("  endpoint: GET /health");
    println!("  endpoint: GET /stats");
    println!("  auth_enabled: {}", auth.is_enabled());
    println!(
        "  max_score_requests: {}",
        max_score_requests
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unbounded".to_owned())
    );
    println!(
        "  audit_log_path: {}",
        audit_log_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "disabled".to_owned())
    );
    println!("  runtime_path: nando_core::PhaseCenterOffloadRuntime");
    println!("  corpus_jsonl_used: false");
    println!("  python_demo_used: false");

    let stats = serve_score_requests(
        listener,
        Arc::new(registry),
        Arc::new(auth),
        PhaseActionDaemonServeConfig {
            request_limit: None,
            max_score_requests,
            server_runtime_config_used: false,
            audit_log_path,
        },
    )?;
    println!("phase-action-daemon-serve-v1: stopped");
    println!("  requests_handled: {}", stats.requests_handled);
    println!("  bad_requests: {}", stats.bad_requests);
    Ok(())
}

pub(crate) fn run_phase_action_daemon_serve_registry_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG));
    let bind_addr = args
        .next()
        .unwrap_or_else(|| DEFAULT_DAEMON_BIND_ADDR.to_owned());
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    let auth_token = args.next();
    let max_score_requests = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max-score-requests '{value}': {error}"))
        })
        .transpose()?;
    let audit_log_path = args.next().map(PathBuf::from);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let registry = load_registry_from_config(&config_path, margin_threshold_micro)?;
    let auth = PhaseActionDaemonAuth::from_optional_token(auth_token)?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|error| format!("failed to bind {bind_addr}: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read listener addr: {error}"))?;
    println!("phase-action-daemon-serve-registry-v1: serving registry over HTTP");
    println!("  bind_addr: {addr}");
    println!("  registry_config_path: {}", config_path.display());
    println!("  package_count: {}", registry.package_count());
    println!(
        "  package_aliases: {}",
        registry
            .package_summaries()
            .iter()
            .map(|package| package.alias.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  margin_threshold_micro: {margin_threshold_micro}");
    println!("  endpoint: POST /score");
    println!("  endpoint: GET /health");
    println!("  endpoint: GET /packages");
    println!("  endpoint: GET /stats");
    println!("  auth_enabled: {}", auth.is_enabled());
    println!(
        "  max_score_requests: {}",
        max_score_requests
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unbounded".to_owned())
    );
    println!(
        "  audit_log_path: {}",
        audit_log_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "disabled".to_owned())
    );
    println!("  runtime_path: nando_core::PhaseCenterOffloadRuntime");
    println!("  server_runtime_config_used: true");
    println!("  server_runtime_compiler_used: false");
    println!("  server_runtime_corpus_jsonl_used: false");
    println!("  python_demo_used: false");

    let stats = serve_score_requests(
        listener,
        Arc::new(registry),
        Arc::new(auth),
        PhaseActionDaemonServeConfig {
            request_limit: None,
            max_score_requests,
            server_runtime_config_used: true,
            audit_log_path,
        },
    )?;
    println!("phase-action-daemon-serve-registry-v1: stopped");
    println!("  requests_handled: {}", stats.requests_handled);
    println!("  bad_requests: {}", stats.bad_requests);
    Ok(())
}

pub(crate) fn run_phase_action_daemon_package_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_MANIFEST));
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_CORPUS));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_PACKAGE_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-package-smoke-v1: loading existing package");
    let runtime = load_offload_runtime_from_package(&package_path, margin_threshold_micro)?;
    let package_info = runtime.package_info();
    let runtime_bytes_estimate = runtime.bytes_estimate();
    let manifest = load_action_package_manifest_fixture(&manifest_path)?;
    if manifest.package_fingerprint64 != package_info.fingerprint64 {
        return Err(format!(
            "manifest/package fingerprint mismatch: manifest={} package={}",
            manifest.package_fingerprint64, package_info.fingerprint64
        ));
    }
    if manifest.cells != package_info.cells || manifest.flat_records != package_info.record_count {
        return Err(format!(
            "manifest/package shape mismatch: manifest cells/records={}/{} package cells/records={}/{}",
            manifest.cells, manifest.flat_records, package_info.cells, package_info.record_count
        ));
    }

    println!("phase-action-daemon-package-smoke-v1: building request fixture from corpus");
    let fixture = load_first_heldout_fixture(&corpus_path, &manifest.operator_keys)?;
    let server = PhaseActionDaemonSmokeServer::start(runtime, 2)?;

    println!(
        "phase-action-daemon-package-smoke-v1: POST /score local candidate, center_index={}",
        fixture.center_index
    );
    let local_response = post_score(server.addr, &fixture.local_request)?;
    println!("phase-action-daemon-package-smoke-v1: POST /score fallback candidate");
    let fallback_response = post_score(server.addr, &fixture.fallback_request)?;
    let serve_stats = server.join()?;

    let local_pass = local_response.status_code == 200
        && local_response.body.action == "local_operator"
        && local_response.body.margin_micro >= margin_threshold_micro;
    let fallback_pass =
        fallback_response.status_code == 200 && fallback_response.body.action == "fallback_to_llm";
    let false_local_accepts = usize::from(
        local_response.body.margin_micro <= 0 && local_response.body.action == "local_operator",
    ) + usize::from(
        fallback_response.body.margin_micro <= 0
            && fallback_response.body.action == "local_operator",
    );
    let pass = local_pass
        && fallback_pass
        && false_local_accepts == 0
        && serve_stats.requests_handled == 2
        && serve_stats.bad_requests == 0;

    let report = PhaseActionDaemonPackageSmokeReport {
        schema: "nando_phase_action_daemon_package_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_PACKAGE_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_PACKAGE_SMOKE_V1_FAIL"
        },
        boundary: "HTTP service smoke over an existing .nwpc package; corpus JSONL is used only to build one request fixture, not by the server runtime",
        package_path: package_path.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        corpus_path: corpus_path.display().to_string(),
        package_fingerprint64: package_info.fingerprint64,
        package_cells: package_info.cells,
        package_record_count: package_info.record_count,
        package_serialized_len: package_info.serialized_len,
        runtime_bytes_estimate,
        margin_threshold_micro,
        fixture_task_id: fixture.task_id,
        fixture_center_index: fixture.center_index,
        fixture_operator_key: fixture.operator_key,
        http_requests: 2,
        http_requests_handled: serve_stats.requests_handled,
        http_bad_requests: serve_stats.bad_requests,
        local_operator_calls: usize::from(local_response.body.action == "local_operator"),
        fallback_to_llm_calls: usize::from(fallback_response.body.action == "fallback_to_llm"),
        false_local_accepts,
        local_status_code: local_response.status_code,
        fallback_status_code: fallback_response.status_code,
        local_margin_micro: local_response.body.margin_micro,
        fallback_margin_micro: fallback_response.body.margin_micro,
        local_action: local_response.body.action,
        fallback_action: fallback_response.body.action,
        request_fixture_corpus_jsonl_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!("phase-action-daemon-package-smoke-v1: {}", report.verdict);
    println!("  report: {}", report_path.display());
    println!("  fixture_task_id: {}", report.fixture_task_id);
    println!("  local_action: {}", report.local_action);
    println!("  fallback_action: {}", report.fallback_action);
    println!("  false_local_accepts: {}", report.false_local_accepts);

    if pass {
        Ok(())
    } else {
        Err("daemon package smoke did not satisfy local/fallback service gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_hardening_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_MANIFEST));
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_CORPUS));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_HARDENING_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-hardening-smoke-v1: loading existing package");
    let runtime = load_offload_runtime_from_package(&package_path, margin_threshold_micro)?;
    let package_info = runtime.package_info();
    let runtime_bytes_estimate = runtime.bytes_estimate();
    let manifest = load_action_package_manifest_fixture(&manifest_path)?;
    if manifest.package_fingerprint64 != package_info.fingerprint64 {
        return Err(format!(
            "manifest/package fingerprint mismatch: manifest={} package={}",
            manifest.package_fingerprint64, package_info.fingerprint64
        ));
    }
    if manifest.cells != package_info.cells || manifest.flat_records != package_info.record_count {
        return Err(format!(
            "manifest/package shape mismatch: manifest cells/records={}/{} package cells/records={}/{}",
            manifest.cells, manifest.flat_records, package_info.cells, package_info.record_count
        ));
    }

    println!("phase-action-daemon-hardening-smoke-v1: building request fixture from corpus");
    let fixture = load_first_heldout_fixture(&corpus_path, &manifest.operator_keys)?;
    let server = PhaseActionDaemonSmokeServer::start(runtime, 5)?;

    println!("phase-action-daemon-hardening-smoke-v1: GET /health");
    let health_response = get_health(server.addr)?;
    println!(
        "phase-action-daemon-hardening-smoke-v1: POST /score local candidate, center_index={}",
        fixture.center_index
    );
    let local_response = post_score(server.addr, &fixture.local_request)?;
    println!("phase-action-daemon-hardening-smoke-v1: POST /score fallback candidate");
    let fallback_response = post_score(server.addr, &fixture.fallback_request)?;
    println!("phase-action-daemon-hardening-smoke-v1: GET /bad-route");
    let bad_route_response = get_raw(server.addr, "/bad-route")?;
    println!("phase-action-daemon-hardening-smoke-v1: GET /stats");
    let stats_response = get_stats(server.addr)?;
    let serve_stats = server.join()?;

    let local_pass = local_response.status_code == 200
        && local_response.body.action == "local_operator"
        && local_response.body.margin_micro >= margin_threshold_micro;
    let fallback_pass =
        fallback_response.status_code == 200 && fallback_response.body.action == "fallback_to_llm";
    let false_local_accepts = usize::from(
        local_response.body.margin_micro <= 0 && local_response.body.action == "local_operator",
    ) + usize::from(
        fallback_response.body.margin_micro <= 0
            && fallback_response.body.action == "local_operator",
    );
    let health_pass = health_response.status_code == 200
        && health_response.body.status == "ok"
        && health_response.body.package_fingerprint64 == package_info.fingerprint64
        && !health_response.body.server_runtime_compiler_used
        && !health_response.body.server_runtime_corpus_jsonl_used
        && !health_response.body.python_demo_used;
    let stats_pass = stats_response.status_code == 200
        && stats_response.body.score_requests == 2
        && stats_response.body.health_requests == 1
        && stats_response.body.bad_requests == 1
        && stats_response.body.local_operator_calls == 1
        && stats_response.body.fallback_to_llm_calls == 1
        && stats_response.body.false_local_accepts == 0;
    let pass = health_pass
        && local_pass
        && fallback_pass
        && bad_route_response.status_code == 404
        && bad_route_response.body.contains("unsupported HTTP route")
        && stats_pass
        && false_local_accepts == 0
        && serve_stats.requests_handled == 4
        && serve_stats.score_requests == 2
        && serve_stats.health_requests == 1
        && serve_stats.stats_requests == 1
        && serve_stats.bad_requests == 1
        && serve_stats.local_operator_calls == 1
        && serve_stats.fallback_to_llm_calls == 1
        && serve_stats.false_local_accepts == 0;

    let report = PhaseActionDaemonHardeningSmokeReport {
        schema: "nando_phase_action_daemon_hardening_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_HARDENING_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_HARDENING_SMOKE_V1_FAIL"
        },
        boundary: "HTTP hardening smoke over an existing .nwpc package: health, stats, bounded request size, route errors, and local/fallback counters; not auth/TLS, service manager, multi-package registry, or real pilot traffic",
        package_path: package_path.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        corpus_path: corpus_path.display().to_string(),
        package_fingerprint64: package_info.fingerprint64,
        package_cells: package_info.cells,
        package_record_count: package_info.record_count,
        package_serialized_len: package_info.serialized_len,
        runtime_bytes_estimate,
        margin_threshold_micro,
        http_max_request_bytes: HTTP_MAX_REQUEST_BYTES,
        max_score_atoms: DAEMON_MAX_SCORE_ATOMS,
        max_score_atom_bytes: DAEMON_MAX_SCORE_ATOM_BYTES,
        fixture_task_id: fixture.task_id,
        fixture_center_index: fixture.center_index,
        fixture_operator_key: fixture.operator_key,
        health_status_code: health_response.status_code,
        stats_status_code: stats_response.status_code,
        bad_route_status_code: bad_route_response.status_code,
        local_status_code: local_response.status_code,
        fallback_status_code: fallback_response.status_code,
        local_action: local_response.body.action,
        fallback_action: fallback_response.body.action,
        local_margin_micro: local_response.body.margin_micro,
        fallback_margin_micro: fallback_response.body.margin_micro,
        http_requests: 5,
        http_requests_handled: serve_stats.requests_handled,
        http_score_requests: serve_stats.score_requests,
        http_health_requests: serve_stats.health_requests,
        http_stats_requests: serve_stats.stats_requests,
        http_bad_requests: serve_stats.bad_requests,
        local_operator_calls: serve_stats.local_operator_calls,
        fallback_to_llm_calls: serve_stats.fallback_to_llm_calls,
        false_local_accepts: serve_stats.false_local_accepts,
        request_fixture_corpus_jsonl_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!("phase-action-daemon-hardening-smoke-v1: {}", report.verdict);
    println!("  report: {}", report_path.display());
    println!("  health_status_code: {}", report.health_status_code);
    println!("  stats_status_code: {}", report.stats_status_code);
    println!("  bad_route_status_code: {}", report.bad_route_status_code);
    println!("  false_local_accepts: {}", report.false_local_accepts);

    if pass {
        Ok(())
    } else {
        Err("daemon hardening smoke did not satisfy service gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_auth_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_PACKAGE));
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_MANIFEST));
    let corpus_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_COVERAGE_CORPUS));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_AUTH_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-auth-smoke-v1: loading existing package");
    let runtime = load_offload_runtime_from_package(&package_path, margin_threshold_micro)?;
    let package_info = runtime.package_info();
    let runtime_bytes_estimate = runtime.bytes_estimate();
    let manifest = load_action_package_manifest_fixture(&manifest_path)?;
    if manifest.package_fingerprint64 != package_info.fingerprint64 {
        return Err(format!(
            "manifest/package fingerprint mismatch: manifest={} package={}",
            manifest.package_fingerprint64, package_info.fingerprint64
        ));
    }
    if manifest.cells != package_info.cells || manifest.flat_records != package_info.record_count {
        return Err(format!(
            "manifest/package shape mismatch: manifest cells/records={}/{} package cells/records={}/{}",
            manifest.cells, manifest.flat_records, package_info.cells, package_info.record_count
        ));
    }

    println!("phase-action-daemon-auth-smoke-v1: building request fixture from corpus");
    let fixture = load_first_heldout_fixture(&corpus_path, &manifest.operator_keys)?;
    let auth =
        PhaseActionDaemonAuth::from_optional_token(Some(DAEMON_AUTH_SMOKE_TOKEN.to_owned()))?;
    let server = PhaseActionDaemonSmokeServer::start_with_auth(runtime, auth, 5)?;

    println!("phase-action-daemon-auth-smoke-v1: GET /health without token");
    let health_response = get_health(server.addr)?;
    println!("phase-action-daemon-auth-smoke-v1: POST /score without token");
    let unauthorized_score_response = post_score_raw(server.addr, &fixture.local_request, None)?;
    println!("phase-action-daemon-auth-smoke-v1: POST /score with token");
    let local_response = post_score_with_auth(
        server.addr,
        &fixture.local_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-auth-smoke-v1: POST /score fallback with token");
    let fallback_response = post_score_with_auth(
        server.addr,
        &fixture.fallback_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-auth-smoke-v1: GET /stats with token");
    let stats_response = get_stats_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    let serve_stats = server.join()?;

    let local_pass = local_response.status_code == 200
        && local_response.body.action == "local_operator"
        && local_response.body.margin_micro >= margin_threshold_micro;
    let fallback_pass =
        fallback_response.status_code == 200 && fallback_response.body.action == "fallback_to_llm";
    let health_pass = health_response.status_code == 200
        && health_response.body.status == "ok"
        && health_response.body.package_fingerprint64 == package_info.fingerprint64;
    let unauthorized_pass = unauthorized_score_response.status_code == 401
        && unauthorized_score_response
            .body
            .contains("missing or invalid bearer token");
    let stats_pass = stats_response.status_code == 200
        && stats_response.body.score_requests == 2
        && stats_response.body.health_requests == 1
        && stats_response.body.stats_requests == 0
        && stats_response.body.bad_requests == 1
        && stats_response.body.local_operator_calls == 1
        && stats_response.body.fallback_to_llm_calls == 1
        && stats_response.body.false_local_accepts == 0;
    let pass = health_pass
        && unauthorized_pass
        && local_pass
        && fallback_pass
        && stats_pass
        && serve_stats.requests_handled == 4
        && serve_stats.score_requests == 2
        && serve_stats.health_requests == 1
        && serve_stats.stats_requests == 1
        && serve_stats.bad_requests == 1
        && serve_stats.local_operator_calls == 1
        && serve_stats.fallback_to_llm_calls == 1
        && serve_stats.false_local_accepts == 0;

    let report = PhaseActionDaemonAuthSmokeReport {
        schema: "nando_phase_action_daemon_auth_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_AUTH_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_AUTH_SMOKE_V1_FAIL"
        },
        boundary: "HTTP bearer-auth smoke over an existing .nwpc package: /health is public, /score and /stats require Authorization bearer token; not TLS, service manager, multi-package registry, or real pilot traffic",
        package_path: package_path.display().to_string(),
        manifest_path: manifest_path.display().to_string(),
        corpus_path: corpus_path.display().to_string(),
        package_fingerprint64: package_info.fingerprint64,
        package_cells: package_info.cells,
        package_record_count: package_info.record_count,
        package_serialized_len: package_info.serialized_len,
        runtime_bytes_estimate,
        margin_threshold_micro,
        fixture_task_id: fixture.task_id,
        fixture_center_index: fixture.center_index,
        fixture_operator_key: fixture.operator_key,
        auth_enabled: true,
        health_public_status_code: health_response.status_code,
        unauthorized_score_status_code: unauthorized_score_response.status_code,
        authorized_score_status_code: local_response.status_code,
        authorized_fallback_status_code: fallback_response.status_code,
        authorized_stats_status_code: stats_response.status_code,
        local_action: local_response.body.action,
        fallback_action: fallback_response.body.action,
        local_margin_micro: local_response.body.margin_micro,
        fallback_margin_micro: fallback_response.body.margin_micro,
        http_requests: 5,
        http_requests_handled: serve_stats.requests_handled,
        http_score_requests: serve_stats.score_requests,
        http_health_requests: serve_stats.health_requests,
        http_stats_requests: serve_stats.stats_requests,
        http_bad_requests: serve_stats.bad_requests,
        local_operator_calls: serve_stats.local_operator_calls,
        fallback_to_llm_calls: serve_stats.fallback_to_llm_calls,
        false_local_accepts: serve_stats.false_local_accepts,
        request_fixture_corpus_jsonl_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!("phase-action-daemon-auth-smoke-v1: {}", report.verdict);
    println!("  report: {}", report_path.display());
    println!(
        "  unauthorized_score_status_code: {}",
        report.unauthorized_score_status_code
    );
    println!(
        "  authorized_score_status_code: {}",
        report.authorized_score_status_code
    );
    println!(
        "  authorized_stats_status_code: {}",
        report.authorized_stats_status_code
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);

    if pass {
        Ok(())
    } else {
        Err("daemon auth smoke did not satisfy bearer-auth service gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_registry_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-registry-smoke-v1: loading generated_action package");
    let generated_manifest =
        load_action_package_manifest_fixture(Path::new(DEFAULT_GENERATED_MANIFEST))?;
    let generated_runtime = load_offload_runtime_from_package(
        Path::new(DEFAULT_GENERATED_PACKAGE),
        margin_threshold_micro,
    )?;
    verify_manifest_matches_runtime("generated_action", &generated_manifest, &generated_runtime)?;
    let generated_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_GENERATED_CORPUS),
        &generated_manifest.operator_keys,
    )?;

    println!("phase-action-daemon-registry-smoke-v1: loading coverage_action package");
    let coverage_manifest =
        load_action_package_manifest_fixture(Path::new(DEFAULT_COVERAGE_MANIFEST))?;
    let coverage_runtime = load_offload_runtime_from_package(
        Path::new(DEFAULT_COVERAGE_PACKAGE),
        margin_threshold_micro,
    )?;
    verify_manifest_matches_runtime("coverage_action", &coverage_manifest, &coverage_runtime)?;
    let coverage_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_COVERAGE_CORPUS),
        &coverage_manifest.operator_keys,
    )?;

    println!("phase-action-daemon-registry-smoke-v1: loading domain_action package");
    let domain_manifest = load_action_package_manifest_fixture(Path::new(DEFAULT_DOMAIN_MANIFEST))?;
    let domain_runtime = load_offload_runtime_from_package(
        Path::new(DEFAULT_DOMAIN_PACKAGE),
        margin_threshold_micro,
    )?;
    verify_manifest_matches_runtime("domain_action", &domain_manifest, &domain_runtime)?;
    let domain_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_DOMAIN_CORPUS),
        &domain_manifest.operator_keys,
    )?;

    let registry = PhaseActionDaemonRuntimeRegistry::new(vec![
        PhaseActionDaemonRuntimePackage {
            alias: "generated_action".to_owned(),
            runtime: generated_runtime,
        },
        PhaseActionDaemonRuntimePackage {
            alias: "domain_action".to_owned(),
            runtime: domain_runtime,
        },
        PhaseActionDaemonRuntimePackage {
            alias: "coverage_action".to_owned(),
            runtime: coverage_runtime,
        },
    ])?;
    let auth =
        PhaseActionDaemonAuth::from_optional_token(Some(DAEMON_AUTH_SMOKE_TOKEN.to_owned()))?;
    let server = PhaseActionDaemonSmokeServer::start_with_registry(registry, auth, 7)?;

    println!("phase-action-daemon-registry-smoke-v1: GET /health without token");
    let health_response = get_health(server.addr)?;
    println!("phase-action-daemon-registry-smoke-v1: GET /packages with token");
    let packages_response = get_packages_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-registry-smoke-v1: POST /score generated_action");
    let generated_request =
        request_with_alias(&generated_fixture.local_request, "generated_action");
    let generated_response = post_score_with_auth(
        server.addr,
        &generated_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-registry-smoke-v1: POST /score domain_action");
    let domain_request = request_with_alias(&domain_fixture.local_request, "domain_action");
    let domain_response =
        post_score_with_auth(server.addr, &domain_request, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-registry-smoke-v1: POST /score coverage_action");
    let coverage_request = request_with_alias(&coverage_fixture.local_request, "coverage_action");
    let coverage_response = post_score_with_auth(
        server.addr,
        &coverage_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-registry-smoke-v1: POST /score missing_package");
    let missing_request = request_with_alias(&coverage_fixture.local_request, "missing_package");
    let missing_response =
        post_score_raw(server.addr, &missing_request, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-registry-smoke-v1: GET /stats with token");
    let stats_response = get_stats_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    let serve_stats = server.join()?;

    let generated_pass = generated_response.status_code == 200
        && generated_response.body.package_alias == "generated_action"
        && generated_response.body.action == "local_operator"
        && generated_response.body.margin_micro >= margin_threshold_micro;
    let coverage_pass = coverage_response.status_code == 200
        && coverage_response.body.package_alias == "coverage_action"
        && coverage_response.body.action == "local_operator"
        && coverage_response.body.margin_micro >= margin_threshold_micro;
    let domain_pass = domain_response.status_code == 200
        && domain_response.body.package_alias == "domain_action"
        && domain_response.body.action == "local_operator"
        && domain_response.body.margin_micro >= margin_threshold_micro;
    let missing_pass = missing_response.status_code == 404
        && missing_response
            .body
            .contains("unknown package alias: missing_package");
    let packages_pass = packages_response.status_code == 200
        && packages_response.body.package_count == 3
        && packages_response
            .body
            .packages
            .iter()
            .any(|package| package.alias == "generated_action")
        && packages_response
            .body
            .packages
            .iter()
            .any(|package| package.alias == "domain_action")
        && packages_response
            .body
            .packages
            .iter()
            .any(|package| package.alias == "coverage_action")
        && !packages_response.body.server_runtime_compiler_used
        && !packages_response.body.server_runtime_corpus_jsonl_used
        && !packages_response.body.python_demo_used;
    let stats_pass = stats_response.status_code == 200
        && stats_response.body.package_count == 3
        && stats_response.body.score_requests == 3
        && stats_response.body.health_requests == 1
        && stats_response.body.packages_requests == 1
        && stats_response.body.bad_requests == 1
        && stats_response.body.local_operator_calls == 3
        && stats_response.body.false_local_accepts == 0;
    let pass = health_response.status_code == 200
        && health_response.body.package_count == 3
        && packages_pass
        && generated_pass
        && domain_pass
        && coverage_pass
        && missing_pass
        && stats_pass
        && serve_stats.requests_handled == 6
        && serve_stats.score_requests == 3
        && serve_stats.health_requests == 1
        && serve_stats.packages_requests == 1
        && serve_stats.stats_requests == 1
        && serve_stats.bad_requests == 1
        && serve_stats.local_operator_calls == 3
        && serve_stats.false_local_accepts == 0;

    let report = PhaseActionDaemonRegistrySmokeReport {
        schema: "nando_phase_action_daemon_registry_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_REGISTRY_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_REGISTRY_SMOKE_V1_FAIL"
        },
        boundary: "HTTP registry smoke over multiple existing .nwpc packages: package aliases route to loaded runtimes, /packages lists shards, unknown aliases reject; not dynamic package reload, rate limits, TLS, or real pilot traffic",
        package_aliases: packages_response
            .body
            .packages
            .iter()
            .map(|package| package.alias.clone())
            .collect(),
        package_count: packages_response.body.package_count,
        generated_package_path: DEFAULT_GENERATED_PACKAGE.to_owned(),
        domain_package_path: DEFAULT_DOMAIN_PACKAGE.to_owned(),
        coverage_package_path: DEFAULT_COVERAGE_PACKAGE.to_owned(),
        generated_package_fingerprint64: generated_response.body.package_fingerprint64,
        domain_package_fingerprint64: domain_response.body.package_fingerprint64,
        coverage_package_fingerprint64: coverage_response.body.package_fingerprint64,
        generated_fixture_task_id: generated_fixture.task_id,
        domain_fixture_task_id: domain_fixture.task_id,
        coverage_fixture_task_id: coverage_fixture.task_id,
        generated_status_code: generated_response.status_code,
        domain_status_code: domain_response.status_code,
        coverage_status_code: coverage_response.status_code,
        missing_alias_status_code: missing_response.status_code,
        packages_status_code: packages_response.status_code,
        stats_status_code: stats_response.status_code,
        health_status_code: health_response.status_code,
        generated_action: generated_response.body.action,
        domain_action: domain_response.body.action,
        coverage_action: coverage_response.body.action,
        generated_margin_micro: generated_response.body.margin_micro,
        domain_margin_micro: domain_response.body.margin_micro,
        coverage_margin_micro: coverage_response.body.margin_micro,
        http_requests: 7,
        http_requests_handled: serve_stats.requests_handled,
        http_score_requests: serve_stats.score_requests,
        http_health_requests: serve_stats.health_requests,
        http_packages_requests: serve_stats.packages_requests,
        http_stats_requests: serve_stats.stats_requests,
        http_bad_requests: serve_stats.bad_requests,
        local_operator_calls: serve_stats.local_operator_calls,
        fallback_to_llm_calls: serve_stats.fallback_to_llm_calls,
        false_local_accepts: serve_stats.false_local_accepts,
        request_fixture_corpus_jsonl_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!("phase-action-daemon-registry-smoke-v1: {}", report.verdict);
    println!("  report: {}", report_path.display());
    println!("  package_count: {}", report.package_count);
    println!("  generated_status_code: {}", report.generated_status_code);
    println!("  coverage_status_code: {}", report.coverage_status_code);
    println!(
        "  missing_alias_status_code: {}",
        report.missing_alias_status_code
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);

    if pass {
        Ok(())
    } else {
        Err("daemon registry smoke did not satisfy multi-package service gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_registry_config_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-registry-config-smoke-v1: writing registry config");
    let config = default_registry_config();
    write_json_file(&config_path, &config)?;

    println!("phase-action-daemon-registry-config-smoke-v1: loading registry from config");
    let registry = load_registry_from_config(&config_path, margin_threshold_micro)?;
    let generated_manifest =
        load_action_package_manifest_fixture(Path::new(DEFAULT_GENERATED_MANIFEST))?;
    let domain_manifest = load_action_package_manifest_fixture(Path::new(DEFAULT_DOMAIN_MANIFEST))?;
    let coverage_manifest =
        load_action_package_manifest_fixture(Path::new(DEFAULT_COVERAGE_MANIFEST))?;
    let generated_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_GENERATED_CORPUS),
        &generated_manifest.operator_keys,
    )?;
    let domain_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_DOMAIN_CORPUS),
        &domain_manifest.operator_keys,
    )?;
    let coverage_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_COVERAGE_CORPUS),
        &coverage_manifest.operator_keys,
    )?;

    let auth =
        PhaseActionDaemonAuth::from_optional_token(Some(DAEMON_AUTH_SMOKE_TOKEN.to_owned()))?;
    let server = PhaseActionDaemonSmokeServer::start_with_registry_and_config(
        registry,
        auth,
        PhaseActionDaemonServeConfig {
            request_limit: Some(7),
            max_score_requests: None,
            server_runtime_config_used: true,
            audit_log_path: None,
        },
    )?;

    println!("phase-action-daemon-registry-config-smoke-v1: GET /health without token");
    let health_response = get_health(server.addr)?;
    println!("phase-action-daemon-registry-config-smoke-v1: GET /packages with token");
    let packages_response = get_packages_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-registry-config-smoke-v1: POST /score generated_action");
    let generated_request =
        request_with_alias(&generated_fixture.local_request, "generated_action");
    let generated_response = post_score_with_auth(
        server.addr,
        &generated_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-registry-config-smoke-v1: POST /score domain_action");
    let domain_request = request_with_alias(&domain_fixture.local_request, "domain_action");
    let domain_response =
        post_score_with_auth(server.addr, &domain_request, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-registry-config-smoke-v1: POST /score coverage_action");
    let coverage_request = request_with_alias(&coverage_fixture.local_request, "coverage_action");
    let coverage_response = post_score_with_auth(
        server.addr,
        &coverage_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-registry-config-smoke-v1: POST /score missing_package");
    let missing_request = request_with_alias(&coverage_fixture.local_request, "missing_package");
    let missing_response =
        post_score_raw(server.addr, &missing_request, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-registry-config-smoke-v1: GET /stats with token");
    let stats_response = get_stats_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    let serve_stats = server.join()?;

    let generated_pass = generated_response.status_code == 200
        && generated_response.body.package_alias == "generated_action"
        && generated_response.body.action == "local_operator"
        && generated_response.body.margin_micro >= margin_threshold_micro;
    let domain_pass = domain_response.status_code == 200
        && domain_response.body.package_alias == "domain_action"
        && domain_response.body.action == "local_operator"
        && domain_response.body.margin_micro >= margin_threshold_micro;
    let coverage_pass = coverage_response.status_code == 200
        && coverage_response.body.package_alias == "coverage_action"
        && coverage_response.body.action == "local_operator"
        && coverage_response.body.margin_micro >= margin_threshold_micro;
    let packages_pass = packages_response.status_code == 200
        && packages_response.body.package_count == 3
        && packages_response
            .body
            .packages
            .iter()
            .any(|package| package.alias == "generated_action")
        && packages_response
            .body
            .packages
            .iter()
            .any(|package| package.alias == "domain_action")
        && packages_response
            .body
            .packages
            .iter()
            .any(|package| package.alias == "coverage_action")
        && !packages_response.body.server_runtime_compiler_used
        && !packages_response.body.server_runtime_corpus_jsonl_used
        && !packages_response.body.python_demo_used;
    let missing_pass = missing_response.status_code == 404
        && missing_response
            .body
            .contains("unknown package alias: missing_package");
    let stats_pass = stats_response.status_code == 200
        && stats_response.body.package_count == 3
        && stats_response.body.score_requests == 3
        && stats_response.body.packages_requests == 1
        && stats_response.body.bad_requests == 1
        && stats_response.body.local_operator_calls == 3
        && stats_response.body.false_local_accepts == 0;
    let pass = health_response.status_code == 200
        && health_response.body.package_count == 3
        && packages_pass
        && generated_pass
        && domain_pass
        && coverage_pass
        && missing_pass
        && stats_pass
        && serve_stats.requests_handled == 6
        && serve_stats.score_requests == 3
        && serve_stats.health_requests == 1
        && serve_stats.packages_requests == 1
        && serve_stats.stats_requests == 1
        && serve_stats.bad_requests == 1
        && serve_stats.local_operator_calls == 3
        && serve_stats.false_local_accepts == 0;

    let report = PhaseActionDaemonRegistryConfigSmokeReport {
        schema: "nando_phase_action_daemon_registry_config_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_REGISTRY_CONFIG_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_REGISTRY_CONFIG_SMOKE_V1_FAIL"
        },
        boundary: "HTTP registry-config smoke over existing .nwpc packages: external JSON config selects aliases/package paths/manifest paths; server loads package bytes and validates manifest parity; not dynamic reload, rate limits, TLS, or real pilot traffic",
        registry_config_path: config_path.display().to_string(),
        registry_config_written_by_smoke: true,
        package_aliases: packages_response
            .body
            .packages
            .iter()
            .map(|package| package.alias.clone())
            .collect(),
        package_count: packages_response.body.package_count,
        generated_package_fingerprint64: generated_response.body.package_fingerprint64,
        domain_package_fingerprint64: domain_response.body.package_fingerprint64,
        coverage_package_fingerprint64: coverage_response.body.package_fingerprint64,
        generated_status_code: generated_response.status_code,
        domain_status_code: domain_response.status_code,
        coverage_status_code: coverage_response.status_code,
        missing_alias_status_code: missing_response.status_code,
        packages_status_code: packages_response.status_code,
        stats_status_code: stats_response.status_code,
        health_status_code: health_response.status_code,
        generated_action: generated_response.body.action,
        domain_action: domain_response.body.action,
        coverage_action: coverage_response.body.action,
        generated_margin_micro: generated_response.body.margin_micro,
        domain_margin_micro: domain_response.body.margin_micro,
        coverage_margin_micro: coverage_response.body.margin_micro,
        http_requests: 7,
        http_requests_handled: serve_stats.requests_handled,
        http_score_requests: serve_stats.score_requests,
        http_health_requests: serve_stats.health_requests,
        http_packages_requests: serve_stats.packages_requests,
        http_stats_requests: serve_stats.stats_requests,
        http_bad_requests: serve_stats.bad_requests,
        local_operator_calls: serve_stats.local_operator_calls,
        fallback_to_llm_calls: serve_stats.fallback_to_llm_calls,
        false_local_accepts: serve_stats.false_local_accepts,
        request_fixture_corpus_jsonl_used: true,
        server_runtime_config_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!(
        "phase-action-daemon-registry-config-smoke-v1: {}",
        report.verdict
    );
    println!("  config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  package_count: {}", report.package_count);
    println!(
        "  missing_alias_status_code: {}",
        report.missing_alias_status_code
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);

    if pass {
        Ok(())
    } else {
        Err("daemon registry config smoke did not satisfy config-loaded service gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_config_validation_smoke_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_CONFIG_VALIDATION_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-config-validation-smoke-v1: writing valid registry config");
    let valid_config = default_registry_config();
    write_json_file(&config_path, &valid_config)?;
    println!("phase-action-daemon-config-validation-smoke-v1: loading valid registry config");
    let valid_registry = load_registry_from_config(&config_path, margin_threshold_micro)?;
    let valid_registry_load_pass = valid_registry.package_count() == valid_config.packages.len();

    let invalid_schema_path = derived_config_path(&config_path, "invalid-schema");
    let empty_alias_path = derived_config_path(&config_path, "empty-alias");
    let duplicate_alias_path = derived_config_path(&config_path, "duplicate-alias");
    let missing_manifest_path = derived_config_path(&config_path, "missing-manifest");
    let manifest_mismatch_path = derived_config_path(&config_path, "manifest-mismatch");

    let mut invalid_schema = valid_config.clone();
    invalid_schema.schema = "nando_phase_action_daemon_registry_config_v0".to_owned();
    write_json_file(&invalid_schema_path, &invalid_schema)?;

    let mut empty_alias = valid_config.clone();
    empty_alias.packages[0].alias = "   ".to_owned();
    write_json_file(&empty_alias_path, &empty_alias)?;

    let mut duplicate_alias = valid_config.clone();
    duplicate_alias.packages[1].alias = duplicate_alias.packages[0].alias.clone();
    write_json_file(&duplicate_alias_path, &duplicate_alias)?;

    let mut missing_manifest = valid_config.clone();
    missing_manifest.packages[0].manifest_path =
        "target/nando-wave/missing-daemon-manifest.json".to_owned();
    write_json_file(&missing_manifest_path, &missing_manifest)?;

    let mut manifest_mismatch = valid_config.clone();
    manifest_mismatch.packages[0].manifest_path = DEFAULT_COVERAGE_MANIFEST.to_owned();
    write_json_file(&manifest_mismatch_path, &manifest_mismatch)?;

    let invalid_cases = vec![
        expect_registry_config_error(
            "invalid_schema",
            &invalid_schema_path,
            margin_threshold_micro,
            "unsupported registry config schema",
        ),
        expect_registry_config_error(
            "empty_alias",
            &empty_alias_path,
            margin_threshold_micro,
            "must not be empty",
        ),
        expect_registry_config_error(
            "duplicate_alias",
            &duplicate_alias_path,
            margin_threshold_micro,
            "duplicate package alias",
        ),
        expect_registry_config_error(
            "missing_manifest",
            &missing_manifest_path,
            margin_threshold_micro,
            "failed to read",
        ),
        expect_registry_config_error(
            "manifest_mismatch",
            &manifest_mismatch_path,
            margin_threshold_micro,
            "manifest/package fingerprint mismatch",
        ),
    ];
    let invalid_case_count = invalid_cases.len();
    let invalid_reject_count = invalid_cases
        .iter()
        .filter(|case| case.rejected && case.expected_error_matched)
        .count();
    let invalid_error_messages_pass = invalid_reject_count == invalid_case_count;
    let pass = valid_registry_load_pass && invalid_error_messages_pass;
    let report = PhaseActionDaemonConfigValidationSmokeReport {
        schema: "nando_phase_action_daemon_config_validation_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_CONFIG_VALIDATION_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_CONFIG_VALIDATION_SMOKE_V1_FAIL"
        },
        boundary: "registry-config validation smoke over existing .nwpc packages: valid config loads, invalid schema/alias/manifest/parity configs reject before HTTP serve; not dynamic reload, TLS, service manager, or real pilot traffic",
        registry_config_path: config_path.display().to_string(),
        valid_registry_load_pass,
        valid_package_count: valid_registry.package_count(),
        invalid_case_count,
        invalid_reject_count,
        invalid_error_messages_pass,
        invalid_cases,
        server_started_for_invalid_configs: false,
        server_runtime_config_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };
    write_json_file(&report_path, &report)?;
    println!(
        "phase-action-daemon-config-validation-smoke-v1: {}",
        report.verdict
    );
    println!("  config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  invalid_case_count: {}", report.invalid_case_count);
    println!("  invalid_reject_count: {}", report.invalid_reject_count);
    println!(
        "  invalid_error_messages_pass: {}",
        report.invalid_error_messages_pass
    );
    if pass {
        Ok(())
    } else {
        Err("daemon config validation smoke did not satisfy reject-before-serve gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_rate_limit_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_RATE_LIMIT_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    let max_score_requests = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max-score-requests '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_RATE_LIMIT_SMOKE_MAX_SCORE_REQUESTS);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }
    if max_score_requests == 0 {
        return Err("max-score-requests must be at least 1 for rate-limit smoke".to_owned());
    }

    println!("phase-action-daemon-rate-limit-smoke-v1: writing registry config");
    write_json_file(&config_path, &default_registry_config())?;
    println!("phase-action-daemon-rate-limit-smoke-v1: loading registry from config");
    let registry = load_registry_from_config(&config_path, margin_threshold_micro)?;

    println!("phase-action-daemon-rate-limit-smoke-v1: loading coverage fixture");
    let coverage_manifest =
        load_action_package_manifest_fixture(Path::new(DEFAULT_COVERAGE_MANIFEST))?;
    let coverage_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_COVERAGE_CORPUS),
        &coverage_manifest.operator_keys,
    )?;
    let coverage_request = request_with_alias(&coverage_fixture.local_request, "coverage_action");
    let coverage_over_limit_request =
        request_with_alias(&coverage_fixture.fallback_request, "coverage_action");

    let auth =
        PhaseActionDaemonAuth::from_optional_token(Some(DAEMON_AUTH_SMOKE_TOKEN.to_owned()))?;
    let server = PhaseActionDaemonSmokeServer::start_with_registry_and_config(
        registry,
        auth,
        PhaseActionDaemonServeConfig {
            request_limit: Some(5),
            max_score_requests: Some(max_score_requests),
            server_runtime_config_used: true,
            audit_log_path: None,
        },
    )?;

    println!("phase-action-daemon-rate-limit-smoke-v1: GET /health without token");
    let health_response = get_health(server.addr)?;
    println!("phase-action-daemon-rate-limit-smoke-v1: GET /packages with token");
    let packages_response = get_packages_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-rate-limit-smoke-v1: POST /score allowed coverage_action");
    let allowed_response = post_score_with_auth(
        server.addr,
        &coverage_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-rate-limit-smoke-v1: POST /score over limit");
    let rate_limited_response = post_score_raw(
        server.addr,
        &coverage_over_limit_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-rate-limit-smoke-v1: GET /stats with token");
    let stats_response = get_stats_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    let serve_stats = server.join()?;

    let packages_pass = packages_response.status_code == 200
        && packages_response.body.package_count == 3
        && packages_response
            .body
            .packages
            .iter()
            .any(|package| package.alias == "coverage_action");
    let stats_pass = stats_response.status_code == 200
        && stats_response.body.package_count == 3
        && stats_response.body.score_requests == max_score_requests
        && stats_response.body.health_requests == 1
        && stats_response.body.packages_requests == 1
        && stats_response.body.bad_requests == 1
        && stats_response.body.rate_limited_requests == 1
        && stats_response.body.local_operator_calls == 1
        && stats_response.body.fallback_to_llm_calls == 0
        && stats_response.body.false_local_accepts == 0;
    let pass = health_response.status_code == 200
        && packages_pass
        && allowed_response.status_code == 200
        && allowed_response.body.action == "local_operator"
        && allowed_response.body.margin_micro >= margin_threshold_micro
        && rate_limited_response.status_code == 429
        && stats_pass
        && serve_stats.requests_handled == 4
        && serve_stats.score_requests == max_score_requests
        && serve_stats.health_requests == 1
        && serve_stats.packages_requests == 1
        && serve_stats.stats_requests == 1
        && serve_stats.bad_requests == 1
        && serve_stats.rate_limited_requests == 1
        && serve_stats.local_operator_calls == 1
        && serve_stats.fallback_to_llm_calls == 0
        && serve_stats.false_local_accepts == 0;

    let report = PhaseActionDaemonRateLimitSmokeReport {
        schema: "nando_phase_action_daemon_rate_limit_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_RATE_LIMIT_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_RATE_LIMIT_SMOKE_V1_FAIL"
        },
        boundary: "HTTP rate-limit smoke over registry-config loaded .nwpc packages: /score is capped by max_score_requests and over-limit requests return 429 without invoking scorer; not time-window rate limiting, TLS, dynamic reload, service manager, or real pilot traffic",
        registry_config_path: config_path.display().to_string(),
        registry_config_written_by_smoke: true,
        package_count: packages_response.body.package_count,
        package_aliases: packages_response
            .body
            .packages
            .iter()
            .map(|package| package.alias.clone())
            .collect(),
        max_score_requests,
        health_status_code: health_response.status_code,
        packages_status_code: packages_response.status_code,
        allowed_score_status_code: allowed_response.status_code,
        rate_limited_score_status_code: rate_limited_response.status_code,
        stats_status_code: stats_response.status_code,
        allowed_action: allowed_response.body.action,
        allowed_margin_micro: allowed_response.body.margin_micro,
        http_requests: serve_stats.total_requests(),
        http_requests_handled: serve_stats.requests_handled,
        http_score_requests: serve_stats.score_requests,
        http_health_requests: serve_stats.health_requests,
        http_packages_requests: serve_stats.packages_requests,
        http_stats_requests: serve_stats.stats_requests,
        http_bad_requests: serve_stats.bad_requests,
        http_rate_limited_requests: serve_stats.rate_limited_requests,
        local_operator_calls: serve_stats.local_operator_calls,
        fallback_to_llm_calls: serve_stats.fallback_to_llm_calls,
        false_local_accepts: serve_stats.false_local_accepts,
        request_fixture_corpus_jsonl_used: true,
        server_runtime_config_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!(
        "phase-action-daemon-rate-limit-smoke-v1: {}",
        report.verdict
    );
    println!("  config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  max_score_requests: {}", report.max_score_requests);
    println!(
        "  rate_limited_score_status_code: {}",
        report.rate_limited_score_status_code
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);

    if pass {
        Ok(())
    } else {
        Err("daemon rate-limit smoke did not satisfy score guard gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_observability_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_OBSERVABILITY_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-observability-smoke-v1: writing registry config");
    write_json_file(&config_path, &default_registry_config())?;
    println!("phase-action-daemon-observability-smoke-v1: loading registry from config");
    let registry = load_registry_from_config(&config_path, margin_threshold_micro)?;

    println!("phase-action-daemon-observability-smoke-v1: loading coverage fixture");
    let coverage_manifest =
        load_action_package_manifest_fixture(Path::new(DEFAULT_COVERAGE_MANIFEST))?;
    let coverage_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_COVERAGE_CORPUS),
        &coverage_manifest.operator_keys,
    )?;
    let missing_request = request_with_alias(&coverage_fixture.local_request, "missing_package");
    let allowed_request = request_with_alias(&coverage_fixture.local_request, "coverage_action");
    let over_limit_request =
        request_with_alias(&coverage_fixture.fallback_request, "coverage_action");

    let auth =
        PhaseActionDaemonAuth::from_optional_token(Some(DAEMON_AUTH_SMOKE_TOKEN.to_owned()))?;
    let server = PhaseActionDaemonSmokeServer::start_with_registry_and_config(
        registry,
        auth,
        PhaseActionDaemonServeConfig {
            request_limit: Some(6),
            max_score_requests: Some(DAEMON_RATE_LIMIT_SMOKE_MAX_SCORE_REQUESTS),
            server_runtime_config_used: true,
            audit_log_path: None,
        },
    )?;

    println!("phase-action-daemon-observability-smoke-v1: GET /health without token");
    let health_response = get_health(server.addr)?;
    println!("phase-action-daemon-observability-smoke-v1: GET /packages with token");
    let packages_response = get_packages_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-observability-smoke-v1: POST /score missing alias");
    let missing_response =
        post_score_raw(server.addr, &missing_request, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-observability-smoke-v1: POST /score allowed coverage_action");
    let allowed_response =
        post_score_with_auth(server.addr, &allowed_request, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-observability-smoke-v1: POST /score over limit");
    let rate_limited_response = post_score_raw(
        server.addr,
        &over_limit_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-observability-smoke-v1: GET /stats with token");
    let stats_response = get_stats_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    let serve_stats = server.join()?;

    let expected_aliases = vec![
        "generated_action".to_owned(),
        "domain_action".to_owned(),
        "coverage_action".to_owned(),
    ];
    let stats = &stats_response.body;
    let packages_pass = packages_response.status_code == 200
        && packages_response.body.package_count == 3
        && packages_response
            .body
            .packages
            .iter()
            .map(|package| package.alias.clone())
            .collect::<Vec<_>>()
            == expected_aliases;
    let stats_pass = stats_response.status_code == 200
        && stats.package_count == 3
        && stats.package_aliases == expected_aliases
        && stats.requests_handled == 3
        && stats.score_requests == 1
        && stats.health_requests == 1
        && stats.packages_requests == 1
        && stats.stats_requests == 0
        && stats.bad_requests == 2
        && stats.rate_limited_requests == 1
        && stats.max_score_requests == Some(DAEMON_RATE_LIMIT_SMOKE_MAX_SCORE_REQUESTS)
        && stats.local_operator_calls == 1
        && stats.fallback_to_llm_calls == 0
        && stats.false_local_accepts == 0
        && stats.server_runtime_config_used
        && !stats.server_runtime_compiler_used
        && !stats.server_runtime_corpus_jsonl_used
        && !stats.python_demo_used;
    let pass = health_response.status_code == 200
        && packages_pass
        && missing_response.status_code == 404
        && allowed_response.status_code == 200
        && allowed_response.body.action == "local_operator"
        && allowed_response.body.margin_micro >= margin_threshold_micro
        && rate_limited_response.status_code == 429
        && stats_pass
        && serve_stats.requests_handled == 4
        && serve_stats.score_requests == 1
        && serve_stats.health_requests == 1
        && serve_stats.packages_requests == 1
        && serve_stats.stats_requests == 1
        && serve_stats.bad_requests == 2
        && serve_stats.rate_limited_requests == 1
        && serve_stats.local_operator_calls == 1
        && serve_stats.fallback_to_llm_calls == 0
        && serve_stats.false_local_accepts == 0;

    let report = PhaseActionDaemonObservabilitySmokeReport {
        schema: "nando_phase_action_daemon_observability_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_OBSERVABILITY_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_OBSERVABILITY_SMOKE_V1_FAIL"
        },
        boundary: "HTTP observability smoke over registry-config loaded .nwpc packages: /stats exposes package aliases, request counters, rate-limit counters, and runtime provenance flags; not tracing, logs, TLS, dynamic reload, service manager, or real pilot traffic",
        registry_config_path: config_path.display().to_string(),
        registry_config_written_by_smoke: true,
        package_count: stats.package_count,
        package_aliases: stats.package_aliases.clone(),
        max_score_requests: stats.max_score_requests,
        health_status_code: health_response.status_code,
        packages_status_code: packages_response.status_code,
        missing_alias_status_code: missing_response.status_code,
        allowed_score_status_code: allowed_response.status_code,
        rate_limited_score_status_code: rate_limited_response.status_code,
        stats_status_code: stats_response.status_code,
        requests_handled_observed_by_stats: stats.requests_handled,
        score_requests_observed_by_stats: stats.score_requests,
        bad_requests_observed_by_stats: stats.bad_requests,
        rate_limited_requests_observed_by_stats: stats.rate_limited_requests,
        local_operator_calls_observed_by_stats: stats.local_operator_calls,
        fallback_to_llm_calls_observed_by_stats: stats.fallback_to_llm_calls,
        false_local_accepts_observed_by_stats: stats.false_local_accepts,
        requests_handled_final: serve_stats.requests_handled,
        score_requests_final: serve_stats.score_requests,
        bad_requests_final: serve_stats.bad_requests,
        rate_limited_requests_final: serve_stats.rate_limited_requests,
        local_operator_calls_final: serve_stats.local_operator_calls,
        fallback_to_llm_calls_final: serve_stats.fallback_to_llm_calls,
        false_local_accepts_final: serve_stats.false_local_accepts,
        server_runtime_config_used: stats.server_runtime_config_used,
        server_runtime_compiler_used: stats.server_runtime_compiler_used,
        server_runtime_corpus_jsonl_used: stats.server_runtime_corpus_jsonl_used,
        python_demo_used: stats.python_demo_used,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!(
        "phase-action-daemon-observability-smoke-v1: {}",
        report.verdict
    );
    println!("  config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!(
        "  rate_limited_requests_observed_by_stats: {}",
        report.rate_limited_requests_observed_by_stats
    );
    println!(
        "  server_runtime_config_used: {}",
        report.server_runtime_config_used
    );
    println!(
        "  false_local_accepts: {}",
        report.false_local_accepts_final
    );

    if pass {
        Ok(())
    } else {
        Err("daemon observability smoke did not satisfy stats/provenance gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_audit_log_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG));
    let audit_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_AUDIT_LOG_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_AUDIT_LOG_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-audit-log-smoke-v1: writing registry config");
    write_json_file(&config_path, &default_registry_config())?;
    if audit_log_path.exists() {
        std::fs::remove_file(&audit_log_path).map_err(|error| {
            format!(
                "failed to remove stale audit log {}: {error}",
                audit_log_path.display()
            )
        })?;
    }

    println!("phase-action-daemon-audit-log-smoke-v1: loading registry from config");
    let registry = load_registry_from_config(&config_path, margin_threshold_micro)?;
    let coverage_manifest =
        load_action_package_manifest_fixture(Path::new(DEFAULT_COVERAGE_MANIFEST))?;
    let coverage_fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_COVERAGE_CORPUS),
        &coverage_manifest.operator_keys,
    )?;
    let missing_request = request_with_alias(&coverage_fixture.local_request, "missing_package");
    let allowed_request = request_with_alias(&coverage_fixture.local_request, "coverage_action");
    let over_limit_request =
        request_with_alias(&coverage_fixture.fallback_request, "coverage_action");

    let auth =
        PhaseActionDaemonAuth::from_optional_token(Some(DAEMON_AUTH_SMOKE_TOKEN.to_owned()))?;
    let server = PhaseActionDaemonSmokeServer::start_with_registry_and_config(
        registry,
        auth,
        PhaseActionDaemonServeConfig {
            request_limit: Some(6),
            max_score_requests: Some(DAEMON_RATE_LIMIT_SMOKE_MAX_SCORE_REQUESTS),
            server_runtime_config_used: true,
            audit_log_path: Some(audit_log_path.clone()),
        },
    )?;

    println!("phase-action-daemon-audit-log-smoke-v1: GET /health without token");
    let health_response = get_health(server.addr)?;
    println!("phase-action-daemon-audit-log-smoke-v1: GET /packages with token");
    let packages_response = get_packages_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-audit-log-smoke-v1: POST /score missing alias");
    let missing_response =
        post_score_raw(server.addr, &missing_request, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-audit-log-smoke-v1: POST /score allowed coverage_action");
    let allowed_response =
        post_score_with_auth(server.addr, &allowed_request, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    println!("phase-action-daemon-audit-log-smoke-v1: POST /score over limit");
    let rate_limited_response = post_score_raw(
        server.addr,
        &over_limit_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-audit-log-smoke-v1: GET /stats with token");
    let stats_response = get_stats_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    let serve_stats = server.join()?;

    let audit_events = load_audit_log_entries(&audit_log_path)?;
    let status_codes = audit_events
        .iter()
        .map(|event| event.status_code)
        .collect::<Vec<_>>();
    let request_kinds = audit_events
        .iter()
        .map(|event| event.request_kind.clone())
        .collect::<Vec<_>>();
    let sequences_are_dense = audit_events
        .iter()
        .enumerate()
        .all(|(index, event)| event.sequence == index + 1);
    let local_score_event = audit_events.iter().find(|event| {
        event.request_kind == "score" && event.action.as_deref() == Some("local_operator")
    });
    let missing_error_event = audit_events.iter().find(|event| {
        event.status_code == 404
            && event
                .error
                .as_deref()
                .is_some_and(|error| error.contains("unknown package alias: missing_package"))
    });
    let rate_limit_event = audit_events.iter().find(|event| {
        event.status_code == 429
            && event
                .error
                .as_deref()
                .is_some_and(|error| error.contains("rate limit exceeded"))
    });
    let audit_flags_pass = audit_events.iter().all(|event| {
        event.schema == "nando_phase_action_daemon_audit_event_v1"
            && event.server_runtime_config_used
            && !event.server_runtime_compiler_used
            && !event.server_runtime_corpus_jsonl_used
            && !event.python_demo_used
    });
    let local_event_pass = local_score_event.is_some_and(|event| {
        event.package_alias.as_deref() == Some("coverage_action")
            && event
                .margin_micro
                .is_some_and(|margin| margin >= margin_threshold_micro)
            && event.error.is_none()
    });
    let pass = health_response.status_code == 200
        && packages_response.status_code == 200
        && missing_response.status_code == 404
        && allowed_response.status_code == 200
        && allowed_response.body.action == "local_operator"
        && rate_limited_response.status_code == 429
        && stats_response.status_code == 200
        && serve_stats.requests_handled == 4
        && serve_stats.bad_requests == 2
        && serve_stats.rate_limited_requests == 1
        && serve_stats.local_operator_calls == 1
        && serve_stats.false_local_accepts == 0
        && audit_events.len() == 6
        && status_codes == vec![200, 200, 404, 200, 429, 200]
        && request_kinds
            == vec![
                "health".to_owned(),
                "packages".to_owned(),
                "error".to_owned(),
                "score".to_owned(),
                "error".to_owned(),
                "stats".to_owned(),
            ]
        && sequences_are_dense
        && local_event_pass
        && missing_error_event.is_some()
        && rate_limit_event.is_some()
        && audit_flags_pass;

    let report = PhaseActionDaemonAuditLogSmokeReport {
        schema: "nando_phase_action_daemon_audit_log_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_AUDIT_LOG_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_AUDIT_LOG_SMOKE_V1_FAIL"
        },
        boundary: "HTTP audit-log smoke over registry-config loaded .nwpc packages: server writes structured JSONL events for handled and rejected requests; not distributed tracing, log rotation, TLS, dynamic reload, service manager, or real pilot traffic",
        registry_config_path: config_path.display().to_string(),
        audit_log_path: audit_log_path.display().to_string(),
        audit_event_count: audit_events.len(),
        audit_status_codes: status_codes,
        audit_request_kinds: request_kinds,
        audit_sequences_are_dense: sequences_are_dense,
        audit_missing_alias_event_found: missing_error_event.is_some(),
        audit_rate_limit_event_found: rate_limit_event.is_some(),
        audit_local_operator_event_found: local_score_event.is_some(),
        audit_flags_pass,
        health_status_code: health_response.status_code,
        packages_status_code: packages_response.status_code,
        missing_alias_status_code: missing_response.status_code,
        allowed_score_status_code: allowed_response.status_code,
        rate_limited_score_status_code: rate_limited_response.status_code,
        stats_status_code: stats_response.status_code,
        http_requests_handled: serve_stats.requests_handled,
        http_bad_requests: serve_stats.bad_requests,
        http_rate_limited_requests: serve_stats.rate_limited_requests,
        local_operator_calls: serve_stats.local_operator_calls,
        fallback_to_llm_calls: serve_stats.fallback_to_llm_calls,
        false_local_accepts: serve_stats.false_local_accepts,
        server_runtime_config_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!("phase-action-daemon-audit-log-smoke-v1: {}", report.verdict);
    println!("  audit_log: {}", report.audit_log_path);
    println!("  report: {}", report_path.display());
    println!("  audit_event_count: {}", report.audit_event_count);
    println!("  audit_flags_pass: {}", report.audit_flags_pass);
    println!("  false_local_accepts: {}", report.false_local_accepts);

    if pass {
        Ok(())
    } else {
        Err("daemon audit-log smoke did not satisfy structured JSONL event gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_error_taxonomy_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_ERROR_TAXONOMY_SMOKE_REPORT));
    let margin_threshold_micro = args
        .next()
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("invalid margin-threshold-micro '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-error-taxonomy-smoke-v1: writing registry config");
    write_json_file(&config_path, &default_registry_config())?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: loading registry from config");
    let registry = load_registry_from_config(&config_path, margin_threshold_micro)?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: loading coverage fixture");
    let coverage_manifest =
        load_action_package_manifest_fixture(Path::new(DEFAULT_COVERAGE_MANIFEST))?;
    let fixture = load_first_heldout_fixture(
        Path::new(DEFAULT_COVERAGE_CORPUS),
        &coverage_manifest.operator_keys,
    )?;
    let auth =
        PhaseActionDaemonAuth::from_optional_token(Some(DAEMON_AUTH_SMOKE_TOKEN.to_owned()))?;
    let server = PhaseActionDaemonSmokeServer::start_with_registry_and_config(
        registry,
        auth,
        PhaseActionDaemonServeConfig {
            request_limit: Some(9),
            max_score_requests: None,
            server_runtime_config_used: true,
            audit_log_path: None,
        },
    )?;

    println!("phase-action-daemon-error-taxonomy-smoke-v1: GET /health without token");
    let health_response = get_health(server.addr)?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: POST /score malformed JSON");
    let malformed_json_response = send_http_request(
        server.addr,
        "POST",
        "/score",
        Some("{not-json"),
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: POST /score unknown package");
    let mut missing_alias_request = fixture.local_request.clone();
    missing_alias_request.package_alias = Some("missing_package".to_owned());
    let missing_alias_response = post_score_raw(
        server.addr,
        &missing_alias_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: POST /score too many atoms");
    let mut too_many_atoms_request = fixture.local_request.clone();
    too_many_atoms_request.candidate_atoms =
        vec!["select:overflow".to_owned(); DAEMON_MAX_SCORE_ATOMS + 1];
    let too_many_atoms_response = post_score_raw(
        server.addr,
        &too_many_atoms_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: POST /score too long atom");
    let mut too_long_atom_request = fixture.local_request.clone();
    too_long_atom_request.candidate_atoms = vec!["x".repeat(DAEMON_MAX_SCORE_ATOM_BYTES + 1)];
    let too_long_atom_response = post_score_raw(
        server.addr,
        &too_long_atom_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: POST /score center index out of bounds");
    let mut out_of_bounds_request = fixture.local_request.clone();
    out_of_bounds_request.center_index = fixture.center_index + 1_000_000;
    let out_of_bounds_response = post_score_raw(
        server.addr,
        &out_of_bounds_request,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: PUT /score unsupported method");
    let bad_method_response = send_http_request(
        server.addr,
        "PUT",
        "/score",
        Some("{}"),
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: POST /score oversized HTTP body");
    let oversized_request_response = send_http_request_with_declared_content_length(
        server.addr,
        "POST",
        "/score",
        HTTP_MAX_REQUEST_BYTES + 1,
        Some(DAEMON_AUTH_SMOKE_TOKEN),
    )?;
    println!("phase-action-daemon-error-taxonomy-smoke-v1: GET /stats with token");
    let stats_response = get_stats_with_auth(server.addr, Some(DAEMON_AUTH_SMOKE_TOKEN))?;
    let serve_stats = server.join()?;

    let error_status_codes = vec![
        malformed_json_response.status_code,
        missing_alias_response.status_code,
        too_many_atoms_response.status_code,
        too_long_atom_response.status_code,
        out_of_bounds_response.status_code,
        bad_method_response.status_code,
        oversized_request_response.status_code,
    ];
    let error_messages_pass = raw_error_contains(&malformed_json_response, "failed to parse")
        && raw_error_contains(&missing_alias_response, "unknown package alias")
        && raw_error_contains(&too_many_atoms_response, "exceeds 1024 atoms")
        && raw_error_contains(&too_long_atom_response, "exceeds 256 bytes")
        && raw_error_contains(&out_of_bounds_response, "out of bounds")
        && raw_error_contains(&bad_method_response, "unsupported HTTP method")
        && raw_error_contains(&oversized_request_response, "HTTP request exceeds");
    let stats = &stats_response.body;
    let stats_pass = stats_response.status_code == 200
        && stats.requests_handled == 1
        && stats.score_requests == 0
        && stats.bad_requests == 7
        && stats.local_operator_calls == 0
        && stats.fallback_to_llm_calls == 0
        && stats.false_local_accepts == 0
        && stats.server_runtime_config_used
        && !stats.server_runtime_compiler_used
        && !stats.server_runtime_corpus_jsonl_used
        && !stats.python_demo_used;
    let pass = health_response.status_code == 200
        && error_status_codes == vec![400, 404, 413, 413, 400, 405, 413]
        && error_messages_pass
        && stats_pass
        && serve_stats.requests_handled == 2
        && serve_stats.score_requests == 0
        && serve_stats.bad_requests == 7
        && serve_stats.local_operator_calls == 0
        && serve_stats.fallback_to_llm_calls == 0
        && serve_stats.false_local_accepts == 0;
    let report = PhaseActionDaemonErrorTaxonomySmokeReport {
        schema: "nando_phase_action_daemon_error_taxonomy_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_ERROR_TAXONOMY_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_ERROR_TAXONOMY_SMOKE_V1_FAIL"
        },
        boundary: "HTTP error-taxonomy smoke over registry-config loaded .nwpc packages: malformed JSON, unknown package alias, oversized atoms/body, out-of-bounds center, and unsupported method reject before scorer; not fuzzing, TLS, dynamic reload, service manager, or real pilot traffic",
        registry_config_path: config_path.display().to_string(),
        health_status_code: health_response.status_code,
        malformed_json_status_code: malformed_json_response.status_code,
        missing_alias_status_code: missing_alias_response.status_code,
        too_many_atoms_status_code: too_many_atoms_response.status_code,
        too_long_atom_status_code: too_long_atom_response.status_code,
        out_of_bounds_status_code: out_of_bounds_response.status_code,
        unsupported_method_status_code: bad_method_response.status_code,
        oversized_request_status_code: oversized_request_response.status_code,
        stats_status_code: stats_response.status_code,
        error_status_codes,
        error_messages_pass,
        http_requests_handled: serve_stats.requests_handled,
        http_score_requests: serve_stats.score_requests,
        http_bad_requests: serve_stats.bad_requests,
        local_operator_calls: serve_stats.local_operator_calls,
        fallback_to_llm_calls: serve_stats.fallback_to_llm_calls,
        false_local_accepts: serve_stats.false_local_accepts,
        stats_requests_handled_before_stats: stats.requests_handled,
        stats_bad_requests_before_stats: stats.bad_requests,
        stats_score_requests_before_stats: stats.score_requests,
        server_runtime_config_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };
    write_json_file(&report_path, &report)?;
    println!(
        "phase-action-daemon-error-taxonomy-smoke-v1: {}",
        report.verdict
    );
    println!("  report: {}", report_path.display());
    println!("  error_status_codes: {:?}", report.error_status_codes);
    println!("  error_messages_pass: {}", report.error_messages_pass);
    println!("  score_requests: {}", report.http_score_requests);
    println!("  false_local_accepts: {}", report.false_local_accepts);
    if pass {
        Ok(())
    } else {
        Err("daemon error-taxonomy smoke did not satisfy rejection/scorer gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_proof_suite_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_PROOF_SUITE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let report = build_daemon_proof_suite_report(
        "nando_phase_action_daemon_proof_suite_report_v1",
        "PHASE_ACTION_DAEMON_PROOF_SUITE_V1_PASS",
        "PHASE_ACTION_DAEMON_PROOF_SUITE_V1_FAIL",
        "daemon proof suite over saved HTTP daemon product-proof JSON reports: verifies smoke verdicts, forbidden flags, hot-path provenance, no false local accepts, and key status/counter invariants; not live rerun, TLS, service manager, dynamic reload, or real pilot traffic",
        false,
        0,
    )?;
    let pass = report.verdict == "PHASE_ACTION_DAEMON_PROOF_SUITE_V1_PASS";
    write_json_file(&report_path, &report)?;
    println!("phase-action-daemon-proof-suite-v1: {}", report.verdict);
    println!("  report: {}", report_path.display());
    println!("  artifact_count: {}", report.artifact_count);
    println!("  pass_count: {}", report.pass_count);
    println!(
        "  all_server_runtime_hot_path_clean: {}",
        report.all_server_runtime_hot_path_clean
    );
    println!(
        "  all_false_local_accepts_zero: {}",
        report.all_false_local_accepts_zero
    );
    if pass {
        Ok(())
    } else {
        Err("daemon proof suite did not satisfy saved-report gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_live_proof_suite_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_LIVE_PROOF_SUITE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_smoke");
    run_phase_action_daemon_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_package_smoke");
    run_phase_action_daemon_package_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_hardening_smoke");
    run_phase_action_daemon_hardening_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_auth_smoke");
    run_phase_action_daemon_auth_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_registry_smoke");
    run_phase_action_daemon_registry_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_registry_config_smoke");
    run_phase_action_daemon_registry_config_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_config_validation_smoke");
    run_phase_action_daemon_config_validation_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_rate_limit_smoke");
    run_phase_action_daemon_rate_limit_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_observability_smoke");
    run_phase_action_daemon_observability_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_audit_log_smoke");
    run_phase_action_daemon_audit_log_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_error_taxonomy_smoke");
    run_phase_action_daemon_error_taxonomy_smoke_v1(std::iter::empty::<String>())?;
    println!("phase-action-daemon-live-proof-suite-v1: rerunning daemon_systemd_smoke");
    run_phase_action_daemon_systemd_smoke_v1(std::iter::empty::<String>())?;

    let report = build_daemon_proof_suite_report(
        "nando_phase_action_daemon_live_proof_suite_report_v1",
        "PHASE_ACTION_DAEMON_LIVE_PROOF_SUITE_V1_PASS",
        "PHASE_ACTION_DAEMON_LIVE_PROOF_SUITE_V1_FAIL",
        "live daemon proof suite over freshly rerun local HTTP daemon and service-packaging smoke gates: reruns all daemon product-proof commands, then verifies smoke verdicts, forbidden flags, hot-path provenance, no false local accepts, service packaging, and key status/counter invariants; not TLS, installed service, dynamic reload, or real pilot traffic",
        true,
        12,
    )?;
    let pass = report.verdict == "PHASE_ACTION_DAEMON_LIVE_PROOF_SUITE_V1_PASS";
    write_json_file(&report_path, &report)?;
    println!(
        "phase-action-daemon-live-proof-suite-v1: {}",
        report.verdict
    );
    println!("  report: {}", report_path.display());
    println!("  artifact_count: {}", report.artifact_count);
    println!("  pass_count: {}", report.pass_count);
    println!("  live_rerun_performed: {}", report.live_rerun_performed);
    println!(
        "  all_server_runtime_hot_path_clean: {}",
        report.all_server_runtime_hot_path_clean
    );
    println!(
        "  all_false_local_accepts_zero: {}",
        report.all_false_local_accepts_zero
    );
    if pass {
        Ok(())
    } else {
        Err("live daemon proof suite did not satisfy rerun gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_systemd_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let service_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_SYSTEMD_SERVICE));
    let env_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_SYSTEMD_ENV));
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_SYSTEMD_SMOKE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-systemd-smoke-v1: writing registry config");
    let registry_config = default_registry_config();
    write_json_file(&config_path, &registry_config)?;
    let registry = load_registry_from_config(&config_path, DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO)?;

    let audit_log_path = PathBuf::from(DEFAULT_DAEMON_SYSTEMD_AUDIT_LOG);
    let cwd = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    let env_abs_path = absolute_path_for_unit(&env_path, &cwd);
    let config_abs_path = absolute_path_for_unit(&config_path, &cwd);
    let audit_log_abs_path = absolute_path_for_unit(&audit_log_path, &cwd);
    let rw_abs_path = absolute_path_for_unit(Path::new("target/nando-wave"), &cwd);
    let auth_token_placeholder = "CHANGE_ME_NON_EMPTY_BEARER_TOKEN";
    let exe_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    let env_text = format!(
        "NANDO_WAVE_REGISTRY_CONFIG={}\nNANDO_WAVE_BIND_ADDR={}\nNANDO_WAVE_MARGIN_THRESHOLD_MICRO={}\nNANDO_WAVE_AUTH_TOKEN={}\nNANDO_WAVE_MAX_SCORE_REQUESTS={}\nNANDO_WAVE_AUDIT_LOG={}\n",
        config_abs_path.display(),
        DEFAULT_DAEMON_BIND_ADDR,
        DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO,
        auth_token_placeholder,
        DAEMON_RATE_LIMIT_SMOKE_MAX_SCORE_REQUESTS,
        audit_log_abs_path.display()
    );
    let service_text = format!(
        "[Unit]\nDescription=Nando Wave proof-gated action daemon\nAfter=network.target\n\n[Service]\nType=simple\nWorkingDirectory={}\nEnvironmentFile={}\nExecStart={} phase-action-daemon-serve-registry-v1 ${{NANDO_WAVE_REGISTRY_CONFIG}} ${{NANDO_WAVE_BIND_ADDR}} ${{NANDO_WAVE_MARGIN_THRESHOLD_MICRO}} ${{NANDO_WAVE_AUTH_TOKEN}} ${{NANDO_WAVE_MAX_SCORE_REQUESTS}} ${{NANDO_WAVE_AUDIT_LOG}}\nRestart=on-failure\nRestartSec=2s\nNoNewPrivileges=true\nPrivateTmp=true\nProtectSystem=full\nProtectHome=read-only\nReadWritePaths={}\n\n[Install]\nWantedBy=default.target\n",
        cwd.display(),
        env_abs_path.display(),
        exe_path.display(),
        rw_abs_path.display()
    );
    write_text_file(&env_path, &env_text)?;
    write_text_file(&service_path, &service_text)?;

    let service_unit_bytes = service_text.len();
    let env_file_bytes = env_text.len();
    let registry_config_bytes = std::fs::metadata(&config_path)
        .map_err(|error| format!("failed to inspect {}: {error}", config_path.display()))?
        .len() as usize;
    let service_manager_artifacts_written = service_path.is_file()
        && env_path.is_file()
        && config_path.is_file()
        && service_unit_bytes > 0
        && env_file_bytes > 0
        && registry_config_bytes > 0;
    let service_exec_serve_registry = service_text
        .contains("phase-action-daemon-serve-registry-v1")
        && service_text.contains("${NANDO_WAVE_REGISTRY_CONFIG}")
        && service_text.contains("${NANDO_WAVE_AUTH_TOKEN}");
    let service_environment_file_matches =
        service_text.contains(format!("EnvironmentFile={}", env_abs_path.display()).as_str());
    let service_restart_on_failure = service_text.contains("Restart=on-failure");
    let service_hardening_pass = [
        "NoNewPrivileges=true",
        "PrivateTmp=true",
        "ProtectSystem=full",
        "ProtectHome=read-only",
    ]
    .iter()
    .all(|directive| service_text.contains(directive));
    let env_registry_config_matches = env_text
        .contains(format!("NANDO_WAVE_REGISTRY_CONFIG={}", config_abs_path.display()).as_str());
    let auth_token_placeholder_used = env_text.contains(auth_token_placeholder);
    let package_count = registry.package_count();
    let pass = service_manager_artifacts_written
        && package_count == registry_config.packages.len()
        && service_exec_serve_registry
        && service_environment_file_matches
        && service_restart_on_failure
        && service_hardening_pass
        && env_registry_config_matches
        && auth_token_placeholder_used;
    let report = PhaseActionDaemonSystemdSmokeReport {
        schema: "nando_phase_action_daemon_systemd_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_SYSTEMD_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_SYSTEMD_SMOKE_V1_FAIL"
        },
        boundary: "systemd user-service packaging smoke for phase-action-daemon-serve-registry-v1: writes service/env/registry artifacts under target and verifies service wiring, auth placeholder, rate-limit/audit parameters, and hardening directives; does not install or start systemd service, configure TLS, dynamic reload, or real pilot traffic",
        service_unit_path: service_path.display().to_string(),
        env_file_path: env_path.display().to_string(),
        registry_config_path: config_path.display().to_string(),
        audit_log_path: audit_log_path.display().to_string(),
        package_count,
        service_unit_bytes,
        env_file_bytes,
        registry_config_bytes,
        service_manager_artifacts_written,
        service_exec_serve_registry,
        service_environment_file_matches,
        service_restart_on_failure,
        service_hardening_pass,
        env_registry_config_matches,
        env_bind_addr: DEFAULT_DAEMON_BIND_ADDR,
        env_margin_threshold_micro: DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO,
        env_max_score_requests: DAEMON_RATE_LIMIT_SMOKE_MAX_SCORE_REQUESTS,
        auth_token_placeholder_used,
        installed_to_systemd: false,
        systemctl_invoked: false,
        server_runtime_config_used: true,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };
    write_json_file(&report_path, &report)?;
    println!("phase-action-daemon-systemd-smoke-v1: {}", report.verdict);
    println!("  service: {}", report.service_unit_path);
    println!("  env: {}", report.env_file_path);
    println!("  report: {}", report_path.display());
    println!("  package_count: {}", report.package_count);
    println!(
        "  service_hardening_pass: {}",
        report.service_hardening_pass
    );
    println!("  systemctl_invoked: {}", report.systemctl_invoked);
    if pass {
        Ok(())
    } else {
        Err("daemon systemd smoke did not satisfy service packaging gate".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_deployment_package_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let live_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_LIVE_PROOF_SUITE_REPORT));
    let systemd_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_SYSTEMD_SMOKE_REPORT));
    let deployment_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_DEPLOYMENT_PACKAGE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let report = build_phase_action_daemon_deployment_package_report(
        &live_suite_report_path,
        &systemd_report_path,
    )?;
    let pass = report.verdict == "PHASE_ACTION_DAEMON_DEPLOYMENT_PACKAGE_V1_PASS";
    write_json_file(&deployment_report_path, &report)?;
    println!(
        "phase-action-daemon-deployment-package-v1: {}",
        report.verdict
    );
    println!("  report: {}", deployment_report_path.display());
    println!(
        "  live_suite_artifact_count: {}",
        report.live_suite_artifact_count
    );
    println!("  live_suite_step_count: {}", report.live_suite_step_count);
    println!(
        "  service_unit_exec_matches: {}",
        report.service_unit_exec_matches
    );
    println!(
        "  systemd_hardening_pass: {}",
        report.systemd_hardening_pass
    );
    println!("  systemctl_invoked: {}", report.systemctl_invoked);
    if pass {
        Ok(())
    } else {
        Err("daemon deployment package did not satisfy package verifier".to_owned())
    }
}

pub(crate) fn run_phase_action_daemon_deployment_verify_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let live_suite_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_LIVE_PROOF_SUITE_REPORT));
    let systemd_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_SYSTEMD_SMOKE_REPORT));
    let deployment_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_DEPLOYMENT_PACKAGE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    let saved_report = read_json_value(&deployment_report_path)?;
    let rebuilt_report = build_phase_action_daemon_deployment_package_report(
        &live_suite_report_path,
        &systemd_report_path,
    )?;
    let rebuilt_report_value = serde_json::to_value(&rebuilt_report)
        .map_err(|error| format!("failed to encode rebuilt deployment report: {error}"))?;
    let report_gate_pass = json_string_value(&saved_report, "verdict")
        == Some("PHASE_ACTION_DAEMON_DEPLOYMENT_PACKAGE_V1_PASS");
    let rebuilt_gate_pass =
        rebuilt_report.verdict == "PHASE_ACTION_DAEMON_DEPLOYMENT_PACKAGE_V1_PASS";
    let report_matches_sources = saved_report == rebuilt_report_value;
    let gate_pass = report_gate_pass && rebuilt_gate_pass && report_matches_sources;

    println!("phase-action-daemon-deployment-verify-v1:");
    println!(
        "  verdict: {}",
        if gate_pass {
            "PHASE_ACTION_DAEMON_DEPLOYMENT_VERIFY_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_DEPLOYMENT_VERIFY_V1_WATCH"
        }
    );
    println!("  report: {}", deployment_report_path.display());
    println!("  report_gate_pass: {report_gate_pass}");
    println!("  rebuilt_gate_pass: {rebuilt_gate_pass}");
    println!("  report_matches_sources: {report_matches_sources}");
    println!(
        "  live_suite_report_path: {}",
        live_suite_report_path.display()
    );
    println!("  systemd_report_path: {}", systemd_report_path.display());
    println!(
        "  live_suite_artifact_count: {}",
        rebuilt_report.live_suite_artifact_count
    );
    println!(
        "  live_suite_step_count: {}",
        rebuilt_report.live_suite_step_count
    );
    println!(
        "  service_unit_exec_matches: {}",
        rebuilt_report.service_unit_exec_matches
    );
    println!(
        "  registry_config_package_count: {}",
        rebuilt_report.registry_config_package_count
    );
    println!(
        "  deployment_artifacts_present: {}",
        rebuilt_report.deployment_artifacts_present
    );

    if gate_pass {
        Ok(())
    } else {
        Err("daemon deployment package report does not match current sources".to_owned())
    }
}

fn build_phase_action_daemon_deployment_package_report(
    live_suite_report_path: &Path,
    systemd_report_path: &Path,
) -> Result<PhaseActionDaemonDeploymentPackageReport, String> {
    let live_suite_report = read_json_value(live_suite_report_path)?;
    let systemd_report = read_json_value(systemd_report_path)?;
    let (live_suite_report_fingerprint64, live_suite_report_bytes) =
        inspect_file(live_suite_report_path)?;
    let (systemd_report_fingerprint64, systemd_report_bytes) = inspect_file(systemd_report_path)?;

    let service_unit_path = json_string_field(&systemd_report, "service_unit_path")?;
    let env_file_path = json_string_field(&systemd_report, "env_file_path")?;
    let registry_config_path = json_string_field(&systemd_report, "registry_config_path")?;
    let service_unit_path_buf = PathBuf::from(&service_unit_path);
    let env_file_path_buf = PathBuf::from(&env_file_path);
    let registry_config_path_buf = PathBuf::from(&registry_config_path);
    let cwd = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    let env_file_abs_path = absolute_path_for_unit(&env_file_path_buf, &cwd);
    let registry_config_abs_path = absolute_path_for_unit(&registry_config_path_buf, &cwd);
    let service_unit_text = read_text_file(&service_unit_path_buf)?;
    let env_file_text = read_text_file(&env_file_path_buf)?;
    let registry_config = read_json_value(&registry_config_path_buf)?;
    let (service_unit_fingerprint64, service_unit_bytes) = inspect_file(&service_unit_path_buf)?;
    let (env_file_fingerprint64, env_file_bytes) = inspect_file(&env_file_path_buf)?;
    let (registry_config_fingerprint64, registry_config_bytes) =
        inspect_file(&registry_config_path_buf)?;

    let live_suite_pass = json_string_value(&live_suite_report, "verdict")
        == Some("PHASE_ACTION_DAEMON_LIVE_PROOF_SUITE_V1_PASS");
    let live_suite_artifact_count = json_usize_value(&live_suite_report, "artifact_count");
    let live_suite_step_count = json_usize_value(&live_suite_report, "live_rerun_step_count");
    let live_suite_contains_systemd = live_suite_report
        .get("artifacts")
        .and_then(|value| value.as_array())
        .is_some_and(|artifacts| {
            artifacts.iter().any(|artifact| {
                artifact.get("label").and_then(|value| value.as_str())
                    == Some("daemon_systemd_smoke")
                    && artifact.get("pass").and_then(|value| value.as_bool()) == Some(true)
            })
        });
    let live_suite_hot_path_clean =
        json_bool_value(&live_suite_report, "all_server_runtime_hot_path_clean") == Some(true);
    let live_suite_forbidden_flags_false =
        json_bool_value(&live_suite_report, "all_forbidden_flags_false") == Some(true);
    let live_suite_python_demo_false =
        json_bool_value(&live_suite_report, "all_python_demo_false") == Some(true);
    let live_suite_false_local_accepts_zero =
        json_bool_value(&live_suite_report, "all_false_local_accepts_zero") == Some(true);

    let systemd_smoke_pass = json_string_value(&systemd_report, "verdict")
        == Some("PHASE_ACTION_DAEMON_SYSTEMD_SMOKE_V1_PASS");
    let systemd_artifacts_written =
        json_bool_value(&systemd_report, "service_manager_artifacts_written") == Some(true);
    let systemd_hardening_pass =
        json_bool_value(&systemd_report, "service_hardening_pass") == Some(true);
    let systemd_auth_placeholder_used =
        json_bool_value(&systemd_report, "auth_token_placeholder_used") == Some(true);
    let systemd_not_installed =
        json_bool_value(&systemd_report, "installed_to_systemd") == Some(false);
    let systemctl_not_invoked =
        json_bool_value(&systemd_report, "systemctl_invoked") == Some(false);
    let systemd_hot_path_clean = json_bool_value(&systemd_report, "server_runtime_config_used")
        == Some(true)
        && json_bool_value(&systemd_report, "server_runtime_compiler_used") == Some(false)
        && json_bool_value(&systemd_report, "server_runtime_corpus_jsonl_used") == Some(false)
        && json_bool_value(&systemd_report, "python_demo_used") == Some(false);
    let systemd_forbidden_flags_false = [
        "target_center_id_training_used",
        "proof_rule_id_training_authority_used",
        "concrete_x_lookup_used",
        "local_out_t_runtime_extension_used",
    ]
    .iter()
    .all(|field| json_bool_value(&systemd_report, field) == Some(false));

    let service_unit_exec_matches = service_unit_text
        .contains("phase-action-daemon-serve-registry-v1")
        && service_unit_text.contains("${NANDO_WAVE_REGISTRY_CONFIG}")
        && service_unit_text.contains("${NANDO_WAVE_AUTH_TOKEN}");
    let service_unit_env_matches = service_unit_text
        .contains(format!("EnvironmentFile={}", env_file_abs_path.display()).as_str());
    let env_file_config_matches = env_file_text.contains(
        format!(
            "NANDO_WAVE_REGISTRY_CONFIG={}",
            registry_config_abs_path.display()
        )
        .as_str(),
    );
    let registry_config_package_count = registry_config
        .get("packages")
        .and_then(|value| value.as_array())
        .map_or(0, Vec::len);
    let registry_config_package_count_matches =
        Some(registry_config_package_count) == json_usize_value(&systemd_report, "package_count");
    let deployment_artifacts_present = service_unit_bytes > 0
        && env_file_bytes > 0
        && registry_config_bytes > 0
        && live_suite_report_bytes > 0
        && systemd_report_bytes > 0;

    let pass = live_suite_pass
        && live_suite_artifact_count == Some(12)
        && live_suite_step_count == Some(12)
        && live_suite_contains_systemd
        && live_suite_hot_path_clean
        && live_suite_forbidden_flags_false
        && live_suite_python_demo_false
        && live_suite_false_local_accepts_zero
        && systemd_smoke_pass
        && systemd_artifacts_written
        && systemd_hardening_pass
        && systemd_auth_placeholder_used
        && systemd_not_installed
        && systemctl_not_invoked
        && systemd_hot_path_clean
        && systemd_forbidden_flags_false
        && service_unit_exec_matches
        && service_unit_env_matches
        && env_file_config_matches
        && registry_config_package_count_matches
        && deployment_artifacts_present;

    let report = PhaseActionDaemonDeploymentPackageReport {
        schema: "nando_phase_action_daemon_deployment_package_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_DEPLOYMENT_PACKAGE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_DEPLOYMENT_PACKAGE_V1_FAIL"
        },
        boundary: "daemon deployment package verifier over live proof suite, systemd smoke report, service unit, env file, and registry config: proves a local packageable daemon surface with service wiring, auth placeholder, rate-limit/audit parameters, hardening directives, and clean hot-path provenance; does not install/start systemd service, configure TLS, dynamic reload, or real pilot traffic",
        live_suite_report_path: live_suite_report_path.display().to_string(),
        live_suite_report_fingerprint64,
        live_suite_report_bytes,
        systemd_report_path: systemd_report_path.display().to_string(),
        systemd_report_fingerprint64,
        systemd_report_bytes,
        service_unit_path,
        service_unit_fingerprint64,
        service_unit_bytes,
        env_file_path,
        env_file_fingerprint64,
        env_file_bytes,
        registry_config_path,
        registry_config_fingerprint64,
        registry_config_bytes,
        live_suite_pass,
        live_suite_artifact_count: live_suite_artifact_count.unwrap_or(0),
        live_suite_step_count: live_suite_step_count.unwrap_or(0),
        live_suite_contains_systemd,
        live_suite_hot_path_clean,
        live_suite_forbidden_flags_false,
        live_suite_python_demo_false,
        live_suite_false_local_accepts_zero,
        systemd_smoke_pass,
        systemd_artifacts_written,
        systemd_hardening_pass,
        systemd_auth_placeholder_used,
        systemd_not_installed,
        systemctl_not_invoked,
        systemd_hot_path_clean,
        systemd_forbidden_flags_false,
        service_unit_exec_matches,
        service_unit_env_matches,
        env_file_config_matches,
        registry_config_package_count,
        registry_config_package_count_matches,
        deployment_artifacts_present,
        installed_to_systemd: false,
        systemctl_invoked: false,
    };
    Ok(report)
}

pub(crate) fn run_phase_action_daemon_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_SMOKE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra}"));
    }

    println!("phase-action-daemon-smoke-v1: building smoke package bytes");
    let package_bytes = build_smoke_package_bytes()?;
    let runtime =
        offload_runtime_from_package_bytes(&package_bytes, DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO)?;
    let package_info = runtime.package_info();
    let runtime_bytes_estimate = runtime.bytes_estimate();

    println!(
        "phase-action-daemon-smoke-v1: starting loopback HTTP service, package_fingerprint64={}",
        package_info.fingerprint64
    );
    let server = PhaseActionDaemonSmokeServer::start(runtime, 2)?;

    let positive_request = PhaseActionDaemonScoreRequest {
        package_alias: None,
        center_index: 0,
        candidate_atoms: positive_atoms()
            .iter()
            .map(|atom| (*atom).to_owned())
            .collect(),
        counterfactual_atoms: negative_atoms()
            .iter()
            .map(|atom| (*atom).to_owned())
            .collect(),
    };
    let negative_request = PhaseActionDaemonScoreRequest {
        package_alias: None,
        center_index: 0,
        candidate_atoms: negative_atoms()
            .iter()
            .map(|atom| (*atom).to_owned())
            .collect(),
        counterfactual_atoms: positive_atoms()
            .iter()
            .map(|atom| (*atom).to_owned())
            .collect(),
    };

    println!("phase-action-daemon-smoke-v1: POST /score local-operator candidate");
    let local_response = post_score(server.addr, &positive_request)?;
    println!("phase-action-daemon-smoke-v1: POST /score fallback candidate");
    let fallback_response = post_score(server.addr, &negative_request)?;
    let serve_stats = server.join()?;

    let local_pass = local_response.status_code == 200
        && local_response.body.action == "local_operator"
        && local_response.body.margin_micro >= DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO;
    let fallback_pass =
        fallback_response.status_code == 200 && fallback_response.body.action == "fallback_to_llm";
    let false_local_accepts = usize::from(
        local_response.body.margin_micro <= 0 && local_response.body.action == "local_operator",
    ) + usize::from(
        fallback_response.body.margin_micro <= 0
            && fallback_response.body.action == "local_operator",
    );
    let pass = local_pass
        && fallback_pass
        && false_local_accepts == 0
        && serve_stats.requests_handled == 2
        && serve_stats.bad_requests == 0
        && package_info.fingerprint64 != 0;

    let report = PhaseActionDaemonSmokeReport {
        schema: "nando_phase_action_daemon_smoke_report_v1",
        verdict: if pass {
            "PHASE_ACTION_DAEMON_SMOKE_V1_PASS"
        } else {
            "PHASE_ACTION_DAEMON_SMOKE_V1_FAIL"
        },
        boundary: "loopback HTTP service smoke over PhaseCenterOffloadRuntime package bytes; not a production daemon, auth layer, TLS layer, or real workflow pilot",
        package_fingerprint64: package_info.fingerprint64,
        package_cells: package_info.cells,
        package_record_count: package_info.record_count,
        package_serialized_len: package_info.serialized_len,
        runtime_bytes_estimate,
        margin_threshold_micro: DAEMON_SMOKE_MARGIN_THRESHOLD_MICRO,
        http_requests: 2,
        http_requests_handled: serve_stats.requests_handled,
        http_bad_requests: serve_stats.bad_requests,
        local_operator_calls: usize::from(local_response.body.action == "local_operator"),
        fallback_to_llm_calls: usize::from(fallback_response.body.action == "fallback_to_llm"),
        false_local_accepts,
        local_status_code: local_response.status_code,
        fallback_status_code: fallback_response.status_code,
        local_margin_micro: local_response.body.margin_micro,
        fallback_margin_micro: fallback_response.body.margin_micro,
        local_action: local_response.body.action,
        fallback_action: fallback_response.body.action,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
    };

    write_json_file(&report_path, &report)?;
    println!("phase-action-daemon-smoke-v1: {}", report.verdict);
    println!("  report: {}", report_path.display());
    println!("  local_action: {}", report.local_action);
    println!("  fallback_action: {}", report.fallback_action);
    println!("  false_local_accepts: {}", report.false_local_accepts);

    if pass {
        Ok(())
    } else {
        Err("daemon smoke did not satisfy local/fallback service gate".to_owned())
    }
}

fn build_smoke_package_bytes() -> Result<Vec<u8>, String> {
    let mut compiler = PhaseCenterCompiler::new(DAEMON_SMOKE_CELLS, 1)
        .map_err(|error| format!("failed to create compiler: {error:?}"))?;
    compiler
        .add_positive_atoms(0, positive_atoms())
        .map_err(|error| format!("failed to add positive atoms: {error:?}"))?;
    compiler
        .add_negative_atoms(0, negative_atoms())
        .map_err(|error| format!("failed to add negative atoms: {error:?}"))?;
    let runtime = compiler
        .compile()
        .map_err(|error| format!("failed to compile smoke runtime: {error:?}"))?;
    runtime
        .to_bytes()
        .map_err(|error| format!("failed to serialize smoke runtime: {error:?}"))
}

fn load_offload_runtime_from_package(
    package_path: &Path,
    margin_threshold_micro: i64,
) -> Result<PhaseCenterOffloadRuntime, String> {
    let package_bytes = std::fs::read(package_path)
        .map_err(|error| format!("failed to read {}: {error}", package_path.display()))?;
    offload_runtime_from_package_bytes(&package_bytes, margin_threshold_micro)
}

fn offload_runtime_from_package_bytes(
    package_bytes: &[u8],
    margin_threshold_micro: i64,
) -> Result<PhaseCenterOffloadRuntime, String> {
    let policy = PhaseCenterOffloadPolicy::new(margin_threshold_micro)
        .map_err(|error| format!("invalid offload policy: {error:?}"))?;
    PhaseCenterOffloadRuntime::from_package_bytes(package_bytes, policy)
        .map_err(|error| format!("failed to load package bytes: {error:?}"))
}

fn load_action_package_manifest_fixture(
    path: &Path,
) -> Result<PhaseActionPackageManifestFixture, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn default_registry_config() -> PhaseActionDaemonRegistryConfig {
    PhaseActionDaemonRegistryConfig {
        schema: "nando_phase_action_daemon_registry_config_v1".to_owned(),
        packages: vec![
            PhaseActionDaemonRegistryConfigPackage {
                alias: "generated_action".to_owned(),
                package_path: DEFAULT_GENERATED_PACKAGE.to_owned(),
                manifest_path: DEFAULT_GENERATED_MANIFEST.to_owned(),
            },
            PhaseActionDaemonRegistryConfigPackage {
                alias: "domain_action".to_owned(),
                package_path: DEFAULT_DOMAIN_PACKAGE.to_owned(),
                manifest_path: DEFAULT_DOMAIN_MANIFEST.to_owned(),
            },
            PhaseActionDaemonRegistryConfigPackage {
                alias: "coverage_action".to_owned(),
                package_path: DEFAULT_COVERAGE_PACKAGE.to_owned(),
                manifest_path: DEFAULT_COVERAGE_MANIFEST.to_owned(),
            },
        ],
    }
}

fn derived_config_path(config_path: &Path, suffix: &str) -> PathBuf {
    let parent = config_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = config_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("registry");
    parent.join(format!("{stem}.{suffix}.json"))
}

fn expect_registry_config_error(
    case: &'static str,
    config_path: &Path,
    margin_threshold_micro: i64,
    expected_error: &'static str,
) -> PhaseActionDaemonConfigValidationCase {
    match load_registry_from_config(config_path, margin_threshold_micro) {
        Ok(_) => PhaseActionDaemonConfigValidationCase {
            case,
            config_path: config_path.display().to_string(),
            rejected: false,
            expected_error,
            observed_error: None,
            expected_error_matched: false,
        },
        Err(error) => {
            let expected_error_matched = error.contains(expected_error);
            PhaseActionDaemonConfigValidationCase {
                case,
                config_path: config_path.display().to_string(),
                rejected: true,
                expected_error,
                observed_error: Some(error),
                expected_error_matched,
            }
        }
    }
}

fn load_registry_from_config(
    config_path: &Path,
    margin_threshold_micro: i64,
) -> Result<PhaseActionDaemonRuntimeRegistry, String> {
    let bytes = std::fs::read(config_path)
        .map_err(|error| format!("failed to read {}: {error}", config_path.display()))?;
    let config = serde_json::from_slice::<PhaseActionDaemonRegistryConfig>(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", config_path.display()))?;
    if config.schema != "nando_phase_action_daemon_registry_config_v1" {
        return Err(format!(
            "unsupported registry config schema in {}: {}",
            config_path.display(),
            config.schema
        ));
    }
    let mut packages = Vec::with_capacity(config.packages.len());
    for package in config.packages {
        let alias = collapse_whitespace(&package.alias);
        let manifest =
            load_action_package_manifest_fixture(Path::new(package.manifest_path.as_str()))?;
        let runtime = load_offload_runtime_from_package(
            Path::new(package.package_path.as_str()),
            margin_threshold_micro,
        )?;
        verify_manifest_matches_runtime(&alias, &manifest, &runtime)?;
        packages.push(PhaseActionDaemonRuntimePackage { alias, runtime });
    }
    PhaseActionDaemonRuntimeRegistry::new(packages)
}

fn verify_manifest_matches_runtime(
    alias: &str,
    manifest: &PhaseActionPackageManifestFixture,
    runtime: &PhaseCenterOffloadRuntime,
) -> Result<(), String> {
    let package_info = runtime.package_info();
    if manifest.package_fingerprint64 != package_info.fingerprint64 {
        return Err(format!(
            "{alias} manifest/package fingerprint mismatch: manifest={} package={}",
            manifest.package_fingerprint64, package_info.fingerprint64
        ));
    }
    if manifest.cells != package_info.cells || manifest.flat_records != package_info.record_count {
        return Err(format!(
            "{alias} manifest/package shape mismatch: manifest cells/records={}/{} package cells/records={}/{}",
            manifest.cells, manifest.flat_records, package_info.cells, package_info.record_count
        ));
    }
    Ok(())
}

fn load_first_heldout_fixture(
    corpus_path: &Path,
    operator_keys: &[String],
) -> Result<PhaseActionDaemonFixture, String> {
    let text = std::fs::read_to_string(corpus_path)
        .map_err(|error| format!("failed to read {}: {error}", corpus_path.display()))?;
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<PhaseActionContractRow>(line).map_err(|error| {
            format!(
                "failed to parse {} line {}: {error}",
                corpus_path.display(),
                line_index + 1
            )
        })?;
        if row.split != "heldout" {
            continue;
        }
        let operator_key = action_contract_key(&row);
        let Some(center_index) = operator_keys.iter().position(|key| *key == operator_key) else {
            continue;
        };
        let Some(correct_atoms) = action_contract_transition_atoms(&row, &row.state_after_correct)
        else {
            continue;
        };
        let Some(wrong_atoms) = action_contract_transition_atoms(&row, &row.state_after_wrong)
        else {
            continue;
        };
        return Ok(PhaseActionDaemonFixture {
            task_id: row.task_id,
            center_index,
            operator_key,
            local_request: PhaseActionDaemonScoreRequest {
                package_alias: None,
                center_index,
                candidate_atoms: correct_atoms.clone(),
                counterfactual_atoms: wrong_atoms.clone(),
            },
            fallback_request: PhaseActionDaemonScoreRequest {
                package_alias: None,
                center_index,
                candidate_atoms: wrong_atoms,
                counterfactual_atoms: correct_atoms,
            },
        });
    }
    Err(format!(
        "no usable heldout fixture found in {}",
        corpus_path.display()
    ))
}

fn load_audit_log_entries(path: &Path) -> Result<Vec<PhaseActionDaemonAuditLogEntry>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read audit log {}: {error}", path.display()))?;
    let mut entries = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry =
            serde_json::from_str::<PhaseActionDaemonAuditLogEntry>(line).map_err(|error| {
                format!(
                    "failed to parse audit log {} line {}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
        entries.push(entry);
    }
    Ok(entries)
}

fn request_with_alias(
    request: &PhaseActionDaemonScoreRequest,
    alias: &str,
) -> PhaseActionDaemonScoreRequest {
    let mut request = request.clone();
    request.package_alias = Some(alias.to_owned());
    request
}

fn action_contract_key(row: &PhaseActionContractRow) -> String {
    collapse_whitespace(&format!(
        "select={}|transform={}|write={}|condition={}|check={}",
        row.action_tree.select,
        row.action_tree.transform,
        row.action_tree.write,
        row.action_tree.condition,
        row.action_tree.check
    ))
}

fn action_contract_transition_atoms(
    row: &PhaseActionContractRow,
    candidate_state: &str,
) -> Option<Vec<String>> {
    let source_tokens = row.state_before.split_whitespace().collect::<Vec<_>>();
    let candidate_tokens = candidate_state.split_whitespace().collect::<Vec<_>>();
    if source_tokens.is_empty() || candidate_tokens.is_empty() {
        return None;
    }

    let mut positions = std::collections::BTreeMap::<&str, Vec<usize>>::new();
    for (index, token) in source_tokens.iter().enumerate() {
        positions.entry(*token).or_default().push(index);
    }

    let mut atoms = vec![
        format!("src_len:{}", source_tokens.len()),
        format!("out_len:{}", candidate_tokens.len()),
    ];
    for (out_slot, token) in candidate_tokens.iter().enumerate() {
        if let Some(source_slots) = positions.get(*token) {
            if source_slots.len() != 1 {
                return None;
            }
            let src_slot = source_slots[0];
            atoms.push(format!("rel:o{out_slot}:s{src_slot}"));
            atoms.push(format!("out:o{out_slot}"));
            atoms.push(format!("src:s{src_slot}"));
            atoms.push(format!("delta:{}", out_slot as isize - src_slot as isize));
        } else {
            atoms.push(format!("insert:o{out_slot}"));
            atoms.push(format!("insert_shape:{}", token_shape(token)));
        }
    }
    Some(atoms)
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn token_shape(token: &str) -> String {
    token
        .chars()
        .map(|character| {
            if character.is_ascii_digit() {
                '0'
            } else if character.is_ascii_uppercase() {
                'A'
            } else if character.is_ascii_lowercase() {
                'a'
            } else {
                '_'
            }
        })
        .collect()
}

fn positive_atoms() -> [&'static str; 5] {
    [
        "select:domain_ticket",
        "transform:normalize_status",
        "write:status_normalized",
        "condition:has_status_marker",
        "check:no_conflict",
    ]
}

fn negative_atoms() -> [&'static str; 5] {
    [
        "select:invoice_amount",
        "transform:delete_field",
        "write:amount_removed",
        "condition:missing_status_marker",
        "check:unsafe_change",
    ]
}

#[derive(Debug)]
struct PhaseActionDaemonSmokeServer {
    addr: SocketAddr,
    handle: thread::JoinHandle<Result<PhaseActionDaemonServeStats, String>>,
}

impl PhaseActionDaemonSmokeServer {
    fn start(runtime: PhaseCenterOffloadRuntime, request_limit: usize) -> Result<Self, String> {
        Self::start_with_auth(runtime, PhaseActionDaemonAuth::disabled(), request_limit)
    }

    fn start_with_auth(
        runtime: PhaseCenterOffloadRuntime,
        auth: PhaseActionDaemonAuth,
        request_limit: usize,
    ) -> Result<Self, String> {
        let registry = PhaseActionDaemonRuntimeRegistry::single("default", runtime)?;
        Self::start_with_registry(registry, auth, request_limit)
    }

    fn start_with_registry(
        registry: PhaseActionDaemonRuntimeRegistry,
        auth: PhaseActionDaemonAuth,
        request_limit: usize,
    ) -> Result<Self, String> {
        Self::start_with_registry_and_config(
            registry,
            auth,
            PhaseActionDaemonServeConfig::with_request_limit(request_limit),
        )
    }

    fn start_with_registry_and_config(
        registry: PhaseActionDaemonRuntimeRegistry,
        auth: PhaseActionDaemonAuth,
        config: PhaseActionDaemonServeConfig,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("failed to bind loopback listener: {error}"))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("failed to read listener addr: {error}"))?;
        let registry = Arc::new(registry);
        let auth = Arc::new(auth);
        let handle = thread::spawn(move || serve_score_requests(listener, registry, auth, config));
        Ok(Self { addr, handle })
    }

    fn join(self) -> Result<PhaseActionDaemonServeStats, String> {
        self.handle
            .join()
            .map_err(|_| "daemon smoke server thread panicked".to_owned())?
    }
}

#[derive(Clone, Debug, Default)]
struct PhaseActionDaemonServeConfig {
    request_limit: Option<usize>,
    max_score_requests: Option<usize>,
    server_runtime_config_used: bool,
    audit_log_path: Option<PathBuf>,
}

impl PhaseActionDaemonServeConfig {
    fn with_request_limit(request_limit: usize) -> Self {
        Self {
            request_limit: Some(request_limit),
            max_score_requests: None,
            server_runtime_config_used: false,
            audit_log_path: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
struct PhaseActionDaemonServeStats {
    requests_handled: usize,
    score_requests: usize,
    health_requests: usize,
    packages_requests: usize,
    stats_requests: usize,
    bad_requests: usize,
    rate_limited_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
}

impl PhaseActionDaemonServeStats {
    fn total_requests(self) -> usize {
        self.requests_handled + self.bad_requests
    }

    fn record_handled(&mut self, handled: PhaseActionDaemonHandledRequest) {
        self.requests_handled += 1;
        match handled {
            PhaseActionDaemonHandledRequest::Health => self.health_requests += 1,
            PhaseActionDaemonHandledRequest::Packages => self.packages_requests += 1,
            PhaseActionDaemonHandledRequest::Stats => self.stats_requests += 1,
            PhaseActionDaemonHandledRequest::Score {
                package_alias: _,
                action,
                margin_micro,
            } => {
                self.score_requests += 1;
                match action {
                    PhaseCenterOffloadAction::LocalOperator => {
                        self.local_operator_calls += 1;
                        if margin_micro <= 0 {
                            self.false_local_accepts += 1;
                        }
                    }
                    PhaseCenterOffloadAction::FallbackToLlm => self.fallback_to_llm_calls += 1,
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum PhaseActionDaemonHandledRequest {
    Health,
    Packages,
    Stats,
    Score {
        package_alias: String,
        action: PhaseCenterOffloadAction,
        margin_micro: i64,
    },
}

#[derive(Clone, Debug)]
struct PhaseActionDaemonHttpError {
    status_code: u16,
    message: String,
}

#[derive(Debug)]
struct PhaseActionDaemonRuntimePackage {
    alias: String,
    runtime: PhaseCenterOffloadRuntime,
}

#[derive(Debug)]
struct PhaseActionDaemonRuntimeRegistry {
    packages: Vec<PhaseActionDaemonRuntimePackage>,
}

impl PhaseActionDaemonRuntimeRegistry {
    fn new(packages: Vec<PhaseActionDaemonRuntimePackage>) -> Result<Self, String> {
        if packages.is_empty() {
            return Err("daemon registry requires at least one package".to_owned());
        }
        for (index, package) in packages.iter().enumerate() {
            if package.alias.trim().is_empty() {
                return Err(format!("package alias at index {index} must not be empty"));
            }
            if packages[..index]
                .iter()
                .any(|previous| previous.alias == package.alias)
            {
                return Err(format!("duplicate package alias: {}", package.alias));
            }
        }
        Ok(Self { packages })
    }

    fn single(alias: &str, runtime: PhaseCenterOffloadRuntime) -> Result<Self, String> {
        Self::new(vec![PhaseActionDaemonRuntimePackage {
            alias: alias.to_owned(),
            runtime,
        }])
    }

    fn default_package(&self) -> &PhaseActionDaemonRuntimePackage {
        &self.packages[0]
    }

    fn package_count(&self) -> usize {
        self.packages.len()
    }

    fn package_summaries(&self) -> Vec<PhaseActionDaemonPackageSummary> {
        self.packages
            .iter()
            .map(PhaseActionDaemonPackageSummary::from_package)
            .collect()
    }

    fn find_package(
        &self,
        alias: Option<&str>,
    ) -> Result<&PhaseActionDaemonRuntimePackage, PhaseActionDaemonHttpError> {
        let alias = alias.unwrap_or_else(|| self.default_package().alias.as_str());
        self.packages
            .iter()
            .find(|package| package.alias == alias)
            .ok_or_else(|| http_error(404, format!("unknown package alias: {alias}")))
    }
}

impl PhaseActionDaemonRuntimePackage {
    fn package_info(&self) -> PhaseCenterRuntimePackageInfo {
        self.runtime.package_info()
    }

    fn runtime(&self) -> &PhaseCenterOffloadRuntime {
        &self.runtime
    }
}

#[derive(Clone, Debug)]
struct PhaseActionDaemonAuth {
    bearer_token: Option<String>,
}

impl PhaseActionDaemonAuth {
    fn disabled() -> Self {
        Self { bearer_token: None }
    }

    fn from_optional_token(token: Option<String>) -> Result<Self, String> {
        let bearer_token = token
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if bearer_token.as_deref().is_some_and(|value| value.len() < 8) {
            return Err("auth token must be at least 8 bytes when provided".to_owned());
        }
        Ok(Self { bearer_token })
    }

    fn is_enabled(&self) -> bool {
        self.bearer_token.is_some()
    }

    fn require_authorized(&self, http_head: &str) -> Result<(), PhaseActionDaemonHttpError> {
        let Some(expected_token) = self.bearer_token.as_deref() else {
            return Ok(());
        };
        let expected_header = format!("Bearer {expected_token}");
        let authorized = http_head.lines().any(|line| {
            let Some((name, value)) = line.split_once(':') else {
                return false;
            };
            name.eq_ignore_ascii_case("authorization") && value.trim() == expected_header
        });
        if authorized {
            Ok(())
        } else {
            Err(http_error(401, "missing or invalid bearer token"))
        }
    }
}

fn serve_score_requests(
    listener: TcpListener,
    registry: Arc<PhaseActionDaemonRuntimeRegistry>,
    auth: Arc<PhaseActionDaemonAuth>,
    config: PhaseActionDaemonServeConfig,
) -> Result<PhaseActionDaemonServeStats, String> {
    let mut stats = PhaseActionDaemonServeStats::default();

    for stream in listener.incoming() {
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
        let mut stream = stream.map_err(|error| format!("failed to accept request: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set read timeout: {error}"))?;
        let sequence = stats.total_requests() + 1;
        match handle_http_request(&mut stream, &registry, &auth, stats, &config) {
            Ok(handled) => {
                append_audit_log_entry(&config, sequence, 200, Some(&handled), None)?;
                stats.record_handled(handled);
            }
            Err(error) => {
                stats.bad_requests += 1;
                if error.status_code == 429 {
                    stats.rate_limited_requests += 1;
                }
                append_audit_log_entry(&config, sequence, error.status_code, None, Some(&error))?;
                write_http_json_error(&mut stream, error.status_code, &error.message)?;
            }
        }
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
    }

    Ok(stats)
}

fn handle_http_request(
    stream: &mut TcpStream,
    registry: &PhaseActionDaemonRuntimeRegistry,
    auth: &PhaseActionDaemonAuth,
    stats_snapshot: PhaseActionDaemonServeStats,
    config: &PhaseActionDaemonServeConfig,
) -> Result<PhaseActionDaemonHandledRequest, PhaseActionDaemonHttpError> {
    let request = read_http_request(stream)?;
    let (head, body) = request
        .split_once("\r\n\r\n")
        .ok_or_else(|| http_error(400, "missing HTTP body"))?;
    let request_line = head
        .lines()
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP method"))?;
    let path = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP path"))?;
    let version = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP version"))?;
    if version != "HTTP/1.1" {
        return Err(http_error(400, "expected HTTP/1.1"));
    }

    match (method, path) {
        ("GET", "/health") => {
            let response = PhaseActionDaemonHealthResponse::from_registry(registry);
            write_http_json(stream, 200, &response)?;
            Ok(PhaseActionDaemonHandledRequest::Health)
        }
        ("GET", "/packages") => {
            auth.require_authorized(head)?;
            let response = PhaseActionDaemonPackagesResponse::from_registry(registry);
            write_http_json(stream, 200, &response)?;
            Ok(PhaseActionDaemonHandledRequest::Packages)
        }
        ("GET", "/stats") => {
            auth.require_authorized(head)?;
            let response =
                PhaseActionDaemonStatsResponse::from_registry(registry, stats_snapshot, config);
            write_http_json(stream, 200, &response)?;
            Ok(PhaseActionDaemonHandledRequest::Stats)
        }
        ("POST", "/score") => {
            auth.require_authorized(head)?;
            if let Some(limit) = config.max_score_requests
                && stats_snapshot.score_requests >= limit
            {
                return Err(http_error(
                    429,
                    format!("score request rate limit exceeded: max_score_requests={limit}"),
                ));
            }
            let score_request: PhaseActionDaemonScoreRequest =
                serde_json::from_str(body).map_err(|error| {
                    http_error(400, format!("failed to parse score request JSON: {error}"))
                })?;
            let package = registry.find_package(score_request.package_alias.as_deref())?;
            validate_score_request(package.runtime(), &score_request)?;
            let response = score_runtime_request(package, &score_request)?;
            let action = response.action_enum();
            let margin_micro = response.margin_micro;
            write_http_json(stream, 200, &response)?;
            Ok(PhaseActionDaemonHandledRequest::Score {
                package_alias: response.package_alias,
                action,
                margin_micro,
            })
        }
        ("POST", _) | ("GET", _) => Err(http_error(
            404,
            format!("unsupported HTTP route: {method} {path}"),
        )),
        _ => Err(http_error(
            405,
            format!("unsupported HTTP method: {method} {path}"),
        )),
    }
}

fn append_audit_log_entry(
    config: &PhaseActionDaemonServeConfig,
    sequence: usize,
    status_code: u16,
    handled: Option<&PhaseActionDaemonHandledRequest>,
    error: Option<&PhaseActionDaemonHttpError>,
) -> Result<(), String> {
    let Some(path) = config.audit_log_path.as_ref() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let (request_kind, package_alias, action, margin_micro) = match handled {
        Some(PhaseActionDaemonHandledRequest::Health) => ("health", None, None, None),
        Some(PhaseActionDaemonHandledRequest::Packages) => ("packages", None, None, None),
        Some(PhaseActionDaemonHandledRequest::Stats) => ("stats", None, None, None),
        Some(PhaseActionDaemonHandledRequest::Score {
            package_alias,
            action,
            margin_micro,
        }) => (
            "score",
            Some(package_alias.clone()),
            Some(offload_action_label(*action).to_owned()),
            Some(*margin_micro),
        ),
        None => ("error", None, None, None),
    };
    let event = PhaseActionDaemonAuditLogEntry {
        schema: "nando_phase_action_daemon_audit_event_v1".to_owned(),
        sequence,
        status_code,
        request_kind: request_kind.to_owned(),
        package_alias,
        action,
        margin_micro,
        error: error.map(|error| error.message.clone()),
        server_runtime_config_used: config.server_runtime_config_used,
        server_runtime_compiler_used: false,
        server_runtime_corpus_jsonl_used: false,
        python_demo_used: false,
    };
    let line = serde_json::to_string(&event)
        .map_err(|error| format!("failed to encode audit log entry: {error}"))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("failed to open audit log {}: {error}", path.display()))?;
    writeln!(file, "{line}")
        .map_err(|error| format!("failed to write audit log {}: {error}", path.display()))
}

fn offload_action_label(action: PhaseCenterOffloadAction) -> &'static str {
    match action {
        PhaseCenterOffloadAction::LocalOperator => "local_operator",
        PhaseCenterOffloadAction::FallbackToLlm => "fallback_to_llm",
    }
}

fn http_error(status_code: u16, message: impl Into<String>) -> PhaseActionDaemonHttpError {
    PhaseActionDaemonHttpError {
        status_code,
        message: message.into(),
    }
}

impl From<String> for PhaseActionDaemonHttpError {
    fn from(message: String) -> Self {
        http_error(400, message)
    }
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, PhaseActionDaemonHttpError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                bytes.extend_from_slice(&buffer[..read]);
                if bytes.len() > HTTP_MAX_REQUEST_BYTES {
                    return Err(http_error(
                        413,
                        format!("HTTP request exceeds {HTTP_MAX_REQUEST_BYTES} bytes"),
                    ));
                }
                if let Some(total_len) = expected_http_request_len(&bytes)?
                    && bytes.len() >= total_len
                {
                    bytes.truncate(total_len);
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) =>
            {
                if error.kind() == ErrorKind::Interrupted {
                    continue;
                }
                if let Some(total_len) = expected_http_request_len(&bytes)?
                    && bytes.len() >= total_len
                {
                    bytes.truncate(total_len);
                    break;
                }
                return Err(http_error(
                    408,
                    format!("incomplete HTTP request before timeout: {error}"),
                ));
            }
            Err(error) => return Err(http_error(400, format!("failed to read request: {error}"))),
        }
    }
    String::from_utf8(bytes)
        .map_err(|error| http_error(400, format!("request is not UTF-8: {error}")))
}

fn expected_http_request_len(bytes: &[u8]) -> Result<Option<usize>, PhaseActionDaemonHttpError> {
    let Some(header_end) = find_header_end(bytes) else {
        return Ok(None);
    };
    let header = std::str::from_utf8(&bytes[..header_end])
        .map_err(|error| http_error(400, format!("HTTP header is not UTF-8: {error}")))?;
    let mut content_len = 0usize;
    for line in header.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_len = value
                .trim()
                .parse::<usize>()
                .map_err(|error| http_error(400, format!("invalid Content-Length: {error}")))?;
        }
    }
    let total_len = header_end + 4 + content_len;
    if total_len > HTTP_MAX_REQUEST_BYTES {
        return Err(http_error(
            413,
            format!("HTTP request exceeds {HTTP_MAX_REQUEST_BYTES} bytes"),
        ));
    }
    Ok(Some(total_len))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn validate_score_request(
    runtime: &PhaseCenterOffloadRuntime,
    request: &PhaseActionDaemonScoreRequest,
) -> Result<(), PhaseActionDaemonHttpError> {
    if request.center_index >= runtime.package_info().record_count {
        return Err(http_error(
            400,
            format!(
                "center_index {} is out of bounds for {} records",
                request.center_index,
                runtime.package_info().record_count
            ),
        ));
    }
    validate_atoms("candidate_atoms", &request.candidate_atoms)?;
    validate_atoms("counterfactual_atoms", &request.counterfactual_atoms)?;
    Ok(())
}

fn validate_atoms(name: &str, atoms: &[String]) -> Result<(), PhaseActionDaemonHttpError> {
    if atoms.is_empty() {
        return Err(http_error(400, format!("{name} must not be empty")));
    }
    if atoms.len() > DAEMON_MAX_SCORE_ATOMS {
        return Err(http_error(
            413,
            format!("{name} exceeds {DAEMON_MAX_SCORE_ATOMS} atoms"),
        ));
    }
    for (index, atom) in atoms.iter().enumerate() {
        if atom.is_empty() {
            return Err(http_error(
                400,
                format!("{name}[{index}] must not be empty"),
            ));
        }
        if atom.len() > DAEMON_MAX_SCORE_ATOM_BYTES {
            return Err(http_error(
                413,
                format!("{name}[{index}] exceeds {DAEMON_MAX_SCORE_ATOM_BYTES} bytes"),
            ));
        }
    }
    Ok(())
}

fn score_runtime_request(
    package: &PhaseActionDaemonRuntimePackage,
    request: &PhaseActionDaemonScoreRequest,
) -> Result<PhaseActionDaemonScoreResponse, String> {
    let runtime = package.runtime();
    let candidate_vec = phase_vector_from_atoms(
        request.candidate_atoms.iter().map(String::as_str),
        runtime.cells(),
    );
    let counterfactual_vec = phase_vector_from_atoms(
        request.counterfactual_atoms.iter().map(String::as_str),
        runtime.cells(),
    );
    let decision = runtime
        .runtime()
        .offload_decision_for(
            request.center_index,
            &candidate_vec,
            &counterfactual_vec,
            runtime.policy(),
        )
        .map_err(|error| format!("failed to score candidate: {error:?}"))?;
    Ok(PhaseActionDaemonScoreResponse::from_decision(
        decision,
        &package.alias,
        runtime.package_info().fingerprint64,
    ))
}

fn post_score(
    addr: SocketAddr,
    request: &PhaseActionDaemonScoreRequest,
) -> Result<PhaseActionDaemonClientResponse, String> {
    post_score_with_auth(addr, request, None)
}

fn post_score_with_auth(
    addr: SocketAddr,
    request: &PhaseActionDaemonScoreRequest,
    bearer_token: Option<&str>,
) -> Result<PhaseActionDaemonClientResponse, String> {
    let response = post_score_raw(addr, request, bearer_token)?;
    parse_client_response(&response)
}

fn post_score_raw(
    addr: SocketAddr,
    request: &PhaseActionDaemonScoreRequest,
    bearer_token: Option<&str>,
) -> Result<PhaseActionDaemonRawClientResponse, String> {
    let body = serde_json::to_string(request)
        .map_err(|error| format!("failed to encode request JSON: {error}"))?;
    send_http_request(addr, "POST", "/score", Some(&body), bearer_token)
}

fn get_health(addr: SocketAddr) -> Result<PhaseActionDaemonHealthClientResponse, String> {
    let response = send_http_request(addr, "GET", "/health", None, None)?;
    parse_client_response(&response)
}

fn get_stats(addr: SocketAddr) -> Result<PhaseActionDaemonStatsClientResponse, String> {
    get_stats_with_auth(addr, None)
}

fn get_packages_with_auth(
    addr: SocketAddr,
    bearer_token: Option<&str>,
) -> Result<PhaseActionDaemonPackagesClientResponse, String> {
    let response = send_http_request(addr, "GET", "/packages", None, bearer_token)?;
    parse_client_response(&response)
}

fn get_stats_with_auth(
    addr: SocketAddr,
    bearer_token: Option<&str>,
) -> Result<PhaseActionDaemonStatsClientResponse, String> {
    let response = send_http_request(addr, "GET", "/stats", None, bearer_token)?;
    parse_client_response(&response)
}

fn get_raw(addr: SocketAddr, path: &str) -> Result<PhaseActionDaemonRawClientResponse, String> {
    send_http_request(addr, "GET", path, None, None)
}

fn send_http_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
    bearer_token: Option<&str>,
) -> Result<PhaseActionDaemonRawClientResponse, String> {
    let body = body.unwrap_or("");
    let auth_header = bearer_token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let wire = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n{auth_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    let mut stream = TcpStream::connect(addr)
        .map_err(|error| format!("failed to connect to smoke server: {error}"))?;
    stream
        .write_all(wire.as_bytes())
        .map_err(|error| format!("failed to write HTTP request: {error}"))?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("failed to finish request write side: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read HTTP response: {error}"))?;
    parse_raw_client_response(&response)
}

fn send_http_request_with_declared_content_length(
    addr: SocketAddr,
    method: &str,
    path: &str,
    content_length: usize,
    bearer_token: Option<&str>,
) -> Result<PhaseActionDaemonRawClientResponse, String> {
    let auth_header = bearer_token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let wire = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n{auth_header}Content-Length: {content_length}\r\nConnection: close\r\n\r\n"
    );

    let mut stream = TcpStream::connect(addr)
        .map_err(|error| format!("failed to connect to smoke server: {error}"))?;
    stream
        .write_all(wire.as_bytes())
        .map_err(|error| format!("failed to write HTTP request: {error}"))?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| format!("failed to finish request write side: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read HTTP response: {error}"))?;
    parse_raw_client_response(&response)
}

fn parse_raw_client_response(response: &str) -> Result<PhaseActionDaemonRawClientResponse, String> {
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "response is missing header/body separator".to_owned())?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| "response is missing status line".to_owned())?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| "response status line is missing status code".to_owned())?
        .parse::<u16>()
        .map_err(|error| format!("invalid response status code: {error}"))?;
    Ok(PhaseActionDaemonRawClientResponse {
        status_code,
        body: body.to_owned(),
    })
}

fn parse_client_response<T: DeserializeOwned>(
    response: &PhaseActionDaemonRawClientResponse,
) -> Result<PhaseActionDaemonTypedClientResponse<T>, String> {
    let body = serde_json::from_str(&response.body)
        .map_err(|error| format!("failed to parse response JSON: {error}"))?;
    Ok(PhaseActionDaemonTypedClientResponse {
        status_code: response.status_code,
        body,
    })
}

fn raw_error_contains(response: &PhaseActionDaemonRawClientResponse, pattern: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(&response.body)
        .ok()
        .and_then(|body| {
            body.get("error")
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
        .is_some_and(|message| message.contains(pattern))
}

fn build_daemon_proof_suite_report(
    schema: &'static str,
    pass_verdict: &'static str,
    fail_verdict: &'static str,
    boundary: &'static str,
    live_rerun_performed: bool,
    live_rerun_step_count: usize,
) -> Result<PhaseActionDaemonProofSuiteReport, String> {
    let specs = daemon_proof_suite_specs();
    let mut artifacts = Vec::with_capacity(specs.len());
    for spec in specs {
        println!(
            "phase-action-daemon-proof-suite-v1: checking {}",
            spec.label
        );
        artifacts.push(evaluate_daemon_proof_suite_artifact(spec)?);
    }

    let artifact_count = artifacts.len();
    let pass_count = artifacts.iter().filter(|artifact| artifact.pass).count();
    let all_reports_pass = pass_count == artifact_count;
    let all_forbidden_flags_false = artifacts
        .iter()
        .all(|artifact| artifact.forbidden_flags_false);
    let all_python_demo_false = artifacts.iter().all(|artifact| artifact.python_demo_false);
    let all_server_runtime_hot_path_clean = artifacts.iter().all(|artifact| {
        artifact.server_runtime_compiler_false && artifact.server_runtime_corpus_jsonl_false
    });
    let all_false_local_accepts_zero = artifacts
        .iter()
        .filter_map(|artifact| artifact.false_local_accepts_zero)
        .all(|value| value);
    let pass = all_reports_pass
        && all_forbidden_flags_false
        && all_python_demo_false
        && all_server_runtime_hot_path_clean
        && all_false_local_accepts_zero;

    Ok(PhaseActionDaemonProofSuiteReport {
        schema,
        verdict: if pass { pass_verdict } else { fail_verdict },
        boundary,
        live_rerun_performed,
        live_rerun_step_count,
        artifact_count,
        pass_count,
        all_reports_pass,
        all_forbidden_flags_false,
        all_python_demo_false,
        all_server_runtime_hot_path_clean,
        all_false_local_accepts_zero,
        artifacts,
    })
}

fn daemon_proof_suite_specs() -> Vec<PhaseActionDaemonProofSuiteSpec> {
    vec![
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_smoke",
            path: DEFAULT_DAEMON_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_package_smoke",
            path: DEFAULT_DAEMON_PACKAGE_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_PACKAGE_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_hardening_smoke",
            path: DEFAULT_DAEMON_HARDENING_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_HARDENING_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_auth_smoke",
            path: DEFAULT_DAEMON_AUTH_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_AUTH_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_registry_smoke",
            path: DEFAULT_DAEMON_REGISTRY_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_REGISTRY_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_registry_config_smoke",
            path: DEFAULT_DAEMON_REGISTRY_CONFIG_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_REGISTRY_CONFIG_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_config_validation_smoke",
            path: DEFAULT_DAEMON_CONFIG_VALIDATION_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_CONFIG_VALIDATION_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_rate_limit_smoke",
            path: DEFAULT_DAEMON_RATE_LIMIT_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_RATE_LIMIT_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_observability_smoke",
            path: DEFAULT_DAEMON_OBSERVABILITY_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_OBSERVABILITY_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_audit_log_smoke",
            path: DEFAULT_DAEMON_AUDIT_LOG_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_AUDIT_LOG_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_error_taxonomy_smoke",
            path: DEFAULT_DAEMON_ERROR_TAXONOMY_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_ERROR_TAXONOMY_SMOKE_V1_PASS",
        },
        PhaseActionDaemonProofSuiteSpec {
            label: "daemon_systemd_smoke",
            path: DEFAULT_DAEMON_SYSTEMD_SMOKE_REPORT,
            expected_verdict: "PHASE_ACTION_DAEMON_SYSTEMD_SMOKE_V1_PASS",
        },
    ]
}

fn evaluate_daemon_proof_suite_artifact(
    spec: PhaseActionDaemonProofSuiteSpec,
) -> Result<PhaseActionDaemonProofSuiteArtifact, String> {
    let path = Path::new(spec.path);
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let report = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    let mut issues = Vec::new();
    check_string_field(&report, "verdict", spec.expected_verdict, &mut issues);
    check_bool_field(&report, "python_demo_used", false, &mut issues);
    check_bool_field(
        &report,
        "target_center_id_training_used",
        false,
        &mut issues,
    );
    check_bool_field(
        &report,
        "proof_rule_id_training_authority_used",
        false,
        &mut issues,
    );
    check_bool_field(&report, "concrete_x_lookup_used", false, &mut issues);
    check_bool_field(
        &report,
        "local_out_t_runtime_extension_used",
        false,
        &mut issues,
    );
    check_bool_field(&report, "server_runtime_compiler_used", false, &mut issues);
    check_bool_field(
        &report,
        "server_runtime_corpus_jsonl_used",
        false,
        &mut issues,
    );
    if report.get("false_local_accepts").is_some() {
        check_usize_field(&report, "false_local_accepts", 0, &mut issues);
    }

    match spec.label {
        "daemon_smoke" => {
            check_usize_field(&report, "http_bad_requests", 0, &mut issues);
            check_string_field(&report, "local_action", "local_operator", &mut issues);
            check_string_field(&report, "fallback_action", "fallback_to_llm", &mut issues);
        }
        "daemon_package_smoke" => {
            check_usize_field(&report, "http_bad_requests", 0, &mut issues);
            check_string_field(&report, "local_action", "local_operator", &mut issues);
            check_string_field(&report, "fallback_action", "fallback_to_llm", &mut issues);
            check_bool_field(
                &report,
                "request_fixture_corpus_jsonl_used",
                true,
                &mut issues,
            );
        }
        "daemon_hardening_smoke" => {
            check_usize_field(&report, "health_status_code", 200, &mut issues);
            check_usize_field(&report, "stats_status_code", 200, &mut issues);
            check_usize_field(&report, "bad_route_status_code", 404, &mut issues);
            check_usize_field(&report, "http_bad_requests", 1, &mut issues);
        }
        "daemon_auth_smoke" => {
            check_bool_field(&report, "auth_enabled", true, &mut issues);
            check_usize_field(&report, "unauthorized_score_status_code", 401, &mut issues);
            check_usize_field(&report, "authorized_score_status_code", 200, &mut issues);
            check_usize_field(&report, "authorized_stats_status_code", 200, &mut issues);
        }
        "daemon_registry_smoke" => {
            check_usize_field(&report, "package_count", 3, &mut issues);
            check_usize_field(&report, "missing_alias_status_code", 404, &mut issues);
            check_usize_field(&report, "local_operator_calls", 3, &mut issues);
        }
        "daemon_registry_config_smoke" => {
            check_usize_field(&report, "package_count", 3, &mut issues);
            check_usize_field(&report, "missing_alias_status_code", 404, &mut issues);
            check_bool_field(&report, "server_runtime_config_used", true, &mut issues);
        }
        "daemon_config_validation_smoke" => {
            check_bool_field(&report, "valid_registry_load_pass", true, &mut issues);
            check_usize_field(&report, "valid_package_count", 3, &mut issues);
            check_usize_field(&report, "invalid_case_count", 5, &mut issues);
            check_usize_field(&report, "invalid_reject_count", 5, &mut issues);
            check_bool_field(&report, "invalid_error_messages_pass", true, &mut issues);
            check_bool_field(
                &report,
                "server_started_for_invalid_configs",
                false,
                &mut issues,
            );
        }
        "daemon_rate_limit_smoke" => {
            check_usize_field(&report, "max_score_requests", 1, &mut issues);
            check_usize_field(&report, "rate_limited_score_status_code", 429, &mut issues);
            check_usize_field(&report, "http_rate_limited_requests", 1, &mut issues);
            check_usize_field(&report, "local_operator_calls", 1, &mut issues);
        }
        "daemon_observability_smoke" => {
            check_usize_field(&report, "package_count", 3, &mut issues);
            check_usize_field(&report, "score_requests_observed_by_stats", 1, &mut issues);
            check_usize_field(&report, "bad_requests_observed_by_stats", 2, &mut issues);
            check_usize_field(
                &report,
                "rate_limited_requests_observed_by_stats",
                1,
                &mut issues,
            );
            check_bool_field(&report, "server_runtime_config_used", true, &mut issues);
        }
        "daemon_audit_log_smoke" => {
            check_usize_field(&report, "audit_event_count", 6, &mut issues);
            check_bool_field(&report, "audit_sequences_are_dense", true, &mut issues);
            check_bool_field(&report, "audit_flags_pass", true, &mut issues);
            check_usize_array_field(
                &report,
                "audit_status_codes",
                &[200, 200, 404, 200, 429, 200],
                &mut issues,
            );
        }
        "daemon_error_taxonomy_smoke" => {
            check_bool_field(&report, "error_messages_pass", true, &mut issues);
            check_usize_field(&report, "http_score_requests", 0, &mut issues);
            check_usize_field(&report, "http_bad_requests", 7, &mut issues);
            check_usize_array_field(
                &report,
                "error_status_codes",
                &[400, 404, 413, 413, 400, 405, 413],
                &mut issues,
            );
        }
        "daemon_systemd_smoke" => {
            check_usize_field(&report, "package_count", 3, &mut issues);
            check_bool_field(
                &report,
                "service_manager_artifacts_written",
                true,
                &mut issues,
            );
            check_bool_field(&report, "service_exec_serve_registry", true, &mut issues);
            check_bool_field(&report, "service_hardening_pass", true, &mut issues);
            check_bool_field(&report, "env_registry_config_matches", true, &mut issues);
            check_bool_field(&report, "auth_token_placeholder_used", true, &mut issues);
            check_bool_field(&report, "installed_to_systemd", false, &mut issues);
            check_bool_field(&report, "systemctl_invoked", false, &mut issues);
            check_bool_field(&report, "server_runtime_config_used", true, &mut issues);
        }
        _ => issues.push(format!("unknown daemon proof suite label: {}", spec.label)),
    }

    let observed_verdict = report
        .get("verdict")
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let false_local_accepts_zero = report
        .get("false_local_accepts")
        .and_then(|value| value.as_u64())
        .map(|value| value == 0);
    let forbidden_flags_false = [
        "target_center_id_training_used",
        "proof_rule_id_training_authority_used",
        "concrete_x_lookup_used",
        "local_out_t_runtime_extension_used",
    ]
    .iter()
    .all(|field| report.get(*field).and_then(|value| value.as_bool()) == Some(false));
    let python_demo_false = report
        .get("python_demo_used")
        .and_then(|value| value.as_bool())
        == Some(false);
    let server_runtime_compiler_false = report
        .get("server_runtime_compiler_used")
        .and_then(|value| value.as_bool())
        == Some(false);
    let server_runtime_corpus_jsonl_false = report
        .get("server_runtime_corpus_jsonl_used")
        .and_then(|value| value.as_bool())
        == Some(false);

    Ok(PhaseActionDaemonProofSuiteArtifact {
        label: spec.label,
        path: spec.path.to_owned(),
        expected_verdict: spec.expected_verdict,
        observed_verdict,
        pass: issues.is_empty(),
        issue_count: issues.len(),
        issues,
        false_local_accepts_zero,
        forbidden_flags_false,
        python_demo_false,
        server_runtime_compiler_false,
        server_runtime_corpus_jsonl_false,
    })
}

fn check_string_field(
    report: &serde_json::Value,
    field: &str,
    expected: &str,
    issues: &mut Vec<String>,
) {
    match report.get(field).and_then(|value| value.as_str()) {
        Some(actual) if actual == expected => {}
        Some(actual) => issues.push(format!("{field}: expected '{expected}', got '{actual}'")),
        None => issues.push(format!("{field}: missing or not string")),
    }
}

fn check_bool_field(
    report: &serde_json::Value,
    field: &str,
    expected: bool,
    issues: &mut Vec<String>,
) {
    match report.get(field).and_then(|value| value.as_bool()) {
        Some(actual) if actual == expected => {}
        Some(actual) => issues.push(format!("{field}: expected {expected}, got {actual}")),
        None => issues.push(format!("{field}: missing or not bool")),
    }
}

fn check_usize_field(
    report: &serde_json::Value,
    field: &str,
    expected: usize,
    issues: &mut Vec<String>,
) {
    match report.get(field).and_then(|value| value.as_u64()) {
        Some(actual) if actual == expected as u64 => {}
        Some(actual) => issues.push(format!("{field}: expected {expected}, got {actual}")),
        None => issues.push(format!("{field}: missing or not unsigned integer")),
    }
}

fn check_usize_array_field(
    report: &serde_json::Value,
    field: &str,
    expected: &[usize],
    issues: &mut Vec<String>,
) {
    let Some(actual_values) = report.get(field).and_then(|value| value.as_array()) else {
        issues.push(format!("{field}: missing or not array"));
        return;
    };
    let actual = actual_values
        .iter()
        .map(|value| value.as_u64().map(|value| value as usize))
        .collect::<Option<Vec<_>>>();
    match actual {
        Some(actual) if actual == expected => {}
        Some(actual) => issues.push(format!(
            "{field}: expected {:?}, got {:?}",
            expected, actual
        )),
        None => issues.push(format!("{field}: contains non unsigned integer")),
    }
}

fn write_http_json<T: Serialize>(
    stream: &mut TcpStream,
    status_code: u16,
    body: &T,
) -> Result<(), String> {
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        _ => "Error",
    };
    let body = serde_json::to_string(body)
        .map_err(|error| format!("failed to encode response JSON: {error}"))?;
    let response = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| format!("failed to write response: {error}"))
}

fn write_http_json_error(
    stream: &mut TcpStream,
    status_code: u16,
    message: &str,
) -> Result<(), String> {
    write_http_json(
        stream,
        status_code,
        &PhaseActionDaemonErrorResponse { error: message },
    )
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {}: {error}", path.display()))?;
    std::fs::write(path, json)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn write_text_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, text)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn read_json_value(path: &Path) -> Result<serde_json::Value, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn read_text_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn file_fingerprint64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn inspect_file(path: &Path) -> Result<(u64, usize), String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Ok((file_fingerprint64(&bytes), bytes.len()))
}

fn absolute_path_for_unit(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn json_string_value<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(|value| value.as_str())
}

fn json_string_field(value: &serde_json::Value, field: &str) -> Result<String, String> {
    json_string_value(value, field)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing or invalid string field: {field}"))
}

fn json_bool_value(value: &serde_json::Value, field: &str) -> Option<bool> {
    value.get(field).and_then(|value| value.as_bool())
}

fn json_usize_value(value: &serde_json::Value, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonScoreRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    package_alias: Option<String>,
    center_index: usize,
    candidate_atoms: Vec<String>,
    counterfactual_atoms: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonScoreResponse {
    action: String,
    package_alias: String,
    margin_micro: i64,
    margin_threshold_micro: i64,
    package_fingerprint64: u64,
}

impl PhaseActionDaemonScoreResponse {
    fn from_decision(
        decision: PhaseCenterOffloadDecision,
        package_alias: &str,
        package_fingerprint64: u64,
    ) -> Self {
        Self {
            action: match decision.action {
                PhaseCenterOffloadAction::LocalOperator => "local_operator".to_owned(),
                PhaseCenterOffloadAction::FallbackToLlm => "fallback_to_llm".to_owned(),
            },
            package_alias: package_alias.to_owned(),
            margin_micro: decision.margin_micro,
            margin_threshold_micro: decision.margin_threshold_micro,
            package_fingerprint64,
        }
    }

    fn action_enum(&self) -> PhaseCenterOffloadAction {
        if self.action == "local_operator" {
            PhaseCenterOffloadAction::LocalOperator
        } else {
            PhaseCenterOffloadAction::FallbackToLlm
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonErrorResponse<'a> {
    error: &'a str,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonPackageSummary {
    alias: String,
    package_fingerprint64: u64,
    package_cells: usize,
    package_record_count: usize,
    package_serialized_len: usize,
    runtime_bytes_estimate: usize,
}

impl PhaseActionDaemonPackageSummary {
    fn from_package(package: &PhaseActionDaemonRuntimePackage) -> Self {
        let package_info = package.package_info();
        Self {
            alias: package.alias.clone(),
            package_fingerprint64: package_info.fingerprint64,
            package_cells: package_info.cells,
            package_record_count: package_info.record_count,
            package_serialized_len: package_info.serialized_len,
            runtime_bytes_estimate: package.runtime.bytes_estimate(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonHealthResponse {
    schema: String,
    status: String,
    package_count: usize,
    package_fingerprint64: u64,
    package_cells: usize,
    package_record_count: usize,
    package_serialized_len: usize,
    runtime_bytes_estimate: usize,
    margin_threshold_micro: i64,
    runtime_path: String,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
}

impl PhaseActionDaemonHealthResponse {
    fn from_registry(registry: &PhaseActionDaemonRuntimeRegistry) -> Self {
        let default_package = registry.default_package();
        let package_info = default_package.package_info();
        Self {
            schema: "nando_phase_action_daemon_health_v1".to_owned(),
            status: "ok".to_owned(),
            package_count: registry.package_count(),
            package_fingerprint64: package_info.fingerprint64,
            package_cells: package_info.cells,
            package_record_count: package_info.record_count,
            package_serialized_len: package_info.serialized_len,
            runtime_bytes_estimate: default_package.runtime.bytes_estimate(),
            margin_threshold_micro: default_package.runtime.policy().margin_threshold_micro,
            runtime_path: "nando_core::PhaseCenterOffloadRuntime".to_owned(),
            server_runtime_compiler_used: false,
            server_runtime_corpus_jsonl_used: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonPackagesResponse {
    schema: String,
    package_count: usize,
    packages: Vec<PhaseActionDaemonPackageSummary>,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
}

impl PhaseActionDaemonPackagesResponse {
    fn from_registry(registry: &PhaseActionDaemonRuntimeRegistry) -> Self {
        Self {
            schema: "nando_phase_action_daemon_packages_v1".to_owned(),
            package_count: registry.package_count(),
            packages: registry.package_summaries(),
            server_runtime_compiler_used: false,
            server_runtime_corpus_jsonl_used: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonStatsResponse {
    schema: String,
    package_count: usize,
    package_aliases: Vec<String>,
    package_fingerprint64: u64,
    requests_handled: usize,
    score_requests: usize,
    health_requests: usize,
    packages_requests: usize,
    stats_requests: usize,
    bad_requests: usize,
    rate_limited_requests: usize,
    max_score_requests: Option<usize>,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonAuditLogEntry {
    schema: String,
    sequence: usize,
    status_code: u16,
    request_kind: String,
    package_alias: Option<String>,
    action: Option<String>,
    margin_micro: Option<i64>,
    error: Option<String>,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
}

impl PhaseActionDaemonStatsResponse {
    fn from_registry(
        registry: &PhaseActionDaemonRuntimeRegistry,
        stats: PhaseActionDaemonServeStats,
        config: &PhaseActionDaemonServeConfig,
    ) -> Self {
        Self {
            schema: "nando_phase_action_daemon_stats_v1".to_owned(),
            package_count: registry.package_count(),
            package_aliases: registry
                .package_summaries()
                .into_iter()
                .map(|package| package.alias)
                .collect(),
            package_fingerprint64: registry.default_package().package_info().fingerprint64,
            requests_handled: stats.requests_handled,
            score_requests: stats.score_requests,
            health_requests: stats.health_requests,
            packages_requests: stats.packages_requests,
            stats_requests: stats.stats_requests,
            bad_requests: stats.bad_requests,
            rate_limited_requests: stats.rate_limited_requests,
            max_score_requests: config.max_score_requests,
            local_operator_calls: stats.local_operator_calls,
            fallback_to_llm_calls: stats.fallback_to_llm_calls,
            false_local_accepts: stats.false_local_accepts,
            server_runtime_config_used: config.server_runtime_config_used,
            server_runtime_compiler_used: false,
            server_runtime_corpus_jsonl_used: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug)]
struct PhaseActionDaemonRawClientResponse {
    status_code: u16,
    body: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonTypedClientResponse<T> {
    status_code: u16,
    body: T,
}

type PhaseActionDaemonClientResponse =
    PhaseActionDaemonTypedClientResponse<PhaseActionDaemonScoreResponse>;
type PhaseActionDaemonHealthClientResponse =
    PhaseActionDaemonTypedClientResponse<PhaseActionDaemonHealthResponse>;
type PhaseActionDaemonPackagesClientResponse =
    PhaseActionDaemonTypedClientResponse<PhaseActionDaemonPackagesResponse>;
type PhaseActionDaemonStatsClientResponse =
    PhaseActionDaemonTypedClientResponse<PhaseActionDaemonStatsResponse>;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonRegistryConfig {
    schema: String,
    packages: Vec<PhaseActionDaemonRegistryConfigPackage>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PhaseActionDaemonRegistryConfigPackage {
    alias: String,
    package_path: String,
    manifest_path: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PhaseActionPackageManifestFixture {
    cells: usize,
    flat_records: usize,
    operator_keys: Vec<String>,
    package_fingerprint64: u64,
}

#[derive(Clone, Debug, Deserialize)]
struct PhaseActionContractRow {
    task_id: String,
    split: String,
    state_before: String,
    action_tree: PhaseActionTree,
    state_after_correct: String,
    state_after_wrong: String,
}

#[derive(Clone, Debug, Deserialize)]
struct PhaseActionTree {
    select: String,
    transform: String,
    write: String,
    condition: String,
    check: String,
}

#[derive(Clone, Debug)]
struct PhaseActionDaemonFixture {
    task_id: String,
    center_index: usize,
    operator_key: String,
    local_request: PhaseActionDaemonScoreRequest,
    fallback_request: PhaseActionDaemonScoreRequest,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    package_fingerprint64: u64,
    package_cells: usize,
    package_record_count: usize,
    package_serialized_len: usize,
    runtime_bytes_estimate: usize,
    margin_threshold_micro: i64,
    http_requests: usize,
    http_requests_handled: usize,
    http_bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    local_status_code: u16,
    fallback_status_code: u16,
    local_margin_micro: i64,
    fallback_margin_micro: i64,
    local_action: String,
    fallback_action: String,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonPackageSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    package_path: String,
    manifest_path: String,
    corpus_path: String,
    package_fingerprint64: u64,
    package_cells: usize,
    package_record_count: usize,
    package_serialized_len: usize,
    runtime_bytes_estimate: usize,
    margin_threshold_micro: i64,
    fixture_task_id: String,
    fixture_center_index: usize,
    fixture_operator_key: String,
    http_requests: usize,
    http_requests_handled: usize,
    http_bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    local_status_code: u16,
    fallback_status_code: u16,
    local_margin_micro: i64,
    fallback_margin_micro: i64,
    local_action: String,
    fallback_action: String,
    request_fixture_corpus_jsonl_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonHardeningSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    package_path: String,
    manifest_path: String,
    corpus_path: String,
    package_fingerprint64: u64,
    package_cells: usize,
    package_record_count: usize,
    package_serialized_len: usize,
    runtime_bytes_estimate: usize,
    margin_threshold_micro: i64,
    http_max_request_bytes: usize,
    max_score_atoms: usize,
    max_score_atom_bytes: usize,
    fixture_task_id: String,
    fixture_center_index: usize,
    fixture_operator_key: String,
    health_status_code: u16,
    stats_status_code: u16,
    bad_route_status_code: u16,
    local_status_code: u16,
    fallback_status_code: u16,
    local_action: String,
    fallback_action: String,
    local_margin_micro: i64,
    fallback_margin_micro: i64,
    http_requests: usize,
    http_requests_handled: usize,
    http_score_requests: usize,
    http_health_requests: usize,
    http_stats_requests: usize,
    http_bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    request_fixture_corpus_jsonl_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonAuthSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    package_path: String,
    manifest_path: String,
    corpus_path: String,
    package_fingerprint64: u64,
    package_cells: usize,
    package_record_count: usize,
    package_serialized_len: usize,
    runtime_bytes_estimate: usize,
    margin_threshold_micro: i64,
    fixture_task_id: String,
    fixture_center_index: usize,
    fixture_operator_key: String,
    auth_enabled: bool,
    health_public_status_code: u16,
    unauthorized_score_status_code: u16,
    authorized_score_status_code: u16,
    authorized_fallback_status_code: u16,
    authorized_stats_status_code: u16,
    local_action: String,
    fallback_action: String,
    local_margin_micro: i64,
    fallback_margin_micro: i64,
    http_requests: usize,
    http_requests_handled: usize,
    http_score_requests: usize,
    http_health_requests: usize,
    http_stats_requests: usize,
    http_bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    request_fixture_corpus_jsonl_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonRegistrySmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    package_aliases: Vec<String>,
    package_count: usize,
    generated_package_path: String,
    domain_package_path: String,
    coverage_package_path: String,
    generated_package_fingerprint64: u64,
    domain_package_fingerprint64: u64,
    coverage_package_fingerprint64: u64,
    generated_fixture_task_id: String,
    domain_fixture_task_id: String,
    coverage_fixture_task_id: String,
    generated_status_code: u16,
    domain_status_code: u16,
    coverage_status_code: u16,
    missing_alias_status_code: u16,
    packages_status_code: u16,
    stats_status_code: u16,
    health_status_code: u16,
    generated_action: String,
    domain_action: String,
    coverage_action: String,
    generated_margin_micro: i64,
    domain_margin_micro: i64,
    coverage_margin_micro: i64,
    http_requests: usize,
    http_requests_handled: usize,
    http_score_requests: usize,
    http_health_requests: usize,
    http_packages_requests: usize,
    http_stats_requests: usize,
    http_bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    request_fixture_corpus_jsonl_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonRegistryConfigSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    registry_config_path: String,
    registry_config_written_by_smoke: bool,
    package_aliases: Vec<String>,
    package_count: usize,
    generated_package_fingerprint64: u64,
    domain_package_fingerprint64: u64,
    coverage_package_fingerprint64: u64,
    generated_status_code: u16,
    domain_status_code: u16,
    coverage_status_code: u16,
    missing_alias_status_code: u16,
    packages_status_code: u16,
    stats_status_code: u16,
    health_status_code: u16,
    generated_action: String,
    domain_action: String,
    coverage_action: String,
    generated_margin_micro: i64,
    domain_margin_micro: i64,
    coverage_margin_micro: i64,
    http_requests: usize,
    http_requests_handled: usize,
    http_score_requests: usize,
    http_health_requests: usize,
    http_packages_requests: usize,
    http_stats_requests: usize,
    http_bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    request_fixture_corpus_jsonl_used: bool,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonConfigValidationSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    registry_config_path: String,
    valid_registry_load_pass: bool,
    valid_package_count: usize,
    invalid_case_count: usize,
    invalid_reject_count: usize,
    invalid_error_messages_pass: bool,
    invalid_cases: Vec<PhaseActionDaemonConfigValidationCase>,
    server_started_for_invalid_configs: bool,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonConfigValidationCase {
    case: &'static str,
    config_path: String,
    rejected: bool,
    expected_error: &'static str,
    observed_error: Option<String>,
    expected_error_matched: bool,
}

#[derive(Clone, Copy, Debug)]
struct PhaseActionDaemonProofSuiteSpec {
    label: &'static str,
    path: &'static str,
    expected_verdict: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonProofSuiteReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    live_rerun_performed: bool,
    live_rerun_step_count: usize,
    artifact_count: usize,
    pass_count: usize,
    all_reports_pass: bool,
    all_forbidden_flags_false: bool,
    all_python_demo_false: bool,
    all_server_runtime_hot_path_clean: bool,
    all_false_local_accepts_zero: bool,
    artifacts: Vec<PhaseActionDaemonProofSuiteArtifact>,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonProofSuiteArtifact {
    label: &'static str,
    path: String,
    expected_verdict: &'static str,
    observed_verdict: Option<String>,
    pass: bool,
    issue_count: usize,
    issues: Vec<String>,
    false_local_accepts_zero: Option<bool>,
    forbidden_flags_false: bool,
    python_demo_false: bool,
    server_runtime_compiler_false: bool,
    server_runtime_corpus_jsonl_false: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonSystemdSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    service_unit_path: String,
    env_file_path: String,
    registry_config_path: String,
    audit_log_path: String,
    package_count: usize,
    service_unit_bytes: usize,
    env_file_bytes: usize,
    registry_config_bytes: usize,
    service_manager_artifacts_written: bool,
    service_exec_serve_registry: bool,
    service_environment_file_matches: bool,
    service_restart_on_failure: bool,
    service_hardening_pass: bool,
    env_registry_config_matches: bool,
    env_bind_addr: &'static str,
    env_margin_threshold_micro: i64,
    env_max_score_requests: usize,
    auth_token_placeholder_used: bool,
    installed_to_systemd: bool,
    systemctl_invoked: bool,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonDeploymentPackageReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    live_suite_report_path: String,
    live_suite_report_fingerprint64: u64,
    live_suite_report_bytes: usize,
    systemd_report_path: String,
    systemd_report_fingerprint64: u64,
    systemd_report_bytes: usize,
    service_unit_path: String,
    service_unit_fingerprint64: u64,
    service_unit_bytes: usize,
    env_file_path: String,
    env_file_fingerprint64: u64,
    env_file_bytes: usize,
    registry_config_path: String,
    registry_config_fingerprint64: u64,
    registry_config_bytes: usize,
    live_suite_pass: bool,
    live_suite_artifact_count: usize,
    live_suite_step_count: usize,
    live_suite_contains_systemd: bool,
    live_suite_hot_path_clean: bool,
    live_suite_forbidden_flags_false: bool,
    live_suite_python_demo_false: bool,
    live_suite_false_local_accepts_zero: bool,
    systemd_smoke_pass: bool,
    systemd_artifacts_written: bool,
    systemd_hardening_pass: bool,
    systemd_auth_placeholder_used: bool,
    systemd_not_installed: bool,
    systemctl_not_invoked: bool,
    systemd_hot_path_clean: bool,
    systemd_forbidden_flags_false: bool,
    service_unit_exec_matches: bool,
    service_unit_env_matches: bool,
    env_file_config_matches: bool,
    registry_config_package_count: usize,
    registry_config_package_count_matches: bool,
    deployment_artifacts_present: bool,
    installed_to_systemd: bool,
    systemctl_invoked: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonRateLimitSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    registry_config_path: String,
    registry_config_written_by_smoke: bool,
    package_count: usize,
    package_aliases: Vec<String>,
    max_score_requests: usize,
    health_status_code: u16,
    packages_status_code: u16,
    allowed_score_status_code: u16,
    rate_limited_score_status_code: u16,
    stats_status_code: u16,
    allowed_action: String,
    allowed_margin_micro: i64,
    http_requests: usize,
    http_requests_handled: usize,
    http_score_requests: usize,
    http_health_requests: usize,
    http_packages_requests: usize,
    http_stats_requests: usize,
    http_bad_requests: usize,
    http_rate_limited_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    request_fixture_corpus_jsonl_used: bool,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonObservabilitySmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    registry_config_path: String,
    registry_config_written_by_smoke: bool,
    package_count: usize,
    package_aliases: Vec<String>,
    max_score_requests: Option<usize>,
    health_status_code: u16,
    packages_status_code: u16,
    missing_alias_status_code: u16,
    allowed_score_status_code: u16,
    rate_limited_score_status_code: u16,
    stats_status_code: u16,
    requests_handled_observed_by_stats: usize,
    score_requests_observed_by_stats: usize,
    bad_requests_observed_by_stats: usize,
    rate_limited_requests_observed_by_stats: usize,
    local_operator_calls_observed_by_stats: usize,
    fallback_to_llm_calls_observed_by_stats: usize,
    false_local_accepts_observed_by_stats: usize,
    requests_handled_final: usize,
    score_requests_final: usize,
    bad_requests_final: usize,
    rate_limited_requests_final: usize,
    local_operator_calls_final: usize,
    fallback_to_llm_calls_final: usize,
    false_local_accepts_final: usize,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonAuditLogSmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    registry_config_path: String,
    audit_log_path: String,
    audit_event_count: usize,
    audit_status_codes: Vec<u16>,
    audit_request_kinds: Vec<String>,
    audit_sequences_are_dense: bool,
    audit_missing_alias_event_found: bool,
    audit_rate_limit_event_found: bool,
    audit_local_operator_event_found: bool,
    audit_flags_pass: bool,
    health_status_code: u16,
    packages_status_code: u16,
    missing_alias_status_code: u16,
    allowed_score_status_code: u16,
    rate_limited_score_status_code: u16,
    stats_status_code: u16,
    http_requests_handled: usize,
    http_bad_requests: usize,
    http_rate_limited_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}

#[derive(Clone, Debug, Serialize)]
struct PhaseActionDaemonErrorTaxonomySmokeReport {
    schema: &'static str,
    verdict: &'static str,
    boundary: &'static str,
    registry_config_path: String,
    health_status_code: u16,
    malformed_json_status_code: u16,
    missing_alias_status_code: u16,
    too_many_atoms_status_code: u16,
    too_long_atom_status_code: u16,
    out_of_bounds_status_code: u16,
    unsupported_method_status_code: u16,
    oversized_request_status_code: u16,
    stats_status_code: u16,
    error_status_codes: Vec<u16>,
    error_messages_pass: bool,
    http_requests_handled: usize,
    http_score_requests: usize,
    http_bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    stats_requests_handled_before_stats: usize,
    stats_bad_requests_before_stats: usize,
    stats_score_requests_before_stats: usize,
    server_runtime_config_used: bool,
    server_runtime_compiler_used: bool,
    server_runtime_corpus_jsonl_used: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
}
