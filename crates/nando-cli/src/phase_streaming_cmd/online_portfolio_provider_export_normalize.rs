use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_PROVIDER_EXPORT_NORMALIZE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-provider-export-normalize-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1.jsonl";
const DEFAULT_PROVIDER_EXPORT_NORMALIZED_EVIDENCE_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-provider-export-normalized-evidence-v1.jsonl";

#[derive(Clone, Debug)]
struct BillingRequestKeyIndex {
    request_file_fingerprint64: u64,
    request_rows: usize,
    key_to_request_indexes: BTreeMap<String, BTreeSet<usize>>,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_provider_export_normalize_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_EXPORT_NORMALIZE_REPORT));
    let billing_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_JSONL));
    let provider_export_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "external provider export JSONL path is required".to_owned())?;
    let evidence_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_EXPORT_NORMALIZED_EVIDENCE_JSONL));

    let request_index = read_billing_request_key_index(&billing_request_jsonl_path)?;
    let provider_export_text =
        std::fs::read_to_string(&provider_export_jsonl_path).map_err(|error| {
            format!(
                "failed to read provider export '{}': {error}",
                provider_export_jsonl_path.display()
            )
        })?;
    let (provider_rows, provider_export_format) =
        parse_provider_export_rows(&provider_export_jsonl_path, &provider_export_text)?;

    if let Some(parent) = evidence_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create normalized evidence dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let evidence_file = std::fs::File::create(&evidence_jsonl_path).map_err(|error| {
        format!(
            "failed to create normalized evidence '{}': {error}",
            evidence_jsonl_path.display()
        )
    })?;
    let mut evidence_writer = BufWriter::new(evidence_file);

    let mut provider_export_rows = 0usize;
    let mut rows_with_match_keys = 0usize;
    let mut rows_with_positive_cost = 0usize;
    let mut rows_with_positive_tokens = 0usize;
    let mut rows_with_billing_evidence_id = 0usize;
    let mut rows_with_provider = 0usize;
    let mut rows_with_billing_source = 0usize;
    let mut skipped_unmatched_rows = 0usize;
    let mut skipped_ambiguous_request_match_rows = 0usize;
    let mut skipped_missing_or_invalid_cost_rows = 0usize;
    let mut skipped_missing_or_invalid_token_rows = 0usize;
    let mut skipped_missing_billing_evidence_id_rows = 0usize;
    let mut skipped_missing_provider_rows = 0usize;
    let mut skipped_missing_billing_source_rows = 0usize;
    let mut normalized_evidence_rows = 0usize;
    let mut normalized_matched_request_indexes = BTreeSet::<usize>::new();
    let mut duplicate_normalized_request_rows = 0usize;
    let mut normalized_rows_with_provider_correlation_keys = 0usize;
    let mut cost_usd_converted_rows = 0usize;
    let mut token_sum_derived_rows = 0usize;
    let mut normalized_provider_cost_microusd = 0u64;
    let mut normalized_provider_total_tokens = 0usize;

    for (line_index, row) in provider_rows {
        provider_export_rows += 1;

        let match_keys = match_keys(&row);
        if !match_keys.is_empty() {
            rows_with_match_keys += 1;
        }
        let matched_request_indexes = matched_request_indexes(&request_index, &match_keys);
        if matched_request_indexes.is_empty() {
            skipped_unmatched_rows += 1;
            continue;
        }
        if matched_request_indexes.len() > 1 {
            skipped_ambiguous_request_match_rows += 1;
            continue;
        }
        let request_index_value = *matched_request_indexes.iter().next().expect("one match");

        let Some((cost, cost_source)) = provider_cost_microusd(&row) else {
            skipped_missing_or_invalid_cost_rows += 1;
            continue;
        };
        if cost == 0 {
            skipped_missing_or_invalid_cost_rows += 1;
            continue;
        }
        rows_with_positive_cost += 1;
        if cost_source == "usd" {
            cost_usd_converted_rows += 1;
        }

        let Some((tokens, token_source)) = provider_total_tokens(&row) else {
            skipped_missing_or_invalid_token_rows += 1;
            continue;
        };
        if tokens == 0 {
            skipped_missing_or_invalid_token_rows += 1;
            continue;
        }
        rows_with_positive_tokens += 1;
        if token_source == "component_sum" {
            token_sum_derived_rows += 1;
        }

        let Some(billing_evidence_id) = json_string_any(
            &row,
            &[&["billing_evidence_id"], &["id"], &["provider_row_id"]],
        )
        .filter(|value| !value.is_empty()) else {
            skipped_missing_billing_evidence_id_rows += 1;
            continue;
        };
        rows_with_billing_evidence_id += 1;

        let Some(provider) = json_string_any(&row, &[&["provider"], &["model_provider"]])
            .filter(|value| !value.is_empty())
        else {
            skipped_missing_provider_rows += 1;
            continue;
        };
        rows_with_provider += 1;

        let Some(billing_source) = json_string_any(
            &row,
            &[
                &["billing_source"],
                &["provider_billing_evidence_source"],
                &["source"],
                &["export_source"],
            ],
        )
        .filter(|value| !value.is_empty()) else {
            skipped_missing_billing_source_rows += 1;
            continue;
        };
        rows_with_billing_source += 1;

        if !normalized_matched_request_indexes.insert(request_index_value) {
            duplicate_normalized_request_rows += 1;
        }

        let matched_keys = match_keys
            .into_iter()
            .filter(|key| {
                request_index
                    .key_to_request_indexes
                    .get(key)
                    .is_some_and(|indexes| indexes.contains(&request_index_value))
            })
            .collect::<Vec<_>>();
        let external_provider_correlation_keys = provider_correlation_match_keys(&matched_keys);
        let provider_correlation_ready = !external_provider_correlation_keys.is_empty();
        if provider_correlation_ready {
            normalized_rows_with_provider_correlation_keys += 1;
        }

        let evidence_row = serde_json::json!({
            "schema_version": "provider_billing_evidence_v1",
            "billing_evidence_id": billing_evidence_id,
            "billing_source": billing_source,
            "provider": provider,
            "provider_cost_microusd": cost,
            "provider_total_tokens": tokens,
            "match_keys": matched_keys,
            "external_provider_correlation_keys": external_provider_correlation_keys,
            "provider_correlation_ready": provider_correlation_ready,
            "request_fingerprint": matched_keys
                .iter()
                .find_map(|key| key.strip_prefix("request_fingerprint:")),
            "exact_cache_key": matched_keys
                .iter()
                .find_map(|key| key.strip_prefix("exact_cache_key:")),
            "request_file_fingerprint64": request_index.request_file_fingerprint64,
            "provider_export_jsonl_path": provider_export_jsonl_path,
            "provider_export_line_index": line_index,
            "market_money_claim_allowed": false,
            "boundary": "normalized external provider billing evidence candidate: must still pass billing evidence gate and admission before any product claim; does not compile, promote, serve, estimate missing cost, or enable local_accept"
        });
        serde_json::to_writer(&mut evidence_writer, &evidence_row).map_err(|error| {
            format!(
                "failed to serialize normalized evidence '{}': {error}",
                evidence_jsonl_path.display()
            )
        })?;
        evidence_writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write normalized evidence '{}': {error}",
                evidence_jsonl_path.display()
            )
        })?;
        normalized_evidence_rows += 1;
        normalized_provider_cost_microusd = normalized_provider_cost_microusd.saturating_add(cost);
        normalized_provider_total_tokens = normalized_provider_total_tokens.saturating_add(tokens);
    }
    evidence_writer.flush().map_err(|error| {
        format!(
            "failed to flush normalized evidence '{}': {error}",
            evidence_jsonl_path.display()
        )
    })?;

    let matched_request_rows = normalized_matched_request_indexes.len();
    let missing_request_rows = request_index
        .request_rows
        .saturating_sub(matched_request_rows);
    let verdict = if request_index.request_rows > 0
        && normalized_evidence_rows > 0
        && matched_request_rows == request_index.request_rows
        && duplicate_normalized_request_rows == 0
    {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_NORMALIZE_V1_READY_FOR_EVIDENCE_GATE"
    } else if normalized_evidence_rows > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_NORMALIZE_V1_PARTIAL"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_PROVIDER_EXPORT_NORMALIZE_V1_WATCH_NO_NORMALIZED_EVIDENCE"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_provider_export_normalize_v1",
        "billing_request_jsonl_path": billing_request_jsonl_path,
        "provider_export_jsonl_path": provider_export_jsonl_path,
        "provider_export_format": provider_export_format,
        "normalized_evidence_jsonl_path": evidence_jsonl_path,
        "request_rows": request_index.request_rows,
        "request_file_fingerprint64": request_index.request_file_fingerprint64,
        "request_match_keys": request_index.key_to_request_indexes.len(),
        "provider_export_rows": provider_export_rows,
        "rows_with_match_keys": rows_with_match_keys,
        "rows_with_positive_cost": rows_with_positive_cost,
        "rows_with_positive_tokens": rows_with_positive_tokens,
        "rows_with_billing_evidence_id": rows_with_billing_evidence_id,
        "rows_with_provider": rows_with_provider,
        "rows_with_billing_source": rows_with_billing_source,
        "normalized_evidence_rows": normalized_evidence_rows,
        "normalized_matched_request_rows": matched_request_rows,
        "normalized_rows_with_provider_correlation_keys": normalized_rows_with_provider_correlation_keys,
        "missing_request_rows": missing_request_rows,
        "duplicate_normalized_request_rows": duplicate_normalized_request_rows,
        "skipped_unmatched_rows": skipped_unmatched_rows,
        "skipped_ambiguous_request_match_rows": skipped_ambiguous_request_match_rows,
        "skipped_missing_or_invalid_cost_rows": skipped_missing_or_invalid_cost_rows,
        "skipped_missing_or_invalid_token_rows": skipped_missing_or_invalid_token_rows,
        "skipped_missing_billing_evidence_id_rows": skipped_missing_billing_evidence_id_rows,
        "skipped_missing_provider_rows": skipped_missing_provider_rows,
        "skipped_missing_billing_source_rows": skipped_missing_billing_source_rows,
        "cost_usd_converted_rows": cost_usd_converted_rows,
        "token_sum_derived_rows": token_sum_derived_rows,
        "provider_cost_microusd": normalized_provider_cost_microusd,
        "provider_total_tokens": normalized_provider_total_tokens,
        "normalizer": {
            "adapter_only": true,
            "requires_evidence_gate": true,
            "requires_admission_gate": true,
            "market_money_claim_allowed": false,
            "local_accept_enabled": false,
            "provider_correlation_metadata_only": true,
            "policy": "normalizes external provider export rows into provider_billing_evidence_v1 rows keyed to the selected request set; provider correlation keys remain metadata for billing join and must not enter phase atoms; gate/admission remain authoritative"
        },
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
        "boundary": "provider export normalization only: does not validate final completeness, compile, promote, serve, enable local_accept, estimate missing money, or revive legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_provider_export_normalize_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  normalized_evidence_jsonl_path: {}",
        evidence_jsonl_path.display()
    );
    println!("  request_rows: {}", request_index.request_rows);
    println!("  normalized_evidence_rows: {normalized_evidence_rows}");
    println!("  normalized_matched_request_rows: {matched_request_rows}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_billing_request_key_index(path: &Path) -> Result<BillingRequestKeyIndex, String> {
    let request_bytes = std::fs::read(path).map_err(|error| {
        format!(
            "failed to read billing request '{}': {error}",
            path.display()
        )
    })?;
    let request_file_fingerprint64 = fnv1a64(&request_bytes);
    let request_text = std::str::from_utf8(&request_bytes)
        .map_err(|error| format!("billing request '{}' is not UTF-8: {error}", path.display()))?;
    let mut request_rows = 0usize;
    let mut key_to_request_indexes = BTreeMap::<String, BTreeSet<usize>>::new();
    for (line_index, line) in request_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse billing request '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        let current_request_index = request_rows;
        request_rows += 1;
        for key in match_keys(&row) {
            key_to_request_indexes
                .entry(key)
                .or_default()
                .insert(current_request_index);
        }
    }
    Ok(BillingRequestKeyIndex {
        request_file_fingerprint64,
        request_rows,
        key_to_request_indexes,
    })
}

fn matched_request_indexes(
    request_index: &BillingRequestKeyIndex,
    keys: &[String],
) -> BTreeSet<usize> {
    let mut matched = BTreeSet::new();
    for key in keys {
        if let Some(indexes) = request_index.key_to_request_indexes.get(key) {
            matched.extend(indexes.iter().copied());
        }
    }
    matched
}

fn parse_provider_export_rows(
    path: &Path,
    text: &str,
) -> Result<(Vec<(usize, Value)>, &'static str), String> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
    {
        return parse_delimited_provider_export_rows(path, text, ',', "csv");
    }
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("tsv"))
    {
        return parse_delimited_provider_export_rows(path, text, '\t', "tsv");
    }
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse provider export '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        rows.push((line_index, row));
    }
    Ok((rows, "jsonl"))
}

fn parse_delimited_provider_export_rows(
    path: &Path,
    text: &str,
    delimiter: char,
    format_name: &'static str,
) -> Result<(Vec<(usize, Value)>, &'static str), String> {
    let mut lines = text.lines().enumerate();
    let Some((header_index, header_line)) = lines.find(|(_, line)| !line.trim().is_empty()) else {
        return Ok((Vec::new(), format_name));
    };
    let headers = parse_delimited_line(header_line, delimiter).map_err(|error| {
        format!(
            "failed to parse provider export '{}' header line {}: {error}",
            path.display(),
            header_index + 1
        )
    })?;
    let mut rows = Vec::new();
    for (line_index, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        let values = parse_delimited_line(line, delimiter).map_err(|error| {
            format!(
                "failed to parse provider export '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if values.len() != headers.len() {
            return Err(format!(
                "provider export '{}' line {} has {} fields, expected {}",
                path.display(),
                line_index + 1,
                values.len(),
                headers.len()
            ));
        }
        let mut object = serde_json::Map::new();
        for (header, value) in headers.iter().zip(values) {
            let key = header.trim();
            if key.is_empty() {
                continue;
            }
            object.insert(key.to_owned(), csv_value_to_json(key, &value));
        }
        rows.push((line_index, Value::Object(object)));
    }
    Ok((rows, format_name))
}

fn parse_delimited_line(line: &str, delimiter: char) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            if current.is_empty() {
                in_quotes = true;
            } else {
                return Err("quote found inside unquoted field".to_owned());
            }
        } else if ch == delimiter {
            fields.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    if in_quotes {
        return Err("unterminated quoted field".to_owned());
    }
    fields.push(current.trim().to_owned());
    Ok(fields)
}

fn csv_value_to_json(key: &str, value: &str) -> Value {
    let trimmed = value.trim();
    if key == "match_keys" {
        return Value::Array(
            split_match_key_text(trimmed)
                .into_iter()
                .map(Value::String)
                .collect(),
        );
    }
    if trimmed.is_empty() {
        Value::Null
    } else if let Ok(number) = trimmed.parse::<u64>() {
        serde_json::json!(number)
    } else if let Ok(number) = trimmed.parse::<i64>() {
        serde_json::json!(number)
    } else if let Ok(number) = trimmed.parse::<f64>() {
        serde_json::json!(number)
    } else {
        Value::String(trimmed.to_owned())
    }
}

fn provider_cost_microusd(row: &Value) -> Option<(u64, &'static str)> {
    for path in [
        &["provider_cost_microusd"][..],
        &["cost_microusd"],
        &["total_cost_microusd"],
        &["token_cost", "provider_cost_microusd"],
        &["token_cost", "total_cost_microusd"],
    ] {
        if let Some(value) = json_u64(row, path) {
            return Some((value, "microusd"));
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
                return Some(((value * 1_000_000.0).round() as u64, "usd"));
            }
        }
    }
    None
}

fn provider_total_tokens(row: &Value) -> Option<(usize, &'static str)> {
    for path in [
        &["provider_total_tokens"][..],
        &["total_tokens"],
        &["tokens_total"],
        &["token_cost", "total_tokens"],
    ] {
        if let Some(value) = json_usize(row, path) {
            return Some((value, "total"));
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
        Some((component_sum, "component_sum"))
    } else {
        None
    }
}

fn match_keys(row: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    keys.extend(super::phase_atom_external_provider_correlation_keys(row));
    if let Some(array) = json_at(row, &["match_keys"]).and_then(Value::as_array) {
        for value in array {
            if let Some(key) = value.as_str().filter(|key| !key.is_empty()) {
                keys.push(key.to_owned());
            }
        }
    }
    if let Some(array) =
        json_at(row, &["external_provider_correlation_keys"]).and_then(Value::as_array)
    {
        for value in array {
            if let Some(key) = value.as_str().filter(|key| !key.is_empty()) {
                keys.push(key.to_owned());
            }
        }
    }
    if let Some(value) = json_string(row, &["match_keys"]) {
        keys.extend(split_match_key_text(&value));
    }
    push_match_key_paths(
        &mut keys,
        row,
        "request_fingerprint",
        &[
            &["request_fingerprint"],
            &["custom_id"],
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
        ],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "trace_id",
        &[&["trace_id"], &["metadata", "trace_id"], &["request_id"]],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "provider_request_id",
        &[
            &["provider_request_id"],
            &["metadata", "provider_request_id"],
        ],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "provider_response_id",
        &[
            &["provider_response_id"],
            &["metadata", "provider_response_id"],
        ],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "provider_trace_id",
        &[&["provider_trace_id"], &["metadata", "provider_trace_id"]],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "external_provider_request_id",
        &[
            &["external_provider_request_id"],
            &["metadata", "external_provider_request_id"],
        ],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "openai_request_id",
        &[&["openai_request_id"], &["metadata", "openai_request_id"]],
    );
    push_match_key_paths(
        &mut keys,
        row,
        "anthropic_request_id",
        &[
            &["anthropic_request_id"],
            &["metadata", "anthropic_request_id"],
        ],
    );
    keys.sort();
    keys.dedup();
    keys
}

fn push_match_key_paths(keys: &mut Vec<String>, row: &Value, prefix: &str, paths: &[&[&str]]) {
    for path in paths {
        if let Some(value) = json_string(row, path).filter(|value| !value.is_empty()) {
            keys.push(format!("{prefix}:{value}"));
        }
    }
}

fn provider_correlation_match_keys(match_keys: &[String]) -> Vec<String> {
    let mut keys = match_keys
        .iter()
        .filter(|key| {
            key.starts_with("provider_correlation:")
                || key.starts_with("provider_request_id:")
                || key.starts_with("provider_response_id:")
                || key.starts_with("provider_trace_id:")
                || key.starts_with("external_provider_request_id:")
                || key.starts_with("openai_request_id:")
                || key.starts_with("anthropic_request_id:")
                || key.starts_with("custom_id:")
        })
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

fn split_match_key_text(text: &str) -> Vec<String> {
    text.split([';', '|', '\n'])
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn write_json_file(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create report dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize report '{}': {error}", path.display()))?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write report '{}': {error}", path.display()))
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_string_any(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| json_string(value, path))
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
                .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
        })
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
            .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
    })
}

fn json_f64(value: &Value, path: &[&str]) -> Option<f64> {
    json_at(value, path).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
    })
}
