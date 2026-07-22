use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    ExecutableProtocolModeArtifactV3, canonical_json_bytes, canonical_json_sha256,
    valid_nonzero_sha256, validate_executable_protocol_mode_artifact_v3,
};

pub const OPERATOR_GENERATION_MANIFEST_SCHEMA_V3: &str = "nando.operator-generation-manifest.v3.f7";
pub const OPERATOR_GENERATION_MANIFEST_MAX_BYTES_V3: usize = 8 * 1024;
pub const OPERATOR_GENERATION_MAX_ARTIFACTS_V3: usize = 32;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationEvidencePartitionV3 {
    Support,
    Future,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OperatorGenerationComponentRootsV3 {
    pub artifact_set_sha256: String,
    pub dispatch_index_sha256: String,
    pub actor_program_sha256: String,
    pub renderer_program_sha256: String,
    pub verifier_contract_sha256: String,
    pub capability_contract_sha256: String,
    pub resource_budget_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// The sole generation identity. Downstream layers may validate this manifest,
/// but they must not derive a competing generation key.
pub struct OperatorGenerationManifestV3 {
    sequence: u64,
    parent_generation_id_sha256: Option<String>,
    components: OperatorGenerationComponentRootsV3,
    generation_id_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorGenerationErrorV3 {
    InvalidSequence,
    InvalidParent,
    InvalidComponent,
    EmptyArtifactSet,
    ArtifactBudgetExhausted,
    DuplicateArtifact,
    InvalidArtifact,
    InvalidManifest,
    ManifestBudgetExhausted,
    Serialization,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OperatorGenerationManifestWireV3 {
    schema: String,
    sequence: u64,
    parent_generation_id_sha256: Option<String>,
    components: OperatorGenerationComponentRootsV3,
    generation_id_sha256: String,
}

pub fn seal_operator_generation_manifest_v3(
    sequence: u64,
    parent_generation_id_sha256: Option<String>,
    components: OperatorGenerationComponentRootsV3,
) -> Result<OperatorGenerationManifestV3, OperatorGenerationErrorV3> {
    validate_lineage(sequence, parent_generation_id_sha256.as_deref())?;
    validate_components(&components)?;
    let generation_id_sha256 = generation_digest(
        sequence,
        parent_generation_id_sha256.as_deref(),
        &components,
    )?;
    Ok(OperatorGenerationManifestV3 {
        sequence,
        parent_generation_id_sha256,
        components,
        generation_id_sha256,
    })
}

pub fn executable_artifact_set_sha256_v3(
    artifacts: &[ExecutableProtocolModeArtifactV3],
) -> Result<String, OperatorGenerationErrorV3> {
    if artifacts.is_empty() {
        return Err(OperatorGenerationErrorV3::EmptyArtifactSet);
    }
    if artifacts.len() > OPERATOR_GENERATION_MAX_ARTIFACTS_V3 {
        return Err(OperatorGenerationErrorV3::ArtifactBudgetExhausted);
    }
    let mut roots = BTreeSet::new();
    for artifact in artifacts {
        validate_executable_protocol_mode_artifact_v3(artifact)
            .map_err(|_| OperatorGenerationErrorV3::InvalidArtifact)?;
        if !roots.insert(artifact.artifact_sha256()) {
            return Err(OperatorGenerationErrorV3::DuplicateArtifact);
        }
    }
    canonical_json_sha256(&("nando.f6.artifact-set.v3", roots))
        .map_err(|_| OperatorGenerationErrorV3::Serialization)
}

impl OperatorGenerationManifestV3 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, OperatorGenerationErrorV3> {
        canonical_json_bytes(&OperatorGenerationManifestWireV3 {
            schema: OPERATOR_GENERATION_MANIFEST_SCHEMA_V3.to_owned(),
            sequence: self.sequence,
            parent_generation_id_sha256: self.parent_generation_id_sha256.clone(),
            components: self.components.clone(),
            generation_id_sha256: self.generation_id_sha256.clone(),
        })
        .map_err(|_| OperatorGenerationErrorV3::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, OperatorGenerationErrorV3> {
        if bytes.len() > OPERATOR_GENERATION_MANIFEST_MAX_BYTES_V3 {
            return Err(OperatorGenerationErrorV3::ManifestBudgetExhausted);
        }
        let wire: OperatorGenerationManifestWireV3 = serde_json::from_slice(bytes)
            .map_err(|_| OperatorGenerationErrorV3::InvalidManifest)?;
        if wire.schema != OPERATOR_GENERATION_MANIFEST_SCHEMA_V3 {
            return Err(OperatorGenerationErrorV3::InvalidManifest);
        }
        let manifest = seal_operator_generation_manifest_v3(
            wire.sequence,
            wire.parent_generation_id_sha256,
            wire.components,
        )?;
        if manifest.generation_id_sha256 != wire.generation_id_sha256
            || manifest.canonical_bytes()? != bytes
        {
            return Err(OperatorGenerationErrorV3::InvalidManifest);
        }
        Ok(manifest)
    }

    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn parent_generation_id_sha256(&self) -> Option<&str> {
        self.parent_generation_id_sha256.as_deref()
    }

    #[must_use]
    pub const fn components(&self) -> &OperatorGenerationComponentRootsV3 {
        &self.components
    }

    #[must_use]
    pub fn generation_id_sha256(&self) -> &str {
        &self.generation_id_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn validate_lineage(sequence: u64, parent: Option<&str>) -> Result<(), OperatorGenerationErrorV3> {
    if sequence == 0 {
        return Err(OperatorGenerationErrorV3::InvalidSequence);
    }
    match (sequence, parent) {
        (1, None) => Ok(()),
        (1, Some(_)) | (_, None) => Err(OperatorGenerationErrorV3::InvalidParent),
        (_, Some(root)) if valid_nonzero_sha256(root) => Ok(()),
        _ => Err(OperatorGenerationErrorV3::InvalidParent),
    }
}

fn validate_components(
    components: &OperatorGenerationComponentRootsV3,
) -> Result<(), OperatorGenerationErrorV3> {
    let roots = [
        components.artifact_set_sha256.as_str(),
        components.dispatch_index_sha256.as_str(),
        components.actor_program_sha256.as_str(),
        components.renderer_program_sha256.as_str(),
        components.verifier_contract_sha256.as_str(),
        components.capability_contract_sha256.as_str(),
        components.resource_budget_sha256.as_str(),
    ];
    roots
        .iter()
        .all(|root| valid_nonzero_sha256(root))
        .then_some(())
        .ok_or(OperatorGenerationErrorV3::InvalidComponent)
}

fn generation_digest(
    sequence: u64,
    parent: Option<&str>,
    components: &OperatorGenerationComponentRootsV3,
) -> Result<String, OperatorGenerationErrorV3> {
    canonical_json_sha256(&(
        OPERATOR_GENERATION_MANIFEST_SCHEMA_V3,
        sequence,
        parent,
        components,
    ))
    .map_err(|_| OperatorGenerationErrorV3::Serialization)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(label: &str) -> String {
        canonical_json_sha256(&label).expect("root")
    }

    fn components() -> OperatorGenerationComponentRootsV3 {
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: root("artifact-set"),
            dispatch_index_sha256: root("dispatch-index"),
            actor_program_sha256: root("actor"),
            renderer_program_sha256: root("renderer"),
            verifier_contract_sha256: root("verifier"),
            capability_contract_sha256: root("capability"),
            resource_budget_sha256: root("budget"),
        }
    }

    #[test]
    fn manifest_is_canonical_and_restart_stable() {
        let manifest =
            seal_operator_generation_manifest_v3(1, None, components()).expect("manifest");
        let bytes = manifest.canonical_bytes().expect("bytes");
        let restored = OperatorGenerationManifestV3::from_canonical_bytes(&bytes).expect("restore");

        assert_eq!(restored, manifest);
        assert_eq!(restored.canonical_bytes(), Ok(bytes));
        assert!(!restored.execution_authority());
    }

    #[test]
    fn every_component_change_creates_a_new_generation() {
        let base = components();
        let base_id = seal_operator_generation_manifest_v3(1, None, base.clone())
            .expect("base")
            .generation_id_sha256()
            .to_owned();
        let mut ids = BTreeSet::new();
        for index in 0..7 {
            let mut changed = base.clone();
            let replacement = root(&format!("changed-{index}"));
            match index {
                0 => changed.artifact_set_sha256 = replacement,
                1 => changed.dispatch_index_sha256 = replacement,
                2 => changed.actor_program_sha256 = replacement,
                3 => changed.renderer_program_sha256 = replacement,
                4 => changed.verifier_contract_sha256 = replacement,
                5 => changed.capability_contract_sha256 = replacement,
                _ => changed.resource_budget_sha256 = replacement,
            }
            let id = seal_operator_generation_manifest_v3(1, None, changed)
                .expect("changed")
                .generation_id_sha256()
                .to_owned();
            assert_ne!(id, base_id);
            ids.insert(id);
        }
        assert_eq!(ids.len(), 7);
    }

    #[test]
    fn lineage_and_manifest_tampering_fail_closed() {
        assert_eq!(
            seal_operator_generation_manifest_v3(2, None, components()),
            Err(OperatorGenerationErrorV3::InvalidParent)
        );
        let first = seal_operator_generation_manifest_v3(1, None, components()).expect("first");
        let second = seal_operator_generation_manifest_v3(
            2,
            Some(first.generation_id_sha256().to_owned()),
            components(),
        )
        .expect("second");
        assert_ne!(first.generation_id_sha256(), second.generation_id_sha256());

        let mut bytes = second.canonical_bytes().expect("bytes");
        let offset = bytes.len() / 2;
        bytes[offset] ^= 1;
        assert!(OperatorGenerationManifestV3::from_canonical_bytes(&bytes).is_err());
    }
}
