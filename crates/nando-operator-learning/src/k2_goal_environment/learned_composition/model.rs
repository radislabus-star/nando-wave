use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub const K2_COMPOSITION_MANIFEST_SCHEMA_V1: &str = "nando.k2-composition-tree-manifest.v1";
pub const K2_COMPOSITION_OBSERVATION_SCHEMA_V1: &str =
    "nando.k2-composition-support-observation.v1";
pub const K2_COMPOSITION_LEARNING_REQUEST_SCHEMA_V1: &str =
    "nando.k2-composition-learning-request.v1";
pub const K2_COMPOSITION_LAW_SCHEMA_V1: &str = "nando.k2-composition-learned-law.v1";
pub const K2_COMPOSITION_LAW_SET_SCHEMA_V1: &str = "nando.k2-composition-law-set.v1";
pub const K2_COMPOSITION_MAPPING_SCHEMA_V1: &str = "nando.k2-composition-private-mapping.v1";
pub const K2_COMPOSITION_GOAL_SCHEMA_V1: &str = "nando.k2-composition-exact-goal.v1";
pub const K2_COMPOSITION_INDEPENDENCE_SCHEMA_V1: &str =
    "nando.k2-composition-target-independence.v1";
pub const K2_COMPOSITION_PLANNING_REQUEST_SCHEMA_V1: &str =
    "nando.k2-composition-planning-request.v1";
pub const K2_COMPOSITION_PROGRAM_SCHEMA_V1: &str = "nando.k2-composition-program.v1";
pub const K2_COMPOSITION_CANDIDATE_SCHEMA_V1: &str = "nando.k2-composition-candidate.v1";
pub const K2_COMPOSITION_CLASS_SCHEMA_V1: &str = "nando.k2-composition-semantic-class.v1";
pub const K2_COMPOSITION_PLANNER_OUTCOME_SCHEMA_V1: &str =
    "nando.k2-composition-planner-outcome.v1";
pub const K2_COMPOSITION_PLAN_VERIFICATION_SCHEMA_V1: &str =
    "nando.k2-composition-plan-verification.v1";
pub const K2_COMPOSITION_PROCESS_RECEIPT_SCHEMA_V1: &str =
    "nando.k2-composition-process-receipt.v1";
pub const K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1: &str =
    "nando.k2-composition-sequential-sandbox-request.v1";
pub const K2_COMPOSITION_SANDBOX_OUTCOME_SCHEMA_V1: &str =
    "nando.k2-composition-sequential-sandbox-outcome.v1";
pub const K2_COMPOSITION_ORACLE_REQUEST_SCHEMA_V1: &str =
    "nando.k2-composition-exact-oracle-request.v1";
pub const K2_COMPOSITION_ORACLE_OUTCOME_SCHEMA_V1: &str =
    "nando.k2-composition-exact-oracle-outcome.v1";
pub const K2_COMPOSITION_ABLATION_SCHEMA_V1: &str = "nando.k2-composition-ablations.v1";
pub const K2_COMPOSITION_OUTCOME_SCHEMA_V1: &str = "nando.k2-composition-capability-outcome.v1";
pub const K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PASS_V1: &str =
    "K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PASS";

pub const K2_COMPOSITION_ACTIONS_PER_ROUTE_V1: usize = 3;
pub const K2_COMPOSITION_SUPPORT_WORLDS_PER_ACTION_V1: usize = 3;
pub const K2_COMPOSITION_PROGRAMS_PER_ROUTE_V1: usize = 15;
pub const K2_COMPOSITION_MAX_DEPTH_V1: u64 = 3;
pub const K2_COMPOSITION_MAX_MANIFEST_ENTRIES_V1: usize = 48;
pub const K2_COMPOSITION_MAX_MANIFEST_BYTES_V1: u64 = 96 * 1024;
pub const K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1: usize = 1024 * 1024;

pub type K2CompositionResultV1<T> = Result<T, K2CompositionErrorV1>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum K2CompositionErrorV1 {
    Invalid(&'static str),
    Io(&'static str),
    Serialization,
    Process(&'static str),
}

impl Display for K2CompositionErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(reason) => write!(formatter, "k2_composition_invalid:{reason}"),
            Self::Io(reason) => write!(formatter, "k2_composition_io:{reason}"),
            Self::Serialization => formatter.write_str("k2_composition_serialization"),
            Self::Process(reason) => write!(formatter, "k2_composition_process:{reason}"),
        }
    }
}

impl std::error::Error for K2CompositionErrorV1 {}

pub fn composition_bytes_v1<T: Serialize>(value: &T) -> K2CompositionResultV1<Vec<u8>> {
    canonical_json_bytes(value).map_err(|_| K2CompositionErrorV1::Serialization)
}

pub fn composition_decode_v1<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
) -> K2CompositionResultV1<T> {
    if bytes.len() > K2_COMPOSITION_MAX_PROTOCOL_BYTES_V1 {
        return Err(K2CompositionErrorV1::Invalid("protocol_bytes_exhausted"));
    }
    let value = serde_json::from_slice(bytes)
        .map_err(|_| K2CompositionErrorV1::Invalid("protocol_decode_failed"))?;
    if composition_bytes_v1(&value)? != bytes {
        return Err(K2CompositionErrorV1::Invalid("protocol_not_canonical"));
    }
    Ok(value)
}

pub fn composition_root_v1<T: Serialize>(value: &T) -> K2CompositionResultV1<String> {
    canonical_json_sha256(value).map_err(|_| K2CompositionErrorV1::Serialization)
}

pub fn composition_sha256_bytes_v1(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn composition_sha256_file_v1(path: &Path) -> K2CompositionResultV1<String> {
    let bytes = fs::read(path).map_err(|_| K2CompositionErrorV1::Io("read_sha_file"))?;
    Ok(composition_sha256_bytes_v1(&bytes))
}

pub fn require_composition_root_v1(root: &str) -> K2CompositionResultV1<()> {
    if valid_nonzero_sha256(root) {
        Ok(())
    } else {
        Err(K2CompositionErrorV1::Invalid("root_invalid"))
    }
}

pub fn valid_composition_path_v1(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 256
        && !path.starts_with('/')
        && !path.ends_with('/')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionAuthorityBoundaryV1 {
    pub natural_k2_authority: bool,
    pub k1_registry_mutated: bool,
    pub product_authority: bool,
    pub phase_memory_mutated: bool,
    pub law_certificate_issued: bool,
    pub package_activated: bool,
    pub deployment_authority: bool,
}

impl K2CompositionAuthorityBoundaryV1 {
    #[must_use]
    pub const fn denied() -> Self {
        Self {
            natural_k2_authority: false,
            k1_registry_mutated: false,
            product_authority: false,
            phase_memory_mutated: false,
            law_certificate_issued: false,
            package_activated: false,
            deployment_authority: false,
        }
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if self == &Self::denied() {
            Ok(())
        } else {
            Err(K2CompositionErrorV1::Invalid("authority_boundary_violated"))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionFileEntryV1 {
    pub path: String,
    pub content_sha256: String,
    pub byte_len: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionTreeManifestV1 {
    pub schema: String,
    pub entries: Vec<K2CompositionFileEntryV1>,
    pub total_file_bytes: u64,
    pub tree_root_sha256: String,
}

impl K2CompositionTreeManifestV1 {
    pub fn from_files(files: &BTreeMap<String, Vec<u8>>) -> K2CompositionResultV1<Self> {
        let entries = files
            .iter()
            .map(|(path, bytes)| K2CompositionFileEntryV1 {
                path: path.clone(),
                content_sha256: composition_sha256_bytes_v1(bytes),
                byte_len: bytes.len() as u64,
            })
            .collect();
        Self::seal_entries(entries)
    }

    pub fn seal_entries(mut entries: Vec<K2CompositionFileEntryV1>) -> K2CompositionResultV1<Self> {
        entries.sort();
        let total_file_bytes = entries.iter().try_fold(0_u64, |total, entry| {
            total
                .checked_add(entry.byte_len)
                .ok_or(K2CompositionErrorV1::Invalid("manifest_bytes_overflow"))
        })?;
        let tree_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_MANIFEST_SCHEMA_V1,
            &entries,
            total_file_bytes,
        ))?;
        let manifest = Self {
            schema: K2_COMPOSITION_MANIFEST_SCHEMA_V1.to_owned(),
            entries,
            total_file_bytes,
            tree_root_sha256,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn scan(root: &Path) -> K2CompositionResultV1<Self> {
        if !root.is_dir() {
            return Err(K2CompositionErrorV1::Invalid("manifest_root_not_directory"));
        }
        let mut paths = Vec::new();
        collect_file_paths_v1(root, root, &mut paths)?;
        paths.sort();
        if paths.len() > K2_COMPOSITION_MAX_MANIFEST_ENTRIES_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "manifest_entry_budget_exhausted",
            ));
        }
        let mut entries = Vec::with_capacity(paths.len());
        for path in paths {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| K2CompositionErrorV1::Invalid("manifest_path_escape"))?;
            let relative = relative
                .to_str()
                .ok_or(K2CompositionErrorV1::Invalid("manifest_path_utf8"))?
                .replace(std::path::MAIN_SEPARATOR, "/");
            let bytes = fs::read(&path).map_err(|_| K2CompositionErrorV1::Io("read_manifest"))?;
            entries.push(K2CompositionFileEntryV1 {
                path: relative,
                content_sha256: composition_sha256_bytes_v1(&bytes),
                byte_len: bytes.len() as u64,
            });
        }
        Self::seal_entries(entries)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if self.schema != K2_COMPOSITION_MANIFEST_SCHEMA_V1
            || self.entries.len() > K2_COMPOSITION_MAX_MANIFEST_ENTRIES_V1
            || self
                .entries
                .windows(2)
                .any(|pair| pair[0].path >= pair[1].path)
            || self.entries.iter().any(|entry| {
                !valid_composition_path_v1(&entry.path)
                    || !valid_nonzero_sha256(&entry.content_sha256)
            })
            || self.total_file_bytes > K2_COMPOSITION_MAX_MANIFEST_BYTES_V1
        {
            return Err(K2CompositionErrorV1::Invalid("manifest_invalid"));
        }
        let total = self.entries.iter().try_fold(0_u64, |sum, entry| {
            sum.checked_add(entry.byte_len)
                .ok_or(K2CompositionErrorV1::Invalid("manifest_bytes_overflow"))
        })?;
        let expected =
            composition_root_v1(&(K2_COMPOSITION_MANIFEST_SCHEMA_V1, &self.entries, total))?;
        if total != self.total_file_bytes || expected != self.tree_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid("manifest_root_mismatch"));
        }
        Ok(())
    }

    #[must_use]
    pub fn entry(&self, path: &str) -> Option<&K2CompositionFileEntryV1> {
        self.entries
            .binary_search_by(|entry| entry.path.as_str().cmp(path))
            .ok()
            .map(|index| &self.entries[index])
    }

    #[must_use]
    pub fn paths(&self) -> BTreeSet<String> {
        self.entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect()
    }
}

fn collect_file_paths_v1(
    root: &Path,
    current: &Path,
    paths: &mut Vec<PathBuf>,
) -> K2CompositionResultV1<()> {
    let mut entries = fs::read_dir(current)
        .map_err(|_| K2CompositionErrorV1::Io("read_manifest_directory"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| K2CompositionErrorV1::Io("read_manifest_entry"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| K2CompositionErrorV1::Io("stat_manifest_entry"))?;
        if metadata.file_type().is_symlink() {
            return Err(K2CompositionErrorV1::Invalid("manifest_symlink_forbidden"));
        }
        if metadata.is_dir() {
            collect_file_paths_v1(root, &path, paths)?;
        } else if metadata.is_file() {
            if path.strip_prefix(root).is_err() {
                return Err(K2CompositionErrorV1::Invalid("manifest_path_escape"));
            }
            paths.push(path);
        } else {
            return Err(K2CompositionErrorV1::Invalid("manifest_entry_kind_invalid"));
        }
        if paths.len() > K2_COMPOSITION_MAX_MANIFEST_ENTRIES_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "manifest_entry_budget_exhausted",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "effect", rename_all = "snake_case", deny_unknown_fields)]
pub enum K2CompositionLearnedEffectV1 {
    CopyFile {
        source_path: String,
        target_path: String,
    },
    RemoveFile {
        path: String,
    },
}

impl K2CompositionLearnedEffectV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let valid = match self {
            Self::CopyFile {
                source_path,
                target_path,
            } => {
                valid_composition_path_v1(source_path)
                    && valid_composition_path_v1(target_path)
                    && source_path != target_path
            }
            Self::RemoveFile { path } => valid_composition_path_v1(path),
        };
        if valid {
            Ok(())
        } else {
            Err(K2CompositionErrorV1::Invalid("effect_path_invalid"))
        }
    }

    #[must_use]
    pub fn read_paths(&self) -> BTreeSet<String> {
        match self {
            Self::CopyFile { source_path, .. } => BTreeSet::from([source_path.clone()]),
            Self::RemoveFile { path } => BTreeSet::from([path.clone()]),
        }
    }

    #[must_use]
    pub fn write_paths(&self) -> BTreeSet<String> {
        match self {
            Self::CopyFile { target_path, .. } => BTreeSet::from([target_path.clone()]),
            Self::RemoveFile { path } => BTreeSet::from([path.clone()]),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionSupportObservationV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub action_id_sha256: String,
    pub support_world_root_sha256: String,
    pub before: K2CompositionTreeManifestV1,
    pub after: K2CompositionTreeManifestV1,
    pub observation_root_sha256: String,
}

impl K2CompositionSupportObservationV1 {
    pub fn seal(
        experiment_id_sha256: String,
        action_id_sha256: String,
        support_world_root_sha256: String,
        before: K2CompositionTreeManifestV1,
        after: K2CompositionTreeManifestV1,
    ) -> K2CompositionResultV1<Self> {
        let observation_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_OBSERVATION_SCHEMA_V1,
            &experiment_id_sha256,
            &action_id_sha256,
            &support_world_root_sha256,
            &before,
            &after,
        ))?;
        let value = Self {
            schema: K2_COMPOSITION_OBSERVATION_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            action_id_sha256,
            support_world_root_sha256,
            before,
            after,
            observation_root_sha256,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.before.validate()?;
        self.after.validate()?;
        for root in [
            &self.experiment_id_sha256,
            &self.action_id_sha256,
            &self.support_world_root_sha256,
            &self.observation_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        let expected = composition_root_v1(&(
            K2_COMPOSITION_OBSERVATION_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.action_id_sha256,
            &self.support_world_root_sha256,
            &self.before,
            &self.after,
        ))?;
        if self.schema != K2_COMPOSITION_OBSERVATION_SCHEMA_V1
            || expected != self.observation_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid("support_observation_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionLearningRequestV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub learner_executable_sha256: String,
    pub observations: Vec<K2CompositionSupportObservationV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2CompositionLearningRequestV1 {
    pub fn seal(
        experiment_id_sha256: String,
        learner_executable_sha256: String,
        mut observations: Vec<K2CompositionSupportObservationV1>,
    ) -> K2CompositionResultV1<Self> {
        observations.sort_by(|left, right| {
            (&left.action_id_sha256, &left.support_world_root_sha256)
                .cmp(&(&right.action_id_sha256, &right.support_world_root_sha256))
        });
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_LEARNING_REQUEST_SCHEMA_V1,
            &experiment_id_sha256,
            &learner_executable_sha256,
            &observations,
            &authority,
        ))?;
        let request = Self {
            schema: K2_COMPOSITION_LEARNING_REQUEST_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            learner_executable_sha256,
            observations,
            authority,
            request_root_sha256,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        require_composition_root_v1(&self.learner_executable_sha256)?;
        self.authority.validate()?;
        if self.schema != K2_COMPOSITION_LEARNING_REQUEST_SCHEMA_V1
            || self.observations.is_empty()
            || self.observations.len()
                > K2_COMPOSITION_ACTIONS_PER_ROUTE_V1 * K2_COMPOSITION_SUPPORT_WORLDS_PER_ACTION_V1
        {
            return Err(K2CompositionErrorV1::Invalid("learning_request_invalid"));
        }
        let mut counts = BTreeMap::new();
        let mut worlds = BTreeSet::new();
        for observation in &self.observations {
            observation.validate()?;
            if observation.experiment_id_sha256 != self.experiment_id_sha256
                || !worlds.insert(observation.support_world_root_sha256.clone())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "learning_observation_identity_invalid",
                ));
            }
            *counts
                .entry(&observation.action_id_sha256)
                .or_insert(0_usize) += 1;
        }
        if counts.len() != K2_COMPOSITION_ACTIONS_PER_ROUTE_V1
            || counts
                .values()
                .any(|count| *count > K2_COMPOSITION_SUPPORT_WORLDS_PER_ACTION_V1)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "learning_support_denominator_invalid",
            ));
        }
        let expected = composition_root_v1(&(
            K2_COMPOSITION_LEARNING_REQUEST_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.learner_executable_sha256,
            &self.observations,
            &self.authority,
        ))?;
        if expected != self.request_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "learning_request_root_mismatch",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionLearnedLawV1 {
    pub schema: String,
    pub action_id_sha256: String,
    pub effect: K2CompositionLearnedEffectV1,
    pub support_observation_roots_sha256: Vec<String>,
    pub enumerated_candidate_count: u64,
    pub rejected_candidate_count: u64,
    pub version_space_size: u64,
    pub law_root_sha256: String,
}

impl K2CompositionLearnedLawV1 {
    pub fn seal(
        action_id_sha256: String,
        effect: K2CompositionLearnedEffectV1,
        mut support_observation_roots_sha256: Vec<String>,
        enumerated_candidate_count: u64,
        rejected_candidate_count: u64,
    ) -> K2CompositionResultV1<Self> {
        support_observation_roots_sha256.sort();
        let version_space_size = 1;
        let law_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_LAW_SCHEMA_V1,
            &action_id_sha256,
            &effect,
            &support_observation_roots_sha256,
            enumerated_candidate_count,
            rejected_candidate_count,
            version_space_size,
        ))?;
        let law = Self {
            schema: K2_COMPOSITION_LAW_SCHEMA_V1.to_owned(),
            action_id_sha256,
            effect,
            support_observation_roots_sha256,
            enumerated_candidate_count,
            rejected_candidate_count,
            version_space_size,
            law_root_sha256,
        };
        law.validate()?;
        Ok(law)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.action_id_sha256)?;
        self.effect.validate()?;
        if self.schema != K2_COMPOSITION_LAW_SCHEMA_V1
            || self.support_observation_roots_sha256.len()
                != K2_COMPOSITION_SUPPORT_WORLDS_PER_ACTION_V1
            || self
                .support_observation_roots_sha256
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.version_space_size != 1
            || self.enumerated_candidate_count
                != self
                    .rejected_candidate_count
                    .saturating_add(self.version_space_size)
        {
            return Err(K2CompositionErrorV1::Invalid("learned_law_invalid"));
        }
        let expected = composition_root_v1(&(
            K2_COMPOSITION_LAW_SCHEMA_V1,
            &self.action_id_sha256,
            &self.effect,
            &self.support_observation_roots_sha256,
            self.enumerated_candidate_count,
            self.rejected_candidate_count,
            self.version_space_size,
        ))?;
        if expected != self.law_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid("learned_law_root_mismatch"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionLearnedLawSetV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub learning_request_root_sha256: String,
    pub laws: Vec<K2CompositionLearnedLawV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub law_set_root_sha256: String,
}

impl K2CompositionLearnedLawSetV1 {
    pub fn seal(
        experiment_id_sha256: String,
        learning_request_root_sha256: String,
        mut laws: Vec<K2CompositionLearnedLawV1>,
    ) -> K2CompositionResultV1<Self> {
        laws.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let law_set_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_LAW_SET_SCHEMA_V1,
            &experiment_id_sha256,
            &learning_request_root_sha256,
            &laws,
            &authority,
        ))?;
        let set = Self {
            schema: K2_COMPOSITION_LAW_SET_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            learning_request_root_sha256,
            laws,
            authority,
            law_set_root_sha256,
        };
        set.validate()?;
        Ok(set)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.authority.validate()?;
        if self.schema != K2_COMPOSITION_LAW_SET_SCHEMA_V1
            || self.laws.len() != K2_COMPOSITION_ACTIONS_PER_ROUTE_V1
            || self
                .laws
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid("law_set_invalid"));
        }
        for law in &self.laws {
            law.validate()?;
        }
        let expected = composition_root_v1(&(
            K2_COMPOSITION_LAW_SET_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.learning_request_root_sha256,
            &self.laws,
            &self.authority,
        ))?;
        if expected != self.law_set_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid("law_set_root_mismatch"));
        }
        Ok(())
    }

    #[must_use]
    pub fn law(&self, action_id: &str) -> Option<&K2CompositionLearnedLawV1> {
        self.laws
            .binary_search_by(|law| law.action_id_sha256.as_str().cmp(action_id))
            .ok()
            .map(|index| &self.laws[index])
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionMappingEntryV1 {
    pub action_id_sha256: String,
    pub effect: K2CompositionLearnedEffectV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionPrivateMappingV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub entries: Vec<K2CompositionMappingEntryV1>,
    pub mapping_root_sha256: String,
}

impl K2CompositionPrivateMappingV1 {
    pub fn seal(
        experiment_id_sha256: String,
        mut entries: Vec<K2CompositionMappingEntryV1>,
    ) -> K2CompositionResultV1<Self> {
        entries.sort_by(|left, right| left.action_id_sha256.cmp(&right.action_id_sha256));
        let mapping_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_MAPPING_SCHEMA_V1,
            &experiment_id_sha256,
            &entries,
        ))?;
        let mapping = Self {
            schema: K2_COMPOSITION_MAPPING_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            entries,
            mapping_root_sha256,
        };
        mapping.validate()?;
        Ok(mapping)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if self.schema != K2_COMPOSITION_MAPPING_SCHEMA_V1
            || self.entries.len() != K2_COMPOSITION_ACTIONS_PER_ROUTE_V1
            || self
                .entries
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid("private_mapping_invalid"));
        }
        for entry in &self.entries {
            require_composition_root_v1(&entry.action_id_sha256)?;
            entry.effect.validate()?;
        }
        let expected = composition_root_v1(&(
            K2_COMPOSITION_MAPPING_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.entries,
        ))?;
        if expected != self.mapping_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "private_mapping_root_mismatch",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn effect(&self, action_id: &str) -> Option<&K2CompositionLearnedEffectV1> {
        self.entries
            .binary_search_by(|entry| entry.action_id_sha256.as_str().cmp(action_id))
            .ok()
            .map(|index| &self.entries[index].effect)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionExactGoalV1 {
    pub schema: String,
    pub expected_terminal: K2CompositionTreeManifestV1,
    pub goal_root_sha256: String,
}

impl K2CompositionExactGoalV1 {
    pub fn seal(expected_terminal: K2CompositionTreeManifestV1) -> K2CompositionResultV1<Self> {
        let goal_root_sha256 =
            composition_root_v1(&(K2_COMPOSITION_GOAL_SCHEMA_V1, &expected_terminal))?;
        Ok(Self {
            schema: K2_COMPOSITION_GOAL_SCHEMA_V1.to_owned(),
            expected_terminal,
            goal_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.expected_terminal.validate()?;
        let expected =
            composition_root_v1(&(K2_COMPOSITION_GOAL_SCHEMA_V1, &self.expected_terminal))?;
        if self.schema != K2_COMPOSITION_GOAL_SCHEMA_V1 || expected != self.goal_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid("exact_goal_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionTargetIndependenceReceiptV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub learning_request_root_sha256: String,
    pub target_tree_root_sha256: String,
    pub target_content_disjoint: bool,
    pub target_root_disjoint: bool,
    pub target_bytes_absent_from_learning_request: bool,
    pub receipt_root_sha256: String,
}

impl K2CompositionTargetIndependenceReceiptV1 {
    pub fn seal(
        experiment_id_sha256: String,
        learning_request_root_sha256: String,
        target_tree_root_sha256: String,
        target_content_disjoint: bool,
        target_root_disjoint: bool,
        target_bytes_absent_from_learning_request: bool,
    ) -> K2CompositionResultV1<Self> {
        let receipt_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_INDEPENDENCE_SCHEMA_V1,
            &experiment_id_sha256,
            &learning_request_root_sha256,
            &target_tree_root_sha256,
            target_content_disjoint,
            target_root_disjoint,
            target_bytes_absent_from_learning_request,
        ))?;
        let receipt = Self {
            schema: K2_COMPOSITION_INDEPENDENCE_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            learning_request_root_sha256,
            target_tree_root_sha256,
            target_content_disjoint,
            target_root_disjoint,
            target_bytes_absent_from_learning_request,
            receipt_root_sha256,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if self.schema != K2_COMPOSITION_INDEPENDENCE_SCHEMA_V1
            || !self.target_content_disjoint
            || !self.target_root_disjoint
            || !self.target_bytes_absent_from_learning_request
        {
            return Err(K2CompositionErrorV1::Invalid("target_independence_invalid"));
        }
        let expected = composition_root_v1(&(
            K2_COMPOSITION_INDEPENDENCE_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.learning_request_root_sha256,
            &self.target_tree_root_sha256,
            self.target_content_disjoint,
            self.target_root_disjoint,
            self.target_bytes_absent_from_learning_request,
        ))?;
        if expected != self.receipt_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "target_independence_root_mismatch",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionPlanningRequestV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub planner_executable_sha256: String,
    pub law_set: K2CompositionLearnedLawSetV1,
    pub target: K2CompositionTreeManifestV1,
    pub goal: K2CompositionExactGoalV1,
    pub independence_receipt_root_sha256: String,
    pub maximum_depth: u64,
    pub each_action_at_most_once: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2CompositionPlanningRequestV1 {
    pub fn seal(
        experiment_id_sha256: String,
        planner_executable_sha256: String,
        law_set: K2CompositionLearnedLawSetV1,
        target: K2CompositionTreeManifestV1,
        goal: K2CompositionExactGoalV1,
        independence_receipt_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let maximum_depth = K2_COMPOSITION_MAX_DEPTH_V1;
        let each_action_at_most_once = true;
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_PLANNING_REQUEST_SCHEMA_V1,
            &experiment_id_sha256,
            &planner_executable_sha256,
            &law_set,
            &target,
            &goal,
            &independence_receipt_root_sha256,
            maximum_depth,
            each_action_at_most_once,
            &authority,
        ))?;
        let request = Self {
            schema: K2_COMPOSITION_PLANNING_REQUEST_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            planner_executable_sha256,
            law_set,
            target,
            goal,
            independence_receipt_root_sha256,
            maximum_depth,
            each_action_at_most_once,
            authority,
            request_root_sha256,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.law_set.validate()?;
        self.target.validate()?;
        self.goal.validate()?;
        self.authority.validate()?;
        if self.schema != K2_COMPOSITION_PLANNING_REQUEST_SCHEMA_V1
            || self.experiment_id_sha256 != self.law_set.experiment_id_sha256
            || self.maximum_depth != K2_COMPOSITION_MAX_DEPTH_V1
            || !self.each_action_at_most_once
        {
            return Err(K2CompositionErrorV1::Invalid("planning_request_invalid"));
        }
        let expected = composition_root_v1(&(
            K2_COMPOSITION_PLANNING_REQUEST_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.planner_executable_sha256,
            &self.law_set,
            &self.target,
            &self.goal,
            &self.independence_receipt_root_sha256,
            self.maximum_depth,
            self.each_action_at_most_once,
            &self.authority,
        ))?;
        if expected != self.request_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "planning_request_root_mismatch",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionProgramV1 {
    pub schema: String,
    pub action_ids_sha256: Vec<String>,
    pub program_root_sha256: String,
}

impl K2CompositionProgramV1 {
    pub fn seal(action_ids_sha256: Vec<String>) -> K2CompositionResultV1<Self> {
        let program_root_sha256 =
            composition_root_v1(&(K2_COMPOSITION_PROGRAM_SCHEMA_V1, &action_ids_sha256))?;
        Ok(Self {
            schema: K2_COMPOSITION_PROGRAM_SCHEMA_V1.to_owned(),
            action_ids_sha256,
            program_root_sha256,
        })
    }

    #[must_use]
    pub fn depth(&self) -> u64 {
        self.action_ids_sha256.len() as u64
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub enum K2CompositionProgramDispositionV1 {
    ValidPrediction {
        terminal: K2CompositionTreeManifestV1,
        exact_goal_satisfied: bool,
    },
    InapplicableAtStep {
        step: u64,
        reason: String,
    },
    BudgetRejected {
        reason: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionCandidateV1 {
    pub schema: String,
    pub program: K2CompositionProgramV1,
    pub disposition: K2CompositionProgramDispositionV1,
    pub candidate_root_sha256: String,
}

impl K2CompositionCandidateV1 {
    pub fn seal(
        program: K2CompositionProgramV1,
        disposition: K2CompositionProgramDispositionV1,
    ) -> K2CompositionResultV1<Self> {
        let candidate_root_sha256 =
            composition_root_v1(&(K2_COMPOSITION_CANDIDATE_SCHEMA_V1, &program, &disposition))?;
        Ok(Self {
            schema: K2_COMPOSITION_CANDIDATE_SCHEMA_V1.to_owned(),
            program,
            disposition,
            candidate_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionSemanticClassV1 {
    pub schema: String,
    pub depth: u64,
    pub action_multiset_sha256: Vec<String>,
    pub terminal_tree_root_sha256: String,
    pub member_program_roots_sha256: Vec<String>,
    pub exact_goal_satisfied: bool,
    pub class_root_sha256: String,
}

impl K2CompositionSemanticClassV1 {
    pub fn seal(
        depth: u64,
        mut action_multiset_sha256: Vec<String>,
        terminal_tree_root_sha256: String,
        mut member_program_roots_sha256: Vec<String>,
        exact_goal_satisfied: bool,
    ) -> K2CompositionResultV1<Self> {
        action_multiset_sha256.sort();
        member_program_roots_sha256.sort();
        let class_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_CLASS_SCHEMA_V1,
            depth,
            &action_multiset_sha256,
            &terminal_tree_root_sha256,
            &member_program_roots_sha256,
            exact_goal_satisfied,
        ))?;
        Ok(Self {
            schema: K2_COMPOSITION_CLASS_SCHEMA_V1.to_owned(),
            depth,
            action_multiset_sha256,
            terminal_tree_root_sha256,
            member_program_roots_sha256,
            exact_goal_satisfied,
            class_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionDependencyEdgeV1 {
    pub writer_action_id_sha256: String,
    pub reader_action_id_sha256: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionPlannerOutcomeV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub request_root_sha256: String,
    pub candidates: Vec<K2CompositionCandidateV1>,
    pub semantic_classes: Vec<K2CompositionSemanticClassV1>,
    pub dependency_edges: Vec<K2CompositionDependencyEdgeV1>,
    pub normalized_topology_root_sha256: String,
    pub selected_class_root_sha256: String,
    pub selected_program: K2CompositionProgramV1,
    pub valid_programs: u64,
    pub inapplicable_programs: u64,
    pub budget_rejected_programs: u64,
    pub minimum_satisfying_depth: u64,
    pub satisfying_strict_prefixes: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

impl K2CompositionPlannerOutcomeV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.outcome_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_PLANNER_OUTCOME_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.request_root_sha256,
            &self.candidates,
            &self.semantic_classes,
            &self.dependency_edges,
            &self.normalized_topology_root_sha256,
            &self.selected_class_root_sha256,
            &self.selected_program,
            self.valid_programs,
            self.inapplicable_programs,
            self.budget_rejected_programs,
            self.minimum_satisfying_depth,
            self.satisfying_strict_prefixes,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionPlanVerificationReceiptV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub planner_outcome_root_sha256: String,
    pub independently_verified_candidates: u64,
    pub independently_verified_classes: u64,
    pub minimum_depth_verified: bool,
    pub strict_prefixes_verified: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub verification_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionProcessReceiptV1 {
    pub schema: String,
    pub role: String,
    pub executable_sha256: String,
    pub request_root_sha256: String,
    pub outcome_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2CompositionProcessReceiptV1 {
    pub fn seal(
        role: &str,
        executable_sha256: String,
        request_root_sha256: String,
        outcome_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let receipt_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_PROCESS_RECEIPT_SCHEMA_V1,
            role,
            &executable_sha256,
            &request_root_sha256,
            &outcome_root_sha256,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_COMPOSITION_PROCESS_RECEIPT_SCHEMA_V1.to_owned(),
            role: role.to_owned(),
            executable_sha256,
            request_root_sha256,
            outcome_root_sha256,
            authority,
            receipt_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionSandboxRequestV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub worker_executable_sha256: String,
    pub initial_manifest: K2CompositionTreeManifestV1,
    pub operations: Vec<K2CompositionLearnedEffectV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2CompositionSandboxRequestV1 {
    pub fn seal(
        experiment_id_sha256: String,
        worker_executable_sha256: String,
        initial_manifest: K2CompositionTreeManifestV1,
        operations: Vec<K2CompositionLearnedEffectV1>,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1,
            &experiment_id_sha256,
            &worker_executable_sha256,
            &initial_manifest,
            &operations,
            &authority,
        ))?;
        let request = Self {
            schema: K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            worker_executable_sha256,
            initial_manifest,
            operations,
            authority,
            request_root_sha256,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.initial_manifest.validate()?;
        self.authority.validate()?;
        if self.schema != K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1
            || self.operations.is_empty()
            || self.operations.len() > K2_COMPOSITION_ACTIONS_PER_ROUTE_V1
        {
            return Err(K2CompositionErrorV1::Invalid("sandbox_request_invalid"));
        }
        for operation in &self.operations {
            operation.validate()?;
        }
        let expected = composition_root_v1(&(
            K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.worker_executable_sha256,
            &self.initial_manifest,
            &self.operations,
            &self.authority,
        ))?;
        if expected != self.request_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "sandbox_request_root_mismatch",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionOperationResultV1 {
    pub step: u64,
    pub applied: bool,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionSandboxOutcomeV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub worker_executable_sha256: String,
    pub pre_manifest: K2CompositionTreeManifestV1,
    pub post_manifest: K2CompositionTreeManifestV1,
    pub operation_results: Vec<K2CompositionOperationResultV1>,
    pub success: bool,
    pub failed_step: Option<u64>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

impl K2CompositionSandboxOutcomeV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.outcome_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_SANDBOX_OUTCOME_SCHEMA_V1,
            &self.request_root_sha256,
            &self.worker_executable_sha256,
            &self.pre_manifest,
            &self.post_manifest,
            &self.operation_results,
            self.success,
            self.failed_step,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionOracleRequestV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub observed_terminal: K2CompositionTreeManifestV1,
    pub goal: K2CompositionExactGoalV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2CompositionOracleRequestV1 {
    pub fn seal(
        experiment_id_sha256: String,
        observed_terminal: K2CompositionTreeManifestV1,
        goal: K2CompositionExactGoalV1,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_ORACLE_REQUEST_SCHEMA_V1,
            &experiment_id_sha256,
            &observed_terminal,
            &goal,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_COMPOSITION_ORACLE_REQUEST_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            observed_terminal,
            goal,
            authority,
            request_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionOracleOutcomeV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub exact_goal_satisfied: bool,
    pub observed_tree_root_sha256: String,
    pub expected_tree_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionControlResultV1 {
    pub control_id: u64,
    pub name: String,
    pub expected_verdict: String,
    pub observed_verdict: String,
    pub passed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionAblationReceiptV1 {
    pub schema: String,
    pub controls: Vec<K2CompositionControlResultV1>,
    pub passed: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2CompositionAblationReceiptV1 {
    pub fn seal(controls: Vec<K2CompositionControlResultV1>) -> K2CompositionResultV1<Self> {
        let passed = controls.iter().filter(|control| control.passed).count() as u64;
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let receipt_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_ABLATION_SCHEMA_V1,
            &controls,
            passed,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_COMPOSITION_ABLATION_SCHEMA_V1.to_owned(),
            controls,
            passed,
            authority,
            receipt_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2CompositionCapabilityOutcomeV1 {
    pub schema: String,
    pub verdict: String,
    pub route_outcome_roots_sha256: Vec<String>,
    pub route_verification_roots_sha256: Vec<String>,
    pub ablation_receipt_root_sha256: String,
    pub support_executions: u64,
    pub learned_laws: u64,
    pub candidate_programs: u64,
    pub verified_candidates: u64,
    pub target_executions: u64,
    pub exact_oracles: u64,
    pub journal_events_per_route: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

impl K2CompositionCapabilityOutcomeV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.outcome_root_sha256 = composition_root_v1(&(
            K2_COMPOSITION_OUTCOME_SCHEMA_V1,
            &self.verdict,
            &self.route_outcome_roots_sha256,
            &self.route_verification_roots_sha256,
            &self.ablation_receipt_root_sha256,
            self.support_executions,
            self.learned_laws,
            self.candidate_programs,
            self.verified_candidates,
            self.target_executions,
            self.exact_oracles,
            self.journal_events_per_route,
            &self.authority,
        ))?;
        Ok(())
    }
}

pub fn opaque_action_id_v1(
    experiment_seed_sha256: &str,
    slot: u64,
) -> K2CompositionResultV1<String> {
    require_composition_root_v1(experiment_seed_sha256)?;
    composition_root_v1(&(
        "nando.k2-composition-opaque-action.v1",
        experiment_seed_sha256,
        slot,
    ))
}
