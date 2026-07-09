use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(super) struct PhaseStreamArchitectureVersionsReport {
    phase_center_core: &'static str,
    online_miner: &'static str,
    live_tail_daemon: &'static str,
    hot_runtime: &'static str,
    auto_subcenter_discovery: &'static str,
    hidden_state_profile: &'static str,
    profile_attribution: &'static str,
    operator_power_gate: &'static str,
    compression_accounting: &'static str,
    package_format: &'static str,
    forbidden_backend_policy: &'static str,
}

pub(super) fn live_store_architecture_versions() -> PhaseStreamArchitectureVersionsReport {
    PhaseStreamArchitectureVersionsReport {
        phase_center_core: "phase_center_core_v1",
        online_miner: "online_phase_center_miner_v1",
        live_tail_daemon: "append_live_tail_shadow_daemon_v6",
        hot_runtime: "phase_center_hot_runtime_v1",
        auto_subcenter_discovery: "auto_subcenter_discovery_v2_hidden_first",
        hidden_state_profile: "hidden_state_cross_layer_profile_v1",
        profile_attribution: "profile_attribution_disjoint_v1",
        operator_power_gate: "operator_power_richness_v1_cold_survivor_gate",
        compression_accounting: "restart_safe_claimsafe_stable_window_calls_tokens_cost_milli_accounting_v4",
        package_format: "nwpc_v1",
        forbidden_backend_policy: "no_nwrb_no_lookup_no_local_accept_without_verifier_v1",
    }
}

pub(super) fn live_store_architecture_version_key() -> String {
    let versions = live_store_architecture_versions();
    [
        versions.phase_center_core,
        versions.online_miner,
        versions.live_tail_daemon,
        versions.hot_runtime,
        versions.auto_subcenter_discovery,
        versions.hidden_state_profile,
        versions.profile_attribution,
        versions.operator_power_gate,
        versions.compression_accounting,
        versions.package_format,
        versions.forbidden_backend_policy,
    ]
    .join("|")
}
