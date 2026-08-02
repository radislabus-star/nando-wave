use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::multi_source::{
    K1ConsequenceTypeV1, K1DeficitSnapshotV1, K1FutureOutcomeReceiptV1,
    K1FuturePredictionContractV1, K1FuturePredictionReceiptV1, K1GenerationTerminalVerdictV1,
    K1IdentificationFreezeV1, K1NaturalCandidateFreezeV1, K1NaturalCandidateQueueV1,
    K1NaturalCohortCandidateV1, K1NaturalCohortCatalogV1, K1ProbeBudgetRemainingV1,
    K1ProbeRoundReceiptV1, K1ProbeRoundStateV1, K1SchedulerEventPayloadV1, K1SchedulerEventV1,
    K1SchedulerLedgerV1, K1TransferSettlementV1, NaturalT1ProgramArtifactV1,
    PreActionTopologyAuditRowV1,
};
use nando_response_actor::{
    CollectionOutputRenderer, ResponseOperation, ResponseProgram, ResponseRegistry,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::operator_certification::{
    CertificationAuthorityConfigV1, read_verifying_key, restore_anchored_ledger,
};
use crate::write_bytes_atomic;

mod authority;
pub(crate) mod duplicate_cohorts;
mod fork;
mod future_authority;
mod journal;
mod projection;
mod selection_authority;

use authority::send_authority_request;
pub(crate) use authority::{handle_authority_line, recover_authority};
use journal::restore_anchored_scheduler_for;
use projection::projection_for;

pub(crate) const K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1: &str =
    "nando.k1-candidate-freeze-authority-request.v1";
pub(crate) const K1_SCHEDULER_APPEND_AUTHORITY_REQUEST_SCHEMA_V1: &str =
    "nando.k1-scheduler-append-authority-request.v1";
pub(crate) const K1_TRANSFER_SETTLEMENT_AUTHORITY_REQUEST_SCHEMA_V1: &str =
    "nando.k1-transfer-settlement-authority-request.v1";
pub(crate) const K1_FUTURE_CONTRACT_AUTHORITY_REQUEST_SCHEMA_V1: &str =
    "nando.k1-future-contract-authority-request.v1";
pub(crate) const K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1: &str =
    "nando.k1-future-prediction-authority-request.v1";
pub(crate) const K1_FUTURE_OUTCOME_AUTHORITY_REQUEST_SCHEMA_V1: &str =
    "nando.k1-future-outcome-authority-request.v1";
const K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1: &str = "nando.k1-scheduler-authority-response.v1";
const K1_SCHEDULER_SIGNED_EVENT_SCHEMA_V1: &str = "nando.k1-scheduler-signed-event.v1";
const K1_SCHEDULER_ANCHOR_SCHEMA_V1: &str = "nando.k1-scheduler-anchor.v1";
const K1_SCHEDULER_PROJECTION_SCHEMA_V1: &str = "nando.k1-scheduler-projection.v1";
const K1_SCHEDULER_JOURNAL_DIR: &str = "k1-natural-scheduler-journal-v1";
const K1_SCHEDULER_CACHE_FILE: &str = "k1-natural-scheduler-ledger-v1.json";
const K1_SCHEDULER_ANCHOR_FILE: &str = "k1-natural-scheduler-anchor-v1.json";
const K1_EPISTEMIC_SCHEDULER_JOURNAL_DIR: &str = "k1-epistemic-scheduler-journal-v1";
const K1_EPISTEMIC_SCHEDULER_CACHE_FILE: &str = "k1-epistemic-scheduler-ledger-v1.json";
const K1_EPISTEMIC_SCHEDULER_ANCHOR_FILE: &str = "k1-epistemic-scheduler-anchor-v1.json";
const K1_SCHEDULER_MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum K1SchedulerLaneV1 {
    #[default]
    Mechanism,
    Epistemic,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1CandidateFreezeAuthorityRequestV1 {
    pub schema: String,
    #[serde(default)]
    pub lane: K1SchedulerLaneV1,
    pub catalog: K1NaturalCohortCatalogV1,
    pub deficit_snapshot: K1DeficitSnapshotV1,
    pub queue: K1NaturalCandidateQueueV1,
    pub candidate: K1NaturalCohortCandidateV1,
    pub freeze: K1NaturalCandidateFreezeV1,
    pub active_protocol_mode_set_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1SchedulerAppendAuthorityRequestV1 {
    pub schema: String,
    #[serde(default)]
    pub lane: K1SchedulerLaneV1,
    pub payload: K1SchedulerEventPayloadV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1TransferSettlementAuthorityRequestV1 {
    pub schema: String,
    #[serde(default)]
    pub lane: K1SchedulerLaneV1,
    pub settlement: K1TransferSettlementV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1FutureContractAuthorityRequestV1 {
    pub schema: String,
    pub lane: K1SchedulerLaneV1,
    pub candidate_freeze_root_sha256: String,
    pub identification_freeze_root_sha256: String,
    pub semantic_class_root_sha256: String,
    pub protocol_mode_root_sha256: String,
    pub canonical_program: ResponseProgram,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1FuturePredictionAuthorityRequestV1 {
    pub schema: String,
    pub lane: K1SchedulerLaneV1,
    pub contract_root_sha256: String,
    pub topology: PreActionTopologyAuditRowV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1FutureOutcomeAuthorityRequestV1 {
    pub schema: String,
    pub lane: K1SchedulerLaneV1,
    pub prediction_root_sha256: String,
    pub topology: PreActionTopologyAuditRowV1,
    pub frame: nando_operator_kernel::RelationFrame,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program_evidence: Option<NaturalT1ProgramArtifactV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct K1SchedulerAuthorityResponseV1 {
    schema: String,
    projection: Option<K1SchedulerProjectionV1>,
    error: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SignedSchedulerEventV1 {
    schema: String,
    signed_root_sha256: String,
    event: K1SchedulerEventV1,
    resulting_ledger_root_sha256: String,
    signer_public_key_sha256: String,
    signature_ed25519_hex: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SchedulerAnchorV1 {
    schema: String,
    anchor_root_sha256: String,
    revision: u64,
    journal_event_root_sha256: String,
    ledger_root_sha256: String,
    signer_public_key_sha256: String,
    signature_ed25519_hex: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct K1SchedulerProjectionV1 {
    pub schema: String,
    pub projection_root_sha256: String,
    pub ledger_revision: u64,
    pub ledger_root_sha256: String,
    pub latest_event_root_sha256: String,
    pub completed_generations: u64,
    pub completed_candidate_roots_sha256: Vec<String>,
    pub next_generation_sequence: u64,
    pub active_candidate_freeze: Option<K1NaturalCandidateFreezeV1>,
    pub identification_freeze: Option<K1IdentificationFreezeV1>,
    pub future_prediction_contract: Option<K1FuturePredictionContractV1>,
    pub future_predictions: Vec<K1FuturePredictionReceiptV1>,
    pub future_outcomes: Vec<K1FutureOutcomeReceiptV1>,
    pub latest_probe_round: Option<K1ProbeRoundReceiptV1>,
    pub completed_probe_rounds: u64,
    pub latest_applied_outcome: Option<K1ProbeRoundReceiptV1>,
    pub consumed_outcome_roots_sha256: Vec<String>,
    pub applied_outcome_roots_sha256: Vec<String>,
    pub remaining_probe_budget: Option<K1ProbeBudgetRemainingV1>,
    pub latest_terminal_verdict: Option<K1GenerationTerminalVerdictV1>,
    pub pending_terminal_transfer: Option<K1GenerationTerminalVerdictV1>,
    pub latest_transfer_settlement: Option<K1TransferSettlementV1>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct K1SchedulerProjectionDigestV1<'a> {
    schema: &'static str,
    ledger_revision: u64,
    ledger_root_sha256: &'a str,
    latest_event_root_sha256: &'a str,
    completed_generations: u64,
    completed_candidate_roots_sha256: &'a [String],
    next_generation_sequence: u64,
    active_candidate_freeze_root_sha256: Option<&'a str>,
    identification_freeze_root_sha256: Option<&'a str>,
    future_prediction_contract_root_sha256: Option<&'a str>,
    future_prediction_roots_sha256: Vec<&'a str>,
    future_outcome_roots_sha256: Vec<&'a str>,
    latest_probe_round_root_sha256: Option<&'a str>,
    completed_probe_rounds: u64,
    latest_applied_outcome_root_sha256: Option<&'a str>,
    consumed_outcome_roots_sha256: &'a [String],
    applied_outcome_roots_sha256: &'a [String],
    remaining_probe_budget: Option<K1ProbeBudgetRemainingV1>,
    latest_terminal_verdict_root_sha256: Option<&'a str>,
    pending_terminal_transfer_root_sha256: Option<&'a str>,
    latest_transfer_settlement_root_sha256: Option<&'a str>,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

pub(crate) fn append_candidate_freeze_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    mut request: K1CandidateFreezeAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    request.lane = lane;
    send_authority_request(config, &request)
}

pub(crate) fn append_scheduler_payload_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    payload: K1SchedulerEventPayloadV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if matches!(
        payload,
        K1SchedulerEventPayloadV1::CandidateFreeze(_)
            | K1SchedulerEventPayloadV1::TransferSettlement(_)
    ) {
        return Err("k1_scheduler_payload_requires_authority_cas".to_owned());
    }
    send_authority_request(
        config,
        &K1SchedulerAppendAuthorityRequestV1 {
            schema: K1_SCHEDULER_APPEND_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
            lane,
            payload,
        },
    )
}

pub(crate) fn append_transfer_settlement(
    config: &CertificationAuthorityConfigV1,
    settlement: K1TransferSettlementV1,
) -> Result<K1SchedulerProjectionV1, String> {
    send_authority_request(
        config,
        &K1TransferSettlementAuthorityRequestV1 {
            schema: K1_TRANSFER_SETTLEMENT_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
            lane: K1SchedulerLaneV1::Epistemic,
            settlement,
        },
    )
}

pub(crate) fn append_future_contract(
    config: &CertificationAuthorityConfigV1,
    request: K1FutureContractAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    send_authority_request(config, &request)
}

pub(crate) fn append_future_prediction(
    config: &CertificationAuthorityConfigV1,
    request: K1FuturePredictionAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    send_authority_request(config, &request)
}

pub(crate) fn append_future_outcome(
    config: &CertificationAuthorityConfigV1,
    request: K1FutureOutcomeAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    send_authority_request(config, &request)
}

pub(crate) fn restore_projection(
    config: &CertificationAuthorityConfigV1,
) -> Result<K1SchedulerProjectionV1, String> {
    restore_projection_for(config, K1SchedulerLaneV1::Epistemic)
}

pub(crate) fn restore_projection_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
) -> Result<K1SchedulerProjectionV1, String> {
    projection_for(&restore_anchored_scheduler_for(config, lane)?)
}

pub(crate) fn candidate_exclusions_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
) -> Result<BTreeSet<String>, String> {
    match lane {
        K1SchedulerLaneV1::Mechanism => Ok(BTreeSet::new()),
        K1SchedulerLaneV1::Epistemic => fork::epistemic_exclusions(config),
    }
}

pub(crate) fn duplicate_candidate_exclusions_for(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    catalog: &K1NaturalCohortCatalogV1,
    active_protocol_mode_set_root_sha256: &str,
) -> Result<BTreeSet<String>, String> {
    if lane == K1SchedulerLaneV1::Mechanism {
        return Ok(BTreeSet::new());
    }
    duplicate_cohorts::duplicate_candidate_exclusions(
        &restore_anchored_scheduler_for(config, lane)?,
        catalog,
        active_protocol_mode_set_root_sha256,
    )
}

pub(crate) fn current_deficit_snapshot(
    config: &CertificationAuthorityConfigV1,
) -> Result<K1DeficitSnapshotV1, String> {
    let ledger = restore_anchored_ledger(config)?;
    let gate = ledger.k1_vocabulary_gate().map_err(str::to_owned)?;
    let eligible = ledger
        .latest_entries()
        .into_iter()
        .filter(|entry| entry.k1_unit_eligible)
        .collect::<Vec<_>>();
    let semantic_roots = eligible
        .iter()
        .map(|entry| entry.semantic_law_id_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let topology_roots = eligible
        .iter()
        .map(|entry| entry.role_topology_id_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let known_consequence_types = known_k1_consequence_types(config, &eligible)?;
    K1DeficitSnapshotV1::seal(
        ledger.revision,
        ledger.ledger_root_sha256,
        gate.gate_root_sha256,
        gate.law_certificates,
        gate.semantic_laws,
        gate.role_topologies,
        gate.cleanup_receipts,
        gate.false_bad_apply,
        gate.min_law_certificates,
        gate.min_semantic_laws,
        gate.min_role_topologies,
        semantic_roots,
        topology_roots,
        known_consequence_types,
        gate.open,
    )
    .map_err(str::to_owned)
}

fn known_k1_consequence_types(
    config: &CertificationAuthorityConfigV1,
    eligible: &[&nando_operator_admission::OperatorCertificationEntryV1],
) -> Result<Vec<K1ConsequenceTypeV1>, String> {
    let registry: ResponseRegistry = serde_json::from_slice(
        &fs::read(&config.response_registry_path)
            .map_err(|error| format!("k1_scheduler_registry_read:{error}"))?,
    )
    .map_err(|error| format!("k1_scheduler_registry_decode:{error}"))?;
    registry.validate().map_err(str::to_owned)?;
    let mut types = eligible
        .iter()
        .map(|entry| {
            registry
                .packages
                .iter()
                .find(|package| package.package_id == entry.package_id)
                .map(|package| response_program_consequence_type(&package.program))
                .ok_or_else(|| "k1_scheduler_registry_package_missing".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    types.sort_unstable();
    types.dedup();
    Ok(types)
}

fn response_program_consequence_type(program: &ResponseProgram) -> K1ConsequenceTypeV1 {
    match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => {
            let mut types = variants
                .iter()
                .map(|variant| response_program_consequence_type(&variant.program));
            let Some(first) = types.next() else {
                return K1ConsequenceTypeV1::Record;
            };
            if types.all(|value| value == first) {
                first
            } else {
                K1ConsequenceTypeV1::Record
            }
        }
        ResponseOperation::ProjectSelectedValue { renderer, .. } => {
            if matches!(renderer, CollectionOutputRenderer::Direct) {
                K1ConsequenceTypeV1::Scalar
            } else {
                K1ConsequenceTypeV1::RenderedSequence
            }
        }
        ResponseOperation::ProjectStatus { renderer, .. } => {
            if matches!(renderer, CollectionOutputRenderer::Direct) {
                K1ConsequenceTypeV1::Boolean
            } else {
                K1ConsequenceTypeV1::RenderedSequence
            }
        }
        ResponseOperation::ComposeCollection { .. } => K1ConsequenceTypeV1::Collection,
        ResponseOperation::CopyAfterPrefix { .. } => K1ConsequenceTypeV1::Scalar,
        ResponseOperation::TestResultSummary { .. }
        | ResponseOperation::AdvancePlan { .. }
        | ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. }
        | ResponseOperation::WaitOnYieldedCell { .. }
        | ResponseOperation::WaitOnAnyYieldedCell { .. }
        | ResponseOperation::WaitOnYieldedSurfaces { .. } => K1ConsequenceTypeV1::Record,
    }
}

#[cfg(test)]
mod tests;
