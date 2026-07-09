use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{json_bool, json_string, json_u64, write_json_file};

const DEFAULT_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-provider-export-acquisition-pack-v1.report.json";
const DEFAULT_OUTPUT_DIR: &str =
    "target/nando-wave/streaming/provider-export-acquisition-pack-v1-current";
const DEFAULT_BILLING_REQUEST_JSONL: &str = "target/nando-wave/streaming/phase-stream-live-store-clean-manifest-shadow-registry-provider-correlation-backfill-v1-current.jsonl";

pub(crate) fn run_phase_stream_provider_export_acquisition_pack_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT));
    let output_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_DIR));
    let billing_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BILLING_REQUEST_JSONL));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    std::fs::create_dir_all(&output_dir).map_err(|error| {
        format!(
            "failed to create provider export acquisition dir '{}': {error}",
            output_dir.display()
        )
    })?;
    let copied_billing_request_jsonl_path = output_dir.join("billing-request.jsonl");
    let acquisition_manifest_jsonl_path =
        output_dir.join("provider-export-acquisition.manifest.jsonl");
    let capture_request_jsonl_path = output_dir.join("provider-boundary-capture-request.jsonl");
    let required_columns_csv_path = output_dir.join("provider-export-required-columns.csv");
    let required_schema_json_path = output_dir.join("provider-export-required-schema.json");

    let billing_request_bytes = std::fs::read(&billing_request_jsonl_path).map_err(|error| {
        format!(
            "failed to read billing request '{}': {error}",
            billing_request_jsonl_path.display()
        )
    })?;
    let request_file_fingerprint64 = fnv1a64(&billing_request_bytes);
    std::fs::write(&copied_billing_request_jsonl_path, &billing_request_bytes).map_err(
        |error| {
            format!(
                "failed to copy billing request into acquisition pack '{}': {error}",
                copied_billing_request_jsonl_path.display()
            )
        },
    )?;

    let file = std::fs::File::open(&billing_request_jsonl_path).map_err(|error| {
        format!(
            "failed to open billing request '{}': {error}",
            billing_request_jsonl_path.display()
        )
    })?;
    let reader = BufReader::new(file);
    let mut manifest_writer = BufWriter::new(
        std::fs::File::create(&acquisition_manifest_jsonl_path).map_err(|error| {
            format!(
                "failed to create acquisition manifest '{}': {error}",
                acquisition_manifest_jsonl_path.display()
            )
        })?,
    );
    let mut capture_writer = BufWriter::new(
        std::fs::File::create(&capture_request_jsonl_path).map_err(|error| {
            format!(
                "failed to create provider-boundary capture request '{}': {error}",
                capture_request_jsonl_path.display()
            )
        })?,
    );

    let mut request_rows = 0usize;
    let mut capture_request_rows = 0usize;
    let mut request_fingerprint_rows = 0usize;
    let mut exact_cache_key_rows = 0usize;
    let mut trace_id_rows = 0usize;
    let mut rows_with_match_keys = 0usize;
    let mut rows_with_external_provider_correlation_keys = 0usize;
    let mut rows_with_provider_id_keys = 0usize;
    let mut provider_id_key_count = 0usize;
    let mut provider_correlation_ready_rows = 0usize;
    let mut provider_request_id_ready_rows = 0usize;
    let mut verified_safe_rows = 0usize;
    let mut unique_cpu_accept_rows = 0usize;
    let mut false_accept_rows = 0usize;
    let mut local_accept_rows = 0usize;
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
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse billing request '{}' line {}: {error}",
                billing_request_jsonl_path.display(),
                line_index + 1
            )
        })?;
        request_rows += 1;
        if json_bool(&row, &["verified_safe_accept"]).unwrap_or(false) {
            verified_safe_rows += 1;
        }
        if json_bool(&row, &["unique_cpu_accept_over_exact_cache"]).unwrap_or(false) {
            unique_cpu_accept_rows += 1;
        }
        if json_bool(&row, &["false_accept"]).unwrap_or(false) {
            false_accept_rows += 1;
        }
        if json_bool(&row, &["local_accept_enabled"]).unwrap_or(false) {
            local_accept_rows += 1;
        }
        if json_bool(&row, &["provider_correlation_ready"]).unwrap_or(false) {
            provider_correlation_ready_rows += 1;
        }
        if json_bool(&row, &["provider_request_id_ready"]).unwrap_or(false) {
            provider_request_id_ready_rows += 1;
        }

        let request_fingerprint = json_string(&row, &["request_fingerprint"]);
        let exact_cache_key = json_string(&row, &["exact_cache_key"]);
        let trace_id = json_string(&row, &["trace_id"]);
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
        if trace_id.as_deref().is_some_and(|value| !value.is_empty()) {
            trace_id_rows += 1;
        }

        let match_keys = string_array(&row, &["match_keys"]);
        if !match_keys.is_empty() {
            rows_with_match_keys += 1;
        }
        let external_provider_correlation_keys =
            string_array(&row, &["external_provider_correlation_keys"]);
        if !external_provider_correlation_keys.is_empty() {
            rows_with_external_provider_correlation_keys += 1;
        }
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
            "schema_version": "provider_export_acquisition_manifest_v1",
            "request_file_fingerprint64": request_file_fingerprint64,
            "billing_request_id": json_string(&row, &["billing_request_id"]),
            "request_fingerprint": request_fingerprint,
            "exact_cache_key": exact_cache_key,
            "trace_id": trace_id,
            "join_keys_to_echo_in_provider_export": match_keys,
            "request_side_provider_correlation_keys": external_provider_correlation_keys,
            "request_side_provider_id_keys": provider_id_keys,
            "provider_export_must_include_one_join_key": true,
            "provider_export_must_include_one_real_provider_id": true,
            "provider_export_real_provider_id_fields": [
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
            "verified_safe_accept": json_bool(&row, &["verified_safe_accept"]).unwrap_or(false),
            "unique_cpu_accept_over_exact_cache": json_bool(&row, &["unique_cpu_accept_over_exact_cache"]).unwrap_or(false),
            "false_accept": json_bool(&row, &["false_accept"]).unwrap_or(false),
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "acquisition manifest row only: tells an external provider-export process which verifier-bound .nwpc billing request must be covered; does not create provider evidence, estimate missing money, promote, serve, or enable local_accept"
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

        let capture_request_id = json_string(&row, &["billing_request_id"]).unwrap_or_else(|| {
            format!(
                "provider_capture_request_{:016x}",
                fnv1a64(trimmed.as_bytes())
            )
        });
        let capture_row = serde_json::json!({
            "schema_version": "provider_boundary_capture_request_v1",
            "capture_request_id": capture_request_id,
            "billing_request_id": json_string(&row, &["billing_request_id"]),
            "primary_join_key": match_keys.first().cloned(),
            "join_keys": match_keys,
            "phase_row_count": 1,
            "total_tokens": estimated_total_tokens,
            "total_cost_microusd": current_total_cost_microusd,
            "token_cost_estimate_rows": usize::from(json_bool(&row, &["token_cost_estimate_used"]).unwrap_or(false)),
            "token_evidence_missing_rows": 0,
            "cost_evidence_missing_rows": 0,
            "provider_capture_required": true,
            "provider_correlation_metadata_only_required": true,
            "must_not_emit_provider_keys_as_atoms": true,
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "boundary": "provider-boundary capture request derived from verifier-bound .nwpc billing request; lets external provider export rows join back by request keys, but does not contain provider ids or runtime authority"
        });
        serde_json::to_writer(&mut capture_writer, &capture_row).map_err(|error| {
            format!(
                "failed to serialize provider-boundary capture request '{}': {error}",
                capture_request_jsonl_path.display()
            )
        })?;
        capture_writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write provider-boundary capture request '{}': {error}",
                capture_request_jsonl_path.display()
            )
        })?;
        capture_request_rows += 1;
    }
    manifest_writer.flush().map_err(|error| {
        format!(
            "failed to flush acquisition manifest '{}': {error}",
            acquisition_manifest_jsonl_path.display()
        )
    })?;
    capture_writer.flush().map_err(|error| {
        format!(
            "failed to flush provider-boundary capture request '{}': {error}",
            capture_request_jsonl_path.display()
        )
    })?;

    write_required_columns_csv(&required_columns_csv_path)?;
    let required_schema = serde_json::json!({
        "schema_version": "provider_export_required_schema_v1",
        "request_file_fingerprint64": request_file_fingerprint64,
        "required_coverage_rows": request_rows,
        "required_join_policy": "Every external provider export row must cover one acquisition manifest row by request_fingerprint, exact_cache_key, trace_id, or a listed join key. It must also carry one real provider id so provider-boundary ingest/backfill can attach provider id coverage before money is claimed.",
        "required_fields": [
            "billing_evidence_id",
            "billing_source",
            "provider",
            "provider_cost_microusd or provider_cost_usd",
            "provider_total_tokens or input_tokens/output_tokens/cached_input_tokens",
            "one join key: request_fingerprint/exact_cache_key/trace_id/match_keys",
            "one real provider id: provider_request_id/provider_response_id/provider_trace_id/external_provider_request_id/openai_request_id/anthropic_request_id/custom_id"
        ],
        "required_after_export_steps": [
            "phase-stream-provider-boundary-export-ingest-v1",
            "phase-stream-online-miner-portfolio-billing-request-provider-correlation-backfill-v1",
            "phase-stream-online-miner-portfolio-provider-export-normalize-v1",
            "phase-stream-online-miner-portfolio-billing-evidence-gate-v1"
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
        "boundary": "schema contract only: external billing export must still pass ingest/backfill/normalize/evidence gates"
    });
    write_json_file(&required_schema_json_path, &required_schema)?;

    let external_provider_collection_worklist_ready = request_rows > 0
        && verified_safe_rows == request_rows
        && unique_cpu_accept_rows == request_rows
        && false_accept_rows == 0
        && local_accept_rows == 0
        && request_fingerprint_rows == request_rows
        && exact_cache_key_rows == request_rows
        && rows_with_match_keys == request_rows;
    let provider_boundary_correlation_ready = external_provider_collection_worklist_ready
        && provider_correlation_ready_rows == request_rows
        && rows_with_external_provider_correlation_keys == request_rows;
    let ready_for_external_provider_export =
        external_provider_collection_worklist_ready && provider_boundary_correlation_ready;
    let provider_id_backfill_required =
        request_rows > 0 && provider_request_id_ready_rows < request_rows;
    let verdict = if ready_for_external_provider_export {
        "PHASE_STREAM_PROVIDER_EXPORT_ACQUISITION_PACK_V1_READY"
    } else if external_provider_collection_worklist_ready {
        "PHASE_STREAM_PROVIDER_EXPORT_ACQUISITION_PACK_V1_READY_FOR_EXTERNAL_COLLECTION"
    } else {
        "PHASE_STREAM_PROVIDER_EXPORT_ACQUISITION_PACK_V1_WATCH"
    };

    let mut report_map = serde_json::Map::new();
    report_map.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_provider_export_acquisition_pack_v1"),
    );
    report_map.insert(
        "source_billing_request_jsonl_path".to_owned(),
        serde_json::json!(billing_request_jsonl_path.display().to_string()),
    );
    report_map.insert(
        "copied_billing_request_jsonl_path".to_owned(),
        serde_json::json!(copied_billing_request_jsonl_path.display().to_string()),
    );
    report_map.insert(
        "output_dir".to_owned(),
        serde_json::json!(output_dir.display().to_string()),
    );
    report_map.insert(
        "acquisition_manifest_jsonl_path".to_owned(),
        serde_json::json!(acquisition_manifest_jsonl_path.display().to_string()),
    );
    report_map.insert(
        "provider_boundary_capture_request_jsonl_path".to_owned(),
        serde_json::json!(capture_request_jsonl_path.display().to_string()),
    );
    report_map.insert(
        "required_columns_csv_path".to_owned(),
        serde_json::json!(required_columns_csv_path.display().to_string()),
    );
    report_map.insert(
        "required_schema_json_path".to_owned(),
        serde_json::json!(required_schema_json_path.display().to_string()),
    );
    report_map.insert(
        "request_file_fingerprint64".to_owned(),
        serde_json::json!(request_file_fingerprint64),
    );
    report_map.insert(
        "billing_request_rows".to_owned(),
        serde_json::json!(request_rows),
    );
    report_map.insert(
        "provider_boundary_capture_request_rows".to_owned(),
        serde_json::json!(capture_request_rows),
    );
    report_map.insert(
        "verified_safe_rows".to_owned(),
        serde_json::json!(verified_safe_rows),
    );
    report_map.insert(
        "unique_cpu_accept_rows".to_owned(),
        serde_json::json!(unique_cpu_accept_rows),
    );
    report_map.insert(
        "false_accept_rows".to_owned(),
        serde_json::json!(false_accept_rows),
    );
    report_map.insert(
        "local_accept_rows".to_owned(),
        serde_json::json!(local_accept_rows),
    );
    report_map.insert(
        "request_fingerprint_rows".to_owned(),
        serde_json::json!(request_fingerprint_rows),
    );
    report_map.insert(
        "exact_cache_key_rows".to_owned(),
        serde_json::json!(exact_cache_key_rows),
    );
    report_map.insert("trace_id_rows".to_owned(), serde_json::json!(trace_id_rows));
    report_map.insert(
        "rows_with_match_keys".to_owned(),
        serde_json::json!(rows_with_match_keys),
    );
    report_map.insert(
        "rows_with_external_provider_correlation_keys".to_owned(),
        serde_json::json!(rows_with_external_provider_correlation_keys),
    );
    report_map.insert(
        "provider_correlation_ready_rows".to_owned(),
        serde_json::json!(provider_correlation_ready_rows),
    );
    report_map.insert(
        "provider_request_id_ready_rows".to_owned(),
        serde_json::json!(provider_request_id_ready_rows),
    );
    report_map.insert(
        "rows_with_provider_id_keys".to_owned(),
        serde_json::json!(rows_with_provider_id_keys),
    );
    report_map.insert(
        "provider_id_key_count".to_owned(),
        serde_json::json!(provider_id_key_count),
    );
    report_map.insert(
        "provider_id_backfill_required".to_owned(),
        serde_json::json!(provider_id_backfill_required),
    );
    report_map.insert(
        "unique_request_fingerprints".to_owned(),
        serde_json::json!(unique_request_fingerprints.len()),
    );
    report_map.insert(
        "unique_exact_cache_keys".to_owned(),
        serde_json::json!(unique_exact_cache_keys.len()),
    );
    report_map.insert(
        "total_tokens_requiring_billing".to_owned(),
        serde_json::json!(total_tokens_requiring_billing),
    );
    report_map.insert(
        "current_known_cost_microusd".to_owned(),
        serde_json::json!(current_known_cost_microusd),
    );
    report_map.insert(
        "ready_for_external_provider_export".to_owned(),
        serde_json::json!(ready_for_external_provider_export),
    );
    report_map.insert(
        "external_provider_collection_worklist_ready".to_owned(),
        serde_json::json!(external_provider_collection_worklist_ready),
    );
    report_map.insert(
        "provider_boundary_correlation_ready".to_owned(),
        serde_json::json!(provider_boundary_correlation_ready),
    );
    report_map.insert(
        "provider_export_still_required".to_owned(),
        serde_json::json!(true),
    );
    report_map.insert(
        "provider_export_attestation_still_required".to_owned(),
        serde_json::json!(true),
    );
    report_map.insert(
        "provider_billing_evidence_present".to_owned(),
        serde_json::json!(false),
    );
    report_map.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report_map.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report_map.insert(
        "serving_registry_mutated".to_owned(),
        serde_json::json!(false),
    );
    report_map.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
    );
    report_map.insert(
        "forbidden_flags".to_owned(),
        serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        }),
    );
    report_map.insert(
        "next_required_external_step".to_owned(),
        serde_json::json!({
            "need_real_external_provider_export": true,
            "need_provider_id_or_call_id": true,
            "need_positive_provider_cost_and_tokens": true,
            "need_attestation": true,
            "collection_worklist_ready": external_provider_collection_worklist_ready,
            "provider_boundary_correlation_ready": provider_boundary_correlation_ready,
            "policy": "Do not run money claim on request-only, template, synthetic, internal, or estimated rows."
        }),
    );
    report_map.insert("verdict".to_owned(), serde_json::json!(verdict));
    report_map.insert(
        "boundary".to_owned(),
        serde_json::json!("provider export acquisition pack only: exports a verifier-bound .nwpc billing worklist and schema for external provider billing collection; does not create evidence, estimate missing money, promote, serve, enable local_accept, or use legacy nwrb"),
    );
    let report = Value::Object(report_map);
    write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_export_acquisition_pack_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  acquisition_manifest_jsonl_path: {}",
        acquisition_manifest_jsonl_path.display()
    );
    println!(
        "  provider_boundary_capture_request_jsonl_path: {}",
        capture_request_jsonl_path.display()
    );
    println!("  billing_request_rows: {request_rows}");
    println!("  provider_boundary_capture_request_rows: {capture_request_rows}");
    println!("  request_file_fingerprint64: {request_file_fingerprint64}");
    println!("  provider_request_id_ready_rows: {provider_request_id_ready_rows}");
    println!("  provider_id_backfill_required: {provider_id_backfill_required}");
    println!(
        "  external_provider_collection_worklist_ready: {external_provider_collection_worklist_ready}"
    );
    println!("  provider_boundary_correlation_ready: {provider_boundary_correlation_ready}");
    println!("  ready_for_external_provider_export: {ready_for_external_provider_export}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn provider_id_match_keys(row: &Value) -> Vec<String> {
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

fn string_array(row: &Value, path: &[&str]) -> Vec<String> {
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
        .filter_map(Value::as_str)
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
        b"billing_evidence_id,billing_source,provider,provider_cost_microusd,provider_total_tokens,request_fingerprint,exact_cache_key,trace_id,match_keys,provider_request_id,provider_response_id,provider_trace_id,external_provider_request_id,openai_request_id,anthropic_request_id,custom_id\n",
    )
    .map_err(|error| format!("failed to write '{}': {error}", path.display()))?;
    file.flush()
        .map_err(|error| format!("failed to flush '{}': {error}", path.display()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
