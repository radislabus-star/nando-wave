use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{
    K2AlternativePredictionSetV1, K2AuthorityBoundaryV1, K2DecisionEpisodeSealV1,
    K2DecisionFreezeV1, K2DecisionOutcomeReceiptV1, K2EvidenceProvenanceV1, K2ExactGoalReceiptV1,
    K2GoalEnvironmentErrorV1, K2GoalEnvironmentResultV1, K2K1ActionRefInputV1, K2K1ActionRefV1,
    K2LawLabBindingV1, K2PreparedSelectionReceiptV1,
};
use crate::{
    LAW_LAB_TREE_MANIFEST_SCHEMA_V1, LawLabProbeDomainV1, LawLabSandboxExecutionV1,
    LawLabSandboxOperationV1, LawLabSandboxPurposeV1, LawLabSandboxRequestV1,
    LawLabTreeEntryKindV1, LawLabTreeEntryV1, LawLabTreeManifestV1, law_lab_sha256_file_v1,
};

pub const K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1: &str = "nando.k2-opaque-action-catalog.v1";
pub const K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1: &str = "nando.k2-learner-public-context.v1";
pub const K2_HIDDEN_ACTION_MAPPING_SCHEMA_V1: &str = "nando.k2-hidden-action-mapping.v1";
pub const K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1: &str =
    "nando.k2-private-experiment-contract.v1";
pub const K2_SUPPORT_WORLD_SCHEMA_V1: &str = "nando.k2-support-world.v1";
pub const K2_SUPPORT_WORLD_SET_SCHEMA_V1: &str = "nando.k2-support-world-set.v1";
pub const K2_SUPPORT_PROBE_PLAN_SCHEMA_V1: &str = "nando.k2-support-probe-plan.v1";
pub const K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1: &str = "nando.k2-learned-capability-freeze.v1";
pub const K2_SUPPORT_DISPATCH_SCHEMA_V1: &str = "nando.k2-support-dispatch.v1";
pub const K2_SUPPORT_OBSERVATION_SCHEMA_V1: &str = "nando.k2-support-observation.v1";
pub const K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1: &str = "nando.k2-support-observation-set.v1";
pub const K2_EFFECT_LEARNER_MANIFEST_SCHEMA_V1: &str = "nando.k2-effect-learner-manifest.v1";
pub const K2_LEARNED_CAPABILITY_BUDGET_SCHEMA_V1: &str = "nando.k2-learned-capability-budget.v1";
pub const K2_EFFECT_LANGUAGE_SCHEMA_V1: &str = "nando.k2-bounded-effect-language.v1";
pub const K2_EFFECT_LEARNING_REQUEST_SCHEMA_V1: &str = "nando.k2-effect-learning-request.v1";
pub const K2_LEARNED_EFFECT_LAW_SCHEMA_V1: &str = "nando.k2-learned-effect-law.v1";
pub const K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1: &str = "nando.k2-learned-effect-law-set.v1";
pub const K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-target-independence-receipt.v1";
pub const K2_TARGET_PREDICTION_REQUEST_SCHEMA_V1: &str = "nando.k2-target-prediction-request.v1";
pub const K2_LEARNED_TARGET_PREDICTION_SCHEMA_V1: &str = "nando.k2-learned-target-prediction.v1";
pub const K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1: &str =
    "nando.k2-learned-target-prediction-set.v1";
pub const K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1: &str =
    "nando.k2-learned-effect-verification.v1";
pub const K2_LEARNED_TO_V1_BINDING_SCHEMA_V1: &str = "nando.k2-learned-to-v1-binding.v1";
pub const K2_V1_EPISODE_EVIDENCE_SCHEMA_V1: &str = "nando.k2-v1-episode-evidence.v1";
pub const K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1: &str = "nando.k2-learned-ablation-receipt.v1";
pub const K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1: &str = "nando.k2-learned-capability-outcome.v1";
pub const K2_LEARNED_CAPABILITY_SEAL_SCHEMA_V1: &str = "nando.k2-learned-capability-seal.v1";
pub const K2_EFFECT_LEARNER_PROTOCOL_SCHEMA_V1: &str = "nando.k2-effect-learner-protocol.v1";
pub const K2_EFFECT_LEARNER_PROCESS_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-effect-learner-process-receipt.v1";
pub const K2_PRIVATE_EXPERIMENT_ARTIFACT_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-private-experiment-artifact-receipt.v1";
pub const K2_LEARNED_TO_V1_BINDING_ENTRY_SCHEMA_V1: &str =
    "nando.k2-learned-to-v1-binding-entry.v1";
pub const K2_LEARNED_ABLATION_CONTROL_SCHEMA_V1: &str = "nando.k2-learned-ablation-control.v1";
pub const K2_GENERATED_ABLATION_OBSERVATION_SCHEMA_V1: &str =
    "nando.k2-generated-ablation-observation.v1";
pub const K2_GENERATED_ABLATION_REQUEST_SCHEMA_V1: &str = "nando.k2-generated-ablation-request.v1";
pub const K2_GENERATED_ABLATION_OUTCOME_SCHEMA_V1: &str = "nando.k2-generated-ablation-outcome.v1";
pub const K2_INDEPENDENT_EFFECT_VERIFIER_CONTRACT_V1: &str =
    "nando.k2-independent-effect-verifier.v1";
pub const K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS_V1: &str =
    "K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS";

pub const K2_LEARNED_SUPPORT_WORLD_COUNT_V1: usize = 3;
pub const K2_LEARNED_ACTION_COUNT_V1: usize = 2;
pub const K2_LEARNED_SUPPORT_PROBE_COUNT_V1: usize = 6;
pub const K2_LEARNED_MAX_TREE_ENTRIES_V1: usize = 32;
pub const K2_LEARNED_MAX_TREE_BYTES_V1: u64 = 64 * 1024;
pub const K2_LEARNED_MAX_CANDIDATES_PER_ACTION_V1: usize = 32;
pub const K2_LEARNER_MAX_REQUEST_BYTES_V1: usize = 512 * 1024;
pub const K2_LEARNER_MAX_OUTCOME_BYTES_V1: usize = 128 * 1024;
pub const K2_LEARNER_MAX_STDERR_BYTES_V1: usize = 4 * 1024;
pub const K2_LEARNER_WALL_MS_V1: u64 = 2_000;
pub const K2_LEARNER_CPU_SECONDS_V1: u64 = 1;
pub const K2_LEARNER_ADDRESS_SPACE_BYTES_V1: u64 = 256 * 1024 * 1024;
pub const K2_LEARNER_PROCESS_COUNT_V1: u64 = 2;

const K2_COPY_SOURCE_PATH_V1: &str = "input.bin";
const K2_COPY_TARGET_PATH_V1: &str = "selected.bin";
const K2_REMOVE_PATH_V1: &str = "obsolete.bin";

fn learned_root_v1<T: Serialize>(value: &T) -> K2GoalEnvironmentResultV1<String> {
    canonical_json_sha256(value).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
}

fn learned_bytes_v1<T: Serialize>(value: &T) -> K2GoalEnvironmentResultV1<Vec<u8>> {
    canonical_json_bytes(value).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
}

fn require_learned_root_v1(root: &str, reason: &'static str) -> K2GoalEnvironmentResultV1<()> {
    if valid_nonzero_sha256(root) {
        Ok(())
    } else {
        Err(K2GoalEnvironmentErrorV1::Invalid(reason))
    }
}

fn require_unique_roots_v1<'a>(
    roots: impl IntoIterator<Item = &'a str>,
    reason: &'static str,
) -> K2GoalEnvironmentResultV1<()> {
    let roots = roots.into_iter().collect::<Vec<_>>();
    if roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.iter().copied().collect::<BTreeSet<_>>().len() == roots.len()
    {
        Ok(())
    } else {
        Err(K2GoalEnvironmentErrorV1::Invalid(reason))
    }
}

fn parse_canonical_v1<T>(
    bytes: &[u8],
    maximum_bytes: usize,
    reason: &'static str,
) -> K2GoalEnvironmentResultV1<T>
where
    T: DeserializeOwned + Serialize,
{
    if bytes.len() > maximum_bytes {
        return Err(K2GoalEnvironmentErrorV1::Invalid(reason));
    }
    let value: T =
        serde_json::from_slice(bytes).map_err(|_| K2GoalEnvironmentErrorV1::Invalid(reason))?;
    if learned_bytes_v1(&value)? != bytes {
        return Err(K2GoalEnvironmentErrorV1::Invalid(reason));
    }
    Ok(value)
}

fn validate_fixture_path_v1(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 256
        && !path.starts_with('/')
        && !path.ends_with('/')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

include!("learned_capability/model.rs");
include!("learned_capability/protocol.rs");
include!("learned_capability/receipt.rs");
