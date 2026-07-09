use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use super::{json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_SELECTED_SPLIT_NWPC_LOSS_AUDIT_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-loss-audit-v1.report.json";
const DEFAULT_SELECTED_SPLIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-constrained-split-miner-v1-realtrace-plus-verifier-sources-v1.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_QUARANTINE_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-quarantine-v1-realtrace-plus-verifier-sources.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-shadow-replay-v1-realtrace-plus-verifier-sources.report.json";

#[derive(Serialize)]
struct SelectedSplitLossAuditReport {
    report_kind: &'static str,
    selected_split_report_path: String,
    quarantine_report_path: String,
    shadow_replay_report_path: String,
    selected_split_count: usize,
    audited_split_count: usize,
    selected_marginal_accepts_over_exact_cache: usize,
    runtime_unique_accepts_over_exact_cache: usize,
    unique_accept_gap: isize,
    selected_tokens_saved: usize,
    runtime_tokens_saved: usize,
    tokens_gap: isize,
    surviving_runtime_split_count: usize,
    failed_runtime_split_count: usize,
    runtime_survival_rate_milli: usize,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    serving_registry_mutated: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    row_level_missing_atom_audit_available: bool,
    splits: Vec<SelectedSplitLossRow>,
    atom_survival_summary: Vec<AtomSurvivalRow>,
    forbidden_flags: BTreeMap<&'static str, bool>,
    verdict: &'static str,
    boundary: &'static str,
}

#[derive(Serialize)]
struct SelectedSplitLossRow {
    broad_class_id: String,
    split_rule: String,
    split_depth: usize,
    required_atoms: Vec<String>,
    selected_support: usize,
    selected_verifier_ready_rows: usize,
    selected_future_rows: usize,
    selected_cache_overlap: usize,
    selected_marginal_accepts_over_exact_cache: usize,
    selected_tokens_saved: usize,
    selected_false_accepts: usize,
    package_present: bool,
    replay_present: bool,
    package_training_rows: usize,
    package_train_positive_rows: usize,
    package_train_negative_rows: usize,
    package_future_rows: usize,
    package_scoreable_future_rows: usize,
    package_matching_future_rows: usize,
    runtime_shadow_accept_rows: usize,
    runtime_unique_accepts_over_exact_cache: usize,
    runtime_tokens_saved: usize,
    runtime_false_accepts: usize,
    runtime_exact_cache_hits_reported: usize,
    runtime_wrong_wins: usize,
    threshold_micro: i64,
    train_max_false_margin_micro: Option<i64>,
    train_min_true_margin_micro: Option<i64>,
    p10_margin_micro: i64,
    median_margin_micro: i64,
    rejection_reason: String,
    rows_lost_before_package: isize,
    rows_lost_by_score_filter: isize,
    rows_lost_by_margin: isize,
    rows_lost_by_exact_cache_or_dedupe_after_accept: isize,
    unique_accept_gap: isize,
    tokens_gap: isize,
    exact_cache_policy_mismatch_suspected: bool,
    margin_threshold_pressure_suspected: bool,
    top_missing_atom_keys: Vec<String>,
    top_margin_failure_reasons: Vec<String>,
    primary_loss_reason: String,
    runtime_survived: bool,
    survival_status: String,
}

#[derive(Default, Serialize)]
struct AtomSurvivalRow {
    atom: String,
    split_count: usize,
    survived_split_count: usize,
    failed_split_count: usize,
    selected_marginal_accepts_over_exact_cache: usize,
    runtime_unique_accepts_over_exact_cache: usize,
    selected_tokens_saved: usize,
    runtime_tokens_saved: usize,
    false_accept_split_count: usize,
    margin_pressure_split_count: usize,
    no_runtime_value_split_count: usize,
}

#[derive(Clone)]
struct SelectedChild {
    broad_class_id: String,
    split_rule: String,
    support: usize,
    verifier_ready_rows: usize,
    future_rows: usize,
    cache_overlap: usize,
    marginal_accepts_over_exact_cache: usize,
    tokens_saved: usize,
    false_accepts: usize,
}

pub(crate) fn run_phase_stream_selected_split_nwpc_loss_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_LOSS_AUDIT_REPORT));
    let selected_split_report_path = next_path(&mut args, DEFAULT_SELECTED_SPLIT_REPORT);
    let quarantine_report_path =
        next_path(&mut args, DEFAULT_SELECTED_SPLIT_NWPC_QUARANTINE_REPORT);
    let shadow_replay_report_path =
        next_path(&mut args, DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT);
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let selected_report = read_json_value(&selected_split_report_path)?;
    let quarantine_report = read_json_value(&quarantine_report_path)?;
    let replay_report = read_json_value(&shadow_replay_report_path)?;

    let selected_children = read_selected_children(&selected_report);
    let quarantine_packages = package_index(&quarantine_report);
    let replay_packages = package_index(&replay_report);

    let mut splits = Vec::new();
    for child in &selected_children {
        let key = split_key(&child.broad_class_id, &child.split_rule);
        let package = quarantine_packages.get(&key);
        let replay = replay_packages.get(&key);
        splits.push(loss_row(child, package, replay));
    }

    let selected_marginal_accepts_over_exact_cache = selected_children
        .iter()
        .map(|child| child.marginal_accepts_over_exact_cache)
        .sum();
    let runtime_unique_accepts_over_exact_cache = splits
        .iter()
        .map(|split| split.runtime_unique_accepts_over_exact_cache)
        .sum();
    let selected_tokens_saved = selected_children
        .iter()
        .map(|child| child.tokens_saved)
        .sum();
    let runtime_tokens_saved = splits.iter().map(|split| split.runtime_tokens_saved).sum();
    let surviving_runtime_split_count =
        splits.iter().filter(|split| split.runtime_survived).count();
    let failed_runtime_split_count = splits.len().saturating_sub(surviving_runtime_split_count);
    let runtime_survival_rate_milli =
        per_thousand_local(surviving_runtime_split_count, splits.len());
    let atom_survival_summary = atom_survival_summary(&splits);

    let report = SelectedSplitLossAuditReport {
        report_kind: "phase_stream_selected_split_nwpc_loss_audit_v1",
        selected_split_report_path: selected_split_report_path.display().to_string(),
        quarantine_report_path: quarantine_report_path.display().to_string(),
        shadow_replay_report_path: shadow_replay_report_path.display().to_string(),
        selected_split_count: selected_children.len(),
        audited_split_count: splits.len(),
        selected_marginal_accepts_over_exact_cache,
        runtime_unique_accepts_over_exact_cache,
        unique_accept_gap: selected_marginal_accepts_over_exact_cache as isize
            - runtime_unique_accepts_over_exact_cache as isize,
        selected_tokens_saved,
        runtime_tokens_saved,
        tokens_gap: selected_tokens_saved as isize - runtime_tokens_saved as isize,
        surviving_runtime_split_count,
        failed_runtime_split_count,
        runtime_survival_rate_milli,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        serving_registry_mutated: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        row_level_missing_atom_audit_available: false,
        splits,
        atom_survival_summary,
        forbidden_flags: [
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
        .collect(),
        verdict: "PHASE_STREAM_SELECTED_SPLIT_NWPC_LOSS_AUDIT_V1_READY",
        boundary: "report-only selected-split .nwpc value-loss audit: compares automatic split value against compiled .nwpc quarantine/replay value; does not read answers, compile packages, serve, promote, enable local_accept, claim money, or use legacy nwrb",
    };
    write_json_file(&report_path, &report)?;

    println!("phase_stream_selected_split_nwpc_loss_audit_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  selected_marginal_accepts_over_exact_cache: {}",
        report.selected_marginal_accepts_over_exact_cache
    );
    println!(
        "  runtime_unique_accepts_over_exact_cache: {}",
        report.runtime_unique_accepts_over_exact_cache
    );
    println!("  unique_accept_gap: {}", report.unique_accept_gap);
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {}", report.verdict);
    Ok(())
}

fn read_selected_children(report: &Value) -> Vec<SelectedChild> {
    let mut children = Vec::new();
    let Some(classes) = report.get("classes").and_then(Value::as_array) else {
        return children;
    };
    for class in classes {
        let Some(broad_class_id) = json_string(class, &["broad_class_id"]) else {
            continue;
        };
        let Some(selected_children) = class.get("selected_children").and_then(Value::as_array)
        else {
            continue;
        };
        for child in selected_children {
            let Some(split_rule) = json_string(child, &["split_rule"]) else {
                continue;
            };
            children.push(SelectedChild {
                broad_class_id: broad_class_id.clone(),
                split_rule,
                support: json_usize(child, &["support"]),
                verifier_ready_rows: json_usize(child, &["verifier_ready_rows"]),
                future_rows: json_usize(child, &["future_rows"]),
                cache_overlap: json_usize(child, &["cache_overlap"]),
                marginal_accepts_over_exact_cache: json_usize(
                    child,
                    &["marginal_accepts_over_exact_cache"],
                ),
                tokens_saved: json_usize(child, &["tokens_saved"]),
                false_accepts: json_usize(child, &["false_accepts"]),
            });
        }
    }
    children
}

fn package_index(report: &Value) -> BTreeMap<String, Value> {
    let mut index = BTreeMap::new();
    let Some(packages) = report.get("packages").and_then(Value::as_array) else {
        return index;
    };
    for package in packages {
        let Some(broad_class_id) = json_string(package, &["broad_class_id"]) else {
            continue;
        };
        let Some(split_rule) = json_string(package, &["split_rule"]) else {
            continue;
        };
        index.insert(split_key(&broad_class_id, &split_rule), package.clone());
    }
    index
}

fn loss_row(
    child: &SelectedChild,
    package: Option<&Value>,
    replay: Option<&Value>,
) -> SelectedSplitLossRow {
    let package_training_rows = package.map_or(0, |value| json_usize(value, &["train_rows"]));
    let package_train_positive_rows =
        package.map_or(0, |value| json_usize(value, &["train_positive_rows"]));
    let package_train_negative_rows =
        package.map_or(0, |value| json_usize(value, &["train_negative_rows"]));
    let package_future_rows = package.map_or(0, |value| json_usize(value, &["future_rows"]));
    let package_scoreable_future_rows =
        package.map_or(0, |value| json_usize(value, &["future_scored_rows"]));
    let package_matching_future_rows = package.map_or(0, |value| {
        json_usize(value, &["future_matching_split_rows"])
    });
    let required_atoms = split_rule_required_atoms(&child.split_rule);
    let split_depth = required_atoms.len();

    let runtime_source = replay;
    let runtime_shadow_accept_rows =
        runtime_source.map_or(0, |value| json_usize(value, &["future_shadow_accepts"]));
    let runtime_unique_accepts_over_exact_cache = runtime_source.map_or(0, |value| {
        json_usize(value, &["future_unique_accepts_over_exact_cache"])
    });
    let runtime_tokens_saved =
        runtime_source.map_or(0, |value| json_usize(value, &["future_tokens_saved"]));
    let runtime_false_accepts =
        runtime_source.map_or(0, |value| json_usize(value, &["future_false_accepts"]));
    let runtime_exact_cache_hits_reported =
        runtime_source.map_or(0, |value| json_usize(value, &["future_exact_cache_hits"]));
    let runtime_wrong_wins =
        runtime_source.map_or(0, |value| json_usize(value, &["future_wrong_wins"]));

    let threshold_micro = package.map_or(0, |value| json_i64(value, &["threshold_micro"]));
    let train_max_false_margin_micro =
        package.and_then(|value| json_i64_option(value, &["train_max_false_margin_micro"]));
    let train_min_true_margin_micro =
        package.and_then(|value| json_i64_option(value, &["train_min_true_margin_micro"]));
    let p10_margin_micro = package.map_or(0, |value| json_i64(value, &["p10_margin_micro"]));
    let median_margin_micro = package.map_or(0, |value| json_i64(value, &["median_margin_micro"]));
    let rejection_reason = package
        .and_then(|value| json_string(value, &["rejection_reason"]))
        .unwrap_or_else(|| "package_missing".to_owned());

    let rows_lost_before_package =
        child.future_rows as isize - package_matching_future_rows as isize;
    let rows_lost_by_score_filter =
        package_matching_future_rows as isize - package_scoreable_future_rows as isize;
    let rows_lost_by_margin =
        package_matching_future_rows as isize - runtime_shadow_accept_rows as isize;
    let rows_lost_by_exact_cache_or_dedupe_after_accept =
        runtime_shadow_accept_rows as isize - runtime_unique_accepts_over_exact_cache as isize;
    let unique_accept_gap = child.marginal_accepts_over_exact_cache as isize
        - runtime_unique_accepts_over_exact_cache as isize;
    let tokens_gap = child.tokens_saved as isize - runtime_tokens_saved as isize;
    let exact_cache_policy_mismatch_suspected =
        runtime_exact_cache_hits_reported > child.cache_overlap.saturating_mul(2).max(1);
    let margin_threshold_pressure_suspected = package.is_some()
        && threshold_micro > median_margin_micro
        && runtime_shadow_accept_rows < child.marginal_accepts_over_exact_cache;
    let top_margin_failure_reasons = margin_failure_reasons(
        threshold_micro,
        p10_margin_micro,
        median_margin_micro,
        runtime_wrong_wins,
        margin_threshold_pressure_suspected,
        exact_cache_policy_mismatch_suspected,
    );
    let primary_loss_reason = primary_loss_reason(
        package.is_some(),
        rows_lost_before_package,
        rows_lost_by_margin,
        rows_lost_by_exact_cache_or_dedupe_after_accept,
        exact_cache_policy_mismatch_suspected,
        margin_threshold_pressure_suspected,
        &rejection_reason,
    );
    let runtime_survived = replay.is_some()
        && runtime_false_accepts == 0
        && runtime_unique_accepts_over_exact_cache > 0;
    let survival_status = survival_status(
        runtime_survived,
        package.is_some(),
        replay.is_some(),
        runtime_unique_accepts_over_exact_cache,
        runtime_false_accepts,
        runtime_wrong_wins,
        margin_threshold_pressure_suspected,
        &rejection_reason,
    );

    SelectedSplitLossRow {
        broad_class_id: child.broad_class_id.clone(),
        split_rule: child.split_rule.clone(),
        split_depth,
        required_atoms,
        selected_support: child.support,
        selected_verifier_ready_rows: child.verifier_ready_rows,
        selected_future_rows: child.future_rows,
        selected_cache_overlap: child.cache_overlap,
        selected_marginal_accepts_over_exact_cache: child.marginal_accepts_over_exact_cache,
        selected_tokens_saved: child.tokens_saved,
        selected_false_accepts: child.false_accepts,
        package_present: package.is_some(),
        replay_present: replay.is_some(),
        package_training_rows,
        package_train_positive_rows,
        package_train_negative_rows,
        package_future_rows,
        package_scoreable_future_rows,
        package_matching_future_rows,
        runtime_shadow_accept_rows,
        runtime_unique_accepts_over_exact_cache,
        runtime_tokens_saved,
        runtime_false_accepts,
        runtime_exact_cache_hits_reported,
        runtime_wrong_wins,
        threshold_micro,
        train_max_false_margin_micro,
        train_min_true_margin_micro,
        p10_margin_micro,
        median_margin_micro,
        rejection_reason,
        rows_lost_before_package,
        rows_lost_by_score_filter,
        rows_lost_by_margin,
        rows_lost_by_exact_cache_or_dedupe_after_accept,
        unique_accept_gap,
        tokens_gap,
        exact_cache_policy_mismatch_suspected,
        margin_threshold_pressure_suspected,
        top_missing_atom_keys: Vec::new(),
        top_margin_failure_reasons,
        primary_loss_reason,
        runtime_survived,
        survival_status,
    }
}

fn atom_survival_summary(splits: &[SelectedSplitLossRow]) -> Vec<AtomSurvivalRow> {
    let mut by_atom = BTreeMap::<String, AtomSurvivalRow>::new();
    for split in splits {
        for atom in &split.required_atoms {
            let row = by_atom
                .entry(atom.clone())
                .or_insert_with(|| AtomSurvivalRow {
                    atom: atom.clone(),
                    ..AtomSurvivalRow::default()
                });
            row.split_count += 1;
            row.survived_split_count += usize::from(split.runtime_survived);
            row.failed_split_count += usize::from(!split.runtime_survived);
            row.selected_marginal_accepts_over_exact_cache = row
                .selected_marginal_accepts_over_exact_cache
                .saturating_add(split.selected_marginal_accepts_over_exact_cache);
            row.runtime_unique_accepts_over_exact_cache = row
                .runtime_unique_accepts_over_exact_cache
                .saturating_add(split.runtime_unique_accepts_over_exact_cache);
            row.selected_tokens_saved = row
                .selected_tokens_saved
                .saturating_add(split.selected_tokens_saved);
            row.runtime_tokens_saved = row
                .runtime_tokens_saved
                .saturating_add(split.runtime_tokens_saved);
            row.false_accept_split_count += usize::from(
                split.runtime_false_accepts > 0
                    || split.selected_false_accepts > 0
                    || split.rejection_reason == "future_false_accepts",
            );
            row.margin_pressure_split_count +=
                usize::from(split.margin_threshold_pressure_suspected);
            row.no_runtime_value_split_count +=
                usize::from(split.runtime_unique_accepts_over_exact_cache == 0);
        }
    }
    let mut rows = by_atom.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .runtime_unique_accepts_over_exact_cache
            .cmp(&left.runtime_unique_accepts_over_exact_cache)
            .then_with(|| right.survived_split_count.cmp(&left.survived_split_count))
            .then_with(|| {
                right
                    .selected_marginal_accepts_over_exact_cache
                    .cmp(&left.selected_marginal_accepts_over_exact_cache)
            })
            .then_with(|| left.atom.cmp(&right.atom))
    });
    rows.truncate(64);
    rows
}

fn split_rule_required_atoms(split_rule: &str) -> Vec<String> {
    let rest = split_rule
        .strip_prefix("pair::")
        .or_else(|| split_rule.strip_prefix("all::"))
        .unwrap_or(split_rule);
    rest.split(" && ").map(ToOwned::to_owned).collect()
}

fn survival_status(
    runtime_survived: bool,
    package_present: bool,
    replay_present: bool,
    runtime_unique_accepts_over_exact_cache: usize,
    runtime_false_accepts: usize,
    runtime_wrong_wins: usize,
    margin_threshold_pressure_suspected: bool,
    rejection_reason: &str,
) -> String {
    if runtime_survived {
        "clean_runtime_survived".to_owned()
    } else if runtime_false_accepts > 0 || rejection_reason == "future_false_accepts" {
        "blocked_by_false_accepts".to_owned()
    } else if !package_present {
        "package_missing".to_owned()
    } else if !replay_present {
        format!("not_promoted:{rejection_reason}")
    } else if runtime_wrong_wins > 0 {
        "runtime_wrong_wins".to_owned()
    } else if margin_threshold_pressure_suspected {
        "margin_threshold_pressure".to_owned()
    } else if runtime_unique_accepts_over_exact_cache == 0 {
        "no_runtime_unique_accepts".to_owned()
    } else {
        rejection_reason.to_owned()
    }
}

fn margin_failure_reasons(
    threshold_micro: i64,
    p10_margin_micro: i64,
    median_margin_micro: i64,
    runtime_wrong_wins: usize,
    margin_threshold_pressure_suspected: bool,
    exact_cache_policy_mismatch_suspected: bool,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if margin_threshold_pressure_suspected {
        reasons.push(format!(
            "threshold_above_median_margin:{threshold_micro}>{median_margin_micro}"
        ));
    }
    if p10_margin_micro < 0 {
        reasons.push(format!("negative_p10_margin:{p10_margin_micro}"));
    }
    if runtime_wrong_wins > 0 {
        reasons.push(format!("runtime_wrong_wins:{runtime_wrong_wins}"));
    }
    if exact_cache_policy_mismatch_suspected {
        reasons.push("runtime_exact_cache_hits_far_above_split_cache_overlap".to_owned());
    }
    if reasons.is_empty() {
        reasons.push("no_aggregate_margin_reason_detected".to_owned());
    }
    reasons
}

fn primary_loss_reason(
    package_present: bool,
    rows_lost_before_package: isize,
    rows_lost_by_margin: isize,
    rows_lost_by_exact_cache_or_dedupe_after_accept: isize,
    exact_cache_policy_mismatch_suspected: bool,
    margin_threshold_pressure_suspected: bool,
    rejection_reason: &str,
) -> String {
    if !package_present {
        "package_missing".to_owned()
    } else if rows_lost_before_package > 0 {
        "rows_lost_before_package".to_owned()
    } else if rows_lost_by_margin > rows_lost_by_exact_cache_or_dedupe_after_accept
        && margin_threshold_pressure_suspected
    {
        "margin_threshold_pressure".to_owned()
    } else if exact_cache_policy_mismatch_suspected {
        "exact_cache_policy_mismatch".to_owned()
    } else if rows_lost_by_margin > 0 {
        "margin_or_vector_separation_loss".to_owned()
    } else if rows_lost_by_exact_cache_or_dedupe_after_accept > 0 {
        "exact_cache_or_dedupe_after_accept".to_owned()
    } else {
        rejection_reason.to_owned()
    }
}

fn split_key(broad_class_id: &str, split_rule: &str) -> String {
    format!("{broad_class_id}\n{split_rule}")
}

fn next_path<I>(args: &mut I, default_path: &str) -> PathBuf
where
    I: Iterator<Item = String>,
{
    args.next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_path))
}

fn json_usize(value: &Value, path: &[&str]) -> usize {
    json_u64(value, path)
        .and_then(|number| usize::try_from(number).ok())
        .unwrap_or(0)
}

fn json_i64(value: &Value, path: &[&str]) -> i64 {
    json_at_local(value, path)
        .and_then(Value::as_i64)
        .or_else(|| {
            json_at_local(value, path)
                .and_then(|value| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        })
        .unwrap_or(0)
}

fn json_i64_option(value: &Value, path: &[&str]) -> Option<i64> {
    json_at_local(value, path)
        .and_then(Value::as_i64)
        .or_else(|| {
            json_at_local(value, path)
                .and_then(|value| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        })
}

fn per_thousand_local(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn json_at_local<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}
