use super::LiveStorePreparedHotPackEval;

pub(super) fn live_store_prepared_hot_pack_blocker(
    pack_rows: usize,
    atom_eval: &LiveStorePreparedHotPackEval,
    prepared_eval: &LiveStorePreparedHotPackEval,
    margin_parity_mismatches: usize,
    decision_parity_mismatches: usize,
    prepared_p99_score_latency_ns: u128,
) -> &'static str {
    if pack_rows == 0 {
        "prepared_hot_pack_empty"
    } else if atom_eval.score_events != pack_rows || prepared_eval.score_events != pack_rows {
        "prepared_hot_pack_score_event_mismatch"
    } else if atom_eval.false_accepts > 0 || prepared_eval.false_accepts > 0 {
        "prepared_hot_pack_false_accepts"
    } else if margin_parity_mismatches > 0 || decision_parity_mismatches > 0 {
        "prepared_hot_pack_atom_prepared_parity_mismatch"
    } else if prepared_eval.score_candidate_events == 0 {
        "prepared_hot_pack_no_score_candidates"
    } else if prepared_eval.verifier_required_events != prepared_eval.score_candidate_events {
        "prepared_hot_pack_missing_verifier_required"
    } else if prepared_eval.local_accept_events > 0 {
        "prepared_hot_pack_local_accept_enabled"
    } else if prepared_p99_score_latency_ns > 1_000 {
        "prepared_hot_pack_p99_budget_exceeded"
    } else {
        "none"
    }
}

pub(super) fn live_store_memory_hot_worker_blocker(
    row_count: usize,
    eval: &LiveStorePreparedHotPackEval,
    p99_score_latency_ns: u128,
) -> &'static str {
    if row_count == 0 {
        "live_worker_memory_no_rows"
    } else if eval.score_events != row_count {
        "live_worker_memory_score_event_mismatch"
    } else if eval.false_accepts > 0 {
        "live_worker_memory_false_accepts"
    } else if eval.score_candidate_events == 0 {
        "live_worker_memory_no_score_candidates"
    } else if eval.verifier_required_events != eval.score_candidate_events {
        "live_worker_memory_missing_verifier_required"
    } else if eval.local_accept_events > 0 {
        "live_worker_memory_local_accept_enabled"
    } else if p99_score_latency_ns > 1_000 {
        "live_worker_memory_p99_budget_exceeded"
    } else {
        "none"
    }
}

pub(super) fn live_store_worker_thread_blocker(
    sent_events: usize,
    eval: &LiveStorePreparedHotPackEval,
    worker_p99_score_latency_ns: u128,
) -> &'static str {
    if sent_events == 0 {
        "live_worker_thread_no_sent_events"
    } else if eval.score_events != sent_events {
        "live_worker_thread_score_event_mismatch"
    } else if eval.false_accepts > 0 {
        "live_worker_thread_false_accepts"
    } else if eval.score_candidate_events == 0 {
        "live_worker_thread_no_score_candidates"
    } else if eval.verifier_required_events != eval.score_candidate_events {
        "live_worker_thread_missing_verifier_required"
    } else if eval.local_accept_events > 0 {
        "live_worker_thread_local_accept_enabled"
    } else if worker_p99_score_latency_ns > 1_000 {
        "live_worker_thread_score_p99_budget_exceeded"
    } else {
        "none"
    }
}
