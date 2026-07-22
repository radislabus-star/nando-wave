use serde::Deserialize;

const SCHEMA: &str = "nando.stop-f8-0-resource-truth.v1";
const VERDICT: &str = "RESOURCE_PASS_LATENCY_WATCH";

pub(super) struct F8ResourceStatus {
    pub(super) max_peak_rss_delta_bytes: u64,
    pub(super) rss_target_bytes: u64,
    pub(super) resource_observations: u64,
}

#[derive(Deserialize)]
struct Receipt {
    schema: String,
    verdict: String,
    authority: bool,
    environment: Environment,
    control: Control,
    production_policy: ProductionPolicy,
    latency_watch: LatencyWatch,
    proof: Proof,
    next: String,
}

#[derive(Deserialize)]
struct Environment {
    required_policy: String,
    repository_unit_has_policy: bool,
    loaded_systemd_unit_has_policy: bool,
    services_restarted: bool,
    deployment_changed: bool,
}

#[derive(Deserialize)]
struct Control {
    peak_rss_delta_bytes: u64,
    target_bytes: u64,
    verdict: String,
}

#[derive(Deserialize)]
struct ProductionPolicy {
    resource_observations: u64,
    observations_within_target: u64,
    max_peak_rss_delta_bytes: u64,
    target_bytes: u64,
    operators: u64,
    verdict: String,
}

#[derive(Deserialize)]
struct LatencyWatch {
    no_match_p99_target_ns: u64,
    observed_no_match_p99_max_ns: u64,
    observed_matched_p99_max_ns: u64,
    matched_p99_target_ns: u64,
    observed_hard_max_ns: u64,
    hard_ceiling_ns: u64,
    verdict: String,
}

#[derive(Deserialize)]
struct Proof {
    canonical_resource_test: String,
    production_policy_required_by_test: bool,
    runtime_unit_pass: u64,
    runtime_release_ignored: u64,
    gateway_control_pass: u64,
    clippy_deny_warnings: String,
    graphify_nodes: u64,
    graphify_edges: u64,
    graphify_communities: u64,
    raw_payload_bytes_persisted: u64,
    local_accepts: u64,
    execution_authority: bool,
}

pub(super) fn parse_and_validate(source: &str) -> Result<F8ResourceStatus, String> {
    let receipt = serde_json::from_str::<Receipt>(source)
        .map_err(|error| format!("F8-0 receipt: {error}"))?;
    require(receipt.schema == SCHEMA, "F8-0 schema")?;
    require(receipt.verdict == VERDICT, "F8-0 verdict")?;
    require(!receipt.authority, "F8-0 authority must be false")?;
    require(
        receipt.environment.required_policy == "MIMALLOC_PURGE_DELAY=0"
            && receipt.environment.repository_unit_has_policy
            && receipt.environment.loaded_systemd_unit_has_policy
            && !receipt.environment.services_restarted
            && !receipt.environment.deployment_changed,
        "F8-0 environment boundary",
    )?;
    require(
        receipt.control.peak_rss_delta_bytes > receipt.control.target_bytes
            && receipt.control.verdict == "WATCH_ALLOCATOR_RETENTION",
        "F8-0 control",
    )?;
    require(
        receipt.production_policy.resource_observations >= 12
            && receipt.production_policy.observations_within_target
                == receipt.production_policy.resource_observations
            && receipt.production_policy.max_peak_rss_delta_bytes
                <= receipt.production_policy.target_bytes
            && receipt.production_policy.operators == 2_048
            && receipt.production_policy.verdict == "PASS",
        "F8-0 resource gate",
    )?;
    require(
        receipt.latency_watch.observed_no_match_p99_max_ns
            > receipt.latency_watch.no_match_p99_target_ns
            && receipt.latency_watch.observed_matched_p99_max_ns
                <= receipt.latency_watch.matched_p99_target_ns
            && receipt.latency_watch.observed_hard_max_ns <= receipt.latency_watch.hard_ceiling_ns
            && receipt.latency_watch.verdict == "WATCH_F8_D_PROTOCOL_NOT_FROZEN",
        "F8-0 latency WATCH boundary",
    )?;
    require(
        receipt.proof.canonical_resource_test == "PASS"
            && receipt.proof.production_policy_required_by_test
            && receipt.proof.runtime_unit_pass >= 47
            && receipt.proof.runtime_release_ignored == 1
            && receipt.proof.gateway_control_pass >= 19
            && receipt.proof.clippy_deny_warnings == "PASS"
            && receipt.proof.graphify_nodes > 0
            && receipt.proof.graphify_edges > 0
            && receipt.proof.graphify_communities > 0
            && receipt.proof.raw_payload_bytes_persisted == 0
            && receipt.proof.local_accepts == 0
            && !receipt.proof.execution_authority
            && receipt.next == "F8_A_LIVE_PROVIDER_CAPTURE_OWNER",
        "F8-0 proof boundary",
    )?;

    Ok(F8ResourceStatus {
        max_peak_rss_delta_bytes: receipt.production_policy.max_peak_rss_delta_bytes,
        rss_target_bytes: receipt.production_policy.target_bytes,
        resource_observations: receipt.production_policy.resource_observations,
    })
}

fn require(condition: bool, label: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| label.to_owned())
}
