use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

const DEFAULT_LIVE_CAPTURE_READINESS_REPORT: &str =
    "target/nando-wave/streaming/phase-atom-live-capture-readiness-v1.report.json";

#[derive(Default)]
struct LiveCaptureReadinessState {
    total_rows: usize,
    rows_with_provider_correlation_keys: usize,
    rows_missing_provider_correlation_keys: usize,
    rows_with_provider_key_atom_leak: usize,
    rows_with_shadow_request: usize,
    rows_with_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    rows_ready_for_route_family_mining: usize,
    rows_ready_for_existing_shadow_scoring: usize,
    score_ready_rows_with_provider_correlation: usize,
    score_ready_rows_missing_provider_correlation: usize,
    economic_score_ready_rows: usize,
    economic_score_ready_rows_with_provider_correlation: usize,
    economic_score_ready_rows_missing_provider_correlation: usize,
    zero_denominator_score_ready_rows: usize,
    zero_denominator_score_ready_rows_missing_provider_correlation: usize,
    mining_ready_rows_with_provider_correlation: usize,
    mining_ready_rows_missing_provider_correlation: usize,
    rows_with_token_or_cost: usize,
    rows_with_provider_cost: usize,
    rows_with_estimated_cost: usize,
    rows_with_positive_tokens: usize,
    rows_with_operator_detail_basis_atoms: usize,
    rows_ready_for_operator_detail_mining: usize,
    provider_key_count: usize,
    schema_counts: BTreeMap<String, usize>,
    traffic_source_counts: BTreeMap<String, usize>,
    top_operator_detail_basis_atoms: BTreeMap<String, usize>,
}

pub(crate) fn run_phase_stream_phase_atom_live_capture_readiness_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LIVE_CAPTURE_READINESS_REPORT));
    let input_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let mut state = LiveCaptureReadinessState::default();
    for path in &input_paths {
        scan_path(path, &mut state)?;
    }

    let discovery_capture_ready = state.rows_ready_for_route_family_mining > 0;
    let score_capture_ready = state.rows_ready_for_existing_shadow_scoring > 0;
    let provider_correlation_complete_for_score_ready = state.economic_score_ready_rows > 0
        && state.economic_score_ready_rows_missing_provider_correlation == 0
        && state.economic_score_ready_rows_with_provider_correlation
            == state.economic_score_ready_rows;
    let full_score_ready_provider_correlation = score_capture_ready
        && state.score_ready_rows_missing_provider_correlation == 0
        && state.score_ready_rows_with_provider_correlation
            == state.rows_ready_for_existing_shadow_scoring;
    let zero_denominator_explains_missing_score_ready =
        state.score_ready_rows_missing_provider_correlation > 0
            && state.score_ready_rows_missing_provider_correlation
                == state.zero_denominator_score_ready_rows_missing_provider_correlation;
    let provider_correlation_complete_for_mining_ready = discovery_capture_ready
        && state.mining_ready_rows_missing_provider_correlation == 0
        && state.mining_ready_rows_with_provider_correlation
            == state.rows_ready_for_route_family_mining;
    let provider_correlation_metadata_only = state.rows_with_provider_key_atom_leak == 0;
    let economic_capture_ready = score_capture_ready
        && provider_correlation_complete_for_score_ready
        && provider_correlation_metadata_only
        && state.rows_with_positive_tokens > 0;
    let operator_detail_capture_ready = state.rows_ready_for_operator_detail_mining > 0;

    let mut blockers = Vec::<String>::new();
    if state.total_rows == 0 {
        blockers.push("empty_input".to_owned());
    }
    if !discovery_capture_ready {
        blockers.push("no_rows_ready_for_route_family_mining".to_owned());
    }
    if !score_capture_ready {
        blockers.push("no_rows_ready_for_existing_shadow_scoring".to_owned());
    }
    if state.economic_score_ready_rows_missing_provider_correlation > 0 {
        blockers.push("provider_correlation_missing_for_score_ready_rows".to_owned());
    }
    if state.mining_ready_rows_missing_provider_correlation > 0 && !economic_capture_ready {
        blockers.push("provider_correlation_missing_for_mining_ready_rows".to_owned());
    }
    if !provider_correlation_metadata_only {
        blockers.push("provider_correlation_atom_leak".to_owned());
    }
    if state.rows_with_positive_tokens == 0 {
        blockers.push("no_positive_token_denominator".to_owned());
    }
    if state.rows_with_operator_detail_basis_atoms == 0 {
        blockers.push("no_operator_detail_basis_atoms_for_multi_split".to_owned());
    }
    if state.rows_ready_for_operator_detail_mining == 0 {
        blockers.push("no_rows_ready_for_operator_detail_multi_split".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    let verdict = if !provider_correlation_metadata_only {
        "PHASE_ATOM_LIVE_CAPTURE_READINESS_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if economic_capture_ready && !operator_detail_capture_ready {
        "PHASE_ATOM_LIVE_CAPTURE_READINESS_V1_WATCH_OPERATOR_DETAIL_BASIS_MISSING"
    } else if economic_capture_ready && full_score_ready_provider_correlation {
        "PHASE_ATOM_LIVE_CAPTURE_READINESS_V1_PASS_ECONOMIC_CAPTURE_READY"
    } else if economic_capture_ready && zero_denominator_explains_missing_score_ready {
        "PHASE_ATOM_LIVE_CAPTURE_READINESS_V1_PASS_ECONOMIC_CAPTURE_READY_WITH_ZERO_DENOMINATOR_EXCLUSIONS"
    } else if state.rows_with_provider_correlation_keys > 0 {
        "PHASE_ATOM_LIVE_CAPTURE_READINESS_V1_WATCH_PARTIAL_PROVIDER_CORRELATION"
    } else if state.total_rows > 0 {
        "PHASE_ATOM_LIVE_CAPTURE_READINESS_V1_WATCH_NO_PROVIDER_CORRELATION"
    } else {
        "PHASE_ATOM_LIVE_CAPTURE_READINESS_V1_WATCH_EMPTY_INPUT"
    };

    let mut report = serde_json::json!({
        "report_kind": "phase_atom_live_capture_readiness_v1",
        "input_paths": input_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>(),
        "total_rows": state.total_rows,
        "rows_with_provider_correlation_keys": state.rows_with_provider_correlation_keys,
        "rows_missing_provider_correlation_keys": state.rows_missing_provider_correlation_keys,
        "provider_key_count": state.provider_key_count,
        "rows_with_provider_key_atom_leak": state.rows_with_provider_key_atom_leak,
        "provider_correlation_metadata_only": provider_correlation_metadata_only,
        "rows_with_shadow_request": state.rows_with_shadow_request,
        "rows_with_verifier_label": state.rows_with_verifier_label,
        "verifier_true_rows": state.verifier_true_rows,
        "verifier_false_rows": state.verifier_false_rows,
        "rows_ready_for_route_family_mining": state.rows_ready_for_route_family_mining,
        "rows_ready_for_existing_shadow_scoring": state.rows_ready_for_existing_shadow_scoring,
        "score_ready_rows_with_provider_correlation": state.score_ready_rows_with_provider_correlation,
        "score_ready_rows_missing_provider_correlation": state.score_ready_rows_missing_provider_correlation,
        "economic_score_ready_rows": state.economic_score_ready_rows,
        "economic_score_ready_rows_with_provider_correlation": state.economic_score_ready_rows_with_provider_correlation,
        "economic_score_ready_rows_missing_provider_correlation": state.economic_score_ready_rows_missing_provider_correlation,
        "zero_denominator_score_ready_rows": state.zero_denominator_score_ready_rows,
        "zero_denominator_score_ready_rows_missing_provider_correlation": state.zero_denominator_score_ready_rows_missing_provider_correlation,
        "mining_ready_rows_with_provider_correlation": state.mining_ready_rows_with_provider_correlation,
        "mining_ready_rows_missing_provider_correlation": state.mining_ready_rows_missing_provider_correlation,
        "rows_with_token_or_cost": state.rows_with_token_or_cost,
        "rows_with_provider_cost": state.rows_with_provider_cost,
        "rows_with_estimated_cost": state.rows_with_estimated_cost,
        "rows_with_positive_tokens": state.rows_with_positive_tokens,
        "schema_counts": state.schema_counts,
        "traffic_source_counts": state.traffic_source_counts,
        "readiness": {
            "discovery_capture_ready": discovery_capture_ready,
            "score_capture_ready": score_capture_ready,
            "provider_correlation_complete_for_score_ready": provider_correlation_complete_for_score_ready,
            "full_score_ready_provider_correlation": full_score_ready_provider_correlation,
            "zero_denominator_explains_missing_score_ready": zero_denominator_explains_missing_score_ready,
            "provider_correlation_complete_for_mining_ready": provider_correlation_complete_for_mining_ready,
            "economic_capture_ready": economic_capture_ready,
            "policy": "economic capture requires score-ready phase-atom rows to carry provider correlation metadata before online mining/billing/admission; provider ids must stay out of phase atoms"
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
        "boundary": "capture-readiness audit only: checks whether phase-atom trace rows are useful for automatic discovery and future external billing join; does not compile, score, promote, serve, enable local_accept, or estimate missing money"
    });
    if let Some(map) = report.as_object_mut() {
        map.insert(
            "rows_with_operator_detail_basis_atoms".to_owned(),
            serde_json::json!(state.rows_with_operator_detail_basis_atoms),
        );
        map.insert(
            "rows_ready_for_operator_detail_mining".to_owned(),
            serde_json::json!(state.rows_ready_for_operator_detail_mining),
        );
        map.insert(
            "top_operator_detail_basis_atoms".to_owned(),
            serde_json::Value::Array(atom_count_rows(&state.top_operator_detail_basis_atoms, 32)),
        );
        if let Some(readiness) = map
            .get_mut("readiness")
            .and_then(serde_json::Value::as_object_mut)
        {
            readiness.insert(
                "operator_detail_basis_ready".to_owned(),
                serde_json::json!(operator_detail_capture_ready),
            );
            readiness.insert(
                "operator_detail_policy".to_owned(),
                serde_json::json!(
                    "automatic multi-split requires non-shortcut operator-detail basis atoms"
                ),
            );
        }
    }
    write_json_file(&report_path, &report)?;
    println!("phase_atom_live_capture_readiness_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  total_rows: {}", state.total_rows);
    println!(
        "  rows_ready_for_existing_shadow_scoring: {}",
        state.rows_ready_for_existing_shadow_scoring
    );
    println!(
        "  score_ready_rows_with_provider_correlation: {}",
        state.score_ready_rows_with_provider_correlation
    );
    println!("  economic_capture_ready: {economic_capture_ready}");
    println!(
        "  rows_ready_for_operator_detail_mining: {}",
        state.rows_ready_for_operator_detail_mining
    );
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn scan_path(path: &Path, state: &mut LiveCaptureReadinessState) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read phase-atom trace '{}': {error}",
            path.display()
        )
    })?;
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse phase-atom trace '{}' line {}: {error}",
                path.display(),
                line_index + 1
            )
        })?;
        if row.is_object() {
            scan_row(&row, state);
        }
    }
    Ok(())
}

fn scan_row(row: &Value, state: &mut LiveCaptureReadinessState) {
    state.total_rows += 1;
    let provider_keys = super::phase_atom_external_provider_correlation_keys(row);
    let has_provider_keys = !provider_keys.is_empty();
    state.provider_key_count = state.provider_key_count.saturating_add(provider_keys.len());
    state.rows_with_provider_correlation_keys += usize::from(has_provider_keys);
    state.rows_missing_provider_correlation_keys += usize::from(!has_provider_keys);

    if provider_key_leaks_into_atoms(row) {
        state.rows_with_provider_key_atom_leak += 1;
    }

    let schema = json_string(row, &["schema_version"]).unwrap_or_else(|| "unknown".to_owned());
    *state.schema_counts.entry(schema).or_default() += 1;
    let traffic_source =
        json_string(row, &["traffic_source"]).unwrap_or_else(|| "unknown".to_owned());
    *state
        .traffic_source_counts
        .entry(traffic_source)
        .or_default() += 1;

    let has_shadow_request = json_bool(row, &["has_shadow_request"]).unwrap_or(false)
        || row
            .get("nando_shadow_request")
            .is_some_and(Value::is_object);
    state.rows_with_shadow_request += usize::from(has_shadow_request);

    let verifier_label = json_bool(row, &["verified_safe_accept"]);
    state.rows_with_verifier_label += usize::from(verifier_label.is_some());
    state.verifier_true_rows += usize::from(verifier_label == Some(true));
    state.verifier_false_rows += usize::from(verifier_label == Some(false));

    let ready_for_route_family_mining =
        json_bool(row, &["ready_for_route_family_mining"]).unwrap_or(false);
    let ready_for_existing_shadow_scoring =
        json_bool(row, &["ready_for_existing_shadow_scoring"]).unwrap_or(false);
    let total_tokens = json_usize(row, &["token_cost", "total_tokens"])
        .or_else(|| json_usize(row, &["estimated_total_tokens"]))
        .unwrap_or(0);
    let economic_score_ready = ready_for_existing_shadow_scoring && total_tokens > 0;
    let zero_denominator_score_ready = ready_for_existing_shadow_scoring && total_tokens == 0;
    state.rows_ready_for_route_family_mining += usize::from(ready_for_route_family_mining);
    state.rows_ready_for_existing_shadow_scoring += usize::from(ready_for_existing_shadow_scoring);
    if ready_for_existing_shadow_scoring && has_provider_keys {
        state.score_ready_rows_with_provider_correlation += 1;
    }
    if ready_for_existing_shadow_scoring && !has_provider_keys {
        state.score_ready_rows_missing_provider_correlation += 1;
    }
    if economic_score_ready {
        state.economic_score_ready_rows += 1;
        if has_provider_keys {
            state.economic_score_ready_rows_with_provider_correlation += 1;
        } else {
            state.economic_score_ready_rows_missing_provider_correlation += 1;
        }
    }
    if zero_denominator_score_ready {
        state.zero_denominator_score_ready_rows += 1;
        if !has_provider_keys {
            state.zero_denominator_score_ready_rows_missing_provider_correlation += 1;
        }
    }
    if ready_for_route_family_mining && has_provider_keys {
        state.mining_ready_rows_with_provider_correlation += 1;
    }
    if ready_for_route_family_mining && !has_provider_keys {
        state.mining_ready_rows_missing_provider_correlation += 1;
    }

    let estimated_cost = json_u64(row, &["token_cost", "total_cost_microusd"])
        .or_else(|| json_u64(row, &["estimated_total_cost_microusd"]))
        .unwrap_or(0);
    let provider_cost = json_u64(row, &["provider_cost_microusd"])
        .or_else(|| json_u64(row, &["token_cost", "provider_cost_microusd"]))
        .unwrap_or(0);
    state.rows_with_positive_tokens += usize::from(total_tokens > 0);
    state.rows_with_estimated_cost += usize::from(estimated_cost > 0 && provider_cost == 0);
    state.rows_with_provider_cost += usize::from(provider_cost > 0);
    state.rows_with_token_or_cost +=
        usize::from(total_tokens > 0 || estimated_cost > 0 || provider_cost > 0);

    let operator_detail_basis_atoms = operator_detail_basis_atoms(row);
    state.rows_with_operator_detail_basis_atoms +=
        usize::from(!operator_detail_basis_atoms.is_empty());
    if ready_for_route_family_mining
        && verifier_label.is_some()
        && has_provider_keys
        && !operator_detail_basis_atoms.is_empty()
    {
        state.rows_ready_for_operator_detail_mining += 1;
    }
    for atom in operator_detail_basis_atoms {
        *state
            .top_operator_detail_basis_atoms
            .entry(atom)
            .or_default() += 1;
    }
}

fn operator_detail_basis_atoms(row: &Value) -> Vec<String> {
    let mut atoms = Vec::new();
    for key in [
        "request_atoms",
        "state_atoms",
        "action_atoms",
        "tool_atoms",
        "route_hint_atoms",
    ] {
        collect_atom_strings(row.get(key), &mut atoms);
        collect_atom_strings(json_at(row, &["atom_groups", key]), &mut atoms);
    }
    let mut atoms = atoms
        .into_iter()
        .filter(|atom| operator_detail_basis_rejection(atom).is_none())
        .collect::<Vec<_>>();
    atoms.sort();
    atoms.dedup();
    atoms
}

fn operator_detail_basis_rejection(atom: &str) -> Option<&'static str> {
    if atom.is_empty() {
        return Some("empty_atom");
    }
    if atom.starts_with("output_hash64:")
        || atom.starts_with("verifier_label:")
        || atom.starts_with("verified_safe_accept:")
        || atom.starts_with("request_fingerprint:")
        || atom.starts_with("exact_cache_key:")
        || atom.starts_with("trace_id:")
        || atom.starts_with("source_trace_id:")
        || atom.starts_with("state_session_bucket:")
        || atom.starts_with("provider_")
        || atom.starts_with("external_provider_")
    {
        return Some("identity_or_provider_atom");
    }
    if atom.starts_with("token_band:")
        || atom.starts_with("cost_band:")
        || atom.starts_with("request_token_band:")
        || atom.starts_with("request_cost_band:")
        || atom.starts_with("state_token_band:")
        || atom.starts_with("state_cost_band:")
    {
        return Some("token_or_cost_band");
    }
    if atom.starts_with("shadow_active_") || atom.starts_with("shadow_slot_") {
        return Some("shadow_payload_shape");
    }
    if atom.starts_with("action_family:")
        || atom.starts_with("domain_family:")
        || atom.starts_with("route_operator:")
        || atom.starts_with("subroute_operator:")
        || atom.starts_with("request_route_family:")
        || atom.starts_with("route_key:")
        || atom.starts_with("profile_id:")
        || atom.starts_with("route_hint:")
        || atom.starts_with("route_hint_from_traffic_source:")
    {
        return Some("broad_route_or_action");
    }
    if atom.starts_with("tool_mention:")
        || atom.starts_with("tool_command_kind:")
        || atom.starts_with("tool_command_shell_family:")
    {
        return Some("tool_identity_atom");
    }
    if atom.starts_with("state_source:")
        || atom.starts_with("state_verification_source_kind:")
        || atom.starts_with("request_traffic_source_kind:")
        || atom.starts_with("traffic_source_kind:")
        || atom.starts_with("metadata_")
    {
        return Some("source_or_metadata_identity");
    }
    if atom == "request_has_shadow_request:true"
        || atom == "request_has_shadow_request:false"
        || atom == "state_has_verifier_label:true"
        || atom == "state_has_verifier_label:false"
        || atom == "tool_call_fingerprint_present:true"
        || atom.starts_with("tool_call_fingerprint_count_band:")
    {
        return Some("shape_or_presence_flag");
    }
    if atom.starts_with("request_char_band:")
        || atom.starts_with("request_line_count_band:")
        || atom.starts_with("request_word_count_band:")
        || atom.starts_with("request_has_code_fence:")
        || atom.starts_with("request_has_json_shape:")
        || atom.starts_with("request_has_cyrillic:")
        || atom.starts_with("request_has_latin:")
        || atom.starts_with("request_has_question:")
        || atom.starts_with("state_session_turn_band:")
    {
        return Some("generic_prompt_shape");
    }
    if atom.contains("_cwd_kind:") {
        return Some("cwd_identity_atom");
    }
    if atom.starts_with("request_command_arg_band:") {
        return Some("command_length_band");
    }
    if atom.starts_with("evidence:exit_code_")
        || atom.starts_with("exit_code_band:")
        || atom.starts_with("state_exit_code_band:")
        || atom.starts_with("state_tool_status_evidence:")
        || atom.starts_with("state_tool_status_exit_band:")
        || atom.starts_with("state_tool_status_exit_zero:")
        || atom.starts_with("state_tool_status_shell_exit:")
        || atom.starts_with("state_tool_status_command_exit:")
    {
        return Some("execution_outcome_atom");
    }
    if atom.starts_with("state_output_marker:")
        || atom.starts_with("state_output_has_")
        || atom.starts_with("state_output_contains_")
        || atom.starts_with("state_output_char_band:")
        || atom.starts_with("state_output_line_band:")
        || atom.starts_with("state_output_has_warning_marker:")
        || atom.starts_with("state_output_has_error_marker:")
    {
        return Some("output_status_marker");
    }
    if atom == "request_has_path:false"
        || atom == "state_followup_marker:false"
        || atom == "state_stop_marker:false"
    {
        return Some("negative_prompt_marker");
    }
    None
}

fn atom_count_rows(counts: &BTreeMap<String, usize>, limit: usize) -> Vec<Value> {
    let mut rows = counts
        .iter()
        .map(|(atom, count)| serde_json::json!({ "atom": atom, "count": count }))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let left_count = left.get("count").and_then(Value::as_u64).unwrap_or(0);
        let right_count = right.get("count").and_then(Value::as_u64).unwrap_or(0);
        right_count.cmp(&left_count).then_with(|| {
            left.get("atom")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .cmp(
                    right
                        .get("atom")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                )
        })
    });
    rows.truncate(limit);
    rows
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
            for value in map.values() {
                collect_atom_strings(Some(value), output);
            }
        }
        _ => {}
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
