use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use nando_core::{
    PhaseCenterCell, PhaseCenterCompiler, PhaseCenterFlatRuntime, PhaseCenterOffloadPolicy,
    PhaseCenterOffloadRuntime, phase_vector_from_atom_ids,
};
use serde::Serialize;
use serde_json::Value;

use super::{
    generic_count_band, json_bool, json_string, margin_to_micro, percentile_i64,
    phase_atom_action_families, phase_atom_binary_token_cost, phase_atom_string_vec,
    read_json_value, sanitize_file_stem, stable_fingerprint, write_binary_file, write_json_file,
};

const DEFAULT_SELECTED_SPLIT_NWPC_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-quarantine-v1.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_PACKAGE_DIR: &str =
    "target/nando-wave/streaming/selected-split-nwpc-quarantine-v1";
const DEFAULT_AUTO_MULTI_SPLIT_TOP_K: usize = 16;

#[derive(Clone)]
struct SplitPackageRow {
    stream_index: usize,
    broad_class_id: String,
    request_fingerprint: String,
    exact_cache_hit: bool,
    verified_safe_accept: bool,
    total_tokens: usize,
    total_cost_microusd: u64,
    atoms: Vec<String>,
    atom_ids: Vec<u64>,
}

#[derive(Clone)]
struct SelectedSplitSpec {
    broad_class_id: String,
    split_rule: String,
}

#[derive(Serialize)]
struct SelectedSplitPackageReport {
    broad_class_id: String,
    split_rule: String,
    task_name: String,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    broad_class_rows: usize,
    verifier_ready_rows: usize,
    train_rows: usize,
    train_positive_rows: usize,
    train_negative_rows: usize,
    future_rows: usize,
    future_scored_rows: usize,
    future_matching_split_rows: usize,
    threshold_micro: i64,
    train_max_false_margin_micro: Option<i64>,
    train_min_true_margin_micro: Option<i64>,
    runtime_margin_parity_mismatches: usize,
    future_shadow_accepts: usize,
    future_unique_accepts_over_exact_cache: usize,
    future_tokens_saved: usize,
    future_cost_saved_microusd: u64,
    future_false_accepts: usize,
    future_wrong_wins: usize,
    future_exact_cache_hits: usize,
    min_margin_micro: i64,
    p10_margin_micro: i64,
    median_margin_micro: i64,
    accepted_for_shadow_review: bool,
    rejection_reason: String,
}

struct PreparedSplitRow<'a> {
    row: &'a SplitPackageRow,
    matches_split: bool,
    safe_vec: Vec<PhaseCenterCell>,
    reject_vec: Vec<PhaseCenterCell>,
}

#[derive(Serialize)]
struct SelectedSplitNwpcReport {
    report_kind: &'static str,
    selected_split_report_path: String,
    input_paths: Vec<String>,
    package_dir: String,
    train_future_split_mode: &'static str,
    selected_split_source_mode: &'static str,
    auto_multi_split_top_k: usize,
    cells: usize,
    total_rows: usize,
    verifier_ready_rows: usize,
    selected_split_count: usize,
    compiled_package_count: usize,
    accepted_package_count: usize,
    future_unique_accepts_over_exact_cache: usize,
    future_tokens_saved: usize,
    future_cost_saved_microusd: u64,
    future_false_accepts: usize,
    accepted_future_unique_accepts_over_exact_cache: usize,
    accepted_future_tokens_saved: usize,
    accepted_future_cost_saved_microusd: u64,
    runtime_margin_parity_mismatches: usize,
    packages: Vec<SelectedSplitPackageReport>,
    forbidden_flags: BTreeMap<&'static str, bool>,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    market_money_claim_allowed: bool,
    verdict: &'static str,
    boundary: &'static str,
}

pub(crate) fn run_phase_stream_selected_split_nwpc_quarantine_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_REPORT));
    let package_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_PACKAGE_DIR));
    let cells = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid cells value '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(32);
    let Some(split_report_path) = args.next().map(PathBuf::from) else {
        return Err("selected split report JSON path is required".to_owned());
    };
    let mut remaining_args = args.collect::<Vec<_>>();
    let mut hash_train_future = false;
    let mut auto_multi_split = false;
    let mut input_args = Vec::new();
    for arg in remaining_args.drain(..) {
        match arg.as_str() {
            "--hash-train-future" => hash_train_future = true,
            "--auto-multi-split" => auto_multi_split = true,
            _ => input_args.push(arg),
        }
    }
    let input_paths = input_args
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let seed_selected_splits = read_selected_splits(&split_report_path)?;
    let selected_classes = seed_selected_splits
        .iter()
        .map(|split| split.broad_class_id.clone())
        .collect::<BTreeSet<_>>();
    let rows = read_split_package_rows(&input_paths, &selected_classes)?;
    let verifier_ready_rows = rows.len();
    let mut by_class = BTreeMap::<String, Vec<SplitPackageRow>>::new();
    for row in rows.iter().cloned() {
        by_class
            .entry(row.broad_class_id.clone())
            .or_default()
            .push(row);
    }
    let selected_splits = if auto_multi_split {
        derive_auto_multi_split_specs(
            &seed_selected_splits,
            &by_class,
            DEFAULT_AUTO_MULTI_SPLIT_TOP_K,
            hash_train_future,
        )
    } else {
        seed_selected_splits
    };

    let mut packages = Vec::new();
    for (index, split) in selected_splits.iter().enumerate() {
        if selected_splits.len() > 8
            && (index == 0 || (index + 1) % 4 == 0 || index + 1 == selected_splits.len())
        {
            println!(
                "  compiling_selected_split_package: {}/{}",
                index + 1,
                selected_splits.len()
            );
        }
        let class_rows = by_class
            .get(&split.broad_class_id)
            .cloned()
            .unwrap_or_default();
        packages.push(compile_selected_split_package(
            split,
            class_rows,
            cells,
            &package_dir,
            hash_train_future,
        )?);
    }

    let compiled_package_count = packages
        .iter()
        .filter(|package| package.package_bytes > 0)
        .count();
    let accepted_package_count = packages
        .iter()
        .filter(|package| package.accepted_for_shadow_review)
        .count();
    let future_unique_accepts_over_exact_cache = packages
        .iter()
        .map(|package| package.future_unique_accepts_over_exact_cache)
        .sum();
    let future_tokens_saved = packages
        .iter()
        .map(|package| package.future_tokens_saved)
        .sum();
    let future_cost_saved_microusd = packages
        .iter()
        .map(|package| package.future_cost_saved_microusd)
        .sum();
    let future_false_accepts = packages
        .iter()
        .map(|package| package.future_false_accepts)
        .sum();
    let accepted_future_unique_accepts_over_exact_cache = packages
        .iter()
        .filter(|package| package.accepted_for_shadow_review)
        .map(|package| package.future_unique_accepts_over_exact_cache)
        .sum();
    let accepted_future_tokens_saved = packages
        .iter()
        .filter(|package| package.accepted_for_shadow_review)
        .map(|package| package.future_tokens_saved)
        .sum();
    let accepted_future_cost_saved_microusd = packages
        .iter()
        .filter(|package| package.accepted_for_shadow_review)
        .map(|package| package.future_cost_saved_microusd)
        .sum();
    let runtime_margin_parity_mismatches = packages
        .iter()
        .map(|package| package.runtime_margin_parity_mismatches)
        .sum();
    let verdict = if future_false_accepts > 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_QUARANTINE_V1_FAIL_FALSE_ACCEPTS"
    } else if accepted_package_count > 0 && runtime_margin_parity_mismatches == 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_QUARANTINE_V1_PASS_SHADOW_READY"
    } else if compiled_package_count > 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_QUARANTINE_V1_WATCH_COMPILED_NOT_ACCEPTED"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_QUARANTINE_V1_WATCH_NO_PACKAGE"
    };
    let report = SelectedSplitNwpcReport {
        report_kind: "phase_stream_selected_split_nwpc_quarantine_v1",
        selected_split_report_path: split_report_path.display().to_string(),
        input_paths: input_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        package_dir: package_dir.display().to_string(),
        train_future_split_mode: if hash_train_future {
            "hash_request_fingerprint_v2"
        } else {
            "source_order_first_half_v1"
        },
        selected_split_source_mode: if auto_multi_split {
            "auto_train_window_observable_atom_multi_split_v2"
        } else {
            "selected_children_from_split_report_v1"
        },
        auto_multi_split_top_k: if auto_multi_split {
            DEFAULT_AUTO_MULTI_SPLIT_TOP_K
        } else {
            0
        },
        cells,
        total_rows: rows.len(),
        verifier_ready_rows,
        selected_split_count: selected_splits.len(),
        compiled_package_count,
        accepted_package_count,
        future_unique_accepts_over_exact_cache,
        future_tokens_saved,
        future_cost_saved_microusd,
        future_false_accepts,
        accepted_future_unique_accepts_over_exact_cache,
        accepted_future_tokens_saved,
        accepted_future_cost_saved_microusd,
        runtime_margin_parity_mismatches,
        packages,
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
        local_accept_enabled: false,
        auto_promote_enabled: false,
        market_money_claim_allowed: false,
        verdict,
        boundary: "selected split quarantine only: consumes automatic constrained split report, compiles verifier-bound .nwpc packages, shadow-scores future rows, and never promotes, serves, enables local_accept, claims market money, or uses legacy nwrb/role-binding paths",
    };
    write_json_file(&report_path, &report)?;

    println!("phase_stream_selected_split_nwpc_quarantine_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  package_dir: {}", package_dir.display());
    println!("  selected_split_count: {}", report.selected_split_count);
    println!(
        "  selected_split_source_mode: {}",
        report.selected_split_source_mode
    );
    println!(
        "  auto_multi_split_top_k: {}",
        report.auto_multi_split_top_k
    );
    println!("  compiled_package_count: {compiled_package_count}");
    println!("  accepted_package_count: {accepted_package_count}");
    println!("  future_unique_accepts_over_exact_cache: {future_unique_accepts_over_exact_cache}");
    println!("  future_tokens_saved: {future_tokens_saved}");
    println!(
        "  accepted_future_unique_accepts_over_exact_cache: {accepted_future_unique_accepts_over_exact_cache}"
    );
    println!("  accepted_future_tokens_saved: {accepted_future_tokens_saved}");
    println!("  future_false_accepts: {future_false_accepts}");
    println!("  runtime_margin_parity_mismatches: {runtime_margin_parity_mismatches}");
    println!("  local_accept_enabled: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_selected_splits(path: &Path) -> Result<Vec<SelectedSplitSpec>, String> {
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
            if let Some(split_rule) = json_string(child, &["split_rule"]) {
                splits.push(SelectedSplitSpec {
                    broad_class_id: broad_class_id.clone(),
                    split_rule,
                });
            }
        }
    }
    Ok(splits)
}

fn derive_auto_multi_split_specs(
    seed_splits: &[SelectedSplitSpec],
    by_class: &BTreeMap<String, Vec<SplitPackageRow>>,
    top_k: usize,
    hash_train_future: bool,
) -> Vec<SelectedSplitSpec> {
    let mut splits = Vec::new();
    let mut seen = BTreeSet::<(String, String)>::new();
    for split in seed_splits {
        let key = (split.broad_class_id.clone(), split.split_rule.clone());
        if seen.insert(key) {
            splits.push(split.clone());
        }
    }
    let selected_classes = seed_splits
        .iter()
        .map(|split| split.broad_class_id.clone())
        .collect::<BTreeSet<_>>();
    for broad_class_id in selected_classes {
        let Some(class_rows) = by_class.get(&broad_class_id) else {
            continue;
        };
        let mut sorted_rows = class_rows.clone();
        sorted_rows.sort_by(|left, right| left.stream_index.cmp(&right.stream_index));
        let (train_rows, _) = train_future_split(&sorted_rows, hash_train_future);
        let mut atom_stats = BTreeMap::<String, AutoMultiAtomStats>::new();
        for row in train_rows {
            for atom in &row.atoms {
                if selected_split_auto_multi_atom_allowed(atom) {
                    let stats = atom_stats.entry(atom.clone()).or_default();
                    stats.train_rows = stats.train_rows.saturating_add(1);
                    if row.verified_safe_accept {
                        stats.train_positive_rows = stats.train_positive_rows.saturating_add(1);
                    } else {
                        stats.train_false_rows = stats.train_false_rows.saturating_add(1);
                    }
                }
            }
        }
        let mut ranked_atoms = atom_stats.into_iter().collect::<Vec<_>>();
        ranked_atoms.sort_by(|(left_atom, left_stats), (right_atom, right_stats)| {
            right_stats
                .train_positive_rows
                .cmp(&left_stats.train_positive_rows)
                .then_with(|| {
                    left_stats
                        .train_false_rows
                        .cmp(&right_stats.train_false_rows)
                })
                .then_with(|| right_stats.train_rows.cmp(&left_stats.train_rows))
                .then_with(|| Reverse(left_atom).cmp(&Reverse(right_atom)))
        });
        let mut emitted_ranked_atoms = 0usize;
        for (atom, stats) in ranked_atoms {
            if stats.train_rows < 2 || stats.train_positive_rows == 0 {
                continue;
            }
            let is_derived = atom.starts_with("derived::");
            if emitted_ranked_atoms >= top_k && !is_derived {
                continue;
            }
            if !is_derived {
                emitted_ranked_atoms = emitted_ranked_atoms.saturating_add(1);
            }
            let split_rule = format!("all::{atom}");
            let key = (broad_class_id.clone(), split_rule.clone());
            if seen.insert(key) {
                splits.push(SelectedSplitSpec {
                    broad_class_id: broad_class_id.clone(),
                    split_rule,
                });
            }
        }
    }
    splits
}

#[derive(Default)]
struct AutoMultiAtomStats {
    train_rows: usize,
    train_positive_rows: usize,
    train_false_rows: usize,
}

fn read_split_package_rows(
    paths: &[PathBuf],
    selected_classes: &BTreeSet<String>,
) -> Result<Vec<SplitPackageRow>, String> {
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
                    "failed to parse selected split trace '{}' line {}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
            let Some(verified_safe_accept) = json_bool(&row, &["verified_safe_accept"]) else {
                continue;
            };
            let action_families =
                phase_atom_action_families(&phase_atom_string_vec(&row, "action_atoms"));
            if action_families.is_empty() {
                continue;
            }
            let selected_action_families = action_families
                .into_iter()
                .filter(|broad_class_id| selected_classes.contains(broad_class_id))
                .collect::<Vec<_>>();
            let request_fingerprint = json_string(&row, &["request_fingerprint"])
                .unwrap_or_else(|| format!("selected-split-row:{}", rows.len() + 1));
            let exact_cache_key = json_string(&row, &["exact_cache_key"])
                .unwrap_or_else(|| request_fingerprint.clone());
            let exact_cache_hit = !seen_exact_cache.insert(exact_cache_key.clone());
            if selected_action_families.is_empty() {
                continue;
            }
            let token_cost = phase_atom_binary_token_cost(&row);
            let atoms = selected_split_atoms(
                &row,
                token_cost.total_tokens,
                token_cost.total_cost_microusd,
            );
            if atoms.is_empty() {
                continue;
            }
            let atom_ids = selected_split_base_atom_ids(&atoms);
            for broad_class_id in selected_action_families {
                rows.push(SplitPackageRow {
                    stream_index: rows.len() + 1,
                    broad_class_id,
                    request_fingerprint: request_fingerprint.clone(),
                    exact_cache_hit,
                    verified_safe_accept,
                    total_tokens: token_cost.total_tokens,
                    total_cost_microusd: token_cost.total_cost_microusd,
                    atoms: atoms.clone(),
                    atom_ids: atom_ids.clone(),
                });
            }
        }
    }
    Ok(rows)
}

fn compile_selected_split_package(
    split: &SelectedSplitSpec,
    mut rows: Vec<SplitPackageRow>,
    cells: usize,
    package_dir: &Path,
    hash_train_future: bool,
) -> Result<SelectedSplitPackageReport, String> {
    rows.sort_by(|left, right| left.stream_index.cmp(&right.stream_index));
    let broad_class_rows = rows.len();
    let verifier_ready_rows = rows.len();
    let task_name = selected_split_task_name(split);
    let package_path = package_dir.join(format!("{task_name}.candidate.nwpc"));
    if rows.len() < 4 {
        return Ok(empty_selected_split_package_report(
            split,
            task_name,
            package_path,
            broad_class_rows,
            verifier_ready_rows,
            "fewer_than_4_verifier_ready_rows",
        ));
    }
    let (train_rows, future_rows) = train_future_split(&rows, hash_train_future);
    let train_positive_rows = train_rows
        .iter()
        .filter(|row| row.verified_safe_accept && row_matches_split(row, &split.split_rule))
        .count();
    let train_negative_rows = train_rows
        .iter()
        .filter(|row| !row.verified_safe_accept || !row_matches_split(row, &split.split_rule))
        .count();
    if train_positive_rows == 0 || train_negative_rows == 0 {
        return Ok(empty_selected_split_package_report(
            split,
            task_name,
            package_path,
            broad_class_rows,
            verifier_ready_rows,
            "train_window_missing_positive_or_negative",
        ));
    }
    let train_scored_rows = train_rows
        .iter()
        .map(|row| {
            prepare_selected_split_row(
                row,
                row_matches_split(row, &split.split_rule),
                cells,
                &task_name,
            )
        })
        .collect::<Vec<_>>();

    let mut compiler = PhaseCenterCompiler::new(cells, 1)
        .map_err(|error| format!("selected split compiler error: {error:?}"))?;
    for prepared in &train_scored_rows {
        if prepared.row.verified_safe_accept && prepared.matches_split {
            compiler
                .add_positive_vector(0, &prepared.safe_vec)
                .map_err(|error| format!("selected split positive update error: {error:?}"))?;
        } else {
            compiler
                .add_negative_vector(0, &prepared.reject_vec)
                .map_err(|error| format!("selected split negative update error: {error:?}"))?;
        }
    }
    let reference_runtime = compiler
        .compile()
        .map_err(|error| format!("selected split compile error: {error:?}"))?;
    let package_bytes = reference_runtime
        .to_bytes()
        .map_err(|error| format!("selected split package serialization error: {error:?}"))?;
    write_binary_file(&package_path, &package_bytes)?;
    let read_package = std::fs::read(&package_path).map_err(|error| {
        format!(
            "failed to read selected split package '{}': {error}",
            package_path.display()
        )
    })?;
    if read_package != package_bytes {
        return Err(format!(
            "selected split package '{}' readback mismatch",
            package_path.display()
        ));
    }
    let package_info = PhaseCenterOffloadRuntime::inspect_package_bytes(&read_package)
        .map_err(|error| format!("selected split package inspect error: {error:?}"))?;
    let offload_runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &read_package,
        PhaseCenterOffloadPolicy::new(1)
            .map_err(|error| format!("selected split policy error: {error:?}"))?,
    )
    .map_err(|error| format!("selected split package load error: {error:?}"))?;

    let mut runtime_margin_parity_mismatches = 0usize;
    let mut train_true_margins = Vec::new();
    let mut train_false_margins = Vec::new();
    for prepared in &train_scored_rows {
        let (margin_micro, parity_mismatch) = selected_split_safe_accept_margin_from_vectors(
            &reference_runtime,
            &offload_runtime,
            &prepared.safe_vec,
            &prepared.reject_vec,
        )?;
        runtime_margin_parity_mismatches += usize::from(parity_mismatch);
        if prepared.row.verified_safe_accept && prepared.matches_split {
            train_true_margins.push(margin_micro);
        } else {
            train_false_margins.push(margin_micro);
        }
    }
    let train_max_false_margin_micro = train_false_margins.iter().max().copied();
    let train_min_true_margin_micro = train_true_margins.iter().min().copied();
    let threshold_micro = train_max_false_margin_micro
        .map_or(0, |margin| margin.saturating_add(1))
        .max(1);

    let mut future_shadow_accepts = 0usize;
    let mut future_unique_accepts_over_exact_cache = 0usize;
    let mut future_tokens_saved = 0usize;
    let mut future_cost_saved_microusd = 0u64;
    let mut future_false_accepts = 0usize;
    let mut future_wrong_wins = 0usize;
    let mut future_exact_cache_hits = 0usize;
    let mut future_matching_split_rows = 0usize;
    let mut margins = Vec::new();
    let mut accepted_fingerprints = BTreeSet::new();
    for row in &future_rows {
        let matches_split = row_matches_split(row, &split.split_rule);
        future_matching_split_rows += usize::from(matches_split);
        future_exact_cache_hits += usize::from(row.exact_cache_hit);
    }
    let future_scored_rows = future_rows
        .iter()
        .map(|row| {
            let matches_split = row_matches_split(row, &split.split_rule);
            prepare_selected_split_row(row, matches_split, cells, &task_name)
        })
        .collect::<Vec<_>>();
    for prepared in &future_scored_rows {
        let row = prepared.row;
        let (margin_micro, parity_mismatch) = selected_split_safe_accept_margin_from_vectors(
            &reference_runtime,
            &offload_runtime,
            &prepared.safe_vec,
            &prepared.reject_vec,
        )?;
        runtime_margin_parity_mismatches += usize::from(parity_mismatch);
        let signed_margin = if row.verified_safe_accept && prepared.matches_split {
            margin_micro
        } else {
            margin_micro.saturating_neg()
        };
        margins.push(signed_margin);
        future_wrong_wins += usize::from(signed_margin <= 0);
        let shadow_accept = margin_micro >= threshold_micro;
        future_shadow_accepts += usize::from(shadow_accept);
        if shadow_accept && !(row.verified_safe_accept && prepared.matches_split) {
            future_false_accepts += 1;
        }
        if shadow_accept
            && row.verified_safe_accept
            && prepared.matches_split
            && !row.exact_cache_hit
            && accepted_fingerprints.insert(row.request_fingerprint.clone())
        {
            future_unique_accepts_over_exact_cache += 1;
            future_tokens_saved = future_tokens_saved.saturating_add(row.total_tokens);
            future_cost_saved_microusd =
                future_cost_saved_microusd.saturating_add(row.total_cost_microusd);
        }
    }
    margins.sort_unstable();
    let accepted_for_shadow_review = !future_rows.is_empty()
        && future_unique_accepts_over_exact_cache > 0
        && future_false_accepts == 0
        && runtime_margin_parity_mismatches == 0
        && package_info.record_count == 1
        && package_info.serialized_len == read_package.len();
    let rejection_reason = if accepted_for_shadow_review {
        "accepted_for_shadow_review".to_owned()
    } else if future_rows.is_empty() {
        "empty_future_window".to_owned()
    } else if future_unique_accepts_over_exact_cache == 0 {
        "no_future_unique_accepts_over_exact_cache".to_owned()
    } else if future_false_accepts > 0 {
        "future_false_accepts".to_owned()
    } else if runtime_margin_parity_mismatches > 0 {
        "runtime_margin_parity_mismatches".to_owned()
    } else {
        "package_shape_mismatch".to_owned()
    };

    Ok(SelectedSplitPackageReport {
        broad_class_id: split.broad_class_id.clone(),
        split_rule: split.split_rule.clone(),
        task_name,
        package_path: package_path.display().to_string(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: read_package.len(),
        package_records: package_info.record_count,
        broad_class_rows,
        verifier_ready_rows,
        train_rows: train_rows.len(),
        train_positive_rows,
        train_negative_rows,
        future_rows: future_rows.len(),
        future_scored_rows: future_scored_rows.len(),
        future_matching_split_rows,
        threshold_micro,
        train_max_false_margin_micro,
        train_min_true_margin_micro,
        runtime_margin_parity_mismatches,
        future_shadow_accepts,
        future_unique_accepts_over_exact_cache,
        future_tokens_saved,
        future_cost_saved_microusd,
        future_false_accepts,
        future_wrong_wins,
        future_exact_cache_hits,
        min_margin_micro: margins.first().copied().unwrap_or(0),
        p10_margin_micro: percentile_i64(&margins, 10),
        median_margin_micro: percentile_i64(&margins, 50),
        accepted_for_shadow_review,
        rejection_reason,
    })
}

fn train_future_split(
    rows: &[SplitPackageRow],
    hash_train_future: bool,
) -> (Vec<&SplitPackageRow>, Vec<&SplitPackageRow>) {
    let mut train_rows = Vec::new();
    let mut future_rows = Vec::new();
    if hash_train_future {
        for row in rows {
            let route = stable_fingerprint([
                "selected_split_nwpc_train_future_v2",
                row.request_fingerprint.as_str(),
            ]);
            if route & 1 == 0 {
                train_rows.push(row);
            } else {
                future_rows.push(row);
            }
        }
    }
    if !hash_train_future || train_rows.is_empty() || future_rows.is_empty() {
        train_rows.clear();
        future_rows.clear();
        let train_len = (rows.len() / 2).clamp(1, rows.len().saturating_sub(1));
        for (index, row) in rows.iter().enumerate() {
            if index < train_len {
                train_rows.push(row);
            } else {
                future_rows.push(row);
            }
        }
    }
    (train_rows, future_rows)
}

fn empty_selected_split_package_report(
    split: &SelectedSplitSpec,
    task_name: String,
    package_path: PathBuf,
    broad_class_rows: usize,
    verifier_ready_rows: usize,
    reason: &str,
) -> SelectedSplitPackageReport {
    SelectedSplitPackageReport {
        broad_class_id: split.broad_class_id.clone(),
        split_rule: split.split_rule.clone(),
        task_name,
        package_path: package_path.display().to_string(),
        package_fingerprint64: 0,
        package_bytes: 0,
        package_records: 0,
        broad_class_rows,
        verifier_ready_rows,
        train_rows: 0,
        train_positive_rows: 0,
        train_negative_rows: 0,
        future_rows: 0,
        future_scored_rows: 0,
        future_matching_split_rows: 0,
        threshold_micro: 0,
        train_max_false_margin_micro: None,
        train_min_true_margin_micro: None,
        runtime_margin_parity_mismatches: 0,
        future_shadow_accepts: 0,
        future_unique_accepts_over_exact_cache: 0,
        future_tokens_saved: 0,
        future_cost_saved_microusd: 0,
        future_false_accepts: 0,
        future_wrong_wins: 0,
        future_exact_cache_hits: 0,
        min_margin_micro: 0,
        p10_margin_micro: 0,
        median_margin_micro: 0,
        accepted_for_shadow_review: false,
        rejection_reason: reason.to_owned(),
    }
}

fn prepare_selected_split_row<'a>(
    row: &'a SplitPackageRow,
    matches_split: bool,
    cells: usize,
    task_name: &str,
) -> PreparedSplitRow<'a> {
    PreparedSplitRow {
        row,
        matches_split,
        safe_vec: selected_split_event_vector(row, true, cells, task_name),
        reject_vec: selected_split_event_vector(row, false, cells, task_name),
    }
}

fn selected_split_safe_accept_margin_from_vectors(
    reference_runtime: &PhaseCenterFlatRuntime,
    offload_runtime: &PhaseCenterOffloadRuntime,
    safe_vec: &[PhaseCenterCell],
    reject_vec: &[PhaseCenterCell],
) -> Result<(i64, bool), String> {
    let reference_margin = reference_runtime
        .margin_for(0, safe_vec, reject_vec)
        .map_err(|error| format!("selected split reference margin error: {error:?}"))?;
    let package_margin = offload_runtime
        .runtime()
        .margin_for(0, safe_vec, reject_vec)
        .map_err(|error| format!("selected split package margin error: {error:?}"))?;
    let reference_micro = margin_to_micro(reference_margin)?;
    Ok((
        reference_micro,
        margin_to_micro(package_margin)? != reference_micro,
    ))
}

fn selected_split_event_vector(
    row: &SplitPackageRow,
    candidate_safe_accept: bool,
    cells: usize,
    task_name: &str,
) -> Vec<nando_core::PhaseCenterCell> {
    let mut atom_ids = row.atom_ids.clone();
    atom_ids.push(stable_fingerprint([format!(
        "phase_atom_binary_task:{task_name}_verifier_bound"
    )
    .as_str()]));
    atom_ids.push(stable_fingerprint([format!(
        "candidate_result_label:{candidate_safe_accept}"
    )
    .as_str()]));
    atom_ids.push(stable_fingerprint([format!(
        "candidate_verified_safe_accept:{candidate_safe_accept}"
    )
    .as_str()]));
    phase_vector_from_atom_ids(atom_ids, cells)
}

fn selected_split_base_atom_ids(atoms: &[String]) -> Vec<u64> {
    atoms
        .iter()
        .map(String::as_str)
        .filter(|atom| !atom.starts_with("phase_atom_binary_task:"))
        .map(|atom| stable_fingerprint([atom]))
        .collect()
}

fn row_matches_split(row: &SplitPackageRow, split_rule: &str) -> bool {
    split_rule_required_atoms(split_rule)
        .iter()
        .all(|required| row.atoms.iter().any(|atom| atom == required))
}

fn split_rule_required_atoms(split_rule: &str) -> Vec<&str> {
    let rest = split_rule
        .strip_prefix("pair::")
        .or_else(|| split_rule.strip_prefix("all::"))
        .unwrap_or(split_rule);
    rest.split(" && ").collect()
}

fn selected_split_atoms(row: &Value, total_tokens: usize, total_cost_microusd: u64) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    let action_atoms = phase_atom_string_vec(row, "action_atoms");
    let state_atoms = phase_atom_string_vec(row, "state_atoms");
    let tool_atoms = phase_atom_string_vec(row, "tool_atoms");
    let route_atoms = phase_atom_string_vec(row, "route_hint_atoms");
    for (group, key) in [
        ("request", "request_atoms"),
        ("state", "state_atoms"),
        ("action", "action_atoms"),
        ("tool", "tool_atoms"),
        ("result", "result_atoms"),
        ("route", "route_hint_atoms"),
    ] {
        for atom in phase_atom_string_vec(row, key) {
            if selected_split_atom_allowed(&atom) {
                atoms.insert(format!("{group}::{atom}"));
            }
        }
    }
    for atom in selected_split_planning_transition_atoms(
        &action_atoms,
        &state_atoms,
        &tool_atoms,
        &route_atoms,
    ) {
        atoms.insert(format!("derived::{atom}"));
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

fn selected_split_planning_transition_atoms(
    action_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    route_atoms: &[String],
) -> Vec<&'static str> {
    if !action_atoms
        .iter()
        .any(|atom| atom == "action_family:planning")
    {
        return Vec::new();
    }
    let update_plan_signal = action_atoms
        .iter()
        .any(|atom| atom == "action:update_plan_state")
        || state_atoms
            .iter()
            .any(|atom| atom == "state_source:codex_session_update_plan")
        || tool_atoms
            .iter()
            .any(|atom| atom == "tool_name:update_plan")
        || route_atoms
            .iter()
            .any(|atom| atom == "route_hint:planning_update");
    if update_plan_signal {
        vec![
            "planning_transition:update_plan_state",
            "planning_transition_source:update_plan_tool",
        ]
    } else {
        vec![
            "planning_transition:non_update_plan_state",
            "planning_transition_source:not_update_plan_tool",
        ]
    }
}

fn selected_split_atom_allowed(atom: &str) -> bool {
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

fn selected_split_auto_multi_atom_allowed(atom: &str) -> bool {
    !atom.starts_with("context::")
        && !atom.starts_with("token::")
        && !atom.starts_with("cost::")
        && !atom.contains("phase_atom_binary_task:")
}

fn selected_split_task_name(split: &SelectedSplitSpec) -> String {
    let stem = sanitize_file_stem(&format!(
        "{}-{}",
        split.broad_class_id.replace("action_family:", ""),
        split.split_rule
    ));
    let short = stem.chars().take(72).collect::<String>();
    let fingerprint =
        stable_fingerprint([split.broad_class_id.as_str(), split.split_rule.as_str()]);
    format!("{short}-{fingerprint:016x}")
}
