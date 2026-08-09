use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    CollectionProgramStep, MultiSourceExtractionStatusV1, OperatorGenerationComponentRootsV3,
    ProgramSemanticClassIdV1, ProgramSemanticClassInputV1, RelationFrame, ResponseArgument,
    ResponseOperation, ResponseProgram, ResponseValueSelector, SemanticRole, canonical_json_sha256,
    response_program_required_routing_atom_ids, response_program_version_root_sha256,
    seal_operator_generation_manifest_v3, seal_program_semantic_class_v1, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use crate::{
    CandidateFreezeReceiptV1, CandidateSearchCompletion, DistinguishingProbeCandidateV1,
    EvidenceSourceContractV1, ExactProgramEvaluation, GenerationLearningOutcomeV3,
    OperatorIdentificationMachineV1, OperatorIdentificationStateV1, OperatorObservationInputV1,
    ProbeClassPredictionV1, VersionSpaceConfig, seal_operator_observation_v1,
    select_distinguishing_probe_v1,
};

use super::{
    BlindThenRevealJoinedTransitionV1, CompletedEffectFormV1, FactorizedMultiSourceRowV1,
    NaturalT1ProgramArtifactV1, PreActionShapeClassV1, factor_multi_source_row_v1,
};

mod natural_artifacts;
use natural_artifacts::{
    ExternalProgramEvidence, collection_candidate_programs, index_candidate_artifacts,
};
mod raw_phase;
pub use raw_phase::{
    RAW_PHASE_T1_HYPOTHESIS_ENVELOPE_SCHEMA_V1, RAW_PHASE_T1_HYPOTHESIS_GENERATOR_V1,
    RawPhaseT1HypothesisEnvelopeV1, RawPhaseT1HypothesisScoreV1,
    seal_raw_phase_t1_hypothesis_envelope_v1,
};

pub const MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3: &str =
    "nando.multi-source-t1-identification.v3";
pub const MULTI_SOURCE_T1_PROOF_BASIS_SCHEMA_V1: &str = "nando.multi-source-t1-proof-basis.v1";
const MULTI_SOURCE_T1_MAX_SUPPORT_BASIS_ROWS: usize = 64;
const MULTI_SOURCE_T1_MAX_FUTURE_BASIS_ROWS: usize = 12;
pub const MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2: &str =
    "nando.multi-source-t1.source-neutral-role-version-space.v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrozenRawPhaseT1ContractV1<'a> {
    pub frozen_domain_root_sha256: &'a str,
    pub support_watermark: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceT1IdentificationStateV1 {
    NoEligibleCohort,
    CandidateGenerationEmpty,
    SearchIncomplete,
    SearchExhausted,
    NoConsistentProgram,
    Ambiguous,
    FrozenAwaitingIndependentFuture,
    FutureContradiction,
    TransferReady,
    InvalidEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PassiveT1ProbeContractV1 {
    pub probe_root_sha256: String,
    pub observable_difference_root_sha256: String,
    pub competing_class_roots_sha256: Vec<String>,
    pub precommitted_predictions_root_sha256: String,
    pub class_partition_predictions: Vec<ProbeClassPredictionV1>,
    pub expected_partition_gain: usize,
    pub estimated_cost_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MultiSourceT1ProofBasisV1 {
    pub schema: String,
    pub basis_root_sha256: String,
    pub support_capture_frame_ids_sha256: Vec<String>,
    pub future_capture_frame_ids_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MultiSourceT1IdentificationV3 {
    pub schema: String,
    pub report_root_sha256: String,
    pub evidence_epoch_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_phase_hypothesis_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_phase_support_watermark: Option<u64>,
    pub selected_shape_root_sha256: Option<String>,
    pub selected_protocol_mode_root_sha256: Option<String>,
    pub selected_marginal_input_tokens: u64,
    pub candidate_programs: usize,
    pub semantic_classes_remaining: usize,
    pub remaining_semantic_class_roots_sha256: Vec<String>,
    pub support_rows: usize,
    pub support_lineages: usize,
    pub support_manifest_root_sha256: Option<String>,
    pub zero_gain_observations: usize,
    pub support_reuse_rows: usize,
    pub independent_future_rows: usize,
    pub independent_future_lineages: usize,
    pub wrong_role_bindings: usize,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub future_rejection_reasons: BTreeMap<String, usize>,
    pub negative_accepts: usize,
    pub candidate_freeze: Option<CandidateFreezeReceiptV1>,
    pub canonical_program: Option<ResponseProgram>,
    pub proof_basis: Option<MultiSourceT1ProofBasisV1>,
    pub passive_probe: Option<PassiveT1ProbeContractV1>,
    pub exact_transfer_parity: bool,
    pub runtime_actor_verifier_parity: bool,
    pub state: MultiSourceT1IdentificationStateV1,
    pub blocker: Option<String>,
    pub execution_authority: bool,
}

#[derive(Clone)]
struct EligibleT1Row {
    joined: BlindThenRevealJoinedTransitionV1,
    frame: RelationFrame,
    factorized: FactorizedMultiSourceRowV1,
    protocol_mode_root_sha256: String,
    seed_programs: BTreeMap<String, ResponseProgram>,
    external_program_evidence: Option<ExternalProgramEvidence>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct T1CohortKey {
    effect_shape_root_sha256: String,
    protocol_mode_root_sha256: String,
}

#[derive(Serialize)]
struct T1IdentificationDigest<'a> {
    schema: &'static str,
    evidence_epoch_sha256: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_phase_hypothesis_root_sha256: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_phase_support_watermark: Option<u64>,
    selected_shape_root_sha256: Option<&'a str>,
    selected_protocol_mode_root_sha256: Option<&'a str>,
    selected_marginal_input_tokens: u64,
    candidate_programs: usize,
    semantic_classes_remaining: usize,
    remaining_semantic_class_roots_sha256: &'a [String],
    support_rows: usize,
    support_lineages: usize,
    support_manifest_root_sha256: Option<&'a str>,
    zero_gain_observations: usize,
    support_reuse_rows: usize,
    independent_future_rows: usize,
    independent_future_lineages: usize,
    wrong_role_bindings: usize,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    future_rejection_reasons: &'a BTreeMap<String, usize>,
    negative_accepts: usize,
    candidate_freeze: &'a Option<CandidateFreezeReceiptV1>,
    canonical_program: &'a Option<ResponseProgram>,
    proof_basis: &'a Option<MultiSourceT1ProofBasisV1>,
    passive_probe: &'a Option<PassiveT1ProbeContractV1>,
    exact_transfer_parity: bool,
    runtime_actor_verifier_parity: bool,
    state: MultiSourceT1IdentificationStateV1,
    blocker: Option<&'a str>,
    execution_authority: bool,
}

#[must_use]
pub fn identify_multi_source_t1_operator_v1(
    joined_rows: &[BlindThenRevealJoinedTransitionV1],
    frames: &[RelationFrame],
    active_intents: &BTreeSet<String>,
    evidence_epoch_sha256: String,
) -> MultiSourceT1IdentificationV3 {
    identify_multi_source_t1_operator_with_active_protocols_v1(
        joined_rows,
        frames,
        active_intents,
        &BTreeSet::new(),
        evidence_epoch_sha256,
    )
}

#[must_use]
pub fn identify_multi_source_t1_operator_with_active_protocols_v1(
    joined_rows: &[BlindThenRevealJoinedTransitionV1],
    frames: &[RelationFrame],
    active_intents: &BTreeSet<String>,
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
    evidence_epoch_sha256: String,
) -> MultiSourceT1IdentificationV3 {
    identify_multi_source_t1_operator_with_candidate_artifacts_v1(
        joined_rows,
        frames,
        active_intents,
        active_protocol_mode_roots_sha256,
        &[],
        evidence_epoch_sha256,
    )
}

#[must_use]
pub fn identify_multi_source_t1_operator_with_candidate_artifacts_v1(
    joined_rows: &[BlindThenRevealJoinedTransitionV1],
    frames: &[RelationFrame],
    active_intents: &BTreeSet<String>,
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
    candidate_artifacts: &[NaturalT1ProgramArtifactV1],
    evidence_epoch_sha256: String,
) -> MultiSourceT1IdentificationV3 {
    identify_multi_source_t1_operator_internal_v1(
        joined_rows,
        frames,
        active_intents,
        active_protocol_mode_roots_sha256,
        candidate_artifacts,
        None,
        evidence_epoch_sha256,
    )
}

/// Runs hypothesis formation only inside an already immutable K1 domain.
/// Raw Phase scores and seals the supplied bounded candidate set; the existing
/// identification machine remains the only component that may collapse it.
#[must_use]
pub fn identify_multi_source_t1_operator_with_frozen_raw_phase_v1(
    joined_rows: &[BlindThenRevealJoinedTransitionV1],
    frames: &[RelationFrame],
    active_intents: &BTreeSet<String>,
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
    candidate_artifacts: &[NaturalT1ProgramArtifactV1],
    raw_phase_contract: FrozenRawPhaseT1ContractV1<'_>,
    evidence_epoch_sha256: String,
) -> MultiSourceT1IdentificationV3 {
    identify_multi_source_t1_operator_internal_v1(
        joined_rows,
        frames,
        active_intents,
        active_protocol_mode_roots_sha256,
        candidate_artifacts,
        Some(raw_phase_contract),
        evidence_epoch_sha256,
    )
}

fn identify_multi_source_t1_operator_internal_v1(
    joined_rows: &[BlindThenRevealJoinedTransitionV1],
    frames: &[RelationFrame],
    active_intents: &BTreeSet<String>,
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
    candidate_artifacts: &[NaturalT1ProgramArtifactV1],
    raw_phase_contract: Option<FrozenRawPhaseT1ContractV1<'_>>,
    evidence_epoch_sha256: String,
) -> MultiSourceT1IdentificationV3 {
    let frame_by_root = frames
        .iter()
        .filter_map(|frame| {
            canonical_json_sha256(frame)
                .ok()
                .map(|root| (root, frame.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let artifact_by_identity = match index_candidate_artifacts(candidate_artifacts) {
        Ok(index) => index,
        Err(blocker) => {
            return terminal_report(
                evidence_epoch_sha256,
                MultiSourceT1IdentificationStateV1::InvalidEvidence,
                blocker,
            );
        }
    };
    let mut cohorts = BTreeMap::<T1CohortKey, Vec<EligibleT1Row>>::new();
    let mut eligible_rows = Vec::new();
    let mut active_duplicate_tokens = 0u64;
    let mut candidate_generation_blocks = BTreeMap::<(String, &'static str), u64>::new();
    for joined in joined_rows {
        let is_hypothesis_support = raw_phase_contract
            .is_none_or(|contract| joined.capture_sequence <= contract.support_watermark);
        let factorized = factor_multi_source_row_v1(joined);
        let supported_shape = matches!(
            factorized.pre_action_shape,
            PreActionShapeClassV1::SingleRoleProjection
                | PreActionShapeClassV1::OneOutputManyScalarRoles
                | PreActionShapeClassV1::ManyOutputsLatestRelevantRole
                | PreActionShapeClassV1::CrossOutputDependency
                | PreActionShapeClassV1::CollectionPlusScalarMetadata
                | PreActionShapeClassV1::MultipleCollections
        );
        let supported_effect = matches!(
            factorized.completed_effect,
            CompletedEffectFormV1::SingleRoleProjection
                | CompletedEffectFormV1::MultiRoleRendering
                | CompletedEffectFormV1::StatusValueBranch
                | CompletedEffectFormV1::CollectionTransform
                | CompletedEffectFormV1::CrossOutputComposition
        );
        if !supported_shape
            || !supported_effect
            || !matches!(
                joined.topology.extraction_status,
                MultiSourceExtractionStatusV1::Complete
            )
            || joined.topology.role_witnesses.len() != joined.topology.roles.len()
            || active_intents.contains(&joined.turn_intent_id_sha256)
        {
            continue;
        }
        let Some(frame) = frame_by_root.get(&joined.completed_frame_root_sha256) else {
            return terminal_report(
                evidence_epoch_sha256,
                MultiSourceT1IdentificationStateV1::InvalidEvidence,
                "joined_frame_missing",
            );
        };
        let external_program_evidence = if factorized.completed_effect
            == CompletedEffectFormV1::CollectionTransform
        {
            match collection_candidate_programs(joined, &artifact_by_identity) {
                Ok(evidence) => Some(evidence),
                Err(blocker) => {
                    if is_hypothesis_support {
                        let blocked_tokens = candidate_generation_blocks
                            .entry((factorized.applicability_shape_root_sha256.clone(), blocker))
                            .or_default();
                        if joined.accepted {
                            *blocked_tokens = blocked_tokens.saturating_add(joined.input_tokens);
                        }
                    }
                    continue;
                }
            }
        } else {
            None
        };
        let seed_programs = match external_program_evidence.as_ref() {
            Some(evidence) => Ok(evidence.programs.clone()),
            None => super::source_neutral_t1::enumerate_source_neutral_t1_candidates(joined, frame),
        };
        let seed_programs = match seed_programs {
            Ok(programs) => programs,
            Err(blocker) => {
                if is_hypothesis_support {
                    let blocked_tokens = candidate_generation_blocks
                        .entry((factorized.applicability_shape_root_sha256.clone(), blocker))
                        .or_default();
                    if joined.accepted {
                        *blocked_tokens = blocked_tokens.saturating_add(joined.input_tokens);
                    }
                }
                continue;
            }
        };
        let protocol_mode_root_sha256 = match t1_protocol_mode_root(&seed_programs) {
            Ok(root) => root,
            Err(blocker) => {
                if is_hypothesis_support {
                    let blocked_tokens = candidate_generation_blocks
                        .entry((factorized.applicability_shape_root_sha256.clone(), blocker))
                        .or_default();
                    if joined.accepted {
                        *blocked_tokens = blocked_tokens.saturating_add(joined.input_tokens);
                    }
                }
                continue;
            }
        };
        if active_protocol_mode_roots_sha256.contains(&protocol_mode_root_sha256) {
            if is_hypothesis_support && joined.accepted {
                active_duplicate_tokens =
                    active_duplicate_tokens.saturating_add(joined.input_tokens);
            }
            continue;
        }
        let row = EligibleT1Row {
            joined: joined.clone(),
            frame: frame.clone(),
            factorized,
            protocol_mode_root_sha256: protocol_mode_root_sha256.clone(),
            seed_programs,
            external_program_evidence,
        };
        if is_hypothesis_support {
            cohorts
                .entry(T1CohortKey {
                    effect_shape_root_sha256: row
                        .factorized
                        .applicability_shape_root_sha256
                        .clone(),
                    protocol_mode_root_sha256,
                })
                .or_default()
                .push(row.clone());
        }
        eligible_rows.push(row);
    }
    let Some((cohort_key, mut cohort)) = select_highest_marginal_cohort(cohorts) else {
        if let Some(((shape_root, blocker), tokens)) =
            candidate_generation_blocks.into_iter().max_by(
                |((left_root, _), left_tokens), ((right_root, _), right_tokens)| {
                    left_tokens
                        .cmp(right_tokens)
                        .then_with(|| right_root.cmp(left_root))
                },
            )
        {
            return selected_terminal_report(
                evidence_epoch_sha256,
                shape_root,
                tokens,
                MultiSourceT1IdentificationStateV1::CandidateGenerationEmpty,
                blocker,
            );
        }
        if active_duplicate_tokens > 0 {
            return terminal_report(
                evidence_epoch_sha256,
                MultiSourceT1IdentificationStateV1::NoEligibleCohort,
                "all_supported_t1_protocol_modes_already_active",
            );
        }
        return terminal_report(
            evidence_epoch_sha256,
            MultiSourceT1IdentificationStateV1::NoEligibleCohort,
            "complete_transferable_projection_missing",
        );
    };
    let shape_root = cohort_key.effect_shape_root_sha256;
    let protocol_mode_root = cohort_key.protocol_mode_root_sha256;
    cohort.sort_by(|left, right| {
        left.joined
            .capture_sequence
            .cmp(&right.joined.capture_sequence)
            .then_with(|| {
                left.joined
                    .join_root_sha256
                    .cmp(&right.joined.join_root_sha256)
            })
    });
    let selected_marginal_input_tokens = cohort
        .iter()
        .filter(|row| row.joined.accepted)
        .map(|row| row.joined.input_tokens)
        .sum();
    let accepted = cohort
        .iter()
        .filter(|row| row.joined.accepted)
        .cloned()
        .collect::<Vec<_>>();
    let Some(seed) = accepted.first() else {
        return terminal_report(
            evidence_epoch_sha256,
            MultiSourceT1IdentificationStateV1::NoEligibleCohort,
            "verified_pass_missing",
        );
    };

    let candidate_programs = seed.seed_programs.clone();
    let raw_phase_envelope = match raw_phase_contract {
        Some(contract) => {
            let frozen_domain_root_sha256 = contract.frozen_domain_root_sha256;
            let support_watermark = contract.support_watermark;
            debug_assert!(
                accepted
                    .iter()
                    .all(|row| row.joined.capture_sequence <= support_watermark)
            );
            let support = accepted.iter().collect::<Vec<_>>();
            let support_frames = support
                .iter()
                .map(|row| row.frame.clone())
                .collect::<Vec<_>>();
            let support_lineages = support
                .iter()
                .map(|row| row.joined.session_lineage_sha256.clone())
                .collect::<Vec<_>>();
            let artifact_roots = support
                .iter()
                .filter_map(|row| row.external_program_evidence.as_ref())
                .flat_map(|evidence| evidence.artifact_roots_sha256.iter().cloned())
                .collect::<Vec<_>>();
            match seal_raw_phase_t1_hypothesis_envelope_v1(
                frozen_domain_root_sha256.to_owned(),
                support_watermark,
                &support_frames,
                support_lineages,
                artifact_roots,
                &candidate_programs,
            ) {
                Ok(envelope) => Some(envelope),
                Err(blocker) => {
                    return selected_terminal_report(
                        evidence_epoch_sha256,
                        shape_root,
                        selected_marginal_input_tokens,
                        MultiSourceT1IdentificationStateV1::InvalidEvidence,
                        blocker,
                    );
                }
            }
        }
        None => None,
    };
    let evidence_epoch_sha256 = match raw_phase_envelope.as_ref() {
        Some(envelope) => match canonical_json_sha256(&(
            "nando.multi-source-t1-raw-phase-evidence-epoch.v1",
            evidence_epoch_sha256.as_str(),
            envelope.envelope_root_sha256.as_str(),
        )) {
            Ok(root) => root,
            Err(_) => {
                return selected_terminal_report(
                    evidence_epoch_sha256,
                    shape_root,
                    selected_marginal_input_tokens,
                    MultiSourceT1IdentificationStateV1::InvalidEvidence,
                    "raw_phase_t1_evidence_epoch_failed",
                );
            }
        },
        None => evidence_epoch_sha256,
    };
    let raw_phase_hypothesis_root_sha256 = raw_phase_envelope
        .as_ref()
        .map(|envelope| envelope.envelope_root_sha256.clone());
    let raw_phase_support_watermark = raw_phase_envelope
        .as_ref()
        .map(|envelope| envelope.support_watermark);
    let manifest = match generation_manifest_with_hypothesis_root(
        &shape_root,
        &protocol_mode_root,
        &candidate_programs,
        raw_phase_envelope
            .as_ref()
            .map(|envelope| envelope.envelope_root_sha256.as_str()),
    ) {
        Ok(manifest) => manifest,
        Err(blocker) => {
            return selected_terminal_report(
                evidence_epoch_sha256,
                shape_root,
                selected_marginal_input_tokens,
                MultiSourceT1IdentificationStateV1::InvalidEvidence,
                blocker,
            );
        }
    };
    let mut machine = OperatorIdentificationMachineV1::new(
        manifest,
        VersionSpaceConfig {
            max_complete_candidates: 4_096,
            ..VersionSpaceConfig::default()
        },
    );
    let mut registered = BTreeMap::<String, ResponseProgram>::new();
    let mut class_by_program = BTreeMap::<String, ProgramSemanticClassIdV1>::new();
    for (program_root, program) in &candidate_programs {
        let descriptor =
            match semantic_descriptor(&shape_root, &protocol_mode_root, program_root, program) {
                Ok(descriptor) => descriptor,
                Err(blocker) => {
                    return selected_terminal_report(
                        evidence_epoch_sha256,
                        shape_root,
                        selected_marginal_input_tokens,
                        MultiSourceT1IdentificationStateV1::InvalidEvidence,
                        blocker,
                    );
                }
            };
        let class_id = descriptor.class_id().clone();
        let registered_root = match machine.register_candidate(program.clone(), descriptor) {
            Ok(root) => root,
            Err(error) => {
                return selected_terminal_report(
                    evidence_epoch_sha256,
                    shape_root,
                    selected_marginal_input_tokens,
                    MultiSourceT1IdentificationStateV1::SearchExhausted,
                    format!("candidate_registration:{error}"),
                );
            }
        };
        registered.insert(registered_root.clone(), program.clone());
        class_by_program.insert(registered_root, class_id);
    }
    match machine.complete_candidate_generation() {
        CandidateSearchCompletion::Incomplete => {
            return selected_terminal_report(
                evidence_epoch_sha256,
                shape_root,
                selected_marginal_input_tokens,
                MultiSourceT1IdentificationStateV1::SearchIncomplete,
                "candidate_generation_incomplete",
            );
        }
        CandidateSearchCompletion::Exhausted => {
            return selected_terminal_report(
                evidence_epoch_sha256,
                shape_root,
                selected_marginal_input_tokens,
                MultiSourceT1IdentificationStateV1::SearchExhausted,
                "candidate_generation_exhausted",
            );
        }
        CandidateSearchCompletion::Complete => {}
    }

    let mut freeze = None;
    let mut canonical_program = None;
    let mut support_capture_frame_ids_sha256 = Vec::new();
    let mut support_lineages = BTreeSet::new();
    for row in accepted {
        if freeze.is_some() {
            continue;
        }
        let observation = match observation_for_row(&row, &registered) {
            Ok(observation) => observation,
            Err(blocker) => {
                return selected_terminal_report(
                    evidence_epoch_sha256,
                    shape_root,
                    selected_marginal_input_tokens,
                    MultiSourceT1IdentificationStateV1::InvalidEvidence,
                    blocker,
                );
            }
        };
        support_lineages.insert(row.joined.session_lineage_sha256.clone());
        let state = match machine.apply_support(observation) {
            Ok(update) => update.state,
            Err(error) => {
                return selected_terminal_report(
                    evidence_epoch_sha256,
                    shape_root,
                    selected_marginal_input_tokens,
                    MultiSourceT1IdentificationStateV1::InvalidEvidence,
                    format!("support_evidence:{error}"),
                );
            }
        };
        support_capture_frame_ids_sha256.push(row.frame.frame_id_sha256.clone());
        if support_capture_frame_ids_sha256.len() > MULTI_SOURCE_T1_MAX_SUPPORT_BASIS_ROWS {
            return selected_terminal_report(
                evidence_epoch_sha256,
                shape_root,
                selected_marginal_input_tokens,
                MultiSourceT1IdentificationStateV1::SearchExhausted,
                "support_proof_basis_budget_exhausted",
            );
        }
        match state {
            OperatorIdentificationStateV1::Identified { class } => {
                let selected = registered
                    .get(class.canonical_program_root_sha256())
                    .cloned();
                let Some(selected) = selected else {
                    return selected_terminal_report(
                        evidence_epoch_sha256,
                        shape_root,
                        selected_marginal_input_tokens,
                        MultiSourceT1IdentificationStateV1::InvalidEvidence,
                        "canonical_program_missing",
                    );
                };
                let scope = canonical_json_sha256(&(
                    "nando.multi-source-t1-applicability-scope.v2",
                    protocol_mode_root.as_str(),
                    class.semantic_class().class_id().as_str(),
                    class.canonical_program_root_sha256(),
                    response_program_required_routing_atom_ids(&selected),
                ))
                .expect("T1 applicability scope serializes");
                let watermark = row.joined.capture_sequence.saturating_add(1);
                let sealed = match machine.freeze_candidate(watermark, scope) {
                    Ok(sealed) => sealed.clone(),
                    Err(error) => {
                        return selected_terminal_report(
                            evidence_epoch_sha256,
                            shape_root,
                            selected_marginal_input_tokens,
                            MultiSourceT1IdentificationStateV1::InvalidEvidence,
                            format!("candidate_freeze:{error}"),
                        );
                    }
                };
                canonical_program = Some(selected);
                freeze = Some(sealed);
            }
            OperatorIdentificationStateV1::Empty { .. } => {
                return selected_terminal_report(
                    evidence_epoch_sha256,
                    shape_root,
                    selected_marginal_input_tokens,
                    MultiSourceT1IdentificationStateV1::NoConsistentProgram,
                    "support_eliminated_all_candidates",
                );
            }
            OperatorIdentificationStateV1::Exhausted { .. } => {
                return selected_terminal_report(
                    evidence_epoch_sha256,
                    shape_root,
                    selected_marginal_input_tokens,
                    MultiSourceT1IdentificationStateV1::SearchExhausted,
                    "search_exhausted_after_evidence",
                );
            }
            OperatorIdentificationStateV1::Contradicted { .. } => {
                return selected_terminal_report(
                    evidence_epoch_sha256,
                    shape_root,
                    selected_marginal_input_tokens,
                    MultiSourceT1IdentificationStateV1::InvalidEvidence,
                    "support_hard_contradiction",
                );
            }
            OperatorIdentificationStateV1::Collecting { .. }
            | OperatorIdentificationStateV1::Ambiguous { .. }
            | OperatorIdentificationStateV1::Frozen { .. } => {}
        }
    }

    let Some(candidate_freeze) = freeze else {
        let metrics = machine.metrics();
        let passive_probe = passive_probe(&shape_root, &machine, &registered, &class_by_program);
        let remaining_semantic_class_roots_sha256 = machine_semantic_class_roots(&machine);
        let support_manifest_root_sha256 =
            seal_t1_support_manifest_root_v1(&support_capture_frame_ids_sha256);
        return finalize_report(MultiSourceT1IdentificationV3 {
            schema: MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3.to_owned(),
            report_root_sha256: String::new(),
            evidence_epoch_sha256,
            raw_phase_hypothesis_root_sha256,
            raw_phase_support_watermark,
            selected_shape_root_sha256: Some(shape_root),
            selected_protocol_mode_root_sha256: Some(protocol_mode_root),
            selected_marginal_input_tokens,
            candidate_programs: registered.len(),
            semantic_classes_remaining: metrics.semantic_classes_remaining,
            remaining_semantic_class_roots_sha256,
            support_rows: metrics.observations,
            support_lineages: support_lineages.len(),
            support_manifest_root_sha256,
            zero_gain_observations: metrics.zero_gain_observations,
            support_reuse_rows: 0,
            independent_future_rows: 0,
            independent_future_lineages: 0,
            wrong_role_bindings: 0,
            future_rejection_reasons: BTreeMap::new(),
            negative_accepts: 0,
            candidate_freeze: None,
            canonical_program: None,
            proof_basis: None,
            passive_probe,
            exact_transfer_parity: false,
            runtime_actor_verifier_parity: false,
            state: MultiSourceT1IdentificationStateV1::Ambiguous,
            blocker: Some("multiple_semantic_classes_require_distinguishing_evidence".to_owned()),
            execution_authority: false,
        });
    };
    let selected_program = canonical_program.expect("freeze owns canonical program");
    let remaining_semantic_class_roots_sha256 =
        vec![candidate_freeze.semantic_class_id().as_str().to_owned()];
    let mut future_candidates = eligible_rows
        .iter()
        .filter(|row| {
            row.joined.accepted
                && row.joined.capture_sequence >= candidate_freeze.support_watermark_next_sequence()
                && row.protocol_mode_root_sha256 == protocol_mode_root
        })
        .cloned()
        .collect::<Vec<_>>();
    future_candidates.sort_by(|left, right| {
        left.joined
            .capture_sequence
            .cmp(&right.joined.capture_sequence)
            .then_with(|| {
                left.joined
                    .join_root_sha256
                    .cmp(&right.joined.join_root_sha256)
            })
    });
    let mut support_reuse_rows = 0usize;
    let mut wrong_role_bindings = 0usize;
    let mut future_rejection_reasons = BTreeMap::<String, usize>::new();
    let mut future_capture_frame_ids_sha256 = Vec::new();
    let mut future_basis_lineages = BTreeSet::new();
    for row in future_candidates {
        if support_lineages.contains(&row.joined.session_lineage_sha256) {
            support_reuse_rows = support_reuse_rows.saturating_add(1);
            continue;
        }
        if let Some(blocker) = row_program_consistency_blocker(&row, &selected_program) {
            wrong_role_bindings = wrong_role_bindings.saturating_add(1);
            *future_rejection_reasons
                .entry(blocker.to_owned())
                .or_default() += 1;
            continue;
        }
        let observation = match observation_for_row(&row, &registered) {
            Ok(observation) => observation,
            Err(_) => {
                wrong_role_bindings = wrong_role_bindings.saturating_add(1);
                *future_rejection_reasons
                    .entry("future_observation_invalid".to_owned())
                    .or_default() += 1;
                continue;
            }
        };
        if machine.apply_future(observation).is_err() {
            wrong_role_bindings = wrong_role_bindings.saturating_add(1);
            *future_rejection_reasons
                .entry("future_ledger_rejected".to_owned())
                .or_default() += 1;
        } else if future_capture_frame_ids_sha256.len() < MULTI_SOURCE_T1_MAX_FUTURE_BASIS_ROWS
            && future_basis_lineages.insert(row.joined.session_lineage_sha256.clone())
        {
            future_capture_frame_ids_sha256.push(row.frame.frame_id_sha256.clone());
        }
    }
    let accounting = machine
        .evidence_ledger()
        .map(|ledger| ledger.accounting())
        .unwrap_or_default();
    let negative_accepts = eligible_rows
        .iter()
        .filter(|row| {
            !row.joined.accepted
                && row.protocol_mode_root_sha256 == protocol_mode_root
                && row_program_consistency_blocker(row, &selected_program).is_none()
        })
        .count();
    let exact_transfer_parity = accounting.future_rows > 0
        && accounting.future_lineages > 0
        && wrong_role_bindings == 0
        && negative_accepts == 0;
    let state = if wrong_role_bindings != 0 || negative_accepts != 0 {
        MultiSourceT1IdentificationStateV1::FutureContradiction
    } else if exact_transfer_parity {
        MultiSourceT1IdentificationStateV1::TransferReady
    } else {
        MultiSourceT1IdentificationStateV1::FrozenAwaitingIndependentFuture
    };
    let blocker = match state {
        MultiSourceT1IdentificationStateV1::TransferReady => None,
        MultiSourceT1IdentificationStateV1::FutureContradiction => {
            Some("post_freeze_exact_parity_or_negative_control_failed".to_owned())
        }
        _ => Some("independent_post_freeze_future_missing".to_owned()),
    };
    let metrics = machine.metrics();
    let support_manifest_root_sha256 =
        seal_t1_support_manifest_root_v1(&support_capture_frame_ids_sha256);
    let Some(proof_basis) = seal_t1_proof_basis_v1(
        support_capture_frame_ids_sha256,
        future_capture_frame_ids_sha256,
    ) else {
        return selected_terminal_report(
            evidence_epoch_sha256,
            shape_root,
            selected_marginal_input_tokens,
            MultiSourceT1IdentificationStateV1::InvalidEvidence,
            "proof_basis_seal_failed",
        );
    };
    finalize_report(MultiSourceT1IdentificationV3 {
        schema: MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3.to_owned(),
        report_root_sha256: String::new(),
        evidence_epoch_sha256,
        raw_phase_hypothesis_root_sha256,
        raw_phase_support_watermark,
        selected_shape_root_sha256: Some(shape_root),
        selected_protocol_mode_root_sha256: Some(protocol_mode_root),
        selected_marginal_input_tokens,
        candidate_programs: registered.len(),
        semantic_classes_remaining: metrics.semantic_classes_remaining,
        remaining_semantic_class_roots_sha256,
        support_rows: accounting.support_rows,
        support_lineages: accounting.support_lineages,
        support_manifest_root_sha256,
        zero_gain_observations: metrics.zero_gain_observations,
        support_reuse_rows,
        independent_future_rows: accounting.future_rows,
        independent_future_lineages: accounting.future_lineages,
        wrong_role_bindings,
        future_rejection_reasons,
        negative_accepts,
        candidate_freeze: Some(candidate_freeze),
        canonical_program: Some(selected_program),
        proof_basis: Some(proof_basis),
        passive_probe: None,
        exact_transfer_parity,
        runtime_actor_verifier_parity: false,
        state,
        blocker,
        execution_authority: false,
    })
}

#[must_use]
pub fn active_t1_protocol_mode_root_v1(program: &ResponseProgram) -> Option<String> {
    let mut canonical = program.clone();
    match &mut canonical.operation {
        ResponseOperation::FunctionCallFromRoles {
            selector,
            arguments,
            ..
        }
        | ResponseOperation::CustomToolCallFromRoles {
            selector,
            arguments,
            ..
        } if arguments.iter().any(|argument| {
            matches!(
                argument,
                ResponseArgument::Role {
                    role: SemanticRole::ContinuationHandle,
                    ..
                }
            )
        }) =>
        {
            let value_type = match selector {
                ResponseValueSelector::ContentLinePrefix { value_type, .. }
                | ResponseValueSelector::ContinuationHandle { value_type } => *value_type,
                _ => return None,
            };
            *selector = ResponseValueSelector::ContinuationHandle { value_type };
        }
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. }
        | ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ProjectStatus { .. }
        | ResponseOperation::ComposeCollection { .. } => {}
        _ => return None,
    }
    canonical.validate().ok()?;
    let root = response_program_version_root_sha256(&canonical).ok()?;
    t1_protocol_mode_root(&BTreeMap::from([(root, canonical)])).ok()
}

fn select_highest_marginal_cohort(
    cohorts: BTreeMap<T1CohortKey, Vec<EligibleT1Row>>,
) -> Option<(T1CohortKey, Vec<EligibleT1Row>)> {
    cohorts
        .into_iter()
        .filter(|(_, rows)| rows.iter().any(|row| row.joined.accepted))
        .max_by(|(left_key, left), (right_key, right)| {
            let left_tokens = left
                .iter()
                .filter(|row| row.joined.accepted)
                .map(|row| row.factorized.input_tokens)
                .sum::<u64>();
            let right_tokens = right
                .iter()
                .filter(|row| row.joined.accepted)
                .map(|row| row.factorized.input_tokens)
                .sum::<u64>();
            left_tokens
                .cmp(&right_tokens)
                .then_with(|| right_key.cmp(left_key))
        })
}

pub(super) fn t1_protocol_mode_root(
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<String, &'static str> {
    if programs.is_empty() {
        return Err("protocol_mode_candidates_empty");
    }
    // Effect identity stays source-neutral. This separate commitment prevents
    // incompatible physical protocol modes from erasing each other's programs.
    let signatures = programs
        .values()
        .map(t1_protocol_signature)
        .collect::<Result<BTreeSet<_>, _>>()?;
    canonical_json_sha256(&("nando.multi-source-t1-protocol-mode-set.v2", signatures))
        .map_err(|_| "protocol_mode_commitment_failed")
}

fn t1_protocol_signature(program: &ResponseProgram) -> Result<String, &'static str> {
    let mut normalized = program.clone();
    match &mut normalized.operation {
        nando_operator_kernel::ResponseOperation::FunctionCallFromRoles {
            selector,
            arguments,
            ..
        }
        | nando_operator_kernel::ResponseOperation::CustomToolCallFromRoles {
            selector,
            arguments,
            ..
        } => {
            *selector = protocol_role_placeholder(selector)?;
            arguments.retain(|argument| {
                !matches!(
                    argument,
                    nando_operator_kernel::ResponseArgument::Integer { name, .. }
                        if crate::teacher_join::is_execution_budget_argument(name)
                )
            });
        }
        nando_operator_kernel::ResponseOperation::ProjectSelectedValue {
            selector,
            renderer,
            ..
        }
        | nando_operator_kernel::ResponseOperation::ProjectStatus {
            selector, renderer, ..
        } => {
            *selector = protocol_role_placeholder(selector)?;
            if let nando_operator_kernel::CollectionOutputRenderer::RenderSequence { segments } =
                renderer
            {
                for segment in segments {
                    if let nando_operator_kernel::ResponseRenderSegment::Selected {
                        selector, ..
                    } = segment
                    {
                        *selector = protocol_role_placeholder(selector)?;
                    }
                }
            }
        }
        nando_operator_kernel::ResponseOperation::ComposeCollection {
            steps, renderer, ..
        } => {
            for step in steps {
                match step {
                    CollectionProgramStep::SelectTurnOutput { output_ordinal } => {
                        *output_ordinal = 0;
                    }
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                        selector,
                        ..
                    } => *selector = protocol_role_placeholder(selector)?,
                    _ => {}
                }
            }
            if let nando_operator_kernel::CollectionOutputRenderer::RenderSequence { segments } =
                renderer
            {
                for segment in segments {
                    if let nando_operator_kernel::ResponseRenderSegment::Selected {
                        selector, ..
                    } = segment
                    {
                        *selector = protocol_role_placeholder(selector)?;
                    }
                }
            }
        }
        _ => return Err("unsupported_t1_protocol_mode"),
    }
    canonical_json_sha256(&("nando.multi-source-t1-protocol-signature.v3", normalized))
        .map_err(|_| "protocol_mode_commitment_failed")
}

fn protocol_role_placeholder(
    selector: &ResponseValueSelector,
) -> Result<ResponseValueSelector, &'static str> {
    let value_type = match selector {
        ResponseValueSelector::ContinuationHandle { value_type }
        | ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. } => *value_type,
        _ => return Err("unsupported_t1_role_selector"),
    };
    Ok(ResponseValueSelector::UniqueScalar { value_type })
}

pub(super) fn generation_manifest(
    shape_root: &str,
    protocol_mode_root: &str,
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<nando_operator_kernel::OperatorGenerationManifestV3, String> {
    generation_manifest_with_hypothesis_root(shape_root, protocol_mode_root, programs, None)
}

fn generation_manifest_with_hypothesis_root(
    shape_root: &str,
    protocol_mode_root: &str,
    programs: &BTreeMap<String, ResponseProgram>,
    raw_phase_hypothesis_root_sha256: Option<&str>,
) -> Result<nando_operator_kernel::OperatorGenerationManifestV3, String> {
    let candidate_roots = programs.keys().cloned().collect::<Vec<_>>();
    let verifier_roots = programs
        .values()
        .map(|program| {
            crate::synthesis::compile_independent_verifier(program)
                .map_err(|error| format!("verifier_compile:{error:?}").to_lowercase())
                .and_then(|verifier| {
                    canonical_json_sha256(&verifier)
                        .map_err(|_| "verifier_commitment_failed".to_owned())
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let artifact_set_sha256 = match raw_phase_hypothesis_root_sha256 {
        Some(root) => canonical_json_sha256(&(
            "nando.multi-source-t1-candidate-set.v3",
            MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2,
            root,
            &candidate_roots,
        )),
        None => canonical_json_sha256(&(
            "nando.multi-source-t1-candidate-set.v2",
            MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2,
            &candidate_roots,
        )),
    }
    .map_err(|_| "candidate_set_commitment_failed".to_owned())?;
    let dispatch_index_sha256 = match raw_phase_hypothesis_root_sha256 {
        Some(root) => canonical_json_sha256(&(
            "nando.multi-source-t1-dispatch.v3",
            MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2,
            root,
            shape_root,
            protocol_mode_root,
        )),
        None => canonical_json_sha256(&(
            "nando.multi-source-t1-dispatch.v2",
            MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2,
            shape_root,
            protocol_mode_root,
        )),
    }
    .map_err(|_| "dispatch_commitment_failed".to_owned())?;
    let resource_budget_sha256 = match raw_phase_hypothesis_root_sha256 {
        Some(root) => canonical_json_sha256(&(
            "nando.multi-source-t1-budget.v3",
            MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2,
            root,
            programs.len(),
            VersionSpaceConfig::default(),
        )),
        None => canonical_json_sha256(&(
            "nando.multi-source-t1-budget.v2",
            MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2,
            programs.len(),
            VersionSpaceConfig::default(),
        )),
    }
    .map_err(|_| "resource_budget_commitment_failed".to_owned())?;
    seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256,
            dispatch_index_sha256,
            actor_program_sha256: canonical_json_sha256(&candidate_roots)
                .map_err(|_| "actor_commitment_failed".to_owned())?,
            renderer_program_sha256: canonical_json_sha256(&(
                "nando.multi-source-t1-renderer.v1",
                &candidate_roots,
            ))
            .map_err(|_| "renderer_commitment_failed".to_owned())?,
            verifier_contract_sha256: canonical_json_sha256(&verifier_roots)
                .map_err(|_| "verifier_set_commitment_failed".to_owned())?,
            capability_contract_sha256: canonical_json_sha256(&(
                "nando.multi-source-t1-capability.v1",
                shape_root,
            ))
            .map_err(|_| "capability_commitment_failed".to_owned())?,
            resource_budget_sha256,
        },
    )
    .map_err(|error| format!("generation_manifest:{error:?}").to_lowercase())
}

pub(super) fn semantic_descriptor(
    shape_root: &str,
    protocol_mode_root: &str,
    program_root: &str,
    program: &ResponseProgram,
) -> Result<nando_operator_kernel::ProgramSemanticClassDescriptorV1, String> {
    let verifier = crate::synthesis::compile_independent_verifier(program)
        .map_err(|error| format!("semantic_verifier_compile:{error:?}").to_lowercase())?;
    seal_program_semantic_class_v1(ProgramSemanticClassInputV1 {
        effect_law_id_sha256: canonical_json_sha256(&(
            "nando.multi-source-t1-effect-law.v1",
            shape_root,
        ))
        .map_err(|_| "effect_law_commitment_failed".to_owned())?,
        role_schema_root_sha256: canonical_json_sha256(&(
            "nando.multi-source-t1-role-schema.v1",
            shape_root,
            response_program_required_routing_atom_ids(program),
        ))
        .map_err(|_| "role_schema_commitment_failed".to_owned())?,
        protocol_mode_set_root_sha256: protocol_mode_root.to_owned(),
        // A physical program remains a competing class until exact evidence
        // proves it action-equivalent to another member.
        executable_behavior_root_sha256: program_root.to_owned(),
        verifier_contract_root_sha256: canonical_json_sha256(&verifier)
            .map_err(|_| "verifier_contract_commitment_failed".to_owned())?,
    })
    .map_err(|error| format!("semantic_descriptor:{error}"))
}

fn observation_for_row(
    row: &EligibleT1Row,
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<crate::OperatorObservationV1, String> {
    observation_for_transition_with_external(
        &row.joined,
        &row.frame,
        programs,
        row.external_program_evidence.as_ref(),
    )
}

pub(super) fn observation_for_transition(
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
    programs: &BTreeMap<String, ResponseProgram>,
) -> Result<crate::OperatorObservationV1, String> {
    observation_for_transition_with_external(joined, frame, programs, None)
}

fn observation_for_transition_with_external(
    joined: &BlindThenRevealJoinedTransitionV1,
    frame: &RelationFrame,
    programs: &BTreeMap<String, ResponseProgram>,
    external: Option<&ExternalProgramEvidence>,
) -> Result<crate::OperatorObservationV1, String> {
    let evaluations = programs
        .iter()
        .map(|(root, program)| {
            let accepted = external.map_or_else(
                || super::source_neutral_t1::t1_program_is_consistent(program, joined, frame),
                |evidence| {
                    let observed = super::observed_typed_consequence_root_v1(frame).ok();
                    evidence
                        .predicted_typed_consequence_roots_sha256
                        .get(root)
                        .is_some_and(|predicted| Some(predicted) == observed.as_ref())
                },
            );
            ExactProgramEvaluation {
                program_digest_sha256: root.clone(),
                accepted,
                reason: if accepted {
                    String::new()
                } else {
                    "exact_completed_transition_mismatch".to_owned()
                },
            }
        })
        .collect();
    let observed_delta_root_sha256 = match external {
        Some(evidence) => canonical_json_sha256(&(
            "nando.multi-source-t1-observed-delta.v2",
            joined.completed_frame_root_sha256.as_str(),
            &joined.effect_atoms,
            joined.accepted,
            &evidence.artifact_roots_sha256,
        )),
        None => canonical_json_sha256(&(
            "nando.multi-source-t1-observed-delta.v1",
            joined.completed_frame_root_sha256.as_str(),
            &joined.effect_atoms,
            joined.accepted,
        )),
    }
    .map_err(|_| "observed_delta_commitment_failed".to_owned())?;
    seal_operator_observation_v1(OperatorObservationInputV1 {
        capture_sequence: joined.capture_sequence,
        lineage_root_sha256: joined.session_lineage_sha256.clone(),
        event_root_sha256: joined.action_event_id_sha256.clone(),
        request_root_sha256: joined.request_event_id_sha256.clone(),
        pre_action_relation_root_sha256: joined.topology_commitment_root_sha256.clone(),
        observed_action_root_sha256: joined.semantic_action_root_sha256.clone(),
        observed_delta_root_sha256,
        verifier_receipt_root_sha256: joined.verifier_receipt_root_sha256.clone(),
        outcome: GenerationLearningOutcomeV3::VerifiedPass,
        evaluations,
    })
    .map_err(|error| format!("operator_observation:{error}"))
}

fn row_program_consistency_blocker(
    row: &EligibleT1Row,
    program: &ResponseProgram,
) -> Option<&'static str> {
    if let Some(evidence) = &row.external_program_evidence {
        let Ok(root) = response_program_version_root_sha256(program) else {
            return Some("external_program_digest_failed");
        };
        let Ok(observed) = super::observed_typed_consequence_root_v1(&row.frame) else {
            return Some("external_observed_typed_consequence_ambiguous");
        };
        return (evidence.predicted_typed_consequence_roots_sha256.get(&root) != Some(&observed))
            .then_some("external_exact_typed_consequence_mismatch");
    }
    super::source_neutral_t1::t1_program_consistency_blocker(program, &row.joined, &row.frame)
}

pub(super) fn passive_probe(
    shape_root: &str,
    machine: &OperatorIdentificationMachineV1,
    programs: &BTreeMap<String, ResponseProgram>,
    class_by_program: &BTreeMap<String, ProgramSemanticClassIdV1>,
) -> Option<PassiveT1ProbeContractV1> {
    let OperatorIdentificationStateV1::Ambiguous { report } = machine.state().ok()? else {
        return None;
    };
    let dimensions = [
        ("role_binding", 1_u64),
        ("temporal_rule", 2_u64),
        ("renderer", 2_u64),
        ("routing_atoms", 3_u64),
    ];
    let mut probes = Vec::new();
    for (dimension, estimated_cost_units) in dimensions {
        let mut predictions = Vec::new();
        for class_id in &report.competing_class_ids {
            let observable_signatures = class_by_program
                .iter()
                .filter(|(_, candidate_class)| *candidate_class == class_id)
                .filter_map(|(root, _)| programs.get(root))
                .filter_map(|program| t1_probe_dimension_signature(program, dimension))
                .collect::<BTreeSet<_>>();
            if observable_signatures.is_empty() {
                predictions.clear();
                break;
            }
            predictions.push(ProbeClassPredictionV1 {
                class_id: class_id.clone(),
                outcome_partition_root_sha256: canonical_json_sha256(&(
                    "nando.multi-source-t1-passive-outcome-partition.v2",
                    dimension,
                    &observable_signatures,
                ))
                .ok()?,
            });
        }
        if predictions.len() != report.competing_class_ids.len()
            || predictions
                .iter()
                .map(|prediction| prediction.outcome_partition_root_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                < 2
        {
            continue;
        }
        let observable_difference_root_sha256 = canonical_json_sha256(&(
            "nando.multi-source-t1-passive-difference.v2",
            dimension,
            &predictions,
        ))
        .ok()?;
        probes.push(DistinguishingProbeCandidateV1 {
            probe_root_sha256: canonical_json_sha256(&(
                "nando.multi-source-t1-passive-probe.v2",
                shape_root,
                dimension,
            ))
            .ok()?,
            observable_difference_root_sha256,
            source: EvidenceSourceContractV1::PassiveLiveTraffic,
            estimated_cost_units,
            predictions,
        });
    }
    let probe = select_distinguishing_probe_v1(&report.competing_class_ids, &probes).ok()?;
    let class_partition_predictions = probes
        .iter()
        .find(|candidate| candidate.probe_root_sha256 == probe.probe_root_sha256())?
        .predictions
        .clone();
    let precommitted_predictions_root_sha256 = canonical_json_sha256(&(
        "nando.multi-source-t1-precommitted-probe-predictions.v1",
        probe.probe_root_sha256(),
        probe.observable_difference_root_sha256(),
        &class_partition_predictions,
    ))
    .ok()?;
    let contract = PassiveT1ProbeContractV1 {
        probe_root_sha256: probe.probe_root_sha256().to_owned(),
        observable_difference_root_sha256: probe.observable_difference_root_sha256().to_owned(),
        competing_class_roots_sha256: probe.competing_class_roots_sha256().to_vec(),
        precommitted_predictions_root_sha256,
        class_partition_predictions,
        expected_partition_gain: probe.expected_partition_gain(),
        estimated_cost_units: probe.estimated_cost_units(),
    };
    contract.validate().then_some(contract)
}

fn machine_semantic_class_roots(machine: &OperatorIdentificationMachineV1) -> Vec<String> {
    match machine.state() {
        Ok(OperatorIdentificationStateV1::Identified { class }) => {
            vec![class.semantic_class().class_id().as_str().to_owned()]
        }
        Ok(OperatorIdentificationStateV1::Ambiguous { report }) => report
            .competing_class_ids
            .into_iter()
            .map(|class| class.as_str().to_owned())
            .collect(),
        _ => Vec::new(),
    }
}

impl PassiveT1ProbeContractV1 {
    fn expected_predictions_root(&self) -> Option<String> {
        canonical_json_sha256(&(
            "nando.multi-source-t1-precommitted-probe-predictions.v1",
            self.probe_root_sha256.as_str(),
            self.observable_difference_root_sha256.as_str(),
            &self.class_partition_predictions,
        ))
        .ok()
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        let predicted_classes = self
            .class_partition_predictions
            .iter()
            .map(|prediction| prediction.class_id.as_str())
            .collect::<BTreeSet<_>>();
        let competing_classes = self
            .competing_class_roots_sha256
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        self.competing_class_roots_sha256.len() >= 2
            && self.competing_class_roots_sha256.len() == competing_classes.len()
            && self.class_partition_predictions.len() == predicted_classes.len()
            && predicted_classes == competing_classes
            && [
                self.probe_root_sha256.as_str(),
                self.observable_difference_root_sha256.as_str(),
                self.precommitted_predictions_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            && self.class_partition_predictions.iter().all(|prediction| {
                valid_nonzero_sha256(prediction.class_id.as_str())
                    && valid_nonzero_sha256(&prediction.outcome_partition_root_sha256)
            })
            && self.expected_partition_gain > 0
            && self.estimated_cost_units > 0
            && self.expected_predictions_root().as_deref()
                == Some(self.precommitted_predictions_root_sha256.as_str())
    }
}

pub(super) fn t1_probe_dimension_signature(
    program: &ResponseProgram,
    dimension: &str,
) -> Option<String> {
    match dimension {
        "role_binding" => canonical_json_sha256(&(
            "nando.multi-source-t1-probe-role-binding.v1",
            t1_program_selectors(program)?,
        ))
        .ok(),
        "temporal_rule" => {
            let selectors = t1_program_selectors(program)?;
            let temporal_rules = selectors
                .iter()
                .map(|selector| match selector {
                    ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. } => "request",
                    ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. } => "latest",
                    ResponseValueSelector::ContinuationHandle { .. } => "producer_linked",
                    ResponseValueSelector::UniqueScalar { .. } => "scope_unique",
                    _ => "unsupported",
                })
                .collect::<Vec<_>>();
            canonical_json_sha256(&(
                "nando.multi-source-t1-probe-temporal-rule.v1",
                temporal_rules,
            ))
            .ok()
        }
        "renderer" => match &program.operation {
            ResponseOperation::ProjectSelectedValue {
                renderer,
                completion_state,
                ..
            }
            | ResponseOperation::ProjectStatus {
                renderer,
                completion_state,
                ..
            } => canonical_json_sha256(&(
                "nando.multi-source-t1-probe-renderer.v1",
                renderer,
                completion_state,
            ))
            .ok(),
            ResponseOperation::ComposeCollection {
                steps,
                format,
                renderer,
                completion_state,
                max_items,
            } => canonical_json_sha256(&(
                "nando.multi-source-t1-probe-collection.v1",
                steps,
                format,
                renderer,
                completion_state,
                max_items,
            ))
            .ok(),
            ResponseOperation::FunctionCallFromRoles {
                function_name,
                arguments,
                ..
            } => canonical_json_sha256(&(
                "nando.multi-source-t1-probe-function.v1",
                function_name,
                arguments,
            ))
            .ok(),
            ResponseOperation::CustomToolCallFromRoles {
                custom_tool_name,
                inner_tool_name,
                arguments,
                projection,
                ..
            } => canonical_json_sha256(&(
                "nando.multi-source-t1-probe-custom-tool.v1",
                custom_tool_name,
                inner_tool_name,
                arguments,
                projection,
            ))
            .ok(),
            _ => None,
        },
        "routing_atoms" => canonical_json_sha256(&(
            "nando.multi-source-t1-probe-routing-atoms.v1",
            response_program_required_routing_atom_ids(program),
        ))
        .ok(),
        _ => None,
    }
}

fn t1_program_selectors(program: &ResponseProgram) -> Option<Vec<ResponseValueSelector>> {
    let mut selectors = match &program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. }
        | ResponseOperation::ProjectSelectedValue { selector, .. }
        | ResponseOperation::ProjectStatus { selector, .. } => vec![selector.clone()],
        _ => return None,
    };
    if let ResponseOperation::ProjectSelectedValue {
        renderer: nando_operator_kernel::CollectionOutputRenderer::RenderSequence { segments },
        ..
    }
    | ResponseOperation::ProjectStatus {
        renderer: nando_operator_kernel::CollectionOutputRenderer::RenderSequence { segments },
        ..
    } = &program.operation
    {
        for segment in segments {
            if let nando_operator_kernel::ResponseRenderSegment::Selected { selector, .. } = segment
                && !selectors.contains(selector)
            {
                selectors.push(selector.clone());
            }
        }
    }
    Some(selectors)
}

fn terminal_report(
    evidence_epoch_sha256: String,
    state: MultiSourceT1IdentificationStateV1,
    blocker: impl Into<String>,
) -> MultiSourceT1IdentificationV3 {
    finalize_report(MultiSourceT1IdentificationV3 {
        schema: MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3.to_owned(),
        report_root_sha256: String::new(),
        evidence_epoch_sha256,
        raw_phase_hypothesis_root_sha256: None,
        raw_phase_support_watermark: None,
        selected_shape_root_sha256: None,
        selected_protocol_mode_root_sha256: None,
        selected_marginal_input_tokens: 0,
        candidate_programs: 0,
        semantic_classes_remaining: 0,
        remaining_semantic_class_roots_sha256: Vec::new(),
        support_rows: 0,
        support_lineages: 0,
        support_manifest_root_sha256: None,
        zero_gain_observations: 0,
        support_reuse_rows: 0,
        independent_future_rows: 0,
        independent_future_lineages: 0,
        wrong_role_bindings: 0,
        future_rejection_reasons: BTreeMap::new(),
        negative_accepts: 0,
        candidate_freeze: None,
        canonical_program: None,
        proof_basis: None,
        passive_probe: None,
        exact_transfer_parity: false,
        runtime_actor_verifier_parity: false,
        state,
        blocker: Some(blocker.into()),
        execution_authority: false,
    })
}

fn selected_terminal_report(
    evidence_epoch_sha256: String,
    shape_root: String,
    selected_marginal_input_tokens: u64,
    state: MultiSourceT1IdentificationStateV1,
    blocker: impl Into<String>,
) -> MultiSourceT1IdentificationV3 {
    let mut report = terminal_report(evidence_epoch_sha256, state, blocker);
    report.selected_shape_root_sha256 = Some(shape_root);
    report.selected_marginal_input_tokens = selected_marginal_input_tokens;
    finalize_report(report)
}

fn finalize_report(mut report: MultiSourceT1IdentificationV3) -> MultiSourceT1IdentificationV3 {
    report.report_root_sha256 = report.expected_root();
    report
}

fn seal_t1_support_manifest_root_v1(frame_ids_sha256: &[String]) -> Option<String> {
    let roots = frame_ids_sha256.iter().collect::<BTreeSet<_>>();
    if roots.is_empty()
        || roots.len() != frame_ids_sha256.len()
        || frame_ids_sha256.len() > MULTI_SOURCE_T1_MAX_SUPPORT_BASIS_ROWS
        || roots.iter().any(|root| !valid_nonzero_sha256(root))
    {
        return None;
    }
    canonical_json_sha256(&(
        "nando.multi-source-t1-support-manifest.v1",
        frame_ids_sha256,
    ))
    .ok()
}

fn seal_t1_proof_basis_v1(
    support_capture_frame_ids_sha256: Vec<String>,
    future_capture_frame_ids_sha256: Vec<String>,
) -> Option<MultiSourceT1ProofBasisV1> {
    if support_capture_frame_ids_sha256.is_empty()
        || support_capture_frame_ids_sha256.len() > MULTI_SOURCE_T1_MAX_SUPPORT_BASIS_ROWS
        || future_capture_frame_ids_sha256.len() > MULTI_SOURCE_T1_MAX_FUTURE_BASIS_ROWS
    {
        return None;
    }
    let mut basis = MultiSourceT1ProofBasisV1 {
        schema: MULTI_SOURCE_T1_PROOF_BASIS_SCHEMA_V1.to_owned(),
        basis_root_sha256: String::new(),
        support_capture_frame_ids_sha256,
        future_capture_frame_ids_sha256,
    };
    if !basis.members_are_valid() {
        return None;
    }
    basis.basis_root_sha256 = basis.expected_root();
    Some(basis)
}

impl MultiSourceT1ProofBasisV1 {
    fn members_are_valid(&self) -> bool {
        let support = self
            .support_capture_frame_ids_sha256
            .iter()
            .collect::<BTreeSet<_>>();
        let future = self
            .future_capture_frame_ids_sha256
            .iter()
            .collect::<BTreeSet<_>>();
        !support.is_empty()
            && support.len() == self.support_capture_frame_ids_sha256.len()
            && future.len() == self.future_capture_frame_ids_sha256.len()
            && support.is_disjoint(&future)
            && support
                .iter()
                .chain(future.iter())
                .all(|root| valid_nonzero_sha256(root))
    }

    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            MULTI_SOURCE_T1_PROOF_BASIS_SCHEMA_V1,
            &self.support_capture_frame_ids_sha256,
            &self.future_capture_frame_ids_sha256,
        ))
        .expect("T1 proof basis serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MULTI_SOURCE_T1_PROOF_BASIS_SCHEMA_V1
            && self.members_are_valid()
            && self.basis_root_sha256 == self.expected_root()
    }
}

impl MultiSourceT1IdentificationV3 {
    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&T1IdentificationDigest {
            schema: MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3,
            evidence_epoch_sha256: self.evidence_epoch_sha256.as_str(),
            raw_phase_hypothesis_root_sha256: self.raw_phase_hypothesis_root_sha256.as_deref(),
            raw_phase_support_watermark: self.raw_phase_support_watermark,
            selected_shape_root_sha256: self.selected_shape_root_sha256.as_deref(),
            selected_protocol_mode_root_sha256: self.selected_protocol_mode_root_sha256.as_deref(),
            selected_marginal_input_tokens: self.selected_marginal_input_tokens,
            candidate_programs: self.candidate_programs,
            semantic_classes_remaining: self.semantic_classes_remaining,
            remaining_semantic_class_roots_sha256: &self.remaining_semantic_class_roots_sha256,
            support_rows: self.support_rows,
            support_lineages: self.support_lineages,
            support_manifest_root_sha256: self.support_manifest_root_sha256.as_deref(),
            zero_gain_observations: self.zero_gain_observations,
            support_reuse_rows: self.support_reuse_rows,
            independent_future_rows: self.independent_future_rows,
            independent_future_lineages: self.independent_future_lineages,
            wrong_role_bindings: self.wrong_role_bindings,
            future_rejection_reasons: &self.future_rejection_reasons,
            negative_accepts: self.negative_accepts,
            candidate_freeze: &self.candidate_freeze,
            canonical_program: &self.canonical_program,
            proof_basis: &self.proof_basis,
            passive_probe: &self.passive_probe,
            exact_transfer_parity: self.exact_transfer_parity,
            runtime_actor_verifier_parity: self.runtime_actor_verifier_parity,
            state: self.state,
            blocker: self.blocker.as_deref(),
            execution_authority: false,
        })
        .expect("T1 identification report serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        let raw_phase_metadata_valid = match (
            self.raw_phase_hypothesis_root_sha256.as_deref(),
            self.raw_phase_support_watermark,
        ) {
            (None, None) => true,
            (Some(root), Some(watermark)) => valid_nonzero_sha256(root) && watermark > 0,
            _ => false,
        };
        if self.schema != MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3
            || self.execution_authority
            || self.report_root_sha256 != self.expected_root()
            || !raw_phase_metadata_valid
            || self.remaining_semantic_class_roots_sha256.len() != self.semantic_classes_remaining
            || self
                .remaining_semantic_class_roots_sha256
                .iter()
                .any(|root| !valid_nonzero_sha256(root))
            || !self
                .remaining_semantic_class_roots_sha256
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self
                .candidate_freeze
                .as_ref()
                .is_some_and(|freeze| freeze.validate().is_err())
            || self
                .proof_basis
                .as_ref()
                .is_some_and(|basis| !basis.validate())
            || self
                .passive_probe
                .as_ref()
                .is_some_and(|probe| !probe.validate())
            || self
                .support_manifest_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || (self.support_rows > 0) != self.support_manifest_root_sha256.is_some()
        {
            return false;
        }
        if let (Some(freeze), Some(program)) = (&self.candidate_freeze, &self.canonical_program)
            && response_program_version_root_sha256(program)
                .ok()
                .as_deref()
                != Some(freeze.canonical_program_root_sha256())
        {
            return false;
        }
        match self.state {
            MultiSourceT1IdentificationStateV1::TransferReady => {
                self.candidate_freeze.is_some()
                    && self.canonical_program.is_some()
                    && self.proof_basis.as_ref().is_some_and(|basis| {
                        !basis.support_capture_frame_ids_sha256.is_empty()
                            && !basis.future_capture_frame_ids_sha256.is_empty()
                    })
                    && self.support_rows > 0
                    && self.independent_future_rows > 0
                    && self.independent_future_lineages > 0
                    && self.wrong_role_bindings == 0
                    && self.negative_accepts == 0
                    && self.exact_transfer_parity
                    && !self.runtime_actor_verifier_parity
            }
            MultiSourceT1IdentificationStateV1::FrozenAwaitingIndependentFuture => {
                self.candidate_freeze.is_some()
                    && self.canonical_program.is_some()
                    && self.proof_basis.as_ref().is_some_and(|basis| {
                        !basis.support_capture_frame_ids_sha256.is_empty()
                            && basis.future_capture_frame_ids_sha256.is_empty()
                    })
                    && !self.exact_transfer_parity
            }
            MultiSourceT1IdentificationStateV1::Ambiguous => {
                self.candidate_freeze.is_none()
                    && self.canonical_program.is_none()
                    && self.proof_basis.is_none()
            }
            _ => self.proof_basis.is_none() && !self.exact_transfer_parity,
        }
    }
}
