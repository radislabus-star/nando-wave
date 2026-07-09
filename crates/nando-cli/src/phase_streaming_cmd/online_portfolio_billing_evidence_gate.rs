use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-evidence-gate-v1.report.json";
const DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/phase-stream-online-miner-portfolio-billing-request-v1.jsonl";

#[derive(Clone, Debug)]
struct BillingRequestRow {
    estimated_total_tokens: usize,
    current_total_cost_microusd: u64,
    match_keys: Vec<String>,
    provider_correlation_ready: bool,
    external_provider_correlation_keys: Vec<String>,
}

pub(crate) fn run_phase_stream_online_miner_portfolio_billing_evidence_gate_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_REPORT)
    });
    let billing_request_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ONLINE_MINER_PORTFOLIO_BILLING_REQUEST_JSONL));
    let provider_billing_evidence_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "provider billing evidence JSONL path is required".to_owned())?;
    let missing_billing_request_jsonl_path = args.next().map(PathBuf::from);

    let billing_request_bytes = std::fs::read(&billing_request_jsonl_path).map_err(|error| {
        format!(
            "failed to read billing request '{}': {error}",
            billing_request_jsonl_path.display()
        )
    })?;
    let request_file_fingerprint64 = fnv1a64(&billing_request_bytes);
    let requests = parse_billing_requests(&billing_request_jsonl_path, &billing_request_bytes)?;
    let mut request_index_by_key = BTreeMap::<String, usize>::new();
    let mut duplicate_request_keys = 0usize;
    let mut request_tokens = 0usize;
    let mut request_current_cost_microusd = 0u64;
    let mut request_provider_correlation_ready_rows = 0usize;
    let mut request_external_provider_correlation_key_count = 0usize;
    for (index, request) in requests.iter().enumerate() {
        request_tokens = request_tokens.saturating_add(request.estimated_total_tokens);
        request_current_cost_microusd =
            request_current_cost_microusd.saturating_add(request.current_total_cost_microusd);
        if request.provider_correlation_ready {
            request_provider_correlation_ready_rows += 1;
            request_external_provider_correlation_key_count =
                request_external_provider_correlation_key_count
                    .saturating_add(request.external_provider_correlation_keys.len());
        }
        for key in &request.match_keys {
            if request_index_by_key.insert(key.clone(), index).is_some() {
                duplicate_request_keys += 1;
            }
        }
    }

    let evidence_text =
        std::fs::read_to_string(&provider_billing_evidence_jsonl_path).map_err(|error| {
            format!(
                "failed to read provider billing evidence '{}': {error}",
                provider_billing_evidence_jsonl_path.display()
            )
        })?;
    let mut evidence_rows = 0usize;
    let mut evidence_rows_with_match_key = 0usize;
    let mut billing_rows_with_provider_cost = 0usize;
    let mut request_only_rows = 0usize;
    let mut invalid_cost_rows = 0usize;
    let mut billing_evidence_id_rows = 0usize;
    let mut duplicate_billing_evidence_id_rows = 0usize;
    let mut seen_billing_evidence_ids = BTreeSet::<String>::new();
    let mut provider_rows = 0usize;
    let mut external_source_rows = 0usize;
    let mut missing_billing_evidence_id_rows = 0usize;
    let mut missing_provider_rows = 0usize;
    let mut missing_external_source_rows = 0usize;
    let mut request_file_fingerprint_rows = 0usize;
    let mut missing_request_file_fingerprint_rows = 0usize;
    let mut mismatched_request_file_fingerprint_rows = 0usize;
    let mut rejected_external_source_rows = 0usize;
    let mut unmatched_evidence_rows = 0usize;
    let mut multi_matched_evidence_rows = 0usize;
    let mut duplicate_matched_request_rows = 0usize;
    let mut covered_requests = BTreeSet::<usize>::new();
    let mut provider_cost_microusd = 0u64;
    let mut provider_total_tokens = 0usize;
    let mut provider_token_rows = 0usize;
    let mut rows_enriched_provider_tokens = 0usize;
    let mut evidence_rows_with_provider_correlation_keys = 0usize;
    let mut rows_enriched_with_provider_correlation = 0usize;

    for (line_index, line) in evidence_text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        evidence_rows += 1;
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse provider billing evidence '{}' line {}: {error}",
                provider_billing_evidence_jsonl_path.display(),
                line_index + 1
            )
        })?;
        if is_request_only_row(&row) {
            request_only_rows += 1;
            continue;
        }
        let keys = match_keys(&row);
        if !keys.is_empty() {
            evidence_rows_with_match_key += 1;
        }
        let evidence_provider_correlation_ready = !provider_correlation_keys(&row).is_empty();
        if evidence_provider_correlation_ready {
            evidence_rows_with_provider_correlation_keys += 1;
        }
        let Some(cost) = json_u64(&row, &["provider_cost_microusd"])
            .or_else(|| json_u64(&row, &["token_cost", "provider_cost_microusd"]))
            .or_else(|| json_u64(&row, &["token_cost", "total_cost_microusd"]))
        else {
            invalid_cost_rows += 1;
            continue;
        };
        if cost == 0 {
            invalid_cost_rows += 1;
            continue;
        }
        billing_rows_with_provider_cost += 1;
        let Some(billing_evidence_id) =
            json_string(&row, &["billing_evidence_id"]).filter(|value| !value.is_empty())
        else {
            missing_billing_evidence_id_rows += 1;
            continue;
        };
        billing_evidence_id_rows += 1;
        if !seen_billing_evidence_ids.insert(billing_evidence_id) {
            duplicate_billing_evidence_id_rows += 1;
            continue;
        }
        if json_string(&row, &["provider"])
            .or_else(|| json_string(&row, &["token_cost", "provider"]))
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        {
            provider_rows += 1;
        } else {
            missing_provider_rows += 1;
            continue;
        }
        let Some(source) = json_string(&row, &["billing_source"])
            .or_else(|| json_string(&row, &["provider_billing_evidence_source"]))
            .or_else(|| json_string(&row, &["source"]))
        else {
            missing_external_source_rows += 1;
            continue;
        };
        if !external_billing_source_allowed(&source) {
            rejected_external_source_rows += 1;
            continue;
        }
        external_source_rows += 1;
        let Some(evidence_request_file_fingerprint64) =
            json_u64(&row, &["request_file_fingerprint64"])
        else {
            missing_request_file_fingerprint_rows += 1;
            continue;
        };
        request_file_fingerprint_rows += 1;
        if evidence_request_file_fingerprint64 != request_file_fingerprint64 {
            mismatched_request_file_fingerprint_rows += 1;
            continue;
        }
        let row_provider_total_tokens = json_usize(&row, &["provider_total_tokens"])
            .or_else(|| json_usize(&row, &["token_cost", "total_tokens"]))
            .unwrap_or(0);
        if row_provider_total_tokens > 0 {
            provider_token_rows += 1;
        }

        let matched = keys
            .iter()
            .filter_map(|key| request_index_by_key.get(key).copied())
            .collect::<BTreeSet<_>>();
        if matched.is_empty() {
            unmatched_evidence_rows += 1;
            continue;
        }
        if matched.len() > 1 {
            multi_matched_evidence_rows += 1;
            continue;
        }
        for request_index in matched {
            if !covered_requests.insert(request_index) {
                duplicate_matched_request_rows += 1;
            } else {
                let request_provider_correlation_ready = requests
                    .get(request_index)
                    .is_some_and(|request| request.provider_correlation_ready);
                if request_provider_correlation_ready && evidence_provider_correlation_ready {
                    rows_enriched_with_provider_correlation += 1;
                }
                provider_cost_microusd = provider_cost_microusd.saturating_add(cost);
                if row_provider_total_tokens > 0 {
                    rows_enriched_provider_tokens += 1;
                    provider_total_tokens =
                        provider_total_tokens.saturating_add(row_provider_total_tokens);
                }
            }
        }
    }

    let request_rows = requests.len();
    let rows_enriched_provider_cost = covered_requests.len();
    let missing_billing_request_rows = request_rows.saturating_sub(rows_enriched_provider_cost);
    if let Some(path) = &missing_billing_request_jsonl_path {
        write_missing_billing_request_jsonl(
            path,
            request_file_fingerprint64,
            &requests,
            &covered_requests,
        )?;
    }
    let evidence_complete = request_rows > 0
        && rows_enriched_provider_cost == request_rows
        && duplicate_request_keys == 0
        && request_provider_correlation_ready_rows == request_rows
        && duplicate_matched_request_rows == 0
        && request_only_rows == 0
        && invalid_cost_rows == 0
        && missing_billing_evidence_id_rows == 0
        && duplicate_billing_evidence_id_rows == 0
        && missing_provider_rows == 0
        && missing_external_source_rows == 0
        && missing_request_file_fingerprint_rows == 0
        && mismatched_request_file_fingerprint_rows == 0
        && rejected_external_source_rows == 0
        && multi_matched_evidence_rows == 0
        && billing_evidence_id_rows >= rows_enriched_provider_cost
        && provider_rows >= rows_enriched_provider_cost
        && external_source_rows >= rows_enriched_provider_cost
        && provider_token_rows >= rows_enriched_provider_cost
        && evidence_rows_with_provider_correlation_keys >= rows_enriched_provider_cost
        && rows_enriched_with_provider_correlation == request_rows
        && rows_enriched_provider_tokens == request_rows
        && provider_total_tokens > 0;
    let verdict = if evidence_complete {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_V1_PASS"
    } else if evidence_rows == 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_V1_REJECT_EMPTY_EVIDENCE"
    } else if request_only_rows > 0 && billing_rows_with_provider_cost == 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_V1_REJECT_REQUEST_ONLY"
    } else if invalid_cost_rows > 0 && billing_rows_with_provider_cost == 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_V1_REJECT_INVALID_COST_OR_TEMPLATE"
    } else if rejected_external_source_rows > 0 || missing_external_source_rows > 0 {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_V1_REJECT_NON_EXTERNAL_SOURCE"
    } else {
        "PHASE_STREAM_ONLINE_MINER_PORTFOLIO_BILLING_EVIDENCE_GATE_V1_WATCH"
    };

    let mut report = serde_json::Map::new();
    report.insert(
        "report_kind".to_owned(),
        serde_json::json!("phase_stream_online_miner_portfolio_billing_evidence_gate_v1"),
    );
    report.insert(
        "billing_request_jsonl_path".to_owned(),
        serde_json::json!(&billing_request_jsonl_path),
    );
    report.insert(
        "provider_billing_evidence_jsonl_path".to_owned(),
        serde_json::json!(&provider_billing_evidence_jsonl_path),
    );
    report.insert("request_rows".to_owned(), serde_json::json!(request_rows));
    report.insert(
        "request_file_fingerprint64".to_owned(),
        serde_json::json!(request_file_fingerprint64),
    );
    report.insert(
        "request_match_keys".to_owned(),
        serde_json::json!(request_index_by_key.len()),
    );
    report.insert(
        "duplicate_request_keys".to_owned(),
        serde_json::json!(duplicate_request_keys),
    );
    report.insert(
        "request_total_tokens".to_owned(),
        serde_json::json!(request_tokens),
    );
    report.insert(
        "request_current_cost_microusd".to_owned(),
        serde_json::json!(request_current_cost_microusd),
    );
    report.insert(
        "request_provider_correlation_ready_rows".to_owned(),
        serde_json::json!(request_provider_correlation_ready_rows),
    );
    report.insert(
        "request_provider_correlation_missing_rows".to_owned(),
        serde_json::json!(request_rows.saturating_sub(request_provider_correlation_ready_rows)),
    );
    report.insert(
        "request_external_provider_correlation_key_count".to_owned(),
        serde_json::json!(request_external_provider_correlation_key_count),
    );
    report.insert("evidence_rows".to_owned(), serde_json::json!(evidence_rows));
    report.insert(
        "evidence_rows_with_match_key".to_owned(),
        serde_json::json!(evidence_rows_with_match_key),
    );
    report.insert(
        "billing_rows_with_provider_cost".to_owned(),
        serde_json::json!(billing_rows_with_provider_cost),
    );
    report.insert(
        "billing_evidence_id_rows".to_owned(),
        serde_json::json!(billing_evidence_id_rows),
    );
    report.insert(
        "unique_billing_evidence_id_rows".to_owned(),
        serde_json::json!(seen_billing_evidence_ids.len()),
    );
    report.insert(
        "duplicate_billing_evidence_id_rows".to_owned(),
        serde_json::json!(duplicate_billing_evidence_id_rows),
    );
    report.insert("provider_rows".to_owned(), serde_json::json!(provider_rows));
    report.insert(
        "external_source_rows".to_owned(),
        serde_json::json!(external_source_rows),
    );
    report.insert(
        "provider_token_rows".to_owned(),
        serde_json::json!(provider_token_rows),
    );
    report.insert(
        "evidence_rows_with_provider_correlation_keys".to_owned(),
        serde_json::json!(evidence_rows_with_provider_correlation_keys),
    );
    report.insert(
        "rows_enriched_with_provider_correlation".to_owned(),
        serde_json::json!(rows_enriched_with_provider_correlation),
    );
    report.insert(
        "rows_enriched_provider_tokens".to_owned(),
        serde_json::json!(rows_enriched_provider_tokens),
    );
    report.insert(
        "request_only_rows".to_owned(),
        serde_json::json!(request_only_rows),
    );
    report.insert(
        "invalid_cost_rows".to_owned(),
        serde_json::json!(invalid_cost_rows),
    );
    report.insert(
        "missing_billing_evidence_id_rows".to_owned(),
        serde_json::json!(missing_billing_evidence_id_rows),
    );
    report.insert(
        "missing_provider_rows".to_owned(),
        serde_json::json!(missing_provider_rows),
    );
    report.insert(
        "missing_external_source_rows".to_owned(),
        serde_json::json!(missing_external_source_rows),
    );
    report.insert(
        "request_file_fingerprint_rows".to_owned(),
        serde_json::json!(request_file_fingerprint_rows),
    );
    report.insert(
        "missing_request_file_fingerprint_rows".to_owned(),
        serde_json::json!(missing_request_file_fingerprint_rows),
    );
    report.insert(
        "mismatched_request_file_fingerprint_rows".to_owned(),
        serde_json::json!(mismatched_request_file_fingerprint_rows),
    );
    report.insert(
        "rejected_external_source_rows".to_owned(),
        serde_json::json!(rejected_external_source_rows),
    );
    report.insert(
        "unmatched_evidence_rows".to_owned(),
        serde_json::json!(unmatched_evidence_rows),
    );
    report.insert(
        "multi_matched_evidence_rows".to_owned(),
        serde_json::json!(multi_matched_evidence_rows),
    );
    report.insert(
        "duplicate_matched_request_rows".to_owned(),
        serde_json::json!(duplicate_matched_request_rows),
    );
    report.insert(
        "rows_enriched_provider_cost".to_owned(),
        serde_json::json!(rows_enriched_provider_cost),
    );
    report.insert(
        "missing_billing_request_rows".to_owned(),
        serde_json::json!(missing_billing_request_rows),
    );
    report.insert(
        "missing_billing_request_jsonl_path".to_owned(),
        serde_json::json!(&missing_billing_request_jsonl_path),
    );
    report.insert(
        "provider_cost_microusd".to_owned(),
        serde_json::json!(provider_cost_microusd),
    );
    report.insert(
        "provider_total_tokens".to_owned(),
        serde_json::json!(provider_total_tokens),
    );
    report.insert(
        "provider_billing_evidence_present".to_owned(),
        serde_json::json!(evidence_complete),
    );
    report.insert(
        "billing_gate".to_owned(),
        serde_json::json!({
            "provider_billing_request_only": request_only_rows > 0 && billing_rows_with_provider_cost == 0,
            "provider_billing_evidence_present": evidence_complete,
            "market_money_claim_allowed": evidence_complete,
            "provider_correlation_required": true,
            "request_provider_correlation_ready_rows": request_provider_correlation_ready_rows,
            "evidence_rows_with_provider_correlation_keys": evidence_rows_with_provider_correlation_keys,
            "rows_enriched_with_provider_correlation": rows_enriched_with_provider_correlation,
            "policy": "external provider billing evidence must cover every selected portfolio billing request with provider correlation keys, matching request_file_fingerprint64, positive provider_cost_microusd, positive provider_total_tokens, unique billing_evidence_id, provider, and accepted external billing_source; request/synthetic/internal JSONL is rejected as evidence"
        }),
    );
    report.insert(
        "required_provider_evidence_fields".to_owned(),
        serde_json::json!([
            "billing_evidence_id",
            "billing_source",
            "provider",
            "provider_cost_microusd",
            "provider_total_tokens",
            "request_file_fingerprint64",
            "provider correlation key in both selected request and external evidence"
        ]),
    );
    report.insert(
        "accepted_external_source_policy".to_owned(),
        serde_json::json!({
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
                "nando",
                "replace",
                "placeholder",
                "todo",
                "example",
                "sample",
                "template"
            ]
        }),
    );
    report.insert(
        "forbidden_flags".to_owned(),
        serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        }),
    );
    report.insert("local_accept_enabled".to_owned(), serde_json::json!(false));
    report.insert("auto_promote_enabled".to_owned(), serde_json::json!(false));
    report.insert(
        "product_promotion_allowed".to_owned(),
        serde_json::json!(false),
    );
    report.insert(
        "market_money_claim_allowed".to_owned(),
        serde_json::json!(false),
    );
    report.insert("verdict".to_owned(), serde_json::json!(verdict));
    report.insert(
        "boundary".to_owned(),
        serde_json::json!("provider billing evidence validation only: validates external billing rows against selected online-miner portfolio requests; does not compile, promote, serve, enable local_accept, estimate money, or revive legacy nwrb"),
    );
    let report = Value::Object(report);
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_portfolio_billing_evidence_gate_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  request_rows: {request_rows}");
    println!("  rows_enriched_provider_cost: {rows_enriched_provider_cost}");
    println!("  request_only_rows: {request_only_rows}");
    println!("  provider_billing_evidence_present: {evidence_complete}");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn write_missing_billing_request_jsonl(
    path: &Path,
    request_file_fingerprint64: u64,
    requests: &[BillingRequestRow],
    covered_requests: &BTreeSet<usize>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create missing billing request dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let mut lines = Vec::new();
    for (request_index, request) in requests.iter().enumerate() {
        if covered_requests.contains(&request_index) {
            continue;
        }
        let row = serde_json::json!({
            "schema_version": "phase_stream_online_miner_portfolio_missing_billing_request_v1",
            "request_index": request_index,
            "request_file_fingerprint64": request_file_fingerprint64,
            "estimated_total_tokens": request.estimated_total_tokens,
            "current_total_cost_microusd": request.current_total_cost_microusd,
            "match_keys": request.match_keys,
            "provider_correlation_ready": request.provider_correlation_ready,
            "external_provider_correlation_keys": request.external_provider_correlation_keys,
            "request_fingerprint": request
                .match_keys
                .iter()
                .find_map(|key| key.strip_prefix("request_fingerprint:")),
            "exact_cache_key": request
                .match_keys
                .iter()
                .find_map(|key| key.strip_prefix("exact_cache_key:")),
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
            "market_money_claim_allowed": false,
            "boundary": "missing provider billing request row only: identifies selected online-miner portfolio requests still lacking complete external provider cost evidence; does not estimate money, promote, serve, or enable local_accept"
        });
        lines.push(serde_json::to_string(&row).map_err(|error| {
            format!(
                "failed to serialize missing billing request '{}': {error}",
                path.display()
            )
        })?);
    }
    std::fs::write(path, format!("{}\n", lines.join("\n"))).map_err(|error| {
        format!(
            "failed to write missing billing request '{}': {error}",
            path.display()
        )
    })
}

fn parse_billing_requests(path: &Path, bytes: &[u8]) -> Result<Vec<BillingRequestRow>, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("billing request '{}' is not UTF-8: {error}", path.display()))?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
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
        let external_provider_correlation_keys = provider_correlation_keys(&row);
        let provider_correlation_ready = json_bool(&row, &["provider_correlation_ready"])
            .unwrap_or(false)
            || !external_provider_correlation_keys.is_empty();
        rows.push(BillingRequestRow {
            estimated_total_tokens: json_usize(&row, &["estimated_total_tokens"]).unwrap_or(0),
            current_total_cost_microusd: json_u64(&row, &["current_total_cost_microusd"])
                .unwrap_or(0),
            match_keys: match_keys(&row),
            provider_correlation_ready,
            external_provider_correlation_keys,
        });
    }
    Ok(rows)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn is_request_only_row(row: &Value) -> bool {
    json_string(row, &["schema_version"]).is_some_and(|schema| schema.contains("billing_request"))
        || json_bool(row, &["market_money_claim_allowed"]) == Some(false)
            && json_string(row, &["boundary"])
                .is_some_and(|boundary| boundary.contains("asks external billing evidence"))
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
    keys.extend(json_string_array(
        row,
        &["external_provider_correlation_keys"],
    ));
    if let Some(value) = json_string(row, &["request_fingerprint"]) {
        keys.push(format!("request_fingerprint:{value}"));
    }
    if let Some(value) = json_string(row, &["exact_cache_key"]) {
        keys.push(format!("exact_cache_key:{value}"));
    }
    for (label, path) in [
        ("provider_request_id", &["provider_request_id"][..]),
        ("provider_response_id", &["provider_response_id"]),
        ("provider_trace_id", &["provider_trace_id"]),
        (
            "external_provider_request_id",
            &["external_provider_request_id"],
        ),
        ("openai_request_id", &["openai_request_id"]),
        ("anthropic_request_id", &["anthropic_request_id"]),
        ("custom_id", &["custom_id"]),
        ("provider_request_id", &["metadata", "provider_request_id"]),
        (
            "provider_response_id",
            &["metadata", "provider_response_id"],
        ),
        (
            "external_provider_request_id",
            &["metadata", "external_provider_request_id"],
        ),
        ("custom_id", &["metadata", "custom_id"]),
    ] {
        if let Some(value) = json_string(row, path).filter(|value| !value.is_empty()) {
            keys.push(format!("{label}:{value}"));
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn provider_correlation_keys(row: &Value) -> Vec<String> {
    let mut keys = super::phase_atom_external_provider_correlation_keys(row);
    keys.extend(json_string_array(
        row,
        &["external_provider_correlation_keys"],
    ));
    keys.extend(json_string_array(row, &["provider_correlation_keys"]));
    if let Some(array) = json_at(row, &["match_keys"]).and_then(Value::as_array) {
        for value in array {
            if let Some(key) = value
                .as_str()
                .filter(|key| key.starts_with("provider_correlation:"))
            {
                keys.push(key.to_owned());
            }
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn external_billing_source_allowed(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    let forbidden = [
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
        "nando",
        "replace",
        "placeholder",
        "todo",
        "example",
        "sample",
        "template",
    ];
    !forbidden
        .iter()
        .any(|forbidden| normalized.contains(forbidden))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_billing_source_rejects_template_and_internal_sources() {
        for source in [
            "",
            "synthetic usage export",
            "internal token report",
            "provider billing template",
            "sample provider csv",
            "replace-with-provider-export",
            "placeholder usage source",
        ] {
            assert!(!external_billing_source_allowed(source), "{source}");
        }

        assert!(external_billing_source_allowed(
            "openai platform usage export 2026-07-07"
        ));
    }
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

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_string_array(value: &Value, path: &[&str]) -> Vec<String> {
    json_at(value, path)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
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
