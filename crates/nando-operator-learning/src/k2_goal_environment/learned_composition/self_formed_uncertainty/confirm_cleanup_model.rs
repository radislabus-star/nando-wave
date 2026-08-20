use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, require_composition_root_v1};
use super::{
    K2_UNCERTAINTY_CLEANUP_ENTRY_SCHEMA_V1, K2_UNCERTAINTY_CLEANUP_MAX_PATH_BYTES_V1,
    uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum K2UncertaintyRetentionClassV1 {
    RetainAlways,
    RetainSealedUntilPostResultReview,
    DeleteAfterTerminalAndObserverFsync,
    SupersededNeverUse,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyCleanupArtifactKindV1 {
    RetainedEvidence,
    SealedPrivateEvidence,
    DisposableWorkspace,
    SupersededEvidence,
}

impl K2UncertaintyCleanupArtifactKindV1 {
    pub const fn retention(self) -> K2UncertaintyRetentionClassV1 {
        match self {
            Self::RetainedEvidence => K2UncertaintyRetentionClassV1::RetainAlways,
            Self::SealedPrivateEvidence => {
                K2UncertaintyRetentionClassV1::RetainSealedUntilPostResultReview
            }
            Self::DisposableWorkspace => {
                K2UncertaintyRetentionClassV1::DeleteAfterTerminalAndObserverFsync
            }
            Self::SupersededEvidence => K2UncertaintyRetentionClassV1::SupersededNeverUse,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyCleanupFileKindV1 {
    Regular,
    Directory,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupRegistryEntryV1 {
    pub relative_path: String,
    pub artifact_kind: K2UncertaintyCleanupArtifactKindV1,
    pub producer_executable_sha256: String,
    pub producing_journal_event_root_sha256: String,
}

impl K2UncertaintyCleanupRegistryEntryV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        validate_cleanup_relative_path_v1(&self.relative_path)?;
        require_composition_root_v1(&self.producer_executable_sha256)?;
        require_composition_root_v1(&self.producing_journal_event_root_sha256)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupClassifiedPathV1 {
    pub schema: String,
    pub relative_path: String,
    pub file_kind: K2UncertaintyCleanupFileKindV1,
    pub artifact_kind: K2UncertaintyCleanupArtifactKindV1,
    pub retention: K2UncertaintyRetentionClassV1,
    pub content_sha256: Option<String>,
    pub mode: u32,
    pub size_bytes: u64,
    pub producer_executable_sha256: String,
    pub producing_journal_event_root_sha256: String,
    pub artifact_registry_root_sha256: String,
    pub classification_policy_root_sha256: String,
    pub entry_root_sha256: String,
}

impl K2UncertaintyCleanupClassifiedPathV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        registry: &K2UncertaintyCleanupRegistryEntryV1,
        file_kind: K2UncertaintyCleanupFileKindV1,
        content_sha256: Option<String>,
        mode: u32,
        size_bytes: u64,
        artifact_registry_root_sha256: String,
        classification_policy_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLEANUP_ENTRY_SCHEMA_V1.to_owned(),
            relative_path: registry.relative_path.clone(),
            file_kind,
            artifact_kind: registry.artifact_kind,
            retention: registry.artifact_kind.retention(),
            content_sha256,
            mode,
            size_bytes,
            producer_executable_sha256: registry.producer_executable_sha256.clone(),
            producing_journal_event_root_sha256: registry
                .producing_journal_event_root_sha256
                .clone(),
            artifact_registry_root_sha256,
            classification_policy_root_sha256,
            entry_root_sha256: String::new(),
        };
        value.entry_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        validate_cleanup_relative_path_v1(&self.relative_path)?;
        for root in [
            self.content_sha256.as_ref(),
            Some(&self.producer_executable_sha256),
            Some(&self.producing_journal_event_root_sha256),
            Some(&self.artifact_registry_root_sha256),
            Some(&self.classification_policy_root_sha256),
        ]
        .into_iter()
        .flatten()
        {
            require_composition_root_v1(root)?;
        }
        let content_shape_valid = match self.file_kind {
            K2UncertaintyCleanupFileKindV1::Regular => self.content_sha256.is_some(),
            K2UncertaintyCleanupFileKindV1::Directory => {
                self.content_sha256.is_none() && self.size_bytes == 0
            }
        };
        if self.schema != K2_UNCERTAINTY_CLEANUP_ENTRY_SCHEMA_V1
            || self.retention != self.artifact_kind.retention()
            || !content_shape_valid
            || self.entry_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_entry_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLEANUP_ENTRY_SCHEMA_V1,
            &self.relative_path,
            self.file_kind,
            self.artifact_kind,
            self.retention,
            &self.content_sha256,
            self.mode,
            self.size_bytes,
            &self.producer_executable_sha256,
            &self.producing_journal_event_root_sha256,
            &self.artifact_registry_root_sha256,
            &self.classification_policy_root_sha256,
        ))
    }
}

pub(crate) fn cleanup_policy_root_v1() -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&(
        "nando.k2-self-formed-cleanup-classification-policy.v1",
        [
            (
                K2UncertaintyCleanupArtifactKindV1::RetainedEvidence,
                K2UncertaintyRetentionClassV1::RetainAlways,
            ),
            (
                K2UncertaintyCleanupArtifactKindV1::SealedPrivateEvidence,
                K2UncertaintyRetentionClassV1::RetainSealedUntilPostResultReview,
            ),
            (
                K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace,
                K2UncertaintyRetentionClassV1::DeleteAfterTerminalAndObserverFsync,
            ),
            (
                K2UncertaintyCleanupArtifactKindV1::SupersededEvidence,
                K2UncertaintyRetentionClassV1::SupersededNeverUse,
            ),
        ],
    ))
}

pub(crate) fn cleanup_registry_root_v1(
    entries: &[K2UncertaintyCleanupRegistryEntryV1],
) -> K2CompositionResultV1<String> {
    let mut canonical = entries.to_vec();
    canonical.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    uncertainty_root_v1(&("nando.k2-self-formed-cleanup-registry.v1", canonical))
}

pub(crate) fn validate_cleanup_relative_path_v1(path: &str) -> K2CompositionResultV1<()> {
    let candidate = Path::new(path);
    if path.is_empty()
        || path.len() > K2_UNCERTAINTY_CLEANUP_MAX_PATH_BYTES_V1
        || candidate.is_absolute()
        || path.ends_with('/')
        || candidate.components().any(|component| {
            !matches!(component, Component::Normal(_))
                || component
                    .as_os_str()
                    .as_encoded_bytes()
                    .iter()
                    .any(|byte| *byte == 0 || byte.is_ascii_control())
        })
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_relative_path_invalid",
        ));
    }
    Ok(())
}
