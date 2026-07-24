use super::model::{
    FirstNonPass, SIGNAL_PATH_SCHEMA_V1, SafetySnapshot, SignalPathInputs, SignalPathSnapshot,
    SignalPathStage, StageStatus, TrafficSnapshot,
};
use crate::client_connections::ClientConnectionSnapshot;

pub(super) fn build(inputs: SignalPathInputs) -> SignalPathSnapshot {
    let window_status = window_status(&inputs.windows);
    let gateway_status = gateway_status(&inputs);
    let serving_status = serving_status(&inputs);
    let package_status = package_status(&inputs);
    let cpu_status = cpu_status(&inputs);
    let active_package_count = inputs.operators.len() as u64;

    let path = vec![
        SignalPathStage::Window {
            position: 1,
            owner: "codex_launcher + session_observer",
            status: window_status.0,
            reason: window_status.1,
            total_windows: inputs.windows.total_windows,
            configured_for_nando: inputs.windows.configured_for_nando,
            active_nando: inputs.windows.active_nando,
            active_outside_nando: inputs.windows.active_outside_nando,
            active_mixed: inputs.windows.active_mixed,
            misrouted: inputs.windows.misrouted,
        },
        SignalPathStage::Gateway {
            position: 2,
            owner: "nando-nginx-gateway",
            status: gateway_status.0,
            reason: gateway_status.1,
            mode: inputs.gateway_mode.clone(),
            route_ready: inputs.gateway_route_ready,
            kill_switch_present: inputs.kill_switch_present,
            kill_switch_check_error: inputs.kill_switch_check_error.clone(),
        },
        SignalPathStage::Serving {
            position: 3,
            owner: "nando-transition-serving",
            status: serving_status.0,
            reason: serving_status.1,
            service_active: inputs.serving_service_active,
            health_ok: inputs.serving_health_ok,
            instance_id_sha256: inputs.serving_instance_id_sha256.clone(),
            sample_age_seconds: inputs.serving_sample_age_seconds,
            response_executor_ready: inputs.serving_response_executor_ready,
            response_local_accept_enabled: inputs.serving_response_local_accept_enabled,
            response_active_packages: inputs.serving_response_active_packages,
            response_admission_seconds_remaining: inputs
                .serving_response_admission_seconds_remaining,
        },
        SignalPathStage::ActivePackage {
            position: 4,
            owner: "nando-response-admission",
            status: package_status.0,
            reason: package_status.1,
            registry_active_packages: active_package_count,
            controller_active_packages: inputs.controller_active_packages,
            controller_verdict: inputs.controller_verdict.clone(),
            controller_diagnostic: inputs.controller_diagnostic.clone(),
            controller_age_seconds: inputs.controller_age_seconds,
            controller_max_age_seconds: inputs.controller_max_age_seconds,
            eligible_for_local_accept: inputs.admission_eligible,
        },
        SignalPathStage::Cpu {
            position: 5,
            owner: "nando-response-actor",
            status: cpu_status.0,
            reason: cpu_status.1,
            enabled: inputs.admission_cpu_allowed && inputs.serving_response_local_accept_enabled,
            verified_local_accepts: inputs.verified_local_accepts,
            cpu_input_tokens: inputs.cpu_input_tokens,
            false_accepts: inputs.false_accepts,
            runtime_parity_failures: inputs.runtime_parity_failures,
        },
    ];

    let first_non_pass = path
        .iter()
        .find(|stage| stage.status() != StageStatus::Pass)
        .map(|stage| FirstNonPass {
            stage: stage.key(),
            status: stage.status(),
            reason: stage.reason().to_owned(),
        });
    let verdict = if path
        .iter()
        .any(|stage| stage.status() == StageStatus::Block)
    {
        StageStatus::Block
    } else if first_non_pass.is_some() {
        StageStatus::Watch
    } else {
        StageStatus::Pass
    };

    let accounting_exact = inputs.total_input_tokens.is_some()
        && inputs.miner_input_tokens.is_some()
        && inputs.cpu_input_tokens.is_some();
    let traffic = TrafficSnapshot {
        accounting_exact,
        nando_input_tokens: inputs.total_input_tokens,
        miner_visible_input_tokens: inputs.miner_input_tokens,
        cpu_input_tokens: inputs.cpu_input_tokens,
        miner_share_ppm: ratio_ppm(inputs.miner_input_tokens, inputs.total_input_tokens),
        cpu_share_ppm: ratio_ppm(inputs.cpu_input_tokens, inputs.total_input_tokens),
        accounting_epoch_nando_input_tokens: inputs.accounting_epoch_total_input_tokens,
        accounting_epoch_cpu_input_tokens: inputs.accounting_epoch_cpu_input_tokens,
        accounting_epoch_cpu_share_ppm: ratio_ppm(
            inputs.accounting_epoch_cpu_input_tokens,
            inputs.accounting_epoch_total_input_tokens,
        ),
        process_nando_input_tokens: inputs.process_nando_input_tokens,
        process_miner_input_tokens: inputs.process_miner_input_tokens,
        process_miner_share_ppm: ratio_ppm(
            Some(inputs.process_miner_input_tokens),
            Some(inputs.process_nando_input_tokens),
        ),
        verified_local_accepts: inputs.verified_local_accepts,
        economics_age_seconds: inputs.economics_age_seconds,
    };

    SignalPathSnapshot {
        schema: SIGNAL_PATH_SCHEMA_V1,
        generated_at_unix_ms: inputs.generated_at_unix_ms,
        verdict,
        complete: first_non_pass.is_none(),
        first_non_pass,
        path,
        traffic,
        safety: SafetySnapshot {
            false_accepts: inputs.false_accepts,
            runtime_parity_failures: inputs.runtime_parity_failures,
            bridge_failures: inputs.bridge_failures,
        },
        operators: inputs.operators,
        windows: inputs.windows,
    }
}

fn window_status(windows: &ClientConnectionSnapshot) -> (StageStatus, String) {
    if windows.total_windows == 0 {
        return (
            StageStatus::Watch,
            "no live Codex window is observable".to_owned(),
        );
    }
    if windows.active_nando == 0 {
        return (
            StageStatus::Block,
            format!(
                "no window has an active 127.0.0.1:8787 route; outside={} mixed={}",
                windows.active_outside_nando, windows.active_mixed
            ),
        );
    }
    if windows.misrouted > 0 || windows.active_mixed > 0 {
        return (
            StageStatus::Watch,
            format!(
                "{} window(s) use Nando, but {} configured window(s) are outside and {} are mixed",
                windows.active_nando, windows.misrouted, windows.active_mixed
            ),
        );
    }
    (
        StageStatus::Pass,
        format!(
            "{} window(s) have an active Nando route",
            windows.active_nando
        ),
    )
}

fn gateway_status(inputs: &SignalPathInputs) -> (StageStatus, String) {
    if inputs.kill_switch_present {
        return (
            StageStatus::Block,
            "transition kill switch is present".to_owned(),
        );
    }
    if let Some(error) = &inputs.kill_switch_check_error {
        return (
            StageStatus::Block,
            format!("kill-switch absence cannot be confirmed: {error}"),
        );
    }
    if !inputs.gateway_route_ready {
        return (
            StageStatus::Block,
            "gateway CPU route contract is not ready".to_owned(),
        );
    }
    if inputs.gateway_mode != "CPU" {
        return (
            StageStatus::Block,
            format!("gateway mode is {}, not CPU", inputs.gateway_mode),
        );
    }
    (StageStatus::Pass, "gateway CPU route is enabled".to_owned())
}

fn serving_status(inputs: &SignalPathInputs) -> (StageStatus, String) {
    if !inputs.serving_service_active {
        return (
            StageStatus::Block,
            "nando-transition-serving.service is inactive".to_owned(),
        );
    }
    if !inputs.serving_health_ok {
        return (
            StageStatus::Block,
            "serving bridge health is unavailable".to_owned(),
        );
    }
    if !inputs.serving_response_executor_ready {
        return (
            StageStatus::Block,
            "serving response executor cache is not ready".to_owned(),
        );
    }
    (
        StageStatus::Pass,
        "serving process and bridge health are live".to_owned(),
    )
}

fn package_status(inputs: &SignalPathInputs) -> (StageStatus, String) {
    let registry_count = inputs.operators.len() as u64;
    if !inputs.admission_fresh {
        return (
            StageStatus::Block,
            "external admission receipt is stale".to_owned(),
        );
    }
    if inputs
        .controller_age_seconds
        .is_none_or(|age| age > inputs.controller_max_age_seconds)
    {
        return (
            StageStatus::Block,
            "admission controller report is stale".to_owned(),
        );
    }
    if inputs.controller_verdict != "PASS" {
        return (
            StageStatus::Block,
            format!(
                "admission controller verdict is {}",
                inputs.controller_verdict
            ),
        );
    }
    if !inputs.admission_eligible {
        return (
            StageStatus::Block,
            "local accept is not eligible".to_owned(),
        );
    }
    if registry_count == 0 {
        return (
            StageStatus::Block,
            "runtime registry contains no ACTIVE package".to_owned(),
        );
    }
    if inputs.controller_active_packages != registry_count {
        return (
            StageStatus::Block,
            format!(
                "controller/registry ACTIVE count mismatch: {}/{}",
                inputs.controller_active_packages, registry_count
            ),
        );
    }
    if inputs.serving_response_active_packages != registry_count {
        return (
            StageStatus::Block,
            format!(
                "serving/registry ACTIVE count mismatch: {}/{}",
                inputs.serving_response_active_packages, registry_count
            ),
        );
    }
    (
        StageStatus::Pass,
        format!("{registry_count} ACTIVE package(s) are externally admitted"),
    )
}

fn cpu_status(inputs: &SignalPathInputs) -> (StageStatus, String) {
    if inputs.false_accepts > 0 || inputs.runtime_parity_failures > 0 {
        return (
            StageStatus::Block,
            format!(
                "safety counters are non-zero: false_accepts={} parity_failures={}",
                inputs.false_accepts, inputs.runtime_parity_failures
            ),
        );
    }
    if !inputs.admission_cpu_allowed || inputs.gateway_mode != "CPU" {
        return (
            StageStatus::Block,
            "CPU local accept is not enabled".to_owned(),
        );
    }
    if !inputs.serving_response_local_accept_enabled {
        return (
            StageStatus::Block,
            "serving response local accept is disabled".to_owned(),
        );
    }
    if inputs.operators.is_empty() {
        return (
            StageStatus::Block,
            "no ACTIVE package can own a CPU accept".to_owned(),
        );
    }
    if inputs.bridge_failures > 0 {
        return (
            StageStatus::Block,
            format!(
                "learning bridge reports {} failure(s)",
                inputs.bridge_failures
            ),
        );
    }
    if inputs.verified_local_accepts == 0 || inputs.cpu_input_tokens == Some(0) {
        return (
            StageStatus::Watch,
            "CPU route is enabled but no verified local accept is observed".to_owned(),
        );
    }
    if inputs.total_input_tokens.is_none() || inputs.cpu_input_tokens.is_none() {
        return (
            StageStatus::Watch,
            "CPU accepts exist but exact token accounting is unavailable".to_owned(),
        );
    }
    if inputs.economics_age_seconds.is_none_or(|age| age > 120) {
        return (
            StageStatus::Watch,
            "CPU accepts exist but the economics receipt is stale".to_owned(),
        );
    }
    (
        StageStatus::Pass,
        format!(
            "{} verified local accept(s) reached CPU",
            inputs.verified_local_accepts
        ),
    )
}

fn ratio_ppm(part: Option<u64>, total: Option<u64>) -> Option<u64> {
    let (Some(part), Some(total)) = (part, total) else {
        return None;
    };
    if total == 0 || part > total {
        return None;
    }
    Some(((u128::from(part) * 1_000_000) / u128::from(total)) as u64)
}
