use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_CODEX_TOKEN_BACKFILL_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-codex-token-backfill-v1.report.json";
const DEFAULT_CODEX_TOKEN_BACKFILL_PROVIDER_JSONL: &str =
    "target/nando-wave/streaming/provider-boundary-codex-token-backfill-v1.provider.jsonl";
const MAX_AFTER_EVENT_MS: i64 = 120_000;

#[derive(Clone, Default)]
struct CaptureRequest {
    capture_request_id: String,
    join_keys: BTreeSet<String>,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Clone)]
struct PhaseSource {
    join_keys: Vec<String>,
    event_timestamp: String,
    event_ms: i64,
    input_trace_path: PathBuf,
    trace_id: Option<String>,
    request_fingerprint: Option<String>,
    exact_cache_key: Option<String>,
}

#[derive(Clone)]
struct TokenBoundary {
    timestamp: String,
    timestamp_ms: i64,
    source_line: usize,
    input_tokens: usize,
    cached_input_tokens: usize,
    output_tokens: usize,
    reasoning_output_tokens: usize,
    total_tokens: usize,
}

#[derive(Default)]
struct CaptureIndex {
    requests: BTreeMap<String, CaptureRequest>,
    key_to_capture_ids: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Default)]
struct BackfillState {
    capture_request_rows: usize,
    capture_requests_with_join_keys: usize,
    phase_trace_rows: usize,
    phase_rows_matching_capture: usize,
    phase_rows_with_input_trace_path: usize,
    source_session_files_read: usize,
    source_session_files_missing: usize,
    token_count_events_seen: usize,
    token_count_events_with_usage: usize,
    emitted_provider_boundary_rows: usize,
    unique_capture_requests_emitted: BTreeSet<String>,
    missing_phase_source_capture_ids: BTreeSet<String>,
    missing_session_file_capture_ids: BTreeSet<String>,
    missing_token_boundary_capture_ids: BTreeSet<String>,
    appended_total_tokens: usize,
    appended_estimated_cost_microusd: u64,
}

pub(crate) fn run_phase_stream_provider_boundary_codex_token_backfill_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_TOKEN_BACKFILL_REPORT));
    let output_provider_boundary_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_TOKEN_BACKFILL_PROVIDER_JSONL));
    let capture_request_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "capture-request JSONL path is required".to_owned())?;
    let phase_trace_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if phase_trace_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let mut state = BackfillState::default();
    let capture_index = read_capture_index(&capture_request_path, &mut state)?;
    let phase_sources = read_phase_sources(&phase_trace_paths, &capture_index, &mut state)?;
    let mut session_cache = BTreeMap::<PathBuf, Option<Vec<TokenBoundary>>>::new();
    let mut output = String::new();
    let mut captures_with_phase_source = BTreeSet::<String>::new();

    for source in &phase_sources {
        let matched_capture_ids = matched_capture_ids(&capture_index, &source.join_keys);
        captures_with_phase_source.extend(matched_capture_ids.iter().cloned());
        let token_boundaries = match session_cache.get(&source.input_trace_path) {
            Some(cached) => cached,
            None => {
                let loaded = read_session_token_boundaries(&source.input_trace_path, &mut state)?;
                session_cache.insert(source.input_trace_path.clone(), loaded);
                session_cache
                    .get(&source.input_trace_path)
                    .expect("session cache entry inserted")
            }
        };
        let Some(token_boundaries) = token_boundaries.as_ref() else {
            state
                .missing_session_file_capture_ids
                .extend(matched_capture_ids.iter().cloned());
            continue;
        };
        let Some(boundary) = first_token_count_after(source.event_ms, token_boundaries) else {
            state
                .missing_token_boundary_capture_ids
                .extend(matched_capture_ids.iter().cloned());
            continue;
        };
        for capture_id in matched_capture_ids {
            if state.unique_capture_requests_emitted.contains(&capture_id) {
                continue;
            }
            let capture = capture_index
                .requests
                .get(&capture_id)
                .ok_or_else(|| format!("internal missing capture id '{capture_id}'"))?;
            let row = provider_boundary_row(capture, source, boundary);
            output.push_str(
                &serde_json::to_string(&row)
                    .map_err(|error| format!("failed to serialize backfill row: {error}"))?,
            );
            output.push('\n');
            state.emitted_provider_boundary_rows += 1;
            state
                .unique_capture_requests_emitted
                .insert(capture_id.clone());
            state.appended_total_tokens = state
                .appended_total_tokens
                .saturating_add(boundary.total_tokens);
        }
    }

    for capture_id in capture_index.requests.keys() {
        if !captures_with_phase_source.contains(capture_id) {
            state
                .missing_phase_source_capture_ids
                .insert(capture_id.clone());
        }
    }

    if let Some(parent) = output_provider_boundary_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create provider boundary output dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&output_provider_boundary_path, output).map_err(|error| {
        format!(
            "failed to write provider boundary backfill '{}': {error}",
            output_provider_boundary_path.display()
        )
    })?;

    let missing_phase_source = state.missing_phase_source_capture_ids.len();
    let missing_session_file = state.missing_session_file_capture_ids.len();
    let missing_token_boundary = state.missing_token_boundary_capture_ids.len();
    let coverage_possible = state.emitted_provider_boundary_rows > 0;
    let full_capture_coverage = state.emitted_provider_boundary_rows == state.capture_request_rows
        && state.capture_request_rows > 0;
    let verdict = if full_capture_coverage {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CODEX_TOKEN_BACKFILL_V1_PASS_TOKEN_BOUNDARY_COVERAGE"
    } else if coverage_possible {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CODEX_TOKEN_BACKFILL_V1_WATCH_PARTIAL_TOKEN_BOUNDARY_COVERAGE"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CODEX_TOKEN_BACKFILL_V1_WATCH_NO_TOKEN_BOUNDARY_COVERAGE"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_codex_token_backfill_v1",
        "capture_request_path": path_string(&capture_request_path),
        "phase_trace_paths": phase_trace_paths.iter().map(path_string).collect::<Vec<_>>(),
        "output_provider_boundary_path": path_string(&output_provider_boundary_path),
        "token_boundary_source": "codex_session_event_msg_token_count",
        "token_boundary_join_policy": {
            "direction": "first_token_count_after_phase_event",
            "max_after_event_ms": MAX_AFTER_EVENT_MS,
            "provider_request_id_available": false,
            "cost_evidence_available": false,
            "market_money_claim_allowed": false
        },
        "scoreboard": {
            "capture_request_rows": state.capture_request_rows,
            "capture_requests_with_join_keys": state.capture_requests_with_join_keys,
            "phase_trace_rows": state.phase_trace_rows,
            "phase_rows_matching_capture": state.phase_rows_matching_capture,
            "phase_rows_with_input_trace_path": state.phase_rows_with_input_trace_path,
            "source_session_files_read": state.source_session_files_read,
            "source_session_files_missing": state.source_session_files_missing,
            "token_count_events_seen": state.token_count_events_seen,
            "token_count_events_with_usage": state.token_count_events_with_usage,
            "emitted_provider_boundary_rows": state.emitted_provider_boundary_rows,
            "unique_capture_requests_emitted": state.unique_capture_requests_emitted.len(),
            "missing_phase_source_capture_requests": missing_phase_source,
            "missing_session_file_capture_requests": missing_session_file,
            "missing_token_boundary_capture_requests": missing_token_boundary,
            "appended_total_tokens": state.appended_total_tokens,
            "appended_estimated_cost_microusd": state.appended_estimated_cost_microusd,
            "full_capture_coverage": full_capture_coverage
        },
        "readiness": {
            "provider_correlation_metadata_only": true,
            "local_accept_enabled": false,
            "market_money_claim_allowed": false,
            "product_promotion_allowed": false,
            "policy": "Codex session token_count backfill only: emits metadata-only provider-boundary rows from local token_count events; no provider request id, no provider cost evidence, no serving, no local accept, no money claim"
        },
        "blockers": blockers(&state, full_capture_coverage),
        "manual_class_list_used": false,
        "selector_used": false,
        "dynamic_discovery_performed": false,
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
        "boundary": "source adapter only: converts local Codex token_count telemetry into provider-boundary metadata rows for coverage audit; does not alter atoms, mine, score, compile, promote, serve, local_accept, or claim money"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_codex_token_backfill_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  output_provider_boundary_path: {}",
        output_provider_boundary_path.display()
    );
    println!("  capture_request_rows: {}", state.capture_request_rows);
    println!(
        "  emitted_provider_boundary_rows: {}",
        state.emitted_provider_boundary_rows
    );
    println!("  appended_total_tokens: {}", state.appended_total_tokens);
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_capture_index(path: &Path, state: &mut BackfillState) -> Result<CaptureIndex, String> {
    let text = read_text(path)?;
    let mut index = CaptureIndex::default();
    for (line_index, line) in text.lines().enumerate() {
        let Some(row) = parse_json_row(path, line_index + 1, line)? else {
            continue;
        };
        state.capture_request_rows += 1;
        let capture_request_id = json_string(&row, &["capture_request_id"])
            .unwrap_or_else(|| format!("capture_request_line_{}", line_index + 1));
        let join_keys = metadata_join_keys(&row)
            .into_iter()
            .collect::<BTreeSet<_>>();
        if !join_keys.is_empty() {
            state.capture_requests_with_join_keys += 1;
        }
        let capture = CaptureRequest {
            capture_request_id: capture_request_id.clone(),
            join_keys: join_keys.clone(),
            total_tokens: json_usize(&row, &["total_tokens"]).unwrap_or(0),
            total_cost_microusd: json_u64(&row, &["total_cost_microusd"]).unwrap_or(0),
        };
        for key in join_keys {
            index
                .key_to_capture_ids
                .entry(key)
                .or_default()
                .insert(capture_request_id.clone());
        }
        index.requests.insert(capture_request_id, capture);
    }
    Ok(index)
}

fn read_phase_sources(
    paths: &[PathBuf],
    index: &CaptureIndex,
    state: &mut BackfillState,
) -> Result<Vec<PhaseSource>, String> {
    let mut sources = Vec::new();
    for path in paths {
        let text = read_text(path)?;
        for (line_index, line) in text.lines().enumerate() {
            let Some(row) = parse_json_row(path, line_index + 1, line)? else {
                continue;
            };
            state.phase_trace_rows += 1;
            let join_keys = metadata_join_keys(&row);
            if matched_capture_ids(index, &join_keys).is_empty() {
                continue;
            }
            state.phase_rows_matching_capture += 1;
            let Some(input_trace_path) = json_string(&row, &["input_trace_path"]) else {
                continue;
            };
            let Some(event_timestamp) = json_string(&row, &["event_timestamp"]) else {
                continue;
            };
            let Some(event_ms) = parse_timestamp_ms(&event_timestamp) else {
                continue;
            };
            state.phase_rows_with_input_trace_path += 1;
            sources.push(PhaseSource {
                join_keys,
                event_timestamp,
                event_ms,
                input_trace_path: PathBuf::from(input_trace_path),
                trace_id: json_string(&row, &["trace_id"]),
                request_fingerprint: json_string(&row, &["request_fingerprint"]),
                exact_cache_key: json_string(&row, &["exact_cache_key"]),
            });
        }
    }
    Ok(sources)
}

fn read_session_token_boundaries(
    path: &Path,
    state: &mut BackfillState,
) -> Result<Option<Vec<TokenBoundary>>, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        state.source_session_files_missing += 1;
        return Ok(None);
    };
    state.source_session_files_read += 1;
    let mut out = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let Some(row) = parse_json_row(path, line_index + 1, line)? else {
            continue;
        };
        if row.get("type").and_then(Value::as_str) != Some("event_msg")
            || json_string(&row, &["payload", "type"]).as_deref() != Some("token_count")
        {
            continue;
        }
        state.token_count_events_seen += 1;
        let Some(timestamp) = json_string(&row, &["timestamp"]) else {
            continue;
        };
        let Some(timestamp_ms) = parse_timestamp_ms(&timestamp) else {
            continue;
        };
        let Some(total_tokens) = json_usize(
            &row,
            &["payload", "info", "last_token_usage", "total_tokens"],
        ) else {
            continue;
        };
        state.token_count_events_with_usage += 1;
        out.push(TokenBoundary {
            timestamp,
            timestamp_ms,
            source_line: line_index + 1,
            input_tokens: json_usize(
                &row,
                &["payload", "info", "last_token_usage", "input_tokens"],
            )
            .unwrap_or(0),
            cached_input_tokens: json_usize(
                &row,
                &["payload", "info", "last_token_usage", "cached_input_tokens"],
            )
            .unwrap_or(0),
            output_tokens: json_usize(
                &row,
                &["payload", "info", "last_token_usage", "output_tokens"],
            )
            .unwrap_or(0),
            reasoning_output_tokens: json_usize(
                &row,
                &[
                    "payload",
                    "info",
                    "last_token_usage",
                    "reasoning_output_tokens",
                ],
            )
            .unwrap_or(0),
            total_tokens,
        });
    }
    Ok(Some(out))
}

fn first_token_count_after(
    event_ms: i64,
    token_boundaries: &[TokenBoundary],
) -> Option<&TokenBoundary> {
    token_boundaries.iter().find(|boundary| {
        let delta = boundary.timestamp_ms.saturating_sub(event_ms);
        delta >= 0 && delta <= MAX_AFTER_EVENT_MS
    })
}

fn provider_boundary_row(
    capture: &CaptureRequest,
    source: &PhaseSource,
    boundary: &TokenBoundary,
) -> Value {
    let session_path_text = source.input_trace_path.display().to_string();
    let source_line_text = boundary.source_line.to_string();
    let provider_key = format!(
        "codex_token_count_event:{:016x}",
        super::stable_fingerprint([
            session_path_text.as_str(),
            source_line_text.as_str(),
            boundary.timestamp.as_str(),
        ])
    );
    let mut row = serde_json::json!({
        "schema_version": "provider_boundary_codex_token_backfill_v1",
        "capture_request_id": capture.capture_request_id,
        "match_keys": capture.join_keys.iter().cloned().collect::<Vec<_>>(),
        "external_provider_correlation_keys": [provider_key],
        "provider_correlation_ready": true,
        "token_boundary_source": "codex_session_event_msg_token_count",
        "token_boundary_join_policy": "first_token_count_after_phase_event",
        "phase_event_timestamp": source.event_timestamp,
        "codex_token_count_timestamp": boundary.timestamp,
        "codex_token_count_line": boundary.source_line,
        "codex_session_path": path_string(&source.input_trace_path),
        "provider_total_tokens": boundary.total_tokens,
        "input_tokens": boundary.input_tokens,
        "cached_input_tokens": boundary.cached_input_tokens,
        "output_tokens": boundary.output_tokens,
        "reasoning_output_tokens": boundary.reasoning_output_tokens,
        "capture_request_total_tokens": capture.total_tokens,
        "capture_request_total_cost_microusd": capture.total_cost_microusd,
        "token_cost": {
            "total_tokens": boundary.total_tokens,
            "input_tokens": boundary.input_tokens,
            "cached_input_tokens": boundary.cached_input_tokens,
            "output_tokens": boundary.output_tokens,
            "reasoning_output_tokens": boundary.reasoning_output_tokens,
            "total_cost_microusd": 0,
            "token_evidence_missing": false,
            "cost_evidence_missing": true,
            "token_cost_estimate_used": false
        },
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "boundary": "provider-boundary metadata row from local Codex token_count telemetry; no provider request id and no provider cost evidence"
    });
    if let Some(map) = row.as_object_mut() {
        if let Some(value) = &source.trace_id {
            map.entry("trace_id".to_owned())
                .or_insert_with(|| Value::String(value.clone()));
        }
        if let Some(value) = &source.request_fingerprint {
            map.entry("request_fingerprint".to_owned())
                .or_insert_with(|| Value::String(value.clone()));
        }
        if let Some(value) = &source.exact_cache_key {
            map.entry("exact_cache_key".to_owned())
                .or_insert_with(|| Value::String(value.clone()));
        }
    }
    row
}

fn blockers(state: &BackfillState, full_capture_coverage: bool) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if state.capture_request_rows == 0 {
        blockers.push("no_capture_request_rows");
    }
    if state.emitted_provider_boundary_rows == 0 {
        blockers.push("no_codex_token_boundary_rows_emitted");
    }
    if !full_capture_coverage {
        blockers.push("codex_token_boundary_coverage_incomplete");
    }
    if !state.missing_phase_source_capture_ids.is_empty() {
        blockers.push("some_capture_requests_missing_phase_source");
    }
    if !state.missing_session_file_capture_ids.is_empty() {
        blockers.push("some_capture_requests_missing_session_file");
    }
    if !state.missing_token_boundary_capture_ids.is_empty() {
        blockers.push("some_capture_requests_missing_token_count_after_event");
    }
    blockers.push("provider_request_id_absent");
    blockers.push("provider_cost_evidence_absent");
    blockers
}

fn matched_capture_ids(index: &CaptureIndex, join_keys: &[String]) -> BTreeSet<String> {
    let mut matched = BTreeSet::new();
    for key in join_keys {
        if let Some(capture_ids) = index.key_to_capture_ids.get(key) {
            matched.extend(capture_ids.iter().cloned());
        }
    }
    matched
}

fn metadata_join_keys(row: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    keys.extend(json_string_vec(json_at(row, &["join_keys"])));
    keys.extend(json_string_vec(json_at(row, &["match_keys"])));
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
        &[&["exact_cache_key"], &["metadata", "exact_cache_key"]],
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

fn parse_timestamp_ms(timestamp: &str) -> Option<i64> {
    let date_time = timestamp.strip_suffix('Z').unwrap_or(timestamp);
    let (date, time) = date_time.split_once('T')?;
    let mut date_parts = date.split('-');
    let year = date_parts.next()?.parse::<i64>().ok()?;
    let month = date_parts.next()?.parse::<usize>().ok()?;
    let day = date_parts.next()?.parse::<i64>().ok()?;
    let mut time_parts = time.split(':');
    let hour = time_parts.next()?.parse::<i64>().ok()?;
    let minute = time_parts.next()?.parse::<i64>().ok()?;
    let second_text = time_parts.next()?;
    let (second_part, fraction_part) = second_text
        .split_once('.')
        .map_or((second_text, ""), |parts| parts);
    let second = second_part.parse::<i64>().ok()?;
    let fraction_ms = fraction_part.chars().take(3).collect::<String>();
    let millis = if fraction_ms.is_empty() {
        0
    } else {
        format!("{fraction_ms:0<3}").parse::<i64>().ok()?
    };
    let days = days_before_year(year)
        .saturating_add(days_before_month(year, month)?)
        .saturating_add(day.saturating_sub(1));
    Some(
        days.saturating_mul(86_400_000)
            .saturating_add(hour.saturating_mul(3_600_000))
            .saturating_add(minute.saturating_mul(60_000))
            .saturating_add(second.saturating_mul(1_000))
            .saturating_add(millis),
    )
}

fn days_before_year(year: i64) -> i64 {
    let y = year.saturating_sub(1);
    y.saturating_mul(365)
        .saturating_add(y / 4)
        .saturating_sub(y / 100)
        .saturating_add(y / 400)
}

fn days_before_month(year: i64, month: usize) -> Option<i64> {
    if !(1..=12).contains(&month) {
        return None;
    }
    let month_days = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    Some(month_days.iter().take(month - 1).sum::<i32>() as i64)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
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
    json_at(value, path)?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path)?.as_u64()
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}
