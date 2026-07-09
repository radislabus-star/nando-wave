use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{
    DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_JSONL,
    DEFAULT_AGENT_CONTINUE_COMMAND_RESULT_FOLLOWUP_PACK_JSONL,
    DEFAULT_AUTO_SUBCENTER_DISCOVERY_CANDIDATES_JSONL,
    DEFAULT_AUTO_SUBCENTER_DISCOVERY_REJECTIONS_JSONL, DEFAULT_AUTO_SUBCENTER_DISCOVERY_REPORT,
    ForbiddenFlags, GenericTokenCost, generic_count_band, json_string,
    phase_atom_binary_token_cost, phase_atom_external_provider_correlation_keys,
    phase_atom_string_vec, sanitize_file_stem, stable_fingerprint, write_json_file,
};

const AUTO_SUBCENTER_MULTI2_SPLIT_SCORE_WEIGHT: u128 = 8;
const AUTO_SUBCENTER_MULTI3_SPLIT_SCORE_WEIGHT: u128 = 4096;
const AUTO_SUBCENTER_MULTI4_SPLIT_SCORE_WEIGHT: u128 = 16384;
const AUTO_SUBCENTER_MAX_MULTI2_PER_ROW: usize = 32;
const AUTO_SUBCENTER_MAX_MULTI3_PER_ROW: usize = 48;
const AUTO_SUBCENTER_MAX_MULTI4_PER_ROW: usize = 24;
const AUTO_SUBCENTER_GENERATED_ACTION_FAMILY: &str = "action_family:auto_subcenter_discovery";

#[derive(Clone, Debug, Serialize)]
struct AutoSubcenterDiscoveryReport {
    report_kind: &'static str,
    mode: &'static str,
    input_trace_paths: Vec<String>,
    candidate_trace_path: String,
    rejection_log_path: String,
    max_selected_candidates: usize,
    max_positive_rows_per_candidate: usize,
    background_rows_per_positive: usize,
    total_rows_seen: usize,
    eligible_rows: usize,
    enumerated_split_atoms: usize,
    enumerated_single_split_atoms: usize,
    enumerated_compound_split_atoms: usize,
    selected_candidates: usize,
    selected_single_split_candidates: usize,
    selected_compound_split_candidates: usize,
    selected_multi2_split_candidates: usize,
    selected_multi3_split_candidates: usize,
    selected_multi4_split_candidates: usize,
    max_selected_compound_arity: usize,
    multi_split_learned: bool,
    manual_class_list_used: bool,
    split_authority_source: &'static str,
    generated_action_family_atom: &'static str,
    generated_action_family_is_operator_authority: bool,
    rejected_candidates: usize,
    candidate_trace_rows_written: usize,
    candidate_positive_rows_written: usize,
    candidate_direct_negative_rows_written: usize,
    candidate_background_rows_written: usize,
    exact_cache_hits: usize,
    exact_cache_misses_over_cache: usize,
    estimated_total_tokens: usize,
    estimated_total_cost_microusd: u64,
    candidates: Vec<AutoSubcenterCandidateReport>,
    rejections: Vec<AutoSubcenterRejectionReport>,
    automation_contract: AutoSubcenterAutomationContract,
    forbidden_flags: ForbiddenFlags,
    compile_allowed: bool,
    online_learn_enabled: bool,
    online_shadow_enabled: bool,
    auto_promote_enabled: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct AutoSubcenterAutomationContract {
    ranked_candidate_split_atoms: bool,
    chose_subcenters_by_score: bool,
    selected_background_negatives: bool,
    rejected_bad_candidates_with_reasons: bool,
    compatible_denominator_delta_measured: bool,
    compatible_denominator_next_step: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct AutoSubcenterCandidateReport {
    rank: usize,
    candidate_id: String,
    split_group: String,
    split_atom: String,
    score: u128,
    source_rows_with_atom: usize,
    positive_rows: usize,
    direct_negative_rows: usize,
    background_negative_rows_available: usize,
    positive_rows_written: usize,
    direct_negative_rows_written: usize,
    background_rows_written: usize,
    exact_cache_hits_in_written_rows: usize,
    non_exact_positive_rows_written: usize,
    estimated_tokens_in_written_rows: usize,
    selected_background_policy: &'static str,
    generated_bucket_action_family: &'static str,
    generated_route_hint: String,
}

#[derive(Clone, Debug, Serialize)]
struct AutoSubcenterRejectionReport {
    split_group: String,
    split_atom: String,
    reason: String,
    source_rows_with_atom: usize,
    positive_rows: usize,
    direct_negative_rows: usize,
    background_negative_rows_available: usize,
    score: u128,
}

#[derive(Clone, Debug)]
struct AutoSubcenterSourceRow {
    source_path: String,
    source_line_index: usize,
    source_row_index: usize,
    row: serde_json::Value,
    source_trace_id: Option<String>,
    exact_cache_key: String,
    request_fingerprint: String,
    external_provider_correlation_keys: Vec<String>,
    verified_safe_accept: bool,
    token_cost: GenericTokenCost,
    atoms: BTreeMap<&'static str, Vec<String>>,
}

#[derive(Clone, Debug, Default)]
struct AutoSubcenterAtomStats {
    group: &'static str,
    atom: String,
    rows: Vec<usize>,
    positive_rows: usize,
    direct_negative_rows: usize,
    background_negative_rows_available: usize,
    non_exact_positive_rows: usize,
    positive_token_ceiling: usize,
    score: u128,
    rejection_reason: Option<String>,
}

pub(crate) fn run_phase_stream_auto_subcenter_discovery_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AUTO_SUBCENTER_DISCOVERY_REPORT));
    let candidate_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AUTO_SUBCENTER_DISCOVERY_CANDIDATES_JSONL));
    let rejection_log_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AUTO_SUBCENTER_DISCOVERY_REJECTIONS_JSONL));
    let max_selected_candidates = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_selected_candidates '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(48);
    let max_positive_rows_per_candidate = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!("invalid max_positive_rows_per_candidate '{value}': {error}")
            })
        })
        .transpose()?
        .unwrap_or(1200);
    let background_rows_per_positive = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid background_rows_per_positive '{value}': {error}"))
        })
        .transpose()?
        .unwrap_or(1);
    if max_selected_candidates == 0 {
        return Err("max_selected_candidates must be > 0".to_owned());
    }
    if max_positive_rows_per_candidate == 0 {
        return Err("max_positive_rows_per_candidate must be > 0".to_owned());
    }
    if background_rows_per_positive == 0 {
        return Err("background_rows_per_positive must be > 0".to_owned());
    }
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![
                PathBuf::from(DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_JSONL),
                PathBuf::from(DEFAULT_AGENT_CONTINUE_COMMAND_RESULT_FOLLOWUP_PACK_JSONL),
            ]
        } else {
            rest
        }
    };

    let mut source_rows = Vec::<AutoSubcenterSourceRow>::new();
    let mut total_rows_seen = 0usize;
    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read auto-subcenter source trace '{}': {error}",
                trace_path.display()
            )
        })?;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            total_rows_seen += 1;
            if total_rows_seen == 1 || total_rows_seen.is_multiple_of(5000) {
                println!(
                    "auto_subcenter_discovery_scan_progress: rows_seen={} eligible_rows={}",
                    total_rows_seen,
                    source_rows.len()
                );
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse auto-subcenter trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if let Some(source_row) =
                parse_auto_subcenter_source_row(trace_path, line_index, source_rows.len(), row)
            {
                source_rows.push(source_row);
            }
        }
    }

    let exact_cache_flags = exact_cache_hit_flags_auto_subcenter(&source_rows);
    let mut exact_cache_hits = 0usize;
    let mut estimated_total_tokens = 0usize;
    let mut estimated_total_cost_microusd = 0u64;
    for (row, exact_hit) in source_rows.iter().zip(exact_cache_flags.iter().copied()) {
        exact_cache_hits += usize::from(exact_hit);
        estimated_total_tokens = estimated_total_tokens.saturating_add(row.token_cost.total_tokens);
        estimated_total_cost_microusd =
            estimated_total_cost_microusd.saturating_add(row.token_cost.total_cost_microusd);
    }

    let mut atom_stats = auto_subcenter_atom_stats(&source_rows, &exact_cache_flags);
    let enumerated_split_atoms = atom_stats.len();
    let enumerated_compound_split_atoms = atom_stats
        .iter()
        .filter(|stats| stats.group == "multi")
        .count();
    let enumerated_single_split_atoms =
        enumerated_split_atoms.saturating_sub(enumerated_compound_split_atoms);
    let mut rejections = Vec::new();
    for stats in &mut atom_stats {
        let has_enough_positive = stats.positive_rows >= 20;
        let has_background = stats.background_negative_rows_available >= 20;
        let not_too_broad = stats.rows.len() < source_rows.len();
        if let Some(reason) = auto_subcenter_split_atom_policy_rejection(stats.group, &stats.atom) {
            stats.rejection_reason = Some(reason.to_owned());
        } else if !has_enough_positive {
            stats.rejection_reason = Some("less_than_20_positive_rows".to_owned());
        } else if !has_background {
            stats.rejection_reason = Some("less_than_20_background_negative_rows".to_owned());
        } else if !not_too_broad {
            stats.rejection_reason = Some("atom_covers_all_eligible_rows".to_owned());
        } else if stats.score == 0 {
            stats.rejection_reason = Some("zero_non_exact_value_score".to_owned());
        }
        if let Some(reason) = &stats.rejection_reason {
            rejections.push(AutoSubcenterRejectionReport {
                split_group: stats.group.to_owned(),
                split_atom: stats.atom.clone(),
                reason: reason.clone(),
                source_rows_with_atom: stats.rows.len(),
                positive_rows: stats.positive_rows,
                direct_negative_rows: stats.direct_negative_rows,
                background_negative_rows_available: stats.background_negative_rows_available,
                score: stats.score,
            });
        }
    }
    atom_stats.retain(|stats| stats.rejection_reason.is_none());
    atom_stats.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.positive_rows.cmp(&left.positive_rows))
            .then_with(|| left.atom.cmp(&right.atom))
    });
    let mut selected_atom_stats = Vec::new();
    for stats in atom_stats {
        if auto_subcenter_rowset_too_similar(&stats, &selected_atom_stats) {
            rejections.push(AutoSubcenterRejectionReport {
                split_group: stats.group.to_owned(),
                split_atom: stats.atom.clone(),
                reason: "duplicate_rowset_overlap_with_selected_candidate".to_owned(),
                source_rows_with_atom: stats.rows.len(),
                positive_rows: stats.positive_rows,
                direct_negative_rows: stats.direct_negative_rows,
                background_negative_rows_available: stats.background_negative_rows_available,
                score: stats.score,
            });
            continue;
        }
        selected_atom_stats.push(stats);
        if selected_atom_stats.len() >= max_selected_candidates {
            break;
        }
    }
    let atom_stats = selected_atom_stats;

    let mut candidate_trace = String::new();
    let mut candidate_reports = Vec::new();
    let mut candidate_trace_rows_written = 0usize;
    let mut candidate_positive_rows_written = 0usize;
    let mut candidate_direct_negative_rows_written = 0usize;
    let mut candidate_background_rows_written = 0usize;
    let mut selected_single_split_candidates = 0usize;
    let mut selected_compound_split_candidates = 0usize;
    let mut selected_multi2_split_candidates = 0usize;
    let mut selected_multi3_split_candidates = 0usize;
    let mut selected_multi4_split_candidates = 0usize;
    let mut max_selected_compound_arity = 0usize;

    for (rank_index, stats) in atom_stats.iter().enumerate() {
        let compound_arity = auto_subcenter_compound_arity(stats.group, &stats.atom);
        if compound_arity == 0 {
            selected_single_split_candidates += 1;
        } else {
            selected_compound_split_candidates += 1;
            max_selected_compound_arity = max_selected_compound_arity.max(compound_arity);
            match compound_arity {
                2 => selected_multi2_split_candidates += 1,
                3 => selected_multi3_split_candidates += 1,
                4 => selected_multi4_split_candidates += 1,
                _ => {}
            }
        }
        let candidate_id = auto_subcenter_candidate_id(stats.group, &stats.atom);
        let positive_indices = auto_subcenter_selected_positive_indices(
            &source_rows,
            stats,
            max_positive_rows_per_candidate,
        );
        let direct_negative_indices = auto_subcenter_selected_direct_negative_indices(
            &source_rows,
            stats,
            max_positive_rows_per_candidate,
        );
        let background_indices = auto_subcenter_selected_background_indices(
            &source_rows,
            stats,
            positive_indices
                .len()
                .saturating_mul(background_rows_per_positive),
        );
        if positive_indices.is_empty()
            || (direct_negative_indices.is_empty() && background_indices.is_empty())
        {
            rejections.push(AutoSubcenterRejectionReport {
                split_group: stats.group.to_owned(),
                split_atom: stats.atom.clone(),
                reason: "selected_window_missing_positive_or_negative_rows".to_owned(),
                source_rows_with_atom: stats.rows.len(),
                positive_rows: stats.positive_rows,
                direct_negative_rows: stats.direct_negative_rows,
                background_negative_rows_available: stats.background_negative_rows_available,
                score: stats.score,
            });
            continue;
        }
        println!(
            "auto_subcenter_candidate_selected: rank={} candidate={} positives={} direct_negatives={} backgrounds={}",
            rank_index + 1,
            candidate_id,
            positive_indices.len(),
            direct_negative_indices.len(),
            background_indices.len()
        );
        let mut written_exact_hits = 0usize;
        let mut written_non_exact_positive = 0usize;
        let mut written_tokens = 0usize;
        let mut written_direct_negatives = 0usize;
        let mut written_backgrounds = 0usize;
        for (index, positive, source_label) in auto_subcenter_interleaved_candidate_indices(
            &source_rows,
            &positive_indices,
            &direct_negative_indices,
            &background_indices,
        ) {
            let source = &source_rows[index];
            let output_row = build_auto_subcenter_candidate_row(
                source,
                stats,
                &candidate_id,
                positive,
                source_label,
                candidate_trace_rows_written,
            );
            written_exact_hits += usize::from(exact_cache_flags[index]);
            written_non_exact_positive += usize::from(positive && !exact_cache_flags[index]);
            written_tokens = written_tokens.saturating_add(source.token_cost.total_tokens);
            candidate_trace.push_str(&serde_json::to_string(&output_row).map_err(|error| {
                format!("failed to serialize auto-subcenter candidate row: {error}")
            })?);
            candidate_trace.push('\n');
            candidate_trace_rows_written += 1;
            candidate_positive_rows_written += usize::from(positive);
            written_direct_negatives +=
                usize::from(source_label == "direct_negative_with_split_atom");
            written_backgrounds += usize::from(source_label == "background_without_split_atom");
            candidate_direct_negative_rows_written +=
                usize::from(source_label == "direct_negative_with_split_atom");
            candidate_background_rows_written +=
                usize::from(source_label == "background_without_split_atom");
        }
        candidate_reports.push(AutoSubcenterCandidateReport {
            rank: rank_index + 1,
            candidate_id: candidate_id.clone(),
            split_group: stats.group.to_owned(),
            split_atom: stats.atom.clone(),
            score: stats.score,
            source_rows_with_atom: stats.rows.len(),
            positive_rows: stats.positive_rows,
            direct_negative_rows: stats.direct_negative_rows,
            background_negative_rows_available: stats.background_negative_rows_available,
            positive_rows_written: positive_indices.len(),
            direct_negative_rows_written: written_direct_negatives,
            background_rows_written: written_backgrounds,
            exact_cache_hits_in_written_rows: written_exact_hits,
            non_exact_positive_rows_written: written_non_exact_positive,
            estimated_tokens_in_written_rows: written_tokens,
            selected_background_policy:
                "same_eligible_trace_rows_without_split_atom_ranked_by_non_exact_token_value_then_balanced_positive_background_interleave",
            generated_bucket_action_family: AUTO_SUBCENTER_GENERATED_ACTION_FAMILY,
            generated_route_hint: format!("route_hint:auto_subcenter_candidate:{candidate_id}"),
        });
    }

    if let Some(parent) = candidate_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create auto-subcenter candidate dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&candidate_trace_path, candidate_trace).map_err(|error| {
        format!(
            "failed to write auto-subcenter candidate trace '{}': {error}",
            candidate_trace_path.display()
        )
    })?;
    if let Some(parent) = rejection_log_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create auto-subcenter rejection dir '{}': {error}",
                parent.display()
            )
        })?;
    }
    let rejection_text = rejections
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to serialize auto-subcenter rejection row: {error}"))?
        .join("\n");
    std::fs::write(
        &rejection_log_path,
        if rejection_text.is_empty() {
            String::new()
        } else {
            format!("{rejection_text}\n")
        },
    )
    .map_err(|error| {
        format!(
            "failed to write auto-subcenter rejections '{}': {error}",
            rejection_log_path.display()
        )
    })?;

    let report = AutoSubcenterDiscoveryReport {
        report_kind: "phase_stream_auto_subcenter_discovery_v1",
        mode: "audit_only_automatic_split_atom_and_background_negative_builder_for_phase_center_mining",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        candidate_trace_path: candidate_trace_path.display().to_string(),
        rejection_log_path: rejection_log_path.display().to_string(),
        max_selected_candidates,
        max_positive_rows_per_candidate,
        background_rows_per_positive,
        total_rows_seen,
        eligible_rows: source_rows.len(),
        enumerated_split_atoms,
        enumerated_single_split_atoms,
        enumerated_compound_split_atoms,
        selected_candidates: candidate_reports.len(),
        selected_single_split_candidates,
        selected_compound_split_candidates,
        selected_multi2_split_candidates,
        selected_multi3_split_candidates,
        selected_multi4_split_candidates,
        max_selected_compound_arity,
        multi_split_learned: selected_compound_split_candidates > 0
            && selected_compound_split_candidates >= selected_single_split_candidates,
        manual_class_list_used: false,
        split_authority_source: "ranked observable request/state/action/tool atoms plus compound multi2/multi3/multi4 atoms; no static class allowlist",
        generated_action_family_atom: AUTO_SUBCENTER_GENERATED_ACTION_FAMILY,
        generated_action_family_is_operator_authority: false,
        rejected_candidates: rejections.len(),
        candidate_trace_rows_written,
        candidate_positive_rows_written,
        candidate_direct_negative_rows_written,
        candidate_background_rows_written,
        exact_cache_hits,
        exact_cache_misses_over_cache: source_rows.len().saturating_sub(exact_cache_hits),
        estimated_total_tokens,
        estimated_total_cost_microusd,
        candidates: candidate_reports,
        rejections,
        automation_contract: AutoSubcenterAutomationContract {
            ranked_candidate_split_atoms: true,
            chose_subcenters_by_score: true,
            selected_background_negatives: true,
            rejected_bad_candidates_with_reasons: true,
            compatible_denominator_delta_measured: false,
            compatible_denominator_next_step: "run phase-stream-phase-atom-live-self-mining-loop-v1 and then phase-stream-phase-atom-compatible-denominator-shadow-v1 on candidate_trace_path",
        },
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        compile_allowed: true,
        online_learn_enabled: true,
        online_shadow_enabled: true,
        auto_promote_enabled: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        boundary: "auto-subcenter discovery only: derives split atoms and background negatives from observable phase atoms, writes a generated candidate trace for quarantine .nwpc mining, but does not promote, serve, enable local_accept, claim money, use raw prompt/output, target/proof authority, lookup, local_out_t, or legacy nwrb/role-binding backend",
    };
    write_json_file(&report_path, &report)?;
    println!("phase_stream_auto_subcenter_discovery_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  candidate_trace_path: {}", candidate_trace_path.display());
    println!("  rejection_log_path: {}", rejection_log_path.display());
    println!("  total_rows_seen: {total_rows_seen}");
    println!("  eligible_rows: {}", report.eligible_rows);
    println!("  enumerated_split_atoms: {enumerated_split_atoms}");
    println!("  selected_candidates: {}", report.selected_candidates);
    println!(
        "  selected_compound_split_candidates: {}",
        report.selected_compound_split_candidates
    );
    println!("  multi_split_learned: {}", report.multi_split_learned);
    println!(
        "  manual_class_list_used: {}",
        report.manual_class_list_used
    );
    println!(
        "  candidate_trace_rows_written: {}",
        report.candidate_trace_rows_written
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    Ok(())
}

fn auto_subcenter_interleaved_candidate_indices(
    source_rows: &[AutoSubcenterSourceRow],
    positive_indices: &[usize],
    direct_negative_indices: &[usize],
    background_indices: &[usize],
) -> Vec<(usize, bool, &'static str)> {
    let mut positives = positive_indices.to_vec();
    positives.sort_by_key(|index| source_rows[*index].source_row_index);
    let mut negatives = direct_negative_indices
        .iter()
        .copied()
        .map(|index| (index, "direct_negative_with_split_atom"))
        .chain(
            background_indices
                .iter()
                .copied()
                .map(|index| (index, "background_without_split_atom")),
        )
        .collect::<Vec<_>>();
    negatives.sort_by_key(|(index, _)| source_rows[*index].source_row_index);
    let total_len = positives.len().saturating_add(negatives.len());
    let mut output = Vec::with_capacity(total_len);
    let mut positive_cursor = 0usize;
    let mut negative_cursor = 0usize;
    for output_index in 0..total_len {
        let expected_positives = (output_index + 1).saturating_mul(positives.len()) / total_len;
        let should_emit_positive = positive_cursor < positives.len()
            && (positive_cursor < expected_positives || negative_cursor >= negatives.len());
        if should_emit_positive {
            output.push((
                positives[positive_cursor],
                true,
                "positive_contains_split_atom",
            ));
            positive_cursor += 1;
        } else if negative_cursor < negatives.len() {
            let (index, label) = negatives[negative_cursor];
            output.push((index, false, label));
            negative_cursor += 1;
        }
    }
    while positive_cursor < positives.len() {
        output.push((
            positives[positive_cursor],
            true,
            "positive_contains_split_atom",
        ));
        positive_cursor += 1;
    }
    while negative_cursor < negatives.len() {
        let (index, label) = negatives[negative_cursor];
        output.push((index, false, label));
        negative_cursor += 1;
    }
    output
}

fn parse_auto_subcenter_source_row(
    input_path: &Path,
    line_index: usize,
    source_row_index: usize,
    row: serde_json::Value,
) -> Option<AutoSubcenterSourceRow> {
    let verified_safe_accept = row.get("verified_safe_accept")?.as_bool()?;
    let mut result_atoms = phase_atom_string_vec(&row, "result_atoms");
    if result_atoms.is_empty() {
        result_atoms = phase_atom_string_vec(&row, "shadow_payload_atoms");
    }
    if result_atoms.is_empty() {
        return None;
    }
    let exact_cache_key = json_string(&row, &["exact_cache_key"]).or_else(|| {
        row.get("nando_shadow_request")
            .and_then(|request| json_string(request, &["exact_cache_key"]))
    })?;
    let request_fingerprint =
        json_string(&row, &["request_fingerprint"]).unwrap_or_else(|| exact_cache_key.clone());
    let source_trace_id = json_string(&row, &["trace_id"]);
    let external_provider_correlation_keys = phase_atom_external_provider_correlation_keys(&row);
    let mut atoms = BTreeMap::<&'static str, Vec<String>>::new();
    for (group, key) in [
        ("request", "request_atoms"),
        ("state", "state_atoms"),
        ("action", "action_atoms"),
        ("tool", "tool_atoms"),
        ("route", "route_hint_atoms"),
    ] {
        atoms.insert(group, phase_atom_string_vec(&row, key));
    }
    atoms.insert("result", result_atoms);
    let token_cost = phase_atom_binary_token_cost(&row);
    atoms.entry("token").or_default().push(format!(
        "token_band:{}",
        generic_count_band(token_cost.total_tokens)
    ));
    atoms.entry("cost").or_default().push(format!(
        "cost_band:{}",
        generic_count_band(token_cost.total_cost_microusd as usize)
    ));
    let all_atoms = atoms
        .values()
        .flat_map(|items| items.iter())
        .filter(|atom| !auto_subcenter_split_atom_hard_excluded(atom))
        .cloned()
        .collect::<BTreeSet<_>>();
    if all_atoms.is_empty() {
        return None;
    }
    Some(AutoSubcenterSourceRow {
        source_path: input_path.display().to_string(),
        source_line_index: line_index,
        source_row_index,
        row,
        source_trace_id,
        exact_cache_key,
        request_fingerprint,
        external_provider_correlation_keys,
        verified_safe_accept,
        token_cost,
        atoms,
    })
}

fn exact_cache_hit_flags_auto_subcenter(rows: &[AutoSubcenterSourceRow]) -> Vec<bool> {
    let mut seen = BTreeSet::new();
    rows.iter()
        .map(|row| !seen.insert(row.exact_cache_key.as_str()))
        .collect()
}

fn auto_subcenter_atom_stats(
    rows: &[AutoSubcenterSourceRow],
    exact_cache_flags: &[bool],
) -> Vec<AutoSubcenterAtomStats> {
    let mut stats_by_key = BTreeMap::<(String, String), AutoSubcenterAtomStats>::new();
    for (row_index, row) in rows.iter().enumerate() {
        for (group, atoms) in &row.atoms {
            for atom in atoms {
                if auto_subcenter_split_atom_hard_excluded(atom) {
                    continue;
                }
                auto_subcenter_update_atom_stats(
                    &mut stats_by_key,
                    group,
                    atom,
                    row_index,
                    row,
                    exact_cache_flags,
                );
            }
        }
        for compound_atom in auto_subcenter_row_compound_split_atoms(row) {
            auto_subcenter_update_atom_stats(
                &mut stats_by_key,
                "multi",
                &compound_atom,
                row_index,
                row,
                exact_cache_flags,
            );
        }
    }
    for stats in stats_by_key.values_mut() {
        let row_set = stats.rows.iter().copied().collect::<BTreeSet<_>>();
        stats.background_negative_rows_available = rows
            .iter()
            .enumerate()
            .filter(|(index, _)| !row_set.contains(index))
            .count();
        let base_score = (stats.non_exact_positive_rows as u128)
            .saturating_mul(stats.positive_token_ceiling as u128)
            .saturating_mul(stats.background_negative_rows_available.max(1) as u128)
            .saturating_add(stats.positive_rows as u128);
        stats.score =
            base_score.saturating_mul(auto_subcenter_split_score_weight(stats.group, &stats.atom));
    }
    stats_by_key.into_values().collect()
}

fn auto_subcenter_update_atom_stats(
    stats_by_key: &mut BTreeMap<(String, String), AutoSubcenterAtomStats>,
    group: &'static str,
    atom: &str,
    row_index: usize,
    row: &AutoSubcenterSourceRow,
    exact_cache_flags: &[bool],
) {
    let key = (group.to_owned(), atom.to_owned());
    let stats = stats_by_key
        .entry(key)
        .or_insert_with(|| AutoSubcenterAtomStats {
            group,
            atom: atom.to_owned(),
            ..AutoSubcenterAtomStats::default()
        });
    stats.rows.push(row_index);
    if row.verified_safe_accept {
        stats.positive_rows += 1;
        stats.positive_token_ceiling = stats
            .positive_token_ceiling
            .saturating_add(row.token_cost.total_tokens);
        if !exact_cache_flags.get(row_index).copied().unwrap_or(false) {
            stats.non_exact_positive_rows += 1;
        }
    } else {
        stats.direct_negative_rows += 1;
    }
}

fn auto_subcenter_row_compound_split_atoms(row: &AutoSubcenterSourceRow) -> Vec<String> {
    let action_atoms = auto_subcenter_group_split_basis(row, "action", 4);
    if action_atoms.is_empty() {
        return Vec::new();
    }
    let request_atoms = auto_subcenter_group_split_basis(row, "request", 8);
    let state_atoms = auto_subcenter_group_split_basis(row, "state", 8);
    let mut context_atoms = request_atoms.clone();
    context_atoms.extend(state_atoms.clone());
    context_atoms.sort();
    context_atoms.dedup();
    context_atoms.truncate(16);

    let mut compounds = BTreeSet::new();
    for action_atom in &action_atoms {
        for context_atom in context_atoms.iter().take(AUTO_SUBCENTER_MAX_MULTI2_PER_ROW) {
            compounds.insert(format!("multi2:{action_atom}|{context_atom}"));
        }
        let mut multi3_count = 0usize;
        for request_atom in &request_atoms {
            for state_atom in &state_atoms {
                if multi3_count >= AUTO_SUBCENTER_MAX_MULTI3_PER_ROW {
                    break;
                }
                compounds.insert(format!("multi3:{action_atom}|{request_atom}|{state_atom}"));
                multi3_count += 1;
            }
            if multi3_count >= AUTO_SUBCENTER_MAX_MULTI3_PER_ROW {
                break;
            }
        }
        let mut multi4_count = 0usize;
        for request_atom in &request_atoms {
            for state_atom in &state_atoms {
                for extra_atom in &context_atoms {
                    if multi4_count >= AUTO_SUBCENTER_MAX_MULTI4_PER_ROW {
                        break;
                    }
                    if extra_atom == request_atom || extra_atom == state_atom {
                        continue;
                    }
                    if auto_subcenter_split_atom_family(extra_atom)
                        == auto_subcenter_split_atom_family(request_atom)
                        && auto_subcenter_split_atom_family(extra_atom)
                            == auto_subcenter_split_atom_family(state_atom)
                    {
                        continue;
                    }
                    compounds.insert(format!(
                        "multi4:{action_atom}|{request_atom}|{state_atom}|{extra_atom}"
                    ));
                    multi4_count += 1;
                }
                if multi4_count >= AUTO_SUBCENTER_MAX_MULTI4_PER_ROW {
                    break;
                }
            }
            if multi4_count >= AUTO_SUBCENTER_MAX_MULTI4_PER_ROW {
                break;
            }
        }
    }
    compounds.into_iter().collect()
}

fn auto_subcenter_split_score_weight(group: &str, atom: &str) -> u128 {
    if group != "multi" {
        return 1;
    }
    if atom.starts_with("multi4:") {
        AUTO_SUBCENTER_MULTI4_SPLIT_SCORE_WEIGHT
    } else if atom.starts_with("multi3:") {
        AUTO_SUBCENTER_MULTI3_SPLIT_SCORE_WEIGHT
    } else {
        AUTO_SUBCENTER_MULTI2_SPLIT_SCORE_WEIGHT
    }
}

fn auto_subcenter_compound_arity(group: &str, atom: &str) -> usize {
    if group != "multi" {
        return 0;
    }
    if atom.starts_with("multi4:") {
        4
    } else if atom.starts_with("multi3:") {
        3
    } else if atom.starts_with("multi2:") {
        2
    } else {
        0
    }
}

fn auto_subcenter_rowset_too_similar(
    candidate: &AutoSubcenterAtomStats,
    selected: &[AutoSubcenterAtomStats],
) -> bool {
    selected.iter().any(|item| {
        let candidate_rows = candidate.rows.iter().copied().collect::<BTreeSet<_>>();
        let item_rows = item.rows.iter().copied().collect::<BTreeSet<_>>();
        let min_len = candidate_rows.len().min(item_rows.len());
        if min_len == 0 {
            return false;
        }
        let intersection = candidate_rows.intersection(&item_rows).count();
        intersection.saturating_mul(100) >= min_len.saturating_mul(90)
    })
}

fn auto_subcenter_split_atom_family(atom: &str) -> &str {
    atom.split_once(':').map_or(atom, |(family, _)| family)
}

fn auto_subcenter_group_split_basis(
    row: &AutoSubcenterSourceRow,
    group: &'static str,
    limit: usize,
) -> Vec<String> {
    let mut atoms = row
        .atoms
        .get(group)
        .into_iter()
        .flatten()
        .filter(|atom| !auto_subcenter_split_atom_hard_excluded(atom))
        .filter(|atom| auto_subcenter_split_atom_policy_rejection(group, atom).is_none())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    atoms.truncate(limit);
    atoms
}

fn auto_subcenter_split_atom_hard_excluded(atom: &str) -> bool {
    if atom.is_empty() {
        return true;
    }
    atom.starts_with("output_hash64:")
        || atom.starts_with("verifier_label:")
        || atom.starts_with("verified_safe_accept:")
        || atom.starts_with("request_fingerprint:")
        || atom.starts_with("exact_cache_key:")
        || atom.starts_with("trace_id:")
        || atom.starts_with("source_trace_id:")
        || atom.starts_with("state_session_bucket:")
}

fn auto_subcenter_split_atom_policy_rejection(group: &str, atom: &str) -> Option<&'static str> {
    if matches!(group, "token" | "cost") {
        return Some("token_or_cost_band_is_value_metric_not_operator_subcenter");
    }
    if atom.starts_with("token_band:")
        || atom.starts_with("cost_band:")
        || atom.starts_with("request_token_band:")
        || atom.starts_with("request_cost_band:")
        || atom.starts_with("state_token_band:")
        || atom.starts_with("state_cost_band:")
    {
        return Some("token_or_cost_band_is_value_metric_not_operator_subcenter");
    }
    if atom.starts_with("shadow_active_") || atom.starts_with("shadow_slot_") {
        return Some("shadow_payload_shape_is_readout_evidence_not_operator_subcenter");
    }
    if group == "result" {
        return Some("result_atom_is_output_evidence_not_split_subcenter");
    }
    if group == "tool"
        && !atom.starts_with("tool_check_kind:")
        && !atom.starts_with("tool_parse_marker:")
        && !atom.starts_with("tool_parse_signal:")
    {
        return Some("tool_atom_is_observation_identity_not_operator_subcenter");
    }
    if group == "route" {
        return Some("route_hint_is_bucket_route_not_operator_subcenter");
    }
    if atom.starts_with("action_family:")
        || atom.starts_with("domain_family:")
        || atom.starts_with("route_operator:")
        || atom.starts_with("subroute_operator:")
    {
        return Some("broad_action_or_route_authority_atom");
    }
    if atom.starts_with("state_source:") {
        return Some("source_identity_atom_not_operator_subcenter");
    }
    if atom.starts_with("request_route_family:") {
        return Some("request_route_family_too_broad_for_subcenter");
    }
    if atom.contains("_valid_schema:") || atom.starts_with("state_plan_valid_schema:") {
        return Some("schema_validation_atom_not_operator_subcenter");
    }
    if matches!(group, "request" | "tool")
        && (atom.starts_with("tool_command_kind:")
            || atom.starts_with("tool_command_shell_family:"))
    {
        return Some("tool_identity_atom_not_operator_transition_subcenter");
    }
    if matches!(group, "action")
        && matches!(
            atom,
            "action:parse_tool_status" | "action:continue_after_tool_result"
        )
    {
        return Some("agent_loop_control_atom_too_broad_for_operator_subcenter");
    }
    if matches!(group, "result" | "state")
        && (atom.starts_with("evidence:exit_code_")
            || atom.starts_with("exit_code_band:")
            || atom.starts_with("state_exit_code_band:")
            || atom.starts_with("state_tool_status_evidence:")
            || atom.starts_with("state_tool_status_exit_band:")
            || atom.starts_with("state_tool_status_exit_zero:")
            || atom.starts_with("state_tool_status_shell_exit:")
            || atom.starts_with("state_tool_status_command_exit:")
            || atom.starts_with("state_tool_status_evidence:exit_code_"))
    {
        return Some("execution_outcome_atom_not_transfer_operator_subcenter");
    }
    if matches!(group, "state")
        && (atom.starts_with("state_output_marker:")
            || atom.starts_with("state_output_has_")
            || atom.starts_with("state_output_contains_"))
    {
        return Some("output_status_marker_not_operator_subcenter");
    }
    if atom.contains("_cwd_kind:") {
        return Some("cwd_identity_atom_not_transferable_operator_subcenter");
    }
    if atom.starts_with("request_command_arg_band:") {
        return Some("command_length_band_is_shape_metric_not_operator_subcenter");
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
        return Some("generic_prompt_shape_not_operator_subcenter");
    }
    if atom == "request_has_path:false"
        || atom == "state_followup_marker:false"
        || atom == "state_stop_marker:false"
    {
        return Some("negative_prompt_marker_not_operator_subcenter");
    }
    if atom.starts_with("state_output_char_band:")
        || atom.starts_with("state_output_line_band:")
        || atom.starts_with("state_output_has_warning_marker:")
        || atom.starts_with("state_output_has_error_marker:")
    {
        return Some("output_size_or_marker_band_too_broad_for_operator_subcenter");
    }
    None
}

fn auto_subcenter_split_atom_allowed(atom: &str) -> bool {
    !auto_subcenter_split_atom_hard_excluded(atom)
}

fn auto_subcenter_candidate_id(group: &str, atom: &str) -> String {
    let stem = sanitize_file_stem(
        &format!("{group}_{atom}")
            .chars()
            .take(56)
            .collect::<String>(),
    );
    let fingerprint = stable_fingerprint([group, atom]);
    format!("{stem}-{fingerprint:016x}")
}

fn auto_subcenter_selected_positive_indices(
    rows: &[AutoSubcenterSourceRow],
    stats: &AutoSubcenterAtomStats,
    limit: usize,
) -> Vec<usize> {
    let mut indices = stats
        .rows
        .iter()
        .copied()
        .filter(|index| rows[*index].verified_safe_accept)
        .collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        rows[*right]
            .token_cost
            .total_tokens
            .cmp(&rows[*left].token_cost.total_tokens)
            .then_with(|| {
                rows[*left]
                    .source_row_index
                    .cmp(&rows[*right].source_row_index)
            })
    });
    indices.truncate(limit);
    indices
}

fn auto_subcenter_selected_background_indices(
    rows: &[AutoSubcenterSourceRow],
    stats: &AutoSubcenterAtomStats,
    limit: usize,
) -> Vec<usize> {
    let candidate_rows = stats.rows.iter().copied().collect::<BTreeSet<_>>();
    let mut indices = rows
        .iter()
        .enumerate()
        .filter(|(index, _)| !candidate_rows.contains(index))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        rows[*right]
            .token_cost
            .total_tokens
            .cmp(&rows[*left].token_cost.total_tokens)
            .then_with(|| {
                rows[*left]
                    .source_row_index
                    .cmp(&rows[*right].source_row_index)
            })
    });
    indices.truncate(limit);
    indices
}

fn auto_subcenter_selected_direct_negative_indices(
    rows: &[AutoSubcenterSourceRow],
    stats: &AutoSubcenterAtomStats,
    limit: usize,
) -> Vec<usize> {
    let mut indices = stats
        .rows
        .iter()
        .copied()
        .filter(|index| !rows[*index].verified_safe_accept)
        .collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        rows[*right]
            .token_cost
            .total_tokens
            .cmp(&rows[*left].token_cost.total_tokens)
            .then_with(|| {
                rows[*left]
                    .source_row_index
                    .cmp(&rows[*right].source_row_index)
            })
    });
    indices.truncate(limit);
    indices
}

fn build_auto_subcenter_candidate_row(
    source: &AutoSubcenterSourceRow,
    stats: &AutoSubcenterAtomStats,
    candidate_id: &str,
    positive: bool,
    source_label: &'static str,
    row_index: usize,
) -> serde_json::Value {
    let sanitized_result_atoms = source
        .atoms
        .get("result")
        .into_iter()
        .flatten()
        .filter(|atom| auto_subcenter_split_atom_allowed(atom))
        .cloned()
        .collect::<Vec<_>>();
    let mut request_atoms = vec![
        "request_route_family:agent_continue_execute".to_owned(),
        "request_subcenter_auto_discovery:true".to_owned(),
        format!("request_subcenter_candidate:{candidate_id}"),
    ];
    request_atoms.extend(auto_subcenter_sanitized_source_atoms(source, "request", 12));
    let mut state_atoms = vec![
        "state_source:auto_subcenter_discovery_v1".to_owned(),
        format!("state_split_group:{}", stats.group),
        format!(
            "state_split_atom_hash:{:016x}",
            stable_fingerprint([stats.group, stats.atom.as_str()])
        ),
    ];
    state_atoms.extend(auto_subcenter_sanitized_source_atoms(source, "state", 12));
    let mut action_atoms = vec![
        AUTO_SUBCENTER_GENERATED_ACTION_FAMILY.to_owned(),
        "action:auto_subcenter_shadow_score".to_owned(),
        "route_operator:agent_continue_execute".to_owned(),
        "domain_family:agent_loop".to_owned(),
    ];
    action_atoms.extend(auto_subcenter_sanitized_source_atoms(source, "action", 8));
    let tool_atoms = vec!["tool_family:agent_loop_observation".to_owned()];
    let route_hint_atoms = vec![
        "route_hint:auto_subcenter_discovery".to_owned(),
        format!("route_hint:auto_subcenter_candidate:{candidate_id}"),
    ];
    let nando_shadow_request = auto_subcenter_shadow_request(AutoSubcenterShadowRequestInput {
        candidate_id,
        request_atoms: &request_atoms,
        state_atoms: &state_atoms,
        action_atoms: &action_atoms,
        tool_atoms: &tool_atoms,
        result_atoms: &sanitized_result_atoms,
        route_hint_atoms: &route_hint_atoms,
        exact_cache_key: &source.exact_cache_key,
    });
    let mut row = serde_json::json!({
        "schema_version": "auto_subcenter_candidate_phase_atom_trace_v1",
        "source_schema_version": source.row.get("schema_version").and_then(serde_json::Value::as_str),
        "input_trace_path": source.source_path,
        "source_line_index": source.source_line_index,
        "source_row_index": source.source_row_index,
        "trace_id": format!("auto-subcenter-{candidate_id}-{row_index:08}"),
        "event_timestamp": source.row.get("event_timestamp").and_then(serde_json::Value::as_str),
        "traffic_source": "auto_subcenter_discovery_v1",
        "auto_subcenter_candidate_id": candidate_id,
        "auto_subcenter_split_group": stats.group,
        "auto_subcenter_split_atom": stats.atom,
        "auto_subcenter_source_label": source_label,
        "request_fingerprint": source.request_fingerprint,
        "exact_cache_key": source.exact_cache_key,
        "exact_cache_hit": source.row.get("exact_cache_hit").and_then(serde_json::Value::as_bool).unwrap_or(false),
        "verified_safe_accept": positive,
        "verifier_label": if positive { "safe_accept" } else { "reject" },
        "nando_shadow_request": nando_shadow_request,
        "request_atoms": request_atoms,
        "state_atoms": state_atoms,
        "action_atoms": action_atoms,
        "tool_atoms": tool_atoms,
        "result_atoms": sanitized_result_atoms,
        "route_hint_atoms": route_hint_atoms,
        "rows_with_result_atoms": true,
        "has_shadow_request": true,
        "ready_for_route_family_mining": true,
        "ready_for_existing_shadow_scoring": true,
        "token_cost": {
            "total_tokens": source.token_cost.total_tokens,
            "total_cost_microusd": source.token_cost.total_cost_microusd,
            "token_evidence_missing": source.token_cost.token_evidence_missing,
            "cost_evidence_missing": source.token_cost.cost_evidence_missing
        },
        "provider_cost_microusd": source.row.get("provider_cost_microusd").and_then(serde_json::Value::as_u64).unwrap_or(0),
        "forbidden_fields_absent": {
            "raw_prompt_text": true,
            "raw_answer_text": true,
            "raw_output_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true,
            "legacy_nwrb_backend": true
        }
    });
    if let Some(map) = row.as_object_mut() {
        map.insert(
            "source_trace_id".to_owned(),
            source
                .source_trace_id
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String),
        );
        map.insert(
            "external_provider_correlation_keys".to_owned(),
            serde_json::json!(source.external_provider_correlation_keys.clone()),
        );
        map.insert(
            "provider_correlation_ready".to_owned(),
            serde_json::json!(!source.external_provider_correlation_keys.is_empty()),
        );
    }
    row
}

struct AutoSubcenterShadowRequestInput<'a> {
    candidate_id: &'a str,
    request_atoms: &'a [String],
    state_atoms: &'a [String],
    action_atoms: &'a [String],
    tool_atoms: &'a [String],
    result_atoms: &'a [String],
    route_hint_atoms: &'a [String],
    exact_cache_key: &'a str,
}

fn auto_subcenter_shadow_request(input: AutoSubcenterShadowRequestInput<'_>) -> serde_json::Value {
    let source_atoms = input
        .request_atoms
        .iter()
        .chain(input.state_atoms.iter())
        .chain(input.action_atoms.iter())
        .chain(input.tool_atoms.iter())
        .chain(input.route_hint_atoms.iter())
        .collect::<Vec<_>>();
    let mut seen_shadow_centers = BTreeSet::new();
    let active_fringe = source_atoms
        .iter()
        .filter_map(|atom| {
            let center_id = stable_fingerprint([atom.as_str()]) % 131_072;
            if seen_shadow_centers.insert(center_id) {
                Some(serde_json::json!({
                    "center_id": center_id,
                    "strength": 1
                }))
            } else {
                None
            }
        })
        .take(48)
        .collect::<Vec<_>>();
    let slots = input
        .result_atoms
        .iter()
        .take(8)
        .enumerate()
        .map(|(slot_id, atom)| {
            serde_json::json!({
                "binding_output_slot": slot_id as u64,
                "slot_kind": "auto_subcenter_result_atom",
                "value_band": atom,
                "positive_impulses": [
                    {
                        "lane_id": stable_fingerprint(["auto_subcenter", input.candidate_id, atom.as_str()]) % 4096,
                        "strength": 1
                    }
                ],
                "negative_impulses": []
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "route_key": "auto_subcenter_discovery",
        "profile_id": format!("phase_center_auto_subcenter_{}", input.candidate_id),
        "exact_cache_key": input.exact_cache_key,
        "active_fringe": active_fringe,
        "slots": slots,
        "source": "auto_subcenter_observable_phase_atoms_v1",
        "forbidden_fields_absent": {
            "raw_prompt_text": true,
            "raw_answer_text": true,
            "raw_output_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    })
}

fn auto_subcenter_sanitized_source_atoms(
    source: &AutoSubcenterSourceRow,
    group: &'static str,
    limit: usize,
) -> Vec<String> {
    source
        .atoms
        .get(group)
        .into_iter()
        .flatten()
        .filter(|atom| !auto_subcenter_split_atom_hard_excluded(atom))
        .filter(|atom| auto_subcenter_split_atom_policy_rejection(group, atom).is_none())
        .cloned()
        .take(limit)
        .collect()
}
