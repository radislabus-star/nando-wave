//! Local control plane for the resilient Nando gateway.
//!
//! This crate never handles model request bodies. It only controls the
//! out-of-band observer/miner lifecycle and keeps CPU mode behind two gates.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fmt::{Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const TRANSITION_SERVING_UNIT: &str = "nando-transition-serving.service";

const DEFAULT_TRANSITION_KILL_SWITCH_PATH: &str = "/etc/nando-wave/TRANSITION_KILL_SWITCH";

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GatewayMode {
    Bypass,
    Shadow,
    Cpu,
}

impl GatewayMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "BYPASS" => Some(Self::Bypass),
            "SHADOW" => Some(Self::Shadow),
            "CPU" => Some(Self::Cpu),
            _ => None,
        }
    }
}

impl Display for GatewayMode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bypass => formatter.write_str("BYPASS"),
            Self::Shadow => formatter.write_str("SHADOW"),
            Self::Cpu => formatter.write_str("CPU"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ControlConfig {
    pub bind: String,
    pub status_key: String,
    pub model_label: String,
    pub state_path: PathBuf,
    pub public_state_path: PathBuf,
    pub audit_path: PathBuf,
    pub admission_path: PathBuf,
    pub kill_switch_path: PathBuf,
    pub metrics_path: PathBuf,
    pub economics_path: PathBuf,
    pub response_registry_path: PathBuf,
    pub response_admission_controller_report_path: PathBuf,
    pub response_miner_status_path: PathBuf,
    pub response_online_miner_report_path: PathBuf,
    pub build_manifest_path: PathBuf,
    pub admission_max_age_seconds: u64,
    pub response_controller_report_max_age_seconds: u64,
    pub cpu_route_ready: bool,
}

impl ControlConfig {
    pub fn from_env() -> Result<Self, ControlError> {
        let status_key = env::var("NANDO_STATUS_DASHBOARD_KEY")
            .or_else(|_| env::var("NANDO_STATUS_ACCESS_KEY"))
            .map_err(|_| ControlError::Config("status dashboard key is missing".into()))?;
        validate_status_key(&status_key)?;

        let state_dir = PathBuf::from(
            env::var("NANDO_GATEWAY_CONTROL_STATE_DIR")
                .unwrap_or_else(|_| "/var/lib/nando-gateway-control".into()),
        );
        let admission_max_age_seconds = env::var("NANDO_TRANSITION_ADMISSION_MAX_AGE_SECONDS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(900);
        let response_controller_report_max_age_seconds =
            env::var("NANDO_RESPONSE_CONTROLLER_REPORT_MAX_AGE_SECONDS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(90);

        Ok(Self {
            bind: env::var("NANDO_GATEWAY_CONTROL_BIND")
                .unwrap_or_else(|_| "127.0.0.1:18788".into()),
            status_key,
            model_label: env::var("NANDO_GATEWAY_MODEL_LABEL")
                .unwrap_or_else(|_| "UNDECLARED".into()),
            state_path: state_dir.join("mode.json"),
            public_state_path: PathBuf::from(
                env::var("NANDO_GATEWAY_PUBLIC_MODE_JSON")
                    .unwrap_or_else(|_| "/run/nando-gateway-control/mode.json".into()),
            ),
            audit_path: state_dir.join("audit.jsonl"),
            admission_path: PathBuf::from(
                env::var("NANDO_TRANSITION_ADMISSION_JSON")
                    .unwrap_or_else(|_| "/var/lib/nando-wave/transition/admission.json".into()),
            ),
            kill_switch_path: PathBuf::from(
                env::var("NANDO_TRANSITION_KILL_SWITCH")
                    .unwrap_or_else(|_| DEFAULT_TRANSITION_KILL_SWITCH_PATH.into()),
            ),
            metrics_path: PathBuf::from(
                env::var("NANDO_TRANSITION_METRICS")
                    .unwrap_or_else(|_| "/var/lib/nando-wave/transition/metrics.json".into()),
            ),
            economics_path: PathBuf::from(
                env::var("NANDO_ECONOMICS_SNAPSHOT_JSON")
                    .unwrap_or_else(|_| "/var/lib/nando-wave/transition/economics.json".into()),
            ),
            response_registry_path: PathBuf::from(
                env::var("NANDO_RESPONSE_REGISTRY").unwrap_or_else(|_| {
                    "/var/lib/nando-wave/transition/response-registry.json".into()
                }),
            ),
            response_admission_controller_report_path: PathBuf::from(
                env::var("NANDO_RESPONSE_ADMISSION_CONTROLLER_REPORT").unwrap_or_else(|_| {
                    "/var/lib/nando-wave/transition/response-admission-controller-report.json"
                        .into()
                }),
            ),
            response_miner_status_path: PathBuf::from(
                env::var("NANDO_RESPONSE_MINER_STATUS").unwrap_or_else(|_| {
                    "/var/lib/nando-wave/transition/response-miner-status.json".into()
                }),
            ),
            response_online_miner_report_path: PathBuf::from(
                env::var("NANDO_RESPONSE_ONLINE_MINER_REPORT").unwrap_or_else(|_| {
                    "/var/lib/nando-wave/transition/response-online-miner-report.json".into()
                }),
            ),
            build_manifest_path: PathBuf::from(
                env::var("NANDO_BUILD_MANIFEST").unwrap_or_else(|_| {
                    "/var/lib/nando-wave/transition/build-manifest.json".into()
                }),
            ),
            admission_max_age_seconds,
            response_controller_report_max_age_seconds,
            cpu_route_ready: env_flag("NANDO_GATEWAY_CPU_ROUTE_READY"),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PersistedState {
    pub schema: String,
    pub mode: GatewayMode,
    pub reason: String,
    pub changed_by: String,
    pub changed_at_unix: u64,
}

impl PersistedState {
    pub fn new(mode: GatewayMode, reason: impl Into<String>) -> Self {
        Self {
            schema: "nando_gateway_control_state_v1".into(),
            mode,
            reason: reason.into(),
            changed_by: "local_control_plane".into(),
            changed_at_unix: unix_now(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct AdmissionStatus {
    pub report_present: bool,
    pub verdict: String,
    pub eligible_for_local_accept: bool,
    pub fresh: bool,
    pub route_ready: bool,
    pub kill_switch_path: PathBuf,
    pub kill_switch_present: bool,
    pub kill_switch_check_error: Option<String>,
    pub cpu_allowed: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStatus {
    pub unit: &'static str,
    pub active: bool,
}

#[derive(Debug)]
pub enum ControlError {
    Config(String),
    InvalidMode(String),
    CpuBlocked(String),
    Service(String),
    Io(io::Error),
    Json(serde_json::Error),
}

impl Display for ControlError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(message) => write!(formatter, "configuration error: {message}"),
            Self::InvalidMode(mode) => write!(formatter, "unknown gateway mode: {mode}"),
            Self::CpuBlocked(reason) => write!(formatter, "CPU mode is blocked: {reason}"),
            Self::Service(message) => write!(formatter, "service control failed: {message}"),
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
        }
    }
}

impl std::error::Error for ControlError {}

impl From<io::Error> for ControlError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ControlError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn read_state(path: &Path) -> PersistedState {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| PersistedState::new(GatewayMode::Shadow, "safe_default"))
}

pub fn admission_status(config: &ControlConfig) -> AdmissionStatus {
    let (kill_switch_present, kill_switch_check_error) = match config.kill_switch_path.try_exists()
    {
        Ok(present) => (present, None),
        Err(error) => (false, Some(error.to_string())),
    };
    let kill_switch_absent = !kill_switch_present && kill_switch_check_error.is_none();
    let kill_switch_block_reason = if kill_switch_present {
        Some(format!(
            "transition kill switch is present at {}",
            config.kill_switch_path.display()
        ))
    } else {
        kill_switch_check_error.as_ref().map(|error| {
            format!(
                "transition kill switch absence cannot be confirmed at {}: {error}",
                config.kill_switch_path.display()
            )
        })
    };

    let Ok(bytes) = fs::read(&config.admission_path) else {
        return AdmissionStatus {
            report_present: false,
            verdict: "MISSING".into(),
            eligible_for_local_accept: false,
            fresh: false,
            route_ready: config.cpu_route_ready,
            kill_switch_path: config.kill_switch_path.clone(),
            kill_switch_present,
            kill_switch_check_error,
            cpu_allowed: false,
            reason: kill_switch_block_reason
                .unwrap_or_else(|| "admission report is missing".into()),
        };
    };
    let Ok(report) = serde_json::from_slice::<Value>(&bytes) else {
        return AdmissionStatus {
            report_present: true,
            verdict: "INVALID".into(),
            eligible_for_local_accept: false,
            fresh: false,
            route_ready: config.cpu_route_ready,
            kill_switch_path: config.kill_switch_path.clone(),
            kill_switch_present,
            kill_switch_check_error,
            cpu_allowed: false,
            reason: kill_switch_block_reason
                .unwrap_or_else(|| "admission report is invalid JSON".into()),
        };
    };

    let verdict = report
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN")
        .to_ascii_uppercase();
    let eligible = report
        .get("eligible_for_local_accept")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let generated_at = report
        .get("generated_at_unix")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let age = unix_now().saturating_sub(generated_at);
    let fresh = generated_at > 0 && age <= config.admission_max_age_seconds;
    let cpu_allowed =
        verdict == "PASS" && eligible && fresh && config.cpu_route_ready && kill_switch_absent;
    let reason = if let Some(reason) = kill_switch_block_reason {
        reason
    } else if verdict != "PASS" {
        format!("composite gate verdict is {verdict}")
    } else if !eligible {
        "admission report does not allow local accept".into()
    } else if !fresh {
        format!("admission report is stale ({age}s)")
    } else if !config.cpu_route_ready {
        "runtime CPU route-ready flag is disabled".into()
    } else {
        "composite gate and runtime route are ready".into()
    };

    AdmissionStatus {
        report_present: true,
        verdict,
        eligible_for_local_accept: eligible,
        fresh,
        route_ready: config.cpu_route_ready,
        kill_switch_path: config.kill_switch_path.clone(),
        kill_switch_present,
        kill_switch_check_error,
        cpu_allowed,
        reason,
    }
}

pub fn service_statuses() -> [ServiceStatus; 1] {
    [TRANSITION_SERVING_UNIT].map(|unit| ServiceStatus {
        unit,
        active: systemctl_is_active(unit),
    })
}

pub fn apply_mode(
    config: &ControlConfig,
    mode: GatewayMode,
    reason: &str,
) -> Result<PersistedState, ControlError> {
    if mode == GatewayMode::Cpu {
        let admission = admission_status(config);
        if !admission.cpu_allowed {
            return Err(ControlError::CpuBlocked(admission.reason));
        }
    }

    if let Err(error) = apply_service_mode(mode) {
        let _ = apply_service_mode(GatewayMode::Bypass);
        let fallback = PersistedState::new(GatewayMode::Bypass, "service_control_failure");
        let _ = persist_state(config, &fallback);
        let _ = append_audit(&config.audit_path, &fallback, Some(&error.to_string()));
        return Err(error);
    }

    let state = PersistedState::new(mode, sanitize_reason(reason));
    persist_state(config, &state)?;
    append_audit(&config.audit_path, &state, None)?;
    Ok(state)
}

fn apply_service_mode(mode: GatewayMode) -> Result<(), ControlError> {
    if !service_mode_requires_active_runtime(mode) || systemctl_is_active(TRANSITION_SERVING_UNIT) {
        return Ok(());
    }
    Err(ControlError::Service(format!(
        "required service {TRANSITION_SERVING_UNIT} is not active"
    )))
}

fn service_mode_requires_active_runtime(mode: GatewayMode) -> bool {
    mode == GatewayMode::Cpu
}

pub fn reconcile_startup(config: &ControlConfig) -> Result<PersistedState, ControlError> {
    let current = read_state(&config.state_path);
    let mode = reconciled_startup_mode(&current, admission_status(config).cpu_allowed);
    apply_mode(config, mode, "startup_reconcile")
}

fn reconciled_startup_mode(current: &PersistedState, cpu_allowed: bool) -> GatewayMode {
    if (current.mode == GatewayMode::Cpu && !cpu_allowed)
        || (current.mode == GatewayMode::Bypass && current.reason == "service_control_failure")
    {
        GatewayMode::Shadow
    } else {
        current.mode
    }
}

fn persist_state(config: &ControlConfig, state: &PersistedState) -> Result<(), ControlError> {
    atomic_write_json(&config.state_path, state)?;
    atomic_write_json(&config.public_state_path, state)
}

fn systemctl_is_active(unit: &str) -> bool {
    Command::new("/usr/bin/systemctl")
        .args(["is-active", "--quiet", unit])
        .status()
        .is_ok_and(|status| status.success())
}

fn append_audit(
    path: &Path,
    state: &PersistedState,
    error: Option<&str>,
) -> Result<(), ControlError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut writer = BufWriter::new(file);
    let row = serde_json::json!({
        "schema": "nando_gateway_control_audit_v1",
        "mode": state.mode,
        "reason": state.reason,
        "changed_by": state.changed_by,
        "changed_at_unix": state.changed_at_unix,
        "error": error,
    });
    serde_json::to_writer(&mut writer, &row)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

pub fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ControlError> {
    let parent = path.parent().ok_or_else(|| {
        ControlError::Config(format!("state path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    let suffix = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(".mode.{}.{}.tmp", std::process::id(), suffix));

    let result = (|| -> Result<(), ControlError> {
        let mut file = File::create(&temp)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn validate_status_key(key: &str) -> Result<(), ControlError> {
    if key.len() < 16
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(ControlError::Config(
            "status dashboard key must be at least 16 URL-safe characters".into(),
        ));
    }
    Ok(())
}

fn sanitize_reason(reason: &str) -> String {
    let compact: String = reason
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || " _-.".contains(*character))
        .take(80)
        .collect();
    if compact.is_empty() {
        "manual_control".into()
    } else {
        compact
    }
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .ok()
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(label: &str) -> ControlConfig {
        let root = env::temp_dir().join(format!(
            "nando-gateway-control-{}-{}-{label}",
            std::process::id(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        ControlConfig {
            bind: "127.0.0.1:0".into(),
            status_key: "0123456789abcdef".into(),
            model_label: "test-model".into(),
            state_path: root.join("mode.json"),
            public_state_path: root.join("public-mode.json"),
            audit_path: root.join("audit.jsonl"),
            admission_path: root.join("admission.json"),
            kill_switch_path: root.join("TRANSITION_KILL_SWITCH"),
            metrics_path: root.join("metrics.json"),
            economics_path: root.join("economics.json"),
            response_registry_path: root.join("response-registry.json"),
            response_admission_controller_report_path: root
                .join("response-admission-controller-report.json"),
            response_miner_status_path: root.join("response-miner-status.json"),
            response_online_miner_report_path: root.join("response-online-miner-report.json"),
            build_manifest_path: root.join("build-manifest.json"),
            admission_max_age_seconds: 900,
            response_controller_report_max_age_seconds: 90,
            cpu_route_ready: false,
        }
    }

    #[test]
    fn cpu_requires_report_and_route_ready() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = test_config("cpu-gate");
        let status = admission_status(&config);
        assert!(!status.cpu_allowed);

        atomic_write_json(
            &config.admission_path,
            &serde_json::json!({
                "verdict": "PASS",
                "eligible_for_local_accept": true,
                "generated_at_unix": unix_now()
            }),
        )?;
        assert!(!admission_status(&config).cpu_allowed);

        config.cpu_route_ready = true;
        assert!(admission_status(&config).cpu_allowed);
        Ok(())
    }

    #[test]
    fn missing_kill_switch_allows_fresh_admission() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = test_config("kill-switch-missing");
        config.cpu_route_ready = true;
        atomic_write_json(
            &config.admission_path,
            &serde_json::json!({
                "verdict": "PASS",
                "eligible_for_local_accept": true,
                "generated_at_unix": unix_now()
            }),
        )?;

        let status = admission_status(&config);
        assert_eq!(status.kill_switch_path, config.kill_switch_path);
        assert!(!status.kill_switch_present);
        assert!(status.kill_switch_check_error.is_none());
        assert!(status.cpu_allowed);
        assert_eq!(status.reason, "composite gate and runtime route are ready");
        Ok(())
    }

    #[test]
    fn present_kill_switch_blocks_fresh_admission() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = test_config("kill-switch-present");
        config.cpu_route_ready = true;
        atomic_write_json(
            &config.admission_path,
            &serde_json::json!({
                "verdict": "PASS",
                "eligible_for_local_accept": true,
                "generated_at_unix": unix_now()
            }),
        )?;
        fs::write(&config.kill_switch_path, b"blocked\n")?;

        let status = admission_status(&config);
        assert!(status.kill_switch_present);
        assert!(status.kill_switch_check_error.is_none());
        assert!(!status.cpu_allowed);
        assert_eq!(
            status.reason,
            format!(
                "transition kill switch is present at {}",
                config.kill_switch_path.display()
            )
        );
        Ok(())
    }

    #[test]
    fn atomic_state_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let config = test_config("state");
        let state = PersistedState::new(GatewayMode::Bypass, "test");
        atomic_write_json(&config.state_path, &state)?;
        assert_eq!(read_state(&config.state_path).mode, GatewayMode::Bypass);
        Ok(())
    }

    #[test]
    fn only_cpu_mode_requires_an_active_runtime() {
        assert!(service_mode_requires_active_runtime(GatewayMode::Cpu));
        assert!(!service_mode_requires_active_runtime(GatewayMode::Shadow));
        assert!(!service_mode_requires_active_runtime(GatewayMode::Bypass));
    }

    #[test]
    fn startup_recovers_only_machine_generated_service_failure_bypass() {
        let failed = PersistedState::new(GatewayMode::Bypass, "service_control_failure");
        assert_eq!(reconciled_startup_mode(&failed, false), GatewayMode::Shadow);

        let manual = PersistedState::new(GatewayMode::Bypass, "manual_html");
        assert_eq!(reconciled_startup_mode(&manual, true), GatewayMode::Bypass);
    }

    #[test]
    fn invalid_or_stale_admission_is_blocked() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = test_config("stale");
        config.cpu_route_ready = true;
        atomic_write_json(
            &config.admission_path,
            &serde_json::json!({
                "verdict": "PASS",
                "eligible_for_local_accept": true,
                "generated_at_unix": 1
            }),
        )?;
        let status = admission_status(&config);
        assert!(!status.fresh);
        assert!(!status.cpu_allowed);
        assert!(status.reason.starts_with("admission report is stale ("));
        Ok(())
    }
}
