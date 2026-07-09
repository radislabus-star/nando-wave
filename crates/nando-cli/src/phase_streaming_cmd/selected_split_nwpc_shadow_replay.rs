use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use nando_core::{PhaseCenterCell, PhaseCenterOffloadRuntime, phase_vector_from_atom_ids};
use serde::Serialize;
use serde_json::Value;

use super::{
    generic_count_band, json_bool, json_string, json_u64, margin_to_micro,
    phase_atom_action_families, phase_atom_binary_token_cost, phase_atom_string_vec,
    read_json_value, stable_fingerprint, write_json_file,
};

const DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT: &str =
    "target/nando-wave/streaming/phase-stream-selected-split-nwpc-shadow-replay-v1.report.json";
const DEFAULT_SELECTED_SPLIT_NWPC_PROMOTION_GATE_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-promotion-gate-v1-realtrace-plus-verifier-sources.report.json";

#[derive(Clone)]
struct ReplayRow {
    stream_index: usize,
    broad_class_id: String,
    request_fingerprint: String,
    exact_cache_key: String,
    exact_cache_hit: bool,
    verified_safe_accept: bool,
    total_tokens: usize,
    total_cost_microusd: u64,
    atoms: Vec<String>,
}

#[derive(Clone, Serialize)]
struct ShadowReplayAcceptedEventReport {
    request_fingerprint: String,
    exact_cache_key: String,
    stream_index: usize,
    broad_class_id: String,
    task_name: String,
    split_rule: String,
    package_fingerprint64: u64,
    margin_micro: i64,
    threshold_micro: i64,
    total_tokens: usize,
    total_cost_microusd: u64,
    token_evidence_missing: bool,
    provider_billing_evidence_present: bool,
    unique_cpu_accept_over_exact_cache: bool,
    verified_safe_accept: bool,
    false_accept: bool,
}

#[derive(Clone)]
struct PromotedPackageSpec {
    broad_class_id: String,
    split_rule: String,
    task_name: String,
    registry_package_path: String,
    expected_package_fingerprint64: u64,
    expected_package_bytes: usize,
    expected_package_records: usize,
    threshold_micro: i64,
    expected_unique_accepts_over_exact_cache: usize,
    expected_tokens_saved: usize,
    expected_cost_saved_microusd: u64,
    decision_count_parity_required: bool,
}

#[derive(Serialize)]
struct ShadowReplayPackageReport {
    broad_class_id: String,
    split_rule: String,
    task_name: String,
    registry_package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    package_records: usize,
    package_matches_promotion_report: bool,
    decision_count_parity_required: bool,
    expected_unique_accepts_over_exact_cache: usize,
    expected_tokens_saved: usize,
    expected_cost_saved_microusd: u64,
    threshold_micro: i64,
    broad_class_rows: usize,
    train_rows: usize,
    future_rows: usize,
    future_scored_rows: usize,
    future_matching_split_rows: usize,
    future_shadow_accepts: usize,
    future_unique_accepts_over_exact_cache: usize,
    future_tokens_saved: usize,
    future_cost_saved_microusd: u64,
    future_false_accepts: usize,
    future_exact_cache_hits: usize,
    unique_accepts: Vec<ShadowReplayAcceptedEventReport>,
    false_accept_examples: Vec<ShadowReplayAcceptedEventReport>,
    replay_matches_promotion_report: bool,
    blockers: Vec<String>,
}

#[derive(Serialize)]
struct SelectedSplitNwpcShadowReplayReport {
    report_kind: &'static str,
    mode: &'static str,
    promotion_report_path: String,
    input_paths: Vec<String>,
    train_future_split_mode: &'static str,
    total_rows: usize,
    promoted_package_count: usize,
    replayed_package_count: usize,
    clean_package_count: usize,
    future_shadow_accepts: usize,
    package_sum_unique_accepts_over_exact_cache: usize,
    package_sum_tokens_saved: usize,
    package_sum_cost_saved_microusd: u64,
    future_unique_accepts_over_exact_cache: usize,
    future_tokens_saved: usize,
    future_cost_saved_microusd: u64,
    global_duplicate_accept_rows: usize,
    future_false_accepts: usize,
    replay_mismatch_count: usize,
    local_accept_enabled: bool,
    auto_promote_enabled: bool,
    serving_registry_mutated: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    token_cost_estimate_used: bool,
    provider_billing_evidence_present: bool,
    unique_accepts: Vec<ShadowReplayAcceptedEventReport>,
    forbidden_flags: serde_json::Value,
    packages: Vec<ShadowReplayPackageReport>,
    verdict: &'static str,
    boundary: &'static str,
}

pub(crate) fn run_phase_stream_selected_split_nwpc_shadow_replay_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_REPORT));
    let promotion_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SELECTED_SPLIT_NWPC_PROMOTION_GATE_REPORT));
    let mut remaining_args = args.collect::<Vec<_>>();
    let hash_train_future = remaining_args
        .first()
        .is_some_and(|arg| arg == "--hash-train-future");
    if hash_train_future {
        remaining_args.remove(0);
    }
    let input_paths = remaining_args
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if input_paths.is_empty() {
        return Err("at least one phase-atom trace JSONL path is required".to_owned());
    }

    let promotion = read_json_value(&promotion_report_path)?;
    let promoted_specs = read_promoted_package_specs(&promotion)?;
    let selected_classes = promoted_specs
        .iter()
        .map(|package| package.broad_class_id.clone())
        .collect::<BTreeSet<_>>();
    let rows = read_shadow_replay_rows(&input_paths, &selected_classes)?;
    let mut rows_by_class = BTreeMap::<String, Vec<ReplayRow>>::new();
    for row in rows.iter().cloned() {
        rows_by_class
            .entry(row.broad_class_id.clone())
            .or_default()
            .push(row);
    }

    let mut packages = Vec::new();
    for spec in &promoted_specs {
        let class_rows = rows_by_class
            .get(&spec.broad_class_id)
            .cloned()
            .unwrap_or_default();
        packages.push(replay_promoted_package(
            spec,
            class_rows,
            hash_train_future,
        )?);
    }

    let replayed_package_count = packages.len();
    let clean_package_count = packages
        .iter()
        .filter(|package| package.blockers.is_empty())
        .count();
    let future_shadow_accepts = packages
        .iter()
        .map(|package| package.future_shadow_accepts)
        .sum();
    let package_sum_unique_accepts_over_exact_cache = packages
        .iter()
        .map(|package| package.future_unique_accepts_over_exact_cache)
        .sum();
    let package_sum_tokens_saved = packages
        .iter()
        .map(|package| package.future_tokens_saved)
        .sum();
    let package_sum_cost_saved_microusd = packages
        .iter()
        .map(|package| package.future_cost_saved_microusd)
        .sum();
    let future_false_accepts = packages
        .iter()
        .map(|package| package.future_false_accepts)
        .sum();
    let mut global_unique_accepts = BTreeMap::<String, ShadowReplayAcceptedEventReport>::new();
    let mut global_duplicate_accept_rows = 0usize;
    for event in packages
        .iter()
        .flat_map(|package| package.unique_accepts.iter().cloned())
    {
        if global_unique_accepts
            .insert(event.request_fingerprint.clone(), event)
            .is_some()
        {
            global_duplicate_accept_rows += 1;
        }
    }
    let unique_accepts = global_unique_accepts.into_values().collect::<Vec<_>>();
    let future_unique_accepts_over_exact_cache = unique_accepts.len();
    let future_tokens_saved = unique_accepts
        .iter()
        .map(|event| event.total_tokens)
        .sum::<usize>();
    let future_cost_saved_microusd = unique_accepts
        .iter()
        .map(|event| event.total_cost_microusd)
        .sum::<u64>();
    let replay_mismatch_count = packages
        .iter()
        .filter(|package| !package.replay_matches_promotion_report)
        .count();
    let promotion_gate_clear = json_string(&promotion, &["verdict"]).as_deref()
        == Some("PHASE_STREAM_SELECTED_SPLIT_NWPC_PROMOTION_GATE_V1_PASS_SHADOW_REGISTRY_READY")
        && json_bool(&promotion, &["local_accept_enabled"]) == Some(false)
        && json_bool(&promotion, &["auto_promote_enabled"]) == Some(false)
        && json_bool(&promotion, &["serving_registry_mutated"]) == Some(false)
        && json_bool(&promotion, &["market_money_claim_allowed"]) == Some(false);
    let verdict = if !promotion_gate_clear {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_V1_BLOCKED_PROMOTION_GATE"
    } else if future_false_accepts > 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_V1_FAIL_FALSE_ACCEPTS"
    } else if replay_mismatch_count > 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_V1_FAIL_REPLAY_MISMATCH"
    } else if clean_package_count > 0 && future_unique_accepts_over_exact_cache > 0 {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_V1_PASS_RUNTIME_REPLAY"
    } else {
        "PHASE_STREAM_SELECTED_SPLIT_NWPC_SHADOW_REPLAY_V1_WATCH_NO_VALUE"
    };
    let report = SelectedSplitNwpcShadowReplayReport {
        report_kind: "phase_stream_selected_split_nwpc_shadow_replay_v1",
        mode: "shadow_registry_runtime_replay_only",
        promotion_report_path: promotion_report_path.display().to_string(),
        input_paths: input_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        train_future_split_mode: if hash_train_future {
            "hash_request_fingerprint_v2"
        } else {
            "source_order_first_half_v1"
        },
        total_rows: rows.len(),
        promoted_package_count: promoted_specs.len(),
        replayed_package_count,
        clean_package_count,
        future_shadow_accepts,
        package_sum_unique_accepts_over_exact_cache,
        package_sum_tokens_saved,
        package_sum_cost_saved_microusd,
        future_unique_accepts_over_exact_cache,
        future_tokens_saved,
        future_cost_saved_microusd,
        global_duplicate_accept_rows,
        future_false_accepts,
        replay_mismatch_count,
        local_accept_enabled: false,
        auto_promote_enabled: false,
        serving_registry_mutated: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        token_cost_estimate_used: true,
        provider_billing_evidence_present: false,
        unique_accepts,
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
        packages,
        verdict,
        boundary: "shadow registry runtime replay only: reloads promoted .nwpc packages and recomputes future shadow decisions over trace rows; it does not mutate serving registry, enable local_accept, auto-promote runtime behavior, claim market money, or use legacy nwrb/role-binding paths",
    };
    write_json_file(&report_path, &report)?;

    println!("phase_stream_selected_split_nwpc_shadow_replay_v1:");
    println!("  report_path: {}", report_path.display());
    println!(
        "  promoted_package_count: {}",
        report.promoted_package_count
    );
    println!("  clean_package_count: {clean_package_count}");
    println!("  future_unique_accepts_over_exact_cache: {future_unique_accepts_over_exact_cache}");
    println!("  future_tokens_saved: {future_tokens_saved}");
    println!("  future_false_accepts: {future_false_accepts}");
    println!("  replay_mismatch_count: {replay_mismatch_count}");
    println!("  local_accept_enabled: false");
    println!("  market_money_claim_allowed: false");
    println!("  verdict: {verdict}");
    Ok(())
}

fn read_promoted_package_specs(report: &Value) -> Result<Vec<PromotedPackageSpec>, String> {
    let mut packages = Vec::new();
    let portfolio_recount_mode = json_string(report, &["report_kind"]).is_some_and(|kind| {
        kind == "phase_stream_selected_split_nwpc_portfolio_select_v1_promotion_report"
    });
    let Some(values) = report.get("packages").and_then(Value::as_array) else {
        return Ok(packages);
    };
    for value in values {
        if json_bool(value, &["promoted_to_shadow_registry"]) != Some(true) {
            continue;
        }
        let Some(broad_class_id) = json_string(value, &["broad_class_id"]) else {
            continue;
        };
        let Some(split_rule) = json_string(value, &["split_rule"]) else {
            continue;
        };
        let Some(task_name) = json_string(value, &["task_name"]) else {
            continue;
        };
        let Some(registry_package_path) = json_string(value, &["registry_package_path"]) else {
            continue;
        };
        packages.push(PromotedPackageSpec {
            broad_class_id,
            split_rule,
            task_name,
            registry_package_path,
            expected_package_fingerprint64: json_u64(value, &["inspected_package_fingerprint64"])
                .unwrap_or_default(),
            expected_package_bytes: json_usize_path(value, &["inspected_package_bytes"])
                .unwrap_or_default(),
            expected_package_records: json_usize_path(value, &["inspected_package_records"])
                .unwrap_or_default(),
            threshold_micro: json_i64_path(value, &["threshold_micro"]).unwrap_or_default(),
            expected_unique_accepts_over_exact_cache: json_usize_path(
                value,
                &["future_unique_accepts_over_exact_cache"],
            )
            .unwrap_or_default(),
            expected_tokens_saved: json_usize_path(value, &["future_tokens_saved"])
                .unwrap_or_default(),
            expected_cost_saved_microusd: json_u64(value, &["future_cost_saved_microusd"])
                .unwrap_or_default(),
            decision_count_parity_required: !portfolio_recount_mode,
        });
    }
    Ok(packages)
}

fn replay_promoted_package(
    spec: &PromotedPackageSpec,
    mut rows: Vec<ReplayRow>,
    hash_train_future: bool,
) -> Result<ShadowReplayPackageReport, String> {
    rows.sort_by(|left, right| left.stream_index.cmp(&right.stream_index));
    let package_bytes = std::fs::read(&spec.registry_package_path).map_err(|error| {
        format!(
            "failed to read shadow registry package '{}': {error}",
            spec.registry_package_path
        )
    })?;
    let package_info =
        PhaseCenterOffloadRuntime::inspect_package_bytes(&package_bytes).map_err(|error| {
            format!(
                "failed to inspect shadow registry package '{}': {error:?}",
                spec.registry_package_path
            )
        })?;
    let runtime = PhaseCenterOffloadRuntime::from_package_bytes(
        &package_bytes,
        nando_core::PhaseCenterOffloadPolicy::new(spec.threshold_micro)
            .map_err(|error| format!("invalid selected split replay threshold: {error:?}"))?,
    )
    .map_err(|error| {
        format!(
            "failed to load shadow registry package '{}': {error:?}",
            spec.registry_package_path
        )
    })?;
    let package_matches_promotion_report = package_info.fingerprint64
        == spec.expected_package_fingerprint64
        && package_bytes.len() == spec.expected_package_bytes
        && package_info.record_count == spec.expected_package_records;
    let broad_class_rows = rows.len();
    let (train_rows, future_rows) = train_future_split(&rows, hash_train_future);

    let mut future_scored_rows = 0usize;
    let mut future_matching_split_rows = 0usize;
    let mut future_shadow_accepts = 0usize;
    let mut future_unique_accepts_over_exact_cache = 0usize;
    let mut future_tokens_saved = 0usize;
    let mut future_cost_saved_microusd = 0u64;
    let mut future_false_accepts = 0usize;
    let mut future_exact_cache_hits = 0usize;
    let mut accepted_fingerprints = BTreeSet::new();
    let mut unique_accepts = Vec::new();
    let mut false_accept_examples = Vec::new();
    let cells = package_info.cells;

    for row in &future_rows {
        let matches_split = row_matches_split(row, &spec.split_rule);
        future_matching_split_rows += usize::from(matches_split);
        future_exact_cache_hits += usize::from(row.exact_cache_hit);
        if row.verified_safe_accept && !matches_split {
            continue;
        }
        future_scored_rows += 1;
        let safe_vec = selected_split_event_vector(row, true, cells, &spec.task_name);
        let reject_vec = selected_split_event_vector(row, false, cells, &spec.task_name);
        let margin = runtime
            .runtime()
            .margin_for(0, &safe_vec, &reject_vec)
            .map_err(|error| format!("shadow replay margin error: {error:?}"))?;
        let margin_micro = margin_to_micro(margin)?;
        let shadow_accept = margin_micro >= spec.threshold_micro;
        future_shadow_accepts += usize::from(shadow_accept);
        if shadow_accept && !(row.verified_safe_accept && matches_split) {
            future_false_accepts += 1;
            if false_accept_examples.len() < 16 {
                false_accept_examples.push(ShadowReplayAcceptedEventReport {
                    request_fingerprint: row.request_fingerprint.clone(),
                    exact_cache_key: row.exact_cache_key.clone(),
                    stream_index: row.stream_index,
                    broad_class_id: spec.broad_class_id.clone(),
                    task_name: spec.task_name.clone(),
                    split_rule: spec.split_rule.clone(),
                    package_fingerprint64: package_info.fingerprint64,
                    margin_micro,
                    threshold_micro: spec.threshold_micro,
                    total_tokens: row.total_tokens,
                    total_cost_microusd: row.total_cost_microusd,
                    token_evidence_missing: row.total_tokens == 0,
                    provider_billing_evidence_present: false,
                    unique_cpu_accept_over_exact_cache: false,
                    verified_safe_accept: row.verified_safe_accept,
                    false_accept: true,
                });
            }
        }
        if shadow_accept
            && row.verified_safe_accept
            && matches_split
            && !row.exact_cache_hit
            && accepted_fingerprints.insert(row.request_fingerprint.clone())
        {
            future_unique_accepts_over_exact_cache += 1;
            future_tokens_saved = future_tokens_saved.saturating_add(row.total_tokens);
            future_cost_saved_microusd =
                future_cost_saved_microusd.saturating_add(row.total_cost_microusd);
            unique_accepts.push(ShadowReplayAcceptedEventReport {
                request_fingerprint: row.request_fingerprint.clone(),
                exact_cache_key: row.exact_cache_key.clone(),
                stream_index: row.stream_index,
                broad_class_id: spec.broad_class_id.clone(),
                task_name: spec.task_name.clone(),
                split_rule: spec.split_rule.clone(),
                package_fingerprint64: package_info.fingerprint64,
                margin_micro,
                threshold_micro: spec.threshold_micro,
                total_tokens: row.total_tokens,
                total_cost_microusd: row.total_cost_microusd,
                token_evidence_missing: row.total_tokens == 0,
                provider_billing_evidence_present: false,
                unique_cpu_accept_over_exact_cache: true,
                verified_safe_accept: true,
                false_accept: false,
            });
        }
    }

    let decision_counts_match = future_unique_accepts_over_exact_cache
        == spec.expected_unique_accepts_over_exact_cache
        && future_tokens_saved == spec.expected_tokens_saved
        && future_cost_saved_microusd == spec.expected_cost_saved_microusd;
    let replay_matches_promotion_report = package_matches_promotion_report
        && future_false_accepts == 0
        && (!spec.decision_count_parity_required || decision_counts_match);
    let mut blockers = Vec::new();
    if !package_matches_promotion_report {
        blockers.push("package_mismatch_with_promotion_report".to_owned());
    }
    if future_false_accepts != 0 {
        blockers.push("future_false_accepts_nonzero".to_owned());
    }
    if future_unique_accepts_over_exact_cache == 0 {
        blockers.push("no_future_unique_accepts_over_exact_cache".to_owned());
    }
    if !replay_matches_promotion_report {
        blockers.push("replay_does_not_match_promotion_report".to_owned());
    }
    blockers.sort();
    blockers.dedup();

    Ok(ShadowReplayPackageReport {
        broad_class_id: spec.broad_class_id.clone(),
        split_rule: spec.split_rule.clone(),
        task_name: spec.task_name.clone(),
        registry_package_path: spec.registry_package_path.clone(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: package_bytes.len(),
        package_records: package_info.record_count,
        package_matches_promotion_report,
        decision_count_parity_required: spec.decision_count_parity_required,
        expected_unique_accepts_over_exact_cache: spec.expected_unique_accepts_over_exact_cache,
        expected_tokens_saved: spec.expected_tokens_saved,
        expected_cost_saved_microusd: spec.expected_cost_saved_microusd,
        threshold_micro: spec.threshold_micro,
        broad_class_rows,
        train_rows: train_rows.len(),
        future_rows: future_rows.len(),
        future_scored_rows,
        future_matching_split_rows,
        future_shadow_accepts,
        future_unique_accepts_over_exact_cache,
        future_tokens_saved,
        future_cost_saved_microusd,
        future_false_accepts,
        future_exact_cache_hits,
        unique_accepts,
        false_accept_examples,
        replay_matches_promotion_report,
        blockers,
    })
}

fn train_future_split(
    rows: &[ReplayRow],
    hash_train_future: bool,
) -> (Vec<&ReplayRow>, Vec<&ReplayRow>) {
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

fn read_shadow_replay_rows(
    paths: &[PathBuf],
    selected_classes: &BTreeSet<String>,
) -> Result<Vec<ReplayRow>, String> {
    let mut rows = Vec::new();
    let mut seen_exact_cache = BTreeSet::new();
    let mut source_index = 0usize;
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
                    "failed to parse shadow replay trace '{}' line {}: {error}",
                    path.display(),
                    line_index + 1
                )
            })?;
            source_index += 1;
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
                .unwrap_or_else(|| format!("selected-split-shadow-row:{source_index}"));
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
            for broad_class_id in selected_action_families {
                rows.push(ReplayRow {
                    stream_index: source_index,
                    broad_class_id,
                    request_fingerprint: request_fingerprint.clone(),
                    exact_cache_key: exact_cache_key.clone(),
                    exact_cache_hit,
                    verified_safe_accept,
                    total_tokens: token_cost.total_tokens,
                    total_cost_microusd: token_cost.total_cost_microusd,
                    atoms: atoms.clone(),
                });
            }
        }
    }
    Ok(rows)
}

fn selected_split_event_vector(
    row: &ReplayRow,
    candidate_safe_accept: bool,
    cells: usize,
    task_name: &str,
) -> Vec<PhaseCenterCell> {
    let mut atom_ids = row
        .atoms
        .iter()
        .map(String::as_str)
        .filter(|atom| !atom.starts_with("phase_atom_binary_task:"))
        .map(|atom| stable_fingerprint([atom]))
        .collect::<Vec<_>>();
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

fn row_matches_split(row: &ReplayRow, split_rule: &str) -> bool {
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
