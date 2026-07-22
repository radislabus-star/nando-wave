use serde::Deserialize;

const SCHEMA: &str = "nando.stop-f7-e-controlled-generation-shadow.v1";
const VERDICT: &str = "COMPLETE_CONTROLLED_SHADOW_PASS";

pub(super) struct F7Status {
    pub(super) receipt_date: String,
    pub(super) no_match_p99_ns: u64,
    pub(super) matched_p99_ns: u64,
    pub(super) hard_max_ns: u64,
    pub(super) queue_max: u64,
    pub(super) rss_delta_bytes: u64,
    pub(super) rss_target_bytes: u64,
}

#[derive(Deserialize)]
struct F7Receipt {
    schema: String,
    date: String,
    verdict: String,
    authority: bool,
    f7_status: String,
    capture_owner: CaptureOwner,
    runtime: Runtime,
    signal: Signal,
    performance_ns: Performance,
    resource_watch: ResourceWatch,
    tests: Tests,
    production: Production,
}

#[derive(Deserialize)]
struct CaptureOwner {
    separate_provider_request_index: bool,
    exact_checkpoint_join: bool,
    session_jsonl_used_as_http_request_provenance: bool,
    missing_or_tampered_relation_blocked: bool,
    live_producer_deployed: bool,
}

#[derive(Deserialize)]
struct Runtime {
    http_bound_before_restore: bool,
    immutable_generation_registry: bool,
    monotonic_swap: bool,
    generation_pinned_before_enqueue: bool,
    empty_store_clears_registry: bool,
    queue_operation: String,
    max_queued_requests: u64,
    raw_payload_bytes_persisted: u64,
    local_accepts: u64,
    execution_authority: bool,
}

#[derive(Deserialize)]
struct Signal {
    checkpoint_restore: String,
    capture_join: String,
    f5_role_grounding_actor: String,
    f6_independent_verifier: String,
    external_admission: String,
}

#[derive(Deserialize)]
struct Performance {
    f7e_no_match_p99: u64,
    f7e_matched_p99: u64,
    f7e_hard_max: u64,
}

#[derive(Deserialize)]
struct ResourceWatch {
    f5_registry_operators: u64,
    f5_registry_rss_delta_bytes: u64,
    f5_registry_rss_target_bytes: u64,
    f5_registry_rss_target_pass: bool,
}

#[derive(Deserialize)]
struct Tests {
    generation_capture_pass: u64,
    capture_join_pass: u64,
    traffic_shadow_pass: u64,
    serving_shadow_pass: u64,
    release_performance_pass: u64,
    broad_pass: u64,
    known_serving_failures_excluded: u64,
    known_failure_set_baseline_identical: bool,
    gateway_control_pass: u64,
    gateway_control_clippy: String,
    clippy_deny_warnings: String,
    changed_file_rustfmt: String,
    nanda_composite_gate: String,
    eligible_for_local_accept: bool,
}

#[derive(Deserialize)]
struct Production {
    feature_default_enabled: bool,
    services_restarted: bool,
    deployment_changed: bool,
    active_packages_changed: bool,
    external_admission_calls: u64,
    execution_authority: bool,
}

pub(super) fn parse_and_validate(source: &str) -> Result<F7Status, String> {
    let receipt = serde_json::from_str::<F7Receipt>(source)
        .map_err(|error| format!("F7 receipt: {error}"))?;
    validate(&receipt)?;

    Ok(F7Status {
        receipt_date: receipt.date,
        no_match_p99_ns: receipt.performance_ns.f7e_no_match_p99,
        matched_p99_ns: receipt.performance_ns.f7e_matched_p99,
        hard_max_ns: receipt.performance_ns.f7e_hard_max,
        queue_max: receipt.runtime.max_queued_requests,
        rss_delta_bytes: receipt.resource_watch.f5_registry_rss_delta_bytes,
        rss_target_bytes: receipt.resource_watch.f5_registry_rss_target_bytes,
    })
}

fn validate(receipt: &F7Receipt) -> Result<(), String> {
    require(receipt.schema == SCHEMA, "F7 schema")?;
    require(receipt.verdict == VERDICT, "F7 verdict")?;
    require(!receipt.authority, "F7 authority must be false")?;
    require(receipt.f7_status == "COMPLETE", "F7 status")?;
    require(
        receipt.capture_owner.separate_provider_request_index
            && receipt.capture_owner.exact_checkpoint_join
            && !receipt
                .capture_owner
                .session_jsonl_used_as_http_request_provenance
            && receipt.capture_owner.missing_or_tampered_relation_blocked
            && !receipt.capture_owner.live_producer_deployed,
        "F7 capture boundary",
    )?;
    require(
        receipt.runtime.http_bound_before_restore
            && receipt.runtime.immutable_generation_registry
            && receipt.runtime.monotonic_swap
            && receipt.runtime.generation_pinned_before_enqueue
            && receipt.runtime.empty_store_clears_registry
            && receipt.runtime.queue_operation == "try_send"
            && receipt.runtime.max_queued_requests <= 48
            && receipt.runtime.raw_payload_bytes_persisted == 0
            && receipt.runtime.local_accepts == 0
            && !receipt.runtime.execution_authority,
        "F7 runtime boundary",
    )?;
    require(
        receipt.signal.checkpoint_restore == "PASS"
            && receipt.signal.capture_join == "PASS"
            && receipt.signal.f5_role_grounding_actor == "PASS"
            && receipt.signal.f6_independent_verifier == "PASS"
            && receipt.signal.external_admission == "BLOCKED_F8",
        "F7 signal path",
    )?;
    require(
        receipt.performance_ns.f7e_no_match_p99 <= 250_000
            && receipt.performance_ns.f7e_matched_p99 <= 1_000_000
            && receipt.performance_ns.f7e_hard_max <= 2_000_000,
        "F7 performance gates",
    )?;
    require(
        receipt.resource_watch.f5_registry_operators == 2_048
            && !receipt.resource_watch.f5_registry_rss_target_pass
            && receipt.resource_watch.f5_registry_rss_delta_bytes
                > receipt.resource_watch.f5_registry_rss_target_bytes,
        "F7 resource WATCH boundary",
    )?;
    require(
        receipt.tests.generation_capture_pass >= 3
            && receipt.tests.capture_join_pass >= 2
            && receipt.tests.traffic_shadow_pass >= 8
            && receipt.tests.serving_shadow_pass >= 5
            && receipt.tests.release_performance_pass >= 3
            && receipt.tests.broad_pass >= 354
            && receipt.tests.known_serving_failures_excluded == 3
            && receipt.tests.known_failure_set_baseline_identical
            && receipt.tests.gateway_control_pass >= 18
            && receipt.tests.gateway_control_clippy == "PASS"
            && receipt.tests.clippy_deny_warnings == "PASS"
            && receipt.tests.changed_file_rustfmt == "PASS"
            && receipt.tests.nanda_composite_gate == "PASS"
            && !receipt.tests.eligible_for_local_accept,
        "F7 proof gates",
    )?;
    require(
        !receipt.production.feature_default_enabled
            && !receipt.production.services_restarted
            && !receipt.production.deployment_changed
            && !receipt.production.active_packages_changed
            && receipt.production.external_admission_calls == 0
            && !receipt.production.execution_authority,
        "F7 production boundary",
    )
}

fn require(condition: bool, label: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| label.to_owned())
}
