use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{
    BlueprintBeamConfig, BlueprintFutureEvaluator, BlueprintFutureEvidence, BlueprintPhaseControl,
    BlueprintSynthesisBlockerCount, BlueprintSynthesisReport, BoundedCircuitBeam,
    BoundedRoleAligner, FrozenOperatorBlueprintSet, LocalRelationFragment, OPERATOR_ROLE_NONE,
    RoleAlignmentConfig, SearchCompletion, StructuralRoleSignature, SurfaceFragmentBundle,
    TernaryRelationState, TypedProgramAtom, phase_vector_from_atoms,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, CollectionScalarType,
    CollectionSynthesisExample, CrystallizationParityReceipt, CrystallizedOperator,
    ProjectStatusMapping, ResponseArgument, ResponseOperation, ResponsePackage,
    ResponsePackageOrigin, ResponsePackageProof, ResponsePackageState, ResponseProgram,
    ResponseRenderSegment, ResponseValueSelector, RuntimeRoleAnchor, TRANSFORM_FLAG_CANONICAL_JSON,
    TRANSFORM_OPCODE_COUNT_COLLECTION, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
    TRANSFORM_OPCODE_PROJECT_STATUS, TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
    TRANSFORM_STATUS_ZERO_IS_OK, TRANSFORM_STATUS_ZERO_IS_PASS, TRANSFORM_STATUS_ZERO_IS_SUCCESS,
    TRANSFORM_STATUS_ZERO_IS_TRUE, TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_COLLECTION,
    TRANSFORM_VALUE_IDENTIFIER, TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING, TeacherTransition,
    ValueProjectionFormat, enumerate_source_neutral_response_programs, execute_response,
    is_privacy_safe_online_response_program, is_source_neutral_response_program,
    response_actor_program_digest, response_independent_verifier_program_digest,
    source_neutral_verifier_for_program, synthesize_response_operator,
};

const LIVE_SCALAR_SUPPORT_ROWS: usize = 32;
const LIVE_SCALAR_FUTURE_ROWS: usize = 32;
const TEACHER_CALL_SELECTOR_BUDGET: usize = 512;

#[derive(Clone, Debug, PartialEq)]
pub struct LiveScalarCircuitSample {
    pub bundle: SurfaceFragmentBundle,
    pub anchors: Box<[RuntimeRoleAnchor]>,
    pub actor_template: ResponseProgram,
    pub actor_hypotheses: Box<[ResponseProgram]>,
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response: String,
    pub raw_input_sha256: [u8; 32],
    pub extractor_version: u32,
    pub law_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveScalarShadowBlocker {
    TeacherRejected,
    MissingParityCase,
    PayloadTooLarge,
    NoExactSourceNeutralProgram,
    ProgramEnumerationFailed,
    CandidateBudgetExhausted,
    EmptyVersionSpace,
    TeacherSynthesisEmptySupport,
    TeacherSynthesisAmbiguousRoles,
    TeacherSynthesisInconsistentRoleFamily,
    TeacherSynthesisMissingPendingState,
    TeacherSynthesisMissingCompletionState,
    TeacherSynthesisMissingUniqueHandle,
    TeacherSynthesisNoConsistentProgram,
    TeacherSynthesisAmbiguousPrograms,
    TeacherProgramNotCall,
    TeacherProgramParityMismatch,
    TeacherProgramRuntimeAbstain,
    TeacherProgramImmediateToolOutputMissing,
    TeacherProgramSelectorPrefixMissing,
    TeacherProgramSelectorPrefixAmbiguous,
    TeacherProgramSelectorTypeMismatch,
    TeacherProgramOutputTextInvalid,
    TeacherProgramSelectorParseFailed,
    TeacherProgramRoleParseFailed,
    TeacherProgramVerificationFailed,
    TeacherProgramInvalid,
    TeacherProgramWireEnvelopeMismatch,
    TeacherProgramSymbolMismatch,
    TeacherProgramInputMismatch,
    TeacherProgramInputWhitespaceMismatch,
    TeacherProgramInputTokenValueMismatch,
    TeacherProgramInputSingleNumericMismatch,
    TeacherProgramInputMultipleNumericMismatch,
    TeacherProgramInputQuotedLiteralMismatch,
    TeacherProgramInputIdentifierMismatch,
    TeacherProgramInputMixedTokenMismatch,
    TeacherProgramDynamicRoleNumericMismatch,
    TeacherProgramDynamicRoleStringMismatch,
    TeacherProgramStaticIntegerMismatch,
    TeacherProgramStaticStringMismatch,
    TeacherProgramRoleValueUnavailable,
    TeacherProgramRoleValueNotObserved,
    TeacherProgramRoleValueCandidateMismatch,
    TeacherProgramRoleValueInRequestText,
    TeacherProgramRoleValueInPayloadScalar,
    TeacherProgramRoleValueInPayloadText,
    TeacherProgramRoleValueAbsentFromPayload,
    TeacherProgramInputSyntaxShapeMismatch,
    TeacherProgramResponseShapeMismatch,
    ExactStatusProgram,
    ExactCollectionProgram,
    UnsupportedRendererProgram,
    ExactTypedCanonicalizationFailed,
    SelectedTemplateCanonicalizationFailed,
    CanonicalCandidateMissing,
    LawShapeMissing,
    HypothesisEncodingFailed,
    HypothesisBudgetExhausted,
    RoleTypeInferenceFailed,
    ObservedRoleExtractionFailed,
    PayloadSerializationFailed,
    RequestTextInvalid,
    ProviderInputMissing,
    UnsupportedTransformOpcode,
    UnsupportedTransformFlags,
    UnsupportedProgramKind,
    // Legacy aggregate retained for checkpoint compatibility.
    UnsupportedScalarProgram,
    InvalidCommitment,
    InvalidBundle,
    // Kept for checkpoints written before support rows were separated from
    // session-diversity evidence. New generations no longer emit this blocker.
    SupportSessionReused,
    // Legacy checkpoints counted every repeated future session as a blocker.
    // New generations only reject overlap across the support/future boundary.
    FutureSessionReused,
    SupportFutureSessionOverlap,
    FutureCapacityReached,
    HistoricalSupportCapacityReached,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LiveScalarShadowState {
    observations: usize,
    executable: usize,
    duplicate_rows: usize,
    blockers: BTreeMap<LiveScalarShadowBlocker, usize>,
    laws: BTreeMap<String, LiveScalarLawState>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct LiveScalarLawState {
    support: Vec<TeacherTransition>,
    future: Vec<TeacherTransition>,
    // Physical selectors are factored out of the semantic law and intersected
    // once as support arrives. Keeping this compact version set here prevents
    // every report from repeating the full selector search over all 32 rows.
    #[serde(default)]
    support_actor_hypotheses: Vec<ResponseProgram>,
    #[serde(default)]
    support_hypotheses_initialized: bool,
}

struct CompetingBlueprintSet {
    support_bundles: Vec<SurfaceFragmentBundle>,
    synthesis: BlueprintSynthesisReport,
    actors_by_blueprint: BTreeMap<[u8; 32], ResponseProgram>,
    actor_hypothesis_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarShadowReport {
    pub observations: usize,
    pub executable: usize,
    pub duplicate_rows: usize,
    pub law_count: usize,
    pub support_rows: usize,
    pub future_rows: usize,
    pub actor_hypotheses: usize,
    pub competing_blueprints: usize,
    pub frozen_laws: usize,
    pub full_phase_winners: usize,
    pub causal_control_passes: usize,
    pub verified_shadow_operators: usize,
    pub shadow_executions: usize,
    pub admission_candidates: usize,
    pub ingest_accounting_complete: bool,
    pub blockers: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveScalarAdmissionCandidate {
    pub package: ResponsePackage,
    pub support: Vec<TeacherTransition>,
    pub future: Vec<TeacherTransition>,
    pub support_root_sha256: String,
    pub future_evidence_root_sha256: String,
    pub future_lineage_root_sha256: String,
    pub winner_seal_sha256: String,
    pub executable_parity_seal_sha256: String,
}

impl LiveScalarShadowState {
    pub fn observe(&mut self, transition: &TeacherTransition) {
        self.observations = self.observations.saturating_add(1);
        let sample = match extract_live_scalar_circuit_sample(transition) {
            Ok(sample) => sample,
            Err(blocker) => {
                *self.blockers.entry(blocker).or_default() += 1;
                return;
            }
        };
        self.executable = self.executable.saturating_add(1);
        let law_key = commitment_hex(&sample.law_sha256);
        let law = self.laws.entry(law_key).or_default();
        if law
            .support
            .iter()
            .chain(&law.future)
            .any(|row| row.before.frame_id_sha256 == transition.before.frame_id_sha256)
        {
            self.duplicate_rows = self.duplicate_rows.saturating_add(1);
            return;
        }
        if law.support.len() < LIVE_SCALAR_SUPPORT_ROWS {
            if let Err(blocker) = update_support_actor_hypotheses(law, &sample.actor_hypotheses) {
                *self.blockers.entry(blocker).or_default() += 1;
                return;
            }
            law.support.push(transition.clone());
            return;
        }
        if law
            .support
            .iter()
            .any(|row| row.before.session_id_sha256 == transition.before.session_id_sha256)
        {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::SupportFutureSessionOverlap)
                .or_default() += 1;
            return;
        }
        if law.future.len() < LIVE_SCALAR_FUTURE_ROWS {
            law.future.push(transition.clone());
        } else {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::FutureCapacityReached)
                .or_default() += 1;
        }
    }

    /// Rebuilds bounded support after a strategy upgrade without reclassifying
    /// historical receipts as post-freeze future evidence.
    pub(crate) fn observe_historical_support(&mut self, transition: &TeacherTransition) {
        self.observations = self.observations.saturating_add(1);
        let sample = match extract_live_scalar_circuit_sample(transition) {
            Ok(sample) => sample,
            Err(blocker) => {
                *self.blockers.entry(blocker).or_default() += 1;
                return;
            }
        };
        self.executable = self.executable.saturating_add(1);
        let law_key = commitment_hex(&sample.law_sha256);
        let law = self.laws.entry(law_key).or_default();
        if law
            .support
            .iter()
            .any(|row| row.before.frame_id_sha256 == transition.before.frame_id_sha256)
        {
            self.duplicate_rows = self.duplicate_rows.saturating_add(1);
            return;
        }
        if law.support.len() >= LIVE_SCALAR_SUPPORT_ROWS {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::HistoricalSupportCapacityReached)
                .or_default() += 1;
            return;
        }
        if let Err(blocker) = update_support_actor_hypotheses(law, &sample.actor_hypotheses) {
            *self.blockers.entry(blocker).or_default() += 1;
            return;
        }
        law.support.push(transition.clone());
    }

    #[must_use]
    pub fn report(&self) -> LiveScalarShadowReport {
        let support_rows = self.laws.values().map(|law| law.support.len()).sum();
        let future_rows = self.laws.values().map(|law| law.future.len()).sum();
        let ingest_blocker_rows = self.blockers.values().copied().sum::<usize>();
        let mut report = LiveScalarShadowReport {
            observations: self.observations,
            executable: self.executable,
            duplicate_rows: self.duplicate_rows,
            law_count: self.laws.len(),
            support_rows,
            future_rows,
            ingest_accounting_complete: self.observations
                == support_rows
                    .saturating_add(future_rows)
                    .saturating_add(self.duplicate_rows)
                    .saturating_add(ingest_blocker_rows),
            blockers: self
                .blockers
                .iter()
                .map(|(blocker, count)| (format!("{blocker:?}").to_lowercase(), *count))
                .collect(),
            ..LiveScalarShadowReport::default()
        };
        let mut candidates = Vec::new();
        for law in self.laws.values() {
            evaluate_live_law(law, &mut report, &mut candidates);
        }
        report.admission_candidates = candidates.len();
        report
    }

    #[must_use]
    pub fn admission_candidates(&self) -> Vec<LiveScalarAdmissionCandidate> {
        let mut report = LiveScalarShadowReport::default();
        let mut candidates = Vec::new();
        for law in self.laws.values() {
            evaluate_live_law(law, &mut report, &mut candidates);
        }
        candidates
    }

    /// Returns only already completed support rows for strategy migration.
    /// Frozen future belongs to the old generation and must never be replayed
    /// into a new strategy as either support or future authority.
    pub(crate) fn historical_support_transitions(&self) -> Vec<TeacherTransition> {
        let mut support = self
            .laws
            .values()
            .flat_map(|law| law.support.iter().cloned())
            .map(|transition| (transition.before.frame_id_sha256.clone(), transition))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect::<Vec<_>>();
        support.sort_by(|left, right| {
            left.before
                .observed_at_unix_nanos
                .cmp(&right.before.observed_at_unix_nanos)
                .then_with(|| {
                    left.before
                        .frame_id_sha256
                        .cmp(&right.before.frame_id_sha256)
                })
        });
        support
    }
}

fn update_support_actor_hypotheses(
    law: &mut LiveScalarLawState,
    observed: &[ResponseProgram],
) -> Result<(), LiveScalarShadowBlocker> {
    let observed = observed
        .iter()
        .map(|program| {
            serde_cbor::to_vec(program)
                .map(|key| (key, program.clone()))
                .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    if observed.is_empty() || observed.len() > TEACHER_CALL_SELECTOR_BUDGET {
        return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
    }
    if !law.support_hypotheses_initialized {
        law.support_actor_hypotheses = observed.into_values().collect();
        law.support_hypotheses_initialized = true;
        return Ok(());
    }
    law.support_actor_hypotheses.retain(|program| {
        serde_cbor::to_vec(program)
            .ok()
            .is_some_and(|key| observed.contains_key(&key))
    });
    Ok(())
}

fn evaluate_live_law(
    law: &LiveScalarLawState,
    report: &mut LiveScalarShadowReport,
    candidates: &mut Vec<LiveScalarAdmissionCandidate>,
) {
    if law.support.len() < LIVE_SCALAR_SUPPORT_ROWS {
        increment_report_blocker(report, "support_below_32");
        return;
    }
    if law.support_actor_hypotheses.is_empty() {
        increment_report_blocker(report, "actor_hypotheses_no_common_version");
        return;
    }
    if law.support_actor_hypotheses.len() > crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS {
        increment_report_blocker(report, "actor_hypothesis_class_budget");
        return;
    }
    let Ok(support) = law
        .support
        .iter()
        .map(|transition| {
            reextract_live_scalar_circuit_sample(transition, &law.support_actor_hypotheses)
        })
        .collect::<Result<Vec<_>, _>>()
    else {
        increment_report_blocker(report, "support_reextract_failed");
        return;
    };
    let competing = match build_competing_blueprint_set(&support) {
        Ok(competing) => competing,
        Err(blocker) => {
            increment_report_blocker(report, &blocker);
            return;
        }
    };
    report.actor_hypotheses = report
        .actor_hypotheses
        .saturating_add(competing.actor_hypothesis_count);
    report.competing_blueprints = report
        .competing_blueprints
        .saturating_add(competing.synthesis.blueprints.len());
    let frozen = match FrozenOperatorBlueprintSet::freeze(
        1,
        &competing.support_bundles,
        BlueprintBeamConfig::default(),
        &competing.synthesis,
    ) {
        Ok(frozen) => frozen,
        Err(error) => {
            increment_report_blocker(
                report,
                &format!("blueprint_freeze_{error:?}").to_lowercase(),
            );
            return;
        }
    };
    report.frozen_laws = report.frozen_laws.saturating_add(1);
    if law.future.len() < LIVE_SCALAR_FUTURE_ROWS {
        increment_report_blocker(report, "future_below_32");
        return;
    }
    let Ok(future) = law
        .future
        .iter()
        .map(|transition| {
            reextract_live_scalar_circuit_sample(transition, &law.support_actor_hypotheses)
        })
        .collect::<Result<Vec<_>, _>>()
    else {
        increment_report_blocker(report, "future_reextract_failed");
        return;
    };
    let future_evidence = future
        .iter()
        .map(|sample| {
            let verifier = source_neutral_verifier_for_program(&sample.actor_template)
                .map_err(|_| nando_core::wave::BlueprintFutureEvidenceError::EmptyRawInput)?;
            let actor_sha256 = parse_commitment(
                &response_actor_program_digest(&sample.actor_template)
                    .map_err(|_| nando_core::wave::BlueprintFutureEvidenceError::EmptyRawInput)?,
            )
            .ok_or(nando_core::wave::BlueprintFutureEvidenceError::EmptyRawInput)?;
            let verifier_sha256 = parse_commitment(
                &response_independent_verifier_program_digest(&verifier)
                    .map_err(|_| nando_core::wave::BlueprintFutureEvidenceError::EmptyRawInput)?,
            )
            .ok_or(nando_core::wave::BlueprintFutureEvidenceError::EmptyRawInput)?;
            BlueprintFutureEvidence::new_with_executable_contracts(
                sample.raw_input_sha256,
                sample.extractor_version.max(1),
                sample.bundle.clone(),
                actor_sha256,
                verifier_sha256,
            )
        })
        .collect::<Result<Vec<_>, _>>();
    let Ok(future_evidence) = future_evidence else {
        increment_report_blocker(report, "future_evidence_invalid");
        return;
    };
    let full = BlueprintFutureEvaluator::evaluate_and_seal(
        &frozen,
        &future_evidence,
        Default::default(),
        BlueprintPhaseControl::Full,
    );
    let Some(winner) = full.winner_receipt() else {
        let evaluation = full.report();
        let transform_clean = evaluation
            .scores
            .iter()
            .filter(|score| score.transform_mismatches == 0)
            .count();
        let transform_mismatches = evaluation
            .scores
            .iter()
            .map(|score| score.transform_mismatches)
            .sum::<usize>();
        let ambiguous_bindings = evaluation
            .scores
            .iter()
            .map(|score| score.ambiguous_bindings)
            .sum::<usize>();
        let executable_contract_mismatches = evaluation
            .scores
            .iter()
            .map(|score| score.executable_contract_mismatches)
            .sum::<usize>();
        let max_coherence = evaluation
            .scores
            .iter()
            .map(|score| score.whole_circuit_coherence_fixed)
            .max()
            .unwrap_or_default();
        increment_report_blocker(
            report,
            &format!(
                "full_phase_no_winner:{:?}:scores={}:transform_clean={transform_clean}:transform_mismatches={transform_mismatches}:contract_mismatches={executable_contract_mismatches}:ambiguous={ambiguous_bindings}:max_coherence={max_coherence}",
                evaluation.blocker,
                evaluation.scores.len(),
            )
            .to_lowercase(),
        );
        return;
    };
    let Some(actor_template) = competing
        .actors_by_blueprint
        .get(winner.winner_sha256())
        .cloned()
    else {
        increment_report_blocker(report, "winner_actor_contract_missing");
        return;
    };
    // A multi-role template is intentionally unbound. Executing it directly
    // would test support selectors against future surfaces and reject every
    // transferable operator. Crystallization below re-extracts and binds each
    // raw future surface, then independently repeats the binding in verifier.
    let direct_actor_mismatches = if rich_scalar_program_roles(&actor_template)
        .is_some_and(|roles| roles.len() > 1)
        || matches!(
            &actor_template.operation,
            ResponseOperation::FunctionCallFromRoles { .. }
                | ResponseOperation::CustomToolCallFromRoles { .. }
        ) {
        0
    } else {
        future
            .iter()
            .filter(|sample| {
                let Ok(provider_view) = crate::runtime::provider_payload_view(
                    &sample.request_text,
                    &sample.provider_payload,
                ) else {
                    return true;
                };
                execute_response(
                    &actor_template,
                    &sample.request_text,
                    provider_view.as_ref(),
                )
                .response
                .as_deref()
                    != Some(sample.expected_response.as_str())
            })
            .count()
    };
    if direct_actor_mismatches != 0 {
        increment_report_blocker(
            report,
            &format!("winner_actor_future_mismatches={direct_actor_mismatches}"),
        );
        return;
    }
    report.full_phase_winners = report.full_phase_winners.saturating_add(1);
    let controls_pass = [
        BlueprintPhaseControl::NoPhase,
        BlueprintPhaseControl::ShuffledPhase,
        BlueprintPhaseControl::MagnitudeOnly,
        BlueprintPhaseControl::MatchedRandomCenter,
    ]
    .into_iter()
    .all(|control| {
        BlueprintFutureEvaluator::evaluate_and_seal(
            &frozen,
            &future_evidence,
            Default::default(),
            control,
        )
        .winner_receipt()
        .is_none()
    });
    if !controls_pass {
        increment_report_blocker(report, "phase_control_selected_winner");
        return;
    }
    report.causal_control_passes = report.causal_control_passes.saturating_add(1);
    let mut future_window = frozen.future_window();
    for sample in &future {
        if future_window.admit_lineage(&sample.bundle).is_err() {
            increment_report_blocker(report, "future_lineage_rejected");
            return;
        }
    }
    let receipts = future
        .iter()
        .zip(&future_evidence)
        .map(|(sample, evidence)| CrystallizationParityReceipt {
            future_lineage_sha256: *sample.bundle.lineage_sha256(),
            future_surface_sha256: *sample.bundle.surface_sha256(),
            future_bundle_sha256: *evidence.bundle_sha256(),
            raw_input_sha256: sample.raw_input_sha256,
            extractor_version: sample.extractor_version.max(1),
            anchors: sample.anchors.clone(),
            request_text: sample.request_text.clone(),
            provider_payload: sample.provider_payload.clone(),
            expected_response: sample.expected_response.clone(),
        })
        .collect::<Vec<_>>();
    match CrystallizedOperator::crystallize_with_actor_template(
        &future_window,
        winner,
        &future_evidence,
        &receipts,
        actor_template,
    ) {
        Ok(operator) => {
            report.verified_shadow_operators = report.verified_shadow_operators.saturating_add(1);
            report.shadow_executions = report.shadow_executions.saturating_add(receipts.len());
            match live_admission_candidate(law, &operator) {
                Ok(candidate) => candidates.push(candidate),
                Err(blocker) => increment_report_blocker(report, &blocker),
            }
        }
        Err(error) => {
            increment_report_blocker(report, &format!("crystallization_{error:?}").to_lowercase())
        }
    }
}

fn build_competing_blueprint_set(
    support: &[LiveScalarCircuitSample],
) -> Result<CompetingBlueprintSet, String> {
    let actors = common_support_actor_hypotheses(support)?;
    let actor_hypothesis_count = actors.len();
    let mut support_bundles = Vec::new();
    let mut blueprints = BTreeMap::new();
    let mut actors_by_blueprint = BTreeMap::new();
    let mut blocker_counts = BTreeMap::new();
    let mut expansions = 0_usize;

    for actor in actors {
        let roles = rich_scalar_program_roles(&actor)
            .ok_or_else(|| "actor_hypothesis_roles_missing".to_owned())?;
        let transform_opcode = program_transform_opcode(&actor)
            .ok_or_else(|| "actor_hypothesis_opcode_missing".to_owned())?;
        let transform_flags = program_transform_flags(&actor)
            .ok_or_else(|| "actor_hypothesis_flags_missing".to_owned())?;
        let actor_bundles = support
            .iter()
            .map(|sample| {
                observed_rich_scalar_surface(
                    &sample.request_text,
                    &sample.provider_payload,
                    &roles,
                    transform_opcode,
                    transform_flags,
                    program_has_filter_count(&actor),
                    &commitment_hex(sample.bundle.surface_sha256()),
                    *sample.bundle.lineage_sha256(),
                    *sample.bundle.surface_sha256(),
                )
                .map(|observed| observed.bundle)
                .map_err(|error| format!("actor_support_bundle_{error:?}").to_lowercase())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let synthesis_bundles = actor_bundles
            .iter()
            .fold(BTreeMap::new(), |mut lineages, bundle| {
                lineages
                    .entry(*bundle.lineage_sha256())
                    .or_insert_with(|| bundle.clone());
                lineages
            })
            .into_values()
            .take(3)
            .collect::<Vec<_>>();
        if synthesis_bundles.len() < 3 {
            return Err("support_sessions_below_3".to_owned());
        }
        let alignments =
            BoundedRoleAligner::align(&synthesis_bundles, RoleAlignmentConfig::default());
        if !alignments.completion.is_complete() {
            return Err("role_alignment_exhausted".to_owned());
        }
        let synthesis = BoundedCircuitBeam::synthesize(
            &synthesis_bundles,
            &alignments,
            BlueprintBeamConfig::default(),
        );
        if !synthesis.completion.is_complete() {
            return Err("circuit_synthesis_exhausted".to_owned());
        }
        if synthesis.blueprints.is_empty() {
            return Err("circuit_synthesis_no_blueprint".to_owned());
        }
        expansions = expansions.saturating_add(synthesis.expansions);
        for blocker in &synthesis.blockers {
            let count = blocker_counts.entry(blocker.blocker).or_insert(0_usize);
            *count = count.saturating_add(blocker.count);
        }

        let verifier = source_neutral_verifier_for_program(&actor)
            .map_err(|error| format!("actor_verifier_build:{error}"))?;
        let actor_sha256 = parse_commitment(
            &response_actor_program_digest(&actor)
                .map_err(|error| format!("actor_digest:{error}"))?,
        )
        .ok_or_else(|| "actor_digest_invalid".to_owned())?;
        let verifier_sha256 = parse_commitment(
            &response_independent_verifier_program_digest(&verifier)
                .map_err(|error| format!("verifier_digest:{error}"))?,
        )
        .ok_or_else(|| "verifier_digest_invalid".to_owned())?;
        for blueprint in synthesis.blueprints {
            let blueprint = blueprint.bind_executable_contracts(actor_sha256, verifier_sha256);
            let fingerprint = *blueprint.fingerprint_sha256();
            if let Some(existing) = actors_by_blueprint.get(&fingerprint) {
                if existing != &actor {
                    return Err("blueprint_actor_commitment_collision".to_owned());
                }
            } else {
                actors_by_blueprint.insert(fingerprint, actor.clone());
            }
            blueprints.entry(fingerprint).or_insert(blueprint);
        }
        // Alternatives from one lineage are committed but never counted as
        // independent evidence; FrozenOperatorBlueprintSet deduplicates lineage.
        support_bundles.extend(actor_bundles);
    }

    if blueprints.is_empty() {
        return Err("competing_blueprints_empty".to_owned());
    }
    Ok(CompetingBlueprintSet {
        support_bundles,
        synthesis: BlueprintSynthesisReport {
            blueprints: blueprints
                .into_values()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            expansions,
            completion: SearchCompletion::Complete {
                explored: expansions,
            },
            blockers: blocker_counts
                .into_iter()
                .map(|(blocker, count)| BlueprintSynthesisBlockerCount { blocker, count })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        },
        actors_by_blueprint,
        actor_hypothesis_count,
    })
}

fn common_support_actor_hypotheses(
    support: &[LiveScalarCircuitSample],
) -> Result<Vec<ResponseProgram>, String> {
    let Some(first) = support.first() else {
        return Err("actor_hypotheses_missing".to_owned());
    };
    let mut common = first
        .actor_hypotheses
        .iter()
        .map(|program| {
            serde_cbor::to_vec(program)
                .map(|key| (key, program.clone()))
                .map_err(|_| "actor_hypothesis_encode_failed".to_owned())
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    for sample in &support[1..] {
        let keys = sample
            .actor_hypotheses
            .iter()
            .map(|program| {
                serde_cbor::to_vec(program).map_err(|_| "actor_hypothesis_encode_failed".to_owned())
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        common.retain(|key, _| keys.contains(key));
        if common.is_empty() {
            return Err("actor_hypotheses_no_common_version".to_owned());
        }
    }
    if common.len() > crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS {
        return Err("actor_hypothesis_class_budget".to_owned());
    }
    Ok(common.into_values().collect())
}

fn live_admission_candidate(
    law: &LiveScalarLawState,
    operator: &crate::VerifiedCrystallizedOperator,
) -> Result<LiveScalarAdmissionCandidate, String> {
    if law.support.len() < LIVE_SCALAR_SUPPORT_ROWS || law.future.len() < LIVE_SCALAR_FUTURE_ROWS {
        return Err("admission_rows_below_32".to_owned());
    }
    let program = operator
        .routing_program()
        .map_err(|_| "admission_routing_program_failed".to_owned())?;
    let verifier = operator
        .routing_verifier()
        .map_err(|_| "admission_routing_verifier_failed".to_owned())?;
    let verifier_schema = crate::response_program_external_verifier_schema(&program)
        .ok_or_else(|| "admission_verifier_schema_missing".to_owned())?;
    let distinct_sessions = law
        .support
        .iter()
        .chain(&law.future)
        .map(|row| row.before.session_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let distinct_surfaces = law
        .support
        .iter()
        .chain(&law.future)
        .map(|row| row.before.frame_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let package_id = format!(
        "crystallized-scalar-{}",
        &commitment_hex(operator.blueprint_sha256())[..16]
    );
    let route_margin = |row: &TeacherTransition| {
        let parity = row.runtime_parity_case.as_ref()?;
        let bound = operator
            .bind_pre_action(&parity.request_text, &parity.provider_payload)
            .ok()?;
        Some(operator.runtime_route_margin(&bound))
    };
    let wave_margin_micro = law
        .support
        .iter()
        .filter_map(&route_margin)
        .min()
        .ok_or_else(|| "admission_circuit_route_missing".to_owned())?;
    if law
        .future
        .iter()
        .any(|row| route_margin(row).is_none_or(|margin| margin < wave_margin_micro))
    {
        return Err("admission_circuit_route_future_mismatch".to_owned());
    }
    let package = ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id,
        origin: ResponsePackageOrigin::GroundedSynthesis,
        state: ResponsePackageState::Quarantine,
        program,
        verifier: Some(verifier),
        routing_predicates: Vec::new(),
        required_routing_atom_ids: Vec::new(),
        // The legacy vector is retained for package ABI validation. Runtime
        // authority comes from the restored circuit binding below, not from a
        // generic response-program atom masquerading as learned evidence.
        phase_centers: vec![operator.relation_program().fingerprint64()],
        anti_centers: Vec::new(),
        wave_margin_micro,
        learned_wave_route: None,
        crystallized_operator: Some(
            operator
                .restart_bundle()
                .map_err(|_| "admission_restart_bundle_failed".to_owned())?,
        ),
        proof: ResponsePackageProof {
            support_rows: law.support.len(),
            future_rows: law.future.len(),
            distinct_sessions,
            distinct_surfaces,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema: verifier_schema.to_owned(),
        },
    };
    package
        .validate()
        .map_err(|error| format!("admission_package_{error}"))?;
    Ok(LiveScalarAdmissionCandidate {
        package,
        support: law.support.clone(),
        future: law.future.clone(),
        support_root_sha256: commitment_hex(operator.support_root_sha256()),
        future_evidence_root_sha256: commitment_hex(operator.future_evidence_root_sha256()),
        future_lineage_root_sha256: commitment_hex(operator.future_lineage_root_sha256()),
        winner_seal_sha256: commitment_hex(operator.winner_seal_sha256()),
        executable_parity_seal_sha256: commitment_hex(operator.parity_seal().seal_sha256()),
    })
}

fn increment_report_blocker(report: &mut LiveScalarShadowReport, blocker: &str) {
    *report.blockers.entry(blocker.to_owned()).or_default() += 1;
}

/// Converts one completed, verified trace into source-neutral circuit evidence.
/// The teacher response selects a hypothesis only after the action; runtime
/// binding remains restricted to the pre-action provider payload.
pub fn extract_live_scalar_circuit_sample(
    transition: &TeacherTransition,
) -> Result<LiveScalarCircuitSample, LiveScalarShadowBlocker> {
    if !transition.outcome.verifier.accepted {
        return Err(LiveScalarShadowBlocker::TeacherRejected);
    }
    let parity = transition
        .runtime_parity_case
        .as_ref()
        .ok_or(LiveScalarShadowBlocker::MissingParityCase)?;
    let payload_bytes = serde_json::to_vec(&parity.provider_payload)
        .map_err(|_| LiveScalarShadowBlocker::PayloadSerializationFailed)?;
    if payload_bytes.len() > 64 * 1024 || parity.expected_response.len() > 4 * 1024 {
        return Err(LiveScalarShadowBlocker::PayloadTooLarge);
    }
    let synthesis_payload =
        synthesis_payload_with_request(&parity.request_text, &parity.provider_payload)?;
    let example = CollectionSynthesisExample {
        provider_payload: synthesis_payload.clone(),
        expected_response: parity.expected_response.clone(),
    };
    let version_space = enumerate_source_neutral_response_programs(&example).map_err(|error| {
        if error == "collection_candidate_budget" {
            LiveScalarShadowBlocker::CandidateBudgetExhausted
        } else {
            LiveScalarShadowBlocker::ProgramEnumerationFailed
        }
    })?;
    let teacher_programs = teacher_action_programs(transition, parity, &synthesis_payload);
    let teacher_programs = match teacher_programs {
        Ok(programs) => programs,
        Err(blocker) if version_space.programs.is_empty() => return Err(blocker),
        Err(_) => Vec::new(),
    };
    if version_space.programs.is_empty() && teacher_programs.is_empty() {
        return Err(LiveScalarShadowBlocker::EmptyVersionSpace);
    }
    let exact_count = version_space
        .programs
        .iter()
        .filter_map(|program| {
            derive_exact_count_program(
                program,
                &parity.request_text,
                &synthesis_payload,
                &parity.expected_response,
            )
        })
        .min_by_key(|program| serde_cbor::to_vec(program).unwrap_or_default());
    let exact_status = version_space
        .programs
        .iter()
        .filter_map(|program| {
            derive_exact_status_program(
                program,
                &parity.request_text,
                &synthesis_payload,
                &parity.expected_response,
            )
        })
        .min_by_key(|program| serde_cbor::to_vec(program).unwrap_or_default());
    let exact_filters = version_space
        .programs
        .iter()
        .flat_map(|program| {
            derive_exact_filter_programs(
                program,
                &parity.request_text,
                &synthesis_payload,
                &parity.expected_response,
            )
        })
        .collect::<Vec<_>>();
    let exact_filter = exact_filters
        .iter()
        .min_by_key(|program| serde_cbor::to_vec(program).unwrap_or_default());
    let mut scalar_programs = version_space
        .programs
        .iter()
        .filter_map(|program| {
            derive_exact_scalar_program(
                program,
                &parity.request_text,
                &parity.provider_payload,
                &parity.expected_response,
            )
        })
        .filter_map(project_scalar_program)
        .collect::<Vec<_>>();
    scalar_programs.sort_by(|left, right| left.0.cmp(&right.0));
    let rich_exact = version_space
        .programs
        .iter()
        .find(|program| rich_scalar_program_roles(program).is_some_and(|roles| roles.len() > 1));
    let selected_template = if let Some(program) = teacher_programs.first() {
        program.clone()
    } else if let Some(program) = &exact_count {
        program.clone()
    } else if let Some(program) = &exact_status {
        program.clone()
    } else if let Some(program) = exact_filter {
        program.clone()
    } else if let Some(program) = rich_exact {
        program.clone()
    } else if let Some((_, selector, _, _, renderer)) = scalar_programs.first() {
        ResponseProgram::project_selected_value(selector.clone(), scalar_programs[0].3, "completed")
            .with_value_renderer(renderer.clone())
    } else {
        version_space
            .programs
            .iter()
            .find(|program| rich_scalar_program_roles(program).is_some())
            .cloned()
            .ok_or_else(|| classify_exact_program_blocker(&version_space.programs))?
    };
    let exact_typed = exact_count.as_ref().or(exact_status.as_ref());
    let mut actor_hypotheses = if !teacher_programs.is_empty() {
        teacher_programs
            .iter()
            .filter_map(|program| {
                canonicalize_scalar_program_roles(
                    program,
                    &parity.request_text,
                    &parity.provider_payload,
                )
            })
            .collect()
    } else if !exact_filters.is_empty() {
        exact_filters
            .iter()
            .filter_map(|program| {
                canonicalize_scalar_program_roles(
                    program,
                    &parity.request_text,
                    &parity.provider_payload,
                )
            })
            .collect()
    } else if let Some(program) = exact_typed {
        vec![
            canonicalize_scalar_program_roles(
                program,
                &parity.request_text,
                &parity.provider_payload,
            )
            .ok_or(LiveScalarShadowBlocker::ExactTypedCanonicalizationFailed)?,
        ]
    } else {
        canonical_rich_actor_hypotheses(
            &version_space.programs,
            &parity.request_text,
            &parity.provider_payload,
            &parity.expected_response,
        )?
    };
    let clean_actor = (exact_typed.is_none() && exact_filters.is_empty())
        .then(|| {
            derive_clean_ordinal_actor(
                &parity.request_text,
                &parity.provider_payload,
                &parity.expected_response,
            )
        })
        .flatten();
    if let Some(actor) = &clean_actor {
        actor_hypotheses.push(actor.clone());
    }
    if actor_hypotheses.is_empty() {
        actor_hypotheses.push(
            canonicalize_scalar_program_roles(
                &selected_template,
                &parity.request_text,
                &parity.provider_payload,
            )
            .ok_or(LiveScalarShadowBlocker::SelectedTemplateCanonicalizationFailed)?,
        );
    }
    actor_hypotheses = expand_actor_hypothesis_set(
        actor_hypotheses,
        &parity.request_text,
        &parity.provider_payload,
        &parity.expected_response,
    )?;
    let canonical_candidates = if let Some(clean_actor) = clean_actor {
        expand_actor_hypothesis_set(
            vec![clean_actor],
            &parity.request_text,
            &parity.provider_payload,
            &parity.expected_response,
        )?
    } else {
        actor_hypotheses.clone()
    };
    let canonical_program = canonical_candidates
        .iter()
        .max_by_key(|program| rich_scalar_program_roles(program).map_or(0, |roles| roles.len()))
        .cloned()
        .ok_or(LiveScalarShadowBlocker::CanonicalCandidateMissing)?;
    finish_live_scalar_circuit_sample(
        transition,
        parity,
        payload_bytes,
        canonical_program,
        actor_hypotheses,
    )
}

fn reextract_live_scalar_circuit_sample(
    transition: &TeacherTransition,
    support_actor_hypotheses: &[ResponseProgram],
) -> Result<LiveScalarCircuitSample, LiveScalarShadowBlocker> {
    if !transition.outcome.verifier.accepted {
        return Err(LiveScalarShadowBlocker::TeacherRejected);
    }
    let parity = transition
        .runtime_parity_case
        .as_ref()
        .ok_or(LiveScalarShadowBlocker::MissingParityCase)?;
    let payload_bytes = serde_json::to_vec(&parity.provider_payload)
        .map_err(|_| LiveScalarShadowBlocker::PayloadSerializationFailed)?;
    if payload_bytes.len() > 64 * 1024 || parity.expected_response.len() > 4 * 1024 {
        return Err(LiveScalarShadowBlocker::PayloadTooLarge);
    }
    let actor_hypotheses = support_actor_hypotheses
        .iter()
        .filter(|program| {
            execute_response(program, &parity.request_text, &parity.provider_payload)
                .response
                .as_deref()
                .is_some_and(|response| {
                    response == parity.expected_response
                        || crate::online_admission::responses_match_after_execution_budget_normalization(
                            response,
                            &parity.expected_response,
                        )
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    let canonical_program = actor_hypotheses
        .iter()
        .max_by_key(|program| rich_scalar_program_roles(program).map_or(0, |roles| roles.len()))
        .cloned()
        .ok_or(LiveScalarShadowBlocker::NoExactSourceNeutralProgram)?;
    finish_live_scalar_circuit_sample(
        transition,
        parity,
        payload_bytes,
        canonical_program,
        actor_hypotheses,
    )
}

fn finish_live_scalar_circuit_sample(
    transition: &TeacherTransition,
    parity: &crate::RuntimeParityCase,
    payload_bytes: Vec<u8>,
    canonical_program: ResponseProgram,
    actor_hypotheses: Vec<ResponseProgram>,
) -> Result<LiveScalarCircuitSample, LiveScalarShadowBlocker> {
    // The relation circuit and executable actor must be derived from the same
    // canonical winner program; otherwise runtime roles cannot match support.
    let roles = rich_scalar_program_roles(&canonical_program)
        .ok_or_else(|| classify_exact_program_blocker(&actor_hypotheses))?;

    let lineage_sha256 = parse_commitment(&transition.before.session_id_sha256)
        .or_else(|| parse_commitment(&transition.before.client_intent_id_sha256))
        .ok_or(LiveScalarShadowBlocker::InvalidCommitment)?;
    let surface_sha256 = digest_parts(
        b"nando.live-scalar-surface.v2",
        &[
            transition.before.frame_id_sha256.as_bytes(),
            transition.before.extractor_version.as_bytes(),
            &payload_bytes,
        ],
    );
    let observed = observed_rich_scalar_surface(
        &parity.request_text,
        &parity.provider_payload,
        &roles,
        program_transform_opcode(&canonical_program)
            .ok_or(LiveScalarShadowBlocker::UnsupportedTransformOpcode)?,
        program_transform_flags(&canonical_program)
            .ok_or(LiveScalarShadowBlocker::UnsupportedTransformFlags)?,
        program_has_filter_count(&canonical_program),
        &transition.before.frame_id_sha256,
        lineage_sha256,
        surface_sha256,
    )?;
    let raw_input_sha256 = digest_parts(
        b"nando.live-scalar-raw-input.v1",
        &[parity.request_text.as_bytes(), &payload_bytes],
    );
    let law_shape = structural_scalar_law_shape(
        &parity.request_text,
        &parity.provider_payload,
        &parity.expected_response,
    )
    .or_else(|| {
        (roles.len() == 1
            || program_transform_opcode(&canonical_program)
                == Some(TRANSFORM_OPCODE_FILTER_REQUEST_VALUE))
        .then(|| source_neutral_scalar_program_shape(&canonical_program))
        .flatten()
    })
    .ok_or(LiveScalarShadowBlocker::LawShapeMissing)?;
    let law_sha256 = digest_parts(b"nando.live-scalar-law.v4", &[&law_shape]);
    Ok(LiveScalarCircuitSample {
        bundle: observed.bundle,
        anchors: observed.anchors,
        actor_template: canonical_program,
        actor_hypotheses: actor_hypotheses.into_boxed_slice(),
        request_text: parity.request_text.clone(),
        provider_payload: parity.provider_payload.clone(),
        expected_response: parity.expected_response.clone(),
        raw_input_sha256,
        extractor_version: extractor_version(&transition.before.extractor_version),
        law_sha256,
    })
}

fn teacher_action_programs(
    transition: &TeacherTransition,
    parity: &crate::RuntimeParityCase,
    provider_payload: &Value,
) -> Result<Vec<ResponseProgram>, LiveScalarShadowBlocker> {
    let frame = transition.as_training_relation_frame();
    let synthesized = synthesize_response_operator(std::slice::from_ref(&frame)).map_err(
        |error| match error {
            crate::SynthesisError::EmptySupport => {
                LiveScalarShadowBlocker::TeacherSynthesisEmptySupport
            }
            crate::SynthesisError::AmbiguousRoles => {
                LiveScalarShadowBlocker::TeacherSynthesisAmbiguousRoles
            }
            crate::SynthesisError::InconsistentRoleFamily => {
                LiveScalarShadowBlocker::TeacherSynthesisInconsistentRoleFamily
            }
            crate::SynthesisError::MissingPendingState => {
                LiveScalarShadowBlocker::TeacherSynthesisMissingPendingState
            }
            crate::SynthesisError::MissingCompletionState => {
                LiveScalarShadowBlocker::TeacherSynthesisMissingCompletionState
            }
            crate::SynthesisError::MissingUniqueHandle => {
                LiveScalarShadowBlocker::TeacherSynthesisMissingUniqueHandle
            }
            crate::SynthesisError::NoConsistentProgram => {
                LiveScalarShadowBlocker::TeacherSynthesisNoConsistentProgram
            }
            crate::SynthesisError::AmbiguousPrograms => {
                LiveScalarShadowBlocker::TeacherSynthesisAmbiguousPrograms
            }
        },
    )?;
    let program = canonicalize_call_execution_arguments(synthesized.candidate.program);
    if !matches!(
        &program.operation,
        ResponseOperation::FunctionCallFromRoles { .. }
            | ResponseOperation::CustomToolCallFromRoles { .. }
    ) {
        return Err(LiveScalarShadowBlocker::TeacherProgramNotCall);
    }
    let selector_type = rich_scalar_program_roles(&program)
        .and_then(|roles| roles.first().map(|role| selector_value_type(&role.0)))
        .ok_or(LiveScalarShadowBlocker::RoleTypeInferenceFailed)?;
    let mut candidates = vec![program.clone()];
    let teacher_alignment = match teacher_call_role_value(&program, &parity.expected_response) {
        Some(teacher_value) => {
            match crate::runtime::structural_output_selectors_for_teacher_value(
                provider_payload,
                &teacher_value,
                selector_type,
            ) {
                Ok(selectors) => {
                    candidates.extend(
                        selectors
                            .into_iter()
                            .filter_map(|selector| replace_call_selector(&program, selector)),
                    );
                    LiveScalarShadowBlocker::TeacherProgramRoleValueCandidateMismatch
                }
                Err(_) => teacher_role_value_location(
                    &parity.request_text,
                    provider_payload,
                    &teacher_value,
                ),
            }
        }
        None => LiveScalarShadowBlocker::TeacherProgramRoleValueUnavailable,
    };
    if let ResponseOperation::FunctionCallFromRoles { selector, .. }
    | ResponseOperation::CustomToolCallFromRoles { selector, .. } = &program.operation
        && let Some((field, value_type)) = teacher_field_selector_hint(selector)
        && let Ok(selectors) = crate::runtime::structural_output_selectors_for_field_hint(
            provider_payload,
            field,
            value_type,
        )
    {
        candidates.extend(
            selectors
                .into_iter()
                .filter_map(|selector| replace_call_selector(&program, selector)),
        );
    }
    candidates.extend(
        crate::collection_synthesis::learned_selector_candidates(provider_payload)
            .into_iter()
            .filter(|selector| {
                selector_value_type(selector) == selector_type
                    && teacher_call_selector_is_structural(selector)
            })
            .filter_map(|selector| replace_call_selector(&program, selector)),
    );
    // Collection synthesis intentionally caps broad ordinal expansion at 16.
    // Tool-call outputs can contain larger diagnostic objects, while the
    // continuation role can remain outside the broad 16-scalar search.
    // Extend only this typed call version space, preferring tail positions
    // where continuation handles commonly occur, without using field names.
    for ordinal in 16_u16..64 {
        for selector in [
            ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                reverse_ordinal: ordinal,
                value_type: selector_type,
            },
            ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                scalar_ordinal: ordinal,
                value_type: selector_type,
            },
        ] {
            if let Some(candidate) = replace_call_selector(&program, selector) {
                candidates.push(candidate);
            }
        }
    }
    let mut exact = BTreeMap::new();
    let mut evaluated = BTreeSet::new();
    let mut first_execution = None;
    let mut first_structural_response = None;
    for candidate in candidates {
        let candidate_key = serde_cbor::to_vec(&candidate)
            .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
        if !evaluated.insert(candidate_key.clone()) {
            continue;
        }
        if evaluated.len() > TEACHER_CALL_SELECTOR_BUDGET {
            return Err(LiveScalarShadowBlocker::CandidateBudgetExhausted);
        }
        let execution = execute_response(&candidate, &parity.request_text, provider_payload);
        first_execution.get_or_insert_with(|| (candidate.clone(), execution.clone()));
        if let Some(response) = &execution.response {
            first_structural_response.get_or_insert_with(|| (candidate.clone(), response.clone()));
        }
        if !execution.response.as_deref().is_some_and(|response| {
            response == parity.expected_response
                || crate::online_admission::responses_match_after_execution_budget_normalization(
                    response,
                    &parity.expected_response,
                )
        }) {
            continue;
        }
        exact.entry(candidate_key).or_insert(candidate);
        // One surface may expose the same semantic role through many physical
        // paths. Preserve the bounded adapter version space here; the support
        // intersection must shrink it to the 64-variant authority limit.
        if exact.len() > TEACHER_CALL_SELECTOR_BUDGET {
            return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
        }
    }
    if exact.is_empty() {
        if let Some((candidate, response)) = first_structural_response {
            let blocker = teacher_program_parity_blocker(
                &parity.expected_response,
                &response,
                Some(&candidate),
            );
            return Err(dynamic_role_alignment_blocker(blocker, teacher_alignment));
        }
        let (candidate, execution) =
            first_execution.ok_or(LiveScalarShadowBlocker::TeacherProgramRuntimeAbstain)?;
        return Err(match execution.response {
            Some(response) => dynamic_role_alignment_blocker(
                teacher_program_parity_blocker(
                    &parity.expected_response,
                    &response,
                    Some(&candidate),
                ),
                teacher_alignment,
            ),
            None => teacher_program_runtime_blocker(&execution.reason),
        });
    }
    Ok(exact.into_values().collect())
}

fn canonicalize_call_execution_arguments(mut program: ResponseProgram) -> ResponseProgram {
    let arguments = match &mut program.operation {
        ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | ResponseOperation::CustomToolCallFromRoles { arguments, .. } => arguments,
        _ => return program,
    };
    arguments.retain(|argument| match argument {
        ResponseArgument::Integer { name, .. } => {
            !crate::teacher_join::is_execution_budget_argument(name)
        }
        ResponseArgument::String { name, value } => !(name == "chars" && value.is_empty()),
        _ => true,
    });
    program
}

fn dynamic_role_alignment_blocker(
    blocker: LiveScalarShadowBlocker,
    alignment: LiveScalarShadowBlocker,
) -> LiveScalarShadowBlocker {
    if matches!(
        blocker,
        LiveScalarShadowBlocker::TeacherProgramDynamicRoleNumericMismatch
            | LiveScalarShadowBlocker::TeacherProgramDynamicRoleStringMismatch
    ) {
        alignment
    } else {
        blocker
    }
}

fn teacher_call_role_value(program: &ResponseProgram, expected_response: &str) -> Option<Value> {
    let response = serde_json::from_str::<Value>(expected_response).ok()?;
    let arguments = if let Some(input) = response.get("input").and_then(Value::as_str) {
        call_input_arguments(input)?
    } else {
        response.get("arguments")?.clone()
    };
    let arguments = arguments.as_object()?;
    let role_name = match &program.operation {
        ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | ResponseOperation::CustomToolCallFromRoles { arguments, .. } => {
            arguments.iter().find_map(|argument| match argument {
                ResponseArgument::Role { name, .. } => Some(name.as_str()),
                _ => None,
            })?
        }
        _ => return None,
    };
    arguments.get(role_name).cloned()
}

fn teacher_role_value_location(
    request_text: &str,
    provider_payload: &Value,
    teacher_value: &Value,
) -> LiveScalarShadowBlocker {
    let Some(text) = scalar_value_text(teacher_value) else {
        return LiveScalarShadowBlocker::TeacherProgramRoleValueNotObserved;
    };
    if contains_bounded_token(request_text, &text) {
        return LiveScalarShadowBlocker::TeacherProgramRoleValueInRequestText;
    }
    if payload_contains_exact_scalar(provider_payload, teacher_value) {
        return LiveScalarShadowBlocker::TeacherProgramRoleValueInPayloadScalar;
    }
    if payload_contains_text_token(provider_payload, &text) {
        return LiveScalarShadowBlocker::TeacherProgramRoleValueInPayloadText;
    }
    LiveScalarShadowBlocker::TeacherProgramRoleValueAbsentFromPayload
}

fn scalar_value_text(value: &Value) -> Option<String> {
    match value {
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) if value.is_i64() || value.is_u64() => Some(value.to_string()),
        Value::String(value) if !value.is_empty() && value.len() <= 512 => Some(value.clone()),
        _ => None,
    }
}

fn payload_contains_exact_scalar(value: &Value, target: &Value) -> bool {
    if value == target {
        return true;
    }
    match value {
        Value::Array(values) => values
            .iter()
            .any(|value| payload_contains_exact_scalar(value, target)),
        Value::Object(values) => values
            .values()
            .any(|value| payload_contains_exact_scalar(value, target)),
        _ => false,
    }
}

fn payload_contains_text_token(value: &Value, target: &str) -> bool {
    match value {
        Value::String(value) => contains_bounded_token(value, target),
        Value::Array(values) => values
            .iter()
            .any(|value| payload_contains_text_token(value, target)),
        Value::Object(values) => values
            .values()
            .any(|value| payload_contains_text_token(value, target)),
        _ => false,
    }
}

fn contains_bounded_token(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() || needle.len() > 512 || haystack.len() > 64 * 1024 {
        return false;
    }
    haystack.match_indices(needle).any(|(start, _)| {
        let end = start.saturating_add(needle.len());
        let left = haystack[..start].chars().next_back();
        let right = haystack[end..].chars().next();
        !left.is_some_and(is_identifier_character) && !right.is_some_and(is_identifier_character)
    })
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':' | '/')
}

// Field labels are post-action teacher hints only; crystallized actors retain
// the ordinal selector derived from the observed output, never this label.
fn teacher_field_selector_hint(selector: &ResponseValueSelector) -> Option<(&str, AtomValueType)> {
    match selector {
        ResponseValueSelector::JsonField { field, value_type }
        | ResponseValueSelector::UniqueTurnJsonField { field, value_type }
        | ResponseValueSelector::UniqueActiveTurnJsonField { field, value_type } => {
            Some((field, *value_type))
        }
        _ => None,
    }
}

fn replace_call_selector(
    program: &ResponseProgram,
    selector: ResponseValueSelector,
) -> Option<ResponseProgram> {
    let mut candidate = program.clone();
    match &mut candidate.operation {
        ResponseOperation::FunctionCallFromRoles {
            selector: current, ..
        }
        | ResponseOperation::CustomToolCallFromRoles {
            selector: current, ..
        } => *current = selector,
        _ => return None,
    }
    Some(candidate)
}

fn teacher_call_selector_is_structural(selector: &ResponseValueSelector) -> bool {
    !matches!(
        selector,
        ResponseValueSelector::JsonField { .. }
            | ResponseValueSelector::UniqueTurnJsonField { .. }
            | ResponseValueSelector::UniqueActiveTurnJsonField { .. }
    )
}

fn teacher_program_runtime_blocker(reason: &str) -> LiveScalarShadowBlocker {
    if reason.starts_with("verification:") {
        return LiveScalarShadowBlocker::TeacherProgramVerificationFailed;
    }
    if reason.starts_with("invalid_program:") {
        return LiveScalarShadowBlocker::TeacherProgramInvalid;
    }
    match reason {
        "immediate_tool_output_missing" => {
            LiveScalarShadowBlocker::TeacherProgramImmediateToolOutputMissing
        }
        "selector_prefix_missing" => LiveScalarShadowBlocker::TeacherProgramSelectorPrefixMissing,
        "selector_prefix_ambiguous" => {
            LiveScalarShadowBlocker::TeacherProgramSelectorPrefixAmbiguous
        }
        "selector_type_mismatch" => LiveScalarShadowBlocker::TeacherProgramSelectorTypeMismatch,
        "selector_integer_parse"
        | "selector_boolean_parse"
        | "selector_collection_parse"
        | "selector_collection_unsupported"
        | "selector_scalar_unsupported" => {
            LiveScalarShadowBlocker::TeacherProgramSelectorParseFailed
        }
        "role_integer_parse"
        | "role_boolean_parse"
        | "role_string_parse"
        | "role_collection_unsupported"
        | "unsupported_runtime_role" => LiveScalarShadowBlocker::TeacherProgramRoleParseFailed,
        "scalar_output_budget"
        | "tool_output_not_text"
        | "output_part_cardinality"
        | "unsupported_output_part_type"
        | "output_part_text_missing" => LiveScalarShadowBlocker::TeacherProgramOutputTextInvalid,
        _ => LiveScalarShadowBlocker::TeacherProgramRuntimeAbstain,
    }
}

fn teacher_program_parity_blocker(
    expected: &str,
    actual: &str,
    program: Option<&ResponseProgram>,
) -> LiveScalarShadowBlocker {
    let (Ok(expected), Ok(actual)) = (
        serde_json::from_str::<Value>(expected),
        serde_json::from_str::<Value>(actual),
    ) else {
        return LiveScalarShadowBlocker::TeacherProgramParityMismatch;
    };
    let (Some(expected), Some(actual)) = (expected.as_object(), actual.as_object()) else {
        return LiveScalarShadowBlocker::TeacherProgramResponseShapeMismatch;
    };
    let expected_kind = expected.get("kind").or_else(|| expected.get("type"));
    let actual_kind = actual.get("kind").or_else(|| actual.get("type"));
    if expected.get("name") != actual.get("name") {
        return LiveScalarShadowBlocker::TeacherProgramSymbolMismatch;
    }
    if expected.get("input") != actual.get("input") {
        let (Some(expected_input), Some(actual_input)) = (
            expected.get("input").and_then(Value::as_str),
            actual.get("input").and_then(Value::as_str),
        ) else {
            return LiveScalarShadowBlocker::TeacherProgramInputMismatch;
        };
        if expected_input
            .chars()
            .filter(|character| !character.is_ascii_whitespace())
            .eq(actual_input
                .chars()
                .filter(|character| !character.is_ascii_whitespace()))
        {
            return LiveScalarShadowBlocker::TeacherProgramInputWhitespaceMismatch;
        }
        if wire_token_shape(expected_input) == wire_token_shape(actual_input) {
            if let Some(blocker) = program.and_then(|program| {
                classify_call_argument_mismatch(expected_input, actual_input, program)
            }) {
                return blocker;
            }
            return classify_wire_token_value_mismatch(expected_input, actual_input);
        }
        return LiveScalarShadowBlocker::TeacherProgramInputSyntaxShapeMismatch;
    }
    if expected_kind == actual_kind {
        return LiveScalarShadowBlocker::TeacherProgramWireEnvelopeMismatch;
    }
    LiveScalarShadowBlocker::TeacherProgramResponseShapeMismatch
}

fn classify_call_argument_mismatch(
    expected: &str,
    actual: &str,
    program: &ResponseProgram,
) -> Option<LiveScalarShadowBlocker> {
    let expected = call_input_arguments(expected)?;
    let actual = call_input_arguments(actual)?;
    let expected = expected.as_object()?;
    let actual = actual.as_object()?;
    let differing = expected
        .iter()
        .filter(|(name, value)| actual.get(*name) != Some(*value))
        .map(|(name, value)| (name.as_str(), value))
        .collect::<Vec<_>>();
    if differing.len() != 1 || expected.len() != actual.len() {
        return None;
    }
    let (name, expected_value) = differing[0];
    let arguments = match &program.operation {
        ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | ResponseOperation::CustomToolCallFromRoles { arguments, .. } => arguments,
        _ => return None,
    };
    arguments.iter().find_map(|argument| match argument {
        ResponseArgument::Role {
            name: argument_name,
            ..
        } if argument_name == name && expected_value.is_number() => {
            Some(LiveScalarShadowBlocker::TeacherProgramDynamicRoleNumericMismatch)
        }
        ResponseArgument::Role {
            name: argument_name,
            ..
        } if argument_name == name && expected_value.is_string() => {
            Some(LiveScalarShadowBlocker::TeacherProgramDynamicRoleStringMismatch)
        }
        ResponseArgument::Integer {
            name: argument_name,
            ..
        } if argument_name == name => {
            Some(LiveScalarShadowBlocker::TeacherProgramStaticIntegerMismatch)
        }
        ResponseArgument::String {
            name: argument_name,
            ..
        } if argument_name == name => {
            Some(LiveScalarShadowBlocker::TeacherProgramStaticStringMismatch)
        }
        _ => None,
    })
}

fn call_input_arguments(input: &str) -> Option<Value> {
    if let Ok(arguments) = serde_json::from_str::<Value>(input)
        && arguments.is_object()
    {
        return Some(arguments);
    }
    let tool = input.find("tools.")?;
    let arguments = input[tool..].find('(')?.saturating_add(tool + 1);
    serde_json::Deserializer::from_str(&input[arguments..])
        .into_iter::<Value>()
        .next()?
        .ok()
        .filter(Value::is_object)
}

fn classify_wire_token_value_mismatch(expected: &str, actual: &str) -> LiveScalarShadowBlocker {
    let expected = wire_tokens(expected);
    let actual = wire_tokens(actual);
    if expected.len() != actual.len() {
        return LiveScalarShadowBlocker::TeacherProgramInputTokenValueMismatch;
    }
    let mut numeric = 0_usize;
    let mut quoted = 0_usize;
    let mut identifiers = 0_usize;
    for ((expected_kind, expected_value), (actual_kind, actual_value)) in
        expected.iter().zip(&actual)
    {
        if expected_kind != actual_kind || expected_value == actual_value {
            continue;
        }
        match expected_kind {
            b'N' => numeric = numeric.saturating_add(1),
            b'Q' => quoted = quoted.saturating_add(1),
            b'A' => identifiers = identifiers.saturating_add(1),
            _ => return LiveScalarShadowBlocker::TeacherProgramInputMixedTokenMismatch,
        }
    }
    match (numeric, quoted, identifiers) {
        (1, 0, 0) => LiveScalarShadowBlocker::TeacherProgramInputSingleNumericMismatch,
        (2.., 0, 0) => LiveScalarShadowBlocker::TeacherProgramInputMultipleNumericMismatch,
        (0, 1.., 0) => LiveScalarShadowBlocker::TeacherProgramInputQuotedLiteralMismatch,
        (0, 0, 1..) => LiveScalarShadowBlocker::TeacherProgramInputIdentifierMismatch,
        (0, 0, 0) => LiveScalarShadowBlocker::TeacherProgramInputTokenValueMismatch,
        _ => LiveScalarShadowBlocker::TeacherProgramInputMixedTokenMismatch,
    }
}

fn wire_tokens(value: &str) -> Vec<(u8, String)> {
    let mut tokens = Vec::new();
    let chars = value.char_indices().collect::<Vec<_>>();
    let mut index = 0_usize;
    while index < chars.len() {
        let (start, character) = chars[index];
        if character.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if character == '"' || character == '\'' {
            let quote = character;
            let mut escaped = false;
            let mut end = value.len();
            index += 1;
            while index < chars.len() {
                let (offset, next) = chars[index];
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    end = offset + next.len_utf8();
                    index += 1;
                    break;
                }
                index += 1;
            }
            tokens.push((b'Q', value[start..end].to_owned()));
            continue;
        }
        if character.is_ascii_digit() {
            index += 1;
            while index < chars.len() && chars[index].1.is_ascii_digit() {
                index += 1;
            }
            let end = chars.get(index).map_or(value.len(), |value| value.0);
            tokens.push((b'N', value[start..end].to_owned()));
            continue;
        }
        if character.is_ascii_alphabetic() || character == '_' {
            index += 1;
            while index < chars.len()
                && (chars[index].1.is_ascii_alphanumeric() || chars[index].1 == '_')
            {
                index += 1;
            }
            let end = chars.get(index).map_or(value.len(), |value| value.0);
            tokens.push((b'A', value[start..end].to_owned()));
            continue;
        }
        index += 1;
    }
    tokens
}

fn wire_token_shape(value: &str) -> Vec<u8> {
    let mut shape = Vec::with_capacity(value.len().min(4_096));
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '"' || character == '\'' {
            let quote = character;
            let mut escaped = false;
            for next in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    break;
                }
            }
            shape.push(b'Q');
        } else if character.is_ascii_digit() {
            while chars.peek().is_some_and(|next| next.is_ascii_digit()) {
                chars.next();
            }
            shape.push(b'N');
        } else if character.is_ascii_alphabetic() || character == '_' {
            while chars
                .peek()
                .is_some_and(|next| next.is_ascii_alphanumeric() || *next == '_')
            {
                chars.next();
            }
            shape.push(b'A');
        } else if character.is_ascii() {
            shape.push(character as u8);
        } else {
            shape.push(b'U');
        }
        if shape.len() >= 4_096 {
            break;
        }
    }
    shape
}

fn canonical_rich_actor_hypotheses(
    programs: &[ResponseProgram],
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Result<Vec<ResponseProgram>, LiveScalarShadowBlocker> {
    let mut hypotheses = BTreeMap::new();
    for program in programs {
        if !rich_scalar_program_roles(program).is_some_and(|roles| roles.len() > 1) {
            continue;
        }
        let Some(canonical) =
            canonicalize_scalar_program_roles(program, request_text, provider_payload)
        else {
            continue;
        };
        let Some(roles) = rich_scalar_program_roles(&canonical) else {
            continue;
        };
        let distinct = roles
            .iter()
            .map(|(selector, _)| selector)
            .collect::<BTreeSet<_>>();
        if distinct.len() != roles.len()
            || roles.iter().any(|(selector, _)| {
                !matches!(
                    selector,
                    ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. }
                )
            })
            || execute_response(&canonical, request_text, provider_payload)
                .response
                .as_deref()
                != Some(expected_response)
        {
            continue;
        }
        let key = serde_cbor::to_vec(&canonical)
            .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
        hypotheses.entry(key).or_insert(canonical);
        if hypotheses.len() > TEACHER_CALL_SELECTOR_BUDGET {
            return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
        }
    }
    Ok(hypotheses.into_values().collect())
}

fn derive_clean_ordinal_actor(
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let observed =
        crate::runtime::observed_request_ordinal_roles(request_text, provider_payload).ok()?;
    let mut selectors_by_value = BTreeMap::<String, Vec<ResponseValueSelector>>::new();
    for role in observed {
        let value = execute_response(
            &ResponseProgram::project_selected_value(
                role.selector.clone(),
                ValueProjectionFormat::PlainText,
                "completed",
            ),
            request_text,
            provider_payload,
        )
        .response?;
        if value.is_empty() {
            return None;
        }
        selectors_by_value
            .entry(value)
            .or_default()
            .push(role.selector);
    }
    let mut spans = selectors_by_value
        .keys()
        .flat_map(|value| {
            expected_response
                .match_indices(value)
                .map(move |(start, _)| (start, start.saturating_add(value.len()), value.clone()))
        })
        .collect::<Vec<_>>();
    spans.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    let mut selected_spans = Vec::new();
    let mut cursor = 0_usize;
    for span in spans {
        if span.0 < cursor {
            continue;
        }
        cursor = span.1;
        selected_spans.push(span);
    }
    if selected_spans.is_empty()
        || selectors_by_value
            .keys()
            .any(|value| !selected_spans.iter().any(|span| &span.2 == value))
    {
        return None;
    }
    let primary_selector = selectors_by_value
        .get(&selected_spans[0].2)?
        .first()?
        .clone();
    let primary_format = ValueProjectionFormat::PlainText;
    let mut segments = Vec::new();
    let mut rendered_until = 0_usize;
    for (start, end, value) in selected_spans {
        if start > rendered_until {
            segments.push(ResponseRenderSegment::Static {
                text: expected_response[rendered_until..start].to_owned(),
            });
        }
        let selector = selectors_by_value.get(&value)?.first()?.clone();
        if selector == primary_selector {
            segments.push(ResponseRenderSegment::Primary);
        } else {
            segments.push(ResponseRenderSegment::Selected {
                selector,
                format: ValueProjectionFormat::PlainText,
            });
        }
        rendered_until = end;
    }
    if rendered_until < expected_response.len() {
        segments.push(ResponseRenderSegment::Static {
            text: expected_response[rendered_until..].to_owned(),
        });
    }
    let actor =
        ResponseProgram::project_selected_value(primary_selector, primary_format, "completed")
            .with_value_renderer(CollectionOutputRenderer::RenderSequence { segments });
    (execute_response(&actor, request_text, provider_payload)
        .response
        .as_deref()
        == Some(expected_response))
    .then_some(actor)
}

fn expand_actor_hypothesis_set(
    seeds: Vec<ResponseProgram>,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Result<Vec<ResponseProgram>, LiveScalarShadowBlocker> {
    let mut hypotheses = BTreeMap::new();
    for seed in seeds {
        let seed_key = serde_cbor::to_vec(&seed)
            .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
        hypotheses.entry(seed_key).or_insert(seed.clone());
        if repeated_primary_slots(&seed) == 0 {
            continue;
        }
        for hypothesis in bounded_ordinal_role_permutations(
            &seed,
            request_text,
            provider_payload,
            expected_response,
        )? {
            let key = serde_cbor::to_vec(&hypothesis)
                .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
            hypotheses.entry(key).or_insert(hypothesis);
            if hypotheses.len() > TEACHER_CALL_SELECTOR_BUDGET {
                return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
            }
        }
    }
    Ok(hypotheses.into_values().collect())
}

fn bounded_ordinal_role_permutations(
    seed: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Result<Vec<ResponseProgram>, LiveScalarShadowBlocker> {
    let role_types = scalar_program_role_slot_types(seed)
        .ok_or(LiveScalarShadowBlocker::RoleTypeInferenceFailed)?;
    let observed = crate::runtime::observed_request_ordinal_roles(request_text, provider_payload)
        .map_err(|_| LiveScalarShadowBlocker::ObservedRoleExtractionFailed)?;
    let candidates = observed
        .into_iter()
        .map(|role| role.selector)
        .collect::<Vec<_>>();
    let mut assignments = Vec::new();
    enumerate_ordinal_assignments(
        &role_types,
        &candidates,
        0,
        &mut vec![false; candidates.len()],
        &mut Vec::new(),
        &mut assignments,
    )?;
    let mut programs = BTreeMap::new();
    for assignment in assignments {
        let Some(program) = replace_scalar_program_selectors(seed, &assignment) else {
            continue;
        };
        if execute_response(&program, request_text, provider_payload)
            .response
            .as_deref()
            != Some(expected_response)
        {
            continue;
        }
        let key = serde_cbor::to_vec(&program)
            .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
        programs.entry(key).or_insert(program);
    }
    Ok(programs.into_values().collect())
}

fn enumerate_ordinal_assignments(
    role_types: &[AtomValueType],
    candidates: &[ResponseValueSelector],
    slot: usize,
    used: &mut [bool],
    current: &mut Vec<ResponseValueSelector>,
    output: &mut Vec<Vec<ResponseValueSelector>>,
) -> Result<(), LiveScalarShadowBlocker> {
    if slot == role_types.len() {
        output.push(current.clone());
        if output.len() > TEACHER_CALL_SELECTOR_BUDGET {
            return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
        }
        return Ok(());
    }
    for (index, candidate) in candidates.iter().enumerate() {
        if used[index] || selector_value_type(candidate) != role_types[slot] {
            continue;
        }
        used[index] = true;
        current.push(candidate.clone());
        enumerate_ordinal_assignments(role_types, candidates, slot + 1, used, current, output)?;
        current.pop();
        used[index] = false;
    }
    Ok(())
}

fn replace_scalar_program_selectors(
    seed: &ResponseProgram,
    selectors: &[ResponseValueSelector],
) -> Option<ResponseProgram> {
    let mut program = seed.clone();
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        ..
    } = &mut program.operation
    else {
        return None;
    };
    let mut replacements = selectors.iter();
    *selector = replacements.next()?.clone();
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        let mut primary_seen = false;
        for segment in segments {
            match segment {
                ResponseRenderSegment::Primary if primary_seen => {
                    *segment = ResponseRenderSegment::Selected {
                        selector: replacements.next()?.clone(),
                        format: *format,
                    };
                }
                ResponseRenderSegment::Primary => primary_seen = true,
                ResponseRenderSegment::Selected { selector, .. } => {
                    *selector = replacements.next()?.clone();
                }
                ResponseRenderSegment::Static { .. } => {}
            }
        }
    }
    replacements.next().is_none().then_some(program)
}

fn scalar_program_role_slot_types(program: &ResponseProgram) -> Option<Vec<AtomValueType>> {
    let ResponseOperation::ProjectSelectedValue {
        selector, renderer, ..
    } = &program.operation
    else {
        return None;
    };
    let primary_type = selector_value_type(selector);
    let mut role_types = vec![primary_type];
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        let mut primary_seen = false;
        for segment in segments {
            match segment {
                ResponseRenderSegment::Primary if primary_seen => role_types.push(primary_type),
                ResponseRenderSegment::Primary => primary_seen = true,
                ResponseRenderSegment::Selected { selector, .. } => {
                    role_types.push(selector_value_type(selector));
                }
                ResponseRenderSegment::Static { .. } => {}
            }
        }
    }
    (role_types.len() <= 16).then_some(role_types)
}

fn repeated_primary_slots(program: &ResponseProgram) -> usize {
    let ResponseOperation::ProjectSelectedValue { renderer, .. } = &program.operation else {
        return 0;
    };
    let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
        return 0;
    };
    segments
        .iter()
        .filter(|segment| matches!(segment, ResponseRenderSegment::Primary))
        .count()
        .saturating_sub(1)
}

fn structural_scalar_law_shape(
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<Vec<u8>> {
    let observed =
        crate::runtime::observed_request_ordinal_roles(request_text, provider_payload).ok()?;
    let mut values = observed
        .iter()
        .filter_map(|role| {
            execute_response(
                &ResponseProgram::project_selected_value(
                    role.selector.clone(),
                    ValueProjectionFormat::PlainText,
                    "completed",
                ),
                request_text,
                provider_payload,
            )
            .response
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.len() != observed.len() {
        return None;
    }
    values.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    values.dedup();
    let mut dynamic = vec![false; expected_response.len()];
    for value in values {
        for (offset, _) in expected_response.match_indices(&value) {
            let end = offset.saturating_add(value.len());
            if end <= dynamic.len() && !dynamic[offset..end].iter().any(|marked| *marked) {
                dynamic[offset..end].fill(true);
            }
        }
    }
    if !dynamic.iter().any(|marked| *marked) {
        return None;
    }
    let mut shape = vec![4, u8::try_from(observed.len()).ok()?];
    shape.extend(observed.iter().map(|role| value_type_tag(role.value_type)));
    let mut previous = None;
    for marked in dynamic {
        if previous != Some(marked) {
            shape.push(u8::from(marked));
            previous = Some(marked);
        }
    }
    Some(shape)
}

fn source_neutral_scalar_program_shape(program: &ResponseProgram) -> Option<Vec<u8>> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            arguments,
            selector,
            ..
        } => {
            let arguments = arguments
                .iter()
                .filter(|argument| semantic_call_argument(argument))
                .collect::<Vec<_>>();
            let arguments = serde_cbor::to_vec(&arguments).ok()?;
            let mut shape = vec![10, value_type_tag(selector_value_type(selector))];
            shape.extend_from_slice(&digest_parts(
                b"nando.live-function-call-law.v1",
                &[function_name.as_bytes(), &arguments],
            ));
            return Some(shape);
        }
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            arguments,
            projection,
            selector,
            ..
        } => {
            let arguments = arguments
                .iter()
                .filter(|argument| semantic_call_argument(argument))
                .collect::<Vec<_>>();
            let arguments = serde_cbor::to_vec(&arguments).ok()?;
            let projection = serde_cbor::to_vec(projection).ok()?;
            let mut shape = vec![11, value_type_tag(selector_value_type(selector))];
            shape.extend_from_slice(&digest_parts(
                b"nando.live-custom-tool-law.v1",
                &[
                    custom_tool_name.as_bytes(),
                    inner_tool_name.as_bytes(),
                    &arguments,
                    &projection,
                ],
            ));
            return Some(shape);
        }
        _ => {}
    }
    if let ResponseOperation::ComposeCollection {
        steps,
        renderer,
        completion_state,
        ..
    } = &program.operation
        && let [
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { value_type, .. },
            tail @ ..,
        ] = steps.as_slice()
    {
        if completion_state != "completed" || !matches!(tail, [] | [CollectionProgramStep::Count]) {
            return None;
        }
        let mut shape = vec![
            if tail.is_empty() { 8 } else { 9 },
            collection_scalar_tag(*value_type),
        ];
        shape.push(match renderer {
            CollectionOutputRenderer::Direct => 0,
            CollectionOutputRenderer::RenderTemplate { .. } => 1,
            _ => return None,
        });
        return Some(shape);
    }
    if let ResponseOperation::ProjectStatus {
        mapping,
        renderer,
        completion_state,
        ..
    } = &program.operation
    {
        if completion_state != "completed" {
            return None;
        }
        let mut shape = vec![7, status_mapping_flags(*mapping)? as u8];
        shape.push(match renderer {
            CollectionOutputRenderer::Direct => 0,
            CollectionOutputRenderer::RenderTemplate { .. } => 1,
            _ => return None,
        });
        return Some(shape);
    }
    if let ResponseOperation::ComposeCollection {
        steps,
        format,
        renderer,
        completion_state,
        ..
    } = &program.operation
    {
        if completion_state != "completed"
            || steps.as_slice()
                != [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ]
        {
            return None;
        }
        let mut shape = vec![6, u8::from(*format == ValueProjectionFormat::CanonicalJson)];
        shape.push(match renderer {
            CollectionOutputRenderer::Direct => 0,
            CollectionOutputRenderer::RenderTemplate { .. } => 1,
            _ => return None,
        });
        return Some(shape);
    }
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let mut shape = vec![
        5,
        value_type_tag(selector_value_type(selector)),
        u8::from(*format == ValueProjectionFormat::CanonicalJson),
    ];
    match renderer {
        CollectionOutputRenderer::Direct => shape.push(0),
        CollectionOutputRenderer::RenderTemplate { .. } => shape.push(1),
        CollectionOutputRenderer::RenderSequence { segments } => {
            shape.push(2);
            shape.extend_from_slice(&(segments.len() as u16).to_le_bytes());
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { .. } => shape.push(0),
                    ResponseRenderSegment::Primary => shape.push(1),
                    ResponseRenderSegment::Selected { selector, format } => {
                        shape.extend_from_slice(&[
                            2,
                            value_type_tag(selector_value_type(selector)),
                            u8::from(*format == ValueProjectionFormat::CanonicalJson),
                        ]);
                    }
                }
            }
        }
        CollectionOutputRenderer::RequestTemplate { marker } => {
            shape.extend_from_slice(&[3, *marker as u8]);
        }
    }
    Some(shape)
}

fn semantic_call_argument(argument: &ResponseArgument) -> bool {
    match argument {
        ResponseArgument::Integer { name, .. } => {
            !crate::teacher_join::is_execution_budget_argument(name)
        }
        ResponseArgument::String { name, value } => !(name == "chars" && value.is_empty()),
        _ => true,
    }
}

fn synthesis_payload_with_request(
    request_text: &str,
    provider_payload: &Value,
) -> Result<Value, LiveScalarShadowBlocker> {
    crate::runtime::provider_payload_view(request_text, provider_payload)
        .map(std::borrow::Cow::into_owned)
        .map_err(|error| match error {
            "provider_view_request_budget" => LiveScalarShadowBlocker::RequestTextInvalid,
            "provider_view_input_missing" => LiveScalarShadowBlocker::ProviderInputMissing,
            "provider_view_payload_budget" => LiveScalarShadowBlocker::PayloadTooLarge,
            _ => LiveScalarShadowBlocker::PayloadSerializationFailed,
        })
}

fn rich_scalar_program_roles(
    program: &ResponseProgram,
) -> Option<Vec<(ResponseValueSelector, ValueProjectionFormat)>> {
    if let ResponseOperation::FunctionCallFromRoles { selector, .. }
    | ResponseOperation::CustomToolCallFromRoles { selector, .. } = &program.operation
    {
        return Some(vec![(selector.clone(), ValueProjectionFormat::PlainText)]);
    }
    if let ResponseOperation::ComposeCollection {
        steps,
        completion_state,
        ..
    } = &program.operation
        && let [
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                selector,
                value_type: _,
            },
            tail @ ..,
        ] = steps.as_slice()
    {
        return (completion_state == "completed"
            && matches!(tail, [] | [CollectionProgramStep::Count]))
        .then(|| {
            vec![
                (
                    ResponseValueSelector::UniqueScalar {
                        value_type: AtomValueType::Collection,
                    },
                    ValueProjectionFormat::CanonicalJson,
                ),
                (selector.clone(), ValueProjectionFormat::CanonicalJson),
            ]
        });
    }
    if let ResponseOperation::ProjectStatus {
        selector,
        completion_state,
        ..
    } = &program.operation
    {
        return (completion_state == "completed")
            .then(|| vec![(selector.clone(), ValueProjectionFormat::PlainText)]);
    }
    if let ResponseOperation::ComposeCollection {
        steps,
        format,
        completion_state,
        ..
    } = &program.operation
    {
        return (completion_state == "completed"
            && steps.as_slice()
                == [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ])
        .then(|| {
            vec![(
                ResponseValueSelector::UniqueScalar {
                    value_type: AtomValueType::Collection,
                },
                *format,
            )]
        });
    }
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let mut roles = vec![(selector.clone(), *format)];
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            if let ResponseRenderSegment::Selected { selector, format } = segment {
                roles.push((selector.clone(), *format));
            }
        }
    }
    (roles.len() <= 16).then_some(roles)
}

fn canonicalize_scalar_program_roles(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> Option<ResponseProgram> {
    if let ResponseOperation::FunctionCallFromRoles { .. }
    | ResponseOperation::CustomToolCallFromRoles { .. } = &program.operation
    {
        let mut canonical = program.clone();
        let selector = match &mut canonical.operation {
            ResponseOperation::FunctionCallFromRoles { selector, .. }
            | ResponseOperation::CustomToolCallFromRoles { selector, .. } => selector,
            _ => unreachable!(),
        };
        *selector = match selector {
            ResponseValueSelector::JsonField { .. }
            | ResponseValueSelector::UniqueTurnJsonField { .. }
            | ResponseValueSelector::UniqueActiveTurnJsonField { .. } => {
                ResponseValueSelector::UniqueScalar {
                    value_type: selector_value_type(selector),
                }
            }
            _ => selector.clone(),
        };
        return Some(canonical);
    }
    if program_transform_opcode(program) == Some(TRANSFORM_OPCODE_FILTER_REQUEST_VALUE) {
        let mut canonical = program.clone();
        let ResponseOperation::ComposeCollection {
            steps, renderer, ..
        } = &mut canonical.operation
        else {
            return None;
        };
        let [
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { selector, .. },
            tail @ ..,
        ] = steps.as_mut_slice()
        else {
            return None;
        };
        if !matches!(tail, [] | [CollectionProgramStep::Count]) {
            return None;
        }
        *selector = canonical_role_selector(selector, request_text, provider_payload)?;
        *renderer = normalized_scalar_renderer(renderer)?;
        return Some(canonical);
    }
    if program_transform_opcode(program) == Some(TRANSFORM_OPCODE_PROJECT_STATUS) {
        let mut canonical = program.clone();
        let ResponseOperation::ProjectStatus {
            selector, renderer, ..
        } = &mut canonical.operation
        else {
            return None;
        };
        *selector = canonical_role_selector(selector, request_text, provider_payload)?;
        *renderer = normalized_scalar_renderer(renderer)?;
        return Some(canonical);
    }
    if program_transform_opcode(program) == Some(TRANSFORM_OPCODE_COUNT_COLLECTION) {
        let mut canonical = program.clone();
        let ResponseOperation::ComposeCollection { renderer, .. } = &mut canonical.operation else {
            return None;
        };
        *renderer = normalized_scalar_renderer(renderer)?;
        return Some(canonical);
    }
    let mut canonical = program.clone();
    let ResponseOperation::ProjectSelectedValue {
        selector,
        renderer,
        completion_state,
        ..
    } = &mut canonical.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    *selector = canonical_role_selector(selector, request_text, provider_payload)?;
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            if let ResponseRenderSegment::Selected { selector, .. } = segment {
                *selector = canonical_role_selector(selector, request_text, provider_payload)?;
            }
        }
    }
    Some(canonical)
}

fn canonical_role_selector(
    selector: &ResponseValueSelector,
    request_text: &str,
    provider_payload: &Value,
) -> Option<ResponseValueSelector> {
    match selector {
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => Some(ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal: *ordinal,
            value_type: *value_type,
        }),
        // Preserve the structural origin of request-bound roles. Collapsing
        // these to UniqueScalar erases the circuit edge that distinguishes a
        // predicate supplied by the request from unrelated payload strings.
        ResponseValueSelector::RequestLastToken | ResponseValueSelector::RequestUniqueLiteral => {
            Some(selector.clone())
        }
        // Content-line selectors are learned structural anchors into tool
        // output, not surface field names. Collapsing one to UniqueScalar
        // loses the only path from a continuation marker to its bound role.
        ResponseValueSelector::ContentLinePrefix { .. } => Some(selector.clone()),
        ResponseValueSelector::JsonField { .. } => {
            crate::runtime::canonical_request_ordinal_selector(
                request_text,
                provider_payload,
                selector,
            )
            .ok()
            .flatten()
            .or(Some(ResponseValueSelector::UniqueScalar {
                value_type: selector_value_type(selector),
            }))
        }
        _ => Some(ResponseValueSelector::UniqueScalar {
            value_type: selector_value_type(selector),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn observed_rich_scalar_surface(
    request_text: &str,
    provider_payload: &Value,
    program_roles: &[(ResponseValueSelector, ValueProjectionFormat)],
    transform_opcode: u8,
    transform_flags: u16,
    compose_count: bool,
    frame_id: &str,
    lineage_sha256: [u8; 32],
    surface_sha256: [u8; 32],
) -> Result<crate::RuntimeSurfaceEvidence, LiveScalarShadowBlocker> {
    let role_count = program_roles
        .len()
        .saturating_add(if compose_count { 3 } else { 2 });
    if !(3..=18).contains(&role_count) {
        return Err(LiveScalarShadowBlocker::InvalidBundle);
    }
    let semantic_to_local = local_role_permutation_n(frame_id, role_count);
    let context = semantic_to_local[0];
    let output = semantic_to_local[role_count - 1];
    let intermediate = compose_count.then_some(semantic_to_local[role_count - 2]);
    let mut roles = vec![StructuralRoleSignature::new(0, 0, 0, 0, Vec::new()); role_count];
    let planes = (0..program_roles.len())
        .map(|index| u8::try_from(index).unwrap_or(u8::MAX))
        .collect::<Vec<_>>();
    roles[usize::from(context)] = StructuralRoleSignature::new(5, 1, 0, 1, planes.clone());
    if let Some(intermediate) = intermediate {
        roles[usize::from(intermediate)] = StructuralRoleSignature::new(
            value_type_tag(AtomValueType::Collection),
            1,
            2,
            4,
            Vec::new(),
        );
    }
    roles[usize::from(output)] = StructuralRoleSignature::new(
        value_type_tag(if compose_count {
            AtomValueType::Integer
        } else {
            selector_value_type(&program_roles[0].0)
        }),
        1,
        2,
        4,
        Vec::new(),
    );
    let mut relations = Vec::with_capacity(program_roles.len());
    let mut atoms = Vec::with_capacity(program_roles.len());
    let mut anchors = Vec::with_capacity(program_roles.len());
    for (index, (selector, format)) in program_roles.iter().enumerate() {
        let source = semantic_to_local[index + 1];
        let plane = u8::try_from(index).map_err(|_| LiveScalarShadowBlocker::InvalidBundle)?;
        let value_type = selector_value_type(selector);
        let phase = if program_roles.len() == 1 {
            roles[usize::from(source)] =
                crate::crystallized_operator::runtime_role_signature_for_selector(selector, plane);
            let phase_atoms = [
                format!("scalar_type:{}", value_type_tag(value_type)),
                "cardinality:unique".to_owned(),
            ];
            phase_vector_from_atoms(phase_atoms.iter().map(String::as_str), 1)[0]
        } else {
            roles[usize::from(source)] =
                crate::crystallized_operator::runtime_role_signature_for_selector(selector, plane);
            let phase_atom = format!("scalar_role:{index}:type:{}", value_type_tag(value_type));
            phase_vector_from_atoms([phase_atom.as_str()], 1)[0]
        };
        relations.push(LocalRelationFragment {
            plane,
            source_local_role: context,
            target_local_role: source,
            state: TernaryRelationState::Supported,
            phase_anchor: phase,
        });
        if transform_opcode != TRANSFORM_OPCODE_FILTER_REQUEST_VALUE {
            atoms.push(TypedProgramAtom {
                opcode: transform_opcode,
                output_local_role: output,
                source_a_local_role: source,
                source_b_local_role: OPERATOR_ROLE_NONE,
                parameter: transform_parameter(value_type)
                    | (u16::try_from(index).unwrap_or(u16::MAX) << 8),
                flags: match transform_opcode {
                    TRANSFORM_OPCODE_PROJECT_STATUS => transform_flags,
                    TRANSFORM_OPCODE_COUNT_COLLECTION => 0,
                    _ if *format == ValueProjectionFormat::CanonicalJson => {
                        TRANSFORM_FLAG_CANONICAL_JSON
                    }
                    _ => 0,
                },
            });
        }
        anchors.push(RuntimeRoleAnchor {
            local_role: source,
            selector: selector.clone(),
            json_path_sha256: None,
        });
    }
    if transform_opcode == TRANSFORM_OPCODE_FILTER_REQUEST_VALUE {
        let predicate_type = selector_value_type(&program_roles[1].0);
        let filter_output = intermediate.unwrap_or(output);
        atoms.push(TypedProgramAtom {
            opcode: TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
            output_local_role: filter_output,
            source_a_local_role: semantic_to_local[1],
            source_b_local_role: semantic_to_local[2],
            parameter: transform_parameter(predicate_type),
            flags: transform_flags,
        });
        if compose_count {
            atoms.push(TypedProgramAtom {
                opcode: TRANSFORM_OPCODE_COUNT_COLLECTION,
                output_local_role: output,
                source_a_local_role: filter_output,
                source_b_local_role: OPERATOR_ROLE_NONE,
                parameter: (1 << 8) | transform_parameter(AtomValueType::Collection),
                flags: 0,
            });
        }
    }
    let bundle =
        SurfaceFragmentBundle::new(lineage_sha256, surface_sha256, roles, relations, atoms)
            .map_err(|_| LiveScalarShadowBlocker::InvalidBundle)?;
    Ok(crate::RuntimeSurfaceEvidence {
        bundle,
        request_text: request_text.to_owned(),
        provider_payload: provider_payload.clone(),
        anchors: anchors.into_boxed_slice(),
    })
}

fn project_scalar_program(
    program: ResponseProgram,
) -> Option<(
    Vec<u8>,
    ResponseValueSelector,
    AtomValueType,
    ValueProjectionFormat,
    CollectionOutputRenderer,
)> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let renderer = normalized_scalar_renderer(renderer)?;
    let value_type = selector_value_type(selector);
    let bytes = serde_json::to_vec(&program).ok()?;
    Some((bytes, selector.clone(), value_type, *format, renderer))
}

fn derive_exact_scalar_program(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let direct = ResponseProgram::project_selected_value(
        selector.clone(),
        *format,
        completion_state.clone(),
    );
    if !is_source_neutral_response_program(&direct) {
        return None;
    }
    let computed = execute_response(&direct, request_text, provider_payload).response?;
    let renderer = infer_scalar_renderer(&computed, expected_response)?;
    let derived = direct.with_value_renderer(renderer);
    if derived.validate().is_err()
        || !is_privacy_safe_online_response_program(&derived)
        || execute_response(&derived, request_text, provider_payload)
            .response
            .as_deref()
            != Some(expected_response)
    {
        return None;
    }
    Some(derived)
}

fn derive_exact_count_program(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let ResponseOperation::ComposeCollection {
        steps,
        format: _,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return None;
    };
    if completion_state != "completed"
        || steps.as_slice()
            != [
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::Count,
            ]
    {
        return None;
    }
    // Count emits a decimal integer, for which PlainText and CanonicalJson are
    // byte-identical. Freeze one representation so equivalent hypotheses do
    // not split the operator field or VM contract.
    let direct = ResponseProgram::compose_collection(
        steps.clone(),
        ValueProjectionFormat::PlainText,
        completion_state.clone(),
    );
    if !is_source_neutral_response_program(&direct) {
        return None;
    }
    let computed = execute_response(&direct, request_text, provider_payload).response?;
    let renderer = infer_scalar_renderer(&computed, expected_response)?;
    let derived = direct.with_collection_renderer(renderer);
    (derived.validate().is_ok()
        && is_privacy_safe_online_response_program(&derived)
        && execute_response(&derived, request_text, provider_payload)
            .response
            .as_deref()
            == Some(expected_response))
    .then_some(derived)
}

fn derive_exact_filter_programs(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Vec<ResponseProgram> {
    let ResponseOperation::ComposeCollection {
        steps,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return Vec::new();
    };
    let [
        CollectionProgramStep::SelectOnlyArrayField,
        CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type },
        tail @ ..,
    ] = steps.as_slice()
    else {
        return Vec::new();
    };
    if completion_state != "completed" || !matches!(tail, [] | [CollectionProgramStep::Count]) {
        return Vec::new();
    }
    let expected_type = collection_atom_type(*value_type);
    crate::collection_synthesis::learned_selector_candidates(provider_payload)
        .into_iter()
        .filter(|selector| selector_value_type(selector) == expected_type)
        // The broad hypothesis is explicitly request-conditioned. Letting an
        // equal payload scalar replace that role creates a second, spurious
        // circuit which cannot be separated while predicate and row agree.
        .filter(crate::collection_synthesis::is_source_neutral_request_selector)
        .filter_map(|selector| {
            let mut steps = vec![
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                    selector,
                    value_type: *value_type,
                },
            ];
            if !tail.is_empty() {
                steps.push(CollectionProgramStep::Count);
            }
            let direct = ResponseProgram::compose_collection(
                steps,
                ValueProjectionFormat::CanonicalJson,
                completion_state.clone(),
            );
            if !is_source_neutral_response_program(&direct) {
                return None;
            }
            let computed = execute_response(&direct, request_text, provider_payload).response?;
            let renderer = infer_scalar_renderer(&computed, expected_response)?;
            let derived = direct.with_collection_renderer(renderer);
            (derived.validate().is_ok()
                && is_privacy_safe_online_response_program(&derived)
                && execute_response(&derived, request_text, provider_payload)
                    .response
                    .as_deref()
                    == Some(expected_response))
            .then_some(derived)
        })
        .collect()
}

fn derive_exact_status_program(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let ResponseOperation::ProjectStatus {
        selector,
        mapping,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let direct =
        ResponseProgram::project_status(selector.clone(), *mapping, completion_state.clone());
    if !is_source_neutral_response_program(&direct) {
        return None;
    }
    let computed = execute_response(&direct, request_text, provider_payload).response?;
    let renderer = infer_scalar_renderer(&computed, expected_response)?;
    let derived = direct.with_status_renderer(renderer);
    (derived.validate().is_ok()
        && is_privacy_safe_online_response_program(&derived)
        && execute_response(&derived, request_text, provider_payload)
            .response
            .as_deref()
            == Some(expected_response))
    .then_some(derived)
}

fn infer_scalar_renderer(computed: &str, expected: &str) -> Option<CollectionOutputRenderer> {
    if computed == expected {
        return Some(CollectionOutputRenderer::Direct);
    }
    if computed.is_empty() {
        return None;
    }
    let mut matches = expected.match_indices(computed);
    let (offset, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(CollectionOutputRenderer::RenderTemplate {
        prefix: expected[..offset].to_owned(),
        suffix: expected[offset + computed.len()..].to_owned(),
    })
}

fn normalized_scalar_renderer(
    renderer: &CollectionOutputRenderer,
) -> Option<CollectionOutputRenderer> {
    match renderer {
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RenderTemplate { .. } => {
            Some(renderer.clone())
        }
        CollectionOutputRenderer::RenderSequence { segments } => {
            let mut prefix = String::new();
            let mut suffix = String::new();
            let mut primary_seen = false;
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { text } if primary_seen => {
                        suffix.push_str(text);
                    }
                    ResponseRenderSegment::Static { text } => prefix.push_str(text),
                    ResponseRenderSegment::Primary if !primary_seen => primary_seen = true,
                    ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. } => {
                        return None;
                    }
                }
            }
            primary_seen.then_some(CollectionOutputRenderer::RenderTemplate { prefix, suffix })
        }
        CollectionOutputRenderer::RequestTemplate { .. } => None,
    }
}

fn classify_exact_program_blocker(programs: &[ResponseProgram]) -> LiveScalarShadowBlocker {
    if programs
        .iter()
        .any(|program| matches!(&program.operation, ResponseOperation::ProjectStatus { .. }))
    {
        LiveScalarShadowBlocker::ExactStatusProgram
    } else if programs.iter().any(|program| {
        matches!(
            &program.operation,
            ResponseOperation::ComposeCollection { .. }
        )
    }) {
        LiveScalarShadowBlocker::ExactCollectionProgram
    } else if programs.iter().any(|program| {
        matches!(
            &program.operation,
            ResponseOperation::ProjectSelectedValue { .. }
        )
    }) {
        LiveScalarShadowBlocker::UnsupportedRendererProgram
    } else {
        LiveScalarShadowBlocker::UnsupportedProgramKind
    }
}

fn selector_value_type(selector: &ResponseValueSelector) -> AtomValueType {
    match selector {
        ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::RequestReferencedJsonField { value_type }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | ResponseValueSelector::TurnOutputLine { value_type, .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => *value_type,
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => AtomValueType::String,
    }
}

fn local_role_permutation_n(frame_id: &str, role_count: usize) -> Vec<u8> {
    let digest = Sha256::digest(frame_id.as_bytes());
    let mut roles = (0..role_count)
        .map(|role| u8::try_from(role).expect("bounded live role count"))
        .collect::<Vec<_>>();
    for index in (1..roles.len()).rev() {
        roles.swap(
            index,
            usize::from(digest[index % digest.len()]) % (index + 1),
        );
    }
    roles
}

const fn transform_parameter(value_type: AtomValueType) -> u16 {
    match value_type {
        AtomValueType::String => TRANSFORM_VALUE_STRING,
        AtomValueType::Integer => TRANSFORM_VALUE_INTEGER,
        AtomValueType::Boolean => TRANSFORM_VALUE_BOOLEAN,
        AtomValueType::Identifier => TRANSFORM_VALUE_IDENTIFIER,
        AtomValueType::Collection => TRANSFORM_VALUE_COLLECTION,
    }
}

fn program_transform_opcode(program: &ResponseProgram) -> Option<u8> {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => {
            Some(TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR)
        }
        ResponseOperation::ProjectStatus { .. } => Some(TRANSFORM_OPCODE_PROJECT_STATUS),
        ResponseOperation::ComposeCollection { steps, .. }
            if matches!(
                steps.as_slice(),
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. },
                    ..
                ]
            ) && matches!(steps.len(), 2 | 3) =>
        {
            Some(TRANSFORM_OPCODE_FILTER_REQUEST_VALUE)
        }
        ResponseOperation::ComposeCollection { steps, .. }
            if steps.as_slice()
                == [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ] =>
        {
            Some(TRANSFORM_OPCODE_COUNT_COLLECTION)
        }
        _ => None,
    }
}

fn program_transform_flags(program: &ResponseProgram) -> Option<u16> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => Some(0),
        ResponseOperation::ProjectSelectedValue { format, .. } => {
            Some(u16::from(*format == ValueProjectionFormat::CanonicalJson))
        }
        ResponseOperation::ProjectStatus { mapping, .. } => status_mapping_flags(*mapping),
        ResponseOperation::ComposeCollection { steps, .. }
            if matches!(
                steps.as_slice(),
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. },
                    ..
                ]
            ) && matches!(steps.len(), 2 | 3) =>
        {
            Some(TRANSFORM_FLAG_CANONICAL_JSON)
        }
        ResponseOperation::ComposeCollection { steps, .. }
            if steps.as_slice()
                == [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ] =>
        {
            Some(0)
        }
        _ => None,
    }
}

fn program_has_filter_count(program: &ResponseProgram) -> bool {
    matches!(
        &program.operation,
        ResponseOperation::ComposeCollection { steps, .. }
            if matches!(
                steps.as_slice(),
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. },
                    CollectionProgramStep::Count,
                ]
            )
    )
}

const fn collection_scalar_tag(value_type: CollectionScalarType) -> u8 {
    match value_type {
        CollectionScalarType::String => 1,
        CollectionScalarType::Integer => 2,
        CollectionScalarType::Boolean => 3,
    }
}

const fn collection_atom_type(value_type: CollectionScalarType) -> AtomValueType {
    match value_type {
        CollectionScalarType::String => AtomValueType::String,
        CollectionScalarType::Integer => AtomValueType::Integer,
        CollectionScalarType::Boolean => AtomValueType::Boolean,
    }
}

const fn status_mapping_flags(mapping: ProjectStatusMapping) -> Option<u16> {
    Some(match mapping {
        ProjectStatusMapping::ZeroIsSuccess => TRANSFORM_STATUS_ZERO_IS_SUCCESS,
        ProjectStatusMapping::ZeroIsPass => TRANSFORM_STATUS_ZERO_IS_PASS,
        ProjectStatusMapping::ZeroIsOk => TRANSFORM_STATUS_ZERO_IS_OK,
        ProjectStatusMapping::ZeroIsTrue => TRANSFORM_STATUS_ZERO_IS_TRUE,
    })
}

const fn value_type_tag(value_type: AtomValueType) -> u8 {
    match value_type {
        AtomValueType::String => 1,
        AtomValueType::Integer => 2,
        AtomValueType::Boolean => 3,
        AtomValueType::Identifier => 4,
        AtomValueType::Collection => 5,
    }
}

fn parse_commitment(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16).ok()?;
    }
    (digest != [0; 32]).then_some(digest)
}

fn commitment_hex(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest_parts(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn extractor_version(value: &str) -> u32 {
    let digest = Sha256::digest(value.as_bytes());
    u32::from_le_bytes(digest[..4].try_into().expect("fixed digest width"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        AtomSource, ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, RUNTIME_FRAME_SCHEMA_V1,
        RelationAtom, RuntimeFrame, SOURCE_NEUTRAL_EXTRACTOR_VERSION, TEACHER_OUTCOME_SCHEMA_V1,
        TEACHER_TRANSITION_SCHEMA_V1, TeacherActionAst, TeacherOutcome, TeacherVerifierEvidence,
    };

    fn transition(field: &str, accepted: bool) -> TeacherTransition {
        let hash = |byte: char| byte.to_string().repeat(64);
        TeacherTransition {
            schema: TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
            before: RuntimeFrame {
                schema: RUNTIME_FRAME_SCHEMA_V1.to_owned(),
                frame_id_sha256: hash('1'),
                event_id_sha256: hash('2'),
                client_intent_id_sha256: hash('3'),
                session_id_sha256: hash('4'),
                observed_at_unix_nanos: 1,
                extractor_version: "live-test-v1".to_owned(),
                atoms: Vec::new(),
                evidence_ref_sha256: hash('5'),
            },
            outcome: TeacherOutcome {
                schema: TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
                action: TeacherActionAst {
                    signature_sha256: hash('6'),
                    action_symbol: "response".to_owned(),
                    atoms: Vec::new(),
                },
                verifier: TeacherVerifierEvidence {
                    accepted,
                    evidence_ref_sha256: hash('7'),
                    output_digest_sha256: hash('8'),
                },
                completed_at_unix_nanos: 2,
            },
            economics: Some(EconomicsReceipt {
                schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
                exact_input_tokens: 100,
                ordinary: true,
                controlled: false,
                replay: false,
                dedupe_eligible: true,
                provider_evidence_ref_sha256: hash('9'),
            }),
            runtime_parity_case: Some(crate::RuntimeParityCase {
                evidence_ref_sha256: hash('a'),
                capture_receipt: None,
                request_text: "Return the count".to_owned(),
                provider_payload: json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": format!("{{\"{field}\":7}}")
                    }]
                }),
                expected_response: "7".to_owned(),
            }),
        }
    }

    fn multi_value_transition(
        first_field: &str,
        second_field: &str,
        expected_response: &str,
    ) -> TeacherTransition {
        let mut row = transition(first_field, true);
        let parity = row.runtime_parity_case.as_mut().expect("parity case");
        parity.request_text = format!("Return {first_field} and {second_field}");
        parity.provider_payload = json!({
            "input": [{
                "type": "function_call_output",
                "output": format!("{{\"{first_field}\":7,\"{second_field}\":2}}")
            }]
        });
        parity.expected_response = expected_response.to_owned();
        row
    }

    fn custom_tool_transition(index: u64) -> TeacherTransition {
        let mut row = transition("unused", true);
        let observation_slot = 3;
        let action_slot = 8;
        let selected = index + 1000;
        let yield_time_ms = 10_000 + (index % 3) * 10_000;
        let max_output_tokens = 1_000 + (index % 5) * 1_000;
        row.before.extractor_version = SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned();
        row.before.atoms = vec![
            RelationAtom::ToolKind {
                value: "exec_command".to_owned(),
            },
            RelationAtom::ObservationCallShape {
                value: "custom_tool_call".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "pending".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "custom_tool_call".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: observation_slot,
                value_type: AtomValueType::Integer,
                source: AtomSource::Observation,
                value_sha256: format!("{selected:064x}"),
            },
            RelationAtom::UniqueSlot {
                slot_id: observation_slot,
            },
            RelationAtom::ObservationSelector {
                slot_id: observation_slot,
                selector: ResponseValueSelector::ContentLinePrefix {
                    prefix: "SESSION_ID=".to_owned(),
                    value_type: AtomValueType::Integer,
                },
            },
        ];
        row.outcome.action.action_symbol = "custom_tool:exec/write_stdin".to_owned();
        row.outcome.action.atoms = vec![
            RelationAtom::TypedSlot {
                slot_id: action_slot,
                value_type: AtomValueType::Integer,
                source: AtomSource::Action,
                value_sha256: format!("{selected:064x}"),
            },
            RelationAtom::SlotEquality {
                left_slot: observation_slot,
                right_slot: action_slot,
            },
            RelationAtom::ActionCustomTool {
                value: "exec".to_owned(),
            },
            RelationAtom::ActionInnerTool {
                value: "write_stdin".to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "session_id".to_owned(),
                slot_id: action_slot,
                value_type: None,
            },
            RelationAtom::ActionStringArgument {
                name: "chars".to_owned(),
                value: String::new(),
            },
            RelationAtom::ActionIntegerArgument {
                name: "yield_time_ms".to_owned(),
                value: yield_time_ms,
            },
            RelationAtom::ActionIntegerArgument {
                name: "max_output_tokens".to_owned(),
                value: max_output_tokens,
            },
            RelationAtom::ActionResultProjection {
                output_field: "output".to_owned(),
                continuation_field: "session_id".to_owned(),
                continuation_prefix: "SESSION_ID=".to_owned(),
            },
        ];
        let arguments = serde_json::to_string(&json!({
            "chars": "",
            "max_output_tokens": max_output_tokens,
            "session_id": selected,
            "yield_time_ms": yield_time_ms,
        }))
        .expect("custom tool arguments");
        let input = format!(
            "const r=await tools.write_stdin({arguments});text(r.output);if(r.session_id)text(\"SESSION_ID=\"+r.session_id);"
        );
        let parity = row.runtime_parity_case.as_mut().expect("parity case");
        parity.request_text = "Continue the running process".to_owned();
        parity.provider_payload = json!({
            "tools": [{"type": "custom", "name": "exec"}],
            "input": [
                {
                    "type": "custom_tool_call",
                    "name": "exec",
                    "call_id": format!("call-{index}"),
                    "input": "source"
                },
                {
                    "type": "custom_tool_call_output",
                    "call_id": format!("call-{index}"),
                    "output": [{"type": "text", "text": format!("still running\nSESSION_ID={selected}")}]
                }
            ]
        });
        parity.expected_response = serde_json::to_string(&json!({
            "kind": "custom_tool_call",
            "name": "exec",
            "input": input,
        }))
        .expect("custom tool response");
        row
    }

    fn custom_tool_transition_with_repeated_outputs(index: u64) -> TeacherTransition {
        let mut row = custom_tool_transition(index);
        let selected = index + 1000;
        let outputs = (0..64)
            .map(|output_index| {
                json!({
                    "type": "custom_tool_call_output",
                    "call_id": format!("call-{index}-{output_index}"),
                    "output": [{
                        "type": "text",
                        "text": format!("still running\nSESSION_ID={selected}"),
                    }],
                })
            })
            .collect::<Vec<_>>();
        row.runtime_parity_case
            .as_mut()
            .expect("parity case")
            .provider_payload = json!({
            "tools": [{"type": "custom", "name": "exec"}],
            "input": outputs,
        });
        row
    }

    fn collection_count_transition(request: &str, prefix: &str) -> TeacherTransition {
        collection_count_transition_n(request, prefix, 3)
    }

    fn collection_count_transition_n(
        request: &str,
        prefix: &str,
        count: usize,
    ) -> TeacherTransition {
        let mut row = transition("unused", true);
        let parity = row.runtime_parity_case.as_mut().expect("parity case");
        parity.request_text = request.to_owned();
        let rows = (0..count)
            .map(|value| json!({"value": value}))
            .collect::<Vec<_>>();
        parity.provider_payload = json!({
            "input": [{
                "type": "function_call_output",
                "output": serde_json::to_string(&rows).expect("rows serialize")
            }]
        });
        parity.expected_response = format!("{prefix}{count}.");
        row
    }

    fn status_transition(field: &str, code: u64) -> TeacherTransition {
        let mut row = transition(field, true);
        let parity = row.runtime_parity_case.as_mut().expect("parity case");
        parity.request_text = "Check build status".to_owned();
        parity.provider_payload = json!({
            "input": [{
                "type": "function_call_output",
                "output": format!("{{\"{field}\":{code}}}")
            }]
        });
        parity.expected_response =
            format!("Build status: {}.", if code == 0 { "OK" } else { "ERROR" });
        row
    }

    fn filter_transition(field: &str, predicate: &str) -> TeacherTransition {
        let mut row = transition(field, true);
        let parity = row.runtime_parity_case.as_mut().expect("parity case");
        parity.request_text = format!("Filter {predicate}");
        let rows = vec![
            json!({(field): predicate, "value": 3}),
            json!({(field): "other", "value": 5}),
        ];
        parity.provider_payload = json!({
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": parity.request_text.clone()
                },
                {
                    "type": "function_call_output",
                    "output": serde_json::to_string(&rows).expect("rows serialize")
                }
            ]
        });
        parity.expected_response = serde_json::to_string(&rows[..1]).expect("result serialize");
        row
    }

    fn filter_count_transition(field: &str, predicate: &str) -> TeacherTransition {
        let mut row = filter_transition(field, predicate);
        row.runtime_parity_case
            .as_mut()
            .expect("parity case")
            .expected_response = "Matching records: 1.".to_owned();
        row
    }

    #[test]
    fn verified_scalar_trace_becomes_source_neutral_circuit_evidence() {
        let first =
            extract_live_scalar_circuit_sample(&transition("total", true)).expect("scalar trace");
        let renamed = extract_live_scalar_circuit_sample(&transition("renamed_total", true))
            .expect("renamed scalar trace");

        assert_eq!(first.bundle.roles().len(), 3);
        // Only the pre-action context/source relation is observed. The output
        // role belongs to the learned transform and must not leak backward as
        // a fabricated support relation.
        assert_eq!(first.bundle.relations().len(), 1);
        assert_eq!(first.bundle.program_atoms().len(), 1);
        assert_eq!(first.law_sha256, renamed.law_sha256);
        assert_eq!(first.anchors, renamed.anchors);
    }

    #[test]
    fn verified_collection_count_trace_becomes_count_circuit_evidence() {
        let first_row = collection_count_transition("Count selected values", "Total values: ");
        let first = extract_live_scalar_circuit_sample(&first_row).expect("collection count trace");
        let renamed = extract_live_scalar_circuit_sample(&collection_count_transition(
            "How many records are present?",
            "Record count: ",
        ))
        .expect("renamed collection count trace");

        let [atom] = first.bundle.program_atoms() else {
            panic!("one count transform expected");
        };
        assert_eq!(atom.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
        assert_eq!(atom.parameter & 0x00ff, TRANSFORM_VALUE_COLLECTION);
        assert_eq!(first.anchors.len(), 1);
        assert_eq!(first.law_sha256, renamed.law_sha256);
    }

    #[test]
    fn direct_collection_payload_becomes_count_circuit_evidence() {
        let mut row = collection_count_transition("Count the records", "Total records: ");
        row.runtime_parity_case
            .as_mut()
            .expect("parity case")
            .provider_payload = json!({
            "records": [
                {"value": 0},
                {"value": 1},
                {"value": 2}
            ]
        });

        let sample = extract_live_scalar_circuit_sample(&row).expect("direct count trace");
        let [atom] = sample.bundle.program_atoms() else {
            panic!("one count transform expected");
        };
        assert_eq!(atom.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
        assert_eq!(atom.parameter & 0x00ff, TRANSFORM_VALUE_COLLECTION);
    }

    #[test]
    fn direct_collection_payload_without_request_becomes_count_evidence() {
        let mut row = collection_count_transition("", "Total records: ");
        row.runtime_parity_case
            .as_mut()
            .expect("parity case")
            .provider_payload = json!({"records": [{"value": 0}, {"value": 1}]});
        row.runtime_parity_case
            .as_mut()
            .expect("parity case")
            .expected_response = "Total records: 2.".to_owned();

        let sample = extract_live_scalar_circuit_sample(&row).expect("request-free count trace");
        let [atom] = sample.bundle.program_atoms() else {
            panic!("one count transform expected");
        };
        assert_eq!(atom.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
    }

    #[test]
    fn verified_status_trace_becomes_status_circuit_evidence() {
        let first = extract_live_scalar_circuit_sample(&status_transition("status", 0))
            .expect("status trace");
        let renamed = extract_live_scalar_circuit_sample(&status_transition("renamed_code", 9))
            .expect("renamed status trace");

        let [atom] = first.bundle.program_atoms() else {
            panic!("one status transform expected");
        };
        assert_eq!(atom.opcode, TRANSFORM_OPCODE_PROJECT_STATUS);
        assert_eq!(atom.flags, TRANSFORM_STATUS_ZERO_IS_OK);
        assert_eq!(first.law_sha256, renamed.law_sha256);
    }

    #[test]
    fn verified_filter_trace_becomes_two_role_circuit_evidence() {
        let first_row = filter_transition("kind", "active");
        let first = extract_live_scalar_circuit_sample(&first_row).expect("filter trace");
        let renamed = extract_live_scalar_circuit_sample(&filter_transition("state", "ready"))
            .expect("renamed filter trace");

        let [atom] = first.bundle.program_atoms() else {
            panic!("one filter transform expected");
        };
        assert_eq!(atom.opcode, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE);
        assert_ne!(atom.source_a_local_role, atom.source_b_local_role);
        assert_eq!(first.anchors.len(), 2);
        assert_eq!(first.law_sha256, renamed.law_sha256);
    }

    #[test]
    fn verified_filter_count_trace_becomes_composed_circuit_evidence() {
        let first = extract_live_scalar_circuit_sample(&filter_count_transition("kind", "active"))
            .expect("filter-count trace");
        let renamed =
            extract_live_scalar_circuit_sample(&filter_count_transition("state", "ready"))
                .expect("renamed filter-count trace");

        let [filter, count] = first.bundle.program_atoms() else {
            panic!("filter and count transforms expected");
        };
        assert_eq!(filter.opcode, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE);
        assert_eq!(count.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
        assert_eq!(filter.output_local_role, count.source_a_local_role);
        assert_eq!(first.anchors.len(), 2);
        assert_eq!(first.law_sha256, renamed.law_sha256);
    }

    #[test]
    fn rejected_teacher_cannot_create_wave_evidence() {
        assert_eq!(
            extract_live_scalar_circuit_sample(&transition("total", false)),
            Err(LiveScalarShadowBlocker::TeacherRejected)
        );
    }

    #[test]
    fn multi_value_surfaces_share_one_structural_law() {
        let first = extract_live_scalar_circuit_sample(&multi_value_transition(
            "total",
            "failed",
            "Total: 7; failed: 2",
        ))
        .expect("first multi-value trace");
        let renamed = extract_live_scalar_circuit_sample(&multi_value_transition(
            "records",
            "errors",
            "Records: 7; errors: 2",
        ))
        .expect("renamed multi-value trace");

        assert_eq!(
            first.anchors.len(),
            2,
            "unexpected canonical actor: {:#?}; hypotheses: {:#?}",
            first.actor_template,
            first.actor_hypotheses
        );
        assert_eq!(first.bundle.roles().len(), 4);
        assert_eq!(first.bundle.relations().len(), 2);
        assert_eq!(first.bundle.program_atoms().len(), 2);
        assert_eq!(first.law_sha256, renamed.law_sha256);
    }

    #[test]
    fn historical_rebuild_never_creates_frozen_future() {
        let mut state = LiveScalarShadowState::default();
        for index in 1_u64..=40 {
            let mut row = transition("total", true);
            row.before.frame_id_sha256 = format!("{index:064x}");
            row.before.session_id_sha256 = format!("{:064x}", index + 100);
            state.observe_historical_support(&row);
        }

        let report = state.report();
        assert_eq!(report.support_rows, LIVE_SCALAR_SUPPORT_ROWS);
        assert_eq!(report.future_rows, 0);
        assert_eq!(
            report
                .blockers
                .get("historicalsupportcapacityreached")
                .copied(),
            Some(8)
        );
    }

    #[test]
    fn repeated_support_session_fills_rows_but_cannot_freeze() {
        let mut state = LiveScalarShadowState::default();
        for index in 1_u64..=LIVE_SCALAR_SUPPORT_ROWS as u64 {
            let mut row = transition("total", true);
            row.before.frame_id_sha256 = format!("{index:064x}");
            state.observe_historical_support(&row);
        }

        let report = state.report();
        assert_eq!(report.support_rows, LIVE_SCALAR_SUPPORT_ROWS);
        assert_eq!(report.future_rows, 0);
        assert_eq!(report.blockers.get("support_sessions_below_3"), Some(&1));
    }

    #[test]
    fn distinct_future_frames_may_share_sessions_without_crossing_support_boundary() {
        let mut state = LiveScalarShadowState::default();
        for index in 1_u64..=LIVE_SCALAR_SUPPORT_ROWS as u64 {
            let mut row = transition("total", true);
            row.before.frame_id_sha256 = format!("{index:064x}");
            row.before.session_id_sha256 = format!("{:064x}", 1 + index % 3);
            state.observe(&row);
        }
        for index in 1_u64..=LIVE_SCALAR_FUTURE_ROWS as u64 {
            let mut row = transition("total", true);
            row.before.frame_id_sha256 = format!("{:064x}", 100 + index);
            row.before.session_id_sha256 = format!("{:064x}", 101 + index % 3);
            state.observe(&row);
        }

        assert_eq!(state.laws.len(), 1);
        let law = state.laws.values().next().expect("one structural law");
        assert_eq!(law.support.len(), LIVE_SCALAR_SUPPORT_ROWS);
        assert_eq!(law.future.len(), LIVE_SCALAR_FUTURE_ROWS);
        assert!(
            !state
                .blockers
                .contains_key(&LiveScalarShadowBlocker::SupportFutureSessionOverlap)
        );
    }

    #[test]
    fn single_primary_sequence_normalizes_to_typed_template() {
        let renderer = CollectionOutputRenderer::RenderSequence {
            segments: vec![
                ResponseRenderSegment::Static {
                    text: "Total records: ".to_owned(),
                },
                ResponseRenderSegment::Primary,
                ResponseRenderSegment::Static {
                    text: ".".to_owned(),
                },
            ],
        };
        assert_eq!(
            normalized_scalar_renderer(&renderer),
            Some(CollectionOutputRenderer::RenderTemplate {
                prefix: "Total records: ".to_owned(),
                suffix: ".".to_owned(),
            })
        );
    }

    #[test]
    fn templated_live_rows_reach_verified_scalar_shadow_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..64_u8 {
            let mut row = transition(&format!("field_{index}"), true);
            row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
            row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
            row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
            let parity = row.runtime_parity_case.as_mut().expect("parity");
            if index % 2 == 0 {
                if index % 4 == 0 {
                    parity.request_text.clear();
                } else {
                    parity.request_text = "Summarize the result".to_owned();
                }
                parity.provider_payload = json!({format!("field_{index}"): index + 7});
            } else {
                parity.provider_payload = json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": format!("{{\"field_{index}\":{}}}", index + 7)
                    }]
                });
            }
            row.runtime_parity_case
                .as_mut()
                .expect("parity")
                .expected_response = format!("Total records: {}.", index + 7);
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.executable, 64, "{report:#?}");
        assert_eq!(report.support_rows, 32, "{report:#?}");
        assert_eq!(report.future_rows, 32, "{report:#?}");
        assert_eq!(report.frozen_laws, 1, "{report:#?}");
        assert!(report.ingest_accounting_complete, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 32, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");

        let candidates = state.admission_candidates();
        assert_eq!(candidates.len(), 1, "{report:#?}");
        let snapshot = crate::build_crystallized_admission_snapshot(
            &candidates,
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("external admission evaluates sealed operator")
        .expect("sealed operator reaches registry");
        assert_eq!(snapshot.registry.packages.len(), 1);
        let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
            .expect("registry restores crystallized operator");
        let execution = executor.execute_shadow(
            "Return the count",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"new_surface_total\":91}"
                }]
            }),
        );
        assert_eq!(
            execution.response.as_deref(),
            Some("Total records: 91."),
            "{execution:#?}"
        );
        let ambiguous = executor.execute_shadow(
            "Return the count",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"left\":91,\"right\":92}"
                }]
            }),
        );
        assert_eq!(ambiguous.status, crate::ResponseExecutionStatus::Abstain);
        let incompatible = executor.execute_shadow(
            "",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"new_surface_total\":true}"
                }]
            }),
        );
        assert_eq!(incompatible.status, crate::ResponseExecutionStatus::Abstain);

        let mut tampered_support = candidates.clone();
        tampered_support[0].support[0]
            .runtime_parity_case
            .as_mut()
            .expect("support parity")
            .expected_response = "999".to_owned();
        assert!(matches!(
            crate::build_crystallized_admission_snapshot(
                &tampered_support,
                "test-project",
                2,
                100,
                30,
                &"a".repeat(64),
                &"b".repeat(64),
            ),
            Err("crystallized_admission_resynthesis_failed")
        ));

        let mut tampered_seal = candidates;
        tampered_seal[0].executable_parity_seal_sha256 = "c".repeat(64);
        assert!(matches!(
            crate::build_crystallized_admission_snapshot(
                &tampered_seal,
                "test-project",
                3,
                100,
                30,
                &"a".repeat(64),
                &"b".repeat(64),
            ),
            Err("crystallized_admission_resynthesis_mismatch")
        ));
    }

    #[test]
    fn typed_custom_tool_rows_reach_verified_crystallized_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..64_u64 {
            let mut row = custom_tool_transition(index);
            row.before.frame_id_sha256 = format!("{:064x}", index + 1);
            row.before.session_id_sha256 = format!("{:064x}", index + 101);
            row.before.client_intent_id_sha256 = format!("{:064x}", index + 201);
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.executable, 64, "{report:#?}");
        assert_eq!(report.support_rows, 32, "{report:#?}");
        assert_eq!(report.future_rows, 32, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 32, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");

        let candidates = state.admission_candidates();
        let snapshot = crate::build_crystallized_admission_snapshot(
            &candidates,
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("external admission evaluates sealed operator")
        .expect("sealed operator reaches registry");
        let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
            .expect("registry restores typed call operator");
        let runtime = custom_tool_transition(777);
        let parity = runtime.runtime_parity_case.expect("runtime parity");
        let execution = executor.execute_shadow(&parity.request_text, &parity.provider_payload);
        assert!(
            execution.response.as_deref().is_some_and(|response| {
                crate::online_admission::responses_match_after_execution_budget_normalization(
                    response,
                    &parity.expected_response,
                )
            }),
            "{execution:#?}"
        );
    }

    #[test]
    fn repeated_physical_call_roles_collapse_before_authority_budget() {
        let broad =
            extract_live_scalar_circuit_sample(&custom_tool_transition_with_repeated_outputs(1))
                .expect("bounded physical adapter space");
        assert!(
            broad.actor_hypotheses.len() > crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
            "the fixture must exercise the intermediate version space"
        );
        assert!(broad.actor_hypotheses.len() <= TEACHER_CALL_SELECTOR_BUDGET);

        let narrow = extract_live_scalar_circuit_sample(&custom_tool_transition(2))
            .expect("independent narrow surface");
        let mut law = LiveScalarLawState::default();
        update_support_actor_hypotheses(&mut law, &broad.actor_hypotheses)
            .expect("first surface initializes the version space");
        update_support_actor_hypotheses(&mut law, &narrow.actor_hypotheses)
            .expect("second surface intersects the version space");

        assert!(!law.support_actor_hypotheses.is_empty());
        assert!(
            law.support_actor_hypotheses.len() <= crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS
        );
    }

    #[test]
    fn all_teacher_field_selectors_can_become_structural_hints() {
        for selector in [
            ResponseValueSelector::JsonField {
                field: "opaque".to_owned(),
                value_type: AtomValueType::Integer,
            },
            ResponseValueSelector::UniqueTurnJsonField {
                field: "opaque".to_owned(),
                value_type: AtomValueType::Integer,
            },
            ResponseValueSelector::UniqueActiveTurnJsonField {
                field: "opaque".to_owned(),
                value_type: AtomValueType::Integer,
            },
        ] {
            assert_eq!(
                teacher_field_selector_hint(&selector),
                Some(("opaque", AtomValueType::Integer))
            );
        }
    }

    #[test]
    fn count_rows_reach_verified_cpu_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..64_u8 {
            let count = usize::from(index % 7) + 1;
            let mut row =
                collection_count_transition_n("Count the records", "Total records: ", count);
            row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
            row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
            row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.support_rows, 32, "{report:#?}");
        assert_eq!(report.future_rows, 32, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 32, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");

        let candidates = state.admission_candidates();
        let snapshot = crate::build_crystallized_admission_snapshot(
            &candidates,
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("external admission evaluates count operator")
        .expect("count operator reaches registry");
        let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
            .expect("registry restores count operator");
        let rows = (0..11)
            .map(|value| json!({"id": value}))
            .collect::<Vec<_>>();
        let execution = executor.execute_shadow(
            "Count the records",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": serde_json::to_string(&rows).expect("rows serialize")
                }]
            }),
        );
        assert_eq!(
            execution.response.as_deref(),
            Some("Total records: 11."),
            "{execution:#?}"
        );
    }

    #[test]
    fn status_rows_reach_verified_cpu_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..64_u8 {
            let mut row = status_transition(&format!("status_{index}"), u64::from(index % 5));
            row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
            row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
            row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.support_rows, 32, "{report:#?}");
        assert_eq!(report.future_rows, 32, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 32, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");

        let candidates = state.admission_candidates();
        let snapshot = crate::build_crystallized_admission_snapshot(
            &candidates,
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("external admission evaluates status operator")
        .expect("status operator reaches registry");
        let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
            .expect("registry restores status operator");
        let execution = executor.execute_shadow(
            "Check build status",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"new_status_field\":0}"
                }]
            }),
        );
        assert_eq!(
            execution.response.as_deref(),
            Some("Build status: OK."),
            "{execution:#?}"
        );
    }

    #[test]
    fn filter_rows_reach_verified_cpu_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..64_u8 {
            let predicate = if index % 2 == 0 { "active" } else { "ready" };
            let mut row = filter_transition(&format!("state_{index}"), predicate);
            row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
            row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
            row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.support_rows, 32, "{report:#?}");
        assert_eq!(report.future_rows, 32, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 32, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");

        let candidates = state.admission_candidates();
        let snapshot = crate::build_crystallized_admission_snapshot(
            &candidates,
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("external admission evaluates filter operator")
        .expect("filter operator reaches registry");
        let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
            .expect("registry restores filter operator");
        let rows = vec![
            json!({"new_kind": "active", "score": 11}),
            json!({"new_kind": "idle", "score": 12}),
        ];
        let payload = json!({
            "input": [
                {"type": "message", "role": "user", "content": "Filter active"},
                {
                    "type": "function_call_output",
                    "output": serde_json::to_string(&rows).expect("rows serialize")
                }
            ]
        });
        let execution = executor.execute_shadow("Filter active", &payload);
        assert_eq!(
            execution.response.as_deref(),
            Some("[{\"new_kind\":\"active\",\"score\":11}]"),
            "{execution:#?}"
        );
    }

    #[test]
    fn filter_count_rows_reach_verified_cpu_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..64_u8 {
            let predicate = if index % 2 == 0 { "active" } else { "ready" };
            let mut row = filter_count_transition(&format!("state_{index}"), predicate);
            row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
            row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
            row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.support_rows, 32, "{report:#?}");
        assert_eq!(report.future_rows, 32, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 32, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");

        let snapshot = crate::build_crystallized_admission_snapshot(
            &state.admission_candidates(),
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("external admission evaluates composed operator")
        .expect("composed operator reaches registry");
        let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
            .expect("registry restores composed operator");
        let rows = vec![
            json!({"new_kind": "active", "score": 11}),
            json!({"new_kind": "idle", "score": 12}),
        ];
        let payload = json!({
            "input": [
                {"type": "message", "role": "user", "content": "Filter active"},
                {
                    "type": "function_call_output",
                    "output": serde_json::to_string(&rows).expect("rows serialize")
                }
            ]
        });
        let execution = executor.execute_shadow("Filter active", &payload);
        assert_eq!(
            execution.response.as_deref(),
            Some("Matching records: 1."),
            "{execution:#?}"
        );
    }

    #[test]
    fn multi_role_rows_reach_verified_crystallized_operator() {
        let mut state = LiveScalarShadowState::default();
        for index in 0..64_u8 {
            let first = format!("total_{index}");
            let second = format!("failed_{index}");
            let first_value = u16::from(index) + 100;
            let second_value = if index < 32 {
                first_value
            } else {
                u16::from(index) + 10
            };
            let mut row = multi_value_transition(
                &first,
                &second,
                &format!("Total: {first_value}; failed: {second_value}"),
            );
            row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
            row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
            row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
            row.runtime_parity_case
                .as_mut()
                .expect("parity")
                .provider_payload = json!({
                "input": [{
                    "type": "function_call_output",
                    "output": format!(
                        "{{\"{first}\":{first_value},\"{second}\":{second_value}}}"
                    )
                }]
            });
            if index == 0 {
                let parity = row.runtime_parity_case.as_ref().expect("parity");
                let observed = crate::runtime::observed_request_ordinal_roles(
                    &parity.request_text,
                    &parity.provider_payload,
                )
                .expect("observed equal roles");
                assert_eq!(observed.len(), 2, "both JSON paths must remain observable");
                let sample =
                    extract_live_scalar_circuit_sample(&row).expect("equal support sample");
                let expanded = bounded_ordinal_role_permutations(
                    &sample.actor_template,
                    &parity.request_text,
                    &parity.provider_payload,
                    &parity.expected_response,
                )
                .expect("bounded equal-role expansion");
                assert_eq!(
                    expanded.len(),
                    2,
                    "renderer must retain both executable role orders: {expanded:#?}"
                );
                assert_eq!(
                    sample.actor_hypotheses.len(),
                    3,
                    "equal-value support must retain repeated-role plus both role orders: {:#?}",
                    sample.actor_hypotheses
                );
            }
            state.observe(&row);
        }

        let report = state.report();
        assert_eq!(report.executable, 64, "{report:#?}");
        assert_eq!(report.support_rows, 32, "{report:#?}");
        assert_eq!(report.future_rows, 32, "{report:#?}");
        assert_eq!(report.frozen_laws, 1, "{report:#?}");
        assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
        assert_eq!(report.shadow_executions, 32, "{report:#?}");
        assert_eq!(report.admission_candidates, 1, "{report:#?}");

        let candidates = state.admission_candidates();
        let bundle = candidates[0]
            .package
            .crystallized_operator
            .as_ref()
            .expect("restart bundle");
        let restored = crate::VerifiedCrystallizedOperator::restore(
            bundle.page_bytes(),
            bundle.registry_cbor(),
        )
        .expect("restore rich operator before admission");
        for (index, row) in candidates[0]
            .support
            .iter()
            .chain(&candidates[0].future)
            .enumerate()
        {
            let parity = row.runtime_parity_case.as_ref().expect("parity row");
            let bound = restored
                .bind_pre_action(&parity.request_text, &parity.provider_payload)
                .unwrap_or_else(|error| panic!("rich bind row {index}: {error:?}"));
            let response = bound
                .execute_verified()
                .unwrap_or_else(|error| panic!("rich execute row {index}: {error:?}"));
            assert_eq!(response, parity.expected_response, "rich row {index}");
        }
        let snapshot = crate::build_crystallized_admission_snapshot(
            &candidates,
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("external admission verifies rich operator")
        .expect("rich operator reaches registry");
        let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
            .expect("hot executor restores rich operator");
        let execution = executor.execute_shadow(
            "Return new_total and new_failed",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"new_total\":777,\"new_failed\":9}"
                }]
            }),
        );
        assert_eq!(
            execution.response.as_deref(),
            Some("Total: 777; failed: 9"),
            "{execution:#?}"
        );
        let reversed = executor.execute_shadow(
            "Return new_failed and new_total",
            &json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"new_total\":777,\"new_failed\":9}"
                }]
            }),
        );
        assert_eq!(
            reversed.response.as_deref(),
            Some("Total: 9; failed: 777"),
            "request ordinal, not field name or JSON order, owns the role: {reversed:#?}"
        );
    }
}
