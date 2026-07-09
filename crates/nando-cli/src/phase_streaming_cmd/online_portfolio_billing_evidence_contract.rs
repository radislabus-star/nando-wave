use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_CONTRACT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-contract-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_TEMPLATE_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-contract-v1.template.jsonl";
const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_TEMPLATE_CSV: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-contract-v1.template.csv";
const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1.report.json";

pub(crate) fn run_phase_stream_online_miner_portfolio_billing_evidence_contract_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_CONTRACT_REPORT)
    });
    let billing_request_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_REPORT));
    let template_jsonl_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_TEMPLATE_JSONL)
    });
    let template_csv_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_template_csv_path(&template_jsonl_path));

    let request_report = read_json_value(&billing_request_report_path)?;
    let request_jsonl_path = PathBuf::from(
        json_string(&request_report, &["billing_request_jsonl_path"]).ok_or_else(|| {
            format!(
                "billing request report '{}' missing billing_request_jsonl_path",
                billing_request_report_path.display()
            )
        })?,
    );
    let request_bytes = std::fs::read(&request_jsonl_path).map_err(|error| {
        format!(
            "failed to read billing request JSONL '{}': {error}",
            request_jsonl_path.display()
        )
    })?;
    let request_text = String::from_utf8(request_bytes.clone()).map_err(|error| {
        format!(
            "billing request JSONL '{}' is not UTF-8: {error}",
            request_jsonl_path.display()
        )
    })?;

    let mut request_rows = 0usize;
    let mut request_rows_with_match_key = 0usize;
    let mut request_rows_with_exact_cache_key = 0usize;
    let mut request_rows_with_request_fingerprint = 0usize;
    let mut request_rows_with_provider_correlation = 0usize;
    let mut total_tokens = 0usize;
    let mut current_cost_microusd = 0u64;
    let mut template_rows = Vec::<Value>::new();
    let request_file_fingerprint64 = fnv1a64(&request_bytes);

    for (line_index, line) in request_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        request_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse billing request JSONL '{}' line {}: {error}",
                request_jsonl_path.display(),
                line_index + 1
            )
        })?;
        let match_keys = match_keys(&row);
        if !match_keys.is_empty() {
            request_rows_with_match_key += 1;
        }
        if json_string(&row, &["exact_cache_key"])
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        {
            request_rows_with_exact_cache_key += 1;
        }
        if json_string(&row, &["request_fingerprint"])
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        {
            request_rows_with_request_fingerprint += 1;
        }
        let provider_correlation_ready = json_bool(&row, &["provider_correlation_ready"])
            .unwrap_or(false)
            || json_string_array(&row, &["external_provider_correlation_keys"])
                .into_iter()
                .any(|value| !value.is_empty());
        if provider_correlation_ready {
            request_rows_with_provider_correlation += 1;
        }
        total_tokens =
            total_tokens.saturating_add(json_usize(&row, &["estimated_total_tokens"]).unwrap_or(0));
        current_cost_microusd = current_cost_microusd
            .saturating_add(json_u64(&row, &["current_total_cost_microusd"]).unwrap_or(0));

        template_rows.push(serde_json::json!({
            "schema_version": "provider_billing_evidence_v1",
            "billing_evidence_id": format!("replace-with-provider-bill-row-id-{request_rows}"),
            "billing_source": "replace-with-external-provider-export-name",
            "provider": "replace-with-provider-name",
            "provider_cost_microusd": "replace-with-positive-integer",
            "provider_total_tokens": "replace-with-positive-provider-token-count",
            "match_keys": match_keys,
            "request_fingerprint": json_string(&row, &["request_fingerprint"]),
            "exact_cache_key": json_string(&row, &["exact_cache_key"]),
            "request_file_fingerprint64": request_file_fingerprint64,
            "boundary": "template only: replace placeholders with external provider billing export values; do not feed this template as evidence"
        }));
    }

    if let Some(parent) = template_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create billing evidence template dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let template_file = std::fs::File::create(&template_jsonl_path).map_err(|error| {
        format!(
            "failed to create billing evidence template '{}': {error}",
            template_jsonl_path.display()
        )
    })?;
    let mut template_writer = BufWriter::new(template_file);
    for template_row in &template_rows {
        serde_json::to_writer(&mut template_writer, template_row).map_err(|error| {
            format!(
                "failed to serialize billing evidence template '{}': {error}",
                template_jsonl_path.display()
            )
        })?;
        template_writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write billing evidence template '{}': {error}",
                template_jsonl_path.display()
            )
        })?;
    }
    template_writer.flush().map_err(|error| {
        format!(
            "failed to flush billing evidence template '{}': {error}",
            template_jsonl_path.display()
        )
    })?;
    write_csv_template(&template_csv_path, &template_rows)?;

    let report_rows = json_usize(&request_report, &["billing_request_rows"]).unwrap_or(0);
    let report_tokens =
        json_usize(&request_report, &["total_tokens_requiring_billing"]).unwrap_or(0);
    let request_contract_ready = request_rows > 0
        && report_rows == request_rows
        && report_tokens == total_tokens
        && request_rows_with_match_key == request_rows;
    let provider_correlation_parity =
        request_rows > 0 && request_rows_with_provider_correlation == request_rows;
    let ready_for_external_provider_evidence =
        request_contract_ready && provider_correlation_parity;
    let verdict = if ready_for_external_provider_evidence {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_CONTRACT_V1_READY"
    } else if request_contract_ready {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_CONTRACT_V1_WATCH_PROVIDER_CORRELATION_MISSING"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_CONTRACT_V1_WATCH"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_billing_evidence_contract_v1",
        "billing_request_report_path": billing_request_report_path,
        "billing_request_jsonl_path": request_jsonl_path,
        "template_jsonl_path": template_jsonl_path,
        "template_csv_path": template_csv_path,
        "template_rows": template_rows.len(),
        "template_rows_match_request_rows": template_rows.len() == request_rows,
        "template_csv_format": "provider_billing_evidence_v1_csv",
        "template_csv_rows": template_rows.len(),
        "template_csv_rows_match_request_rows": template_rows.len() == request_rows,
        "request_rows": request_rows,
        "report_billing_request_rows": report_rows,
        "request_rows_with_match_key": request_rows_with_match_key,
        "request_rows_with_exact_cache_key": request_rows_with_exact_cache_key,
        "request_rows_with_request_fingerprint": request_rows_with_request_fingerprint,
        "request_rows_with_provider_correlation": request_rows_with_provider_correlation,
        "request_rows_missing_provider_correlation": request_rows.saturating_sub(request_rows_with_provider_correlation),
        "provider_correlation_parity": provider_correlation_parity,
        "ready_for_external_provider_evidence": ready_for_external_provider_evidence,
        "request_total_tokens": total_tokens,
        "report_total_tokens_requiring_billing": report_tokens,
        "request_current_cost_microusd": current_cost_microusd,
        "request_file_bytes": request_bytes.len(),
        "request_file_fingerprint64": request_file_fingerprint64,
        "required_provider_evidence_fields": [
            "billing_evidence_id",
            "billing_source",
            "provider",
            "provider_cost_microusd",
            "provider_total_tokens",
            "request_file_fingerprint64",
            "match_keys or request_fingerprint/exact_cache_key/provider correlation key"
        ],
        "accepted_external_source_policy": {
            "must_be_external_provider_export": true,
            "forbidden_source_fragments": [
                "synthetic",
                "estimate",
                "estimated",
                "request",
                "generated",
                "internal",
                "debug",
                "test",
                "fixture",
                "user_approved",
                "price_config",
                "nando"
            ]
        },
        "coverage_requirements": {
            "rows_enriched_provider_cost_must_equal_request_rows": request_rows,
            "provider_cost_microusd_must_be_positive": true,
            "provider_total_tokens_must_be_positive": true,
            "request_file_fingerprint64_must_match": request_file_fingerprint64,
            "duplicate_billing_evidence_ids_allowed": false,
            "duplicate_request_keys_allowed": false,
            "duplicate_matched_request_rows_allowed": false,
            "multi_request_evidence_rows_allowed": false,
            "request_only_rows_allowed": false,
            "synthetic_or_internal_source_allowed": false
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
        "boundary": "provider billing evidence contract only: records the selected portfolio request set and exact external-evidence requirements; does not create evidence, estimate money, promote, serve, or enable local_accept"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_billing_evidence_contract_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  template_jsonl_path: {}", template_jsonl_path.display());
    println!("  template_csv_path: {}", template_csv_path.display());
    println!("  request_rows: {request_rows}");
    println!("  request_file_fingerprint64: {request_file_fingerprint64}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn match_keys(row: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(array) = json_at(row, &["match_keys"]).and_then(Value::as_array) {
        for value in array {
            if let Some(key) = value.as_str().filter(|key| !key.is_empty()) {
                keys.push(key.to_owned());
            }
        }
    }
    if let Some(value) = json_string(row, &["request_fingerprint"]) {
        keys.push(format!("request_fingerprint:{value}"));
    }
    if let Some(value) = json_string(row, &["exact_cache_key"]) {
        keys.push(format!("exact_cache_key:{value}"));
    }
    keys.sort();
    keys.dedup();
    keys
}

fn default_template_csv_path(template_jsonl_path: &Path) -> PathBuf {
    if template_jsonl_path
        == Path::new(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_TEMPLATE_JSONL)
    {
        return PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_TEMPLATE_CSV);
    }
    template_jsonl_path.with_extension("csv")
}

fn write_csv_template(path: &Path, rows: &[Value]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create billing evidence CSV template dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(path).map_err(|error| {
        format!(
            "failed to create billing evidence CSV template '{}': {error}",
            path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);
    let header = [
        "schema_version",
        "billing_evidence_id",
        "billing_source",
        "provider",
        "provider_cost_microusd",
        "provider_total_tokens",
        "request_file_fingerprint64",
        "request_fingerprint",
        "exact_cache_key",
        "match_keys",
        "boundary",
    ];
    writeln!(writer, "{}", header.join(",")).map_err(|error| {
        format!(
            "failed to write billing evidence CSV template '{}': {error}",
            path.display()
        )
    })?;
    for row in rows {
        let match_keys = json_at(row, &["match_keys"])
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(";")
            })
            .unwrap_or_default();
        let fields = [
            csv_escape(
                json_string(row, &["schema_version"])
                    .as_deref()
                    .unwrap_or(""),
            ),
            csv_escape(
                json_string(row, &["billing_evidence_id"])
                    .as_deref()
                    .unwrap_or(""),
            ),
            csv_escape(
                json_string(row, &["billing_source"])
                    .as_deref()
                    .unwrap_or(""),
            ),
            csv_escape(json_string(row, &["provider"]).as_deref().unwrap_or("")),
            csv_escape(
                json_string(row, &["provider_cost_microusd"])
                    .as_deref()
                    .unwrap_or(""),
            ),
            csv_escape(
                json_string(row, &["provider_total_tokens"])
                    .as_deref()
                    .unwrap_or(""),
            ),
            csv_escape(
                json_at(row, &["request_file_fingerprint64"])
                    .map(Value::to_string)
                    .as_deref()
                    .unwrap_or(""),
            ),
            csv_escape(
                json_string(row, &["request_fingerprint"])
                    .as_deref()
                    .unwrap_or(""),
            ),
            csv_escape(
                json_string(row, &["exact_cache_key"])
                    .as_deref()
                    .unwrap_or(""),
            ),
            csv_escape(&match_keys),
            csv_escape(json_string(row, &["boundary"]).as_deref().unwrap_or("")),
        ];
        writeln!(writer, "{}", fields.join(",")).map_err(|error| {
            format!(
                "failed to write billing evidence CSV template '{}': {error}",
                path.display()
            )
        })?;
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush billing evidence CSV template '{}': {error}",
            path.display()
        )
    })
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn read_json_value(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse JSON report '{}': {error}", path.display()))
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

fn json_string_array(value: &Value, path: &[&str]) -> Vec<String> {
    json_at(value, path)
        .and_then(Value::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
        })
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
    })
}
