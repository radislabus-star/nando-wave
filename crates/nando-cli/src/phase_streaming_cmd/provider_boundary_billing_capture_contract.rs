use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_CONTRACT_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-billing-capture-contract-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_TEMPLATE_JSONL: &str =
    "target/nando-wave/streaming/provider-boundary-billing-capture-contract-v1.template.jsonl";
const DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_TEMPLATE_CSV: &str =
    "target/nando-wave/streaming/provider-boundary-billing-capture-contract-v1.template.csv";
const DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_EVIDENCE_GATE_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-billing-capture-evidence-gate-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_EVIDENCE_MISSING_JSONL: &str =
    "target/nando-wave/streaming/provider-boundary-billing-capture-evidence-gate-v1.missing.jsonl";
const DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-billing-capture-chain-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_PREFIX: &str =
    "target/nando-wave/streaming/provider-boundary-billing-capture-chain-v1";

#[derive(Default)]
struct ContractState {
    capture_request_rows: usize,
    template_rows: usize,
    rows_with_join_keys: usize,
    rows_missing_join_keys: usize,
    total_phase_rows: usize,
    total_tokens_estimate: usize,
    total_cost_estimate_microusd: u64,
    token_estimate_rows: usize,
    cost_estimate_rows: usize,
}

#[derive(Clone)]
struct CaptureRequest {
    row: Value,
    join_keys: Vec<String>,
    phase_row_count: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Default)]
struct EvidenceGateState {
    capture_request_rows: usize,
    capture_requests_with_join_keys: usize,
    capture_requests_missing_join_keys: usize,
    evidence_rows: usize,
    rows_with_match_keys: usize,
    rows_with_provider_keys: usize,
    rows_with_required_identity: usize,
    rows_with_provider_tokens: usize,
    rows_with_provider_cost: usize,
    rows_with_synthetic_or_test_source: usize,
    rows_with_provider_key_atom_leak: usize,
    valid_evidence_rows: usize,
    covered_capture_requests: usize,
    missing_capture_requests: usize,
    covered_phase_rows: usize,
    missing_phase_rows: usize,
    covered_tokens_estimate_from_trace: usize,
    missing_tokens_estimate_from_trace: usize,
    covered_cost_estimate_microusd_from_trace: u64,
    missing_cost_estimate_microusd_from_trace: u64,
    provider_total_tokens: usize,
    provider_cost_microusd: u64,
    duplicate_billing_evidence_ids: usize,
}

struct BillingCaptureChainPaths {
    evidence_gate_report: PathBuf,
    evidence_missing_jsonl: PathBuf,
    append_sink_report: PathBuf,
    provider_boundary_jsonl: PathBuf,
    capture_coverage_report: PathBuf,
    match_readiness_report: PathBuf,
}

impl BillingCaptureChainPaths {
    fn from_prefix(prefix: &Path) -> Self {
        let stem = path_string(prefix);
        Self {
            evidence_gate_report: PathBuf::from(format!("{stem}.evidence-gate.report.json")),
            evidence_missing_jsonl: PathBuf::from(format!("{stem}.evidence-gate.missing.jsonl")),
            append_sink_report: PathBuf::from(format!("{stem}.append-sink.report.json")),
            provider_boundary_jsonl: PathBuf::from(format!("{stem}.provider-boundary.jsonl")),
            capture_coverage_report: PathBuf::from(format!("{stem}.capture-coverage.report.json")),
            match_readiness_report: PathBuf::from(format!("{stem}.match-readiness.report.json")),
        }
    }
}

pub(crate) fn run_phase_stream_provider_boundary_billing_capture_contract_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_CONTRACT_REPORT)
    });
    let template_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_TEMPLATE_JSONL));
    let template_csv_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_TEMPLATE_CSV));
    let capture_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let text = std::fs::read_to_string(&capture_request_jsonl_path).map_err(|error| {
        format!(
            "failed to read capture request '{}': {error}",
            capture_request_jsonl_path.display()
        )
    })?;
    let jsonl_file = create_file(&template_jsonl_path)?;
    let csv_file = create_file(&template_csv_path)?;
    let mut jsonl_writer = BufWriter::new(jsonl_file);
    let mut csv_writer = BufWriter::new(csv_file);
    write_csv_header(&mut csv_writer)?;

    let mut state = ContractState::default();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse capture request '{}' line {}: {error}",
                capture_request_jsonl_path.display(),
                line_index + 1
            )
        })?;
        state.capture_request_rows += 1;
        let join_keys = json_string_vec(row.get("join_keys"));
        state.rows_with_join_keys += usize::from(!join_keys.is_empty());
        state.rows_missing_join_keys += usize::from(join_keys.is_empty());
        let phase_row_count = json_usize(&row, &["phase_row_count"]).unwrap_or(0);
        let total_tokens = json_usize(&row, &["total_tokens"]).unwrap_or(0);
        let total_cost = json_u64(&row, &["total_cost_microusd"]).unwrap_or(0);
        state.total_phase_rows = state.total_phase_rows.saturating_add(phase_row_count);
        state.total_tokens_estimate = state.total_tokens_estimate.saturating_add(total_tokens);
        state.total_cost_estimate_microusd = state
            .total_cost_estimate_microusd
            .saturating_add(total_cost);
        state.token_estimate_rows += usize::from(total_tokens > 0);
        state.cost_estimate_rows += usize::from(total_cost > 0);

        let template_row = serde_json::json!({
            "schema_version": "provider_boundary_billing_capture_contract_v1",
            "capture_request_id": json_string(&row, &["capture_request_id"]),
            "primary_join_key": json_string(&row, &["primary_join_key"]),
            "match_keys": join_keys,
            "required_provider_fields": [
                "billing_evidence_id",
                "billing_source",
                "provider",
                "provider_request_id or provider_response_id or provider_trace_id or external_provider_request_id or openai_request_id or anthropic_request_id",
                "provider_total_tokens",
                "provider_cost_microusd"
            ],
            "billing_evidence_id": null,
            "billing_source": null,
            "provider": null,
            "model_id": null,
            "provider_request_id": null,
            "provider_response_id": null,
            "provider_trace_id": null,
            "external_provider_request_id": null,
            "openai_request_id": null,
            "anthropic_request_id": null,
            "provider_total_tokens": null,
            "provider_cost_microusd": null,
            "estimated_total_tokens_from_trace": total_tokens,
            "estimated_total_cost_microusd_from_trace": total_cost,
            "phase_row_count": phase_row_count,
            "sample_sources": row.get("sample_sources").cloned().unwrap_or_else(|| serde_json::json!([])),
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "template only: fill with real provider boundary ids/tokens/cost before using as provider evidence; trace estimates are not provider billing evidence"
        });
        serde_json::to_writer(&mut jsonl_writer, &template_row).map_err(|error| {
            format!(
                "failed to serialize billing capture template '{}': {error}",
                template_jsonl_path.display()
            )
        })?;
        jsonl_writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write billing capture template '{}': {error}",
                template_jsonl_path.display()
            )
        })?;
        write_csv_row(&mut csv_writer, &template_row)?;
        state.template_rows += 1;
    }
    jsonl_writer.flush().map_err(|error| {
        format!(
            "failed to flush billing capture template '{}': {error}",
            template_jsonl_path.display()
        )
    })?;
    csv_writer.flush().map_err(|error| {
        format!(
            "failed to flush billing capture CSV '{}': {error}",
            template_csv_path.display()
        )
    })?;

    let contract_ready = state.capture_request_rows > 0
        && state.template_rows == state.capture_request_rows
        && state.rows_missing_join_keys == 0;
    let verdict = if contract_ready {
        "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_CONTRACT_V1_READY"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_CONTRACT_V1_WATCH_INCOMPLETE_CAPTURE_REQUESTS"
    };
    let mut blockers = Vec::<&'static str>::new();
    if state.capture_request_rows == 0 {
        blockers.push("no_capture_request_rows");
    }
    if state.rows_missing_join_keys > 0 {
        blockers.push("some_capture_requests_missing_join_keys");
    }

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_billing_capture_contract_v1",
        "capture_request_jsonl_path": path_string(&capture_request_jsonl_path),
        "template_jsonl_path": path_string(&template_jsonl_path),
        "template_csv_path": path_string(&template_csv_path),
        "capture_request_rows": state.capture_request_rows,
        "template_rows": state.template_rows,
        "rows_with_join_keys": state.rows_with_join_keys,
        "rows_missing_join_keys": state.rows_missing_join_keys,
        "total_phase_rows": state.total_phase_rows,
        "total_tokens_estimate_from_trace": state.total_tokens_estimate,
        "total_cost_estimate_microusd_from_trace": state.total_cost_estimate_microusd,
        "token_estimate_rows": state.token_estimate_rows,
        "cost_estimate_rows": state.cost_estimate_rows,
        "required_provider_event_fields": [
            "billing_evidence_id",
            "billing_source",
            "provider",
            "provider_request_id or provider_response_id or provider_trace_id or external_provider_request_id or openai_request_id or anthropic_request_id",
            "provider_total_tokens",
            "provider_cost_microusd"
        ],
        "readiness": {
            "contract_ready_for_live_provider_boundary_capture": contract_ready,
            "trace_estimates_are_not_provider_billing_evidence": true,
            "market_money_claim_allowed": false,
            "local_accept_enabled": false
        },
        "blockers": blockers,
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
        "boundary": "provider-boundary billing capture contract only: emits fillable templates for real provider ids/tokens/cost; does not fabricate evidence, mine, score, compile .nwpc, serve, promote, local-accept, or claim money"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_billing_capture_contract_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  template_jsonl_path: {}", template_jsonl_path.display());
    println!("  template_csv_path: {}", template_csv_path.display());
    println!("  capture_request_rows: {}", state.capture_request_rows);
    println!("  template_rows: {}", state.template_rows);
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_provider_boundary_billing_capture_evidence_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_EVIDENCE_GATE_REPORT)
    });
    let capture_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;
    let provider_evidence_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "filled provider evidence JSONL path is required".to_owned())?;
    let missing_jsonl_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_EVIDENCE_MISSING_JSONL)
    });
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let captures = read_capture_requests(&capture_request_jsonl_path)?;
    let mut state = EvidenceGateState {
        capture_request_rows: captures.len(),
        ..EvidenceGateState::default()
    };
    for capture in captures.values() {
        state.capture_requests_with_join_keys += usize::from(!capture.join_keys.is_empty());
        state.capture_requests_missing_join_keys += usize::from(capture.join_keys.is_empty());
    }

    let mut key_to_capture_ids = BTreeMap::<String, BTreeSet<String>>::new();
    for (capture_id, capture) in &captures {
        for key in &capture.join_keys {
            key_to_capture_ids
                .entry(key.clone())
                .or_default()
                .insert(capture_id.clone());
        }
    }

    let mut covered_capture_ids = BTreeSet::<String>::new();
    let mut billing_evidence_ids = BTreeSet::<String>::new();
    let evidence_text =
        std::fs::read_to_string(&provider_evidence_jsonl_path).map_err(|error| {
            format!(
                "failed to read provider evidence '{}': {error}",
                provider_evidence_jsonl_path.display()
            )
        })?;
    for (line_index, line) in evidence_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse provider evidence '{}' line {}: {error}",
                provider_evidence_jsonl_path.display(),
                line_index + 1
            )
        })?;
        state.evidence_rows += 1;

        let match_keys = evidence_match_keys(&row);
        let provider_keys = super::phase_atom_external_provider_correlation_keys(&row);
        let billing_evidence_id = json_string(&row, &["billing_evidence_id"])
            .or_else(|| json_string(&row, &["id"]))
            .unwrap_or_default();
        let billing_source = json_string(&row, &["billing_source"])
            .or_else(|| json_string(&row, &["provider_billing_evidence_source"]))
            .or_else(|| json_string(&row, &["source"]))
            .unwrap_or_default();
        let provider = json_string(&row, &["provider"])
            .or_else(|| json_string(&row, &["model_provider"]))
            .unwrap_or_default();
        let provider_tokens = provider_total_tokens(&row).unwrap_or(0);
        let provider_cost = provider_cost_microusd(&row).unwrap_or(0);
        let provider_key_atom_leak = provider_key_leaks_into_atoms(&row);
        let synthetic_or_test_source = synthetic_or_test_source(&billing_source)
            || synthetic_or_test_source(&billing_evidence_id);

        state.rows_with_match_keys += usize::from(!match_keys.is_empty());
        state.rows_with_provider_keys += usize::from(!provider_keys.is_empty());
        state.rows_with_required_identity += usize::from(
            !billing_evidence_id.is_empty() && !billing_source.is_empty() && !provider.is_empty(),
        );
        state.rows_with_provider_tokens += usize::from(provider_tokens > 0);
        state.rows_with_provider_cost += usize::from(provider_cost > 0);
        state.rows_with_synthetic_or_test_source += usize::from(synthetic_or_test_source);
        state.rows_with_provider_key_atom_leak += usize::from(provider_key_atom_leak);
        state.provider_total_tokens = state.provider_total_tokens.saturating_add(provider_tokens);
        state.provider_cost_microusd = state.provider_cost_microusd.saturating_add(provider_cost);
        if !billing_evidence_id.is_empty() && !billing_evidence_ids.insert(billing_evidence_id) {
            state.duplicate_billing_evidence_ids += 1;
        }

        let valid_evidence_row = !match_keys.is_empty()
            && !provider_keys.is_empty()
            && !billing_source.is_empty()
            && !provider.is_empty()
            && provider_tokens > 0
            && provider_cost > 0
            && !provider_key_atom_leak
            && !synthetic_or_test_source;
        if !valid_evidence_row {
            continue;
        }
        state.valid_evidence_rows += 1;
        for key in match_keys {
            if let Some(capture_ids) = key_to_capture_ids.get(&key) {
                covered_capture_ids.extend(capture_ids.iter().cloned());
            }
        }
    }

    state.covered_capture_requests = covered_capture_ids.len();
    state.missing_capture_requests = captures.len().saturating_sub(covered_capture_ids.len());
    let missing_file = create_file(&missing_jsonl_path)?;
    let mut missing_writer = BufWriter::new(missing_file);
    for (capture_id, capture) in &captures {
        if covered_capture_ids.contains(capture_id) {
            state.covered_phase_rows = state
                .covered_phase_rows
                .saturating_add(capture.phase_row_count);
            state.covered_tokens_estimate_from_trace = state
                .covered_tokens_estimate_from_trace
                .saturating_add(capture.total_tokens);
            state.covered_cost_estimate_microusd_from_trace = state
                .covered_cost_estimate_microusd_from_trace
                .saturating_add(capture.total_cost_microusd);
        } else {
            state.missing_phase_rows = state
                .missing_phase_rows
                .saturating_add(capture.phase_row_count);
            state.missing_tokens_estimate_from_trace = state
                .missing_tokens_estimate_from_trace
                .saturating_add(capture.total_tokens);
            state.missing_cost_estimate_microusd_from_trace = state
                .missing_cost_estimate_microusd_from_trace
                .saturating_add(capture.total_cost_microusd);
            serde_json::to_writer(&mut missing_writer, &capture.row).map_err(|error| {
                format!(
                    "failed to serialize missing capture request '{}': {error}",
                    missing_jsonl_path.display()
                )
            })?;
            missing_writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write missing capture request '{}': {error}",
                    missing_jsonl_path.display()
                )
            })?;
        }
    }
    missing_writer.flush().map_err(|error| {
        format!(
            "failed to flush missing capture requests '{}': {error}",
            missing_jsonl_path.display()
        )
    })?;

    let provider_billing_evidence_complete = state.capture_request_rows > 0
        && state.evidence_rows > 0
        && state.valid_evidence_rows > 0
        && state.missing_capture_requests == 0
        && state.rows_with_synthetic_or_test_source == 0
        && state.rows_with_provider_key_atom_leak == 0
        && state.duplicate_billing_evidence_ids == 0;
    let mut blockers = Vec::<&'static str>::new();
    if state.capture_request_rows == 0 {
        blockers.push("no_capture_request_rows");
    }
    if state.evidence_rows == 0 {
        blockers.push("no_provider_evidence_rows");
    }
    if state.valid_evidence_rows == 0 {
        blockers.push("no_valid_provider_evidence_rows");
    }
    if state.missing_capture_requests > 0 {
        blockers.push("some_capture_requests_missing_valid_provider_evidence");
    }
    if state.rows_with_synthetic_or_test_source > 0 {
        blockers.push("synthetic_or_test_provider_evidence_source_present");
    }
    if state.rows_with_provider_key_atom_leak > 0 {
        blockers.push("provider_key_atom_leak");
    }
    if state.duplicate_billing_evidence_ids > 0 {
        blockers.push("duplicate_billing_evidence_id");
    }
    let verdict = if provider_billing_evidence_complete {
        "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_EVIDENCE_GATE_V1_READY_FOR_PROVIDER_BOUNDARY_JOIN"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_EVIDENCE_GATE_V1_BLOCKED"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_billing_capture_evidence_gate_v1",
        "capture_request_jsonl_path": path_string(&capture_request_jsonl_path),
        "provider_evidence_jsonl_path": path_string(&provider_evidence_jsonl_path),
        "missing_jsonl_path": path_string(&missing_jsonl_path),
        "capture_request_rows": state.capture_request_rows,
        "capture_requests_with_join_keys": state.capture_requests_with_join_keys,
        "capture_requests_missing_join_keys": state.capture_requests_missing_join_keys,
        "evidence_rows": state.evidence_rows,
        "valid_evidence_rows": state.valid_evidence_rows,
        "rows_with_match_keys": state.rows_with_match_keys,
        "rows_with_provider_keys": state.rows_with_provider_keys,
        "rows_with_required_identity": state.rows_with_required_identity,
        "rows_with_provider_tokens": state.rows_with_provider_tokens,
        "rows_with_provider_cost": state.rows_with_provider_cost,
        "rows_with_synthetic_or_test_source": state.rows_with_synthetic_or_test_source,
        "rows_with_provider_key_atom_leak": state.rows_with_provider_key_atom_leak,
        "duplicate_billing_evidence_ids": state.duplicate_billing_evidence_ids,
        "covered_capture_requests": state.covered_capture_requests,
        "missing_capture_requests": state.missing_capture_requests,
        "covered_phase_rows": state.covered_phase_rows,
        "missing_phase_rows": state.missing_phase_rows,
        "covered_tokens_estimate_from_trace": state.covered_tokens_estimate_from_trace,
        "missing_tokens_estimate_from_trace": state.missing_tokens_estimate_from_trace,
        "covered_cost_estimate_microusd_from_trace": state.covered_cost_estimate_microusd_from_trace,
        "missing_cost_estimate_microusd_from_trace": state.missing_cost_estimate_microusd_from_trace,
        "provider_total_tokens": state.provider_total_tokens,
        "provider_cost_microusd": state.provider_cost_microusd,
        "readiness": {
            "provider_billing_evidence_complete": provider_billing_evidence_complete,
            "requires_external_non_synthetic_provider_source": true,
            "market_money_claim_allowed": false,
            "local_accept_enabled": false
        },
        "blockers": blockers,
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
        "boundary": "provider-boundary billing capture evidence gate only: validates real filled provider ids/tokens/cost against capture requests; does not mine, score, compile .nwpc, serve, promote, local-accept, or claim money"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_billing_capture_evidence_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  capture_request_rows: {}", state.capture_request_rows);
    println!("  evidence_rows: {}", state.evidence_rows);
    println!("  valid_evidence_rows: {}", state.valid_evidence_rows);
    println!(
        "  covered_capture_requests: {}",
        state.covered_capture_requests
    );
    println!(
        "  missing_capture_requests: {}",
        state.missing_capture_requests
    );
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

pub(crate) fn run_phase_stream_provider_boundary_billing_capture_chain_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_REPORT));
    let artifact_prefix = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_PREFIX));
    let capture_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;

    let mut phase_paths = Vec::<PathBuf>::new();
    let mut provider_evidence_path = None::<PathBuf>;
    let mut provider_mode = false;
    for arg in args {
        if arg == "--provider-evidence" {
            provider_mode = true;
            continue;
        }
        if provider_mode {
            if provider_evidence_path.is_some() {
                return Err(format!(
                    "unexpected extra provider evidence path '{arg}'; pass one filled evidence JSONL"
                ));
            }
            provider_evidence_path = Some(PathBuf::from(arg));
        } else {
            phase_paths.push(PathBuf::from(arg));
        }
    }
    if phase_paths.is_empty() {
        return Err(
            "at least one phase-atom trace JSONL path is required before --provider-evidence"
                .to_owned(),
        );
    }
    let provider_evidence_path = provider_evidence_path.ok_or_else(|| {
        "filled provider evidence JSONL path is required after --provider-evidence".to_owned()
    })?;

    let paths = BillingCaptureChainPaths::from_prefix(&artifact_prefix);
    let mut executed_steps = Vec::<&'static str>::new();

    run_phase_stream_provider_boundary_billing_capture_evidence_gate_v1(
        vec![
            path_string(&paths.evidence_gate_report),
            path_string(&capture_request_jsonl_path),
            path_string(&provider_evidence_path),
            path_string(&paths.evidence_missing_jsonl),
        ]
        .into_iter(),
    )?;
    executed_steps.push("billing_capture_evidence_gate");

    let evidence_gate = read_json_value(&paths.evidence_gate_report)?;
    let evidence_ready = json_bool(
        &evidence_gate,
        &["readiness", "provider_billing_evidence_complete"],
    )
    .unwrap_or(false);
    if !evidence_ready {
        write_billing_capture_chain_report(
            &report_path,
            &artifact_prefix,
            &capture_request_jsonl_path,
            &phase_paths,
            &provider_evidence_path,
            &paths,
            &executed_steps,
            &evidence_gate,
            None,
            None,
            None,
            "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_V1_BLOCKED_EVIDENCE_GATE",
        )?;
        print_billing_capture_chain_summary(
            &report_path,
            0,
            0,
            0,
            "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_V1_BLOCKED_EVIDENCE_GATE",
        );
        return Ok(());
    }

    super::run_phase_stream_provider_boundary_append_sink_v1(
        vec![
            path_string(&paths.append_sink_report),
            path_string(&paths.provider_boundary_jsonl),
            path_string(&provider_evidence_path),
        ]
        .into_iter(),
    )?;
    executed_steps.push("provider_boundary_append_sink");

    super::run_phase_stream_provider_boundary_capture_coverage_gate_v1(
        vec![
            path_string(&paths.capture_coverage_report),
            path_string(&capture_request_jsonl_path),
            "--provider".to_owned(),
            path_string(&paths.provider_boundary_jsonl),
        ]
        .into_iter(),
    )?;
    executed_steps.push("provider_boundary_capture_coverage_gate");

    let mut readiness_args = vec![path_string(&paths.match_readiness_report)];
    readiness_args.extend(phase_paths.iter().map(|path| path_string(path)));
    readiness_args.push("--provider".to_owned());
    readiness_args.push(path_string(&paths.provider_boundary_jsonl));
    super::run_phase_stream_provider_boundary_match_readiness_v1(readiness_args.into_iter())?;
    executed_steps.push("provider_boundary_match_readiness");

    let append = read_json_value(&paths.append_sink_report)?;
    let coverage = read_json_value(&paths.capture_coverage_report)?;
    let readiness = read_json_value(&paths.match_readiness_report)?;
    let appended_rows = json_usize(&append, &["appended_rows"]).unwrap_or(0);
    let covered_capture_requests =
        json_usize(&coverage, &["capture_requests", "covered_capture_requests"]).unwrap_or(0);
    let score_ready_rows_with_provider_join = json_usize(
        &readiness,
        &["phase", "score_ready_rows_with_provider_join"],
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
    let full_capture_coverage =
        json_bool(&coverage, &["readiness", "full_capture_coverage"]).unwrap_or(false);
    let economic_join_ready =
        json_bool(&readiness, &["readiness", "economic_join_ready"]).unwrap_or(false);
    let verdict = if provider_key_atom_leak > 0 {
        "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if full_capture_coverage && economic_join_ready && appended_rows > 0 {
        "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_V1_PASS_READY_FOR_NP_CHAIN"
    } else if appended_rows > 0 && covered_capture_requests > 0 {
        "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_V1_WATCH_PARTIAL_PROVIDER_BOUNDARY_READY"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_BILLING_CAPTURE_CHAIN_V1_WATCH_PROVIDER_BOUNDARY_NOT_READY"
    };

    write_billing_capture_chain_report(
        &report_path,
        &artifact_prefix,
        &capture_request_jsonl_path,
        &phase_paths,
        &provider_evidence_path,
        &paths,
        &executed_steps,
        &evidence_gate,
        Some(&append),
        Some(&coverage),
        Some(&readiness),
        verdict,
    )?;
    print_billing_capture_chain_summary(
        &report_path,
        appended_rows,
        covered_capture_requests,
        score_ready_rows_with_provider_join,
        verdict,
    );
    Ok(())
}

fn write_billing_capture_chain_report(
    report_path: &Path,
    artifact_prefix: &Path,
    capture_request_jsonl_path: &Path,
    phase_paths: &[PathBuf],
    provider_evidence_path: &Path,
    paths: &BillingCaptureChainPaths,
    executed_steps: &[&'static str],
    evidence_gate: &Value,
    append_sink: Option<&Value>,
    capture_coverage: Option<&Value>,
    match_readiness: Option<&Value>,
    verdict: &'static str,
) -> Result<(), String> {
    let appended_rows = append_sink
        .and_then(|value| json_usize(value, &["appended_rows"]))
        .unwrap_or(0);
    let covered_capture_requests = capture_coverage
        .and_then(|value| json_usize(value, &["capture_requests", "covered_capture_requests"]))
        .unwrap_or(0);
    let score_ready_rows_with_provider_join = match_readiness
        .and_then(|value| json_usize(value, &["phase", "score_ready_rows_with_provider_join"]))
        .unwrap_or(0);
    let provider_billing_evidence_complete = json_bool(
        evidence_gate,
        &["readiness", "provider_billing_evidence_complete"],
    )
    .unwrap_or(false);
    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_billing_capture_chain_v1",
        "mode": "evidence_gate_first_provider_boundary_chain",
        "artifact_prefix": path_string(artifact_prefix),
        "capture_request_jsonl_path": path_string(capture_request_jsonl_path),
        "phase_atom_trace_paths": phase_paths.iter().map(|path| path_string(path)).collect::<Vec<_>>(),
        "provider_evidence_path": path_string(provider_evidence_path),
        "artifacts": {
            "evidence_gate_report": path_string(&paths.evidence_gate_report),
            "evidence_missing_jsonl": path_string(&paths.evidence_missing_jsonl),
            "append_sink_report": path_string(&paths.append_sink_report),
            "provider_boundary_jsonl": path_string(&paths.provider_boundary_jsonl),
            "capture_coverage_report": path_string(&paths.capture_coverage_report),
            "match_readiness_report": path_string(&paths.match_readiness_report)
        },
        "executed_steps": executed_steps,
        "provider_billing_evidence_complete": provider_billing_evidence_complete,
        "appended_rows": appended_rows,
        "covered_capture_requests": covered_capture_requests,
        "score_ready_rows_with_provider_join": score_ready_rows_with_provider_join,
        "evidence_gate": evidence_gate,
        "append_sink": append_sink,
        "capture_coverage": capture_coverage,
        "match_readiness": match_readiness,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "market_money_claim_allowed": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "provider_evidence_template_accepted": false,
            "synthetic_provider_evidence_accepted": false,
            "local_accept_without_verifier_used": false
        },
        "boundary": "billing-capture chain is cold evidence plumbing only: evidence gate must pass before append/join; no mining, promotion, serving, local_accept, or market money claim",
        "verdict": verdict
    });
    super::write_json_file(report_path, &report)
}

fn print_billing_capture_chain_summary(
    report_path: &Path,
    appended_rows: usize,
    covered_capture_requests: usize,
    score_ready_rows_with_provider_join: usize,
    verdict: &str,
) {
    println!("phase_stream_provider_boundary_billing_capture_chain_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  appended_rows: {appended_rows}");
    println!("  covered_capture_requests: {covered_capture_requests}");
    println!("  score_ready_rows_with_provider_join: {score_ready_rows_with_provider_join}");
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
}

fn create_file(path: &Path) -> Result<std::fs::File, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::File::create(path)
        .map_err(|error| format!("failed to create '{}': {error}", path.display()))
}

fn read_capture_requests(path: &Path) -> Result<BTreeMap<String, CaptureRequest>, String> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read capture requests '{}': {error}",
            path.display()
        )
    })?;
    let mut captures = BTreeMap::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse capture requests '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        let capture_id = json_string(&row, &["capture_request_id"])
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("capture_request_line_{}", line_index + 1));
        let join_keys = json_string_vec(row.get("join_keys"));
        captures.insert(
            capture_id,
            CaptureRequest {
                phase_row_count: json_usize(&row, &["phase_row_count"]).unwrap_or(0),
                total_tokens: json_usize(&row, &["total_tokens"]).unwrap_or(0),
                total_cost_microusd: json_u64(&row, &["total_cost_microusd"]).unwrap_or(0),
                row,
                join_keys,
            },
        );
    }
    Ok(captures)
}

fn evidence_match_keys(row: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    keys.extend(json_string_vec(row.get("match_keys")));
    keys.extend(json_string_vec(json_at(row, &["metadata", "match_keys"])));
    push_match_key_paths(
        &mut keys,
        row,
        "request_fingerprint",
        &[
            &["request_fingerprint"],
            &["metadata", "request_fingerprint"],
        ],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "exact_cache_key",
        &[
            &["exact_cache_key"],
            &["metadata", "exact_cache_key"],
            &["cache_key"],
            &["primary_join_key"],
        ],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "trace_id",
        &[&["trace_id"], &["metadata", "trace_id"]],
    );
    keys.sort();
    keys.dedup();
    keys
}

fn push_match_key_paths(keys: &mut Vec<String>, row: &Value, prefix: &str, paths: &[&[&str]]) {
    for path in paths {
        if let Some(value) = json_string(row, path).filter(|value| !value.is_empty()) {
            if value.contains(':') && value.starts_with(prefix) {
                keys.push(value);
            } else {
                keys.push(format!("{prefix}:{value}"));
            }
        }
    }
}

fn provider_total_tokens(row: &Value) -> Option<usize> {
    for path in [
        &["provider_total_tokens"][..],
        &["total_tokens"],
        &["tokens_total"],
        &["token_cost", "total_tokens"],
    ] {
        if let Some(value) = json_usize(row, path) {
            return Some(value);
        }
    }
    let component_sum = [
        &["input_tokens"][..],
        &["output_tokens"],
        &["cached_input_tokens"],
        &["token_cost", "input_tokens"],
        &["token_cost", "output_tokens"],
        &["token_cost", "cached_input_tokens"],
    ]
    .into_iter()
    .filter_map(|path| json_usize(row, path))
    .sum::<usize>();
    if component_sum > 0 {
        Some(component_sum)
    } else {
        None
    }
}

fn provider_cost_microusd(row: &Value) -> Option<u64> {
    for path in [
        &["provider_cost_microusd"][..],
        &["cost_microusd"],
        &["total_cost_microusd"],
        &["token_cost", "provider_cost_microusd"],
        &["token_cost", "total_cost_microusd"],
    ] {
        if let Some(value) = json_u64(row, path) {
            return Some(value);
        }
    }
    for path in [
        &["provider_cost_usd"][..],
        &["total_cost_usd"],
        &["cost_usd"],
        &["token_cost", "provider_cost_usd"],
        &["token_cost", "total_cost_usd"],
    ] {
        if let Some(value) = json_f64(row, path) {
            if value.is_finite() && value > 0.0 {
                return Some((value * 1_000_000.0).round() as u64);
            }
        }
    }
    None
}

fn provider_key_leaks_into_atoms(row: &Value) -> bool {
    let mut atoms = Vec::new();
    collect_atom_strings(row.get("atom_groups"), &mut atoms);
    for key in [
        "request_atoms",
        "state_atoms",
        "action_atoms",
        "tool_atoms",
        "route_hint_atoms",
        "metadata_atoms",
    ] {
        collect_atom_strings(row.get(key), &mut atoms);
    }
    atoms.iter().any(|atom| {
        atom.starts_with("provider_correlation:")
            || atom.starts_with("provider_request_id:")
            || atom.starts_with("provider_response_id:")
            || atom.starts_with("provider_trace_id:")
            || atom.starts_with("external_provider_request_id:")
            || atom.starts_with("openai_request_id:")
            || atom.starts_with("anthropic_request_id:")
            || atom.starts_with("custom_id:")
    })
}

fn collect_atom_strings(value: Option<&Value>, output: &mut Vec<String>) {
    match value {
        Some(Value::String(text)) => output.push(text.clone()),
        Some(Value::Array(items)) => {
            for item in items {
                collect_atom_strings(Some(item), output);
            }
        }
        Some(Value::Object(map)) => {
            for item in map.values() {
                collect_atom_strings(Some(item), output);
            }
        }
        _ => {}
    }
}

fn synthetic_or_test_source(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["synthetic", "fake", "template", "negative", "smoke", "test"]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn write_csv_header(writer: &mut BufWriter<std::fs::File>) -> Result<(), String> {
    writer
        .write_all(
            b"capture_request_id,primary_join_key,billing_evidence_id,billing_source,provider,model_id,provider_request_id,provider_response_id,provider_trace_id,external_provider_request_id,openai_request_id,anthropic_request_id,provider_total_tokens,provider_cost_microusd,estimated_total_tokens_from_trace,estimated_total_cost_microusd_from_trace,phase_row_count\n",
        )
        .map_err(|error| format!("failed to write billing capture CSV header: {error}"))
}

fn write_csv_row(writer: &mut BufWriter<std::fs::File>, row: &Value) -> Result<(), String> {
    let fields = [
        "capture_request_id",
        "primary_join_key",
        "billing_evidence_id",
        "billing_source",
        "provider",
        "model_id",
        "provider_request_id",
        "provider_response_id",
        "provider_trace_id",
        "external_provider_request_id",
        "openai_request_id",
        "anthropic_request_id",
        "provider_total_tokens",
        "provider_cost_microusd",
        "estimated_total_tokens_from_trace",
        "estimated_total_cost_microusd_from_trace",
        "phase_row_count",
    ];
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            writer
                .write_all(b",")
                .map_err(|error| format!("failed to write billing capture CSV: {error}"))?;
        }
        let value = row.get(*field).cloned().unwrap_or(Value::Null);
        let text = match value {
            Value::Null => String::new(),
            Value::String(value) => value,
            other => other.to_string(),
        };
        write_csv_cell(writer, &text)?;
    }
    writer
        .write_all(b"\n")
        .map_err(|error| format!("failed to write billing capture CSV row: {error}"))
}

fn write_csv_cell(writer: &mut BufWriter<std::fs::File>, value: &str) -> Result<(), String> {
    let escaped = value.replace('"', "\"\"");
    writer
        .write_all(format!("\"{escaped}\"").as_bytes())
        .map_err(|error| format!("failed to write billing capture CSV cell: {error}"))
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSON '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON '{}': {error}", path.display()))
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(Value::as_u64)
}

fn json_f64(value: &Value, path: &[&str]) -> Option<f64> {
    json_at(value, path).and_then(Value::as_f64)
}

fn json_string_vec(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cursor = value;
    for key in path {
        cursor = cursor.get(*key)?;
    }
    Some(cursor)
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
