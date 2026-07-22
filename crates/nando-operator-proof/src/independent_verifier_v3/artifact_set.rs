use nando_operator_kernel::{
    CanonicalEffectLawV3, ExecutableProtocolModeArtifactV3, canonical_json_bytes,
    executable_artifact_set_sha256_v3, validate_canonical_effect_law_v3,
    validate_executable_protocol_mode_artifact_v3,
};

use super::input::F6_MAX_MODES_V3;

pub struct IndependentVerifierArtifactSetV3 {
    artifact_set_sha256: String,
    mode_count: usize,
    artifacts: Box<[VerifiedArtifactV3]>,
}

pub(super) struct VerifiedArtifactV3 {
    artifact: ExecutableProtocolModeArtifactV3,
    law: CanonicalEffectLawV3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndependentVerifierArtifactSetErrorV3 {
    Empty,
    OverBudget,
    DuplicateArtifact,
    InvalidArtifact,
    Serialization,
}

impl IndependentVerifierArtifactSetV3 {
    pub fn new(
        artifacts: &[ExecutableProtocolModeArtifactV3],
    ) -> Result<Self, IndependentVerifierArtifactSetErrorV3> {
        if artifacts.is_empty() {
            return Err(IndependentVerifierArtifactSetErrorV3::Empty);
        }
        let mut verified = artifacts
            .iter()
            .map(verified_artifact_v3)
            .collect::<Result<Vec<_>, _>>()?;
        verified.sort_by(|left, right| {
            left.artifact
                .artifact_sha256()
                .cmp(right.artifact.artifact_sha256())
        });
        if verified
            .windows(2)
            .any(|pair| pair[0].artifact.artifact_sha256() == pair[1].artifact.artifact_sha256())
        {
            return Err(IndependentVerifierArtifactSetErrorV3::DuplicateArtifact);
        }
        let mode_count = verified
            .iter()
            .map(|entry| entry.artifact.modes().len())
            .sum::<usize>();
        if mode_count == 0 || mode_count > F6_MAX_MODES_V3 {
            return Err(IndependentVerifierArtifactSetErrorV3::OverBudget);
        }
        let artifacts = verified
            .iter()
            .map(|entry| entry.artifact.clone())
            .collect::<Vec<_>>();
        let artifact_set_sha256 = executable_artifact_set_sha256_v3(&artifacts)
            .map_err(|_| IndependentVerifierArtifactSetErrorV3::Serialization)?;
        Ok(Self {
            artifact_set_sha256,
            mode_count,
            artifacts: verified.into_boxed_slice(),
        })
    }

    #[must_use]
    pub fn artifact_set_sha256(&self) -> &str {
        &self.artifact_set_sha256
    }

    #[must_use]
    pub const fn mode_count(&self) -> usize {
        self.mode_count
    }

    pub(super) const fn artifacts(&self) -> &[VerifiedArtifactV3] {
        &self.artifacts
    }
}

impl VerifiedArtifactV3 {
    pub(super) const fn artifact(&self) -> &ExecutableProtocolModeArtifactV3 {
        &self.artifact
    }

    pub(super) const fn law(&self) -> &CanonicalEffectLawV3 {
        &self.law
    }
}

fn verified_artifact_v3(
    artifact: &ExecutableProtocolModeArtifactV3,
) -> Result<VerifiedArtifactV3, IndependentVerifierArtifactSetErrorV3> {
    validate_executable_protocol_mode_artifact_v3(artifact)
        .map_err(|_| IndependentVerifierArtifactSetErrorV3::InvalidArtifact)?;
    let bytes = artifact
        .canonical_bytes()
        .map_err(|_| IndependentVerifierArtifactSetErrorV3::Serialization)?;
    if ExecutableProtocolModeArtifactV3::from_canonical_bytes(&bytes, artifact.artifact_sha256())
        .map_err(|_| IndependentVerifierArtifactSetErrorV3::InvalidArtifact)?
        != *artifact
    {
        return Err(IndependentVerifierArtifactSetErrorV3::InvalidArtifact);
    }
    let law_bytes = canonical_json_bytes(artifact.effect_law_payload())
        .map_err(|_| IndependentVerifierArtifactSetErrorV3::Serialization)?;
    let law = CanonicalEffectLawV3::from_canonical_bytes(&law_bytes)
        .map_err(|_| IndependentVerifierArtifactSetErrorV3::InvalidArtifact)?;
    validate_canonical_effect_law_v3(&law)
        .map_err(|_| IndependentVerifierArtifactSetErrorV3::InvalidArtifact)?;
    if law
        .effect_law_id()
        .map_err(|_| IndependentVerifierArtifactSetErrorV3::Serialization)?
        .as_str()
        != artifact.source_mode_set().effect_law_id_sha256
    {
        return Err(IndependentVerifierArtifactSetErrorV3::InvalidArtifact);
    }
    Ok(VerifiedArtifactV3 {
        artifact: artifact.clone(),
        law,
    })
}
