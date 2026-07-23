use std::collections::BTreeMap;

use nando_operator_kernel::{
    OperatorGenerationComponentRootsV3, ProgramSemanticClassInputV1, canonical_json_sha256,
    seal_operator_generation_manifest_v3, seal_program_semantic_class_v1, valid_nonzero_sha256,
};
use nando_operator_learning::{
    ExactProgramEvaluation, GenerationLearningOutcomeV3, OperatorIdentificationMachineV1,
    OperatorIdentificationStateV1, OperatorObservationInputV1, VersionSpaceConfig,
    seal_operator_observation_v1,
};

use super::induction::{
    program_transform_flags, program_transform_opcode, rich_scalar_program_roles,
    selector_value_type,
};
use super::state::{response_operation_kind, source_neutral_actor_topology};
use super::*;

pub(super) struct LiveScalarIdentificationV1 {
    pub freeze_root_sha256: String,
    pub semantic_class_id_sha256: String,
    pub canonical_program_root_sha256: String,
    pub applicability_scope_root_sha256: String,
}

pub(super) fn identify_live_scalar_law_v1(
    law_key: &str,
    law: &LiveScalarLawState,
) -> Result<LiveScalarIdentificationV1, String> {
    let first_transition = law
        .support
        .first()
        .ok_or_else(|| "identification_support_missing".to_owned())?;
    let first_parity = first_transition
        .runtime_parity_case
        .as_ref()
        .ok_or_else(|| "identification_parity_missing".to_owned())?;
    if law.support_actor_hypotheses.is_empty() {
        return Err("identification_hypotheses_missing".to_owned());
    }

    let candidate_set_root = canonical_json_sha256(&(
        "nando.live-scalar-identification-candidates.v1",
        law_key,
        &law.support_actor_hypotheses,
    ))
    .map_err(|_| "identification_candidate_root_failed".to_owned())?;
    let verifier_set_root = law
        .support_actor_hypotheses
        .iter()
        .map(|program| {
            source_neutral_verifier_for_program(program)
                .map_err(|error| format!("identification_verifier_build:{error}"))
                .and_then(|verifier| {
                    response_independent_verifier_program_digest(&verifier)
                        .map_err(|error| format!("identification_verifier_digest:{error}"))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: candidate_set_root.clone(),
            dispatch_index_sha256: canonical_json_sha256(&(
                "nando.live-scalar-identification-dispatch.v1",
                law_key,
            ))
            .map_err(|_| "identification_dispatch_root_failed".to_owned())?,
            actor_program_sha256: candidate_set_root,
            renderer_program_sha256: canonical_json_sha256(&(
                "nando.live-scalar-identification-renderer.v1",
                &law.support_actor_hypotheses,
            ))
            .map_err(|_| "identification_renderer_root_failed".to_owned())?,
            verifier_contract_sha256: canonical_json_sha256(&verifier_set_root)
                .map_err(|_| "identification_verifier_root_failed".to_owned())?,
            capability_contract_sha256: canonical_json_sha256(&(
                "nando.live-scalar-identification-capability.v1",
                first_transition.outcome.action.action_symbol.as_str(),
            ))
            .map_err(|_| "identification_capability_root_failed".to_owned())?,
            resource_budget_sha256: canonical_json_sha256(&(
                "nando.live-scalar-identification-budget.v1",
                LIVE_SCALAR_MAX_EVIDENCE_ROWS,
                TEACHER_CALL_SELECTOR_BUDGET,
                COMMON_ACTOR_TOPOLOGY_BUDGET,
            ))
            .map_err(|_| "identification_budget_root_failed".to_owned())?,
        },
    )
    .map_err(|error| format!("identification_manifest:{error:?}").to_lowercase())?;

    let mut machine = OperatorIdentificationMachineV1::new(manifest, VersionSpaceConfig::default());
    let mut candidates = BTreeMap::new();
    for program in &law.support_actor_hypotheses {
        let (topology, canonical) = source_neutral_actor_topology(
            program,
            &first_parity.request_text,
            &first_parity.provider_payload,
        )
        .ok_or_else(|| "identification_topology_missing".to_owned())?;
        let roles = rich_scalar_program_roles(&canonical)
            .ok_or_else(|| "identification_roles_missing".to_owned())?;
        let semantic_roles = roles
            .iter()
            .map(|(selector, format)| (selector_value_type(selector), *format))
            .collect::<Vec<_>>();
        let descriptor = seal_program_semantic_class_v1(ProgramSemanticClassInputV1 {
            effect_law_id_sha256: canonical_json_sha256(&(
                "nando.live-scalar-effect.v1",
                law_key,
                program_transform_opcode(&canonical),
                program_transform_flags(&canonical),
            ))
            .map_err(|_| "identification_effect_root_failed".to_owned())?,
            role_schema_root_sha256: canonical_json_sha256(&(
                "nando.live-scalar-role-schema.v1",
                &semantic_roles,
            ))
            .map_err(|_| "identification_role_root_failed".to_owned())?,
            protocol_mode_set_root_sha256: canonical_json_sha256(&(
                "nando.live-scalar-protocol-modes.v1",
                response_operation_kind(&canonical),
                first_transition.outcome.action.action_symbol.as_str(),
            ))
            .map_err(|_| "identification_protocol_root_failed".to_owned())?,
            executable_behavior_root_sha256: canonical_json_sha256(&(
                "nando.live-scalar-executable-behavior.v1",
                topology,
            ))
            .map_err(|_| "identification_behavior_root_failed".to_owned())?,
            // Physical selectors are protocol modes. The semantic class owns
            // the independently checked effect contract, not one adapter.
            verifier_contract_root_sha256: canonical_json_sha256(&(
                "nando.live-scalar-semantic-verifier-contract.v1",
                response_operation_kind(&canonical),
                crate::response_program_external_verifier_schema(&canonical),
                program_transform_opcode(&canonical),
                program_transform_flags(&canonical),
            ))
            .map_err(|_| "identification_semantic_verifier_root_failed".to_owned())?,
        })
        .map_err(|error| format!("identification_descriptor:{error}"))?;
        let digest = machine
            .register_candidate(canonical.clone(), descriptor)
            .map_err(|error| format!("identification_register:{error}"))?;
        candidates.insert(digest, canonical);
    }
    machine.complete_candidate_generation();

    let mut ordered_support = law.support.iter().collect::<Vec<_>>();
    ordered_support.sort_by(|left, right| {
        left.before
            .observed_at_unix_nanos
            .cmp(&right.before.observed_at_unix_nanos)
            .then_with(|| {
                left.before
                    .frame_id_sha256
                    .cmp(&right.before.frame_id_sha256)
            })
    });
    for (index, transition) in ordered_support.into_iter().enumerate() {
        let parity = transition
            .runtime_parity_case
            .as_ref()
            .ok_or_else(|| "identification_parity_missing".to_owned())?;
        let provider_view =
            crate::runtime::provider_payload_view(&parity.request_text, &parity.provider_payload)
                .map_err(|error| format!("identification_provider_view:{error}"))?;
        let evaluations = candidates
            .iter()
            .map(|(digest, program)| {
                let response =
                    execute_response(program, &parity.request_text, provider_view.as_ref()).response;
                let accepted = response.as_deref().is_some_and(|response| {
                    response == parity.expected_response
                        || crate::online_admission::responses_match_after_execution_budget_normalization(
                            response,
                            &parity.expected_response,
                        )
                });
                ExactProgramEvaluation {
                    program_digest_sha256: digest.clone(),
                    accepted,
                    reason: if accepted {
                        String::new()
                    } else {
                        "runtime_parity_mismatch".to_owned()
                    },
                }
            })
            .collect();
        let observation = seal_operator_observation_v1(OperatorObservationInputV1 {
            capture_sequence: u64::try_from(index.saturating_add(1)).unwrap_or(u64::MAX),
            lineage_root_sha256: root_or_commitment(
                &transition.before.session_id_sha256,
                "lineage",
            )?,
            event_root_sha256: unique_observation_root(
                &transition.before.event_id_sha256,
                &transition.before.frame_id_sha256,
                "event",
            )?,
            request_root_sha256: unique_observation_root(
                &transition.before.client_intent_id_sha256,
                &transition.before.frame_id_sha256,
                "request",
            )?,
            pre_action_relation_root_sha256: root_or_commitment(
                &transition.before.frame_id_sha256,
                "before",
            )?,
            observed_action_root_sha256: root_or_commitment(
                &transition.outcome.action.signature_sha256,
                "action",
            )?,
            observed_delta_root_sha256: canonical_json_sha256(&(
                "nando.live-scalar-observed-delta.v1",
                &parity.provider_payload,
                parity.expected_response.as_str(),
            ))
            .map_err(|_| "identification_delta_root_failed".to_owned())?,
            verifier_receipt_root_sha256: unique_observation_root(
                &transition.outcome.verifier.evidence_ref_sha256,
                &transition.before.frame_id_sha256,
                "verifier",
            )?,
            outcome: GenerationLearningOutcomeV3::VerifiedPass,
            evaluations,
        })
        .map_err(|error| format!("identification_observation:{error}"))?;
        machine
            .apply_support(observation)
            .map_err(|error| format!("identification_support:{error:?}"))?;
    }

    let identified = match machine
        .state()
        .map_err(|error| format!("identification_state:{error}"))?
    {
        OperatorIdentificationStateV1::Identified { class } => class,
        OperatorIdentificationStateV1::Ambiguous { report } => {
            return Err(format!(
                "identification_ambiguous:classes={}:programs={}",
                report.surviving_semantic_classes, report.surviving_programs
            ));
        }
        other => return Err(format!("identification_not_ready:{other:?}").to_lowercase()),
    };
    let semantic_class_id_sha256 = identified.semantic_class().class_id().as_str().to_owned();
    let canonical_program_root_sha256 = identified.canonical_program_root_sha256().to_owned();
    let scope_root_sha256 = canonical_json_sha256(&(
        "nando.live-scalar-applicability-scope.v1",
        law_key,
        semantic_class_id_sha256.as_str(),
        canonical_program_root_sha256.as_str(),
    ))
    .map_err(|_| "identification_scope_root_failed".to_owned())?;
    let freeze = machine
        .freeze_candidate(
            u64::try_from(law.support.len().saturating_add(1)).unwrap_or(u64::MAX),
            scope_root_sha256.clone(),
        )
        .map_err(|error| format!("identification_freeze:{error}"))?;
    Ok(LiveScalarIdentificationV1 {
        freeze_root_sha256: freeze.freeze_root_sha256().to_owned(),
        semantic_class_id_sha256,
        canonical_program_root_sha256,
        applicability_scope_root_sha256: scope_root_sha256,
    })
}

fn root_or_commitment(value: &str, domain: &str) -> Result<String, String> {
    if valid_nonzero_sha256(value) {
        return Ok(value.to_owned());
    }
    canonical_json_sha256(&("nando.live-scalar-observation-root.v1", domain, value))
        .map_err(|_| format!("identification_{domain}_root_failed"))
}

fn unique_observation_root(value: &str, frame_id: &str, domain: &str) -> Result<String, String> {
    canonical_json_sha256(&(
        "nando.live-scalar-unique-observation-root.v1",
        domain,
        value,
        frame_id,
    ))
    .map_err(|_| format!("identification_{domain}_root_failed"))
}
