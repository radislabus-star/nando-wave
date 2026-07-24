use super::evaluation::build;
use super::handler::active_operator_summaries;
use super::model::{OperatorSummary, SignalPathInputs, StageStatus};
use crate::client_connections::{ClientConnectionSnapshot, ClientRoute, CodexWindowConnection};
use serde_json::json;

fn connected_windows() -> ClientConnectionSnapshot {
    ClientConnectionSnapshot {
        generated_at_unix_ms: 1,
        total_windows: 1,
        configured_for_nando: 1,
        active_nando: 1,
        active_outside_nando: 0,
        active_mixed: 0,
        idle: 0,
        misrouted: 0,
        windows: vec![CodexWindowConnection {
            project: "test".to_owned(),
            session: "session".to_owned(),
            pids: vec![7],
            configured_for_nando: true,
            route: ClientRoute::Nando,
            remote_endpoints: vec!["127.0.0.1:8787".to_owned()],
        }],
    }
}

fn active_operator() -> OperatorSummary {
    OperatorSummary {
        package_id: "operator-1".to_owned(),
        function_name: "wait".to_owned(),
        origin: "grounded_synthesis".to_owned(),
        state: "active".to_owned(),
        support_rows: 1,
        future_rows: 1,
        distinct_sessions: 2,
        wrong_accepts: 0,
        runtime_parity_failures: 0,
        live_cpu_counters_valid: true,
        live_cpu_accepts: 2,
        live_cpu_input_tokens: 125,
    }
}

fn passing_inputs() -> SignalPathInputs {
    SignalPathInputs {
        generated_at_unix_ms: 10_000,
        windows: connected_windows(),
        gateway_mode: "CPU".to_owned(),
        gateway_route_ready: true,
        kill_switch_present: false,
        kill_switch_check_error: None,
        serving_service_active: true,
        serving_health_ok: true,
        serving_instance_id_sha256: Some("instance".to_owned()),
        serving_sample_age_seconds: Some(1),
        serving_response_executor_ready: true,
        serving_response_local_accept_enabled: true,
        serving_response_active_packages: 1,
        serving_response_local_accepts: 2,
        serving_response_admission_seconds_remaining: Some(20),
        admission_cpu_allowed: true,
        admission_eligible: true,
        admission_fresh: true,
        controller_verdict: "PASS".to_owned(),
        controller_diagnostic: "active_generation_immutable".to_owned(),
        controller_active_packages: 1,
        controller_age_seconds: Some(1),
        controller_max_age_seconds: 900,
        operators: vec![active_operator()],
        total_input_tokens: Some(1_000),
        miner_input_tokens: Some(400),
        cpu_input_tokens: Some(100),
        accounting_epoch_total_input_tokens: Some(500),
        accounting_epoch_cpu_input_tokens: Some(125),
        process_nando_input_tokens: 250,
        process_miner_input_tokens: 250,
        process_cpu_input_tokens: 125,
        verified_local_accepts: 2,
        economics_age_seconds: Some(1),
        false_accepts: 0,
        runtime_revocation_state_valid: true,
        unresolved_active_runtime_revocations: 0,
        runtime_parity_failures: 0,
        bridge_failures: 0,
    }
}

#[test]
fn complete_path_requires_an_observed_cpu_accept() {
    let snapshot = build(passing_inputs());

    assert_eq!(snapshot.verdict, StageStatus::Pass);
    assert!(snapshot.complete);
    assert_eq!(snapshot.first_non_pass, None);
    assert_eq!(snapshot.traffic.miner_share_ppm, Some(400_000));
    assert_eq!(snapshot.traffic.cpu_share_ppm, Some(100_000));
    assert_eq!(
        snapshot.traffic.accounting_epoch_cpu_share_ppm,
        Some(250_000)
    );
    assert_eq!(snapshot.traffic.process_miner_share_ppm, Some(1_000_000));
    assert_eq!(snapshot.traffic.process_cpu_share_ppm, Some(500_000));
}

#[test]
fn first_blocker_is_the_window_not_downstream_cpu() {
    let mut inputs = passing_inputs();
    inputs.windows.active_nando = 0;
    inputs.windows.active_outside_nando = 1;
    inputs.windows.misrouted = 1;

    let snapshot = build(inputs);

    assert_eq!(snapshot.verdict, StageStatus::Block);
    assert_eq!(
        snapshot.first_non_pass.as_ref().map(|row| row.stage),
        Some("window")
    );
    assert_eq!(snapshot.path[4].status(), StageStatus::Pass);
}

#[test]
fn admission_open_without_active_package_does_not_enable_cpu() {
    let mut inputs = passing_inputs();
    inputs.operators.clear();
    inputs.controller_active_packages = 0;

    let snapshot = build(inputs);

    assert_eq!(snapshot.path[3].status(), StageStatus::Block);
    assert_eq!(snapshot.path[4].status(), StageStatus::Block);
    assert_eq!(
        snapshot.first_non_pass.as_ref().map(|row| row.stage),
        Some("active_package")
    );
}

#[test]
fn enabled_route_without_observed_accept_is_watch() {
    let mut inputs = passing_inputs();
    inputs.serving_response_local_accepts = 0;
    inputs.verified_local_accepts = 0;
    inputs.cpu_input_tokens = Some(0);

    let snapshot = build(inputs);

    assert_eq!(snapshot.path[4].status(), StageStatus::Watch);
    assert_eq!(snapshot.verdict, StageStatus::Watch);
}

#[test]
fn live_process_accept_proves_cpu_route_when_cumulative_receipt_is_idle() {
    let mut inputs = passing_inputs();
    inputs.economics_age_seconds = Some(600);

    let snapshot = build(inputs);

    assert_eq!(snapshot.path[4].status(), StageStatus::Pass);
    assert_eq!(snapshot.verdict, StageStatus::Pass);
}

#[test]
fn safety_failure_blocks_cpu_even_when_authority_is_open() {
    let mut inputs = passing_inputs();
    inputs.runtime_parity_failures = 1;

    let snapshot = build(inputs);

    assert_eq!(snapshot.path[4].status(), StageStatus::Block);
    assert_eq!(snapshot.verdict, StageStatus::Block);
}

#[test]
fn historical_false_accept_remains_visible_after_payload_containment() {
    let mut inputs = passing_inputs();
    inputs.false_accepts = 6;

    let snapshot = build(inputs);

    assert_eq!(snapshot.path[4].status(), StageStatus::Pass);
    assert_eq!(snapshot.safety.false_accepts, 6);
    assert_eq!(snapshot.safety.unresolved_active_runtime_revocations, 0);
}

#[test]
fn unresolved_active_revocation_blocks_cpu() {
    let mut inputs = passing_inputs();
    inputs.unresolved_active_runtime_revocations = 1;

    let snapshot = build(inputs);

    assert_eq!(snapshot.path[4].status(), StageStatus::Block);
    assert!(
        snapshot.path[4]
            .reason()
            .contains("unresolved_revocations=1")
    );
}

#[test]
fn control_authority_cannot_mask_a_disabled_serving_route() {
    let mut inputs = passing_inputs();
    inputs.serving_response_local_accept_enabled = false;

    let snapshot = build(inputs);

    assert_eq!(snapshot.path[3].status(), StageStatus::Pass);
    assert_eq!(snapshot.path[4].status(), StageStatus::Block);
    assert!(snapshot.path[4].reason().contains("serving"));
}

#[test]
fn active_operator_inventory_includes_function_and_vm_programs() {
    let registry = json!({
        "packages": [
            {
                "package_id": "function",
                "origin": "grounded_synthesis",
                "state": "active",
                "program": {"operation": {"op": "function_call", "function_name": "wait"}},
                "proof": {"support_rows": 1, "future_rows": 2}
            },
            {
                "package_id": "projection",
                "origin": "grounded_synthesis",
                "state": "active",
                "program": {"operation": {"op": "project_selected_value"}},
                "proof": {"support_rows": 1, "future_rows": 2}
            }
        ]
    });

    let runtime_health = json!({
        "response_cpu_by_package_valid": true,
        "response_cpu_by_package": {
            "projection": {
                "ordinary_accepts": 3,
                "ordinary_input_tokens": 144
            }
        }
    });
    let operators = active_operator_summaries(&registry, &runtime_health);

    assert_eq!(operators.len(), 2);
    assert_eq!(operators[0].function_name, "wait");
    assert_eq!(operators[1].function_name, "project_selected_value");
    assert_eq!(operators[0].live_cpu_accepts, 0);
    assert_eq!(operators[1].live_cpu_accepts, 3);
    assert_eq!(operators[1].live_cpu_input_tokens, 144);
}
