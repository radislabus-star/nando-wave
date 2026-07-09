use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use super::{json_bool, json_string, read_json_value, write_json_file};

const DEFAULT_STAGE_FILTER_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-stage-filter-v1.report.json";
const DEFAULT_FILTERED_SPLIT_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-stage-filter-v1.selected.json";
const DEFAULT_SELECTED_SPLIT_REPORT: &str = "target/nando-wave/streaming/phase-stream-automatic-continuation-split-v1-realtrace-plus-verifier-sources-survival-merged.report.json";
const DEFAULT_QUARANTINE_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-quarantine-v1-survival-merged.report.json";

#[derive(Serialize)]
struct StageFilterReport {
    report_kind: &'static str,
    selected_split_report_path: String,
    quarantine_report_path: String,
    quarantine_report_paths: Vec<String>,
    filtered_split_report_path: String,
    source_selected_split_count: usize,
    stage_accepted_package_count: usize,
    filtered_selected_split_count: usize,
    source_selected_pre_runtime_accepts: usize,
    filtered_pre_runtime_accepts: usize,
    stage_runtime_unique_accepts_over_exact_cache: usize,
    stage_runtime_tokens_saved: usize,
    stage_future_false_accepts_excluded: usize,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    serving_registry_mutated: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: serde_json::Value,
    verdict: &'static str,
    boundary: &'static str,
}

pub(crate) fn run_phase_stream_selected_split_nwpc_stage_filter_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STAGE_FILTER_REPORT));
    let filtered_split_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_FILTERED_SPLIT_REPORT));
    let selected_split_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_REPORT));
    let first_quarantine_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_QUARANTINE_REPORT));
    let mut quarantine_report_paths = vec![first_quarantine_report_path];
    quarantine_report_paths.extend(args.map(PathBuf::from));

    let selected_report = read_json_value(&selected_split_report_path)?;
    let quarantine_reports = quarantine_report_paths
        .iter()
        .map(|path| read_json_value(path))
        .collect::<Result<Vec<_>, _>>()?;
    let stage_packages = accepted_stage_packages(&quarantine_reports);
    let stage_keys = stage_packages.keys().cloned().collect::<BTreeSet<_>>();
    let stage_metrics = accepted_stage_metrics(&quarantine_reports, &stage_packages);
    let source_selected_split_count = count_selected_children(&selected_report);
    let source_selected_pre_runtime_accepts = selected_pre_runtime_accepts(&selected_report);

    let mut filtered_report = selected_report.clone();
    let filtered_selected_split_count = filter_selected_children(&mut filtered_report, &stage_keys);
    let filtered_pre_runtime_accepts = selected_pre_runtime_accepts(&filtered_report);
    stamp_filtered_report(
        &mut filtered_report,
        &selected_split_report_path,
        &quarantine_report_paths,
    );
    write_json_file(&filtered_split_report_path, &filtered_report)?;

    let verdict = if filtered_selected_split_count == 0 || stage_metrics.runtime_unique_accepts == 0
    {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_STAGE_FILTER_V1_WATCH_NO_STAGE_SURVIVORS"
    } else if stage_metrics.future_false_accepts_excluded > 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_STAGE_FILTER_V1_PASS_FILTERED_WITH_UNSAFE_EXCLUDED"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_STAGE_FILTER_V1_PASS_FILTERED"
    };
    let report = StageFilterReport {
        report_kind: "phase_stream_selected_split_nwpc_stage_filter_v1",
        selected_split_report_path: selected_split_report_path.display().to_string(),
        quarantine_report_path: quarantine_report_paths
            .first()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        quarantine_report_paths: quarantine_report_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        filtered_split_report_path: filtered_split_report_path.display().to_string(),
        source_selected_split_count,
        stage_accepted_package_count: stage_keys.len(),
        filtered_selected_split_count,
        source_selected_pre_runtime_accepts,
        filtered_pre_runtime_accepts,
        stage_runtime_unique_accepts_over_exact_cache: stage_metrics.runtime_unique_accepts,
        stage_runtime_tokens_saved: stage_metrics.runtime_tokens_saved,
        stage_future_false_accepts_excluded: stage_metrics.future_false_accepts_excluded,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        serving_registry_mutated: false,
        market_money_claim_allowed: false,
        forbidden_flags: serde_json::json!({
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        }),
        verdict,
        boundary: "cold stage filter only: copies automatic selected split report and keeps only package-level .nwpc survivors from a quarantine evidence report; it does not compile, promote, serve, enable local_accept, claim market money, or use legacy nwrb",
    };
    write_json_file(&report_path, &report)?;

    println!("phase_stream_selected_split_nwpc_stage_filter_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  filtered_split_report_path: {}",
        filtered_split_report_path.display()
    );
    println!("  source_selected_split_count: {source_selected_split_count}");
    println!(
        "  filtered_selected_split_count: {}",
        report.filtered_selected_split_count
    );
    println!(
        "  stage_runtime_unique_accepts_over_exact_cache: {}",
        report.stage_runtime_unique_accepts_over_exact_cache
    );
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

#[derive(Default)]
struct StageMetrics {
    runtime_unique_accepts: usize,
    runtime_tokens_saved: usize,
    future_false_accepts_excluded: usize,
}

#[derive(Clone, Copy, Default)]
struct StagePackageMetric {
    runtime_unique_accepts: usize,
    runtime_tokens_saved: usize,
}

fn accepted_stage_packages(reports: &[Value]) -> BTreeMap<String, StagePackageMetric> {
    let mut packages_by_key = BTreeMap::new();
    for report in reports {
        let Some(packages) = report.get("packages").and_then(Value::as_array) else {
            continue;
        };
        for package in packages {
            if package_stage_accepted(package) {
                let Some(broad_class_id) = json_string(package, &["broad_class_id"]) else {
                    continue;
                };
                let Some(split_rule) = json_string(package, &["split_rule"]) else {
                    continue;
                };
                let key = stage_key(&broad_class_id, &split_rule);
                let next = StagePackageMetric {
                    runtime_unique_accepts: json_usize_local(
                        package,
                        &["future_unique_accepts_over_exact_cache"],
                    ),
                    runtime_tokens_saved: json_usize_local(package, &["future_tokens_saved"]),
                };
                packages_by_key
                    .entry(key)
                    .and_modify(|current: &mut StagePackageMetric| {
                        if next.runtime_unique_accepts > current.runtime_unique_accepts {
                            *current = next;
                        }
                    })
                    .or_insert(next);
            }
        }
    }
    packages_by_key
}

fn accepted_stage_metrics(
    reports: &[Value],
    accepted_packages: &BTreeMap<String, StagePackageMetric>,
) -> StageMetrics {
    let mut metrics = StageMetrics::default();
    for accepted in accepted_packages.values() {
        metrics.runtime_unique_accepts = metrics
            .runtime_unique_accepts
            .saturating_add(accepted.runtime_unique_accepts);
        metrics.runtime_tokens_saved = metrics
            .runtime_tokens_saved
            .saturating_add(accepted.runtime_tokens_saved);
    }
    for report in reports {
        let Some(packages) = report.get("packages").and_then(Value::as_array) else {
            continue;
        };
        for package in packages {
            let Some(broad_class_id) = json_string(package, &["broad_class_id"]) else {
                continue;
            };
            let Some(split_rule) = json_string(package, &["split_rule"]) else {
                continue;
            };
            let key = stage_key(&broad_class_id, &split_rule);
            if !accepted_packages.contains_key(&key) {
                metrics.future_false_accepts_excluded = metrics
                    .future_false_accepts_excluded
                    .saturating_add(json_usize_local(package, &["future_false_accepts"]));
            }
        }
    }
    metrics
}

fn package_stage_accepted(package: &Value) -> bool {
    json_bool(package, &["accepted_for_shadow_review"]).unwrap_or(false)
        && json_usize_local(package, &["future_false_accepts"]) == 0
        && json_usize_local(package, &["runtime_margin_parity_mismatches"]) == 0
        && json_usize_local(package, &["future_unique_accepts_over_exact_cache"]) > 0
}

fn filter_selected_children(report: &mut Value, keep_keys: &BTreeSet<String>) -> usize {
    let Some(classes) = report.get_mut("classes").and_then(Value::as_array_mut) else {
        return 0;
    };
    let mut total_selected = 0usize;
    for class in classes {
        let broad_class_id = json_string(class, &["broad_class_id"]).unwrap_or_default();
        let Some(children) = class
            .get_mut("selected_children")
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        children.retain(|child| {
            json_string(child, &["split_rule"])
                .map(|split_rule| keep_keys.contains(&stage_key(&broad_class_id, &split_rule)))
                .unwrap_or(false)
        });
        total_selected = total_selected.saturating_add(children.len());
        update_class_selected_sums(class);
    }
    update_global_selected_sums(report);
    total_selected
}

fn update_class_selected_sums(class: &mut Value) {
    let children = class
        .get("selected_children")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let selected_count = children.len();
    let accepts = children
        .iter()
        .map(|child| json_usize_local(child, &["marginal_accepts_over_exact_cache"]))
        .sum::<usize>();
    let tokens = children
        .iter()
        .map(|child| json_usize_local(child, &["tokens_saved"]))
        .sum::<usize>();
    let cost = children
        .iter()
        .map(|child| json_u64_local(child, &["cost_saved_microusd"]).unwrap_or(0))
        .sum::<u64>();
    let false_accepts = children
        .iter()
        .map(|child| json_usize_local(child, &["false_accepts"]))
        .sum::<usize>();
    set_usize(class, "selected_split_count", selected_count);
    set_usize(
        class,
        "selected_future_unique_accepts_over_exact_cache",
        accepts,
    );
    set_usize(class, "selected_future_tokens_saved", tokens);
    set_u64(class, "selected_future_cost_saved_microusd", cost);
    set_usize(class, "selected_future_false_accepts", false_accepts);
}

fn update_global_selected_sums(report: &mut Value) {
    let Some(classes) = report.get("classes").and_then(Value::as_array) else {
        return;
    };
    let selected_count = classes
        .iter()
        .map(|class| json_usize_local(class, &["selected_split_count"]))
        .sum::<usize>();
    let accepts = classes
        .iter()
        .map(|class| json_usize_local(class, &["selected_future_unique_accepts_over_exact_cache"]))
        .sum::<usize>();
    let tokens = classes
        .iter()
        .map(|class| json_usize_local(class, &["selected_future_tokens_saved"]))
        .sum::<usize>();
    let cost = classes
        .iter()
        .map(|class| json_u64_local(class, &["selected_future_cost_saved_microusd"]).unwrap_or(0))
        .sum::<u64>();
    let false_accepts = classes
        .iter()
        .map(|class| json_usize_local(class, &["selected_future_false_accepts"]))
        .sum::<usize>();
    set_usize(report, "selected_cpu_subsplits", selected_count);
    if let Some(global_delta) = report
        .get_mut("global_delta")
        .and_then(Value::as_object_mut)
    {
        global_delta.insert("after_accepts".to_owned(), serde_json::json!(accepts));
        global_delta.insert("after_tokens".to_owned(), serde_json::json!(tokens));
        global_delta.insert("after_cost_microusd".to_owned(), serde_json::json!(cost));
        global_delta.insert("false_accepts".to_owned(), serde_json::json!(false_accepts));
    }
}

fn stamp_filtered_report(
    report: &mut Value,
    selected_split_report_path: &Path,
    quarantine_report_paths: &[PathBuf],
) {
    if let Some(object) = report.as_object_mut() {
        object.insert(
            "report_kind".to_owned(),
            serde_json::json!("phase_stream_selected_split_nwpc_stage_filter_v1_filtered_split"),
        );
        object.insert(
            "stage_filter".to_owned(),
            serde_json::json!({
                "source_selected_split_report_path": selected_split_report_path.display().to_string(),
                "quarantine_report_paths": quarantine_report_paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>(),
                "policy": "keep only package-level .nwpc survivors with accepted_for_shadow_review=true, future_false_accepts=0, runtime parity=0, and future_unique_accepts>0",
                "local_accept_enabled": false,
                "market_money_claim_allowed": false
            }),
        );
    }
}

fn count_selected_children(report: &Value) -> usize {
    report
        .get("classes")
        .and_then(Value::as_array)
        .map(|classes| {
            classes
                .iter()
                .map(|class| {
                    class
                        .get("selected_children")
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len)
                })
                .sum()
        })
        .unwrap_or(0)
}

fn selected_pre_runtime_accepts(report: &Value) -> usize {
    report
        .get("classes")
        .and_then(Value::as_array)
        .map(|classes| {
            classes
                .iter()
                .flat_map(|class| {
                    class
                        .get("selected_children")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .map(|child| json_usize_local(child, &["marginal_accepts_over_exact_cache"]))
                .sum()
        })
        .unwrap_or(0)
}

fn stage_key(broad_class_id: &str, split_rule: &str) -> String {
    format!("{broad_class_id}\n{split_rule}")
}

fn set_usize(value: &mut Value, key: &str, number: usize) {
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_owned(), serde_json::json!(number));
    }
}

fn set_u64(value: &mut Value, key: &str, number: u64) {
    if let Some(object) = value.as_object_mut() {
        object.insert(key.to_owned(), serde_json::json!(number));
    }
}

fn json_usize_local(value: &Value, path: &[&str]) -> usize {
    json_u64_local(value, path)
        .and_then(|number| usize::try_from(number).ok())
        .unwrap_or(0)
}

fn json_u64_local(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_u64()
}
