use nando_core::wave::{
    WavePredictorRoleBindingOffloadPolicy, WavePredictorRoleBindingOffloadRuntime,
    WavePredictorRoleBindingPackageInfo, WavePredictorRoleBindingPreparedFringe,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT: &str =
    "target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json";
const DEFAULT_ROLE_BINDING_PROFILE_RUNTIME_SMOKE_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-runtime-smoke-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT: &str = "target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_REPLAY_SUITE_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-replay-suite-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_FALLBACK_SMOKE_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-fallback-smoke-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_WORKER_SCALING_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-worker-scaling-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_WORKER_REPLAY_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-worker-replay-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_LB_REPLAY_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_LB_THROUGHPUT_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/real-traffic-shadow-v1.trace.jsonl";
const DEFAULT_ROLE_BINDING_REAL_TRAFFIC_SHADOW_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/real-traffic-shadow-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_REAL_TRAFFIC_INGEST_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/real-traffic-ingest-v1.report.json";
const DEFAULT_CODEX_HISTORY_REAL_TRAFFIC_EVENTS_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-events-v1.events.jsonl";
const DEFAULT_CODEX_HISTORY_REAL_TRAFFIC_INGEST_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-events-v1.report.json";
const DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_EVENTS_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.events.jsonl";
const DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.report.json";
const DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_SHADOW_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.shadow-report.json";
const DEFAULT_REAL_TRAFFIC_CPU_ROUTE_FORECAST_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/cpu-route-forecast-v1.report.json";
const DEFAULT_EDIT_PAYLOAD_READINESS_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-payload-readiness-v1.report.json";
const ROLE_BINDING_EVAL_PACK_BINARY_MAGIC: [u8; 8] = *b"NWRE0001";
const HTTP_READ_TIMEOUT_SECS: u64 = 10;
const MAX_HTTP_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE: usize = 128;
const DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES: usize = 4;
const DEFAULT_PROFILE_WORKER_COUNT: usize = 2;
const DEFAULT_THROUGHPUT_CLIENT_THREADS: usize = 4;
const DEFAULT_THROUGHPUT_SEQUENCE_REPETITIONS: usize = 1;

pub(crate) fn run_role_binding_profile_registry_from_release_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let release_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT));
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));

    let release_suite =
        read_json_file::<RoleBindingRuntimeReleaseSuiteReport>(&release_suite_path)?;
    if !release_suite.gate_pass {
        return Err(format!(
            "release-suite is not green: verdict={}",
            release_suite.verdict
        ));
    }

    let mut profiles = Vec::new();
    for row in release_suite.rows {
        profiles.push(RoleBindingProfileConfig {
            profile_id: role_binding_profile_id(&row.label, row.seed),
            profile_kind: "role_binding_nwrb".to_owned(),
            operator_classes: role_binding_operator_classes(&row.label),
            package_path: row.package_path,
            runtime_bytes_estimate: 0,
            edge_count: row.package_edge_count,
            slot_count: 32,
            threshold: row.margin_threshold,
            accepted_route_keys: vec![
                role_binding_profile_id(&row.label, row.seed),
                format!("{}-seed{}", row.label, row.seed),
            ],
        });
    }

    let config = RoleBindingProfileRegistryConfig {
        schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
        profiles,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        claim_boundary: "serving registry generated from a green release-suite report; it references only .nwrb runtime packages and does not load .nwreb eval packs in serving mode".to_owned(),
    };
    validate_registry_config(&config)?;
    write_json_file(&config_path, &config)?;

    println!(
        "role-binding-profile-registry-from-release-v1: ROLE_BINDING_PROFILE_REGISTRY_FROM_RELEASE_V1_PASS"
    );
    println!("  release_suite: {}", release_suite_path.display());
    println!("  registry_config: {}", config_path.display());
    println!("  profiles: {}", config.profiles.len());
    println!("  compiler_used: false");
    println!("  eval_packs_loaded: false");
    Ok(())
}

pub(crate) fn run_role_binding_profile_serve_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let bind_addr = args.next().unwrap_or_else(|| "127.0.0.1:18090".to_owned());
    let request_limit = parse_optional_usize(args.next(), "request_limit")?;
    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|error| format!("failed to bind {bind_addr}: {error}"))?;

    println!("role-binding-profile-serve-v1: serving .nwrb profile registry");
    println!("  bind_addr: {bind_addr}");
    println!("  registry_config: {}", config_path.display());
    println!("  profiles: {}", registry.profile_count());
    println!("  endpoint: GET /health");
    println!("  endpoint: GET /profiles");
    println!("  endpoint: POST /score");
    println!("  endpoint: POST /score-compact");
    println!("  endpoint: POST /replay");
    println!("  endpoint: GET /metrics");
    println!("  compiler_used: false");
    println!("  eval_packs_loaded: false");
    println!("  corpus_jsonl_loaded: false");

    let stats = serve_role_binding_profile_requests(
        listener,
        Arc::new(registry),
        RoleBindingProfileServeConfig { request_limit },
    )?;
    println!("role-binding-profile-serve-v1: stopped");
    println!("  requests_handled: {}", stats.requests_handled);
    println!("  score_requests: {}", stats.score_requests);
    println!("  replay_requests: {}", stats.replay_requests);
    println!("  false_local_accepts: {}", stats.false_local_accepts);
    Ok(())
}

pub(crate) fn run_role_binding_profile_lb_serve_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "role-binding-profile-lb-serve-v1 requires lb-config-json".to_owned())?;
    let bind_addr = args.next().unwrap_or_else(|| "127.0.0.1:18190".to_owned());
    let request_limit = parse_optional_usize(args.next(), "request_limit")?;
    let load_balancer = RoleBindingProfileLoadBalancerRuntime::from_config_path(&config_path)?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|error| format!("failed to bind {bind_addr}: {error}"))?;

    println!("role-binding-profile-lb-serve-v1: serving profile load-balancer");
    println!("  bind_addr: {bind_addr}");
    println!("  lb_config: {}", config_path.display());
    println!("  upstreams: {}", load_balancer.upstream_count());
    println!("  profiles: {}", load_balancer.profile_count());
    println!("  endpoint: GET /health");
    println!("  endpoint: GET /profiles");
    println!("  endpoint: POST /score");
    println!("  endpoint: POST /replay");
    println!("  endpoint: GET /metrics");
    println!("  compiler_used: false");
    println!("  eval_packs_loaded: false");
    println!("  corpus_jsonl_loaded: false");

    let stats = serve_role_binding_profile_lb_requests(
        listener,
        Arc::new(load_balancer),
        RoleBindingProfileServeConfig { request_limit },
    )?;
    println!("role-binding-profile-lb-serve-v1: stopped");
    println!("  requests_handled: {}", stats.requests_handled);
    println!("  score_requests: {}", stats.score_requests);
    println!("  replay_requests: {}", stats.replay_requests);
    println!("  false_local_accepts: {}", stats.false_local_accepts);
    Ok(())
}

pub(crate) fn run_role_binding_profile_runtime_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_RUNTIME_SMOKE_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let local_score = registry
        .self_test_score_request()
        .ok_or_else(|| "registry has no scorable positive role-binding edge".to_owned())?;
    let fallback_score = RoleBindingProfileScoreRequest {
        request_id: "smoke_fallback".to_owned(),
        route_key: local_score.route_key.clone(),
        profile_id: local_score.profile_id.clone(),
        exact_cache_key: Some("smoke_fallback_cache_key".to_owned()),
        active_fringe: Vec::new(),
        slots: local_score.slots.clone(),
        expect_local_operator: Some(false),
    };
    let replay = RoleBindingProfileReplayRequest {
        request_id: "smoke_replay".to_owned(),
        requests: vec![local_score.clone(), fallback_score.clone()],
    };

    let bind_addr = reserve_local_bind_addr()?;
    let mut server = RoleBindingProfileServeProcess::start(&config_path, &bind_addr, Some(7))?;
    let addr = bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid smoke bind addr {bind_addr}: {error}"))?;
    let health = send_json_get::<RoleBindingProfileHealthResponse>(addr, "/health")?;
    let profiles = send_json_get::<RoleBindingProfileProfilesResponse>(addr, "/profiles")?;
    let score = send_json_post::<_, RoleBindingProfileScoreResponse>(addr, "/score", &local_score)?;
    let replay_response =
        send_json_post::<_, RoleBindingProfileReplayResponse>(addr, "/replay", &replay)?;
    let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(addr, "/metrics")?;
    server.wait()?;

    let pass = health.status == "ok"
        && !health.compiler_used
        && !health.eval_packs_loaded
        && !health.corpus_jsonl_loaded
        && profiles.profile_count > 0
        && score.accepted
        && !score.fallback
        && score.energy_margin >= score.threshold
        && replay_response.exact_cache_incremental_reduction_milli >= 200
        && replay_response.false_local_accepts == 0
        && metrics.false_local_accepts == 0
        && metrics.runtime_bytes_estimate <= 3 * 1024 * 1024;

    let report = RoleBindingProfileRuntimeSmokeReport {
        schema_version: "nando_role_binding_profile_runtime_smoke_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        profile_count: profiles.profile_count,
        endpoint_health_pass: health.status == "ok",
        endpoint_profiles_pass: profiles.profile_count > 0,
        endpoint_score_pass: score.accepted && !score.fallback,
        endpoint_replay_pass: replay_response.exact_cache_incremental_reduction_milli >= 200,
        endpoint_metrics_pass: metrics.false_local_accepts == 0,
        exact_cache_llm_calls: replay_response.exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls: replay_response.exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli: replay_response
            .exact_cache_incremental_reduction_milli,
        false_local_accepts: replay_response.false_local_accepts,
        p50_latency_ns: metrics.p50_latency_ns,
        p90_latency_ns: metrics.p90_latency_ns,
        p99_latency_ns: metrics.p99_latency_ns,
        rss_bytes: metrics.rss_bytes,
        runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        claim_boundary: "HTTP serving smoke over .nwrb role-binding profiles: /health, /profiles, /score, /replay, /metrics, exact-cache comparison, counters, and latency/RSS reporting; not real Codex traffic, not .nwreb serving, not compiler/training/corpus in hot path".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-runtime-smoke-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  profile_count: {}", report.profile_count);
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  exact_cache_incremental_reduction_milli: {}",
        report.exact_cache_incremental_reduction_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!(
        "  runtime_bytes_estimate: {}",
        report.runtime_bytes_estimate
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile runtime smoke failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_replay_suite_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REPLAY_SUITE_REPORT));
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE);
    let batch_unique_sequences = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid batch_unique_sequences '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES);
    if batch_unique_sequences == 0 {
        return Err("batch_unique_sequences must be positive".to_owned());
    }

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }

    let bind_addr = reserve_local_bind_addr()?;
    let _server = RoleBindingProfileServeProcess::start(&config_path, &bind_addr, None)?;
    let addr = bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid replay bind addr {bind_addr}: {error}"))?;
    let health = send_json_get::<RoleBindingProfileHealthResponse>(addr, "/health")?;
    let profiles = send_json_get::<RoleBindingProfileProfilesResponse>(addr, "/profiles")?;

    let mut rows = Vec::new();
    let mut total_unique_sequences = 0usize;
    let mut total_http_replay_batches = 0usize;
    let mut no_cache_llm_calls = 0usize;
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        if !registry
            .profiles
            .iter()
            .any(|profile| profile.config.profile_id == profile_id)
        {
            return Err(format!(
                "profile {} from binary suite is missing in serving registry",
                profile_id
            ));
        }
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        if eval_pack.package_fingerprint64 != Some(suite_row.package_fingerprint64) {
            return Err(format!(
                "binary eval-pack fingerprint mismatch for {} seed{}: pack={:?} suite={}",
                suite_row.label,
                suite_row.seed,
                eval_pack.package_fingerprint64,
                suite_row.package_fingerprint64
            ));
        }
        if eval_pack.generation_method.trim().is_empty() {
            return Err(format!(
                "binary eval-pack generation_method is empty for {}",
                eval_pack_path.display()
            ));
        }
        let _source_package_path = eval_pack.source_package_path.as_deref().unwrap_or("");
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        let mut row_unique_sequences = 0usize;
        let mut row_exact_cache_llm_calls = 0usize;
        let mut row_exact_cache_plus_nando_llm_calls = 0usize;
        let mut row_false_local_accepts = 0usize;
        let mut row_missed_expected_local = 0usize;
        let mut batch = Vec::new();
        let mut batch_index = 0usize;

        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            row_unique_sequences += 1;
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                0,
            ));
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                1,
            ));
            if row_unique_sequences.is_multiple_of(batch_unique_sequences) {
                let response = send_replay_batch(addr, &profile_id, batch_index, &mut batch)?;
                total_http_replay_batches += 1;
                batch_index += 1;
                row_exact_cache_llm_calls += response.exact_cache_llm_calls;
                row_exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
                row_false_local_accepts += response.false_local_accepts;
                row_missed_expected_local += response.missed_expected_local;
                local_operator_calls += response.local_operator_calls;
                fallback_to_llm_calls += response.fallback_to_llm_calls;
            }
        }
        if !batch.is_empty() {
            let response = send_replay_batch(addr, &profile_id, batch_index, &mut batch)?;
            total_http_replay_batches += 1;
            row_exact_cache_llm_calls += response.exact_cache_llm_calls;
            row_exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
            row_false_local_accepts += response.false_local_accepts;
            row_missed_expected_local += response.missed_expected_local;
            local_operator_calls += response.local_operator_calls;
            fallback_to_llm_calls += response.fallback_to_llm_calls;
        }

        let row_reduction_milli = reduction_milli(
            row_exact_cache_llm_calls,
            row_exact_cache_plus_nando_llm_calls,
        );
        rows.push(RoleBindingProfileReplaySuiteRow {
            profile_id,
            label: suite_row.label.clone(),
            seed: suite_row.seed,
            binary_eval_pack_path: suite_row.binary_eval_pack_path.clone(),
            unique_sequences_replayed: row_unique_sequences,
            no_cache_llm_calls: row_unique_sequences * 2,
            exact_cache_llm_calls: row_exact_cache_llm_calls,
            exact_cache_plus_nando_llm_calls: row_exact_cache_plus_nando_llm_calls,
            exact_cache_incremental_reduction_milli: row_reduction_milli,
            false_local_accepts: row_false_local_accepts,
            missed_expected_local: row_missed_expected_local,
        });
        total_unique_sequences += row_unique_sequences;
        no_cache_llm_calls += row_unique_sequences * 2;
        exact_cache_llm_calls += row_exact_cache_llm_calls;
        exact_cache_plus_nando_llm_calls += row_exact_cache_plus_nando_llm_calls;
        false_local_accepts += row_false_local_accepts;
        missed_expected_local += row_missed_expected_local;
    }

    let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(addr, "/metrics")?;
    let exact_cache_incremental_reduction_milli =
        reduction_milli(exact_cache_llm_calls, exact_cache_plus_nando_llm_calls);
    let pass = health.status == "ok"
        && !health.compiler_used
        && !health.eval_packs_loaded
        && !health.corpus_jsonl_loaded
        && profiles.profile_count == registry.profile_count()
        && total_unique_sequences > 0
        && exact_cache_incremental_reduction_milli >= 200
        && false_local_accepts == 0
        && metrics.runtime_bytes_estimate <= 3 * 1024 * 1024
        && metrics.p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileReplaySuiteReport {
        schema_version: "nando_role_binding_profile_replay_suite_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        binary_suite_report_path: binary_suite_path.display().to_string(),
        profile_count: profiles.profile_count,
        unique_sequences_replayed: total_unique_sequences,
        http_replay_batches: total_http_replay_batches,
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        p50_latency_ns: metrics.p50_latency_ns,
        p90_latency_ns: metrics.p90_latency_ns,
        p99_latency_ns: metrics.p99_latency_ns,
        rss_bytes: metrics.rss_bytes,
        runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        compiler_used: false,
        eval_packs_loaded_in_serving_worker: false,
        corpus_jsonl_loaded_in_serving_worker: false,
        eval_packs_used_by_replay_client: true,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        rows,
        claim_boundary: "HTTP replay-suite over the serving-only .nwrb role-binding profile runtime. .nwreb eval packs are read only by the replay client to generate requests; the serving worker loads no eval packs, corpora, compiler, training state, or Python demo code.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-replay-suite-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  binary_suite_report: {}", report.binary_suite_report_path);
    println!("  report: {}", report_path.display());
    println!("  profile_count: {}", report.profile_count);
    println!(
        "  unique_sequences_replayed: {}",
        report.unique_sequences_replayed
    );
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  exact_cache_incremental_reduction_milli: {}",
        report.exact_cache_incremental_reduction_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!(
        "  runtime_bytes_estimate: {}",
        report.runtime_bytes_estimate
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile replay suite failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_fallback_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_FALLBACK_SMOKE_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let bind_addr = reserve_local_bind_addr()?;
    let _server = RoleBindingProfileServeProcess::start(&config_path, &bind_addr, None)?;
    let addr = bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid fallback smoke bind addr {bind_addr}: {error}"))?;

    let health = send_json_get::<RoleBindingProfileHealthResponse>(addr, "/health")?;
    let mut local_request = registry
        .self_test_score_request()
        .ok_or_else(|| "failed to build local profile score request".to_owned())?;
    local_request.request_id = "fallback_smoke_local_accept".to_owned();
    local_request.expect_local_operator = Some(true);
    let local_response =
        send_json_post::<_, RoleBindingProfileScoreResponse>(addr, "/score", &local_request)?;

    let mut bad_route_request = local_request.clone();
    bad_route_request.request_id = "fallback_smoke_bad_route".to_owned();
    bad_route_request.profile_id = None;
    bad_route_request.route_key = Some("__missing_route__".to_owned());
    bad_route_request.exact_cache_key = Some("fallback_smoke_bad_route".to_owned());
    bad_route_request.expect_local_operator = Some(false);
    let bad_route_response =
        send_json_post::<_, RoleBindingProfileScoreResponse>(addr, "/score", &bad_route_request)?;

    let mut low_margin_request = local_request.clone();
    low_margin_request.request_id = "fallback_smoke_low_margin".to_owned();
    low_margin_request.exact_cache_key = Some("fallback_smoke_low_margin".to_owned());
    for active in &mut low_margin_request.active_fringe {
        active.strength = active.strength.signum();
    }
    for slot in &mut low_margin_request.slots {
        for impulse in &mut slot.positive_impulses {
            impulse.signed_strength = impulse.signed_strength.signum();
        }
        for impulse in &mut slot.negative_impulses {
            impulse.signed_strength = impulse.signed_strength.signum();
        }
    }
    low_margin_request.expect_local_operator = Some(true);
    let low_margin_response =
        send_json_post::<_, RoleBindingProfileScoreResponse>(addr, "/score", &low_margin_request)?;

    let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(addr, "/metrics")?;
    let local_accept_pass = local_response.accepted
        && !local_response.fallback
        && local_response.action == "local_operator"
        && !local_response.false_local_accept;
    let bad_route_pass = !bad_route_response.accepted
        && bad_route_response.fallback
        && bad_route_response.fallback_reason.as_deref() == Some("profile_not_found")
        && !bad_route_response.false_local_accept;
    let low_margin_pass = !low_margin_response.accepted
        && low_margin_response.fallback
        && low_margin_response.fallback_reason.as_deref() == Some("margin_below_threshold")
        && low_margin_response.strict_ordered_pass
        && low_margin_response.energy_margin < low_margin_response.threshold
        && !low_margin_response.false_local_accept;
    let pass = health.status == "ok"
        && !health.compiler_used
        && !health.eval_packs_loaded
        && !health.corpus_jsonl_loaded
        && local_accept_pass
        && bad_route_pass
        && low_margin_pass
        && metrics.local_operator_calls == 1
        && metrics.fallback_to_llm_calls == 2
        && metrics.false_local_accepts == 0
        && metrics.runtime_bytes_estimate <= 3 * 1024 * 1024
        && metrics.p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileFallbackSmokeReport {
        schema_version: "nando_role_binding_profile_fallback_smoke_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        profile_count: health.profile_count,
        local_accept_pass,
        bad_route_fallback_pass: bad_route_pass,
        low_margin_fallback_pass: low_margin_pass,
        local_action: local_response.action.clone(),
        bad_route_fallback_reason: bad_route_response.fallback_reason.clone(),
        low_margin_fallback_reason: low_margin_response.fallback_reason.clone(),
        local_energy_margin: local_response.energy_margin,
        low_margin_energy_margin: low_margin_response.energy_margin,
        low_margin_threshold: low_margin_response.threshold,
        local_operator_calls: metrics.local_operator_calls,
        fallback_to_llm_calls: metrics.fallback_to_llm_calls,
        false_local_accepts: metrics.false_local_accepts,
        p50_latency_ns: metrics.p50_latency_ns,
        p90_latency_ns: metrics.p90_latency_ns,
        p99_latency_ns: metrics.p99_latency_ns,
        rss_bytes: metrics.rss_bytes,
        runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        claim_boundary: "HTTP fallback smoke over the serving-only .nwrb role-binding profile runtime: verifies local accept, missing-route fallback, and low-margin fallback; not worker scaling or real Codex traffic.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-fallback-smoke-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  profile_count: {}", report.profile_count);
    println!("  local_accept_pass: {}", report.local_accept_pass);
    println!(
        "  bad_route_fallback_pass: {}",
        report.bad_route_fallback_pass
    );
    println!(
        "  low_margin_fallback_pass: {}",
        report.low_margin_fallback_pass
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    if pass {
        Ok(())
    } else {
        Err("role-binding profile fallback smoke failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_worker_scaling_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_WORKER_SCALING_REPORT));
    let worker_count = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker_count '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROFILE_WORKER_COUNT);
    if worker_count < 2 {
        return Err("worker_count must be at least 2 for product scaling proof".to_owned());
    }

    let base_config = read_json_file::<RoleBindingProfileRegistryConfig>(&config_path)?;
    validate_registry_config(&base_config)?;
    if worker_count > base_config.profiles.len() {
        return Err(format!(
            "worker_count {} exceeds profile_count {}",
            worker_count,
            base_config.profiles.len()
        ));
    }

    let shard_dir = report_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("profile-worker-shards-v1");
    let mut shard_profiles = vec![Vec::<RoleBindingProfileConfig>::new(); worker_count];
    for (index, profile) in base_config.profiles.iter().cloned().enumerate() {
        shard_profiles[index % worker_count].push(profile);
    }

    struct WorkerHarness {
        worker_id: usize,
        config_path: PathBuf,
        registry: RoleBindingProfileRuntimeRegistry,
        addr: SocketAddr,
        _server: RoleBindingProfileServeProcess,
    }

    let mut workers = Vec::new();
    for (worker_id, profiles) in shard_profiles.into_iter().enumerate() {
        if profiles.is_empty() {
            return Err(format!("worker {worker_id} has no profiles"));
        }
        let shard_config_path = shard_dir.join(format!("worker-{worker_id}.registry.json"));
        let shard_config = RoleBindingProfileRegistryConfig {
            schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
            claim_boundary: format!(
                "Serving-only .nwrb worker shard {worker_id}; no eval packs, corpus, compiler, training state, or Python demo code."
            ),
        };
        write_json_file(&shard_config_path, &shard_config)?;
        let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&shard_config_path)?;
        let bind_addr = reserve_local_bind_addr()?;
        let server = RoleBindingProfileServeProcess::start(&shard_config_path, &bind_addr, None)?;
        let addr = bind_addr
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid worker bind addr {bind_addr}: {error}"))?;
        workers.push(WorkerHarness {
            worker_id,
            config_path: shard_config_path,
            registry,
            addr,
            _server: server,
        });
    }

    let mut rows = Vec::new();
    let mut total_profile_count = 0usize;
    let mut total_local_operator_calls = 0usize;
    let mut total_fallback_to_llm_calls = 0usize;
    let mut total_false_local_accepts = 0usize;
    let mut total_wrong_worker_route_fallbacks = 0usize;
    let mut max_worker_runtime_bytes_estimate = 0usize;
    let mut max_worker_rss_bytes = 0usize;
    let mut max_worker_p99_latency_ns = 0u128;
    let mut all_workers_serving_only = true;
    let mut all_profile_score_pass = true;
    let mut all_wrong_worker_routes_fallback = true;

    for worker in &workers {
        let health = send_json_get::<RoleBindingProfileHealthResponse>(worker.addr, "/health")?;
        let profiles_response =
            send_json_get::<RoleBindingProfileProfilesResponse>(worker.addr, "/profiles")?;
        all_workers_serving_only &= !health.compiler_used
            && !health.eval_packs_loaded
            && !health.corpus_jsonl_loaded
            && !health.python_demo_used
            && !profiles_response.compiler_used
            && !profiles_response.eval_packs_loaded
            && !profiles_response.corpus_jsonl_loaded
            && !profiles_response.python_demo_used;

        let mut local_accepts = 0usize;
        let mut worker_false_local_accepts = 0usize;
        for profile in &worker.registry.profiles {
            let mut request = worker
                .registry
                .self_test_score_request_for_profile(&profile.config.profile_id)
                .ok_or_else(|| {
                    format!(
                        "failed to build worker {} score request for {}",
                        worker.worker_id, profile.config.profile_id
                    )
                })?;
            request.request_id = format!(
                "worker{}_local_accept_{}",
                worker.worker_id, profile.config.profile_id
            );
            request.expect_local_operator = Some(true);
            let response = send_json_post::<_, RoleBindingProfileScoreResponse>(
                worker.addr,
                "/score",
                &request,
            )?;
            let pass = response.accepted
                && !response.fallback
                && response.action == "local_operator"
                && !response.false_local_accept;
            all_profile_score_pass &= pass;
            local_accepts += usize::from(pass);
            worker_false_local_accepts += usize::from(response.false_local_accept);
        }

        let other_route = workers
            .iter()
            .find(|candidate| candidate.worker_id != worker.worker_id)
            .and_then(|candidate| candidate.registry.profiles.first())
            .and_then(|profile| profile.config.accepted_route_keys.first())
            .cloned()
            .ok_or_else(|| "failed to choose wrong-worker route".to_owned())?;
        let first_profile = worker
            .registry
            .profiles
            .first()
            .ok_or_else(|| format!("worker {} has no profiles", worker.worker_id))?;
        let mut wrong_worker_request = worker
            .registry
            .self_test_score_request_for_profile(&first_profile.config.profile_id)
            .ok_or_else(|| {
                format!(
                    "failed to build wrong-worker route request for worker {}",
                    worker.worker_id
                )
            })?;
        wrong_worker_request.request_id = format!("worker{}_wrong_route", worker.worker_id);
        wrong_worker_request.profile_id = None;
        wrong_worker_request.route_key = Some(other_route);
        wrong_worker_request.expect_local_operator = Some(false);
        let wrong_worker_response = send_json_post::<_, RoleBindingProfileScoreResponse>(
            worker.addr,
            "/score",
            &wrong_worker_request,
        )?;
        let wrong_worker_fallback = !wrong_worker_response.accepted
            && wrong_worker_response.fallback
            && wrong_worker_response.fallback_reason.as_deref() == Some("profile_not_found")
            && !wrong_worker_response.false_local_accept;
        all_wrong_worker_routes_fallback &= wrong_worker_fallback;
        worker_false_local_accepts += usize::from(wrong_worker_response.false_local_accept);

        let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(worker.addr, "/metrics")?;
        total_profile_count += health.profile_count;
        total_local_operator_calls += metrics.local_operator_calls;
        total_fallback_to_llm_calls += metrics.fallback_to_llm_calls;
        total_false_local_accepts += metrics.false_local_accepts;
        total_wrong_worker_route_fallbacks += usize::from(wrong_worker_fallback);
        max_worker_runtime_bytes_estimate =
            max_worker_runtime_bytes_estimate.max(metrics.runtime_bytes_estimate);
        max_worker_rss_bytes = max_worker_rss_bytes.max(metrics.rss_bytes);
        max_worker_p99_latency_ns = max_worker_p99_latency_ns.max(metrics.p99_latency_ns);

        rows.push(RoleBindingProfileWorkerScalingRow {
            worker_id: worker.worker_id,
            shard_config_path: worker.config_path.display().to_string(),
            bind_addr: worker.addr.to_string(),
            profile_count: health.profile_count,
            profile_ids: profiles_response
                .profiles
                .iter()
                .map(|profile| profile.profile_id.clone())
                .collect(),
            runtime_bytes_estimate: metrics.runtime_bytes_estimate,
            package_bytes: health.package_bytes,
            edge_count: health.edge_count,
            local_accepts,
            wrong_worker_route_fallbacks: usize::from(wrong_worker_fallback),
            false_local_accepts: worker_false_local_accepts,
            p50_latency_ns: metrics.p50_latency_ns,
            p90_latency_ns: metrics.p90_latency_ns,
            p99_latency_ns: metrics.p99_latency_ns,
            rss_bytes: metrics.rss_bytes,
            compiler_used: health.compiler_used,
            eval_packs_loaded: health.eval_packs_loaded,
            corpus_jsonl_loaded: health.corpus_jsonl_loaded,
            python_demo_used: health.python_demo_used,
        });
    }

    let pass = workers.len() == worker_count
        && total_profile_count == base_config.profiles.len()
        && total_local_operator_calls == base_config.profiles.len()
        && total_wrong_worker_route_fallbacks == worker_count
        && total_false_local_accepts == 0
        && all_workers_serving_only
        && all_profile_score_pass
        && all_wrong_worker_routes_fallback
        && max_worker_runtime_bytes_estimate <= 3 * 1024 * 1024
        && max_worker_p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileWorkerScalingReport {
        schema_version: "nando_role_binding_profile_worker_scaling_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_WORKER_SCALING_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        worker_count,
        total_profile_count,
        total_local_operator_calls,
        total_fallback_to_llm_calls,
        wrong_worker_route_fallbacks: total_wrong_worker_route_fallbacks,
        false_local_accepts: total_false_local_accepts,
        max_worker_runtime_bytes_estimate,
        max_worker_rss_bytes,
        max_worker_p99_latency_ns,
        all_workers_serving_only,
        all_profile_score_pass,
        all_wrong_worker_routes_fallback,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        rows,
        claim_boundary: "Product acceptance check for serving-only .nwrb profile worker sharding: multiple workers load separate profile shards, score local accepts, and reject wrong-worker routes. This is not real Codex traffic or external load-balancer proof.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-worker-scaling-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  worker_count: {}", report.worker_count);
    println!("  total_profile_count: {}", report.total_profile_count);
    println!(
        "  total_local_operator_calls: {}",
        report.total_local_operator_calls
    );
    println!(
        "  wrong_worker_route_fallbacks: {}",
        report.wrong_worker_route_fallbacks
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  max_worker_runtime_bytes_estimate: {}",
        report.max_worker_runtime_bytes_estimate
    );
    println!(
        "  max_worker_p99_latency_ns: {}",
        report.max_worker_p99_latency_ns
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile worker scaling failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_worker_replay_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_WORKER_REPLAY_REPORT));
    let worker_count = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker_count '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROFILE_WORKER_COUNT);
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE);
    let batch_unique_sequences = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid batch_unique_sequences '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES);
    if worker_count < 2 {
        return Err("worker_count must be at least 2 for sharded replay proof".to_owned());
    }
    if batch_unique_sequences == 0 {
        return Err("batch_unique_sequences must be positive".to_owned());
    }

    let base_config = read_json_file::<RoleBindingProfileRegistryConfig>(&config_path)?;
    validate_registry_config(&base_config)?;
    if worker_count > base_config.profiles.len() {
        return Err(format!(
            "worker_count {} exceeds profile_count {}",
            worker_count,
            base_config.profiles.len()
        ));
    }
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }

    let shard_dir = report_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("profile-worker-replay-shards-v1");
    let mut shard_profiles = vec![Vec::<RoleBindingProfileConfig>::new(); worker_count];
    for (index, profile) in base_config.profiles.iter().cloned().enumerate() {
        shard_profiles[index % worker_count].push(profile);
    }

    struct WorkerReplayHarness {
        worker_id: usize,
        config_path: PathBuf,
        registry: RoleBindingProfileRuntimeRegistry,
        addr: SocketAddr,
        _server: RoleBindingProfileServeProcess,
    }

    let mut workers = Vec::new();
    let mut profile_to_worker = HashMap::new();
    for (worker_id, profiles) in shard_profiles.into_iter().enumerate() {
        if profiles.is_empty() {
            return Err(format!("worker {worker_id} has no profiles"));
        }
        let shard_config_path = shard_dir.join(format!("worker-{worker_id}.registry.json"));
        let shard_config = RoleBindingProfileRegistryConfig {
            schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
            claim_boundary: format!(
                "Serving-only .nwrb replay worker shard {worker_id}; no eval packs, corpus, compiler, training state, or Python demo code."
            ),
        };
        write_json_file(&shard_config_path, &shard_config)?;
        let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&shard_config_path)?;
        for profile in &registry.profiles {
            if profile_to_worker
                .insert(profile.config.profile_id.clone(), worker_id)
                .is_some()
            {
                return Err(format!(
                    "duplicate profile_id {}",
                    profile.config.profile_id
                ));
            }
        }
        let bind_addr = reserve_local_bind_addr()?;
        let server = RoleBindingProfileServeProcess::start(&shard_config_path, &bind_addr, None)?;
        let addr = bind_addr
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid worker replay bind addr {bind_addr}: {error}"))?;
        workers.push(WorkerReplayHarness {
            worker_id,
            config_path: shard_config_path,
            registry,
            addr,
            _server: server,
        });
    }

    let mut worker_rows = workers
        .iter()
        .map(|worker| RoleBindingProfileWorkerReplayRow {
            worker_id: worker.worker_id,
            shard_config_path: worker.config_path.display().to_string(),
            bind_addr: worker.addr.to_string(),
            profile_count: worker.registry.profile_count(),
            profile_ids: worker
                .registry
                .profiles
                .iter()
                .map(|profile| profile.config.profile_id.clone())
                .collect(),
            unique_sequences_replayed: 0,
            http_replay_batches: 0,
            no_cache_llm_calls: 0,
            exact_cache_llm_calls: 0,
            exact_cache_plus_nando_llm_calls: 0,
            exact_cache_incremental_reduction_milli: 0,
            local_operator_calls: 0,
            fallback_to_llm_calls: 0,
            false_local_accepts: 0,
            missed_expected_local: 0,
            p50_latency_ns: 0,
            p90_latency_ns: 0,
            p99_latency_ns: 0,
            rss_bytes: 0,
            runtime_bytes_estimate: 0,
        })
        .collect::<Vec<_>>();

    let mut total_unique_sequences = 0usize;
    let mut total_http_replay_batches = 0usize;
    let mut no_cache_llm_calls = 0usize;
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;

    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        let worker_id = *profile_to_worker
            .get(&profile_id)
            .ok_or_else(|| format!("profile {profile_id} missing from worker shards"))?;
        let worker = workers
            .iter()
            .find(|worker| worker.worker_id == worker_id)
            .ok_or_else(|| format!("worker {worker_id} missing"))?;
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        if eval_pack.package_fingerprint64 != Some(suite_row.package_fingerprint64) {
            return Err(format!(
                "binary eval-pack fingerprint mismatch for {} seed{}: pack={:?} suite={}",
                suite_row.label,
                suite_row.seed,
                eval_pack.package_fingerprint64,
                suite_row.package_fingerprint64
            ));
        }
        if eval_pack.generation_method.trim().is_empty() {
            return Err(format!(
                "binary eval-pack generation_method is empty for {}",
                eval_pack_path.display()
            ));
        }
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        let mut profile_unique_sequences = 0usize;
        let mut batch = Vec::new();
        let mut batch_index = 0usize;
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            profile_unique_sequences += 1;
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                0,
            ));
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                1,
            ));
            if profile_unique_sequences.is_multiple_of(batch_unique_sequences) {
                let response =
                    send_replay_batch(worker.addr, &profile_id, batch_index, &mut batch)?;
                total_http_replay_batches += 1;
                batch_index += 1;
                let row = &mut worker_rows[worker_id];
                row.http_replay_batches += 1;
                row.exact_cache_llm_calls += response.exact_cache_llm_calls;
                row.exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
                row.local_operator_calls += response.local_operator_calls;
                row.fallback_to_llm_calls += response.fallback_to_llm_calls;
                row.false_local_accepts += response.false_local_accepts;
                row.missed_expected_local += response.missed_expected_local;
            }
        }
        if !batch.is_empty() {
            let response = send_replay_batch(worker.addr, &profile_id, batch_index, &mut batch)?;
            total_http_replay_batches += 1;
            let row = &mut worker_rows[worker_id];
            row.http_replay_batches += 1;
            row.exact_cache_llm_calls += response.exact_cache_llm_calls;
            row.exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
            row.local_operator_calls += response.local_operator_calls;
            row.fallback_to_llm_calls += response.fallback_to_llm_calls;
            row.false_local_accepts += response.false_local_accepts;
            row.missed_expected_local += response.missed_expected_local;
        }
        let row = &mut worker_rows[worker_id];
        row.unique_sequences_replayed += profile_unique_sequences;
        row.no_cache_llm_calls += profile_unique_sequences * 2;
        total_unique_sequences += profile_unique_sequences;
        no_cache_llm_calls += profile_unique_sequences * 2;
    }

    let mut all_workers_serving_only = true;
    let mut total_runtime_bytes_estimate = 0usize;
    let mut total_rss_bytes = 0usize;
    let mut max_worker_runtime_bytes_estimate = 0usize;
    let mut max_worker_rss_bytes = 0usize;
    let mut max_worker_p99_latency_ns = 0u128;
    for worker in &workers {
        let health = send_json_get::<RoleBindingProfileHealthResponse>(worker.addr, "/health")?;
        let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(worker.addr, "/metrics")?;
        all_workers_serving_only &= !health.compiler_used
            && !health.eval_packs_loaded
            && !health.corpus_jsonl_loaded
            && !health.python_demo_used
            && !metrics.compiler_used
            && !metrics.eval_packs_loaded
            && !metrics.corpus_jsonl_loaded
            && !metrics.python_demo_used;
        let row = &mut worker_rows[worker.worker_id];
        row.p50_latency_ns = metrics.p50_latency_ns;
        row.p90_latency_ns = metrics.p90_latency_ns;
        row.p99_latency_ns = metrics.p99_latency_ns;
        row.rss_bytes = metrics.rss_bytes;
        row.runtime_bytes_estimate = metrics.runtime_bytes_estimate;
        row.exact_cache_incremental_reduction_milli = reduction_milli(
            row.exact_cache_llm_calls,
            row.exact_cache_plus_nando_llm_calls,
        );
        total_runtime_bytes_estimate += metrics.runtime_bytes_estimate;
        total_rss_bytes += metrics.rss_bytes;
        max_worker_runtime_bytes_estimate =
            max_worker_runtime_bytes_estimate.max(metrics.runtime_bytes_estimate);
        max_worker_rss_bytes = max_worker_rss_bytes.max(metrics.rss_bytes);
        max_worker_p99_latency_ns = max_worker_p99_latency_ns.max(metrics.p99_latency_ns);
        exact_cache_llm_calls += row.exact_cache_llm_calls;
        exact_cache_plus_nando_llm_calls += row.exact_cache_plus_nando_llm_calls;
        local_operator_calls += row.local_operator_calls;
        fallback_to_llm_calls += row.fallback_to_llm_calls;
        false_local_accepts += row.false_local_accepts;
        missed_expected_local += row.missed_expected_local;
    }
    let exact_cache_incremental_reduction_milli =
        reduction_milli(exact_cache_llm_calls, exact_cache_plus_nando_llm_calls);
    let pass = workers.len() == worker_count
        && total_unique_sequences > 0
        && exact_cache_incremental_reduction_milli >= 200
        && false_local_accepts == 0
        && missed_expected_local == 0
        && all_workers_serving_only
        && max_worker_runtime_bytes_estimate <= 3 * 1024 * 1024
        && max_worker_p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileWorkerReplayReport {
        schema_version: "nando_role_binding_profile_worker_replay_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        binary_suite_report_path: binary_suite_path.display().to_string(),
        worker_count,
        total_profile_count: base_config.profiles.len(),
        unique_sequences_replayed: total_unique_sequences,
        http_replay_batches: total_http_replay_batches,
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        total_runtime_bytes_estimate,
        max_worker_runtime_bytes_estimate,
        total_rss_bytes,
        max_worker_rss_bytes,
        max_worker_p99_latency_ns,
        all_workers_serving_only,
        eval_packs_used_by_replay_client: true,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        rows: worker_rows,
        claim_boundary: "Sharded HTTP replay over multiple serving-only .nwrb profile workers. .nwreb eval packs are read only by the replay client to generate requests; workers load no eval packs, corpora, compiler, training state, or Python demo code. This is not real Codex traffic or external load-balancer proof.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-worker-replay-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  binary_suite_report: {}", report.binary_suite_report_path);
    println!("  report: {}", report_path.display());
    println!("  worker_count: {}", report.worker_count);
    println!(
        "  unique_sequences_replayed: {}",
        report.unique_sequences_replayed
    );
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  exact_cache_incremental_reduction_milli: {}",
        report.exact_cache_incremental_reduction_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  max_worker_runtime_bytes_estimate: {}",
        report.max_worker_runtime_bytes_estimate
    );
    println!(
        "  max_worker_p99_latency_ns: {}",
        report.max_worker_p99_latency_ns
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile worker replay failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_lb_replay_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_LB_REPLAY_REPORT));
    let worker_count = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker_count '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROFILE_WORKER_COUNT);
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE);
    let batch_unique_sequences = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid batch_unique_sequences '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES);
    if worker_count < 2 {
        return Err("worker_count must be at least 2 for load-balancer replay proof".to_owned());
    }
    if batch_unique_sequences == 0 {
        return Err("batch_unique_sequences must be positive".to_owned());
    }

    let base_config = read_json_file::<RoleBindingProfileRegistryConfig>(&config_path)?;
    validate_registry_config(&base_config)?;
    if worker_count > base_config.profiles.len() {
        return Err(format!(
            "worker_count {} exceeds profile_count {}",
            worker_count,
            base_config.profiles.len()
        ));
    }
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }
    let parity_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path_with_reference(&config_path)?;
    let (packed_score_parity_checks, packed_score_parity_mismatches) =
        verify_role_binding_profile_packed_score_parity(
            &parity_registry,
            &binary_suite,
            max_unique_sequences_per_profile,
        )?;

    let root_dir = report_path.parent().unwrap_or_else(|| Path::new("."));
    let shard_dir = root_dir.join("profile-lb-replay-shards-v1");
    let lb_config_path = root_dir.join("profile-lb-replay-v1.lb.json");
    let mut shard_profiles = vec![Vec::<RoleBindingProfileConfig>::new(); worker_count];
    for (index, profile) in base_config.profiles.iter().cloned().enumerate() {
        shard_profiles[index % worker_count].push(profile);
    }

    struct LbReplayWorkerHarness {
        worker_id: usize,
        config_path: PathBuf,
        registry: RoleBindingProfileRuntimeRegistry,
        addr: SocketAddr,
        _server: RoleBindingProfileServeProcess,
    }

    let mut workers = Vec::new();
    for (worker_id, profiles) in shard_profiles.into_iter().enumerate() {
        if profiles.is_empty() {
            return Err(format!("worker {worker_id} has no profiles"));
        }
        let shard_config_path = shard_dir.join(format!("worker-{worker_id}.registry.json"));
        let shard_config = RoleBindingProfileRegistryConfig {
            schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
            claim_boundary: format!(
                "Serving-only .nwrb load-balancer replay worker shard {worker_id}; no eval packs, corpus, compiler, training state, or Python demo code."
            ),
        };
        write_json_file(&shard_config_path, &shard_config)?;
        let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&shard_config_path)?;
        let bind_addr = reserve_local_bind_addr()?;
        let server = RoleBindingProfileServeProcess::start(&shard_config_path, &bind_addr, None)?;
        let addr = bind_addr
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid lb replay worker bind addr {bind_addr}: {error}"))?;
        workers.push(LbReplayWorkerHarness {
            worker_id,
            config_path: shard_config_path,
            registry,
            addr,
            _server: server,
        });
    }

    let lb_config = RoleBindingProfileLoadBalancerConfig {
        schema_version: "nando_role_binding_profile_lb_v1".to_owned(),
        upstreams: workers
            .iter()
            .map(|worker| RoleBindingProfileLoadBalancerUpstreamConfig {
                worker_id: worker.worker_id,
                upstream_addr: worker.addr.to_string(),
                shard_config_path: worker.config_path.clone(),
                profile_ids: worker
                    .registry
                    .profiles
                    .iter()
                    .map(|profile| profile.config.profile_id.clone())
                    .collect(),
                accepted_route_keys: worker
                    .registry
                    .profiles
                    .iter()
                    .flat_map(|profile| profile.config.accepted_route_keys.clone())
                    .collect(),
            })
            .collect(),
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        claim_boundary: "Local external load-balancer config: routes requests to serving-only .nwrb profile workers; the proxy itself loads no .nwrb/.nwreb packages, corpora, compiler, training state, or Python demo code.".to_owned(),
    };
    validate_load_balancer_config(&lb_config)?;
    write_json_file(&lb_config_path, &lb_config)?;

    let lb_bind_addr = reserve_local_bind_addr()?;
    let load_balancer_process =
        RoleBindingProfileLoadBalancerProcess::start(&lb_config_path, &lb_bind_addr, None)?;
    let lb_addr = lb_bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid lb replay bind addr {lb_bind_addr}: {error}"))?;
    let lb_health =
        send_json_get::<RoleBindingProfileLoadBalancerHealthResponse>(lb_addr, "/health")?;
    let lb_profiles =
        send_json_get::<RoleBindingProfileLoadBalancerProfilesResponse>(lb_addr, "/profiles")?;

    let mut total_unique_sequences = 0usize;
    let mut total_http_replay_batches = 0usize;
    let mut no_cache_llm_calls = 0usize;
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    let mut replay_client_wall_latencies_ns: Vec<u128> = Vec::new();

    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        if !lb_config
            .upstreams
            .iter()
            .any(|upstream| upstream.profile_ids.iter().any(|id| id == &profile_id))
        {
            return Err(format!(
                "profile {profile_id} from binary suite is missing in lb config"
            ));
        }
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        if eval_pack.package_fingerprint64 != Some(suite_row.package_fingerprint64) {
            return Err(format!(
                "binary eval-pack fingerprint mismatch for {} seed{}: pack={:?} suite={}",
                suite_row.label,
                suite_row.seed,
                eval_pack.package_fingerprint64,
                suite_row.package_fingerprint64
            ));
        }
        if eval_pack.generation_method.trim().is_empty() {
            return Err(format!(
                "binary eval-pack generation_method is empty for {}",
                eval_pack_path.display()
            ));
        }
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        let mut profile_unique_sequences = 0usize;
        let mut batch = Vec::new();
        let mut batch_index = 0usize;
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            profile_unique_sequences += 1;
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                0,
            ));
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                1,
            ));
            if profile_unique_sequences.is_multiple_of(batch_unique_sequences) {
                let (response, wall_latency_ns) =
                    send_replay_batch_timed(lb_addr, &profile_id, batch_index, &mut batch)?;
                replay_client_wall_latencies_ns.push(wall_latency_ns);
                total_http_replay_batches += 1;
                batch_index += 1;
                exact_cache_llm_calls += response.exact_cache_llm_calls;
                exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
                local_operator_calls += response.local_operator_calls;
                fallback_to_llm_calls += response.fallback_to_llm_calls;
                false_local_accepts += response.false_local_accepts;
                missed_expected_local += response.missed_expected_local;
            }
        }
        if !batch.is_empty() {
            let (response, wall_latency_ns) =
                send_replay_batch_timed(lb_addr, &profile_id, batch_index, &mut batch)?;
            replay_client_wall_latencies_ns.push(wall_latency_ns);
            total_http_replay_batches += 1;
            exact_cache_llm_calls += response.exact_cache_llm_calls;
            exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
            local_operator_calls += response.local_operator_calls;
            fallback_to_llm_calls += response.fallback_to_llm_calls;
            false_local_accepts += response.false_local_accepts;
            missed_expected_local += response.missed_expected_local;
        }
        total_unique_sequences += profile_unique_sequences;
        no_cache_llm_calls += profile_unique_sequences * 2;
    }

    let lb_metrics = send_json_get::<RoleBindingProfileMetricsResponse>(lb_addr, "/metrics")?;
    drop(load_balancer_process);
    let mut all_workers_serving_only = true;
    let mut worker_rows = Vec::new();
    let mut total_worker_runtime_bytes_estimate = 0usize;
    let mut max_worker_runtime_bytes_estimate = 0usize;
    let mut total_worker_rss_bytes = 0usize;
    let mut max_worker_rss_bytes = 0usize;
    let mut max_worker_p99_latency_ns = 0u128;
    let mut max_worker_core_score_p50_latency_ns = 0u128;
    let mut max_worker_core_score_p90_latency_ns = 0u128;
    let mut max_worker_core_score_p99_latency_ns = 0u128;
    let mut max_worker_score_p50_latency_ns = 0u128;
    let mut max_worker_score_p90_latency_ns = 0u128;
    let mut max_worker_score_p99_latency_ns = 0u128;
    for worker in &workers {
        let health = send_json_get::<RoleBindingProfileHealthResponse>(worker.addr, "/health")?;
        let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(worker.addr, "/metrics")?;
        all_workers_serving_only &= !health.compiler_used
            && !health.eval_packs_loaded
            && !health.corpus_jsonl_loaded
            && !health.python_demo_used
            && !metrics.compiler_used
            && !metrics.eval_packs_loaded
            && !metrics.corpus_jsonl_loaded
            && !metrics.python_demo_used;
        total_worker_runtime_bytes_estimate += metrics.runtime_bytes_estimate;
        max_worker_runtime_bytes_estimate =
            max_worker_runtime_bytes_estimate.max(metrics.runtime_bytes_estimate);
        total_worker_rss_bytes += metrics.rss_bytes;
        max_worker_rss_bytes = max_worker_rss_bytes.max(metrics.rss_bytes);
        max_worker_p99_latency_ns = max_worker_p99_latency_ns.max(metrics.p99_latency_ns);
        max_worker_core_score_p50_latency_ns =
            max_worker_core_score_p50_latency_ns.max(metrics.core_score_p50_latency_ns);
        max_worker_core_score_p90_latency_ns =
            max_worker_core_score_p90_latency_ns.max(metrics.core_score_p90_latency_ns);
        max_worker_core_score_p99_latency_ns =
            max_worker_core_score_p99_latency_ns.max(metrics.core_score_p99_latency_ns);
        max_worker_score_p50_latency_ns =
            max_worker_score_p50_latency_ns.max(metrics.worker_score_p50_latency_ns);
        max_worker_score_p90_latency_ns =
            max_worker_score_p90_latency_ns.max(metrics.worker_score_p90_latency_ns);
        max_worker_score_p99_latency_ns =
            max_worker_score_p99_latency_ns.max(metrics.worker_score_p99_latency_ns);
        worker_rows.push(RoleBindingProfileLoadBalancerReplayWorkerRow {
            worker_id: worker.worker_id,
            shard_config_path: worker.config_path.display().to_string(),
            bind_addr: worker.addr.to_string(),
            profile_count: health.profile_count,
            profile_ids: worker
                .registry
                .profiles
                .iter()
                .map(|profile| profile.config.profile_id.clone())
                .collect(),
            local_operator_calls: metrics.local_operator_calls,
            fallback_to_llm_calls: metrics.fallback_to_llm_calls,
            false_local_accepts: metrics.false_local_accepts,
            missed_expected_local: metrics.missed_expected_local,
            p50_latency_ns: metrics.p50_latency_ns,
            p90_latency_ns: metrics.p90_latency_ns,
            p99_latency_ns: metrics.p99_latency_ns,
            core_score_p50_latency_ns: metrics.core_score_p50_latency_ns,
            core_score_p90_latency_ns: metrics.core_score_p90_latency_ns,
            core_score_p99_latency_ns: metrics.core_score_p99_latency_ns,
            worker_score_p50_latency_ns: metrics.worker_score_p50_latency_ns,
            worker_score_p90_latency_ns: metrics.worker_score_p90_latency_ns,
            worker_score_p99_latency_ns: metrics.worker_score_p99_latency_ns,
            lb_upstream_roundtrip_p50_latency_ns: metrics.lb_upstream_roundtrip_p50_latency_ns,
            lb_upstream_roundtrip_p90_latency_ns: metrics.lb_upstream_roundtrip_p90_latency_ns,
            lb_upstream_roundtrip_p99_latency_ns: metrics.lb_upstream_roundtrip_p99_latency_ns,
            rss_bytes: metrics.rss_bytes,
            runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        });
    }

    let load_balancer_serving_only = !lb_health.compiler_used
        && !lb_health.eval_packs_loaded
        && !lb_health.corpus_jsonl_loaded
        && !lb_health.python_demo_used
        && !lb_metrics.compiler_used
        && !lb_metrics.eval_packs_loaded
        && !lb_metrics.corpus_jsonl_loaded
        && !lb_metrics.python_demo_used;
    let exact_cache_incremental_reduction_milli =
        reduction_milli(exact_cache_llm_calls, exact_cache_plus_nando_llm_calls);
    let estimated_lb_overhead_p99_ns = lb_metrics
        .p99_latency_ns
        .saturating_sub(max_worker_score_p99_latency_ns);
    let pass = lb_health.status == "ok"
        && lb_profiles.profile_count == base_config.profiles.len()
        && workers.len() == worker_count
        && total_unique_sequences > 0
        && exact_cache_incremental_reduction_milli >= 200
        && false_local_accepts == 0
        && missed_expected_local == 0
        && packed_score_parity_mismatches == 0
        && all_workers_serving_only
        && load_balancer_serving_only
        && max_worker_runtime_bytes_estimate <= 3 * 1024 * 1024
        && lb_metrics.p99_latency_ns <= 3_000_000
        && max_worker_p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileLoadBalancerReplayReport {
        schema_version: "nando_role_binding_profile_lb_replay_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_LB_REPLAY_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        binary_suite_report_path: binary_suite_path.display().to_string(),
        lb_config_path: lb_config_path.display().to_string(),
        lb_bind_addr: lb_addr.to_string(),
        worker_count,
        total_profile_count: base_config.profiles.len(),
        unique_sequences_replayed: total_unique_sequences,
        http_replay_batches: total_http_replay_batches,
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        load_balancer_p50_latency_ns: lb_metrics.p50_latency_ns,
        load_balancer_p90_latency_ns: lb_metrics.p90_latency_ns,
        load_balancer_p99_latency_ns: lb_metrics.p99_latency_ns,
        core_score_p50_latency_ns: max_worker_core_score_p50_latency_ns,
        core_score_p90_latency_ns: max_worker_core_score_p90_latency_ns,
        core_score_p99_latency_ns: max_worker_core_score_p99_latency_ns,
        worker_score_p50_latency_ns: max_worker_score_p50_latency_ns,
        worker_score_p90_latency_ns: max_worker_score_p90_latency_ns,
        worker_score_p99_latency_ns: max_worker_score_p99_latency_ns,
        lb_upstream_roundtrip_p50_latency_ns: lb_metrics.lb_upstream_roundtrip_p50_latency_ns,
        lb_upstream_roundtrip_p90_latency_ns: lb_metrics.lb_upstream_roundtrip_p90_latency_ns,
        lb_upstream_roundtrip_p99_latency_ns: lb_metrics.lb_upstream_roundtrip_p99_latency_ns,
        replay_client_wall_p50_latency_ns: percentile(&replay_client_wall_latencies_ns, 50),
        replay_client_wall_p90_latency_ns: percentile(&replay_client_wall_latencies_ns, 90),
        replay_client_wall_p99_latency_ns: percentile(&replay_client_wall_latencies_ns, 99),
        estimated_lb_overhead_p99_ns,
        packed_score_parity_checks,
        packed_score_parity_mismatches,
        load_balancer_rss_bytes: lb_metrics.rss_bytes,
        total_worker_runtime_bytes_estimate,
        max_worker_runtime_bytes_estimate,
        total_worker_rss_bytes,
        max_worker_rss_bytes,
        max_worker_p99_latency_ns,
        all_workers_serving_only,
        load_balancer_serving_only,
        eval_packs_used_by_replay_client: true,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        workers: worker_rows,
        claim_boundary: "Local external load-balancer replay over multiple serving-only .nwrb profile workers. .nwreb eval packs are read only by the replay client to generate requests; workers load no eval packs, corpora, compiler, training state, or Python demo code; the load-balancer loads only route-to-upstream metadata. This is not real Codex/API traffic, not concurrent throughput proof, and not cheap-VPS deployment.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-lb-replay-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  binary_suite_report: {}", report.binary_suite_report_path);
    println!("  lb_config: {}", report.lb_config_path);
    println!("  report: {}", report_path.display());
    println!("  worker_count: {}", report.worker_count);
    println!(
        "  unique_sequences_replayed: {}",
        report.unique_sequences_replayed
    );
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  exact_cache_incremental_reduction_milli: {}",
        report.exact_cache_incremental_reduction_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  load_balancer_p99_latency_ns: {}",
        report.load_balancer_p99_latency_ns
    );
    println!(
        "  core_score_p99_latency_ns: {}",
        report.core_score_p99_latency_ns
    );
    println!(
        "  worker_score_p99_latency_ns: {}",
        report.worker_score_p99_latency_ns
    );
    println!(
        "  lb_upstream_roundtrip_p99_latency_ns: {}",
        report.lb_upstream_roundtrip_p99_latency_ns
    );
    println!(
        "  replay_client_wall_p99_latency_ns: {}",
        report.replay_client_wall_p99_latency_ns
    );
    println!(
        "  packed_score_parity_checks: {}",
        report.packed_score_parity_checks
    );
    println!(
        "  packed_score_parity_mismatches: {}",
        report.packed_score_parity_mismatches
    );
    println!(
        "  max_worker_runtime_bytes_estimate: {}",
        report.max_worker_runtime_bytes_estimate
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile load-balancer replay failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_lb_throughput_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_LB_THROUGHPUT_REPORT));
    let worker_count = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker_count '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROFILE_WORKER_COUNT);
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE);
    let client_threads = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid client_threads '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_THROUGHPUT_CLIENT_THREADS);
    let sequence_repetitions = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid sequence_repetitions '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_THROUGHPUT_SEQUENCE_REPETITIONS);
    if worker_count < 2 {
        return Err("worker_count must be at least 2 for throughput proof".to_owned());
    }
    if client_threads == 0 {
        return Err("client_threads must be positive".to_owned());
    }
    if sequence_repetitions == 0 {
        return Err("sequence_repetitions must be positive".to_owned());
    }

    let base_config = read_json_file::<RoleBindingProfileRegistryConfig>(&config_path)?;
    validate_registry_config(&base_config)?;
    if worker_count > base_config.profiles.len() {
        return Err(format!(
            "worker_count {} exceeds profile_count {}",
            worker_count,
            base_config.profiles.len()
        ));
    }
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }
    let parity_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path_with_reference(&config_path)?;
    let (packed_score_parity_checks, packed_score_parity_mismatches) =
        verify_role_binding_profile_packed_score_parity(
            &parity_registry,
            &binary_suite,
            max_unique_sequences_per_profile,
        )?;

    let root_dir = report_path.parent().unwrap_or_else(|| Path::new("."));
    let shard_dir = root_dir.join("profile-lb-throughput-shards-v1");
    let lb_config_path = root_dir.join("profile-lb-throughput-v1.lb.json");
    let mut shard_profiles = vec![Vec::<RoleBindingProfileConfig>::new(); worker_count];
    for (index, profile) in base_config.profiles.iter().cloned().enumerate() {
        shard_profiles[index % worker_count].push(profile);
    }

    struct LbThroughputWorkerHarness {
        worker_id: usize,
        config_path: PathBuf,
        registry: RoleBindingProfileRuntimeRegistry,
        addr: SocketAddr,
        _server: RoleBindingProfileServeProcess,
    }

    let mut workers = Vec::new();
    for (worker_id, profiles) in shard_profiles.into_iter().enumerate() {
        if profiles.is_empty() {
            return Err(format!("worker {worker_id} has no profiles"));
        }
        let shard_config_path = shard_dir.join(format!("worker-{worker_id}.registry.json"));
        let shard_config = RoleBindingProfileRegistryConfig {
            schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
            claim_boundary: format!(
                "Serving-only .nwrb throughput worker shard {worker_id}; no eval packs, corpus, compiler, training state, or Python demo code."
            ),
        };
        write_json_file(&shard_config_path, &shard_config)?;
        let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&shard_config_path)?;
        let bind_addr = reserve_local_bind_addr()?;
        let server = RoleBindingProfileServeProcess::start(&shard_config_path, &bind_addr, None)?;
        let addr = bind_addr.parse::<SocketAddr>().map_err(|error| {
            format!("invalid lb throughput worker bind addr {bind_addr}: {error}")
        })?;
        workers.push(LbThroughputWorkerHarness {
            worker_id,
            config_path: shard_config_path,
            registry,
            addr,
            _server: server,
        });
    }

    let lb_config = RoleBindingProfileLoadBalancerConfig {
        schema_version: "nando_role_binding_profile_lb_v1".to_owned(),
        upstreams: workers
            .iter()
            .map(|worker| RoleBindingProfileLoadBalancerUpstreamConfig {
                worker_id: worker.worker_id,
                upstream_addr: worker.addr.to_string(),
                shard_config_path: worker.config_path.clone(),
                profile_ids: worker
                    .registry
                    .profiles
                    .iter()
                    .map(|profile| profile.config.profile_id.clone())
                    .collect(),
                accepted_route_keys: worker
                    .registry
                    .profiles
                    .iter()
                    .flat_map(|profile| profile.config.accepted_route_keys.clone())
                    .collect(),
            })
            .collect(),
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        claim_boundary: "Local external load-balancer config for bounded concurrent throughput proof; routes requests to serving-only .nwrb profile workers.".to_owned(),
    };
    validate_load_balancer_config(&lb_config)?;
    write_json_file(&lb_config_path, &lb_config)?;

    let request_set = build_role_binding_profile_throughput_requests(
        &binary_suite,
        &lb_config,
        max_unique_sequences_per_profile,
    )?;
    if request_set.requests.is_empty() {
        return Err("throughput request set is empty".to_owned());
    }
    let total_score_requests = request_set
        .requests
        .len()
        .checked_mul(sequence_repetitions)
        .ok_or_else(|| "throughput request count overflow".to_owned())?;

    let lb_bind_addr = reserve_local_bind_addr()?;
    let load_balancer_process =
        RoleBindingProfileLoadBalancerProcess::start(&lb_config_path, &lb_bind_addr, None)?;
    let lb_addr = lb_bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid lb throughput bind addr {lb_bind_addr}: {error}"))?;
    let lb_health =
        send_json_get::<RoleBindingProfileLoadBalancerHealthResponse>(lb_addr, "/health")?;
    let lb_profiles =
        send_json_get::<RoleBindingProfileLoadBalancerProfilesResponse>(lb_addr, "/profiles")?;

    println!("role-binding-profile-lb-throughput-v1: starting bounded pressure");
    println!("  worker_count: {worker_count}");
    println!("  client_threads: {client_threads}");
    println!(
        "  unique_sequences_replayed: {}",
        request_set.unique_sequences
    );
    println!("  score_requests: {total_score_requests}");

    let requests = Arc::new(request_set.requests);
    let wall_start = Instant::now();
    let mut handles = Vec::new();
    for client_id in 0..client_threads {
        let requests = Arc::clone(&requests);
        handles.push(thread::spawn(
            move || -> RoleBindingProfileThroughputClientRow {
                let mut row = RoleBindingProfileThroughputClientRow {
                    client_id,
                    score_requests: 0,
                    local_operator_calls: 0,
                    fallback_to_llm_calls: 0,
                    false_local_accepts: 0,
                    missed_expected_local: 0,
                    unexpected_local_accepts: 0,
                    errors: 0,
                    first_error: None,
                    p50_latency_ns: 0,
                    p90_latency_ns: 0,
                    p99_latency_ns: 0,
                };
                let mut latencies = Vec::new();
                for ordinal in (client_id..total_score_requests).step_by(client_threads) {
                    let request = &requests[ordinal % requests.len()];
                    let start = Instant::now();
                    match send_json_post::<_, RoleBindingProfileScoreResponse>(
                        lb_addr, "/score", request,
                    ) {
                        Ok(response) => {
                            latencies.push(start.elapsed().as_nanos());
                            row.score_requests += 1;
                            row.local_operator_calls += usize::from(response.accepted);
                            row.fallback_to_llm_calls += usize::from(response.fallback);
                            row.false_local_accepts += usize::from(response.false_local_accept);
                            row.missed_expected_local += usize::from(
                                request.expect_local_operator == Some(true) && !response.accepted,
                            );
                            row.unexpected_local_accepts += usize::from(
                                request.expect_local_operator == Some(false) && response.accepted,
                            );
                        }
                        Err(error) => {
                            latencies.push(start.elapsed().as_nanos());
                            row.errors += 1;
                            if row.first_error.is_none() {
                                row.first_error = Some(error);
                            }
                        }
                    }
                }
                row.p50_latency_ns = percentile(&latencies, 50);
                row.p90_latency_ns = percentile(&latencies, 90);
                row.p99_latency_ns = percentile(&latencies, 99);
                println!(
                    "  throughput_client_done: id={} requests={} errors={} p99_ns={}",
                    row.client_id, row.score_requests, row.errors, row.p99_latency_ns
                );
                row
            },
        ));
    }

    let mut client_rows = Vec::new();
    for handle in handles {
        client_rows.push(
            handle
                .join()
                .map_err(|_| "throughput client thread panicked".to_owned())?,
        );
    }
    let total_wall_latency_ns = wall_start.elapsed().as_nanos();

    let lb_metrics = send_json_get::<RoleBindingProfileMetricsResponse>(lb_addr, "/metrics")?;
    drop(load_balancer_process);
    let mut all_workers_serving_only = true;
    let mut worker_rows = Vec::new();
    let mut total_worker_runtime_bytes_estimate = 0usize;
    let mut max_worker_runtime_bytes_estimate = 0usize;
    let mut total_worker_rss_bytes = 0usize;
    let mut max_worker_rss_bytes = 0usize;
    let mut max_worker_p99_latency_ns = 0u128;
    let mut max_worker_core_score_p50_latency_ns = 0u128;
    let mut max_worker_core_score_p90_latency_ns = 0u128;
    let mut max_worker_core_score_p99_latency_ns = 0u128;
    let mut max_worker_score_p50_latency_ns = 0u128;
    let mut max_worker_score_p90_latency_ns = 0u128;
    let mut max_worker_score_p99_latency_ns = 0u128;
    for worker in &workers {
        let health = send_json_get::<RoleBindingProfileHealthResponse>(worker.addr, "/health")?;
        let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(worker.addr, "/metrics")?;
        all_workers_serving_only &= !health.compiler_used
            && !health.eval_packs_loaded
            && !health.corpus_jsonl_loaded
            && !health.python_demo_used
            && !metrics.compiler_used
            && !metrics.eval_packs_loaded
            && !metrics.corpus_jsonl_loaded
            && !metrics.python_demo_used;
        total_worker_runtime_bytes_estimate += metrics.runtime_bytes_estimate;
        max_worker_runtime_bytes_estimate =
            max_worker_runtime_bytes_estimate.max(metrics.runtime_bytes_estimate);
        total_worker_rss_bytes += metrics.rss_bytes;
        max_worker_rss_bytes = max_worker_rss_bytes.max(metrics.rss_bytes);
        max_worker_p99_latency_ns = max_worker_p99_latency_ns.max(metrics.p99_latency_ns);
        max_worker_core_score_p50_latency_ns =
            max_worker_core_score_p50_latency_ns.max(metrics.core_score_p50_latency_ns);
        max_worker_core_score_p90_latency_ns =
            max_worker_core_score_p90_latency_ns.max(metrics.core_score_p90_latency_ns);
        max_worker_core_score_p99_latency_ns =
            max_worker_core_score_p99_latency_ns.max(metrics.core_score_p99_latency_ns);
        max_worker_score_p50_latency_ns =
            max_worker_score_p50_latency_ns.max(metrics.worker_score_p50_latency_ns);
        max_worker_score_p90_latency_ns =
            max_worker_score_p90_latency_ns.max(metrics.worker_score_p90_latency_ns);
        max_worker_score_p99_latency_ns =
            max_worker_score_p99_latency_ns.max(metrics.worker_score_p99_latency_ns);
        worker_rows.push(RoleBindingProfileLoadBalancerReplayWorkerRow {
            worker_id: worker.worker_id,
            shard_config_path: worker.config_path.display().to_string(),
            bind_addr: worker.addr.to_string(),
            profile_count: health.profile_count,
            profile_ids: worker
                .registry
                .profiles
                .iter()
                .map(|profile| profile.config.profile_id.clone())
                .collect(),
            local_operator_calls: metrics.local_operator_calls,
            fallback_to_llm_calls: metrics.fallback_to_llm_calls,
            false_local_accepts: metrics.false_local_accepts,
            missed_expected_local: metrics.missed_expected_local,
            p50_latency_ns: metrics.p50_latency_ns,
            p90_latency_ns: metrics.p90_latency_ns,
            p99_latency_ns: metrics.p99_latency_ns,
            core_score_p50_latency_ns: metrics.core_score_p50_latency_ns,
            core_score_p90_latency_ns: metrics.core_score_p90_latency_ns,
            core_score_p99_latency_ns: metrics.core_score_p99_latency_ns,
            worker_score_p50_latency_ns: metrics.worker_score_p50_latency_ns,
            worker_score_p90_latency_ns: metrics.worker_score_p90_latency_ns,
            worker_score_p99_latency_ns: metrics.worker_score_p99_latency_ns,
            lb_upstream_roundtrip_p50_latency_ns: metrics.lb_upstream_roundtrip_p50_latency_ns,
            lb_upstream_roundtrip_p90_latency_ns: metrics.lb_upstream_roundtrip_p90_latency_ns,
            lb_upstream_roundtrip_p99_latency_ns: metrics.lb_upstream_roundtrip_p99_latency_ns,
            rss_bytes: metrics.rss_bytes,
            runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        });
    }

    let load_balancer_serving_only = !lb_health.compiler_used
        && !lb_health.eval_packs_loaded
        && !lb_health.corpus_jsonl_loaded
        && !lb_health.python_demo_used
        && !lb_metrics.compiler_used
        && !lb_metrics.eval_packs_loaded
        && !lb_metrics.corpus_jsonl_loaded
        && !lb_metrics.python_demo_used;
    let score_requests: usize = client_rows.iter().map(|row| row.score_requests).sum();
    let local_operator_calls: usize = client_rows.iter().map(|row| row.local_operator_calls).sum();
    let fallback_to_llm_calls: usize = client_rows
        .iter()
        .map(|row| row.fallback_to_llm_calls)
        .sum();
    let false_local_accepts: usize = client_rows.iter().map(|row| row.false_local_accepts).sum();
    let missed_expected_local: usize = client_rows
        .iter()
        .map(|row| row.missed_expected_local)
        .sum();
    let unexpected_local_accepts: usize = client_rows
        .iter()
        .map(|row| row.unexpected_local_accepts)
        .sum();
    let client_errors: usize = client_rows.iter().map(|row| row.errors).sum();
    let mut client_latencies = Vec::new();
    for row in &client_rows {
        client_latencies.push(row.p50_latency_ns);
        client_latencies.push(row.p90_latency_ns);
        client_latencies.push(row.p99_latency_ns);
    }
    let throughput_requests_per_second_milli = if total_wall_latency_ns == 0 {
        0
    } else {
        (score_requests as u128)
            .saturating_mul(1_000_000_000_000)
            .checked_div(total_wall_latency_ns)
            .unwrap_or(0)
    };
    let estimated_lb_overhead_p99_ns = lb_metrics
        .p99_latency_ns
        .saturating_sub(max_worker_score_p99_latency_ns);
    let pass = lb_health.status == "ok"
        && lb_profiles.profile_count == base_config.profiles.len()
        && score_requests == total_score_requests
        && client_errors == 0
        && false_local_accepts == 0
        && missed_expected_local == 0
        && unexpected_local_accepts == 0
        && packed_score_parity_mismatches == 0
        && all_workers_serving_only
        && load_balancer_serving_only
        && max_worker_runtime_bytes_estimate <= 3 * 1024 * 1024
        && lb_metrics.p99_latency_ns <= 3_000_000
        && max_worker_p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileLoadBalancerThroughputReport {
        schema_version: "nando_role_binding_profile_lb_throughput_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        binary_suite_report_path: binary_suite_path.display().to_string(),
        lb_config_path: lb_config_path.display().to_string(),
        lb_bind_addr: lb_addr.to_string(),
        worker_count,
        client_threads,
        sequence_repetitions,
        total_profile_count: base_config.profiles.len(),
        unique_sequences_replayed: request_set.unique_sequences,
        score_requests,
        expected_local_sequences: request_set.expected_local_sequences,
        expected_fallback_sequences: request_set.expected_fallback_sequences,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        unexpected_local_accepts,
        client_errors,
        total_wall_latency_ns,
        throughput_requests_per_second_milli,
        client_p50_latency_ns: percentile(&client_latencies, 50),
        client_p90_latency_ns: percentile(&client_latencies, 90),
        client_p99_latency_ns: percentile(&client_latencies, 99),
        load_balancer_p50_latency_ns: lb_metrics.p50_latency_ns,
        load_balancer_p90_latency_ns: lb_metrics.p90_latency_ns,
        load_balancer_p99_latency_ns: lb_metrics.p99_latency_ns,
        core_score_p50_latency_ns: max_worker_core_score_p50_latency_ns,
        core_score_p90_latency_ns: max_worker_core_score_p90_latency_ns,
        core_score_p99_latency_ns: max_worker_core_score_p99_latency_ns,
        worker_score_p50_latency_ns: max_worker_score_p50_latency_ns,
        worker_score_p90_latency_ns: max_worker_score_p90_latency_ns,
        worker_score_p99_latency_ns: max_worker_score_p99_latency_ns,
        lb_upstream_roundtrip_p50_latency_ns: lb_metrics.lb_upstream_roundtrip_p50_latency_ns,
        lb_upstream_roundtrip_p90_latency_ns: lb_metrics.lb_upstream_roundtrip_p90_latency_ns,
        lb_upstream_roundtrip_p99_latency_ns: lb_metrics.lb_upstream_roundtrip_p99_latency_ns,
        estimated_lb_overhead_p99_ns,
        packed_score_parity_checks,
        packed_score_parity_mismatches,
        load_balancer_rss_bytes: lb_metrics.rss_bytes,
        total_worker_runtime_bytes_estimate,
        max_worker_runtime_bytes_estimate,
        total_worker_rss_bytes,
        max_worker_rss_bytes,
        max_worker_p99_latency_ns,
        all_workers_serving_only,
        load_balancer_serving_only,
        eval_packs_used_by_replay_client: true,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        clients: client_rows,
        workers: worker_rows,
        claim_boundary: "Bounded concurrent local load-balancer /score pressure over multiple serving-only .nwrb profile workers. .nwreb eval packs are read only by the pressure client to generate requests; workers load no eval packs, corpora, compiler, training state, or Python demo code; the load-balancer loads only route-to-upstream metadata. This is not real Codex/API traffic, not a long-running daemon soak, and not cheap-VPS deployment.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-lb-throughput-v1: {}", report.verdict);
    println!("  report: {}", report_path.display());
    println!("  score_requests: {}", report.score_requests);
    println!(
        "  throughput_requests_per_second_milli: {}",
        report.throughput_requests_per_second_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  client_errors: {}", report.client_errors);
    println!(
        "  load_balancer_p99_latency_ns: {}",
        report.load_balancer_p99_latency_ns
    );
    println!("  client_p99_latency_ns: {}", report.client_p99_latency_ns);
    println!(
        "  max_worker_runtime_bytes_estimate: {}",
        report.max_worker_runtime_bytes_estimate
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile load-balancer throughput failed".to_owned())
    }
}

pub(crate) fn run_role_binding_real_traffic_record_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL));
    let record_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "role-binding-real-traffic-record-v1 requires record-json".to_owned())?;
    let record = read_json_file::<RoleBindingRealTrafficTraceRow>(&record_path)?;
    validate_real_traffic_trace_row(&record)?;
    append_jsonl_row(&trace_path, &record)?;
    println!("role-binding-real-traffic-record-v1: REAL_TRAFFIC_TRACE_ROW_RECORDED");
    println!("  trace: {}", trace_path.display());
    println!("  record: {}", record_path.display());
    println!("  trace_id: {}", record.trace_id);
    println!("  llm_call: {}", record.llm_call);
    println!(
        "  has_nando_shadow_request: {}",
        record.nando_shadow_request.is_some()
    );
    Ok(())
}

pub(crate) fn run_role_binding_real_traffic_record_serve_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL));
    let bind_addr = args.next().unwrap_or_else(|| "127.0.0.1:0".to_owned());
    let request_limit = parse_optional_usize(args.next(), "request_limit")?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|error| format!("failed to bind real-traffic recorder {bind_addr}: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read real-traffic recorder addr: {error}"))?;
    println!("role-binding-real-traffic-record-serve-v1: REAL_TRAFFIC_RECORDER_LISTENING");
    println!("  addr: {addr}");
    println!("  trace: {}", trace_path.display());
    println!(
        "  request_limit: {}",
        request_limit
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
    let stats = serve_real_traffic_record_requests(
        listener,
        trace_path.clone(),
        RoleBindingProfileServeConfig { request_limit },
    )?;
    println!("role-binding-real-traffic-record-serve-v1: REAL_TRAFFIC_RECORDER_STOPPED");
    println!("  trace: {}", trace_path.display());
    println!("  rows_written: {}", stats.rows_written);
    println!("  requests_handled: {}", stats.requests_handled);
    println!("  bad_requests: {}", stats.bad_requests);
    Ok(())
}

pub(crate) fn run_role_binding_real_traffic_ingest_events_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let events_path = args.next().map(PathBuf::from).ok_or_else(|| {
        "role-binding-real-traffic-ingest-events-v1 requires events-jsonl".to_owned()
    })?;
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_INGEST_REPORT));

    let events = read_real_traffic_event_jsonl(&events_path)?;
    let mut rows = Vec::with_capacity(events.len());
    let mut llm_calls = 0usize;
    let mut operator_candidate_events = 0usize;
    let mut synthetic_events = 0usize;
    let mut events_without_shadow_request = 0usize;

    for event in events {
        llm_calls += usize::from(event.llm_call);
        operator_candidate_events += usize::from(event.nando_shadow_request.is_some());
        synthetic_events += usize::from(event.synthetic_source == Some(true));
        events_without_shadow_request += usize::from(event.nando_shadow_request.is_none());
        let row = RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event.event_id,
            traffic_source: event.traffic_source,
            time_ms: event.time_ms,
            request_fingerprint: event.request_fingerprint,
            response_fingerprint: event.response_fingerprint,
            tool_call_fingerprints: event.tool_call_fingerprints,
            verification_source: event.verification_source,
            llm_call: event.llm_call,
            exact_cache_key: event.exact_cache_key,
            provider_cache_hit: event.provider_cache_hit,
            provider_cost_microusd: event.provider_cost_microusd,
            nando_shadow_request: event.nando_shadow_request,
            verified_safe_accept: event.verified_safe_accept,
            synthetic_source: event.synthetic_source,
            notes: event.notes,
        };
        validate_real_traffic_trace_row(&row)?;
        rows.push(row);
    }
    write_real_traffic_trace_jsonl(&trace_path, &rows)?;

    let pass = !rows.is_empty() && synthetic_events == 0 && operator_candidate_events > 0;
    let report = RoleBindingRealTrafficIngestReport {
        schema_version: "nando_role_binding_real_traffic_ingest_report_v1".to_owned(),
        verdict: if pass {
            "REAL_TRAFFIC_INGEST_V1_READY_FOR_SHADOW"
        } else {
            "REAL_TRAFFIC_INGEST_V1_REVIEW"
        }
        .to_owned(),
        events_path: events_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        total_events: rows.len(),
        llm_calls,
        operator_candidate_events,
        events_without_shadow_request,
        synthetic_events,
        rows_written: rows.len(),
        claim_boundary: "Batch ingestion only converts agent/API events into the real-traffic shadow trace contract. It does not prove savings. Synthetic events and traces without nando_shadow_request stay REVIEW.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-ingest-events-v1: {}",
        report.verdict
    );
    println!("  events: {}", events_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  total_events: {}", report.total_events);
    println!("  llm_calls: {}", report.llm_calls);
    println!(
        "  operator_candidate_events: {}",
        report.operator_candidate_events
    );
    println!("  synthetic_events: {}", report.synthetic_events);
    if pass {
        Ok(())
    } else {
        Err("real-traffic ingest report is review-only".to_owned())
    }
}

pub(crate) fn run_role_binding_real_traffic_codex_history_ingest_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let events_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_REAL_TRAFFIC_EVENTS_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_REAL_TRAFFIC_INGEST_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut events = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        events.push(RoleBindingRealTrafficEventRow {
            schema_version: "nando_role_binding_real_traffic_event_v1".to_owned(),
            event_id: format!("codex_history::{}::{}::{}", row.session_id, row.ts, index),
            traffic_source: Some("codex_history_local".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(format!("fnv1a64:{fingerprint:016x}")),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "local Codex history prompt fingerprint; raw text not written".to_owned(),
            ),
            llm_call: true,
            exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request: None,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(
                "real local Codex prompt history; no prepared Nando shadow request yet".to_owned(),
            ),
        });
    }
    write_real_traffic_event_jsonl(&events_path, &events)?;

    let report = RoleBindingCodexHistoryIngestReport {
        schema_version: "nando_role_binding_codex_history_ingest_report_v1".to_owned(),
        verdict: if events.is_empty() {
            "CODEX_HISTORY_EVENTS_V1_REVIEW"
        } else {
            "CODEX_HISTORY_EVENTS_V1_READY"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        events_path: events_path.display().to_string(),
        total_history_rows: history_rows.len(),
        max_events,
        events_written: events.len(),
        llm_calls: events.len(),
        nando_shadow_requests: 0,
        synthetic_events: 0,
        raw_text_written: false,
        claim_boundary: "Codex history ingestion creates privacy-safe real event fingerprints only. It does not prove savings because nando_shadow_request is not inferred from raw text.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-codex-history-ingest-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  events: {}", events_path.display());
    println!("  report: {}", report_path.display());
    println!("  total_history_rows: {}", report.total_history_rows);
    println!("  events_written: {}", report.events_written);
    println!("  raw_text_written: {}", report.raw_text_written);
    Ok(())
}

pub(crate) fn run_role_binding_real_traffic_codex_history_route_candidates_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let events_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_EVENTS_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut events = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut candidate_events = 0usize;
    let mut no_candidate_events = 0usize;
    let mut route_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let event_id = format!(
            "codex_history_route::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let candidate = route_catalog.classify_request_text(&row.text);
        let nando_shadow_request = candidate.as_ref().map(|candidate| {
            *route_counts.entry(candidate.route_key.clone()).or_insert(0) += 1;
            RoleBindingProfileScoreRequest {
                request_id: event_id.clone(),
                route_key: Some(candidate.route_key.clone()),
                profile_id: Some(candidate.profile_id.clone()),
                exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
                active_fringe: Vec::new(),
                slots: Vec::new(),
                expect_local_operator: Some(false),
            }
        });
        candidate_events += usize::from(nando_shadow_request.is_some());
        no_candidate_events += usize::from(nando_shadow_request.is_none());
        events.push(RoleBindingRealTrafficEventRow {
            schema_version: "nando_role_binding_real_traffic_event_v1".to_owned(),
            event_id,
            traffic_source: Some("codex_history_local_route_only".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(format!("fnv1a64:{fingerprint:016x}")),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "route-only candidate from local Codex prompt; raw text not written".to_owned(),
            ),
            llm_call: true,
            exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(
                "route-only Nando candidate; empty payload forces safe fallback until request builder exists"
                    .to_owned(),
            ),
        });
    }
    write_real_traffic_event_jsonl(&events_path, &events)?;
    let route_counts = route_counts
        .into_iter()
        .map(
            |(route_key, candidate_events)| RoleBindingCodexHistoryRouteCandidateCount {
                route_key,
                candidate_events,
            },
        )
        .collect::<Vec<_>>();
    let report = RoleBindingCodexHistoryRouteCandidateReport {
        schema_version: "nando_role_binding_codex_history_route_candidate_report_v1".to_owned(),
        verdict: "CODEX_HISTORY_ROUTE_CANDIDATES_V1_REVIEW".to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        events_path: events_path.display().to_string(),
        total_history_rows: history_rows.len(),
        max_events,
        events_written: events.len(),
        candidate_events,
        no_candidate_events,
        route_counts,
        raw_text_written: false,
        full_shadow_request_payload_built: false,
        claim_boundary: "Route-only candidate adapter over local Codex prompt text. It selects route/profile without response, target, or proof labels, but emits empty payloads so scoring must fallback. This is not a savings proof.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-codex-history-route-candidates-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  events: {}", events_path.display());
    println!("  report: {}", report_path.display());
    println!("  events_written: {}", report.events_written);
    println!("  candidate_events: {}", report.candidate_events);
    println!("  no_candidate_events: {}", report.no_candidate_events);
    println!("  full_shadow_request_payload_built: false");
    Err("codex history route candidates are review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_shadow_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_SHADOW_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let trace_rows = read_real_traffic_trace_jsonl(&trace_path)?;
    let mut exact_cache_seen = HashSet::new();
    let mut exact_cache_hits = 0usize;
    let mut provider_cache_hits = 0usize;
    let mut total_llm_calls = 0usize;
    let mut operator_candidate_calls = 0usize;
    let mut nando_shadow_accepts = 0usize;
    let mut nando_shadow_fallbacks = 0usize;
    let mut verified_safe_accepts = 0usize;
    let mut unverified_shadow_accepts = 0usize;
    let mut false_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    let mut incremental_savings_over_exact_cache = 0usize;
    let mut estimated_cost_saved_microusd = 0u128;
    let mut shadow_score_latencies_ns = Vec::new();
    let mut operator_accumulators =
        BTreeMap::<String, RoleBindingRealTrafficOperatorAccumulator>::new();
    let mut rows = Vec::new();

    for trace in &trace_rows {
        if trace.llm_call {
            total_llm_calls += 1;
        }
        let exact_cache_hit = trace
            .exact_cache_key
            .as_ref()
            .map(|key| !exact_cache_seen.insert(key.clone()))
            .unwrap_or(false);
        exact_cache_hits += usize::from(exact_cache_hit);
        provider_cache_hits += usize::from(trace.provider_cache_hit.unwrap_or(false));

        let mut shadow_accepted = false;
        let mut shadow_fallback = false;
        let mut shadow_action = "not_routable".to_owned();
        let mut shadow_latency_ns = 0u128;
        let mut energy_margin = 0i32;
        let mut min_slot_margin = 0i32;
        let mut false_local_accept = false;
        let mut verified_safe_accept = false;
        let mut unverified_accept = false;

        if let Some(request) = &trace.nando_shadow_request {
            operator_candidate_calls += 1;
            let response = score_role_binding_profile_request(&registry, request);
            shadow_accepted = response.accepted;
            shadow_fallback = response.fallback;
            shadow_action = response.action.clone();
            shadow_latency_ns = response.latency_ns;
            energy_margin = response.energy_margin;
            min_slot_margin = response.min_slot_margin;
            false_local_accept = trace
                .verified_safe_accept
                .map(|safe| response.accepted && !safe)
                .unwrap_or(response.false_local_accept);
            verified_safe_accept = response.accepted && trace.verified_safe_accept == Some(true);
            unverified_accept = response.accepted && trace.verified_safe_accept.is_none();

            shadow_score_latencies_ns.push(response.latency_ns);
            nando_shadow_accepts += usize::from(response.accepted);
            nando_shadow_fallbacks += usize::from(response.fallback);
            false_accepts += usize::from(false_local_accept);
            verified_safe_accepts += usize::from(verified_safe_accept);
            unverified_shadow_accepts += usize::from(unverified_accept);
            missed_expected_local += usize::from(
                trace.verified_safe_accept == Some(true)
                    && response.fallback
                    && request.expect_local_operator != Some(false),
            );
            if verified_safe_accept && !exact_cache_hit {
                incremental_savings_over_exact_cache += 1;
                estimated_cost_saved_microusd +=
                    u128::from(trace.provider_cost_microusd.unwrap_or(0));
            }

            let operator_key = real_traffic_operator_key(request, &shadow_action);
            let route_key = request
                .route_key
                .clone()
                .unwrap_or_else(|| "none".to_owned());
            let profile_id = request
                .profile_id
                .clone()
                .unwrap_or_else(|| "route_selected".to_owned());
            let accumulator = operator_accumulators
                .entry(operator_key)
                .or_insert_with(|| RoleBindingRealTrafficOperatorAccumulator {
                    route_key,
                    profile_id,
                    action: shadow_action.clone(),
                    ..RoleBindingRealTrafficOperatorAccumulator::default()
                });
            accumulator.candidate_calls += 1;
            accumulator.llm_calls += usize::from(trace.llm_call);
            accumulator.exact_cache_hits += usize::from(exact_cache_hit);
            accumulator.nando_shadow_accepts += usize::from(shadow_accepted);
            accumulator.verified_safe_accepts += usize::from(verified_safe_accept);
            accumulator.unverified_shadow_accepts += usize::from(unverified_accept);
            accumulator.false_accepts += usize::from(false_local_accept);
            accumulator.incremental_savings_over_exact_cache +=
                usize::from(verified_safe_accept && !exact_cache_hit);
            if verified_safe_accept && !exact_cache_hit {
                accumulator.estimated_cost_saved_microusd +=
                    u128::from(trace.provider_cost_microusd.unwrap_or(0));
            }
            accumulator.latencies_ns.push(shadow_latency_ns);
        }

        rows.push(RoleBindingRealTrafficShadowRow {
            trace_id: trace.trace_id.clone(),
            traffic_source: trace.traffic_source.clone(),
            request_fingerprint: trace.request_fingerprint.clone(),
            response_fingerprint: trace.response_fingerprint.clone(),
            tool_call_fingerprints: trace.tool_call_fingerprints.clone(),
            verification_source: trace.verification_source.clone(),
            llm_call: trace.llm_call,
            exact_cache_hit,
            provider_cache_hit: trace.provider_cache_hit.unwrap_or(false),
            nando_routable: trace.nando_shadow_request.is_some(),
            nando_shadow_accepted: shadow_accepted,
            nando_shadow_fallback: shadow_fallback,
            nando_shadow_action: shadow_action,
            verified_safe_accept,
            unverified_shadow_accept: unverified_accept,
            false_local_accept,
            shadow_score_latency_ns: shadow_latency_ns,
            energy_margin,
            min_slot_margin,
        });
    }

    let exact_cache_plus_nando_llm_calls = total_llm_calls
        .saturating_sub(exact_cache_hits)
        .saturating_sub(incremental_savings_over_exact_cache);
    let operator_rankings =
        real_traffic_operator_rankings(operator_accumulators, operator_candidate_calls);
    let pass = false_accepts == 0
        && unverified_shadow_accepts == 0
        && verified_safe_accepts > 0
        && incremental_savings_over_exact_cache > 0
        && operator_candidate_calls > 0
        && total_llm_calls > 0
        && trace_rows
            .iter()
            .all(|row| row.synthetic_source != Some(true))
        && !trace_rows.is_empty();

    let report = RoleBindingRealTrafficShadowReport {
        schema_version: "nando_role_binding_real_traffic_shadow_report_v1".to_owned(),
        verdict: if pass {
            "REAL_TRAFFIC_SHADOW_V1_PASS"
        } else {
            "REAL_TRAFFIC_SHADOW_V1_REVIEW"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        traffic_source: common_traffic_source(&trace_rows),
        time_window: real_traffic_time_window(&trace_rows),
        total_requests: trace_rows.len(),
        total_llm_calls,
        operator_candidate_calls,
        exact_cache_hits,
        provider_cache_hits,
        nando_shadow_accepts,
        nando_shadow_fallbacks,
        verified_safe_accepts,
        unverified_shadow_accepts,
        false_accepts,
        missed_expected_local,
        incremental_savings_over_exact_cache,
        exact_cache_llm_calls: total_llm_calls.saturating_sub(exact_cache_hits),
        exact_cache_plus_nando_llm_calls,
        incremental_reduction_vs_exact_cache_milli: reduction_milli(
            total_llm_calls.saturating_sub(exact_cache_hits),
            exact_cache_plus_nando_llm_calls,
        ),
        estimated_cost_saved_microusd,
        p50_shadow_score_latency_ns: percentile(&shadow_score_latencies_ns, 50),
        p90_shadow_score_latency_ns: percentile(&shadow_score_latencies_ns, 90),
        p99_shadow_score_latency_ns: percentile(&shadow_score_latencies_ns, 99),
        runtime_bytes_estimate: registry.total_runtime_bytes_estimate(),
        rss_bytes: current_rss_bytes(),
        serving_only_registry: true,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        synthetic_trace_used: trace_rows.iter().any(|row| row.synthetic_source == Some(true)),
        operator_rankings,
        rows,
        claim_boundary: "Real-traffic shadow analysis over recorded JSONL traces. Nando does not replace the LLM in this mode; verified savings count only when a shadow local accept is marked verified_safe_accept=true and is not an exact-cache hit. Synthetic traces force REVIEW and cannot be used as market savings claim.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-real-traffic-shadow-v1: {}", report.verdict);
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  total_requests: {}", report.total_requests);
    println!("  total_llm_calls: {}", report.total_llm_calls);
    println!("  exact_cache_hits: {}", report.exact_cache_hits);
    println!("  nando_shadow_accepts: {}", report.nando_shadow_accepts);
    println!("  verified_safe_accepts: {}", report.verified_safe_accepts);
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  incremental_reduction_vs_exact_cache_milli: {}",
        report.incremental_reduction_vs_exact_cache_milli
    );
    println!(
        "  p99_shadow_score_latency_ns: {}",
        report.p99_shadow_score_latency_ns
    );
    if pass {
        Ok(())
    } else {
        Err("real-traffic shadow report is review-only".to_owned())
    }
}

pub(crate) fn run_role_binding_real_traffic_cpu_route_forecast_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let route_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_REPORT));
    let shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_SHADOW_REPORT));
    let forecast_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_CPU_ROUTE_FORECAST_REPORT));

    let route_report =
        read_json_file::<RoleBindingCodexHistoryRouteCandidateReport>(&route_report_path)?;
    let shadow_report = read_json_file::<RoleBindingRealTrafficShadowReport>(&shadow_report_path)?;

    let rankings_by_route = shadow_report
        .operator_rankings
        .iter()
        .map(|ranking| (ranking.route_key.as_str(), ranking))
        .collect::<BTreeMap<_, _>>();
    let mut routes = route_report
        .route_counts
        .iter()
        .map(|route_count| {
            let ranking = rankings_by_route.get(route_count.route_key.as_str());
            let exact_cache_hits_inside_route = ranking
                .map(|ranking| ranking.exact_cache_hits)
                .unwrap_or_default();
            let current_accepts = ranking
                .map(|ranking| ranking.nando_shadow_accepts)
                .unwrap_or_default();
            let current_verified_safe_accepts = ranking
                .map(|ranking| ranking.verified_safe_accepts)
                .unwrap_or_default();
            let current_false_accepts = ranking
                .map(|ranking| ranking.false_accepts)
                .unwrap_or_default();
            let current_incremental_savings = ranking
                .map(|ranking| ranking.incremental_savings_over_exact_cache)
                .unwrap_or_default();
            let non_exact_candidate_calls = route_count
                .candidate_events
                .saturating_sub(exact_cache_hits_inside_route);
            let forecast_accept_25_percent_calls =
                projected_accepts(non_exact_candidate_calls, 250);
            let forecast_accept_50_percent_calls =
                projected_accepts(non_exact_candidate_calls, 500);
            let forecast_accept_80_percent_calls =
                projected_accepts(non_exact_candidate_calls, 800);
            let (recommended_cpu_work, recommended_payload_builder) =
                cpu_route_builder_recommendation(&route_count.route_key);
            RoleBindingCpuRouteForecastRow {
                route_key: route_count.route_key.clone(),
                profile_id: ranking
                    .map(|ranking| ranking.profile_id.clone())
                    .unwrap_or_else(|| route_count.route_key.clone()),
                candidate_events: route_count.candidate_events,
                candidate_share_milli_of_all_llm_calls: ratio_milli(
                    route_count.candidate_events,
                    shadow_report.total_llm_calls,
                ),
                candidate_share_milli_of_candidate_zone: ratio_milli(
                    route_count.candidate_events,
                    route_report.candidate_events,
                ),
                exact_cache_hits_inside_route,
                non_exact_candidate_calls,
                current_accepts,
                current_verified_safe_accepts,
                current_false_accepts,
                current_incremental_savings,
                payload_builder_status: "missing_request_side_active_fringe_and_slots".to_owned(),
                recommended_cpu_work,
                recommended_payload_builder,
                forecast_accept_25_percent_calls,
                forecast_accept_50_percent_calls,
                forecast_accept_80_percent_calls,
                priority_rank: 0,
            }
        })
        .collect::<Vec<_>>();
    routes.sort_by(|left, right| {
        right
            .non_exact_candidate_calls
            .cmp(&left.non_exact_candidate_calls)
            .then_with(|| left.route_key.cmp(&right.route_key))
    });
    for (index, route) in routes.iter_mut().enumerate() {
        route.priority_rank = index + 1;
    }

    let forecast_25_percent_additional_savings = routes
        .iter()
        .map(|route| route.forecast_accept_25_percent_calls)
        .sum();
    let forecast_50_percent_additional_savings = routes
        .iter()
        .map(|route| route.forecast_accept_50_percent_calls)
        .sum();
    let forecast_80_percent_additional_savings = routes
        .iter()
        .map(|route| route.forecast_accept_80_percent_calls)
        .sum();
    let forecast_25_percent_total_calls_removed = shadow_report
        .exact_cache_hits
        .saturating_add(forecast_25_percent_additional_savings);
    let forecast_50_percent_total_calls_removed = shadow_report
        .exact_cache_hits
        .saturating_add(forecast_50_percent_additional_savings);
    let forecast_80_percent_total_calls_removed = shadow_report
        .exact_cache_hits
        .saturating_add(forecast_80_percent_additional_savings);

    let market_claim_allowed = shadow_report.verified_safe_accepts > 0
        && shadow_report.incremental_savings_over_exact_cache > 0
        && !shadow_report.synthetic_trace_used
        && shadow_report.false_accepts == 0;
    let report = RoleBindingCpuRouteForecastReport {
        schema_version: "nando_role_binding_cpu_route_forecast_v1".to_owned(),
        verdict: if route_report.candidate_events > 0 {
            "CPU_ROUTE_FORECAST_V1_REVIEW"
        } else {
            "CPU_ROUTE_FORECAST_V1_NO_ROUTE_CANDIDATES"
        }
        .to_owned(),
        route_report_path: route_report_path.display().to_string(),
        shadow_report_path: shadow_report_path.display().to_string(),
        traffic_source: shadow_report.traffic_source.clone(),
        total_llm_calls: shadow_report.total_llm_calls,
        exact_cache_hits: shadow_report.exact_cache_hits,
        exact_cache_coverage_milli: ratio_milli(
            shadow_report.exact_cache_hits,
            shadow_report.total_llm_calls,
        ),
        operator_candidate_calls: route_report.candidate_events,
        operator_candidate_coverage_milli: ratio_milli(
            route_report.candidate_events,
            shadow_report.total_llm_calls,
        ),
        no_candidate_calls: route_report.no_candidate_events,
        current_nando_accepts: shadow_report.nando_shadow_accepts,
        current_verified_safe_accepts: shadow_report.verified_safe_accepts,
        current_false_accepts: shadow_report.false_accepts,
        current_incremental_savings_over_exact_cache: shadow_report
            .incremental_savings_over_exact_cache,
        current_incremental_reduction_vs_exact_cache_milli: shadow_report
            .incremental_reduction_vs_exact_cache_milli,
        full_shadow_request_payload_built: route_report.full_shadow_request_payload_built,
        market_claim_allowed,
        forecast_25_percent_additional_savings,
        forecast_50_percent_additional_savings,
        forecast_80_percent_additional_savings,
        forecast_25_percent_total_calls_removed,
        forecast_50_percent_total_calls_removed,
        forecast_80_percent_total_calls_removed,
        forecast_25_percent_total_reduction_milli: ratio_milli(
            forecast_25_percent_total_calls_removed,
            shadow_report.total_llm_calls,
        ),
        forecast_50_percent_total_reduction_milli: ratio_milli(
            forecast_50_percent_total_calls_removed,
            shadow_report.total_llm_calls,
        ),
        forecast_80_percent_total_reduction_milli: ratio_milli(
            forecast_80_percent_total_calls_removed,
            shadow_report.total_llm_calls,
        ),
        routes,
        claim_boundary: "Forecast only. It ranks real request-side CPU route candidates and estimates capacity if future payload builders create verified_safe_accepts. It is not market savings: current local accepts are zero and route candidates have empty active_fringe/slots.".to_owned(),
        next_engineering_debt: "Build request-side payload builders for priority routes: route/profile candidate -> active_fringe + slots, without response text, target labels, proof labels, or expected answer.".to_owned(),
    };

    write_json_file(&forecast_report_path, &report)?;
    println!(
        "role-binding-real-traffic-cpu-route-forecast-v1: {}",
        report.verdict
    );
    println!("  route_report: {}", route_report_path.display());
    println!("  shadow_report: {}", shadow_report_path.display());
    println!("  forecast_report: {}", forecast_report_path.display());
    println!("  total_llm_calls: {}", report.total_llm_calls);
    println!("  exact_cache_hits: {}", report.exact_cache_hits);
    println!(
        "  operator_candidate_calls: {}",
        report.operator_candidate_calls
    );
    println!(
        "  operator_candidate_coverage_milli: {}",
        report.operator_candidate_coverage_milli
    );
    println!("  current_nando_accepts: {}", report.current_nando_accepts);
    println!(
        "  forecast_50_percent_additional_savings: {}",
        report.forecast_50_percent_additional_savings
    );
    println!("  market_claim_allowed: {}", report.market_claim_allowed);
    Err("CPU route forecast is review-only; it is not verified savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_edit_payload_readiness_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_PAYLOAD_READINESS_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut rows = Vec::new();
    let mut candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut missing_scope_or_file = 0usize;
    let mut missing_marker = 0usize;
    let mut missing_length_or_shape = 0usize;
    let mut missing_edit_intent = 0usize;
    let mut route_counts = BTreeMap::<String, usize>::new();
    let mut builder_kind_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let Some(candidate) = route_catalog.classify_request_text(&row.text) else {
            continue;
        };
        if !candidate.route_key.contains("edit_marker_length") {
            continue;
        }
        candidate_events += 1;
        *route_counts.entry(candidate.route_key.clone()).or_insert(0) += 1;
        let readiness = analyze_edit_payload_readiness(&row.text);
        payload_ready_events += usize::from(readiness.payload_ready);
        missing_scope_or_file += usize::from(!readiness.has_scope_or_file);
        missing_marker += usize::from(!readiness.has_marker);
        missing_length_or_shape += usize::from(!readiness.has_length_or_shape);
        missing_edit_intent += usize::from(!readiness.has_edit_intent);
        *builder_kind_counts
            .entry(readiness.recommended_builder_kind.clone())
            .or_insert(0) += 1;
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        rows.push(RoleBindingEditPayloadReadinessRow {
            event_id: format!(
                "codex_history_edit_readiness::{}::{}::{}",
                row.session_id, row.ts, index
            ),
            request_fingerprint: format!("fnv1a64:{fingerprint:016x}"),
            route_key: candidate.route_key,
            profile_id: candidate.profile_id,
            has_edit_intent: readiness.has_edit_intent,
            has_scope_or_file: readiness.has_scope_or_file,
            has_marker: readiness.has_marker,
            has_length_or_shape: readiness.has_length_or_shape,
            has_code_or_patch_signal: readiness.has_code_or_patch_signal,
            payload_ready: readiness.payload_ready,
            recommended_builder_kind: readiness.recommended_builder_kind,
            missing_reasons: readiness.missing_reasons,
        });
    }

    let report = RoleBindingEditPayloadReadinessReport {
        schema_version: "nando_role_binding_edit_payload_readiness_v1".to_owned(),
        verdict: if payload_ready_events > 0 {
            "EDIT_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND"
        } else {
            "EDIT_PAYLOAD_READINESS_V1_REVIEW_NO_READY_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        candidate_events,
        payload_ready_events,
        payload_ready_rate_milli: ratio_milli(payload_ready_events, candidate_events),
        missing_scope_or_file,
        missing_marker,
        missing_length_or_shape,
        missing_edit_intent,
        route_counts: route_counts
            .into_iter()
            .map(|(route_key, count)| RoleBindingNamedCount { name: route_key, count })
            .collect(),
        builder_kind_counts: builder_kind_counts
            .into_iter()
            .map(|(kind, count)| RoleBindingNamedCount { name: kind, count })
            .collect(),
        raw_text_written: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows,
        claim_boundary: "Request-side edit payload readiness only. This reads local Codex prompt text at analysis time, writes no raw text, and does not create local accepts. Payload-ready means the request has enough request-side structure to attempt a future active_fringe/slot builder; it is not verified savings.".to_owned(),
        next_engineering_debt: "Use ready rows to build edit_marker_length_payload_builder_v1 that emits active_fringe and slots from request text only, then verify shadow accepts with false_accepts=0.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-payload-readiness-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  report: {}", report_path.display());
    println!("  candidate_events: {}", report.candidate_events);
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!(
        "  payload_ready_rate_milli: {}",
        report.payload_ready_rate_milli
    );
    println!("  raw_text_written: {}", report.raw_text_written);
    Err("edit payload readiness is review-only; it is not verified savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_shadow_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let trace_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(
            "target/nando-wave/real-traffic-shadow/real-traffic-shadow-smoke-v1.trace.jsonl",
        )
    });
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(8);
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }
    if let Some(parent) = trace_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create real-traffic shadow smoke trace dir {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(&trace_path, "").map_err(|error| {
        format!(
            "failed to reset real-traffic shadow smoke trace {}: {error}",
            trace_path.display()
        )
    })?;
    let mut rows_written = 0usize;
    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        let eval_pack =
            parse_profile_binary_eval_pack(Path::new(&suite_row.binary_eval_pack_path))?;
        let profile_limit = max_unique_sequences_per_profile.min(eval_pack.sequences.len());
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            let request = sequence_to_profile_request(&profile_id, sequence, &cache_key, 0);
            let row = RoleBindingRealTrafficTraceRow {
                schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
                trace_id: format!("synthetic_smoke::{}::{}", profile_id, sequence.task_id),
                traffic_source: Some("synthetic_binary_eval_pack_smoke".to_owned()),
                time_ms: None,
                request_fingerprint: Some(format!(
                    "synthetic_request::{}::{}",
                    profile_id, sequence.task_id
                )),
                response_fingerprint: Some(format!(
                    "synthetic_response::{}::{}",
                    profile_id, sequence.task_id
                )),
                tool_call_fingerprints: Vec::new(),
                verification_source: Some("binary_eval_pack_sequence_expectation".to_owned()),
                llm_call: true,
                exact_cache_key: Some(cache_key),
                provider_cache_hit: Some(false),
                provider_cost_microusd: Some(100),
                nando_shadow_request: Some(request),
                verified_safe_accept: Some(sequence.expect_local_operator),
                synthetic_source: Some(true),
                notes: Some("synthetic smoke only; forbidden as market savings claim".to_owned()),
            };
            append_jsonl_row(&trace_path, &row)?;
            rows_written += 1;
        }
    }
    println!("role-binding-real-traffic-shadow-smoke-v1: REAL_TRAFFIC_SHADOW_SMOKE_TRACE_READY");
    println!("  binary_suite: {}", binary_suite_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  rows_written: {rows_written}");
    println!("  synthetic_source: true");
    Ok(())
}

#[derive(Clone, Debug)]
struct RoleBindingProfileRuntime {
    config: RoleBindingProfileConfig,
    runtime: WavePredictorRoleBindingOffloadRuntime,
    package_info: WavePredictorRoleBindingPackageInfo,
    package_bytes: usize,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileRuntimeRegistry {
    profiles: Vec<RoleBindingProfileRuntime>,
    route_index: HashMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoleBindingProfileRuntimeLoadMode {
    ServingPackedOnly,
    WithReference,
}

impl RoleBindingProfileRuntimeRegistry {
    fn from_config_path(config_path: &Path) -> Result<Self, String> {
        Self::from_config_path_with_mode(
            config_path,
            RoleBindingProfileRuntimeLoadMode::ServingPackedOnly,
        )
    }

    fn from_config_path_with_reference(config_path: &Path) -> Result<Self, String> {
        Self::from_config_path_with_mode(
            config_path,
            RoleBindingProfileRuntimeLoadMode::WithReference,
        )
    }

    fn from_config_path_with_mode(
        config_path: &Path,
        load_mode: RoleBindingProfileRuntimeLoadMode,
    ) -> Result<Self, String> {
        let config = read_json_file::<RoleBindingProfileRegistryConfig>(config_path)?;
        validate_registry_config(&config)?;
        let mut profiles = Vec::new();
        let mut route_index = HashMap::new();
        for profile in config.profiles {
            if profile
                .package_path
                .extension()
                .and_then(|ext| ext.to_str())
                != Some("nwrb")
            {
                return Err(format!(
                    "serving registry may load only .nwrb packages: {}",
                    profile.package_path.display()
                ));
            }
            let package_bytes = fs::read(&profile.package_path).map_err(|error| {
                format!("failed to read {}: {error}", profile.package_path.display())
            })?;
            let policy =
                WavePredictorRoleBindingOffloadPolicy::new(profile.threshold).map_err(|error| {
                    format!("invalid threshold for {}: {error:?}", profile.profile_id)
                })?;
            let runtime = match load_mode {
                RoleBindingProfileRuntimeLoadMode::ServingPackedOnly => {
                    WavePredictorRoleBindingOffloadRuntime::from_package_bytes_serving_packed_only(
                        &package_bytes,
                        policy,
                    )
                }
                RoleBindingProfileRuntimeLoadMode::WithReference => {
                    WavePredictorRoleBindingOffloadRuntime::from_package_bytes_with_reference(
                        &package_bytes,
                        policy,
                    )
                }
            }
            .map_err(|error| {
                format!(
                    "failed to load .nwrb package {}: {error:?}",
                    profile.package_path.display()
                )
            })?;
            let package_info = runtime.package_info();
            let index = profiles.len();
            for route_key in &profile.accepted_route_keys {
                if route_index.insert(route_key.clone(), index).is_some() {
                    return Err(format!("duplicate accepted route_key: {route_key}"));
                }
            }
            profiles.push(RoleBindingProfileRuntime {
                config: profile,
                runtime,
                package_info,
                package_bytes: package_bytes.len(),
            });
        }
        if profiles.is_empty() {
            return Err("profile registry has no profiles".to_owned());
        }
        Ok(Self {
            profiles,
            route_index,
        })
    }

    fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    fn total_runtime_bytes_estimate(&self) -> usize {
        self.profiles
            .iter()
            .map(|profile| profile.runtime.bytes_estimate())
            .sum()
    }

    fn total_package_bytes(&self) -> usize {
        self.profiles
            .iter()
            .map(|profile| profile.package_bytes)
            .sum()
    }

    fn total_edge_count(&self) -> usize {
        self.profiles
            .iter()
            .map(|profile| profile.package_info.edge_count)
            .sum()
    }

    fn profile_summaries(&self) -> Vec<RoleBindingProfileSummary> {
        self.profiles
            .iter()
            .map(|profile| RoleBindingProfileSummary {
                profile_id: profile.config.profile_id.clone(),
                profile_kind: profile.config.profile_kind.clone(),
                operator_classes: profile.config.operator_classes.clone(),
                package_path: profile.config.package_path.display().to_string(),
                package_fingerprint64: profile.package_info.fingerprint64,
                package_bytes: profile.package_bytes,
                runtime_bytes_estimate: profile.runtime.bytes_estimate(),
                edge_count: profile.package_info.edge_count,
                slot_count: profile.config.slot_count,
                threshold: profile.config.threshold,
                accepted_route_keys: profile.config.accepted_route_keys.clone(),
            })
            .collect()
    }

    fn select_profile(
        &self,
        request: &RoleBindingProfileScoreRequest,
    ) -> Option<&RoleBindingProfileRuntime> {
        if let Some(profile_id) = request.profile_id.as_deref() {
            return self
                .profiles
                .iter()
                .find(|profile| profile.config.profile_id == profile_id);
        }
        let route_key = request.route_key.as_deref()?;
        let index = self.route_index.get(route_key)?;
        self.profiles.get(*index)
    }

    fn self_test_score_request(&self) -> Option<RoleBindingProfileScoreRequest> {
        let profile_id = self.profiles.first()?.config.profile_id.clone();
        self.self_test_score_request_for_profile(&profile_id)
    }

    fn self_test_score_request_for_profile(
        &self,
        profile_id: &str,
    ) -> Option<RoleBindingProfileScoreRequest> {
        let smoke_strength = 64_i16;
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.config.profile_id == profile_id)?;
        let edge = profile.runtime.sample_positive_edge()?;
        let estimated_margin =
            i32::from(smoke_strength) * i32::from(smoke_strength) * i32::from(edge.weight);
        if estimated_margin < profile.runtime.policy().local_margin_threshold {
            return None;
        }
        let lane_id = 17_u16;
        let signed_strength = if edge.sign_key == 0 { 1 } else { -1 };
        let role_center = profile
            .package_info
            .role_base
            .checked_add(u32::from(edge.slot_id).saturating_mul(profile.package_info.role_stride))?
            .checked_add(u32::from(lane_id))?;
        Some(RoleBindingProfileScoreRequest {
            request_id: "smoke_local".to_owned(),
            route_key: profile.config.accepted_route_keys.first().cloned(),
            profile_id: Some(profile.config.profile_id.clone()),
            exact_cache_key: Some("smoke_local_cache_key".to_owned()),
            active_fringe: vec![
                RoleBindingProfileActiveCenterRow {
                    center_id: edge.action_center,
                    strength: smoke_strength,
                },
                RoleBindingProfileActiveCenterRow {
                    center_id: role_center,
                    strength: smoke_strength,
                },
            ],
            slots: vec![RoleBindingProfileScoreSlotRow {
                binding_output_slot: Some(edge.output_slot_id),
                positive_impulses: vec![RoleBindingProfileImpulseRow {
                    lane_id,
                    signed_strength,
                }],
                negative_impulses: vec![RoleBindingProfileImpulseRow {
                    lane_id: lane_id.saturating_add(1),
                    signed_strength,
                }],
            }],
            expect_local_operator: Some(true),
        })
    }
}

#[derive(Debug)]
struct RoleBindingProfileLoadBalancerRuntime {
    upstreams: Vec<RoleBindingProfileLoadBalancerUpstreamRuntime>,
    profile_index: HashMap<String, usize>,
    route_index: HashMap<String, usize>,
}

#[derive(Debug)]
struct RoleBindingProfileLoadBalancerUpstreamRuntime {
    config: RoleBindingProfileLoadBalancerUpstreamConfig,
    addr: SocketAddr,
}

impl RoleBindingProfileLoadBalancerRuntime {
    fn from_config_path(config_path: &Path) -> Result<Self, String> {
        let config = read_json_file::<RoleBindingProfileLoadBalancerConfig>(config_path)?;
        validate_load_balancer_config(&config)?;
        let mut upstreams = Vec::new();
        let mut profile_index = HashMap::new();
        let mut route_index = HashMap::new();
        for upstream in config.upstreams {
            let index = upstreams.len();
            let addr = upstream
                .upstream_addr
                .parse::<SocketAddr>()
                .map_err(|error| {
                    format!(
                        "invalid load-balancer upstream addr {}: {error}",
                        upstream.upstream_addr
                    )
                })?;
            for profile_id in &upstream.profile_ids {
                if profile_index.insert(profile_id.clone(), index).is_some() {
                    return Err(format!("duplicate load-balancer profile_id: {profile_id}"));
                }
            }
            for route_key in &upstream.accepted_route_keys {
                if route_index.insert(route_key.clone(), index).is_some() {
                    return Err(format!("duplicate load-balancer route_key: {route_key}"));
                }
            }
            upstreams.push(RoleBindingProfileLoadBalancerUpstreamRuntime {
                config: upstream,
                addr,
            });
        }
        if upstreams.is_empty() {
            return Err("load-balancer config has no upstreams".to_owned());
        }
        Ok(Self {
            upstreams,
            profile_index,
            route_index,
        })
    }

    fn upstream_count(&self) -> usize {
        self.upstreams.len()
    }

    fn profile_count(&self) -> usize {
        self.profile_index.len()
    }

    fn select_upstream(
        &self,
        request: &RoleBindingProfileScoreRequest,
    ) -> Option<&RoleBindingProfileLoadBalancerUpstreamRuntime> {
        self.select_upstream_index(request)
            .and_then(|index| self.upstreams.get(index))
    }

    fn select_upstream_index(&self, request: &RoleBindingProfileScoreRequest) -> Option<usize> {
        if let Some(profile_id) = request.profile_id.as_deref() {
            return self.profile_index.get(profile_id).copied();
        }
        let route_key = request.route_key.as_deref()?;
        self.route_index.get(route_key).copied()
    }

    fn profile_rows(&self) -> Vec<RoleBindingProfileLoadBalancerProfileRow> {
        self.upstreams
            .iter()
            .flat_map(|upstream| {
                upstream.config.profile_ids.iter().map(move |profile_id| {
                    RoleBindingProfileLoadBalancerProfileRow {
                        profile_id: profile_id.clone(),
                        worker_id: upstream.config.worker_id,
                        upstream_addr: upstream.config.upstream_addr.clone(),
                        shard_config_path: upstream.config.shard_config_path.display().to_string(),
                    }
                })
            })
            .collect()
    }
}

fn serve_role_binding_profile_lb_requests(
    listener: TcpListener,
    load_balancer: Arc<RoleBindingProfileLoadBalancerRuntime>,
    config: RoleBindingProfileServeConfig,
) -> Result<RoleBindingProfileServeStats, String> {
    let mut stats = RoleBindingProfileServeStats::default();
    for stream in listener.incoming() {
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
        let mut stream = stream.map_err(|error| format!("failed to accept lb request: {error}"))?;
        stream
            .set_nodelay(true)
            .map_err(|error| format!("failed to set lb tcp nodelay: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set lb read timeout: {error}"))?;
        match handle_role_binding_profile_lb_http_request(&mut stream, &load_balancer, &mut stats) {
            Ok(()) => {}
            Err(error) => {
                stats.bad_requests += 1;
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

fn handle_role_binding_profile_lb_http_request(
    stream: &mut TcpStream,
    load_balancer: &RoleBindingProfileLoadBalancerRuntime,
    stats: &mut RoleBindingProfileServeStats,
) -> Result<(), RoleBindingProfileHttpError> {
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
            stats.requests_handled += 1;
            stats.health_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileLoadBalancerHealthResponse::from_runtime(load_balancer),
            )
        }
        ("GET", "/profiles") => {
            stats.requests_handled += 1;
            stats.profiles_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileLoadBalancerProfilesResponse::from_runtime(load_balancer),
            )
        }
        ("GET", "/metrics") => {
            stats.requests_handled += 1;
            stats.metrics_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileMetricsResponse::from_load_balancer(load_balancer, stats),
            )
        }
        ("POST", "/score") => {
            let score_request = serde_json::from_str::<RoleBindingProfileScoreRequest>(body)
                .map_err(|error| {
                    http_error(400, format!("failed to parse lb score JSON: {error}"))
                })?;
            let response = score_role_binding_profile_lb_request(load_balancer, &score_request);
            stats.record_score(&response);
            write_http_json(stream, 200, &response)
        }
        ("POST", "/replay") => {
            let replay_request = serde_json::from_str::<RoleBindingProfileReplayRequest>(body)
                .map_err(|error| {
                    http_error(400, format!("failed to parse lb replay JSON: {error}"))
                })?;
            let response =
                replay_role_binding_profile_lb_requests(load_balancer, &replay_request, stats);
            write_http_json(stream, 200, &response)
        }
        ("POST", _) | ("GET", _) => Err(http_error(
            404,
            format!("unsupported load-balancer HTTP route: {method} {path}"),
        )),
        _ => Err(http_error(
            405,
            format!("unsupported load-balancer HTTP method: {method} {path}"),
        )),
    }
}

fn serve_role_binding_profile_requests(
    listener: TcpListener,
    registry: Arc<RoleBindingProfileRuntimeRegistry>,
    config: RoleBindingProfileServeConfig,
) -> Result<RoleBindingProfileServeStats, String> {
    let mut stats = RoleBindingProfileServeStats::default();
    for stream in listener.incoming() {
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
        let mut stream = stream.map_err(|error| format!("failed to accept request: {error}"))?;
        stream
            .set_nodelay(true)
            .map_err(|error| format!("failed to set tcp nodelay: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set read timeout: {error}"))?;
        match handle_role_binding_profile_http_request(&mut stream, &registry, &mut stats) {
            Ok(()) => {}
            Err(error) => {
                stats.bad_requests += 1;
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

fn handle_role_binding_profile_http_request(
    stream: &mut TcpStream,
    registry: &RoleBindingProfileRuntimeRegistry,
    stats: &mut RoleBindingProfileServeStats,
) -> Result<(), RoleBindingProfileHttpError> {
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
            stats.requests_handled += 1;
            stats.health_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileHealthResponse::from_registry(registry),
            )
        }
        ("GET", "/profiles") => {
            stats.requests_handled += 1;
            stats.profiles_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileProfilesResponse::from_registry(registry),
            )
        }
        ("GET", "/metrics") => {
            stats.requests_handled += 1;
            stats.metrics_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileMetricsResponse::from_registry(registry, stats),
            )
        }
        ("POST", "/score") => {
            let score_request = serde_json::from_str::<RoleBindingProfileScoreRequest>(body)
                .map_err(|error| http_error(400, format!("failed to parse score JSON: {error}")))?;
            let response = score_role_binding_profile_request(registry, &score_request);
            stats.record_score(&response);
            write_http_json(stream, 200, &response)
        }
        ("POST", "/score-compact") => {
            let score_request = serde_json::from_str::<RoleBindingProfileScoreRequest>(body)
                .map_err(|error| {
                    http_error(400, format!("failed to parse compact score JSON: {error}"))
                })?;
            let response = score_role_binding_profile_request(registry, &score_request);
            stats.record_score(&response);
            write_http_json(
                stream,
                200,
                &RoleBindingProfileScoreCompactResponse::from_score_response(&response),
            )
        }
        ("POST", "/replay") => {
            let replay_request = serde_json::from_str::<RoleBindingProfileReplayRequest>(body)
                .map_err(|error| {
                    http_error(400, format!("failed to parse replay JSON: {error}"))
                })?;
            let response = replay_role_binding_profile_requests(registry, &replay_request, stats);
            write_http_json(stream, 200, &response)
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

fn serve_real_traffic_record_requests(
    listener: TcpListener,
    trace_path: PathBuf,
    config: RoleBindingProfileServeConfig,
) -> Result<RoleBindingRealTrafficRecordServeStats, String> {
    let mut stats = RoleBindingRealTrafficRecordServeStats::default();
    for stream in listener.incoming() {
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
        let mut stream = stream
            .map_err(|error| format!("failed to accept real-traffic recorder request: {error}"))?;
        stream
            .set_nodelay(true)
            .map_err(|error| format!("failed to set recorder tcp nodelay: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set recorder read timeout: {error}"))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set recorder write timeout: {error}"))?;
        match handle_real_traffic_record_http_request(&mut stream, &trace_path, &mut stats) {
            Ok(()) => {}
            Err(error) => {
                stats.bad_requests += 1;
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

fn handle_real_traffic_record_http_request(
    stream: &mut TcpStream,
    trace_path: &Path,
    stats: &mut RoleBindingRealTrafficRecordServeStats,
) -> Result<(), RoleBindingProfileHttpError> {
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
            stats.requests_handled += 1;
            stats.health_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingRealTrafficRecordHealthResponse {
                    schema_version: "nando_role_binding_real_traffic_record_health_v1".to_owned(),
                    status: "ok".to_owned(),
                    runtime: "nando-wave-real-traffic-recorder".to_owned(),
                    trace_path: trace_path.display().to_string(),
                    compiler_used: false,
                    eval_packs_loaded: false,
                    corpus_jsonl_loaded: false,
                    python_demo_used: false,
                },
            )
        }
        ("GET", "/metrics") => {
            stats.requests_handled += 1;
            stats.metrics_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingRealTrafficRecordMetricsResponse::from_stats(trace_path, stats),
            )
        }
        ("POST", "/trace") => {
            let row = serde_json::from_str::<RoleBindingRealTrafficTraceRow>(body)
                .map_err(|error| http_error(400, format!("failed to parse trace JSON: {error}")))?;
            validate_real_traffic_trace_row(&row)
                .map_err(|error| http_error(400, format!("invalid trace row: {error}")))?;
            append_jsonl_row(trace_path, &row)
                .map_err(|error| http_error(500, format!("failed to append trace row: {error}")))?;
            stats.requests_handled += 1;
            stats.trace_requests += 1;
            stats.rows_written += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingRealTrafficRecordTraceResponse {
                    schema_version: "nando_role_binding_real_traffic_record_trace_response_v1"
                        .to_owned(),
                    status: "recorded".to_owned(),
                    trace_path: trace_path.display().to_string(),
                    trace_id: row.trace_id,
                    rows_written: stats.rows_written,
                    synthetic_source: row.synthetic_source.unwrap_or(false),
                },
            )
        }
        ("POST", _) | ("GET", _) => Err(http_error(
            404,
            format!("unsupported real-traffic recorder route: {method} {path}"),
        )),
        _ => Err(http_error(
            405,
            format!("unsupported real-traffic recorder method: {method} {path}"),
        )),
    }
}

fn score_role_binding_profile_request(
    registry: &RoleBindingProfileRuntimeRegistry,
    request: &RoleBindingProfileScoreRequest,
) -> RoleBindingProfileScoreResponse {
    let start = Instant::now();
    let Some(profile) = registry.select_profile(request) else {
        return RoleBindingProfileScoreResponse::fallback(
            request,
            "profile_not_found",
            start.elapsed().as_nanos(),
        );
    };
    if request.active_fringe.is_empty() || request.slots.is_empty() {
        return RoleBindingProfileScoreResponse::fallback(
            request,
            "input_outside_profile_contract",
            start.elapsed().as_nanos(),
        );
    }
    let prepared = profile.runtime.prepare_active_fringe_from_iter(
        request
            .active_fringe
            .iter()
            .map(|active| (active.center_id, active.strength)),
    );
    let mut energy_margin = 0i32;
    let mut min_slot_margin = i32::MAX;
    let mut strict_ordered_pass = true;
    let core_start = Instant::now();
    for slot in &request.slots {
        let (positive_score, negative_score) =
            score_role_binding_profile_slot(&profile.runtime, &prepared, slot);
        let slot_margin = positive_score - negative_score;
        energy_margin += slot_margin;
        min_slot_margin = min_slot_margin.min(slot_margin);
        strict_ordered_pass &= slot_margin > 0;
    }
    let core_score_latency_ns = core_start.elapsed().as_nanos();
    if min_slot_margin == i32::MAX {
        return RoleBindingProfileScoreResponse::fallback(
            request,
            "no_scorable_slots",
            start.elapsed().as_nanos(),
        );
    }
    let threshold = profile.runtime.policy().local_margin_threshold;
    let accepted = strict_ordered_pass && energy_margin >= threshold;
    let fallback_reason = if accepted {
        None
    } else if !strict_ordered_pass {
        Some("strict_slot_check_failed".to_owned())
    } else {
        Some("margin_below_threshold".to_owned())
    };
    let false_local_accept = accepted && request.expect_local_operator == Some(false);
    let worker_score_latency_ns = start.elapsed().as_nanos();
    RoleBindingProfileScoreResponse {
        schema_version: "nando_role_binding_profile_score_response_v1".to_owned(),
        request_id: request.request_id.clone(),
        route_key: request.route_key.clone(),
        profile_id: Some(profile.config.profile_id.clone()),
        accepted,
        fallback: !accepted,
        fallback_reason,
        action: if accepted {
            "local_operator"
        } else {
            "fallback_to_llm"
        }
        .to_owned(),
        energy_margin,
        min_slot_margin,
        threshold,
        strict_ordered_pass,
        false_local_accept,
        latency_ns: worker_score_latency_ns,
        core_score_latency_ns,
        worker_score_latency_ns,
        lb_upstream_roundtrip_latency_ns: 0,
        lb_total_latency_ns: 0,
        package_fingerprint64: Some(profile.package_info.fingerprint64),
        runtime_bytes_estimate: profile.runtime.bytes_estimate(),
        forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
    }
}

fn score_role_binding_profile_lb_request(
    load_balancer: &RoleBindingProfileLoadBalancerRuntime,
    request: &RoleBindingProfileScoreRequest,
) -> RoleBindingProfileScoreResponse {
    let start = Instant::now();
    let Some(upstream) = load_balancer.select_upstream(request) else {
        let mut response = RoleBindingProfileScoreResponse::fallback(
            request,
            "load_balancer_route_not_found",
            start.elapsed().as_nanos(),
        );
        response.lb_total_latency_ns = response.latency_ns;
        return response;
    };
    let upstream_start = Instant::now();
    match send_json_post::<_, RoleBindingProfileScoreCompactResponse>(
        upstream.addr,
        "/score-compact",
        request,
    ) {
        Ok(compact_response) => {
            let upstream_roundtrip_latency_ns = upstream_start.elapsed().as_nanos();
            let lb_total_latency_ns = start.elapsed().as_nanos();
            let fallback_reason = if compact_response.accepted {
                None
            } else {
                Some("compact_worker_fallback".to_owned())
            };
            RoleBindingProfileScoreResponse {
                schema_version: "nando_role_binding_profile_score_response_v1".to_owned(),
                request_id: request.request_id.clone(),
                route_key: request.route_key.clone(),
                profile_id: request.profile_id.clone(),
                accepted: compact_response.accepted,
                fallback: compact_response.fallback,
                fallback_reason,
                action: compact_response.action,
                energy_margin: compact_response.energy_margin,
                min_slot_margin: compact_response.min_slot_margin,
                threshold: 0,
                strict_ordered_pass: compact_response.strict_ordered_pass,
                false_local_accept: compact_response.false_local_accept,
                latency_ns: lb_total_latency_ns,
                core_score_latency_ns: 0,
                worker_score_latency_ns: 0,
                lb_upstream_roundtrip_latency_ns: upstream_roundtrip_latency_ns,
                lb_total_latency_ns,
                package_fingerprint64: None,
                runtime_bytes_estimate: 0,
                forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
            }
        }
        Err(_) => {
            let upstream_roundtrip_latency_ns = upstream_start.elapsed().as_nanos();
            let mut response = RoleBindingProfileScoreResponse::fallback(
                request,
                "load_balancer_upstream_error",
                start.elapsed().as_nanos(),
            );
            response.lb_upstream_roundtrip_latency_ns = upstream_roundtrip_latency_ns;
            response.lb_total_latency_ns = response.latency_ns;
            response
        }
    }
}

fn score_role_binding_profile_slot(
    runtime: &WavePredictorRoleBindingOffloadRuntime,
    prepared: &WavePredictorRoleBindingPreparedFringe,
    slot: &RoleBindingProfileScoreSlotRow,
) -> (i32, i32) {
    let positive_score = slot
        .positive_impulses
        .iter()
        .map(|impulse| {
            runtime.score_alignment_prepared(
                prepared,
                impulse.lane_id,
                impulse.signed_strength,
                slot.binding_output_slot,
            )
        })
        .sum::<i32>();
    let negative_score = slot
        .negative_impulses
        .iter()
        .map(|impulse| {
            runtime.score_alignment_prepared(
                prepared,
                impulse.lane_id,
                impulse.signed_strength,
                slot.binding_output_slot,
            )
        })
        .sum::<i32>();
    (positive_score, negative_score)
}

fn replay_role_binding_profile_requests(
    registry: &RoleBindingProfileRuntimeRegistry,
    replay: &RoleBindingProfileReplayRequest,
    stats: &mut RoleBindingProfileServeStats,
) -> RoleBindingProfileReplayResponse {
    let mut seen_cache_keys = HashSet::new();
    let no_cache_llm_calls = replay.requests.len();
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut rows = Vec::new();

    for request in &replay.requests {
        let cache_key = request
            .exact_cache_key
            .clone()
            .unwrap_or_else(|| request.request_id.clone());
        if !seen_cache_keys.insert(cache_key.clone()) {
            rows.push(RoleBindingProfileReplayRow {
                request_id: request.request_id.clone(),
                exact_cache_key: cache_key,
                exact_cache_hit: true,
                action: "exact_cache_hit".to_owned(),
                accepted: false,
                fallback: false,
                margin: 0,
            });
            continue;
        }

        exact_cache_llm_calls += 1;
        let response = score_role_binding_profile_request(registry, request);
        stats.record_score(&response);
        false_local_accepts += usize::from(response.false_local_accept);
        missed_expected_local +=
            usize::from(request.expect_local_operator == Some(true) && !response.accepted);
        local_operator_calls += usize::from(response.accepted);
        fallback_to_llm_calls += usize::from(response.fallback);
        exact_cache_plus_nando_llm_calls += usize::from(!response.accepted);
        rows.push(RoleBindingProfileReplayRow {
            request_id: request.request_id.clone(),
            exact_cache_key: cache_key,
            exact_cache_hit: false,
            action: response.action,
            accepted: response.accepted,
            fallback: response.fallback,
            margin: response.energy_margin,
        });
    }

    stats.requests_handled += 1;
    stats.replay_requests += 1;
    let exact_cache_incremental_reduction_milli =
        ((exact_cache_llm_calls - exact_cache_plus_nando_llm_calls) * 1000)
            .checked_div(exact_cache_llm_calls)
            .unwrap_or(0);
    RoleBindingProfileReplayResponse {
        schema_version: "nando_role_binding_profile_replay_response_v1".to_owned(),
        request_id: replay.request_id.clone(),
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        p50_latency_ns: percentile(&stats.score_latencies_ns, 50),
        p90_latency_ns: percentile(&stats.score_latencies_ns, 90),
        p99_latency_ns: percentile(&stats.score_latencies_ns, 99),
        rows,
        forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
    }
}

fn replay_role_binding_profile_lb_requests(
    load_balancer: &RoleBindingProfileLoadBalancerRuntime,
    replay: &RoleBindingProfileReplayRequest,
    stats: &mut RoleBindingProfileServeStats,
) -> RoleBindingProfileReplayResponse {
    let mut seen_cache_keys = HashSet::new();
    let no_cache_llm_calls = replay.requests.len();
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut rows = Vec::new();

    for request in &replay.requests {
        let cache_key = request
            .exact_cache_key
            .clone()
            .unwrap_or_else(|| request.request_id.clone());
        if !seen_cache_keys.insert(cache_key.clone()) {
            rows.push(RoleBindingProfileReplayRow {
                request_id: request.request_id.clone(),
                exact_cache_key: cache_key,
                exact_cache_hit: true,
                action: "exact_cache_hit".to_owned(),
                accepted: false,
                fallback: false,
                margin: 0,
            });
            continue;
        }

        exact_cache_llm_calls += 1;
        let response = score_role_binding_profile_lb_request(load_balancer, request);
        stats.record_score(&response);
        false_local_accepts += usize::from(response.false_local_accept);
        missed_expected_local +=
            usize::from(request.expect_local_operator == Some(true) && !response.accepted);
        local_operator_calls += usize::from(response.accepted);
        fallback_to_llm_calls += usize::from(response.fallback);
        exact_cache_plus_nando_llm_calls += usize::from(!response.accepted);
        rows.push(RoleBindingProfileReplayRow {
            request_id: request.request_id.clone(),
            exact_cache_key: cache_key,
            exact_cache_hit: false,
            action: response.action,
            accepted: response.accepted,
            fallback: response.fallback,
            margin: response.energy_margin,
        });
    }

    stats.requests_handled += 1;
    stats.replay_requests += 1;
    let exact_cache_incremental_reduction_milli =
        ((exact_cache_llm_calls - exact_cache_plus_nando_llm_calls) * 1000)
            .checked_div(exact_cache_llm_calls)
            .unwrap_or(0);
    RoleBindingProfileReplayResponse {
        schema_version: "nando_role_binding_profile_replay_response_v1".to_owned(),
        request_id: replay.request_id.clone(),
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        p50_latency_ns: percentile(&stats.score_latencies_ns, 50),
        p90_latency_ns: percentile(&stats.score_latencies_ns, 90),
        p99_latency_ns: percentile(&stats.score_latencies_ns, 99),
        rows,
        forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct RoleBindingProfileServeConfig {
    request_limit: Option<usize>,
}

#[derive(Clone, Debug, Default)]
struct RoleBindingProfileServeStats {
    requests_handled: usize,
    score_requests: usize,
    replay_requests: usize,
    health_requests: usize,
    profiles_requests: usize,
    metrics_requests: usize,
    bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    score_latencies_ns: Vec<u128>,
    core_score_latencies_ns: Vec<u128>,
    worker_score_latencies_ns: Vec<u128>,
    lb_upstream_roundtrip_latencies_ns: Vec<u128>,
}

impl RoleBindingProfileServeStats {
    fn total_requests(&self) -> usize {
        self.requests_handled + self.bad_requests
    }

    fn record_score(&mut self, response: &RoleBindingProfileScoreResponse) {
        self.requests_handled += 1;
        self.score_requests += 1;
        self.local_operator_calls += usize::from(response.accepted);
        self.fallback_to_llm_calls += usize::from(response.fallback);
        self.false_local_accepts += usize::from(response.false_local_accept);
        self.missed_expected_local +=
            usize::from(response.fallback && response.strict_ordered_pass);
        self.score_latencies_ns.push(response.latency_ns);
        if response.core_score_latency_ns > 0 {
            self.core_score_latencies_ns
                .push(response.core_score_latency_ns);
        }
        if response.worker_score_latency_ns > 0 {
            self.worker_score_latencies_ns
                .push(response.worker_score_latency_ns);
        }
        if response.lb_upstream_roundtrip_latency_ns > 0 {
            self.lb_upstream_roundtrip_latencies_ns
                .push(response.lb_upstream_roundtrip_latency_ns);
        }
    }
}

#[derive(Clone, Debug, Default)]
struct RoleBindingRealTrafficRecordServeStats {
    requests_handled: usize,
    health_requests: usize,
    trace_requests: usize,
    metrics_requests: usize,
    bad_requests: usize,
    rows_written: usize,
}

impl RoleBindingRealTrafficRecordServeStats {
    fn total_requests(&self) -> usize {
        self.requests_handled + self.bad_requests
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileRegistryConfig {
    schema_version: String,
    profiles: Vec<RoleBindingProfileConfig>,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileConfig {
    profile_id: String,
    profile_kind: String,
    operator_classes: Vec<String>,
    package_path: PathBuf,
    runtime_bytes_estimate: usize,
    edge_count: usize,
    slot_count: u8,
    threshold: i32,
    accepted_route_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerConfig {
    schema_version: String,
    upstreams: Vec<RoleBindingProfileLoadBalancerUpstreamConfig>,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerUpstreamConfig {
    worker_id: usize,
    upstream_addr: String,
    shard_config_path: PathBuf,
    profile_ids: Vec<String>,
    accepted_route_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRuntimeReleaseSuiteReport {
    verdict: String,
    gate_pass: bool,
    rows: Vec<RoleBindingRuntimeReleaseSuiteRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRuntimeReleaseSuiteRow {
    label: String,
    seed: u8,
    margin_threshold: i32,
    package_path: PathBuf,
    package_bytes: usize,
    package_edge_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileBinaryEvalPackSuiteReport {
    verdict: String,
    gate_pass: bool,
    rows: Vec<RoleBindingProfileBinaryEvalPackSuiteRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileBinaryEvalPackSuiteRow {
    label: String,
    seed: u8,
    margin_threshold: i32,
    package_fingerprint64: u64,
    binary_eval_pack_path: String,
    sequence_count: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileBinaryEvalPack {
    package_fingerprint64: Option<u64>,
    source_package_path: Option<String>,
    generation_method: String,
    sequences: Vec<RoleBindingProfileSequenceEvalRow>,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileSequenceEvalRow {
    task_id: String,
    active_fringe: Vec<RoleBindingProfileActiveCenterRow>,
    slots: Vec<RoleBindingProfileScoreSlotRow>,
    expect_local_operator: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileActiveCenterRow {
    center_id: u32,
    strength: i16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileImpulseRow {
    lane_id: u16,
    signed_strength: i16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileScoreSlotRow {
    binding_output_slot: Option<u8>,
    positive_impulses: Vec<RoleBindingProfileImpulseRow>,
    negative_impulses: Vec<RoleBindingProfileImpulseRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileScoreRequest {
    request_id: String,
    route_key: Option<String>,
    profile_id: Option<String>,
    exact_cache_key: Option<String>,
    active_fringe: Vec<RoleBindingProfileActiveCenterRow>,
    slots: Vec<RoleBindingProfileScoreSlotRow>,
    expect_local_operator: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplayRequest {
    request_id: String,
    requests: Vec<RoleBindingProfileScoreRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileScoreResponse {
    schema_version: String,
    request_id: String,
    route_key: Option<String>,
    profile_id: Option<String>,
    accepted: bool,
    fallback: bool,
    fallback_reason: Option<String>,
    action: String,
    energy_margin: i32,
    min_slot_margin: i32,
    threshold: i32,
    strict_ordered_pass: bool,
    false_local_accept: bool,
    latency_ns: u128,
    #[serde(default, skip_serializing)]
    core_score_latency_ns: u128,
    #[serde(default, skip_serializing)]
    worker_score_latency_ns: u128,
    #[serde(default, skip_serializing)]
    lb_upstream_roundtrip_latency_ns: u128,
    #[serde(default, skip_serializing)]
    lb_total_latency_ns: u128,
    package_fingerprint64: Option<u64>,
    runtime_bytes_estimate: usize,
    forbidden_flags: RoleBindingProfileForbiddenFlags,
}

impl RoleBindingProfileScoreResponse {
    fn fallback(request: &RoleBindingProfileScoreRequest, reason: &str, latency_ns: u128) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_score_response_v1".to_owned(),
            request_id: request.request_id.clone(),
            route_key: request.route_key.clone(),
            profile_id: request.profile_id.clone(),
            accepted: false,
            fallback: true,
            fallback_reason: Some(reason.to_owned()),
            action: "fallback_to_llm".to_owned(),
            energy_margin: 0,
            min_slot_margin: 0,
            threshold: 0,
            strict_ordered_pass: false,
            false_local_accept: false,
            latency_ns,
            core_score_latency_ns: 0,
            worker_score_latency_ns: latency_ns,
            lb_upstream_roundtrip_latency_ns: 0,
            lb_total_latency_ns: 0,
            package_fingerprint64: None,
            runtime_bytes_estimate: 0,
            forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileScoreCompactResponse {
    accepted: bool,
    fallback: bool,
    action: String,
    energy_margin: i32,
    min_slot_margin: i32,
    strict_ordered_pass: bool,
    false_local_accept: bool,
}

impl RoleBindingProfileScoreCompactResponse {
    fn from_score_response(response: &RoleBindingProfileScoreResponse) -> Self {
        Self {
            accepted: response.accepted,
            fallback: response.fallback,
            action: response.action.clone(),
            energy_margin: response.energy_margin,
            min_slot_margin: response.min_slot_margin,
            strict_ordered_pass: response.strict_ordered_pass,
            false_local_accept: response.false_local_accept,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplayResponse {
    schema_version: String,
    request_id: String,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rows: Vec<RoleBindingProfileReplayRow>,
    forbidden_flags: RoleBindingProfileForbiddenFlags,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplayRow {
    request_id: String,
    exact_cache_key: String,
    exact_cache_hit: bool,
    action: String,
    accepted: bool,
    fallback: bool,
    margin: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficTraceRow {
    schema_version: String,
    trace_id: String,
    traffic_source: Option<String>,
    time_ms: Option<u64>,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    #[serde(default)]
    tool_call_fingerprints: Vec<String>,
    verification_source: Option<String>,
    llm_call: bool,
    exact_cache_key: Option<String>,
    provider_cache_hit: Option<bool>,
    provider_cost_microusd: Option<u64>,
    nando_shadow_request: Option<RoleBindingProfileScoreRequest>,
    verified_safe_accept: Option<bool>,
    synthetic_source: Option<bool>,
    notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficEventRow {
    schema_version: String,
    event_id: String,
    traffic_source: Option<String>,
    time_ms: Option<u64>,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    #[serde(default)]
    tool_call_fingerprints: Vec<String>,
    verification_source: Option<String>,
    llm_call: bool,
    exact_cache_key: Option<String>,
    provider_cache_hit: Option<bool>,
    provider_cost_microusd: Option<u64>,
    nando_shadow_request: Option<RoleBindingProfileScoreRequest>,
    verified_safe_accept: Option<bool>,
    synthetic_source: Option<bool>,
    notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficIngestReport {
    schema_version: String,
    verdict: String,
    events_path: String,
    trace_path: String,
    total_events: usize,
    llm_calls: usize,
    operator_candidate_events: usize,
    events_without_shadow_request: usize,
    synthetic_events: usize,
    rows_written: usize,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCodexHistoryRow {
    session_id: String,
    ts: u64,
    text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCodexHistoryIngestReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    events_path: String,
    total_history_rows: usize,
    max_events: usize,
    events_written: usize,
    llm_calls: usize,
    nando_shadow_requests: usize,
    synthetic_events: usize,
    raw_text_written: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCodexHistoryRouteCandidateReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    events_path: String,
    total_history_rows: usize,
    max_events: usize,
    events_written: usize,
    candidate_events: usize,
    no_candidate_events: usize,
    route_counts: Vec<RoleBindingCodexHistoryRouteCandidateCount>,
    raw_text_written: bool,
    full_shadow_request_payload_built: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCodexHistoryRouteCandidateCount {
    route_key: String,
    candidate_events: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficShadowReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    trace_path: String,
    traffic_source: String,
    time_window: String,
    total_requests: usize,
    total_llm_calls: usize,
    operator_candidate_calls: usize,
    exact_cache_hits: usize,
    provider_cache_hits: usize,
    nando_shadow_accepts: usize,
    nando_shadow_fallbacks: usize,
    verified_safe_accepts: usize,
    unverified_shadow_accepts: usize,
    false_accepts: usize,
    missed_expected_local: usize,
    incremental_savings_over_exact_cache: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    incremental_reduction_vs_exact_cache_milli: usize,
    estimated_cost_saved_microusd: u128,
    p50_shadow_score_latency_ns: u128,
    p90_shadow_score_latency_ns: u128,
    p99_shadow_score_latency_ns: u128,
    runtime_bytes_estimate: usize,
    rss_bytes: usize,
    serving_only_registry: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    synthetic_trace_used: bool,
    operator_rankings: Vec<RoleBindingRealTrafficOperatorRanking>,
    rows: Vec<RoleBindingRealTrafficShadowRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficShadowRow {
    trace_id: String,
    traffic_source: Option<String>,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    tool_call_fingerprints: Vec<String>,
    verification_source: Option<String>,
    llm_call: bool,
    exact_cache_hit: bool,
    provider_cache_hit: bool,
    nando_routable: bool,
    nando_shadow_accepted: bool,
    nando_shadow_fallback: bool,
    nando_shadow_action: String,
    verified_safe_accept: bool,
    unverified_shadow_accept: bool,
    false_local_accept: bool,
    shadow_score_latency_ns: u128,
    energy_margin: i32,
    min_slot_margin: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficRecordHealthResponse {
    schema_version: String,
    status: String,
    runtime: String,
    trace_path: String,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficRecordMetricsResponse {
    schema_version: String,
    trace_path: String,
    requests_handled: usize,
    health_requests: usize,
    trace_requests: usize,
    metrics_requests: usize,
    bad_requests: usize,
    rows_written: usize,
}

impl RoleBindingRealTrafficRecordMetricsResponse {
    fn from_stats(trace_path: &Path, stats: &RoleBindingRealTrafficRecordServeStats) -> Self {
        Self {
            schema_version: "nando_role_binding_real_traffic_record_metrics_v1".to_owned(),
            trace_path: trace_path.display().to_string(),
            requests_handled: stats.requests_handled,
            health_requests: stats.health_requests,
            trace_requests: stats.trace_requests,
            metrics_requests: stats.metrics_requests,
            bad_requests: stats.bad_requests,
            rows_written: stats.rows_written,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficRecordTraceResponse {
    schema_version: String,
    status: String,
    trace_path: String,
    trace_id: String,
    rows_written: usize,
    synthetic_source: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficOperatorRanking {
    operator_key: String,
    route_key: String,
    profile_id: String,
    action: String,
    candidate_calls: usize,
    traffic_share_milli: usize,
    llm_calls: usize,
    exact_cache_hits: usize,
    nando_shadow_accepts: usize,
    verified_safe_accepts: usize,
    unverified_shadow_accepts: usize,
    false_accepts: usize,
    incremental_savings_over_exact_cache: usize,
    estimated_cost_saved_microusd: u128,
    local_accept_rate_milli: usize,
    verified_accept_rate_milli: usize,
    p99_shadow_score_latency_ns: u128,
    value_score_microusd_per_ms: u128,
}

#[derive(Clone, Debug, Default)]
struct RoleBindingRealTrafficOperatorAccumulator {
    route_key: String,
    profile_id: String,
    action: String,
    candidate_calls: usize,
    llm_calls: usize,
    exact_cache_hits: usize,
    nando_shadow_accepts: usize,
    verified_safe_accepts: usize,
    unverified_shadow_accepts: usize,
    false_accepts: usize,
    incremental_savings_over_exact_cache: usize,
    estimated_cost_saved_microusd: u128,
    latencies_ns: Vec<u128>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCpuRouteForecastReport {
    schema_version: String,
    verdict: String,
    route_report_path: String,
    shadow_report_path: String,
    traffic_source: String,
    total_llm_calls: usize,
    exact_cache_hits: usize,
    exact_cache_coverage_milli: usize,
    operator_candidate_calls: usize,
    operator_candidate_coverage_milli: usize,
    no_candidate_calls: usize,
    current_nando_accepts: usize,
    current_verified_safe_accepts: usize,
    current_false_accepts: usize,
    current_incremental_savings_over_exact_cache: usize,
    current_incremental_reduction_vs_exact_cache_milli: usize,
    full_shadow_request_payload_built: bool,
    market_claim_allowed: bool,
    forecast_25_percent_additional_savings: usize,
    forecast_50_percent_additional_savings: usize,
    forecast_80_percent_additional_savings: usize,
    forecast_25_percent_total_calls_removed: usize,
    forecast_50_percent_total_calls_removed: usize,
    forecast_80_percent_total_calls_removed: usize,
    forecast_25_percent_total_reduction_milli: usize,
    forecast_50_percent_total_reduction_milli: usize,
    forecast_80_percent_total_reduction_milli: usize,
    routes: Vec<RoleBindingCpuRouteForecastRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCpuRouteForecastRow {
    priority_rank: usize,
    route_key: String,
    profile_id: String,
    candidate_events: usize,
    candidate_share_milli_of_all_llm_calls: usize,
    candidate_share_milli_of_candidate_zone: usize,
    exact_cache_hits_inside_route: usize,
    non_exact_candidate_calls: usize,
    current_accepts: usize,
    current_verified_safe_accepts: usize,
    current_false_accepts: usize,
    current_incremental_savings: usize,
    payload_builder_status: String,
    recommended_cpu_work: String,
    recommended_payload_builder: String,
    forecast_accept_25_percent_calls: usize,
    forecast_accept_50_percent_calls: usize,
    forecast_accept_80_percent_calls: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditPayloadReadinessReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    max_events: usize,
    total_history_rows: usize,
    candidate_events: usize,
    payload_ready_events: usize,
    payload_ready_rate_milli: usize,
    missing_scope_or_file: usize,
    missing_marker: usize,
    missing_length_or_shape: usize,
    missing_edit_intent: usize,
    route_counts: Vec<RoleBindingNamedCount>,
    builder_kind_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    rows: Vec<RoleBindingEditPayloadReadinessRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditPayloadReadinessRow {
    event_id: String,
    request_fingerprint: String,
    route_key: String,
    profile_id: String,
    has_edit_intent: bool,
    has_scope_or_file: bool,
    has_marker: bool,
    has_length_or_shape: bool,
    has_code_or_patch_signal: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingNamedCount {
    name: String,
    count: usize,
}

#[derive(Clone, Debug)]
struct EditPayloadReadiness {
    has_edit_intent: bool,
    has_scope_or_file: bool,
    has_marker: bool,
    has_length_or_shape: bool,
    has_code_or_patch_signal: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug)]
struct CodexHistoryRouteCatalog {
    edit: Option<CodexHistoryRouteCandidate>,
    conditional: Option<CodexHistoryRouteCandidate>,
    mixed: Option<CodexHistoryRouteCandidate>,
}

impl CodexHistoryRouteCatalog {
    fn from_registry(config: &RoleBindingProfileRegistryConfig) -> Result<Self, String> {
        let mut catalog = Self {
            edit: None,
            conditional: None,
            mixed: None,
        };
        for profile in &config.profiles {
            let route_key = profile
                .accepted_route_keys
                .first()
                .cloned()
                .ok_or_else(|| format!("profile {} has no route key", profile.profile_id))?;
            let candidate = CodexHistoryRouteCandidate {
                route_key,
                profile_id: profile.profile_id.clone(),
            };
            if profile.operator_classes.iter().any(|class| class == "edit") {
                catalog.edit.get_or_insert(candidate);
            } else if profile
                .operator_classes
                .iter()
                .any(|class| class == "condition_route" || class == "verify_repair")
            {
                catalog.conditional.get_or_insert(candidate);
            } else if profile
                .operator_classes
                .iter()
                .any(|class| class == "move_copy" || class == "order" || class == "compose")
            {
                catalog.mixed.get_or_insert(candidate);
            }
        }
        Ok(catalog)
    }

    fn classify_request_text(&self, text: &str) -> Option<CodexHistoryRouteCandidate> {
        let lower = text.to_lowercase();
        if contains_any(
            &lower,
            &[
                "исправ",
                "почини",
                "чини",
                "код",
                "файл",
                "diff",
                "patch",
                "clippy",
                "cargo",
                "git",
                "commit",
                "runtime",
                "refactor",
                "рефактор",
                "ошиб",
                "тест",
            ],
        ) {
            return self.edit.clone().or_else(|| self.mixed.clone());
        }
        if contains_any(
            &lower,
            &[
                "если",
                "проверь",
                "провер",
                "gate",
                "verify",
                "услов",
                "false_accept",
                "fallback",
                "margin",
                "review",
                "pass",
            ],
        ) {
            return self.conditional.clone().or_else(|| self.mixed.clone());
        }
        if contains_any(
            &lower,
            &[
                "сделай",
                "добавь",
                "запиши",
                "обнови",
                "перенеси",
                "route",
                "trace",
                "оператор",
                "рантайм",
                "runtime",
                "shadow",
                "traffic",
                "трафик",
            ],
        ) {
            return self.mixed.clone().or_else(|| self.edit.clone());
        }
        None
    }
}

#[derive(Clone, Debug)]
struct CodexHistoryRouteCandidate {
    route_key: String,
    profile_id: String,
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplaySuiteReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    binary_suite_report_path: String,
    profile_count: usize,
    unique_sequences_replayed: usize,
    http_replay_batches: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
    compiler_used: bool,
    eval_packs_loaded_in_serving_worker: bool,
    corpus_jsonl_loaded_in_serving_worker: bool,
    eval_packs_used_by_replay_client: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    rows: Vec<RoleBindingProfileReplaySuiteRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileFallbackSmokeReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    profile_count: usize,
    local_accept_pass: bool,
    bad_route_fallback_pass: bool,
    low_margin_fallback_pass: bool,
    local_action: String,
    bad_route_fallback_reason: Option<String>,
    low_margin_fallback_reason: Option<String>,
    local_energy_margin: i32,
    low_margin_energy_margin: i32,
    low_margin_threshold: i32,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileWorkerScalingReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    worker_count: usize,
    total_profile_count: usize,
    total_local_operator_calls: usize,
    total_fallback_to_llm_calls: usize,
    wrong_worker_route_fallbacks: usize,
    false_local_accepts: usize,
    max_worker_runtime_bytes_estimate: usize,
    max_worker_rss_bytes: usize,
    max_worker_p99_latency_ns: u128,
    all_workers_serving_only: bool,
    all_profile_score_pass: bool,
    all_wrong_worker_routes_fallback: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    rows: Vec<RoleBindingProfileWorkerScalingRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileWorkerScalingRow {
    worker_id: usize,
    shard_config_path: String,
    bind_addr: String,
    profile_count: usize,
    profile_ids: Vec<String>,
    runtime_bytes_estimate: usize,
    package_bytes: usize,
    edge_count: usize,
    local_accepts: usize,
    wrong_worker_route_fallbacks: usize,
    false_local_accepts: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileWorkerReplayReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    binary_suite_report_path: String,
    worker_count: usize,
    total_profile_count: usize,
    unique_sequences_replayed: usize,
    http_replay_batches: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    total_runtime_bytes_estimate: usize,
    max_worker_runtime_bytes_estimate: usize,
    total_rss_bytes: usize,
    max_worker_rss_bytes: usize,
    max_worker_p99_latency_ns: u128,
    all_workers_serving_only: bool,
    eval_packs_used_by_replay_client: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    rows: Vec<RoleBindingProfileWorkerReplayRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileWorkerReplayRow {
    worker_id: usize,
    shard_config_path: String,
    bind_addr: String,
    profile_count: usize,
    profile_ids: Vec<String>,
    unique_sequences_replayed: usize,
    http_replay_batches: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerReplayReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    binary_suite_report_path: String,
    lb_config_path: String,
    lb_bind_addr: String,
    worker_count: usize,
    total_profile_count: usize,
    unique_sequences_replayed: usize,
    http_replay_batches: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    load_balancer_p50_latency_ns: u128,
    load_balancer_p90_latency_ns: u128,
    load_balancer_p99_latency_ns: u128,
    core_score_p50_latency_ns: u128,
    core_score_p90_latency_ns: u128,
    core_score_p99_latency_ns: u128,
    worker_score_p50_latency_ns: u128,
    worker_score_p90_latency_ns: u128,
    worker_score_p99_latency_ns: u128,
    lb_upstream_roundtrip_p50_latency_ns: u128,
    lb_upstream_roundtrip_p90_latency_ns: u128,
    lb_upstream_roundtrip_p99_latency_ns: u128,
    replay_client_wall_p50_latency_ns: u128,
    replay_client_wall_p90_latency_ns: u128,
    replay_client_wall_p99_latency_ns: u128,
    estimated_lb_overhead_p99_ns: u128,
    packed_score_parity_checks: usize,
    packed_score_parity_mismatches: usize,
    load_balancer_rss_bytes: usize,
    total_worker_runtime_bytes_estimate: usize,
    max_worker_runtime_bytes_estimate: usize,
    total_worker_rss_bytes: usize,
    max_worker_rss_bytes: usize,
    max_worker_p99_latency_ns: u128,
    all_workers_serving_only: bool,
    load_balancer_serving_only: bool,
    eval_packs_used_by_replay_client: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    workers: Vec<RoleBindingProfileLoadBalancerReplayWorkerRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerThroughputReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    binary_suite_report_path: String,
    lb_config_path: String,
    lb_bind_addr: String,
    worker_count: usize,
    client_threads: usize,
    sequence_repetitions: usize,
    total_profile_count: usize,
    unique_sequences_replayed: usize,
    score_requests: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    unexpected_local_accepts: usize,
    client_errors: usize,
    total_wall_latency_ns: u128,
    throughput_requests_per_second_milli: u128,
    client_p50_latency_ns: u128,
    client_p90_latency_ns: u128,
    client_p99_latency_ns: u128,
    load_balancer_p50_latency_ns: u128,
    load_balancer_p90_latency_ns: u128,
    load_balancer_p99_latency_ns: u128,
    core_score_p50_latency_ns: u128,
    core_score_p90_latency_ns: u128,
    core_score_p99_latency_ns: u128,
    worker_score_p50_latency_ns: u128,
    worker_score_p90_latency_ns: u128,
    worker_score_p99_latency_ns: u128,
    lb_upstream_roundtrip_p50_latency_ns: u128,
    lb_upstream_roundtrip_p90_latency_ns: u128,
    lb_upstream_roundtrip_p99_latency_ns: u128,
    estimated_lb_overhead_p99_ns: u128,
    packed_score_parity_checks: usize,
    packed_score_parity_mismatches: usize,
    load_balancer_rss_bytes: usize,
    total_worker_runtime_bytes_estimate: usize,
    max_worker_runtime_bytes_estimate: usize,
    total_worker_rss_bytes: usize,
    max_worker_rss_bytes: usize,
    max_worker_p99_latency_ns: u128,
    all_workers_serving_only: bool,
    load_balancer_serving_only: bool,
    eval_packs_used_by_replay_client: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    clients: Vec<RoleBindingProfileThroughputClientRow>,
    workers: Vec<RoleBindingProfileLoadBalancerReplayWorkerRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileThroughputClientRow {
    client_id: usize,
    score_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    unexpected_local_accepts: usize,
    errors: usize,
    first_error: Option<String>,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerReplayWorkerRow {
    worker_id: usize,
    shard_config_path: String,
    bind_addr: String,
    profile_count: usize,
    profile_ids: Vec<String>,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    core_score_p50_latency_ns: u128,
    core_score_p90_latency_ns: u128,
    core_score_p99_latency_ns: u128,
    worker_score_p50_latency_ns: u128,
    worker_score_p90_latency_ns: u128,
    worker_score_p99_latency_ns: u128,
    lb_upstream_roundtrip_p50_latency_ns: u128,
    lb_upstream_roundtrip_p90_latency_ns: u128,
    lb_upstream_roundtrip_p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplaySuiteRow {
    profile_id: String,
    label: String,
    seed: u8,
    binary_eval_pack_path: String,
    unique_sequences_replayed: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileHealthResponse {
    schema_version: String,
    status: String,
    runtime: String,
    profile_count: usize,
    runtime_bytes_estimate: usize,
    package_bytes: usize,
    edge_count: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerHealthResponse {
    schema_version: String,
    status: String,
    runtime: String,
    upstream_count: usize,
    profile_count: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

impl RoleBindingProfileLoadBalancerHealthResponse {
    fn from_runtime(load_balancer: &RoleBindingProfileLoadBalancerRuntime) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_lb_health_v1".to_owned(),
            status: "ok".to_owned(),
            runtime: "nando-wave-profile-load-balancer".to_owned(),
            upstream_count: load_balancer.upstream_count(),
            profile_count: load_balancer.profile_count(),
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

impl RoleBindingProfileHealthResponse {
    fn from_registry(registry: &RoleBindingProfileRuntimeRegistry) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_health_v1".to_owned(),
            status: "ok".to_owned(),
            runtime: "nando-wave-profile-runtime".to_owned(),
            profile_count: registry.profile_count(),
            runtime_bytes_estimate: registry.total_runtime_bytes_estimate(),
            package_bytes: registry.total_package_bytes(),
            edge_count: registry.total_edge_count(),
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileProfilesResponse {
    schema_version: String,
    profile_count: usize,
    profiles: Vec<RoleBindingProfileSummary>,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerProfilesResponse {
    schema_version: String,
    upstream_count: usize,
    profile_count: usize,
    profiles: Vec<RoleBindingProfileLoadBalancerProfileRow>,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerProfileRow {
    profile_id: String,
    worker_id: usize,
    upstream_addr: String,
    shard_config_path: String,
}

impl RoleBindingProfileLoadBalancerProfilesResponse {
    fn from_runtime(load_balancer: &RoleBindingProfileLoadBalancerRuntime) -> Self {
        let profiles = load_balancer.profile_rows();
        Self {
            schema_version: "nando_role_binding_profile_lb_profiles_v1".to_owned(),
            upstream_count: load_balancer.upstream_count(),
            profile_count: profiles.len(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

impl RoleBindingProfileProfilesResponse {
    fn from_registry(registry: &RoleBindingProfileRuntimeRegistry) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_profiles_v1".to_owned(),
            profile_count: registry.profile_count(),
            profiles: registry.profile_summaries(),
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileSummary {
    profile_id: String,
    profile_kind: String,
    operator_classes: Vec<String>,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    runtime_bytes_estimate: usize,
    edge_count: usize,
    slot_count: u8,
    threshold: i32,
    accepted_route_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileMetricsResponse {
    schema_version: String,
    profile_count: usize,
    requests_handled: usize,
    score_requests: usize,
    replay_requests: usize,
    health_requests: usize,
    profiles_requests: usize,
    metrics_requests: usize,
    bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    core_score_p50_latency_ns: u128,
    core_score_p90_latency_ns: u128,
    core_score_p99_latency_ns: u128,
    worker_score_p50_latency_ns: u128,
    worker_score_p90_latency_ns: u128,
    worker_score_p99_latency_ns: u128,
    lb_upstream_roundtrip_p50_latency_ns: u128,
    lb_upstream_roundtrip_p90_latency_ns: u128,
    lb_upstream_roundtrip_p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

impl RoleBindingProfileMetricsResponse {
    fn from_registry(
        registry: &RoleBindingProfileRuntimeRegistry,
        stats: &RoleBindingProfileServeStats,
    ) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_metrics_v1".to_owned(),
            profile_count: registry.profile_count(),
            requests_handled: stats.requests_handled,
            score_requests: stats.score_requests,
            replay_requests: stats.replay_requests,
            health_requests: stats.health_requests,
            profiles_requests: stats.profiles_requests,
            metrics_requests: stats.metrics_requests,
            bad_requests: stats.bad_requests,
            local_operator_calls: stats.local_operator_calls,
            fallback_to_llm_calls: stats.fallback_to_llm_calls,
            false_local_accepts: stats.false_local_accepts,
            missed_expected_local: stats.missed_expected_local,
            p50_latency_ns: percentile(&stats.score_latencies_ns, 50),
            p90_latency_ns: percentile(&stats.score_latencies_ns, 90),
            p99_latency_ns: percentile(&stats.score_latencies_ns, 99),
            core_score_p50_latency_ns: percentile(&stats.core_score_latencies_ns, 50),
            core_score_p90_latency_ns: percentile(&stats.core_score_latencies_ns, 90),
            core_score_p99_latency_ns: percentile(&stats.core_score_latencies_ns, 99),
            worker_score_p50_latency_ns: percentile(&stats.worker_score_latencies_ns, 50),
            worker_score_p90_latency_ns: percentile(&stats.worker_score_latencies_ns, 90),
            worker_score_p99_latency_ns: percentile(&stats.worker_score_latencies_ns, 99),
            lb_upstream_roundtrip_p50_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                50,
            ),
            lb_upstream_roundtrip_p90_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                90,
            ),
            lb_upstream_roundtrip_p99_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                99,
            ),
            rss_bytes: current_rss_bytes(),
            runtime_bytes_estimate: registry.total_runtime_bytes_estimate(),
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }

    fn from_load_balancer(
        load_balancer: &RoleBindingProfileLoadBalancerRuntime,
        stats: &RoleBindingProfileServeStats,
    ) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_metrics_v1".to_owned(),
            profile_count: load_balancer.profile_count(),
            requests_handled: stats.requests_handled,
            score_requests: stats.score_requests,
            replay_requests: stats.replay_requests,
            health_requests: stats.health_requests,
            profiles_requests: stats.profiles_requests,
            metrics_requests: stats.metrics_requests,
            bad_requests: stats.bad_requests,
            local_operator_calls: stats.local_operator_calls,
            fallback_to_llm_calls: stats.fallback_to_llm_calls,
            false_local_accepts: stats.false_local_accepts,
            missed_expected_local: stats.missed_expected_local,
            p50_latency_ns: percentile(&stats.score_latencies_ns, 50),
            p90_latency_ns: percentile(&stats.score_latencies_ns, 90),
            p99_latency_ns: percentile(&stats.score_latencies_ns, 99),
            core_score_p50_latency_ns: percentile(&stats.core_score_latencies_ns, 50),
            core_score_p90_latency_ns: percentile(&stats.core_score_latencies_ns, 90),
            core_score_p99_latency_ns: percentile(&stats.core_score_latencies_ns, 99),
            worker_score_p50_latency_ns: percentile(&stats.worker_score_latencies_ns, 50),
            worker_score_p90_latency_ns: percentile(&stats.worker_score_latencies_ns, 90),
            worker_score_p99_latency_ns: percentile(&stats.worker_score_latencies_ns, 99),
            lb_upstream_roundtrip_p50_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                50,
            ),
            lb_upstream_roundtrip_p90_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                90,
            ),
            lb_upstream_roundtrip_p99_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                99,
            ),
            rss_bytes: current_rss_bytes(),
            runtime_bytes_estimate: 0,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileForbiddenFlags {
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

impl RoleBindingProfileForbiddenFlags {
    fn clean() -> Self {
        Self {
            target_center_id_training_used: false,
            proof_rule_id_training_authority_used: false,
            concrete_x_lookup_used: false,
            local_out_t_runtime_extension_used: false,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileRuntimeSmokeReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    profile_count: usize,
    endpoint_health_pass: bool,
    endpoint_profiles_pass: bool,
    endpoint_score_pass: bool,
    endpoint_replay_pass: bool,
    endpoint_metrics_pass: bool,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    false_local_accepts: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileHttpError {
    status_code: u16,
    message: String,
}

struct RoleBindingProfileServeProcess {
    child: Child,
}

impl RoleBindingProfileServeProcess {
    fn start(
        config_path: &Path,
        bind_addr: &str,
        request_limit: Option<usize>,
    ) -> Result<Self, String> {
        let exe = std::env::current_exe()
            .map_err(|error| format!("failed to locate current executable: {error}"))?;
        let mut command = Command::new(exe);
        command
            .arg("role-binding-profile-serve-v1")
            .arg(config_path)
            .arg(bind_addr);
        if let Some(request_limit) = request_limit {
            command.arg(request_limit.to_string());
        }
        let child = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("failed to spawn role-binding profile server: {error}"))?;
        thread::sleep(Duration::from_millis(100));
        Ok(Self { child })
    }

    fn wait(&mut self) -> Result<(), String> {
        let status = self
            .child
            .wait()
            .map_err(|error| format!("failed to wait for role-binding profile server: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("role-binding profile server exited with {status}"))
        }
    }
}

impl Drop for RoleBindingProfileServeProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

struct RoleBindingProfileLoadBalancerProcess {
    child: Child,
}

impl RoleBindingProfileLoadBalancerProcess {
    fn start(
        config_path: &Path,
        bind_addr: &str,
        request_limit: Option<usize>,
    ) -> Result<Self, String> {
        let exe = std::env::current_exe()
            .map_err(|error| format!("failed to locate current executable: {error}"))?;
        let mut command = Command::new(exe);
        command
            .arg("role-binding-profile-lb-serve-v1")
            .arg(config_path)
            .arg(bind_addr);
        if let Some(request_limit) = request_limit {
            command.arg(request_limit.to_string());
        }
        let child = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                format!("failed to spawn role-binding profile load-balancer: {error}")
            })?;
        thread::sleep(Duration::from_millis(100));
        Ok(Self { child })
    }
}

impl Drop for RoleBindingProfileLoadBalancerProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn send_replay_batch(
    addr: SocketAddr,
    profile_id: &str,
    batch_index: usize,
    batch: &mut Vec<RoleBindingProfileScoreRequest>,
) -> Result<RoleBindingProfileReplayResponse, String> {
    let replay = RoleBindingProfileReplayRequest {
        request_id: format!("{profile_id}_batch_{batch_index}"),
        requests: std::mem::take(batch),
    };
    send_json_post::<_, RoleBindingProfileReplayResponse>(addr, "/replay", &replay)
}

fn send_replay_batch_timed(
    addr: SocketAddr,
    profile_id: &str,
    batch_index: usize,
    batch: &mut Vec<RoleBindingProfileScoreRequest>,
) -> Result<(RoleBindingProfileReplayResponse, u128), String> {
    let start = Instant::now();
    let response = send_replay_batch(addr, profile_id, batch_index, batch)?;
    Ok((response, start.elapsed().as_nanos()))
}

fn verify_role_binding_profile_packed_score_parity(
    registry: &RoleBindingProfileRuntimeRegistry,
    binary_suite: &RoleBindingProfileBinaryEvalPackSuiteReport,
    max_unique_sequences_per_profile: usize,
) -> Result<(usize, usize), String> {
    let mut checks = 0usize;
    let mut mismatches = 0usize;
    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        let profile = registry
            .profiles
            .iter()
            .find(|profile| profile.config.profile_id == profile_id)
            .ok_or_else(|| format!("profile {profile_id} missing from parity registry"))?;
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            let prepared = profile.runtime.prepare_active_fringe_from_iter(
                sequence
                    .active_fringe
                    .iter()
                    .map(|active| (active.center_id, active.strength)),
            );
            for slot in &sequence.slots {
                for impulse in slot
                    .positive_impulses
                    .iter()
                    .chain(slot.negative_impulses.iter())
                {
                    let packed = profile.runtime.score_alignment_prepared(
                        &prepared,
                        impulse.lane_id,
                        impulse.signed_strength,
                        slot.binding_output_slot,
                    );
                    let reference = profile.runtime.score_alignment_prepared_reference(
                        &prepared,
                        impulse.lane_id,
                        impulse.signed_strength,
                        slot.binding_output_slot,
                    );
                    checks += 1;
                    mismatches += usize::from(packed != reference);
                }
            }
        }
    }
    Ok((checks, mismatches))
}

fn sequence_to_profile_request(
    profile_id: &str,
    sequence: &RoleBindingProfileSequenceEvalRow,
    exact_cache_key: &str,
    repeat_index: usize,
) -> RoleBindingProfileScoreRequest {
    RoleBindingProfileScoreRequest {
        request_id: format!("{}::r{}", sequence.task_id, repeat_index),
        route_key: Some(profile_id.to_owned()),
        profile_id: Some(profile_id.to_owned()),
        exact_cache_key: Some(exact_cache_key.to_owned()),
        active_fringe: sequence.active_fringe.clone(),
        slots: sequence.slots.clone(),
        expect_local_operator: Some(sequence.expect_local_operator),
    }
}

#[derive(Clone, Debug)]
struct RoleBindingProfileThroughputRequestSet {
    requests: Vec<RoleBindingProfileScoreRequest>,
    unique_sequences: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
}

fn build_role_binding_profile_throughput_requests(
    binary_suite: &RoleBindingProfileBinaryEvalPackSuiteReport,
    lb_config: &RoleBindingProfileLoadBalancerConfig,
    max_unique_sequences_per_profile: usize,
) -> Result<RoleBindingProfileThroughputRequestSet, String> {
    let mut requests = Vec::new();
    let mut unique_sequences = 0usize;
    let mut expected_local_sequences = 0usize;
    let mut expected_fallback_sequences = 0usize;
    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        if !lb_config
            .upstreams
            .iter()
            .any(|upstream| upstream.profile_ids.iter().any(|id| id == &profile_id))
        {
            return Err(format!(
                "profile {profile_id} from binary suite is missing in throughput lb config"
            ));
        }
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        if eval_pack.package_fingerprint64 != Some(suite_row.package_fingerprint64) {
            return Err(format!(
                "binary eval-pack fingerprint mismatch for {} seed{}: pack={:?} suite={}",
                suite_row.label,
                suite_row.seed,
                eval_pack.package_fingerprint64,
                suite_row.package_fingerprint64
            ));
        }
        if eval_pack.generation_method.trim().is_empty() {
            return Err(format!(
                "binary eval-pack generation_method is empty for {}",
                eval_pack_path.display()
            ));
        }
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            let mut request = sequence_to_profile_request(&profile_id, sequence, &cache_key, 0);
            request.request_id = format!("{}::{}::throughput", profile_id, sequence.task_id);
            expected_local_sequences += usize::from(sequence.expect_local_operator);
            expected_fallback_sequences += usize::from(!sequence.expect_local_operator);
            unique_sequences += 1;
            requests.push(request);
        }
    }
    Ok(RoleBindingProfileThroughputRequestSet {
        requests,
        unique_sequences,
        expected_local_sequences,
        expected_fallback_sequences,
    })
}

fn parse_profile_binary_eval_pack(path: &Path) -> Result<RoleBindingProfileBinaryEvalPack, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "failed to read binary eval-pack {}: {error}",
            path.display()
        )
    })?;
    let mut reader = RoleBindingProfileBinaryEvalPackReader::new(path, &bytes);
    reader.expect_magic()?;
    let package_fingerprint64 = match reader.read_u64()? {
        0 => None,
        value => Some(value),
    };
    let source_package_path = match reader.read_string()?.as_str() {
        "" => None,
        value => Some(value.to_owned()),
    };
    let generation_method = reader.read_string()?;
    let task_count = reader.read_u32()? as usize;
    let sequence_count = reader.read_u32()? as usize;
    reader.skip_tasks(task_count)?;
    let mut sequences = Vec::with_capacity(sequence_count);
    for _ in 0..sequence_count {
        let task_id = reader.read_string()?;
        let expect_local_operator = reader.read_bool()?;
        let active_fringe = reader.read_active_fringe()?;
        let slot_count = reader.read_u32()? as usize;
        let mut slots = Vec::with_capacity(slot_count);
        for _ in 0..slot_count {
            slots.push(RoleBindingProfileScoreSlotRow {
                binding_output_slot: reader.read_optional_slot()?,
                positive_impulses: reader.read_impulses()?,
                negative_impulses: reader.read_impulses()?,
            });
        }
        sequences.push(RoleBindingProfileSequenceEvalRow {
            task_id,
            active_fringe,
            slots,
            expect_local_operator,
        });
    }
    reader.finish()?;
    Ok(RoleBindingProfileBinaryEvalPack {
        package_fingerprint64,
        source_package_path,
        generation_method,
        sequences,
    })
}

struct RoleBindingProfileBinaryEvalPackReader<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> RoleBindingProfileBinaryEvalPackReader<'a> {
    fn new(path: &'a Path, bytes: &'a [u8]) -> Self {
        Self {
            path,
            bytes,
            offset: 0,
        }
    }

    fn expect_magic(&mut self) -> Result<(), String> {
        let magic = self.take(ROLE_BINDING_EVAL_PACK_BINARY_MAGIC.len())?;
        if magic != ROLE_BINDING_EVAL_PACK_BINARY_MAGIC {
            return Err(format!(
                "invalid role-binding binary eval-pack magic in {}",
                self.path.display()
            ));
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), String> {
        if self.offset != self.bytes.len() {
            return Err(format!(
                "trailing bytes in role-binding binary eval-pack {}: offset={} len={}",
                self.path.display(),
                self.offset,
                self.bytes.len()
            ));
        }
        Ok(())
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "role-binding binary eval-pack offset overflow".to_owned())?;
        let slice = self.bytes.get(self.offset..end).ok_or_else(|| {
            format!(
                "truncated role-binding binary eval-pack {} at offset {}",
                self.path.display(),
                self.offset
            )
        })?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(self.take(2)?);
        Ok(u16::from_le_bytes(buf))
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(self.take(2)?);
        Ok(i16::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(buf))
    }

    fn read_bool(&mut self) -> Result<bool, String> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(format!(
                "invalid bool value {value} in role-binding binary eval-pack {}",
                self.path.display()
            )),
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        let bytes = self.take(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| {
                format!(
                    "invalid utf8 string in role-binding binary eval-pack {}: {error}",
                    self.path.display()
                )
            })
    }

    fn read_optional_slot(&mut self) -> Result<Option<u8>, String> {
        match self.read_u16()? {
            u16::MAX => Ok(None),
            value => u8::try_from(value)
                .map(Some)
                .map_err(|_| format!("invalid output slot {value} in {}", self.path.display())),
        }
    }

    fn read_active_fringe(&mut self) -> Result<Vec<RoleBindingProfileActiveCenterRow>, String> {
        let len = self.read_u32()? as usize;
        let mut active_fringe = Vec::with_capacity(len);
        for _ in 0..len {
            active_fringe.push(RoleBindingProfileActiveCenterRow {
                center_id: self.read_u32()?,
                strength: self.read_i16()?,
            });
        }
        Ok(active_fringe)
    }

    fn read_impulses(&mut self) -> Result<Vec<RoleBindingProfileImpulseRow>, String> {
        let len = self.read_u32()? as usize;
        let mut impulses = Vec::with_capacity(len);
        for _ in 0..len {
            impulses.push(RoleBindingProfileImpulseRow {
                lane_id: self.read_u16()?,
                signed_strength: self.read_i16()?,
            });
        }
        Ok(impulses)
    }

    fn skip_tasks(&mut self, task_count: usize) -> Result<(), String> {
        for _ in 0..task_count {
            let _task_id = self.read_string()?;
            let _target_lane_id = self.read_u16()?;
            let _target_signed_strength = self.read_i16()?;
            let _wrong_lane_id = self.read_u16()?;
            let _wrong_signed_strength = self.read_i16()?;
            let _binding_output_slot = self.read_optional_slot()?;
            let _expect_local_operator = self.read_bool()?;
            let _active_fringe = self.read_active_fringe()?;
        }
        Ok(())
    }
}

fn validate_registry_config(config: &RoleBindingProfileRegistryConfig) -> Result<(), String> {
    if config.schema_version != "nando_role_binding_profile_registry_v1" {
        return Err(format!(
            "unsupported role-binding profile registry schema: {}",
            config.schema_version
        ));
    }
    if config.compiler_used
        || config.eval_packs_loaded
        || config.corpus_jsonl_loaded
        || config.python_demo_used
    {
        return Err("serving registry provenance flags must be false".to_owned());
    }
    if config.profiles.is_empty() {
        return Err("serving registry requires at least one profile".to_owned());
    }
    let mut profile_ids = HashSet::new();
    let mut routes = HashSet::new();
    for profile in &config.profiles {
        if profile.profile_id.trim().is_empty() {
            return Err("profile_id must not be empty".to_owned());
        }
        if !profile_ids.insert(profile.profile_id.clone()) {
            return Err(format!("duplicate profile_id: {}", profile.profile_id));
        }
        if profile.profile_kind != "role_binding_nwrb" {
            return Err(format!(
                "unsupported profile_kind for {}: {}",
                profile.profile_id, profile.profile_kind
            ));
        }
        if profile.threshold <= 0 {
            return Err(format!(
                "profile {} threshold must be positive",
                profile.profile_id
            ));
        }
        if profile.accepted_route_keys.is_empty() {
            return Err(format!(
                "profile {} needs at least one accepted_route_key",
                profile.profile_id
            ));
        }
        for route in &profile.accepted_route_keys {
            if !routes.insert(route.clone()) {
                return Err(format!("duplicate accepted_route_key: {route}"));
            }
        }
    }
    Ok(())
}

fn validate_load_balancer_config(
    config: &RoleBindingProfileLoadBalancerConfig,
) -> Result<(), String> {
    if config.schema_version != "nando_role_binding_profile_lb_v1" {
        return Err(format!(
            "unsupported role-binding profile load-balancer schema: {}",
            config.schema_version
        ));
    }
    if config.compiler_used
        || config.eval_packs_loaded
        || config.corpus_jsonl_loaded
        || config.python_demo_used
    {
        return Err("load-balancer provenance flags must be false".to_owned());
    }
    if config.upstreams.is_empty() {
        return Err("load-balancer config requires at least one upstream".to_owned());
    }
    let mut worker_ids = HashSet::new();
    let mut profile_ids = HashSet::new();
    let mut route_keys = HashSet::new();
    for upstream in &config.upstreams {
        if !worker_ids.insert(upstream.worker_id) {
            return Err(format!(
                "duplicate load-balancer worker_id: {}",
                upstream.worker_id
            ));
        }
        if upstream.upstream_addr.trim().is_empty() {
            return Err(format!(
                "load-balancer worker {} upstream_addr must not be empty",
                upstream.worker_id
            ));
        }
        upstream
            .upstream_addr
            .parse::<SocketAddr>()
            .map_err(|error| {
                format!(
                    "invalid load-balancer worker {} upstream_addr {}: {error}",
                    upstream.worker_id, upstream.upstream_addr
                )
            })?;
        if upstream.profile_ids.is_empty() {
            return Err(format!(
                "load-balancer worker {} needs at least one profile_id",
                upstream.worker_id
            ));
        }
        if upstream.accepted_route_keys.is_empty() {
            return Err(format!(
                "load-balancer worker {} needs at least one accepted_route_key",
                upstream.worker_id
            ));
        }
        for profile_id in &upstream.profile_ids {
            if profile_id.trim().is_empty() {
                return Err(format!(
                    "load-balancer worker {} has empty profile_id",
                    upstream.worker_id
                ));
            }
            if !profile_ids.insert(profile_id.clone()) {
                return Err(format!("duplicate load-balancer profile_id: {profile_id}"));
            }
        }
        for route_key in &upstream.accepted_route_keys {
            if route_key.trim().is_empty() {
                return Err(format!(
                    "load-balancer worker {} has empty route_key",
                    upstream.worker_id
                ));
            }
            if !route_keys.insert(route_key.clone()) {
                return Err(format!("duplicate load-balancer route_key: {route_key}"));
            }
        }
    }
    Ok(())
}

fn role_binding_profile_id(label: &str, seed: u8) -> String {
    let label = label.strip_prefix("sdk_").unwrap_or(label);
    format!("role_binding_{label}_seed{seed}")
}

fn role_binding_operator_classes(label: &str) -> Vec<String> {
    if label.contains("conditional") {
        return vec!["condition_route".to_owned(), "verify_repair".to_owned()];
    }
    if label.contains("edit") {
        return vec!["edit".to_owned()];
    }
    vec![
        "select".to_owned(),
        "move_copy".to_owned(),
        "order".to_owned(),
        "compose".to_owned(),
    ]
}

fn parse_optional_usize(value: Option<String>, name: &str) -> Result<Option<usize>, String> {
    value
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid {name} '{}': {error}", value))
        })
        .transpose()
}

fn read_json_file<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse JSON {}: {error}", path.display()))
}

fn write_json_file<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode JSON {}: {error}", path.display()))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, RoleBindingProfileHttpError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(|error| http_error(400, format!("failed to read request: {error}")))?;
        if read == 0 {
            if bytes.is_empty() {
                return Err(http_connection_closed());
            }
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_HTTP_REQUEST_BYTES {
            return Err(http_error(413, "HTTP request body too large"));
        }
        if let Some(header_end) = find_header_end(&bytes) {
            let head = std::str::from_utf8(&bytes[..header_end])
                .map_err(|error| http_error(400, format!("invalid HTTP header utf8: {error}")))?;
            let content_length = http_content_length(head)?;
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    String::from_utf8(bytes)
        .map_err(|error| http_error(400, format!("invalid HTTP request utf8: {error}")))
}

fn http_connection_closed() -> RoleBindingProfileHttpError {
    http_error(499, "connection closed before request")
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn http_content_length(head: &str) -> Result<usize, RoleBindingProfileHttpError> {
    for line in head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value
                .trim()
                .parse::<usize>()
                .map_err(|error| http_error(400, format!("invalid content-length: {error}")));
        }
    }
    Ok(0)
}

fn write_http_json<T>(
    stream: &mut TcpStream,
    status_code: u16,
    value: &T,
) -> Result<(), RoleBindingProfileHttpError>
where
    T: Serialize,
{
    let body = serde_json::to_string(value)
        .map_err(|error| http_error(500, format!("failed to encode response JSON: {error}")))?;
    let response = format!(
        "HTTP/1.1 {status_code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| http_error(500, format!("failed to write response: {error}")))
}

fn write_http_json_error(
    stream: &mut TcpStream,
    status_code: u16,
    message: &str,
) -> Result<(), String> {
    let body = serde_json::json!({
        "schema_version": "nando_role_binding_profile_error_v1",
        "status": "error",
        "message": message,
    });
    write_http_json(stream, status_code, &body).map_err(|error| error.message)
}

fn send_json_get<T>(addr: SocketAddr, path: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = send_http_request(addr, "GET", path, None)?;
    serde_json::from_str(&response.body).map_err(|error| {
        format!(
            "failed to parse GET {path} response: {error}; body={}",
            response.body
        )
    })
}

fn send_json_post<T, U>(addr: SocketAddr, path: &str, body: &T) -> Result<U, String>
where
    T: Serialize,
    U: for<'de> Deserialize<'de>,
{
    let body = serde_json::to_string(body)
        .map_err(|error| format!("failed to encode POST {path} request: {error}"))?;
    let response = send_http_request(addr, "POST", path, Some(&body))?;
    serde_json::from_str(&response.body).map_err(|error| {
        format!(
            "failed to parse POST {path} response: {error}; body={}",
            response.body
        )
    })
}

fn send_http_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<RoleBindingProfileHttpResponse, String> {
    let mut stream = connect_with_retry(addr)?;
    let body = body.unwrap_or("");
    send_http_request_on_stream(&mut stream, addr, method, path, body, "close")
}

fn send_http_request_on_stream(
    stream: &mut TcpStream,
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: &str,
    connection: &str,
) -> Result<RoleBindingProfileHttpResponse, String> {
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: {connection}\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to send HTTP request: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("failed to flush HTTP request: {error}"))?;
    read_http_response_from_stream(stream, method, path)
}

fn read_http_response_from_stream(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
) -> Result<RoleBindingProfileHttpResponse, String> {
    let mut response_bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                if response_bytes.is_empty() {
                    return Err("upstream closed before HTTP response".to_owned());
                }
                break;
            }
            Ok(read) => {
                response_bytes.extend_from_slice(&buffer[..read]);
                if let Some(header_end) = find_header_end(&response_bytes) {
                    let head = std::str::from_utf8(&response_bytes[..header_end])
                        .map_err(|error| format!("invalid HTTP response header utf8: {error}"))?;
                    let content_length =
                        http_content_length(head).map_err(|error| error.message)?;
                    if response_bytes.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
            }
            Err(error)
                if error.kind() == ErrorKind::ConnectionReset && !response_bytes.is_empty() =>
            {
                break;
            }
            Err(error) => return Err(format!("failed to read HTTP response: {error}")),
        }
    }
    let response = String::from_utf8(response_bytes)
        .map_err(|error| format!("invalid HTTP response utf8: {error}"))?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| format!("malformed HTTP response: {response}"))?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| "missing HTTP response status line".to_owned())?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("malformed HTTP response status line: {status_line}"))?
        .parse::<u16>()
        .map_err(|error| format!("invalid HTTP status code: {error}"))?;
    if !(200..300).contains(&status_code) {
        return Err(format!(
            "HTTP {method} {path} failed: status={status_code} body={body}"
        ));
    }
    Ok(RoleBindingProfileHttpResponse {
        body: body.to_owned(),
    })
}

#[derive(Clone, Debug)]
struct RoleBindingProfileHttpResponse {
    body: String,
}

fn connect_with_retry(addr: SocketAddr) -> Result<TcpStream, String> {
    let mut last_error = None;
    for _ in 0..50 {
        match TcpStream::connect(addr) {
            Ok(stream) => {
                stream
                    .set_nodelay(true)
                    .map_err(|error| format!("failed to set client tcp nodelay: {error}"))?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
                    .map_err(|error| format!("failed to set client read timeout: {error}"))?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
                    .map_err(|error| format!("failed to set client write timeout: {error}"))?;
                return Ok(stream);
            }
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    Err(format!(
        "failed to connect to smoke server {addr}: {}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "unknown connection error".to_owned())
    ))
}

fn reserve_local_bind_addr() -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to reserve local smoke port: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read reserved local smoke addr: {error}"))?;
    drop(listener);
    Ok(addr.to_string())
}

fn http_error(status_code: u16, message: impl Into<String>) -> RoleBindingProfileHttpError {
    RoleBindingProfileHttpError {
        status_code,
        message: message.into(),
    }
}

fn real_traffic_operator_key(request: &RoleBindingProfileScoreRequest, action: &str) -> String {
    let route_key = request.route_key.as_deref().unwrap_or("none");
    let profile_id = request.profile_id.as_deref().unwrap_or("route_selected");
    format!("route={route_key}|profile={profile_id}|action={action}")
}

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    numerator
        .saturating_mul(1000)
        .checked_div(denominator)
        .unwrap_or(0)
}

fn projected_accepts(non_exact_candidate_calls: usize, accept_rate_milli: usize) -> usize {
    non_exact_candidate_calls
        .saturating_mul(accept_rate_milli)
        .checked_div(1000)
        .unwrap_or(0)
}

fn cpu_route_builder_recommendation(route_key: &str) -> (String, String) {
    if route_key.contains("edit_marker_length") {
        (
            "CPU-process structured edit requests: detect edit intent, affected file/text marker, requested length/shape constraint, and deterministic patch slots from request text only.".to_owned(),
            "edit_marker_length_payload_builder_v1".to_owned(),
        )
    } else if route_key.contains("conditional_branch") {
        (
            "CPU-process check/gate/branch requests: extract condition, evidence slots, allowed/refused branch, and fallback threshold from request text only.".to_owned(),
            "conditional_branch_payload_builder_v1".to_owned(),
        )
    } else if route_key.contains("mixed_map") {
        (
            "CPU-process mapping/update/reorder requests: extract source slots, destination slots, ordered mapping action, and invariant checks from request text only.".to_owned(),
            "mixed_map_payload_builder_v1".to_owned(),
        )
    } else {
        (
            "CPU-process route candidates only after a request-side payload builder can emit active_fringe and slots without answer leakage.".to_owned(),
            "generic_request_side_payload_builder_v1".to_owned(),
        )
    }
}

fn analyze_edit_payload_readiness(text: &str) -> EditPayloadReadiness {
    let lower = text.to_lowercase();
    let has_edit_intent = contains_any(
        &lower,
        &[
            "edit",
            "rewrite",
            "replace",
            "patch",
            "fix",
            "update",
            "change",
            "refactor",
            "format",
            "shorten",
            "перепиши",
            "исправ",
            "измени",
            "замени",
            "сделай",
            "добавь",
            "убери",
            "сократи",
            "почини",
            "отредакт",
        ],
    );
    let has_scope_or_file = contains_file_like_token(text)
        || contains_any(
            &lower,
            &[
                "file",
                "path",
                "function",
                "module",
                "test",
                "doc",
                "readme",
                "код",
                "файл",
                "функц",
                "модул",
                "тест",
                "док",
                "строк",
            ],
        );
    let has_marker = contains_marker_like_signal(text)
        || contains_any(
            &lower,
            &[
                "this",
                "above",
                "below",
                "selected",
                "fragment",
                "snippet",
                "block",
                "это",
                "этот",
                "выше",
                "ниже",
                "фрагмент",
                "кусок",
                "блок",
                "вот это",
            ],
        );
    let has_length_or_shape = contains_any(
        &lower,
        &[
            "short",
            "shorter",
            "brief",
            "compact",
            "concise",
            "one line",
            "lines",
            "tokens",
            "json",
            "jsonl",
            "markdown",
            "table",
            "format",
            "schema",
            "коротко",
            "короче",
            "кратко",
            "без простын",
            "меньше",
            "строк",
            "формат",
            "таблиц",
            "схем",
        ],
    );
    let has_code_or_patch_signal = contains_any(
        &lower,
        &[
            "diff",
            "patch",
            "apply_patch",
            "cargo",
            "rust",
            "python",
            "json",
            "yaml",
            "toml",
            "markdown",
            "```",
            "код",
            "патч",
            "дифф",
        ],
    ) || contains_file_like_token(text);
    let payload_ready = has_edit_intent && has_scope_or_file && has_marker && has_length_or_shape;
    let mut missing_reasons = Vec::new();
    if !has_edit_intent {
        missing_reasons.push("missing_edit_intent".to_owned());
    }
    if !has_scope_or_file {
        missing_reasons.push("missing_scope_or_file".to_owned());
    }
    if !has_marker {
        missing_reasons.push("missing_marker".to_owned());
    }
    if !has_length_or_shape {
        missing_reasons.push("missing_length_or_shape".to_owned());
    }
    let recommended_builder_kind = if payload_ready {
        "edit_marker_length_payload_builder_v1_candidate".to_owned()
    } else if has_edit_intent && has_scope_or_file {
        "edit_scope_payload_builder_needs_marker_or_shape".to_owned()
    } else if has_edit_intent {
        "edit_intent_router_needs_scope_marker_shape".to_owned()
    } else {
        "not_edit_payload_ready".to_owned()
    };
    EditPayloadReadiness {
        has_edit_intent,
        has_scope_or_file,
        has_marker,
        has_length_or_shape,
        has_code_or_patch_signal,
        payload_ready,
        recommended_builder_kind,
        missing_reasons,
    }
}

fn contains_marker_like_signal(text: &str) -> bool {
    text.contains('`')
        || text.contains('"')
        || text.contains('\'')
        || text.contains("```")
        || text.contains("->")
        || text.contains("=>")
}

fn contains_file_like_token(text: &str) -> bool {
    text.split(|ch: char| {
        ch.is_whitespace() || matches!(ch, ',' | ';' | ':' | '(' | ')' | '[' | ']')
    })
    .any(|token| {
        let token = token
            .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`' | '<' | '>' | '.' | ',' | ';'));
        contains_any(
            token,
            &[
                ".rs", ".py", ".md", ".json", ".jsonl", ".toml", ".yaml", ".yml", ".ts", ".tsx",
                ".js", ".jsx", ".html", ".css", ".sh", ".sql", ".txt",
            ],
        ) || token.contains('/')
    })
}

fn real_traffic_operator_rankings(
    accumulators: BTreeMap<String, RoleBindingRealTrafficOperatorAccumulator>,
    total_operator_candidate_calls: usize,
) -> Vec<RoleBindingRealTrafficOperatorRanking> {
    let mut rankings = accumulators
        .into_iter()
        .map(|(operator_key, accumulator)| {
            let p99_latency_ns = percentile(&accumulator.latencies_ns, 99);
            let value_score_microusd_per_ms = if accumulator.false_accepts == 0
                && accumulator.unverified_shadow_accepts == 0
                && p99_latency_ns > 0
            {
                accumulator
                    .estimated_cost_saved_microusd
                    .saturating_mul(1_000_000)
                    .checked_div(p99_latency_ns)
                    .unwrap_or(0)
            } else {
                0
            };
            RoleBindingRealTrafficOperatorRanking {
                operator_key,
                route_key: accumulator.route_key,
                profile_id: accumulator.profile_id,
                action: accumulator.action,
                candidate_calls: accumulator.candidate_calls,
                traffic_share_milli: ratio_milli(
                    accumulator.candidate_calls,
                    total_operator_candidate_calls,
                ),
                llm_calls: accumulator.llm_calls,
                exact_cache_hits: accumulator.exact_cache_hits,
                nando_shadow_accepts: accumulator.nando_shadow_accepts,
                verified_safe_accepts: accumulator.verified_safe_accepts,
                unverified_shadow_accepts: accumulator.unverified_shadow_accepts,
                false_accepts: accumulator.false_accepts,
                incremental_savings_over_exact_cache: accumulator
                    .incremental_savings_over_exact_cache,
                estimated_cost_saved_microusd: accumulator.estimated_cost_saved_microusd,
                local_accept_rate_milli: ratio_milli(
                    accumulator.nando_shadow_accepts,
                    accumulator.candidate_calls,
                ),
                verified_accept_rate_milli: ratio_milli(
                    accumulator.verified_safe_accepts,
                    accumulator.candidate_calls,
                ),
                p99_shadow_score_latency_ns: p99_latency_ns,
                value_score_microusd_per_ms,
            }
        })
        .collect::<Vec<_>>();
    rankings.sort_by(|left, right| {
        right
            .value_score_microusd_per_ms
            .cmp(&left.value_score_microusd_per_ms)
            .then_with(|| {
                right
                    .incremental_savings_over_exact_cache
                    .cmp(&left.incremental_savings_over_exact_cache)
            })
            .then_with(|| right.candidate_calls.cmp(&left.candidate_calls))
            .then_with(|| left.operator_key.cmp(&right.operator_key))
    });
    rankings
}

fn validate_real_traffic_event_row(row: &RoleBindingRealTrafficEventRow) -> Result<(), String> {
    if row.schema_version != "nando_role_binding_real_traffic_event_v1" {
        return Err(format!(
            "unsupported real-traffic event schema_version '{}'",
            row.schema_version
        ));
    }
    if row.event_id.trim().is_empty() {
        return Err("real-traffic event row has empty event_id".to_owned());
    }
    if !row.llm_call && row.provider_cost_microusd.unwrap_or(0) > 0 {
        return Err(format!(
            "event_id={} has provider_cost_microusd but llm_call=false",
            row.event_id
        ));
    }
    if row.verified_safe_accept.is_some() && row.nando_shadow_request.is_none() {
        return Err(format!(
            "event_id={} has verified_safe_accept without nando_shadow_request",
            row.event_id
        ));
    }
    Ok(())
}

fn validate_real_traffic_trace_row(row: &RoleBindingRealTrafficTraceRow) -> Result<(), String> {
    if row.schema_version != "nando_role_binding_real_traffic_trace_v1" {
        return Err(format!(
            "unsupported real-traffic trace schema_version '{}'",
            row.schema_version
        ));
    }
    if row.trace_id.trim().is_empty() {
        return Err("real-traffic trace row has empty trace_id".to_owned());
    }
    if !row.llm_call && row.provider_cost_microusd.unwrap_or(0) > 0 {
        return Err(format!(
            "trace_id={} has provider_cost_microusd but llm_call=false",
            row.trace_id
        ));
    }
    if row.verified_safe_accept.is_some() && row.nando_shadow_request.is_none() {
        return Err(format!(
            "trace_id={} has verified_safe_accept without nando_shadow_request",
            row.trace_id
        ));
    }
    Ok(())
}

fn read_real_traffic_event_jsonl(
    events_path: &Path,
) -> Result<Vec<RoleBindingRealTrafficEventRow>, String> {
    let text = fs::read_to_string(events_path).map_err(|error| {
        format!(
            "failed to read real-traffic event JSONL {}: {error}",
            events_path.display()
        )
    })?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row =
            serde_json::from_str::<RoleBindingRealTrafficEventRow>(trimmed).map_err(|error| {
                format!(
                    "failed to parse real-traffic event JSONL {} line {}: {error}",
                    events_path.display(),
                    line_index + 1
                )
            })?;
        validate_real_traffic_event_row(&row)?;
        rows.push(row);
    }
    Ok(rows)
}

fn read_codex_history_jsonl(
    history_path: &Path,
) -> Result<Vec<RoleBindingCodexHistoryRow>, String> {
    let text = fs::read_to_string(history_path).map_err(|error| {
        format!(
            "failed to read Codex history JSONL {}: {error}",
            history_path.display()
        )
    })?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<RoleBindingCodexHistoryRow>(trimmed).map_err(|error| {
            format!(
                "failed to parse Codex history JSONL {} line {}: {error}",
                history_path.display(),
                line_index + 1
            )
        })?;
        rows.push(row);
    }
    Ok(rows)
}

fn read_real_traffic_trace_jsonl(
    trace_path: &Path,
) -> Result<Vec<RoleBindingRealTrafficTraceRow>, String> {
    let text = fs::read_to_string(trace_path).map_err(|error| {
        format!(
            "failed to read real-traffic trace JSONL {}: {error}",
            trace_path.display()
        )
    })?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row =
            serde_json::from_str::<RoleBindingRealTrafficTraceRow>(trimmed).map_err(|error| {
                format!(
                    "failed to parse real-traffic trace JSONL {} line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
        validate_real_traffic_trace_row(&row)?;
        rows.push(row);
    }
    Ok(rows)
}

fn write_real_traffic_event_jsonl(
    events_path: &Path,
    rows: &[RoleBindingRealTrafficEventRow],
) -> Result<(), String> {
    if let Some(parent) = events_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create event parent directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut text = String::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|error| format!("failed to serialize event JSONL row: {error}"))?;
        text.push_str(&line);
        text.push('\n');
    }
    fs::write(events_path, text).map_err(|error| {
        format!(
            "failed to write event JSONL {}: {error}",
            events_path.display()
        )
    })
}

fn write_real_traffic_trace_jsonl(
    trace_path: &Path,
    rows: &[RoleBindingRealTrafficTraceRow],
) -> Result<(), String> {
    if let Some(parent) = trace_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create trace parent directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut text = String::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|error| format!("failed to serialize trace JSONL row: {error}"))?;
        text.push_str(&line);
        text.push('\n');
    }
    fs::write(trace_path, text).map_err(|error| {
        format!(
            "failed to write trace JSONL {}: {error}",
            trace_path.display()
        )
    })
}

fn append_jsonl_row<T>(path: &Path, row: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create JSONL parent directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut line = serde_json::to_string(row)
        .map_err(|error| format!("failed to serialize JSONL row: {error}"))?;
    line.push('\n');
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("failed to open JSONL {}: {error}", path.display()))?;
    file.write_all(line.as_bytes())
        .map_err(|error| format!("failed to append JSONL {}: {error}", path.display()))
}

fn stable_real_traffic_fingerprint64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn common_traffic_source(rows: &[RoleBindingRealTrafficTraceRow]) -> String {
    let mut sources = HashSet::new();
    for row in rows {
        sources.insert(
            row.traffic_source
                .clone()
                .unwrap_or_else(|| "unknown".to_owned()),
        );
    }
    if sources.len() == 1 {
        sources
            .into_iter()
            .next()
            .unwrap_or_else(|| "unknown".to_owned())
    } else {
        format!("mixed:{}sources", sources.len())
    }
}

fn real_traffic_time_window(rows: &[RoleBindingRealTrafficTraceRow]) -> String {
    let mut min_time = u64::MAX;
    let mut max_time = 0u64;
    let mut seen = false;
    for row in rows {
        if let Some(time_ms) = row.time_ms {
            min_time = min_time.min(time_ms);
            max_time = max_time.max(time_ms);
            seen = true;
        }
    }
    if seen {
        format!("{min_time}..{max_time}")
    } else {
        "unknown".to_owned()
    }
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() * percentile) / 100).min(sorted.len() - 1);
    sorted[index]
}

fn reduction_milli(exact_cache_llm_calls: usize, exact_cache_plus_nando_llm_calls: usize) -> usize {
    exact_cache_llm_calls
        .saturating_sub(exact_cache_plus_nando_llm_calls)
        .saturating_mul(1000)
        .checked_div(exact_cache_llm_calls)
        .unwrap_or(0)
}

fn current_rss_bytes() -> usize {
    let Ok(statm) = fs::read_to_string("/proc/self/statm") else {
        return 0;
    };
    let Some(resident_pages) = statm
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return 0;
    };
    resident_pages.saturating_mul(4096)
}
