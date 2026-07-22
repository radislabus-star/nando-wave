use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};

pub const RUNTIME_PHASE_CONTROL_EVIDENCE_SCHEMA_V3: &str =
    "nando.runtime-phase-control-evidence.v3.f8d";
pub const RUNTIME_PHASE_COHERENCE_SCALE_FIXED_V3: i64 = 1_000_000_000;
pub const RUNTIME_PHASE_APPLICABILITY_FLOOR_FIXED_V3: i64 = 900_000_000;
pub const RUNTIME_PHASE_CONTROL_MAX_EXACT_CHECKS_V3: u32 = 2_048;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePhaseControlKindV3 {
    Full,
    NoPhase,
    ShuffledPhase,
    MagnitudeOnly,
    MatchedRandomCenter,
}

impl RuntimePhaseControlKindV3 {
    pub const ALL: [Self; 5] = [
        Self::Full,
        Self::NoPhase,
        Self::ShuffledPhase,
        Self::MagnitudeOnly,
        Self::MatchedRandomCenter,
    ];
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePhaseSelectionV3 {
    Selected,
    AbstainStructuralBoundary,
    AbstainAmbiguousAction,
    AbstainTie,
    AbstainNoCandidate,
    AbstainCoherenceFloor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePhaseControlObservationV3 {
    control: RuntimePhaseControlKindV3,
    selection: RuntimePhaseSelectionV3,
    exact_action_checks: u32,
    selected_physical_action_sha256: Option<String>,
    winner_coherence_fixed: Option<i64>,
    runner_up_coherence_fixed: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePhaseControlEvidenceV3 {
    schema: String,
    index_sha256: String,
    request_view_sha256: String,
    report_sha256: String,
    observations: Vec<RuntimePhaseControlObservationV3>,
    evidence_sha256: String,
    raw_payloads_persisted: u8,
    execution_authority: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePhaseControlObservationInputV3 {
    pub control: RuntimePhaseControlKindV3,
    pub selection: RuntimePhaseSelectionV3,
    pub exact_action_checks: u32,
    pub selected_physical_action_sha256: Option<String>,
    pub winner_coherence_fixed: Option<i64>,
    pub runner_up_coherence_fixed: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePhaseControlEvidenceInputV3 {
    pub index_sha256: String,
    pub request_view_sha256: String,
    pub report_sha256: String,
    pub observations: Vec<RuntimePhaseControlObservationInputV3>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimePhaseControlEvidenceErrorV3 {
    InvalidInput,
    InvalidEvidence,
    Serialization,
}

pub fn seal_runtime_phase_control_evidence_v3(
    input: RuntimePhaseControlEvidenceInputV3,
) -> Result<RuntimePhaseControlEvidenceV3, RuntimePhaseControlEvidenceErrorV3> {
    let mut evidence = RuntimePhaseControlEvidenceV3 {
        schema: RUNTIME_PHASE_CONTROL_EVIDENCE_SCHEMA_V3.to_owned(),
        index_sha256: input.index_sha256,
        request_view_sha256: input.request_view_sha256,
        report_sha256: input.report_sha256,
        observations: input
            .observations
            .into_iter()
            .map(|observation| RuntimePhaseControlObservationV3 {
                control: observation.control,
                selection: observation.selection,
                exact_action_checks: observation.exact_action_checks,
                selected_physical_action_sha256: observation.selected_physical_action_sha256,
                winner_coherence_fixed: observation.winner_coherence_fixed,
                runner_up_coherence_fixed: observation.runner_up_coherence_fixed,
            })
            .collect(),
        evidence_sha256: String::new(),
        raw_payloads_persisted: 0,
        execution_authority: false,
    };
    evidence.evidence_sha256 = evidence_digest(&evidence)?;
    validate_evidence(&evidence)?;
    Ok(evidence)
}

impl RuntimePhaseControlEvidenceV3 {
    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, RuntimePhaseControlEvidenceErrorV3> {
        validate_evidence(self)?;
        canonical_json_bytes(self)
            .map(Vec::into_boxed_slice)
            .map_err(|_| RuntimePhaseControlEvidenceErrorV3::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RuntimePhaseControlEvidenceErrorV3> {
        let evidence: Self = serde_json::from_slice(bytes)
            .map_err(|_| RuntimePhaseControlEvidenceErrorV3::InvalidEvidence)?;
        validate_evidence(&evidence)?;
        if evidence.canonical_bytes()?.as_ref() != bytes {
            return Err(RuntimePhaseControlEvidenceErrorV3::InvalidEvidence);
        }
        Ok(evidence)
    }

    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn request_view_sha256(&self) -> &str {
        &self.request_view_sha256
    }

    #[must_use]
    pub fn report_sha256(&self) -> &str {
        &self.report_sha256
    }

    #[must_use]
    pub fn observations(&self) -> &[RuntimePhaseControlObservationV3] {
        &self.observations
    }

    #[must_use]
    pub fn evidence_sha256(&self) -> &str {
        &self.evidence_sha256
    }

    #[must_use]
    pub const fn raw_payloads_persisted(&self) -> u8 {
        self.raw_payloads_persisted
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        self.execution_authority
    }
}

impl RuntimePhaseControlObservationV3 {
    #[must_use]
    pub const fn control(&self) -> RuntimePhaseControlKindV3 {
        self.control
    }

    #[must_use]
    pub const fn selection(&self) -> RuntimePhaseSelectionV3 {
        self.selection
    }

    #[must_use]
    pub const fn exact_action_checks(&self) -> u32 {
        self.exact_action_checks
    }

    #[must_use]
    pub fn selected_physical_action_sha256(&self) -> Option<&str> {
        self.selected_physical_action_sha256.as_deref()
    }

    #[must_use]
    pub const fn winner_coherence_fixed(&self) -> Option<i64> {
        self.winner_coherence_fixed
    }

    #[must_use]
    pub const fn runner_up_coherence_fixed(&self) -> Option<i64> {
        self.runner_up_coherence_fixed
    }
}

fn validate_evidence(
    evidence: &RuntimePhaseControlEvidenceV3,
) -> Result<(), RuntimePhaseControlEvidenceErrorV3> {
    let controls = evidence
        .observations
        .iter()
        .map(RuntimePhaseControlObservationV3::control)
        .collect::<BTreeSet<_>>();
    if evidence.schema != RUNTIME_PHASE_CONTROL_EVIDENCE_SCHEMA_V3
        || !valid_nonzero_sha256(&evidence.index_sha256)
        || !valid_nonzero_sha256(&evidence.request_view_sha256)
        || !valid_nonzero_sha256(&evidence.report_sha256)
        || !valid_nonzero_sha256(&evidence.evidence_sha256)
        || controls != RuntimePhaseControlKindV3::ALL.into_iter().collect()
        || evidence.observations.len() != RuntimePhaseControlKindV3::ALL.len()
        || evidence
            .observations
            .iter()
            .zip(RuntimePhaseControlKindV3::ALL)
            .any(|(observation, expected)| observation.control != expected)
        || evidence.raw_payloads_persisted != 0
        || evidence.execution_authority
        || evidence.observations.iter().any(invalid_observation)
        || evidence_digest(evidence)? != evidence.evidence_sha256
    {
        return Err(RuntimePhaseControlEvidenceErrorV3::InvalidEvidence);
    }
    Ok(())
}

fn invalid_observation(observation: &RuntimePhaseControlObservationV3) -> bool {
    let selected = observation.selection == RuntimePhaseSelectionV3::Selected;
    let selected_root_valid = observation
        .selected_physical_action_sha256
        .as_deref()
        .is_some_and(valid_nonzero_sha256);
    observation.exact_action_checks > RUNTIME_PHASE_CONTROL_MAX_EXACT_CHECKS_V3
        || selected != selected_root_valid
        || observation
            .winner_coherence_fixed
            .is_some_and(|value| value.abs() > RUNTIME_PHASE_COHERENCE_SCALE_FIXED_V3)
        || observation
            .runner_up_coherence_fixed
            .is_some_and(|value| value.abs() > RUNTIME_PHASE_COHERENCE_SCALE_FIXED_V3)
        || (selected
            && observation
                .winner_coherence_fixed
                .is_none_or(|value| value < RUNTIME_PHASE_APPLICABILITY_FLOOR_FIXED_V3))
}

fn evidence_digest(
    evidence: &RuntimePhaseControlEvidenceV3,
) -> Result<String, RuntimePhaseControlEvidenceErrorV3> {
    canonical_json_sha256(&(
        RUNTIME_PHASE_CONTROL_EVIDENCE_SCHEMA_V3,
        evidence.index_sha256.as_str(),
        evidence.request_view_sha256.as_str(),
        evidence.report_sha256.as_str(),
        &evidence.observations,
        0_u8,
        false,
    ))
    .map_err(|_| RuntimePhaseControlEvidenceErrorV3::Serialization)
}
