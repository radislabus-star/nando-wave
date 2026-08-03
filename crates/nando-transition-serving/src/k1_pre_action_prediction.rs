use std::time::{SystemTime, UNIX_EPOCH};

use nando_operator_kernel::{
    AtomValueType, LearningRequestStructureV2, MultiSourceEvidenceOriginV1,
    PreActionTopologyCommitV1, ResponseOperation, ResponseProgram, canonical_json_sha256,
    response_program_version_root_sha256, sha256_bytes,
};
use nando_operator_learning::multi_source::{
    K1PreActionExecutionReceiptV1, PreActionTopologyAuditRowV1,
    pre_action_applicability_shape_root_v1, pre_action_t1_binding_root,
    pre_action_t1_selector_witnesses_v1, source_neutral_topology_root_v1,
};
use nando_response_actor::{
    ResponseExecutionStatus, execute_response, response_runtime_contract_sha256,
    selected_value_with_request,
};

use crate::k1_natural_scheduler::{
    K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1,
    K1_PRE_ACTION_EVIDENCE_AUTHORITY_REQUEST_SCHEMA_V1, K1FuturePredictionAuthorityRequestV1,
    K1PreActionEvidenceAuthorityRequestV1, K1SchedulerLaneV1, K1SchedulerProjectionV1,
    append_future_prediction, archive_pre_action_evidence, restore_projection,
};
use crate::operator_certification::CertificationAuthorityConfigV1;

pub(crate) fn candidate_match_requires_fence(
    projection: &K1SchedulerProjectionV1,
    structure: &LearningRequestStructureV2,
    commit: &PreActionTopologyCommitV1,
) -> Result<bool, String> {
    let Some(candidate) = projection.active_candidate_freeze.as_ref() else {
        return Ok(false);
    };
    let Some(contract) = projection.future_prediction_contract.as_ref() else {
        return Ok(false);
    };
    Ok(matches!(
        contract.canonical_program.operation,
        ResponseOperation::ComposeCollection { .. }
    ) && commit.capture_sequence >= candidate.future_min_sequence
        && commit.evidence_origin == MultiSourceEvidenceOriginV1::FreshLive
        && pre_action_applicability_shape_root_v1(&structure.topology).map_err(str::to_owned)?
            == candidate.candidate_structural_root_sha256
        && source_neutral_topology_root_v1(&structure.topology).map_err(str::to_owned)?
            == candidate.source_neutral_topology_root_sha256
        && pre_action_t1_binding_root(&contract.canonical_program, &structure.topology).is_ok()
        && !projection.future_predictions.iter().any(|prediction| {
            !projection
                .future_outcomes
                .iter()
                .any(|outcome| outcome.prediction_root_sha256 == prediction.prediction_root_sha256)
        }))
}

pub(crate) fn precommit_candidate_match(
    config: &CertificationAuthorityConfigV1,
    topology: PreActionTopologyAuditRowV1,
    provider_payload_json: &str,
) -> Result<bool, String> {
    let projection = restore_projection(config)?;
    let Some(candidate) = projection.active_candidate_freeze.as_ref() else {
        return Ok(false);
    };
    let Some(contract) = projection.future_prediction_contract.as_ref() else {
        return Ok(false);
    };
    if !matches!(
        contract.canonical_program.operation,
        ResponseOperation::ComposeCollection { .. }
    ) || topology.commit.capture_sequence < candidate.future_min_sequence
        || topology.commit.evidence_origin != MultiSourceEvidenceOriginV1::FreshLive
        || pre_action_applicability_shape_root_v1(&topology.structure.topology)
            .map_err(str::to_owned)?
            != candidate.candidate_structural_root_sha256
        || source_neutral_topology_root_v1(&topology.structure.topology).map_err(str::to_owned)?
            != candidate.source_neutral_topology_root_sha256
        || pre_action_t1_binding_root(&contract.canonical_program, &topology.structure.topology)
            .is_err()
        || projection.future_predictions.iter().any(|prediction| {
            !projection
                .future_outcomes
                .iter()
                .any(|outcome| outcome.prediction_root_sha256 == prediction.prediction_root_sha256)
        })
    {
        return Ok(false);
    }
    let archive_request = K1PreActionEvidenceAuthorityRequestV1 {
        schema: K1_PRE_ACTION_EVIDENCE_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Epistemic,
        contract_root_sha256: contract.contract_root_sha256.clone(),
        topology_commitment_root_sha256: topology.commit.commitment_root_sha256.clone(),
        provider_capture_request_root_sha256: topology
            .commit
            .provider_capture_request_root_sha256
            .clone(),
        provider_payload_json: provider_payload_json.to_owned(),
    };
    let mut archived = false;
    for _ in 0..100 {
        match archive_pre_action_evidence(config, archive_request.clone()) {
            Ok(_) => {
                archived = true;
                break;
            }
            Err(error) if error.contains("multi_source_topology_archive_row_missing") => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(error) => return Err(error),
        }
    }
    if !archived {
        return Err("k1_pre_action_topology_archive_ack_timeout".to_owned());
    }
    append_future_prediction(
        config,
        K1FuturePredictionAuthorityRequestV1 {
            schema: K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
            lane: K1SchedulerLaneV1::Epistemic,
            contract_root_sha256: contract.contract_root_sha256.clone(),
            topology_commitment_root_sha256: topology.commit.commitment_root_sha256,
            provider_capture_request_root_sha256: topology
                .commit
                .provider_capture_request_root_sha256,
        },
    )?;
    Ok(true)
}

pub(crate) fn execute_collection_prediction(
    contract_root_sha256: String,
    canonical_program: &ResponseProgram,
    topology: &PreActionTopologyAuditRowV1,
    provider_payload_json: &str,
) -> Result<K1PreActionExecutionReceiptV1, String> {
    let provider_capture_request_root_sha256 =
        topology.commit.provider_capture_request_root_sha256.clone();
    if sha256_bytes(provider_payload_json.as_bytes()) != provider_capture_request_root_sha256 {
        return Err("k1_pre_action_request_digest_mismatch".to_owned());
    }
    let payload: serde_json::Value = serde_json::from_str(provider_payload_json)
        .map_err(|_| "k1_pre_action_provider_payload_invalid".to_owned())?;
    let request_text = crate::extract_request_text(&payload);
    let structural_binding_root =
        pre_action_t1_binding_root(canonical_program, &topology.structure.topology)
            .map_err(|reason| format!("k1_pre_action_role_binding:{reason}"))?;
    for (selector, frozen_witness_hash) in
        pre_action_t1_selector_witnesses_v1(canonical_program, &topology.structure.topology)
            .map_err(|reason| format!("k1_pre_action_selector_binding:{reason}"))?
    {
        let selected = selected_value_with_request(&request_text, &payload, &selector)
            .map_err(|reason| format!("k1_pre_action_selector_runtime:{reason}"))?;
        let runtime_value_hash = canonical_json_sha256(&selected.value)
            .map_err(|error| format!("k1_pre_action_selector_hash:{error}"))?;
        if runtime_value_hash != frozen_witness_hash {
            return Err("k1_pre_action_selector_witness_mismatch".to_owned());
        }
    }
    let execution = execute_response(canonical_program, &request_text, &payload);
    if execution.status != ResponseExecutionStatus::Executed {
        return Err(format!(
            "k1_pre_action_program_not_executed:{}",
            execution.reason
        ));
    }
    let response = execution
        .response
        .ok_or_else(|| "k1_pre_action_response_missing".to_owned())?;
    let program_root =
        response_program_version_root_sha256(canonical_program).map_err(str::to_owned)?;
    let consequence_root = nando_operator_learning::multi_source::typed_consequence_root_v1(
        typed_response_value(&response),
        &sha256_bytes(response.as_bytes()),
    )
    .map_err(str::to_owned)?;
    let complete_binding_root = canonical_json_sha256(&(
        "nando.k1-complete-pre-action-input-binding.v3",
        program_root.as_str(),
        structural_binding_root.as_str(),
        provider_capture_request_root_sha256.as_str(),
        sha256_bytes(request_text.as_bytes()),
        sha256_bytes(provider_payload_json.as_bytes()),
    ))
    .map_err(str::to_owned)?;
    let verifier_contract_root = canonical_json_sha256(&(
        "nando.k1-pre-action-execution-verifier-contract.v2",
        response_runtime_contract_sha256(),
        structural_binding_root,
        "exact_typed_consequence_root",
        "authority_owned_execution",
    ))
    .map_err(str::to_owned)?;
    K1PreActionExecutionReceiptV1::seal(
        contract_root_sha256,
        program_root,
        topology
            .capture_event_sha256
            .clone()
            .ok_or_else(|| "k1_pre_action_capture_event_missing".to_owned())?,
        provider_capture_request_root_sha256,
        topology.structure.turn_intent_id_sha256.clone(),
        complete_binding_root,
        consequence_root,
        verifier_contract_root,
        topology.commit.capture_sequence,
        topology
            .captured_at_unix_ms
            .ok_or_else(|| "k1_pre_action_capture_time_missing".to_owned())?,
        unix_now_nanos()?,
    )
    .map_err(str::to_owned)
}

fn typed_response_value(response: &str) -> AtomValueType {
    match serde_json::from_str::<serde_json::Value>(response) {
        Ok(serde_json::Value::Bool(_)) => AtomValueType::Boolean,
        Ok(serde_json::Value::Number(_)) => AtomValueType::Integer,
        Ok(serde_json::Value::Array(_) | serde_json::Value::Object(_)) => AtomValueType::Collection,
        _ => AtomValueType::String,
    }
}

fn unix_now_nanos() -> Result<u64, String> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("k1_pre_action_clock:{error}"))?
            .as_nanos(),
    )
    .map_err(|_| "k1_pre_action_clock_overflow".to_owned())
}

#[cfg(test)]
#[path = "k1_pre_action_prediction_tests.rs"]
mod tests;
