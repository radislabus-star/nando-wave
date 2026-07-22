use std::collections::BTreeMap;

use nando_operator_kernel::{
    BindingValueTypeV1, BoundProtocolValueV3, CanonicalRuntimeRequestViewV3, RuntimeProjectionV3,
    StructuralExtractionBudgetV3, StructuralExtractionScopeV3,
    build_canonical_runtime_request_view_v3, canonical_json_sha256,
    canonicalize_runtime_structural_projection_v3, extract_structural_surface_v3, stable_atom_id,
};
use serde_json::Value;

use super::capability::{
    CapabilitySurfaceErrorV3, IndependentCapabilityV3, extract_capability_surface_v3,
};
use super::reconstruct::features_match_mode_v3;
use super::request_provenance::{RequestTextErrorV3, derive_request_text_v3};
use super::{
    IndependentVerifierBudgetV3, IndependentVerifierInputV3, IndependentVerifierVerdictV3,
};

pub(super) struct IndependentSurfaceV3 {
    pub request_view: CanonicalRuntimeRequestViewV3,
    pub role_values: BTreeMap<u16, Box<[BoundProtocolValueV3]>>,
    pub capabilities: Box<[IndependentCapabilityV3]>,
    pub raw_payload_sha256: String,
}

pub(super) enum SurfaceOutcomeV3 {
    Complete(Box<IndependentSurfaceV3>),
    Blocked(IndependentVerifierVerdictV3),
}

pub(super) fn extract_surface_v3(
    input: &IndependentVerifierInputV3<'_>,
    raw_payload_sha256: &str,
    budget: IndependentVerifierBudgetV3,
) -> SurfaceOutcomeV3 {
    if input.provider_payload_bytes().len() > budget.max_raw_request_bytes {
        return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::AbstainBudgetExhausted);
    }
    if !matches!(
        input.projection(),
        RuntimeProjectionV3::Responses | RuntimeProjectionV3::ChatCompletions
    ) {
        return SurfaceOutcomeV3::Blocked(
            IndependentVerifierVerdictV3::AbstainUnsupportedProjection,
        );
    }
    if raw_payload_sha256 != input.request_sha256() {
        return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
    }
    let Ok(payload) = serde_json::from_slice::<Value>(input.provider_payload_bytes()) else {
        return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
    };
    // Request text is evidence, not an actor-owned hint: derive it from the sealed bytes.
    let request_text =
        match derive_request_text_v3(&payload, input.projection(), budget.max_request_text_bytes) {
            Ok(value) => value,
            Err(RequestTextErrorV3::MissingOrInvalid) => {
                return SurfaceOutcomeV3::Blocked(
                    IndependentVerifierVerdictV3::RejectInvalidEvidence,
                );
            }
            Err(RequestTextErrorV3::BudgetExhausted) => {
                return SurfaceOutcomeV3::Blocked(
                    IndependentVerifierVerdictV3::AbstainBudgetExhausted,
                );
            }
        };
    let capability = match extract_capability_surface_v3(&payload, budget) {
        Ok(value) => value,
        Err(CapabilitySurfaceErrorV3::BudgetExhausted) => {
            return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::AbstainBudgetExhausted);
        }
        Err(CapabilitySurfaceErrorV3::Serialization) => {
            return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
        }
    };
    let Ok(extraction) = extract_structural_surface_v3(
        &request_text,
        &payload,
        capability.context.clone(),
        StructuralExtractionBudgetV3 {
            max_json_nodes: budget.max_json_nodes,
            max_text_bytes: budget.max_raw_request_bytes,
            max_recent_events: 32,
            max_role_candidates: budget.max_role_candidates,
            max_relations: budget.max_relations,
        },
        StructuralExtractionScopeV3::PreActionRuntime,
    ) else {
        return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
    };
    if extraction.candidate_budget_exhausted || extraction.relation_budget_exhausted {
        return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::AbstainBudgetExhausted);
    }
    let has_matching_role = input
        .artifact_set()
        .artifacts()
        .iter()
        .flat_map(|entry| entry.artifact().source_mode_set().modes.iter())
        .any(|mode| {
            extraction
                .candidates
                .iter()
                .any(|candidate| features_match_mode_v3(&candidate.features, mode))
        });
    if !has_matching_role {
        return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::AbstainMissingRole);
    }
    let Ok((structural, source_bindings)) =
        canonicalize_runtime_structural_projection_v3(capability.context, &extraction)
    else {
        return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
    };
    let Ok(request_view) = build_canonical_runtime_request_view_v3(
        input.projection(),
        request_phase_atoms_v3(&request_text),
        structural,
        capability.descriptors,
    ) else {
        return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
    };
    let mut role_values = BTreeMap::new();
    for binding in source_bindings {
        let Some(candidate) = extraction
            .candidates
            .iter()
            .find(|candidate| candidate.source_role_id == binding.source_role_id)
        else {
            return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
        };
        let Some(mut values) = candidate
            .normalized_values
            .iter()
            .map(|value| typed_value_v3(value, candidate.features.value_type))
            .collect::<Option<Vec<_>>>()
        else {
            return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
        };
        values.sort();
        values.dedup();
        if role_values
            .insert(binding.canonical_role_id, values.into_boxed_slice())
            .is_some()
        {
            return SurfaceOutcomeV3::Blocked(IndependentVerifierVerdictV3::RejectInvalidEvidence);
        }
    }
    SurfaceOutcomeV3::Complete(Box::new(IndependentSurfaceV3 {
        request_view,
        role_values,
        capabilities: capability.capabilities.into_boxed_slice(),
        raw_payload_sha256: raw_payload_sha256.to_owned(),
    }))
}

fn typed_value_v3(value: &str, value_type: BindingValueTypeV1) -> Option<BoundProtocolValueV3> {
    match value_type {
        BindingValueTypeV1::String => Some(BoundProtocolValueV3::String(value.to_owned())),
        BindingValueTypeV1::Identifier => Some(BoundProtocolValueV3::Identifier(value.to_owned())),
        BindingValueTypeV1::Integer => value.parse().ok().map(BoundProtocolValueV3::Integer),
        BindingValueTypeV1::Boolean => value.parse().ok().map(BoundProtocolValueV3::Boolean),
    }
}

fn request_phase_atoms_v3(text: &str) -> Vec<u64> {
    let all_tokens = text
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty() && token.len() <= 32)
        .map(str::to_lowercase)
        .take(256)
        .collect::<Vec<_>>();
    let tokens = if all_tokens.len() <= 64 {
        all_tokens
    } else {
        all_tokens[..32]
            .iter()
            .chain(&all_tokens[all_tokens.len().saturating_sub(32)..])
            .cloned()
            .collect()
    };
    let mut atoms = tokens
        .iter()
        .map(|token| stable_atom_id(&format!("request_token:{token}")))
        .collect::<Vec<_>>();
    atoms.extend(
        tokens
            .windows(2)
            .map(|pair| stable_atom_id(&format!("request_bigram:{}:{}", pair[0], pair[1]))),
    );
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

pub(super) fn role_value_root_v3(
    role_id: u16,
    values: &[BoundProtocolValueV3],
) -> Result<String, ()> {
    canonical_json_sha256(&("nando.f6.role-value.v3", role_id, values)).map_err(|_| ())
}
