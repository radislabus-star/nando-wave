use serde::Deserialize;
use sha2::{Digest, Sha256};

const SCHEMA: &str = "nando.stop-f8.v1";
const VERDICT: &str = "PASS_CONTROLLED_LIVE_SHADOW";

pub(super) struct F8FinalStatus {
    pub(super) provider_records: u64,
    pub(super) verified_receipts: u64,
    pub(super) full_phase_gain: u64,
    pub(super) search_gain: u64,
    pub(super) external_verdict: String,
    pub(super) commitments_sha256: String,
    pub(super) matched_p99_max_ns: u64,
    pub(super) no_match_p99_max_ns: u64,
    pub(super) hard_max_ns: u64,
    pub(super) hot_rss_bytes: u64,
}

#[derive(Deserialize)]
struct Receipt {
    schema: String,
    verdict: String,
    scope: Scope,
    live: Live,
    causal_controls: CausalControls,
    external_admission: ExternalAdmission,
    artifacts: Artifacts,
    performance: Performance,
    tests: Tests,
    next: String,
}

#[derive(Deserialize)]
struct Scope {
    seed: String,
    natural_operator: String,
    ordinary_traffic_coverage: String,
    m3: String,
    execution_authority: bool,
    local_accept: bool,
    active_change: u64,
}

#[derive(Deserialize)]
struct Live {
    generation_id_sha256: String,
    generation_checkpoint_sha256: String,
    provider_capture_index_sha256: String,
    shadow_ledger_sha256: String,
    provider_records: u64,
    verified_receipts: u64,
    false_accepts: u64,
    wrong_actions: u64,
    parity_mismatches: u64,
    raw_payload_bytes_persisted: u64,
    restart_sequence_reuse: u64,
    duplicate_terminal_us: u64,
    automatic_service_restarts: u64,
    deployed_binary_sha256: String,
}

#[derive(Deserialize)]
struct CausalControls {
    receipt_sha256: String,
    traffic_receipt_set_sha256: String,
    full_correct: u64,
    full_selected: u64,
    best_control_correct: u64,
    full_phase_gain: u64,
    search_gain: u64,
    gain_kind: String,
    support_future_overlap: u64,
    censored_semantic_updates: u64,
}

#[derive(Deserialize)]
struct ExternalAdmission {
    verdict: String,
    commitments_sha256: String,
    live_shadow_denominator: u64,
    live_verified_passes: u64,
    negative_denominator: u64,
    censored_denominator: u64,
    execution_authority: bool,
}

#[derive(Deserialize)]
struct Artifacts {
    phase_control_file_sha256: String,
    external_candidate_file_sha256: String,
}

#[derive(Deserialize)]
struct Performance {
    matched_p99_max_ns: u64,
    no_match_p99_max_ns: u64,
    hard_max_ns: u64,
    hot_rss_peak_delta_bytes: u64,
    matched_p99_budget_ns: u64,
    no_match_p99_budget_ns: u64,
    hard_ceiling_ns: u64,
    hot_rss_budget_bytes: u64,
}

#[derive(Deserialize)]
struct Tests {
    kernel_pass: u64,
    learning_pass: u64,
    persistence_pass: u64,
    admission_pass: u64,
    explicit_live_audit_pass: u64,
    runtime_pass: u64,
    transition_serving_pass: u64,
    transition_serving_known_fail: u64,
    gateway_control_pass: u64,
    baseline_failure_set_unchanged: bool,
    clippy_deny_warnings: String,
}

pub(super) fn parse_and_validate(
    source: &str,
    phase_control_source: &str,
    external_candidate_source: &str,
) -> Result<F8FinalStatus, String> {
    let receipt =
        serde_json::from_str::<Receipt>(source).map_err(|error| format!("F8 receipt: {error}"))?;
    require(receipt.schema == SCHEMA, "F8 schema")?;
    require(receipt.verdict == VERDICT, "F8 verdict")?;
    require(
        receipt.scope.seed == "CONTROLLED_SHADOW_ONLY"
            && receipt.scope.natural_operator == "NOT_EVALUATED"
            && receipt.scope.ordinary_traffic_coverage == "NOT_EVALUATED"
            && receipt.scope.m3 == "WATCH"
            && !receipt.scope.execution_authority
            && !receipt.scope.local_accept
            && receipt.scope.active_change == 0,
        "F8 scope boundary",
    )?;
    require(
        valid_sha256(&receipt.live.generation_id_sha256)
            && valid_sha256(&receipt.live.generation_checkpoint_sha256)
            && valid_sha256(&receipt.live.provider_capture_index_sha256)
            && valid_sha256(&receipt.live.shadow_ledger_sha256)
            && valid_sha256(&receipt.live.deployed_binary_sha256)
            && receipt.live.provider_records >= 4
            && receipt.live.verified_receipts >= 3
            && receipt.live.false_accepts == 0
            && receipt.live.wrong_actions == 0
            && receipt.live.parity_mismatches == 0
            && receipt.live.raw_payload_bytes_persisted == 0
            && receipt.live.restart_sequence_reuse == 0
            && receipt.live.duplicate_terminal_us <= 500_000
            && receipt.live.automatic_service_restarts == 0,
        "F8 live boundary",
    )?;
    require(
        valid_sha256(&receipt.causal_controls.receipt_sha256)
            && valid_sha256(&receipt.causal_controls.traffic_receipt_set_sha256)
            && receipt.causal_controls.full_correct == receipt.live.verified_receipts
            && receipt.causal_controls.full_selected == receipt.live.verified_receipts
            && receipt.causal_controls.best_control_correct == 0
            && receipt.causal_controls.full_phase_gain == receipt.live.verified_receipts
            && receipt.causal_controls.search_gain == 0
            && receipt.causal_controls.gain_kind == "APPLICABILITY"
            && receipt.causal_controls.support_future_overlap == 0
            && receipt.causal_controls.censored_semantic_updates == 0,
        "F8 causal controls",
    )?;
    require(
        receipt.external_admission.verdict == "SHADOW_READY"
            && valid_sha256(&receipt.external_admission.commitments_sha256)
            && receipt.external_admission.live_shadow_denominator == receipt.live.verified_receipts
            && receipt.external_admission.live_verified_passes == receipt.live.verified_receipts
            && receipt.external_admission.negative_denominator == 0
            && receipt.external_admission.censored_denominator == 0
            && !receipt.external_admission.execution_authority,
        "F8 external admission",
    )?;
    require(
        sha256_hex(phase_control_source.as_bytes()) == receipt.artifacts.phase_control_file_sha256
            && sha256_hex(external_candidate_source.as_bytes())
                == receipt.artifacts.external_candidate_file_sha256,
        "F8 canonical artifact bytes",
    )?;
    require(
        receipt.performance.matched_p99_max_ns <= receipt.performance.matched_p99_budget_ns
            && receipt.performance.no_match_p99_max_ns
                <= receipt.performance.no_match_p99_budget_ns
            && receipt.performance.hard_max_ns <= receipt.performance.hard_ceiling_ns
            && receipt.performance.hot_rss_peak_delta_bytes
                <= receipt.performance.hot_rss_budget_bytes,
        "F8 performance budgets",
    )?;
    require(
        receipt.tests.kernel_pass >= 18
            && receipt.tests.learning_pass >= 217
            && receipt.tests.persistence_pass >= 16
            && receipt.tests.admission_pass >= 7
            && receipt.tests.explicit_live_audit_pass >= 1
            && receipt.tests.runtime_pass >= 47
            && receipt.tests.transition_serving_pass >= 51
            && receipt.tests.transition_serving_known_fail == 3
            && receipt.tests.gateway_control_pass >= 20
            && receipt.tests.baseline_failure_set_unchanged
            && receipt.tests.clippy_deny_warnings == "PASS"
            && receipt.next == "SEPARATE_EXPLICIT_AUTHORITY_ROLLOUT_OR_NATURAL_OPERATOR_EVIDENCE",
        "F8 proof boundary",
    )?;

    Ok(F8FinalStatus {
        provider_records: receipt.live.provider_records,
        verified_receipts: receipt.live.verified_receipts,
        full_phase_gain: receipt.causal_controls.full_phase_gain,
        search_gain: receipt.causal_controls.search_gain,
        external_verdict: receipt.external_admission.verdict,
        commitments_sha256: receipt.external_admission.commitments_sha256,
        matched_p99_max_ns: receipt.performance.matched_p99_max_ns,
        no_match_p99_max_ns: receipt.performance.no_match_p99_max_ns,
        hard_max_ns: receipt.performance.hard_max_ns,
        hot_rss_bytes: receipt.performance.hot_rss_peak_delta_bytes,
    })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value.bytes().any(|byte| byte != b'0')
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn require(condition: bool, label: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| label.to_owned())
}
