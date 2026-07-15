use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    InducedExecutionStatus, InducedTransitionPackage, LivePackageOrigin, LiveProfileRegistry,
    LiveProfileState, read_package,
};

pub const LIVE_TRANSITION_REQUEST_SCHEMA: &str = "nando.live-transition-request.v1";
pub const LIVE_TRANSITION_RESPONSE_SCHEMA: &str = "nando.live-transition-response.v1";
pub const LIVE_TRANSITION_MAX_REQUEST_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LiveTransitionRequest {
    pub schema: String,
    pub before: Value,
    pub action: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveTransitionResponse {
    pub local_accept: bool,
    pub verifier_ok: bool,
    pub false_accepts: usize,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_receipt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_after_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier_schema: Option<String>,
    pub elapsed_ns: u64,
}

impl LiveTransitionResponse {
    #[must_use]
    pub fn decline(reason: impl Into<String>, elapsed_ns: u64) -> Self {
        Self {
            local_accept: false,
            verifier_ok: false,
            false_accepts: 0,
            reason: reason.into(),
            route: None,
            response: None,
            verification_receipt_id: None,
            verified_after_digest: None,
            verifier_schema: None,
            elapsed_ns,
        }
    }
}

struct LoadedProfile {
    profile_id: String,
    operator_kind: String,
}

struct LoadedPackage {
    package: InducedTransitionPackage,
    active_profiles: BTreeMap<usize, LoadedProfile>,
}

pub struct LiveTransitionExecutor {
    packages: Vec<LoadedPackage>,
    registry_revision: u64,
    active_profile_count: usize,
}

impl LiveTransitionExecutor {
    pub fn load(registry_path: &Path) -> Result<Self, String> {
        let registry = LiveProfileRegistry::load(registry_path)?;
        let mut packages = Vec::new();
        let mut active_profile_count = 0usize;

        for record in registry.packages.values() {
            if record.origin != LivePackageOrigin::RawPhaseInduction {
                continue;
            }
            let active_profiles = record
                .profiles
                .iter()
                .filter(|profile| profile.state == LiveProfileState::Active)
                .map(|profile| {
                    (
                        profile.transition_index,
                        LoadedProfile {
                            profile_id: profile.profile_id.clone(),
                            operator_kind: profile.operator_kind.clone(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if active_profiles.is_empty() {
                continue;
            }
            let package_path = PathBuf::from(&record.package_path);
            let package = read_package(&package_path)?;
            for index in active_profiles.keys() {
                if package.transitions.get(*index).is_none() {
                    return Err(format!(
                        "active_profile_transition_missing:{}:{index}",
                        package.package_id
                    ));
                }
            }
            active_profile_count = active_profile_count.saturating_add(active_profiles.len());
            packages.push(LoadedPackage {
                package,
                active_profiles,
            });
        }

        Ok(Self {
            packages,
            registry_revision: registry.revision,
            active_profile_count,
        })
    }

    #[must_use]
    pub fn registry_revision(&self) -> u64 {
        self.registry_revision
    }

    #[must_use]
    pub fn active_profile_count(&self) -> usize {
        self.active_profile_count
    }

    #[must_use]
    pub fn execute(&self, request: &LiveTransitionRequest) -> LiveTransitionResponse {
        let started = std::time::Instant::now();
        if request.schema != LIVE_TRANSITION_REQUEST_SCHEMA {
            return LiveTransitionResponse::decline(
                "unsupported_live_transition_request_schema",
                elapsed_ns(started),
            );
        }

        for loaded in &self.packages {
            let execution = loaded.package.execute_routed_indices(
                &request.before,
                &request.action,
                loaded.active_profiles.keys().copied(),
            );
            if execution.status != InducedExecutionStatus::Executed {
                continue;
            }
            let Some(after) = execution.after else {
                continue;
            };
            let Some(index) = execution.transition_index else {
                continue;
            };
            let Some(profile) = loaded.active_profiles.get(&index) else {
                continue;
            };
            let payload = json!({
                "schema": LIVE_TRANSITION_RESPONSE_SCHEMA,
                "status": "executed",
                "package_id": loaded.package.package_id,
                "profile_id": profile.profile_id,
                "operator_kind": profile.operator_kind,
                "after": after,
            });
            let Some(after) = payload.get("after") else {
                return LiveTransitionResponse::decline(
                    "verified_after_missing",
                    elapsed_ns(started),
                );
            };
            let Ok(verified_after_digest) = sha256_json(after) else {
                return LiveTransitionResponse::decline(
                    "verified_after_digest_failed",
                    elapsed_ns(started),
                );
            };
            let receipt = json!({
                "schema": "nando.transition-verification-receipt.v1",
                "package_id": loaded.package.package_id,
                "profile_id": profile.profile_id,
                "operator_kind": profile.operator_kind,
                "before": request.before,
                "action": request.action,
                "verified_after_digest": verified_after_digest,
                "verifier_schema": "typed_actor_independent_verifier.v1",
            });
            let Ok(verification_receipt_id) = sha256_json(&receipt) else {
                return LiveTransitionResponse::decline(
                    "verification_receipt_failed",
                    elapsed_ns(started),
                );
            };
            let Ok(response) = serde_json::to_string(&payload) else {
                return LiveTransitionResponse::decline(
                    "response_serialization_failed",
                    elapsed_ns(started),
                );
            };
            return LiveTransitionResponse {
                local_accept: true,
                verifier_ok: true,
                false_accepts: 0,
                reason: "active_profile_guard_actor_verifier_pass".to_owned(),
                route: Some(format!("typed_transition:{}", profile.profile_id)),
                response: Some(response),
                verification_receipt_id: Some(verification_receipt_id),
                verified_after_digest: Some(verified_after_digest),
                verifier_schema: Some("typed_actor_independent_verifier.v1".to_owned()),
                elapsed_ns: elapsed_ns(started),
            };
        }
        LiveTransitionResponse::decline("no_active_transition_profile", elapsed_ns(started))
    }
}

fn sha256_json(value: &Value) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn elapsed_ns(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decline_has_no_authority_receipts() {
        let response = LiveTransitionResponse::decline("blocked", 10);
        assert!(!response.local_accept);
        assert!(!response.verifier_ok);
        assert!(response.response.is_none());
        assert!(response.verification_receipt_id.is_none());
        assert!(response.verified_after_digest.is_none());
    }
}
