use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{ExternalGenerationAdmissionErrorV3, ExternalGenerationAdmissionVerdictV3};

pub const EXTERNAL_GENERATION_ADMISSION_COMMITMENTS_SCHEMA_V3: &str =
    "nando.external-generation-admission-commitments.v3.f8c";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalGenerationAdmissionCommitmentsV3 {
    schema: String,
    generation_id_sha256: String,
    generation_checkpoint_sha256: String,
    generation_capture_index_sha256: String,
    provider_capture_index_sha256: String,
    shadow_ledger_sha256: String,
    artifact_set_sha256: String,
    dispatch_index_sha256: String,
    support_evidence_sha256: String,
    future_partition_sha256: String,
    phase_control_receipt_sha256: String,
    resource_receipt_sha256: String,
    support_denominator: u32,
    future_denominator: u32,
    live_shadow_denominator: u32,
    live_verified_passes: u32,
    negative_denominator: u32,
    censored_denominator: u32,
    verdict: ExternalGenerationAdmissionVerdictWireV3,
    commitments_sha256: String,
    execution_authority: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ExternalGenerationAdmissionVerdictWireV3 {
    ShadowReady,
    WatchNoCausalGain,
}

pub struct ExternalGenerationAdmissionCandidateV3 {
    commitments: ExternalGenerationAdmissionCommitmentsV3,
    canonical_commitments: Box<[u8]>,
}

pub struct VerifiedExternalGenerationSubmissionV3 {
    commitments_sha256: String,
}

pub(super) struct ReconstructedCommitmentInputV3 {
    pub generation_id_sha256: String,
    pub generation_checkpoint_sha256: String,
    pub generation_capture_index_sha256: String,
    pub provider_capture_index_sha256: String,
    pub shadow_ledger_sha256: String,
    pub artifact_set_sha256: String,
    pub dispatch_index_sha256: String,
    pub support_evidence_sha256: String,
    pub future_partition_sha256: String,
    pub phase_control_receipt_sha256: String,
    pub resource_receipt_sha256: String,
    pub support_denominator: u32,
    pub future_denominator: u32,
    pub live_shadow_denominator: u32,
    pub live_verified_passes: u32,
    pub negative_denominator: u32,
    pub censored_denominator: u32,
    pub verdict: ExternalGenerationAdmissionVerdictV3,
}

impl ExternalGenerationAdmissionCandidateV3 {
    pub(super) fn from_reconstructed(
        input: ReconstructedCommitmentInputV3,
    ) -> Result<Self, ExternalGenerationAdmissionErrorV3> {
        let verdict = match input.verdict {
            ExternalGenerationAdmissionVerdictV3::ShadowReady => {
                ExternalGenerationAdmissionVerdictWireV3::ShadowReady
            }
            ExternalGenerationAdmissionVerdictV3::WatchNoCausalGain => {
                ExternalGenerationAdmissionVerdictWireV3::WatchNoCausalGain
            }
        };
        let mut commitments = ExternalGenerationAdmissionCommitmentsV3 {
            schema: EXTERNAL_GENERATION_ADMISSION_COMMITMENTS_SCHEMA_V3.to_owned(),
            generation_id_sha256: input.generation_id_sha256,
            generation_checkpoint_sha256: input.generation_checkpoint_sha256,
            generation_capture_index_sha256: input.generation_capture_index_sha256,
            provider_capture_index_sha256: input.provider_capture_index_sha256,
            shadow_ledger_sha256: input.shadow_ledger_sha256,
            artifact_set_sha256: input.artifact_set_sha256,
            dispatch_index_sha256: input.dispatch_index_sha256,
            support_evidence_sha256: input.support_evidence_sha256,
            future_partition_sha256: input.future_partition_sha256,
            phase_control_receipt_sha256: input.phase_control_receipt_sha256,
            resource_receipt_sha256: input.resource_receipt_sha256,
            support_denominator: input.support_denominator,
            future_denominator: input.future_denominator,
            live_shadow_denominator: input.live_shadow_denominator,
            live_verified_passes: input.live_verified_passes,
            negative_denominator: input.negative_denominator,
            censored_denominator: input.censored_denominator,
            verdict,
            commitments_sha256: String::new(),
            execution_authority: false,
        };
        commitments.commitments_sha256 = commitments_digest(&commitments)?;
        validate_commitments(&commitments)?;
        let canonical_commitments = canonical_json_bytes(&commitments)
            .map(Vec::into_boxed_slice)
            .map_err(|_| ExternalGenerationAdmissionErrorV3::Serialization)?;
        Ok(Self {
            commitments,
            canonical_commitments,
        })
    }

    #[must_use]
    pub fn generation_id_sha256(&self) -> &str {
        &self.commitments.generation_id_sha256
    }

    #[must_use]
    pub const fn verdict(&self) -> ExternalGenerationAdmissionVerdictV3 {
        match self.commitments.verdict {
            ExternalGenerationAdmissionVerdictWireV3::ShadowReady => {
                ExternalGenerationAdmissionVerdictV3::ShadowReady
            }
            ExternalGenerationAdmissionVerdictWireV3::WatchNoCausalGain => {
                ExternalGenerationAdmissionVerdictV3::WatchNoCausalGain
            }
        }
    }

    #[must_use]
    pub const fn support_denominator(&self) -> u32 {
        self.commitments.support_denominator
    }

    #[must_use]
    pub const fn future_denominator(&self) -> u32 {
        self.commitments.future_denominator
    }

    #[must_use]
    pub const fn live_shadow_denominator(&self) -> u32 {
        self.commitments.live_shadow_denominator
    }

    #[must_use]
    pub const fn live_verified_passes(&self) -> u32 {
        self.commitments.live_verified_passes
    }

    #[must_use]
    pub fn commitments_sha256(&self) -> &str {
        &self.commitments.commitments_sha256
    }

    #[must_use]
    pub const fn canonical_commitments(&self) -> &[u8] {
        &self.canonical_commitments
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

pub fn verify_external_generation_submission_v3(
    candidate: &ExternalGenerationAdmissionCandidateV3,
    submitted_commitments: &[u8],
) -> Result<VerifiedExternalGenerationSubmissionV3, ExternalGenerationAdmissionErrorV3> {
    if submitted_commitments.is_empty() {
        return Err(ExternalGenerationAdmissionErrorV3::MissingInput);
    }
    let submitted: ExternalGenerationAdmissionCommitmentsV3 =
        serde_json::from_slice(submitted_commitments)
            .map_err(|_| ExternalGenerationAdmissionErrorV3::CommitmentDrift)?;
    validate_commitments(&submitted)?;
    let canonical = canonical_json_bytes(&submitted)
        .map_err(|_| ExternalGenerationAdmissionErrorV3::Serialization)?;
    if canonical != submitted_commitments
        || submitted_commitments != candidate.canonical_commitments()
    {
        return Err(ExternalGenerationAdmissionErrorV3::CommitmentDrift);
    }
    Ok(VerifiedExternalGenerationSubmissionV3 {
        commitments_sha256: submitted.commitments_sha256,
    })
}

impl VerifiedExternalGenerationSubmissionV3 {
    #[must_use]
    pub fn commitments_sha256(&self) -> &str {
        &self.commitments_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn validate_commitments(
    commitments: &ExternalGenerationAdmissionCommitmentsV3,
) -> Result<(), ExternalGenerationAdmissionErrorV3> {
    let roots = [
        commitments.generation_id_sha256.as_str(),
        commitments.generation_checkpoint_sha256.as_str(),
        commitments.generation_capture_index_sha256.as_str(),
        commitments.provider_capture_index_sha256.as_str(),
        commitments.shadow_ledger_sha256.as_str(),
        commitments.artifact_set_sha256.as_str(),
        commitments.dispatch_index_sha256.as_str(),
        commitments.support_evidence_sha256.as_str(),
        commitments.future_partition_sha256.as_str(),
        commitments.phase_control_receipt_sha256.as_str(),
        commitments.resource_receipt_sha256.as_str(),
        commitments.commitments_sha256.as_str(),
    ];
    if commitments.schema != EXTERNAL_GENERATION_ADMISSION_COMMITMENTS_SCHEMA_V3
        || roots.into_iter().any(|root| !valid_nonzero_sha256(root))
        || commitments.support_denominator == 0
        || commitments.future_denominator == 0
        || commitments.live_shadow_denominator == 0
        || commitments
            .live_verified_passes
            .saturating_add(commitments.negative_denominator)
            .saturating_add(commitments.censored_denominator)
            > commitments.live_shadow_denominator
        || (commitments.verdict == ExternalGenerationAdmissionVerdictWireV3::ShadowReady
            && commitments.live_verified_passes == 0)
        || commitments.execution_authority
        || commitments_digest(commitments)? != commitments.commitments_sha256
    {
        return Err(ExternalGenerationAdmissionErrorV3::CommitmentDrift);
    }
    Ok(())
}

fn commitments_digest(
    commitments: &ExternalGenerationAdmissionCommitmentsV3,
) -> Result<String, ExternalGenerationAdmissionErrorV3> {
    canonical_json_sha256(&(
        EXTERNAL_GENERATION_ADMISSION_COMMITMENTS_SCHEMA_V3,
        (
            commitments.generation_id_sha256.as_str(),
            commitments.generation_checkpoint_sha256.as_str(),
            commitments.generation_capture_index_sha256.as_str(),
            commitments.provider_capture_index_sha256.as_str(),
            commitments.shadow_ledger_sha256.as_str(),
        ),
        (
            commitments.artifact_set_sha256.as_str(),
            commitments.dispatch_index_sha256.as_str(),
            commitments.support_evidence_sha256.as_str(),
            commitments.future_partition_sha256.as_str(),
            commitments.phase_control_receipt_sha256.as_str(),
            commitments.resource_receipt_sha256.as_str(),
        ),
        (
            commitments.support_denominator,
            commitments.future_denominator,
            commitments.live_shadow_denominator,
            commitments.live_verified_passes,
            commitments.negative_denominator,
            commitments.censored_denominator,
        ),
        commitments.verdict,
        false,
    ))
    .map_err(|_| ExternalGenerationAdmissionErrorV3::Serialization)
}
