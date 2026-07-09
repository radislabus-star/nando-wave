use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::{
    json_bool, json_string, phase_atom_binary_token_cost, phase_atom_string_vec, write_json_file,
};

const DEFAULT_VERIFIER_EVIDENCE_JOIN_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-verifier-evidence-join-v1.report.json";
const DEFAULT_VERIFIER_EVIDENCE_JOIN_JSONL: &str =
    "target/nando-wave/streaming/phase-stream-verifier-evidence-join-v1.jsonl";

#[derive(Clone, Default)]
struct EvidenceEntry {
    label: Option<bool>,
    conflict: bool,
    source_traces: BTreeSet<String>,
    action_families: BTreeSet<String>,
    request_route_families: BTreeSet<String>,
    result_atoms: BTreeSet<String>,
    state_atoms: BTreeSet<String>,
    route_hint_atoms: BTreeSet<String>,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Default)]
struct EvidenceIndex {
    by_request_fingerprint: BTreeMap<String, EvidenceEntry>,
    by_exact_cache_key: BTreeMap<String, EvidenceEntry>,
    evidence_rows: usize,
    evidence_rows_with_label: usize,
    evidence_rows_without_label: usize,
    evidence_conflict_keys: usize,
}

#[derive(Default, Serialize)]
struct VerifierEvidenceJoinReport {
    report_kind: &'static str,
    base_trace_path: String,
    evidence_paths: Vec<String>,
    output_jsonl_path: String,
    base_rows: usize,
    base_rows_with_verifier_label: usize,
    base_rows_missing_verifier_label: usize,
    evidence_rows: usize,
    evidence_rows_with_label: usize,
    evidence_rows_without_label: usize,
    joined_rows: usize,
    joined_by_request_fingerprint: usize,
    joined_by_exact_cache_key: usize,
    already_labeled_rows: usize,
    no_evidence_rows: usize,
    conflict_rows: usize,
    label_true_joined_rows: usize,
    label_false_joined_rows: usize,
    token_cost_joined_rows: usize,
    result_atom_joined_rows: usize,
    route_incompatible_evidence_rows: usize,
    output_rows: usize,
    output_rows_with_verifier_label: usize,
    output_rows_missing_verifier_label: usize,
    forbidden_flags: BTreeMap<&'static str, bool>,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    market_money_claim_allowed: bool,
    verdict: &'static str,
    boundary: &'static str,
}

pub(crate) fn run_phase_stream_verifier_evidence_join_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_VERIFIER_EVIDENCE_JOIN_REPORT));
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_VERIFIER_EVIDENCE_JOIN_JSONL));
    let Some(base_path) = args.next().map(PathBuf::from) else {
        return Err(
            "base phase-atom trace JSONL path is required after report-json and output-jsonl"
                .to_owned(),
        );
    };
    let evidence_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if evidence_paths.is_empty() {
        return Err("at least one verifier evidence JSONL path is required".to_owned());
    }

    let evidence = build_evidence_index(&evidence_paths)?;
    let mut report = VerifierEvidenceJoinReport {
        report_kind: "phase_stream_verifier_evidence_join_v1",
        base_trace_path: base_path.display().to_string(),
        evidence_paths: evidence_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        output_jsonl_path: output_path.display().to_string(),
        evidence_rows: evidence.evidence_rows,
        evidence_rows_with_label: evidence.evidence_rows_with_label,
        evidence_rows_without_label: evidence.evidence_rows_without_label,
        boundary: "cold evidence join only: copies verifier labels/evidence atoms from matching verifier traces by request_fingerprint or exact_cache_key; does not compile .nwpc, score, promote, serve, enable local_accept, claim market money, or use legacy nwrb/role-binding paths",
        ..VerifierEvidenceJoinReport::default()
    };
    join_base_trace(&base_path, &output_path, &evidence, &mut report)?;
    report.forbidden_flags = [
        ("nwrb_used", false),
        ("role_binding_backend_used", false),
        ("lookup_used", false),
        ("target_id_or_proof_rule_id_authority_used", false),
        ("concrete_x_lookup_used", false),
        ("manual_local_out_t_used", false),
        ("manual_class_list_used", false),
        ("manual_threshold_selection_used", false),
        ("local_accept_without_verifier_used", false),
    ]
    .into_iter()
    .collect();
    report.local_accept_enabled = false;
    report.auto_promote_enabled = false;
    report.market_money_claim_allowed = false;
    report.verdict = if report.conflict_rows > 0 {
        "PHASE_STREAM_VERIFIER_EVIDENCE_JOIN_V1_WATCH_CONFLICTS_SKIPPED"
    } else if report.joined_rows > 0 {
        "PHASE_STREAM_VERIFIER_EVIDENCE_JOIN_V1_PASS_JOINED_EVIDENCE"
    } else {
        "PHASE_STREAM_VERIFIER_EVIDENCE_JOIN_V1_WATCH_NO_MATCHING_EVIDENCE"
    };
    write_json_file(&report_path, &report)?;

    println!("phase_stream_verifier_evidence_join_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_jsonl_path: {}", output_path.display());
    println!("  base_rows: {}", report.base_rows);
    println!("  joined_rows: {}", report.joined_rows);
    println!(
        "  output_rows_with_verifier_label: {}",
        report.output_rows_with_verifier_label
    );
    println!("  conflict_rows: {}", report.conflict_rows);
    println!("  local_accept_enabled: false");
    println!("  verdict: {}", report.verdict);
    Ok(())
}

fn build_evidence_index(paths: &[PathBuf]) -> Result<EvidenceIndex, String> {
    let mut index = EvidenceIndex::default();
    for path in paths {
        let text = std::fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read verifier evidence input '{}': {error}",
                path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            index.evidence_rows += 1;
            let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse verifier evidence input '{}' line {}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
            let Some(label) = json_bool(&row, &["verified_safe_accept"]) else {
                index.evidence_rows_without_label += 1;
                continue;
            };
            index.evidence_rows_with_label += 1;
            let entry = evidence_entry_from_row(&row, label, path, line_index);
            if let Some(request_fingerprint) = json_string(&row, &["request_fingerprint"]) {
                merge_evidence_entry(
                    index
                        .by_request_fingerprint
                        .entry(request_fingerprint)
                        .or_default(),
                    &entry,
                );
            }
            if let Some(exact_cache_key) = json_string(&row, &["exact_cache_key"]) {
                merge_evidence_entry(
                    index.by_exact_cache_key.entry(exact_cache_key).or_default(),
                    &entry,
                );
            }
        }
    }
    index.evidence_conflict_keys = index
        .by_request_fingerprint
        .values()
        .chain(index.by_exact_cache_key.values())
        .filter(|entry| entry.conflict)
        .count();
    Ok(index)
}

fn evidence_entry_from_row(
    row: &Value,
    label: bool,
    path: &Path,
    line_index: usize,
) -> EvidenceEntry {
    let token_cost = phase_atom_binary_token_cost(row);
    let mut entry = EvidenceEntry {
        label: Some(label),
        total_tokens: token_cost.total_tokens,
        total_cost_microusd: token_cost.total_cost_microusd,
        ..EvidenceEntry::default()
    };
    let trace_id = json_string(row, &["trace_id"])
        .or_else(|| json_string(row, &["request_fingerprint"]))
        .unwrap_or_else(|| format!("{}:{}", path.display(), line_index + 1));
    entry.source_traces.insert(trace_id);
    for atom in phase_atom_string_vec(row, "result_atoms") {
        if verifier_join_atom_allowed(&atom) {
            entry.result_atoms.insert(atom);
        }
    }
    for atom in phase_atom_string_vec(row, "state_atoms") {
        if verifier_join_atom_allowed(&atom) && verifier_join_state_atom_allowed(&atom) {
            entry.state_atoms.insert(atom);
        }
    }
    for atom in phase_atom_string_vec(row, "route_hint_atoms") {
        if verifier_join_atom_allowed(&atom) {
            entry.route_hint_atoms.insert(atom);
        }
    }
    entry.action_families = action_family_set(row);
    entry.request_route_families = request_route_family_set(row);
    entry
}

fn merge_evidence_entry(target: &mut EvidenceEntry, source: &EvidenceEntry) {
    if let Some(label) = source.label {
        match target.label {
            Some(existing) if existing != label => target.conflict = true,
            Some(_) => {}
            None => target.label = Some(label),
        }
    }
    target.conflict |= source.conflict;
    target
        .source_traces
        .extend(source.source_traces.iter().cloned());
    target
        .action_families
        .extend(source.action_families.iter().cloned());
    target
        .request_route_families
        .extend(source.request_route_families.iter().cloned());
    target
        .result_atoms
        .extend(source.result_atoms.iter().cloned());
    target
        .state_atoms
        .extend(source.state_atoms.iter().cloned());
    target
        .route_hint_atoms
        .extend(source.route_hint_atoms.iter().cloned());
    target.total_tokens = target.total_tokens.max(source.total_tokens);
    target.total_cost_microusd = target.total_cost_microusd.max(source.total_cost_microusd);
}

fn join_base_trace(
    base_path: &Path,
    output_path: &Path,
    evidence: &EvidenceIndex,
    report: &mut VerifierEvidenceJoinReport,
) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let input = std::fs::read_to_string(base_path).map_err(|error| {
        format!(
            "failed to read base trace '{}': {error}",
            base_path.display()
        )
    })?;
    let mut writer = std::io::BufWriter::new(
        std::fs::File::create(output_path)
            .map_err(|error| format!("failed to create '{}': {error}", output_path.display()))?,
    );
    for (line_index, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        report.base_rows += 1;
        let mut row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse base trace '{}' line {}: {error}",
                base_path.display(),
                line_index + 1
            )
        })?;
        if json_bool(&row, &["verified_safe_accept"]).is_some() {
            report.base_rows_with_verifier_label += 1;
            report.already_labeled_rows += 1;
        } else {
            report.base_rows_missing_verifier_label += 1;
            apply_join_for_missing_label(&mut row, evidence, report);
        }
        report.output_rows += 1;
        if json_bool(&row, &["verified_safe_accept"]).is_some() {
            report.output_rows_with_verifier_label += 1;
        } else {
            report.output_rows_missing_verifier_label += 1;
        }
        let text = serde_json::to_string(&row)
            .map_err(|error| format!("failed to serialize joined trace row: {error}"))?;
        writeln!(writer, "{text}")
            .map_err(|error| format!("failed to write '{}': {error}", output_path.display()))?;
    }
    writer
        .flush()
        .map_err(|error| format!("failed to flush '{}': {error}", output_path.display()))
}

fn apply_join_for_missing_label(
    row: &mut Value,
    evidence: &EvidenceIndex,
    report: &mut VerifierEvidenceJoinReport,
) {
    let request_fingerprint = json_string(row, &["request_fingerprint"]);
    let exact_cache_key = json_string(row, &["exact_cache_key"]);
    let by_fingerprint = request_fingerprint
        .as_ref()
        .and_then(|key| evidence.by_request_fingerprint.get(key));
    let by_exact_cache = exact_cache_key
        .as_ref()
        .and_then(|key| evidence.by_exact_cache_key.get(key));
    let (join_kind, entry) = match (by_fingerprint, by_exact_cache) {
        (Some(left), Some(right))
            if left.conflict || right.conflict || left.label != right.label =>
        {
            report.conflict_rows += 1;
            return;
        }
        (Some(entry), _) if entry.conflict => {
            report.conflict_rows += 1;
            return;
        }
        (_, Some(entry)) if entry.conflict => {
            report.conflict_rows += 1;
            return;
        }
        (Some(entry), _) => ("request_fingerprint", entry),
        (None, Some(entry)) => ("exact_cache_key", entry),
        (None, None) => {
            report.no_evidence_rows += 1;
            return;
        }
    };
    let Some(label) = entry.label else {
        report.no_evidence_rows += 1;
        return;
    };
    if !evidence_route_compatible(row, entry) {
        report.route_incompatible_evidence_rows += 1;
        return;
    }
    report.joined_rows += 1;
    report.joined_by_request_fingerprint += usize::from(join_kind == "request_fingerprint");
    report.joined_by_exact_cache_key += usize::from(join_kind == "exact_cache_key");
    report.label_true_joined_rows += usize::from(label);
    report.label_false_joined_rows += usize::from(!label);
    set_field(row, "verified_safe_accept", Value::Bool(label));
    set_field(row, "missing_verifier_label", Value::Bool(false));
    set_field(
        row,
        "verification_source_kind",
        Value::String("joined_verifier_evidence".to_owned()),
    );
    merge_array_field(
        row,
        "state_atoms",
        ["state_has_verifier_label:true".to_owned()],
    );
    remove_array_value(row, "state_atoms", "state_has_verifier_label:false");
    let result_count_before = phase_atom_string_vec(row, "result_atoms").len();
    merge_array_field(row, "result_atoms", entry.result_atoms.iter().cloned());
    merge_array_field(row, "state_atoms", entry.state_atoms.iter().cloned());
    merge_array_field(
        row,
        "route_hint_atoms",
        entry.route_hint_atoms.iter().cloned(),
    );
    if phase_atom_string_vec(row, "result_atoms").len() > result_count_before {
        report.result_atom_joined_rows += 1;
    }
    maybe_join_token_cost(row, entry, report);
    set_field(
        row,
        "verifier_evidence_join",
        serde_json::json!({
            "joined": true,
            "join_key": join_kind,
            "source_trace_count": entry.source_traces.len(),
            "copied_output_hash64": false,
            "copied_raw_text": false
        }),
    );
}

fn maybe_join_token_cost(
    row: &mut Value,
    entry: &EvidenceEntry,
    report: &mut VerifierEvidenceJoinReport,
) {
    let current = phase_atom_binary_token_cost(row);
    if current.total_tokens > 0 || entry.total_tokens == 0 {
        return;
    }
    let Some(object) = row.as_object_mut() else {
        return;
    };
    object.insert(
        "token_cost".to_owned(),
        serde_json::json!({
            "total_tokens": entry.total_tokens,
            "total_cost_microusd": entry.total_cost_microusd,
            "token_evidence_missing": false,
            "cost_evidence_missing": entry.total_cost_microusd == 0,
            "token_cost_joined_from_verifier_evidence": true
        }),
    );
    report.token_cost_joined_rows += 1;
}

fn merge_array_field<I>(row: &mut Value, key: &'static str, atoms: I)
where
    I: IntoIterator<Item = String>,
{
    let mut merged = phase_atom_string_vec(row, key)
        .into_iter()
        .collect::<BTreeSet<_>>();
    merged.extend(atoms);
    if let Some(object) = row.as_object_mut() {
        object.insert(
            key.to_owned(),
            Value::Array(merged.into_iter().map(Value::String).collect()),
        );
    }
}

fn remove_array_value(row: &mut Value, key: &'static str, value: &str) {
    let merged = phase_atom_string_vec(row, key)
        .into_iter()
        .filter(|atom| atom != value)
        .collect::<BTreeSet<_>>();
    if let Some(object) = row.as_object_mut() {
        object.insert(
            key.to_owned(),
            Value::Array(merged.into_iter().map(Value::String).collect()),
        );
    }
}

fn set_field(row: &mut Value, key: &'static str, value: Value) {
    if let Some(object) = row.as_object_mut() {
        object.insert(key.to_owned(), value);
    }
}

fn verifier_join_atom_allowed(atom: &str) -> bool {
    if atom.is_empty() {
        return false;
    }
    let lower = atom.to_ascii_lowercase();
    !atom.starts_with("output_hash64:")
        && !atom.starts_with("request_fingerprint:")
        && !atom.starts_with("exact_cache_key:")
        && !atom.starts_with("trace_id:")
        && !atom.starts_with("source_trace_id:")
        && !lower.contains("target_id")
        && !lower.contains("proof_rule")
        && !lower.contains("local_out_t")
        && !lower.contains("concrete_x")
        && !lower.contains("nwrb")
        && !lower.contains("role_binding")
}

fn verifier_join_state_atom_allowed(atom: &str) -> bool {
    atom.starts_with("state_source:")
        || atom.starts_with("state_cwd_kind:")
        || atom.starts_with("state_exit_code_band:")
        || atom.starts_with("state_output_")
        || atom.starts_with("state_tool_status_")
        || atom.starts_with("state_plan_")
}

fn evidence_route_compatible(row: &Value, entry: &EvidenceEntry) -> bool {
    let base_actions = action_family_set(row);
    if !base_actions.is_empty()
        && base_actions
            .iter()
            .any(|action| entry.action_families.contains(action))
    {
        return true;
    }
    let base_routes = request_route_family_set(row);
    !base_routes.is_empty()
        && base_routes
            .iter()
            .any(|route| entry.request_route_families.contains(route))
}

fn action_family_set(row: &Value) -> BTreeSet<String> {
    phase_atom_string_vec(row, "action_atoms")
        .into_iter()
        .filter(|atom| atom.starts_with("action_family:"))
        .collect()
}

fn request_route_family_set(row: &Value) -> BTreeSet<String> {
    phase_atom_string_vec(row, "request_atoms")
        .into_iter()
        .filter(|atom| atom.starts_with("request_route_family:"))
        .collect()
}
