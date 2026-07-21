use std::collections::BTreeMap;

use crate::EvidenceLedgerRecord;
use crate::binding_evidence::{
    BindingBaselineOutcomeV1, BindingEvaluationLabelV1, FrozenCandidateRelationGraphV1,
};
use crate::binding_evidence_preregistration::BindingEvidencePartitionV1;
use crate::capture_provenance::CaptureEvidenceReceipt;

use super::canonical::{
    action_digest, load_frozen_evidence, observed_relation_digest, physical_label_receipt_digest,
    physical_receipt_set_digest, sha256_bytes, sha256_json, validate_physical_label_receipt,
    validate_physical_receipt_set,
};
use super::controlled_replay::{
    PhysicalBindingScene, ReplayPartition, future_context, future_scene, render_future_scene,
    render_support_scene, replay_capture_record, support_context, support_scene,
    validate_replayed_row,
};
use super::wire::{
    BINDING_OBSERVED_RELATION_SCHEMA_V1, BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1,
    BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1, BINDING_TRIAL_PARITY_DOMAIN_V1,
    BINDING_TRIAL_VERIFIER_DOMAIN_V1, BindingAdjudicationErrorV1, BindingObservedCandidateV1,
    BindingObservedParentV1, BindingObservedRelationV1, BindingPhysicalActorOutcomeV1,
    BindingPhysicalCandidateTrialV1, BindingPhysicalLabelReceiptSetV1,
    BindingPhysicalLabelReceiptV1, CONTROLLED_ROWS_PER_PARTITION_V1,
};

struct PhysicalReceiptInput<'a> {
    graph: &'a FrozenCandidateRelationGraphV1,
    capture_receipt: &'a CaptureEvidenceReceipt,
    capture_record: &'a EvidenceLedgerRecord,
    pre_action_wire_root_sha256: &'a str,
    session_lineage_sha256: &'a str,
    partition: BindingEvidencePartitionV1,
    intervention_id: &'a str,
}

pub fn observe_frozen_binding_labels_v1(
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
) -> Result<BindingPhysicalLabelReceiptSetV1, BindingAdjudicationErrorV1> {
    let (support, future) = load_frozen_evidence(
        support_freeze_bytes,
        support_watermark_bytes,
        future_freeze_bytes,
        future_external_receipt_bytes,
    )?;
    let mut receipts = Vec::with_capacity(CONTROLLED_ROWS_PER_PARTITION_V1 * 2);

    let mut support_rows = support.support_label_rows().iter().collect::<Vec<_>>();
    support_rows.sort_by_key(|row| row.capture_record().sequence);
    let mut previous_record_sha256 = "0".repeat(64);
    for (row_index, row) in support_rows.into_iter().enumerate() {
        let intervention = row_index % 6 + 1;
        if row.intervention_id() != format!("I{intervention}") {
            return Err(BindingAdjudicationErrorV1::InvalidIntervention);
        }
        let replicate = row_index / 6;
        let scene = support_scene(intervention, replicate);
        let payload = render_support_scene(&scene, replicate);
        let context = support_context(&scene)?;
        let rebuilt_record = replay_capture_record(
            ReplayPartition::Support,
            row_index,
            row_index / 3,
            &payload,
            &previous_record_sha256,
            0,
        )?;
        previous_record_sha256 = rebuilt_record.record_sha256.clone();
        validate_replayed_row(
            row.frozen_graph(),
            row.capture_record(),
            &rebuilt_record,
            &payload,
            context,
        )?;
        receipts.push(physical_label_receipt(
            PhysicalReceiptInput {
                graph: row.frozen_graph(),
                capture_receipt: row.capture_receipt(),
                capture_record: row.capture_record(),
                pre_action_wire_root_sha256: row.pre_action_wire_root_sha256(),
                session_lineage_sha256: row.session_lineage_sha256(),
                partition: BindingEvidencePartitionV1::Support,
                intervention_id: row.intervention_id(),
            },
            &scene,
        )?);
    }

    let mut future_rows = future.future_label_rows().iter().collect::<Vec<_>>();
    future_rows.sort_by_key(|row| row.capture_record().sequence);
    previous_record_sha256 = support
        .capture_index()
        .records
        .last()
        .ok_or(BindingAdjudicationErrorV1::InvalidFrozenSupport)?
        .record_sha256
        .clone();
    let mut intervention_replicates = BTreeMap::<String, usize>::new();
    for (row_index, row) in future_rows.into_iter().enumerate() {
        let slot = future
            .protocol()
            .source
            .slots
            .iter()
            .find(|slot| slot.slot_id == row.slot_id())
            .ok_or(BindingAdjudicationErrorV1::InvalidIntervention)?;
        let intervention = slot
            .intervention_id
            .strip_prefix('I')
            .ok_or(BindingAdjudicationErrorV1::InvalidIntervention)?
            .parse::<usize>()
            .map_err(|_| BindingAdjudicationErrorV1::InvalidIntervention)?;
        let replicate = intervention_replicates
            .entry(slot.intervention_id.clone())
            .or_default();
        let scene = future_scene(intervention, *replicate);
        *replicate += 1;
        let payload = render_future_scene(&scene, row_index % 4);
        let context = future_context(&scene)?;
        let rebuilt_record = replay_capture_record(
            ReplayPartition::Future,
            row_index,
            row_index / 3,
            &payload,
            &previous_record_sha256,
            support.watermark_next_sequence(),
        )?;
        previous_record_sha256 = rebuilt_record.record_sha256.clone();
        validate_replayed_row(
            row.frozen_graph(),
            row.capture_record(),
            &rebuilt_record,
            &payload,
            context,
        )?;
        receipts.push(physical_label_receipt(
            PhysicalReceiptInput {
                graph: row.frozen_graph(),
                capture_receipt: row.capture_receipt(),
                capture_record: row.capture_record(),
                pre_action_wire_root_sha256: row.pre_action_wire_root_sha256(),
                session_lineage_sha256: row.session_lineage_sha256(),
                partition: BindingEvidencePartitionV1::Future,
                intervention_id: &slot.intervention_id,
            },
            &scene,
        )?);
    }

    receipts.sort_by(|left, right| left.row_id_sha256.cmp(&right.row_id_sha256));
    let mut set = BindingPhysicalLabelReceiptSetV1 {
        schema: BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        support_freeze_file_sha256: sha256_bytes(support_freeze_bytes),
        future_freeze_file_sha256: sha256_bytes(future_freeze_bytes),
        future_external_receipt_file_sha256: sha256_bytes(future_external_receipt_bytes),
        capture_index_sha256: future.capture_index().index_sha256.clone(),
        receipts,
        execution_authority: false,
    };
    set.receipt_sha256 = physical_receipt_set_digest(&set)?;
    validate_physical_receipt_set(&set)?;
    Ok(set)
}

fn physical_label_receipt(
    input: PhysicalReceiptInput<'_>,
    scene: &PhysicalBindingScene,
) -> Result<BindingPhysicalLabelReceiptV1, BindingAdjudicationErrorV1> {
    let observed_relation = observe_pre_action_relation(scene)?;
    let mut trials = Vec::with_capacity(scene.candidates.len());
    for (candidate_ordinal, candidate) in scene.candidates.iter().enumerate() {
        let (actor_outcome, applied_parent_ordinal) = execute_physical_candidate(scene, candidate);
        let verifier_agrees =
            verify_physical_candidate(scene, candidate, actor_outcome, applied_parent_ordinal);
        if !verifier_agrees {
            return Err(BindingAdjudicationErrorV1::ParityMismatch);
        }
        trials.push(BindingPhysicalCandidateTrialV1 {
            candidate_ordinal,
            action_equivalence_sha256: action_digest(candidate)?,
            actor_outcome,
            applied_parent_ordinal,
            verifier_agrees,
        });
    }
    let applied = trials
        .iter()
        .filter(|trial| trial.actor_outcome == BindingPhysicalActorOutcomeV1::Applied)
        .collect::<Vec<_>>();
    let (label, expected_action_equivalence_sha256) = match applied.as_slice() {
        [] => (BindingEvaluationLabelV1::ApplicabilityNegative, None),
        [trial] => (
            BindingEvaluationLabelV1::Positive,
            Some(trial.action_equivalence_sha256.clone()),
        ),
        _ => return Err(BindingAdjudicationErrorV1::InvalidRelation),
    };
    if label == BindingEvaluationLabelV1::Positive
        && !input.graph.graph.nodes.iter().any(|node| {
            Some(node.action_equivalence_sha256.as_str())
                == expected_action_equivalence_sha256.as_deref()
        })
    {
        return Err(BindingAdjudicationErrorV1::FrozenReplayMismatch);
    }
    let parity_receipt_root_sha256 = sha256_json(&(
        BINDING_TRIAL_PARITY_DOMAIN_V1,
        input.graph.graph.row_id_sha256.as_str(),
        &trials,
    ))?;
    let verifier_root_sha256 = sha256_json(&(
        BINDING_TRIAL_VERIFIER_DOMAIN_V1,
        input.graph.graph.row_id_sha256.as_str(),
        trials
            .iter()
            .map(|trial| {
                (
                    trial.action_equivalence_sha256.as_str(),
                    trial.actor_outcome,
                    trial.applied_parent_ordinal,
                    trial.verifier_agrees,
                )
            })
            .collect::<Vec<_>>(),
    ))?;
    let mut receipt = BindingPhysicalLabelReceiptV1 {
        schema: BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        row_id_sha256: input.graph.graph.row_id_sha256.clone(),
        evidence_ref_sha256: input.graph.graph.evidence_ref_sha256.clone(),
        frozen_graph_root_sha256: input.graph.graph_root_sha256.clone(),
        capture_receipt_root_sha256: input.capture_receipt.records_root_sha256.clone(),
        capture_sequence: input.capture_record.sequence,
        capture_record_sha256: input.capture_record.record_sha256.clone(),
        pre_action_wire_root_sha256: input.pre_action_wire_root_sha256.to_owned(),
        session_lineage_sha256: input.session_lineage_sha256.to_owned(),
        partition: input.partition,
        intervention_id: input.intervention_id.to_owned(),
        observed_relation,
        trials,
        parity_receipt_root_sha256,
        verifier_root_sha256,
        label,
        expected_action_equivalence_sha256,
        baseline_outcome: BindingBaselineOutcomeV1::Abstain,
    };
    receipt.receipt_sha256 = physical_label_receipt_digest(&receipt)?;
    validate_physical_label_receipt(&receipt)?;
    Ok(receipt)
}

fn observe_pre_action_relation(
    scene: &PhysicalBindingScene,
) -> Result<BindingObservedRelationV1, BindingAdjudicationErrorV1> {
    let parents = scene
        .parents
        .iter()
        .enumerate()
        .map(|(parent_ordinal, parent)| {
            Ok(BindingObservedParentV1 {
                parent_ordinal,
                parent_instance_sha256: action_digest(&parent.marker)?,
                capability_action_sha256: action_digest(&parent.capability)?,
                active: parent.active,
            })
        })
        .collect::<Result<Vec<_>, BindingAdjudicationErrorV1>>()?;
    let requested_parent_instance_sha256 = scene
        .requested_parents
        .iter()
        .map(|value| action_digest(value))
        .collect::<Result<Vec<_>, _>>()?;
    let requested_capability_action_sha256 = scene
        .requested_capability
        .as_ref()
        .map(|value| action_digest(value))
        .transpose()?;
    let candidates = scene
        .candidates
        .iter()
        .enumerate()
        .map(|(candidate_ordinal, candidate)| {
            Ok(BindingObservedCandidateV1 {
                candidate_ordinal,
                action_equivalence_sha256: action_digest(candidate)?,
            })
        })
        .collect::<Result<Vec<_>, BindingAdjudicationErrorV1>>()?;
    let mut relation = BindingObservedRelationV1 {
        schema: BINDING_OBSERVED_RELATION_SCHEMA_V1.to_owned(),
        relation_root_sha256: String::new(),
        parents,
        requested_parent_instance_sha256,
        requested_capability_action_sha256,
        candidates,
    };
    relation.relation_root_sha256 = observed_relation_digest(&relation)?;
    Ok(relation)
}

fn execute_physical_candidate(
    scene: &PhysicalBindingScene,
    candidate: &str,
) -> (BindingPhysicalActorOutcomeV1, Option<usize>) {
    if scene.requested_parents.len() != 1
        || scene.requested_capability.as_deref() != Some(candidate)
    {
        return (BindingPhysicalActorOutcomeV1::Abstained, None);
    }
    let requested_parent = &scene.requested_parents[0];
    let matching = scene
        .parents
        .iter()
        .enumerate()
        .filter(|(_, parent)| {
            parent.active && parent.marker == *requested_parent && parent.capability == candidate
        })
        .map(|(ordinal, _)| ordinal)
        .collect::<Vec<_>>();
    match matching.as_slice() {
        [parent_ordinal] => (
            BindingPhysicalActorOutcomeV1::Applied,
            Some(*parent_ordinal),
        ),
        _ => (BindingPhysicalActorOutcomeV1::Abstained, None),
    }
}

fn verify_physical_candidate(
    scene: &PhysicalBindingScene,
    candidate: &str,
    actor_outcome: BindingPhysicalActorOutcomeV1,
    applied_parent_ordinal: Option<usize>,
) -> bool {
    let candidate_advertised = scene.candidates.iter().any(|value| value == candidate);
    let requested_source = match scene.requested_parents.as_slice() {
        [source] => Some(source.as_str()),
        _ => None,
    };
    let valid_parent_ordinals = scene
        .parents
        .iter()
        .enumerate()
        .filter(|(_, parent)| {
            parent.active
                && Some(parent.marker.as_str()) == requested_source
                && parent.capability == candidate
        })
        .map(|(ordinal, _)| ordinal)
        .collect::<Vec<_>>();
    let should_apply = candidate_advertised
        && scene.requested_capability.as_deref() == Some(candidate)
        && valid_parent_ordinals.len() == 1;
    match (should_apply, actor_outcome, applied_parent_ordinal) {
        (true, BindingPhysicalActorOutcomeV1::Applied, Some(ordinal)) => {
            valid_parent_ordinals[0] == ordinal
        }
        (false, BindingPhysicalActorOutcomeV1::Abstained, None) => true,
        _ => false,
    }
}
