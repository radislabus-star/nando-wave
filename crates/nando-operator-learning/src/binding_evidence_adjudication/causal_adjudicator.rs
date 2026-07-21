use std::collections::{BTreeMap, BTreeSet};

use crate::binding_evidence::BindingEvaluationLabelV1;
use crate::binding_evidence_preregistration::{
    BindingEvidencePartitionV1, binding_trust_roots_from_external_commitment_v1,
    resolve_trusted_binding_label_set_v1,
};

use super::canonical::{
    observed_relation_digest, sha256_json, validate_b1a_report, validate_preregistration,
};
use super::external_trust::validate_external_trust_inputs;
use super::label_manifest::build_binding_label_manifest_v1;
use super::physical_trial::observe_frozen_binding_labels_v1;
use super::report::adjudication_report_digest;
use super::wire::{
    BINDING_ADJUDICATION_REPORT_SCHEMA_V1, BINDING_RELATION_LAW_V1, BindingAdjudicationErrorV1,
    BindingCausalAdjudicationReportV1, BindingExternalLabelTrustReceiptV1,
    BindingHypothesisAdjudicationStatusV1, BindingInterventionAdjudicationV1,
    BindingObservedRelationV1, BindingPhysicalLabelReceiptSetV1, BindingPhysicalLabelReceiptV1,
    BindingPhysicalRelationStateV1, CONTROLLED_ROWS_PER_PARTITION_V1,
};

struct H1SelectionV1 {
    state: BindingPhysicalRelationStateV1,
    parent_ordinal: Option<usize>,
    candidate_ordinal: Option<usize>,
    action_equivalence_sha256: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn adjudicate_binding_hypotheses_v1(
    preregistration_bytes: &[u8],
    b1a_report_bytes: &[u8],
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
    physical_receipts_bytes: &[u8],
    label_manifest_bytes: &[u8],
    trust_receipt_bytes: &[u8],
) -> Result<BindingCausalAdjudicationReportV1, BindingAdjudicationErrorV1> {
    validate_preregistration(preregistration_bytes)?;
    let b1a = validate_b1a_report(b1a_report_bytes)?;
    let trust = BindingExternalLabelTrustReceiptV1::from_canonical_bytes(trust_receipt_bytes)?;
    validate_external_trust_inputs(
        &trust,
        preregistration_bytes,
        b1a_report_bytes,
        support_freeze_bytes,
        support_watermark_bytes,
        future_freeze_bytes,
        future_external_receipt_bytes,
        physical_receipts_bytes,
        label_manifest_bytes,
    )?;

    let physical = BindingPhysicalLabelReceiptSetV1::from_canonical_bytes(physical_receipts_bytes)?;
    let replayed = observe_frozen_binding_labels_v1(
        support_freeze_bytes,
        support_watermark_bytes,
        future_freeze_bytes,
        future_external_receipt_bytes,
    )?;
    if replayed != physical || replayed.canonical_bytes()? != physical_receipts_bytes {
        return Err(BindingAdjudicationErrorV1::FrozenReplayMismatch);
    }
    let rebuilt_manifest = build_binding_label_manifest_v1(
        support_freeze_bytes,
        support_watermark_bytes,
        future_freeze_bytes,
        future_external_receipt_bytes,
        &physical,
    )?;
    if rebuilt_manifest
        .canonical_bytes()
        .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)?
        != label_manifest_bytes
    {
        return Err(BindingAdjudicationErrorV1::InvalidLabelManifest);
    }

    let (manifest_root, watermark_root) = binding_trust_roots_from_external_commitment_v1(
        &trust.label_manifest_file_sha256,
        &trust.external_manifest_root_sha256,
        &trust.support_watermark_file_sha256,
    )
    .map_err(|_| BindingAdjudicationErrorV1::InvalidTrustReceipt)?;
    let trusted_labels = resolve_trusted_binding_label_set_v1(
        label_manifest_bytes,
        &manifest_root,
        support_watermark_bytes,
        &watermark_root,
    )
    .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)?;

    let mut wrong_bindings = 0_usize;
    let mut negative_accepts = 0_usize;
    let mut parity_failures = 0_usize;
    let mut by_intervention = BTreeMap::<String, Vec<&BindingPhysicalLabelReceiptV1>>::new();
    for receipt in &physical.receipts {
        let selection = select_h1_from_observed_relation(&receipt.observed_relation)?;
        parity_failures += receipt
            .trials
            .iter()
            .filter(|trial| !trial.verifier_agrees)
            .count();
        match receipt.label {
            BindingEvaluationLabelV1::Positive => {
                if selection.state != BindingPhysicalRelationStateV1::Unique
                    || selection.action_equivalence_sha256
                        != receipt.expected_action_equivalence_sha256
                    || selection.parent_ordinal.is_none()
                    || selection.candidate_ordinal.is_none()
                {
                    wrong_bindings += 1;
                }
            }
            BindingEvaluationLabelV1::ApplicabilityNegative => {
                if selection.state == BindingPhysicalRelationStateV1::Unique {
                    negative_accepts += 1;
                }
            }
        }
        by_intervention
            .entry(receipt.intervention_id.clone())
            .or_default()
            .push(receipt);
    }

    let mut interventions = Vec::new();
    for intervention_id in ["I1", "I2", "I3", "I4", "I5", "I6"] {
        let rows = by_intervention
            .get(intervention_id)
            .ok_or(BindingAdjudicationErrorV1::InvalidIntervention)?;
        interventions.push(adjudicate_intervention(intervention_id, rows)?);
    }
    let intervention_pass = interventions
        .iter()
        .all(|intervention| intervention.prediction_matched);
    let denominators_pass = trusted_labels.positive_rows() == 12
        && trusted_labels.applicability_negative_rows() == 12
        && physical.receipts.len() == CONTROLLED_ROWS_PER_PARTITION_V1 * 2;
    let h1_supported = denominators_pass
        && intervention_pass
        && wrong_bindings == 0
        && negative_accepts == 0
        && parity_failures == 0;

    let support_positive_rows = count_partition_labels(
        &physical.receipts,
        BindingEvidencePartitionV1::Support,
        BindingEvaluationLabelV1::Positive,
    );
    let support_negative_rows = count_partition_labels(
        &physical.receipts,
        BindingEvidencePartitionV1::Support,
        BindingEvaluationLabelV1::ApplicabilityNegative,
    );
    let future_positive_rows = count_partition_labels(
        &physical.receipts,
        BindingEvidencePartitionV1::Future,
        BindingEvaluationLabelV1::Positive,
    );
    let future_negative_rows = count_partition_labels(
        &physical.receipts,
        BindingEvidencePartitionV1::Future,
        BindingEvaluationLabelV1::ApplicabilityNegative,
    );
    let causal_relation_id_sha256 = sha256_json(&(
        "nando.binding-causal-relation.v1",
        BINDING_RELATION_LAW_V1,
        &interventions,
    ))?;
    let mut report = BindingCausalAdjudicationReportV1 {
        schema: BINDING_ADJUDICATION_REPORT_SCHEMA_V1.to_owned(),
        report_sha256: String::new(),
        stop_id: "STOP-B1B".to_owned(),
        trusted_label_manifest_sha256: trusted_labels.manifest_bytes_sha256().to_owned(),
        trusted_label_root_sha256: trusted_labels.external_manifest_root_sha256().to_owned(),
        physical_receipts_root_sha256: physical.receipt_sha256,
        support_rows: support_positive_rows + support_negative_rows,
        future_rows: future_positive_rows + future_negative_rows,
        support_positive_rows,
        support_applicability_negative_rows: support_negative_rows,
        future_positive_rows,
        future_applicability_negative_rows: future_negative_rows,
        b1a_ties_total: b1a.ties_total,
        b1a_ties_evaluated_against_relation: if h1_supported { b1a.ties_total } else { 0 },
        causal_relation: BINDING_RELATION_LAW_V1.to_owned(),
        causal_relation_id_sha256,
        h0_status: if h1_supported {
            BindingHypothesisAdjudicationStatusV1::Rejected
        } else {
            BindingHypothesisAdjudicationStatusV1::InsufficientEvidence
        },
        h1_status: if h1_supported {
            BindingHypothesisAdjudicationStatusV1::Supported
        } else {
            BindingHypothesisAdjudicationStatusV1::InsufficientEvidence
        },
        wrong_bindings,
        applicability_negative_accepts: negative_accepts,
        parity_failures,
        interventions,
        selector_compiled: false,
        protocol_mode_compiled: false,
        f4_status: if h1_supported {
            "UNLOCKED_NOT_STARTED".to_owned()
        } else {
            "BLOCKED_INSUFFICIENT_BINDING_EVIDENCE".to_owned()
        },
        execution_authority: false,
    };
    report.report_sha256 = adjudication_report_digest(&report)?;
    Ok(report)
}

fn select_h1_from_observed_relation(
    relation: &BindingObservedRelationV1,
) -> Result<H1SelectionV1, BindingAdjudicationErrorV1> {
    if observed_relation_digest(relation)? != relation.relation_root_sha256 {
        return Err(BindingAdjudicationErrorV1::InvalidRelation);
    }
    let requested = relation
        .parents
        .iter()
        .filter(|parent| {
            parent.active
                && relation
                    .requested_parent_instance_sha256
                    .contains(&parent.parent_instance_sha256)
        })
        .collect::<Vec<_>>();
    if relation.requested_parent_instance_sha256.len() > 1 && requested.len() > 1 {
        return Ok(H1SelectionV1 {
            state: BindingPhysicalRelationStateV1::Ambiguous,
            parent_ordinal: None,
            candidate_ordinal: None,
            action_equivalence_sha256: None,
        });
    }
    let Some(target) = relation.requested_capability_action_sha256.as_ref() else {
        return Ok(H1SelectionV1 {
            state: BindingPhysicalRelationStateV1::NotApplicable,
            parent_ordinal: None,
            candidate_ordinal: None,
            action_equivalence_sha256: None,
        });
    };
    let matching_parents = requested
        .into_iter()
        .filter(|parent| parent.capability_action_sha256 == *target)
        .collect::<Vec<_>>();
    let matching_candidates = relation
        .candidates
        .iter()
        .filter(|candidate| candidate.action_equivalence_sha256 == *target)
        .collect::<Vec<_>>();
    match (matching_parents.as_slice(), matching_candidates.as_slice()) {
        ([parent], [candidate]) => Ok(H1SelectionV1 {
            state: BindingPhysicalRelationStateV1::Unique,
            parent_ordinal: Some(parent.parent_ordinal),
            candidate_ordinal: Some(candidate.candidate_ordinal),
            action_equivalence_sha256: Some(target.clone()),
        }),
        _ => Ok(H1SelectionV1 {
            state: BindingPhysicalRelationStateV1::NotApplicable,
            parent_ordinal: None,
            candidate_ordinal: None,
            action_equivalence_sha256: None,
        }),
    }
}

fn adjudicate_intervention(
    intervention_id: &str,
    rows: &[&BindingPhysicalLabelReceiptV1],
) -> Result<BindingInterventionAdjudicationV1, BindingAdjudicationErrorV1> {
    if rows.len() != 4 {
        return Err(BindingAdjudicationErrorV1::InvalidDenominator);
    }
    let mut states = BTreeSet::new();
    let mut parent_ordinals = BTreeSet::new();
    let mut candidate_ordinals = BTreeSet::new();
    for row in rows {
        let selection = select_h1_from_observed_relation(&row.observed_relation)?;
        states.insert(selection.state);
        parent_ordinals.extend(selection.parent_ordinal);
        candidate_ordinals.extend(selection.candidate_ordinal);
    }
    let positive_rows = rows
        .iter()
        .filter(|row| row.label == BindingEvaluationLabelV1::Positive)
        .count();
    let negative_rows = rows.len() - positive_rows;
    let prediction_matched = match intervention_id {
        "I1" => {
            states == BTreeSet::from([BindingPhysicalRelationStateV1::Unique])
                && parent_ordinals == BTreeSet::from([0])
                && candidate_ordinals.len() >= 2
        }
        "I2" => {
            states == BTreeSet::from([BindingPhysicalRelationStateV1::Unique])
                && parent_ordinals == BTreeSet::from([0, 1])
        }
        "I3" => {
            states == BTreeSet::from([BindingPhysicalRelationStateV1::Unique])
                && parent_ordinals == BTreeSet::from([0])
                && rows
                    .iter()
                    .all(|row| row.observed_relation.candidates.len() >= 3)
        }
        "I4" | "I6" => {
            states == BTreeSet::from([BindingPhysicalRelationStateV1::NotApplicable])
                && positive_rows == 0
        }
        "I5" => {
            states == BTreeSet::from([BindingPhysicalRelationStateV1::Ambiguous])
                && positive_rows == 0
        }
        _ => return Err(BindingAdjudicationErrorV1::InvalidIntervention),
    };
    Ok(BindingInterventionAdjudicationV1 {
        intervention_id: intervention_id.to_owned(),
        support_rows: rows
            .iter()
            .filter(|row| row.partition == BindingEvidencePartitionV1::Support)
            .count(),
        future_rows: rows
            .iter()
            .filter(|row| row.partition == BindingEvidencePartitionV1::Future)
            .count(),
        positive_rows,
        applicability_negative_rows: negative_rows,
        observed_relation_states: states.into_iter().collect(),
        selected_parent_ordinals: parent_ordinals.into_iter().collect(),
        selected_candidate_ordinals: candidate_ordinals.into_iter().collect(),
        prediction_matched,
    })
}

pub(super) fn count_partition_labels(
    receipts: &[BindingPhysicalLabelReceiptV1],
    partition: BindingEvidencePartitionV1,
    label: BindingEvaluationLabelV1,
) -> usize {
    receipts
        .iter()
        .filter(|receipt| receipt.partition == partition && receipt.label == label)
        .count()
}
