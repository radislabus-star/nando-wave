use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use super::super::provider_boundary_billing_capture_contract::run_phase_stream_provider_boundary_billing_capture_contract_v1;
use super::super::provider_boundary_codex_token_backfill::run_phase_stream_provider_boundary_codex_token_backfill_v1;
use super::super::provider_boundary_realtrace_token_cost_backfill::run_phase_stream_provider_boundary_realtrace_token_cost_backfill_v1;
use super::super::provider_export_acquisition_pack::run_phase_stream_provider_export_acquisition_pack_v1;
use super::super::provider_export_evidence_chain::run_phase_stream_provider_export_evidence_chain_v1;
use super::super::selected_split_nwpc_provider_export_attestation::run_phase_stream_selected_split_nwpc_provider_export_attestation_contract_v1;
use super::frozen_candidates::LiveStoreFrozenCandidate;
use super::reports::LiveStoreProviderEvidenceArtifactsReport;
use super::state::{
    LiveStoreFutureShadowBillingRequestSummary, LiveStoreProviderArtifactSignature,
};

pub(super) fn live_store_future_shadow_billing_request_path(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("append-live-tail");
    report_path.with_file_name(format!("{stem}-future-shadow-billing-request.jsonl"))
}

pub(super) fn write_live_store_future_shadow_billing_requests(
    request_path: &Path,
    frozen_candidates: &BTreeMap<u32, LiveStoreFrozenCandidate>,
) -> Result<LiveStoreFutureShadowBillingRequestSummary, String> {
    if let Some(parent) = request_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create future-shadow billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = File::create(request_path).map_err(|error| {
        format!(
            "failed to create future-shadow billing request '{}': {error}",
            request_path.display()
        )
    })?;
    let mut writer = io::BufWriter::new(file);
    let mut seen = BTreeSet::<String>::new();
    let mut summary = LiveStoreFutureShadowBillingRequestSummary::default();

    for candidate in frozen_candidates.values() {
        if candidate.future_false_accepts > 0 {
            continue;
        }
        for (event_index, event) in candidate.future_events.iter().enumerate() {
            if !event.verified_safe_accept || event.exact_cache_hit {
                continue;
            }
            let margin_micro = candidate
                .flat_runtime
                .score_vector_margin_micro(0, &event.phase_vector)
                .map_err(|error| {
                    format!("failed to score future-shadow billing request: {error:?}")
                })?;
            if margin_micro < candidate.package.threshold_micro {
                continue;
            }
            let identity = event
                .request_fingerprint
                .as_deref()
                .map(|value| format!("request_fingerprint:{value}"))
                .or_else(|| {
                    event
                        .exact_cache_key
                        .as_deref()
                        .map(|value| format!("exact_cache_key:{value}"))
                })
                .or_else(|| {
                    event
                        .trace_id
                        .as_deref()
                        .map(|value| format!("trace_id:{value}"))
                })
                .unwrap_or_else(|| {
                    format!(
                        "candidate:{:08x}:future_event:{event_index}",
                        candidate.bucket_id
                    )
                });
            if !seen.insert(identity.clone()) {
                continue;
            }
            let mut match_keys = Vec::new();
            if let Some(value) = &event.request_fingerprint {
                match_keys.push(format!("request_fingerprint:{value}"));
            }
            if let Some(value) = &event.exact_cache_key {
                match_keys.push(format!("exact_cache_key:{value}"));
            }
            if let Some(value) = &event.trace_id {
                match_keys.push(format!("trace_id:{value}"));
            }
            match_keys.sort();
            match_keys.dedup();
            let provider_correlation_ready = !match_keys.is_empty();
            summary.rows += 1;
            summary.tokens = summary.tokens.saturating_add(event.tokens);
            summary.current_cost_microusd = summary
                .current_cost_microusd
                .saturating_add(event.cost_microusd);
            summary.ready_for_external_provider_evidence |= provider_correlation_ready;
            let row = serde_json::json!({
                "schema_version": "phase_stream_live_tail_future_shadow_billing_request_v1",
                "billing_request_id": format!(
                    "live-tail-future-shadow-{:08x}-{event_index}",
                    candidate.bucket_id
                ),
                "identity": identity,
                "request_fingerprint": event.request_fingerprint.clone(),
                "exact_cache_key": event.exact_cache_key.clone(),
                "trace_id": event.trace_id.clone(),
                "match_keys": match_keys,
                "provider_correlation_ready": provider_correlation_ready,
                "input_trace_path": event.input_trace_path.clone(),
                "event_timestamp": event.event_timestamp.clone(),
                "route_id": candidate.route_id,
                "bucket_id": candidate.bucket_id,
                "package_fingerprint64": candidate.package.package_info.fingerprint64,
                "threshold_micro": candidate.package.threshold_micro,
                "margin_micro": margin_micro,
                "estimated_total_tokens": event.tokens,
                "current_total_cost_microusd": event.cost_microusd,
                "token_evidence_missing": event.tokens == 0,
                "cost_evidence_missing": event.cost_microusd == 0,
                "verified_safe_accept": true,
                "unique_cpu_accept_over_exact_cache": true,
                "false_accept": false,
                "local_accept_enabled": false,
                "market_money_claim_allowed": false,
                "provider_billing_evidence_present": false,
                "boundary": "future-shadow billing request only: exports verifier-bound .nwpc shadow CPU accepts for external provider billing evidence; does not estimate missing money, promote, serve, enable local_accept, or claim market savings"
            });
            serde_json::to_writer(&mut writer, &row).map_err(|error| {
                format!(
                    "failed to serialize future-shadow billing request '{}': {error}",
                    request_path.display()
                )
            })?;
            writer.write_all(b"\n").map_err(|error| {
                format!(
                    "failed to write future-shadow billing request '{}': {error}",
                    request_path.display()
                )
            })?;
        }
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush future-shadow billing request '{}': {error}",
            request_path.display()
        )
    })?;
    Ok(summary)
}

fn live_store_stable_billing_decision_score_candidate_count(row: &serde_json::Value) -> usize {
    row.get("decisions")
        .and_then(serde_json::Value::as_array)
        .map(|decisions| {
            decisions
                .iter()
                .filter(|decision| {
                    super::super::json_bool(decision, &["score_candidate"]) == Some(true)
                        && super::super::json_bool(decision, &["product_hot_profile_quarantined"])
                            != Some(true)
                })
                .count()
        })
        .unwrap_or(0)
}

fn live_store_decision_row_billing_false_accept_count(row: &serde_json::Value) -> usize {
    if super::super::json_bool(row, &["verified_safe_accept"]).unwrap_or(false)
        || super::super::json_bool(row, &["exact_cache_hit"]).unwrap_or(false)
    {
        0
    } else {
        live_store_stable_billing_decision_score_candidate_count(row)
    }
}

fn live_store_cached_source_row(
    source_path: &Path,
    tail_line_index: usize,
    source_cache: &mut BTreeMap<PathBuf, Vec<serde_json::Value>>,
) -> Result<Option<serde_json::Value>, String> {
    if tail_line_index == 0 {
        return Ok(None);
    }
    if !source_cache.contains_key(source_path) {
        let file = File::open(source_path).map_err(|error| {
            format!(
                "failed to open billing source trace '{}': {error}",
                source_path.display()
            )
        })?;
        let mut rows = Vec::new();
        for line in io::BufReader::new(file).lines() {
            let line = line.map_err(|error| {
                format!(
                    "failed to read billing source trace '{}': {error}",
                    source_path.display()
                )
            })?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) {
                rows.push(row);
            }
        }
        source_cache.insert(source_path.to_path_buf(), rows);
    }
    Ok(source_cache
        .get(source_path)
        .and_then(|rows| rows.get(tail_line_index.saturating_sub(1)))
        .cloned())
}

pub(super) fn write_live_store_stable_clean_suffix_billing_requests(
    request_path: &Path,
    decision_log_path: &Path,
    architecture_key: &str,
) -> Result<LiveStoreFutureShadowBillingRequestSummary, String> {
    if let Some(parent) = request_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create stable-clean billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut clean_suffix_rows = Vec::<serde_json::Value>::new();
    if decision_log_path.exists() {
        let file = File::open(decision_log_path).map_err(|error| {
            format!(
                "failed to open stable-clean billing decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
        for line in io::BufReader::new(file).lines() {
            let line = line.map_err(|error| {
                format!(
                    "failed to read stable-clean billing decision log '{}': {error}",
                    decision_log_path.display()
                )
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if super::super::json_string(&row, &["architecture_version_key"]).as_deref()
                != Some(architecture_key)
            {
                continue;
            }
            if live_store_decision_row_billing_false_accept_count(&row) > 0 {
                clean_suffix_rows.clear();
                continue;
            }
            clean_suffix_rows.push(row);
        }
    }

    let file = File::create(request_path).map_err(|error| {
        format!(
            "failed to create stable-clean billing request '{}': {error}",
            request_path.display()
        )
    })?;
    let mut writer = io::BufWriter::new(file);
    let mut source_cache = BTreeMap::<PathBuf, Vec<serde_json::Value>>::new();
    let mut seen = BTreeSet::<String>::new();
    let mut summary = LiveStoreFutureShadowBillingRequestSummary::default();

    for (row_index, row) in clean_suffix_rows.iter().enumerate() {
        if !super::super::json_bool(row, &["verified_safe_accept"]).unwrap_or(false)
            || super::super::json_bool(row, &["exact_cache_hit"]).unwrap_or(false)
            || super::super::json_u64(row, &["row_unique_cpu_accepts_over_exact_cache"])
                .unwrap_or(0)
                == 0
            || super::super::json_u64(row, &["row_false_accepts"]).unwrap_or(0) > 0
        {
            continue;
        }
        let source_path = super::super::json_string(row, &["source"]).map(PathBuf::from);
        let tail_line_index = super::super::json_u64(row, &["tail_line_index"]).unwrap_or(0);
        let source_row = source_path
            .as_deref()
            .filter(|path| path.exists())
            .and_then(|path| {
                live_store_cached_source_row(path, tail_line_index as usize, &mut source_cache)
                    .ok()
                    .flatten()
            });
        let request_fingerprint =
            super::super::json_string(row, &["request_fingerprint"]).or_else(|| {
                source_row
                    .as_ref()
                    .and_then(|source| super::super::json_string(source, &["request_fingerprint"]))
            });
        let exact_cache_key = super::super::json_string(row, &["exact_cache_key"]).or_else(|| {
            source_row
                .as_ref()
                .and_then(|source| super::super::json_string(source, &["exact_cache_key"]))
        });
        let trace_id = super::super::json_string(row, &["trace_id"]).or_else(|| {
            source_row
                .as_ref()
                .and_then(|source| super::super::json_string(source, &["trace_id"]))
        });
        let input_trace_path =
            super::super::json_string(row, &["input_trace_path"]).or_else(|| {
                source_row
                    .as_ref()
                    .and_then(|source| super::super::json_string(source, &["input_trace_path"]))
            });
        let event_timestamp = super::super::json_string(row, &["event_timestamp"]).or_else(|| {
            source_row
                .as_ref()
                .and_then(|source| super::super::json_string(source, &["event_timestamp"]))
        });
        let mut match_keys = Vec::new();
        if let Some(value) = &request_fingerprint {
            match_keys.push(format!("request_fingerprint:{value}"));
        }
        if let Some(value) = &exact_cache_key {
            match_keys.push(format!("exact_cache_key:{value}"));
        }
        if let Some(value) = &trace_id {
            match_keys.push(format!("trace_id:{value}"));
        }
        match_keys.sort();
        match_keys.dedup();
        let provider_correlation_ready = !match_keys.is_empty();
        let identity = match_keys
            .first()
            .cloned()
            .unwrap_or_else(|| format!("stable_clean_suffix_row:{row_index}"));
        if !seen.insert(identity.clone()) {
            continue;
        }
        let tokens = super::super::json_u64(row, &["tokens"]).unwrap_or(0);
        let cost_microusd = super::super::json_u64(row, &["cost_microusd"]).unwrap_or(tokens);
        let route_id = super::super::json_u64(row, &["route_id"]).unwrap_or_default();
        let bucket_id = super::super::json_u64(row, &["bucket_id"]).unwrap_or_default();

        summary.rows += 1;
        summary.tokens = summary.tokens.saturating_add(tokens);
        summary.current_cost_microusd = summary.current_cost_microusd.saturating_add(cost_microusd);
        summary.ready_for_external_provider_evidence |= provider_correlation_ready;
        let row = serde_json::json!({
            "schema_version": "phase_stream_live_tail_stable_clean_suffix_billing_request_v1",
            "billing_request_id": format!("live-tail-stable-clean-suffix-{row_index}"),
            "identity": identity,
            "request_fingerprint": request_fingerprint,
            "exact_cache_key": exact_cache_key,
            "trace_id": trace_id,
            "match_keys": match_keys,
            "provider_correlation_ready": provider_correlation_ready,
            "input_trace_path": input_trace_path,
            "event_timestamp": event_timestamp,
            "source_trace_path": source_path.map(|path| path.display().to_string()),
            "tail_line_index": tail_line_index,
            "route_id": route_id,
            "bucket_id": bucket_id,
            "estimated_total_tokens": tokens,
            "current_total_cost_microusd": cost_microusd,
            "token_evidence_missing": tokens == 0,
            "cost_evidence_missing": cost_microusd == 0,
            "verified_safe_accept": true,
            "unique_cpu_accept_over_exact_cache": true,
            "false_accept": false,
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "provider_billing_evidence_present": false,
            "boundary": "stable-clean billing request only: exports verifier-bound clean suffix shadow CPU accepts for external provider billing evidence; does not estimate missing money, promote, serve, enable local_accept, or claim market savings"
        });
        serde_json::to_writer(&mut writer, &row).map_err(|error| {
            format!(
                "failed to serialize stable-clean billing request '{}': {error}",
                request_path.display()
            )
        })?;
        writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write stable-clean billing request '{}': {error}",
                request_path.display()
            )
        })?;
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush stable-clean billing request '{}': {error}",
            request_path.display()
        )
    })?;
    Ok(summary)
}

fn live_store_future_shadow_provider_artifact_dir(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("append-live-tail");
    report_path.with_file_name(format!("{stem}-provider-evidence-artifacts"))
}

fn live_store_future_shadow_provider_export_drop_path(report_path: &Path) -> PathBuf {
    live_store_future_shadow_provider_artifact_dir(report_path)
        .join("provider-export.external.jsonl")
}

pub(super) fn live_store_provider_export_file_signature(report_path: &Path) -> (bool, u64, u64) {
    let provider_export_drop_path = live_store_future_shadow_provider_export_drop_path(report_path);
    let Ok(metadata) = std::fs::metadata(provider_export_drop_path) else {
        return (false, 0, 0);
    };
    let modified_secs = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |value| value.as_secs());
    (true, metadata.len(), modified_secs)
}

pub(super) fn live_store_provider_artifact_signature(
    report_path: &Path,
    billing_request: &LiveStoreFutureShadowBillingRequestSummary,
) -> LiveStoreProviderArtifactSignature {
    let (provider_export_present, provider_export_len, provider_export_modified_secs) =
        live_store_provider_export_file_signature(report_path);
    LiveStoreProviderArtifactSignature {
        billing_request_rows: billing_request.rows,
        billing_request_tokens: billing_request.tokens,
        billing_request_cost_microusd: billing_request.current_cost_microusd,
        provider_export_present,
        provider_export_len,
        provider_export_modified_secs,
    }
}

pub(super) fn live_store_provider_money_claim_blocker(
    artifacts: &LiveStoreProviderEvidenceArtifactsReport,
    billing_request: &LiveStoreFutureShadowBillingRequestSummary,
) -> &'static str {
    if billing_request.rows == 0 {
        "no_future_shadow_billing_request_rows"
    } else if !artifacts.provider_export_present {
        "external_provider_export_missing"
    } else if !artifacts.evidence_chain_provider_billing_evidence_present {
        "provider_billing_evidence_missing"
    } else if !artifacts.capture_contract_ready {
        "provider_capture_contract_not_ready"
    } else if !artifacts.market_money_claim_allowed {
        "provider_money_gate_not_allowed"
    } else {
        "none"
    }
}

// Provider export acquisition is a cold/proof task. The live-tail miner must not
// block on it; keep this helper available for a separate evidence worker.
#[allow(dead_code)]
pub(super) fn refresh_live_store_provider_evidence_artifacts(
    report_path: &Path,
    billing_request_path: &Path,
    billing_request: &LiveStoreFutureShadowBillingRequestSummary,
) -> LiveStoreProviderEvidenceArtifactsReport {
    let artifact_dir = live_store_future_shadow_provider_artifact_dir(report_path);
    let provider_export_drop_path = live_store_future_shadow_provider_export_drop_path(report_path);
    let provider_export_attestation_contract_report_path =
        artifact_dir.join("provider-export-attestation-contract.report.json");
    let provider_export_attestation_template_path = PathBuf::from(format!(
        "{}.attestation.template.json",
        provider_export_drop_path.display()
    ));
    let provider_export_present = provider_export_drop_path.is_file();
    let acquisition_report_path = artifact_dir.join("provider-export-acquisition.report.json");
    let acquisition_pack_dir = artifact_dir.join("provider-export-acquisition-pack");
    let acquisition_manifest_jsonl_path =
        acquisition_pack_dir.join("provider-export-acquisition.manifest.jsonl");
    let acquisition_capture_request_jsonl_path =
        acquisition_pack_dir.join("provider-boundary-capture-request.jsonl");
    let acquisition_required_schema_json_path =
        acquisition_pack_dir.join("provider-export-required-schema.json");
    let codex_token_backfill_report_path =
        artifact_dir.join("provider-boundary-codex-token-backfill.report.json");
    let codex_token_backfill_output_provider_boundary_path =
        artifact_dir.join("provider-boundary-codex-token-backfill.provider.jsonl");
    let realtrace_token_cost_backfill_report_path =
        artifact_dir.join("provider-boundary-realtrace-token-cost-backfill.report.json");
    let realtrace_token_cost_backfill_output_provider_boundary_path =
        artifact_dir.join("provider-boundary-realtrace-token-cost-backfill.provider.jsonl");
    let evidence_chain_report_path =
        artifact_dir.join("provider-export-evidence-chain.report.json");
    let evidence_chain_work_dir = artifact_dir.join("provider-export-evidence-chain");
    let capture_contract_report_path =
        artifact_dir.join("provider-billing-capture-contract.report.json");
    let capture_contract_template_jsonl_path =
        artifact_dir.join("provider-billing-capture-contract.template.jsonl");
    let capture_contract_template_csv_path =
        artifact_dir.join("provider-billing-capture-contract.template.csv");

    let mut report = LiveStoreProviderEvidenceArtifactsReport {
        billing_request_rows: billing_request.rows,
        billing_request_tokens: billing_request.tokens,
        billing_request_current_cost_microusd: billing_request.current_cost_microusd,
        provider_export_drop_path: provider_export_drop_path.display().to_string(),
        provider_export_present,
        provider_export_attestation_contract_report_path:
            provider_export_attestation_contract_report_path
                .display()
                .to_string(),
        provider_export_attestation_template_path: provider_export_attestation_template_path
            .display()
            .to_string(),
        provider_export_attestation_contract_refreshed: false,
        acquisition_report_path: acquisition_report_path.display().to_string(),
        acquisition_manifest_jsonl_path: acquisition_manifest_jsonl_path.display().to_string(),
        acquisition_capture_request_jsonl_path: acquisition_capture_request_jsonl_path
            .display()
            .to_string(),
        acquisition_required_schema_json_path: acquisition_required_schema_json_path
            .display()
            .to_string(),
        codex_token_backfill_report_path: codex_token_backfill_report_path.display().to_string(),
        codex_token_backfill_output_provider_boundary_path:
            codex_token_backfill_output_provider_boundary_path
                .display()
                .to_string(),
        realtrace_token_cost_backfill_report_path: realtrace_token_cost_backfill_report_path
            .display()
            .to_string(),
        realtrace_token_cost_backfill_output_provider_boundary_path:
            realtrace_token_cost_backfill_output_provider_boundary_path
                .display()
                .to_string(),
        evidence_chain_report_path: evidence_chain_report_path.display().to_string(),
        capture_contract_report_path: capture_contract_report_path.display().to_string(),
        capture_contract_template_jsonl_path: capture_contract_template_jsonl_path
            .display()
            .to_string(),
        capture_contract_template_csv_path: capture_contract_template_csv_path
            .display()
            .to_string(),
        local_accept_enabled: false,
        market_money_claim_allowed: false,
        boundary: "cold provider-evidence artifact refresh only: consumes verifier-bound .nwpc future-shadow billing worklist and writes acquisition/capture/evidence-chain reports; does not mine, score hot path, promote, serve, local_accept, or claim money",
        ..LiveStoreProviderEvidenceArtifactsReport::default()
    };
    if billing_request.rows == 0 {
        report.refresh_error = Some("no_billing_request_rows".to_owned());
        return report;
    }

    let result = (|| -> Result<(), String> {
        run_phase_stream_provider_export_acquisition_pack_v1(
            vec![
                acquisition_report_path.display().to_string(),
                acquisition_pack_dir.display().to_string(),
                billing_request_path.display().to_string(),
            ]
            .into_iter(),
        )?;
        run_phase_stream_provider_boundary_codex_token_backfill_v1(
            vec![
                codex_token_backfill_report_path.display().to_string(),
                codex_token_backfill_output_provider_boundary_path
                    .display()
                    .to_string(),
                acquisition_capture_request_jsonl_path.display().to_string(),
                billing_request_path.display().to_string(),
            ]
            .into_iter(),
        )?;
        run_phase_stream_provider_boundary_realtrace_token_cost_backfill_v1(
            vec![
                realtrace_token_cost_backfill_report_path
                    .display()
                    .to_string(),
                realtrace_token_cost_backfill_output_provider_boundary_path
                    .display()
                    .to_string(),
                acquisition_capture_request_jsonl_path.display().to_string(),
                billing_request_path.display().to_string(),
            ]
            .into_iter(),
        )?;
        if provider_export_present {
            run_phase_stream_selected_split_nwpc_provider_export_attestation_contract_v1(
                vec![
                    provider_export_attestation_contract_report_path
                        .display()
                        .to_string(),
                    provider_export_drop_path.display().to_string(),
                    provider_export_attestation_template_path
                        .display()
                        .to_string(),
                ]
                .into_iter(),
            )?;
        }
        let mut evidence_chain_args = vec![
            evidence_chain_report_path.display().to_string(),
            evidence_chain_work_dir.display().to_string(),
            billing_request_path.display().to_string(),
            acquisition_capture_request_jsonl_path.display().to_string(),
        ];
        if provider_export_present {
            evidence_chain_args.push(provider_export_drop_path.display().to_string());
        }
        run_phase_stream_provider_export_evidence_chain_v1(evidence_chain_args.into_iter())?;
        run_phase_stream_provider_boundary_billing_capture_contract_v1(
            vec![
                capture_contract_report_path.display().to_string(),
                capture_contract_template_jsonl_path.display().to_string(),
                capture_contract_template_csv_path.display().to_string(),
                acquisition_capture_request_jsonl_path.display().to_string(),
            ]
            .into_iter(),
        )?;
        Ok(())
    })();
    if let Err(error) = result {
        report.refresh_error = Some(error);
        return report;
    }

    report.refreshed = true;
    report.provider_export_attestation_contract_refreshed = provider_export_present;
    if let Ok(acquisition) = super::super::read_json_value(&acquisition_report_path) {
        report.acquisition_worklist_ready = super::super::json_bool(
            &acquisition,
            &["external_provider_collection_worklist_ready"],
        )
        .unwrap_or(false);
        report.acquisition_provider_boundary_correlation_ready =
            super::super::json_bool(&acquisition, &["provider_boundary_correlation_ready"])
                .unwrap_or(false);
        report.acquisition_provider_id_backfill_required =
            super::super::json_bool(&acquisition, &["provider_id_backfill_required"])
                .unwrap_or(false);
        report.acquisition_market_money_claim_allowed =
            super::super::json_bool(&acquisition, &["market_money_claim_allowed"]).unwrap_or(false);
        report.acquisition_verdict =
            super::super::json_string(&acquisition, &["verdict"]).unwrap_or_default();
    }
    if let Ok(codex_backfill) = super::super::read_json_value(&codex_token_backfill_report_path) {
        report.codex_token_backfill_emitted_provider_boundary_rows = super::super::json_u64(
            &codex_backfill,
            &["scoreboard", "emitted_provider_boundary_rows"],
        )
        .unwrap_or(0) as usize;
        report.codex_token_backfill_appended_total_tokens =
            super::super::json_u64(&codex_backfill, &["scoreboard", "appended_total_tokens"])
                .unwrap_or(0);
        report.codex_token_backfill_full_capture_coverage =
            super::super::json_bool(&codex_backfill, &["scoreboard", "full_capture_coverage"])
                .unwrap_or(false);
        report.codex_token_backfill_verdict =
            super::super::json_string(&codex_backfill, &["verdict"]).unwrap_or_default();
    }
    if let Ok(realtrace_backfill) =
        super::super::read_json_value(&realtrace_token_cost_backfill_report_path)
    {
        report.realtrace_token_cost_backfill_emitted_provider_boundary_rows =
            super::super::json_u64(
                &realtrace_backfill,
                &["scoreboard", "emitted_provider_boundary_rows"],
            )
            .unwrap_or(0) as usize;
        report.realtrace_token_cost_backfill_appended_total_tokens = super::super::json_u64(
            &realtrace_backfill,
            &["scoreboard", "appended_total_tokens"],
        )
        .unwrap_or(0);
        report.realtrace_token_cost_backfill_appended_total_cost_microusd = super::super::json_u64(
            &realtrace_backfill,
            &["scoreboard", "appended_total_cost_microusd"],
        )
        .unwrap_or(0);
        report.realtrace_token_cost_backfill_full_capture_coverage = super::super::json_bool(
            &realtrace_backfill,
            &["scoreboard", "full_capture_coverage"],
        )
        .unwrap_or(false);
        report.realtrace_token_cost_backfill_verdict =
            super::super::json_string(&realtrace_backfill, &["verdict"]).unwrap_or_default();
    }
    if let Ok(evidence_chain) = super::super::read_json_value(&evidence_chain_report_path) {
        report.evidence_chain_provider_export_required =
            super::super::json_bool(&evidence_chain, &["provider_export_required"])
                .unwrap_or(false);
        report.evidence_chain_provider_billing_evidence_present =
            super::super::json_bool(&evidence_chain, &["provider_billing_evidence_present"])
                .unwrap_or(false);
        report.evidence_chain_external_ready =
            super::super::json_bool(&evidence_chain, &["external_evidence_chain_ready"])
                .unwrap_or(false);
        report.evidence_chain_market_money_claim_allowed =
            super::super::json_bool(&evidence_chain, &["market_money_claim_allowed"])
                .unwrap_or(false);
        report.evidence_chain_verdict =
            super::super::json_string(&evidence_chain, &["verdict"]).unwrap_or_default();
    }
    if let Ok(capture_contract) = super::super::read_json_value(&capture_contract_report_path) {
        report.capture_contract_ready = super::super::json_bool(
            &capture_contract,
            &[
                "readiness",
                "contract_ready_for_live_provider_boundary_capture",
            ],
        )
        .unwrap_or(false);
        report.capture_contract_market_money_claim_allowed =
            super::super::json_bool(&capture_contract, &["market_money_claim_allowed"])
                .unwrap_or(false);
        report.capture_contract_verdict =
            super::super::json_string(&capture_contract, &["verdict"]).unwrap_or_default();
    }
    report.market_money_claim_allowed = report.acquisition_market_money_claim_allowed
        && report.evidence_chain_market_money_claim_allowed
        && report.capture_contract_market_money_claim_allowed;
    report
}
