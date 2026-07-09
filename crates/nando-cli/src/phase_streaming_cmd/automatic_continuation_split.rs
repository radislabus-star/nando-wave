use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::{
    generic_count_band, json_bool, json_string, per_thousand, phase_atom_action_families,
    phase_atom_binary_token_cost, phase_atom_string_vec, read_json_value, write_json_file,
};

const DEFAULT_AUTOMATIC_CONTINUATION_SPLIT_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-automatic-continuation-split-v1.report.json";
const DEFAULT_SELECTED_SPLIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-constrained-split-miner-v1-realtrace-plus-verifier-sources-v1.report.json";
const MIN_POLICY_SUPPORT: usize = 4;
const MIN_FUTURE_ROWS: usize = 1;
const MAX_PARENT_COVERAGE_MILLI: usize = 900;
const MAX_SELECTED_SUBSPLITS_PER_PARENT: usize = 16;
const MAX_REJECTED_SUBSPLITS_PER_PARENT: usize = 16;
const MAX_SECONDARY_ATOMS_PER_ROW: usize = 96;
const MAX_DEPTH2_SEED_ATOMS_PER_PARENT: usize = 24;
const MAX_DEPTH2_ATOMS_PER_ROW: usize = 24;
const MAX_DEPTH3_SEED_ATOMS_PER_PARENT: usize = 12;
const MAX_DEPTH3_ATOMS_PER_ROW: usize = 12;

#[derive(Clone)]
struct ContinuationSourceRow {
    stream_index: usize,
    broad_class_id: String,
    request_fingerprint: String,
    exact_cache_hit: bool,
    verified_safe_accept: Option<bool>,
    total_tokens: usize,
    total_cost_microusd: u64,
    atoms: Vec<String>,
}

#[derive(Clone)]
struct ParentSplit {
    broad_class_id: String,
    split_rule: String,
}

#[derive(Default)]
struct CandidateCounts {
    policy_rows: usize,
    policy_true: usize,
    policy_false: usize,
    future_rows: usize,
    future_true: usize,
    future_false: usize,
    future_non_exact_true: usize,
    future_tokens_over_exact_cache: usize,
    future_cost_microusd_over_exact_cache: u64,
    future_exact_cache_hits: usize,
    future_fingerprints: BTreeSet<String>,
    future_stream_indices: BTreeSet<usize>,
}

#[derive(Clone, Default)]
struct AtomSurvivalFeedback {
    split_count: usize,
    survived_split_count: usize,
    runtime_unique_accepts_over_exact_cache: usize,
    runtime_tokens_saved: usize,
    false_accept_split_count: usize,
    no_runtime_value_split_count: usize,
}

#[derive(Serialize)]
struct SelectedContinuationSubsplit {
    split_rule: String,
    parent_split_rule: String,
    secondary_atom: String,
    support: usize,
    future_rows: usize,
    marginal_accepts_over_exact_cache: usize,
    tokens_saved: usize,
    cost_saved_microusd: u64,
    false_accepts: usize,
    information_gain: usize,
    mdl_penalty: usize,
    net_gain: i64,
    split_depth: usize,
    survival_prior_score: i64,
    verifier_ready_rows: usize,
    cache_overlap: usize,
}

#[derive(Serialize)]
struct RejectedContinuationSubsplit {
    split_rule: String,
    parent_split_rule: String,
    secondary_atom: String,
    reason: &'static str,
    support: usize,
    future_rows: usize,
    marginal_accepts_over_exact_cache: usize,
    false_accepts: usize,
    information_gain: usize,
    net_gain: i64,
    split_depth: usize,
    survival_prior_score: i64,
}

#[derive(Serialize)]
struct ContinuationClassReport {
    broad_class_id: String,
    broad_class_rows: usize,
    verifier_ready_rows: usize,
    rows_missing_verifier: usize,
    broad_class_true_rows: usize,
    broad_class_false_accepts: usize,
    candidate_split_count: usize,
    selected_split_count: usize,
    rejected_split_count: usize,
    policy_rows: usize,
    future_rows: usize,
    parent_split_rule: String,
    parent_policy_rows: usize,
    parent_future_rows: usize,
    selected_children: Vec<SelectedContinuationSubsplit>,
    rejected_children: Vec<RejectedContinuationSubsplit>,
    selected_future_unique_accepts_over_exact_cache: usize,
    selected_future_tokens_saved: usize,
    selected_future_cost_saved_microusd: u64,
    selected_future_false_accepts: usize,
    cpu_ready_future_rows: usize,
    rejected_openai_needed_or_unknown_rows: usize,
    exact_cache_overlap_milli: usize,
}

#[derive(Default, Serialize)]
struct ContinuationGlobalDeltaReport {
    broad_split_accepts: usize,
    after_accepts: usize,
    broad_split_tokens: usize,
    after_tokens: usize,
    broad_split_cost_microusd: u64,
    after_cost_microusd: u64,
    false_accepts: usize,
}

pub(crate) fn run_phase_stream_automatic_continuation_split_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AUTOMATIC_CONTINUATION_SPLIT_REPORT));
    let selected_split_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_REPORT));
    let mut remaining_args = args.collect::<Vec<_>>();
    let mut survival_feedback_paths = Vec::new();
    while remaining_args
        .first()
        .is_some_and(|arg| arg == "--survival-feedback")
    {
        remaining_args.remove(0);
        survival_feedback_paths.push(PathBuf::from(
            remaining_args
                .first()
                .ok_or_else(|| "--survival-feedback requires a report path".to_owned())?
                .clone(),
        ));
        remaining_args.remove(0);
    }
    let input_paths = remaining_args
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let parent_splits = read_parent_splits(&selected_split_report_path)?;
    let survival_feedback = read_survival_feedback_reports(&survival_feedback_paths)?;
    let selected_classes = parent_splits
        .iter()
        .map(|split| split.broad_class_id.clone())
        .collect::<BTreeSet<_>>();
    let rows = read_continuation_rows(&input_paths, &selected_classes)?;

    let mut rows_by_class = BTreeMap::<String, Vec<ContinuationSourceRow>>::new();
    for row in rows {
        rows_by_class
            .entry(row.broad_class_id.clone())
            .or_default()
            .push(row);
    }

    let parent_accepts = read_parent_accepts(&selected_split_report_path);
    let mut global_seen = BTreeSet::<String>::new();
    let mut reports = Vec::new();
    for parent in &parent_splits {
        let class_rows = rows_by_class
            .get(&parent.broad_class_id)
            .cloned()
            .unwrap_or_default();
        reports.push(continuation_class_report(
            parent,
            class_rows,
            &mut global_seen,
            &survival_feedback,
        ));
    }
    reports.sort_by(|left, right| {
        right
            .selected_future_tokens_saved
            .cmp(&left.selected_future_tokens_saved)
            .then_with(|| {
                right
                    .selected_future_unique_accepts_over_exact_cache
                    .cmp(&left.selected_future_unique_accepts_over_exact_cache)
            })
            .then_with(|| left.broad_class_id.cmp(&right.broad_class_id))
    });

    let selected_split_count = reports
        .iter()
        .map(|class| class.selected_split_count)
        .sum::<usize>();
    let candidate_split_count = reports
        .iter()
        .map(|class| class.candidate_split_count)
        .sum::<usize>();
    let rejected_split_count = reports
        .iter()
        .map(|class| class.rejected_split_count)
        .sum::<usize>();
    let global_delta = ContinuationGlobalDeltaReport {
        broad_split_accepts: parent_accepts.0,
        after_accepts: reports
            .iter()
            .map(|class| class.selected_future_unique_accepts_over_exact_cache)
            .sum(),
        broad_split_tokens: parent_accepts.1,
        after_tokens: reports
            .iter()
            .map(|class| class.selected_future_tokens_saved)
            .sum(),
        broad_split_cost_microusd: parent_accepts.2,
        after_cost_microusd: reports
            .iter()
            .map(|class| class.selected_future_cost_saved_microusd)
            .sum(),
        false_accepts: reports
            .iter()
            .map(|class| class.selected_future_false_accepts)
            .sum(),
    };
    let verdict = if global_delta.false_accepts > 0 {
        "PHASE_STREAM_AUTOMATIC_CONTINUATION_SPLIT_V1_FAIL_FALSE_ACCEPTS"
    } else if selected_split_count == 0 || global_delta.after_accepts == 0 {
        "PHASE_STREAM_AUTOMATIC_CONTINUATION_SPLIT_V1_WATCH_NO_SAFE_SUBSPLITS"
    } else {
        "PHASE_STREAM_AUTOMATIC_CONTINUATION_SPLIT_V1_PASS_SAFE_AUTOMATIC_SUBSPLITS"
    };

    let report = serde_json::json!({
        "report_kind": "phase_stream_automatic_continuation_split_v1",
        "selected_split_report_path": selected_split_report_path,
        "survival_feedback_paths": survival_feedback_paths,
        "input_paths": input_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>(),
        "parent_broad_split_count": parent_splits.len(),
        "automatic_candidate_subsplits": candidate_split_count,
        "selected_cpu_subsplits": selected_split_count,
        "rejected_subsplit_count": rejected_split_count,
        "global_delta": global_delta,
        "classes": reports,
        "selection_policy": {
            "candidate_generator": "automatic multi-split: pair::parent_selected_split && observable_atom plus bounded all::parent && atom_a && atom_b and all::parent && atom_a && atom_b && atom_c sub-splits from request/state/action/tool/result/route/context evidence",
            "policy_window": "first half of verifier-ready parent-split rows per broad action class",
            "future_window": "second half of verifier-ready parent-split rows per broad action class",
            "constraint": "select sub-split only when policy_false_accepts = 0 and future_false_accepts = 0",
            "objective": "maximize future unique accepts/tokens/cost over exact cache with support/MDL/cache-overlap penalties",
            "survival_feedback": "optional repeated prior from selected_split_nwpc_loss_audit atom_survival_summary reports; merged across reports and used only to rank cold candidates before the same .nwpc false_accept gate",
            "survival_feedback_hard_veto_used": false,
            "survival_feedback_atom_count": survival_feedback.len(),
            "minimum_policy_support": MIN_POLICY_SUPPORT,
            "minimum_future_rows": MIN_FUTURE_ROWS,
            "max_selected_subsplits_per_parent": MAX_SELECTED_SUBSPLITS_PER_PARENT,
            "manual_class_list_used": false
        },
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
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "product_promotion_allowed": false,
        "market_money_claim_allowed": false,
        "verdict": verdict,
        "boundary": "cold automatic continuation split only: refines broad selected splits into automatic CPU-ready sub-splits before .nwpc compilation; does not compile, promote, serve, enable local_accept, claim market money, or use legacy nwrb/role-binding paths"
    });
    write_json_file(&report_path, &report)?;

    println!("phase_stream_automatic_continuation_split_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  automatic_candidate_subsplits: {candidate_split_count}");
    println!("  selected_cpu_subsplits: {selected_split_count}");
    println!(
        "  broad_split_accepts: {}",
        global_delta.broad_split_accepts
    );
    println!(
        "  safe_subsplit_accepts_over_exact_cache: {}",
        global_delta.after_accepts
    );
    println!("  false_accepts: {}", global_delta.false_accepts);
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_parent_splits(path: &Path) -> Result<Vec<ParentSplit>, String> {
    let report = read_json_value(path)?;
    let mut splits = Vec::new();
    let Some(classes) = report.get("classes").and_then(Value::as_array) else {
        return Ok(splits);
    };
    for class in classes {
        let Some(broad_class_id) = json_string(class, &["broad_class_id"]) else {
            continue;
        };
        let Some(children) = class.get("selected_children").and_then(Value::as_array) else {
            continue;
        };
        for child in children {
            let Some(split_rule) = json_string(child, &["split_rule"]) else {
                continue;
            };
            splits.push(ParentSplit {
                broad_class_id: broad_class_id.clone(),
                split_rule,
            });
        }
    }
    Ok(splits)
}

fn read_parent_accepts(path: &Path) -> (usize, usize, u64) {
    let Ok(report) = read_json_value(path) else {
        return (0, 0, 0);
    };
    let Some(global_delta) = report.get("global_delta") else {
        return (0, 0, 0);
    };
    (
        json_usize(global_delta, &["after_accepts"]),
        json_usize(global_delta, &["after_tokens"]),
        json_u64_at(global_delta, &["after_cost_microusd"]).unwrap_or(0),
    )
}

fn read_survival_feedback_reports(
    paths: &[PathBuf],
) -> Result<BTreeMap<String, AtomSurvivalFeedback>, String> {
    let mut merged = BTreeMap::new();
    for path in paths {
        merge_survival_feedback(&mut merged, &read_survival_feedback(path)?);
    }
    Ok(merged)
}

fn read_survival_feedback(path: &Path) -> Result<BTreeMap<String, AtomSurvivalFeedback>, String> {
    let report = read_json_value(path)?;
    let mut feedback = BTreeMap::new();
    let Some(rows) = report
        .get("atom_survival_summary")
        .and_then(Value::as_array)
    else {
        return Ok(feedback);
    };
    for row in rows {
        let Some(atom) = json_string(row, &["atom"]) else {
            continue;
        };
        feedback.insert(
            atom,
            AtomSurvivalFeedback {
                split_count: json_usize(row, &["split_count"]),
                survived_split_count: json_usize(row, &["survived_split_count"]),
                runtime_unique_accepts_over_exact_cache: json_usize(
                    row,
                    &["runtime_unique_accepts_over_exact_cache"],
                ),
                runtime_tokens_saved: json_usize(row, &["runtime_tokens_saved"]),
                false_accept_split_count: json_usize(row, &["false_accept_split_count"]),
                no_runtime_value_split_count: json_usize(row, &["no_runtime_value_split_count"]),
            },
        );
    }
    Ok(feedback)
}

fn merge_survival_feedback(
    merged: &mut BTreeMap<String, AtomSurvivalFeedback>,
    next: &BTreeMap<String, AtomSurvivalFeedback>,
) {
    for (atom, feedback) in next {
        let target = merged.entry(atom.clone()).or_default();
        target.split_count = target.split_count.saturating_add(feedback.split_count);
        target.survived_split_count = target
            .survived_split_count
            .saturating_add(feedback.survived_split_count);
        target.runtime_unique_accepts_over_exact_cache = target
            .runtime_unique_accepts_over_exact_cache
            .saturating_add(feedback.runtime_unique_accepts_over_exact_cache);
        target.runtime_tokens_saved = target
            .runtime_tokens_saved
            .saturating_add(feedback.runtime_tokens_saved);
        target.false_accept_split_count = target
            .false_accept_split_count
            .saturating_add(feedback.false_accept_split_count);
        target.no_runtime_value_split_count = target
            .no_runtime_value_split_count
            .saturating_add(feedback.no_runtime_value_split_count);
    }
}

fn read_continuation_rows(
    paths: &[PathBuf],
    selected_classes: &BTreeSet<String>,
) -> Result<Vec<ContinuationSourceRow>, String> {
    let mut rows = Vec::new();
    let mut seen_exact_cache = BTreeSet::new();
    for path in paths {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let row = serde_json::from_str::<Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse continuation split input '{}' line {}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
            let Some(verified_safe_accept) = json_bool(&row, &["verified_safe_accept"]) else {
                continue;
            };
            let action_families =
                phase_atom_action_families(&phase_atom_string_vec(&row, "action_atoms"));
            let selected_action_families = action_families
                .into_iter()
                .filter(|family| selected_classes.contains(family))
                .collect::<Vec<_>>();
            if selected_action_families.is_empty() {
                continue;
            }
            let request_fingerprint = json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("automatic-continuation-row:{}", rows.len() + 1));
            let exact_cache_key = json_string(&row, &["exact_cache_key"])
                .unwrap_or_else(|| request_fingerprint.clone());
            let exact_cache_hit = !seen_exact_cache.insert(exact_cache_key);
            let token_cost = phase_atom_binary_token_cost(&row);
            let atoms = continuation_atoms(
                &row,
                token_cost.total_tokens,
                token_cost.total_cost_microusd,
            );
            if atoms.is_empty() {
                continue;
            }
            for broad_class_id in selected_action_families {
                rows.push(ContinuationSourceRow {
                    stream_index: rows.len() + 1,
                    broad_class_id,
                    request_fingerprint: request_fingerprint.clone(),
                    exact_cache_hit,
                    verified_safe_accept: Some(verified_safe_accept),
                    total_tokens: token_cost.total_tokens,
                    total_cost_microusd: token_cost.total_cost_microusd,
                    atoms: atoms.clone(),
                });
            }
        }
    }
    Ok(rows)
}

fn continuation_class_report(
    parent: &ParentSplit,
    mut class_rows: Vec<ContinuationSourceRow>,
    global_seen: &mut BTreeSet<String>,
    survival_feedback: &BTreeMap<String, AtomSurvivalFeedback>,
) -> ContinuationClassReport {
    class_rows.sort_by(|left, right| left.stream_index.cmp(&right.stream_index));
    let broad_class_rows = class_rows.len();
    let verifier_ready_rows = class_rows.len();
    let rows_missing_verifier = 0usize;
    let broad_class_true_rows = class_rows
        .iter()
        .filter(|row| row.verified_safe_accept == Some(true))
        .count();
    let broad_class_false_accepts = class_rows
        .iter()
        .filter(|row| row.verified_safe_accept == Some(false))
        .count();
    let exact_cache_hits = class_rows.iter().filter(|row| row.exact_cache_hit).count();
    let parent_rows = class_rows
        .into_iter()
        .filter(|row| row_matches_rule(row, &parent.split_rule))
        .collect::<Vec<_>>();
    let (policy_rows, future_rows) = split_policy_future(parent_rows);
    let parent_policy_true = policy_rows
        .iter()
        .filter(|row| row.verified_safe_accept == Some(true))
        .count();
    let parent_policy_false = policy_rows.len().saturating_sub(parent_policy_true);
    let candidate_map = continuation_candidate_counts(parent, &policy_rows, &future_rows);
    let candidate_split_count = candidate_map.len();
    let mut selected = Vec::new();
    let mut rejected = Vec::new();
    let mut selected_future_rows = BTreeSet::<usize>::new();

    let mut candidates = candidate_map
        .into_iter()
        .map(|(secondary_atom, counts)| {
            let information_gain =
                information_gain_milli(parent_policy_true, parent_policy_false, &counts);
            let split_rule = continuation_split_rule(&parent.split_rule, &secondary_atom);
            let survival_prior_score =
                split_survival_prior_score(&secondary_atom, survival_feedback);
            let mdl_penalty = split_rule.len().div_ceil(16).max(1);
            let cache_overlap_penalty = counts.future_exact_cache_hits.saturating_mul(100);
            let false_accepts = counts.policy_false.saturating_add(counts.future_false);
            let split_depth = split_rule_required_atoms(&split_rule).len();
            let net_gain = counts
                .future_tokens_over_exact_cache
                .saturating_add(counts.future_non_exact_true.saturating_mul(1000))
                .saturating_add(information_gain)
                .saturating_sub(mdl_penalty)
                .saturating_sub(cache_overlap_penalty) as i64;
            (
                secondary_atom,
                split_rule,
                counts,
                information_gain,
                mdl_penalty,
                net_gain,
                false_accepts,
                split_depth,
                survival_prior_score,
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .8
            .cmp(&left.8)
            .then_with(|| right.5.cmp(&left.5))
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| left.7.cmp(&right.7))
            .then_with(|| left.1.cmp(&right.1))
    });

    for (
        secondary_atom,
        split_rule,
        counts,
        information_gain,
        mdl_penalty,
        net_gain,
        false_accepts,
        split_depth,
        survival_prior_score,
    ) in candidates
    {
        let reject_reason =
            split_reject_reason(&counts, false_accepts, policy_rows.len(), future_rows.len());
        if reject_reason.is_none() {
            let mut marginal_accepts = 0usize;
            for fingerprint in &counts.future_fingerprints {
                if global_seen.insert(fingerprint.clone()) {
                    marginal_accepts += 1;
                }
            }
            if marginal_accepts > 0 && selected.len() < MAX_SELECTED_SUBSPLITS_PER_PARENT {
                selected_future_rows.extend(counts.future_stream_indices.iter().copied());
                selected.push(SelectedContinuationSubsplit {
                    split_rule,
                    parent_split_rule: parent.split_rule.clone(),
                    secondary_atom,
                    support: counts.policy_rows,
                    future_rows: counts.future_rows,
                    marginal_accepts_over_exact_cache: marginal_accepts,
                    tokens_saved: counts.future_tokens_over_exact_cache,
                    cost_saved_microusd: counts.future_cost_microusd_over_exact_cache,
                    false_accepts,
                    information_gain,
                    mdl_penalty,
                    net_gain,
                    split_depth,
                    survival_prior_score,
                    verifier_ready_rows: counts.policy_rows.saturating_add(counts.future_rows),
                    cache_overlap: per_thousand(counts.future_exact_cache_hits, counts.future_rows),
                });
            }
        } else if rejected.len() < MAX_REJECTED_SUBSPLITS_PER_PARENT {
            rejected.push(RejectedContinuationSubsplit {
                split_rule,
                parent_split_rule: parent.split_rule.clone(),
                secondary_atom,
                reason: reject_reason.unwrap_or("unknown"),
                support: counts.policy_rows,
                future_rows: counts.future_rows,
                marginal_accepts_over_exact_cache: counts.future_non_exact_true,
                false_accepts,
                information_gain,
                net_gain,
                split_depth,
                survival_prior_score,
            });
        }
    }

    let selected_future_unique_accepts_over_exact_cache = selected
        .iter()
        .map(|child| child.marginal_accepts_over_exact_cache)
        .sum();
    let selected_future_tokens_saved = selected.iter().map(|child| child.tokens_saved).sum();
    let selected_future_cost_saved_microusd =
        selected.iter().map(|child| child.cost_saved_microusd).sum();
    let selected_future_false_accepts = selected.iter().map(|child| child.false_accepts).sum();
    let cpu_ready_future_rows = selected_future_rows.len();
    let rejected_openai_needed_or_unknown_rows =
        future_rows.len().saturating_sub(cpu_ready_future_rows);
    let selected_split_count = selected.len();
    let rejected_split_count = candidate_split_count.saturating_sub(selected_split_count);

    ContinuationClassReport {
        broad_class_id: parent.broad_class_id.clone(),
        broad_class_rows,
        verifier_ready_rows,
        rows_missing_verifier,
        broad_class_true_rows,
        broad_class_false_accepts,
        candidate_split_count,
        selected_split_count,
        rejected_split_count,
        policy_rows: policy_rows.len(),
        future_rows: future_rows.len(),
        parent_split_rule: parent.split_rule.clone(),
        parent_policy_rows: policy_rows.len(),
        parent_future_rows: future_rows.len(),
        selected_children: selected,
        rejected_children: rejected,
        selected_future_unique_accepts_over_exact_cache,
        selected_future_tokens_saved,
        selected_future_cost_saved_microusd,
        selected_future_false_accepts,
        cpu_ready_future_rows,
        rejected_openai_needed_or_unknown_rows,
        exact_cache_overlap_milli: per_thousand(exact_cache_hits, broad_class_rows),
    }
}

fn continuation_candidate_counts(
    parent: &ParentSplit,
    policy_rows: &[ContinuationSourceRow],
    future_rows: &[ContinuationSourceRow],
) -> BTreeMap<String, CandidateCounts> {
    let mut single_candidates = BTreeMap::<String, CandidateCounts>::new();
    for row in policy_rows {
        for atom in row_secondary_atoms(row, &parent.split_rule) {
            update_candidate_counts(single_candidates.entry(atom).or_default(), row, false);
        }
    }
    for row in future_rows {
        for atom in row_secondary_atoms(row, &parent.split_rule) {
            update_candidate_counts(single_candidates.entry(atom).or_default(), row, true);
        }
    }
    let depth2_seed_atoms =
        depth2_seed_atoms(&single_candidates, policy_rows.len(), future_rows.len());
    let depth2_seed_set = depth2_seed_atoms.iter().cloned().collect::<BTreeSet<_>>();
    let depth3_seed_set = depth2_seed_atoms
        .iter()
        .take(MAX_DEPTH3_SEED_ATOMS_PER_PARENT)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut candidates = single_candidates;
    for row in policy_rows {
        for atom_pair in row_depth2_atom_pairs(row, &parent.split_rule, &depth2_seed_set) {
            update_candidate_counts(candidates.entry(atom_pair).or_default(), row, false);
        }
        for atom_triple in row_depth3_atom_triples(row, &parent.split_rule, &depth3_seed_set) {
            update_candidate_counts(candidates.entry(atom_triple).or_default(), row, false);
        }
    }
    for row in future_rows {
        for atom_pair in row_depth2_atom_pairs(row, &parent.split_rule, &depth2_seed_set) {
            update_candidate_counts(candidates.entry(atom_pair).or_default(), row, true);
        }
        for atom_triple in row_depth3_atom_triples(row, &parent.split_rule, &depth3_seed_set) {
            update_candidate_counts(candidates.entry(atom_triple).or_default(), row, true);
        }
    }
    candidates
}

fn depth2_seed_atoms(
    candidates: &BTreeMap<String, CandidateCounts>,
    parent_policy_rows: usize,
    parent_future_rows: usize,
) -> Vec<String> {
    let mut seeds = candidates
        .iter()
        .filter(|(_, counts)| {
            counts.policy_rows >= MIN_POLICY_SUPPORT
                && counts.policy_true > 0
                && !atom_covers_parent(counts, parent_policy_rows, parent_future_rows)
        })
        .map(|(atom, counts)| {
            let false_accepts = counts.policy_false.saturating_add(counts.future_false);
            let score = counts
                .future_tokens_over_exact_cache
                .saturating_add(counts.future_non_exact_true.saturating_mul(1000))
                .saturating_sub(false_accepts.saturating_mul(10_000));
            (
                atom.clone(),
                score,
                counts.future_non_exact_true,
                counts.policy_rows,
            )
        })
        .collect::<Vec<_>>();
    seeds.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| left.0.cmp(&right.0))
    });
    seeds
        .into_iter()
        .take(MAX_DEPTH2_SEED_ATOMS_PER_PARENT)
        .map(|(atom, _, _, _)| atom)
        .collect()
}

fn atom_covers_parent(
    counts: &CandidateCounts,
    parent_policy_rows: usize,
    parent_future_rows: usize,
) -> bool {
    per_thousand(counts.policy_rows, parent_policy_rows) >= MAX_PARENT_COVERAGE_MILLI
        && per_thousand(counts.future_rows, parent_future_rows) >= MAX_PARENT_COVERAGE_MILLI
}

fn split_survival_prior_score(
    secondary_atom: &str,
    feedback: &BTreeMap<String, AtomSurvivalFeedback>,
) -> i64 {
    if feedback.is_empty() {
        return 0;
    }
    let mut score = 0i64;
    let mut matched_atoms = 0i64;
    for atom in split_rule_required_atoms(secondary_atom) {
        let Some(atom_feedback) = feedback.get(atom) else {
            continue;
        };
        matched_atoms += 1;
        score = score
            .saturating_add(
                (atom_feedback.runtime_unique_accepts_over_exact_cache as i64)
                    .saturating_mul(1_000),
            )
            .saturating_add((atom_feedback.runtime_tokens_saved as i64) / 100)
            .saturating_add((atom_feedback.survived_split_count as i64).saturating_mul(10_000))
            .saturating_sub((atom_feedback.false_accept_split_count as i64).saturating_mul(50_000))
            .saturating_sub(
                (atom_feedback.no_runtime_value_split_count as i64).saturating_mul(2_000),
            );
        if atom_feedback.split_count > 0 && atom_feedback.survived_split_count == 0 {
            score = score.saturating_sub(10_000);
        }
    }
    if matched_atoms == 0 {
        0
    } else {
        score / matched_atoms
    }
}

fn row_depth2_atom_pairs(
    row: &ContinuationSourceRow,
    parent_split_rule: &str,
    depth2_seed_atoms: &BTreeSet<String>,
) -> Vec<String> {
    let atoms = row_secondary_atoms(row, parent_split_rule)
        .into_iter()
        .filter(|atom| depth2_seed_atoms.contains(atom))
        .take(MAX_DEPTH2_ATOMS_PER_ROW)
        .collect::<Vec<_>>();
    let mut pairs = Vec::new();
    for left_index in 0..atoms.len() {
        for right in atoms.iter().skip(left_index + 1) {
            pairs.push(format!("{} && {}", atoms[left_index], right));
        }
    }
    pairs
}

fn row_depth3_atom_triples(
    row: &ContinuationSourceRow,
    parent_split_rule: &str,
    depth3_seed_atoms: &BTreeSet<String>,
) -> Vec<String> {
    let atoms = row_secondary_atoms(row, parent_split_rule)
        .into_iter()
        .filter(|atom| depth3_seed_atoms.contains(atom))
        .take(MAX_DEPTH3_ATOMS_PER_ROW)
        .collect::<Vec<_>>();
    let mut triples = Vec::new();
    for left_index in 0..atoms.len() {
        for middle_index in (left_index + 1)..atoms.len() {
            for right in atoms.iter().skip(middle_index + 1) {
                triples.push(format!(
                    "{} && {} && {}",
                    atoms[left_index], atoms[middle_index], right
                ));
            }
        }
    }
    triples
}

fn row_secondary_atoms(row: &ContinuationSourceRow, parent_split_rule: &str) -> Vec<String> {
    let parent_required = split_rule_required_atoms(parent_split_rule)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut atoms = row
        .atoms
        .iter()
        .filter(|atom| !parent_required.contains(atom.as_str()))
        .filter(|atom| secondary_split_atom_allowed(atom))
        .take(MAX_SECONDARY_ATOMS_PER_ROW)
        .cloned()
        .collect::<Vec<_>>();
    atoms.sort();
    atoms.dedup();
    atoms
}

fn update_candidate_counts(
    counts: &mut CandidateCounts,
    row: &ContinuationSourceRow,
    future: bool,
) {
    let positive = row.verified_safe_accept == Some(true);
    let negative = row.verified_safe_accept == Some(false);
    if future {
        counts.future_rows += 1;
        counts.future_true += usize::from(positive);
        counts.future_false += usize::from(negative);
        counts.future_exact_cache_hits += usize::from(row.exact_cache_hit);
        counts.future_stream_indices.insert(row.stream_index);
        if positive && !row.exact_cache_hit {
            counts.future_non_exact_true += 1;
            counts.future_tokens_over_exact_cache = counts
                .future_tokens_over_exact_cache
                .saturating_add(row.total_tokens);
            counts.future_cost_microusd_over_exact_cache = counts
                .future_cost_microusd_over_exact_cache
                .saturating_add(row.total_cost_microusd);
            counts
                .future_fingerprints
                .insert(row.request_fingerprint.clone());
        }
    } else {
        counts.policy_rows += 1;
        counts.policy_true += usize::from(positive);
        counts.policy_false += usize::from(negative);
    }
}

fn split_reject_reason(
    counts: &CandidateCounts,
    false_accepts: usize,
    parent_policy_rows: usize,
    parent_future_rows: usize,
) -> Option<&'static str> {
    if counts.policy_rows < MIN_POLICY_SUPPORT {
        return Some("below_min_policy_support");
    }
    if counts.future_rows < MIN_FUTURE_ROWS {
        return Some("missing_future_window");
    }
    let policy_coverage = per_thousand(counts.policy_rows, parent_policy_rows);
    let future_coverage = per_thousand(counts.future_rows, parent_future_rows);
    if policy_coverage >= MAX_PARENT_COVERAGE_MILLI && future_coverage >= MAX_PARENT_COVERAGE_MILLI
    {
        return Some("subsplit_too_broad_no_refinement");
    }
    if counts.policy_true == 0 {
        return Some("no_policy_safe_evidence");
    }
    if counts.future_non_exact_true == 0 {
        return Some("no_future_accepts_over_exact_cache");
    }
    if false_accepts > 0 {
        return Some("false_accept_constraint_failed");
    }
    None
}

fn split_policy_future(
    mut rows: Vec<ContinuationSourceRow>,
) -> (Vec<ContinuationSourceRow>, Vec<ContinuationSourceRow>) {
    if rows.len() < 2 {
        return (rows, Vec::new());
    }
    rows.sort_by(|left, right| left.stream_index.cmp(&right.stream_index));
    let policy_len = (rows.len() / 2).clamp(1, rows.len().saturating_sub(1));
    let future = rows.split_off(policy_len);
    (rows, future)
}

fn continuation_atoms(row: &Value, total_tokens: usize, total_cost_microusd: u64) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    for (group, key) in [
        ("request", "request_atoms"),
        ("state", "state_atoms"),
        ("action", "action_atoms"),
        ("tool", "tool_atoms"),
        ("result", "result_atoms"),
        ("route", "route_hint_atoms"),
    ] {
        for atom in phase_atom_string_vec(row, key) {
            if continuation_atom_allowed(&atom) {
                atoms.insert(format!("{group}::{atom}"));
            }
        }
    }
    let has_shadow_request = json_bool(row, &["has_shadow_request"]).unwrap_or(false)
        || row
            .get("nando_shadow_request")
            .is_some_and(Value::is_object);
    atoms.insert(format!(
        "context::request_has_shadow_request:{has_shadow_request}"
    ));
    atoms.insert(format!(
        "token::token_band:{}",
        generic_count_band(total_tokens)
    ));
    atoms.insert(format!(
        "cost::cost_band:{}",
        generic_count_band(total_cost_microusd as usize)
    ));
    atoms.into_iter().collect()
}

fn continuation_atom_allowed(atom: &str) -> bool {
    if atom.is_empty() {
        return false;
    }
    let lower = atom.to_ascii_lowercase();
    !atom.starts_with("output_hash64:")
        && !atom.starts_with("verifier_label:")
        && !atom.starts_with("verified_safe_accept:")
        && !atom.starts_with("request_fingerprint:")
        && !atom.starts_with("exact_cache_key:")
        && !atom.starts_with("trace_id:")
        && !atom.starts_with("source_trace_id:")
        && !atom.starts_with("state_session_bucket:")
        && !atom.starts_with("action_family:")
        && !lower.contains("target_id")
        && !lower.contains("proof_rule")
        && !lower.contains("local_out_t")
        && !lower.contains("concrete_x")
        && !lower.contains("nwrb")
        && !lower.contains("role_binding")
}

fn secondary_split_atom_allowed(atom: &str) -> bool {
    !atom.starts_with("token::")
        && !atom.starts_with("cost::")
        && !atom.starts_with("action::")
        && !atom.starts_with("context::request_has_shadow_request:")
}

fn row_matches_rule(row: &ContinuationSourceRow, split_rule: &str) -> bool {
    split_rule_required_atoms(split_rule)
        .iter()
        .all(|required| row.atoms.iter().any(|atom| atom == required))
}

fn continuation_split_rule(parent_split_rule: &str, secondary_atom: &str) -> String {
    let atoms = split_rule_required_atoms(parent_split_rule)
        .into_iter()
        .chain(split_rule_required_atoms(secondary_atom))
        .filter(|atom| !atom.is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if atoms.len() <= 1 {
        atoms.join(" && ")
    } else if atoms.len() == 2 {
        format!("pair::{}", atoms.join(" && "))
    } else {
        format!("all::{}", atoms.join(" && "))
    }
}

fn split_rule_required_atoms(split_rule: &str) -> Vec<&str> {
    let rest = split_rule
        .strip_prefix("pair::")
        .or_else(|| split_rule.strip_prefix("all::"))
        .unwrap_or(split_rule);
    rest.split(" && ").collect()
}

fn information_gain_milli(
    parent_policy_true: usize,
    parent_policy_false: usize,
    counts: &CandidateCounts,
) -> usize {
    let parent_total = parent_policy_true.saturating_add(parent_policy_false);
    if parent_total == 0 || counts.policy_rows == 0 || counts.policy_rows >= parent_total {
        return 0;
    }
    let child_true = counts.policy_true;
    let child_false = counts.policy_false;
    let other_true = parent_policy_true.saturating_sub(child_true);
    let other_false = parent_policy_false.saturating_sub(child_false);
    let parent_gini = gini_milli(parent_policy_true, parent_policy_false);
    let child_total = child_true.saturating_add(child_false);
    let other_total = other_true.saturating_add(other_false);
    let weighted_child = gini_milli(child_true, child_false).saturating_mul(child_total);
    let weighted_other = gini_milli(other_true, other_false).saturating_mul(other_total);
    let weighted = weighted_child.saturating_add(weighted_other) / parent_total;
    parent_gini.saturating_sub(weighted)
}

fn gini_milli(positive: usize, negative: usize) -> usize {
    let total = positive.saturating_add(negative);
    if total == 0 {
        return 0;
    }
    let total_sq = total.saturating_mul(total);
    let pure_sq = positive
        .saturating_mul(positive)
        .saturating_add(negative.saturating_mul(negative));
    1000usize.saturating_sub(pure_sq.saturating_mul(1000) / total_sq)
}

fn json_usize(value: &Value, path: &[&str]) -> usize {
    json_u64_at(value, path)
        .and_then(|number| usize::try_from(number).ok())
        .unwrap_or(0)
}

fn json_u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_u64()
}
