//! Closed B1B proof route: physical outcomes, external label trust, and H0/H1 adjudication.
//!
//! This module is proof/eval only. It can neither compile a protocol mode nor
//! grant execution authority. Labels are derived from candidate executions in
//! a reconstructed physical scene; the candidate relation is evaluated later
//! from a separate pre-action relation receipt.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::binding_evidence::{
    BindingBaselineOutcomeV1, BindingEvaluationLabelV1, BindingVersionSpaceReportV1,
    FrozenCandidateRelationGraphV1, PreActionBindingContextV1, PreActionBindingSurfaceV1,
};
use super::binding_evidence_capture_owner::BindingSupportFreezeV1;
use super::binding_evidence_future_capture::BindingFutureCaptureFreezeV1;
use super::binding_evidence_preregistration::{
    BINDING_LABEL_ENVELOPE_SCHEMA_V1, BindingCaptureReceiptEntryV1, BindingEvidencePartitionV1,
    BindingEvidencePreregistrationV1, BindingLabelObservationSourceV1,
    UntrustedBindingLabelEnvelopeV1, UntrustedBindingLabelManifestV1,
    binding_evidence_preregistration_v1, binding_trust_roots_from_external_commitment_v1,
    resolve_trusted_binding_label_set_v1,
};
use crate::capture_provenance::CaptureEvidenceReceipt;
use crate::{
    EVIDENCE_LEDGER_SCHEMA_V1, EvidenceIngestOutcome, EvidenceLedgerRecord, EvidencePolicyV1,
    RawEvidenceEnvelope, canonical_json_sha256, canonicalize_evidence_envelope,
};

pub const BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1: &str =
    "nando.binding-physical-label-receipt.v1";
pub const BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1: &str =
    "nando.binding-physical-label-receipt-set.v1";
pub const BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1: &str = "nando.binding-external-label-trust.v1";
pub const BINDING_ADJUDICATION_REPORT_SCHEMA_V1: &str = "nando.binding-causal-adjudication.v1";
const BINDING_OBSERVED_RELATION_SCHEMA_V1: &str = "nando.binding-observed-relation.v1";
const BINDING_TRIAL_PARITY_DOMAIN_V1: &str = "nando.binding-trial-parity.v1";
const BINDING_TRIAL_VERIFIER_DOMAIN_V1: &str = "nando.binding-trial-verifier.v1";
const BINDING_RELATION_LAW_V1: &str = "parent_action_to_capability_instance";
const REQUEST_CONTRACT_V1: &str = "continue active execution";
const CONTROLLED_ROWS_PER_PARTITION_V1: usize = 12;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingPhysicalRelationStateV1 {
    Unique,
    Ambiguous,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingPhysicalActorOutcomeV1 {
    Applied,
    Abstained,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingObservedParentV1 {
    pub parent_ordinal: usize,
    pub parent_instance_sha256: String,
    pub capability_action_sha256: String,
    pub active: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingObservedCandidateV1 {
    pub candidate_ordinal: usize,
    pub action_equivalence_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingObservedRelationV1 {
    pub schema: String,
    pub relation_root_sha256: String,
    pub parents: Vec<BindingObservedParentV1>,
    pub requested_parent_instance_sha256: Vec<String>,
    pub requested_capability_action_sha256: Option<String>,
    pub candidates: Vec<BindingObservedCandidateV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingPhysicalCandidateTrialV1 {
    pub candidate_ordinal: usize,
    pub action_equivalence_sha256: String,
    pub actor_outcome: BindingPhysicalActorOutcomeV1,
    pub applied_parent_ordinal: Option<usize>,
    pub verifier_agrees: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingPhysicalLabelReceiptV1 {
    pub schema: String,
    pub receipt_sha256: String,
    pub row_id_sha256: String,
    pub evidence_ref_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub capture_receipt_root_sha256: String,
    pub capture_sequence: u64,
    pub capture_record_sha256: String,
    pub pre_action_wire_root_sha256: String,
    pub session_lineage_sha256: String,
    pub partition: BindingEvidencePartitionV1,
    pub intervention_id: String,
    pub observed_relation: BindingObservedRelationV1,
    pub trials: Vec<BindingPhysicalCandidateTrialV1>,
    pub parity_receipt_root_sha256: String,
    pub verifier_root_sha256: String,
    pub label: BindingEvaluationLabelV1,
    pub expected_action_equivalence_sha256: Option<String>,
    pub baseline_outcome: BindingBaselineOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingPhysicalLabelReceiptSetV1 {
    pub schema: String,
    pub receipt_sha256: String,
    pub support_freeze_file_sha256: String,
    pub future_freeze_file_sha256: String,
    pub future_external_receipt_file_sha256: String,
    pub capture_index_sha256: String,
    pub receipts: Vec<BindingPhysicalLabelReceiptV1>,
    pub execution_authority: bool,
}

impl BindingPhysicalLabelReceiptSetV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingAdjudicationErrorV1> {
        serde_json::to_vec(self).map_err(|_| BindingAdjudicationErrorV1::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BindingAdjudicationErrorV1> {
        let set: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingAdjudicationErrorV1::InvalidPhysicalReceipt)?;
        if set.canonical_bytes()? != bytes {
            return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
        }
        validate_physical_receipt_set(&set)?;
        Ok(set)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingExternalLabelTrustReceiptV1 {
    pub schema: String,
    pub receipt_sha256: String,
    pub stop_id: String,
    pub owner_challenge_root_sha256: String,
    pub preregistration_file_sha256: String,
    pub b1a_report_file_sha256: String,
    pub support_freeze_file_sha256: String,
    pub support_watermark_file_sha256: String,
    pub future_freeze_file_sha256: String,
    pub future_external_receipt_file_sha256: String,
    pub physical_receipts_file_sha256: String,
    pub physical_receipts_root_sha256: String,
    pub label_manifest_file_sha256: String,
    pub external_manifest_root_sha256: String,
    pub expected_labels_joined: bool,
    pub protocol_mode_compiled: bool,
    pub execution_authority: bool,
}

impl BindingExternalLabelTrustReceiptV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingAdjudicationErrorV1> {
        pretty_json_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BindingAdjudicationErrorV1> {
        let receipt: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingAdjudicationErrorV1::InvalidTrustReceipt)?;
        if receipt.canonical_bytes()? != bytes
            || receipt.receipt_sha256 != external_trust_receipt_digest(&receipt)?
        {
            return Err(BindingAdjudicationErrorV1::InvalidTrustReceipt);
        }
        Ok(receipt)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingHypothesisAdjudicationStatusV1 {
    Supported,
    Rejected,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingInterventionAdjudicationV1 {
    pub intervention_id: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub positive_rows: usize,
    pub applicability_negative_rows: usize,
    pub observed_relation_states: Vec<BindingPhysicalRelationStateV1>,
    pub selected_parent_ordinals: Vec<usize>,
    pub selected_candidate_ordinals: Vec<usize>,
    pub prediction_matched: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingCausalAdjudicationReportV1 {
    pub schema: String,
    pub report_sha256: String,
    pub stop_id: String,
    pub trusted_label_manifest_sha256: String,
    pub trusted_label_root_sha256: String,
    pub physical_receipts_root_sha256: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub support_positive_rows: usize,
    pub support_applicability_negative_rows: usize,
    pub future_positive_rows: usize,
    pub future_applicability_negative_rows: usize,
    pub b1a_ties_total: usize,
    pub b1a_ties_evaluated_against_relation: usize,
    pub causal_relation: String,
    pub causal_relation_id_sha256: String,
    pub h0_status: BindingHypothesisAdjudicationStatusV1,
    pub h1_status: BindingHypothesisAdjudicationStatusV1,
    pub wrong_bindings: usize,
    pub applicability_negative_accepts: usize,
    pub parity_failures: usize,
    pub interventions: Vec<BindingInterventionAdjudicationV1>,
    pub selector_compiled: bool,
    pub protocol_mode_compiled: bool,
    pub f4_status: String,
    pub execution_authority: bool,
}

impl BindingCausalAdjudicationReportV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingAdjudicationErrorV1> {
        pretty_json_bytes(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingAdjudicationErrorV1 {
    InvalidFrozenSupport,
    InvalidFrozenFuture,
    InvalidExternalFutureReceipt,
    FrozenReplayMismatch,
    InvalidPhysicalReceipt,
    InvalidLabelManifest,
    InvalidTrustReceipt,
    InvalidPreregistration,
    InvalidB1aReport,
    InvalidDigest,
    InvalidDenominator,
    InvalidIntervention,
    InvalidRelation,
    ParityMismatch,
    Serialization,
}

#[derive(Clone)]
struct PhysicalParent {
    marker: String,
    capability: String,
    active: bool,
    rank: u64,
}

#[derive(Clone)]
struct PhysicalBindingScene {
    parents: Vec<PhysicalParent>,
    candidates: Vec<String>,
    requested_parents: Vec<String>,
    requested_capability: Option<String>,
}

struct PhysicalReceiptInput<'a> {
    graph: &'a FrozenCandidateRelationGraphV1,
    capture_receipt: &'a CaptureEvidenceReceipt,
    capture_record: &'a EvidenceLedgerRecord,
    pre_action_wire_root_sha256: &'a str,
    session_lineage_sha256: &'a str,
    partition: BindingEvidencePartitionV1,
    intervention_id: &'a str,
}

struct H1SelectionV1 {
    state: BindingPhysicalRelationStateV1,
    parent_ordinal: Option<usize>,
    candidate_ordinal: Option<usize>,
    action_equivalence_sha256: Option<String>,
}

#[derive(Clone, Copy)]
enum ReplayPartition {
    Support,
    Future,
}

#[derive(Deserialize)]
struct FutureExternalReceiptWireV1 {
    schema: String,
    future_freeze_file_sha256: String,
    trusted_future_receipt_sha256: String,
    expected_labels_joined: bool,
    execution_authority: bool,
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

pub fn build_binding_label_manifest_v1(
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
    physical: &BindingPhysicalLabelReceiptSetV1,
) -> Result<UntrustedBindingLabelManifestV1, BindingAdjudicationErrorV1> {
    validate_physical_receipt_set(physical)?;
    let (support, future) = load_frozen_evidence(
        support_freeze_bytes,
        support_watermark_bytes,
        future_freeze_bytes,
        future_external_receipt_bytes,
    )?;
    if physical.support_freeze_file_sha256 != sha256_bytes(support_freeze_bytes)
        || physical.future_freeze_file_sha256 != sha256_bytes(future_freeze_bytes)
        || physical.future_external_receipt_file_sha256
            != sha256_bytes(future_external_receipt_bytes)
        || physical.capture_index_sha256 != future.capture_index().index_sha256
    {
        return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
    }

    let receipt_by_row = physical
        .receipts
        .iter()
        .map(|receipt| (receipt.row_id_sha256.as_str(), receipt))
        .collect::<BTreeMap<_, _>>();
    let mut envelopes = Vec::with_capacity(physical.receipts.len());
    let mut capture_receipts = Vec::with_capacity(physical.receipts.len());
    for row in support.support_label_rows() {
        append_manifest_row(
            &mut envelopes,
            &mut capture_receipts,
            &receipt_by_row,
            row.frozen_graph(),
            row.capture_receipt(),
            row.capture_record(),
            row.pre_action_wire_root_sha256(),
            row.session_lineage_sha256(),
            BindingEvidencePartitionV1::Support,
            row.intervention_id(),
            &physical.receipt_sha256,
        )?;
    }
    for row in future.future_label_rows() {
        let intervention_id = future
            .protocol()
            .source
            .slots
            .iter()
            .find(|slot| slot.slot_id == row.slot_id())
            .ok_or(BindingAdjudicationErrorV1::InvalidIntervention)?
            .intervention_id
            .as_str();
        append_manifest_row(
            &mut envelopes,
            &mut capture_receipts,
            &receipt_by_row,
            row.frozen_graph(),
            row.capture_receipt(),
            row.capture_record(),
            row.pre_action_wire_root_sha256(),
            row.session_lineage_sha256(),
            BindingEvidencePartitionV1::Future,
            intervention_id,
            &physical.receipt_sha256,
        )?;
    }
    if envelopes.len() != physical.receipts.len() {
        return Err(BindingAdjudicationErrorV1::InvalidDenominator);
    }

    UntrustedBindingLabelManifestV1::new(
        physical.receipt_sha256.clone(),
        sha256_bytes(support_watermark_bytes),
        future.capture_index().clone(),
        capture_receipts,
        envelopes,
    )
    .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)
}

#[allow(clippy::too_many_arguments)]
pub fn seal_binding_external_label_trust_v1(
    preregistration_bytes: &[u8],
    b1a_report_bytes: &[u8],
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
    physical_receipts_bytes: &[u8],
    label_manifest_bytes: &[u8],
) -> Result<BindingExternalLabelTrustReceiptV1, BindingAdjudicationErrorV1> {
    validate_preregistration(preregistration_bytes)?;
    validate_b1a_report(b1a_report_bytes)?;
    let physical = BindingPhysicalLabelReceiptSetV1::from_canonical_bytes(physical_receipts_bytes)?;
    let manifest: UntrustedBindingLabelManifestV1 = serde_json::from_slice(label_manifest_bytes)
        .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)?;
    if manifest
        .canonical_bytes()
        .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)?
        != label_manifest_bytes
        || manifest.external_manifest_root_sha256 != physical.receipt_sha256
        || manifest.freeze_watermark_root_sha256 != sha256_bytes(support_watermark_bytes)
        || physical.support_freeze_file_sha256 != sha256_bytes(support_freeze_bytes)
        || physical.future_freeze_file_sha256 != sha256_bytes(future_freeze_bytes)
        || physical.future_external_receipt_file_sha256
            != sha256_bytes(future_external_receipt_bytes)
    {
        return Err(BindingAdjudicationErrorV1::InvalidTrustReceipt);
    }
    load_frozen_evidence(
        support_freeze_bytes,
        support_watermark_bytes,
        future_freeze_bytes,
        future_external_receipt_bytes,
    )?;

    let owner_challenge_root_sha256 = sha256_json(&(
        "nando.binding-label-owner-challenge.v1",
        sha256_bytes(preregistration_bytes),
        sha256_bytes(b1a_report_bytes),
        sha256_bytes(support_freeze_bytes),
        sha256_bytes(future_external_receipt_bytes),
    ))?;
    let mut receipt = BindingExternalLabelTrustReceiptV1 {
        schema: BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        stop_id: "STOP-B1B-LABEL-TRUST".to_owned(),
        owner_challenge_root_sha256,
        preregistration_file_sha256: sha256_bytes(preregistration_bytes),
        b1a_report_file_sha256: sha256_bytes(b1a_report_bytes),
        support_freeze_file_sha256: sha256_bytes(support_freeze_bytes),
        support_watermark_file_sha256: sha256_bytes(support_watermark_bytes),
        future_freeze_file_sha256: sha256_bytes(future_freeze_bytes),
        future_external_receipt_file_sha256: sha256_bytes(future_external_receipt_bytes),
        physical_receipts_file_sha256: sha256_bytes(physical_receipts_bytes),
        physical_receipts_root_sha256: physical.receipt_sha256.clone(),
        label_manifest_file_sha256: sha256_bytes(label_manifest_bytes),
        external_manifest_root_sha256: manifest.external_manifest_root_sha256,
        expected_labels_joined: true,
        protocol_mode_compiled: false,
        execution_authority: false,
    };
    receipt.receipt_sha256 = external_trust_receipt_digest(&receipt)?;
    Ok(receipt)
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

#[allow(clippy::too_many_arguments)]
fn append_manifest_row(
    envelopes: &mut Vec<UntrustedBindingLabelEnvelopeV1>,
    capture_receipts: &mut Vec<BindingCaptureReceiptEntryV1>,
    receipt_by_row: &BTreeMap<&str, &BindingPhysicalLabelReceiptV1>,
    graph: &FrozenCandidateRelationGraphV1,
    capture_receipt: &CaptureEvidenceReceipt,
    capture_record: &EvidenceLedgerRecord,
    pre_action_wire_root_sha256: &str,
    session_lineage_sha256: &str,
    partition: BindingEvidencePartitionV1,
    intervention_id: &str,
    external_manifest_root_sha256: &str,
) -> Result<(), BindingAdjudicationErrorV1> {
    let physical = receipt_by_row
        .get(graph.graph.row_id_sha256.as_str())
        .ok_or(BindingAdjudicationErrorV1::InvalidPhysicalReceipt)?;
    if physical.frozen_graph_root_sha256 != graph.graph_root_sha256
        || physical.capture_record_sha256 != capture_record.record_sha256
        || physical.capture_receipt_root_sha256 != capture_receipt.records_root_sha256
        || physical.pre_action_wire_root_sha256 != pre_action_wire_root_sha256
        || physical.session_lineage_sha256 != session_lineage_sha256
        || physical.partition != partition
        || physical.intervention_id != intervention_id
    {
        return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
    }
    let mut envelope = UntrustedBindingLabelEnvelopeV1 {
        schema: BINDING_LABEL_ENVELOPE_SCHEMA_V1.to_owned(),
        envelope_sha256: String::new(),
        row_id_sha256: graph.graph.row_id_sha256.clone(),
        evidence_ref_sha256: graph.graph.evidence_ref_sha256.clone(),
        frozen_graph_root_sha256: graph.graph_root_sha256.clone(),
        capture_receipt_root_sha256: capture_receipt.records_root_sha256.clone(),
        capture_sequence: capture_record.sequence,
        capture_record_sha256: capture_record.record_sha256.clone(),
        parity_receipt_root_sha256: physical.parity_receipt_root_sha256.clone(),
        verifier_root_sha256: physical.verifier_root_sha256.clone(),
        external_manifest_root_sha256: external_manifest_root_sha256.to_owned(),
        pre_action_wire_root_sha256: pre_action_wire_root_sha256.to_owned(),
        observed_relation_root_sha256: Some(
            physical.observed_relation.relation_root_sha256.clone(),
        ),
        observation_source: BindingLabelObservationSourceV1::PreActionWire,
        intervention_id: intervention_id.to_owned(),
        session_lineage_sha256: session_lineage_sha256.to_owned(),
        partition,
        captured_post_freeze: partition == BindingEvidencePartitionV1::Future,
        label: physical.label,
        expected_action_equivalence_sha256: physical.expected_action_equivalence_sha256.clone(),
        baseline_outcome: physical.baseline_outcome,
    };
    envelope
        .refresh_integrity_checksum()
        .map_err(|_| BindingAdjudicationErrorV1::InvalidLabelManifest)?;
    capture_receipts.push(BindingCaptureReceiptEntryV1 {
        evidence_ref_sha256: graph.graph.evidence_ref_sha256.clone(),
        receipt: capture_receipt.clone(),
    });
    envelopes.push(envelope);
    Ok(())
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

fn support_scene(intervention: usize, replicate: usize) -> PhysicalBindingScene {
    let left = format!("opaque-capability-{replicate}-left");
    let right = format!("opaque-capability-{replicate}-right");
    let parent_left = format!("opaque-parent-{replicate}-left");
    let parent_right = format!("opaque-parent-{replicate}-right");
    let mut parents = vec![PhysicalParent {
        marker: parent_left.clone(),
        capability: left.clone(),
        active: true,
        rank: 1,
    }];
    let mut candidates = vec![left.clone()];
    let (requested_parents, requested_capability) = match intervention {
        1 => {
            candidates.push(right.clone());
            if replicate == 1 {
                candidates.reverse();
            }
            (vec![parent_left], Some(left))
        }
        2 => {
            parents.push(PhysicalParent {
                marker: parent_right.clone(),
                capability: right.clone(),
                active: true,
                rank: 1,
            });
            candidates.push(right.clone());
            if replicate == 0 {
                (vec![parent_left], Some(left))
            } else {
                (vec![parent_right], Some(right))
            }
        }
        3 => {
            candidates.extend([right, format!("opaque-decoy-{replicate}")]);
            (vec![parent_left], Some(left))
        }
        4 => {
            parents[0].active = false;
            (vec![parent_left], Some(left))
        }
        5 => {
            parents.push(PhysicalParent {
                marker: parent_right.clone(),
                capability: right.clone(),
                active: true,
                rank: 1,
            });
            candidates.push(right);
            (vec![parent_left, parent_right], None)
        }
        6 => {
            candidates.push(right);
            (vec![format!("opaque-missing-parent-{replicate}")], None)
        }
        _ => unreachable!("bounded intervention"),
    };
    PhysicalBindingScene {
        parents,
        candidates,
        requested_parents,
        requested_capability,
    }
}

fn future_scene(intervention: usize, replicate: usize) -> PhysicalBindingScene {
    let base = format!("future-i{intervention}");
    let left = format!("{base}-capability-left");
    let right = format!("{base}-capability-right");
    let decoy = format!("{base}-capability-decoy");
    let parent_left = format!("{base}-parent-left");
    let parent_right = format!("{base}-parent-right");
    let mut parents = vec![PhysicalParent {
        marker: parent_left.clone(),
        capability: left.clone(),
        active: true,
        rank: 1,
    }];
    let mut candidates = vec![left.clone(), right.clone()];
    let (requested_parents, requested_capability) = match intervention {
        1 => {
            if replicate == 1 {
                candidates.reverse();
            }
            (vec![parent_left], Some(left))
        }
        2 => {
            parents.push(PhysicalParent {
                marker: parent_right.clone(),
                capability: right.clone(),
                active: true,
                rank: 1,
            });
            if replicate == 0 {
                (vec![parent_left], Some(left))
            } else {
                (vec![parent_right], Some(right))
            }
        }
        3 => {
            candidates.push(decoy);
            (vec![parent_left], Some(left))
        }
        4 => {
            parents[0].active = false;
            (vec![parent_left], Some(left))
        }
        5 => {
            parents.push(PhysicalParent {
                marker: parent_right.clone(),
                capability: right,
                active: true,
                rank: 1,
            });
            (vec![parent_left, parent_right], None)
        }
        6 => (vec![format!("{base}-parent-missing")], None),
        _ => unreachable!("bounded intervention"),
    };
    PhysicalBindingScene {
        parents,
        candidates,
        requested_parents,
        requested_capability,
    }
}

fn render_support_scene(scene: &PhysicalBindingScene, replicate: usize) -> Value {
    let parents = scene
        .parents
        .iter()
        .map(|parent| {
            json!({
                "anchor": parent.marker,
                "capability": parent.capability,
                "state": if parent.active { "active" } else { "completed" },
                "distance": parent.rank,
            })
        })
        .collect::<Vec<_>>();
    let relation_source = if scene.requested_parents.len() == 1 {
        json!(scene.requested_parents[0])
    } else {
        json!(scene.requested_parents)
    };
    let relation = json!({
        "source": relation_source,
        "capability": scene.requested_capability,
    });
    if replicate == 0 {
        json!({
            "history": parents,
            "available": scene.candidates,
            "request_relation": relation,
        })
    } else {
        json!({
            "transport": {
                "items": parents,
                "choices": scene.candidates,
                "relation": relation,
            }
        })
    }
}

fn render_future_scene(scene: &PhysicalBindingScene, shape: usize) -> Value {
    let relation_source = if scene.requested_parents.len() == 1 {
        json!(scene.requested_parents[0])
    } else {
        json!(scene.requested_parents)
    };
    let relation_target = json!(scene.requested_capability);
    match shape {
        0 => json!({
            "future_alpha_timeline": render_future_parents(&scene.parents, "alpha"),
            "future_alpha_options": scene.candidates,
            "future_alpha_binding": {
                "future_alpha_origin": relation_source,
                "future_alpha_target": relation_target,
            }
        }),
        1 => json!({
            "future_beta_packet": {
                "future_beta_records": render_future_parents(&scene.parents, "beta"),
                "future_beta_choices": scene.candidates,
                "future_beta_link": {
                    "future_beta_from": relation_source,
                    "future_beta_to": relation_target,
                }
            }
        }),
        2 => json!([{
            "future_gamma_records": render_future_parents(&scene.parents, "gamma"),
            "future_gamma_choices": scene.candidates,
            "future_gamma_link": [relation_source, relation_target],
        }]),
        _ => json!({
            "future_delta_state": {
                "future_delta_records": render_future_parents(&scene.parents, "delta"),
            },
            "future_delta_selection": {
                "future_delta_choices": scene.candidates,
            },
            "future_delta_relation": {
                "future_delta_from": relation_source,
                "future_delta_to": relation_target,
            }
        }),
    }
}

fn render_future_parents(parents: &[PhysicalParent], prefix: &str) -> Vec<Value> {
    parents
        .iter()
        .map(|parent| {
            json!({
                format!("future_{prefix}_marker"): parent.marker,
                format!("future_{prefix}_endpoint"): parent.capability,
                format!("future_{prefix}_phase"): if parent.active { "active" } else { "completed" },
                format!("future_{prefix}_rank"): parent.rank,
            })
        })
        .collect()
}

fn support_context(
    scene: &PhysicalBindingScene,
) -> Result<PreActionBindingContextV1, BindingAdjudicationErrorV1> {
    let active = scene.parents.iter().filter(|parent| parent.active).count();
    let completed = scene.parents.len().saturating_sub(active);
    Ok(PreActionBindingContextV1 {
        call_shape_count: checked_u16(scene.parents.len())?,
        capability_count: checked_u16(scene.candidates.len())?,
        completion_state: if active > 0 {
            super::binding_evidence::BindingCompletionStateV1::Unresolved
        } else if completed > 0 {
            super::binding_evidence::BindingCompletionStateV1::Completed
        } else {
            super::binding_evidence::BindingCompletionStateV1::Unknown
        },
        temporal_relation_count: checked_u16(scene.parents.len())?,
        cardinality_relation_count: 1,
        topology_neighborhood_root_sha256: canonical_json_sha256(&json!({
            "parents": scene.parents.len(),
            "active": active,
            "completed": completed,
            "candidates": scene.candidates.len(),
            "relation_present": true,
        }))
        .map_err(|_| BindingAdjudicationErrorV1::Serialization)?,
    })
}

fn future_context(
    scene: &PhysicalBindingScene,
) -> Result<PreActionBindingContextV1, BindingAdjudicationErrorV1> {
    let active = scene.parents.iter().filter(|parent| parent.active).count();
    let completed = scene.parents.len().saturating_sub(active);
    Ok(PreActionBindingContextV1 {
        call_shape_count: checked_u16(scene.parents.len())?,
        capability_count: checked_u16(scene.candidates.len())?,
        completion_state: if active > 0 {
            super::binding_evidence::BindingCompletionStateV1::Unresolved
        } else {
            super::binding_evidence::BindingCompletionStateV1::Completed
        },
        temporal_relation_count: checked_u16(scene.parents.len())?,
        cardinality_relation_count: if scene.requested_parents.len() > 1 {
            checked_u16(scene.requested_parents.len())?
        } else {
            1
        },
        topology_neighborhood_root_sha256: canonical_json_sha256(&json!({
            "parents": scene.parents.len(),
            "active": active,
            "completed": completed,
            "candidates": scene.candidates.len(),
            "relation_sources": scene.requested_parents.len(),
            "relation_target_present": scene.requested_capability.is_some(),
        }))
        .map_err(|_| BindingAdjudicationErrorV1::Serialization)?,
    })
}

fn replay_capture_record(
    partition: ReplayPartition,
    row_index: usize,
    session: usize,
    payload: &Value,
    previous_record_sha256: &str,
    first_sequence: u64,
) -> Result<EvidenceLedgerRecord, BindingAdjudicationErrorV1> {
    let (source_stream_id, event_id, session_id, intent_id, call_id, event_time) = match partition {
        ReplayPartition::Support => (
            "nando-b1b-support-acquisition-v1".to_owned(),
            format!("b1b-support-event-{row_index}"),
            format!("b1b-support-session-{session}"),
            format!("b1b-support-intent-{row_index}"),
            format!("b1b-support-call-{row_index}"),
            10_000_000 + row_index as u64,
        ),
        ReplayPartition::Future => (
            "nando-b1b-future-acquisition-v1".to_owned(),
            format!("b1b-future-event-{row_index}"),
            format!("b1b-future-session-S{session}"),
            format!("b1b-future-intent-{row_index}"),
            format!("b1b-future-call-{row_index}"),
            20_000_000 + row_index as u64,
        ),
    };
    let envelope = RawEvidenceEnvelope {
        source_stream_id,
        source_offset: row_index as u64,
        event_id,
        session_id,
        client_intent_id: Some(intent_id),
        call_id: Some(call_id),
        output_ordinal: Some(row_index as u32),
        event_time_unix_nanos: Some(event_time),
        schema_version: 1,
        payload: serde_json::to_vec(payload)
            .map_err(|_| BindingAdjudicationErrorV1::Serialization)?,
    };
    let outcome = EvidenceIngestOutcome::Normalized {
        graph: canonicalize_evidence_envelope(&envelope, EvidencePolicyV1::streaming_bounded())
            .map_err(|_| BindingAdjudicationErrorV1::FrozenReplayMismatch)?,
    };
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        sequence: u64,
        previous_record_sha256: &'a str,
        outcome: &'a EvidenceIngestOutcome,
    }
    let sequence = first_sequence + row_index as u64;
    let record_sha256 = canonical_json_sha256(&DigestFields {
        schema: EVIDENCE_LEDGER_SCHEMA_V1,
        sequence,
        previous_record_sha256,
        outcome: &outcome,
    })
    .map_err(|_| BindingAdjudicationErrorV1::Serialization)?;
    Ok(EvidenceLedgerRecord {
        schema: EVIDENCE_LEDGER_SCHEMA_V1.to_owned(),
        sequence,
        previous_record_sha256: previous_record_sha256.to_owned(),
        outcome,
        record_sha256,
    })
}

fn validate_replayed_row(
    frozen_graph: &FrozenCandidateRelationGraphV1,
    frozen_record: &EvidenceLedgerRecord,
    replayed_record: &EvidenceLedgerRecord,
    payload: &Value,
    context: PreActionBindingContextV1,
) -> Result<(), BindingAdjudicationErrorV1> {
    if frozen_record != replayed_record {
        return Err(BindingAdjudicationErrorV1::FrozenReplayMismatch);
    }
    let replayed_graph = PreActionBindingSurfaceV1::capture(
        frozen_graph.graph.row_id_sha256.clone(),
        frozen_graph.graph.evidence_ref_sha256.clone(),
        REQUEST_CONTRACT_V1,
        payload,
        context,
        Default::default(),
    )
    .map_err(|_| BindingAdjudicationErrorV1::FrozenReplayMismatch)?
    .candidate_relation_graph(Default::default())
    .map_err(|_| BindingAdjudicationErrorV1::FrozenReplayMismatch)?
    .freeze()
    .map_err(|_| BindingAdjudicationErrorV1::FrozenReplayMismatch)?;
    if replayed_graph != *frozen_graph {
        return Err(BindingAdjudicationErrorV1::FrozenReplayMismatch);
    }
    Ok(())
}

fn load_frozen_evidence(
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
) -> Result<(BindingSupportFreezeV1, BindingFutureCaptureFreezeV1), BindingAdjudicationErrorV1> {
    let support = BindingSupportFreezeV1::from_canonical_bytes(support_freeze_bytes)
        .map_err(|_| BindingAdjudicationErrorV1::InvalidFrozenSupport)?;
    if support
        .watermark_canonical_bytes()
        .map_err(|_| BindingAdjudicationErrorV1::InvalidFrozenSupport)?
        != support_watermark_bytes
    {
        return Err(BindingAdjudicationErrorV1::InvalidFrozenSupport);
    }
    let external: FutureExternalReceiptWireV1 =
        serde_json::from_slice(future_external_receipt_bytes)
            .map_err(|_| BindingAdjudicationErrorV1::InvalidExternalFutureReceipt)?;
    if external.schema != "nando.binding-future-external-receipt.v1"
        || external.future_freeze_file_sha256 != sha256_bytes(future_freeze_bytes)
        || external.expected_labels_joined
        || external.execution_authority
        || !is_sha256(&external.trusted_future_receipt_sha256)
    {
        return Err(BindingAdjudicationErrorV1::InvalidExternalFutureReceipt);
    }
    let future = BindingFutureCaptureFreezeV1::from_canonical_bytes(
        future_freeze_bytes,
        &external.trusted_future_receipt_sha256,
        support_freeze_bytes,
        support_watermark_bytes,
    )
    .map_err(|_| BindingAdjudicationErrorV1::InvalidFrozenFuture)?;
    Ok((support, future))
}

fn validate_physical_receipt_set(
    set: &BindingPhysicalLabelReceiptSetV1,
) -> Result<(), BindingAdjudicationErrorV1> {
    if set.schema != BINDING_PHYSICAL_LABEL_SET_SCHEMA_V1
        || set.execution_authority
        || set.receipts.len() != CONTROLLED_ROWS_PER_PARTITION_V1 * 2
        || !is_sha256(&set.support_freeze_file_sha256)
        || !is_sha256(&set.future_freeze_file_sha256)
        || !is_sha256(&set.future_external_receipt_file_sha256)
        || !is_sha256(&set.capture_index_sha256)
        || set
            .receipts
            .windows(2)
            .any(|pair| pair[0].row_id_sha256 >= pair[1].row_id_sha256)
        || physical_receipt_set_digest(set)? != set.receipt_sha256
    {
        return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
    }
    let mut evidence_refs = BTreeSet::new();
    for receipt in &set.receipts {
        validate_physical_label_receipt(receipt)?;
        if !evidence_refs.insert(receipt.evidence_ref_sha256.as_str()) {
            return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
        }
    }
    Ok(())
}

fn validate_physical_label_receipt(
    receipt: &BindingPhysicalLabelReceiptV1,
) -> Result<(), BindingAdjudicationErrorV1> {
    let digests = [
        receipt.receipt_sha256.as_str(),
        receipt.row_id_sha256.as_str(),
        receipt.evidence_ref_sha256.as_str(),
        receipt.frozen_graph_root_sha256.as_str(),
        receipt.capture_receipt_root_sha256.as_str(),
        receipt.capture_record_sha256.as_str(),
        receipt.pre_action_wire_root_sha256.as_str(),
        receipt.session_lineage_sha256.as_str(),
        receipt.observed_relation.relation_root_sha256.as_str(),
        receipt.parity_receipt_root_sha256.as_str(),
        receipt.verifier_root_sha256.as_str(),
    ];
    if receipt.schema != BINDING_PHYSICAL_LABEL_RECEIPT_SCHEMA_V1
        || digests.into_iter().any(|digest| !is_sha256(digest))
        || !matches!(
            receipt.intervention_id.as_str(),
            "I1" | "I2" | "I3" | "I4" | "I5" | "I6"
        )
        || receipt.trials.is_empty()
        || receipt
            .trials
            .iter()
            .any(|trial| !is_sha256(&trial.action_equivalence_sha256) || !trial.verifier_agrees)
        || receipt.parity_receipt_root_sha256
            != sha256_json(&(
                BINDING_TRIAL_PARITY_DOMAIN_V1,
                receipt.row_id_sha256.as_str(),
                &receipt.trials,
            ))?
        || receipt.verifier_root_sha256
            != sha256_json(&(
                BINDING_TRIAL_VERIFIER_DOMAIN_V1,
                receipt.row_id_sha256.as_str(),
                receipt
                    .trials
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
            ))?
        || physical_label_receipt_digest(receipt)? != receipt.receipt_sha256
        || observed_relation_digest(&receipt.observed_relation)?
            != receipt.observed_relation.relation_root_sha256
    {
        return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt);
    }
    let applied = receipt
        .trials
        .iter()
        .filter(|trial| trial.actor_outcome == BindingPhysicalActorOutcomeV1::Applied)
        .collect::<Vec<_>>();
    match (receipt.label, applied.as_slice()) {
        (BindingEvaluationLabelV1::Positive, [trial])
            if receipt.expected_action_equivalence_sha256.as_deref()
                == Some(trial.action_equivalence_sha256.as_str()) => {}
        (BindingEvaluationLabelV1::ApplicabilityNegative, [])
            if receipt.expected_action_equivalence_sha256.is_none() => {}
        _ => return Err(BindingAdjudicationErrorV1::InvalidPhysicalReceipt),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_external_trust_inputs(
    trust: &BindingExternalLabelTrustReceiptV1,
    preregistration_bytes: &[u8],
    b1a_report_bytes: &[u8],
    support_freeze_bytes: &[u8],
    support_watermark_bytes: &[u8],
    future_freeze_bytes: &[u8],
    future_external_receipt_bytes: &[u8],
    physical_receipts_bytes: &[u8],
    label_manifest_bytes: &[u8],
) -> Result<(), BindingAdjudicationErrorV1> {
    let owner_challenge_root_sha256 = sha256_json(&(
        "nando.binding-label-owner-challenge.v1",
        sha256_bytes(preregistration_bytes),
        sha256_bytes(b1a_report_bytes),
        sha256_bytes(support_freeze_bytes),
        sha256_bytes(future_external_receipt_bytes),
    ))?;
    if trust.schema != BINDING_EXTERNAL_LABEL_TRUST_SCHEMA_V1
        || trust.stop_id != "STOP-B1B-LABEL-TRUST"
        || trust.owner_challenge_root_sha256 != owner_challenge_root_sha256
        || trust.preregistration_file_sha256 != sha256_bytes(preregistration_bytes)
        || trust.b1a_report_file_sha256 != sha256_bytes(b1a_report_bytes)
        || trust.support_freeze_file_sha256 != sha256_bytes(support_freeze_bytes)
        || trust.support_watermark_file_sha256 != sha256_bytes(support_watermark_bytes)
        || trust.future_freeze_file_sha256 != sha256_bytes(future_freeze_bytes)
        || trust.future_external_receipt_file_sha256 != sha256_bytes(future_external_receipt_bytes)
        || trust.physical_receipts_file_sha256 != sha256_bytes(physical_receipts_bytes)
        || trust.label_manifest_file_sha256 != sha256_bytes(label_manifest_bytes)
        || trust.physical_receipts_root_sha256 != trust.external_manifest_root_sha256
        || !trust.expected_labels_joined
        || trust.protocol_mode_compiled
        || trust.execution_authority
    {
        return Err(BindingAdjudicationErrorV1::InvalidTrustReceipt);
    }
    Ok(())
}

fn validate_preregistration(
    bytes: &[u8],
) -> Result<BindingEvidencePreregistrationV1, BindingAdjudicationErrorV1> {
    let report: BindingEvidencePreregistrationV1 = serde_json::from_slice(bytes)
        .map_err(|_| BindingAdjudicationErrorV1::InvalidPreregistration)?;
    if report != binding_evidence_preregistration_v1()
        || report.stop_id != "STOP-B1B0R"
        || report.acquisition_run
        || report.protocol_mode_compiled
        || report.f4_started
        || report.execution_authority
    {
        return Err(BindingAdjudicationErrorV1::InvalidPreregistration);
    }
    Ok(report)
}

fn validate_b1a_report(
    bytes: &[u8],
) -> Result<BindingVersionSpaceReportV1, BindingAdjudicationErrorV1> {
    let report: BindingVersionSpaceReportV1 =
        serde_json::from_slice(bytes).map_err(|_| BindingAdjudicationErrorV1::InvalidB1aReport)?;
    if report.ties_total == 0
        || report.tie_budget_exhausted
        || report.ties_total != report.ties.len()
        || report.distinguishing_probes.len() != report.ties.len()
        || report.protocol_mode_compiled
        || report.execution_authority
        || report.distinguishing_probes.iter().any(|probe| {
            probe.required_distinction != "expected_action_class_vs_competing_action_classes"
        })
    {
        return Err(BindingAdjudicationErrorV1::InvalidB1aReport);
    }
    Ok(report)
}

fn count_partition_labels(
    receipts: &[BindingPhysicalLabelReceiptV1],
    partition: BindingEvidencePartitionV1,
    label: BindingEvaluationLabelV1,
) -> usize {
    receipts
        .iter()
        .filter(|receipt| receipt.partition == partition && receipt.label == label)
        .count()
}

fn checked_u16(value: usize) -> Result<u16, BindingAdjudicationErrorV1> {
    u16::try_from(value).map_err(|_| BindingAdjudicationErrorV1::InvalidDenominator)
}

fn action_digest(value: &str) -> Result<String, BindingAdjudicationErrorV1> {
    canonical_json_sha256(&value).map_err(|_| BindingAdjudicationErrorV1::Serialization)
}

fn observed_relation_digest(
    relation: &BindingObservedRelationV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    sha256_json(&(
        relation.schema.as_str(),
        &relation.parents,
        &relation.requested_parent_instance_sha256,
        &relation.requested_capability_action_sha256,
        &relation.candidates,
    ))
}

fn physical_label_receipt_digest(
    receipt: &BindingPhysicalLabelReceiptV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        row_id_sha256: &'a str,
        evidence_ref_sha256: &'a str,
        frozen_graph_root_sha256: &'a str,
        capture_receipt_root_sha256: &'a str,
        capture_sequence: u64,
        capture_record_sha256: &'a str,
        pre_action_wire_root_sha256: &'a str,
        session_lineage_sha256: &'a str,
        partition: BindingEvidencePartitionV1,
        intervention_id: &'a str,
        observed_relation: &'a BindingObservedRelationV1,
        trials: &'a [BindingPhysicalCandidateTrialV1],
        parity_receipt_root_sha256: &'a str,
        verifier_root_sha256: &'a str,
        label: BindingEvaluationLabelV1,
        expected_action_equivalence_sha256: &'a Option<String>,
        baseline_outcome: BindingBaselineOutcomeV1,
    }
    sha256_json(&DigestFields {
        schema: &receipt.schema,
        row_id_sha256: &receipt.row_id_sha256,
        evidence_ref_sha256: &receipt.evidence_ref_sha256,
        frozen_graph_root_sha256: &receipt.frozen_graph_root_sha256,
        capture_receipt_root_sha256: &receipt.capture_receipt_root_sha256,
        capture_sequence: receipt.capture_sequence,
        capture_record_sha256: &receipt.capture_record_sha256,
        pre_action_wire_root_sha256: &receipt.pre_action_wire_root_sha256,
        session_lineage_sha256: &receipt.session_lineage_sha256,
        partition: receipt.partition,
        intervention_id: &receipt.intervention_id,
        observed_relation: &receipt.observed_relation,
        trials: &receipt.trials,
        parity_receipt_root_sha256: &receipt.parity_receipt_root_sha256,
        verifier_root_sha256: &receipt.verifier_root_sha256,
        label: receipt.label,
        expected_action_equivalence_sha256: &receipt.expected_action_equivalence_sha256,
        baseline_outcome: receipt.baseline_outcome,
    })
}

fn physical_receipt_set_digest(
    set: &BindingPhysicalLabelReceiptSetV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    sha256_json(&(
        set.schema.as_str(),
        set.support_freeze_file_sha256.as_str(),
        set.future_freeze_file_sha256.as_str(),
        set.future_external_receipt_file_sha256.as_str(),
        set.capture_index_sha256.as_str(),
        &set.receipts,
        set.execution_authority,
    ))
}

fn external_trust_receipt_digest(
    receipt: &BindingExternalLabelTrustReceiptV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    sha256_json(&(
        receipt.schema.as_str(),
        receipt.stop_id.as_str(),
        receipt.owner_challenge_root_sha256.as_str(),
        receipt.preregistration_file_sha256.as_str(),
        receipt.b1a_report_file_sha256.as_str(),
        receipt.support_freeze_file_sha256.as_str(),
        receipt.support_watermark_file_sha256.as_str(),
        receipt.future_freeze_file_sha256.as_str(),
        receipt.future_external_receipt_file_sha256.as_str(),
        receipt.physical_receipts_file_sha256.as_str(),
        receipt.physical_receipts_root_sha256.as_str(),
        receipt.label_manifest_file_sha256.as_str(),
        receipt.external_manifest_root_sha256.as_str(),
        receipt.expected_labels_joined,
        receipt.protocol_mode_compiled,
        receipt.execution_authority,
    ))
}

fn adjudication_report_digest(
    report: &BindingCausalAdjudicationReportV1,
) -> Result<String, BindingAdjudicationErrorV1> {
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        stop_id: &'a str,
        trusted_label_manifest_sha256: &'a str,
        trusted_label_root_sha256: &'a str,
        physical_receipts_root_sha256: &'a str,
        support_rows: usize,
        future_rows: usize,
        support_positive_rows: usize,
        support_applicability_negative_rows: usize,
        future_positive_rows: usize,
        future_applicability_negative_rows: usize,
        b1a_ties_total: usize,
        b1a_ties_evaluated_against_relation: usize,
        causal_relation: &'a str,
        causal_relation_id_sha256: &'a str,
        h0_status: BindingHypothesisAdjudicationStatusV1,
        h1_status: BindingHypothesisAdjudicationStatusV1,
        wrong_bindings: usize,
        applicability_negative_accepts: usize,
        parity_failures: usize,
        interventions: &'a [BindingInterventionAdjudicationV1],
        selector_compiled: bool,
        protocol_mode_compiled: bool,
        f4_status: &'a str,
        execution_authority: bool,
    }
    sha256_json(&DigestFields {
        schema: &report.schema,
        stop_id: &report.stop_id,
        trusted_label_manifest_sha256: &report.trusted_label_manifest_sha256,
        trusted_label_root_sha256: &report.trusted_label_root_sha256,
        physical_receipts_root_sha256: &report.physical_receipts_root_sha256,
        support_rows: report.support_rows,
        future_rows: report.future_rows,
        support_positive_rows: report.support_positive_rows,
        support_applicability_negative_rows: report.support_applicability_negative_rows,
        future_positive_rows: report.future_positive_rows,
        future_applicability_negative_rows: report.future_applicability_negative_rows,
        b1a_ties_total: report.b1a_ties_total,
        b1a_ties_evaluated_against_relation: report.b1a_ties_evaluated_against_relation,
        causal_relation: &report.causal_relation,
        causal_relation_id_sha256: &report.causal_relation_id_sha256,
        h0_status: report.h0_status,
        h1_status: report.h1_status,
        wrong_bindings: report.wrong_bindings,
        applicability_negative_accepts: report.applicability_negative_accepts,
        parity_failures: report.parity_failures,
        interventions: &report.interventions,
        selector_compiled: report.selector_compiled,
        protocol_mode_compiled: report.protocol_mode_compiled,
        f4_status: &report.f4_status,
        execution_authority: report.execution_authority,
    })
}

fn pretty_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, BindingAdjudicationErrorV1> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|_| BindingAdjudicationErrorV1::Serialization)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn sha256_json<T: Serialize>(value: &T) -> Result<String, BindingAdjudicationErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|_| BindingAdjudicationErrorV1::Serialization)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
#[path = "binding_evidence_adjudication_tests.rs"]
mod tests;
