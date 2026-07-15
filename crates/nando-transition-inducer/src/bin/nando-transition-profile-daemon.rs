use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use nando_transition_inducer::{
    InducedExecutionStatus, LiveObservedTransition, LivePackageOrigin, LiveProfileRegistry,
    LiveProfileState, TransitionInducer, TransitionTrace, atomic_write_json, import_package,
    import_package_with_origin, packages_from_value, read_package, timestamp_unix_nanos,
    transition_surface_key,
};
use serde::Serialize;
use serde_json::Value;

const MIN_LIVE_SUPPORT_ROWS: usize = 32;
const MAX_LIVE_SUPPORT_ROWS: usize = 256;

#[path = "nando-transition-profile-daemon/raw_phase_live.rs"]
mod raw_phase_live;

#[derive(Clone, Debug)]
struct Config {
    state_dir: PathBuf,
    trace_path: PathBuf,
    event_path: PathBuf,
    economics_path: PathBuf,
    inbox_dir: PathBuf,
    registry_path: PathBuf,
    metrics_path: PathBuf,
    legacy_named_inducer_enabled: bool,
    once: bool,
}

#[derive(Clone, Debug, Serialize)]
struct MetricsSnapshot {
    report_kind: &'static str,
    verdict: &'static str,
    policy_version: String,
    registry_revision: u64,
    package_count: usize,
    profile_count: usize,
    active_profile_count: usize,
    non_raw_active_profile_count: usize,
    quarantined_profile_count: usize,
    revoked_profile_count: usize,
    traces_seen: usize,
    shadow_executions: usize,
    shadow_abstains: usize,
    false_accepts: usize,
    runtime_parity_mismatches: usize,
    profiles_promoted: usize,
    profiles_revoked: usize,
    local_accepts: usize,
    llm_calls_avoided: usize,
    tokens_saved: u64,
    total_bridge_requests: usize,
    total_bridge_tokens: u64,
    traffic_share_milli: usize,
    token_savings_share_milli: usize,
    execution_p99_ns: u64,
    raw_phase_enabled: bool,
    legacy_named_inducer_enabled: bool,
    raw_phase_family_count: usize,
    raw_phase_memorization_families: usize,
    raw_phase_circuit_families: usize,
    raw_phase_cleanup_families: usize,
    raw_phase_max_observed_surfaces: usize,
    raw_phase_total_observed_surfaces: usize,
    raw_phase_total_covered_surfaces: usize,
    raw_phase_total_unsupported_surfaces: usize,
    raw_phase_frontier_observed_rows: usize,
    raw_phase_frontier_covered_rows: usize,
    raw_phase_frontier_observed_tokens: u64,
    raw_phase_frontier_covered_tokens: u64,
    raw_phase_frontier_token_coverage_milli: usize,
    raw_phase_max_surface_sessions: usize,
    raw_phase_transfer_pass_families: usize,
    raw_phase_transfer_tested_surfaces: usize,
    raw_phase_transfer_passed_surfaces: usize,
    raw_phase_transfer_query_rows: usize,
    raw_phase_transfer_correct_executions: usize,
    raw_phase_transfer_abstains: usize,
    raw_phase_transfer_wrong_accepts: usize,
    raw_phase_session_transfer_pass_families: usize,
    raw_phase_session_transfer_query_rows: usize,
    raw_phase_session_transfer_correct_executions: usize,
    raw_phase_session_transfer_abstains: usize,
    raw_phase_session_transfer_wrong_accepts: usize,
    raw_phase_time_transfer_pass_families: usize,
    raw_phase_time_transfer_query_rows: usize,
    raw_phase_time_transfer_correct_executions: usize,
    raw_phase_time_transfer_abstains: usize,
    raw_phase_time_transfer_wrong_accepts: usize,
    raw_phase_max_discovered_predicates: usize,
    raw_phase_training_attempts: usize,
    raw_phase_packages_induced: usize,
    raw_phase_training_cpu_ns: u64,
    raw_phase_induction_cpu_ns: u64,
    boundary: &'static str,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("nando-transition-profile-daemon: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_env_and_args()?;
    fs::create_dir_all(&config.state_dir)
        .map_err(|error| format!("state_dir:{}:{error}", config.state_dir.display()))?;
    fs::create_dir_all(&config.inbox_dir)
        .map_err(|error| format!("inbox_dir:{}:{error}", config.inbox_dir.display()))?;
    loop {
        run_cycle(&config)?;
        if config.once {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(1_000));
    }
}

fn run_cycle(config: &Config) -> Result<(), String> {
    let mut registry = LiveProfileRegistry::load(&config.registry_path)?;
    let original = registry.clone();
    enforce_production_origin(&mut registry);
    ingest_package_inbox(&mut registry, config)?;
    let traces = read_new_traces(&config.trace_path, &mut registry)?;
    process_future_shadow(&mut registry, &traces)?;
    let frontier_migration_needed = registry
        .raw_phase_families
        .values()
        .any(|family| family.surface_frontier.is_empty());
    if !traces.is_empty() || registry.raw_phase_families.is_empty() || frontier_migration_needed {
        let all_traces = read_all_valid_traces(&config.trace_path)?;
        if !all_traces.is_empty() {
            raw_phase_live::induce_raw_phase_families(&mut registry, &all_traces, config)?;
            if config.legacy_named_inducer_enabled {
                induce_legacy_named_classes(&mut registry, &all_traces, config)?;
            }
        }
    }
    ingest_bridge_events(&config.event_path, &mut registry)?;
    reconcile_verified_economics(&config.economics_path, &mut registry)?;
    let changed = registry != original;
    if changed || !config.registry_path.exists() {
        registry.save(&config.registry_path)?;
    }
    if changed || !config.metrics_path.exists() {
        write_metrics(&registry, config)?;
    }
    Ok(())
}

fn enforce_production_origin(registry: &mut LiveProfileRegistry) {
    let mut revoked = 0usize;
    for record in registry.packages.values_mut() {
        if record.origin == LivePackageOrigin::RawPhaseInduction {
            continue;
        }
        for profile in &mut record.profiles {
            if profile.state == LiveProfileState::Active {
                profile.state = LiveProfileState::Revoked;
                profile.revoked_at_trace = Some("authority-origin-migration-v1".to_owned());
                profile.last_reason = "automatic_revoke_non_raw_production_authority".to_owned();
                revoked = revoked.saturating_add(1);
            }
        }
    }
    registry.telemetry.profiles_revoked =
        registry.telemetry.profiles_revoked.saturating_add(revoked);
}

impl Config {
    fn from_env_and_args() -> Result<Self, String> {
        let state_dir = env::var_os("NANDO_TRANSITION_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/lib/nando-wave/transition"));
        let mut once = false;
        for argument in env::args().skip(1) {
            if argument == "--once" {
                once = true;
            } else {
                return Err(format!("unexpected_argument:{argument}"));
            }
        }
        Ok(Self {
            trace_path: env_path(
                "NANDO_TRANSITION_TRACE_JSONL",
                state_dir.join("live-transitions.jsonl"),
            ),
            event_path: env_path(
                "NANDO_TRANSITION_EXECUTION_EVENTS_JSONL",
                state_dir.join("execution-events.jsonl"),
            ),
            economics_path: env_path(
                "NANDO_TRANSITION_ECONOMICS_JSONL",
                state_dir.join("economics-terminal.jsonl"),
            ),
            inbox_dir: env_path(
                "NANDO_TRANSITION_PACKAGE_INBOX",
                state_dir.join("package-inbox"),
            ),
            registry_path: env_path("NANDO_TRANSITION_REGISTRY", state_dir.join("registry.json")),
            metrics_path: env_path("NANDO_TRANSITION_METRICS", state_dir.join("metrics.json")),
            legacy_named_inducer_enabled: env_bool("NANDO_TRANSITION_LEGACY_NAMED_INDUCER"),
            state_dir,
            once,
        })
    }
}

fn env_path(name: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(fallback)
}

fn env_bool(name: &str) -> bool {
    env::var(name).is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn ingest_package_inbox(registry: &mut LiveProfileRegistry, config: &Config) -> Result<(), String> {
    let mut paths = fs::read_dir(&config.inbox_dir)
        .map_err(|error| format!("inbox_read:{}:{error}", config.inbox_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let key = path.display().to_string();
        if registry.seen_inbox_files.contains(&key) {
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("inbox_package_read:{}:{error}", path.display()))?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("inbox_package_json:{}:{error}", path.display()))?;
        for package in packages_from_value(&value)? {
            import_package(registry, &package, &path, &config.state_dir, false)?;
        }
        registry.seen_inbox_files.insert(key);
    }
    Ok(())
}

fn read_new_traces(
    path: &Path,
    registry: &mut LiveProfileRegistry,
) -> Result<Vec<LiveObservedTransition>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file =
        fs::File::open(path).map_err(|error| format!("trace_open:{}:{error}", path.display()))?;
    let file_len = file
        .metadata()
        .map_err(|error| format!("trace_metadata:{}:{error}", path.display()))?
        .len();
    if registry.trace_watermark_bytes > file_len {
        registry.trace_watermark_bytes = 0;
        registry.trace_watermark_rows = 0;
    }
    let start = registry.trace_watermark_bytes;
    if start == 0 {
        registry.trace_watermark_rows = 0;
    }
    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("trace_seek:{}:{error}", path.display()))?;
    let mut out = Vec::new();
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("trace_read:{}:{error}", path.display()))?;
        if bytes == 0 {
            break;
        }
        registry.trace_watermark_bytes =
            registry.trace_watermark_bytes.saturating_add(bytes as u64);
        registry.trace_watermark_rows = registry.trace_watermark_rows.saturating_add(1);
        let trace: LiveObservedTransition = match serde_json::from_str(&line) {
            Ok(trace) => trace,
            Err(_) => {
                registry.telemetry.traces_invalid =
                    registry.telemetry.traces_invalid.saturating_add(1);
                continue;
            }
        };
        if trace.validate().is_err() || registry.seen_trace_ids.contains(&trace.trace_id) {
            continue;
        }
        registry.seen_trace_ids.insert(trace.trace_id.clone());
        registry.telemetry.traces_seen = registry.telemetry.traces_seen.saturating_add(1);
        out.push(trace);
    }
    Ok(out)
}

fn read_all_valid_traces(path: &Path) -> Result<Vec<LiveObservedTransition>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file =
        fs::File::open(path).map_err(|error| format!("trace_open:{}:{error}", path.display()))?;
    Ok(BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<LiveObservedTransition>(&line).ok())
        .filter(|trace| trace.validate().is_ok() && trace.is_grounded())
        .collect())
}

fn process_future_shadow(
    registry: &mut LiveProfileRegistry,
    traces: &[LiveObservedTransition],
) -> Result<(), String> {
    let package_ids = registry.packages.keys().cloned().collect::<Vec<_>>();
    for trace in traces {
        for package_id in &package_ids {
            let (package_path, allowed, origin, future_evidence_not_before_unix_nanos) = {
                let record = &registry.packages[package_id];
                (
                    PathBuf::from(&record.package_path),
                    record
                        .profiles
                        .iter()
                        .filter(|profile| profile.state != LiveProfileState::Revoked)
                        .map(|profile| profile.transition_index)
                        .collect::<Vec<_>>(),
                    record.origin,
                    record.future_evidence_not_before_unix_nanos,
                )
            };
            if allowed.is_empty() {
                continue;
            }
            let Some(observed_at_unix_nanos) = timestamp_unix_nanos(&trace.timestamp) else {
                continue;
            };
            if observed_at_unix_nanos <= future_evidence_not_before_unix_nanos {
                continue;
            }
            let package = read_package(&package_path)?;
            let started = std::time::Instant::now();
            let execution =
                package.execute_routed_indices(&trace.before, &trace.action, allowed.clone());
            let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
            registry.telemetry.execution_latency_ns.push(elapsed_ns);
            let Some(index) = execution.transition_index else {
                continue;
            };
            let Some(profile) = registry
                .packages
                .get_mut(package_id)
                .and_then(|record| record.profiles.get_mut(index))
            else {
                continue;
            };
            profile.future_rows = profile.future_rows.saturating_add(1);
            if trace.is_grounded() {
                profile.grounded_future_rows = profile.grounded_future_rows.saturating_add(1);
            }
            profile.execution_latency_ns.push(elapsed_ns);
            match execution.status {
                InducedExecutionStatus::Executed => {
                    registry.telemetry.shadow_executions =
                        registry.telemetry.shadow_executions.saturating_add(1);
                    if execution.after.as_ref() == Some(&trace.after) {
                        profile.future_clean_rows = profile.future_clean_rows.saturating_add(1);
                        if trace.is_grounded() {
                            profile.grounded_future_clean_rows =
                                profile.grounded_future_clean_rows.saturating_add(1);
                        }
                        profile.last_reason = "future_shadow_clean".to_owned();
                        maybe_promote_profile(
                            profile,
                            &registry.policy,
                            &trace.trace_id,
                            &mut registry.telemetry.profiles_promoted,
                            origin,
                        );
                    } else {
                        profile.false_accepts = profile.false_accepts.saturating_add(1);
                        profile.negative_memory_rows =
                            profile.negative_memory_rows.saturating_add(1);
                        registry.telemetry.false_accepts =
                            registry.telemetry.false_accepts.saturating_add(1);
                        revoke_or_quarantine(
                            profile,
                            &trace.trace_id,
                            "future_shadow_false_accept",
                            &mut registry.telemetry.profiles_revoked,
                        );
                    }
                }
                InducedExecutionStatus::Abstain | InducedExecutionStatus::VerifyFailed => {
                    profile.abstains = profile.abstains.saturating_add(1);
                    registry.telemetry.shadow_abstains =
                        registry.telemetry.shadow_abstains.saturating_add(1);
                }
            }
        }
    }
    Ok(())
}

fn maybe_promote_profile(
    profile: &mut nando_transition_inducer::LiveRuntimeProfile,
    policy: &nando_transition_inducer::AutonomousPromotionPolicy,
    trace_id: &str,
    promoted_counter: &mut usize,
    origin: LivePackageOrigin,
) {
    let mut latencies = profile.execution_latency_ns.clone();
    latencies.sort_unstable();
    let execution_p99_ns = percentile(&latencies, 99);
    if origin == LivePackageOrigin::RawPhaseInduction
        && profile.state == LiveProfileState::Quarantine
        && policy.auto_promote
        && profile.future_clean_rows >= policy.min_future_clean_rows
        && profile.grounded_future_clean_rows >= policy.min_future_clean_rows
        && profile.false_accepts <= policy.max_false_accepts
        && profile.runtime_parity_mismatches <= policy.max_runtime_parity_mismatches
        && execution_p99_ns <= policy.max_execution_p99_ns
    {
        profile.state = LiveProfileState::Active;
        profile.promoted_at_trace = Some(trace_id.to_owned());
        profile.last_reason = "automatic_future_shadow_promotion".to_owned();
        *promoted_counter = promoted_counter.saturating_add(1);
    }
}

fn revoke_or_quarantine(
    profile: &mut nando_transition_inducer::LiveRuntimeProfile,
    trace_id: &str,
    reason: &str,
    revoked_counter: &mut usize,
) {
    if profile.state == LiveProfileState::Active {
        profile.state = LiveProfileState::Revoked;
        profile.revoked_at_trace = Some(trace_id.to_owned());
        *revoked_counter = revoked_counter.saturating_add(1);
    } else {
        profile.state = LiveProfileState::Quarantine;
        profile.future_clean_rows = 0;
    }
    profile.last_reason = reason.to_owned();
}

fn induce_legacy_named_classes(
    registry: &mut LiveProfileRegistry,
    traces: &[LiveObservedTransition],
    config: &Config,
) -> Result<(), String> {
    let mut classes: BTreeMap<String, Vec<TransitionTrace>> = BTreeMap::new();
    for trace in traces {
        classes
            .entry(structural_class_key(trace))
            .or_default()
            .push(TransitionTrace {
                before: trace.before.clone(),
                action: trace.action.clone(),
                after: trace.after.clone(),
            });
    }
    for (class_key, mut support) in classes {
        if registry.induced_class_keys.contains(&class_key) || support.len() < MIN_LIVE_SUPPORT_ROWS
        {
            continue;
        }
        if support.len() > MAX_LIVE_SUPPORT_ROWS {
            support = support.split_off(support.len() - MAX_LIVE_SUPPORT_ROWS);
        }
        let mut inducer = match TransitionInducer::train(&[support.clone()], 16, 0.90, 16) {
            Ok(inducer) => inducer,
            Err(_) => continue,
        };
        let (package, _) = match inducer.induce(&support) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let source = PathBuf::from(format!("live-class-{class_key}"));
        import_package_with_origin(
            registry,
            &package,
            &source,
            &config.state_dir,
            LivePackageOrigin::LegacyNamedInduction,
        )?;
        registry.induced_class_keys.insert(class_key);
    }
    Ok(())
}

fn structural_class_key(trace: &LiveObservedTransition) -> String {
    transition_surface_key(&trace.before, &trace.action).unwrap_or_else(|_| {
        let mut signature = String::new();
        append_shape(&trace.before, &mut signature);
        signature.push('|');
        append_shape(&trace.action, &mut signature);
        format!("{:016x}", stable_hash(signature.as_bytes()))
    })
}

fn append_shape(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push('0'),
        Value::Bool(_) => output.push('b'),
        Value::Number(_) => output.push('n'),
        Value::String(_) => output.push('s'),
        Value::Array(values) => {
            output.push('[');
            if let Some(first) = values.first() {
                append_shape(first, output);
            }
            output.push(']');
        }
        Value::Object(object) => {
            output.push('{');
            for (key, child) in object {
                output.push_str(key);
                output.push(':');
                append_shape(child, output);
                output.push(',');
            }
            output.push('}');
        }
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |state, byte| {
        (state ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn ingest_bridge_events(path: &Path, registry: &mut LiveProfileRegistry) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut file =
        fs::File::open(path).map_err(|error| format!("event_open:{}:{error}", path.display()))?;
    let file_len = file
        .metadata()
        .map_err(|error| format!("event_metadata:{}:{error}", path.display()))?
        .len();
    if registry.execution_event_watermark_bytes > file_len {
        registry.execution_event_watermark_bytes = 0;
        registry.seen_bridge_request_ids.clear();
        registry.seen_local_accept_request_ids.clear();
    }
    if registry.execution_event_watermark_bytes == 0 && registry.seen_bridge_request_ids.is_empty()
    {
        registry.telemetry.total_bridge_requests = 0;
        registry.telemetry.total_bridge_tokens = 0;
        registry.telemetry.active_local_accepts = 0;
        registry.telemetry.llm_calls_avoided = 0;
        registry.telemetry.tokens_saved = 0;
    }
    file.seek(SeekFrom::Start(registry.execution_event_watermark_bytes))
        .map_err(|error| format!("event_seek:{}:{error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("event_read:{}:{error}", path.display()))?;
        if bytes == 0 {
            break;
        }
        registry.execution_event_watermark_bytes = registry
            .execution_event_watermark_bytes
            .saturating_add(bytes as u64);
        let Ok(event) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let event_kind = event.get("event").and_then(Value::as_str);
        let Some(request_id) = event.get("request_sha256").and_then(Value::as_str) else {
            continue;
        };
        let tokens = event.get("tokens").and_then(Value::as_u64).unwrap_or(0);
        if matches!(event_kind, Some("bridge_request" | "transition_request"))
            && registry
                .seen_bridge_request_ids
                .insert(request_id.to_owned())
        {
            registry.telemetry.total_bridge_requests =
                registry.telemetry.total_bridge_requests.saturating_add(1);
            registry.telemetry.total_bridge_tokens = registry
                .telemetry
                .total_bridge_tokens
                .saturating_add(tokens);
        }
    }
    Ok(())
}

fn reconcile_verified_economics(
    path: &Path,
    registry: &mut LiveProfileRegistry,
) -> Result<(), String> {
    let mut request_ids = std::collections::BTreeSet::new();
    let mut avoided_calls = 0usize;
    let mut tokens_saved = 0u64;
    if path.exists() {
        let file = fs::File::open(path)
            .map_err(|error| format!("economics_open:{}:{error}", path.display()))?;
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(row) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let request_id = row
                .get("request_sha256")
                .and_then(Value::as_str)
                .unwrap_or("");
            let verification_receipt = row
                .get("verification_receipt_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let projector_receipt = row
                .get("projector_receipt_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let verified_after_digest = row
                .get("verified_after_digest")
                .and_then(Value::as_str)
                .unwrap_or("");
            let verified_local_delivery = row.get("schema").and_then(Value::as_str)
                == Some("nando.economics-terminal.v1")
                && row.get("route").and_then(Value::as_str) == Some("local_actor")
                && row.get("terminal_state").and_then(Value::as_str)
                    == Some("delivered_by_local_worker")
                && row.get("avoided_call").and_then(Value::as_bool) == Some(true)
                && row.get("upstream_socket_opened").and_then(Value::as_bool) == Some(false)
                && row.get("verification_status").and_then(Value::as_str) == Some("verified")
                && row.get("verifier_schema").and_then(Value::as_str)
                    == Some("typed_actor_independent_verifier.v1")
                && valid_sha256(request_id)
                && valid_sha256(verification_receipt)
                && valid_sha256(projector_receipt)
                && valid_sha256(verified_after_digest);
            if verified_local_delivery && request_ids.insert(request_id.to_owned()) {
                avoided_calls = avoided_calls.saturating_add(1);
                tokens_saved = tokens_saved
                    .saturating_add(row.get("input_tokens").and_then(Value::as_u64).unwrap_or(0));
            }
        }
    }
    registry.seen_local_accept_request_ids = request_ids;
    registry.telemetry.active_local_accepts = avoided_calls;
    registry.telemetry.llm_calls_avoided = avoided_calls;
    registry.telemetry.tokens_saved = tokens_saved;
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn write_metrics(registry: &LiveProfileRegistry, config: &Config) -> Result<(), String> {
    let mut latencies = registry.telemetry.execution_latency_ns.clone();
    latencies.sort_unstable();
    let profile_count = registry
        .packages
        .values()
        .map(|package| package.profiles.len())
        .sum();
    let raw_phase_memorization_families = registry
        .raw_phase_families
        .values()
        .filter(|family| family.stage == "memorization")
        .count();
    let raw_phase_circuit_families = registry
        .raw_phase_families
        .values()
        .filter(|family| family.stage == "circuit_formation")
        .count();
    let raw_phase_cleanup_families = registry
        .raw_phase_families
        .values()
        .filter(|family| family.stage == "cleanup")
        .count();
    let snapshot = MetricsSnapshot {
        report_kind: "nando_autonomous_transition_profiles_live_v1",
        verdict: if registry.active_profile_count() > 0 {
            "ACTIVE_GUARDED_CPU_EXECUTION"
        } else {
            "SHADOW_QUARANTINE"
        },
        policy_version: registry.policy.version.clone(),
        registry_revision: registry.revision,
        package_count: registry.packages.len(),
        profile_count,
        active_profile_count: registry.active_profile_count(),
        non_raw_active_profile_count: registry.non_raw_active_profile_count(),
        quarantined_profile_count: registry.quarantined_profile_count(),
        revoked_profile_count: registry.revoked_profile_count(),
        traces_seen: registry.telemetry.traces_seen,
        shadow_executions: registry.telemetry.shadow_executions,
        shadow_abstains: registry.telemetry.shadow_abstains,
        false_accepts: registry.telemetry.false_accepts,
        runtime_parity_mismatches: registry.telemetry.runtime_parity_mismatches,
        profiles_promoted: registry.telemetry.profiles_promoted,
        profiles_revoked: registry.telemetry.profiles_revoked,
        local_accepts: registry.telemetry.active_local_accepts,
        llm_calls_avoided: registry.telemetry.llm_calls_avoided,
        tokens_saved: registry.telemetry.tokens_saved,
        total_bridge_requests: registry.telemetry.total_bridge_requests,
        total_bridge_tokens: registry.telemetry.total_bridge_tokens,
        traffic_share_milli: ratio_milli(
            registry.telemetry.active_local_accepts,
            registry.telemetry.total_bridge_requests,
        ),
        token_savings_share_milli: ratio_milli_u64(
            registry.telemetry.tokens_saved,
            registry.telemetry.total_bridge_tokens,
        ),
        execution_p99_ns: percentile(&latencies, 99),
        raw_phase_enabled: true,
        legacy_named_inducer_enabled: config.legacy_named_inducer_enabled,
        raw_phase_family_count: registry.raw_phase_families.len(),
        raw_phase_memorization_families,
        raw_phase_circuit_families,
        raw_phase_cleanup_families,
        raw_phase_max_observed_surfaces: registry
            .raw_phase_families
            .values()
            .map(|family| family.observed_surfaces)
            .max()
            .unwrap_or(0),
        raw_phase_total_observed_surfaces: registry
            .raw_phase_families
            .values()
            .map(|family| family.surface_frontier.len())
            .sum(),
        raw_phase_total_covered_surfaces: registry
            .raw_phase_families
            .values()
            .flat_map(|family| family.surface_frontier.values())
            .filter(|surface| surface.circuit_covered)
            .count(),
        raw_phase_total_unsupported_surfaces: registry
            .raw_phase_families
            .values()
            .flat_map(|family| family.surface_frontier.values())
            .filter(|surface| !surface.circuit_covered)
            .count(),
        raw_phase_frontier_observed_rows: registry
            .raw_phase_families
            .values()
            .map(|family| family.frontier_observed_rows)
            .sum(),
        raw_phase_frontier_covered_rows: registry
            .raw_phase_families
            .values()
            .map(|family| family.frontier_covered_rows)
            .sum(),
        raw_phase_frontier_observed_tokens: registry
            .raw_phase_families
            .values()
            .map(|family| family.frontier_observed_tokens)
            .sum(),
        raw_phase_frontier_covered_tokens: registry
            .raw_phase_families
            .values()
            .map(|family| family.frontier_covered_tokens)
            .sum(),
        raw_phase_frontier_token_coverage_milli: ratio_milli_u64(
            registry
                .raw_phase_families
                .values()
                .map(|family| family.frontier_covered_tokens)
                .sum(),
            registry
                .raw_phase_families
                .values()
                .map(|family| family.frontier_observed_tokens)
                .sum(),
        ),
        raw_phase_max_surface_sessions: registry
            .raw_phase_families
            .values()
            .flat_map(|family| family.surface_frontier.values())
            .map(|surface| surface.session_count)
            .max()
            .unwrap_or(0),
        raw_phase_transfer_pass_families: registry
            .raw_phase_families
            .values()
            .filter(|family| family.leave_one_surface_out_pass)
            .count(),
        raw_phase_transfer_tested_surfaces: registry
            .raw_phase_families
            .values()
            .map(|family| family.transfer_tested_surfaces)
            .sum(),
        raw_phase_transfer_passed_surfaces: registry
            .raw_phase_families
            .values()
            .map(|family| family.transfer_passed_surfaces)
            .sum(),
        raw_phase_transfer_query_rows: registry
            .raw_phase_families
            .values()
            .map(|family| family.transfer_query_rows)
            .sum(),
        raw_phase_transfer_correct_executions: registry
            .raw_phase_families
            .values()
            .map(|family| family.transfer_correct_executions)
            .sum(),
        raw_phase_transfer_abstains: registry
            .raw_phase_families
            .values()
            .map(|family| family.transfer_abstains)
            .sum(),
        raw_phase_transfer_wrong_accepts: registry
            .raw_phase_families
            .values()
            .map(|family| family.transfer_wrong_accepts)
            .sum(),
        raw_phase_session_transfer_pass_families: registry
            .raw_phase_families
            .values()
            .filter(|family| family.new_session_split_pass)
            .count(),
        raw_phase_session_transfer_query_rows: registry
            .raw_phase_families
            .values()
            .map(|family| family.session_transfer_query_rows)
            .sum(),
        raw_phase_session_transfer_correct_executions: registry
            .raw_phase_families
            .values()
            .map(|family| family.session_transfer_correct_executions)
            .sum(),
        raw_phase_session_transfer_abstains: registry
            .raw_phase_families
            .values()
            .map(|family| family.session_transfer_abstains)
            .sum(),
        raw_phase_session_transfer_wrong_accepts: registry
            .raw_phase_families
            .values()
            .map(|family| family.session_transfer_wrong_accepts)
            .sum(),
        raw_phase_time_transfer_pass_families: registry
            .raw_phase_families
            .values()
            .filter(|family| family.forward_time_split_pass)
            .count(),
        raw_phase_time_transfer_query_rows: registry
            .raw_phase_families
            .values()
            .map(|family| family.time_transfer_query_rows)
            .sum(),
        raw_phase_time_transfer_correct_executions: registry
            .raw_phase_families
            .values()
            .map(|family| family.time_transfer_correct_executions)
            .sum(),
        raw_phase_time_transfer_abstains: registry
            .raw_phase_families
            .values()
            .map(|family| family.time_transfer_abstains)
            .sum(),
        raw_phase_time_transfer_wrong_accepts: registry
            .raw_phase_families
            .values()
            .map(|family| family.time_transfer_wrong_accepts)
            .sum(),
        raw_phase_max_discovered_predicates: registry
            .raw_phase_families
            .values()
            .map(|family| family.discovered_predicates)
            .max()
            .unwrap_or(0),
        raw_phase_training_attempts: registry.telemetry.raw_phase_training_attempts,
        raw_phase_packages_induced: registry.telemetry.raw_phase_packages_induced,
        raw_phase_training_cpu_ns: registry.telemetry.raw_phase_training_cpu_ns,
        raw_phase_induction_cpu_ns: registry.telemetry.raw_phase_induction_cpu_ns,
        boundary: "live server metrics; savings count only active verified local accepts; no active profile means zero savings",
    };
    atomic_write_json(&config.metrics_path, &snapshot)
}

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    numerator
        .saturating_mul(1_000)
        .checked_div(denominator)
        .unwrap_or(0)
}

fn ratio_milli_u64(numerator: u64, denominator: u64) -> usize {
    usize::try_from(
        numerator
            .saturating_mul(1_000)
            .checked_div(denominator)
            .unwrap_or(0),
    )
    .unwrap_or(usize::MAX)
}

fn percentile(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = sorted
        .len()
        .saturating_mul(percentile)
        .div_ceil(100)
        .saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn economics_requires_verified_terminal_delivery_and_deduplicates() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/nando-wave/economics-reconcile-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("economics root");
        let path = root.join("economics.jsonl");
        let digest = "a".repeat(64);
        let valid = serde_json::json!({
            "schema": "nando.economics-terminal.v1",
            "request_sha256": digest,
            "route": "local_actor",
            "terminal_state": "delivered_by_local_worker",
            "input_tokens": 11,
            "upstream_socket_opened": false,
            "avoided_call": true,
            "verification_status": "verified",
            "verification_receipt_id": digest,
            "projector_receipt_id": digest,
            "verified_after_digest": digest,
            "verifier_schema": "typed_actor_independent_verifier.v1"
        });
        let invalid = serde_json::json!({
            "schema": "nando.economics-terminal.v1",
            "request_sha256": "b".repeat(64),
            "route": "local_actor",
            "terminal_state": "delivered_by_local_worker",
            "input_tokens": 99,
            "upstream_socket_opened": true,
            "avoided_call": true
        });
        fs::write(&path, format!("{}\n{}\n{}\n", valid, valid, invalid)).expect("economics write");

        let mut registry = LiveProfileRegistry::default();
        registry.telemetry.active_local_accepts = 8;
        registry.telemetry.llm_calls_avoided = 8;
        registry.telemetry.tokens_saved = 999;
        reconcile_verified_economics(&path, &mut registry).expect("economics reconcile");

        assert_eq!(registry.telemetry.active_local_accepts, 1);
        assert_eq!(registry.telemetry.llm_calls_avoided, 1);
        assert_eq!(registry.telemetry.tokens_saved, 11);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn idle_cycle_does_not_rewrite_registry() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/nando-wave/idle-registry-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let inbox_dir = root.join("package-inbox");
        fs::create_dir_all(&inbox_dir).expect("idle inbox");
        let config = Config {
            state_dir: root.clone(),
            trace_path: root.join("traces.jsonl"),
            event_path: root.join("events.jsonl"),
            economics_path: root.join("economics.jsonl"),
            inbox_dir,
            registry_path: root.join("registry.json"),
            metrics_path: root.join("metrics.json"),
            legacy_named_inducer_enabled: false,
            once: true,
        };

        run_cycle(&config).expect("first cycle");
        let first = LiveProfileRegistry::load(&config.registry_path).expect("first registry");
        run_cycle(&config).expect("second cycle");
        let second = LiveProfileRegistry::load(&config.registry_path).expect("second registry");

        assert_eq!(first.revision, 1);
        assert_eq!(second.revision, first.revision);
        let _ = fs::remove_dir_all(root);
    }
}
