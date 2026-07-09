use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_LIVE_CHAIN_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-live-chain-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_LIVE_CHAIN_PREFIX: &str =
    "target/nando-wave/streaming/provider-boundary-live-chain-v1";

struct LiveChainPaths {
    append_sink_report: PathBuf,
    append_provider_boundary_jsonl: PathBuf,
    coverage_report: PathBuf,
    match_readiness_report: PathBuf,
}

pub(crate) fn run_phase_stream_provider_boundary_live_chain_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_LIVE_CHAIN_REPORT));
    let artifact_prefix = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_LIVE_CHAIN_PREFIX));
    let capture_request_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;

    let mut phase_paths = Vec::<PathBuf>::new();
    let mut provider_event_paths = Vec::<String>::new();
    let mut provider_mode = false;
    for arg in args {
        if arg == "--provider-events" {
            provider_mode = true;
            continue;
        }
        if provider_mode {
            provider_event_paths.push(arg);
        } else {
            phase_paths.push(PathBuf::from(arg));
        }
    }
    if phase_paths.is_empty() {
        return Err(
            "at least one phase-atom trace JSONL path is required before --provider-events"
                .to_owned(),
        );
    }
    if provider_event_paths.is_empty() {
        return Err(
            "at least one provider event JSONL path or '-' is required after --provider-events"
                .to_owned(),
        );
    }

    let paths = LiveChainPaths::from_prefix(&artifact_prefix);
    let mut executed_steps = Vec::<&'static str>::new();

    let mut append_args = vec![
        path_string(&paths.append_sink_report),
        path_string(&paths.append_provider_boundary_jsonl),
    ];
    append_args.extend(provider_event_paths.iter().cloned());
    super::run_phase_stream_provider_boundary_append_sink_v1(append_args.into_iter())?;
    executed_steps.push("provider_boundary_append_sink");

    super::run_phase_stream_provider_boundary_capture_coverage_gate_v1(
        vec![
            path_string(&paths.coverage_report),
            path_string(&capture_request_path),
            "--provider".to_owned(),
            path_string(&paths.append_provider_boundary_jsonl),
        ]
        .into_iter(),
    )?;
    executed_steps.push("provider_boundary_capture_coverage_gate");

    let mut match_args = vec![path_string(&paths.match_readiness_report)];
    match_args.extend(phase_paths.iter().map(|path| path_string(path)));
    match_args.push("--provider".to_owned());
    match_args.push(path_string(&paths.append_provider_boundary_jsonl));
    super::run_phase_stream_provider_boundary_match_readiness_v1(match_args.into_iter())?;
    executed_steps.push("provider_boundary_match_readiness");

    let append = read_json(&paths.append_sink_report)?;
    let coverage = read_json(&paths.coverage_report)?;
    let readiness = read_json(&paths.match_readiness_report)?;

    let appended_rows = json_usize(&append, &["appended_rows"]).unwrap_or(0);
    let covered_capture_requests =
        json_usize(&coverage, &["capture_requests", "covered_capture_requests"]).unwrap_or(0);
    let missing_capture_requests =
        json_usize(&coverage, &["capture_requests", "missing_capture_requests"]).unwrap_or(0);
    let score_ready_rows_with_provider_join = json_usize(
        &readiness,
        &["phase", "score_ready_rows_with_provider_join"],
    )
    .unwrap_or(0);
    let score_ready_rows_missing_provider_join = json_usize(
        &readiness,
        &["phase", "score_ready_rows_missing_provider_join"],
    )
    .unwrap_or(0);
    let provider_key_atom_leak = json_usize(&append, &["skipped_provider_key_atom_leak"])
        .unwrap_or(0)
        .saturating_add(
            json_usize(&coverage, &["provider", "provider_key_atom_leak_rows"]).unwrap_or(0),
        )
        .saturating_add(
            json_usize(&readiness, &["provider", "provider_key_atom_leak_rows"]).unwrap_or(0),
        );
    let chain_provider_capture_observed = appended_rows > 0
        && covered_capture_requests > 0
        && score_ready_rows_with_provider_join > 0
        && provider_key_atom_leak == 0;
    let full_capture_coverage =
        json_bool(&coverage, &["readiness", "full_capture_coverage"]).unwrap_or(false);
    let mut blockers = Vec::<&'static str>::new();
    if appended_rows == 0 {
        blockers.push("append_sink_no_rows");
    }
    if covered_capture_requests == 0 {
        blockers.push("coverage_gate_no_covered_capture_requests");
    }
    if score_ready_rows_with_provider_join == 0 {
        blockers.push("match_readiness_no_provider_join");
    }
    if missing_capture_requests > 0 {
        blockers.push("coverage_gate_still_missing_capture_requests");
    }
    if score_ready_rows_missing_provider_join > 0 {
        blockers.push("match_readiness_still_missing_provider_join");
    }
    if provider_key_atom_leak > 0 {
        blockers.push("provider_key_atom_leak");
    }

    let verdict = if provider_key_atom_leak > 0 {
        "PHASE_STREAM_PROVIDER_BOUNDARY_LIVE_CHAIN_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if full_capture_coverage && chain_provider_capture_observed {
        "PHASE_STREAM_PROVIDER_BOUNDARY_LIVE_CHAIN_V1_PASS_FULL_CAPTURE_COVERAGE"
    } else if chain_provider_capture_observed {
        "PHASE_STREAM_PROVIDER_BOUNDARY_LIVE_CHAIN_V1_WATCH_PARTIAL_PROVIDER_CAPTURE"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_LIVE_CHAIN_V1_WATCH_NO_PROVIDER_CAPTURE"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_live_chain_v1",
        "artifact_prefix": path_string(&artifact_prefix),
        "capture_request_path": path_string(&capture_request_path),
        "phase_trace_paths": phase_paths.iter().map(|path| path_string(path)).collect::<Vec<_>>(),
        "provider_event_paths": provider_event_paths,
        "executed_steps": executed_steps,
        "artifacts": {
            "append_sink_report": paths.append_sink_report,
            "append_provider_boundary_jsonl": paths.append_provider_boundary_jsonl,
            "coverage_report": paths.coverage_report,
            "match_readiness_report": paths.match_readiness_report
        },
        "scoreboard": {
            "appended_rows": appended_rows,
            "covered_capture_requests": covered_capture_requests,
            "missing_capture_requests": missing_capture_requests,
            "score_ready_rows_with_provider_join": score_ready_rows_with_provider_join,
            "score_ready_rows_missing_provider_join": score_ready_rows_missing_provider_join,
            "provider_key_atom_leak_rows": provider_key_atom_leak,
            "chain_provider_capture_observed": chain_provider_capture_observed,
            "full_capture_coverage": full_capture_coverage
        },
        "readiness": {
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "product_promotion_allowed": false,
            "policy": "provider-boundary live chain only: append provider metadata, check capture coverage, and check phase trace match-readiness; no mining, scoring, .nwpc compile, serving, promotion, local accept, or money claim"
        },
        "blockers": blockers,
        "manual_class_list_used": false,
        "selector_used": false,
        "dynamic_discovery_performed": false,
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
        "boundary": "orchestration evidence chain only: does not intercept all traffic, does not serve requests, does not local-accept, and does not claim savings"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_live_chain_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  append_provider_boundary_jsonl: {}",
        paths.append_provider_boundary_jsonl.display()
    );
    println!("  appended_rows: {appended_rows}");
    println!("  covered_capture_requests: {covered_capture_requests}");
    println!("  score_ready_rows_with_provider_join: {score_ready_rows_with_provider_join}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

impl LiveChainPaths {
    fn from_prefix(prefix: &Path) -> Self {
        Self {
            append_sink_report: suffixed(prefix, ".append-sink.report.json"),
            append_provider_boundary_jsonl: suffixed(prefix, ".provider.jsonl"),
            coverage_report: suffixed(prefix, ".coverage.report.json"),
            match_readiness_report: suffixed(prefix, ".match-readiness.report.json"),
        }
    }
}

fn suffixed(prefix: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{}", prefix.display(), suffix))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSON report '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON report '{}': {error}", path.display()))
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
