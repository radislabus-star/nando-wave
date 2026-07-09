use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_LIVE_NP_CHAIN_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-live-np-chain-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_LIVE_NP_CHAIN_PREFIX: &str =
    "target/nando-wave/streaming/provider-boundary-live-np-chain-v1";

struct LiveNpChainPaths {
    append_sink_report: PathBuf,
    append_provider_boundary_jsonl: PathBuf,
    coverage_report: PathBuf,
    np_chain_report: PathBuf,
    np_chain_prefix: PathBuf,
}

pub(crate) fn run_phase_stream_provider_boundary_live_np_chain_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_LIVE_NP_CHAIN_REPORT));
    let artifact_prefix = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_LIVE_NP_CHAIN_PREFIX));
    let provider_export_arg = args.next().unwrap_or_else(|| "-".to_owned());
    let capture_request_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;
    let phase_trace_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "score-ready phase-atom trace JSONL path is required".to_owned())?;

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
            return Err(format!(
                "unexpected argument before --provider-events: {arg}"
            ));
        }
    }
    if provider_event_paths.is_empty() {
        return Err(
            "at least one provider event JSONL path or '-' is required after --provider-events"
                .to_owned(),
        );
    }

    let paths = LiveNpChainPaths::from_prefix(&artifact_prefix);
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

    super::run_phase_stream_provider_boundary_np_chain_from_phase_trace_v1(
        vec![
            path_string(&paths.np_chain_report),
            path_string(&paths.np_chain_prefix),
            provider_export_arg.clone(),
            path_string(&phase_trace_path),
            path_string(&paths.append_provider_boundary_jsonl),
        ]
        .into_iter(),
    )?;
    executed_steps.push("provider_boundary_np_chain_from_phase_trace");

    let append = read_json(&paths.append_sink_report)?;
    let coverage = read_json(&paths.coverage_report)?;
    let np_chain = read_json(&paths.np_chain_report)?;

    let appended_rows = json_usize(&append, &["appended_rows"]).unwrap_or(0);
    let covered_capture_requests =
        json_usize(&coverage, &["capture_requests", "covered_capture_requests"]).unwrap_or(0);
    let missing_capture_requests =
        json_usize(&coverage, &["capture_requests", "missing_capture_requests"]).unwrap_or(0);
    let covered_tokens =
        json_usize(&coverage, &["capture_requests", "covered_tokens"]).unwrap_or(0);
    let missing_tokens =
        json_usize(&coverage, &["capture_requests", "missing_tokens"]).unwrap_or(0);
    let provider_key_atom_leak = json_usize(&append, &["skipped_provider_key_atom_leak"])
        .unwrap_or(0)
        .saturating_add(
            json_usize(&coverage, &["provider", "provider_key_atom_leak_rows"]).unwrap_or(0),
        )
        .saturating_add(
            json_usize(
                &np_chain,
                &["provider_correlation", "rows_with_provider_key_atom_leak"],
            )
            .unwrap_or(0),
        );
    let selected_subcenter_count =
        json_usize(&np_chain, &["np_rescue", "selected_subcenter_count"]).unwrap_or(0);
    let portfolio_accepts = json_usize(
        &np_chain,
        &[
            "np_runtime",
            "portfolio_unique_cpu_accepts_over_exact_cache",
        ],
    )
    .unwrap_or(0);
    let portfolio_tokens =
        json_usize(&np_chain, &["np_runtime", "portfolio_tokens_saved"]).unwrap_or(0);
    let product_ready = json_bool(&np_chain, &["product_ready"]).unwrap_or(false);
    let np_chain_market_money_claim_allowed =
        json_bool(&np_chain, &["market_money_claim_allowed"]).unwrap_or(false);
    let full_capture_coverage =
        json_bool(&coverage, &["readiness", "full_capture_coverage"]).unwrap_or(false);

    let mut blockers = Vec::<&'static str>::new();
    if appended_rows == 0 {
        blockers.push("append_sink_no_rows");
    }
    if covered_capture_requests == 0 {
        blockers.push("coverage_gate_no_covered_capture_requests");
    }
    if missing_capture_requests > 0 {
        blockers.push("coverage_gate_still_missing_capture_requests");
    }
    if provider_key_atom_leak > 0 {
        blockers.push("provider_key_atom_leak");
    }
    if selected_subcenter_count == 0 {
        blockers.push("np_chain_no_selected_subcenters");
    }
    if portfolio_accepts == 0 {
        blockers.push("np_chain_no_unique_cpu_accepts_over_exact_cache");
    }
    if !product_ready {
        blockers.push("np_chain_not_product_ready");
    }
    if np_chain_market_money_claim_allowed {
        blockers.push("np_chain_attempted_market_money_claim");
    }

    let verdict = if provider_key_atom_leak > 0 || np_chain_market_money_claim_allowed {
        "PHASE_STREAM_PROVIDER_BOUNDARY_LIVE_NP_CHAIN_V1_FAIL_FORBIDDEN_EVIDENCE_BOUNDARY"
    } else if full_capture_coverage && product_ready && portfolio_accepts > 0 {
        "PHASE_STREAM_PROVIDER_BOUNDARY_LIVE_NP_CHAIN_V1_PASS_PROVIDER_CAPTURED_NP_EVIDENCE"
    } else if appended_rows > 0 && covered_capture_requests > 0 {
        "PHASE_STREAM_PROVIDER_BOUNDARY_LIVE_NP_CHAIN_V1_WATCH_PARTIAL_PROVIDER_CAPTURE"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_LIVE_NP_CHAIN_V1_WATCH_NO_PROVIDER_CAPTURE"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_live_np_chain_v1",
        "artifact_prefix": path_string(&artifact_prefix),
        "provider_export_path": provider_export_arg,
        "capture_request_path": path_string(&capture_request_path),
        "phase_trace_path": path_string(&phase_trace_path),
        "provider_event_paths": provider_event_paths,
        "executed_steps": executed_steps,
        "artifact_paths": {
            "append_sink_report": paths.append_sink_report,
            "append_provider_boundary_jsonl": paths.append_provider_boundary_jsonl,
            "coverage_report": paths.coverage_report,
            "np_chain_report": paths.np_chain_report,
            "np_chain_prefix": paths.np_chain_prefix
        },
        "scoreboard": {
            "appended_rows": appended_rows,
            "covered_capture_requests": covered_capture_requests,
            "missing_capture_requests": missing_capture_requests,
            "covered_tokens": covered_tokens,
            "missing_tokens": missing_tokens,
            "provider_key_atom_leak_rows": provider_key_atom_leak,
            "selected_subcenter_count": selected_subcenter_count,
            "portfolio_unique_cpu_accepts_over_exact_cache": portfolio_accepts,
            "portfolio_tokens_saved": portfolio_tokens,
            "full_capture_coverage": full_capture_coverage,
            "np_chain_product_ready": product_ready
        },
        "append_sink": summarize(&append, &[
            "verdict",
            "input_rows",
            "appended_rows",
            "skipped_rows",
            "skipped_provider_key_atom_leak"
        ]),
        "coverage": summarize(&coverage, &[
            "verdict",
            "capture_requests",
            "tokens",
            "readiness",
            "provider"
        ]),
        "np_chain": summarize(&np_chain, &[
            "verdict",
            "stopped_after",
            "np_rescue",
            "np_runtime",
            "provider_correlation",
            "billing_evidence_gate",
            "admission",
            "evidence_chain"
        ]),
        "readiness": {
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "product_promotion_allowed": false,
            "policy": "live provider event to append sink, capture coverage, then cold NP evidence chain; no serving, no promotion, no local accept, no product money claim"
        },
        "blockers": blockers,
        "manual_class_list_used": false,
        "selector_used_as_manual_authority": false,
        "dynamic_discovery_performed": true,
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
        "boundary": "cold evidence-chain orchestration only: does not intercept all traffic, does not serve, does not promote, and does not local-accept"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_live_np_chain_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  append_provider_boundary_jsonl: {}",
        paths.append_provider_boundary_jsonl.display()
    );
    println!("  appended_rows: {appended_rows}");
    println!("  covered_capture_requests: {covered_capture_requests}");
    println!("  selected_subcenter_count: {selected_subcenter_count}");
    println!("  portfolio_unique_cpu_accepts_over_exact_cache: {portfolio_accepts}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

impl LiveNpChainPaths {
    fn from_prefix(prefix: &Path) -> Self {
        Self {
            append_sink_report: suffixed(prefix, ".append-sink.report.json"),
            append_provider_boundary_jsonl: suffixed(prefix, ".provider.jsonl"),
            coverage_report: suffixed(prefix, ".coverage.report.json"),
            np_chain_report: suffixed(prefix, ".np-chain.report.json"),
            np_chain_prefix: suffixed(prefix, ".np-chain"),
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

fn summarize(value: &Value, keys: &[&str]) -> Value {
    let mut out = serde_json::Map::new();
    out.insert("present".to_owned(), serde_json::json!(true));
    for key in keys {
        out.insert(
            (*key).to_owned(),
            value.get(*key).cloned().unwrap_or(serde_json::Value::Null),
        );
    }
    out.into()
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path)?.as_bool()
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)?
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}
