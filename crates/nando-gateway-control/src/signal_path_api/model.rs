use crate::client_connections::ClientConnectionSnapshot;
use serde::Serialize;

// This projection may diagnose an authority path, but it can never mint or widen authority.
pub(crate) const SIGNAL_PATH_SCHEMA_V1: &str = "nando.signal-path-status.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum StageStatus {
    Pass,
    Watch,
    Block,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct OperatorSummary {
    pub(crate) package_id: String,
    pub(crate) function_name: String,
    pub(crate) origin: String,
    pub(crate) state: String,
    pub(crate) support_rows: u64,
    pub(crate) future_rows: u64,
    pub(crate) distinct_sessions: u64,
    pub(crate) wrong_accepts: u64,
    pub(crate) runtime_parity_failures: u64,
    pub(crate) live_cpu_counters_valid: bool,
    pub(crate) live_cpu_accepts: u64,
    pub(crate) live_cpu_input_tokens: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct TrafficSnapshot {
    pub(crate) accounting_exact: bool,
    pub(crate) nando_input_tokens: Option<u64>,
    pub(crate) miner_visible_input_tokens: Option<u64>,
    pub(crate) cpu_input_tokens: Option<u64>,
    pub(crate) miner_share_ppm: Option<u64>,
    pub(crate) cpu_share_ppm: Option<u64>,
    pub(crate) accounting_epoch_nando_input_tokens: Option<u64>,
    pub(crate) accounting_epoch_cpu_input_tokens: Option<u64>,
    pub(crate) accounting_epoch_cpu_share_ppm: Option<u64>,
    pub(crate) process_nando_input_tokens: u64,
    pub(crate) process_miner_input_tokens: u64,
    pub(crate) process_miner_share_ppm: Option<u64>,
    pub(crate) process_cpu_input_tokens: u64,
    pub(crate) process_cpu_share_ppm: Option<u64>,
    pub(crate) verified_local_accepts: u64,
    pub(crate) economics_age_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct SafetySnapshot {
    pub(crate) false_accepts: u64,
    pub(crate) runtime_revocation_state_valid: bool,
    pub(crate) unresolved_active_runtime_revocations: u64,
    pub(crate) runtime_parity_failures: u64,
    pub(crate) bridge_failures: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FirstNonPass {
    pub(crate) stage: &'static str,
    pub(crate) status: StageStatus,
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub(crate) enum SignalPathStage {
    Window {
        position: u8,
        owner: &'static str,
        status: StageStatus,
        reason: String,
        total_windows: u64,
        configured_for_nando: u64,
        active_nando: u64,
        active_outside_nando: u64,
        active_mixed: u64,
        misrouted: u64,
    },
    Gateway {
        position: u8,
        owner: &'static str,
        status: StageStatus,
        reason: String,
        mode: String,
        route_ready: bool,
        kill_switch_present: bool,
        kill_switch_check_error: Option<String>,
    },
    Serving {
        position: u8,
        owner: &'static str,
        status: StageStatus,
        reason: String,
        service_active: bool,
        health_ok: bool,
        instance_id_sha256: Option<String>,
        sample_age_seconds: Option<u64>,
        response_executor_ready: bool,
        response_local_accept_enabled: bool,
        response_active_packages: u64,
        response_admission_seconds_remaining: Option<u64>,
    },
    ActivePackage {
        position: u8,
        owner: &'static str,
        status: StageStatus,
        reason: String,
        registry_active_packages: u64,
        controller_active_packages: u64,
        controller_verdict: String,
        controller_diagnostic: String,
        controller_age_seconds: Option<u64>,
        controller_max_age_seconds: u64,
        eligible_for_local_accept: bool,
    },
    Cpu {
        position: u8,
        owner: &'static str,
        status: StageStatus,
        reason: String,
        enabled: bool,
        live_process_accepts: u64,
        verified_local_accepts: u64,
        cpu_input_tokens: Option<u64>,
        false_accepts: u64,
        runtime_revocation_state_valid: bool,
        unresolved_active_runtime_revocations: u64,
        runtime_parity_failures: u64,
    },
}

impl SignalPathStage {
    pub(super) fn key(&self) -> &'static str {
        match self {
            Self::Window { .. } => "window",
            Self::Gateway { .. } => "gateway",
            Self::Serving { .. } => "serving",
            Self::ActivePackage { .. } => "active_package",
            Self::Cpu { .. } => "cpu",
        }
    }

    pub(super) fn status(&self) -> StageStatus {
        match self {
            Self::Window { status, .. }
            | Self::Gateway { status, .. }
            | Self::Serving { status, .. }
            | Self::ActivePackage { status, .. }
            | Self::Cpu { status, .. } => *status,
        }
    }

    pub(super) fn reason(&self) -> &str {
        match self {
            Self::Window { reason, .. }
            | Self::Gateway { reason, .. }
            | Self::Serving { reason, .. }
            | Self::ActivePackage { reason, .. }
            | Self::Cpu { reason, .. } => reason,
        }
    }
}

pub(crate) struct SignalPathInputs {
    pub(crate) generated_at_unix_ms: u64,
    pub(crate) windows: ClientConnectionSnapshot,
    pub(crate) gateway_mode: String,
    pub(crate) gateway_route_ready: bool,
    pub(crate) kill_switch_present: bool,
    pub(crate) kill_switch_check_error: Option<String>,
    pub(crate) serving_service_active: bool,
    pub(crate) serving_health_ok: bool,
    pub(crate) serving_instance_id_sha256: Option<String>,
    pub(crate) serving_sample_age_seconds: Option<u64>,
    pub(crate) serving_response_executor_ready: bool,
    pub(crate) serving_response_local_accept_enabled: bool,
    pub(crate) serving_response_active_packages: u64,
    pub(crate) serving_response_local_accepts: u64,
    pub(crate) serving_response_admission_seconds_remaining: Option<u64>,
    pub(crate) admission_cpu_allowed: bool,
    pub(crate) admission_eligible: bool,
    pub(crate) admission_fresh: bool,
    pub(crate) controller_verdict: String,
    pub(crate) controller_diagnostic: String,
    pub(crate) controller_active_packages: u64,
    pub(crate) controller_age_seconds: Option<u64>,
    pub(crate) controller_max_age_seconds: u64,
    pub(crate) operators: Vec<OperatorSummary>,
    pub(crate) total_input_tokens: Option<u64>,
    pub(crate) miner_input_tokens: Option<u64>,
    pub(crate) cpu_input_tokens: Option<u64>,
    pub(crate) accounting_epoch_total_input_tokens: Option<u64>,
    pub(crate) accounting_epoch_cpu_input_tokens: Option<u64>,
    pub(crate) process_nando_input_tokens: u64,
    pub(crate) process_miner_input_tokens: u64,
    pub(crate) process_cpu_input_tokens: u64,
    pub(crate) verified_local_accepts: u64,
    pub(crate) economics_age_seconds: Option<u64>,
    pub(crate) false_accepts: u64,
    pub(crate) runtime_revocation_state_valid: bool,
    pub(crate) unresolved_active_runtime_revocations: u64,
    pub(crate) runtime_parity_failures: u64,
    pub(crate) bridge_failures: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct SignalPathSnapshot {
    pub(crate) schema: &'static str,
    pub(crate) generated_at_unix_ms: u64,
    pub(crate) verdict: StageStatus,
    pub(crate) complete: bool,
    pub(crate) first_non_pass: Option<FirstNonPass>,
    pub(crate) path: Vec<SignalPathStage>,
    pub(crate) traffic: TrafficSnapshot,
    pub(crate) safety: SafetySnapshot,
    pub(crate) operators: Vec<OperatorSummary>,
    pub(crate) windows: ClientConnectionSnapshot,
}
