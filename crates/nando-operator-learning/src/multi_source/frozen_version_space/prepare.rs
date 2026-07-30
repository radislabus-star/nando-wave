use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    ProgramSemanticClassIdV1, RelationFrame, ResponseProgram, canonical_json_sha256,
    valid_nonzero_sha256,
};

use crate::{
    CandidateSearchCompletion, OperatorIdentificationMachineV1, OperatorIdentificationStateV1,
    VersionSpaceConfig,
};

use super::super::{
    Ms3LinkedFrameAcquisitionReportV1, Ms3LinkedFrameAcquisitionVerdictV1, Ms3LinkedFrameReceiptV1,
    TransportBoundJoinedTransitionV1, factor_multi_source_row_v1,
    identification::{
        generation_manifest, observation_for_transition, passive_probe, semantic_descriptor,
        t1_probe_dimension_signature, t1_protocol_mode_root,
    },
    source_neutral_t1::enumerate_source_neutral_t1_candidates,
};
use super::types::{
    ClassPredictionsV1, Ms3FrozenVersionSpaceErrorV1, Ms3ZeroClassReasonV1,
    PreparedMs3VersionSpaceV1, PreparedStateV1,
};

pub fn prepare_ms3_frozen_version_space_v1(
    acquisition_report: &Ms3LinkedFrameAcquisitionReportV1,
    bound: &TransportBoundJoinedTransitionV1,
    frame: &RelationFrame,
) -> Result<PreparedMs3VersionSpaceV1, Ms3FrozenVersionSpaceErrorV1> {
    prepare_ms3_frozen_version_space(acquisition_report, bound, frame, None)
}

pub fn prepare_ms3_frozen_version_space_with_denominator_v1(
    acquisition_report: &Ms3LinkedFrameAcquisitionReportV1,
    bound: &TransportBoundJoinedTransitionV1,
    frame: &RelationFrame,
    scientific_denominator_receipt_root_sha256: &str,
) -> Result<PreparedMs3VersionSpaceV1, Ms3FrozenVersionSpaceErrorV1> {
    if !valid_nonzero_sha256(scientific_denominator_receipt_root_sha256) {
        return Err(Ms3FrozenVersionSpaceErrorV1::InvalidScientificDenominator);
    }
    prepare_ms3_frozen_version_space(
        acquisition_report,
        bound,
        frame,
        Some(scientific_denominator_receipt_root_sha256.to_owned()),
    )
}

fn prepare_ms3_frozen_version_space(
    acquisition_report: &Ms3LinkedFrameAcquisitionReportV1,
    bound: &TransportBoundJoinedTransitionV1,
    frame: &RelationFrame,
    scientific_denominator_receipt_root_sha256: Option<String>,
) -> Result<PreparedMs3VersionSpaceV1, Ms3FrozenVersionSpaceErrorV1> {
    let linked_receipt = validate_linked_evidence(acquisition_report, bound, frame)?;
    let factorized = factor_multi_source_row_v1(&bound.joined);
    let support_watermark = bound.joined.capture_sequence;
    let support_rows_root_sha256 =
        if let Some(denominator_root) = &scientific_denominator_receipt_root_sha256 {
            canonical_json_sha256(&(
                "nando.ms3-version-space-support-rows.v2",
                denominator_root.as_str(),
                linked_receipt.receipt_root_sha256.as_str(),
                bound.joined.join_root_sha256.as_str(),
                support_watermark,
            ))
        } else {
            canonical_json_sha256(&(
                "nando.ms3-version-space-support-rows.v1",
                linked_receipt.receipt_root_sha256.as_str(),
                bound.joined.join_root_sha256.as_str(),
                support_watermark,
            ))
        }
        .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)?;

    let candidates = match enumerate_source_neutral_t1_candidates(&bound.joined, frame) {
        Ok(candidates) => candidates,
        Err(blocker) => {
            if reopens_representation_gap(blocker) {
                return Err(Ms3FrozenVersionSpaceErrorV1::RepresentationGapReopened);
            }
            let machine = empty_machine(
                &factorized.applicability_shape_root_sha256,
                linked_receipt.receipt_root_sha256.as_str(),
            )?;
            return Ok(PreparedMs3VersionSpaceV1 {
                acquisition_report_root_sha256: acquisition_report.report_root_sha256.clone(),
                scientific_denominator_receipt_root_sha256,
                linked_receipt,
                extractor_schema: frame.schema.clone(),
                extractor_version: frame.extractor_version.clone(),
                support_rows_root_sha256,
                support_watermark,
                candidate_program_roots_sha256: Vec::new(),
                semantic_class_roots_sha256: Vec::new(),
                quotient_root_sha256: empty_root("quotient", blocker)?,
                class_predictions_root_sha256: empty_root("predictions", blocker)?,
                passive_probe: None,
                state: PreparedStateV1::ZeroClasses {
                    reason: zero_class_reason(blocker),
                    blocker: blocker.to_owned(),
                },
                machine,
            });
        }
    };
    build_prepared_space(
        acquisition_report,
        linked_receipt,
        bound,
        frame,
        factorized.applicability_shape_root_sha256,
        support_rows_root_sha256,
        support_watermark,
        scientific_denominator_receipt_root_sha256,
        candidates,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_prepared_space(
    acquisition_report: &Ms3LinkedFrameAcquisitionReportV1,
    linked_receipt: Ms3LinkedFrameReceiptV1,
    bound: &TransportBoundJoinedTransitionV1,
    frame: &RelationFrame,
    shape_root: String,
    support_rows_root_sha256: String,
    support_watermark: u64,
    scientific_denominator_receipt_root_sha256: Option<String>,
    candidates: BTreeMap<String, ResponseProgram>,
) -> Result<PreparedMs3VersionSpaceV1, Ms3FrozenVersionSpaceErrorV1> {
    let protocol_mode_root_sha256 = t1_protocol_mode_root(&candidates)
        .map_err(|error| Ms3FrozenVersionSpaceErrorV1::CandidateGeneration(error.to_owned()))?;
    let manifest = generation_manifest(&shape_root, &protocol_mode_root_sha256, &candidates)
        .map_err(Ms3FrozenVersionSpaceErrorV1::CandidateGeneration)?;
    let mut machine = OperatorIdentificationMachineV1::new(
        manifest,
        VersionSpaceConfig {
            max_complete_candidates: 4_096,
            ..VersionSpaceConfig::default()
        },
    );
    let mut registered = BTreeMap::<String, ResponseProgram>::new();
    let mut class_by_program = BTreeMap::<String, ProgramSemanticClassIdV1>::new();
    for (program_root, program) in &candidates {
        let descriptor = semantic_descriptor(
            &shape_root,
            &protocol_mode_root_sha256,
            program_root,
            program,
        )
        .map_err(Ms3FrozenVersionSpaceErrorV1::CandidateGeneration)?;
        let class_id = descriptor.class_id().clone();
        let registered_root = machine
            .register_candidate(program.clone(), descriptor)
            .map_err(|error| {
                Ms3FrozenVersionSpaceErrorV1::CandidateRegistration(error.to_string())
            })?;
        registered.insert(registered_root.clone(), program.clone());
        class_by_program.insert(registered_root, class_id);
    }
    if !matches!(
        machine.complete_candidate_generation(),
        CandidateSearchCompletion::Complete
    ) {
        return Err(Ms3FrozenVersionSpaceErrorV1::CandidateSearchIncomplete);
    }
    let observation = observation_for_transition(&bound.joined, frame, &registered)
        .map_err(Ms3FrozenVersionSpaceErrorV1::SupportReplay)?;
    let accepted_program_roots = observation
        .evaluations()
        .iter()
        .filter(|evaluation| evaluation.accepted)
        .map(|evaluation| evaluation.program_digest_sha256.clone())
        .collect::<BTreeSet<_>>();
    let state = machine
        .apply_support(observation)
        .map_err(|error| Ms3FrozenVersionSpaceErrorV1::SupportReplay(error.to_string()))?
        .state;
    let semantic_classes = accepted_program_roots
        .iter()
        .filter_map(|root| class_by_program.get(root))
        .map(|class| class.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let quotient_root_sha256 = canonical_json_sha256(&(
        "nando.ms3-version-space-semantic-quotient.v1",
        accepted_program_roots.iter().collect::<Vec<_>>(),
        &semantic_classes,
    ))
    .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)?;
    let class_predictions_root_sha256 =
        class_predictions_root(&registered, &class_by_program, &accepted_program_roots)?;
    let passive_probe = passive_probe(&shape_root, &machine, &registered, &class_by_program);
    let prepared_state = match state {
        OperatorIdentificationStateV1::Identified { class } => {
            let canonical_program = registered
                .get(class.canonical_program_root_sha256())
                .cloned()
                .ok_or_else(|| {
                    Ms3FrozenVersionSpaceErrorV1::CandidateGeneration(
                        "canonical_program_missing".to_owned(),
                    )
                })?;
            PreparedStateV1::Unique {
                semantic_class_root_sha256: class.semantic_class().class_id().as_str().to_owned(),
                canonical_program: Box::new(canonical_program),
                protocol_mode_root_sha256,
            }
        }
        OperatorIdentificationStateV1::Ambiguous { report } => PreparedStateV1::Ambiguous {
            semantic_classes: report.surviving_semantic_classes,
        },
        OperatorIdentificationStateV1::Empty { .. } => PreparedStateV1::ZeroClasses {
            reason: Ms3ZeroClassReasonV1::SelfReplayInconsistency,
            blocker: "support_eliminated_all_candidates".to_owned(),
        },
        other => {
            return Err(Ms3FrozenVersionSpaceErrorV1::SupportReplay(format!(
                "unexpected_identification_state:{other:?}"
            )));
        }
    };
    Ok(PreparedMs3VersionSpaceV1 {
        acquisition_report_root_sha256: acquisition_report.report_root_sha256.clone(),
        scientific_denominator_receipt_root_sha256,
        linked_receipt,
        extractor_schema: frame.schema.clone(),
        extractor_version: frame.extractor_version.clone(),
        support_rows_root_sha256,
        support_watermark,
        candidate_program_roots_sha256: candidates.keys().cloned().collect(),
        semantic_class_roots_sha256: semantic_classes,
        quotient_root_sha256,
        class_predictions_root_sha256,
        passive_probe,
        state: prepared_state,
        machine,
    })
}

fn validate_linked_evidence(
    acquisition_report: &Ms3LinkedFrameAcquisitionReportV1,
    bound: &TransportBoundJoinedTransitionV1,
    frame: &RelationFrame,
) -> Result<Ms3LinkedFrameReceiptV1, Ms3FrozenVersionSpaceErrorV1> {
    if !acquisition_report.validate()
        || acquisition_report.verdict != Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved
        || acquisition_report.authority_ready
        || acquisition_report.phase_update_allowed
    {
        return Err(Ms3FrozenVersionSpaceErrorV1::InvalidAcquisition);
    }
    let frame_root = canonical_json_sha256(frame)
        .map_err(|_| Ms3FrozenVersionSpaceErrorV1::LinkedEvidenceMismatch)?;
    let receipt = acquisition_report
        .receipts
        .iter()
        .find(|receipt| {
            receipt.gap_class.is_none()
                && receipt.topology_commitment_root_sha256
                    == bound.binding.topology_commitment_root_sha256
                && receipt.completed_frame_root_sha256 == frame_root
                && receipt.terminal_receipt_root_sha256
                    == bound.binding.terminal_receipt_root_sha256
                && receipt.transport_binding_root_sha256 == bound.binding.binding_root_sha256
        })
        .cloned()
        .ok_or(Ms3FrozenVersionSpaceErrorV1::LinkedReceiptMissing)?;
    let identities_match = receipt.session_lineage_sha256 == bound.joined.session_lineage_sha256
        && receipt.session_id_sha256 == bound.joined.session_id_sha256
        && receipt.turn_intent_id_sha256 == bound.joined.turn_intent_id_sha256
        && receipt.request_event_id_sha256 == bound.joined.request_event_id_sha256
        && receipt.action_event_id_sha256 == bound.joined.action_event_id_sha256
        && receipt.topology_commitment_root_sha256 == bound.joined.topology_commitment_root_sha256
        && receipt.completed_frame_root_sha256 == bound.joined.completed_frame_root_sha256
        && receipt.validate();
    if !identities_match {
        return Err(Ms3FrozenVersionSpaceErrorV1::LinkedEvidenceMismatch);
    }
    Ok(receipt)
}

fn empty_machine(
    shape_root: &str,
    linked_receipt_root: &str,
) -> Result<OperatorIdentificationMachineV1, Ms3FrozenVersionSpaceErrorV1> {
    let empty_protocol_root =
        canonical_json_sha256(&("nando.ms3-empty-protocol-mode.v1", linked_receipt_root))
            .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)?;
    let manifest = generation_manifest(shape_root, &empty_protocol_root, &BTreeMap::new())
        .map_err(Ms3FrozenVersionSpaceErrorV1::CandidateGeneration)?;
    let mut machine = OperatorIdentificationMachineV1::new(manifest, VersionSpaceConfig::default());
    let _ = machine.complete_candidate_generation();
    Ok(machine)
}

fn empty_root(kind: &str, blocker: &str) -> Result<String, Ms3FrozenVersionSpaceErrorV1> {
    canonical_json_sha256(&("nando.ms3-empty-version-space.v1", kind, blocker))
        .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)
}

fn class_predictions_root(
    programs: &BTreeMap<String, ResponseProgram>,
    class_by_program: &BTreeMap<String, ProgramSemanticClassIdV1>,
    accepted_program_roots: &BTreeSet<String>,
) -> Result<String, Ms3FrozenVersionSpaceErrorV1> {
    let dimensions = ["role_binding", "temporal_rule", "renderer", "routing_atoms"];
    let predictions = accepted_program_roots
        .iter()
        .filter_map(|program_root| {
            let program = programs.get(program_root)?;
            let class = class_by_program.get(program_root)?;
            Some((
                class.as_str().to_owned(),
                program_root.clone(),
                dimensions
                    .iter()
                    .filter_map(|dimension| {
                        t1_probe_dimension_signature(program, dimension)
                            .map(|root| ((*dimension).to_owned(), root))
                    })
                    .collect::<BTreeMap<_, _>>(),
            ))
        })
        .collect::<ClassPredictionsV1>();
    canonical_json_sha256(&("nando.ms3-class-predictions.v1", predictions))
        .map_err(|_| Ms3FrozenVersionSpaceErrorV1::Serialization)
}

fn zero_class_reason(blocker: &str) -> Ms3ZeroClassReasonV1 {
    match blocker {
        "source_neutral_self_replay_failed" | "physical_transition_mismatch" => {
            Ms3ZeroClassReasonV1::SelfReplayInconsistency
        }
        "unsupported_t1_protocol_mode"
        | "unsupported_t1_role_selector"
        | "physical_program_selector_rewrite_failed" => Ms3ZeroClassReasonV1::UnsupportedRenderer,
        "physical_t1_program_missing" => Ms3ZeroClassReasonV1::ProgramAlgebraGap,
        "selected_observation_missing"
        | "selected_role_witness_missing"
        | "selected_structural_selector_missing"
        | "structural_role_missing_or_ambiguous" => {
            Ms3ZeroClassReasonV1::InvalidHypothesisGeneration
        }
        _ if blocker.contains("budget") => Ms3ZeroClassReasonV1::InvalidHypothesisGeneration,
        _ => Ms3ZeroClassReasonV1::PermanentAbstain,
    }
}

fn reopens_representation_gap(blocker: &str) -> bool {
    matches!(
        blocker,
        "selected_role_witness_missing"
            | "selected_structural_selector_missing"
            | "physical_program_selector_rewrite_failed"
    )
}
