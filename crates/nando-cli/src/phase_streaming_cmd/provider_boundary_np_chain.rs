use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_NP_CHAIN_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-np-chain-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_NP_CHAIN_PREFIX: &str =
    "target/nando-wave/streaming/provider-boundary-np-chain-v1";

const CHAIN_CELLS: &str = "32";
const CHAIN_MIN_BUCKET_EVENTS: &str = "2";
const CHAIN_BASE_MARGIN_FLOOR_MICRO: &str = "1";
const CHAIN_COMPILE_EVERY_ROWS: &str = "256";
const CHAIN_MAX_ACTIVE_BUCKETS: &str = "64";
const CHAIN_RESERVOIR_PER_LABEL: &str = "1200";
const CHAIN_MAX_SELECTED_BUCKETS: &str = "64";
const CHAIN_MAX_SELECTED_SUBCENTERS: &str = "64";

#[derive(Clone, Debug)]
struct ProviderBoundaryNpChainPaths {
    phase_trace: PathBuf,
    capture_report: PathBuf,
    joined_trace: PathBuf,
    join_report: PathBuf,
    readiness_report: PathBuf,
    miner_report: PathBuf,
    checkpoint_dir: PathBuf,
    decision_log: PathBuf,
    selector_report: PathBuf,
    np_rescue_report: PathBuf,
    np_runtime_report: PathBuf,
    provider_correlation_audit_report: PathBuf,
    provider_export_empty: PathBuf,
    billing_contract_report: PathBuf,
    billing_template_jsonl: PathBuf,
    provider_normalize_report: PathBuf,
    provider_normalized_jsonl: PathBuf,
    billing_evidence_gate_report: PathBuf,
    billing_missing_jsonl: PathBuf,
    admission_report: PathBuf,
    promotion_report: PathBuf,
    evidence_chain_report: PathBuf,
}

pub(crate) fn run_phase_stream_provider_boundary_np_chain_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_NP_CHAIN_REPORT));
    let artifact_prefix = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_NP_CHAIN_PREFIX));
    let provider_export_arg = args.next().unwrap_or_else(|| "-".to_owned());
    let provider_boundary_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if provider_boundary_paths.is_empty() {
        return Err("at least one provider-boundary event JSONL path is required".to_owned());
    }

    let paths = ProviderBoundaryNpChainPaths::from_prefix(&artifact_prefix);
    let provider_export_path = if provider_export_arg == "-" {
        write_empty_file(&paths.provider_export_empty)?;
        paths.provider_export_empty.clone()
    } else {
        PathBuf::from(provider_export_arg)
    };

    let mut executed_steps = Vec::<&'static str>::new();

    let mut capture_args = vec![
        path_string(&paths.capture_report),
        path_string(&paths.phase_trace),
    ];
    capture_args.extend(provider_boundary_paths.iter().map(path_string));
    super::run_phase_stream_provider_boundary_phase_atom_trace_v1(capture_args.into_iter())?;
    executed_steps.push("provider_boundary_phase_atom_capture");

    let mut join_args = vec![
        path_string(&paths.join_report),
        path_string(&paths.joined_trace),
    ];
    join_args.push(path_string(&paths.phase_trace));
    join_args.extend(provider_boundary_paths.iter().map(path_string));
    super::run_phase_stream_provider_boundary_correlation_join_v1(join_args.into_iter())?;
    executed_steps.push("provider_boundary_correlation_join");

    run_chain_after_join(
        &report_path,
        &paths,
        &provider_boundary_paths,
        &provider_export_path,
        &mut executed_steps,
    )
}

pub(crate) fn run_phase_stream_provider_boundary_np_chain_from_phase_trace_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                "target/nando-wave/streaming/provider-boundary-np-chain-from-phase-trace-v1.report.json",
            )
        });
    let artifact_prefix = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("target/nando-wave/streaming/provider-boundary-np-chain-from-phase-trace-v1")
    });
    let provider_export_arg = args.next().unwrap_or_else(|| "-".to_owned());
    let source_phase_trace_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "source phase-atom trace JSONL path is required".to_owned())?;
    let provider_boundary_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if provider_boundary_paths.is_empty() {
        return Err("at least one provider-boundary event JSONL path is required".to_owned());
    }

    let paths = ProviderBoundaryNpChainPaths::from_prefix(&artifact_prefix);
    copy_file(&source_phase_trace_path, &paths.phase_trace)?;
    let provider_export_path = if provider_export_arg == "-" {
        write_empty_file(&paths.provider_export_empty)?;
        paths.provider_export_empty.clone()
    } else {
        PathBuf::from(provider_export_arg)
    };

    let mut executed_steps = vec!["external_score_ready_phase_trace_input"];

    let mut join_args = vec![
        path_string(&paths.join_report),
        path_string(&paths.joined_trace),
    ];
    join_args.push(path_string(&paths.phase_trace));
    join_args.extend(provider_boundary_paths.iter().map(path_string));
    super::run_phase_stream_provider_boundary_correlation_join_v1(join_args.into_iter())?;
    executed_steps.push("provider_boundary_correlation_join");

    run_chain_after_join(
        &report_path,
        &paths,
        &provider_boundary_paths,
        &provider_export_path,
        &mut executed_steps,
    )
}

fn run_chain_after_join(
    report_path: &Path,
    paths: &ProviderBoundaryNpChainPaths,
    provider_boundary_paths: &[PathBuf],
    provider_export_path: &Path,
    executed_steps: &mut Vec<&'static str>,
) -> Result<(), String> {
    super::run_phase_stream_phase_atom_live_capture_readiness_v1(
        vec![
            path_string(&paths.readiness_report),
            path_string(&paths.joined_trace),
        ]
        .into_iter(),
    )?;
    executed_steps.push("phase_atom_live_capture_readiness");

    super::run_phase_stream_online_miner_daemon_v1(
        vec![
            path_string(&paths.miner_report),
            path_string(&paths.checkpoint_dir),
            path_string(&paths.decision_log),
            CHAIN_CELLS.to_owned(),
            CHAIN_MIN_BUCKET_EVENTS.to_owned(),
            CHAIN_BASE_MARGIN_FLOOR_MICRO.to_owned(),
            CHAIN_COMPILE_EVERY_ROWS.to_owned(),
            CHAIN_MAX_ACTIVE_BUCKETS.to_owned(),
            CHAIN_RESERVOIR_PER_LABEL.to_owned(),
            path_string(&paths.joined_trace),
        ]
        .into_iter(),
    )?;
    executed_steps.push("online_phase_center_miner");

    super::run_phase_stream_online_miner_portfolio_selector_v1(
        vec![
            path_string(&paths.selector_report),
            path_string(&paths.miner_report),
            path_string(&paths.decision_log),
            CHAIN_MAX_SELECTED_BUCKETS.to_owned(),
        ]
        .into_iter(),
    )?;
    executed_steps.push("automatic_portfolio_selector_baseline");

    super::run_phase_stream_online_miner_portfolio_np_rescue_v1(
        vec![
            path_string(&paths.np_rescue_report),
            path_string(&paths.selector_report),
            path_string(&paths.decision_log),
            CHAIN_MAX_SELECTED_SUBCENTERS.to_owned(),
            path_string(&paths.joined_trace),
        ]
        .into_iter(),
    )?;
    executed_steps.push("np_rescue");

    super::run_phase_stream_online_miner_portfolio_provider_correlation_audit_v1(
        vec![
            path_string(&paths.provider_correlation_audit_report),
            path_string(&paths.joined_trace),
            path_string(&paths.decision_log),
        ]
        .into_iter(),
    )?;
    executed_steps.push("provider_correlation_audit_pre_runtime");

    let np_rescue = read_json_value(&paths.np_rescue_report)?;
    let selected_subcenter_count =
        json_usize(&np_rescue, &["selected_subcenter_count"]).unwrap_or(0);
    if selected_subcenter_count == 0 {
        write_chain_report(
            report_path,
            paths,
            provider_boundary_paths,
            provider_export_path,
            executed_steps,
            None,
            "np_rescue",
            "PHASE_STREAM_PROVIDER_BOUNDARY_NP_CHAIN_V1_WATCH_NO_NP_RESCUE_SUBCENTERS",
        )?;
        print_summary(
            report_path,
            selected_subcenter_count,
            0,
            0,
            "PHASE_STREAM_PROVIDER_BOUNDARY_NP_CHAIN_V1_WATCH_NO_NP_RESCUE_SUBCENTERS",
        );
        return Ok(());
    }

    super::run_phase_stream_online_miner_portfolio_np_rescue_runtime_replay_v1(
        vec![
            path_string(&paths.np_runtime_report),
            path_string(&paths.np_rescue_report),
        ]
        .into_iter(),
    )?;
    executed_steps.push("np_runtime_replay");

    let np_runtime = read_json_value(&paths.np_runtime_report)?;
    let billing_request_jsonl_path = json_string(&np_runtime, &["billing_request_jsonl_path"])
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "NP runtime report '{}' missing billing_request_jsonl_path",
                paths.np_runtime_report.display()
            )
        })?;

    super::run_phase_stream_online_miner_portfolio_provider_correlation_audit_v1(
        vec![
            path_string(&paths.provider_correlation_audit_report),
            path_string(&paths.joined_trace),
            path_string(&paths.decision_log),
            path_string(&billing_request_jsonl_path),
        ]
        .into_iter(),
    )?;
    executed_steps.push("provider_correlation_audit_with_billing_requests");

    super::run_phase_stream_online_miner_portfolio_billing_evidence_contract_v1(
        vec![
            path_string(&paths.billing_contract_report),
            path_string(&paths.np_runtime_report),
            path_string(&paths.billing_template_jsonl),
        ]
        .into_iter(),
    )?;
    executed_steps.push("billing_evidence_contract");

    super::run_phase_stream_online_miner_portfolio_provider_export_normalize_v1(
        vec![
            path_string(&paths.provider_normalize_report),
            path_string(&billing_request_jsonl_path),
            path_string(provider_export_path),
            path_string(&paths.provider_normalized_jsonl),
        ]
        .into_iter(),
    )?;
    executed_steps.push("provider_export_normalize");

    super::run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1(
        vec![
            path_string(&paths.billing_evidence_gate_report),
            path_string(&billing_request_jsonl_path),
            path_string(&paths.provider_normalized_jsonl),
            path_string(&paths.billing_missing_jsonl),
        ]
        .into_iter(),
    )?;
    executed_steps.push("billing_evidence_gate");

    super::run_phase_stream_online_miner_portfolio_admission_gate_v1(
        vec![
            path_string(&paths.admission_report),
            path_string(&paths.np_runtime_report),
            path_string(&paths.billing_evidence_gate_report),
        ]
        .into_iter(),
    )?;
    executed_steps.push("admission_gate");

    super::run_phase_stream_online_miner_portfolio_promotion_manifest_v1(
        vec![
            path_string(&paths.promotion_report),
            path_string(&paths.admission_report),
            path_string(&paths.billing_contract_report),
        ]
        .into_iter(),
    )?;
    executed_steps.push("promotion_manifest");

    super::run_phase_stream_online_miner_portfolio_evidence_chain_audit_v1(
        vec![
            path_string(&paths.evidence_chain_report),
            path_string(&paths.np_runtime_report),
            path_string(&paths.np_runtime_report),
            path_string(&paths.billing_contract_report),
            path_string(&paths.provider_normalize_report),
            path_string(&paths.billing_evidence_gate_report),
            path_string(&paths.admission_report),
            path_string(&paths.promotion_report),
            path_string(&paths.provider_correlation_audit_report),
        ]
        .into_iter(),
    )?;
    executed_steps.push("evidence_chain_audit");

    let evidence_chain = read_json_value(&paths.evidence_chain_report)?;
    let portfolio_accepts = json_usize(
        &evidence_chain,
        &["runtime", "portfolio_unique_cpu_accepts_over_exact_cache"],
    )
    .unwrap_or(0);
    let portfolio_tokens =
        json_usize(&evidence_chain, &["runtime", "portfolio_tokens_saved"]).unwrap_or(0);
    let product_ready = json_bool(&evidence_chain, &["product_ready"]).unwrap_or(false);
    let market_money_claim_allowed =
        json_bool(&evidence_chain, &["market_money_claim_allowed"]).unwrap_or(false);
    let verdict = if product_ready && !market_money_claim_allowed {
        "PHASE_STREAM_PROVIDER_BOUNDARY_NP_CHAIN_V1_READY_FOR_PRODUCT_REVIEW_MONEY_BLOCKED"
    } else if product_ready {
        "PHASE_STREAM_PROVIDER_BOUNDARY_NP_CHAIN_V1_READY_FOR_PRODUCT_REVIEW"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_NP_CHAIN_V1_WATCH_EVIDENCE_CHAIN_BLOCKED"
    };

    write_chain_report(
        report_path,
        paths,
        provider_boundary_paths,
        provider_export_path,
        executed_steps,
        Some(&billing_request_jsonl_path),
        "evidence_chain_audit",
        verdict,
    )?;
    print_summary(
        report_path,
        selected_subcenter_count,
        portfolio_accepts,
        portfolio_tokens,
        verdict,
    );
    Ok(())
}

impl ProviderBoundaryNpChainPaths {
    fn from_prefix(prefix: &Path) -> Self {
        let prefix = path_string(prefix);
        Self {
            phase_trace: suffixed(&prefix, ".phase.jsonl"),
            capture_report: suffixed(&prefix, ".capture.report.json"),
            joined_trace: suffixed(&prefix, ".joined.jsonl"),
            join_report: suffixed(&prefix, ".join.report.json"),
            readiness_report: suffixed(&prefix, ".readiness.report.json"),
            miner_report: suffixed(&prefix, ".miner.report.json"),
            checkpoint_dir: suffixed(&prefix, ".online-miner"),
            decision_log: suffixed(&prefix, ".decisions.jsonl"),
            selector_report: suffixed(&prefix, ".selector.report.json"),
            np_rescue_report: suffixed(&prefix, ".np-rescue.report.json"),
            np_runtime_report: suffixed(&prefix, ".np-runtime.report.json"),
            provider_correlation_audit_report: suffixed(
                &prefix,
                ".provider-correlation-audit.report.json",
            ),
            provider_export_empty: suffixed(&prefix, ".provider-export-empty.jsonl"),
            billing_contract_report: suffixed(&prefix, ".billing-contract.report.json"),
            billing_template_jsonl: suffixed(&prefix, ".billing-evidence-template.jsonl"),
            provider_normalize_report: suffixed(&prefix, ".provider-normalize.report.json"),
            provider_normalized_jsonl: suffixed(&prefix, ".provider-normalized.jsonl"),
            billing_evidence_gate_report: suffixed(&prefix, ".billing-evidence-gate.report.json"),
            billing_missing_jsonl: suffixed(&prefix, ".billing-missing.jsonl"),
            admission_report: suffixed(&prefix, ".admission.report.json"),
            promotion_report: suffixed(&prefix, ".promotion.report.json"),
            evidence_chain_report: suffixed(&prefix, ".evidence-chain.report.json"),
        }
    }
}

fn write_chain_report(
    report_path: &Path,
    paths: &ProviderBoundaryNpChainPaths,
    provider_boundary_paths: &[PathBuf],
    provider_export_path: &Path,
    executed_steps: &[&'static str],
    billing_request_jsonl_path: Option<&Path>,
    stopped_after: &'static str,
    verdict: &'static str,
) -> Result<(), String> {
    let capture = read_optional_json(&paths.capture_report)?;
    let join = read_optional_json(&paths.join_report)?;
    let readiness = read_optional_json(&paths.readiness_report)?;
    let miner = read_optional_json(&paths.miner_report)?;
    let selector = read_optional_json(&paths.selector_report)?;
    let np_rescue = read_optional_json(&paths.np_rescue_report)?;
    let runtime = read_optional_json(&paths.np_runtime_report)?;
    let provider_correlation = read_optional_json(&paths.provider_correlation_audit_report)?;
    let billing_contract = read_optional_json(&paths.billing_contract_report)?;
    let provider_normalize = read_optional_json(&paths.provider_normalize_report)?;
    let billing_evidence_gate = read_optional_json(&paths.billing_evidence_gate_report)?;
    let admission = read_optional_json(&paths.admission_report)?;
    let promotion = read_optional_json(&paths.promotion_report)?;
    let evidence_chain = read_optional_json(&paths.evidence_chain_report)?;

    let product_ready = evidence_chain
        .as_ref()
        .and_then(|value| json_bool(value, &["product_ready"]))
        .unwrap_or(false);
    let chain_market_money_claim = evidence_chain
        .as_ref()
        .and_then(|value| json_bool(value, &["market_money_claim_allowed"]))
        .unwrap_or(false);
    let promotion_ready = promotion
        .as_ref()
        .and_then(|value| json_bool(value, &["promotion", "promotion_ready"]))
        .unwrap_or(false);

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_np_chain_v1",
        "mode": "provider_boundary_to_np_evidence_chain_orchestrator",
        "provider_boundary_event_paths": provider_boundary_paths
            .iter()
            .map(path_string)
            .collect::<Vec<_>>(),
        "provider_export_path": provider_export_path,
        "provider_export_placeholder_empty": provider_export_path == paths.provider_export_empty,
        "executed_steps": executed_steps,
        "stopped_after": stopped_after,
        "artifact_paths": {
            "phase_trace": paths.phase_trace,
            "capture_report": paths.capture_report,
            "joined_trace": paths.joined_trace,
            "join_report": paths.join_report,
            "readiness_report": paths.readiness_report,
            "miner_report": paths.miner_report,
            "checkpoint_dir": paths.checkpoint_dir,
            "decision_log": paths.decision_log,
            "selector_report": paths.selector_report,
            "np_rescue_report": paths.np_rescue_report,
            "np_runtime_report": paths.np_runtime_report,
            "provider_correlation_audit_report": paths.provider_correlation_audit_report,
            "billing_request_jsonl_path": billing_request_jsonl_path.map(path_string),
            "billing_contract_report": paths.billing_contract_report,
            "billing_template_jsonl": paths.billing_template_jsonl,
            "provider_normalize_report": paths.provider_normalize_report,
            "provider_normalized_jsonl": paths.provider_normalized_jsonl,
            "billing_evidence_gate_report": paths.billing_evidence_gate_report,
            "billing_missing_jsonl": paths.billing_missing_jsonl,
            "admission_report": paths.admission_report,
            "promotion_report": paths.promotion_report,
            "evidence_chain_report": paths.evidence_chain_report
        },
        "chain_defaults": {
            "cells": CHAIN_CELLS,
            "min_bucket_events": CHAIN_MIN_BUCKET_EVENTS,
            "base_margin_floor_micro": CHAIN_BASE_MARGIN_FLOOR_MICRO,
            "compile_every_rows": CHAIN_COMPILE_EVERY_ROWS,
            "max_active_buckets": CHAIN_MAX_ACTIVE_BUCKETS,
            "reservoir_per_label": CHAIN_RESERVOIR_PER_LABEL,
            "max_selected_buckets": CHAIN_MAX_SELECTED_BUCKETS,
            "max_selected_subcenters": CHAIN_MAX_SELECTED_SUBCENTERS
        },
        "capture": summarize(&capture, &[
            "verdict",
            "output_rows",
            "rows_with_provider_correlation_keys",
            "rows_ready_for_route_family_mining",
            "rows_ready_for_existing_shadow_scoring"
        ]),
        "join": summarize(&join, &[
            "verdict",
            "output_rows",
            "output_rows_with_provider_correlation_keys",
            "score_ready_rows_with_provider_correlation",
            "score_ready_rows_missing_provider_correlation"
        ]),
        "readiness": summarize(&readiness, &[
            "verdict",
            "total_rows",
            "rows_ready_for_route_family_mining",
            "rows_ready_for_existing_shadow_scoring"
        ]),
        "online_miner": summarize(&miner, &[
            "verdict",
            "parsed_events",
            "bucket_count",
            "compiled_checkpoint_count",
            "auto_calibrated_unique_cpu_accepts_over_exact_cache",
            "auto_calibrated_false_accepts"
        ]),
        "selector_baseline": summarize(&selector, &[
            "verdict",
            "selected_bucket_count",
            "manual_class_list_used",
            "static_topn_seed_used",
            "product_dynamic_discovery_claim_allowed"
        ]),
        "np_rescue": summarize(&np_rescue, &[
            "verdict",
            "selected_subcenter_count",
            "candidate_subcenter_count",
            "recovered_total_accepts",
            "false_accepts_after_rescue",
            "rescued_safe_accepts"
        ]),
        "np_runtime": summarize(&runtime, &[
            "verdict",
            "runtime_replay_passed",
            "selected_subcenter_count",
            "portfolio_unique_cpu_accepts_over_exact_cache",
            "portfolio_tokens_saved",
            "billing_request_rows",
            "external_provider_correlation_key_rows",
            "false_accepts"
        ]),
        "provider_correlation": summarize(&provider_correlation, &[
            "verdict",
            "total_rows",
            "rows_with_provider_correlation_keys",
            "cpu_shadow_accept_rows_with_provider_correlation_keys",
            "billing_request_rows_with_provider_correlation_keys",
            "rows_with_provider_key_atom_leak"
        ]),
        "billing_contract": summarize(&billing_contract, &[
            "verdict",
            "request_rows",
            "template_rows",
            "template_rows_match_request_rows"
        ]),
        "provider_normalize": summarize(&provider_normalize, &[
            "verdict",
            "provider_export_rows",
            "normalized_evidence_rows",
            "normalized_matched_request_rows"
        ]),
        "billing_evidence_gate": summarize(&billing_evidence_gate, &[
            "verdict",
            "request_rows",
            "evidence_rows",
            "rows_enriched_provider_cost",
            "rows_enriched_provider_tokens",
            "provider_billing_evidence_present"
        ]),
        "admission": summarize(&admission, &[
            "verdict",
            "market_money_claim_allowed",
            "product_promotion_allowed"
        ]),
        "promotion": summarize(&promotion, &[
            "verdict",
            "promotion"
        ]),
        "evidence_chain": summarize(&evidence_chain, &[
            "verdict",
            "product_ready",
            "market_money_claim_allowed",
            "blockers"
        ]),
        "product_ready": product_ready,
        "promotion_ready": promotion_ready,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": chain_market_money_claim,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": verdict,
        "boundary": "cold evidence-chain orchestration only: consumes provider-boundary event rows, builds phase-center shadow artifacts and billing evidence gates, but does not change scoring, compile product registry, serve, promote, enable local_accept, or claim missing money"
    });
    super::write_json_file(report_path, &report)
}

fn summarize(value: &Option<Value>, keys: &[&str]) -> Value {
    let Some(value) = value else {
        return serde_json::json!({
            "present": false
        });
    };
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

fn print_summary(
    report_path: &Path,
    selected_subcenter_count: usize,
    portfolio_accepts: usize,
    portfolio_tokens: usize,
    verdict: &str,
) {
    println!("phase_stream_provider_boundary_np_chain_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  selected_subcenter_count: {selected_subcenter_count}");
    println!("  portfolio_unique_cpu_accepts_over_exact_cache: {portfolio_accepts}");
    println!("  portfolio_tokens_saved: {portfolio_tokens}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
}

fn suffixed(prefix: &str, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{prefix}{suffix}"))
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn write_empty_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::write(path, "").map_err(|error| {
        format!(
            "failed to write empty provider export '{}': {error}",
            path.display()
        )
    })
}

fn copy_file(source: &Path, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::copy(source, destination).map_err(|error| {
        format!(
            "failed to copy '{}' to '{}': {error}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn read_optional_json(path: &Path) -> Result<Option<Value>, String> {
    if path.exists() {
        read_json_value(path).map(Some)
    } else {
        Ok(None)
    }
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON '{}': {error}", path.display()))
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)?.as_str().map(ToOwned::to_owned)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path)?.as_bool()
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    let value = json_at(value, path)?;
    value
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .or_else(|| {
            value
                .as_i64()
                .and_then(|number| usize::try_from(number).ok())
        })
}
