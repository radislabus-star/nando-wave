use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_CAPTURE_REQUEST_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-capture-request-v1.report.json";
const DEFAULT_PROVIDER_BOUNDARY_CAPTURE_REQUEST_JSONL: &str =
    "target/nando-wave/streaming/provider-boundary-capture-request-v1.jsonl";

#[derive(Default)]
struct ProviderIndex {
    rows: usize,
    rows_with_join_keys: usize,
    rows_with_provider_keys: usize,
    join_key_to_provider_keys: BTreeMap<String, BTreeSet<String>>,
    provider_key_atom_leak_rows: usize,
}

#[derive(Default)]
struct CaptureState {
    phase_rows: usize,
    score_ready_rows: usize,
    rows_with_join_keys: usize,
    rows_missing_join_keys: usize,
    rows_with_existing_provider_keys: usize,
    score_ready_rows_with_provider_join: usize,
    score_ready_rows_missing_provider_join: usize,
    rows_requiring_provider_capture: usize,
    rows_requiring_join_key_capture: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    total_tokens_requiring_provider_capture: usize,
    total_cost_requiring_provider_capture_microusd: u64,
    provider_key_atom_leak_rows: usize,
    files: Vec<PhaseFileReport>,
}

#[derive(Default, Serialize)]
struct PhaseFileReport {
    path: String,
    rows: usize,
    score_ready_rows: usize,
    rows_with_join_keys: usize,
    rows_missing_join_keys: usize,
    rows_with_existing_provider_keys: usize,
    score_ready_rows_with_provider_join: usize,
    score_ready_rows_missing_provider_join: usize,
    rows_requiring_provider_capture: usize,
    rows_requiring_join_key_capture: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    total_tokens_requiring_provider_capture: usize,
    total_cost_requiring_provider_capture_microusd: u64,
    provider_key_atom_leak_rows: usize,
}

#[derive(Default)]
struct CaptureBucket {
    primary_join_key: String,
    join_keys: BTreeSet<String>,
    row_count: usize,
    total_tokens: usize,
    total_cost_microusd: u64,
    token_cost_estimate_rows: usize,
    token_evidence_missing_rows: usize,
    cost_evidence_missing_rows: usize,
    action_families: BTreeMap<String, usize>,
    route_keys: BTreeMap<String, usize>,
    profile_ids: BTreeMap<String, usize>,
    traffic_sources: BTreeMap<String, usize>,
    verification_sources: BTreeMap<String, usize>,
    sample_sources: Vec<Value>,
}

pub(crate) fn run_phase_stream_provider_boundary_capture_request_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_CAPTURE_REQUEST_REPORT));
    let output_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_CAPTURE_REQUEST_JSONL));

    let mut phase_paths = Vec::<PathBuf>::new();
    let mut provider_paths = Vec::<PathBuf>::new();
    let mut provider_mode = false;
    for arg in args {
        if arg == "--provider" {
            provider_mode = true;
            continue;
        }
        if provider_mode {
            provider_paths.push(PathBuf::from(arg));
        } else {
            phase_paths.push(PathBuf::from(arg));
        }
    }
    if phase_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let provider_index = build_provider_index(&provider_paths)?;
    let mut state = CaptureState::default();
    let mut buckets = BTreeMap::<String, CaptureBucket>::new();
    for path in &phase_paths {
        let file = scan_phase_file(path, &provider_index, &mut state, &mut buckets)?;
        state.files.push(file);
    }

    let mut output = String::new();
    for bucket in buckets.values() {
        output.push_str(
            &serde_json::to_string(&capture_request_row(bucket))
                .map_err(|error| format!("failed to serialize capture request row: {error}"))?,
        );
        output.push('\n');
    }
    write_text_file(&output_jsonl_path, &output)?;

    let provider_join_key_count = provider_index.join_key_to_provider_keys.len();
    let unique_capture_requests = buckets.len();
    let provider_capture_required = state.rows_requiring_provider_capture > 0;
    let join_key_capture_required = state.rows_requiring_join_key_capture > 0;
    let provider_correlation_metadata_only =
        provider_index.provider_key_atom_leak_rows == 0 && state.provider_key_atom_leak_rows == 0;
    let economic_join_ready = state.score_ready_rows > 0
        && state.score_ready_rows_missing_provider_join == 0
        && state.score_ready_rows_with_provider_join == state.score_ready_rows
        && provider_correlation_metadata_only;

    let mut blockers = Vec::<&'static str>::new();
    if state.score_ready_rows == 0 {
        blockers.push("no_score_ready_phase_rows");
    }
    if provider_capture_required {
        blockers.push("provider_boundary_capture_required_for_score_ready_rows");
    }
    if join_key_capture_required {
        blockers.push("phase_rows_missing_stable_join_keys");
    }
    if !provider_correlation_metadata_only {
        blockers.push("provider_key_atom_leak");
    }
    if state.total_tokens_requiring_provider_capture == 0 && provider_capture_required {
        blockers.push("missing_token_denominator_for_capture_requests");
    }

    let verdict = if !provider_correlation_metadata_only {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CAPTURE_REQUEST_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if economic_join_ready {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CAPTURE_REQUEST_V1_PASS_NO_CAPTURE_REQUESTS"
    } else if provider_capture_required {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CAPTURE_REQUEST_V1_WATCH_CAPTURE_REQUESTS_EMITTED"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_CAPTURE_REQUEST_V1_WATCH_NOT_ECONOMIC_JOIN_READY"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_capture_request_v1",
        "phase_trace_paths": phase_paths.iter().map(|path| path_string(path)).collect::<Vec<_>>(),
        "provider_boundary_paths": provider_paths.iter().map(|path| path_string(path)).collect::<Vec<_>>(),
        "output_jsonl_path": path_string(&output_jsonl_path),
        "phase": {
            "rows": state.phase_rows,
            "score_ready_rows": state.score_ready_rows,
            "rows_with_join_keys": state.rows_with_join_keys,
            "rows_missing_join_keys": state.rows_missing_join_keys,
            "rows_with_existing_provider_keys": state.rows_with_existing_provider_keys,
            "score_ready_rows_with_provider_join": state.score_ready_rows_with_provider_join,
            "score_ready_rows_missing_provider_join": state.score_ready_rows_missing_provider_join,
            "rows_requiring_provider_capture": state.rows_requiring_provider_capture,
            "rows_requiring_join_key_capture": state.rows_requiring_join_key_capture,
            "total_tokens": state.total_tokens,
            "total_cost_microusd": state.total_cost_microusd,
            "total_tokens_requiring_provider_capture": state.total_tokens_requiring_provider_capture,
            "total_cost_requiring_provider_capture_microusd": state.total_cost_requiring_provider_capture_microusd,
            "provider_key_atom_leak_rows": state.provider_key_atom_leak_rows,
            "files": state.files
        },
        "provider": {
            "rows": provider_index.rows,
            "rows_with_join_keys": provider_index.rows_with_join_keys,
            "rows_with_provider_keys": provider_index.rows_with_provider_keys,
            "join_key_count": provider_join_key_count,
            "provider_key_atom_leak_rows": provider_index.provider_key_atom_leak_rows
        },
        "capture_requests": {
            "unique_capture_requests": unique_capture_requests,
            "provider_capture_required": provider_capture_required,
            "join_key_capture_required": join_key_capture_required,
            "policy": "worklist only: live adapter or provider export must supply provider-boundary metadata for these score-ready rows; this command does not fabricate provider ids"
        },
        "readiness": {
            "provider_correlation_metadata_only": provider_correlation_metadata_only,
            "economic_join_ready": economic_join_ready,
            "market_money_claim_allowed": false,
            "local_accept_enabled": false
        },
        "blockers": blockers,
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
        "boundary": "capture-request export only: no mining, no scoring, no .nwpc compile, no serving, no promotion, no local accept, no market money claim"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_capture_request_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_jsonl_path: {}", output_jsonl_path.display());
    println!("  score_ready_rows: {}", state.score_ready_rows);
    println!(
        "  rows_requiring_provider_capture: {}",
        state.rows_requiring_provider_capture
    );
    println!("  unique_capture_requests: {unique_capture_requests}");
    println!(
        "  total_tokens_requiring_provider_capture: {}",
        state.total_tokens_requiring_provider_capture
    );
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn build_provider_index(paths: &[PathBuf]) -> Result<ProviderIndex, String> {
    let mut index = ProviderIndex::default();
    for path in paths {
        let text = read_text(path)?;
        for (line_index, line) in text.lines().enumerate() {
            let Some(row) = parse_jsonl_row(path, line_index + 1, line)? else {
                continue;
            };
            index.rows += 1;
            let join_keys = metadata_join_keys(&row);
            let provider_keys = super::phase_atom_external_provider_correlation_keys(&row)
                .into_iter()
                .collect::<BTreeSet<_>>();
            index.rows_with_join_keys += usize::from(!join_keys.is_empty());
            index.rows_with_provider_keys += usize::from(!provider_keys.is_empty());
            if provider_key_leaks_into_atoms(&row) {
                index.provider_key_atom_leak_rows += 1;
            }
            if join_keys.is_empty() || provider_keys.is_empty() {
                continue;
            }
            for join_key in join_keys {
                index
                    .join_key_to_provider_keys
                    .entry(join_key)
                    .or_default()
                    .extend(provider_keys.iter().cloned());
            }
        }
    }
    Ok(index)
}

fn scan_phase_file(
    path: &Path,
    provider_index: &ProviderIndex,
    state: &mut CaptureState,
    buckets: &mut BTreeMap<String, CaptureBucket>,
) -> Result<PhaseFileReport, String> {
    let text = read_text(path)?;
    let mut file = PhaseFileReport {
        path: path_string(path),
        ..PhaseFileReport::default()
    };
    for (line_index, line) in text.lines().enumerate() {
        let Some(row) = parse_jsonl_row(path, line_index + 1, line)? else {
            continue;
        };
        state.phase_rows += 1;
        file.rows += 1;

        let score_ready = json_bool(&row, &["ready_for_existing_shadow_scoring"]).unwrap_or(false);
        state.score_ready_rows += usize::from(score_ready);
        file.score_ready_rows += usize::from(score_ready);

        let join_keys = metadata_join_keys(&row);
        state.rows_with_join_keys += usize::from(!join_keys.is_empty());
        state.rows_missing_join_keys += usize::from(join_keys.is_empty());
        file.rows_with_join_keys += usize::from(!join_keys.is_empty());
        file.rows_missing_join_keys += usize::from(join_keys.is_empty());

        let mut provider_keys = super::phase_atom_external_provider_correlation_keys(&row)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let had_existing_provider_keys = !provider_keys.is_empty();
        state.rows_with_existing_provider_keys += usize::from(had_existing_provider_keys);
        file.rows_with_existing_provider_keys += usize::from(had_existing_provider_keys);
        for join_key in &join_keys {
            if let Some(keys) = provider_index.join_key_to_provider_keys.get(join_key) {
                provider_keys.extend(keys.iter().cloned());
            }
        }
        let joined_with_provider = !provider_keys.is_empty();
        if score_ready && joined_with_provider {
            state.score_ready_rows_with_provider_join += 1;
            file.score_ready_rows_with_provider_join += 1;
        }
        if score_ready && !joined_with_provider {
            state.score_ready_rows_missing_provider_join += 1;
            file.score_ready_rows_missing_provider_join += 1;
        }

        let total_tokens = total_tokens(&row);
        let total_cost = total_cost_microusd(&row);
        state.total_tokens = state.total_tokens.saturating_add(total_tokens);
        state.total_cost_microusd = state.total_cost_microusd.saturating_add(total_cost);
        file.total_tokens = file.total_tokens.saturating_add(total_tokens);
        file.total_cost_microusd = file.total_cost_microusd.saturating_add(total_cost);

        if provider_key_leaks_into_atoms(&row) {
            state.provider_key_atom_leak_rows += 1;
            file.provider_key_atom_leak_rows += 1;
        }

        if !score_ready || joined_with_provider {
            continue;
        }
        if join_keys.is_empty() {
            state.rows_requiring_join_key_capture += 1;
            file.rows_requiring_join_key_capture += 1;
            continue;
        }

        state.rows_requiring_provider_capture += 1;
        state.total_tokens_requiring_provider_capture = state
            .total_tokens_requiring_provider_capture
            .saturating_add(total_tokens);
        state.total_cost_requiring_provider_capture_microusd = state
            .total_cost_requiring_provider_capture_microusd
            .saturating_add(total_cost);
        file.rows_requiring_provider_capture += 1;
        file.total_tokens_requiring_provider_capture = file
            .total_tokens_requiring_provider_capture
            .saturating_add(total_tokens);
        file.total_cost_requiring_provider_capture_microusd = file
            .total_cost_requiring_provider_capture_microusd
            .saturating_add(total_cost);

        let primary_join_key = join_keys[0].clone();
        let bucket = buckets
            .entry(primary_join_key.clone())
            .or_insert_with(|| CaptureBucket {
                primary_join_key,
                ..CaptureBucket::default()
            });
        observe_bucket_row(
            bucket,
            path,
            line_index + 1,
            &row,
            &join_keys,
            total_tokens,
            total_cost,
        );
    }
    Ok(file)
}

fn observe_bucket_row(
    bucket: &mut CaptureBucket,
    path: &Path,
    line_number: usize,
    row: &Value,
    join_keys: &[String],
    total_tokens: usize,
    total_cost_microusd: u64,
) {
    bucket.row_count += 1;
    bucket.total_tokens = bucket.total_tokens.saturating_add(total_tokens);
    bucket.total_cost_microusd = bucket
        .total_cost_microusd
        .saturating_add(total_cost_microusd);
    bucket.join_keys.extend(join_keys.iter().cloned());
    if json_bool(row, &["token_cost", "token_cost_estimate_used"]).unwrap_or(false) {
        bucket.token_cost_estimate_rows += 1;
    }
    if json_bool(row, &["token_cost", "token_evidence_missing"]).unwrap_or(false) {
        bucket.token_evidence_missing_rows += 1;
    }
    if json_bool(row, &["token_cost", "cost_evidence_missing"]).unwrap_or(false) {
        bucket.cost_evidence_missing_rows += 1;
    }
    for action_family in action_families(row) {
        increment(&mut bucket.action_families, action_family);
    }
    if let Some(route_key) = json_string(row, &["nando_shadow_request", "route_key"]) {
        increment(&mut bucket.route_keys, route_key);
    }
    if let Some(profile_id) = json_string(row, &["nando_shadow_request", "profile_id"]) {
        increment(&mut bucket.profile_ids, profile_id);
    }
    if let Some(source) = json_string(row, &["traffic_source"]) {
        increment(&mut bucket.traffic_sources, source);
    }
    if let Some(source) = json_string(row, &["verification_source_kind"]) {
        increment(&mut bucket.verification_sources, source);
    }
    if bucket.sample_sources.len() < 3 {
        bucket.sample_sources.push(serde_json::json!({
            "phase_trace_path": path_string(path),
            "phase_line_number": line_number,
            "trace_id": json_string(row, &["trace_id"]),
            "request_fingerprint": json_string(row, &["request_fingerprint"]),
            "exact_cache_key": json_string(row, &["exact_cache_key"]),
            "event_timestamp": json_string(row, &["event_timestamp"])
        }));
    }
}

fn capture_request_row(bucket: &CaptureBucket) -> Value {
    let capture_id = format!(
        "provider_capture_request_{:016x}",
        super::stable_fingerprint([bucket.primary_join_key.as_str()])
    );
    serde_json::json!({
        "schema_version": "provider_boundary_capture_request_v1",
        "capture_request_id": capture_id,
        "primary_join_key": bucket.primary_join_key,
        "join_keys": bucket.join_keys.iter().cloned().collect::<Vec<_>>(),
        "phase_row_count": bucket.row_count,
        "total_tokens": bucket.total_tokens,
        "total_cost_microusd": bucket.total_cost_microusd,
        "token_cost_estimate_rows": bucket.token_cost_estimate_rows,
        "token_evidence_missing_rows": bucket.token_evidence_missing_rows,
        "cost_evidence_missing_rows": bucket.cost_evidence_missing_rows,
        "action_families": bucket.action_families,
        "route_keys": bucket.route_keys,
        "profile_ids": bucket.profile_ids,
        "traffic_sources": bucket.traffic_sources,
        "verification_sources": bucket.verification_sources,
        "sample_sources": bucket.sample_sources,
        "provider_capture_required": true,
        "provider_correlation_metadata_only_required": true,
        "must_not_emit_provider_keys_as_atoms": true,
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "boundary": "request for live provider-boundary capture or external provider export join; does not contain provider ids and must not be used as runtime authority"
    })
}

fn metadata_join_keys(row: &Value) -> Vec<String> {
    let paths: &[(&str, &[&str])] = &[
        ("request_fingerprint", &["request_fingerprint"]),
        ("request_fingerprint", &["metadata", "request_fingerprint"]),
        ("exact_cache_key", &["exact_cache_key"]),
        ("exact_cache_key", &["metadata", "exact_cache_key"]),
        ("trace_id", &["trace_id"]),
        ("trace_id", &["metadata", "trace_id"]),
        ("event_id", &["event_id"]),
        ("event_id", &["metadata", "event_id"]),
        ("client_correlation_id", &["client_correlation_id"]),
        (
            "client_correlation_id",
            &["metadata", "client_correlation_id"],
        ),
    ];
    let mut keys = paths
        .iter()
        .filter_map(|(label, path)| json_string(row, path).map(|value| format!("{label}:{value}")))
        .filter(|value| !value.ends_with(':'))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

fn action_families(row: &Value) -> Vec<String> {
    let mut atoms = Vec::<String>::new();
    collect_atom_strings(row.get("action_atoms"), &mut atoms);
    collect_atom_strings(json_at(row, &["atom_groups", "action_atoms"]), &mut atoms);
    let mut families = atoms
        .into_iter()
        .filter(|atom| atom.starts_with("action_family:"))
        .collect::<Vec<_>>();
    families.sort();
    families.dedup();
    families
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

fn total_tokens(row: &Value) -> usize {
    json_usize(row, &["token_cost", "total_tokens"])
        .or_else(|| json_usize(row, &["estimated_total_tokens"]))
        .unwrap_or(0)
}

fn total_cost_microusd(row: &Value) -> u64 {
    json_u64(row, &["token_cost", "total_cost_microusd"])
        .or_else(|| json_u64(row, &["estimated_total_cost_microusd"]))
        .unwrap_or(0)
}

fn increment(map: &mut BTreeMap<String, usize>, key: String) {
    *map.entry(key).or_insert(0) += 1;
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSONL '{}': {error}", path.display()))
}

fn write_text_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    std::fs::write(path, text)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn parse_jsonl_row(path: &Path, line_number: usize, line: &str) -> Result<Option<Value>, String> {
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

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    json_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(Value::as_u64)
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}
