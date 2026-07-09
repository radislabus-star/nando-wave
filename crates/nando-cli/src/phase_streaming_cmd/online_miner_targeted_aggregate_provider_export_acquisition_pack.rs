use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use super::online_miner_targeted_aggregate_billing_request::run_phase_stream_online_miner_targeted_aggregate_billing_request_v1;
use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ACQUISITION_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-provider-export-acquisition-pack-v1.report.json";
const DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ACQUISITION_DIR: &str =
    "target/nando-wave/streaming/targeted-aggregate-provider-export-acquisition-pack-v1";
const DEFAULT_TARGETED_AGGREGATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-aggregate-gate-v1-agent-followup-12k-current.report.json";

pub(crate) fn run_phase_stream_online_miner_targeted_aggregate_provider_export_acquisition_pack_v1<
    I,
>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ACQUISITION_REPORT)
    });
    let output_dir = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_TARGETED_AGGREGATE_PROVIDER_EXPORT_ACQUISITION_DIR)
    });
    let aggregate_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_AGGREGATE_REPORT));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    std::fs::create_dir_all(&output_dir).map_err(|error| {
        format!(
            "failed to create targeted aggregate provider export acquisition dir '{}': {error}",
            output_dir.display()
        )
    })?;
    let billing_request_report_path = output_dir.join("aggregate-billing-request.report.json");
    let billing_request_jsonl_path = output_dir.join("aggregate-billing-request.jsonl");
    let acquisition_manifest_jsonl_path =
        output_dir.join("provider-export-acquisition.manifest.jsonl");
    let required_columns_csv_path = output_dir.join("provider-export-required-columns.csv");
    let required_schema_json_path = output_dir.join("provider-export-required-schema.json");

    run_phase_stream_online_miner_targeted_aggregate_billing_request_v1(
        vec![
            billing_request_report_path.display().to_string(),
            billing_request_jsonl_path.display().to_string(),
            aggregate_report_path.display().to_string(),
        ]
        .into_iter(),
    )?;

    let billing_request = read_json_value(&billing_request_report_path)?;
    let mut manifest_writer = BufWriter::new(
        std::fs::File::create(&acquisition_manifest_jsonl_path).map_err(|error| {
            format!(
                "failed to create acquisition manifest '{}': {error}",
                acquisition_manifest_jsonl_path.display()
            )
        })?,
    );
    let file = std::fs::File::open(&billing_request_jsonl_path).map_err(|error| {
        format!(
            "failed to read billing request '{}': {error}",
            billing_request_jsonl_path.display()
        )
    })?;
    let reader = BufReader::new(file);

    let mut request_rows = 0usize;
    let mut request_fingerprint_rows = 0usize;
    let mut exact_cache_key_rows = 0usize;
    let mut rows_with_internal_match_keys = 0usize;
    let mut rows_with_provider_id_keys = 0usize;
    let mut provider_id_key_count = 0usize;
    let mut total_tokens_requiring_billing = 0usize;
    let mut current_known_cost_microusd = 0u64;
    let mut unique_request_fingerprints = BTreeSet::new();
    let mut unique_exact_cache_keys = BTreeSet::new();

    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read billing request '{}' line {}: {error}",
                billing_request_jsonl_path.display(),
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse billing request '{}' line {}: {error}",
                billing_request_jsonl_path.display(),
                line_index + 1
            )
        })?;
        request_rows += 1;
        let request_fingerprint = json_string(&row, &["request_fingerprint"]);
        let exact_cache_key = json_string(&row, &["exact_cache_key"]);
        if let Some(value) = request_fingerprint
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            request_fingerprint_rows += 1;
            unique_request_fingerprints.insert(value.to_owned());
        }
        if let Some(value) = exact_cache_key.as_deref().filter(|value| !value.is_empty()) {
            exact_cache_key_rows += 1;
            unique_exact_cache_keys.insert(value.to_owned());
        }
        let match_keys = string_array(&row, &["match_keys"]);
        rows_with_internal_match_keys += usize::from(!match_keys.is_empty());
        let provider_id_keys = provider_id_match_keys(&row);
        if !provider_id_keys.is_empty() {
            rows_with_provider_id_keys += 1;
            provider_id_key_count = provider_id_key_count.saturating_add(provider_id_keys.len());
        }
        let estimated_total_tokens = json_u64(&row, &["estimated_total_tokens"])
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let current_total_cost_microusd =
            json_u64(&row, &["current_total_cost_microusd"]).unwrap_or(0);
        total_tokens_requiring_billing =
            total_tokens_requiring_billing.saturating_add(estimated_total_tokens);
        current_known_cost_microusd =
            current_known_cost_microusd.saturating_add(current_total_cost_microusd);

        let manifest_row = serde_json::json!({
            "schema_version": "targeted_aggregate_provider_export_acquisition_manifest_v1",
            "billing_request_id": json_string(&row, &["billing_request_id"]),
            "request_fingerprint": request_fingerprint,
            "exact_cache_key": exact_cache_key,
            "join_keys_to_echo_in_provider_export": match_keys,
            "provider_id_keys_already_known": provider_id_keys,
            "provider_export_must_include_one_of": [
                "provider_request_id",
                "provider_response_id",
                "provider_trace_id",
                "external_provider_request_id",
                "openai_request_id",
                "anthropic_request_id",
                "custom_id"
            ],
            "provider_export_must_include_all_of": [
                "billing_evidence_id",
                "billing_source",
                "provider",
                "provider_cost_microusd or provider_cost_usd",
                "provider_total_tokens or input_tokens/output_tokens/cached_input_tokens"
            ],
            "estimated_total_tokens": estimated_total_tokens,
            "current_total_cost_microusd": current_total_cost_microusd,
            "source": json_string(&row, &["source"]),
            "package_fingerprint64": json_u64(&row, &["package_fingerprint64"]),
            "unique_cpu_accept_over_exact_cache": json_bool(&row, &["unique_cpu_accept_over_exact_cache"]).unwrap_or(false),
            "verified_safe_accept": json_bool(&row, &["verified_safe_accept"]).unwrap_or(false),
            "false_accept": json_bool(&row, &["false_accept"]).unwrap_or(false),
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "acquisition manifest row only: tells an external provider-export process which selected request keys must be covered; does not create provider evidence, estimate missing money, promote, serve, or enable local_accept"
        });
        serde_json::to_writer(&mut manifest_writer, &manifest_row).map_err(|error| {
            format!(
                "failed to serialize acquisition manifest '{}': {error}",
                acquisition_manifest_jsonl_path.display()
            )
        })?;
        manifest_writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write acquisition manifest '{}': {error}",
                acquisition_manifest_jsonl_path.display()
            )
        })?;
    }
    manifest_writer.flush().map_err(|error| {
        format!(
            "failed to flush acquisition manifest '{}': {error}",
            acquisition_manifest_jsonl_path.display()
        )
    })?;

    write_required_columns_csv(&required_columns_csv_path)?;
    let required_schema = serde_json::json!({
        "schema_version": "targeted_aggregate_provider_export_required_schema_v1",
        "required_coverage_rows": request_rows,
        "required_join_policy": "Every provider export row must match one selected acquisition row by request_fingerprint/exact_cache_key/match_keys and must carry at least one real provider id key so the billing evidence gate can distinguish external provider evidence from internal request rows.",
        "required_fields": [
            "billing_evidence_id",
            "billing_source",
            "provider",
            "provider_cost_microusd or provider_cost_usd",
            "provider_total_tokens or input_tokens/output_tokens/cached_input_tokens",
            "request_fingerprint or exact_cache_key or match_keys",
            "one provider id: provider_request_id/provider_response_id/provider_trace_id/external_provider_request_id/openai_request_id/anthropic_request_id/custom_id"
        ],
        "forbidden_sources": [
            "synthetic",
            "estimate",
            "request",
            "generated",
            "internal",
            "debug",
            "test",
            "fixture",
            "template"
        ],
        "market_money_claim_allowed": false,
        "local_accept_enabled": false,
        "boundary": "schema contract only: external billing export must still pass normalize/evidence/admission/attestation gates"
    });
    write_json_file(&required_schema_json_path, &required_schema)?;

    let accept_parity = json_bool(&billing_request, &["accept_parity"]).unwrap_or(false);
    let token_parity = json_bool(&billing_request, &["token_parity"]).unwrap_or(false);
    let provider_id_coverage_complete =
        request_rows > 0 && rows_with_provider_id_keys == request_rows;
    let ready_for_provider_export = request_rows > 0
        && accept_parity
        && token_parity
        && rows_with_internal_match_keys == request_rows
        && request_fingerprint_rows == request_rows
        && exact_cache_key_rows == request_rows;
    let verdict = if ready_for_provider_export {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_PROVIDER_EXPORT_ACQUISITION_PACK_V1_READY"
    } else {
        "PHASE_STREAM_ONLINE_MINER_TARGETED_AGGREGATE_PROVIDER_EXPORT_ACQUISITION_PACK_V1_WATCH"
    };
    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_aggregate_provider_export_acquisition_pack_v1",
        "aggregate_report_path": aggregate_report_path,
        "output_dir": output_dir,
        "billing_request_report_path": billing_request_report_path,
        "billing_request_jsonl_path": billing_request_jsonl_path,
        "acquisition_manifest_jsonl_path": acquisition_manifest_jsonl_path,
        "required_columns_csv_path": required_columns_csv_path,
        "required_schema_json_path": required_schema_json_path,
        "billing_request_rows": request_rows,
        "request_fingerprint_rows": request_fingerprint_rows,
        "exact_cache_key_rows": exact_cache_key_rows,
        "rows_with_internal_match_keys": rows_with_internal_match_keys,
        "rows_with_provider_id_keys": rows_with_provider_id_keys,
        "provider_id_key_count": provider_id_key_count,
        "provider_id_coverage_complete": provider_id_coverage_complete,
        "unique_request_fingerprints": unique_request_fingerprints.len(),
        "unique_exact_cache_keys": unique_exact_cache_keys.len(),
        "total_tokens_requiring_billing": total_tokens_requiring_billing,
        "current_known_cost_microusd": current_known_cost_microusd,
        "accept_parity": accept_parity,
        "token_parity": token_parity,
        "ready_for_provider_export": ready_for_provider_export,
        "provider_export_still_required": true,
        "provider_export_attestation_still_required": true,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": false,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": verdict,
        "boundary": "provider export acquisition pack only: exports the selected aggregate billing worklist and required external billing schema; does not create evidence, normalize money, promote, serve, enable local_accept, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_aggregate_provider_export_acquisition_pack_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  acquisition_manifest_jsonl_path: {}",
        acquisition_manifest_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  total_tokens_requiring_billing: {total_tokens_requiring_billing}");
    println!("  rows_with_provider_id_keys: {rows_with_provider_id_keys}");
    println!("  ready_for_provider_export: {ready_for_provider_export}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn provider_id_match_keys(row: &serde_json::Value) -> Vec<String> {
    let mut keys = Vec::new();
    keys.extend(
        string_array(row, &["external_provider_correlation_keys"])
            .into_iter()
            .filter(|key| provider_id_key(key)),
    );
    keys.extend(
        string_array(row, &["match_keys"])
            .into_iter()
            .filter(|key| provider_id_key(key)),
    );
    keys.sort();
    keys.dedup();
    keys
}

fn provider_id_key(key: &str) -> bool {
    key.starts_with("provider_request_id:")
        || key.starts_with("provider_response_id:")
        || key.starts_with("provider_trace_id:")
        || key.starts_with("external_provider_request_id:")
        || key.starts_with("openai_request_id:")
        || key.starts_with("anthropic_request_id:")
        || key.starts_with("custom_id:")
}

fn string_array(row: &serde_json::Value, path: &[&str]) -> Vec<String> {
    let mut current = row;
    for key in path {
        let Some(next) = current.get(*key) else {
            return Vec::new();
        };
        current = next;
    }
    current
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn write_required_columns_csv(path: &Path) -> Result<(), String> {
    let mut file = BufWriter::new(
        std::fs::File::create(path)
            .map_err(|error| format!("failed to create '{}': {error}", path.display()))?,
    );
    file.write_all(
        b"billing_evidence_id,billing_source,provider,provider_cost_microusd,provider_total_tokens,request_fingerprint,exact_cache_key,match_keys,provider_request_id,provider_response_id,provider_trace_id,external_provider_request_id,openai_request_id,anthropic_request_id,custom_id\n",
    )
    .map_err(|error| format!("failed to write '{}': {error}", path.display()))?;
    file.flush()
        .map_err(|error| format!("failed to flush '{}': {error}", path.display()))
}
