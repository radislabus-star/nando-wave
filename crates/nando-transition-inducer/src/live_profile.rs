use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::InducedTransitionPackage;

pub const LIVE_TRACE_SCHEMA: &str = "nando.live-observed-transition.v1";
pub const LIVE_GROUNDED_TRACE_SCHEMA: &str = "nando.live-observed-transition.v2";
pub const LIVE_REGISTRY_SCHEMA: &str = "nando.autonomous-transition-registry.v1";
pub const LIVE_POLICY_VERSION: &str = "autonomous_event_time_v2";
const LEGACY_LIVE_POLICY_VERSION: &str = "autonomous_zero_error_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveObservedTransition {
    pub schema: String,
    pub trace_id: String,
    #[serde(default)]
    pub timestamp: String,
    pub before: Value,
    pub action: Value,
    pub after: Value,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub request_sha256: String,
    #[serde(default)]
    pub evidence_source: String,
    #[serde(default)]
    pub evidence_verifier: String,
    #[serde(default)]
    pub evidence_receipt_sha256: String,
    #[serde(default)]
    pub source_session_id_sha256: String,
    #[serde(default)]
    pub source_event_id_sha256: String,
}

impl LiveObservedTransition {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != LIVE_TRACE_SCHEMA && self.schema != LIVE_GROUNDED_TRACE_SCHEMA {
            return Err("unsupported_live_trace_schema");
        }
        if self.trace_id.is_empty() {
            return Err("empty_trace_id");
        }
        if self.schema == LIVE_GROUNDED_TRACE_SCHEMA && !self.is_grounded() {
            return Err("invalid_grounded_trace_evidence");
        }
        Ok(())
    }

    #[must_use]
    pub fn is_grounded(&self) -> bool {
        self.schema == LIVE_GROUNDED_TRACE_SCHEMA
            && matches!(
                self.evidence_source.as_str(),
                "application_state" | "tool_result" | "environment_snapshot"
            )
            && !self.evidence_verifier.is_empty()
            && self.evidence_receipt_sha256.len() == 64
            && self
                .evidence_receipt_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveProfileState {
    Quarantine,
    Active,
    Revoked,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LivePackageOrigin {
    #[default]
    Imported,
    LegacyNamedInduction,
    RawPhaseInduction,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AutonomousPromotionPolicy {
    pub version: String,
    pub min_future_clean_rows: usize,
    pub max_false_accepts: usize,
    pub max_runtime_parity_mismatches: usize,
    pub max_package_bytes: usize,
    pub max_execution_p99_ns: u64,
    pub exact_cache_overlap_allowed: bool,
    pub auto_promote: bool,
    pub auto_demote_on_first_error: bool,
}

impl AutonomousPromotionPolicy {
    #[must_use]
    pub fn v1() -> Self {
        Self {
            version: LIVE_POLICY_VERSION.to_owned(),
            min_future_clean_rows: 16,
            max_false_accepts: 0,
            max_runtime_parity_mismatches: 0,
            max_package_bytes: 262_144,
            max_execution_p99_ns: 200_000_000,
            exact_cache_overlap_allowed: false,
            auto_promote: true,
            auto_demote_on_first_error: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveRuntimeProfile {
    pub profile_id: String,
    pub package_id: String,
    pub transition_index: usize,
    pub action_surface: String,
    pub operator_kind: String,
    pub adapter_name: String,
    pub state: LiveProfileState,
    pub phase_margin_micro: i64,
    pub routing_atoms: Vec<String>,
    pub guard_schema: String,
    pub verifier_schema: String,
    pub future_rows: usize,
    pub future_clean_rows: usize,
    #[serde(default)]
    pub grounded_future_rows: usize,
    #[serde(default)]
    pub grounded_future_clean_rows: usize,
    pub false_accepts: usize,
    pub runtime_parity_mismatches: usize,
    pub abstains: usize,
    pub negative_memory_rows: usize,
    #[serde(default)]
    pub execution_latency_ns: Vec<u64>,
    pub promoted_at_trace: Option<String>,
    pub revoked_at_trace: Option<String>,
    pub last_reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LivePackageRecord {
    pub package_id: String,
    pub package_path: String,
    pub package_bytes: usize,
    pub source_path: String,
    #[serde(default)]
    pub origin: LivePackageOrigin,
    pub imported_at_unix_secs: u64,
    #[serde(default)]
    pub future_evidence_not_before_unix_nanos: u64,
    pub profiles: Vec<LiveRuntimeProfile>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawPhaseFamilyState {
    pub family_key: String,
    pub stage: String,
    pub observed_surfaces: usize,
    pub eligible_surfaces: usize,
    pub support_rows: usize,
    pub verifier_positive_candidates: usize,
    pub verifier_negative_candidates: usize,
    pub compact_predicate_candidates: usize,
    pub discovered_predicates: usize,
    pub predicate_confidence_milli: usize,
    pub phase_circuit_ready: bool,
    pub training_attempts: usize,
    pub training_cpu_ns: u64,
    pub induction_cpu_ns: u64,
    pub package_ids: Vec<String>,
    pub last_reason: String,
    #[serde(default)]
    pub surface_frontier: BTreeMap<String, RawPhaseSurfaceState>,
    #[serde(default)]
    pub frontier_observed_rows: usize,
    #[serde(default)]
    pub frontier_covered_rows: usize,
    #[serde(default)]
    pub frontier_observed_tokens: u64,
    #[serde(default)]
    pub frontier_covered_tokens: u64,
    #[serde(default)]
    pub transfer_tested_surfaces: usize,
    #[serde(default)]
    pub transfer_passed_surfaces: usize,
    #[serde(default)]
    pub transfer_query_rows: usize,
    #[serde(default)]
    pub transfer_correct_executions: usize,
    #[serde(default)]
    pub transfer_abstains: usize,
    #[serde(default)]
    pub transfer_wrong_accepts: usize,
    #[serde(default)]
    pub leave_one_surface_out_pass: bool,
    #[serde(default)]
    pub new_session_split_pass: bool,
    #[serde(default)]
    pub session_transfer_query_rows: usize,
    #[serde(default)]
    pub session_transfer_correct_executions: usize,
    #[serde(default)]
    pub session_transfer_abstains: usize,
    #[serde(default)]
    pub session_transfer_wrong_accepts: usize,
    #[serde(default)]
    pub forward_time_split_pass: bool,
    #[serde(default)]
    pub time_transfer_query_rows: usize,
    #[serde(default)]
    pub time_transfer_correct_executions: usize,
    #[serde(default)]
    pub time_transfer_abstains: usize,
    #[serde(default)]
    pub time_transfer_wrong_accepts: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawPhaseSurfaceState {
    pub surface_key: String,
    pub observed_rows: usize,
    pub observed_tokens: u64,
    pub session_count: usize,
    pub first_timestamp: String,
    pub last_timestamp: String,
    pub eligible_for_training: bool,
    pub circuit_covered: bool,
    pub package_id: Option<String>,
    pub last_reason: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveTransitionTelemetry {
    pub traces_seen: usize,
    pub traces_invalid: usize,
    pub shadow_executions: usize,
    pub shadow_abstains: usize,
    pub false_accepts: usize,
    pub runtime_parity_mismatches: usize,
    pub packages_imported: usize,
    pub packages_deduplicated: usize,
    pub packages_induced: usize,
    pub profiles_created: usize,
    pub profiles_promoted: usize,
    pub profiles_revoked: usize,
    pub active_local_accepts: usize,
    pub llm_calls_avoided: usize,
    pub tokens_saved: u64,
    pub total_bridge_requests: usize,
    pub total_bridge_tokens: u64,
    pub execution_latency_ns: Vec<u64>,
    #[serde(default)]
    pub raw_phase_training_attempts: usize,
    #[serde(default)]
    pub raw_phase_packages_induced: usize,
    #[serde(default)]
    pub raw_phase_training_cpu_ns: u64,
    #[serde(default)]
    pub raw_phase_induction_cpu_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveProfileRegistry {
    pub schema: String,
    pub policy: AutonomousPromotionPolicy,
    pub revision: u64,
    pub trace_watermark_rows: usize,
    #[serde(default)]
    pub trace_watermark_bytes: u64,
    #[serde(default)]
    pub execution_event_watermark_bytes: u64,
    pub packages: BTreeMap<String, LivePackageRecord>,
    #[serde(default)]
    pub seen_trace_ids: BTreeSet<String>,
    #[serde(default)]
    pub seen_inbox_files: BTreeSet<String>,
    #[serde(default)]
    pub induced_class_keys: BTreeSet<String>,
    #[serde(default)]
    pub induced_raw_family_keys: BTreeSet<String>,
    #[serde(default)]
    pub raw_phase_families: BTreeMap<String, RawPhaseFamilyState>,
    #[serde(default)]
    pub seen_bridge_request_ids: BTreeSet<String>,
    #[serde(default)]
    pub seen_local_accept_request_ids: BTreeSet<String>,
    #[serde(default)]
    pub telemetry: LiveTransitionTelemetry,
    pub kill_switch_only_manual_control: bool,
    pub boundary: String,
}

impl Default for LiveProfileRegistry {
    fn default() -> Self {
        Self {
            schema: LIVE_REGISTRY_SCHEMA.to_owned(),
            policy: AutonomousPromotionPolicy::v1(),
            revision: 0,
            trace_watermark_rows: 0,
            trace_watermark_bytes: 0,
            execution_event_watermark_bytes: 0,
            packages: BTreeMap::new(),
            seen_trace_ids: BTreeSet::new(),
            seen_inbox_files: BTreeSet::new(),
            induced_class_keys: BTreeSet::new(),
            induced_raw_family_keys: BTreeSet::new(),
            raw_phase_families: BTreeMap::new(),
            seen_bridge_request_ids: BTreeSet::new(),
            seen_local_accept_request_ids: BTreeSet::new(),
            telemetry: LiveTransitionTelemetry::default(),
            kill_switch_only_manual_control: true,
            boundary: "autonomous profile lifecycle; quarantine cannot serve; active profiles require grounded event-time future zero-error evidence; any mismatch revokes and falls back".to_owned(),
        }
    }
}

impl LiveProfileRegistry {
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes =
            fs::read(path).map_err(|error| format!("registry_read:{}:{error}", path.display()))?;
        let mut registry: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("registry_json:{}:{error}", path.display()))?;
        if registry.schema != LIVE_REGISTRY_SCHEMA {
            return Err("unsupported_live_registry_schema".to_owned());
        }
        let legacy_policy = registry.policy.version == LEGACY_LIVE_POLICY_VERSION;
        if !legacy_policy && registry.policy.version != LIVE_POLICY_VERSION {
            return Err("unsupported_live_policy_version".to_owned());
        }
        let missing_cutoff = registry.packages.values().any(|record| {
            record.origin == LivePackageOrigin::RawPhaseInduction
                && record.future_evidence_not_before_unix_nanos == 0
        });
        if legacy_policy || missing_cutoff {
            registry.migrate_event_time_future_evidence();
        }
        Ok(registry)
    }

    fn migrate_event_time_future_evidence(&mut self) {
        let mut package_cutoffs = BTreeMap::<String, u64>::new();
        for family in self.raw_phase_families.values() {
            let cutoff = family
                .surface_frontier
                .values()
                .filter_map(|surface| timestamp_unix_nanos(&surface.last_timestamp))
                .max();
            let Some(cutoff) = cutoff else {
                continue;
            };
            for package_id in &family.package_ids {
                package_cutoffs
                    .entry(package_id.clone())
                    .and_modify(|current| *current = (*current).max(cutoff))
                    .or_insert(cutoff);
            }
        }
        for record in self
            .packages
            .values_mut()
            .filter(|record| record.origin == LivePackageOrigin::RawPhaseInduction)
        {
            let fallback = record.imported_at_unix_secs.saturating_mul(1_000_000_000);
            record.future_evidence_not_before_unix_nanos = package_cutoffs
                .get(&record.package_id)
                .copied()
                .unwrap_or(fallback);
            for profile in &mut record.profiles {
                if profile.state != LiveProfileState::Revoked {
                    profile.state = LiveProfileState::Quarantine;
                    profile.future_rows = 0;
                    profile.future_clean_rows = 0;
                    profile.grounded_future_rows = 0;
                    profile.grounded_future_clean_rows = 0;
                    profile.execution_latency_ns.clear();
                    profile.promoted_at_trace = None;
                    profile.last_reason = "awaiting_event_time_future_shadow".to_owned();
                }
            }
        }
        self.policy.version = LIVE_POLICY_VERSION.to_owned();
        self.boundary = "autonomous profile lifecycle; quarantine cannot serve; active profiles require grounded event-time future zero-error evidence; any mismatch revokes and falls back".to_owned();
    }

    pub fn save(&mut self, path: &Path) -> Result<(), String> {
        self.revision = self.revision.saturating_add(1);
        atomic_write_json(path, self)
    }

    #[must_use]
    pub fn active_profile_indices(&self, package_id: &str) -> Vec<usize> {
        self.packages
            .get(package_id)
            .filter(|record| record.origin == LivePackageOrigin::RawPhaseInduction)
            .map(|record| {
                record
                    .profiles
                    .iter()
                    .filter(|profile| profile.state == LiveProfileState::Active)
                    .map(|profile| profile.transition_index)
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn active_profile_count(&self) -> usize {
        self.packages
            .values()
            .filter(|package| package.origin == LivePackageOrigin::RawPhaseInduction)
            .flat_map(|package| package.profiles.iter())
            .filter(|profile| profile.state == LiveProfileState::Active)
            .count()
    }

    #[must_use]
    pub fn non_raw_active_profile_count(&self) -> usize {
        self.packages
            .values()
            .filter(|package| package.origin != LivePackageOrigin::RawPhaseInduction)
            .flat_map(|package| package.profiles.iter())
            .filter(|profile| profile.state == LiveProfileState::Active)
            .count()
    }

    #[must_use]
    pub fn quarantined_profile_count(&self) -> usize {
        self.packages
            .values()
            .flat_map(|package| package.profiles.iter())
            .filter(|profile| profile.state == LiveProfileState::Quarantine)
            .count()
    }

    #[must_use]
    pub fn revoked_profile_count(&self) -> usize {
        self.packages
            .values()
            .flat_map(|package| package.profiles.iter())
            .filter(|profile| profile.state == LiveProfileState::Revoked)
            .count()
    }
}

pub fn import_package(
    registry: &mut LiveProfileRegistry,
    package: &InducedTransitionPackage,
    source_path: &Path,
    state_dir: &Path,
    induced: bool,
) -> Result<bool, String> {
    let origin = if induced {
        LivePackageOrigin::RawPhaseInduction
    } else {
        LivePackageOrigin::Imported
    };
    import_package_with_origin(registry, package, source_path, state_dir, origin)
}

pub fn import_package_with_origin(
    registry: &mut LiveProfileRegistry,
    package: &InducedTransitionPackage,
    source_path: &Path,
    state_dir: &Path,
    origin: LivePackageOrigin,
) -> Result<bool, String> {
    validate_live_package(package, registry.policy.max_package_bytes)?;
    if registry.packages.contains_key(&package.package_id) {
        registry.telemetry.packages_deduplicated =
            registry.telemetry.packages_deduplicated.saturating_add(1);
        return Ok(false);
    }
    let package_dir = state_dir.join("packages");
    fs::create_dir_all(&package_dir)
        .map_err(|error| format!("package_dir:{}:{error}", package_dir.display()))?;
    let package_path = package_dir.join(format!("{}.json", package.package_id));
    atomic_write_json(&package_path, package)?;
    let package_bytes = package
        .artifact_bytes()
        .map_err(|error| format!("package_bytes:{error}"))?
        .len();
    let imported_at_unix_secs = unix_secs();
    let profiles = package
        .transitions
        .iter()
        .enumerate()
        .map(|(index, transition)| LiveRuntimeProfile {
            profile_id: format!("{}:{index}", package.package_id),
            package_id: package.package_id.clone(),
            transition_index: index,
            action_surface: transition.action_surface.clone(),
            operator_kind: transition.program.action_kind.clone(),
            adapter_name: transition.adapter.name.clone(),
            state: LiveProfileState::Quarantine,
            phase_margin_micro: package.route_margin(index).unwrap_or(i64::MIN),
            routing_atoms: transition.routing_atoms.clone(),
            guard_schema: transition.guard.schema.clone(),
            verifier_schema: transition.verifier.schema.clone(),
            future_rows: 0,
            future_clean_rows: 0,
            grounded_future_rows: 0,
            grounded_future_clean_rows: 0,
            false_accepts: 0,
            runtime_parity_mismatches: 0,
            abstains: 0,
            negative_memory_rows: 0,
            execution_latency_ns: Vec::new(),
            promoted_at_trace: None,
            revoked_at_trace: None,
            last_reason: "awaiting_future_shadow".to_owned(),
        })
        .collect::<Vec<_>>();
    registry.telemetry.packages_imported = registry.telemetry.packages_imported.saturating_add(1);
    registry.telemetry.profiles_created = registry
        .telemetry
        .profiles_created
        .saturating_add(profiles.len());
    if origin != LivePackageOrigin::Imported {
        registry.telemetry.packages_induced = registry.telemetry.packages_induced.saturating_add(1);
    }
    registry.packages.insert(
        package.package_id.clone(),
        LivePackageRecord {
            package_id: package.package_id.clone(),
            package_path: package_path.display().to_string(),
            package_bytes,
            source_path: source_path.display().to_string(),
            origin,
            imported_at_unix_secs,
            future_evidence_not_before_unix_nanos: imported_at_unix_secs
                .saturating_mul(1_000_000_000),
            profiles,
        },
    );
    Ok(true)
}

#[must_use]
pub fn timestamp_unix_nanos(value: &str) -> Option<u64> {
    let timestamp = OffsetDateTime::parse(value, &Rfc3339).ok()?;
    u64::try_from(timestamp.unix_timestamp_nanos()).ok()
}

pub fn packages_from_value(value: &Value) -> Result<Vec<InducedTransitionPackage>, String> {
    if let Ok(package) = serde_json::from_value::<InducedTransitionPackage>(value.clone()) {
        return Ok(vec![package]);
    }
    let candidates = value
        .get("canonical_proof")
        .and_then(|proof| proof.get("induced_packages"))
        .or_else(|| value.get("induced_packages"))
        .and_then(Value::as_array)
        .ok_or_else(|| "package_bundle_missing_induced_packages".to_owned())?;
    candidates
        .iter()
        .map(|package| {
            serde_json::from_value(package.clone())
                .map_err(|error| format!("induced_package_json:{error}"))
        })
        .collect()
}

pub fn read_package(path: &Path) -> Result<InducedTransitionPackage, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("package_read:{}:{error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("package_json:{}:{error}", path.display()))
}

pub fn validate_live_package(
    package: &InducedTransitionPackage,
    max_package_bytes: usize,
) -> Result<(), String> {
    if package.schema != "nando.induced-transition-package.v1" {
        return Err("unsupported_induced_package_schema".to_owned());
    }
    if package.package_id.is_empty() || package.transitions.is_empty() {
        return Err("empty_induced_package".to_owned());
    }
    let bytes = package
        .artifact_bytes()
        .map_err(|error| format!("package_serialization:{error}"))?;
    if bytes.len() > max_package_bytes {
        return Err("package_budget_exceeded".to_owned());
    }
    let roundtrip: InducedTransitionPackage =
        serde_json::from_slice(&bytes).map_err(|error| format!("package_roundtrip:{error}"))?;
    if &roundtrip != package {
        return Err("package_roundtrip_mismatch".to_owned());
    }
    for (index, transition) in package.transitions.iter().enumerate() {
        transition
            .program
            .validate()
            .map_err(|error| format!("program:{index}:{error}"))?;
        transition
            .adapter
            .validate()
            .map_err(|error| format!("adapter:{index}:{error}"))?;
        if transition.guard.schema != "nando.transition-guard.v1"
            || transition.verifier.schema != "nando.transition-verifier.v1"
        {
            return Err(format!("safety_profile_schema:{index}"));
        }
        if package.route_margin(index).is_none_or(|margin| margin <= 0) {
            return Err(format!("routing_profile_margin:{index}"));
        }
    }
    Ok(())
}

pub fn atomic_write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("output_dir:{}:{error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| format!("json:{error}"))?;
    let next = PathBuf::from(format!("{}.next", path.display()));
    fs::write(&next, bytes).map_err(|error| format!("write:{}:{error}", next.display()))?;
    fs::rename(&next, path)
        .map_err(|error| format!("rename:{}:{}:{error}", next.display(), path.display()))
}

fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
