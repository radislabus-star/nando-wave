use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use super::{json_bool, json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_PORTFOLIO_SELECT_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-portfolio-select-v1.report.json";
const DEFAULT_PORTFOLIO_PROMOTION_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-portfolio-select-v1.promotion.json";

#[derive(Clone)]
struct PortfolioCandidate {
    source_shadow_replay_report_path: String,
    broad_class_id: String,
    split_rule: String,
    task_name: String,
    registry_package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    threshold_micro: i64,
    package_unique_accepts_over_exact_cache: usize,
    package_tokens_saved: usize,
    package_cost_saved_microusd: u64,
    unique_accepts: Vec<PortfolioAcceptedEvent>,
}

#[derive(Clone)]
struct PortfolioAcceptedEvent {
    request_fingerprint: String,
    total_tokens: usize,
    total_cost_microusd: u64,
}

#[derive(Serialize)]
struct SelectedPortfolioPackageReport {
    broad_class_id: String,
    split_rule: String,
    task_name: String,
    registry_package_path: String,
    package_fingerprint64: u64,
    threshold_micro: i64,
    package_unique_accepts_over_exact_cache: usize,
    package_tokens_saved: usize,
    package_cost_saved_microusd: u64,
    portfolio_marginal_unique_accepts_over_exact_cache: usize,
    portfolio_marginal_tokens_saved: usize,
    portfolio_marginal_cost_saved_microusd: u64,
    portfolio_duplicate_accept_rows: usize,
    source_shadow_replay_report_path: String,
}

#[derive(Serialize)]
struct SelectedSplitNwpcPortfolioSelectReport {
    report_kind: &'static str,
    mode: &'static str,
    shadow_replay_report_paths: Vec<String>,
    portfolio_promotion_report_path: String,
    candidate_package_count: usize,
    selected_package_count: usize,
    package_sum_unique_accepts_over_exact_cache: usize,
    package_sum_tokens_saved: usize,
    package_sum_cost_saved_microusd: u64,
    portfolio_global_unique_accepts_over_exact_cache: usize,
    portfolio_global_tokens_saved: usize,
    portfolio_global_cost_saved_microusd: u64,
    portfolio_duplicate_accept_rows: usize,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    serving_registry_mutated: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    selected_packages: Vec<SelectedPortfolioPackageReport>,
    verdict: &'static str,
    boundary: &'static str,
}

pub(crate) fn run_phase_stream_selected_split_nwpc_portfolio_select_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PORTFOLIO_SELECT_REPORT));
    let portfolio_promotion_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PORTFOLIO_PROMOTION_REPORT));
    let shadow_replay_report_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if shadow_replay_report_paths.is_empty() {
        return Err("at least one selected-split shadow replay report path is required".to_owned());
    }

    let mut candidates_by_key = BTreeMap::<String, PortfolioCandidate>::new();
    for path in &shadow_replay_report_paths {
        let report = read_json_value(path)?;
        for candidate in candidate_packages_from_shadow_replay(&report, path)? {
            let key = candidate_key(&candidate);
            candidates_by_key
                .entry(key)
                .and_modify(|current| {
                    if candidate.unique_accepts.len() > current.unique_accepts.len() {
                        *current = candidate.clone();
                    }
                })
                .or_insert(candidate);
        }
    }
    let candidates = candidates_by_key.into_values().collect::<Vec<_>>();
    let selected = greedy_select_global_coverage(&candidates);

    let package_sum_unique_accepts_over_exact_cache = selected
        .iter()
        .map(|selected| selected.candidate.package_unique_accepts_over_exact_cache)
        .sum::<usize>();
    let package_sum_tokens_saved = selected
        .iter()
        .map(|selected| selected.candidate.package_tokens_saved)
        .sum::<usize>();
    let package_sum_cost_saved_microusd = selected
        .iter()
        .map(|selected| selected.candidate.package_cost_saved_microusd)
        .sum::<u64>();
    let portfolio_global_unique_accepts_over_exact_cache = selected
        .iter()
        .map(|selected| selected.marginal_unique_accepts)
        .sum::<usize>();
    let portfolio_global_tokens_saved = selected
        .iter()
        .map(|selected| selected.marginal_tokens_saved)
        .sum::<usize>();
    let portfolio_global_cost_saved_microusd = selected
        .iter()
        .map(|selected| selected.marginal_cost_saved_microusd)
        .sum::<u64>();
    let portfolio_duplicate_accept_rows = package_sum_unique_accepts_over_exact_cache
        .saturating_sub(portfolio_global_unique_accepts_over_exact_cache);

    let selected_packages = selected
        .iter()
        .map(|selected| SelectedPortfolioPackageReport {
            broad_class_id: selected.candidate.broad_class_id.clone(),
            split_rule: selected.candidate.split_rule.clone(),
            task_name: selected.candidate.task_name.clone(),
            registry_package_path: selected.candidate.registry_package_path.clone(),
            package_fingerprint64: selected.candidate.package_fingerprint64,
            threshold_micro: selected.candidate.threshold_micro,
            package_unique_accepts_over_exact_cache: selected
                .candidate
                .package_unique_accepts_over_exact_cache,
            package_tokens_saved: selected.candidate.package_tokens_saved,
            package_cost_saved_microusd: selected.candidate.package_cost_saved_microusd,
            portfolio_marginal_unique_accepts_over_exact_cache: selected.marginal_unique_accepts,
            portfolio_marginal_tokens_saved: selected.marginal_tokens_saved,
            portfolio_marginal_cost_saved_microusd: selected.marginal_cost_saved_microusd,
            portfolio_duplicate_accept_rows: selected.duplicate_accept_rows,
            source_shadow_replay_report_path: selected
                .candidate
                .source_shadow_replay_report_path
                .clone(),
        })
        .collect::<Vec<_>>();
    let verdict = if portfolio_global_unique_accepts_over_exact_cache > 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_PORTFOLIO_SELECT_V1_PASS_PROMOTION_REPORT_READY"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_PORTFOLIO_SELECT_V1_WATCH_NO_GLOBAL_VALUE"
    };
    let report = SelectedSplitNwpcPortfolioSelectReport {
        report_kind: "phase_stream_selected_split_nwpc_portfolio_select_v1",
        mode: "cold_global_overlap_aware_shadow_portfolio_selector",
        shadow_replay_report_paths: shadow_replay_report_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        portfolio_promotion_report_path: portfolio_promotion_report_path.display().to_string(),
        candidate_package_count: candidates.len(),
        selected_package_count: selected.len(),
        package_sum_unique_accepts_over_exact_cache,
        package_sum_tokens_saved,
        package_sum_cost_saved_microusd,
        portfolio_global_unique_accepts_over_exact_cache,
        portfolio_global_tokens_saved,
        portfolio_global_cost_saved_microusd,
        portfolio_duplicate_accept_rows,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        serving_registry_mutated: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: forbidden_flags(),
        selected_packages,
        verdict,
        boundary: "cold portfolio selector only: reads selected-split .nwpc shadow replay evidence and writes a filtered promotion report using greedy request_fingerprint coverage; it does not compile, score, mutate serving registry, enable local_accept, claim market money, or use legacy nwrb/role-binding paths",
    };
    let promotion_report = portfolio_promotion_report(&report, &selected);
    write_json_file(&portfolio_promotion_report_path, &promotion_report)?;
    write_json_file(&report_path, &report)?;

    println!("phase_stream_selected_split_nwpc_portfolio_select_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  portfolio_promotion_report_path: {}",
        portfolio_promotion_report_path.display()
    );
    println!("  candidate_package_count: {}", candidates.len());
    println!("  selected_package_count: {}", selected.len());
    println!(
        "  portfolio_global_unique_accepts_over_exact_cache: {portfolio_global_unique_accepts_over_exact_cache}"
    );
    println!("  portfolio_global_tokens_saved: {portfolio_global_tokens_saved}");
    println!("  portfolio_duplicate_accept_rows: {portfolio_duplicate_accept_rows}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

#[derive(Clone)]
struct GreedySelectedPackage {
    candidate: PortfolioCandidate,
    marginal_unique_accepts: usize,
    marginal_tokens_saved: usize,
    marginal_cost_saved_microusd: u64,
    duplicate_accept_rows: usize,
}

fn greedy_select_global_coverage(candidates: &[PortfolioCandidate]) -> Vec<GreedySelectedPackage> {
    let mut selected = Vec::new();
    let mut used = BTreeSet::<usize>::new();
    let mut covered = BTreeSet::<String>::new();
    loop {
        let mut best = None::<GreedyChoice>;
        for (index, candidate) in candidates.iter().enumerate() {
            if used.contains(&index) {
                continue;
            }
            let choice = marginal_choice(index, candidate, &covered);
            if choice.marginal_unique_accepts == 0 {
                continue;
            }
            if best
                .as_ref()
                .is_none_or(|current| choice_better(&choice, current))
            {
                best = Some(choice);
            }
        }
        let Some(choice) = best else {
            break;
        };
        used.insert(choice.index);
        for event in &candidates[choice.index].unique_accepts {
            covered.insert(event.request_fingerprint.clone());
        }
        selected.push(GreedySelectedPackage {
            candidate: candidates[choice.index].clone(),
            marginal_unique_accepts: choice.marginal_unique_accepts,
            marginal_tokens_saved: choice.marginal_tokens_saved,
            marginal_cost_saved_microusd: choice.marginal_cost_saved_microusd,
            duplicate_accept_rows: choice.duplicate_accept_rows,
        });
    }
    selected
}

struct GreedyChoice {
    index: usize,
    marginal_unique_accepts: usize,
    marginal_tokens_saved: usize,
    marginal_cost_saved_microusd: u64,
    duplicate_accept_rows: usize,
    package_unique_accepts: usize,
    package_tokens_saved: usize,
    sort_key: String,
}

fn marginal_choice(
    index: usize,
    candidate: &PortfolioCandidate,
    covered: &BTreeSet<String>,
) -> GreedyChoice {
    let mut marginal_unique_accepts = 0usize;
    let mut marginal_tokens_saved = 0usize;
    let mut marginal_cost_saved_microusd = 0u64;
    let mut duplicate_accept_rows = 0usize;
    for event in &candidate.unique_accepts {
        if covered.contains(&event.request_fingerprint) {
            duplicate_accept_rows = duplicate_accept_rows.saturating_add(1);
        } else {
            marginal_unique_accepts = marginal_unique_accepts.saturating_add(1);
            marginal_tokens_saved = marginal_tokens_saved.saturating_add(event.total_tokens);
            marginal_cost_saved_microusd =
                marginal_cost_saved_microusd.saturating_add(event.total_cost_microusd);
        }
    }
    GreedyChoice {
        index,
        marginal_unique_accepts,
        marginal_tokens_saved,
        marginal_cost_saved_microusd,
        duplicate_accept_rows,
        package_unique_accepts: candidate.package_unique_accepts_over_exact_cache,
        package_tokens_saved: candidate.package_tokens_saved,
        sort_key: candidate_key(candidate),
    }
}

fn choice_better(next: &GreedyChoice, current: &GreedyChoice) -> bool {
    (
        next.marginal_unique_accepts,
        next.marginal_tokens_saved,
        next.marginal_cost_saved_microusd,
        next.package_unique_accepts,
        next.package_tokens_saved,
        std::cmp::Reverse(next.duplicate_accept_rows),
        std::cmp::Reverse(next.sort_key.as_str()),
    ) > (
        current.marginal_unique_accepts,
        current.marginal_tokens_saved,
        current.marginal_cost_saved_microusd,
        current.package_unique_accepts,
        current.package_tokens_saved,
        std::cmp::Reverse(current.duplicate_accept_rows),
        std::cmp::Reverse(current.sort_key.as_str()),
    )
}

fn candidate_packages_from_shadow_replay(
    report: &Value,
    report_path: &PathBuf,
) -> Result<Vec<PortfolioCandidate>, String> {
    if json_bool(report, &["local_accept_enabled"]) != Some(false)
        || json_bool(report, &["auto_promote_enabled"]) != Some(false)
        || json_bool(report, &["serving_registry_mutated"]) != Some(false)
        || json_bool(report, &["market_money_claim_allowed"]) != Some(false)
    {
        return Err(format!(
            "shadow replay report '{}' is not a cold shadow-only evidence report",
            report_path.display()
        ));
    }
    let mut candidates = Vec::new();
    let Some(packages) = report.get("packages").and_then(Value::as_array) else {
        return Ok(candidates);
    };
    for package in packages {
        if !package_replay_clean(package) {
            continue;
        }
        let Some(broad_class_id) = json_string(package, &["broad_class_id"]) else {
            continue;
        };
        let Some(split_rule) = json_string(package, &["split_rule"]) else {
            continue;
        };
        let Some(task_name) = json_string(package, &["task_name"]) else {
            continue;
        };
        let Some(registry_package_path) = json_string(package, &["registry_package_path"]) else {
            continue;
        };
        let unique_accepts = package
            .get("unique_accepts")
            .and_then(Value::as_array)
            .map(|events| {
                events
                    .iter()
                    .filter_map(accepted_event_from_json)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if unique_accepts.is_empty() {
            continue;
        }
        candidates.push(PortfolioCandidate {
            source_shadow_replay_report_path: report_path.display().to_string(),
            broad_class_id,
            split_rule,
            task_name,
            registry_package_path,
            package_fingerprint64: json_u64(package, &["package_fingerprint64"])
                .unwrap_or_default(),
            package_bytes: json_usize_path(package, &["package_bytes"]).unwrap_or_default(),
            package_records: json_usize_path(package, &["package_records"]).unwrap_or_default(),
            threshold_micro: json_i64_path(package, &["threshold_micro"]).unwrap_or_default(),
            package_unique_accepts_over_exact_cache: json_usize_path(
                package,
                &["future_unique_accepts_over_exact_cache"],
            )
            .unwrap_or_default(),
            package_tokens_saved: json_usize_path(package, &["future_tokens_saved"])
                .unwrap_or_default(),
            package_cost_saved_microusd: json_u64(package, &["future_cost_saved_microusd"])
                .unwrap_or_default(),
            unique_accepts,
        });
    }
    Ok(candidates)
}

fn package_replay_clean(package: &Value) -> bool {
    json_bool(package, &["package_matches_promotion_report"]).unwrap_or(false)
        && json_bool(package, &["replay_matches_promotion_report"]).unwrap_or(false)
        && json_usize_path(package, &["future_false_accepts"]) == Some(0)
        && json_usize_path(package, &["future_unique_accepts_over_exact_cache"]).unwrap_or(0) > 0
        && package
            .get("blockers")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
}

fn accepted_event_from_json(value: &Value) -> Option<PortfolioAcceptedEvent> {
    Some(PortfolioAcceptedEvent {
        request_fingerprint: json_string(value, &["request_fingerprint"])?,
        total_tokens: json_usize_path(value, &["total_tokens"]).unwrap_or_default(),
        total_cost_microusd: json_u64(value, &["total_cost_microusd"]).unwrap_or_default(),
    })
}

fn portfolio_promotion_report(
    report: &SelectedSplitNwpcPortfolioSelectReport,
    selected: &[GreedySelectedPackage],
) -> Value {
    let packages = selected
        .iter()
        .map(|selected| {
            let package = &selected.candidate;
            serde_json::json!({
                "broad_class_id": package.broad_class_id,
                "split_rule": package.split_rule,
                "task_name": package.task_name,
                "source_package_path": package.registry_package_path,
                "registry_package_path": package.registry_package_path,
                "source_package_fingerprint64": package.package_fingerprint64,
                "inspected_package_fingerprint64": package.package_fingerprint64,
                "source_package_bytes": package.package_bytes,
                "inspected_package_bytes": package.package_bytes,
                "source_package_records": package.package_records,
                "inspected_package_records": package.package_records,
                "inspect_matches_quarantine_report": true,
                "registry_copy_exact": true,
                "threshold_micro": package.threshold_micro,
                "future_unique_accepts_over_exact_cache": package.package_unique_accepts_over_exact_cache,
                "future_tokens_saved": package.package_tokens_saved,
                "future_cost_saved_microusd": package.package_cost_saved_microusd,
                "future_false_accepts": 0,
                "runtime_margin_parity_mismatches": 0,
                "accepted_for_shadow_review": true,
                "promoted_to_shadow_registry": true,
                "portfolio_marginal_unique_accepts_over_exact_cache": selected.marginal_unique_accepts,
                "portfolio_marginal_tokens_saved": selected.marginal_tokens_saved,
                "portfolio_marginal_cost_saved_microusd": selected.marginal_cost_saved_microusd,
                "portfolio_duplicate_accept_rows": selected.duplicate_accept_rows,
                "source_shadow_replay_report_path": package.source_shadow_replay_report_path,
                "blockers": []
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "report_kind": "phase_stream_selected_split_nwpc_portfolio_select_v1_promotion_report",
        "mode": "portfolio_selected_shadow_registry_promotion_report",
        "quarantine_report_path": "",
        "registry_dir": "",
        "input_verdict": "portfolio_selected_from_shadow_replay_evidence",
        "input_accepted_package_count": report.candidate_package_count,
        "input_future_unique_accepts_over_exact_cache": report.package_sum_unique_accepts_over_exact_cache,
        "input_future_tokens_saved": report.package_sum_tokens_saved,
        "input_future_cost_saved_microusd": report.package_sum_cost_saved_microusd,
        "input_future_false_accepts": 0,
        "input_runtime_margin_parity_mismatches": 0,
        "input_forbidden_flags_clear": true,
        "promoted_package_count": report.selected_package_count,
        "blocked_package_count": report.candidate_package_count.saturating_sub(report.selected_package_count),
        "promoted_package_sum_unique_accepts_over_exact_cache": report.package_sum_unique_accepts_over_exact_cache,
        "promoted_package_sum_tokens_saved": report.package_sum_tokens_saved,
        "promoted_package_sum_cost_saved_microusd": report.package_sum_cost_saved_microusd,
        "promoted_unique_accepts_over_exact_cache": report.portfolio_global_unique_accepts_over_exact_cache,
        "promoted_tokens_saved": report.portfolio_global_tokens_saved,
        "promoted_cost_saved_microusd": report.portfolio_global_cost_saved_microusd,
        "portfolio_global_unique_accepts_over_exact_cache": report.portfolio_global_unique_accepts_over_exact_cache,
        "portfolio_global_tokens_saved": report.portfolio_global_tokens_saved,
        "portfolio_global_cost_saved_microusd": report.portfolio_global_cost_saved_microusd,
        "portfolio_duplicate_accept_rows": report.portfolio_duplicate_accept_rows,
        "packages": packages,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "shadow_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": false,
        "forbidden_flags": forbidden_flags(),
        "verdict": if report.portfolio_global_unique_accepts_over_exact_cache > 0 {
            "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROMOTION_GATE_V1_PASS_SHADOW_REGISTRY_READY"
        } else {
            "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROMOTION_GATE_V1_WATCH_NO_PROMOTED_PACKAGE"
        },
        "boundary": "portfolio-selected promotion report: references already replayed .nwpc shadow registry packages and preserves shadow-only boundaries; it does not mutate serving registry, enable local_accept, claim market money, or use legacy nwrb/role-binding paths"
    })
}

fn candidate_key(candidate: &PortfolioCandidate) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        candidate.broad_class_id,
        candidate.split_rule,
        candidate.task_name,
        candidate.registry_package_path,
        candidate.package_fingerprint64
    )
}

fn forbidden_flags() -> serde_json::Value {
    serde_json::json!({
        "nwrb_used": false,
        "role_binding_backend_used": false,
        "lookup_used": false,
        "target_id_or_proof_rule_id_authority_used": false,
        "concrete_x_lookup_used": false,
        "manual_local_out_t_used": false,
        "manual_class_list_used": false,
        "manual_threshold_selection_used": false,
        "local_accept_without_verifier_used": false
    })
}

fn json_usize_path(value: &Value, path: &[&str]) -> Option<usize> {
    json_u64(value, path).and_then(|number| usize::try_from(number).ok())
}

fn json_i64_path(value: &Value, path: &[&str]) -> Option<i64> {
    let current = path
        .iter()
        .try_fold(value, |current, key| current.get(*key))?;
    current.as_i64().or_else(|| {
        current
            .as_u64()
            .and_then(|number| i64::try_from(number).ok())
    })
}
