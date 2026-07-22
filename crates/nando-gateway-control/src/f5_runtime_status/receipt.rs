use serde::Deserialize;

const CONVERGENCE_SCHEMA: &str = "nando.stop-f5-runtime-convergence.v1";
const CONVERGENCE_STATUS: &str = "F5_COMPLETE_F6_UNLOCKED_NOT_STARTED";
const TRAFFIC_SCHEMA: &str = "nando.stop-f5-g-traffic-shadow.v1";
const TRAFFIC_STATUS: &str = "COMPLETE_PERFORMANCE_WATCH";
const F6_STATUS: &str = "F6_UNLOCKED_NOT_STARTED";

pub(super) struct F5Status {
    pub(super) implementation_commit: String,
    pub(super) phase_search_gain: String,
    pub(super) ordinary_rows: u64,
    pub(super) accounted_rows: u64,
    pub(super) organic_runtime_replay: String,
    pub(super) projection_controls_passed: u64,
    pub(super) projection_controls_total: u64,
    pub(super) no_match_p99_ns: u64,
    pub(super) no_match_target_ns: u64,
    pub(super) matched_shadow_p99_ns: u64,
    pub(super) matched_target_ns: u64,
    pub(super) hard_ceiling_ns: u64,
    pub(super) rss_delta_bytes: u64,
    pub(super) rss_target_bytes: u64,
}

#[derive(Deserialize)]
struct ConvergenceReceipt {
    schema: String,
    status: String,
    authority: bool,
    stages: StageReceipts,
    matrix: ConvergenceMatrix,
    watch: WatchReceipts,
    live: LiveReceipts,
    next: String,
}

#[derive(Deserialize)]
struct StageReceipts {
    f5_a: String,
    f5_b: String,
    f5_c: String,
    f5_d: String,
    f5_e: String,
    f5_f: String,
    f5_g: String,
}

#[derive(Deserialize)]
struct ConvergenceMatrix {
    actor_vm_parity_mismatches: u64,
    wrong_bindings: u64,
    negative_accepts: u64,
    ordinary_rows: u64,
    ordinary_rows_accounted: u64,
    production_callers: u64,
    execution_authority: bool,
}

#[derive(Deserialize)]
struct WatchReceipts {
    phase_search_gain: String,
}

#[derive(Deserialize)]
struct LiveReceipts {
    composite_gate: String,
    eligible_for_local_accept: bool,
    response_active_packages: u64,
    response_false_accepts: u64,
    response_runtime_parity_failures: u64,
}

#[derive(Deserialize)]
struct TrafficReceipt {
    schema: String,
    status: String,
    implementation_commit: String,
    authority: bool,
    production_callers: u64,
    ordinary_window: OrdinaryWindow,
    traffic_matrix: TrafficMatrix,
    performance: PerformanceReceipt,
    verification: VerificationReceipt,
    next: String,
}

#[derive(Deserialize)]
struct OrdinaryWindow {
    ordinary_rows: u64,
    accounted_rows: u64,
    censored_payload_unavailable: u64,
    invented_replay_rows: u64,
    organic_runtime_replay: String,
}

#[derive(Deserialize)]
struct TrafficMatrix {
    projection_controls_passed: u64,
    projection_controls_total: u64,
    mixed_generation_receipts: u64,
    false_accepts: u64,
    local_accepts: u64,
}

#[derive(Deserialize)]
struct PerformanceReceipt {
    no_match_p99_ns: u64,
    no_match_target_ns: u64,
    no_match_verdict: String,
    matched_shadow_p99_ns: u64,
    matched_target_ns: u64,
    matched_verdict: String,
    hard_ceiling_ns: u64,
    hard_ceiling_verdict: String,
    rss_delta_bytes: u64,
    rss_target_bytes: u64,
}

#[derive(Deserialize)]
struct VerificationReceipt {
    clippy_deny_warnings: String,
    owner_local_authority_ready: bool,
    live_composite_gate: String,
    eligible_for_local_accept: bool,
    response_active_packages: u64,
    response_false_accepts: u64,
    response_runtime_parity_failures: u64,
}

pub(super) fn parse_and_validate(
    convergence_source: &str,
    traffic_source: &str,
) -> Result<F5Status, String> {
    let convergence = serde_json::from_str::<ConvergenceReceipt>(convergence_source)
        .map_err(|error| format!("convergence receipt: {error}"))?;
    let traffic = serde_json::from_str::<TrafficReceipt>(traffic_source)
        .map_err(|error| format!("traffic receipt: {error}"))?;
    validate(&convergence, &traffic)?;

    Ok(F5Status {
        implementation_commit: traffic.implementation_commit,
        phase_search_gain: convergence.watch.phase_search_gain,
        ordinary_rows: traffic.ordinary_window.ordinary_rows,
        accounted_rows: traffic.ordinary_window.accounted_rows,
        organic_runtime_replay: traffic.ordinary_window.organic_runtime_replay,
        projection_controls_passed: traffic.traffic_matrix.projection_controls_passed,
        projection_controls_total: traffic.traffic_matrix.projection_controls_total,
        no_match_p99_ns: traffic.performance.no_match_p99_ns,
        no_match_target_ns: traffic.performance.no_match_target_ns,
        matched_shadow_p99_ns: traffic.performance.matched_shadow_p99_ns,
        matched_target_ns: traffic.performance.matched_target_ns,
        hard_ceiling_ns: traffic.performance.hard_ceiling_ns,
        rss_delta_bytes: traffic.performance.rss_delta_bytes,
        rss_target_bytes: traffic.performance.rss_target_bytes,
    })
}

fn validate(convergence: &ConvergenceReceipt, traffic: &TrafficReceipt) -> Result<(), String> {
    require(
        convergence.schema == CONVERGENCE_SCHEMA,
        "convergence schema",
    )?;
    require(
        convergence.status == CONVERGENCE_STATUS,
        "convergence status",
    )?;
    require(
        !convergence.authority,
        "convergence authority must be false",
    )?;
    require(convergence.stages.f5_a == "COMPLETE", "F5-A status")?;
    require(convergence.stages.f5_b == "COMPLETE", "F5-B status")?;
    require(convergence.stages.f5_c == "COMPLETE", "F5-C status")?;
    require(convergence.stages.f5_d == "COMPLETE", "F5-D status")?;
    require(convergence.stages.f5_e == "COMPLETE", "F5-E status")?;
    require(
        convergence.stages.f5_f == "SAFETY_PASS_WAVE_GAIN_WATCH",
        "F5-F status",
    )?;
    require(
        convergence.stages.f5_g == "COMPLETE_PERFORMANCE_WATCH",
        "F5-G status",
    )?;
    require(convergence.next == F6_STATUS, "convergence F6 boundary")?;
    require(
        convergence.matrix.ordinary_rows == convergence.matrix.ordinary_rows_accounted,
        "convergence ordinary denominator",
    )?;
    require(
        convergence.matrix.actor_vm_parity_mismatches == 0
            && convergence.matrix.wrong_bindings == 0
            && convergence.matrix.negative_accepts == 0,
        "convergence safety counters",
    )?;
    require(
        convergence.matrix.production_callers == 0 && !convergence.matrix.execution_authority,
        "convergence authority boundary",
    )?;
    require(
        convergence.live.composite_gate == "PASS"
            && !convergence.live.eligible_for_local_accept
            && convergence.live.response_active_packages == 0
            && convergence.live.response_false_accepts == 0
            && convergence.live.response_runtime_parity_failures == 0,
        "live fail-closed boundary",
    )?;

    require(traffic.schema == TRAFFIC_SCHEMA, "traffic schema")?;
    require(traffic.status == TRAFFIC_STATUS, "traffic status")?;
    require(!traffic.authority, "traffic authority must be false")?;
    require(
        traffic.production_callers == 0,
        "traffic production callers",
    )?;
    require(traffic.next == F6_STATUS, "traffic F6 boundary")?;
    require(
        traffic.ordinary_window.ordinary_rows == traffic.ordinary_window.accounted_rows
            && traffic.ordinary_window.ordinary_rows == convergence.matrix.ordinary_rows
            && traffic.ordinary_window.censored_payload_unavailable
                == traffic.ordinary_window.ordinary_rows
            && traffic.ordinary_window.invented_replay_rows == 0,
        "traffic ordinary denominator",
    )?;
    require(
        traffic.traffic_matrix.projection_controls_passed
            == traffic.traffic_matrix.projection_controls_total
            && traffic.traffic_matrix.mixed_generation_receipts == 0
            && traffic.traffic_matrix.false_accepts == 0
            && traffic.traffic_matrix.local_accepts == 0,
        "traffic safety matrix",
    )?;
    require(
        traffic.performance.hard_ceiling_verdict == "PASS"
            && traffic.performance.no_match_verdict == "WATCH"
            && traffic.performance.matched_verdict == "WATCH",
        "traffic performance verdicts",
    )?;
    require(
        traffic.verification.clippy_deny_warnings == "PASS"
            && !traffic.verification.owner_local_authority_ready
            && traffic.verification.live_composite_gate == "PASS"
            && !traffic.verification.eligible_for_local_accept
            && traffic.verification.response_active_packages == 0
            && traffic.verification.response_false_accepts == 0
            && traffic.verification.response_runtime_parity_failures == 0,
        "traffic verification boundary",
    )?;
    Ok(())
}

fn require(condition: bool, label: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| label.to_owned())
}
