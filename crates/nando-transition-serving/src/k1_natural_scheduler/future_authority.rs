use std::time::{SystemTime, UNIX_EPOCH};

use nando_operator_kernel::MultiSourceEvidenceOriginV1;
use nando_operator_learning::multi_source::{
    K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1, MultiSourceJoinLedgerV1, t1_program_is_consistent,
    validate_pre_action_topology_join_eligibility_v1,
};

use super::authority::append_and_persist;
use super::journal::restore_anchored_scheduler_for;
use super::projection::projection_for;
use super::*;

pub(super) fn append_future_contract_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1FutureContractAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_FUTURE_CONTRACT_AUTHORITY_REQUEST_SCHEMA_V1
        || request.lane != K1SchedulerLaneV1::Epistemic
    {
        return Err("k1_future_contract_request_invalid".to_owned());
    }
    request
        .canonical_program
        .validate()
        .map_err(str::to_owned)?;
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let projection = projection_for(&scheduler)?;
    let candidate = projection
        .active_candidate_freeze
        .as_ref()
        .ok_or_else(|| "k1_future_contract_candidate_missing".to_owned())?;
    let identification = projection
        .identification_freeze
        .as_ref()
        .ok_or_else(|| "k1_future_contract_identification_missing".to_owned())?;
    if candidate.freeze_root_sha256 != request.candidate_freeze_root_sha256
        || identification.freeze_root_sha256 != request.identification_freeze_root_sha256
        || identification.prediction_schema != K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1
        || identification
            .initial_semantic_class_roots_sha256
            .as_slice()
            != [request.semantic_class_root_sha256.as_str()]
    {
        return Err("k1_future_contract_freeze_mismatch".to_owned());
    }
    if let Some(existing) = projection.future_prediction_contract {
        if existing.candidate_freeze_root_sha256 == request.candidate_freeze_root_sha256
            && existing.identification_freeze_root_sha256
                == request.identification_freeze_root_sha256
            && existing.semantic_class_root_sha256 == request.semantic_class_root_sha256
            && existing.protocol_mode_root_sha256 == request.protocol_mode_root_sha256
            && existing.canonical_program == request.canonical_program
        {
            return projection_for(&scheduler);
        }
        return Err("k1_future_contract_replacement_forbidden".to_owned());
    }
    let contract = K1FuturePredictionContractV1::seal(
        request.candidate_freeze_root_sha256,
        request.identification_freeze_root_sha256,
        request.semantic_class_root_sha256,
        request.protocol_mode_root_sha256,
        request.canonical_program,
        unix_now_nanos()?,
    )
    .map_err(str::to_owned)?;
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::FuturePredictionContract(contract),
    )
}

pub(super) fn append_future_prediction_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1FuturePredictionAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1
        || request.lane != K1SchedulerLaneV1::Epistemic
    {
        return Err("k1_future_prediction_request_invalid".to_owned());
    }
    let topology = pre_action_evidence::restore_topology(
        config,
        &request.topology_commitment_root_sha256,
        &request.provider_capture_request_root_sha256,
    )?;
    validate_pre_action_topology_join_eligibility_v1(&topology)
        .map_err(|reason| format!("k1_future_prediction_topology:{reason:?}"))?;
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let projection = projection_for(&scheduler)?;
    let candidate = projection
        .active_candidate_freeze
        .as_ref()
        .ok_or_else(|| "k1_future_prediction_candidate_missing".to_owned())?;
    let identification = projection
        .identification_freeze
        .as_ref()
        .ok_or_else(|| "k1_future_prediction_identification_missing".to_owned())?;
    let contract = projection
        .future_prediction_contract
        .as_ref()
        .ok_or_else(|| "k1_future_prediction_contract_missing".to_owned())?;
    if contract.contract_root_sha256 != request.contract_root_sha256
        || topology.commit.capture_sequence < candidate.future_min_sequence
        || topology.commit.evidence_origin != MultiSourceEvidenceOriginV1::FreshLive
        || candidate_program_binding_root(
            candidate,
            &contract.canonical_program,
            &topology.structure.topology,
        )
        .is_err()
    {
        return Err("k1_future_prediction_candidate_mismatch".to_owned());
    }
    if projection.future_predictions.iter().any(|prediction| {
        prediction.topology_commitment_root_sha256 == topology.commit.commitment_root_sha256
    }) {
        return Ok(projection);
    }
    let captured_at_unix_ms = topology
        .captured_at_unix_ms
        .ok_or_else(|| "k1_future_prediction_capture_time_missing".to_owned())?;
    let prediction = match &contract.canonical_program.operation {
        ResponseOperation::ComposeCollection { .. } => {
            let evidence = pre_action_evidence::restore(
                config,
                &request.topology_commitment_root_sha256,
                &request.provider_capture_request_root_sha256,
            )?;
            let receipt =
                crate::k1_pre_action_prediction::execute_collection_prediction_for_candidate(
                    contract.contract_root_sha256.clone(),
                    candidate,
                    &contract.canonical_program,
                    &evidence.topology,
                    &evidence.provider_payload_json,
                )?;
            let predicted_at_unix_nanos = unix_now_nanos()?;
            if receipt.contract_root_sha256 != contract.contract_root_sha256
                || receipt.canonical_program_root_sha256 != contract.canonical_program_root_sha256
                || topology.capture_event_sha256.as_deref()
                    != Some(receipt.provider_capture_event_root_sha256.as_str())
                || receipt.provider_capture_request_root_sha256
                    != topology.commit.provider_capture_request_root_sha256
                || receipt.turn_intent_id_sha256 != topology.structure.turn_intent_id_sha256
                || receipt.capture_sequence != topology.commit.capture_sequence
                || receipt.captured_at_unix_ms != captured_at_unix_ms
                || receipt.executed_at_unix_nanos > predicted_at_unix_nanos
            {
                return Err("k1_future_collection_typed_prediction_rebound".to_owned());
            }
            K1FuturePredictionReceiptV1::seal_typed(
                contract.contract_root_sha256.clone(),
                candidate.freeze_root_sha256.clone(),
                identification.freeze_root_sha256.clone(),
                contract.semantic_class_root_sha256.clone(),
                topology.commit.commitment_root_sha256,
                topology.commit.provider_capture_request_root_sha256,
                topology.structure.turn_intent_id_sha256,
                receipt.complete_pre_action_binding_root_sha256,
                &contract.canonical_program_root_sha256,
                receipt.receipt_root_sha256,
                receipt.predicted_typed_consequence_root_sha256,
                receipt.execution_verifier_contract_root_sha256,
                topology.commit.capture_sequence,
                captured_at_unix_ms,
                receipt.executed_at_unix_nanos,
                predicted_at_unix_nanos,
            )
        }
        _ => {
            let predicted_at_unix_nanos = unix_now_nanos()?;
            let binding_root = candidate_program_binding_root(
                candidate,
                &contract.canonical_program,
                &topology.structure.topology,
            )?;
            K1FuturePredictionReceiptV1::seal(
                contract.contract_root_sha256.clone(),
                candidate.freeze_root_sha256.clone(),
                identification.freeze_root_sha256.clone(),
                contract.semantic_class_root_sha256.clone(),
                topology.commit.commitment_root_sha256,
                topology.commit.provider_capture_request_root_sha256,
                topology.structure.turn_intent_id_sha256,
                binding_root,
                &contract.canonical_program_root_sha256,
                topology.commit.capture_sequence,
                captured_at_unix_ms,
                predicted_at_unix_nanos,
            )
        }
    }
    .map_err(str::to_owned)?;
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::FuturePrediction(prediction),
    )
}

pub(super) fn append_future_prediction_censor_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1FuturePredictionCensorAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_FUTURE_PREDICTION_CENSOR_AUTHORITY_REQUEST_SCHEMA_V1
        || request.lane != K1SchedulerLaneV1::Epistemic
    {
        return Err("k1_future_prediction_censor_request_invalid".to_owned());
    }
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let projection = projection_for(&scheduler)?;
    if let Some(existing) = projection
        .future_prediction_censors
        .iter()
        .find(|receipt| receipt.prediction_root_sha256 == request.prediction_root_sha256)
    {
        return if existing.fence_topology_commitment_root_sha256
            == request.fence_topology_commitment_root_sha256
        {
            Ok(projection)
        } else {
            Err("k1_future_prediction_censor_rebound".to_owned())
        };
    }
    if projection
        .future_outcomes
        .iter()
        .any(|outcome| outcome.prediction_root_sha256 == request.prediction_root_sha256)
    {
        return Err("k1_future_prediction_already_settled".to_owned());
    }
    let prediction = projection
        .future_predictions
        .iter()
        .find(|prediction| prediction.prediction_root_sha256 == request.prediction_root_sha256)
        .ok_or_else(|| "k1_future_prediction_censor_prediction_missing".to_owned())?;
    let predicted_topology = pre_action_evidence::restore_topology(
        config,
        &prediction.topology_commitment_root_sha256,
        &prediction.provider_capture_request_root_sha256,
    )?;
    let fence_topology = pre_action_evidence::restore_topology(
        config,
        &request.fence_topology_commitment_root_sha256,
        &request.fence_provider_capture_request_root_sha256,
    )?;
    let predicted_lineage = predicted_topology
        .session_lineage_sha256
        .as_deref()
        .ok_or_else(|| "k1_future_prediction_censor_lineage_missing".to_owned())?;
    let fence_lineage = fence_topology
        .session_lineage_sha256
        .as_deref()
        .ok_or_else(|| "k1_future_prediction_censor_fence_lineage_missing".to_owned())?;
    let terminal_archive = config
        .root
        .parent()
        .ok_or_else(|| "k1_future_prediction_censor_root_parent_missing".to_owned())?
        .join("terminal-receipt-archive-v1");
    let terminal = crate::terminal_receipt_archive::read_terminal_receipt_for_request(
        &terminal_archive,
        &predicted_topology.structure.request_event_id_sha256,
    )?
    .ok_or_else(|| "k1_future_prediction_censor_terminal_missing".to_owned())?;
    let frame_archive = config
        .root
        .parent()
        .ok_or_else(|| "k1_future_prediction_censor_root_parent_missing".to_owned())?
        .join("relation-frame-archive-v1");
    if crate::multi_source_frame_archive::completed_frame_exists_for_intent(
        &frame_archive,
        &prediction.turn_intent_id_sha256,
    )? {
        return Err("k1_future_prediction_censor_completed_frame_exists".to_owned());
    }
    let fence_captured_at_unix_nanos = fence_topology
        .captured_at_unix_ms
        .ok_or_else(|| "k1_future_prediction_censor_fence_time_missing".to_owned())?
        .saturating_mul(1_000_000);
    if predicted_topology.commit.commitment_root_sha256
        != prediction.topology_commitment_root_sha256
        || predicted_topology.commit.capture_sequence != prediction.capture_sequence
        || predicted_topology.structure.turn_intent_id_sha256 != prediction.turn_intent_id_sha256
        || predicted_topology.commit.evidence_origin != MultiSourceEvidenceOriginV1::FreshLive
        || fence_topology.commit.evidence_origin != MultiSourceEvidenceOriginV1::FreshLive
        || predicted_lineage != fence_lineage
        || fence_topology.commit.capture_sequence <= prediction.capture_sequence
        || fence_captured_at_unix_nanos <= terminal.completed_at_unix_nanos
        || terminal.completed_at_unix_nanos <= prediction.predicted_at_unix_nanos
        || fence_topology.structure.request_event_id_sha256
            == predicted_topology.structure.request_event_id_sha256
    {
        return Err("k1_future_prediction_censor_fence_invalid".to_owned());
    }
    let receipt = K1FuturePredictionCensorReceiptV1::seal_missing_completed_frame(
        prediction.prediction_root_sha256.clone(),
        prediction.topology_commitment_root_sha256.clone(),
        prediction.capture_sequence,
        predicted_topology.structure.request_event_id_sha256,
        terminal.receipt_root_sha256,
        terminal.completed_at_unix_nanos,
        fence_topology.commit.commitment_root_sha256,
        fence_topology.structure.request_event_id_sha256,
        predicted_lineage.to_owned(),
        fence_topology.commit.capture_sequence,
        fence_captured_at_unix_nanos,
        unix_now_nanos()?,
    )
    .map_err(str::to_owned)?;
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::FuturePredictionCensored(receipt),
    )
}

pub(super) fn archive_pre_action_evidence_authoritative(
    config: &CertificationAuthorityConfigV1,
    request: K1PreActionEvidenceAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_PRE_ACTION_EVIDENCE_AUTHORITY_REQUEST_SCHEMA_V1
        || request.lane != K1SchedulerLaneV1::Epistemic
    {
        return Err("k1_pre_action_evidence_request_invalid".to_owned());
    }
    let scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let projection = projection_for(&scheduler)?;
    let candidate = projection
        .active_candidate_freeze
        .as_ref()
        .ok_or_else(|| "k1_pre_action_evidence_candidate_missing".to_owned())?;
    let contract = projection
        .future_prediction_contract
        .as_ref()
        .filter(|contract| contract.contract_root_sha256 == request.contract_root_sha256)
        .ok_or_else(|| "k1_pre_action_evidence_contract_mismatch".to_owned())?;
    if !matches!(
        contract.canonical_program.operation,
        ResponseOperation::ComposeCollection { .. }
    ) {
        return Err("k1_pre_action_evidence_collection_contract_required".to_owned());
    }
    let topology = pre_action_evidence::restore_topology(
        config,
        &request.topology_commitment_root_sha256,
        &request.provider_capture_request_root_sha256,
    )?;
    if topology.commit.capture_sequence < candidate.future_min_sequence
        || topology.commit.evidence_origin != MultiSourceEvidenceOriginV1::FreshLive
        || candidate_program_binding_root(
            candidate,
            &contract.canonical_program,
            &topology.structure.topology,
        )
        .is_err()
    {
        return Err("k1_pre_action_evidence_candidate_mismatch".to_owned());
    }
    pre_action_evidence::archive(
        config,
        &request.topology_commitment_root_sha256,
        &request.provider_capture_request_root_sha256,
        request.provider_payload_json,
    )?;
    Ok(projection)
}

pub(super) fn append_future_outcome_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1FutureOutcomeAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_FUTURE_OUTCOME_AUTHORITY_REQUEST_SCHEMA_V1
        || request.lane != K1SchedulerLaneV1::Epistemic
    {
        return Err("k1_future_outcome_request_invalid".to_owned());
    }
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let projection = projection_for(&scheduler)?;
    if projection
        .future_outcomes
        .iter()
        .any(|outcome| outcome.prediction_root_sha256 == request.prediction_root_sha256)
    {
        return Ok(projection);
    }
    let prediction = projection
        .future_predictions
        .iter()
        .find(|prediction| prediction.prediction_root_sha256 == request.prediction_root_sha256)
        .ok_or_else(|| "k1_future_outcome_prediction_missing".to_owned())?;
    let contract = projection
        .future_prediction_contract
        .as_ref()
        .ok_or_else(|| "k1_future_outcome_contract_missing".to_owned())?;
    let candidate = projection
        .active_candidate_freeze
        .as_ref()
        .ok_or_else(|| "k1_future_outcome_candidate_missing".to_owned())?;
    candidate_program_binding_root(
        candidate,
        &contract.canonical_program,
        &request.topology.structure.topology,
    )
    .map_err(|_| "k1_future_outcome_candidate_mismatch".to_owned())?;
    let join = MultiSourceJoinLedgerV1::build(
        std::slice::from_ref(&request.topology),
        std::slice::from_ref(&request.frame),
    );
    let rows = join.rows();
    let joined = rows
        .first()
        .filter(|_| rows.len() == 1)
        .ok_or_else(|| "k1_future_outcome_join_failed".to_owned())?;
    if joined.topology_commitment_root_sha256 != prediction.topology_commitment_root_sha256
        || joined.turn_intent_id_sha256 != prediction.turn_intent_id_sha256
        || joined.capture_sequence != prediction.capture_sequence
        || request.frame.observed_at_unix_nanos <= prediction.predicted_at_unix_nanos
    {
        return Err("k1_future_outcome_prediction_rebound".to_owned());
    }
    let (program_evidence_root, typed_consequences, program_consistent) =
        match &contract.canonical_program.operation {
            ResponseOperation::ComposeCollection { .. } => {
                if request.program_evidence.is_some() {
                    return Err("k1_future_collection_hypothesis_has_no_authority".to_owned());
                }
                let predicted = prediction
                    .predicted_typed_consequence_root_sha256
                    .as_deref()
                    .ok_or_else(|| "k1_future_collection_typed_prediction_missing".to_owned())?;
                let observed =
                    nando_operator_learning::multi_source::observed_typed_consequence_root_v1(
                        &request.frame,
                    )
                    .map_err(str::to_owned)?;
                (
                    None,
                    Some((predicted.to_owned(), observed.clone())),
                    predicted == observed,
                )
            }
            _ => {
                if request.program_evidence.is_some() {
                    return Err("k1_future_unexpected_program_evidence".to_owned());
                }
                (
                    None,
                    None,
                    t1_program_is_consistent(&contract.canonical_program, joined, &request.frame),
                )
            }
        };
    let independent_verifier_pass =
        joined.accepted && request.frame.verifier_label == Some(true) && program_consistent;
    let outcome = match (typed_consequences, program_evidence_root) {
        (Some((predicted, observed)), None) => {
            K1FutureOutcomeReceiptV1::seal_with_typed_consequence(
                prediction.prediction_root_sha256.clone(),
                joined.join_root_sha256.clone(),
                joined.completed_frame_root_sha256.clone(),
                joined.semantic_action_root_sha256.clone(),
                joined.verifier_receipt_root_sha256.clone(),
                predicted,
                observed,
                request.frame.observed_at_unix_nanos,
                joined.accepted && request.frame.verifier_label == Some(true),
            )
        }
        (None, Some(evidence_root)) => K1FutureOutcomeReceiptV1::seal_with_program_evidence(
            prediction.prediction_root_sha256.clone(),
            joined.join_root_sha256.clone(),
            joined.completed_frame_root_sha256.clone(),
            joined.semantic_action_root_sha256.clone(),
            joined.verifier_receipt_root_sha256.clone(),
            evidence_root,
            request.frame.observed_at_unix_nanos,
            program_consistent,
            independent_verifier_pass,
        ),
        (None, None) => K1FutureOutcomeReceiptV1::seal(
            prediction.prediction_root_sha256.clone(),
            joined.join_root_sha256.clone(),
            joined.completed_frame_root_sha256.clone(),
            joined.semantic_action_root_sha256.clone(),
            joined.verifier_receipt_root_sha256.clone(),
            request.frame.observed_at_unix_nanos,
            program_consistent,
            independent_verifier_pass,
        ),
        (Some(_), Some(_)) => Err("k1_future_outcome_evidence_owner_overlap"),
    }
    .map_err(str::to_owned)?;
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::FutureOutcome(outcome),
    )
}

fn unix_now_nanos() -> Result<u64, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("k1_future_authority_clock:{error}"))?
        .as_nanos();
    u64::try_from(nanos).map_err(|_| "k1_future_authority_clock_overflow".to_owned())
}
