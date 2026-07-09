use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{
    DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_JSONL,
    DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_REPORT,
    DEFAULT_AGENT_CONTINUE_COMMAND_RESULT_FOLLOWUP_PACK_JSONL,
    DEFAULT_AGENT_CONTINUE_COMMAND_RESULT_FOLLOWUP_PACK_REPORT,
    DEFAULT_AGENT_CONTINUE_SUBROUTE_SCOREBOARD_REPORT,
    DEFAULT_CODEX_SESSION_PLANNING_VERIFIER_JSONL, ForbiddenFlags, GenericTokenCost, json_at,
    json_string, per_thousand, phase_atom_binary_token_cost,
    phase_atom_external_provider_correlation_keys, phase_atom_false_accept_risk,
    phase_atom_string_vec, stable_fingerprint, write_json_file,
};

#[derive(Clone, Debug, Serialize)]
struct AgentContinueActiveTurnStateTraceReport {
    report_kind: &'static str,
    mode: &'static str,
    input_trace_paths: Vec<String>,
    output_trace_path: String,
    total_rows_seen: usize,
    agent_continue_rows_written: usize,
    rows_with_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    rows_with_result_atoms: usize,
    rows_with_shadow_request: usize,
    exact_cache_hits: usize,
    exact_cache_misses_over_cache: usize,
    token_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    estimated_total_tokens: usize,
    estimated_total_cost_microusd: u64,
    subroute_count: usize,
    top_subroutes: Vec<AgentContinueSubrouteScoreboardRow>,
    raw_prompt_text_written: bool,
    raw_answer_text_written: bool,
    compile_allowed: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct AgentContinueSubrouteScoreboardReport {
    report_kind: &'static str,
    mode: &'static str,
    input_trace_path: String,
    total_rows: usize,
    exact_cache_hits: usize,
    exact_cache_misses_over_cache: usize,
    rows_with_verifier_label: usize,
    verifier_true_rows: usize,
    verifier_false_rows: usize,
    rows_with_result_atoms: usize,
    rows_with_shadow_request: usize,
    rows_ready_for_subroute_mining: usize,
    subroute_count: usize,
    subroutes: Vec<AgentContinueSubrouteScoreboardRow>,
    compile_allowed: bool,
    local_accept_enabled: bool,
    product_runtime_changed: bool,
    serving_runtime_changed: bool,
    market_money_claim_allowed: bool,
    forbidden_flags: ForbiddenFlags,
    boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct AgentContinueSubrouteScoreboardRow {
    subroute_hint: String,
    rows: usize,
    traffic_share_milli: usize,
    exact_cache_hits: usize,
    exact_cache_misses: usize,
    exact_cache_overlap_milli: usize,
    verifier_true: usize,
    verifier_false: usize,
    rows_with_verifier_label: usize,
    rows_missing_verifier_label: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    rows_with_result_atoms: usize,
    rows_missing_result_atoms: usize,
    rows_with_shadow_request: usize,
    rows_missing_shadow_request: usize,
    token_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    expected_tokens_saved_over_exact_cache: usize,
    expected_cost_saved_microusd_over_exact_cache: u64,
    ready_for_subroute_mining: bool,
    false_accept_risk: &'static str,
    recommended_next_action: &'static str,
}

#[derive(Clone, Debug, Default)]
struct AgentContinueSubrouteState {
    subroute_hint: String,
    rows: usize,
    exact_cache_hits: usize,
    verifier_true: usize,
    verifier_false: usize,
    rows_with_verifier_label: usize,
    verifier_true_over_exact_cache_ceiling: usize,
    rows_with_result_atoms: usize,
    rows_with_shadow_request: usize,
    token_events: usize,
    provider_cost_events: usize,
    estimated_cost_events: usize,
    expected_tokens_saved_over_exact_cache: usize,
    expected_cost_saved_microusd_over_exact_cache: u64,
}

#[derive(Clone, Copy, Debug)]
struct AgentContinueSubrouteObservation<'a> {
    exact_cache_hit: bool,
    verifier_label: Option<bool>,
    has_result_atoms: bool,
    has_shadow_request: bool,
    has_tokens: bool,
    has_provider_cost: bool,
    has_estimated_cost: bool,
    token_cost: &'a GenericTokenCost,
}

pub(crate) fn run_phase_stream_agent_continue_active_turn_state_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_REPORT));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_JSONL));
    let trace_paths = {
        let rest = args.map(PathBuf::from).collect::<Vec<_>>();
        if rest.is_empty() {
            vec![PathBuf::from(DEFAULT_CODEX_SESSION_PLANNING_VERIFIER_JSONL)]
        } else {
            rest
        }
    };
    if trace_paths.is_empty() {
        return Err("no phase atom trace paths provided".to_owned());
    }

    let mut total_rows_seen = 0usize;
    let mut agent_continue_rows_written = 0usize;
    let mut rows_with_verifier_label = 0usize;
    let mut verifier_true_rows = 0usize;
    let mut verifier_false_rows = 0usize;
    let mut rows_with_result_atoms = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut exact_cache_hits = 0usize;
    let mut token_events = 0usize;
    let mut provider_cost_events = 0usize;
    let mut estimated_cost_events = 0usize;
    let mut estimated_total_tokens = 0usize;
    let mut estimated_total_cost_microusd = 0u64;
    let mut seen_exact_cache_keys = BTreeSet::<String>::new();
    let mut subroutes = BTreeMap::<String, AgentContinueSubrouteState>::new();
    let mut output = String::new();

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read agent_continue input trace '{}': {error}",
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
                    "agent_continue_active_turn_scan_progress: rows_seen={} rows_written={}",
                    total_rows_seen, agent_continue_rows_written
                );
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse agent_continue input trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if !is_agent_continue_phase_atom_row(&row) {
                continue;
            }

            let output_row = build_agent_continue_active_turn_row(
                trace_path,
                &row,
                agent_continue_rows_written,
                &mut seen_exact_cache_keys,
            );
            let exact_cache_hit = output_row
                .get("exact_cache_hit")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            exact_cache_hits += usize::from(exact_cache_hit);
            let verifier_label = output_row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool);
            rows_with_verifier_label += usize::from(verifier_label.is_some());
            verifier_true_rows += usize::from(verifier_label == Some(true));
            verifier_false_rows += usize::from(verifier_label == Some(false));
            let has_result_atoms = output_row
                .get("result_atoms")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| !items.is_empty());
            rows_with_result_atoms += usize::from(has_result_atoms);
            let has_shadow_request = output_row
                .get("nando_shadow_request")
                .and_then(serde_json::Value::as_object)
                .is_some();
            rows_with_shadow_request += usize::from(has_shadow_request);
            let token_cost = GenericTokenCost {
                total_tokens: json_at(&output_row, &["token_cost", "total_tokens"])
                    .and_then(serde_json::Value::as_u64)
                    .map_or(0, |value| value as usize),
                total_cost_microusd: json_at(&output_row, &["token_cost", "total_cost_microusd"])
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                evidence_missing: false,
                token_evidence_missing: false,
                cost_evidence_missing: false,
            };
            let has_tokens = token_cost.total_tokens > 0;
            let has_provider_cost = output_row
                .get("provider_cost_microusd")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                > 0;
            let has_estimated_cost = token_cost.total_cost_microusd > 0 && !has_provider_cost;
            token_events += usize::from(has_tokens);
            provider_cost_events += usize::from(has_provider_cost);
            estimated_cost_events += usize::from(has_estimated_cost);
            estimated_total_tokens = estimated_total_tokens.saturating_add(token_cost.total_tokens);
            estimated_total_cost_microusd =
                estimated_total_cost_microusd.saturating_add(token_cost.total_cost_microusd);

            let subroute_hint = output_row
                .get("subroute_hint")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("artifact_progress")
                .to_owned();
            observe_agent_continue_subroute(
                &mut subroutes,
                &subroute_hint,
                AgentContinueSubrouteObservation {
                    exact_cache_hit,
                    verifier_label,
                    has_result_atoms,
                    has_shadow_request,
                    has_tokens,
                    has_provider_cost,
                    has_estimated_cost,
                    token_cost: &token_cost,
                },
            );

            output.push_str(&serde_json::to_string(&output_row).map_err(|error| {
                format!("failed to serialize agent_continue active-turn row: {error}")
            })?);
            output.push('\n');
            agent_continue_rows_written += 1;
        }
    }

    if let Some(parent) = output_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create active-turn trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&output_trace_path, output).map_err(|error| {
        format!(
            "failed to write agent_continue active-turn trace '{}': {error}",
            output_trace_path.display()
        )
    })?;

    let subroute_count = subroutes.len();
    let mut top_subroutes = agent_continue_subroute_reports(subroutes, agent_continue_rows_written);
    top_subroutes.truncate(16);
    let report = AgentContinueActiveTurnStateTraceReport {
        report_kind: "agent_continue_active_turn_state_v1",
        mode: "phase_atom_trace_to_agent_continue_active_turn_state_audit_only",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        output_trace_path: output_trace_path.display().to_string(),
        total_rows_seen,
        agent_continue_rows_written,
        rows_with_verifier_label,
        verifier_true_rows,
        verifier_false_rows,
        rows_with_result_atoms,
        rows_with_shadow_request,
        exact_cache_hits,
        exact_cache_misses_over_cache: agent_continue_rows_written.saturating_sub(exact_cache_hits),
        token_events,
        provider_cost_events,
        estimated_cost_events,
        estimated_total_tokens,
        estimated_total_cost_microusd,
        subroute_count,
        top_subroutes,
        raw_prompt_text_written: false,
        raw_answer_text_written: false,
        compile_allowed: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "audit only: filters phase-atom traces into agent_continue active_turn_state rows with atoms/fingerprints/cost flags only; no raw prompt/answer text, no .nwpc compile, no promotion, no serving, no local_accept, no money claim, and no legacy nwrb/role-binding backend",
    };
    write_json_file(&report_path, &report)?;
    println!("agent_continue_active_turn_state_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", report.output_trace_path);
    println!("  total_rows_seen: {}", report.total_rows_seen);
    println!(
        "  agent_continue_rows_written: {}",
        report.agent_continue_rows_written
    );
    println!("  verifier_true_rows: {}", report.verifier_true_rows);
    println!("  verifier_false_rows: {}", report.verifier_false_rows);
    println!(
        "  rows_with_shadow_request: {}",
        report.rows_with_shadow_request
    );
    println!(
        "  exact_cache_misses_over_cache: {}",
        report.exact_cache_misses_over_cache
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    Ok(())
}

pub(crate) fn run_phase_stream_agent_continue_command_result_followup_pack_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_AGENT_CONTINUE_COMMAND_RESULT_FOLLOWUP_PACK_REPORT)
    });
    let output_trace_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_AGENT_CONTINUE_COMMAND_RESULT_FOLLOWUP_PACK_JSONL)
    });
    let trace_paths = args.map(PathBuf::from).collect::<Vec<_>>();
    if trace_paths.is_empty() {
        return Err("no tool-status phase atom trace paths provided".to_owned());
    }

    let mut total_rows_seen = 0usize;
    let mut agent_continue_rows_written = 0usize;
    let mut rows_with_verifier_label = 0usize;
    let mut verifier_true_rows = 0usize;
    let mut verifier_false_rows = 0usize;
    let mut rows_with_result_atoms = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut exact_cache_hits = 0usize;
    let mut token_events = 0usize;
    let mut provider_cost_events = 0usize;
    let mut estimated_cost_events = 0usize;
    let mut estimated_total_tokens = 0usize;
    let mut estimated_total_cost_microusd = 0u64;
    let mut seen_exact_cache_keys = BTreeSet::<String>::new();
    let mut subroutes = BTreeMap::<String, AgentContinueSubrouteState>::new();
    let mut output = String::new();

    for trace_path in &trace_paths {
        let text = std::fs::read_to_string(trace_path).map_err(|error| {
            format!(
                "failed to read command-result followup source trace '{}': {error}",
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
                    "agent_continue_command_result_followup_pack_progress: rows_seen={} rows_written={}",
                    total_rows_seen, agent_continue_rows_written
                );
            }
            let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse command-result followup source trace '{}' line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
            if !is_agent_continue_command_result_followup_source_row(&row) {
                continue;
            }

            let output_row = build_agent_continue_active_turn_row_with_forced_subroute(
                trace_path,
                &row,
                agent_continue_rows_written,
                &mut seen_exact_cache_keys,
                "command_result_followup",
            );
            let exact_cache_hit = output_row
                .get("exact_cache_hit")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            exact_cache_hits += usize::from(exact_cache_hit);
            let verifier_label = output_row
                .get("verified_safe_accept")
                .and_then(serde_json::Value::as_bool);
            rows_with_verifier_label += usize::from(verifier_label.is_some());
            verifier_true_rows += usize::from(verifier_label == Some(true));
            verifier_false_rows += usize::from(verifier_label == Some(false));
            let has_result_atoms = output_row
                .get("result_atoms")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| !items.is_empty());
            rows_with_result_atoms += usize::from(has_result_atoms);
            let has_shadow_request = output_row
                .get("nando_shadow_request")
                .and_then(serde_json::Value::as_object)
                .is_some();
            rows_with_shadow_request += usize::from(has_shadow_request);
            let token_cost = GenericTokenCost {
                total_tokens: json_at(&output_row, &["token_cost", "total_tokens"])
                    .and_then(serde_json::Value::as_u64)
                    .map_or(0, |value| value as usize),
                total_cost_microusd: json_at(&output_row, &["token_cost", "total_cost_microusd"])
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                evidence_missing: false,
                token_evidence_missing: false,
                cost_evidence_missing: false,
            };
            let has_tokens = token_cost.total_tokens > 0;
            let has_provider_cost = output_row
                .get("provider_cost_microusd")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                > 0;
            let has_estimated_cost = token_cost.total_cost_microusd > 0 && !has_provider_cost;
            token_events += usize::from(has_tokens);
            provider_cost_events += usize::from(has_provider_cost);
            estimated_cost_events += usize::from(has_estimated_cost);
            estimated_total_tokens = estimated_total_tokens.saturating_add(token_cost.total_tokens);
            estimated_total_cost_microusd =
                estimated_total_cost_microusd.saturating_add(token_cost.total_cost_microusd);

            observe_agent_continue_subroute(
                &mut subroutes,
                "command_result_followup",
                AgentContinueSubrouteObservation {
                    exact_cache_hit,
                    verifier_label,
                    has_result_atoms,
                    has_shadow_request,
                    has_tokens,
                    has_provider_cost,
                    has_estimated_cost,
                    token_cost: &token_cost,
                },
            );

            output.push_str(&serde_json::to_string(&output_row).map_err(|error| {
                format!("failed to serialize command-result followup active-turn row: {error}")
            })?);
            output.push('\n');
            agent_continue_rows_written += 1;
        }
    }

    if let Some(parent) = output_trace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create command-result followup trace directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&output_trace_path, output).map_err(|error| {
        format!(
            "failed to write command-result followup active-turn trace '{}': {error}",
            output_trace_path.display()
        )
    })?;

    let subroute_count = subroutes.len();
    let top_subroutes = agent_continue_subroute_reports(subroutes, agent_continue_rows_written);
    let report = AgentContinueActiveTurnStateTraceReport {
        report_kind: "agent_continue_command_result_followup_pack_v1",
        mode: "tool_status_phase_atoms_to_agent_continue_command_result_followup_audit_only",
        input_trace_paths: trace_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        output_trace_path: output_trace_path.display().to_string(),
        total_rows_seen,
        agent_continue_rows_written,
        rows_with_verifier_label,
        verifier_true_rows,
        verifier_false_rows,
        rows_with_result_atoms,
        rows_with_shadow_request,
        exact_cache_hits,
        exact_cache_misses_over_cache: agent_continue_rows_written.saturating_sub(exact_cache_hits),
        token_events,
        provider_cost_events,
        estimated_cost_events,
        estimated_total_tokens,
        estimated_total_cost_microusd,
        subroute_count,
        top_subroutes,
        raw_prompt_text_written: false,
        raw_answer_text_written: false,
        compile_allowed: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "audit only: repacks existing tool_status phase rows into the agent_continue command_result_followup active-turn state; copies observable result/verifier atoms and creates a phase-center shadow request, but does not compile .nwpc, promote, serve, local_accept, claim money, use target/proof authority, or revive legacy nwrb/role-binding backend",
    };
    write_json_file(&report_path, &report)?;
    println!("agent_continue_command_result_followup_pack_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  output_trace_path: {}", report.output_trace_path);
    println!("  total_rows_seen: {}", report.total_rows_seen);
    println!(
        "  agent_continue_rows_written: {}",
        report.agent_continue_rows_written
    );
    println!("  verifier_true_rows: {}", report.verifier_true_rows);
    println!("  verifier_false_rows: {}", report.verifier_false_rows);
    println!(
        "  rows_with_shadow_request: {}",
        report.rows_with_shadow_request
    );
    println!(
        "  exact_cache_misses_over_cache: {}",
        report.exact_cache_misses_over_cache
    );
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    Ok(())
}

pub(crate) fn run_phase_stream_agent_continue_subroute_scoreboard_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTINUE_SUBROUTE_SCOREBOARD_REPORT));
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTINUE_ACTIVE_TURN_STATE_JSONL));
    let text = std::fs::read_to_string(&input_trace_path).map_err(|error| {
        format!(
            "failed to read agent_continue active-turn trace '{}': {error}",
            input_trace_path.display()
        )
    })?;

    let mut total_rows = 0usize;
    let mut exact_cache_hits = 0usize;
    let mut rows_with_verifier_label = 0usize;
    let mut verifier_true_rows = 0usize;
    let mut verifier_false_rows = 0usize;
    let mut rows_with_result_atoms = 0usize;
    let mut rows_with_shadow_request = 0usize;
    let mut subroutes = BTreeMap::<String, AgentContinueSubrouteState>::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        total_rows += 1;
        if total_rows == 1 || total_rows.is_multiple_of(5000) {
            println!(
                "agent_continue_subroute_scoreboard_progress: rows_scored={}",
                total_rows
            );
        }
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse agent_continue active-turn trace '{}' line {}: {error}",
                input_trace_path.display(),
                line_index + 1
            )
        })?;
        let exact_cache_hit = row
            .get("exact_cache_hit")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        exact_cache_hits += usize::from(exact_cache_hit);
        let verifier_label = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool);
        rows_with_verifier_label += usize::from(verifier_label.is_some());
        verifier_true_rows += usize::from(verifier_label == Some(true));
        verifier_false_rows += usize::from(verifier_label == Some(false));
        let has_result_atoms = row
            .get("result_atoms")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| !items.is_empty());
        rows_with_result_atoms += usize::from(has_result_atoms);
        let has_shadow_request = row
            .get("nando_shadow_request")
            .and_then(serde_json::Value::as_object)
            .is_some();
        rows_with_shadow_request += usize::from(has_shadow_request);
        let token_cost = GenericTokenCost {
            total_tokens: json_at(&row, &["token_cost", "total_tokens"])
                .and_then(serde_json::Value::as_u64)
                .map_or(0, |value| value as usize),
            total_cost_microusd: json_at(&row, &["token_cost", "total_cost_microusd"])
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            evidence_missing: false,
            token_evidence_missing: false,
            cost_evidence_missing: false,
        };
        let has_tokens = token_cost.total_tokens > 0;
        let has_provider_cost = row
            .get("provider_cost_microusd")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            > 0;
        let has_estimated_cost = token_cost.total_cost_microusd > 0 && !has_provider_cost;
        let subroute_hint = row
            .get("subroute_hint")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("artifact_progress");
        observe_agent_continue_subroute(
            &mut subroutes,
            subroute_hint,
            AgentContinueSubrouteObservation {
                exact_cache_hit,
                verifier_label,
                has_result_atoms,
                has_shadow_request,
                has_tokens,
                has_provider_cost,
                has_estimated_cost,
                token_cost: &token_cost,
            },
        );
    }

    let subroutes = agent_continue_subroute_reports(subroutes, total_rows);
    let rows_ready_for_subroute_mining = subroutes
        .iter()
        .filter(|row| row.ready_for_subroute_mining)
        .map(|row| row.rows)
        .sum();
    let report = AgentContinueSubrouteScoreboardReport {
        report_kind: "agent_continue_subroute_scoreboard_v1",
        mode: "active_turn_state_subroute_scoreboard_audit_only",
        input_trace_path: input_trace_path.display().to_string(),
        total_rows,
        exact_cache_hits,
        exact_cache_misses_over_cache: total_rows.saturating_sub(exact_cache_hits),
        rows_with_verifier_label,
        verifier_true_rows,
        verifier_false_rows,
        rows_with_result_atoms,
        rows_with_shadow_request,
        rows_ready_for_subroute_mining,
        subroute_count: subroutes.len(),
        subroutes,
        compile_allowed: false,
        local_accept_enabled: false,
        product_runtime_changed: false,
        serving_runtime_changed: false,
        market_money_claim_allowed: false,
        forbidden_flags: ForbiddenFlags {
            target_id_used: false,
            proof_rule_id_authority_used: false,
            concrete_x_lookup_used: false,
            manual_local_out_t_used: false,
            hidden_frame_id_or_bind_x_used: false,
            legacy_backend_used: false,
        },
        boundary: "scoreboard only: ranks agent_continue subroutes for future verifier-bound phase-center mining; no .nwpc compile, no promote, no serving, no local_accept, no money claim, and no legacy nwrb/role-binding backend",
    };
    write_json_file(&report_path, &report)?;
    println!("agent_continue_subroute_scoreboard_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  input_trace_path: {}", report.input_trace_path);
    println!("  total_rows: {}", report.total_rows);
    println!("  subroute_count: {}", report.subroute_count);
    println!(
        "  rows_ready_for_subroute_mining: {}",
        report.rows_ready_for_subroute_mining
    );
    println!("  verifier_true_rows: {}", report.verifier_true_rows);
    println!("  verifier_false_rows: {}", report.verifier_false_rows);
    println!("  local_accept_enabled: {}", report.local_accept_enabled);
    Ok(())
}

fn is_agent_continue_phase_atom_row(row: &serde_json::Value) -> bool {
    let traffic_source = row
        .get("traffic_source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if traffic_source.contains("agent_continue") {
        return true;
    }
    phase_atom_string_vec(row, "request_atoms")
        .into_iter()
        .chain(phase_atom_string_vec(row, "action_atoms"))
        .chain(phase_atom_string_vec(row, "route_hint_atoms"))
        .any(|atom| {
            atom == "request_route_family:agent_continue_execute"
                || atom == "route_operator:agent_continue_execute"
                || atom == "route_hint_from_traffic_source:agent_continue_execute"
        })
}

fn is_agent_continue_command_result_followup_source_row(row: &serde_json::Value) -> bool {
    let traffic_source = row
        .get("traffic_source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let source_schema_version = row
        .get("source_schema_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let has_tool_status_source = traffic_source.contains("tool_status")
        || source_schema_version.contains("tool_status")
        || phase_atom_string_vec(row, "route_hint_atoms")
            .iter()
            .any(|atom| atom == "route_hint:tool_status_parse");
    let has_result_atoms = row
        .get("result_atoms")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let has_shadow_request = row
        .get("nando_shadow_request")
        .and_then(serde_json::Value::as_object)
        .is_some();
    has_tool_status_source && has_result_atoms && has_shadow_request
}

fn build_agent_continue_active_turn_row(
    input_path: &Path,
    row: &serde_json::Value,
    row_index: usize,
    seen_exact_cache_keys: &mut BTreeSet<String>,
) -> serde_json::Value {
    build_agent_continue_active_turn_row_with_optional_subroute(
        input_path,
        row,
        row_index,
        seen_exact_cache_keys,
        None,
    )
}

fn build_agent_continue_active_turn_row_with_forced_subroute(
    input_path: &Path,
    row: &serde_json::Value,
    row_index: usize,
    seen_exact_cache_keys: &mut BTreeSet<String>,
    forced_subroute_hint: &'static str,
) -> serde_json::Value {
    build_agent_continue_active_turn_row_with_optional_subroute(
        input_path,
        row,
        row_index,
        seen_exact_cache_keys,
        Some(forced_subroute_hint),
    )
}

fn build_agent_continue_active_turn_row_with_optional_subroute(
    input_path: &Path,
    row: &serde_json::Value,
    row_index: usize,
    seen_exact_cache_keys: &mut BTreeSet<String>,
    forced_subroute_hint: Option<&'static str>,
) -> serde_json::Value {
    let request_atoms = phase_atom_string_vec(row, "request_atoms");
    let state_atoms = phase_atom_string_vec(row, "state_atoms");
    let action_atoms = phase_atom_string_vec(row, "action_atoms");
    let tool_atoms = phase_atom_string_vec(row, "tool_atoms");
    let mut result_atoms = phase_atom_string_vec(row, "result_atoms");
    let result_atom_source = if result_atoms.is_empty() {
        result_atoms = phase_atom_string_vec(row, "shadow_payload_atoms");
        "shadow_payload_atoms"
    } else {
        "result_atoms"
    };
    let route_hint_atoms = phase_atom_string_vec(row, "route_hint_atoms");
    let traffic_source = row
        .get("traffic_source")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown_traffic_source");
    let subroute_hint = forced_subroute_hint.unwrap_or_else(|| {
        infer_agent_continue_subroute_hint(
            traffic_source,
            &request_atoms,
            &state_atoms,
            &action_atoms,
            &tool_atoms,
            &result_atoms,
            &route_hint_atoms,
        )
    });
    let output_traffic_source = if forced_subroute_hint == Some("command_result_followup") {
        format!("{traffic_source}::agent_continue_command_result_followup_pack_v1")
    } else {
        traffic_source.to_owned()
    };
    let mut output_request_atoms = request_atoms.clone();
    let mut output_action_atoms = if forced_subroute_hint == Some("command_result_followup") {
        action_atoms
            .iter()
            .filter(|atom| !atom.starts_with("action_family:"))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        action_atoms.clone()
    };
    let mut output_route_hint_atoms = route_hint_atoms.clone();
    if forced_subroute_hint == Some("command_result_followup") {
        output_request_atoms.push("request_route_family:agent_continue_execute".to_owned());
        output_action_atoms.push("action_family:planning".to_owned());
        output_action_atoms.push("action:continue_after_tool_result".to_owned());
        output_action_atoms.push("route_operator:agent_continue_execute".to_owned());
        output_action_atoms.push("subroute_operator:command_result_followup".to_owned());
        output_route_hint_atoms.push("route_hint:agent_continue_execute".to_owned());
        output_route_hint_atoms.push("subroute_hint:command_result_followup".to_owned());
    }
    output_request_atoms.sort();
    output_request_atoms.dedup();
    output_action_atoms.sort();
    output_action_atoms.dedup();
    output_route_hint_atoms.sort();
    output_route_hint_atoms.dedup();
    let request_fingerprint = json_string(row, &["request_fingerprint"])
        .unwrap_or_else(|| format!("agent_continue_active_turn:{row_index:08}"));
    let exact_cache_key =
        json_string(row, &["exact_cache_key"]).unwrap_or_else(|| request_fingerprint.clone());
    let exact_cache_hit = !seen_exact_cache_keys.insert(exact_cache_key.clone());
    let external_provider_correlation_keys = phase_atom_external_provider_correlation_keys(row);
    let provider_correlation_ready = !external_provider_correlation_keys.is_empty();
    let verified_safe_accept = row
        .get("verified_safe_accept")
        .and_then(serde_json::Value::as_bool);
    let verifier_label = match verified_safe_accept {
        Some(true) => serde_json::Value::String("safe_accept".to_owned()),
        Some(false) => serde_json::Value::String("reject".to_owned()),
        None => serde_json::Value::Null,
    };
    let token_cost = phase_atom_binary_token_cost(row);
    let provider_cost_microusd = row
        .get("provider_cost_microusd")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let nando_shadow_request = if forced_subroute_hint == Some("command_result_followup") {
        agent_continue_command_result_followup_shadow_request(
            &output_request_atoms,
            &state_atoms,
            &output_action_atoms,
            &tool_atoms,
            &result_atoms,
            &output_route_hint_atoms,
            &exact_cache_key,
        )
    } else {
        row.get("nando_shadow_request")
            .and_then(|value| value.as_object().map(|_| value.clone()))
            .unwrap_or_else(|| {
                if result_atoms.is_empty() {
                    serde_json::Value::Null
                } else {
                    agent_continue_observable_shadow_request(
                        subroute_hint,
                        &output_request_atoms,
                        &state_atoms,
                        &output_action_atoms,
                        &tool_atoms,
                        &result_atoms,
                        &output_route_hint_atoms,
                        &exact_cache_key,
                    )
                }
            })
    };
    let state_before_atoms = agent_continue_state_before_atoms(
        &output_request_atoms,
        &state_atoms,
        &tool_atoms,
        &output_route_hint_atoms,
        subroute_hint,
    );
    let subroute_evidence_atoms = agent_continue_subroute_evidence_atoms(
        subroute_hint,
        &output_request_atoms,
        &state_atoms,
        &tool_atoms,
        &result_atoms,
        &output_route_hint_atoms,
    );

    serde_json::json!({
        "schema_version": "agent_continue_active_turn_state_v1",
        "source_schema_version": row.get("schema_version").and_then(serde_json::Value::as_str),
        "input_trace_path": input_path.display().to_string(),
        "source_trace_id": row.get("trace_id").and_then(serde_json::Value::as_str),
        "event_timestamp": row.get("event_timestamp").and_then(serde_json::Value::as_str),
        "traffic_source": output_traffic_source,
        "action_family": "planning",
        "route_hint": "agent_continue_execute",
        "subroute_hint": subroute_hint,
        "subroute_evidence_atoms": subroute_evidence_atoms,
        "result_atom_source": result_atom_source,
        "request_fingerprint": request_fingerprint,
        "exact_cache_key": exact_cache_key,
        "exact_cache_hit": exact_cache_hit,
        "external_provider_correlation_keys": external_provider_correlation_keys,
        "provider_correlation_ready": provider_correlation_ready,
        "verified_safe_accept": verified_safe_accept,
        "verifier_label": verifier_label,
        "nando_shadow_request": nando_shadow_request,
        "state_before_atoms": state_before_atoms,
        "request_atoms": output_request_atoms,
        "state_atoms": state_atoms,
        "action_atoms": output_action_atoms,
        "tool_atoms": tool_atoms,
        "result_atoms": result_atoms,
        "route_hint_atoms": output_route_hint_atoms,
        "rows_with_result_atoms": !result_atoms.is_empty(),
        "has_shadow_request": nando_shadow_request.as_object().is_some(),
        "ready_for_route_family_mining": row.get("ready_for_route_family_mining").and_then(serde_json::Value::as_bool).unwrap_or(false),
        "ready_for_existing_shadow_scoring": nando_shadow_request.as_object().is_some(),
        "token_cost": {
            "total_tokens": token_cost.total_tokens,
            "total_cost_microusd": token_cost.total_cost_microusd,
            "token_evidence_missing": token_cost.token_evidence_missing,
            "cost_evidence_missing": token_cost.cost_evidence_missing
        },
        "provider_cost_microusd": provider_cost_microusd,
        "forbidden_fields_absent": {
            "raw_prompt_text": true,
            "raw_answer_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true,
            "legacy_nwrb_backend": true
        }
    })
}

fn agent_continue_command_result_followup_shadow_request(
    request_atoms: &[String],
    state_atoms: &[String],
    action_atoms: &[String],
    tool_atoms: &[String],
    result_atoms: &[String],
    route_hint_atoms: &[String],
    source_exact_cache_key: &str,
) -> serde_json::Value {
    let shadow_route_key = "agent_continue_command_result_followup";
    let shadow_profile_id = "phase_center_agent_continue_command_result_followup_v1";
    let shadow_source_atoms = request_atoms
        .iter()
        .chain(state_atoms.iter())
        .chain(action_atoms.iter())
        .chain(tool_atoms.iter())
        .chain(route_hint_atoms.iter())
        .collect::<Vec<_>>();
    let mut seen_shadow_centers = BTreeSet::new();
    let active_fringe = shadow_source_atoms
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
        .take(40)
        .collect::<Vec<_>>();
    let mut target_atoms = result_atoms
        .iter()
        .filter(|atom| !atom.starts_with("output_hash64:"))
        .cloned()
        .collect::<Vec<_>>();
    target_atoms.sort();
    target_atoms.dedup();
    let slots = target_atoms
        .into_iter()
        .take(8)
        .enumerate()
        .map(|(slot_id, atom)| {
            let lane_id =
                stable_fingerprint(["command_result_followup_target", atom.as_str()]) % 4096;
            serde_json::json!({
                "binding_output_slot": slot_id as u64,
                "slot_kind": "command_result_followup_target_atom",
                "value_band": atom,
                "positive_impulses": [
                    {
                        "lane_id": lane_id,
                        "strength": 1
                    }
                ],
                "negative_impulses": []
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "route_key": shadow_route_key,
        "profile_id": shadow_profile_id,
        "exact_cache_key": format!("agent_continue_command_result_followup:{source_exact_cache_key}"),
        "active_fringe": active_fringe,
        "slots": slots,
        "source": "agent_continue_command_result_followup_observable_tool_status_atoms_v1",
        "forbidden_fields_absent": {
            "raw_tool_output": true,
            "raw_request_text": true,
            "raw_response_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    })
}

fn agent_continue_observable_shadow_request(
    subroute_hint: &str,
    request_atoms: &[String],
    state_atoms: &[String],
    action_atoms: &[String],
    tool_atoms: &[String],
    result_atoms: &[String],
    route_hint_atoms: &[String],
    source_exact_cache_key: &str,
) -> serde_json::Value {
    let shadow_source_atoms = request_atoms
        .iter()
        .chain(state_atoms.iter())
        .chain(action_atoms.iter())
        .chain(tool_atoms.iter())
        .chain(route_hint_atoms.iter())
        .collect::<Vec<_>>();
    let mut seen_shadow_centers = BTreeSet::new();
    let active_fringe = shadow_source_atoms
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
    let mut target_atoms = result_atoms
        .iter()
        .filter(|atom| {
            !atom.starts_with("output_hash64:")
                && !atom.starts_with("provider_correlation:")
                && !atom.starts_with("provider_request_id:")
                && !atom.starts_with("provider_response_id:")
                && !atom.starts_with("external_provider_request_id:")
        })
        .cloned()
        .collect::<Vec<_>>();
    target_atoms.sort();
    target_atoms.dedup();
    let slots = target_atoms
        .into_iter()
        .take(8)
        .enumerate()
        .map(|(slot_id, atom)| {
            let lane_id = stable_fingerprint([
                "agent_continue_observable_target",
                subroute_hint,
                atom.as_str(),
            ]) % 4096;
            serde_json::json!({
                "binding_output_slot": slot_id as u64,
                "slot_kind": "agent_continue_observable_target_atom",
                "value_band": atom,
                "positive_impulses": [
                    {
                        "lane_id": lane_id,
                        "strength": 1
                    }
                ],
                "negative_impulses": []
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "route_key": "agent_continue_observable_shadow",
        "profile_id": format!("phase_center_agent_continue_{subroute_hint}_observable_v1"),
        "exact_cache_key": source_exact_cache_key,
        "active_fringe": active_fringe,
        "slots": slots,
        "source": "agent_continue_observable_shadow_payload_atoms_v1",
        "forbidden_fields_absent": {
            "raw_tool_output": true,
            "raw_request_text": true,
            "raw_response_text": true,
            "target_id": true,
            "proof_rule_id": true,
            "concrete_x_lookup": true,
            "manual_local_out_t": true
        }
    })
}

fn infer_agent_continue_subroute_hint(
    traffic_source: &str,
    request_atoms: &[String],
    state_atoms: &[String],
    action_atoms: &[String],
    tool_atoms: &[String],
    result_atoms: &[String],
    route_hint_atoms: &[String],
) -> &'static str {
    let atom_groups = [
        request_atoms,
        state_atoms,
        action_atoms,
        tool_atoms,
        result_atoms,
        route_hint_atoms,
    ];
    if agent_continue_atoms_have_signal(&atom_groups, traffic_source, &["git", "commit", "branch"])
    {
        "git_status_summary"
    } else if agent_continue_atoms_have_signal(
        &atom_groups,
        traffic_source,
        &["patch", "diff", "mutation", "edit", "write", "apply"],
    ) {
        "patch_applied_state"
    } else if agent_continue_atoms_have_signal(
        &atom_groups,
        traffic_source,
        &[
            "test", "cargo", "check", "compile", "panic", "failed", "passed",
        ],
    ) {
        "test_fix_iteration"
    } else if agent_continue_atoms_have_signal(
        &atom_groups,
        traffic_source,
        &["file", "inspect", "read", "rg", "sed", "cat", "path"],
    ) {
        "file_inspection_result"
    } else if agent_continue_atoms_have_signal(
        &atom_groups,
        traffic_source,
        &[
            "report", "doc", "docs", "metric", "metrics", "json", "jsonl",
        ],
    ) {
        "report_sync_after_action"
    } else if agent_continue_atoms_have_signal(
        &atom_groups,
        traffic_source,
        &["plan", "artifact", "progress"],
    ) {
        "artifact_progress"
    } else if agent_continue_atoms_have_signal(
        &atom_groups,
        traffic_source,
        &["exec", "command", "tool", "toolcall"],
    ) {
        "command_result_followup"
    } else {
        "artifact_progress"
    }
}

fn agent_continue_atoms_have_signal(
    atom_groups: &[&[String]],
    traffic_source: &str,
    signals: &[&str],
) -> bool {
    atom_groups
        .iter()
        .flat_map(|atoms| atoms.iter().map(String::as_str))
        .chain(std::iter::once(traffic_source))
        .any(|atom| agent_continue_atom_has_signal(atom, signals))
}

fn agent_continue_atom_has_signal(atom: &str, signals: &[&str]) -> bool {
    let lower = atom.to_ascii_lowercase();
    lower
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .any(|part| signals.contains(&part))
}

fn agent_continue_state_before_atoms(
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    route_hint_atoms: &[String],
    subroute_hint: &str,
) -> Vec<String> {
    let mut atoms = BTreeSet::new();
    atoms.insert(format!("active_turn_subroute:{subroute_hint}"));
    for atom in request_atoms
        .iter()
        .chain(state_atoms)
        .chain(tool_atoms)
        .chain(route_hint_atoms)
    {
        atoms.insert(atom.clone());
    }
    atoms.into_iter().collect()
}

fn agent_continue_subroute_evidence_atoms(
    subroute_hint: &str,
    request_atoms: &[String],
    state_atoms: &[String],
    tool_atoms: &[String],
    result_atoms: &[String],
    route_hint_atoms: &[String],
) -> Vec<String> {
    let mut atoms = Vec::new();
    let needles: &[&str] = match subroute_hint {
        "git_status_summary" => &["git", "commit", "branch"],
        "patch_applied_state" => &["patch", "diff", "mutation", "edit", "write", "apply"],
        "test_fix_iteration" => &[
            "test", "cargo", "check", "compile", "panic", "failed", "passed",
        ],
        "file_inspection_result" => &["file", "inspect", "read", "rg", "sed", "cat", "path"],
        "report_sync_after_action" => &[
            "report", "doc", "docs", "metric", "metrics", "json", "jsonl",
        ],
        "command_result_followup" => &["exec", "command", "tool", "toolcall"],
        _ => &["plan", "artifact", "progress"],
    };
    for atom in request_atoms
        .iter()
        .chain(state_atoms)
        .chain(tool_atoms)
        .chain(result_atoms)
        .chain(route_hint_atoms)
    {
        if agent_continue_atom_has_signal(atom, needles) {
            atoms.push(atom.clone());
            if atoms.len() >= 8 {
                break;
            }
        }
    }
    if atoms.is_empty() {
        atoms.push(format!("subroute_default:{subroute_hint}"));
    }
    atoms
}

fn observe_agent_continue_subroute(
    states: &mut BTreeMap<String, AgentContinueSubrouteState>,
    subroute_hint: &str,
    observation: AgentContinueSubrouteObservation<'_>,
) {
    let state =
        states
            .entry(subroute_hint.to_owned())
            .or_insert_with(|| AgentContinueSubrouteState {
                subroute_hint: subroute_hint.to_owned(),
                ..Default::default()
            });
    state.rows += 1;
    state.exact_cache_hits += usize::from(observation.exact_cache_hit);
    state.rows_with_verifier_label += usize::from(observation.verifier_label.is_some());
    state.verifier_true += usize::from(observation.verifier_label == Some(true));
    state.verifier_false += usize::from(observation.verifier_label == Some(false));
    if observation.verifier_label == Some(true) && !observation.exact_cache_hit {
        state.verifier_true_over_exact_cache_ceiling += 1;
        state.expected_tokens_saved_over_exact_cache = state
            .expected_tokens_saved_over_exact_cache
            .saturating_add(observation.token_cost.total_tokens);
        state.expected_cost_saved_microusd_over_exact_cache = state
            .expected_cost_saved_microusd_over_exact_cache
            .saturating_add(observation.token_cost.total_cost_microusd);
    }
    state.rows_with_result_atoms += usize::from(observation.has_result_atoms);
    state.rows_with_shadow_request += usize::from(observation.has_shadow_request);
    state.token_events += usize::from(observation.has_tokens);
    state.provider_cost_events += usize::from(observation.has_provider_cost);
    state.estimated_cost_events += usize::from(observation.has_estimated_cost);
}

fn agent_continue_subroute_reports(
    states: BTreeMap<String, AgentContinueSubrouteState>,
    total_rows: usize,
) -> Vec<AgentContinueSubrouteScoreboardRow> {
    let mut reports = states
        .into_values()
        .map(|state| {
            let ready_for_subroute_mining = state.verifier_true >= 20
                && state.verifier_false > 0
                && state.rows_with_result_atoms > 0
                && state.rows_with_shadow_request > 0
                && state.rows.saturating_sub(state.exact_cache_hits) > 0;
            let false_accept_risk = phase_atom_false_accept_risk(
                state.verifier_true,
                state.verifier_false,
                state.rows.saturating_sub(state.rows_with_verifier_label),
                ready_for_subroute_mining,
            );
            let recommended_next_action =
                agent_continue_subroute_recommended_next_action(&state, ready_for_subroute_mining);
            AgentContinueSubrouteScoreboardRow {
                subroute_hint: state.subroute_hint,
                rows: state.rows,
                traffic_share_milli: per_thousand(state.rows, total_rows),
                exact_cache_hits: state.exact_cache_hits,
                exact_cache_misses: state.rows.saturating_sub(state.exact_cache_hits),
                exact_cache_overlap_milli: per_thousand(state.exact_cache_hits, state.rows),
                verifier_true: state.verifier_true,
                verifier_false: state.verifier_false,
                rows_with_verifier_label: state.rows_with_verifier_label,
                rows_missing_verifier_label: state
                    .rows
                    .saturating_sub(state.rows_with_verifier_label),
                verifier_true_over_exact_cache_ceiling: state
                    .verifier_true_over_exact_cache_ceiling,
                rows_with_result_atoms: state.rows_with_result_atoms,
                rows_missing_result_atoms: state.rows.saturating_sub(state.rows_with_result_atoms),
                rows_with_shadow_request: state.rows_with_shadow_request,
                rows_missing_shadow_request: state
                    .rows
                    .saturating_sub(state.rows_with_shadow_request),
                token_events: state.token_events,
                provider_cost_events: state.provider_cost_events,
                estimated_cost_events: state.estimated_cost_events,
                expected_tokens_saved_over_exact_cache: state
                    .expected_tokens_saved_over_exact_cache,
                expected_cost_saved_microusd_over_exact_cache: state
                    .expected_cost_saved_microusd_over_exact_cache,
                ready_for_subroute_mining,
                false_accept_risk,
                recommended_next_action,
            }
        })
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| {
        right
            .verifier_true_over_exact_cache_ceiling
            .cmp(&left.verifier_true_over_exact_cache_ceiling)
            .then_with(|| right.rows.cmp(&left.rows))
            .then_with(|| left.subroute_hint.cmp(&right.subroute_hint))
    });
    reports
}

fn agent_continue_subroute_recommended_next_action(
    state: &AgentContinueSubrouteState,
    ready_for_subroute_mining: bool,
) -> &'static str {
    if ready_for_subroute_mining {
        "run_phase_center_mining_for_this_subroute"
    } else if state.verifier_true < 20 {
        "collect_verified_positive_active_turn_rows"
    } else if state.verifier_false == 0 {
        "collect_negative_or_background_verifier_rows_before_mining"
    } else if state.rows_with_result_atoms == 0 {
        "capture_result_atoms_before_subroute_mining"
    } else if state.rows_with_shadow_request == 0 {
        "attach_shadow_request_payload_before_subroute_mining"
    } else if state.rows.saturating_sub(state.exact_cache_hits) == 0 {
        "deprioritize_exact_cache_overlap_until_unique_rows_exist"
    } else {
        "keep_quarantined_and_inspect_state_action_atoms"
    }
}
