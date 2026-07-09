use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_BILLING_REQUEST_PROVIDER_CORRELATION_BACKFILL_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-provider-correlation-backfill-v1.report.json";
const DEFAULT_BILLING_REQUEST_PROVIDER_CORRELATION_BACKFILL_JSONL: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-provider-correlation-backfill-v1.jsonl";

#[derive(Default)]
struct ProviderKeyIndex {
    key_to_provider_keys: BTreeMap<String, BTreeSet<String>>,
    rows: usize,
    rows_with_provider_keys: usize,
    provider_key_count: usize,
    provider_request_id_key_rows: usize,
}

#[derive(Default)]
struct BackfillState {
    billing_request_rows: usize,
    rows_with_existing_provider_correlation: usize,
    rows_with_added_provider_correlation: usize,
    rows_with_any_provider_correlation: usize,
    rows_missing_provider_correlation: usize,
    provider_key_count_added: usize,
    provider_request_id_ready_rows: usize,
    total_tokens_requiring_billing: usize,
    current_known_cost_microusd: u64,
    request_rows_with_exact_cache_key: usize,
    request_rows_with_request_fingerprint: usize,
    request_rows_with_trace_id: usize,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_billing_request_provider_correlation_backfill_v1<
    I,
>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_BILLING_REQUEST_PROVIDER_CORRELATION_BACKFILL_REPORT)
    });
    let output_jsonl_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_BILLING_REQUEST_PROVIDER_CORRELATION_BACKFILL_JSONL)
    });
    let billing_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "billing request JSONL path is required".to_owned())?;
    let provider_boundary_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if provider_boundary_paths.is_empty() {
        return Err("at least one provider-boundary JSONL path is required".to_owned());
    }

    let provider_index = read_provider_key_index(&provider_boundary_paths)?;
    let billing_text = read_text(&billing_request_jsonl_path)?;
    if let Some(parent) = output_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create enriched billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(&output_jsonl_path).map_err(|error| {
        format!(
            "failed to create enriched billing request JSONL '{}': {error}",
            output_jsonl_path.display()
        )
    })?;
    let mut writer = BufWriter::new(file);
    let mut state = BackfillState::default();

    for (line_index, line) in billing_text.lines().enumerate() {
        let Some(mut row) = parse_json_row(&billing_request_jsonl_path, line_index + 1, line)?
        else {
            continue;
        };
        state.billing_request_rows += 1;
        state.total_tokens_requiring_billing = state
            .total_tokens_requiring_billing
            .saturating_add(json_usize(&row, &["estimated_total_tokens"]).unwrap_or(0));
        state.current_known_cost_microusd = state
            .current_known_cost_microusd
            .saturating_add(json_u64(&row, &["current_total_cost_microusd"]).unwrap_or(0));
        state.request_rows_with_exact_cache_key += usize::from(
            json_string(&row, &["exact_cache_key"])
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
        );
        state.request_rows_with_request_fingerprint += usize::from(
            json_string(&row, &["request_fingerprint"])
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
        );
        state.request_rows_with_trace_id += usize::from(
            json_string(&row, &["trace_id"])
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
        );

        let existing_keys = provider_correlation_keys(&row);
        if !existing_keys.is_empty() {
            state.rows_with_existing_provider_correlation += 1;
        }
        let mut merged_provider_keys = existing_keys.into_iter().collect::<BTreeSet<_>>();
        let before_len = merged_provider_keys.len();
        for key in billing_match_keys(&row) {
            if let Some(provider_keys) = provider_index.key_to_provider_keys.get(&key) {
                merged_provider_keys.extend(provider_keys.iter().cloned());
            }
        }
        let added = merged_provider_keys.len().saturating_sub(before_len);
        if added > 0 {
            state.rows_with_added_provider_correlation += 1;
            state.provider_key_count_added = state.provider_key_count_added.saturating_add(added);
        }
        let provider_keys = merged_provider_keys.into_iter().collect::<Vec<_>>();
        if provider_keys.is_empty() {
            state.rows_missing_provider_correlation += 1;
        } else {
            state.rows_with_any_provider_correlation += 1;
        }
        if provider_keys
            .iter()
            .any(|key| real_provider_request_key(key))
        {
            state.provider_request_id_ready_rows += 1;
        }

        enrich_billing_request_row(&mut row, &provider_keys, &provider_boundary_paths)?;
        serde_json::to_writer(&mut writer, &row).map_err(|error| {
            format!(
                "failed to serialize enriched billing request '{}': {error}",
                output_jsonl_path.display()
            )
        })?;
        writer.write_all(b"\n").map_err(|error| {
            format!(
                "failed to write enriched billing request '{}': {error}",
                output_jsonl_path.display()
            )
        })?;
    }
    writer.flush().map_err(|error| {
        format!(
            "failed to flush enriched billing request '{}': {error}",
            output_jsonl_path.display()
        )
    })?;

    let output_bytes = std::fs::read(&output_jsonl_path).map_err(|error| {
        format!(
            "failed to read enriched billing request '{}': {error}",
            output_jsonl_path.display()
        )
    })?;
    let request_file_fingerprint64 = fnv1a64(&output_bytes);
    let full_provider_correlation = state.billing_request_rows > 0
        && state.rows_with_any_provider_correlation == state.billing_request_rows;
    let real_provider_request_id_coverage = state.billing_request_rows > 0
        && state.provider_request_id_ready_rows == state.billing_request_rows;
    let verdict = if full_provider_correlation {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_PROVIDER_CORRELATION_BACKFILL_V1_PASS_METADATA_CORRELATED"
    } else if state.rows_with_any_provider_correlation > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_PROVIDER_CORRELATION_BACKFILL_V1_WATCH_PARTIAL_METADATA_CORRELATION"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_PROVIDER_CORRELATION_BACKFILL_V1_WATCH_NO_METADATA_CORRELATION"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_portfolio_billing_request_provider_correlation_backfill_v1",
        "source_billing_request_jsonl_path": billing_request_jsonl_path,
        "provider_boundary_paths": provider_boundary_paths,
        "billing_request_jsonl_path": output_jsonl_path,
        "output_billing_request_jsonl_path": output_jsonl_path,
        "request_file_fingerprint64": request_file_fingerprint64,
        "provider_boundary_rows": provider_index.rows,
        "provider_boundary_rows_with_provider_keys": provider_index.rows_with_provider_keys,
        "provider_boundary_provider_key_count": provider_index.provider_key_count,
        "provider_boundary_provider_request_id_key_rows": provider_index.provider_request_id_key_rows,
        "billing_request_rows": state.billing_request_rows,
        "rows_with_existing_provider_correlation": state.rows_with_existing_provider_correlation,
        "rows_with_added_provider_correlation": state.rows_with_added_provider_correlation,
        "rows_with_any_provider_correlation": state.rows_with_any_provider_correlation,
        "rows_missing_provider_correlation": state.rows_missing_provider_correlation,
        "provider_key_count_added": state.provider_key_count_added,
        "provider_request_id_ready_rows": state.provider_request_id_ready_rows,
        "real_provider_request_id_coverage": real_provider_request_id_coverage,
        "full_provider_correlation": full_provider_correlation,
        "request_rows_with_exact_cache_key": state.request_rows_with_exact_cache_key,
        "request_rows_with_request_fingerprint": state.request_rows_with_request_fingerprint,
        "request_rows_with_trace_id": state.request_rows_with_trace_id,
        "total_tokens_requiring_billing": state.total_tokens_requiring_billing,
        "current_known_cost_microusd": state.current_known_cost_microusd,
        "billing_gate": {
            "provider_billing_request_only": true,
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "policy": "provider-boundary correlation was backfilled into billing request rows; this is not external billing evidence and not a money claim"
        },
        "required_next_gate": {
            "command": "phase-stream-online-miner-portfolio-billing-evidence-gate-v1",
            "needs_external_provider_billing_evidence": true,
            "request_file_fingerprint64": request_file_fingerprint64,
            "provider_request_id_absent_if_zero_ready_rows": state.provider_request_id_ready_rows == 0,
            "policy": "external evidence must include positive provider_cost_microusd, positive provider_total_tokens, provider, billing_source, unique billing_evidence_id, request_file_fingerprint64, and a matching provider correlation key"
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
        "boundary": "provider-correlation backfill only: enriches billing request metadata from provider-boundary rows; does not create external billing evidence, estimate money, score, promote, serve, or enable local_accept"
    });
    super::write_json_file(&report_path, &report)?;
    println!(
        "phase_stream_online_miner_portfolio_billing_request_provider_correlation_backfill_v1:"
    );
    println!("  report_path: {}", report_path.display());
    println!(
        "  output_billing_request_jsonl_path: {}",
        output_jsonl_path.display()
    );
    println!("  billing_request_rows: {}", state.billing_request_rows);
    println!(
        "  rows_with_any_provider_correlation: {}",
        state.rows_with_any_provider_correlation
    );
    println!(
        "  provider_request_id_ready_rows: {}",
        state.provider_request_id_ready_rows
    );
    println!("  request_file_fingerprint64: {request_file_fingerprint64}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_provider_key_index(paths: &[PathBuf]) -> Result<ProviderKeyIndex, String> {
    let mut index = ProviderKeyIndex::default();
    for path in paths {
        let text = read_text(path)?;
        for (line_index, line) in text.lines().enumerate() {
            let Some(row) = parse_json_row(path, line_index + 1, line)? else {
                continue;
            };
            index.rows += 1;
            let provider_keys = provider_correlation_keys(&row);
            if provider_keys.is_empty() {
                continue;
            }
            index.rows_with_provider_keys += 1;
            index.provider_key_count = index.provider_key_count.saturating_add(provider_keys.len());
            index.provider_request_id_key_rows += usize::from(
                provider_keys
                    .iter()
                    .any(|key| real_provider_request_key(key)),
            );
            for key in provider_match_keys(&row) {
                index
                    .key_to_provider_keys
                    .entry(key)
                    .or_default()
                    .extend(provider_keys.iter().cloned());
            }
        }
    }
    Ok(index)
}

fn enrich_billing_request_row(
    row: &mut Value,
    provider_keys: &[String],
    provider_boundary_paths: &[PathBuf],
) -> Result<(), String> {
    let mut match_keys = billing_match_keys(row).into_iter().collect::<BTreeSet<_>>();
    match_keys.extend(provider_keys.iter().cloned());
    let Some(map) = row.as_object_mut() else {
        return Err("billing request row is not an object".to_owned());
    };
    map.insert(
        "external_provider_correlation_keys".to_owned(),
        serde_json::json!(provider_keys),
    );
    map.insert(
        "provider_correlation_ready".to_owned(),
        serde_json::json!(!provider_keys.is_empty()),
    );
    map.insert(
        "provider_request_id_ready".to_owned(),
        serde_json::json!(
            provider_keys
                .iter()
                .any(|key| real_provider_request_key(key))
        ),
    );
    map.insert(
        "provider_correlation_backfill".to_owned(),
        serde_json::json!({
            "schema_version": "provider_correlation_backfill_v1",
            "source": "provider_boundary_metadata",
            "provider_boundary_paths": provider_boundary_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
            "provider_billing_evidence_present": false,
            "market_money_claim_allowed": false,
            "boundary": "metadata correlation only, not billing evidence"
        }),
    );

    map.insert(
        "match_keys".to_owned(),
        serde_json::json!(match_keys.into_iter().collect::<Vec<_>>()),
    );
    map.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    map.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
    );
    Ok(())
}

fn provider_match_keys(row: &Value) -> Vec<String> {
    let mut keys = common_match_keys(row);
    keys.extend(json_string_vec(json_at(row, &["match_keys"])));
    keys.sort();
    keys.dedup();
    keys
}

fn billing_match_keys(row: &Value) -> Vec<String> {
    let mut keys = common_match_keys(row);
    keys.extend(json_string_vec(json_at(row, &["match_keys"])));
    keys.extend(provider_correlation_keys(row));
    keys.sort();
    keys.dedup();
    keys
}

fn common_match_keys(row: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    push_key(
        &mut keys,
        row,
        "request_fingerprint",
        &["request_fingerprint"],
    );
    push_key(&mut keys, row, "exact_cache_key", &["exact_cache_key"]);
    push_key(&mut keys, row, "trace_id", &["trace_id"]);
    push_key(
        &mut keys,
        row,
        "request_fingerprint",
        &["metadata", "request_fingerprint"],
    );
    push_key(
        &mut keys,
        row,
        "exact_cache_key",
        &["metadata", "exact_cache_key"],
    );
    push_key(&mut keys, row, "trace_id", &["metadata", "trace_id"]);
    keys
}

fn push_key(keys: &mut Vec<String>, row: &Value, prefix: &str, path: &[&str]) {
    if let Some(value) = json_string(row, path).filter(|value| !value.is_empty()) {
        if value.starts_with(prefix) && value.contains(':') {
            keys.push(value);
        } else {
            keys.push(format!("{prefix}:{value}"));
        }
    }
}

fn provider_correlation_keys(row: &Value) -> Vec<String> {
    let mut keys = super::phase_atom_external_provider_correlation_keys(row);
    keys.sort();
    keys.dedup();
    keys
}

fn real_provider_request_key(key: &str) -> bool {
    key.starts_with("provider_request_id:")
        || key.starts_with("external_provider_request_id:")
        || key.starts_with("openai_request_id:")
        || key.starts_with("anthropic_request_id:")
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))
}

fn parse_json_row(path: &Path, line_number: usize, line: &str) -> Result<Option<Value>, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
        format!(
            "failed to parse JSONL '{}' line {}: {error}",
            path.display(),
            line_number
        )
    })?;
    if row.is_object() {
        Ok(Some(row))
    } else {
        Ok(None)
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
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

fn json_string_vec(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(text)) if !text.is_empty() => vec![text.to_owned()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    }
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
