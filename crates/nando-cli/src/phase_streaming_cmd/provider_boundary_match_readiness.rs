use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

const DEFAULT_PROVIDER_BOUNDARY_MATCH_READINESS_REPORT: &str =
    "target/nando-wave/streaming/provider-boundary-match-readiness-v1.report.json";

#[derive(Default)]
struct ProviderIndex {
    rows: usize,
    rows_with_join_keys: usize,
    rows_with_provider_keys: usize,
    join_key_to_provider_keys: BTreeMap<String, BTreeSet<String>>,
    provider_key_atom_leak_rows: usize,
}

#[derive(Default)]
struct PhaseTotals {
    rows: usize,
    score_ready_rows: usize,
    route_ready_rows: usize,
    verifier_label_rows: usize,
    shadow_request_rows: usize,
    rows_with_join_keys: usize,
    rows_with_existing_provider_keys: usize,
    rows_joined_with_provider_keys: usize,
    score_ready_rows_with_provider_join: usize,
    score_ready_rows_missing_provider_join: usize,
    economic_score_ready_rows: usize,
    economic_score_ready_rows_with_provider_join: usize,
    economic_score_ready_rows_missing_provider_join: usize,
    zero_denominator_score_ready_rows: usize,
    zero_denominator_score_ready_rows_missing_provider_join: usize,
    rows_with_positive_tokens: usize,
    total_tokens: usize,
    provider_key_atom_leak_rows: usize,
}

#[derive(Serialize)]
struct PhaseFileReport {
    path: String,
    rows: usize,
    score_ready_rows: usize,
    route_ready_rows: usize,
    verifier_label_rows: usize,
    shadow_request_rows: usize,
    rows_with_join_keys: usize,
    rows_with_existing_provider_keys: usize,
    rows_joined_with_provider_keys: usize,
    score_ready_rows_with_provider_join: usize,
    score_ready_rows_missing_provider_join: usize,
    economic_score_ready_rows: usize,
    economic_score_ready_rows_with_provider_join: usize,
    economic_score_ready_rows_missing_provider_join: usize,
    zero_denominator_score_ready_rows: usize,
    zero_denominator_score_ready_rows_missing_provider_join: usize,
    rows_with_positive_tokens: usize,
    total_tokens: usize,
    provider_key_atom_leak_rows: usize,
}

#[derive(Serialize)]
struct ProviderFileReport {
    path: String,
    rows: usize,
    rows_with_join_keys: usize,
    rows_with_provider_keys: usize,
    provider_key_atom_leak_rows: usize,
}

pub(crate) fn run_phase_stream_provider_boundary_match_readiness_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDER_BOUNDARY_MATCH_READINESS_REPORT));

    let mut phase_paths = Vec::<PathBuf>::new();
    let mut provider_paths = Vec::<PathBuf>::new();
    let mut seen_provider_separator = false;
    for arg in args {
        if arg == "--provider" {
            seen_provider_separator = true;
            continue;
        }
        if seen_provider_separator {
            provider_paths.push(PathBuf::from(arg));
        } else {
            phase_paths.push(PathBuf::from(arg));
        }
    }
    if phase_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }
    if provider_paths.is_empty() {
        return Err(
            "at least one provider-boundary JSONL path is required after --provider".to_owned(),
        );
    }

    let mut provider_index = ProviderIndex::default();
    let mut provider_reports = Vec::<ProviderFileReport>::new();
    for path in &provider_paths {
        let file = scan_provider_file(path, &mut provider_index)?;
        provider_reports.push(file);
    }

    let mut phase_totals = PhaseTotals::default();
    let mut phase_reports = Vec::<PhaseFileReport>::new();
    for path in &phase_paths {
        let file = scan_phase_file(path, &provider_index, &mut phase_totals)?;
        phase_reports.push(file);
    }

    let provider_join_key_count = provider_index.join_key_to_provider_keys.len();
    let provider_correlation_metadata_only = provider_index.provider_key_atom_leak_rows == 0
        && phase_totals.provider_key_atom_leak_rows == 0;
    let economic_join_ready = phase_totals.economic_score_ready_rows > 0
        && phase_totals.economic_score_ready_rows_missing_provider_join == 0
        && phase_totals.economic_score_ready_rows_with_provider_join
            == phase_totals.economic_score_ready_rows
        && phase_totals.rows_with_positive_tokens > 0
        && provider_correlation_metadata_only;
    let full_score_ready_provider_coverage = phase_totals.score_ready_rows > 0
        && phase_totals.score_ready_rows_missing_provider_join == 0
        && phase_totals.score_ready_rows_with_provider_join == phase_totals.score_ready_rows;
    let zero_denominator_explains_missing_score_ready =
        phase_totals.score_ready_rows_missing_provider_join > 0
            && phase_totals.score_ready_rows_missing_provider_join
                == phase_totals.zero_denominator_score_ready_rows_missing_provider_join;

    let mut blockers = Vec::<&'static str>::new();
    if phase_totals.score_ready_rows == 0 {
        blockers.push("no_score_ready_phase_rows");
    }
    if provider_join_key_count == 0 {
        blockers.push("no_provider_join_keys");
    }
    if provider_index.rows_with_provider_keys == 0 {
        blockers.push("no_provider_correlation_keys");
    }
    if phase_totals.economic_score_ready_rows_with_provider_join == 0 {
        blockers.push("no_score_ready_rows_matched_to_provider_keys");
    }
    if phase_totals.economic_score_ready_rows_missing_provider_join > 0 {
        blockers.push("some_score_ready_rows_missing_provider_join");
    }
    if phase_totals.rows_with_positive_tokens == 0 {
        blockers.push("no_positive_token_denominator");
    }
    if !provider_correlation_metadata_only {
        blockers.push("provider_key_atom_leak");
    }

    let verdict = if !provider_correlation_metadata_only {
        "PHASE_STREAM_PROVIDER_BOUNDARY_MATCH_READINESS_V1_FAIL_PROVIDER_KEY_ATOM_LEAK"
    } else if economic_join_ready && full_score_ready_provider_coverage {
        "PHASE_STREAM_PROVIDER_BOUNDARY_MATCH_READINESS_V1_PASS_FULL_ECONOMIC_JOIN_READY"
    } else if economic_join_ready && zero_denominator_explains_missing_score_ready {
        "PHASE_STREAM_PROVIDER_BOUNDARY_MATCH_READINESS_V1_PASS_ECONOMIC_JOIN_READY_WITH_ZERO_DENOMINATOR_EXCLUSIONS"
    } else if economic_join_ready {
        "PHASE_STREAM_PROVIDER_BOUNDARY_MATCH_READINESS_V1_WATCH_PARTIAL_ECONOMIC_JOIN_READY"
    } else if phase_totals.score_ready_rows > 0 && provider_join_key_count > 0 {
        "PHASE_STREAM_PROVIDER_BOUNDARY_MATCH_READINESS_V1_WATCH_SCORE_READY_WITHOUT_PROVIDER_MATCH"
    } else {
        "PHASE_STREAM_PROVIDER_BOUNDARY_MATCH_READINESS_V1_WATCH_NOT_JOIN_READY"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_provider_boundary_match_readiness_v1",
        "phase_trace_paths": phase_paths.iter().map(path_string).collect::<Vec<_>>(),
        "provider_boundary_paths": provider_paths.iter().map(path_string).collect::<Vec<_>>(),
        "provider": {
            "rows": provider_index.rows,
            "rows_with_join_keys": provider_index.rows_with_join_keys,
            "rows_with_provider_keys": provider_index.rows_with_provider_keys,
            "join_key_count": provider_join_key_count,
            "provider_key_atom_leak_rows": provider_index.provider_key_atom_leak_rows,
            "files": provider_reports
        },
        "phase": {
            "rows": phase_totals.rows,
            "score_ready_rows": phase_totals.score_ready_rows,
            "route_ready_rows": phase_totals.route_ready_rows,
            "verifier_label_rows": phase_totals.verifier_label_rows,
            "shadow_request_rows": phase_totals.shadow_request_rows,
            "rows_with_join_keys": phase_totals.rows_with_join_keys,
            "rows_with_existing_provider_keys": phase_totals.rows_with_existing_provider_keys,
            "rows_joined_with_provider_keys": phase_totals.rows_joined_with_provider_keys,
            "score_ready_rows_with_provider_join": phase_totals.score_ready_rows_with_provider_join,
            "score_ready_rows_missing_provider_join": phase_totals.score_ready_rows_missing_provider_join,
            "economic_score_ready_rows": phase_totals.economic_score_ready_rows,
            "economic_score_ready_rows_with_provider_join": phase_totals.economic_score_ready_rows_with_provider_join,
            "economic_score_ready_rows_missing_provider_join": phase_totals.economic_score_ready_rows_missing_provider_join,
            "zero_denominator_score_ready_rows": phase_totals.zero_denominator_score_ready_rows,
            "zero_denominator_score_ready_rows_missing_provider_join": phase_totals.zero_denominator_score_ready_rows_missing_provider_join,
            "rows_with_positive_tokens": phase_totals.rows_with_positive_tokens,
            "total_tokens": phase_totals.total_tokens,
            "provider_key_atom_leak_rows": phase_totals.provider_key_atom_leak_rows,
            "files": phase_reports
        },
        "readiness": {
            "provider_correlation_metadata_only": provider_correlation_metadata_only,
            "economic_join_ready": economic_join_ready,
            "full_score_ready_provider_coverage": full_score_ready_provider_coverage,
            "zero_denominator_explains_missing_score_ready": zero_denominator_explains_missing_score_ready,
            "policy": "audit only: measures whether existing score-ready phase traces can be joined to provider-boundary metadata before NP chain; does not mine, compile, score, promote, serve, local-accept, or claim money"
        },
        "blockers": blockers,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": false,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": verdict,
        "boundary": "readiness audit only: no trace mutation, no mining, no runtime replay, no promotion, no local accept, no market money claim"
    });
    super::write_json_file(&report_path, &report)?;

    println!("phase_stream_provider_boundary_match_readiness_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  phase_score_ready_rows: {}",
        phase_totals.score_ready_rows
    );
    println!(
        "  score_ready_rows_with_provider_join: {}",
        phase_totals.score_ready_rows_with_provider_join
    );
    println!(
        "  score_ready_rows_missing_provider_join: {}",
        phase_totals.score_ready_rows_missing_provider_join
    );
    println!("  provider_join_key_count: {provider_join_key_count}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn scan_provider_file(
    path: &Path,
    index: &mut ProviderIndex,
) -> Result<ProviderFileReport, String> {
    let text = read_text(path)?;
    let mut file = ProviderFileReport {
        path: path_string(path),
        rows: 0,
        rows_with_join_keys: 0,
        rows_with_provider_keys: 0,
        provider_key_atom_leak_rows: 0,
    };
    for (line_index, line) in text.lines().enumerate() {
        let Some(row) = parse_jsonl_row(path, line_index + 1, line)? else {
            continue;
        };
        file.rows += 1;
        index.rows += 1;
        let join_keys = metadata_join_keys(&row);
        let provider_keys = super::phase_atom_external_provider_correlation_keys(&row);
        file.rows_with_join_keys += usize::from(!join_keys.is_empty());
        file.rows_with_provider_keys += usize::from(!provider_keys.is_empty());
        index.rows_with_join_keys += usize::from(!join_keys.is_empty());
        index.rows_with_provider_keys += usize::from(!provider_keys.is_empty());
        if provider_key_leaks_into_atoms(&row) {
            file.provider_key_atom_leak_rows += 1;
            index.provider_key_atom_leak_rows += 1;
        }
        if join_keys.is_empty() || provider_keys.is_empty() {
            continue;
        }
        let provider_keys = provider_keys.into_iter().collect::<BTreeSet<_>>();
        for join_key in join_keys {
            index
                .join_key_to_provider_keys
                .entry(join_key)
                .or_default()
                .extend(provider_keys.iter().cloned());
        }
    }
    Ok(file)
}

fn scan_phase_file(
    path: &Path,
    provider_index: &ProviderIndex,
    totals: &mut PhaseTotals,
) -> Result<PhaseFileReport, String> {
    let text = read_text(path)?;
    let mut file = PhaseFileReport {
        path: path_string(path),
        rows: 0,
        score_ready_rows: 0,
        route_ready_rows: 0,
        verifier_label_rows: 0,
        shadow_request_rows: 0,
        rows_with_join_keys: 0,
        rows_with_existing_provider_keys: 0,
        rows_joined_with_provider_keys: 0,
        score_ready_rows_with_provider_join: 0,
        score_ready_rows_missing_provider_join: 0,
        economic_score_ready_rows: 0,
        economic_score_ready_rows_with_provider_join: 0,
        economic_score_ready_rows_missing_provider_join: 0,
        zero_denominator_score_ready_rows: 0,
        zero_denominator_score_ready_rows_missing_provider_join: 0,
        rows_with_positive_tokens: 0,
        total_tokens: 0,
        provider_key_atom_leak_rows: 0,
    };
    for (line_index, line) in text.lines().enumerate() {
        let Some(row) = parse_jsonl_row(path, line_index + 1, line)? else {
            continue;
        };
        file.rows += 1;
        totals.rows += 1;

        let score_ready = json_bool(&row, &["ready_for_existing_shadow_scoring"]).unwrap_or(false);
        let route_ready = json_bool(&row, &["ready_for_route_family_mining"]).unwrap_or(false);
        let verifier_label = row
            .get("verified_safe_accept")
            .and_then(Value::as_bool)
            .is_some();
        let shadow_request = json_bool(&row, &["has_shadow_request"]).unwrap_or(false)
            || row.get("nando_shadow_request").is_some();
        file.score_ready_rows += usize::from(score_ready);
        file.route_ready_rows += usize::from(route_ready);
        file.verifier_label_rows += usize::from(verifier_label);
        file.shadow_request_rows += usize::from(shadow_request);
        totals.score_ready_rows += usize::from(score_ready);
        totals.route_ready_rows += usize::from(route_ready);
        totals.verifier_label_rows += usize::from(verifier_label);
        totals.shadow_request_rows += usize::from(shadow_request);

        let total_tokens = json_usize(&row, &["token_cost", "total_tokens"])
            .or_else(|| json_usize(&row, &["estimated_total_tokens"]))
            .unwrap_or(0);
        let economic_score_ready = score_ready && total_tokens > 0;
        let zero_denominator_score_ready = score_ready && total_tokens == 0;

        let join_keys = metadata_join_keys(&row);
        let mut provider_keys = super::phase_atom_external_provider_correlation_keys(&row)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let before_join_len = provider_keys.len();
        for join_key in &join_keys {
            if let Some(keys) = provider_index.join_key_to_provider_keys.get(join_key) {
                provider_keys.extend(keys.iter().cloned());
            }
        }
        let joined_with_provider = !provider_keys.is_empty();
        file.rows_with_join_keys += usize::from(!join_keys.is_empty());
        file.rows_with_existing_provider_keys += usize::from(before_join_len > 0);
        file.rows_joined_with_provider_keys += usize::from(joined_with_provider);
        totals.rows_with_join_keys += usize::from(!join_keys.is_empty());
        totals.rows_with_existing_provider_keys += usize::from(before_join_len > 0);
        totals.rows_joined_with_provider_keys += usize::from(joined_with_provider);
        if score_ready && joined_with_provider {
            file.score_ready_rows_with_provider_join += 1;
            totals.score_ready_rows_with_provider_join += 1;
        }
        if score_ready && !joined_with_provider {
            file.score_ready_rows_missing_provider_join += 1;
            totals.score_ready_rows_missing_provider_join += 1;
        }
        if economic_score_ready {
            file.economic_score_ready_rows += 1;
            totals.economic_score_ready_rows += 1;
            if joined_with_provider {
                file.economic_score_ready_rows_with_provider_join += 1;
                totals.economic_score_ready_rows_with_provider_join += 1;
            } else {
                file.economic_score_ready_rows_missing_provider_join += 1;
                totals.economic_score_ready_rows_missing_provider_join += 1;
            }
        }
        if zero_denominator_score_ready {
            file.zero_denominator_score_ready_rows += 1;
            totals.zero_denominator_score_ready_rows += 1;
            if !joined_with_provider {
                file.zero_denominator_score_ready_rows_missing_provider_join += 1;
                totals.zero_denominator_score_ready_rows_missing_provider_join += 1;
            }
        }
        file.rows_with_positive_tokens += usize::from(total_tokens > 0);
        file.total_tokens = file.total_tokens.saturating_add(total_tokens);
        totals.rows_with_positive_tokens += usize::from(total_tokens > 0);
        totals.total_tokens = totals.total_tokens.saturating_add(total_tokens);

        if provider_key_leaks_into_atoms(&row) {
            file.provider_key_atom_leak_rows += 1;
            totals.provider_key_atom_leak_rows += 1;
        }
    }
    Ok(file)
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read JSONL '{}': {error}", path.display()))
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

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)?.as_str().map(ToOwned::to_owned)
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path)?.as_bool()
}

fn json_usize(value: &Value, path: &[&str]) -> Option<usize> {
    let value = json_at(value, path)?;
    value
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .or_else(|| {
            value
                .as_i64()
                .and_then(|number| usize::try_from(number).ok())
        })
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}
