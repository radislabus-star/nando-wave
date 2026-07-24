use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    MultiSourceExtractionStatusV1, OperatorGenerationComponentRootsV3, ProgramSemanticClassIdV1,
    ProgramSemanticClassInputV1, RelationFrame, ResponseProgram, canonical_json_sha256,
    response_program_required_routing_atom_ids, response_program_version_root_sha256,
    seal_operator_generation_manifest_v3, seal_program_semantic_class_v1,
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
    PreActionShapeClassV1, factor_multi_source_row_v1,
};

pub const MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V1: &str =
    "nando.multi-source-t1-identification.v1";

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
    pub expected_partition_gain: usize,
    pub estimated_cost_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MultiSourceT1IdentificationV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub evidence_epoch_sha256: String,
    pub selected_shape_root_sha256: Option<String>,
    pub selected_marginal_input_tokens: u64,
    pub candidate_programs: usize,
    pub semantic_classes_remaining: usize,
    pub support_rows: usize,
    pub support_lineages: usize,
    pub zero_gain_observations: usize,
    pub support_reuse_rows: usize,
    pub independent_future_rows: usize,
    pub independent_future_lineages: usize,
    pub wrong_role_bindings: usize,
    pub negative_accepts: usize,
    pub candidate_freeze: Option<CandidateFreezeReceiptV1>,
    pub canonical_program: Option<ResponseProgram>,
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
}

#[derive(Serialize)]
struct T1IdentificationDigest<'a> {
    schema: &'static str,
    evidence_epoch_sha256: &'a str,
    selected_shape_root_sha256: Option<&'a str>,
    selected_marginal_input_tokens: u64,
    candidate_programs: usize,
    semantic_classes_remaining: usize,
    support_rows: usize,
    support_lineages: usize,
    zero_gain_observations: usize,
    support_reuse_rows: usize,
    independent_future_rows: usize,
    independent_future_lineages: usize,
    wrong_role_bindings: usize,
    negative_accepts: usize,
    candidate_freeze: &'a Option<CandidateFreezeReceiptV1>,
    canonical_program: &'a Option<ResponseProgram>,
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
) -> MultiSourceT1IdentificationV1 {
    let frame_by_root = frames
        .iter()
        .filter_map(|frame| {
            canonical_json_sha256(frame)
                .ok()
                .map(|root| (root, frame.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut cohorts = BTreeMap::<String, Vec<EligibleT1Row>>::new();
    for joined in joined_rows {
        let factorized = factor_multi_source_row_v1(joined);
        if !matches!(
            factorized.pre_action_shape,
            PreActionShapeClassV1::SingleRoleProjection
                | PreActionShapeClassV1::OneOutputManyScalarRoles
        )
            || factorized.completed_effect != CompletedEffectFormV1::SingleRoleProjection
            || !matches!(
                joined.topology.extraction_status,
                MultiSourceExtractionStatusV1::Complete
            )
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
        cohorts
            .entry(factorized.applicability_shape_root_sha256.clone())
            .or_default()
            .push(EligibleT1Row {
                joined: joined.clone(),
                frame: frame.clone(),
                factorized,
            });
    }
    let Some((shape_root, mut cohort)) = select_highest_marginal_cohort(cohorts) else {
        return terminal_report(
            evidence_epoch_sha256,
            MultiSourceT1IdentificationStateV1::NoEligibleCohort,
            "complete_single_role_projection_missing",
        );
    };
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

    let candidate_programs =
        crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(&seed.frame))
            .into_iter()
            .filter(|program| program.validate().is_ok())
            .filter_map(|program| {
                response_program_version_root_sha256(&program)
                    .ok()
                    .map(|root| (root, program))
            })
            .collect::<BTreeMap<_, _>>();
    if candidate_programs.is_empty() {
        return selected_terminal_report(
            evidence_epoch_sha256,
            shape_root,
            selected_marginal_input_tokens,
            MultiSourceT1IdentificationStateV1::CandidateGenerationEmpty,
            "bounded_t1_grammar_generated_no_program",
        );
    }
    let manifest = match generation_manifest(&shape_root, &candidate_programs) {
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
        let descriptor = match semantic_descriptor(&shape_root, program_root, program) {
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
    let mut support_lineages = BTreeSet::new();
    let mut future_candidates = Vec::new();
    for row in accepted {
        if freeze.is_some() {
            future_candidates.push(row);
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
                    "nando.multi-source-t1-applicability-scope.v1",
                    shape_root.as_str(),
                    class.semantic_class().class_id().as_str(),
                    class.canonical_program_root_sha256(),
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
        return finalize_report(MultiSourceT1IdentificationV1 {
            schema: MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V1.to_owned(),
            report_root_sha256: String::new(),
            evidence_epoch_sha256,
            selected_shape_root_sha256: Some(shape_root),
            selected_marginal_input_tokens,
            candidate_programs: registered.len(),
            semantic_classes_remaining: metrics.semantic_classes_remaining,
            support_rows: metrics.observations,
            support_lineages: support_lineages.len(),
            zero_gain_observations: metrics.zero_gain_observations,
            support_reuse_rows: 0,
            independent_future_rows: 0,
            independent_future_lineages: 0,
            wrong_role_bindings: 0,
            negative_accepts: 0,
            candidate_freeze: None,
            canonical_program: None,
            passive_probe,
            exact_transfer_parity: false,
            runtime_actor_verifier_parity: false,
            state: MultiSourceT1IdentificationStateV1::Ambiguous,
            blocker: Some("multiple_semantic_classes_require_distinguishing_evidence".to_owned()),
            execution_authority: false,
        });
    };
    let selected_program = canonical_program.expect("freeze owns canonical program");
    let mut support_reuse_rows = 0usize;
    let mut wrong_role_bindings = 0usize;
    for row in future_candidates {
        if support_lineages.contains(&row.joined.session_lineage_sha256) {
            support_reuse_rows = support_reuse_rows.saturating_add(1);
            continue;
        }
        let observation = match observation_for_row(&row, &registered) {
            Ok(observation) => observation,
            Err(_) => {
                wrong_role_bindings = wrong_role_bindings.saturating_add(1);
                continue;
            }
        };
        if machine.apply_future(observation).is_err() {
            wrong_role_bindings = wrong_role_bindings.saturating_add(1);
        }
    }
    let accounting = machine
        .evidence_ledger()
        .map(|ledger| ledger.accounting())
        .unwrap_or_default();
    let negative_accepts = cohort
        .iter()
        .filter(|row| {
            !row.joined.accepted
                && crate::synthesis::program_is_consistent(&selected_program, &row.frame)
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
    finalize_report(MultiSourceT1IdentificationV1 {
        schema: MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        evidence_epoch_sha256,
        selected_shape_root_sha256: Some(shape_root),
        selected_marginal_input_tokens,
        candidate_programs: registered.len(),
        semantic_classes_remaining: metrics.semantic_classes_remaining,
        support_rows: accounting.support_rows,
        support_lineages: accounting.support_lineages,
        zero_gain_observations: metrics.zero_gain_observations,
        support_reuse_rows,
        independent_future_rows: accounting.future_rows,
        independent_future_lineages: accounting.future_lineages,
        wrong_role_bindings,
        negative_accepts,
        candidate_freeze: Some(candidate_freeze),
        canonical_program: Some(selected_program),
        passive_probe: None,
        exact_transfer_parity,
        runtime_actor_verifier_parity: false,
        state,
        blocker,
        execution_authority: false,
    })
}

fn select_highest_marginal_cohort(
    cohorts: BTreeMap<String, Vec<EligibleT1Row>>,
) -> Option<(String, Vec<EligibleT1Row>)> {
    cohorts
        .into_iter()
        .filter(|(_, rows)| rows.iter().any(|row| row.joined.accepted))
        .max_by(|(left_root, left), (right_root, right)| {
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
                .then_with(|| right_root.cmp(left_root))
        })
}

fn generation_manifest(
    shape_root: &str,
    programs: &BTreeMap<String, ResponseProgram>,
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
    seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: canonical_json_sha256(&(
                "nando.multi-source-t1-candidate-set.v1",
                &candidate_roots,
            ))
            .map_err(|_| "candidate_set_commitment_failed".to_owned())?,
            dispatch_index_sha256: canonical_json_sha256(&(
                "nando.multi-source-t1-dispatch.v1",
                shape_root,
            ))
            .map_err(|_| "dispatch_commitment_failed".to_owned())?,
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
            resource_budget_sha256: canonical_json_sha256(&(
                "nando.multi-source-t1-budget.v1",
                programs.len(),
                VersionSpaceConfig::default(),
            ))
            .map_err(|_| "resource_budget_commitment_failed".to_owned())?,
        },
    )
    .map_err(|error| format!("generation_manifest:{error:?}").to_lowercase())
}

fn semantic_descriptor(
    shape_root: &str,
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
        protocol_mode_set_root_sha256: canonical_json_sha256(&(
            "nando.multi-source-t1-protocol-mode.v1",
            response_program_required_routing_atom_ids(program),
        ))
        .map_err(|_| "protocol_mode_commitment_failed".to_owned())?,
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
    let evaluations = programs
        .iter()
        .map(|(root, program)| {
            let accepted = crate::synthesis::program_is_consistent(program, &row.frame);
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
    seal_operator_observation_v1(OperatorObservationInputV1 {
        capture_sequence: row.joined.capture_sequence,
        lineage_root_sha256: row.joined.session_lineage_sha256.clone(),
        event_root_sha256: row.joined.action_event_id_sha256.clone(),
        request_root_sha256: row.joined.request_event_id_sha256.clone(),
        pre_action_relation_root_sha256: row.joined.topology_commitment_root_sha256.clone(),
        observed_action_root_sha256: row.joined.semantic_action_root_sha256.clone(),
        observed_delta_root_sha256: canonical_json_sha256(&(
            "nando.multi-source-t1-observed-delta.v1",
            row.joined.completed_frame_root_sha256.as_str(),
            &row.joined.effect_atoms,
            row.joined.accepted,
        ))
        .map_err(|_| "observed_delta_commitment_failed".to_owned())?,
        verifier_receipt_root_sha256: row.joined.verifier_receipt_root_sha256.clone(),
        outcome: GenerationLearningOutcomeV3::VerifiedPass,
        evaluations,
    })
    .map_err(|error| format!("operator_observation:{error}"))
}

fn passive_probe(
    shape_root: &str,
    machine: &OperatorIdentificationMachineV1,
    programs: &BTreeMap<String, ResponseProgram>,
    class_by_program: &BTreeMap<String, ProgramSemanticClassIdV1>,
) -> Option<PassiveT1ProbeContractV1> {
    let OperatorIdentificationStateV1::Ambiguous { report } = machine.state().ok()? else {
        return None;
    };
    let mut predictions = Vec::new();
    for class_id in &report.competing_class_ids {
        let observable_signatures = class_by_program
            .iter()
            .filter(|(_, candidate_class)| *candidate_class == class_id)
            .filter_map(|(root, _)| programs.get(root))
            .map(response_program_required_routing_atom_ids)
            .collect::<BTreeSet<_>>();
        predictions.push(ProbeClassPredictionV1 {
            class_id: class_id.clone(),
            outcome_partition_root_sha256: canonical_json_sha256(&(
                "nando.multi-source-t1-passive-observable.v1",
                &observable_signatures,
            ))
            .ok()?,
        });
    }
    let observable_difference_root_sha256 =
        canonical_json_sha256(&("nando.multi-source-t1-passive-difference.v1", &predictions))
            .ok()?;
    let probe = select_distinguishing_probe_v1(
        &report.competing_class_ids,
        &[DistinguishingProbeCandidateV1 {
            probe_root_sha256: canonical_json_sha256(&(
                "nando.multi-source-t1-passive-probe.v1",
                shape_root,
            ))
            .ok()?,
            observable_difference_root_sha256,
            source: EvidenceSourceContractV1::PassiveLiveTraffic,
            estimated_cost_units: 1,
            predictions,
        }],
    )
    .ok()?;
    Some(PassiveT1ProbeContractV1 {
        probe_root_sha256: probe.probe_root_sha256().to_owned(),
        observable_difference_root_sha256: probe.observable_difference_root_sha256().to_owned(),
        competing_class_roots_sha256: probe.competing_class_roots_sha256().to_vec(),
        expected_partition_gain: probe.expected_partition_gain(),
        estimated_cost_units: probe.estimated_cost_units(),
    })
}

fn terminal_report(
    evidence_epoch_sha256: String,
    state: MultiSourceT1IdentificationStateV1,
    blocker: impl Into<String>,
) -> MultiSourceT1IdentificationV1 {
    finalize_report(MultiSourceT1IdentificationV1 {
        schema: MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        evidence_epoch_sha256,
        selected_shape_root_sha256: None,
        selected_marginal_input_tokens: 0,
        candidate_programs: 0,
        semantic_classes_remaining: 0,
        support_rows: 0,
        support_lineages: 0,
        zero_gain_observations: 0,
        support_reuse_rows: 0,
        independent_future_rows: 0,
        independent_future_lineages: 0,
        wrong_role_bindings: 0,
        negative_accepts: 0,
        candidate_freeze: None,
        canonical_program: None,
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
) -> MultiSourceT1IdentificationV1 {
    let mut report = terminal_report(evidence_epoch_sha256, state, blocker);
    report.selected_shape_root_sha256 = Some(shape_root);
    report.selected_marginal_input_tokens = selected_marginal_input_tokens;
    finalize_report(report)
}

fn finalize_report(mut report: MultiSourceT1IdentificationV1) -> MultiSourceT1IdentificationV1 {
    report.report_root_sha256 = report.expected_root();
    report
}

impl MultiSourceT1IdentificationV1 {
    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&T1IdentificationDigest {
            schema: MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V1,
            evidence_epoch_sha256: self.evidence_epoch_sha256.as_str(),
            selected_shape_root_sha256: self.selected_shape_root_sha256.as_deref(),
            selected_marginal_input_tokens: self.selected_marginal_input_tokens,
            candidate_programs: self.candidate_programs,
            semantic_classes_remaining: self.semantic_classes_remaining,
            support_rows: self.support_rows,
            support_lineages: self.support_lineages,
            zero_gain_observations: self.zero_gain_observations,
            support_reuse_rows: self.support_reuse_rows,
            independent_future_rows: self.independent_future_rows,
            independent_future_lineages: self.independent_future_lineages,
            wrong_role_bindings: self.wrong_role_bindings,
            negative_accepts: self.negative_accepts,
            candidate_freeze: &self.candidate_freeze,
            canonical_program: &self.canonical_program,
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
        if self.schema != MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V1
            || self.execution_authority
            || self.report_root_sha256 != self.expected_root()
            || self
                .candidate_freeze
                .as_ref()
                .is_some_and(|freeze| freeze.validate().is_err())
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
                    && !self.exact_transfer_parity
            }
            MultiSourceT1IdentificationStateV1::Ambiguous => {
                self.candidate_freeze.is_none() && self.canonical_program.is_none()
            }
            _ => !self.exact_transfer_parity,
        }
    }
}
