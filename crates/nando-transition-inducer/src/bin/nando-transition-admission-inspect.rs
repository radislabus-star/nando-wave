use std::env;
use std::path::{Path, PathBuf};

use nando_transition_inducer::{
    LivePackageOrigin, LiveProfileRegistry, LiveProfileState, read_package,
    read_package_artifact_bytes, validate_live_package,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct AdmissionReport {
    report_kind: &'static str,
    verdict: &'static str,
    registry_path: String,
    registry_revision: u64,
    policy_version: String,
    package_count: usize,
    profile_count: usize,
    active_profile_count: usize,
    non_raw_active_profile_count: usize,
    quarantined_profile_count: usize,
    revoked_profile_count: usize,
    validated_package_count: usize,
    validated_profile_count: usize,
    active_future_rows: usize,
    active_future_clean_rows: usize,
    active_grounded_future_rows: usize,
    active_grounded_future_clean_rows: usize,
    active_false_accepts: usize,
    active_runtime_parity_mismatches: usize,
    active_execution_p99_ns: u64,
    package_bytes: usize,
    traces_seen: usize,
    shadow_executions: usize,
    global_false_accepts: usize,
    global_runtime_parity_mismatches: usize,
    kill_switch_only_manual_control: bool,
    package_validation_failures: Vec<String>,
    package_accounting_warnings: Vec<String>,
    profile_linkage_failures: Vec<String>,
    active_policy_failures: Vec<String>,
    local_accept_eligible: bool,
    boundary: &'static str,
}

fn main() {
    let report = inspect().unwrap_or_else(|error| AdmissionReport {
        report_kind: "nando_transition_admission_inspection_v1",
        verdict: "ERROR",
        registry_path: registry_path().display().to_string(),
        registry_revision: 0,
        policy_version: String::new(),
        package_count: 0,
        profile_count: 0,
        active_profile_count: 0,
        non_raw_active_profile_count: 0,
        quarantined_profile_count: 0,
        revoked_profile_count: 0,
        validated_package_count: 0,
        validated_profile_count: 0,
        active_future_rows: 0,
        active_future_clean_rows: 0,
        active_grounded_future_rows: 0,
        active_grounded_future_clean_rows: 0,
        active_false_accepts: 0,
        active_runtime_parity_mismatches: 0,
        active_execution_p99_ns: 0,
        package_bytes: 0,
        traces_seen: 0,
        shadow_executions: 0,
        global_false_accepts: 0,
        global_runtime_parity_mismatches: 0,
        kill_switch_only_manual_control: false,
        package_validation_failures: vec![error],
        package_accounting_warnings: Vec::new(),
        profile_linkage_failures: Vec::new(),
        active_policy_failures: Vec::new(),
        local_accept_eligible: false,
        boundary: "read-only admission inspection failed; local accept is not eligible",
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| {
            "{\"report_kind\":\"nando_transition_admission_inspection_v1\",\"verdict\":\"ERROR\",\"local_accept_eligible\":false}".to_owned()
        })
    );
    if report.verdict != "PASS" {
        std::process::exit(1);
    }
}

fn inspect() -> Result<AdmissionReport, String> {
    let path = registry_path();
    let registry = LiveProfileRegistry::load(&path)?;
    let mut package_validation_failures = Vec::new();
    let mut package_accounting_warnings = Vec::new();
    let mut profile_linkage_failures = Vec::new();
    let mut active_policy_failures = Vec::new();
    let mut profile_count = 0usize;
    let mut active_profile_count = 0usize;
    let mut non_raw_active_profile_count = 0usize;
    let mut quarantined_profile_count = 0usize;
    let mut revoked_profile_count = 0usize;
    let mut validated_package_count = 0usize;
    let mut validated_profile_count = 0usize;
    let mut active_future_rows = 0usize;
    let mut active_future_clean_rows = 0usize;
    let mut active_grounded_future_rows = 0usize;
    let mut active_grounded_future_clean_rows = 0usize;
    let mut active_false_accepts = 0usize;
    let mut active_runtime_parity_mismatches = 0usize;
    let mut active_latencies = Vec::new();
    let mut package_bytes = 0usize;

    for (package_id, record) in &registry.packages {
        profile_count = profile_count.saturating_add(record.profiles.len());
        let path = Path::new(&record.package_path);
        let package = match read_package(path) {
            Ok(package) => package,
            Err(error) => {
                package_validation_failures.push(format!("{package_id}:read:{error}"));
                continue;
            }
        };
        if let Err(error) = validate_live_package(&package, registry.policy.max_package_bytes) {
            package_validation_failures.push(format!("{package_id}:validate:{error}"));
            continue;
        }
        if package.package_id != *package_id || record.package_id != *package_id {
            package_validation_failures.push(format!("{package_id}:package_id_mismatch"));
            continue;
        }
        let actual_bytes = read_package_artifact_bytes(path)?;
        package_bytes = package_bytes.saturating_add(actual_bytes);
        if actual_bytes != record.package_bytes {
            package_accounting_warnings.push(format!(
                "{package_id}:package_bytes_mismatch:recorded={}:actual={actual_bytes}",
                record.package_bytes
            ));
        }
        if package.transitions.len() != record.profiles.len() {
            package_validation_failures.push(format!("{package_id}:profile_count_mismatch"));
        }
        validated_package_count = validated_package_count.saturating_add(1);

        for profile in &record.profiles {
            let Some(transition) = package.transitions.get(profile.transition_index) else {
                profile_linkage_failures.push(format!(
                    "{}:transition_index:{}",
                    profile.profile_id, profile.transition_index
                ));
                continue;
            };
            let expected_profile_id = format!("{package_id}:{}", profile.transition_index);
            let linkage_ok = profile.profile_id == expected_profile_id
                && profile.package_id == *package_id
                && profile.action_surface == transition.action_surface
                && profile.operator_kind == transition.program.action_kind
                && profile.adapter_name == transition.adapter.name
                && profile.guard_schema == transition.guard.schema
                && profile.verifier_schema == transition.verifier.schema
                && profile.phase_margin_micro
                    == package
                        .route_margin(profile.transition_index)
                        .unwrap_or(i64::MIN);
            if linkage_ok {
                validated_profile_count = validated_profile_count.saturating_add(1);
            } else {
                profile_linkage_failures
                    .push(format!("{}:linked_contract_mismatch", profile.profile_id));
            }

            match profile.state {
                LiveProfileState::Active => {
                    active_profile_count = active_profile_count.saturating_add(1);
                    if record.origin != LivePackageOrigin::RawPhaseInduction {
                        non_raw_active_profile_count =
                            non_raw_active_profile_count.saturating_add(1);
                        active_policy_failures
                            .push(format!("{}:non_raw_package_origin", profile.profile_id));
                    }
                    active_future_rows = active_future_rows.saturating_add(profile.future_rows);
                    active_future_clean_rows =
                        active_future_clean_rows.saturating_add(profile.future_clean_rows);
                    active_grounded_future_rows =
                        active_grounded_future_rows.saturating_add(profile.grounded_future_rows);
                    active_grounded_future_clean_rows = active_grounded_future_clean_rows
                        .saturating_add(profile.grounded_future_clean_rows);
                    active_false_accepts =
                        active_false_accepts.saturating_add(profile.false_accepts);
                    active_runtime_parity_mismatches = active_runtime_parity_mismatches
                        .saturating_add(profile.runtime_parity_mismatches);
                    active_latencies.extend_from_slice(&profile.execution_latency_ns);
                    if profile.future_clean_rows < registry.policy.min_future_clean_rows {
                        active_policy_failures
                            .push(format!("{}:future_denominator", profile.profile_id));
                    }
                    if profile.grounded_future_clean_rows < registry.policy.min_future_clean_rows {
                        active_policy_failures.push(format!(
                            "{}:grounded_future_denominator",
                            profile.profile_id
                        ));
                    }
                    if profile.false_accepts > registry.policy.max_false_accepts {
                        active_policy_failures
                            .push(format!("{}:false_accepts", profile.profile_id));
                    }
                    if profile.runtime_parity_mismatches
                        > registry.policy.max_runtime_parity_mismatches
                    {
                        active_policy_failures
                            .push(format!("{}:runtime_parity", profile.profile_id));
                    }
                    let p99 = percentile(&profile.execution_latency_ns, 99);
                    if p99 > registry.policy.max_execution_p99_ns {
                        active_policy_failures
                            .push(format!("{}:execution_p99", profile.profile_id));
                    }
                }
                LiveProfileState::Quarantine => {
                    quarantined_profile_count = quarantined_profile_count.saturating_add(1);
                }
                LiveProfileState::Revoked => {
                    revoked_profile_count = revoked_profile_count.saturating_add(1);
                }
            }
        }
    }

    let global_false_accepts = registry.telemetry.false_accepts;
    let global_runtime_parity_mismatches = registry.telemetry.runtime_parity_mismatches;
    if global_false_accepts != 0 {
        active_policy_failures.push("global_false_accepts".to_owned());
    }
    if global_runtime_parity_mismatches != 0 {
        active_policy_failures.push("global_runtime_parity_mismatches".to_owned());
    }
    if !registry.kill_switch_only_manual_control {
        active_policy_failures.push("manual_profile_controls_allowed".to_owned());
    }
    let local_accept_eligible = !registry.packages.is_empty()
        && active_profile_count > 0
        && non_raw_active_profile_count == 0
        && validated_package_count == registry.packages.len()
        && validated_profile_count == profile_count
        && package_validation_failures.is_empty()
        && profile_linkage_failures.is_empty()
        && active_policy_failures.is_empty();

    Ok(AdmissionReport {
        report_kind: "nando_transition_admission_inspection_v1",
        verdict: if local_accept_eligible {
            "PASS"
        } else {
            "VETO"
        },
        registry_path: path.display().to_string(),
        registry_revision: registry.revision,
        policy_version: registry.policy.version.clone(),
        package_count: registry.packages.len(),
        profile_count,
        active_profile_count,
        non_raw_active_profile_count,
        quarantined_profile_count,
        revoked_profile_count,
        validated_package_count,
        validated_profile_count,
        active_future_rows,
        active_future_clean_rows,
        active_grounded_future_rows,
        active_grounded_future_clean_rows,
        active_false_accepts,
        active_runtime_parity_mismatches,
        active_execution_p99_ns: percentile(&active_latencies, 99),
        package_bytes,
        traces_seen: registry.telemetry.traces_seen,
        shadow_executions: registry.telemetry.shadow_executions,
        global_false_accepts,
        global_runtime_parity_mismatches,
        kill_switch_only_manual_control: registry.kill_switch_only_manual_control,
        package_validation_failures,
        package_accounting_warnings,
        profile_linkage_failures,
        active_policy_failures,
        local_accept_eligible,
        boundary: "read-only package/profile/future-shadow inspection; does not promote or mutate registry",
    })
}

fn registry_path() -> PathBuf {
    env::var_os("NANDO_TRANSITION_REGISTRY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/nando-wave/transition/registry.json"))
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = sorted
        .len()
        .saturating_mul(percentile)
        .div_ceil(100)
        .saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}
