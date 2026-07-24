use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nando_operator_kernel::{
    OperatorGenerationManifestV3, ProgramSemanticClassDescriptorV1, ProgramSemanticClassIdV1,
    ResponseProgram, canonical_json_sha256, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use crate::{
    CandidateSearchCompletion, EvidenceUpdateReport, GenerationEvidenceAccountingV3,
    GenerationEvidenceLedgerV3, GenerationEvidenceObservationInputV3, GenerationLearningOutcomeV3,
    VersionSpaceArena, VersionSpaceConfig, VersionSpaceEvidenceError,
    seal_generation_evidence_observation_v3,
};

use super::{
    CandidateFreezeErrorV1, CandidateFreezeInputV1, CandidateFreezeReceiptV1,
    OperatorIdentificationMetricsV1, OperatorObservationErrorV1, OperatorObservationV1,
    SemanticProgramClassV1, SemanticQuotientErrorV1, build_semantic_program_quotient_v1,
    seal_candidate_freeze_v1,
};

pub const OPERATOR_IDENTIFICATION_MAX_SUPPORT_V1: usize = 2_048;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IdentifiedSemanticClassV1 {
    semantic_class: SemanticProgramClassV1,
    canonical_program_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AmbiguityReportV1 {
    pub surviving_programs: usize,
    pub surviving_semantic_classes: usize,
    pub competing_class_ids: Vec<ProgramSemanticClassIdV1>,
    pub search_complete: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum OperatorIdentificationStateV1 {
    Collecting { reason: String },
    Empty { reason: String },
    Ambiguous { report: AmbiguityReportV1 },
    Identified { class: IdentifiedSemanticClassV1 },
    Exhausted { reason: String },
    Contradicted { reason: String },
    Frozen { freeze_root_sha256: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservationUpdateV1 {
    pub evidence: EvidenceUpdateReport,
    pub state: OperatorIdentificationStateV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorIdentificationErrorV1 {
    CandidateRegistrationClosed,
    CandidateRejected,
    ConflictingSemanticClass,
    InvalidObservation,
    DuplicateObservation,
    InvalidSequence,
    SupportBudgetExhausted,
    SearchIncomplete,
    NotIdentified,
    SupportClosed,
    FutureBeforeFreeze,
    FutureSemanticContradiction,
    InvalidScope,
    ObservationEvidence,
    SemanticQuotient,
    EvidenceLedger,
    Freeze,
    Serialization,
}

impl fmt::Display for OperatorIdentificationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CandidateRegistrationClosed => "candidate registration is closed",
            Self::CandidateRejected => "candidate was rejected by the bounded version space",
            Self::ConflictingSemanticClass => "candidate has conflicting semantic class metadata",
            Self::InvalidObservation => "operator observation is invalid",
            Self::DuplicateObservation => "operator observation reuses an immutable root",
            Self::InvalidSequence => "operator observation sequence is not strictly increasing",
            Self::SupportBudgetExhausted => "operator identification support budget is exhausted",
            Self::SearchIncomplete => "candidate generation is not complete",
            Self::NotIdentified => "version space has not identified one semantic class",
            Self::SupportClosed => "operator support is already frozen",
            Self::FutureBeforeFreeze => "operator future arrived before candidate freeze",
            Self::FutureSemanticContradiction => {
                "operator future contradicts the frozen semantic class"
            }
            Self::InvalidScope => "applicability scope root is invalid",
            Self::ObservationEvidence => "exact observation evidence is incomplete or invalid",
            Self::SemanticQuotient => "semantic program quotient is invalid",
            Self::EvidenceLedger => "generation evidence ledger rejected the transition",
            Self::Freeze => "candidate freeze could not be sealed",
            Self::Serialization => "operator identification serialization failed",
        })
    }
}

impl std::error::Error for OperatorIdentificationErrorV1 {}

pub struct OperatorIdentificationMachineV1 {
    pub(super) manifest: OperatorGenerationManifestV3,
    pub(super) arena: VersionSpaceArena,
    pub(super) descriptors: BTreeMap<String, ProgramSemanticClassDescriptorV1>,
    pub(super) support: Vec<OperatorObservationV1>,
    pub(super) observation_roots: BTreeSet<String>,
    pub(super) event_roots: BTreeSet<String>,
    pub(super) request_roots: BTreeSet<String>,
    pub(super) receipt_roots: BTreeSet<String>,
    pub(super) last_capture_sequence: u64,
    pub(super) hard_contradiction: bool,
    pub(super) zero_gain_observations: usize,
    pub(super) total_information_gain: usize,
    pub(super) freeze: Option<CandidateFreezeReceiptV1>,
    pub(super) evidence_ledger: Option<GenerationEvidenceLedgerV3>,
}

impl OperatorIdentificationMachineV1 {
    #[must_use]
    pub fn new(manifest: OperatorGenerationManifestV3, config: VersionSpaceConfig) -> Self {
        Self {
            manifest,
            arena: VersionSpaceArena::new(config),
            descriptors: BTreeMap::new(),
            support: Vec::new(),
            observation_roots: BTreeSet::new(),
            event_roots: BTreeSet::new(),
            request_roots: BTreeSet::new(),
            receipt_roots: BTreeSet::new(),
            last_capture_sequence: 0,
            hard_contradiction: false,
            zero_gain_observations: 0,
            total_information_gain: 0,
            freeze: None,
            evidence_ledger: None,
        }
    }

    pub fn register_candidate(
        &mut self,
        program: ResponseProgram,
        descriptor: ProgramSemanticClassDescriptorV1,
    ) -> Result<String, OperatorIdentificationErrorV1> {
        if !self.support.is_empty() || self.freeze.is_some() {
            return Err(OperatorIdentificationErrorV1::CandidateRegistrationClosed);
        }
        descriptor
            .validate()
            .map_err(|_| OperatorIdentificationErrorV1::ConflictingSemanticClass)?;
        let node_id = self
            .arena
            .intern(program)
            .ok_or(OperatorIdentificationErrorV1::CandidateRejected)?;
        let digest = self
            .arena
            .survivor_programs()
            .into_iter()
            .find(|program| program.node_id == node_id)
            .map(|program| program.digest_sha256)
            .ok_or(OperatorIdentificationErrorV1::CandidateRejected)?;
        if self
            .descriptors
            .insert(digest.clone(), descriptor.clone())
            .is_some_and(|existing| existing != descriptor)
        {
            return Err(OperatorIdentificationErrorV1::ConflictingSemanticClass);
        }
        Ok(digest)
    }

    pub fn complete_candidate_generation(&mut self) -> CandidateSearchCompletion {
        self.arena.mark_candidate_generation_complete()
    }

    pub fn apply_support(
        &mut self,
        observation: OperatorObservationV1,
    ) -> Result<ObservationUpdateV1, OperatorIdentificationErrorV1> {
        if self.freeze.is_some() {
            return Err(OperatorIdentificationErrorV1::SupportClosed);
        }
        observation
            .validate()
            .map_err(|_| OperatorIdentificationErrorV1::InvalidObservation)?;
        if self.support.len() >= OPERATOR_IDENTIFICATION_MAX_SUPPORT_V1 {
            return Err(OperatorIdentificationErrorV1::SupportBudgetExhausted);
        }
        self.validate_new_observation(&observation)?;

        let evidence = if matches!(
            observation.outcome(),
            GenerationLearningOutcomeV3::Censored(_)
        ) {
            EvidenceUpdateReport {
                survivors_before: self.arena.report().survivors,
                survivors_after: self.arena.report().survivors,
                ..EvidenceUpdateReport::default()
            }
        } else {
            self.arena
                .apply_evaluations(observation.evaluations())
                .map_err(|_| OperatorIdentificationErrorV1::ObservationEvidence)?
        };
        if evidence.information_gain == 0 {
            self.zero_gain_observations = self.zero_gain_observations.saturating_add(1);
        }
        self.total_information_gain = self
            .total_information_gain
            .saturating_add(evidence.information_gain);
        if observation.outcome() == GenerationLearningOutcomeV3::HardContradiction {
            self.hard_contradiction = true;
        }
        self.last_capture_sequence = observation.capture_sequence();
        self.observation_roots
            .insert(observation.observation_id_sha256().to_owned());
        self.event_roots
            .insert(observation.event_root_sha256().to_owned());
        self.request_roots
            .insert(observation.request_root_sha256().to_owned());
        self.receipt_roots
            .insert(observation.verifier_receipt_root_sha256().to_owned());
        self.support.push(observation);
        Ok(ObservationUpdateV1 {
            evidence,
            state: self.state()?,
        })
    }

    pub fn state(&self) -> Result<OperatorIdentificationStateV1, OperatorIdentificationErrorV1> {
        if let Some(freeze) = &self.freeze {
            return Ok(OperatorIdentificationStateV1::Frozen {
                freeze_root_sha256: freeze.freeze_root_sha256().to_owned(),
            });
        }
        if self.hard_contradiction {
            return Ok(OperatorIdentificationStateV1::Contradicted {
                reason: "verified_hard_contradiction".to_owned(),
            });
        }
        match self.arena.search_completion() {
            CandidateSearchCompletion::Incomplete => {
                return Ok(OperatorIdentificationStateV1::Collecting {
                    reason: "candidate_generation_incomplete".to_owned(),
                });
            }
            CandidateSearchCompletion::Exhausted => {
                return Ok(OperatorIdentificationStateV1::Exhausted {
                    reason: "candidate_generation_budget_exhausted".to_owned(),
                });
            }
            CandidateSearchCompletion::Complete => {}
        }
        if !self
            .support
            .iter()
            .any(|observation| observation.outcome() == GenerationLearningOutcomeV3::VerifiedPass)
        {
            return Ok(OperatorIdentificationStateV1::Collecting {
                reason: "verified_support_missing".to_owned(),
            });
        }
        let survivors = self.arena.survivor_programs();
        if survivors.is_empty() {
            return Ok(OperatorIdentificationStateV1::Empty {
                reason: "no_consistent_program".to_owned(),
            });
        }
        let quotient = build_semantic_program_quotient_v1(&survivors, &self.descriptors)
            .map_err(|_| OperatorIdentificationErrorV1::SemanticQuotient)?;
        if quotient.class_count() == 1 {
            let semantic_class = quotient.classes()[0].clone();
            let canonical_program_root_sha256 = semantic_class
                .member_program_sha256()
                .iter()
                .min()
                .cloned()
                .ok_or(OperatorIdentificationErrorV1::SemanticQuotient)?;
            return Ok(OperatorIdentificationStateV1::Identified {
                class: IdentifiedSemanticClassV1 {
                    semantic_class,
                    canonical_program_root_sha256,
                },
            });
        }
        Ok(OperatorIdentificationStateV1::Ambiguous {
            report: AmbiguityReportV1 {
                surviving_programs: survivors.len(),
                surviving_semantic_classes: quotient.class_count(),
                competing_class_ids: quotient
                    .classes()
                    .iter()
                    .map(|class| class.class_id().clone())
                    .collect(),
                search_complete: true,
            },
        })
    }

    pub fn freeze_candidate(
        &mut self,
        support_watermark_next_sequence: u64,
        applicability_scope_root_sha256: String,
    ) -> Result<&CandidateFreezeReceiptV1, OperatorIdentificationErrorV1> {
        if self.freeze.is_some() {
            return self
                .freeze
                .as_ref()
                .ok_or(OperatorIdentificationErrorV1::Freeze);
        }
        if support_watermark_next_sequence <= self.last_capture_sequence {
            return Err(OperatorIdentificationErrorV1::InvalidSequence);
        }
        if !valid_nonzero_sha256(&applicability_scope_root_sha256) {
            return Err(OperatorIdentificationErrorV1::InvalidScope);
        }
        let identified = match self.state()? {
            OperatorIdentificationStateV1::Identified { class } => class,
            _ => return Err(OperatorIdentificationErrorV1::NotIdentified),
        };
        let mut ledger = GenerationEvidenceLedgerV3::new(&self.manifest);
        for observation in &self.support {
            let evidence =
                seal_generation_evidence_observation_v3(GenerationEvidenceObservationInputV3 {
                    generation_id_sha256: self.manifest.generation_id_sha256().to_owned(),
                    capture_sequence: observation.capture_sequence(),
                    support_watermark_next_sequence,
                    support_freeze_sha256: None,
                    lineage_root_sha256: observation.lineage_root_sha256().to_owned(),
                    event_root_sha256: observation.event_root_sha256().to_owned(),
                    request_root_sha256: observation.request_root_sha256().to_owned(),
                    verifier_receipt_root_sha256: observation
                        .verifier_receipt_root_sha256()
                        .to_owned(),
                    outcome: observation.outcome(),
                })
                .map_err(|_| OperatorIdentificationErrorV1::EvidenceLedger)?;
            ledger
                .append_support(evidence)
                .map_err(|_| OperatorIdentificationErrorV1::EvidenceLedger)?;
        }
        let watermark_root_sha256 = canonical_json_sha256(&(
            "nando.operator-identification-watermark.v1",
            self.manifest.generation_id_sha256(),
            support_watermark_next_sequence,
            &self.observation_roots,
        ))
        .map_err(|_| OperatorIdentificationErrorV1::Serialization)?;
        ledger
            .freeze_support(support_watermark_next_sequence, watermark_root_sha256)
            .map_err(|_| OperatorIdentificationErrorV1::EvidenceLedger)?;
        let support_evidence_root_sha256 = ledger
            .support_evidence_root_sha256()
            .map_err(|_| OperatorIdentificationErrorV1::EvidenceLedger)?;
        let search_completion_root_sha256 = canonical_json_sha256(&(
            "nando.operator-identification-search.v1",
            self.arena.report(),
            self.arena.elimination_reasons(),
            identified.semantic_class.class_id().as_str(),
        ))
        .map_err(|_| OperatorIdentificationErrorV1::Serialization)?;
        let eliminated_class_root_sha256 = canonical_json_sha256(&(
            "nando.operator-identification-eliminated.v1",
            self.arena.elimination_reasons(),
        ))
        .map_err(|_| OperatorIdentificationErrorV1::Serialization)?;
        let receipt = seal_candidate_freeze_v1(CandidateFreezeInputV1 {
            generation_id_sha256: self.manifest.generation_id_sha256().to_owned(),
            semantic_class_id: identified.semantic_class.class_id().clone(),
            canonical_program_root_sha256: identified.canonical_program_root_sha256,
            support_evidence_root_sha256,
            support_watermark_next_sequence,
            search_completion_root_sha256,
            eliminated_class_root_sha256,
            applicability_scope_root_sha256,
        })
        .map_err(|_| OperatorIdentificationErrorV1::Freeze)?;
        self.evidence_ledger = Some(ledger);
        self.freeze = Some(receipt);
        self.freeze
            .as_ref()
            .ok_or(OperatorIdentificationErrorV1::Freeze)
    }

    pub fn apply_future(
        &mut self,
        observation: OperatorObservationV1,
    ) -> Result<GenerationEvidenceAccountingV3, OperatorIdentificationErrorV1> {
        observation
            .validate()
            .map_err(|_| OperatorIdentificationErrorV1::InvalidObservation)?;
        let freeze = self
            .freeze
            .as_ref()
            .ok_or(OperatorIdentificationErrorV1::FutureBeforeFreeze)?;
        let expected_programs = self.descriptors.keys().cloned().collect::<BTreeSet<_>>();
        let evaluated_programs = observation
            .evaluations()
            .iter()
            .map(|evaluation| evaluation.program_digest_sha256.clone())
            .collect::<BTreeSet<_>>();
        if evaluated_programs != expected_programs
            || evaluated_programs.len() != observation.evaluations().len()
        {
            return Err(OperatorIdentificationErrorV1::ObservationEvidence);
        }
        let canonical_accepted = observation.evaluations().iter().any(|evaluation| {
            evaluation.program_digest_sha256 == freeze.canonical_program_root_sha256()
                && evaluation.accepted
        });
        let foreign_class_accepted = observation.evaluations().iter().any(|evaluation| {
            evaluation.accepted
                && self
                    .descriptors
                    .get(&evaluation.program_digest_sha256)
                    .is_none_or(|descriptor| descriptor.class_id() != freeze.semantic_class_id())
        });
        if observation.outcome() != GenerationLearningOutcomeV3::VerifiedPass
            || !canonical_accepted
            || foreign_class_accepted
        {
            return Err(OperatorIdentificationErrorV1::FutureSemanticContradiction);
        }
        let ledger = self
            .evidence_ledger
            .as_mut()
            .ok_or(OperatorIdentificationErrorV1::FutureBeforeFreeze)?;
        let support_freeze_sha256 = ledger
            .freeze()
            .map(|support_freeze| support_freeze.freeze_sha256().to_owned())
            .ok_or(OperatorIdentificationErrorV1::FutureBeforeFreeze)?;
        let evidence =
            seal_generation_evidence_observation_v3(GenerationEvidenceObservationInputV3 {
                generation_id_sha256: self.manifest.generation_id_sha256().to_owned(),
                capture_sequence: observation.capture_sequence(),
                support_watermark_next_sequence: freeze.support_watermark_next_sequence(),
                support_freeze_sha256: Some(support_freeze_sha256),
                lineage_root_sha256: observation.lineage_root_sha256().to_owned(),
                event_root_sha256: observation.event_root_sha256().to_owned(),
                request_root_sha256: observation.request_root_sha256().to_owned(),
                verifier_receipt_root_sha256: observation.verifier_receipt_root_sha256().to_owned(),
                outcome: observation.outcome(),
            })
            .map_err(|_| OperatorIdentificationErrorV1::EvidenceLedger)?;
        ledger
            .append_future(evidence)
            .map_err(|_| OperatorIdentificationErrorV1::EvidenceLedger)?;
        Ok(ledger.accounting())
    }

    #[must_use]
    pub fn metrics(&self) -> OperatorIdentificationMetricsV1 {
        let mut metrics = OperatorIdentificationMetricsV1 {
            observations: self.support.len(),
            zero_gain_observations: self.zero_gain_observations,
            total_information_gain: self.total_information_gain,
            surviving_programs: self.arena.report().survivors,
            ..OperatorIdentificationMetricsV1::default()
        };
        for observation in &self.support {
            match observation.outcome() {
                GenerationLearningOutcomeV3::VerifiedPass => metrics.verified_passes += 1,
                GenerationLearningOutcomeV3::ApplicabilityNegative => {
                    metrics.applicability_negatives += 1;
                }
                GenerationLearningOutcomeV3::HardContradiction => {
                    metrics.hard_contradictions += 1;
                }
                GenerationLearningOutcomeV3::Censored(_) => metrics.censored += 1,
            }
        }
        metrics.semantic_classes_remaining = self
            .state()
            .ok()
            .and_then(|state| match state {
                OperatorIdentificationStateV1::Ambiguous { report } => {
                    Some(report.surviving_semantic_classes)
                }
                OperatorIdentificationStateV1::Identified { .. }
                | OperatorIdentificationStateV1::Frozen { .. } => Some(1),
                _ => None,
            })
            .unwrap_or(0);
        metrics
    }

    #[must_use]
    pub const fn freeze(&self) -> Option<&CandidateFreezeReceiptV1> {
        self.freeze.as_ref()
    }

    #[must_use]
    pub const fn evidence_ledger(&self) -> Option<&GenerationEvidenceLedgerV3> {
        self.evidence_ledger.as_ref()
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }

    fn validate_new_observation(
        &self,
        observation: &OperatorObservationV1,
    ) -> Result<(), OperatorIdentificationErrorV1> {
        if observation.capture_sequence() <= self.last_capture_sequence {
            return Err(OperatorIdentificationErrorV1::InvalidSequence);
        }
        if self
            .observation_roots
            .contains(observation.observation_id_sha256())
            || self.event_roots.contains(observation.event_root_sha256())
            || self
                .request_roots
                .contains(observation.request_root_sha256())
            || self
                .receipt_roots
                .contains(observation.verifier_receipt_root_sha256())
        {
            return Err(OperatorIdentificationErrorV1::DuplicateObservation);
        }
        Ok(())
    }
}

impl IdentifiedSemanticClassV1 {
    #[must_use]
    pub const fn semantic_class(&self) -> &SemanticProgramClassV1 {
        &self.semantic_class
    }

    #[must_use]
    pub fn canonical_program_root_sha256(&self) -> &str {
        &self.canonical_program_root_sha256
    }
}

impl From<OperatorObservationErrorV1> for OperatorIdentificationErrorV1 {
    fn from(_: OperatorObservationErrorV1) -> Self {
        Self::InvalidObservation
    }
}

impl From<VersionSpaceEvidenceError> for OperatorIdentificationErrorV1 {
    fn from(_: VersionSpaceEvidenceError) -> Self {
        Self::ObservationEvidence
    }
}

impl From<SemanticQuotientErrorV1> for OperatorIdentificationErrorV1 {
    fn from(_: SemanticQuotientErrorV1) -> Self {
        Self::SemanticQuotient
    }
}

impl From<CandidateFreezeErrorV1> for OperatorIdentificationErrorV1 {
    fn from(_: CandidateFreezeErrorV1) -> Self {
        Self::Freeze
    }
}
