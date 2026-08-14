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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedEffectV1 {
    CopyFile,
    RemoveFile,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "effect", rename_all = "snake_case", deny_unknown_fields)]
pub enum K2LearnedEffectLawBodyV1 {
    CopyFile {
        source_path: String,
        target_path: String,
    },
    RemoveFile {
        path: String,
    },
}

impl K2LearnedEffectLawBodyV1 {
    fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        let valid = match self {
            Self::CopyFile {
                source_path,
                target_path,
            } => {
                validate_fixture_path_v1(source_path)
                    && validate_fixture_path_v1(target_path)
                    && source_path != target_path
            }
            Self::RemoveFile { path } => validate_fixture_path_v1(path),
        };
        if valid {
            Ok(())
        } else {
            Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_effect_path_invalid",
            ))
        }
    }

    #[must_use]
    pub fn operation_v1(&self) -> LawLabSandboxOperationV1 {
        match self {
            Self::CopyFile {
                source_path,
                target_path,
            } => LawLabSandboxOperationV1::CopySourceFile {
                source_path: source_path.clone(),
                work_path: target_path.clone(),
            },
            Self::RemoveFile { path } => LawLabSandboxOperationV1::RemoveWorkPath {
                work_path: path.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilityBudgetV1 {
    pub schema: String,
    pub support_worlds: u64,
    pub opaque_actions: u64,
    pub support_probes: u64,
    pub target_probes: u64,
    pub maximum_tree_entries: u64,
    pub maximum_tree_bytes: u64,
    pub maximum_candidates_per_action: u64,
    pub maximum_learning_request_bytes: u64,
    pub maximum_learner_outcome_bytes: u64,
    pub learner_wall_ms: u64,
    pub learner_cpu_seconds: u64,
    pub learner_address_space_bytes: u64,
    pub learner_process_count: u64,
}

impl K2LearnedCapabilityBudgetV1 {
    #[must_use]
    pub fn preregistered_v1() -> Self {
        Self {
            schema: K2_LEARNED_CAPABILITY_BUDGET_SCHEMA_V1.to_owned(),
            support_worlds: K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64,
            opaque_actions: K2_LEARNED_ACTION_COUNT_V1 as u64,
            support_probes: K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64,
            target_probes: 1,
            maximum_tree_entries: K2_LEARNED_MAX_TREE_ENTRIES_V1 as u64,
            maximum_tree_bytes: K2_LEARNED_MAX_TREE_BYTES_V1,
            maximum_candidates_per_action: K2_LEARNED_MAX_CANDIDATES_PER_ACTION_V1 as u64,
            maximum_learning_request_bytes: K2_LEARNER_MAX_REQUEST_BYTES_V1 as u64,
            maximum_learner_outcome_bytes: K2_LEARNER_MAX_OUTCOME_BYTES_V1 as u64,
            learner_wall_ms: K2_LEARNER_WALL_MS_V1,
            learner_cpu_seconds: K2_LEARNER_CPU_SECONDS_V1,
            learner_address_space_bytes: K2_LEARNER_ADDRESS_SPACE_BYTES_V1,
            learner_process_count: K2_LEARNER_PROCESS_COUNT_V1,
        }
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        if self == &Self::preregistered_v1() {
            Ok(())
        } else {
            Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_budget_invalid",
            ))
        }
    }

    pub fn root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        self.validate()?;
        learned_root_v1(&(K2_LEARNED_CAPABILITY_BUDGET_SCHEMA_V1, self))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2EffectLearnerManifestV1 {
    pub schema: String,
    pub manifest_root_sha256: String,
    pub executable_sha256: String,
    pub protocol_schema: String,
    pub effect_language_root_sha256: String,
}

impl K2EffectLearnerManifestV1 {
    pub fn seal(executable_sha256: String) -> K2GoalEnvironmentResultV1<Self> {
        require_learned_root_v1(
            &executable_sha256,
            "k2_effect_learner_executable_sha_invalid",
        )?;
        let effect_language_root_sha256 = bounded_effect_language_root_v1()?;
        let mut value = Self {
            schema: K2_EFFECT_LEARNER_MANIFEST_SCHEMA_V1.to_owned(),
            manifest_root_sha256: String::new(),
            executable_sha256,
            protocol_schema: K2_EFFECT_LEARNER_PROTOCOL_SCHEMA_V1.to_owned(),
            effect_language_root_sha256,
        };
        value.manifest_root_sha256 = value.expected_root_v1()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        require_learned_root_v1(
            &self.executable_sha256,
            "k2_effect_learner_executable_sha_invalid",
        )?;
        if self.schema != K2_EFFECT_LEARNER_MANIFEST_SCHEMA_V1
            || self.protocol_schema != K2_EFFECT_LEARNER_PROTOCOL_SCHEMA_V1
            || self.effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.manifest_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_manifest_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_EFFECT_LEARNER_MANIFEST_SCHEMA_V1,
            self.executable_sha256.as_str(),
            self.protocol_schema.as_str(),
            self.effect_language_root_sha256.as_str(),
        ))
    }
}

pub fn bounded_effect_language_root_v1() -> K2GoalEnvironmentResultV1<String> {
    learned_root_v1(&(
        K2_EFFECT_LANGUAGE_SCHEMA_V1,
        ["copy_file{source_path,target_path}", "remove_file{path}"],
    ))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2OpaqueActionCatalogV1 {
    pub schema: String,
    pub catalog_root_sha256: String,
    pub action_ids_sha256: Vec<String>,
}

impl K2OpaqueActionCatalogV1 {
    pub fn from_harness_commitment_v1(
        harness_commitment_sha256: &str,
    ) -> K2GoalEnvironmentResultV1<Self> {
        require_learned_root_v1(harness_commitment_sha256, "k2_harness_commitment_invalid")?;
        let mut action_ids_sha256 = (0_u64..K2_LEARNED_ACTION_COUNT_V1 as u64)
            .map(|slot| {
                learned_root_v1(&(
                    K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1,
                    "opaque-action-id",
                    harness_commitment_sha256,
                    slot,
                ))
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        action_ids_sha256.sort();
        Self::seal(action_ids_sha256)
    }

    pub fn seal(mut action_ids_sha256: Vec<String>) -> K2GoalEnvironmentResultV1<Self> {
        action_ids_sha256.sort();
        require_unique_roots_v1(
            action_ids_sha256.iter().map(String::as_str),
            "k2_opaque_action_ids_invalid",
        )?;
        if action_ids_sha256.len() != K2_LEARNED_ACTION_COUNT_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_opaque_action_count_invalid",
            ));
        }
        let mut catalog = Self {
            schema: K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1.to_owned(),
            catalog_root_sha256: String::new(),
            action_ids_sha256,
        };
        catalog.catalog_root_sha256 = catalog.expected_root_v1()?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        require_unique_roots_v1(
            self.action_ids_sha256.iter().map(String::as_str),
            "k2_opaque_action_ids_invalid",
        )?;
        if self.schema != K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1
            || self.action_ids_sha256.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .action_ids_sha256
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.catalog_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_opaque_action_catalog_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(K2_OPAQUE_ACTION_CATALOG_SCHEMA_V1, &self.action_ids_sha256))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2HiddenActionEntryV1 {
    pub action_id_sha256: String,
    pub effect: K2LearnedEffectLawBodyV1,
    pub operation_plan_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2HiddenActionMappingV1 {
    pub schema: String,
    pub mapping_root_sha256: String,
    pub catalog_root_sha256: String,
    pub entries: Vec<K2HiddenActionEntryV1>,
}

impl K2HiddenActionMappingV1 {
    pub fn seal_fixture_v1(
        catalog: &K2OpaqueActionCatalogV1,
        copy_action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        catalog.validate()?;
        if !catalog.action_ids_sha256.contains(&copy_action_id_sha256) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_hidden_copy_action_missing",
            ));
        }
        let remove_action_id_sha256 = catalog
            .action_ids_sha256
            .iter()
            .find(|action_id| **action_id != copy_action_id_sha256)
            .cloned()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_hidden_remove_action_missing",
            ))?;
        let mut entries = vec![
            K2HiddenActionEntryV1 {
                action_id_sha256: copy_action_id_sha256,
                effect: K2LearnedEffectLawBodyV1::CopyFile {
                    source_path: K2_COPY_SOURCE_PATH_V1.to_owned(),
                    target_path: K2_COPY_TARGET_PATH_V1.to_owned(),
                },
                operation_plan_root_sha256: String::new(),
            },
            K2HiddenActionEntryV1 {
                action_id_sha256: remove_action_id_sha256,
                effect: K2LearnedEffectLawBodyV1::RemoveFile {
                    path: K2_REMOVE_PATH_V1.to_owned(),
                },
                operation_plan_root_sha256: String::new(),
            },
        ];
        for entry in &mut entries {
            entry.operation_plan_root_sha256 = learned_root_v1(&vec![entry.effect.operation_v1()])?;
        }
        entries.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
        let mut mapping = Self {
            schema: K2_HIDDEN_ACTION_MAPPING_SCHEMA_V1.to_owned(),
            mapping_root_sha256: String::new(),
            catalog_root_sha256: catalog.catalog_root_sha256.clone(),
            entries,
        };
        mapping.mapping_root_sha256 = mapping.expected_root_v1()?;
        mapping.validate(catalog)?;
        Ok(mapping)
    }

    pub fn validate(&self, catalog: &K2OpaqueActionCatalogV1) -> K2GoalEnvironmentResultV1<()> {
        catalog.validate()?;
        if self.schema != K2_HIDDEN_ACTION_MAPPING_SCHEMA_V1
            || self.catalog_root_sha256 != catalog.catalog_root_sha256
            || self.entries.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .entries
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
            || self
                .entries
                .iter()
                .map(|entry| entry.action_id_sha256.as_str())
                .collect::<Vec<_>>()
                != catalog
                    .action_ids_sha256
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            || self.mapping_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_hidden_action_mapping_invalid",
            ));
        }
        let mut kinds = BTreeSet::new();
        for entry in &self.entries {
            entry.effect.validate()?;
            require_learned_root_v1(
                &entry.operation_plan_root_sha256,
                "k2_hidden_operation_root_invalid",
            )?;
            if entry.operation_plan_root_sha256
                != learned_root_v1(&vec![entry.effect.operation_v1()])?
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_hidden_operation_binding_invalid",
                ));
            }
            kinds.insert(match &entry.effect {
                K2LearnedEffectLawBodyV1::CopyFile { .. } => K2LearnedEffectV1::CopyFile,
                K2LearnedEffectLawBodyV1::RemoveFile { .. } => K2LearnedEffectV1::RemoveFile,
            });
        }
        if kinds != BTreeSet::from([K2LearnedEffectV1::CopyFile, K2LearnedEffectV1::RemoveFile]) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_hidden_effect_diversity_invalid",
            ));
        }
        Ok(())
    }

    pub fn entry(&self, action_id_sha256: &str) -> Option<&K2HiddenActionEntryV1> {
        self.entries
            .iter()
            .find(|entry| entry.action_id_sha256 == action_id_sha256)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_HIDDEN_ACTION_MAPPING_SCHEMA_V1,
            self.catalog_root_sha256.as_str(),
            &self.entries,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportWorldV1 {
    pub schema: String,
    pub world_root_sha256: String,
    pub world_ordinal: u64,
    pub source_manifest: LawLabTreeManifestV1,
    pub fixture_provenance_root_sha256: String,
}

impl K2SupportWorldV1 {
    pub fn seal(
        world_ordinal: u64,
        source_manifest: LawLabTreeManifestV1,
        fixture_provenance_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        require_learned_root_v1(
            &fixture_provenance_root_sha256,
            "k2_support_fixture_provenance_invalid",
        )?;
        let mut world = Self {
            schema: K2_SUPPORT_WORLD_SCHEMA_V1.to_owned(),
            world_root_sha256: String::new(),
            world_ordinal,
            source_manifest,
            fixture_provenance_root_sha256,
        };
        world.world_root_sha256 = world.expected_root_v1()?;
        world.validate()?;
        Ok(world)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.source_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        require_learned_root_v1(
            &self.fixture_provenance_root_sha256,
            "k2_support_fixture_provenance_invalid",
        )?;
        validate_fixture_manifest_v1(&self.source_manifest)?;
        if self.schema != K2_SUPPORT_WORLD_SCHEMA_V1
            || self.world_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_world_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_WORLD_SCHEMA_V1,
            self.world_ordinal,
            &self.source_manifest,
            self.fixture_provenance_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportWorldSetV1 {
    pub schema: String,
    pub support_set_root_sha256: String,
    pub worlds: Vec<K2SupportWorldV1>,
}

impl K2SupportWorldSetV1 {
    pub fn seal(mut worlds: Vec<K2SupportWorldV1>) -> K2GoalEnvironmentResultV1<Self> {
        worlds.sort_by_key(|world| world.world_ordinal);
        let mut set = Self {
            schema: K2_SUPPORT_WORLD_SET_SCHEMA_V1.to_owned(),
            support_set_root_sha256: String::new(),
            worlds,
        };
        set.support_set_root_sha256 = set.expected_root_v1()?;
        set.validate()?;
        Ok(set)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        if self.schema != K2_SUPPORT_WORLD_SET_SCHEMA_V1
            || self.worlds.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1
            || self
                .worlds
                .iter()
                .enumerate()
                .any(|(ordinal, world)| world.world_ordinal != ordinal as u64)
            || self.support_set_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_world_set_invalid",
            ));
        }
        for world in &self.worlds {
            world.validate()?;
        }
        require_unique_roots_v1(
            self.worlds
                .iter()
                .map(|world| world.world_root_sha256.as_str()),
            "k2_support_world_roots_not_unique",
        )?;
        require_unique_roots_v1(
            self.worlds
                .iter()
                .map(|world| world.source_manifest.tree_root_sha256.as_str()),
            "k2_support_tree_roots_not_unique",
        )?;
        require_distinct_fixture_file_values_v1(&self.worlds, K2_COPY_SOURCE_PATH_V1)?;
        require_distinct_fixture_file_values_v1(&self.worlds, K2_REMOVE_PATH_V1)?;
        let topology_roots = self
            .worlds
            .iter()
            .map(|world| distractor_topology_root_v1(&world.source_manifest))
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        require_unique_roots_v1(
            topology_roots.iter().map(String::as_str),
            "k2_support_distractor_topology_not_unique",
        )?;
        Ok(())
    }

    pub fn world(&self, world_root_sha256: &str) -> Option<&K2SupportWorldV1> {
        self.worlds
            .iter()
            .find(|world| world.world_root_sha256 == world_root_sha256)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(K2_SUPPORT_WORLD_SET_SCHEMA_V1, &self.worlds))
    }
}

fn validate_fixture_manifest_v1(manifest: &LawLabTreeManifestV1) -> K2GoalEnvironmentResultV1<()> {
    if manifest.entries.len() > K2_LEARNED_MAX_TREE_ENTRIES_V1
        || manifest.total_file_bytes > K2_LEARNED_MAX_TREE_BYTES_V1
        || manifest.entry(K2_COPY_TARGET_PATH_V1).is_some()
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_manifest_budget_or_target_invalid",
        ));
    }
    let source = manifest
        .entry(K2_COPY_SOURCE_PATH_V1)
        .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_copy_source_missing",
        ))?;
    manifest
        .entry(K2_REMOVE_PATH_V1)
        .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_remove_path_missing",
        ))?;
    let duplicate_source = manifest.entries.iter().filter(|entry| {
        entry.kind == LawLabTreeEntryKindV1::File
            && entry.byte_length == source.byte_length
            && entry.content_sha256 == source.content_sha256
            && entry.executable == source.executable
    });
    if duplicate_source.count() != 1
        || manifest.entries.iter().all(|entry| {
            matches!(
                entry.relative_path.as_str(),
                K2_COPY_SOURCE_PATH_V1 | K2_REMOVE_PATH_V1
            )
        })
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_distractor_or_source_uniqueness_invalid",
        ));
    }
    Ok(())
}

fn require_distinct_fixture_file_values_v1(
    worlds: &[K2SupportWorldV1],
    path: &str,
) -> K2GoalEnvironmentResultV1<()> {
    let entries = worlds
        .iter()
        .map(|world| {
            world
                .source_manifest
                .entry(path)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_fixture_required_file_missing",
                ))
        })
        .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
    let hashes = entries
        .iter()
        .filter_map(|entry| entry.content_sha256.as_deref())
        .collect::<BTreeSet<_>>();
    let lengths = entries
        .iter()
        .map(|entry| entry.byte_length)
        .collect::<BTreeSet<_>>();
    if hashes.len() != worlds.len() || lengths.len() != worlds.len() {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_fixture_file_values_not_distinct",
        ));
    }
    Ok(())
}

fn distractor_topology_root_v1(
    manifest: &LawLabTreeManifestV1,
) -> K2GoalEnvironmentResultV1<String> {
    let topology = manifest
        .entries
        .iter()
        .filter(|entry| {
            !matches!(
                entry.relative_path.as_str(),
                K2_COPY_SOURCE_PATH_V1 | K2_COPY_TARGET_PATH_V1 | K2_REMOVE_PATH_V1
            )
        })
        .map(|entry| (&entry.relative_path, entry.kind, entry.executable))
        .collect::<Vec<_>>();
    learned_root_v1(&("nando.k2-distractor-topology.v1", topology))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportProbeV1 {
    pub probe_root_sha256: String,
    pub probe_ordinal: u64,
    pub support_world_root_sha256: String,
    pub action_id_sha256: String,
    pub deterministic_seed_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportProbePlanV1 {
    pub schema: String,
    pub plan_root_sha256: String,
    pub public_schedule_root_sha256: String,
    pub experiment_id_sha256: String,
    pub catalog_root_sha256: String,
    pub support_set_root_sha256: String,
    pub hidden_mapping_root_sha256: String,
    pub ordered_probes: Vec<K2SupportProbeV1>,
}

impl K2SupportProbePlanV1 {
    pub fn seal(
        experiment_id_sha256: String,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        mapping: &K2HiddenActionMappingV1,
        deterministic_seed_sha256: &str,
    ) -> K2GoalEnvironmentResultV1<Self> {
        catalog.validate()?;
        support.validate()?;
        mapping.validate(catalog)?;
        for root in [&experiment_id_sha256, deterministic_seed_sha256] {
            require_learned_root_v1(root, "k2_support_probe_plan_root_invalid")?;
        }
        let mut ordered_probes = Vec::with_capacity(K2_LEARNED_SUPPORT_PROBE_COUNT_V1);
        for world in &support.worlds {
            for action_id_sha256 in &catalog.action_ids_sha256 {
                let probe_ordinal = ordered_probes.len() as u64;
                let probe_seed = learned_root_v1(&(
                    K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
                    deterministic_seed_sha256,
                    world.world_root_sha256.as_str(),
                    action_id_sha256.as_str(),
                    probe_ordinal,
                ))?;
                let probe_root_sha256 = learned_root_v1(&(
                    K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
                    "probe",
                    experiment_id_sha256.as_str(),
                    probe_ordinal,
                    world.world_root_sha256.as_str(),
                    action_id_sha256.as_str(),
                    probe_seed.as_str(),
                ))?;
                ordered_probes.push(K2SupportProbeV1 {
                    probe_root_sha256,
                    probe_ordinal,
                    support_world_root_sha256: world.world_root_sha256.clone(),
                    action_id_sha256: action_id_sha256.clone(),
                    deterministic_seed_sha256: probe_seed,
                });
            }
        }
        let public_schedule_root_sha256 = learned_root_v1(&(
            K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
            "public-schedule",
            catalog.catalog_root_sha256.as_str(),
            support.support_set_root_sha256.as_str(),
            &ordered_probes,
        ))?;
        let mut plan = Self {
            schema: K2_SUPPORT_PROBE_PLAN_SCHEMA_V1.to_owned(),
            plan_root_sha256: String::new(),
            public_schedule_root_sha256,
            experiment_id_sha256,
            catalog_root_sha256: catalog.catalog_root_sha256.clone(),
            support_set_root_sha256: support.support_set_root_sha256.clone(),
            hidden_mapping_root_sha256: mapping.mapping_root_sha256.clone(),
            ordered_probes,
        };
        plan.plan_root_sha256 = plan.expected_root_v1()?;
        plan.validate(catalog, support, mapping)?;
        Ok(plan)
    }

    pub fn validate(
        &self,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        mapping: &K2HiddenActionMappingV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        catalog.validate()?;
        support.validate()?;
        mapping.validate(catalog)?;
        require_learned_root_v1(&self.experiment_id_sha256, "k2_experiment_id_invalid")?;
        if self.schema != K2_SUPPORT_PROBE_PLAN_SCHEMA_V1
            || self.catalog_root_sha256 != catalog.catalog_root_sha256
            || self.support_set_root_sha256 != support.support_set_root_sha256
            || self.hidden_mapping_root_sha256 != mapping.mapping_root_sha256
            || self.ordered_probes.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self
                .ordered_probes
                .iter()
                .enumerate()
                .any(|(ordinal, probe)| probe.probe_ordinal != ordinal as u64)
            || self.public_schedule_root_sha256
                != learned_root_v1(&(
                    K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
                    "public-schedule",
                    catalog.catalog_root_sha256.as_str(),
                    support.support_set_root_sha256.as_str(),
                    &self.ordered_probes,
                ))?
            || self.plan_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_probe_plan_invalid",
            ));
        }
        let expected_pairs = support
            .worlds
            .iter()
            .flat_map(|world| {
                catalog
                    .action_ids_sha256
                    .iter()
                    .map(move |action_id| (world.world_root_sha256.as_str(), action_id.as_str()))
            })
            .collect::<Vec<_>>();
        for (probe, expected) in self.ordered_probes.iter().zip(expected_pairs) {
            for root in [
                probe.probe_root_sha256.as_str(),
                probe.deterministic_seed_sha256.as_str(),
            ] {
                require_learned_root_v1(root, "k2_support_probe_root_invalid")?;
            }
            if (
                probe.support_world_root_sha256.as_str(),
                probe.action_id_sha256.as_str(),
            ) != expected
                || probe.probe_root_sha256
                    != learned_root_v1(&(
                        K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
                        "probe",
                        self.experiment_id_sha256.as_str(),
                        probe.probe_ordinal,
                        probe.support_world_root_sha256.as_str(),
                        probe.action_id_sha256.as_str(),
                        probe.deterministic_seed_sha256.as_str(),
                    ))?
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_probe_binding_invalid",
                ));
            }
        }
        Ok(())
    }

    pub fn probe(&self, probe_ordinal: u64) -> Option<&K2SupportProbeV1> {
        self.ordered_probes.get(probe_ordinal as usize)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_PROBE_PLAN_SCHEMA_V1,
            self.public_schedule_root_sha256.as_str(),
            self.experiment_id_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.hidden_mapping_root_sha256.as_str(),
            &self.ordered_probes,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnerPublicContextV1 {
    pub schema: String,
    pub public_context_root_sha256: String,
    pub public_experiment_id_sha256: String,
    pub catalog_root_sha256: String,
    pub support_set_root_sha256: String,
    pub support_probe_schedule_public_root_sha256: String,
    pub allowed_effect_language_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub learner_budget_root_sha256: String,
}

impl K2LearnerPublicContextV1 {
    pub fn seal(
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        plan: &K2SupportProbePlanV1,
        learner: &K2EffectLearnerManifestV1,
        budget: &K2LearnedCapabilityBudgetV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        catalog.validate()?;
        support.validate()?;
        learner.validate()?;
        budget.validate()?;
        if plan.catalog_root_sha256 != catalog.catalog_root_sha256
            || plan.support_set_root_sha256 != support.support_set_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_public_context_plan_binding_invalid",
            ));
        }
        let public_experiment_id_sha256 = learned_root_v1(&(
            K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1,
            "public-experiment",
            catalog.catalog_root_sha256.as_str(),
            support.support_set_root_sha256.as_str(),
            plan.public_schedule_root_sha256.as_str(),
            learner.manifest_root_sha256.as_str(),
        ))?;
        let mut context = Self {
            schema: K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1.to_owned(),
            public_context_root_sha256: String::new(),
            public_experiment_id_sha256,
            catalog_root_sha256: catalog.catalog_root_sha256.clone(),
            support_set_root_sha256: support.support_set_root_sha256.clone(),
            support_probe_schedule_public_root_sha256: plan.public_schedule_root_sha256.clone(),
            allowed_effect_language_root_sha256: bounded_effect_language_root_v1()?,
            learner_manifest_root_sha256: learner.manifest_root_sha256.clone(),
            learner_executable_sha256: learner.executable_sha256.clone(),
            learner_budget_root_sha256: budget.root_v1()?,
        };
        context.public_context_root_sha256 = context.expected_root_v1()?;
        context.validate(catalog, support, plan, learner, budget)?;
        Ok(context)
    }

    pub fn validate(
        &self,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        plan: &K2SupportProbePlanV1,
        learner: &K2EffectLearnerManifestV1,
        budget: &K2LearnedCapabilityBudgetV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        catalog.validate()?;
        support.validate()?;
        learner.validate()?;
        budget.validate()?;
        if self.schema != K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1
            || self.catalog_root_sha256 != catalog.catalog_root_sha256
            || self.support_set_root_sha256 != support.support_set_root_sha256
            || self.support_probe_schedule_public_root_sha256 != plan.public_schedule_root_sha256
            || self.allowed_effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.learner_manifest_root_sha256 != learner.manifest_root_sha256
            || self.learner_executable_sha256 != learner.executable_sha256
            || self.learner_budget_root_sha256 != budget.root_v1()?
            || self.public_context_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learner_public_context_invalid",
            ));
        }
        require_learned_root_v1(
            &self.public_experiment_id_sha256,
            "k2_public_experiment_id_invalid",
        )
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1,
            self.public_experiment_id_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.support_probe_schedule_public_root_sha256.as_str(),
            self.allowed_effect_language_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.learner_budget_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2PrivateExperimentContractV1 {
    pub schema: String,
    pub private_contract_root_sha256: String,
    pub experiment_id_sha256: String,
    pub harness_commitment_sha256: String,
    pub public_context_root_sha256: String,
    pub hidden_action_mapping: K2HiddenActionMappingV1,
    pub support_source_manifest_roots_sha256: Vec<String>,
    pub target_pre_manifest: LawLabTreeManifestV1,
    pub target_expected_goal_manifest: LawLabTreeManifestV1,
    pub target_goal_store_snapshot_root_sha256: String,
}

impl K2PrivateExperimentContractV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        experiment_id_sha256: String,
        harness_commitment_sha256: String,
        context: &K2LearnerPublicContextV1,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
        mapping: K2HiddenActionMappingV1,
        target_pre_manifest: LawLabTreeManifestV1,
        target_expected_goal_manifest: LawLabTreeManifestV1,
        target_goal_store_snapshot_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        for root in [
            experiment_id_sha256.as_str(),
            harness_commitment_sha256.as_str(),
            target_goal_store_snapshot_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_private_contract_root_invalid")?;
        }
        mapping.validate(catalog)?;
        support.validate()?;
        target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        target_expected_goal_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let mut contract = Self {
            schema: K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1.to_owned(),
            private_contract_root_sha256: String::new(),
            experiment_id_sha256,
            harness_commitment_sha256,
            public_context_root_sha256: context.public_context_root_sha256.clone(),
            hidden_action_mapping: mapping,
            support_source_manifest_roots_sha256: support
                .worlds
                .iter()
                .map(|world| world.source_manifest.tree_root_sha256.clone())
                .collect(),
            target_pre_manifest,
            target_expected_goal_manifest,
            target_goal_store_snapshot_root_sha256,
        };
        contract.private_contract_root_sha256 = contract.expected_root_v1()?;
        contract.validate(context, catalog, support)?;
        Ok(contract)
    }

    pub fn validate(
        &self,
        context: &K2LearnerPublicContextV1,
        catalog: &K2OpaqueActionCatalogV1,
        support: &K2SupportWorldSetV1,
    ) -> K2GoalEnvironmentResultV1<()> {
        self.hidden_action_mapping.validate(catalog)?;
        support.validate()?;
        self.target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        self.target_expected_goal_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        for root in [
            self.experiment_id_sha256.as_str(),
            self.harness_commitment_sha256.as_str(),
            self.target_goal_store_snapshot_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_private_contract_root_invalid")?;
        }
        let expected_support_roots = support
            .worlds
            .iter()
            .map(|world| world.source_manifest.tree_root_sha256.as_str())
            .collect::<Vec<_>>();
        if self.schema != K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1
            || self.public_context_root_sha256 != context.public_context_root_sha256
            || self.hidden_action_mapping.catalog_root_sha256 != catalog.catalog_root_sha256
            || self
                .support_source_manifest_roots_sha256
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_support_roots
            || self.private_contract_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_private_experiment_contract_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    pub fn artifact_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(self)
    }

    pub fn target_holdout_commitment_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1,
            "target-holdout",
            self.target_pre_manifest.tree_root_sha256.as_str(),
            self.target_expected_goal_manifest.tree_root_sha256.as_str(),
            self.target_goal_store_snapshot_root_sha256.as_str(),
        ))
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_PRIVATE_EXPERIMENT_CONTRACT_SCHEMA_V1,
            self.experiment_id_sha256.as_str(),
            self.harness_commitment_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            &self.hidden_action_mapping,
            &self.support_source_manifest_roots_sha256,
            &self.target_pre_manifest,
            &self.target_expected_goal_manifest,
            self.target_goal_store_snapshot_root_sha256.as_str(),
        ))
    }
}

pub struct K2LearnedCapabilityFreezeInputV1<'a> {
    pub private_contract: &'a K2PrivateExperimentContractV1,
    pub public_context: &'a K2LearnerPublicContextV1,
    pub catalog: &'a K2OpaqueActionCatalogV1,
    pub support: &'a K2SupportWorldSetV1,
    pub plan: &'a K2SupportProbePlanV1,
    pub learner: &'a K2EffectLearnerManifestV1,
    pub budget: &'a K2LearnedCapabilityBudgetV1,
    pub independent_verifier_contract_root_sha256: String,
    pub selector_executable_sha256: String,
    pub sandbox_executor_manifest_root_sha256: String,
    pub sandbox_worker_sha256: String,
    pub exact_oracle_manifest_root_sha256: String,
    pub exact_oracle_executable_sha256: String,
    pub deterministic_seed_sha256: String,
    pub frozen_at_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilityFreezeV1 {
    pub schema: String,
    pub freeze_root_sha256: String,
    pub experiment_id_sha256: String,
    pub public_context_root_sha256: String,
    pub private_contract_artifact_root_sha256: String,
    pub catalog_root_sha256: String,
    pub support_set_root_sha256: String,
    pub support_probe_plan_root_sha256: String,
    pub hidden_mapping_root_sha256: String,
    pub target_holdout_commitment_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub independent_verifier_contract_root_sha256: String,
    pub selector_executable_sha256: String,
    pub sandbox_executor_manifest_root_sha256: String,
    pub sandbox_worker_sha256: String,
    pub exact_oracle_manifest_root_sha256: String,
    pub exact_oracle_executable_sha256: String,
    pub budget_root_sha256: String,
    pub deterministic_seed_sha256: String,
    pub frozen_at_unix_ms: u64,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedCapabilityFreezeV1 {
    pub fn seal(input: K2LearnedCapabilityFreezeInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input
            .private_contract
            .validate(input.public_context, input.catalog, input.support)?;
        input.plan.validate(
            input.catalog,
            input.support,
            &input.private_contract.hidden_action_mapping,
        )?;
        input.learner.validate()?;
        input.budget.validate()?;
        let executable_roots = [
            input.learner.executable_sha256.as_str(),
            input.selector_executable_sha256.as_str(),
            input.sandbox_worker_sha256.as_str(),
            input.exact_oracle_executable_sha256.as_str(),
        ];
        require_unique_roots_v1(
            executable_roots,
            "k2_learned_executable_identities_not_distinct",
        )?;
        for root in [
            input.independent_verifier_contract_root_sha256.as_str(),
            input.sandbox_executor_manifest_root_sha256.as_str(),
            input.exact_oracle_manifest_root_sha256.as_str(),
            input.deterministic_seed_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_freeze_root_invalid")?;
        }
        let mut freeze = Self {
            schema: K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1.to_owned(),
            freeze_root_sha256: String::new(),
            experiment_id_sha256: input.private_contract.experiment_id_sha256.clone(),
            public_context_root_sha256: input.public_context.public_context_root_sha256.clone(),
            private_contract_artifact_root_sha256: input.private_contract.artifact_root_v1()?,
            catalog_root_sha256: input.catalog.catalog_root_sha256.clone(),
            support_set_root_sha256: input.support.support_set_root_sha256.clone(),
            support_probe_plan_root_sha256: input.plan.plan_root_sha256.clone(),
            hidden_mapping_root_sha256: input
                .private_contract
                .hidden_action_mapping
                .mapping_root_sha256
                .clone(),
            target_holdout_commitment_root_sha256: input
                .private_contract
                .target_holdout_commitment_root_v1()?,
            learner_manifest_root_sha256: input.learner.manifest_root_sha256.clone(),
            learner_executable_sha256: input.learner.executable_sha256.clone(),
            independent_verifier_contract_root_sha256: input
                .independent_verifier_contract_root_sha256,
            selector_executable_sha256: input.selector_executable_sha256,
            sandbox_executor_manifest_root_sha256: input.sandbox_executor_manifest_root_sha256,
            sandbox_worker_sha256: input.sandbox_worker_sha256,
            exact_oracle_manifest_root_sha256: input.exact_oracle_manifest_root_sha256,
            exact_oracle_executable_sha256: input.exact_oracle_executable_sha256,
            budget_root_sha256: input.budget.root_v1()?,
            deterministic_seed_sha256: input.deterministic_seed_sha256,
            frozen_at_unix_ms: input.frozen_at_unix_ms,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        freeze.freeze_root_sha256 = freeze.expected_root_v1()?;
        freeze.validate_persisted_v1()?;
        Ok(freeze)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.freeze_root_sha256.as_str(),
            self.experiment_id_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.private_contract_artifact_root_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.support_probe_plan_root_sha256.as_str(),
            self.hidden_mapping_root_sha256.as_str(),
            self.target_holdout_commitment_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.independent_verifier_contract_root_sha256.as_str(),
            self.sandbox_executor_manifest_root_sha256.as_str(),
            self.exact_oracle_manifest_root_sha256.as_str(),
            self.budget_root_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_freeze_root_invalid")?;
        }
        require_unique_roots_v1(
            [
                self.learner_executable_sha256.as_str(),
                self.selector_executable_sha256.as_str(),
                self.sandbox_worker_sha256.as_str(),
                self.exact_oracle_executable_sha256.as_str(),
            ],
            "k2_learned_executable_identities_not_distinct",
        )?;
        if self.schema != K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1
            || self.freeze_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_capability_freeze_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1,
            (
                self.experiment_id_sha256.as_str(),
                self.public_context_root_sha256.as_str(),
                self.private_contract_artifact_root_sha256.as_str(),
                self.catalog_root_sha256.as_str(),
                self.support_set_root_sha256.as_str(),
                self.support_probe_plan_root_sha256.as_str(),
                self.hidden_mapping_root_sha256.as_str(),
                self.target_holdout_commitment_root_sha256.as_str(),
                self.learner_manifest_root_sha256.as_str(),
                self.learner_executable_sha256.as_str(),
            ),
            (
                self.independent_verifier_contract_root_sha256.as_str(),
                self.selector_executable_sha256.as_str(),
                self.sandbox_executor_manifest_root_sha256.as_str(),
                self.sandbox_worker_sha256.as_str(),
                self.exact_oracle_manifest_root_sha256.as_str(),
                self.exact_oracle_executable_sha256.as_str(),
                self.budget_root_sha256.as_str(),
                self.deterministic_seed_sha256.as_str(),
                self.frozen_at_unix_ms,
                &self.authority,
            ),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportDispatchV1 {
    pub schema: String,
    pub dispatch_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub probe_ordinal: u64,
    pub probe_root_sha256: String,
    pub support_world_root_sha256: String,
    pub source_tree_root_sha256: String,
    pub action_id_sha256: String,
    pub hidden_operation_plan_root_sha256: String,
    pub request_root_sha256: String,
    pub worker_sha256: String,
    pub executor_manifest_root_sha256: String,
    pub deterministic_seed_sha256: String,
}

impl K2SupportDispatchV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        freeze: &K2LearnedCapabilityFreezeV1,
        plan: &K2SupportProbePlanV1,
        probe: &K2SupportProbeV1,
        world: &K2SupportWorldV1,
        mapping: &K2HiddenActionMappingV1,
        request: &LawLabSandboxRequestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        freeze.validate_persisted_v1()?;
        world.validate()?;
        request
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let entry =
            mapping
                .entry(&probe.action_id_sha256)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_dispatch_action_missing",
                ))?;
        if plan.plan_root_sha256 != freeze.support_probe_plan_root_sha256
            || probe.probe_ordinal >= K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || plan.probe(probe.probe_ordinal) != Some(probe)
            || probe.support_world_root_sha256 != world.world_root_sha256
            || request.source_tree_root_sha256 != world.source_manifest.tree_root_sha256
            || request.operations != [entry.effect.operation_v1()]
            || request.purpose != LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest
            || request.domain != LawLabProbeDomainV1::Filesystem
            || request.executor_manifest_root_sha256 != freeze.sandbox_executor_manifest_root_sha256
            || request.worker_sha256 != freeze.sandbox_worker_sha256
            || request.probe_root_sha256 != probe.probe_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_dispatch_binding_invalid",
            ));
        }
        let mut dispatch = Self {
            schema: K2_SUPPORT_DISPATCH_SCHEMA_V1.to_owned(),
            dispatch_root_sha256: String::new(),
            experiment_freeze_root_sha256: freeze.freeze_root_sha256.clone(),
            probe_ordinal: probe.probe_ordinal,
            probe_root_sha256: probe.probe_root_sha256.clone(),
            support_world_root_sha256: world.world_root_sha256.clone(),
            source_tree_root_sha256: world.source_manifest.tree_root_sha256.clone(),
            action_id_sha256: probe.action_id_sha256.clone(),
            hidden_operation_plan_root_sha256: entry.operation_plan_root_sha256.clone(),
            request_root_sha256: request.request_root_sha256.clone(),
            worker_sha256: request.worker_sha256.clone(),
            executor_manifest_root_sha256: request.executor_manifest_root_sha256.clone(),
            deterministic_seed_sha256: probe.deterministic_seed_sha256.clone(),
        };
        dispatch.dispatch_root_sha256 = dispatch.expected_root_v1()?;
        dispatch.validate_persisted_v1()?;
        Ok(dispatch)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.dispatch_root_sha256.as_str(),
            self.experiment_freeze_root_sha256.as_str(),
            self.probe_root_sha256.as_str(),
            self.support_world_root_sha256.as_str(),
            self.source_tree_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            self.hidden_operation_plan_root_sha256.as_str(),
            self.request_root_sha256.as_str(),
            self.worker_sha256.as_str(),
            self.executor_manifest_root_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_support_dispatch_root_invalid")?;
        }
        if self.schema != K2_SUPPORT_DISPATCH_SCHEMA_V1
            || self.probe_ordinal >= K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || self.dispatch_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_dispatch_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_DISPATCH_SCHEMA_V1,
            self.experiment_freeze_root_sha256.as_str(),
            self.probe_ordinal,
            self.probe_root_sha256.as_str(),
            self.support_world_root_sha256.as_str(),
            self.source_tree_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            self.hidden_operation_plan_root_sha256.as_str(),
            self.request_root_sha256.as_str(),
            self.worker_sha256.as_str(),
            self.executor_manifest_root_sha256.as_str(),
            self.deterministic_seed_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportObservationV1 {
    pub schema: String,
    pub observation_root_sha256: String,
    pub public_context_root_sha256: String,
    pub dispatch_root_sha256: String,
    pub probe_ordinal: u64,
    pub support_world_root_sha256: String,
    pub action_id_sha256: String,
    pub source_manifest_root_sha256: String,
    pub pre_work_manifest: LawLabTreeManifestV1,
    pub post_work_manifest: LawLabTreeManifestV1,
    pub sandbox_receipt_root_sha256: String,
}

impl K2SupportObservationV1 {
    pub fn seal(
        public_context: &K2LearnerPublicContextV1,
        world: &K2SupportWorldV1,
        dispatch: &K2SupportDispatchV1,
        request: &LawLabSandboxRequestV1,
        execution: &LawLabSandboxExecutionV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        world.validate()?;
        dispatch.validate_persisted_v1()?;
        execution
            .receipt
            .validate(request, &execution.worker_outcome)
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let worker = &execution.worker_outcome;
        if dispatch.request_root_sha256 != request.request_root_sha256
            || dispatch.support_world_root_sha256 != world.world_root_sha256
            || dispatch.source_tree_root_sha256 != world.source_manifest.tree_root_sha256
            || worker.source_manifest != world.source_manifest
            || worker.pre_work_manifest != world.source_manifest
            || worker.source_manifest.tree_root_sha256 != dispatch.source_tree_root_sha256
            || worker.request_root_sha256 != dispatch.request_root_sha256
            || execution.receipt.request_root_sha256 != dispatch.request_root_sha256
            || execution.receipt.source_tree_root_sha256 != dispatch.source_tree_root_sha256
            || execution.receipt.post_tree_root_sha256 != worker.post_work_manifest.tree_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_evidence_invalid",
            ));
        }
        let mut observation = Self {
            schema: K2_SUPPORT_OBSERVATION_SCHEMA_V1.to_owned(),
            observation_root_sha256: String::new(),
            public_context_root_sha256: public_context.public_context_root_sha256.clone(),
            dispatch_root_sha256: dispatch.dispatch_root_sha256.clone(),
            probe_ordinal: dispatch.probe_ordinal,
            support_world_root_sha256: world.world_root_sha256.clone(),
            action_id_sha256: dispatch.action_id_sha256.clone(),
            source_manifest_root_sha256: world.source_manifest.tree_root_sha256.clone(),
            pre_work_manifest: worker.pre_work_manifest.clone(),
            post_work_manifest: worker.post_work_manifest.clone(),
            sandbox_receipt_root_sha256: execution.receipt.receipt_root_sha256.clone(),
        };
        observation.observation_root_sha256 = observation.expected_root_v1()?;
        observation.validate_persisted_v1()?;
        Ok(observation)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.pre_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        self.post_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        for root in [
            self.observation_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.dispatch_root_sha256.as_str(),
            self.support_world_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            self.source_manifest_root_sha256.as_str(),
            self.sandbox_receipt_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_support_observation_root_invalid")?;
        }
        if self.schema != K2_SUPPORT_OBSERVATION_SCHEMA_V1
            || self.probe_ordinal >= K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || self.source_manifest_root_sha256 != self.pre_work_manifest.tree_root_sha256
            || self.observation_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_observation_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_OBSERVATION_SCHEMA_V1,
            self.public_context_root_sha256.as_str(),
            self.dispatch_root_sha256.as_str(),
            self.probe_ordinal,
            self.support_world_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            self.source_manifest_root_sha256.as_str(),
            &self.pre_work_manifest,
            &self.post_work_manifest,
            self.sandbox_receipt_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2SupportObservationSetV1 {
    pub schema: String,
    pub observation_set_root_sha256: String,
    pub public_context_root_sha256: String,
    pub observations: Vec<K2SupportObservationV1>,
}

impl K2SupportObservationSetV1 {
    pub fn seal(
        public_context: &K2LearnerPublicContextV1,
        plan: &K2SupportProbePlanV1,
        mut observations: Vec<K2SupportObservationV1>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        observations.sort_by_key(|observation| observation.probe_ordinal);
        if observations.len() != plan.ordered_probes.len() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_observation_count_invalid",
            ));
        }
        for (observation, probe) in observations.iter().zip(&plan.ordered_probes) {
            observation.validate_persisted_v1()?;
            if observation.public_context_root_sha256 != public_context.public_context_root_sha256
                || observation.probe_ordinal != probe.probe_ordinal
                || observation.support_world_root_sha256 != probe.support_world_root_sha256
                || observation.action_id_sha256 != probe.action_id_sha256
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_observation_schedule_invalid",
                ));
            }
        }
        let mut set = Self {
            schema: K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1.to_owned(),
            observation_set_root_sha256: String::new(),
            public_context_root_sha256: public_context.public_context_root_sha256.clone(),
            observations,
        };
        set.observation_set_root_sha256 = set.expected_root_v1()?;
        set.validate_persisted_v1()?;
        Ok(set)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        if self.schema != K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1
            || self.observations.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self
                .observations
                .iter()
                .enumerate()
                .any(|(ordinal, observation)| observation.probe_ordinal != ordinal as u64)
            || self.observations.iter().any(|observation| {
                observation.public_context_root_sha256 != self.public_context_root_sha256
            })
            || self.observation_set_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_observation_set_invalid",
            ));
        }
        for observation in &self.observations {
            observation.validate_persisted_v1()?;
        }
        require_unique_roots_v1(
            self.observations
                .iter()
                .map(|observation| observation.observation_root_sha256.as_str()),
            "k2_support_observation_roots_not_unique",
        )?;
        require_unique_roots_v1(
            self.observations
                .iter()
                .map(|observation| observation.sandbox_receipt_root_sha256.as_str()),
            "k2_support_receipt_roots_not_unique",
        )?;
        let per_action = self.observations.iter().fold(
            BTreeMap::<&str, BTreeSet<&str>>::new(),
            |mut groups, observation| {
                groups
                    .entry(&observation.action_id_sha256)
                    .or_default()
                    .insert(&observation.support_world_root_sha256);
                groups
            },
        );
        if per_action.len() != K2_LEARNED_ACTION_COUNT_V1
            || per_action
                .values()
                .any(|worlds| worlds.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1)
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_observation_denominator_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1,
            self.public_context_root_sha256.as_str(),
            &self.observations,
        ))
    }
}

impl K2LearnerPublicContextV1 {
    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.public_context_root_sha256.as_str(),
            self.public_experiment_id_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.support_probe_schedule_public_root_sha256.as_str(),
            self.allowed_effect_language_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.learner_budget_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_public_context_root_invalid")?;
        }
        if self.schema != K2_LEARNER_PUBLIC_CONTEXT_SCHEMA_V1
            || self.allowed_effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.public_context_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_public_context_persisted_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2EffectLearningRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub public_context: K2LearnerPublicContextV1,
    pub catalog: K2OpaqueActionCatalogV1,
    pub support_observations: K2SupportObservationSetV1,
    pub minimum_support_worlds_per_action: u64,
    pub allowed_effect_language_root_sha256: String,
}

impl K2EffectLearningRequestV1 {
    pub fn seal(
        public_context: K2LearnerPublicContextV1,
        catalog: K2OpaqueActionCatalogV1,
        support_observations: K2SupportObservationSetV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        public_context.validate_persisted_v1()?;
        catalog.validate()?;
        support_observations.validate_persisted_v1()?;
        let mut request = Self {
            schema: K2_EFFECT_LEARNING_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            public_context,
            catalog,
            support_observations,
            minimum_support_worlds_per_action: K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64,
            allowed_effect_language_root_sha256: bounded_effect_language_root_v1()?,
        };
        request.request_root_sha256 = request.expected_root_v1()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.public_context.validate_persisted_v1()?;
        self.catalog.validate()?;
        self.support_observations.validate_persisted_v1()?;
        if self.schema != K2_EFFECT_LEARNING_REQUEST_SCHEMA_V1
            || self.public_context.catalog_root_sha256 != self.catalog.catalog_root_sha256
            || self.public_context.public_context_root_sha256
                != self.support_observations.public_context_root_sha256
            || self.minimum_support_worlds_per_action != K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64
            || self.allowed_effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.public_context.allowed_effect_language_root_sha256
                != self.allowed_effect_language_root_sha256
            || self.request_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learning_request_invalid",
            ));
        }
        if self.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learning_request_budget_exhausted",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let request = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_REQUEST_BYTES_V1,
            "k2_effect_learning_request_protocol_invalid",
        )?;
        request.validate()?;
        Ok(request)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_EFFECT_LEARNING_REQUEST_SCHEMA_V1,
            &self.public_context,
            &self.catalog,
            &self.support_observations,
            self.minimum_support_worlds_per_action,
            self.allowed_effect_language_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedEffectLawV1 {
    pub schema: String,
    pub law_root_sha256: String,
    pub action_id_sha256: String,
    pub effect: K2LearnedEffectLawBodyV1,
    pub supporting_world_roots_sha256: Vec<String>,
    pub supporting_observation_roots_sha256: Vec<String>,
    pub enumerated_candidate_count: u64,
    pub enumerated_candidate_roots_sha256: Vec<String>,
    pub rejected_candidate_count: u64,
    pub rejection_counts_by_reason: BTreeMap<String, u64>,
    pub version_space_size: u64,
}

impl K2LearnedEffectLawV1 {
    fn seal(
        action_id_sha256: String,
        effect: K2LearnedEffectLawBodyV1,
        observations: &[&K2SupportObservationV1],
        mut enumerated_candidates: Vec<K2LearnedEffectLawBodyV1>,
        rejected_candidate_count: u64,
        rejection_counts_by_reason: BTreeMap<String, u64>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        effect.validate()?;
        enumerated_candidates.sort();
        enumerated_candidates.dedup();
        let mut enumerated_candidate_roots_sha256 = enumerated_candidates
            .iter()
            .map(|candidate| {
                learned_root_v1(&(
                    K2_LEARNED_EFFECT_LAW_SCHEMA_V1,
                    "candidate",
                    action_id_sha256.as_str(),
                    candidate,
                ))
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        enumerated_candidate_roots_sha256.sort();
        let mut supporting_world_roots_sha256 = observations
            .iter()
            .map(|observation| observation.support_world_root_sha256.clone())
            .collect::<Vec<_>>();
        let mut supporting_observation_roots_sha256 = observations
            .iter()
            .map(|observation| observation.observation_root_sha256.clone())
            .collect::<Vec<_>>();
        supporting_world_roots_sha256.sort();
        supporting_observation_roots_sha256.sort();
        let mut law = Self {
            schema: K2_LEARNED_EFFECT_LAW_SCHEMA_V1.to_owned(),
            law_root_sha256: String::new(),
            action_id_sha256,
            effect,
            supporting_world_roots_sha256,
            supporting_observation_roots_sha256,
            enumerated_candidate_count: enumerated_candidate_roots_sha256.len() as u64,
            enumerated_candidate_roots_sha256,
            rejected_candidate_count,
            rejection_counts_by_reason,
            version_space_size: 1,
        };
        law.law_root_sha256 = law.expected_root_v1()?;
        law.validate()?;
        Ok(law)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.effect.validate()?;
        require_learned_root_v1(&self.action_id_sha256, "k2_learned_law_action_invalid")?;
        require_unique_roots_v1(
            self.supporting_world_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_law_worlds_invalid",
        )?;
        require_unique_roots_v1(
            self.supporting_observation_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_law_observations_invalid",
        )?;
        require_unique_roots_v1(
            self.enumerated_candidate_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_candidate_roots_invalid",
        )?;
        let rejected_total = self
            .rejection_counts_by_reason
            .values()
            .try_fold(0_u64, |total, count| total.checked_add(*count))
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_rejection_count_overflow",
            ))?;
        if self.schema != K2_LEARNED_EFFECT_LAW_SCHEMA_V1
            || self.supporting_world_roots_sha256.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1
            || self.supporting_observation_roots_sha256.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1
            || self.enumerated_candidate_count
                != self.enumerated_candidate_roots_sha256.len() as u64
            || self.enumerated_candidate_count == 0
            || self.enumerated_candidate_count > K2_LEARNED_MAX_CANDIDATES_PER_ACTION_V1 as u64
            || self.rejected_candidate_count != rejected_total
            || self
                .rejected_candidate_count
                .checked_add(self.version_space_size)
                != Some(self.enumerated_candidate_count)
            || self.version_space_size != 1
            || self.law_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_effect_law_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_EFFECT_LAW_SCHEMA_V1,
            self.action_id_sha256.as_str(),
            &self.effect,
            &self.supporting_world_roots_sha256,
            &self.supporting_observation_roots_sha256,
            self.enumerated_candidate_count,
            &self.enumerated_candidate_roots_sha256,
            self.rejected_candidate_count,
            &self.rejection_counts_by_reason,
            self.version_space_size,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedEffectLawSetV1 {
    pub schema: String,
    pub law_set_root_sha256: String,
    pub learning_request_root_sha256: String,
    pub public_context_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub support_observation_set_root_sha256: String,
    pub allowed_effect_language_root_sha256: String,
    pub laws: Vec<K2LearnedEffectLawV1>,
    pub learned: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedEffectLawSetV1 {
    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.learning_request_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.support_observation_set_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_law_set_root_invalid")?;
        }
        if self.schema != K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1
            || self.allowed_effect_language_root_sha256 != bounded_effect_language_root_v1()?
            || self.laws.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .laws
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
            || !self.learned
            || self.law_set_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_effect_law_set_invalid",
            ));
        }
        for law in &self.laws {
            law.validate()?;
        }
        Ok(())
    }

    pub fn law(&self, action_id_sha256: &str) -> Option<&K2LearnedEffectLawV1> {
        self.laws
            .iter()
            .find(|law| law.action_id_sha256 == action_id_sha256)
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let set = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_OUTCOME_BYTES_V1,
            "k2_learned_law_set_protocol_invalid",
        )?;
        set.validate()?;
        Ok(set)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1,
            self.learning_request_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.support_observation_set_root_sha256.as_str(),
            self.allowed_effect_language_root_sha256.as_str(),
            &self.laws,
            self.learned,
            &self.authority,
        ))
    }
}

pub fn learn_effects_v1(
    request: &K2EffectLearningRequestV1,
) -> K2GoalEnvironmentResultV1<K2LearnedEffectLawSetV1> {
    request.validate()?;
    let observation_views = request
        .support_observations
        .observations
        .iter()
        .map(|observation| K2EffectObservationViewV1 {
            action_id_sha256: &observation.action_id_sha256,
            pre_work_manifest: &observation.pre_work_manifest,
            post_work_manifest: &observation.post_work_manifest,
        })
        .collect::<Vec<_>>();
    let inferred = infer_effects_v1(&request.catalog.action_ids_sha256, &observation_views)?;
    let mut laws = Vec::with_capacity(K2_LEARNED_ACTION_COUNT_V1);
    for inference in inferred {
        let observations = request
            .support_observations
            .observations
            .iter()
            .filter(|observation| observation.action_id_sha256 == inference.action_id_sha256)
            .collect::<Vec<_>>();
        laws.push(K2LearnedEffectLawV1::seal(
            inference.action_id_sha256,
            inference.effect,
            &observations,
            inference.enumerated_candidates,
            inference.rejected_candidate_count,
            inference.rejection_counts_by_reason,
        )?);
    }
    laws.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
    let mut set = K2LearnedEffectLawSetV1 {
        schema: K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1.to_owned(),
        law_set_root_sha256: String::new(),
        learning_request_root_sha256: request.request_root_sha256.clone(),
        public_context_root_sha256: request.public_context.public_context_root_sha256.clone(),
        learner_manifest_root_sha256: request.public_context.learner_manifest_root_sha256.clone(),
        learner_executable_sha256: request.public_context.learner_executable_sha256.clone(),
        support_observation_set_root_sha256: request
            .support_observations
            .observation_set_root_sha256
            .clone(),
        allowed_effect_language_root_sha256: request.allowed_effect_language_root_sha256.clone(),
        laws,
        learned: true,
        authority: K2AuthorityBoundaryV1::authority_free_v1(),
    };
    set.law_set_root_sha256 = set.expected_root_v1()?;
    set.validate()?;
    if set.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_OUTCOME_BYTES_V1 {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_learned_law_set_budget_exhausted",
        ));
    }
    Ok(set)
}

struct K2EffectObservationViewV1<'a> {
    action_id_sha256: &'a str,
    pre_work_manifest: &'a LawLabTreeManifestV1,
    post_work_manifest: &'a LawLabTreeManifestV1,
}

struct K2InferredEffectV1 {
    action_id_sha256: String,
    effect: K2LearnedEffectLawBodyV1,
    enumerated_candidates: Vec<K2LearnedEffectLawBodyV1>,
    rejected_candidate_count: u64,
    rejection_counts_by_reason: BTreeMap<String, u64>,
}

fn infer_effects_v1(
    action_ids_sha256: &[String],
    observations: &[K2EffectObservationViewV1<'_>],
) -> K2GoalEnvironmentResultV1<Vec<K2InferredEffectV1>> {
    let mut inferred = Vec::with_capacity(action_ids_sha256.len());
    for action_id_sha256 in action_ids_sha256 {
        let matching = observations
            .iter()
            .filter(|observation| observation.action_id_sha256 == action_id_sha256)
            .collect::<Vec<_>>();
        if matching.len() != K2_LEARNED_SUPPORT_WORLD_COUNT_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_insufficient_support"));
        }
        let candidates = enumerate_effect_candidates_from_manifests_v1(
            matching[0].pre_work_manifest,
            matching[0].post_work_manifest,
        )?;
        if candidates.is_empty() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_non_transferable_delta",
            ));
        }
        if candidates.len() > K2_LEARNED_MAX_CANDIDATES_PER_ACTION_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_version_space_budget_exhausted",
            ));
        }
        let surviving = candidates
            .iter()
            .filter(|candidate| {
                matching.iter().all(|observation| {
                    apply_learned_effect_v1(observation.pre_work_manifest, candidate)
                        .is_ok_and(|predicted| predicted == *observation.post_work_manifest)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if surviving.is_empty() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_non_transferable_delta",
            ));
        }
        if surviving.len() > 1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ambiguous_source_match",
            ));
        }
        validate_effect_variation_views_v1(&surviving[0], &matching)?;
        let rejected_candidate_count = (candidates.len() - surviving.len()) as u64;
        let rejection_counts_by_reason = if rejected_candidate_count == 0 {
            BTreeMap::new()
        } else {
            BTreeMap::from([("support_mismatch".to_owned(), rejected_candidate_count)])
        };
        inferred.push(K2InferredEffectV1 {
            action_id_sha256: action_id_sha256.clone(),
            effect: surviving[0].clone(),
            enumerated_candidates: candidates,
            rejected_candidate_count,
            rejection_counts_by_reason,
        });
    }
    Ok(inferred)
}

fn enumerate_effect_candidates_from_manifests_v1(
    pre_work_manifest: &LawLabTreeManifestV1,
    post_work_manifest: &LawLabTreeManifestV1,
) -> K2GoalEnvironmentResultV1<Vec<K2LearnedEffectLawBodyV1>> {
    pre_work_manifest
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    post_work_manifest
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    let (added, removed, changed) = manifest_delta_v1(pre_work_manifest, post_work_manifest);
    let mut candidates = Vec::new();
    if added.len() == 1
        && removed.is_empty()
        && changed.is_empty()
        && added[0].kind == LawLabTreeEntryKindV1::File
    {
        let added_entry = added[0];
        for source in pre_work_manifest.entries.iter().filter(|entry| {
            entry.kind == LawLabTreeEntryKindV1::File
                && entry.byte_length == added_entry.byte_length
                && entry.content_sha256 == added_entry.content_sha256
                && entry.executable == added_entry.executable
                && entry.relative_path != added_entry.relative_path
        }) {
            candidates.push(K2LearnedEffectLawBodyV1::CopyFile {
                source_path: source.relative_path.clone(),
                target_path: added_entry.relative_path.clone(),
            });
        }
    }
    if removed.len() == 1
        && added.is_empty()
        && changed.is_empty()
        && removed[0].kind == LawLabTreeEntryKindV1::File
    {
        candidates.push(K2LearnedEffectLawBodyV1::RemoveFile {
            path: removed[0].relative_path.clone(),
        });
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn manifest_delta_v1<'a>(
    pre: &'a LawLabTreeManifestV1,
    post: &'a LawLabTreeManifestV1,
) -> (
    Vec<&'a LawLabTreeEntryV1>,
    Vec<&'a LawLabTreeEntryV1>,
    Vec<(&'a LawLabTreeEntryV1, &'a LawLabTreeEntryV1)>,
) {
    let pre_by_path = pre
        .entries
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let post_by_path = post
        .entries
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let added = post_by_path
        .iter()
        .filter_map(|(path, entry)| (!pre_by_path.contains_key(path)).then_some(*entry))
        .collect();
    let removed = pre_by_path
        .iter()
        .filter_map(|(path, entry)| (!post_by_path.contains_key(path)).then_some(*entry))
        .collect();
    let changed = pre_by_path
        .iter()
        .filter_map(|(path, pre_entry)| {
            post_by_path
                .get(path)
                .filter(|post_entry| **post_entry != *pre_entry)
                .map(|post_entry| (*pre_entry, *post_entry))
        })
        .collect();
    (added, removed, changed)
}

fn validate_effect_variation_views_v1(
    effect: &K2LearnedEffectLawBodyV1,
    observations: &[&K2EffectObservationViewV1<'_>],
) -> K2GoalEnvironmentResultV1<()> {
    let path = match effect {
        K2LearnedEffectLawBodyV1::CopyFile { source_path, .. } => source_path,
        K2LearnedEffectLawBodyV1::RemoveFile { path } => path,
    };
    let entries = observations
        .iter()
        .map(|observation| {
            observation
                .pre_work_manifest
                .entry(path)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_effect_variation_source_missing",
                ))
        })
        .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
    let hashes = entries
        .iter()
        .filter_map(|entry| entry.content_sha256.as_deref())
        .collect::<BTreeSet<_>>();
    let lengths = entries
        .iter()
        .map(|entry| entry.byte_length)
        .collect::<BTreeSet<_>>();
    if hashes.len() != observations.len() || lengths.len() != observations.len() {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_values_not_transferable",
        ));
    }
    Ok(())
}

fn apply_learned_effect_v1(
    pre: &LawLabTreeManifestV1,
    effect: &K2LearnedEffectLawBodyV1,
) -> K2GoalEnvironmentResultV1<LawLabTreeManifestV1> {
    pre.validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    effect.validate()?;
    let mut entries = pre.entries.clone();
    match effect {
        K2LearnedEffectLawBodyV1::CopyFile {
            source_path,
            target_path,
        } => {
            if entries
                .iter()
                .any(|entry| entry.relative_path == *target_path)
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_prediction_target_already_exists",
                ));
            }
            let source = entries
                .iter()
                .find(|entry| {
                    entry.relative_path == *source_path && entry.kind == LawLabTreeEntryKindV1::File
                })
                .cloned()
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_prediction_copy_source_missing",
                ))?;
            let mut target = source;
            target.relative_path = target_path.clone();
            entries.push(target);
        }
        K2LearnedEffectLawBodyV1::RemoveFile { path } => {
            let before = entries.len();
            entries.retain(|entry| entry.relative_path != *path);
            if entries.len() + 1 != before {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_prediction_remove_path_missing",
                ));
            }
        }
    }
    seal_manifest_entries_v1(entries)
}

fn seal_manifest_entries_v1(
    mut entries: Vec<LawLabTreeEntryV1>,
) -> K2GoalEnvironmentResultV1<LawLabTreeManifestV1> {
    entries.sort();
    let total_file_bytes = entries
        .iter()
        .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        .try_fold(0_u64, |total, entry| total.checked_add(entry.byte_length))
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_predicted_manifest_bytes_overflow",
        ))?;
    #[derive(Serialize)]
    struct ManifestDigestV1<'a> {
        schema: &'static str,
        total_file_bytes: u64,
        entries: &'a [LawLabTreeEntryV1],
    }
    let tree_root_sha256 = learned_root_v1(&ManifestDigestV1 {
        schema: LAW_LAB_TREE_MANIFEST_SCHEMA_V1,
        total_file_bytes,
        entries: &entries,
    })?;
    let manifest = LawLabTreeManifestV1 {
        schema: LAW_LAB_TREE_MANIFEST_SCHEMA_V1.to_owned(),
        tree_root_sha256,
        total_file_bytes,
        entries,
    };
    manifest
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    Ok(manifest)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2TargetPredictionRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub public_context_root_sha256: String,
    pub catalog: K2OpaqueActionCatalogV1,
    pub learned_law_set: K2LearnedEffectLawSetV1,
    pub target_pre_manifest: LawLabTreeManifestV1,
}

impl K2TargetPredictionRequestV1 {
    pub fn seal(
        public_context: &K2LearnerPublicContextV1,
        catalog: K2OpaqueActionCatalogV1,
        learned_law_set: K2LearnedEffectLawSetV1,
        target_pre_manifest: LawLabTreeManifestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        public_context.validate_persisted_v1()?;
        catalog.validate()?;
        learned_law_set.validate()?;
        target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let mut request = Self {
            schema: K2_TARGET_PREDICTION_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            public_context_root_sha256: public_context.public_context_root_sha256.clone(),
            catalog,
            learned_law_set,
            target_pre_manifest,
        };
        request.request_root_sha256 = request.expected_root_v1()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.catalog.validate()?;
        self.learned_law_set.validate()?;
        self.target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        if self.schema != K2_TARGET_PREDICTION_REQUEST_SCHEMA_V1
            || self.public_context_root_sha256 != self.learned_law_set.public_context_root_sha256
            || self
                .catalog
                .action_ids_sha256
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != self
                    .learned_law_set
                    .laws
                    .iter()
                    .map(|law| law.action_id_sha256.as_str())
                    .collect::<Vec<_>>()
            || self.request_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_request_invalid",
            ));
        }
        if self.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_request_budget_exhausted",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let request = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_REQUEST_BYTES_V1,
            "k2_target_prediction_request_protocol_invalid",
        )?;
        request.validate()?;
        Ok(request)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_TARGET_PREDICTION_REQUEST_SCHEMA_V1,
            self.public_context_root_sha256.as_str(),
            &self.catalog,
            &self.learned_law_set,
            &self.target_pre_manifest,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedTargetPredictionV1 {
    pub schema: String,
    pub prediction_root_sha256: String,
    pub action_id_sha256: String,
    pub learned_law_root_sha256: String,
    pub predicted_terminal_manifest: LawLabTreeManifestV1,
}

impl K2LearnedTargetPredictionV1 {
    fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.predicted_terminal_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        for root in [
            self.action_id_sha256.as_str(),
            self.learned_law_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_target_prediction_root_invalid")?;
        }
        if self.schema != K2_LEARNED_TARGET_PREDICTION_SCHEMA_V1
            || self.prediction_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_TARGET_PREDICTION_SCHEMA_V1,
            self.action_id_sha256.as_str(),
            self.learned_law_root_sha256.as_str(),
            &self.predicted_terminal_manifest,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedTargetPredictionSetV1 {
    pub schema: String,
    pub prediction_set_root_sha256: String,
    pub target_prediction_request_root_sha256: String,
    pub public_context_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub learned_law_set_root_sha256: String,
    pub target_pre_tree_root_sha256: String,
    pub predictions: Vec<K2LearnedTargetPredictionV1>,
    pub learned: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedTargetPredictionSetV1 {
    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.target_prediction_request_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_target_prediction_set_root_invalid")?;
        }
        if self.schema != K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1
            || self.predictions.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .predictions
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
            || !self.learned
            || self.prediction_set_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_set_invalid",
            ));
        }
        for prediction in &self.predictions {
            prediction.validate()?;
        }
        Ok(())
    }

    pub fn prediction(&self, action_id_sha256: &str) -> Option<&K2LearnedTargetPredictionV1> {
        self.predictions
            .iter()
            .find(|prediction| prediction.action_id_sha256 == action_id_sha256)
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let set = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_OUTCOME_BYTES_V1,
            "k2_target_prediction_set_protocol_invalid",
        )?;
        set.validate()?;
        Ok(set)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1,
            self.target_prediction_request_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
            &self.predictions,
            self.learned,
            &self.authority,
        ))
    }
}

pub fn verify_target_prediction_replay_v1(
    frozen: &K2LearnedTargetPredictionSetV1,
    replayed: &K2LearnedTargetPredictionSetV1,
) -> K2GoalEnvironmentResultV1<()> {
    frozen.validate()?;
    if replayed != frozen {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_target_prediction_root_mismatch",
        ));
    }
    replayed.validate()
}

pub fn require_exact_goal_for_learned_capability_v1(
    exact_goal: &K2ExactGoalReceiptV1,
) -> K2GoalEnvironmentResultV1<()> {
    exact_goal.validate_persisted_v1()?;
    if !exact_goal.goal_satisfied {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_exact_goal_unsatisfied",
        ));
    }
    Ok(())
}

pub fn predict_target_v1(
    request: &K2TargetPredictionRequestV1,
) -> K2GoalEnvironmentResultV1<K2LearnedTargetPredictionSetV1> {
    request.validate()?;
    let mut predictions = request
        .learned_law_set
        .laws
        .iter()
        .map(|law| {
            let predicted_terminal_manifest =
                apply_learned_effect_v1(&request.target_pre_manifest, &law.effect)?;
            let mut prediction = K2LearnedTargetPredictionV1 {
                schema: K2_LEARNED_TARGET_PREDICTION_SCHEMA_V1.to_owned(),
                prediction_root_sha256: String::new(),
                action_id_sha256: law.action_id_sha256.clone(),
                learned_law_root_sha256: law.law_root_sha256.clone(),
                predicted_terminal_manifest,
            };
            prediction.prediction_root_sha256 = prediction.expected_root_v1()?;
            prediction.validate()?;
            Ok(prediction)
        })
        .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
    predictions.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
    let mut set = K2LearnedTargetPredictionSetV1 {
        schema: K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1.to_owned(),
        prediction_set_root_sha256: String::new(),
        target_prediction_request_root_sha256: request.request_root_sha256.clone(),
        public_context_root_sha256: request.public_context_root_sha256.clone(),
        learner_manifest_root_sha256: request.learned_law_set.learner_manifest_root_sha256.clone(),
        learner_executable_sha256: request.learned_law_set.learner_executable_sha256.clone(),
        learned_law_set_root_sha256: request.learned_law_set.law_set_root_sha256.clone(),
        target_pre_tree_root_sha256: request.target_pre_manifest.tree_root_sha256.clone(),
        predictions,
        learned: true,
        authority: K2AuthorityBoundaryV1::authority_free_v1(),
    };
    set.prediction_set_root_sha256 = set.expected_root_v1()?;
    set.validate()?;
    if set.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_OUTCOME_BYTES_V1 {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_target_prediction_set_budget_exhausted",
        ));
    }
    Ok(set)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2GeneratedAblationProvenanceV1 {
    GeneratedCapabilityAblation,
    GeneratedCapabilitySelfTest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GeneratedAblationObservationV1 {
    pub schema: String,
    pub observation_root_sha256: String,
    pub provenance: K2GeneratedAblationProvenanceV1,
    pub source_observation_root_sha256: String,
    pub source_probe_ordinal: u64,
    pub support_world_root_sha256: String,
    pub action_id_sha256: String,
    pub pre_work_manifest: LawLabTreeManifestV1,
    pub post_work_manifest: LawLabTreeManifestV1,
}

impl K2GeneratedAblationObservationV1 {
    pub fn unchanged_from_support_v1(
        source: &K2SupportObservationV1,
        action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        Self::seal_from_support_v1(
            source,
            action_id_sha256,
            source.pre_work_manifest.clone(),
            source.post_work_manifest.clone(),
        )
    }

    pub fn ambiguous_copy_source_from_support_v1(
        source: &K2SupportObservationV1,
        action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source.validate_persisted_v1()?;
        let mut duplicate = source
            .pre_work_manifest
            .entry(K2_COPY_SOURCE_PATH_V1)
            .cloned()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_copy_source_missing",
            ))?;
        duplicate.relative_path = "duplicate-input.bin".to_owned();
        let mut pre_entries = source.pre_work_manifest.entries.clone();
        pre_entries.push(duplicate.clone());
        let mut post_entries = source.post_work_manifest.entries.clone();
        post_entries.push(duplicate);
        Self::seal_from_support_v1(
            source,
            action_id_sha256,
            seal_manifest_entries_v1(pre_entries)?,
            seal_manifest_entries_v1(post_entries)?,
        )
    }

    pub fn constant_output_from_support_v1(
        source: &K2SupportObservationV1,
        donor: &K2SupportObservationV1,
        action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source.validate_persisted_v1()?;
        donor.validate_persisted_v1()?;
        let mut donor_entry = donor
            .post_work_manifest
            .entry(K2_COPY_TARGET_PATH_V1)
            .cloned()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_constant_donor_missing",
            ))?;
        donor_entry.relative_path = K2_COPY_TARGET_PATH_V1.to_owned();
        let mut post_entries = source.post_work_manifest.entries.clone();
        let target = post_entries
            .iter_mut()
            .find(|entry| entry.relative_path == K2_COPY_TARGET_PATH_V1)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_constant_target_missing",
            ))?;
        *target = donor_entry;
        Self::seal_from_support_v1(
            source,
            action_id_sha256,
            source.pre_work_manifest.clone(),
            seal_manifest_entries_v1(post_entries)?,
        )
    }

    pub fn outcome_equals_pre_from_support_v1(
        source: &K2SupportObservationV1,
        action_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        Self::seal_from_support_v1(
            source,
            action_id_sha256,
            source.pre_work_manifest.clone(),
            source.pre_work_manifest.clone(),
        )
    }

    fn seal_from_support_v1(
        source: &K2SupportObservationV1,
        action_id_sha256: String,
        pre_work_manifest: LawLabTreeManifestV1,
        post_work_manifest: LawLabTreeManifestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source.validate_persisted_v1()?;
        require_learned_root_v1(&action_id_sha256, "k2_ablation_action_id_invalid")?;
        pre_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        post_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        let mut observation = Self {
            schema: K2_GENERATED_ABLATION_OBSERVATION_SCHEMA_V1.to_owned(),
            observation_root_sha256: String::new(),
            provenance: K2GeneratedAblationProvenanceV1::GeneratedCapabilityAblation,
            source_observation_root_sha256: source.observation_root_sha256.clone(),
            source_probe_ordinal: source.probe_ordinal,
            support_world_root_sha256: source.support_world_root_sha256.clone(),
            action_id_sha256,
            pre_work_manifest,
            post_work_manifest,
        };
        observation.observation_root_sha256 = observation.expected_root_v1()?;
        observation.validate_persisted_v1()?;
        Ok(observation)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.pre_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        self.post_work_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        for root in [
            self.observation_root_sha256.as_str(),
            self.source_observation_root_sha256.as_str(),
            self.support_world_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_ablation_observation_root_invalid")?;
        }
        if self.schema != K2_GENERATED_ABLATION_OBSERVATION_SCHEMA_V1
            || self.provenance != K2GeneratedAblationProvenanceV1::GeneratedCapabilityAblation
            || self.source_probe_ordinal >= K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || self.observation_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_evidence_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_GENERATED_ABLATION_OBSERVATION_SCHEMA_V1,
            self.provenance,
            self.source_observation_root_sha256.as_str(),
            self.source_probe_ordinal,
            self.support_world_root_sha256.as_str(),
            self.action_id_sha256.as_str(),
            &self.pre_work_manifest,
            &self.post_work_manifest,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GeneratedAblationRequestV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub provenance: K2GeneratedAblationProvenanceV1,
    pub source_learning_request: K2EffectLearningRequestV1,
    pub catalog: K2OpaqueActionCatalogV1,
    pub observations: Vec<K2GeneratedAblationObservationV1>,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2GeneratedAblationRequestV1 {
    pub fn seal(
        source_learning_request: K2EffectLearningRequestV1,
        catalog: K2OpaqueActionCatalogV1,
        mut observations: Vec<K2GeneratedAblationObservationV1>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        source_learning_request.validate()?;
        catalog.validate()?;
        observations.sort_by_key(|observation| observation.source_probe_ordinal);
        let mut request = Self {
            schema: K2_GENERATED_ABLATION_REQUEST_SCHEMA_V1.to_owned(),
            request_root_sha256: String::new(),
            provenance: K2GeneratedAblationProvenanceV1::GeneratedCapabilityAblation,
            source_learning_request,
            catalog,
            observations,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        request.request_root_sha256 = request.expected_root_v1()?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        self.source_learning_request.validate()?;
        self.catalog.validate()?;
        require_learned_root_v1(
            &self.request_root_sha256,
            "k2_ablation_request_root_invalid",
        )?;
        if self.schema != K2_GENERATED_ABLATION_REQUEST_SCHEMA_V1
            || self.provenance != K2GeneratedAblationProvenanceV1::GeneratedCapabilityAblation
            || !(4..=K2_LEARNED_SUPPORT_PROBE_COUNT_V1).contains(&self.observations.len())
            || self
                .observations
                .windows(2)
                .any(|pair| pair[0].source_probe_ordinal >= pair[1].source_probe_ordinal)
            || self.request_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_evidence_invalid",
            ));
        }
        let catalog_ids = self
            .catalog
            .action_ids_sha256
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let observed_ids = self
            .observations
            .iter()
            .map(|observation| observation.action_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if observed_ids != catalog_ids {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_support_evidence_invalid",
            ));
        }
        for observation in &self.observations {
            observation.validate_persisted_v1()?;
            let source = self
                .source_learning_request
                .support_observations
                .observations
                .iter()
                .find(|source| source.probe_ordinal == observation.source_probe_ordinal)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_evidence_invalid",
                ))?;
            if source.observation_root_sha256 != observation.source_observation_root_sha256
                || source.support_world_root_sha256 != observation.support_world_root_sha256
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_support_evidence_invalid",
                ));
            }
        }
        if self.canonical_bytes_v1()?.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_request_budget_exhausted",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_GENERATED_ABLATION_REQUEST_SCHEMA_V1,
            self.provenance,
            &self.source_learning_request,
            &self.catalog,
            &self.observations,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "command", content = "payload", rename_all = "snake_case")]
pub enum K2EffectLearnerProtocolRequestV1 {
    LearnEffects(K2EffectLearningRequestV1),
    PredictTarget(K2TargetPredictionRequestV1),
    EvaluateGeneratedAblation(K2GeneratedAblationRequestV1),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "outcome", content = "payload", rename_all = "snake_case")]
pub enum K2EffectLearnerProtocolOutcomeV1 {
    LearnedEffects(K2LearnedEffectLawSetV1),
    TargetPredictions(K2LearnedTargetPredictionSetV1),
    GeneratedAblation(K2GeneratedAblationOutcomeV1),
}

impl K2EffectLearnerProtocolRequestV1 {
    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        learned_bytes_v1(self)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let request = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_REQUEST_BYTES_V1,
            "k2_effect_learner_protocol_request_invalid",
        )?;
        match &request {
            Self::LearnEffects(value) => value.validate()?,
            Self::PredictTarget(value) => value.validate()?,
            Self::EvaluateGeneratedAblation(value) => value.validate()?,
        }
        Ok(request)
    }

    pub fn evaluate_v1(&self) -> K2GoalEnvironmentResultV1<K2EffectLearnerProtocolOutcomeV1> {
        match self {
            Self::LearnEffects(request) => Ok(K2EffectLearnerProtocolOutcomeV1::LearnedEffects(
                learn_effects_v1(request)?,
            )),
            Self::PredictTarget(request) => Ok(
                K2EffectLearnerProtocolOutcomeV1::TargetPredictions(predict_target_v1(request)?),
            ),
            Self::EvaluateGeneratedAblation(request) => {
                Ok(K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(
                    K2GeneratedAblationOutcomeV1::evaluate(request)?,
                ))
            }
        }
    }
}

impl K2EffectLearnerProtocolOutcomeV1 {
    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        let bytes = learned_bytes_v1(self)?;
        if bytes.len() > K2_LEARNER_MAX_OUTCOME_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_protocol_outcome_too_large",
            ));
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes_v1(bytes: &[u8]) -> K2GoalEnvironmentResultV1<Self> {
        let outcome = parse_canonical_v1::<Self>(
            bytes,
            K2_LEARNER_MAX_OUTCOME_BYTES_V1,
            "k2_effect_learner_protocol_outcome_invalid",
        )?;
        match &outcome {
            Self::LearnedEffects(value) => value.validate()?,
            Self::TargetPredictions(value) => value.validate()?,
            Self::GeneratedAblation(value) => value.validate_persisted_v1()?,
        }
        Ok(outcome)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2TargetIndependenceReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub support_set_root_sha256: String,
    pub target_pre_tree_root_sha256: String,
    pub support_tree_roots_pairwise_distinct: bool,
    pub target_tree_root_novel: bool,
    pub target_input_hash_novel: bool,
    pub target_input_length_novel: bool,
    pub target_obsolete_hash_novel: bool,
    pub target_obsolete_length_novel: bool,
    pub target_distractor_topology_novel: bool,
    pub target_absent_from_learning_request: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2TargetIndependenceReceiptV1 {
    pub fn verify(
        support: &K2SupportWorldSetV1,
        target_pre_manifest: &LawLabTreeManifestV1,
        learning_request: &K2EffectLearningRequestV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        support.validate()?;
        target_pre_manifest
            .validate()
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        learning_request.validate()?;
        if learning_request.public_context.support_set_root_sha256
            != support.support_set_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_holdout_support_binding_invalid",
            ));
        }
        let support_roots = support
            .worlds
            .iter()
            .map(|world| world.source_manifest.tree_root_sha256.as_str())
            .collect::<Vec<_>>();
        let support_tree_roots_pairwise_distinct =
            support_roots.iter().copied().collect::<BTreeSet<_>>().len() == support_roots.len();
        let target_tree_root_novel = !support_roots
            .iter()
            .any(|root| **root == target_pre_manifest.tree_root_sha256);
        let target_input = target_pre_manifest
            .entry(K2_COPY_SOURCE_PATH_V1)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid("k2_target_input_missing"))?;
        let target_obsolete = target_pre_manifest.entry(K2_REMOVE_PATH_V1).ok_or(
            K2GoalEnvironmentErrorV1::Invalid("k2_target_obsolete_missing"),
        )?;
        let support_input = support
            .worlds
            .iter()
            .map(|world| {
                world.source_manifest.entry(K2_COPY_SOURCE_PATH_V1).ok_or(
                    K2GoalEnvironmentErrorV1::Invalid("k2_support_input_missing"),
                )
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        let support_obsolete = support
            .worlds
            .iter()
            .map(|world| {
                world.source_manifest.entry(K2_REMOVE_PATH_V1).ok_or(
                    K2GoalEnvironmentErrorV1::Invalid("k2_support_obsolete_missing"),
                )
            })
            .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
        let target_input_hash_novel = target_input.content_sha256.is_some()
            && support_input
                .iter()
                .all(|entry| entry.content_sha256 != target_input.content_sha256);
        let target_input_length_novel = support_input
            .iter()
            .all(|entry| entry.byte_length != target_input.byte_length);
        let target_obsolete_hash_novel = target_obsolete.content_sha256.is_some()
            && support_obsolete
                .iter()
                .all(|entry| entry.content_sha256 != target_obsolete.content_sha256);
        let target_obsolete_length_novel = support_obsolete
            .iter()
            .all(|entry| entry.byte_length != target_obsolete.byte_length);
        let target_topology_root = distractor_topology_root_v1(target_pre_manifest)?;
        let target_distractor_topology_novel = support.worlds.iter().all(|world| {
            distractor_topology_root_v1(&world.source_manifest)
                .is_ok_and(|root| root != target_topology_root)
        });
        let learning_bytes = learning_request.canonical_bytes_v1()?;
        let target_manifest_bytes = learned_bytes_v1(target_pre_manifest)?;
        let target_absent_from_learning_request =
            !contains_bytes_v1(
                &learning_bytes,
                target_pre_manifest.tree_root_sha256.as_bytes(),
            ) && !contains_bytes_v1(&learning_bytes, &target_manifest_bytes);
        let mut receipt = Self {
            schema: K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            support_set_root_sha256: support.support_set_root_sha256.clone(),
            target_pre_tree_root_sha256: target_pre_manifest.tree_root_sha256.clone(),
            support_tree_roots_pairwise_distinct,
            target_tree_root_novel,
            target_input_hash_novel,
            target_input_length_novel,
            target_obsolete_hash_novel,
            target_obsolete_length_novel,
            target_distractor_topology_novel,
            target_absent_from_learning_request,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok(receipt)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.support_set_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_target_independence_root_invalid")?;
        }
        let all_independent = self.support_tree_roots_pairwise_distinct
            && self.target_tree_root_novel
            && self.target_input_hash_novel
            && self.target_input_length_novel
            && self.target_obsolete_hash_novel
            && self.target_obsolete_length_novel
            && self.target_distractor_topology_novel
            && self.target_absent_from_learning_request;
        if self.schema != K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1
            || !all_independent
            || self.receipt_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_not_independent",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1,
            self.support_set_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
            self.support_tree_roots_pairwise_distinct,
            self.target_tree_root_novel,
            self.target_input_hash_novel,
            self.target_input_length_novel,
            self.target_obsolete_hash_novel,
            self.target_obsolete_length_novel,
            self.target_distractor_topology_novel,
            self.target_absent_from_learning_request,
            &self.authority,
        ))
    }
}

fn contains_bytes_v1(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedEffectVerificationReceiptV1 {
    pub schema: String,
    pub verification_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub verifier_contract_root_sha256: String,
    pub public_context_root_sha256: String,
    pub support_observation_set_root_sha256: String,
    pub learned_law_set_root_sha256: String,
    pub target_prediction_set_root_sha256: String,
    pub verified_support_laws: u64,
    pub verified_target_predictions: u64,
    pub wrong_laws: u64,
    pub wrong_predictions: u64,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedEffectVerificationReceiptV1 {
    pub fn verify(
        freeze: &K2LearnedCapabilityFreezeV1,
        learning_request: &K2EffectLearningRequestV1,
        laws: &K2LearnedEffectLawSetV1,
        prediction_request: &K2TargetPredictionRequestV1,
        predictions: &K2LearnedTargetPredictionSetV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        freeze.validate_persisted_v1()?;
        learning_request.validate()?;
        laws.validate()?;
        prediction_request.validate()?;
        predictions.validate()?;
        if laws.learning_request_root_sha256 != learning_request.request_root_sha256
            || prediction_request.learned_law_set.law_set_root_sha256 != laws.law_set_root_sha256
            || predictions.target_prediction_request_root_sha256
                != prediction_request.request_root_sha256
            || freeze.independent_verifier_contract_root_sha256
                != learned_root_v1(&K2_INDEPENDENT_EFFECT_VERIFIER_CONTRACT_V1)?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_independent_verifier_binding_invalid",
            ));
        }
        let mut verified_support_laws = 0_u64;
        let mut wrong_laws = 0_u64;
        for law in &laws.laws {
            let observations = learning_request
                .support_observations
                .observations
                .iter()
                .filter(|observation| observation.action_id_sha256 == law.action_id_sha256)
                .collect::<Vec<_>>();
            let candidates = observations.first().map_or_else(
                || Ok(Vec::new()),
                |observation| independent_effect_candidates_v1(observation),
            )?;
            let survivors = candidates
                .iter()
                .filter(|candidate| {
                    observations.iter().all(|observation| {
                        independent_apply_effect_v1(&observation.pre_work_manifest, candidate)
                            .is_ok_and(|manifest| manifest == observation.post_work_manifest)
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            let candidate_roots = candidates
                .iter()
                .map(|candidate| independent_candidate_root_v1(&law.action_id_sha256, candidate))
                .collect::<K2GoalEnvironmentResultV1<Vec<_>>>()?;
            let valid = observations.len() == K2_LEARNED_SUPPORT_WORLD_COUNT_V1
                && survivors.as_slice() == [law.effect.clone()]
                && law.enumerated_candidate_count == candidates.len() as u64
                && law.enumerated_candidate_roots_sha256 == candidate_roots
                && law.rejected_candidate_count + law.version_space_size
                    == law.enumerated_candidate_count
                && law.version_space_size == 1;
            if valid {
                verified_support_laws += 1;
            } else {
                wrong_laws += 1;
            }
        }
        let mut verified_target_predictions = 0_u64;
        let mut wrong_predictions = 0_u64;
        for prediction in &predictions.predictions {
            let law = laws.law(&prediction.action_id_sha256);
            let valid = law.is_some_and(|law| {
                prediction.learned_law_root_sha256 == law.law_root_sha256
                    && independent_apply_effect_v1(
                        &prediction_request.target_pre_manifest,
                        &law.effect,
                    )
                    .is_ok_and(|manifest| manifest == prediction.predicted_terminal_manifest)
            });
            if valid {
                verified_target_predictions += 1;
            } else {
                wrong_predictions += 1;
            }
        }
        let mut receipt = Self {
            schema: K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1.to_owned(),
            verification_root_sha256: String::new(),
            experiment_freeze_root_sha256: freeze.freeze_root_sha256.clone(),
            verifier_contract_root_sha256: freeze.independent_verifier_contract_root_sha256.clone(),
            public_context_root_sha256: learning_request
                .public_context
                .public_context_root_sha256
                .clone(),
            support_observation_set_root_sha256: learning_request
                .support_observations
                .observation_set_root_sha256
                .clone(),
            learned_law_set_root_sha256: laws.law_set_root_sha256.clone(),
            target_prediction_set_root_sha256: predictions.prediction_set_root_sha256.clone(),
            verified_support_laws,
            verified_target_predictions,
            wrong_laws,
            wrong_predictions,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.verification_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok(receipt)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.verification_root_sha256.as_str(),
            self.experiment_freeze_root_sha256.as_str(),
            self.verifier_contract_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.support_observation_set_root_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_prediction_set_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_effect_verification_root_invalid")?;
        }
        if self.schema != K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1
            || self.verified_support_laws != K2_LEARNED_ACTION_COUNT_V1 as u64
            || self.verified_target_predictions != K2_LEARNED_ACTION_COUNT_V1 as u64
            || self.wrong_laws != 0
            || self.wrong_predictions != 0
            || self.verification_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_independent_effect_verification_failed",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1,
            self.experiment_freeze_root_sha256.as_str(),
            self.verifier_contract_root_sha256.as_str(),
            self.public_context_root_sha256.as_str(),
            self.support_observation_set_root_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_prediction_set_root_sha256.as_str(),
            self.verified_support_laws,
            self.verified_target_predictions,
            self.wrong_laws,
            self.wrong_predictions,
            &self.authority,
        ))
    }
}

fn independent_candidate_root_v1(
    action_id_sha256: &str,
    candidate: &K2LearnedEffectLawBodyV1,
) -> K2GoalEnvironmentResultV1<String> {
    learned_root_v1(&(
        K2_LEARNED_EFFECT_LAW_SCHEMA_V1,
        "candidate",
        action_id_sha256,
        candidate,
    ))
}

fn independent_effect_candidates_v1(
    observation: &K2SupportObservationV1,
) -> K2GoalEnvironmentResultV1<Vec<K2LearnedEffectLawBodyV1>> {
    observation.validate_persisted_v1()?;
    let pre = observation
        .pre_work_manifest
        .entries
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let post = observation
        .post_work_manifest
        .entries
        .iter()
        .map(|entry| (entry.relative_path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let added = post
        .iter()
        .filter_map(|(path, entry)| (!pre.contains_key(path)).then_some(*entry))
        .collect::<Vec<_>>();
    let removed = pre
        .iter()
        .filter_map(|(path, entry)| (!post.contains_key(path)).then_some(*entry))
        .collect::<Vec<_>>();
    let changed = pre.iter().any(|(path, entry)| {
        post.get(path)
            .is_some_and(|post_entry| **post_entry != **entry)
    });
    let mut candidates = Vec::new();
    if added.len() == 1 && removed.is_empty() && !changed {
        let target = added[0];
        if target.kind == LawLabTreeEntryKindV1::File {
            candidates.extend(
                pre.values()
                    .filter(|source| {
                        source.kind == LawLabTreeEntryKindV1::File
                            && source.byte_length == target.byte_length
                            && source.content_sha256 == target.content_sha256
                            && source.executable == target.executable
                    })
                    .map(|source| K2LearnedEffectLawBodyV1::CopyFile {
                        source_path: source.relative_path.clone(),
                        target_path: target.relative_path.clone(),
                    }),
            );
        }
    }
    if removed.len() == 1
        && added.is_empty()
        && !changed
        && removed[0].kind == LawLabTreeEntryKindV1::File
    {
        candidates.push(K2LearnedEffectLawBodyV1::RemoveFile {
            path: removed[0].relative_path.clone(),
        });
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn independent_apply_effect_v1(
    pre: &LawLabTreeManifestV1,
    effect: &K2LearnedEffectLawBodyV1,
) -> K2GoalEnvironmentResultV1<LawLabTreeManifestV1> {
    pre.validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    effect.validate()?;
    let mut entries = pre.entries.clone();
    match effect {
        K2LearnedEffectLawBodyV1::CopyFile {
            source_path,
            target_path,
        } => {
            if entries
                .iter()
                .any(|entry| entry.relative_path == *target_path)
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_independent_target_exists",
                ));
            }
            let mut copied = entries
                .iter()
                .find(|entry| {
                    entry.relative_path == *source_path && entry.kind == LawLabTreeEntryKindV1::File
                })
                .cloned()
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_independent_copy_source_missing",
                ))?;
            copied.relative_path.clone_from(target_path);
            entries.push(copied);
        }
        K2LearnedEffectLawBodyV1::RemoveFile { path } => {
            let previous_len = entries.len();
            entries.retain(|entry| entry.relative_path != *path);
            if entries.len() + 1 != previous_len {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_independent_remove_path_missing",
                ));
            }
        }
    }
    independent_seal_manifest_v1(entries)
}

fn independent_seal_manifest_v1(
    mut entries: Vec<LawLabTreeEntryV1>,
) -> K2GoalEnvironmentResultV1<LawLabTreeManifestV1> {
    entries.sort();
    let total_file_bytes = entries
        .iter()
        .filter(|entry| entry.kind == LawLabTreeEntryKindV1::File)
        .try_fold(0_u64, |total, entry| total.checked_add(entry.byte_length))
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_independent_manifest_bytes_overflow",
        ))?;
    #[derive(Serialize)]
    struct IndependentManifestDigestV1<'a> {
        schema: &'static str,
        total_file_bytes: u64,
        entries: &'a [LawLabTreeEntryV1],
    }
    let tree_root_sha256 = learned_root_v1(&IndependentManifestDigestV1 {
        schema: LAW_LAB_TREE_MANIFEST_SCHEMA_V1,
        total_file_bytes,
        entries: &entries,
    })?;
    let manifest = LawLabTreeManifestV1 {
        schema: LAW_LAB_TREE_MANIFEST_SCHEMA_V1.to_owned(),
        tree_root_sha256,
        total_file_bytes,
        entries,
    };
    manifest
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    Ok(manifest)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedToV1BindingEntryV1 {
    pub schema: String,
    pub entry_root_sha256: String,
    pub opaque_action_id_sha256: String,
    pub learned_law_root_sha256: String,
    pub predicted_terminal_tree_root_sha256: String,
    pub v1_fixture_action_root_sha256: String,
    pub hidden_operation_plan_root_sha256: String,
    pub v1_predicted_consequence_root_sha256: String,
}

impl K2LearnedToV1BindingEntryV1 {
    fn seal(
        action_id_sha256: String,
        law_root_sha256: String,
        prediction_root_sha256: String,
        operation_plan_root_sha256: String,
        action: &K2K1ActionRefV1,
    ) -> K2GoalEnvironmentResultV1<Self> {
        action.validate()?;
        let mut entry = Self {
            schema: K2_LEARNED_TO_V1_BINDING_ENTRY_SCHEMA_V1.to_owned(),
            entry_root_sha256: String::new(),
            opaque_action_id_sha256: action_id_sha256,
            learned_law_root_sha256: law_root_sha256,
            predicted_terminal_tree_root_sha256: prediction_root_sha256,
            v1_fixture_action_root_sha256: action.action_root_sha256.clone(),
            hidden_operation_plan_root_sha256: operation_plan_root_sha256,
            v1_predicted_consequence_root_sha256: action.predicted_consequence_root_sha256.clone(),
        };
        entry.entry_root_sha256 = entry.expected_root_v1()?;
        entry.validate_persisted_v1()?;
        Ok(entry)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.entry_root_sha256.as_str(),
            self.opaque_action_id_sha256.as_str(),
            self.learned_law_root_sha256.as_str(),
            self.predicted_terminal_tree_root_sha256.as_str(),
            self.v1_fixture_action_root_sha256.as_str(),
            self.hidden_operation_plan_root_sha256.as_str(),
            self.v1_predicted_consequence_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_to_v1_entry_root_invalid")?;
        }
        if self.schema != K2_LEARNED_TO_V1_BINDING_ENTRY_SCHEMA_V1
            || self.predicted_terminal_tree_root_sha256 != self.v1_predicted_consequence_root_sha256
            || self.entry_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_target_prediction_root_mismatch",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_TO_V1_BINDING_ENTRY_SCHEMA_V1,
            self.opaque_action_id_sha256.as_str(),
            self.learned_law_root_sha256.as_str(),
            self.predicted_terminal_tree_root_sha256.as_str(),
            self.v1_fixture_action_root_sha256.as_str(),
            self.hidden_operation_plan_root_sha256.as_str(),
            self.v1_predicted_consequence_root_sha256.as_str(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedToV1BindingV1 {
    pub schema: String,
    pub binding_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub hidden_mapping_root_sha256: String,
    pub learned_law_set_root_sha256: String,
    pub target_prediction_set_root_sha256: String,
    pub independent_verification_root_sha256: String,
    pub target_pre_tree_root_sha256: String,
    pub entries: Vec<K2LearnedToV1BindingEntryV1>,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedToV1BindingV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        freeze: &K2LearnedCapabilityFreezeV1,
        catalog: &K2OpaqueActionCatalogV1,
        mapping: &K2HiddenActionMappingV1,
        laws: &K2LearnedEffectLawSetV1,
        predictions: &K2LearnedTargetPredictionSetV1,
        verification: &K2LearnedEffectVerificationReceiptV1,
    ) -> K2GoalEnvironmentResultV1<(Self, Vec<K2K1ActionRefV1>)> {
        freeze.validate_persisted_v1()?;
        mapping.validate(catalog)?;
        laws.validate()?;
        predictions.validate()?;
        verification.validate_persisted_v1()?;
        if mapping.mapping_root_sha256 != freeze.hidden_mapping_root_sha256
            || laws.law_set_root_sha256 != verification.learned_law_set_root_sha256
            || predictions.prediction_set_root_sha256
                != verification.target_prediction_set_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_to_v1_input_binding_invalid",
            ));
        }
        let mut actions = Vec::with_capacity(K2_LEARNED_ACTION_COUNT_V1);
        let mut entries = Vec::with_capacity(K2_LEARNED_ACTION_COUNT_V1);
        for prediction in &predictions.predictions {
            let law =
                laws.law(&prediction.action_id_sha256)
                    .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_learned_to_v1_law_missing",
                    ))?;
            let hidden = mapping.entry(&prediction.action_id_sha256).ok_or(
                K2GoalEnvironmentErrorV1::Invalid("k2_learned_to_v1_mapping_missing"),
            )?;
            let action = K2K1ActionRefV1::seal(K2K1ActionRefInputV1 {
                provenance: K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest,
                applicability_environment_root_sha256: predictions
                    .target_pre_tree_root_sha256
                    .clone(),
                applicability_receipt_root_sha256: learned_root_v1(&(
                    K2_LEARNED_TO_V1_BINDING_SCHEMA_V1,
                    "applicability",
                    freeze.freeze_root_sha256.as_str(),
                    prediction.action_id_sha256.as_str(),
                ))?,
                operation_plan_root_sha256: hidden.operation_plan_root_sha256.clone(),
                predicted_consequence_root_sha256: prediction
                    .predicted_terminal_manifest
                    .tree_root_sha256
                    .clone(),
                fixture_effect_root_sha256: Some(law.law_root_sha256.clone()),
                law_certificate_root_sha256: None,
                epistemic_registry_member_root_sha256: None,
                bundle_v4_root_sha256: None,
                execution_certificate_root_sha256: None,
                applicability_guard_root_sha256: None,
                effect_contract_root_sha256: None,
                semantic_class_root_sha256: None,
                role_topology_root_sha256: None,
            })?;
            entries.push(K2LearnedToV1BindingEntryV1::seal(
                prediction.action_id_sha256.clone(),
                law.law_root_sha256.clone(),
                prediction
                    .predicted_terminal_manifest
                    .tree_root_sha256
                    .clone(),
                hidden.operation_plan_root_sha256.clone(),
                &action,
            )?);
            actions.push(action);
        }
        entries.sort_by(|left, right| {
            left.opaque_action_id_sha256
                .cmp(&right.opaque_action_id_sha256)
        });
        let mut binding = Self {
            schema: K2_LEARNED_TO_V1_BINDING_SCHEMA_V1.to_owned(),
            binding_root_sha256: String::new(),
            experiment_freeze_root_sha256: freeze.freeze_root_sha256.clone(),
            hidden_mapping_root_sha256: mapping.mapping_root_sha256.clone(),
            learned_law_set_root_sha256: laws.law_set_root_sha256.clone(),
            target_prediction_set_root_sha256: predictions.prediction_set_root_sha256.clone(),
            independent_verification_root_sha256: verification.verification_root_sha256.clone(),
            target_pre_tree_root_sha256: predictions.target_pre_tree_root_sha256.clone(),
            entries,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        binding.binding_root_sha256 = binding.expected_root_v1()?;
        binding.validate_persisted_v1()?;
        Ok((binding, actions))
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.binding_root_sha256.as_str(),
            self.experiment_freeze_root_sha256.as_str(),
            self.hidden_mapping_root_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_prediction_set_root_sha256.as_str(),
            self.independent_verification_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_to_v1_binding_root_invalid")?;
        }
        if self.schema != K2_LEARNED_TO_V1_BINDING_SCHEMA_V1
            || self.entries.len() != K2_LEARNED_ACTION_COUNT_V1
            || self
                .entries
                .windows(2)
                .any(|pair| pair[0].opaque_action_id_sha256 >= pair[1].opaque_action_id_sha256)
            || self.binding_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_to_v1_binding_invalid",
            ));
        }
        for entry in &self.entries {
            entry.validate_persisted_v1()?;
        }
        require_unique_roots_v1(
            self.entries
                .iter()
                .map(|entry| entry.v1_fixture_action_root_sha256.as_str()),
            "k2_learned_to_v1_action_roots_not_unique",
        )
    }

    pub fn entry_for_v1_action(
        &self,
        action_root_sha256: &str,
    ) -> Option<&K2LearnedToV1BindingEntryV1> {
        self.entries
            .iter()
            .find(|entry| entry.v1_fixture_action_root_sha256 == action_root_sha256)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_TO_V1_BINDING_SCHEMA_V1,
            self.experiment_freeze_root_sha256.as_str(),
            self.hidden_mapping_root_sha256.as_str(),
            self.learned_law_set_root_sha256.as_str(),
            self.target_prediction_set_root_sha256.as_str(),
            self.independent_verification_root_sha256.as_str(),
            self.target_pre_tree_root_sha256.as_str(),
            &self.entries,
            &self.authority,
        ))
    }
}

pub struct K2V1EpisodeEvidenceInputV1<'a> {
    pub learned_binding: &'a K2LearnedToV1BindingV1,
    pub decision_freeze: &'a K2DecisionFreezeV1,
    pub predictions: &'a K2AlternativePredictionSetV1,
    pub selection: &'a K2PreparedSelectionReceiptV1,
    pub law_lab_binding: &'a K2LawLabBindingV1,
    pub execution: &'a LawLabSandboxExecutionV1,
    pub exact_goal: &'a K2ExactGoalReceiptV1,
    pub outcome: &'a K2DecisionOutcomeReceiptV1,
    pub episode_seal: &'a K2DecisionEpisodeSealV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2V1EpisodeEvidenceV1 {
    pub schema: String,
    pub evidence_root_sha256: String,
    pub learned_to_v1_binding_root_sha256: String,
    pub v1_episode_id_sha256: String,
    pub v1_decision_freeze_root_sha256: String,
    pub v1_prediction_set_root_sha256: String,
    pub v1_selection_root_sha256: String,
    pub v1_selected_action_root_sha256: String,
    pub v1_law_lab_binding_root_sha256: String,
    pub v1_sandbox_receipt_root_sha256: String,
    pub v1_exact_goal_receipt_root_sha256: String,
    pub v1_terminal_outcome_root_sha256: String,
    pub v1_episode_seal_root_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2V1EpisodeEvidenceV1 {
    pub fn seal(input: K2V1EpisodeEvidenceInputV1<'_>) -> K2GoalEnvironmentResultV1<Self> {
        input.learned_binding.validate_persisted_v1()?;
        input.decision_freeze.validate_persisted_v1()?;
        input.predictions.validate_persisted_v1()?;
        input
            .selection
            .validate(input.decision_freeze, input.predictions)?;
        input.law_lab_binding.validate_persisted_v1()?;
        input.exact_goal.validate_persisted_v1()?;
        input.outcome.validate()?;
        input.episode_seal.validate()?;
        let selected_entry = input
            .learned_binding
            .entry_for_v1_action(&input.selection.selected_action_root_sha256)
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_v1_selected_action_not_learned",
            ))?;
        let prediction = input
            .predictions
            .predictions
            .iter()
            .find(|prediction| {
                prediction.action_root_sha256 == input.selection.selected_action_root_sha256
            })
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_v1_selected_prediction_missing",
            ))?;
        if selected_entry.predicted_terminal_tree_root_sha256
            != prediction.predicted_terminal_tree_root_sha256
            || input.law_lab_binding.selected_action_root_sha256
                != input.selection.selected_action_root_sha256
            || input.execution.receipt.request_root_sha256
                != input.law_lab_binding.law_lab_request_root_sha256
            || input.exact_goal.law_lab_binding_root_sha256
                != input.law_lab_binding.binding_root_sha256
            || input.exact_goal.law_lab_receipt_root_sha256
                != input.execution.receipt.receipt_root_sha256
            || input.outcome.decision_freeze_root_sha256
                != input.decision_freeze.decision_freeze_root_sha256
            || input.outcome.prediction_set_root_sha256
                != input.predictions.prediction_set_root_sha256
            || input.outcome.law_lab_binding_root_sha256
                != input.law_lab_binding.binding_root_sha256
            || input.outcome.sandbox_receipt_root_sha256
                != input.execution.receipt.receipt_root_sha256
            || input.outcome.exact_goal_receipt_root_sha256 != input.exact_goal.receipt_root_sha256
            || input.episode_seal.episode_id_sha256 != input.decision_freeze.episode_id_sha256
            || input.episode_seal.outcome_root_sha256 != input.outcome.outcome_root_sha256
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_v1_episode_evidence_binding_invalid",
            ));
        }
        let mut evidence = Self {
            schema: K2_V1_EPISODE_EVIDENCE_SCHEMA_V1.to_owned(),
            evidence_root_sha256: String::new(),
            learned_to_v1_binding_root_sha256: input.learned_binding.binding_root_sha256.clone(),
            v1_episode_id_sha256: input.decision_freeze.episode_id_sha256.clone(),
            v1_decision_freeze_root_sha256: input
                .decision_freeze
                .decision_freeze_root_sha256
                .clone(),
            v1_prediction_set_root_sha256: input.predictions.prediction_set_root_sha256.clone(),
            v1_selection_root_sha256: input.selection.selection_root_sha256.clone(),
            v1_selected_action_root_sha256: input.selection.selected_action_root_sha256.clone(),
            v1_law_lab_binding_root_sha256: input.law_lab_binding.binding_root_sha256.clone(),
            v1_sandbox_receipt_root_sha256: input.execution.receipt.receipt_root_sha256.clone(),
            v1_exact_goal_receipt_root_sha256: input.exact_goal.receipt_root_sha256.clone(),
            v1_terminal_outcome_root_sha256: input.outcome.outcome_root_sha256.clone(),
            v1_episode_seal_root_sha256: input.episode_seal.seal_root_sha256.clone(),
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        evidence.evidence_root_sha256 = evidence.expected_root_v1()?;
        evidence.validate_persisted_v1()?;
        Ok(evidence)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.evidence_root_sha256.as_str(),
            self.learned_to_v1_binding_root_sha256.as_str(),
            self.v1_episode_id_sha256.as_str(),
            self.v1_decision_freeze_root_sha256.as_str(),
            self.v1_prediction_set_root_sha256.as_str(),
            self.v1_selection_root_sha256.as_str(),
            self.v1_selected_action_root_sha256.as_str(),
            self.v1_law_lab_binding_root_sha256.as_str(),
            self.v1_sandbox_receipt_root_sha256.as_str(),
            self.v1_exact_goal_receipt_root_sha256.as_str(),
            self.v1_terminal_outcome_root_sha256.as_str(),
            self.v1_episode_seal_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_v1_episode_evidence_root_invalid")?;
        }
        if self.schema != K2_V1_EPISODE_EVIDENCE_SCHEMA_V1
            || self.evidence_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_v1_episode_evidence_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_V1_EPISODE_EVIDENCE_SCHEMA_V1,
            (
                self.learned_to_v1_binding_root_sha256.as_str(),
                self.v1_episode_id_sha256.as_str(),
                self.v1_decision_freeze_root_sha256.as_str(),
                self.v1_prediction_set_root_sha256.as_str(),
                self.v1_selection_root_sha256.as_str(),
                self.v1_selected_action_root_sha256.as_str(),
            ),
            (
                self.v1_law_lab_binding_root_sha256.as_str(),
                self.v1_sandbox_receipt_root_sha256.as_str(),
                self.v1_exact_goal_receipt_root_sha256.as_str(),
                self.v1_terminal_outcome_root_sha256.as_str(),
                self.v1_episode_seal_root_sha256.as_str(),
                &self.authority,
            ),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2PrivateExperimentArtifactReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub private_contract_root_sha256: String,
    pub artifact_root_sha256: String,
    pub artifact_bytes_sha256: String,
    pub artifact_bytes: u64,
    pub file_mode: u32,
    pub file_synced: bool,
    pub directory_synced: bool,
    pub no_replace_publication: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2PrivateExperimentArtifactReceiptV1 {
    fn seal(
        contract: &K2PrivateExperimentContractV1,
        bytes: &[u8],
    ) -> K2GoalEnvironmentResultV1<Self> {
        let artifact_root_sha256 = learned_root_v1(contract)?;
        let mut receipt = Self {
            schema: K2_PRIVATE_EXPERIMENT_ARTIFACT_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            private_contract_root_sha256: contract.private_contract_root_sha256.clone(),
            artifact_root_sha256: artifact_root_sha256.clone(),
            artifact_bytes_sha256: artifact_root_sha256,
            artifact_bytes: u64::try_from(bytes.len()).map_err(|_| {
                K2GoalEnvironmentErrorV1::Invalid("k2_private_artifact_size_overflow")
            })?,
            file_mode: 0o400,
            file_synced: true,
            directory_synced: true,
            no_replace_publication: true,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok(receipt)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.private_contract_root_sha256.as_str(),
            self.artifact_root_sha256.as_str(),
            self.artifact_bytes_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_private_artifact_root_invalid")?;
        }
        if self.schema != K2_PRIVATE_EXPERIMENT_ARTIFACT_RECEIPT_SCHEMA_V1
            || self.artifact_root_sha256 != self.artifact_bytes_sha256
            || self.artifact_bytes == 0
            || self.file_mode != 0o400
            || !self.file_synced
            || !self.directory_synced
            || !self.no_replace_publication
            || self.receipt_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_private_artifact_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_PRIVATE_EXPERIMENT_ARTIFACT_RECEIPT_SCHEMA_V1,
            self.private_contract_root_sha256.as_str(),
            self.artifact_root_sha256.as_str(),
            self.artifact_bytes_sha256.as_str(),
            self.artifact_bytes,
            self.file_mode,
            self.file_synced,
            self.directory_synced,
            self.no_replace_publication,
            &self.authority,
        ))
    }
}

pub fn publish_private_experiment_contract_v1(
    path: &Path,
    contract: &K2PrivateExperimentContractV1,
) -> K2GoalEnvironmentResultV1<K2PrivateExperimentArtifactReceiptV1> {
    let parent = path.parent().ok_or(K2GoalEnvironmentErrorV1::Invalid(
        "k2_private_artifact_parent_missing",
    ))?;
    if !parent.is_dir() || path.file_name().is_none() {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_private_artifact_path_invalid",
        ));
    }
    let bytes = contract.canonical_bytes_v1()?;
    let file_name = path.file_name().and_then(|name| name.to_str()).ok_or(
        K2GoalEnvironmentErrorV1::Invalid("k2_private_artifact_name_invalid"),
    )?;
    let temp_path = parent.join(format!(".{file_name}.tmp"));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp_path)
            .map_err(io_error_learned_v1("create_private_artifact_temp"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(io_error_learned_v1("sync_private_artifact_temp"))?;
        fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o400))
            .map_err(io_error_learned_v1("chmod_private_artifact_temp"))?;
        drop(file);
        fs::hard_link(&temp_path, path)
            .map_err(io_error_learned_v1("publish_private_artifact_no_replace"))?;
        sync_directory_learned_v1(parent)?;
        fs::remove_file(&temp_path).map_err(io_error_learned_v1("remove_private_artifact_temp"))?;
        sync_directory_learned_v1(parent)?;
        let receipt = K2PrivateExperimentArtifactReceiptV1::seal(contract, &bytes)?;
        verify_private_artifact_file_v1(path, &receipt)?;
        Ok(receipt)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn reopen_private_experiment_contract_v1(
    path: &Path,
    receipt: &K2PrivateExperimentArtifactReceiptV1,
    context: &K2LearnerPublicContextV1,
    catalog: &K2OpaqueActionCatalogV1,
    support: &K2SupportWorldSetV1,
) -> K2GoalEnvironmentResultV1<K2PrivateExperimentContractV1> {
    receipt.validate_persisted_v1()?;
    verify_private_artifact_file_v1(path, receipt)?;
    let bytes = fs::read(path).map_err(io_error_learned_v1("read_private_artifact"))?;
    if bytes.len() as u64 != receipt.artifact_bytes {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_private_artifact_size_mismatch",
        ));
    }
    let contract = parse_canonical_v1::<K2PrivateExperimentContractV1>(
        &bytes,
        K2_LEARNER_MAX_REQUEST_BYTES_V1,
        "k2_private_artifact_decode_invalid",
    )?;
    contract.validate(context, catalog, support)?;
    if contract.private_contract_root_sha256 != receipt.private_contract_root_sha256
        || learned_root_v1(&contract)? != receipt.artifact_root_sha256
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_private_artifact_reopen_mismatch",
        ));
    }
    Ok(contract)
}

fn verify_private_artifact_file_v1(
    path: &Path,
    receipt: &K2PrivateExperimentArtifactReceiptV1,
) -> K2GoalEnvironmentResultV1<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(io_error_learned_v1("stat_private_artifact"))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o777 != receipt.file_mode
        || metadata.len() != receipt.artifact_bytes
        || law_lab_sha256_file_v1(path)
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?
            != receipt.artifact_bytes_sha256
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_private_artifact_file_invalid",
        ));
    }
    Ok(())
}

fn sync_directory_learned_v1(path: &Path) -> K2GoalEnvironmentResultV1<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(io_error_learned_v1("sync_directory"))
}

fn io_error_learned_v1(
    operation: &'static str,
) -> impl FnOnce(std::io::Error) -> K2GoalEnvironmentErrorV1 {
    move |error| K2GoalEnvironmentErrorV1::Io(format!("{operation}:{error}"))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2EffectLearnerProcessReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub learner_manifest_root_sha256: String,
    pub learner_executable_sha256: String,
    pub protocol_request_root_sha256: String,
    pub protocol_outcome_root_sha256: String,
    pub request_bytes: u64,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub elapsed_ms: u64,
    pub wall_limit_ms: u64,
    pub cpu_limit_seconds: u64,
    pub address_space_limit_bytes: u64,
    pub process_limit: u64,
    pub environment_cleared: bool,
    pub network_enabled: bool,
    pub repository_mounted: bool,
    pub private_contract_mounted: bool,
    pub target_store_mounted: bool,
    pub bwrap_sha256: String,
    pub prlimit_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2EffectLearnerProcessReceiptV1 {
    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.learner_manifest_root_sha256.as_str(),
            self.learner_executable_sha256.as_str(),
            self.protocol_request_root_sha256.as_str(),
            self.protocol_outcome_root_sha256.as_str(),
            self.bwrap_sha256.as_str(),
            self.prlimit_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learner_process_root_invalid")?;
        }
        if self.schema != K2_EFFECT_LEARNER_PROCESS_RECEIPT_SCHEMA_V1
            || self.request_bytes == 0
            || self.request_bytes > K2_LEARNER_MAX_REQUEST_BYTES_V1 as u64
            || self.stdout_bytes == 0
            || self.stdout_bytes > K2_LEARNER_MAX_OUTCOME_BYTES_V1 as u64
            || self.stderr_bytes != 0
            || self.elapsed_ms > K2_LEARNER_WALL_MS_V1
            || self.wall_limit_ms != K2_LEARNER_WALL_MS_V1
            || self.cpu_limit_seconds != K2_LEARNER_CPU_SECONDS_V1
            || self.address_space_limit_bytes != K2_LEARNER_ADDRESS_SPACE_BYTES_V1
            || self.process_limit != K2_LEARNER_PROCESS_COUNT_V1
            || !self.environment_cleared
            || self.network_enabled
            || self.repository_mounted
            || self.private_contract_mounted
            || self.target_store_mounted
            || self.receipt_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_process_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_EFFECT_LEARNER_PROCESS_RECEIPT_SCHEMA_V1,
            (
                self.learner_manifest_root_sha256.as_str(),
                self.learner_executable_sha256.as_str(),
                self.protocol_request_root_sha256.as_str(),
                self.protocol_outcome_root_sha256.as_str(),
                self.request_bytes,
                self.stdout_bytes,
                self.stderr_bytes,
                self.elapsed_ms,
            ),
            (
                self.wall_limit_ms,
                self.cpu_limit_seconds,
                self.address_space_limit_bytes,
                self.process_limit,
                self.environment_cleared,
                self.network_enabled,
                self.repository_mounted,
                self.private_contract_mounted,
                self.target_store_mounted,
                self.bwrap_sha256.as_str(),
                self.prlimit_sha256.as_str(),
                &self.authority,
            ),
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2EffectLearnerRunnerV1 {
    learner_path: PathBuf,
    bwrap_path: PathBuf,
    prlimit_path: PathBuf,
}

impl K2EffectLearnerRunnerV1 {
    #[must_use]
    pub fn new(learner_path: PathBuf) -> Self {
        Self {
            learner_path,
            bwrap_path: PathBuf::from("/usr/bin/bwrap"),
            prlimit_path: PathBuf::from("/usr/bin/prlimit"),
        }
    }

    pub fn learner_manifest_v1(&self) -> K2GoalEnvironmentResultV1<K2EffectLearnerManifestV1> {
        validate_executable_learned_v1(&self.learner_path)?;
        K2EffectLearnerManifestV1::seal(
            law_lab_sha256_file_v1(&self.learner_path)
                .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?,
        )
    }

    pub fn run_v1(
        &self,
        frozen_manifest: &K2EffectLearnerManifestV1,
        request: &K2EffectLearnerProtocolRequestV1,
    ) -> K2GoalEnvironmentResultV1<(
        K2EffectLearnerProtocolOutcomeV1,
        K2EffectLearnerProcessReceiptV1,
    )> {
        frozen_manifest.validate()?;
        validate_executable_learned_v1(&self.bwrap_path)?;
        validate_executable_learned_v1(&self.prlimit_path)?;
        validate_executable_learned_v1(&self.learner_path)?;
        let actual_learner_sha = law_lab_sha256_file_v1(&self.learner_path)
            .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
        if actual_learner_sha != frozen_manifest.executable_sha256 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_hash_mismatch",
            ));
        }
        let input = request.canonical_bytes_v1()?;
        if input.len() > K2_LEARNER_MAX_REQUEST_BYTES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_request_budget_exhausted",
            ));
        }
        let args = self.command_args_v1();
        let started = Instant::now();
        let (stdout, stderr) = run_bounded_learner_process_v1(
            &self.bwrap_path,
            &args,
            &input,
            Duration::from_millis(K2_LEARNER_WALL_MS_V1),
        )?;
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if !stderr.is_empty() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_stderr_not_empty",
            ));
        }
        let outcome = K2EffectLearnerProtocolOutcomeV1::from_canonical_bytes_v1(&stdout)?;
        validate_protocol_binding_v1(request, &outcome)?;
        let mut receipt = K2EffectLearnerProcessReceiptV1 {
            schema: K2_EFFECT_LEARNER_PROCESS_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            learner_manifest_root_sha256: frozen_manifest.manifest_root_sha256.clone(),
            learner_executable_sha256: actual_learner_sha,
            protocol_request_root_sha256: protocol_request_root_v1(request).to_owned(),
            protocol_outcome_root_sha256: protocol_outcome_root_v1(&outcome).to_owned(),
            request_bytes: input.len() as u64,
            stdout_bytes: stdout.len() as u64,
            stderr_bytes: stderr.len() as u64,
            elapsed_ms,
            wall_limit_ms: K2_LEARNER_WALL_MS_V1,
            cpu_limit_seconds: K2_LEARNER_CPU_SECONDS_V1,
            address_space_limit_bytes: K2_LEARNER_ADDRESS_SPACE_BYTES_V1,
            process_limit: K2_LEARNER_PROCESS_COUNT_V1,
            environment_cleared: true,
            network_enabled: false,
            repository_mounted: false,
            private_contract_mounted: false,
            target_store_mounted: false,
            bwrap_sha256: law_lab_sha256_file_v1(&self.bwrap_path)
                .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?,
            prlimit_sha256: law_lab_sha256_file_v1(&self.prlimit_path)
                .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok((outcome, receipt))
    }

    fn command_args_v1(&self) -> Vec<OsString> {
        const GUEST_LEARNER: &str = "/nando/bin/nando-k2-effect-learner";
        let mut args = vec![
            OsString::from("--unshare-all"),
            OsString::from("--die-with-parent"),
            OsString::from("--new-session"),
            OsString::from("--cap-drop"),
            OsString::from("ALL"),
            OsString::from("--clearenv"),
        ];
        for path in ["/usr", "/lib", "/lib64"]
            .into_iter()
            .filter(|path| Path::new(path).exists())
        {
            args.extend([
                OsString::from("--ro-bind"),
                OsString::from(path),
                OsString::from(path),
            ]);
        }
        args.extend([
            OsString::from("--proc"),
            OsString::from("/proc"),
            OsString::from("--dev"),
            OsString::from("/dev"),
            OsString::from("--tmpfs"),
            OsString::from("/tmp"),
            OsString::from("--dir"),
            OsString::from("/nando"),
            OsString::from("--dir"),
            OsString::from("/nando/bin"),
            OsString::from("--ro-bind"),
            self.learner_path.as_os_str().to_owned(),
            OsString::from(GUEST_LEARNER),
            OsString::from("--chdir"),
            OsString::from("/tmp"),
            OsString::from("--setenv"),
            OsString::from("LANG"),
            OsString::from("C"),
            OsString::from("--setenv"),
            OsString::from("LC_ALL"),
            OsString::from("C"),
            OsString::from("--setenv"),
            OsString::from("TZ"),
            OsString::from("UTC"),
            OsString::from("--"),
            self.prlimit_path.as_os_str().to_owned(),
            OsString::from(format!("--cpu={0}:{0}", K2_LEARNER_CPU_SECONDS_V1)),
            OsString::from(format!("--as={0}:{0}", K2_LEARNER_ADDRESS_SPACE_BYTES_V1)),
            OsString::from(format!("--nproc={0}:{0}", K2_LEARNER_PROCESS_COUNT_V1)),
            OsString::from(format!("--fsize={0}:{0}", K2_LEARNER_MAX_OUTCOME_BYTES_V1)),
            OsString::from("--"),
            OsString::from(GUEST_LEARNER),
        ]);
        args
    }
}

fn validate_executable_learned_v1(path: &Path) -> K2GoalEnvironmentResultV1<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(io_error_learned_v1("stat_learned_executable"))?;
    if !path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o111 == 0
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_executable_invalid",
        ));
    }
    Ok(())
}

fn protocol_request_root_v1(request: &K2EffectLearnerProtocolRequestV1) -> &str {
    match request {
        K2EffectLearnerProtocolRequestV1::LearnEffects(value) => &value.request_root_sha256,
        K2EffectLearnerProtocolRequestV1::PredictTarget(value) => &value.request_root_sha256,
        K2EffectLearnerProtocolRequestV1::EvaluateGeneratedAblation(value) => {
            &value.request_root_sha256
        }
    }
}

fn protocol_outcome_root_v1(outcome: &K2EffectLearnerProtocolOutcomeV1) -> &str {
    match outcome {
        K2EffectLearnerProtocolOutcomeV1::LearnedEffects(value) => &value.law_set_root_sha256,
        K2EffectLearnerProtocolOutcomeV1::TargetPredictions(value) => {
            &value.prediction_set_root_sha256
        }
        K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(value) => &value.outcome_root_sha256,
    }
}

fn validate_protocol_binding_v1(
    request: &K2EffectLearnerProtocolRequestV1,
    outcome: &K2EffectLearnerProtocolOutcomeV1,
) -> K2GoalEnvironmentResultV1<()> {
    let valid = match (request, outcome) {
        (
            K2EffectLearnerProtocolRequestV1::LearnEffects(request),
            K2EffectLearnerProtocolOutcomeV1::LearnedEffects(outcome),
        ) => outcome.learning_request_root_sha256 == request.request_root_sha256,
        (
            K2EffectLearnerProtocolRequestV1::PredictTarget(request),
            K2EffectLearnerProtocolOutcomeV1::TargetPredictions(outcome),
        ) => outcome.target_prediction_request_root_sha256 == request.request_root_sha256,
        (
            K2EffectLearnerProtocolRequestV1::EvaluateGeneratedAblation(request),
            K2EffectLearnerProtocolOutcomeV1::GeneratedAblation(outcome),
        ) => outcome.request_root_sha256 == request.request_root_sha256,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_protocol_binding_invalid",
        ))
    }
}

fn run_bounded_learner_process_v1(
    program: &Path,
    args: &[OsString],
    input: &[u8],
    deadline: Duration,
) -> K2GoalEnvironmentResultV1<(Vec<u8>, Vec<u8>)> {
    let mut child = Command::new(program)
        .args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(io_error_learned_v1("spawn_effect_learner"))?;
    child
        .stdin
        .take()
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_stdin_missing",
        ))?
        .write_all(input)
        .map_err(io_error_learned_v1("write_effect_learner_stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_stdout_missing",
        ))?;
    let stderr = child
        .stderr
        .take()
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_stderr_missing",
        ))?;
    let stdout_reader = thread::spawn(move || {
        read_limited_pipe_learned_v1(stdout, K2_LEARNER_MAX_OUTCOME_BYTES_V1)
    });
    let stderr_reader =
        thread::spawn(move || read_limited_pipe_learned_v1(stderr, K2_LEARNER_MAX_STDERR_BYTES_V1));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(io_error_learned_v1("poll_effect_learner"))?
        {
            break status;
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_effect_learner_timed_out",
            ));
        }
        thread::sleep(Duration::from_millis(2));
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_effect_learner_stdout_join_failed"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_effect_learner_stderr_join_failed"))??;
    if !status.success() {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_process_failed",
        ));
    }
    Ok((stdout, stderr))
}

fn read_limited_pipe_learned_v1(
    mut pipe: impl Read,
    maximum_bytes: usize,
) -> K2GoalEnvironmentResultV1<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 16 * 1024];
    let mut exceeded = false;
    loop {
        let read = pipe
            .read(&mut buffer)
            .map_err(io_error_learned_v1("read_effect_learner_pipe"))?;
        if read == 0 {
            break;
        }
        if output.len().saturating_add(read) <= maximum_bytes {
            output.extend_from_slice(&buffer[..read]);
        } else {
            exceeded = true;
        }
    }
    if exceeded {
        Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_effect_learner_pipe_budget_exhausted",
        ))
    } else {
        Ok(output)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedAblationKindV1 {
    SupportCount,
    ActionIdentityShuffle,
    AmbiguousCopySource,
    ConstantOutput,
    OutcomeDependence,
    DynamicId,
    HoldoutAlias,
    SupportProvenanceMismatch,
    TargetGoalLeakage,
    PredictionTamper,
    WrongActionExactOracle,
    CrossExperimentReplay,
    AuthorityTamper,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedAblationVerdictV1 {
    InsufficientSupport,
    NonTransferableDelta,
    AmbiguousSourceMatch,
    TransferableWithDynamicIds,
    TargetNotIndependent,
    SupportEvidenceInvalid,
    LearnerRequestPrivateFieldRejected,
    TargetPredictionRootMismatch,
    ExactGoalUnsatisfied,
    CrossExperimentReplay,
    AuthorityBoundaryViolated,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GeneratedAblationLearnedEffectV1 {
    pub action_id_sha256: String,
    pub effect: K2LearnedEffectLawBodyV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2GeneratedAblationOutcomeV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub request_root_sha256: String,
    pub observed_verdict: K2LearnedAblationVerdictV1,
    pub rejection_code: Option<String>,
    pub learned_effects: Vec<K2GeneratedAblationLearnedEffectV1>,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2GeneratedAblationOutcomeV1 {
    pub fn evaluate(request: &K2GeneratedAblationRequestV1) -> K2GoalEnvironmentResultV1<Self> {
        request.validate()?;
        let observations = request
            .observations
            .iter()
            .map(|observation| K2EffectObservationViewV1 {
                action_id_sha256: &observation.action_id_sha256,
                pre_work_manifest: &observation.pre_work_manifest,
                post_work_manifest: &observation.post_work_manifest,
            })
            .collect::<Vec<_>>();
        let (observed_verdict, rejection_code, learned_effects) =
            match infer_effects_v1(&request.catalog.action_ids_sha256, &observations) {
                Ok(inferred) => (
                    K2LearnedAblationVerdictV1::TransferableWithDynamicIds,
                    None,
                    inferred
                        .into_iter()
                        .map(|value| K2GeneratedAblationLearnedEffectV1 {
                            action_id_sha256: value.action_id_sha256,
                            effect: value.effect,
                        })
                        .collect(),
                ),
                Err(K2GoalEnvironmentErrorV1::Invalid(reason)) => {
                    let verdict = generated_ablation_verdict_for_rejection_v1(reason)?;
                    (verdict, Some(reason.to_owned()), Vec::new())
                }
                Err(error) => return Err(error),
            };
        let mut outcome = Self {
            schema: K2_GENERATED_ABLATION_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            request_root_sha256: request.request_root_sha256.clone(),
            observed_verdict,
            rejection_code,
            learned_effects,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        outcome.learned_effects.sort();
        outcome.outcome_root_sha256 = outcome.expected_root_v1()?;
        outcome.validate_persisted_v1()?;
        Ok(outcome)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.outcome_root_sha256.as_str(),
            self.request_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_generated_ablation_outcome_root_invalid")?;
        }
        let result_shape_valid = match self.observed_verdict {
            K2LearnedAblationVerdictV1::TransferableWithDynamicIds => {
                self.rejection_code.is_none()
                    && self.learned_effects.len() == K2_LEARNED_ACTION_COUNT_V1
                    && self
                        .learned_effects
                        .windows(2)
                        .all(|pair| pair[0].action_id_sha256 < pair[1].action_id_sha256)
                    && self.learned_effects.iter().all(|value| {
                        valid_nonzero_sha256(&value.action_id_sha256)
                            && value.effect.validate().is_ok()
                    })
            }
            K2LearnedAblationVerdictV1::InsufficientSupport
            | K2LearnedAblationVerdictV1::NonTransferableDelta
            | K2LearnedAblationVerdictV1::AmbiguousSourceMatch => {
                self.learned_effects.is_empty()
                    && self.rejection_code.as_deref()
                        == generated_ablation_rejection_code_v1(self.observed_verdict)
            }
            _ => false,
        };
        if self.schema != K2_GENERATED_ABLATION_OUTCOME_SCHEMA_V1
            || !result_shape_valid
            || self.outcome_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_generated_ablation_outcome_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate_persisted_v1()?;
        learned_bytes_v1(self)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_GENERATED_ABLATION_OUTCOME_SCHEMA_V1,
            self.request_root_sha256.as_str(),
            self.observed_verdict,
            self.rejection_code.as_deref(),
            &self.learned_effects,
            &self.authority,
        ))
    }
}

fn generated_ablation_verdict_for_rejection_v1(
    reason: &'static str,
) -> K2GoalEnvironmentResultV1<K2LearnedAblationVerdictV1> {
    match reason {
        "k2_insufficient_support" => Ok(K2LearnedAblationVerdictV1::InsufficientSupport),
        "k2_non_transferable_delta" | "k2_effect_values_not_transferable" => {
            Ok(K2LearnedAblationVerdictV1::NonTransferableDelta)
        }
        "k2_ambiguous_source_match" => Ok(K2LearnedAblationVerdictV1::AmbiguousSourceMatch),
        _ => Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_generated_ablation_unexpected_rejection",
        )),
    }
}

fn generated_ablation_rejection_code_v1(
    verdict: K2LearnedAblationVerdictV1,
) -> Option<&'static str> {
    match verdict {
        K2LearnedAblationVerdictV1::InsufficientSupport => Some("k2_insufficient_support"),
        K2LearnedAblationVerdictV1::NonTransferableDelta => Some("k2_non_transferable_delta"),
        K2LearnedAblationVerdictV1::AmbiguousSourceMatch => Some("k2_ambiguous_source_match"),
        _ => None,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedAblationControlV1 {
    pub schema: String,
    pub control_root_sha256: String,
    pub kind: K2LearnedAblationKindV1,
    pub input_root_sha256: String,
    pub expected_verdict: K2LearnedAblationVerdictV1,
    pub observed_verdict: K2LearnedAblationVerdictV1,
    pub learner_processes: u64,
    pub sandbox_probes: u64,
    pub oracle_invocations: u64,
    pub canonical_outcome_root_sha256: String,
    pub passed: bool,
}

impl K2LearnedAblationControlV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        kind: K2LearnedAblationKindV1,
        input_root_sha256: String,
        expected_verdict: K2LearnedAblationVerdictV1,
        observed_verdict: K2LearnedAblationVerdictV1,
        learner_processes: u64,
        sandbox_probes: u64,
        oracle_invocations: u64,
        canonical_outcome_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let mut control = Self {
            schema: K2_LEARNED_ABLATION_CONTROL_SCHEMA_V1.to_owned(),
            control_root_sha256: String::new(),
            kind,
            input_root_sha256,
            expected_verdict,
            observed_verdict,
            learner_processes,
            sandbox_probes,
            oracle_invocations,
            canonical_outcome_root_sha256,
            passed: expected_verdict == observed_verdict,
        };
        control.control_root_sha256 = control.expected_root_v1()?;
        control.validate_persisted_v1()?;
        Ok(control)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.control_root_sha256.as_str(),
            self.input_root_sha256.as_str(),
            self.canonical_outcome_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_ablation_control_root_invalid")?;
        }
        if self.schema != K2_LEARNED_ABLATION_CONTROL_SCHEMA_V1
            || self.expected_verdict != required_ablation_verdict_v1(self.kind)
            || self.observed_verdict != self.expected_verdict
            || !self.passed
            || self.learner_processes > 1
            || self.sandbox_probes > 1
            || self.oracle_invocations > 1
            || self.control_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_ablation_control_failed",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_ABLATION_CONTROL_SCHEMA_V1,
            self.kind,
            self.input_root_sha256.as_str(),
            self.expected_verdict,
            self.observed_verdict,
            self.learner_processes,
            self.sandbox_probes,
            self.oracle_invocations,
            self.canonical_outcome_root_sha256.as_str(),
            self.passed,
        ))
    }
}

fn required_ablation_verdict_v1(kind: K2LearnedAblationKindV1) -> K2LearnedAblationVerdictV1 {
    match kind {
        K2LearnedAblationKindV1::SupportCount => K2LearnedAblationVerdictV1::InsufficientSupport,
        K2LearnedAblationKindV1::ActionIdentityShuffle
        | K2LearnedAblationKindV1::ConstantOutput
        | K2LearnedAblationKindV1::OutcomeDependence => {
            K2LearnedAblationVerdictV1::NonTransferableDelta
        }
        K2LearnedAblationKindV1::AmbiguousCopySource => {
            K2LearnedAblationVerdictV1::AmbiguousSourceMatch
        }
        K2LearnedAblationKindV1::DynamicId => {
            K2LearnedAblationVerdictV1::TransferableWithDynamicIds
        }
        K2LearnedAblationKindV1::HoldoutAlias => K2LearnedAblationVerdictV1::TargetNotIndependent,
        K2LearnedAblationKindV1::SupportProvenanceMismatch => {
            K2LearnedAblationVerdictV1::SupportEvidenceInvalid
        }
        K2LearnedAblationKindV1::TargetGoalLeakage => {
            K2LearnedAblationVerdictV1::LearnerRequestPrivateFieldRejected
        }
        K2LearnedAblationKindV1::PredictionTamper => {
            K2LearnedAblationVerdictV1::TargetPredictionRootMismatch
        }
        K2LearnedAblationKindV1::WrongActionExactOracle => {
            K2LearnedAblationVerdictV1::ExactGoalUnsatisfied
        }
        K2LearnedAblationKindV1::CrossExperimentReplay => {
            K2LearnedAblationVerdictV1::CrossExperimentReplay
        }
        K2LearnedAblationKindV1::AuthorityTamper => {
            K2LearnedAblationVerdictV1::AuthorityBoundaryViolated
        }
    }
}

fn required_ablation_kinds_v1() -> [K2LearnedAblationKindV1; 13] {
    [
        K2LearnedAblationKindV1::SupportCount,
        K2LearnedAblationKindV1::ActionIdentityShuffle,
        K2LearnedAblationKindV1::AmbiguousCopySource,
        K2LearnedAblationKindV1::ConstantOutput,
        K2LearnedAblationKindV1::OutcomeDependence,
        K2LearnedAblationKindV1::DynamicId,
        K2LearnedAblationKindV1::HoldoutAlias,
        K2LearnedAblationKindV1::SupportProvenanceMismatch,
        K2LearnedAblationKindV1::TargetGoalLeakage,
        K2LearnedAblationKindV1::PredictionTamper,
        K2LearnedAblationKindV1::WrongActionExactOracle,
        K2LearnedAblationKindV1::CrossExperimentReplay,
        K2LearnedAblationKindV1::AuthorityTamper,
    ]
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedAblationReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub controls: Vec<K2LearnedAblationControlV1>,
    pub learner_processes: u64,
    pub sandbox_probes: u64,
    pub oracle_invocations: u64,
    pub canonical_bytes: u64,
    pub all_passed: bool,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedAblationReceiptV1 {
    pub fn seal(
        freeze: &K2LearnedCapabilityFreezeV1,
        mut controls: Vec<K2LearnedAblationControlV1>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        freeze.validate_persisted_v1()?;
        controls.sort_by_key(|control| control.kind);
        for control in &controls {
            control.validate_persisted_v1()?;
        }
        let learner_processes = controls
            .iter()
            .map(|control| control.learner_processes)
            .sum();
        let sandbox_probes = controls.iter().map(|control| control.sandbox_probes).sum();
        let oracle_invocations = controls
            .iter()
            .map(|control| control.oracle_invocations)
            .sum();
        let canonical_bytes = controls.iter().try_fold(0_u64, |total, control| {
            let bytes = learned_bytes_v1(control)?.len() as u64;
            total
                .checked_add(bytes)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_ablation_bytes_overflow",
                ))
        })?;
        let mut receipt = Self {
            schema: K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            experiment_freeze_root_sha256: freeze.freeze_root_sha256.clone(),
            controls,
            learner_processes,
            sandbox_probes,
            oracle_invocations,
            canonical_bytes,
            all_passed: true,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        receipt.receipt_root_sha256 = receipt.expected_root_v1()?;
        receipt.validate_persisted_v1()?;
        Ok(receipt)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.receipt_root_sha256.as_str(),
            self.experiment_freeze_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_ablation_receipt_root_invalid")?;
        }
        for control in &self.controls {
            control.validate_persisted_v1()?;
        }
        let observed_kinds = self
            .controls
            .iter()
            .map(|control| control.kind)
            .collect::<Vec<_>>();
        let learner_processes = self
            .controls
            .iter()
            .map(|control| control.learner_processes)
            .sum::<u64>();
        let sandbox_probes = self
            .controls
            .iter()
            .map(|control| control.sandbox_probes)
            .sum::<u64>();
        let oracle_invocations = self
            .controls
            .iter()
            .map(|control| control.oracle_invocations)
            .sum::<u64>();
        let canonical_bytes = self.controls.iter().try_fold(0_u64, |total, control| {
            total
                .checked_add(learned_bytes_v1(control)?.len() as u64)
                .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_ablation_bytes_overflow",
                ))
        })?;
        if self.schema != K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1
            || observed_kinds != required_ablation_kinds_v1()
            || self.learner_processes != learner_processes
            || self.learner_processes > 8
            || self.sandbox_probes != sandbox_probes
            || self.sandbox_probes != 1
            || self.oracle_invocations != oracle_invocations
            || self.oracle_invocations != 1
            || self.canonical_bytes != canonical_bytes
            || self.canonical_bytes > 2 * 1024 * 1024
            || !self.all_passed
            || self.receipt_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_ablation_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1,
            self.experiment_freeze_root_sha256.as_str(),
            &self.controls,
            self.learner_processes,
            self.sandbox_probes,
            self.oracle_invocations,
            self.canonical_bytes,
            self.all_passed,
            &self.authority,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedCapabilityEvidenceClassV1 {
    CapabilityPass,
    LearningNegative,
    InfrastructureFailure,
    IndeterminateAfterDispatch,
}

pub struct K2LearnedCapabilityOutcomeInputV1<'a> {
    pub freeze: &'a K2LearnedCapabilityFreezeV1,
    pub dispatches: &'a [K2SupportDispatchV1],
    pub observations: &'a K2SupportObservationSetV1,
    pub learning_request: &'a K2EffectLearningRequestV1,
    pub laws: &'a K2LearnedEffectLawSetV1,
    pub independence: &'a K2TargetIndependenceReceiptV1,
    pub prediction_request: &'a K2TargetPredictionRequestV1,
    pub predictions: &'a K2LearnedTargetPredictionSetV1,
    pub verification: &'a K2LearnedEffectVerificationReceiptV1,
    pub v1_binding: &'a K2LearnedToV1BindingV1,
    pub v1_episode: &'a K2V1EpisodeEvidenceV1,
    pub ablations: &'a K2LearnedAblationReceiptV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilityOutcomeV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub experiment_freeze_root_sha256: String,
    pub support_dispatch_roots_sha256: Vec<String>,
    pub support_observation_roots_sha256: Vec<String>,
    pub support_evidence_set_root_sha256: String,
    pub learning_request_root_sha256: String,
    pub learned_law_set_root_sha256: String,
    pub target_independence_receipt_root_sha256: String,
    pub target_prediction_request_root_sha256: String,
    pub target_prediction_set_root_sha256: String,
    pub independent_verification_root_sha256: String,
    pub learned_to_v1_binding_root_sha256: String,
    pub v1_decision_freeze_root_sha256: String,
    pub v1_prediction_set_root_sha256: String,
    pub v1_selection_root_sha256: String,
    pub v1_law_lab_binding_root_sha256: String,
    pub v1_sandbox_receipt_root_sha256: String,
    pub v1_exact_goal_receipt_root_sha256: String,
    pub v1_terminal_outcome_root_sha256: String,
    pub v1_episode_seal_root_sha256: String,
    pub ablation_receipt_root_sha256: String,
    pub support_worlds: u64,
    pub support_executions: u64,
    pub learned_laws: u64,
    pub target_predictions: u64,
    pub wrong_predictions: u64,
    pub verdict: String,
    pub evidence_class: K2LearnedCapabilityEvidenceClassV1,
    pub authority: K2AuthorityBoundaryV1,
}

#[derive(Serialize)]
struct K2LearnedCapabilityOutcomeDigestV1<'a> {
    schema: &'static str,
    experiment_freeze_root_sha256: &'a str,
    support_dispatch_roots_sha256: &'a [String],
    support_observation_roots_sha256: &'a [String],
    support_evidence_set_root_sha256: &'a str,
    learning_request_root_sha256: &'a str,
    learned_law_set_root_sha256: &'a str,
    target_independence_receipt_root_sha256: &'a str,
    target_prediction_request_root_sha256: &'a str,
    target_prediction_set_root_sha256: &'a str,
    independent_verification_root_sha256: &'a str,
    learned_to_v1_binding_root_sha256: &'a str,
    v1_decision_freeze_root_sha256: &'a str,
    v1_prediction_set_root_sha256: &'a str,
    v1_selection_root_sha256: &'a str,
    v1_law_lab_binding_root_sha256: &'a str,
    v1_sandbox_receipt_root_sha256: &'a str,
    v1_exact_goal_receipt_root_sha256: &'a str,
    v1_terminal_outcome_root_sha256: &'a str,
    v1_episode_seal_root_sha256: &'a str,
    ablation_receipt_root_sha256: &'a str,
    support_worlds: u64,
    support_executions: u64,
    learned_laws: u64,
    target_predictions: u64,
    wrong_predictions: u64,
    verdict: &'a str,
    evidence_class: K2LearnedCapabilityEvidenceClassV1,
    authority: &'a K2AuthorityBoundaryV1,
}

impl K2LearnedCapabilityOutcomeV1 {
    pub fn capability_pass(
        input: K2LearnedCapabilityOutcomeInputV1<'_>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        input.freeze.validate_persisted_v1()?;
        input.observations.validate_persisted_v1()?;
        input.learning_request.validate()?;
        input.laws.validate()?;
        input.independence.validate_persisted_v1()?;
        input.prediction_request.validate()?;
        input.predictions.validate()?;
        input.verification.validate_persisted_v1()?;
        input.v1_binding.validate_persisted_v1()?;
        input.v1_episode.validate_persisted_v1()?;
        input.ablations.validate_persisted_v1()?;
        if input.dispatches.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_outcome_dispatch_count_invalid",
            ));
        }
        for (ordinal, dispatch) in input.dispatches.iter().enumerate() {
            dispatch.validate_persisted_v1()?;
            let observation = &input.observations.observations[ordinal];
            if dispatch.probe_ordinal != ordinal as u64
                || dispatch.experiment_freeze_root_sha256 != input.freeze.freeze_root_sha256
                || observation.dispatch_root_sha256 != dispatch.dispatch_root_sha256
                || observation.probe_ordinal != dispatch.probe_ordinal
                || observation.action_id_sha256 != dispatch.action_id_sha256
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_learned_outcome_support_binding_invalid",
                ));
            }
        }
        let cross_roots_valid = input
            .learning_request
            .support_observations
            .observation_set_root_sha256
            == input.observations.observation_set_root_sha256
            && input.laws.learning_request_root_sha256
                == input.learning_request.request_root_sha256
            && input.laws.support_observation_set_root_sha256
                == input.observations.observation_set_root_sha256
            && input.independence.support_set_root_sha256 == input.freeze.support_set_root_sha256
            && input.independence.target_pre_tree_root_sha256
                == input
                    .prediction_request
                    .target_pre_manifest
                    .tree_root_sha256
            && input.predictions.target_prediction_request_root_sha256
                == input.prediction_request.request_root_sha256
            && input.predictions.learned_law_set_root_sha256 == input.laws.law_set_root_sha256
            && input.verification.learned_law_set_root_sha256 == input.laws.law_set_root_sha256
            && input.verification.target_prediction_set_root_sha256
                == input.predictions.prediction_set_root_sha256
            && input.v1_binding.experiment_freeze_root_sha256 == input.freeze.freeze_root_sha256
            && input.v1_binding.learned_law_set_root_sha256 == input.laws.law_set_root_sha256
            && input.v1_binding.target_prediction_set_root_sha256
                == input.predictions.prediction_set_root_sha256
            && input.v1_binding.independent_verification_root_sha256
                == input.verification.verification_root_sha256
            && input.v1_episode.learned_to_v1_binding_root_sha256
                == input.v1_binding.binding_root_sha256
            && input.ablations.experiment_freeze_root_sha256 == input.freeze.freeze_root_sha256;
        if !cross_roots_valid {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_outcome_cross_root_invalid",
            ));
        }
        let mut outcome = Self {
            schema: K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            experiment_freeze_root_sha256: input.freeze.freeze_root_sha256.clone(),
            support_dispatch_roots_sha256: input
                .dispatches
                .iter()
                .map(|dispatch| dispatch.dispatch_root_sha256.clone())
                .collect(),
            support_observation_roots_sha256: input
                .observations
                .observations
                .iter()
                .map(|observation| observation.observation_root_sha256.clone())
                .collect(),
            support_evidence_set_root_sha256: input
                .observations
                .observation_set_root_sha256
                .clone(),
            learning_request_root_sha256: input.learning_request.request_root_sha256.clone(),
            learned_law_set_root_sha256: input.laws.law_set_root_sha256.clone(),
            target_independence_receipt_root_sha256: input.independence.receipt_root_sha256.clone(),
            target_prediction_request_root_sha256: input
                .prediction_request
                .request_root_sha256
                .clone(),
            target_prediction_set_root_sha256: input.predictions.prediction_set_root_sha256.clone(),
            independent_verification_root_sha256: input
                .verification
                .verification_root_sha256
                .clone(),
            learned_to_v1_binding_root_sha256: input.v1_binding.binding_root_sha256.clone(),
            v1_decision_freeze_root_sha256: input.v1_episode.v1_decision_freeze_root_sha256.clone(),
            v1_prediction_set_root_sha256: input.v1_episode.v1_prediction_set_root_sha256.clone(),
            v1_selection_root_sha256: input.v1_episode.v1_selection_root_sha256.clone(),
            v1_law_lab_binding_root_sha256: input.v1_episode.v1_law_lab_binding_root_sha256.clone(),
            v1_sandbox_receipt_root_sha256: input.v1_episode.v1_sandbox_receipt_root_sha256.clone(),
            v1_exact_goal_receipt_root_sha256: input
                .v1_episode
                .v1_exact_goal_receipt_root_sha256
                .clone(),
            v1_terminal_outcome_root_sha256: input
                .v1_episode
                .v1_terminal_outcome_root_sha256
                .clone(),
            v1_episode_seal_root_sha256: input.v1_episode.v1_episode_seal_root_sha256.clone(),
            ablation_receipt_root_sha256: input.ablations.receipt_root_sha256.clone(),
            support_worlds: K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64,
            support_executions: K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64,
            learned_laws: input.laws.laws.len() as u64,
            target_predictions: input.predictions.predictions.len() as u64,
            wrong_predictions: input.verification.wrong_predictions,
            verdict: K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS_V1.to_owned(),
            evidence_class: K2LearnedCapabilityEvidenceClassV1::CapabilityPass,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        outcome.outcome_root_sha256 = outcome.expected_root_v1()?;
        outcome.validate_persisted_v1()?;
        Ok(outcome)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in std::iter::once(self.outcome_root_sha256.as_str())
            .chain(std::iter::once(self.experiment_freeze_root_sha256.as_str()))
            .chain(
                self.support_dispatch_roots_sha256
                    .iter()
                    .map(String::as_str),
            )
            .chain(
                self.support_observation_roots_sha256
                    .iter()
                    .map(String::as_str),
            )
            .chain([
                self.support_evidence_set_root_sha256.as_str(),
                self.learning_request_root_sha256.as_str(),
                self.learned_law_set_root_sha256.as_str(),
                self.target_independence_receipt_root_sha256.as_str(),
                self.target_prediction_request_root_sha256.as_str(),
                self.target_prediction_set_root_sha256.as_str(),
                self.independent_verification_root_sha256.as_str(),
                self.learned_to_v1_binding_root_sha256.as_str(),
                self.v1_decision_freeze_root_sha256.as_str(),
                self.v1_prediction_set_root_sha256.as_str(),
                self.v1_selection_root_sha256.as_str(),
                self.v1_law_lab_binding_root_sha256.as_str(),
                self.v1_sandbox_receipt_root_sha256.as_str(),
                self.v1_exact_goal_receipt_root_sha256.as_str(),
                self.v1_terminal_outcome_root_sha256.as_str(),
                self.v1_episode_seal_root_sha256.as_str(),
                self.ablation_receipt_root_sha256.as_str(),
            ])
        {
            require_learned_root_v1(root, "k2_learned_outcome_root_invalid")?;
        }
        if self.schema != K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1
            || self.support_dispatch_roots_sha256.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self.support_observation_roots_sha256.len() != K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self.support_worlds != K2_LEARNED_SUPPORT_WORLD_COUNT_V1 as u64
            || self.support_executions != K2_LEARNED_SUPPORT_PROBE_COUNT_V1 as u64
            || self.learned_laws != K2_LEARNED_ACTION_COUNT_V1 as u64
            || self.target_predictions != K2_LEARNED_ACTION_COUNT_V1 as u64
            || self.wrong_predictions != 0
            || self.verdict != K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PASS_V1
            || self.evidence_class != K2LearnedCapabilityEvidenceClassV1::CapabilityPass
            || self.outcome_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_capability_outcome_invalid",
            ));
        }
        require_unique_roots_v1(
            self.support_dispatch_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_outcome_dispatch_roots_not_unique",
        )?;
        require_unique_roots_v1(
            self.support_observation_roots_sha256
                .iter()
                .map(String::as_str),
            "k2_learned_outcome_observation_roots_not_unique",
        )
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&K2LearnedCapabilityOutcomeDigestV1 {
            schema: K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1,
            experiment_freeze_root_sha256: &self.experiment_freeze_root_sha256,
            support_dispatch_roots_sha256: &self.support_dispatch_roots_sha256,
            support_observation_roots_sha256: &self.support_observation_roots_sha256,
            support_evidence_set_root_sha256: &self.support_evidence_set_root_sha256,
            learning_request_root_sha256: &self.learning_request_root_sha256,
            learned_law_set_root_sha256: &self.learned_law_set_root_sha256,
            target_independence_receipt_root_sha256: &self.target_independence_receipt_root_sha256,
            target_prediction_request_root_sha256: &self.target_prediction_request_root_sha256,
            target_prediction_set_root_sha256: &self.target_prediction_set_root_sha256,
            independent_verification_root_sha256: &self.independent_verification_root_sha256,
            learned_to_v1_binding_root_sha256: &self.learned_to_v1_binding_root_sha256,
            v1_decision_freeze_root_sha256: &self.v1_decision_freeze_root_sha256,
            v1_prediction_set_root_sha256: &self.v1_prediction_set_root_sha256,
            v1_selection_root_sha256: &self.v1_selection_root_sha256,
            v1_law_lab_binding_root_sha256: &self.v1_law_lab_binding_root_sha256,
            v1_sandbox_receipt_root_sha256: &self.v1_sandbox_receipt_root_sha256,
            v1_exact_goal_receipt_root_sha256: &self.v1_exact_goal_receipt_root_sha256,
            v1_terminal_outcome_root_sha256: &self.v1_terminal_outcome_root_sha256,
            v1_episode_seal_root_sha256: &self.v1_episode_seal_root_sha256,
            ablation_receipt_root_sha256: &self.ablation_receipt_root_sha256,
            support_worlds: self.support_worlds,
            support_executions: self.support_executions,
            learned_laws: self.learned_laws,
            target_predictions: self.target_predictions,
            wrong_predictions: self.wrong_predictions,
            verdict: &self.verdict,
            evidence_class: self.evidence_class,
            authority: &self.authority,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilitySealV1 {
    pub schema: String,
    pub seal_root_sha256: String,
    pub experiment_id_sha256: String,
    pub outcome_root_sha256: String,
    pub terminal_event_root_sha256: String,
    pub final_projection_root_sha256: String,
    pub authority: K2AuthorityBoundaryV1,
}

impl K2LearnedCapabilitySealV1 {
    pub fn derive(
        experiment_id_sha256: String,
        outcome_root_sha256: String,
        terminal_event_root_sha256: String,
        final_projection_root_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let mut seal = Self {
            schema: K2_LEARNED_CAPABILITY_SEAL_SCHEMA_V1.to_owned(),
            seal_root_sha256: String::new(),
            experiment_id_sha256,
            outcome_root_sha256,
            terminal_event_root_sha256,
            final_projection_root_sha256,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        seal.seal_root_sha256 = seal.expected_root_v1()?;
        seal.validate_persisted_v1()?;
        Ok(seal)
    }

    pub fn validate_persisted_v1(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        for root in [
            self.seal_root_sha256.as_str(),
            self.experiment_id_sha256.as_str(),
            self.outcome_root_sha256.as_str(),
            self.terminal_event_root_sha256.as_str(),
            self.final_projection_root_sha256.as_str(),
        ] {
            require_learned_root_v1(root, "k2_learned_capability_seal_root_invalid")?;
        }
        if self.schema != K2_LEARNED_CAPABILITY_SEAL_SCHEMA_V1
            || self.seal_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_capability_seal_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        learned_root_v1(&(
            K2_LEARNED_CAPABILITY_SEAL_SCHEMA_V1,
            self.experiment_id_sha256.as_str(),
            self.outcome_root_sha256.as_str(),
            self.terminal_event_root_sha256.as_str(),
            self.final_projection_root_sha256.as_str(),
            &self.authority,
        ))
    }
}
