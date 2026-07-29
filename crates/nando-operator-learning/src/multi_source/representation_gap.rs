use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    AtomSource, CollectionOutputRenderer, MultiSourceExtractionStatusV1,
    PreActionMultiSourceTopologyV1, RelationAtom, RelationFrame, ResponseOperation,
    ResponseRenderSegment, ResponseValueSelector, canonical_json_sha256, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use super::{
    BlindThenRevealJoinedTransitionV1, PreActionTopologyAuditRowV1,
    RequestStructureAuditSnapshotV1, TransportBindingLedgerV1, TransportTerminalReceiptV1,
    source_neutral_t1::enumerate_source_neutral_t1_candidates,
};

pub const REPRESENTATION_GAP_ADJUDICATION_SCHEMA_V1: &str =
    "nando.representation-gap-adjudication.v1";
pub const REPRESENTATION_GAP_REPORT_SCHEMA_V1: &str =
    "nando.representation-gap-adjudication-report.v1";
const MIN_TRANSFORM_SUPPORT_LINEAGES: usize = 3;
const MAX_GAP_ROWS: usize = 1_024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationGapClassV1 {
    CaptureGapA,
    TransformGapB,
    PostActionOnlyC,
    FreeGenerationD,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepresentationGapAdjudicationV1 {
    pub schema: String,
    pub adjudication_root_sha256: String,
    pub topology_commitment_root_sha256: String,
    pub transport_binding_root_sha256: String,
    pub completed_frame_root_sha256: String,
    pub gap_class: RepresentationGapClassV1,
    pub detail: String,
    pub pre_action_observation_values: usize,
    pub topology_witness_values: usize,
    pub direct_values_missing_from_topology: usize,
    pub transform_support_lineages: usize,
    pub representation_change_allowed: bool,
    pub requires_new_schema_epoch: bool,
    pub requires_new_post_freeze_future: bool,
    pub phase_update_allowed: bool,
    pub authority_ready: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepresentationGapAdjudicationReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub evidence_epoch_sha256: String,
    pub gap_denominator: usize,
    pub class_counts: BTreeMap<RepresentationGapClassV1, usize>,
    pub rows: Vec<RepresentationGapAdjudicationV1>,
    pub authority_ready: bool,
}

struct GapCase<'a> {
    topology: &'a PreActionTopologyAuditRowV1,
    joined: &'a BlindThenRevealJoinedTransitionV1,
    binding_root_sha256: &'a str,
    frame_root_sha256: &'a str,
    frame: &'a RelationFrame,
    transform_signature: Option<String>,
}

#[must_use]
pub fn build_representation_gap_adjudication_report_v1(
    mut requests: RequestStructureAuditSnapshotV1,
    mut frames: Vec<RelationFrame>,
    mut terminals: Vec<TransportTerminalReceiptV1>,
) -> RepresentationGapAdjudicationReportV1 {
    requests.topologies.sort_by(|left, right| {
        left.commit
            .commitment_root_sha256
            .cmp(&right.commit.commitment_root_sha256)
    });
    frames.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    terminals.sort_by(|left, right| {
        left.request_event_id_sha256
            .cmp(&right.request_event_id_sha256)
            .then_with(|| left.receipt_root_sha256.cmp(&right.receipt_root_sha256))
    });

    let ledger = TransportBindingLedgerV1::build(&requests.topologies, &frames, &terminals);
    let frame_by_root = frames
        .iter()
        .filter_map(|frame| canonical_json_sha256(frame).ok().map(|root| (root, frame)))
        .collect::<BTreeMap<_, _>>();
    let mut cases = Vec::new();
    for topology in &requests.topologies {
        if !eligible_topology(topology) {
            continue;
        }
        for bound in ledger.bound_for_topology(&topology.commit.commitment_root_sha256) {
            let Some(frame) = frame_by_root
                .get(&bound.joined.completed_frame_root_sha256)
                .copied()
            else {
                continue;
            };
            let Err(blocker) = enumerate_source_neutral_t1_candidates(&bound.joined, frame) else {
                continue;
            };
            if !matches!(
                blocker,
                "selected_role_witness_missing"
                    | "selected_structural_selector_missing"
                    | "physical_t1_program_missing"
                    | "physical_program_selector_rewrite_failed"
            ) {
                continue;
            }
            cases.push(GapCase {
                topology,
                joined: &bound.joined,
                binding_root_sha256: &bound.binding.binding_root_sha256,
                frame_root_sha256: &bound.joined.completed_frame_root_sha256,
                frame,
                transform_signature: existing_transform_signature(frame),
            });
            if cases.len() >= MAX_GAP_ROWS {
                break;
            }
        }
        if cases.len() >= MAX_GAP_ROWS {
            break;
        }
    }

    let transform_lineages = transform_lineage_counts(&cases);
    let mut rows = cases
        .into_iter()
        .map(|case| {
            let lineages = case
                .transform_signature
                .as_ref()
                .and_then(|signature| transform_lineages.get(signature))
                .map_or(0, BTreeSet::len);
            adjudicate_case(case, lineages)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.topology_commitment_root_sha256
            .cmp(&right.topology_commitment_root_sha256)
            .then_with(|| {
                left.completed_frame_root_sha256
                    .cmp(&right.completed_frame_root_sha256)
            })
    });
    let class_counts = rows.iter().fold(
        BTreeMap::<RepresentationGapClassV1, usize>::new(),
        |mut counts, row| {
            *counts.entry(row.gap_class).or_default() += 1;
            counts
        },
    );
    let evidence_epoch_sha256 = canonical_json_sha256(&(
        REPRESENTATION_GAP_REPORT_SCHEMA_V1,
        requests
            .topologies
            .iter()
            .map(|row| row.commit.commitment_root_sha256.as_str())
            .collect::<Vec<_>>(),
        frame_by_root.keys().map(String::as_str).collect::<Vec<_>>(),
        terminals
            .iter()
            .map(|receipt| receipt.receipt_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .expect("representation gap evidence epoch serializes");
    let mut report = RepresentationGapAdjudicationReportV1 {
        schema: REPRESENTATION_GAP_REPORT_SCHEMA_V1.to_owned(),
        report_root_sha256: String::new(),
        evidence_epoch_sha256,
        gap_denominator: rows.len(),
        class_counts,
        rows,
        authority_ready: false,
    };
    report.report_root_sha256 = report.expected_root();
    report
}

fn transform_lineage_counts(cases: &[GapCase<'_>]) -> BTreeMap<String, BTreeSet<String>> {
    let mut counts = BTreeMap::<String, BTreeSet<String>>::new();
    for case in cases {
        if let Some(signature) = &case.transform_signature {
            counts
                .entry(signature.clone())
                .or_default()
                .insert(case.joined.session_lineage_sha256.clone());
        }
    }
    counts
}

fn adjudicate_case(
    case: GapCase<'_>,
    transform_support_lineages: usize,
) -> RepresentationGapAdjudicationV1 {
    let topology = &case.topology.structure.topology;
    let evidence = gap_evidence(topology, case.frame);
    let (gap_class, detail) = classify_gap(&evidence, transform_support_lineages);
    let representation_change_allowed = matches!(
        gap_class,
        RepresentationGapClassV1::CaptureGapA | RepresentationGapClassV1::TransformGapB
    );
    let mut adjudication = RepresentationGapAdjudicationV1 {
        schema: REPRESENTATION_GAP_ADJUDICATION_SCHEMA_V1.to_owned(),
        adjudication_root_sha256: String::new(),
        topology_commitment_root_sha256: case.topology.commit.commitment_root_sha256.clone(),
        transport_binding_root_sha256: case.binding_root_sha256.to_owned(),
        completed_frame_root_sha256: case.frame_root_sha256.to_owned(),
        gap_class,
        detail: detail.to_owned(),
        pre_action_observation_values: evidence.pre_action_observation_values.len(),
        topology_witness_values: evidence.topology_witness_values.len(),
        direct_values_missing_from_topology: evidence.direct_values_missing_from_topology.len(),
        transform_support_lineages,
        representation_change_allowed,
        requires_new_schema_epoch: representation_change_allowed,
        requires_new_post_freeze_future: representation_change_allowed,
        phase_update_allowed: false,
        authority_ready: false,
    };
    adjudication.adjudication_root_sha256 = adjudication.expected_root();
    adjudication
}

struct GapEvidence {
    pre_action_observation_values: BTreeSet<String>,
    topology_witness_values: BTreeSet<String>,
    direct_values_missing_from_topology: BTreeSet<String>,
    existing_multi_role_transform: bool,
    post_action_values: usize,
    free_literal_action: bool,
}

fn gap_evidence(topology: &PreActionMultiSourceTopologyV1, frame: &RelationFrame) -> GapEvidence {
    let topology_witness_values = topology
        .role_witnesses
        .iter()
        .map(|witness| witness.value_sha256.clone())
        .collect::<BTreeSet<_>>();
    let observation_slots = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                slot_id,
                source: AtomSource::Observation | AtomSource::Request,
                value_sha256,
                ..
            } => Some((*slot_id, value_sha256.clone())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let selected_slots = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ObservationSelector { slot_id, .. } => Some(*slot_id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let pre_action_observation_values = selected_slots
        .iter()
        .filter_map(|slot| observation_slots.get(slot).cloned())
        .collect::<BTreeSet<_>>();
    let direct_values_missing_from_topology = pre_action_observation_values
        .difference(&topology_witness_values)
        .cloned()
        .collect();
    let post_action_values = frame
        .atoms
        .iter()
        .filter(|atom| {
            matches!(
                atom,
                RelationAtom::TypedSlot {
                    source: AtomSource::Action | AtomSource::Outcome,
                    ..
                }
            )
        })
        .count();
    let free_literal_action = frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ActionStringArgument { .. }
                | RelationAtom::ActionIntegerArgument { .. }
                | RelationAtom::ActionBooleanArgument { .. }
        )
    });
    GapEvidence {
        pre_action_observation_values,
        topology_witness_values,
        direct_values_missing_from_topology,
        existing_multi_role_transform: existing_transform_signature(frame).is_some(),
        post_action_values,
        free_literal_action,
    }
}

fn classify_gap(
    evidence: &GapEvidence,
    transform_support_lineages: usize,
) -> (RepresentationGapClassV1, &'static str) {
    if !evidence.direct_values_missing_from_topology.is_empty() {
        return (
            RepresentationGapClassV1::CaptureGapA,
            "pre_action_observation_missing_from_topology",
        );
    }
    if evidence.existing_multi_role_transform {
        return if transform_support_lineages >= MIN_TRANSFORM_SUPPORT_LINEAGES {
            (
                RepresentationGapClassV1::TransformGapB,
                "existing_dsl_transform_confirmed_across_independent_lineages",
            )
        } else {
            (
                RepresentationGapClassV1::InsufficientEvidence,
                "transform_candidate_requires_three_independent_lineages",
            )
        };
    }
    if evidence.pre_action_observation_values.is_empty() && evidence.free_literal_action {
        return (
            RepresentationGapClassV1::FreeGenerationD,
            "action_literal_has_no_pre_action_derivation",
        );
    }
    if evidence.pre_action_observation_values.is_empty() && evidence.post_action_values > 0 {
        return (
            RepresentationGapClassV1::PostActionOnlyC,
            "value_exists_only_in_action_or_outcome_state",
        );
    }
    (
        RepresentationGapClassV1::InsufficientEvidence,
        "backward_derivability_unresolved",
    )
}

fn existing_transform_signature(frame: &RelationFrame) -> Option<String> {
    if let Some(signature) = frame.atoms.iter().find_map(|atom| {
        let RelationAtom::ActionValueProjection {
            renderer: CollectionOutputRenderer::RenderSequence { segments },
            ..
        } = atom
        else {
            return None;
        };
        transform_signature_from_segments(segments)
    }) {
        return Some(signature);
    }
    crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(frame))
        .into_iter()
        .find_map(|program| {
            let ResponseOperation::ProjectSelectedValue { renderer, .. } = &program.operation
            else {
                return None;
            };
            let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
                return None;
            };
            transform_signature_from_segments(segments)
        })
}

fn transform_signature_from_segments(segments: &[ResponseRenderSegment]) -> Option<String> {
    let selectors = segments
        .iter()
        .filter_map(|segment| match segment {
            ResponseRenderSegment::Selected { selector, .. } => selector_value_type(selector),
            _ => None,
        })
        .collect::<Vec<_>>();
    (selectors.len() >= 2)
        .then(|| canonical_json_sha256(&("nando.existing-multi-role-transform.v1", selectors)).ok())
        .flatten()
}

const fn selector_value_type(
    selector: &ResponseValueSelector,
) -> Option<nando_operator_kernel::AtomValueType> {
    match selector {
        ResponseValueSelector::ContinuationHandle { value_type }
        | ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. } => Some(*value_type),
        _ => None,
    }
}

fn eligible_topology(row: &PreActionTopologyAuditRowV1) -> bool {
    row.physical_order_proven
        && row.structure.provider_bound_turn_identity
        && !matches!(
            row.structure.topology.extraction_status,
            MultiSourceExtractionStatusV1::Censored { .. }
        )
        && row.structure.validate().is_ok()
        && row.commit.validate().is_ok()
}

impl RepresentationGapAdjudicationV1 {
    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            REPRESENTATION_GAP_ADJUDICATION_SCHEMA_V1,
            self.topology_commitment_root_sha256.as_str(),
            self.transport_binding_root_sha256.as_str(),
            self.completed_frame_root_sha256.as_str(),
            self.gap_class,
            self.detail.as_str(),
            self.pre_action_observation_values,
            self.topology_witness_values,
            self.direct_values_missing_from_topology,
            self.transform_support_lineages,
            self.representation_change_allowed,
            self.requires_new_schema_epoch,
            self.requires_new_post_freeze_future,
            false,
            false,
        ))
        .expect("representation gap adjudication serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == REPRESENTATION_GAP_ADJUDICATION_SCHEMA_V1
            && [
                &self.adjudication_root_sha256,
                &self.topology_commitment_root_sha256,
                &self.transport_binding_root_sha256,
                &self.completed_frame_root_sha256,
            ]
            .into_iter()
            .all(|root| valid_nonzero_sha256(root))
            && !self.phase_update_allowed
            && !self.authority_ready
            && self.representation_change_allowed
                == matches!(
                    self.gap_class,
                    RepresentationGapClassV1::CaptureGapA | RepresentationGapClassV1::TransformGapB
                )
            && self.requires_new_schema_epoch == self.representation_change_allowed
            && self.requires_new_post_freeze_future == self.representation_change_allowed
            && self.adjudication_root_sha256 == self.expected_root()
    }
}

impl RepresentationGapAdjudicationReportV1 {
    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            REPRESENTATION_GAP_REPORT_SCHEMA_V1,
            self.evidence_epoch_sha256.as_str(),
            self.gap_denominator,
            &self.class_counts,
            &self.rows,
            false,
        ))
        .expect("representation gap report serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == REPRESENTATION_GAP_REPORT_SCHEMA_V1
            && valid_nonzero_sha256(&self.evidence_epoch_sha256)
            && !self.authority_ready
            && self.gap_denominator == self.rows.len()
            && self.class_counts.values().sum::<usize>() == self.gap_denominator
            && self
                .rows
                .iter()
                .all(RepresentationGapAdjudicationV1::validate)
            && self.report_root_sha256 == self.expected_root()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_kernel::{AtomValueType, ValueProjectionFormat};

    fn root(seed: &str) -> String {
        nando_operator_kernel::sha256_bytes(seed.as_bytes())
    }

    fn topology(values: &[&str]) -> PreActionMultiSourceTopologyV1 {
        PreActionMultiSourceTopologyV1 {
            extraction_status: MultiSourceExtractionStatusV1::Complete,
            grounded_output_count: 1,
            output_part_count: 1,
            roles: Vec::new(),
            role_witnesses: values
                .iter()
                .enumerate()
                .map(
                    |(index, value)| nando_operator_kernel::MultiSourceRoleWitnessV1 {
                        local_role_id: u16::try_from(index).expect("role id"),
                        value_sha256: root(value),
                        request_reference_ordinal: None,
                        request_reference_ordinal_candidates: Vec::new(),
                    },
                )
                .collect(),
            relations: Vec::new(),
        }
    }

    fn frame(atoms: Vec<RelationAtom>) -> RelationFrame {
        RelationFrame {
            schema: nando_operator_kernel::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: root("frame"),
            event_id_sha256: root("event"),
            client_intent_id_sha256: root("intent"),
            session_id_sha256: root("session"),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 1,
            extractor_version: "test".to_owned(),
            verifier_label: Some(true),
            atoms,
            evidence_ref_sha256: root("evidence"),
        }
    }

    fn selected(slot_id: u16, value: &str) -> Vec<RelationAtom> {
        vec![
            RelationAtom::TypedSlot {
                slot_id,
                value_type: AtomValueType::String,
                source: AtomSource::Observation,
                value_sha256: root(value),
            },
            RelationAtom::ObservationSelector {
                slot_id,
                selector: ResponseValueSelector::JsonScalarOrdinal {
                    ordinal: slot_id,
                    value_type: AtomValueType::String,
                },
            },
        ]
    }

    #[test]
    fn direct_pre_action_value_missing_from_topology_is_capture_gap_a() {
        let evidence = gap_evidence(&topology(&["kept"]), &frame(selected(1, "lost")));
        assert_eq!(
            classify_gap(&evidence, 1).0,
            RepresentationGapClassV1::CaptureGapA
        );
    }

    #[test]
    fn existing_multi_role_transform_requires_three_independent_lineages() {
        let mut atoms = selected(1, "first");
        atoms.extend(selected(2, "second"));
        atoms.push(RelationAtom::ActionValueProjection {
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::RenderSequence {
                segments: vec![
                    ResponseRenderSegment::Selected {
                        selector: ResponseValueSelector::JsonScalarOrdinal {
                            ordinal: 1,
                            value_type: AtomValueType::String,
                        },
                        format: ValueProjectionFormat::PlainText,
                    },
                    ResponseRenderSegment::Selected {
                        selector: ResponseValueSelector::JsonScalarOrdinal {
                            ordinal: 2,
                            value_type: AtomValueType::String,
                        },
                        format: ValueProjectionFormat::PlainText,
                    },
                ],
            },
        });
        let evidence = gap_evidence(&topology(&["first", "second"]), &frame(atoms));
        assert_eq!(
            classify_gap(&evidence, 2).0,
            RepresentationGapClassV1::InsufficientEvidence
        );
        assert_eq!(
            classify_gap(&evidence, 3).0,
            RepresentationGapClassV1::TransformGapB
        );
    }

    #[test]
    fn post_action_only_and_free_generation_never_change_representation() {
        let post_action = gap_evidence(
            &topology(&[]),
            &frame(vec![RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: AtomValueType::String,
                source: AtomSource::Action,
                value_sha256: root("late"),
            }]),
        );
        assert_eq!(
            classify_gap(&post_action, 0).0,
            RepresentationGapClassV1::PostActionOnlyC
        );

        let generated = gap_evidence(
            &topology(&[]),
            &frame(vec![RelationAtom::ActionStringArgument {
                name: "value".to_owned(),
                value: "teacher-only".to_owned(),
            }]),
        );
        assert_eq!(
            classify_gap(&generated, 0).0,
            RepresentationGapClassV1::FreeGenerationD
        );
    }
}
