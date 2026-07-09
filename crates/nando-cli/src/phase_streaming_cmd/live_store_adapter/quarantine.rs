use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use super::state::LiveStoreStableDecisionLogWindow;

#[derive(Clone, Debug, Default)]
pub(super) struct LiveStoreStableDecisionLogCleanSuffix {
    pub(super) window: LiveStoreStableDecisionLogWindow,
    pub(super) last_quarantine_row_index: Option<usize>,
    matching_rows_seen: usize,
}

fn live_store_decision_log_row_matches_architecture(
    row: &serde_json::Value,
    architecture_key: &str,
) -> bool {
    super::super::json_string(row, &["architecture_version_key"]).as_deref()
        == Some(architecture_key)
}

fn live_store_decision_log_row_compatible_for_quarantine(
    row: &serde_json::Value,
    architecture_key: &str,
) -> bool {
    super::super::json_string(row, &["architecture_version_key"])
        .filter(|value| !value.is_empty())
        .is_none_or(|value| value == architecture_key)
}

fn live_store_decision_log_score_candidate_count(row: &serde_json::Value) -> usize {
    row.get("decisions")
        .and_then(serde_json::Value::as_array)
        .map(|decisions| {
            decisions
                .iter()
                .filter(|decision| {
                    super::super::json_bool(decision, &["score_candidate"]) == Some(true)
                        && super::super::json_bool(decision, &["product_hot_profile_quarantined"])
                            != Some(true)
                })
                .count()
        })
        .unwrap_or(0)
}

fn live_store_decision_log_local_accept_count(row: &serde_json::Value) -> usize {
    row.get("decisions")
        .and_then(serde_json::Value::as_array)
        .map(|decisions| {
            decisions
                .iter()
                .filter(|decision| {
                    super::super::json_bool(decision, &["local_accept"]) == Some(true)
                        && super::super::json_bool(decision, &["product_hot_profile_quarantined"])
                            != Some(true)
                })
                .count()
        })
        .unwrap_or(0)
}

fn live_store_decision_log_non_exact_false_profile_ids(row: &serde_json::Value) -> Vec<u32> {
    if super::super::json_bool(row, &["verified_safe_accept"]).unwrap_or(false)
        || super::super::json_bool(row, &["exact_cache_hit"]).unwrap_or(false)
    {
        return Vec::new();
    }
    row.get("decisions")
        .and_then(serde_json::Value::as_array)
        .map(|decisions| {
            decisions
                .iter()
                .filter(|decision| {
                    super::super::json_bool(decision, &["score_candidate"]) == Some(true)
                })
                .filter_map(|decision| {
                    super::super::json_u64(decision, &["profile_id"])
                        .and_then(|id| u32::try_from(id).ok())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn live_store_stable_decision_log_non_exact_false_profile_ids(
    decision_log_path: &Path,
    architecture_key: &str,
) -> Result<BTreeSet<u32>, String> {
    let mut profile_ids = BTreeSet::new();
    if !decision_log_path.exists() {
        return Ok(profile_ids);
    }
    let file = File::open(decision_log_path).map_err(|error| {
        format!(
            "failed to open stable append live-tail decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    for line in io::BufReader::new(file).lines() {
        let line = line.map_err(|error| {
            format!(
                "failed to read stable append live-tail decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if !live_store_decision_log_row_compatible_for_quarantine(&row, architecture_key) {
            continue;
        }
        profile_ids.extend(live_store_decision_log_non_exact_false_profile_ids(&row));
    }
    Ok(profile_ids)
}

pub(super) fn live_store_observe_stable_decision_log_row(
    window: &mut LiveStoreStableDecisionLogWindow,
    row: &serde_json::Value,
    architecture_key: &str,
) {
    if !live_store_decision_log_row_matches_architecture(row, architecture_key) {
        return;
    }
    let score_candidate_count = live_store_decision_log_score_candidate_count(row);
    let local_accept_count = live_store_decision_log_local_accept_count(row);
    let tokens = super::super::json_u64(row, &["tokens"]).unwrap_or(0);
    let cost_microusd = super::super::json_u64(row, &["cost_microusd"]).unwrap_or(tokens);
    let verified_safe_accept =
        super::super::json_bool(row, &["verified_safe_accept"]).unwrap_or(false);
    let exact_cache_hit = super::super::json_bool(row, &["exact_cache_hit"]).unwrap_or(false);

    window.rows = window.rows.saturating_add(1);
    window.score_candidate_events = window
        .score_candidate_events
        .saturating_add(score_candidate_count);
    window.local_accept_events = window
        .local_accept_events
        .saturating_add(local_accept_count);
    window.total_tokens = window.total_tokens.saturating_add(tokens);
    window.total_cost_microusd = window.total_cost_microusd.saturating_add(cost_microusd);
    if score_candidate_count == 0 {
        return;
    }
    if verified_safe_accept {
        if !exact_cache_hit {
            window.unique_cpu_accepts_over_exact_cache =
                window.unique_cpu_accepts_over_exact_cache.saturating_add(1);
            window.tokens_saved = window.tokens_saved.saturating_add(tokens);
            window.cost_saved_microusd = window.cost_saved_microusd.saturating_add(cost_microusd);
        }
    } else {
        window.false_accepts = window.false_accepts.saturating_add(score_candidate_count);
    }
}

pub(super) fn live_store_observe_stable_decision_log_serving_row(
    window: &mut LiveStoreStableDecisionLogWindow,
    row: &serde_json::Value,
    architecture_key: &str,
) {
    if !live_store_decision_log_row_matches_architecture(row, architecture_key) {
        return;
    }
    let score_candidate_count = live_store_decision_log_score_candidate_count(row);
    let local_accept_count = live_store_decision_log_local_accept_count(row);
    let tokens = super::super::json_u64(row, &["tokens"]).unwrap_or(0);
    let cost_microusd = super::super::json_u64(row, &["cost_microusd"]).unwrap_or(tokens);
    let verified_safe_accept =
        super::super::json_bool(row, &["verified_safe_accept"]).unwrap_or(false);
    let exact_cache_hit = super::super::json_bool(row, &["exact_cache_hit"]).unwrap_or(false);

    window.rows = window.rows.saturating_add(1);
    window.score_candidate_events = window
        .score_candidate_events
        .saturating_add(score_candidate_count);
    window.local_accept_events = window
        .local_accept_events
        .saturating_add(local_accept_count);
    window.total_tokens = window.total_tokens.saturating_add(tokens);
    window.total_cost_microusd = window.total_cost_microusd.saturating_add(cost_microusd);
    if local_accept_count == 0 {
        return;
    }
    if verified_safe_accept {
        if !exact_cache_hit {
            window.unique_cpu_accepts_over_exact_cache =
                window.unique_cpu_accepts_over_exact_cache.saturating_add(1);
            window.tokens_saved = window.tokens_saved.saturating_add(tokens);
            window.cost_saved_microusd = window.cost_saved_microusd.saturating_add(cost_microusd);
        }
    } else {
        window.false_accepts = window.false_accepts.saturating_add(local_accept_count);
    }
}

pub(super) fn live_store_observe_stable_decision_log_clean_suffix_row(
    clean_suffix: &mut LiveStoreStableDecisionLogCleanSuffix,
    row: &serde_json::Value,
    architecture_key: &str,
) {
    if !live_store_decision_log_row_matches_architecture(row, architecture_key) {
        return;
    }
    clean_suffix.matching_rows_seen = clean_suffix.matching_rows_seen.saturating_add(1);
    let false_accept_count = if super::super::json_bool(row, &["verified_safe_accept"])
        .unwrap_or(false)
        || super::super::json_bool(row, &["exact_cache_hit"]).unwrap_or(false)
    {
        0
    } else {
        live_store_decision_log_score_candidate_count(row)
    };
    if false_accept_count > 0 {
        clean_suffix.window = LiveStoreStableDecisionLogWindow::default();
        clean_suffix.last_quarantine_row_index = Some(clean_suffix.matching_rows_seen);
        return;
    }
    live_store_observe_stable_decision_log_row(&mut clean_suffix.window, row, architecture_key);
}

pub(super) fn live_store_observe_stable_decision_log_serving_clean_suffix_row(
    clean_suffix: &mut LiveStoreStableDecisionLogCleanSuffix,
    row: &serde_json::Value,
    architecture_key: &str,
) {
    if !live_store_decision_log_row_matches_architecture(row, architecture_key) {
        return;
    }
    clean_suffix.matching_rows_seen = clean_suffix.matching_rows_seen.saturating_add(1);
    let false_accept_count = if super::super::json_bool(row, &["verified_safe_accept"])
        .unwrap_or(false)
        || super::super::json_bool(row, &["exact_cache_hit"]).unwrap_or(false)
    {
        0
    } else {
        live_store_decision_log_local_accept_count(row)
    };
    if false_accept_count > 0 {
        clean_suffix.window = LiveStoreStableDecisionLogWindow::default();
        clean_suffix.last_quarantine_row_index = Some(clean_suffix.matching_rows_seen);
        return;
    }
    live_store_observe_stable_decision_log_serving_row(
        &mut clean_suffix.window,
        row,
        architecture_key,
    );
}

pub(super) fn live_store_stable_decision_log_window_from_path(
    decision_log_path: &Path,
    architecture_key: &str,
) -> Result<LiveStoreStableDecisionLogWindow, String> {
    let mut window = LiveStoreStableDecisionLogWindow::default();
    if !decision_log_path.exists() {
        return Ok(window);
    }
    let file = File::open(decision_log_path).map_err(|error| {
        format!(
            "failed to open stable append live-tail decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    for line in io::BufReader::new(file).lines() {
        let line = line.map_err(|error| {
            format!(
                "failed to read stable append live-tail decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        live_store_observe_stable_decision_log_row(&mut window, &row, architecture_key);
    }
    Ok(window)
}

pub(super) fn live_store_stable_decision_log_serving_window_from_path(
    decision_log_path: &Path,
    architecture_key: &str,
) -> Result<LiveStoreStableDecisionLogWindow, String> {
    let mut window = LiveStoreStableDecisionLogWindow::default();
    if !decision_log_path.exists() {
        return Ok(window);
    }
    let file = File::open(decision_log_path).map_err(|error| {
        format!(
            "failed to open stable append live-tail decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    for line in io::BufReader::new(file).lines() {
        let line = line.map_err(|error| {
            format!(
                "failed to read stable append live-tail decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        live_store_observe_stable_decision_log_serving_row(&mut window, &row, architecture_key);
    }
    Ok(window)
}

pub(super) fn live_store_stable_decision_log_clean_suffix_from_path(
    decision_log_path: &Path,
    architecture_key: &str,
) -> Result<LiveStoreStableDecisionLogCleanSuffix, String> {
    let mut clean_suffix = LiveStoreStableDecisionLogCleanSuffix::default();
    if !decision_log_path.exists() {
        return Ok(clean_suffix);
    }
    let file = File::open(decision_log_path).map_err(|error| {
        format!(
            "failed to open stable append live-tail decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    for line in io::BufReader::new(file).lines() {
        let line = line.map_err(|error| {
            format!(
                "failed to read stable append live-tail decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        live_store_observe_stable_decision_log_clean_suffix_row(
            &mut clean_suffix,
            &row,
            architecture_key,
        );
    }
    Ok(clean_suffix)
}

pub(super) fn live_store_stable_decision_log_serving_clean_suffix_from_path(
    decision_log_path: &Path,
    architecture_key: &str,
) -> Result<LiveStoreStableDecisionLogCleanSuffix, String> {
    let mut clean_suffix = LiveStoreStableDecisionLogCleanSuffix::default();
    if !decision_log_path.exists() {
        return Ok(clean_suffix);
    }
    let file = File::open(decision_log_path).map_err(|error| {
        format!(
            "failed to open stable append live-tail decision log '{}': {error}",
            decision_log_path.display()
        )
    })?;
    for line in io::BufReader::new(file).lines() {
        let line = line.map_err(|error| {
            format!(
                "failed to read stable append live-tail decision log '{}': {error}",
                decision_log_path.display()
            )
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        live_store_observe_stable_decision_log_serving_clean_suffix_row(
            &mut clean_suffix,
            &row,
            architecture_key,
        );
    }
    Ok(clean_suffix)
}
