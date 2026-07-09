use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    json_bool, json_string, json_u64, read_json_value, stable_fingerprint, write_json_file,
};

const DEFAULT_TARGETED_SPLIT_REFINEMENT_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-split-refinement-v1.report.json";
const DEFAULT_TARGETED_SPLIT_REFINEMENT_CANDIDATES: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-split-refinement-v1.candidates.jsonl";
const DEFAULT_TARGETED_REJECTION_DRILLDOWN_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-rejection-drilldown-v1-agent-followup-12k-current.report.json";
const DEFAULT_TARGETED_SHADOW_REPORT: &str = "target/nando-wave/streaming/phase-stream-online-miner-targeted-shadow-v1-agent-followup-12k-current.report.json";
const TOP_SPLIT_CANDIDATES_PER_BUCKET: usize = 12;
const PAIR_BASIS_ATOMS_PER_BUCKET: usize = 24;

#[derive(Clone, Debug, Default)]
struct BucketRefinementRows {
    rows: Vec<RefinementDecisionRow>,
    raw_unique_accepts: usize,
    raw_false_accepts: usize,
}

#[derive(Clone, Debug)]
struct RefinementDecisionRow {
    safe_unique: bool,
    false_accept: bool,
    tokens: usize,
    atoms: Vec<String>,
    trace_row: Value,
}

#[derive(Clone, Debug, Default)]
struct SplitAtomStats {
    split_atom: String,
    split_group: &'static str,
    safe_unique_hits: usize,
    safe_unique_tokens: usize,
    false_hits: usize,
    total_hits: usize,
}

impl SplitAtomStats {
    fn score(&self) -> u128 {
        (self.safe_unique_tokens as u128)
            .saturating_mul(1_000_000)
            .saturating_add((self.safe_unique_hits as u128).saturating_mul(1_000))
            .saturating_sub((self.false_hits as u128).saturating_mul(10_000_000))
    }

    fn clean(&self) -> bool {
        self.safe_unique_hits > 0 && self.false_hits == 0
    }

    fn stageable(&self) -> bool {
        self.safe_unique_hits >= 2 && self.false_hits == 0
    }
}

pub(crate) fn run_phase_stream_online_miner_targeted_split_refinement_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_SPLIT_REFINEMENT_REPORT));
    let candidate_jsonl_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_SPLIT_REFINEMENT_CANDIDATES));
    let drilldown_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_REJECTION_DRILLDOWN_REPORT));
    let targeted_shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_TARGETED_SHADOW_REPORT));
    let selected_split_report_path = targeted_split_refinement_selected_report_path(&report_path);
    let selected_split_trace_path = targeted_split_refinement_selected_trace_path(&report_path);
    let explicit_trace_paths = args.map(PathBuf::from).collect::<Vec<_>>();

    let drilldown = read_json_value(&drilldown_report_path)?;
    let targeted = read_json_value(&targeted_shadow_report_path)?;
    let decision_log_path = json_string(&targeted, &["decision_log_path"])
        .map(PathBuf::from)
        .ok_or_else(|| "targeted shadow report missing decision_log_path".to_owned())?;
    let trace_paths = if explicit_trace_paths.is_empty() {
        targeted
            .get("trace_paths")
            .and_then(Value::as_array)
            .ok_or_else(|| "targeted shadow report missing trace_paths".to_owned())?
            .iter()
            .filter_map(Value::as_str)
            .map(PathBuf::from)
            .collect::<Vec<_>>()
    } else {
        explicit_trace_paths
    };
    if trace_paths.is_empty() {
        return Err("targeted split refinement needs at least one trace path".to_owned());
    }

    let risk_buckets = targeted_refinement_risk_buckets(&drilldown);
    let trace_rows = read_trace_rows(&trace_paths)?;
    let bucket_rows =
        targeted_refinement_bucket_rows(&decision_log_path, &trace_rows, &risk_buckets)?;
    let mut bucket_reports = Vec::new();
    let mut candidate_lines = Vec::new();
    let mut clean_split_candidate_count = 0usize;
    let mut stageable_split_candidate_count = 0usize;
    let mut raw_unique_accepts_in_risk_buckets = 0usize;
    let mut raw_false_accepts_in_risk_buckets = 0usize;
    let mut risk_buckets_with_stageable_split = 0usize;
    let mut selected_children_by_class = BTreeMap::<String, Vec<Value>>::new();
    let mut selected_trace_lines = Vec::<String>::new();

    for (bucket_key, bucket) in &bucket_rows {
        raw_unique_accepts_in_risk_buckets =
            raw_unique_accepts_in_risk_buckets.saturating_add(bucket.raw_unique_accepts);
        raw_false_accepts_in_risk_buckets =
            raw_false_accepts_in_risk_buckets.saturating_add(bucket.raw_false_accepts);
        let candidates = targeted_refinement_candidates(bucket);
        clean_split_candidate_count = clean_split_candidate_count.saturating_add(
            candidates
                .iter()
                .filter(|candidate| candidate.clean())
                .count(),
        );
        stageable_split_candidate_count = stageable_split_candidate_count.saturating_add(
            candidates
                .iter()
                .filter(|candidate| candidate.stageable())
                .count(),
        );
        if candidates.iter().any(SplitAtomStats::stageable) {
            risk_buckets_with_stageable_split += 1;
        }
        let mut candidate_reports = Vec::new();
        let mut selected_stageable_split_emitted = false;
        for (rank_index, candidate) in candidates
            .iter()
            .filter(|candidate| candidate.clean())
            .take(TOP_SPLIT_CANDIDATES_PER_BUCKET)
            .enumerate()
        {
            let candidate_id = format!(
                "split_refinement-{:016x}",
                stable_fingerprint([bucket_key.as_str(), candidate.split_atom.as_str()])
            );
            let row = serde_json::json!({
                "candidate_id": candidate_id,
                "bucket_key": bucket_key,
                "split_atom": candidate.split_atom,
                "split_group": candidate.split_group,
                "rank": rank_index + 1,
                "safe_unique_hits": candidate.safe_unique_hits,
                "safe_unique_tokens": candidate.safe_unique_tokens,
                "false_hits": candidate.false_hits,
                "total_hits": candidate.total_hits,
                "stageable_clean_split_candidate": candidate.stageable(),
                "generated_route_hint": format!("route_hint:auto_split_refinement:{candidate_id}")
            });
            candidate_lines.push(serde_json::to_string(&row).map_err(|error| {
                format!("failed to serialize targeted split refinement candidate: {error}")
            })?);
            if candidate.stageable() && !selected_stageable_split_emitted {
                if let Some(split_rule) = targeted_refinement_selected_split_rule(candidate) {
                    selected_stageable_split_emitted = true;
                    let broad_class_id =
                        targeted_refinement_generated_broad_class_id(&candidate_id);
                    for decision_row in &bucket.rows {
                        selected_trace_lines.push(
                            serde_json::to_string(&targeted_refinement_selected_trace_row(
                                decision_row,
                                &broad_class_id,
                                &candidate_id,
                            ))
                            .map_err(|error| {
                                format!(
                                    "failed to serialize targeted split selected trace row: {error}"
                                )
                            })?,
                        );
                    }
                    selected_children_by_class
                        .entry(broad_class_id)
                        .or_default()
                        .push(serde_json::json!({
                            "split_rule": split_rule,
                            "source_bucket_key": bucket_key,
                            "source_candidate_id": candidate_id,
                            "source_split_atom": candidate.split_atom,
                            "source_split_group": candidate.split_group,
                            "source_safe_unique_hits": candidate.safe_unique_hits,
                            "source_safe_unique_tokens": candidate.safe_unique_tokens,
                            "source_false_hits": candidate.false_hits,
                            "selection_policy": "top_stageable_source_neutral_split_per_risk_bucket_v1"
                        }));
                }
            }
            candidate_reports.push(row);
        }
        bucket_reports.push(serde_json::json!({
            "bucket_key": bucket_key,
            "decision_rows": bucket.rows.len(),
            "raw_unique_accepts": bucket.raw_unique_accepts,
            "raw_false_accepts": bucket.raw_false_accepts,
            "clean_split_candidates": candidates.iter().filter(|candidate| candidate.clean()).count(),
            "stageable_split_candidates": candidates.iter().filter(|candidate| candidate.stageable()).count(),
            "top_candidates": candidate_reports,
            "next_action": if candidates.iter().any(SplitAtomStats::stageable) {
                "build generated candidate trace for stageable source-neutral split atoms"
            } else {
                "needs richer observable atoms or verifier evidence before split promotion"
            }
        }));
    }

    if let Some(parent) = candidate_jsonl_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create targeted split refinement candidate dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(
        &candidate_jsonl_path,
        if candidate_lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", candidate_lines.join("\n"))
        },
    )
    .map_err(|error| {
        format!(
            "failed to write targeted split refinement candidates '{}': {error}",
            candidate_jsonl_path.display()
        )
    })?;
    std::fs::write(
        &selected_split_trace_path,
        if selected_trace_lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", selected_trace_lines.join("\n"))
        },
    )
    .map_err(|error| {
        format!(
            "failed to write targeted split selected trace '{}': {error}",
            selected_split_trace_path.display()
        )
    })?;

    let selected_split_classes = selected_children_by_class
        .into_iter()
        .map(|(broad_class_id, selected_children)| {
            serde_json::json!({
                "broad_class_id": broad_class_id,
                "selected_children": selected_children
            })
        })
        .collect::<Vec<_>>();
    let selected_split_report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_split_refinement_selected_splits_v1",
        "source_split_refinement_report_path": report_path,
        "source_candidate_jsonl_path": candidate_jsonl_path,
        "generated_selected_trace_path": selected_split_trace_path,
        "source_drilldown_report_path": drilldown_report_path,
        "source_targeted_shadow_report_path": targeted_shadow_report_path,
        "selection_policy": "top_stageable_source_neutral_split_per_false_accept_risk_bucket_v1",
        "selected_class_count": selected_split_classes.len(),
        "selected_split_count": selected_split_classes
            .iter()
            .filter_map(|class| class.get("selected_children").and_then(Value::as_array))
            .map(Vec::len)
            .sum::<usize>(),
        "classes": selected_split_classes,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "compile_allowed": false,
        "local_accept_enabled": false,
        "market_money_claim_allowed": false,
        "boundary": "selected-split seed report only: converts automatic targeted split-refinement candidates into existing .nwpc quarantine input; does not compile, promote, serve, enable local_accept, or use legacy nwrb"
    });
    write_json_file(&selected_split_report_path, &selected_split_report)?;

    let report = serde_json::json!({
        "report_kind": "phase_stream_online_miner_targeted_split_refinement_v1",
        "mode": "audit_only_source_neutral_false_accept_split_refinement",
        "drilldown_report_path": drilldown_report_path,
        "targeted_shadow_report_path": targeted_shadow_report_path,
        "decision_log_path": decision_log_path,
        "trace_paths": trace_paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        "candidate_jsonl_path": candidate_jsonl_path,
        "selected_split_report_path": selected_split_report_path,
        "selected_split_trace_path": selected_split_trace_path,
        "risk_bucket_count": risk_buckets.len(),
        "risk_buckets_with_decision_rows": bucket_rows.len(),
        "raw_unique_accepts_in_risk_buckets": raw_unique_accepts_in_risk_buckets,
        "raw_false_accepts_in_risk_buckets": raw_false_accepts_in_risk_buckets,
        "clean_split_candidate_count": clean_split_candidate_count,
        "stageable_split_candidate_count": stageable_split_candidate_count,
        "risk_buckets_with_stageable_split": risk_buckets_with_stageable_split,
        "buckets": bucket_reports,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "compile_allowed": false,
        "online_learn_enabled": false,
        "online_shadow_enabled": false,
        "auto_promote_enabled": false,
        "local_accept_enabled": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": false,
        "verdict": if stageable_split_candidate_count > 0 {
            "PHASE_STREAM_ONLINE_MINER_TARGETED_SPLIT_REFINEMENT_V1_READY"
        } else {
            "PHASE_STREAM_ONLINE_MINER_TARGETED_SPLIT_REFINEMENT_V1_WATCH_NO_STAGEABLE_SPLITS"
        },
        "boundary": "audit only: ranks source-neutral observable split atoms for targeted false-accept buckets; does not compile .nwpc, score new events, tune thresholds, promote, serve, enable local_accept, or use legacy nwrb"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_online_miner_targeted_split_refinement_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  candidate_jsonl_path: {}", candidate_jsonl_path.display());
    println!(
        "  selected_split_report_path: {}",
        selected_split_report_path.display()
    );
    println!(
        "  selected_split_trace_path: {}",
        selected_split_trace_path.display()
    );
    println!("  risk_bucket_count: {}", risk_buckets.len());
    println!("  risk_buckets_with_decision_rows: {}", bucket_rows.len());
    println!("  clean_split_candidate_count: {clean_split_candidate_count}");
    println!("  stageable_split_candidate_count: {stageable_split_candidate_count}");
    println!("  local_accept_enabled: false");
    Ok(())
}

fn targeted_refinement_risk_buckets(drilldown: &Value) -> BTreeSet<String> {
    drilldown
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| {
            json_string(row, &["rejection_reason"]).as_deref() == Some("actual_false_accept_risk")
        })
        .filter_map(|row| json_string(row, &["bucket_key"]))
        .collect()
}

fn read_trace_rows(trace_paths: &[PathBuf]) -> Result<Vec<Value>, String> {
    let mut rows = Vec::new();
    for trace_path in trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read targeted split refinement trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            rows.push(serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse targeted split refinement trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?);
        }
    }
    Ok(rows)
}

fn targeted_refinement_bucket_rows(
    decision_log_path: &Path,
    trace_rows: &[Value],
    risk_buckets: &BTreeSet<String>,
) -> Result<BTreeMap<String, BucketRefinementRows>, String> {
    let text = std::fs::read_to_string(decision_log_path).map_err(|error| {
        format!(
            "failed to read targeted split refinement decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    let mut buckets = BTreeMap::<String, BucketRefinementRows>::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let decision = serde_json::from_str::<Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse targeted split refinement decision log '{}' line {}: {error}",
                decision_log_path.display(),
                line_index + 1
            )
        })?;
        let Some(bucket_key) = json_string(&decision, &["bucket_key"]) else {
            continue;
        };
        if !risk_buckets.contains(&bucket_key) {
            continue;
        }
        let safe_unique = json_bool(&decision, &["unique_cpu_accept_over_exact_cache"])
            == Some(true)
            && json_bool(&decision, &["verified_safe_accept"]) == Some(true)
            && json_bool(&decision, &["false_accept"]) != Some(true);
        let false_accept = json_bool(&decision, &["false_accept"]) == Some(true);
        if !safe_unique && !false_accept {
            continue;
        }
        let denominator_row_index = json_u64(&decision, &["denominator_row_index"])
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                format!(
                    "targeted split refinement decision line {} missing denominator_row_index",
                    line_index + 1
                )
            })?;
        let trace_row = trace_rows
            .get(denominator_row_index.saturating_sub(1))
            .ok_or_else(|| {
                format!(
                    "targeted split refinement denominator_row_index {} outside trace rows",
                    denominator_row_index
                )
            })?;
        let decision_fingerprint = json_string(&decision, &["request_fingerprint"]);
        let trace_fingerprint = json_string(trace_row, &["request_fingerprint"]);
        if decision_fingerprint != trace_fingerprint {
            return Err(format!(
                "targeted split refinement fingerprint mismatch at decision line {}",
                line_index + 1
            ));
        }
        let tokens = json_u64(&decision, &["token_cost", "total_tokens"])
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let atoms = source_neutral_refinement_atoms(trace_row);
        if atoms.is_empty() {
            continue;
        }
        let bucket = buckets.entry(bucket_key).or_default();
        bucket.raw_unique_accepts += usize::from(safe_unique);
        bucket.raw_false_accepts += usize::from(false_accept);
        bucket.rows.push(RefinementDecisionRow {
            safe_unique,
            false_accept,
            tokens,
            atoms,
            trace_row: trace_row.clone(),
        });
    }
    Ok(buckets)
}

fn source_neutral_refinement_atoms(row: &Value) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    for key in [
        "action_atoms",
        "request_atoms",
        "state_atoms",
        "tool_atoms",
        "route_hint_atoms",
        "result_atoms",
        "subroute_evidence_atoms",
    ] {
        if let Some(values) = row.get(key).and_then(Value::as_array) {
            for value in values {
                if let Some(atom) = value
                    .as_str()
                    .filter(|atom| source_neutral_atom_allowed(atom))
                {
                    atoms.insert(atom.to_owned());
                }
            }
        }
    }
    atoms.into_iter().collect()
}

fn source_neutral_atom_allowed(atom: &str) -> bool {
    if atom.len() > 180 || !atom.contains(':') {
        return false;
    }
    let lower = atom.to_ascii_lowercase();
    ![
        "exact_cache",
        "fingerprint",
        "provider",
        "proof_rule",
        "target_id",
        "concrete_x",
        "local_out_t",
        "source_trace",
        "traffic_source",
        "schema_version",
        "event_timestamp",
        "session_bucket",
        "cwd_kind",
        "input_trace",
        "verifier_label",
    ]
    .iter()
    .any(|blocked| lower.contains(blocked))
}

fn targeted_refinement_candidates(bucket: &BucketRefinementRows) -> Vec<SplitAtomStats> {
    let mut singles = BTreeMap::<String, SplitAtomStats>::new();
    for row in &bucket.rows {
        for atom in &row.atoms {
            let stats = singles
                .entry(atom.clone())
                .or_insert_with(|| SplitAtomStats {
                    split_atom: atom.clone(),
                    split_group: "single",
                    ..SplitAtomStats::default()
                });
            observe_split_stats(stats, row);
        }
    }
    let mut basis = singles
        .values()
        .filter(|stats| stats.safe_unique_hits > 0)
        .cloned()
        .collect::<Vec<_>>();
    basis.sort_by(|left, right| {
        right
            .safe_unique_tokens
            .cmp(&left.safe_unique_tokens)
            .then_with(|| right.safe_unique_hits.cmp(&left.safe_unique_hits))
            .then_with(|| left.false_hits.cmp(&right.false_hits))
            .then_with(|| left.split_atom.cmp(&right.split_atom))
    });
    basis.truncate(PAIR_BASIS_ATOMS_PER_BUCKET);
    let basis_atoms = basis
        .iter()
        .map(|stats| stats.split_atom.clone())
        .collect::<BTreeSet<_>>();
    let mut pairs = BTreeMap::<String, SplitAtomStats>::new();
    for row in &bucket.rows {
        let row_atoms = row
            .atoms
            .iter()
            .filter(|atom| basis_atoms.contains(*atom))
            .collect::<Vec<_>>();
        for left_index in 0..row_atoms.len() {
            for right_index in (left_index + 1)..row_atoms.len() {
                let pair = format!(
                    "multi2:{}|{}",
                    row_atoms[left_index], row_atoms[right_index]
                );
                let stats = pairs.entry(pair.clone()).or_insert_with(|| SplitAtomStats {
                    split_atom: pair,
                    split_group: "multi2",
                    ..SplitAtomStats::default()
                });
                observe_split_stats(stats, row);
            }
        }
    }
    let mut candidates = singles
        .into_values()
        .chain(pairs.into_values())
        .filter(|stats| stats.safe_unique_hits > 0)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .clean()
            .cmp(&left.clean())
            .then_with(|| right.stageable().cmp(&left.stageable()))
            .then_with(|| right.score().cmp(&left.score()))
            .then_with(|| left.split_atom.cmp(&right.split_atom))
    });
    candidates
}

fn observe_split_stats(stats: &mut SplitAtomStats, row: &RefinementDecisionRow) {
    stats.total_hits += 1;
    if row.safe_unique {
        stats.safe_unique_hits += 1;
        stats.safe_unique_tokens = stats.safe_unique_tokens.saturating_add(row.tokens);
    }
    if row.false_accept {
        stats.false_hits += 1;
    }
}

fn targeted_split_refinement_selected_report_path(report_path: &Path) -> PathBuf {
    let mut path = report_path.to_path_buf();
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-online-miner-targeted-split-refinement-v1");
    path.set_file_name(format!("{stem}.selected-splits.json"));
    path
}

fn targeted_split_refinement_selected_trace_path(report_path: &Path) -> PathBuf {
    let mut path = report_path.to_path_buf();
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-online-miner-targeted-split-refinement-v1");
    path.set_file_name(format!("{stem}.selected-splits-trace.jsonl"));
    path
}

fn targeted_refinement_selected_split_rule(candidate: &SplitAtomStats) -> Option<String> {
    let mut required = BTreeSet::<String>::new();
    for atom in targeted_refinement_split_atoms(&candidate.split_atom) {
        required.insert(targeted_refinement_selected_atom(&atom)?);
    }
    if required.is_empty() {
        return None;
    }
    Some(format!(
        "all::{}",
        required.into_iter().collect::<Vec<_>>().join(" && ")
    ))
}

fn targeted_refinement_split_atoms(split_atom: &str) -> Vec<String> {
    split_atom
        .strip_prefix("multi2:")
        .unwrap_or(split_atom)
        .split('|')
        .filter(|atom| !atom.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

fn targeted_refinement_selected_atom(atom: &str) -> Option<String> {
    let group = if atom.starts_with("request_command_kind:") {
        "request"
    } else if atom.starts_with("tool_command_kind:")
        || atom.starts_with("tool_command_shell_family:")
        || atom.starts_with("tool_name:")
    {
        "tool"
    } else if atom.starts_with("evidence:") || atom.starts_with("exit_code_band:") {
        "result"
    } else if atom.starts_with("route_hint:") {
        "route"
    } else if atom.starts_with("state_") || atom.starts_with("state:") {
        "state"
    } else if atom.starts_with("action:") {
        "action"
    } else {
        return None;
    };
    Some(format!("{group}::{atom}"))
}

fn targeted_refinement_generated_broad_class_id(candidate_id: &str) -> String {
    let suffix = candidate_id
        .strip_prefix("split_refinement-")
        .unwrap_or(candidate_id)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    format!("action_family:targeted_split_refinement_{suffix}")
}

fn targeted_refinement_selected_trace_row(
    row: &RefinementDecisionRow,
    generated_broad_class_id: &str,
    candidate_id: &str,
) -> Value {
    let mut trace_row = row.trace_row.clone();
    trace_row["verified_safe_accept"] = Value::Bool(row.safe_unique);
    trace_row["exact_cache_key"] = Value::String(format!(
        "{candidate_id}:{}",
        json_string(&trace_row, &["exact_cache_key"])
            .or_else(|| json_string(&trace_row, &["request_fingerprint"]))
            .unwrap_or_else(|| "missing_exact_cache_key".to_owned())
    ));
    append_string_atom(&mut trace_row, "action_atoms", generated_broad_class_id);
    append_string_atom(
        &mut trace_row,
        "route_hint_atoms",
        &format!("route_hint:auto_split_refinement:{candidate_id}"),
    );
    trace_row
}

fn append_string_atom(row: &mut Value, key: &str, atom: &str) {
    if !row.get(key).is_some_and(Value::is_array) {
        row[key] = Value::Array(Vec::new());
    }
    if let Some(values) = row.get_mut(key).and_then(Value::as_array_mut) {
        if !values.iter().any(|value| value.as_str() == Some(atom)) {
            values.push(Value::String(atom.to_owned()));
        }
    }
}
