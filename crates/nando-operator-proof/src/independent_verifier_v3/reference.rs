use std::collections::BTreeMap;

use nando_operator_kernel::{
    BoundProtocolActionV3, BoundProtocolValueV3, EFFECT_OPERATION_CALL_V3, EFFECT_REL_COPY,
    RuntimeCapabilityKindV3, canonical_json_sha256, sha256_bytes,
};
use serde_json::{Value, json};

use super::reconstruct::ReconstructedActionV3;
use super::surface::IndependentSurfaceV3;
use super::{
    IndependentVerifierBudgetV3, IndependentVerifierErrorV3, IndependentVerifierInputV3,
    IndependentVerifierVerdictV3,
};

pub(super) struct ReferenceVerificationV3 {
    pub expected_output_sha256: Option<String>,
    pub effect_postcondition_sha256: Option<String>,
    pub preserved_frame_contract_sha256: Option<String>,
    pub preserved_frame_observation_sha256: Option<String>,
    pub verdict: IndependentVerifierVerdictV3,
}

pub(super) fn verify_reference_effect_v3(
    input: &IndependentVerifierInputV3<'_>,
    surface: &IndependentSurfaceV3,
    reconstructed: &ReconstructedActionV3,
    budget: IndependentVerifierBudgetV3,
) -> Result<ReferenceVerificationV3, IndependentVerifierErrorV3> {
    let preserved_contract = reconstructed.law.preserved_frame_root_sha256().to_owned();
    if !supported_call_law_v3(reconstructed) {
        return Ok(blocked(
            preserved_contract,
            IndependentVerifierVerdictV3::AbstainUnsupportedEffect,
        ));
    }
    let expected_output = match reference_output_v3(&reconstructed.action) {
        Some(output) if output.len() <= budget.max_actor_output_bytes => output,
        _ => {
            return Ok(blocked(
                preserved_contract,
                IndependentVerifierVerdictV3::AbstainBudgetExhausted,
            ));
        }
    };
    let expected_output_sha256 = sha256_bytes(expected_output.as_bytes());
    let preserved_frame_observation_sha256 = canonical_json_sha256(&(
        "nando.f6.output-only-preserved-frame.v3",
        reconstructed.law.preserved_frame_root_sha256(),
        surface.raw_payload_sha256.as_str(),
        surface.raw_payload_sha256.as_str(),
    ))
    .map_err(|_| IndependentVerifierErrorV3::Serialization)?;
    let effect_postcondition_sha256 = canonical_json_sha256(&(
        "nando.f6.effect-postcondition.v3",
        reconstructed.law.effect_invariant_root_sha256(),
        reconstructed.law.relation_program(),
        reconstructed.action.semantic_action_sha256(),
        reconstructed.action.physical_action_sha256(),
        expected_output_sha256.as_str(),
        preserved_frame_observation_sha256.as_str(),
    ))
    .map_err(|_| IndependentVerifierErrorV3::Serialization)?;
    let verdict = if input.actor_output().len() > budget.max_actor_output_bytes {
        IndependentVerifierVerdictV3::AbstainBudgetExhausted
    } else if input.actor_output().as_bytes() != expected_output.as_bytes()
        || serde_json::from_str::<Value>(input.actor_output()).ok()
            != serde_json::from_str::<Value>(&expected_output).ok()
    {
        IndependentVerifierVerdictV3::RejectProtocolParity
    } else {
        IndependentVerifierVerdictV3::Verified
    };
    Ok(ReferenceVerificationV3 {
        expected_output_sha256: Some(expected_output_sha256),
        effect_postcondition_sha256: Some(effect_postcondition_sha256),
        preserved_frame_contract_sha256: Some(preserved_contract),
        preserved_frame_observation_sha256: Some(preserved_frame_observation_sha256),
        verdict,
    })
}

fn supported_call_law_v3(reconstructed: &ReconstructedActionV3) -> bool {
    let law = &reconstructed.law;
    let call_nodes = law
        .topology_nodes()
        .iter()
        .filter(|node| node.operation_code == Some(EFFECT_OPERATION_CALL_V3))
        .count();
    call_nodes == 1
        && !law.relation_program().is_empty()
        && law.relation_program().iter().all(|clause| {
            clause.relation_code == EFFECT_REL_COPY
                && clause.rhs.is_some()
                && clause.argument_ordinal.is_some()
                && clause.constant_type_code.is_none()
                && clause.constant_sha256.is_none()
        })
        && law
            .relation_program()
            .iter()
            .filter_map(|clause| clause.argument_ordinal)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == reconstructed.action.arguments().len()
}

fn reference_output_v3(action: &BoundProtocolActionV3) -> Option<String> {
    if action.capability_kind() != RuntimeCapabilityKindV3::Function {
        return None;
    }
    let mut arguments = BTreeMap::<String, Value>::new();
    for argument in action.arguments() {
        if arguments
            .insert(
                argument.physical_name().to_owned(),
                reference_value_v3(argument.value()),
            )
            .is_some()
        {
            return None;
        }
    }
    serde_json::to_string(&json!({
        "name": action.physical_symbol(),
        "arguments": arguments,
    }))
    .ok()
}

fn reference_value_v3(value: &BoundProtocolValueV3) -> Value {
    match value {
        BoundProtocolValueV3::String(value) | BoundProtocolValueV3::Identifier(value) => {
            Value::String(value.clone())
        }
        BoundProtocolValueV3::Integer(value) => Value::from(*value),
        BoundProtocolValueV3::Boolean(value) => Value::from(*value),
    }
}

fn blocked(
    preserved_frame_contract_sha256: String,
    verdict: IndependentVerifierVerdictV3,
) -> ReferenceVerificationV3 {
    ReferenceVerificationV3 {
        expected_output_sha256: None,
        effect_postcondition_sha256: None,
        preserved_frame_contract_sha256: Some(preserved_frame_contract_sha256),
        preserved_frame_observation_sha256: None,
        verdict,
    }
}
