use nando_operator_kernel::{
    BindingCompletionStateV1, BindingPredicateV1, BindingProtocolCompileVerdictV2,
    BindingRequestRelationV1, BindingValueTypeV1, BoundedProtocolModeCandidateV2,
    CANONICAL_EFFECT_LAW_SCHEMA_V3, EFFECT_ATOM_ACTION_RELATION, EFFECT_ATOM_PRECONDITION,
    EFFECT_LAW_IR_VERSION_V3, EFFECT_OPERATION_CALL_V3, EFFECT_REL_COPY, EFFECT_VALUE_OPERATION_V3,
    EFFECT_VALUE_STRING_V3, ExecutableProtocolModeArtifactV3, PROTOCOL_MODE_SET_SCHEMA_V2,
    ProtocolArgumentRoleSchemaV2, ProtocolArgumentRoleV2, ProtocolCapabilityContractV2,
    ProtocolCapabilityKindV3, ProtocolConstantContractV2, ProtocolModeProgramV2, ProtocolModeSetV2,
    ProtocolRoleCardinalityV2, ProtocolSelectorProgramV2, ProtocolSourceRoleSchemaV2,
    ProtocolSourceRoleV2, ProtocolStructuralGuardV2, ProtocolTemporalCardinalityContractV2,
    ProtocolValueContractV2, build_executable_protocol_mode_artifact_v3,
    build_executable_protocol_mode_v3, build_protocol_facet_payload_v3, canonical_json_sha256,
    derived_mode_root_v2, protocol_mode_from_candidate_v2, protocol_mode_set_digest_v2,
    sha256_bytes,
};
use serde_json::{Value, json};

use crate::{
    CanonicalRuntimeRequestV3, RuntimeContextBudgetV3, extract_canonical_runtime_request_v3,
};
use nando_operator_kernel::RuntimeProjectionV3;

pub(crate) fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("fixture root")
}

pub(crate) fn request_payload(output: Value) -> Value {
    json!({
        "model": "ignored-model",
        "tools": [{
            "type": "function",
            "name": "renamable_capability",
            "parameters": {
                "type": "object",
                "properties": {"handle": {"type": "string"}},
                "required": ["handle"]
            }
        }],
        "input": [
            {"type": "message", "role": "user", "content": "continue CellA17"},
            {"type": "function_call", "name": "renamable_capability", "call_id": "call-1"},
            {"type": "function_call_output", "call_id": "call-1", "output": output}
        ]
    })
}

pub(crate) fn runtime_context<'a>(
    request_text: &str,
    payload: &'a Value,
) -> CanonicalRuntimeRequestV3<'a> {
    let request_sha256 = sha256_bytes(
        serde_json::to_vec(payload)
            .expect("request fixture bytes")
            .as_slice(),
    );
    extract_canonical_runtime_request_v3(
        &request_sha256,
        request_text,
        RuntimeProjectionV3::Responses,
        payload,
        RuntimeContextBudgetV3::default(),
    )
    .expect("runtime extraction")
    .into_context()
    .expect("complete runtime context")
}

pub(crate) fn artifact(
    seed: u16,
    mut predicates: Vec<BindingPredicateV1>,
) -> ExecutableProtocolModeArtifactV3 {
    predicates.sort();
    let invariant = root("f5c-invariant");
    let action_class = root("f5c-action-class");
    let law_payload = json!({
        "schema": CANONICAL_EFFECT_LAW_SCHEMA_V3,
        "ir_version": EFFECT_LAW_IR_VERSION_V3,
        "dictionary_root_sha256": root("f5c-dictionary"),
        "quotient_hypothesis_root_sha256": root("f5c-quotient"),
        "topology_nodes": [
            {
                "canonical_node": 0,
                "source": "observation",
                "node_kind_code": EFFECT_ATOM_PRECONDITION,
                "value_type_code": EFFECT_VALUE_STRING_V3,
                "unique": true,
                "operation_code": null
            },
            {
                "canonical_node": 1,
                "source": "action",
                "node_kind_code": EFFECT_ATOM_ACTION_RELATION,
                "value_type_code": EFFECT_VALUE_OPERATION_V3,
                "unique": true,
                "operation_code": EFFECT_OPERATION_CALL_V3
            }
        ],
        "topology_edges": [{"from": 0, "to": 1, "relation_code": EFFECT_REL_COPY}],
        "relation_program": [{
            "relation_code": EFFECT_REL_COPY,
            "lhs": 0,
            "rhs": 1,
            "argument_ordinal": 0,
            "constant_type_code": null,
            "constant_sha256": null
        }],
        "effect_invariant_root_sha256": invariant,
        "preserved_frame_root_sha256": root("f5c-frame"),
        "action_equivalence_root_sha256": action_class
    });
    let effect_law_id = canonical_json_sha256(&law_payload).expect("effect law id");
    let selector_program = ProtocolSelectorProgramV2 {
        predicates,
        max_action_classes: 1,
    };
    let selector_root =
        derived_mode_root_v2("selector-program", &selector_program).expect("selector root");
    let facet_root = root(&format!("f5c-facet-{seed}"));
    let program = ProtocolModeProgramV2 {
        source_role_schema: ProtocolSourceRoleSchemaV2 {
            roles: vec![ProtocolSourceRoleV2 {
                role_id: 0,
                value_type: BindingValueTypeV1::String,
                cardinality: ProtocolRoleCardinalityV2::OneActionClass,
            }],
        },
        selector_program,
        value_contract: ProtocolValueContractV2 {
            observed: BindingValueTypeV1::String,
            emitted: BindingValueTypeV1::String,
        },
        capability_contract: ProtocolCapabilityContractV2 {
            protocol_facet_root_sha256: facet_root.clone(),
            physical_program_ids_sha256: vec![root(&format!("f5c-physical-{seed}"))],
        },
        argument_role_schema: ProtocolArgumentRoleSchemaV2 {
            roles: vec![ProtocolArgumentRoleV2 {
                argument_ordinal: 0,
                source_role_id: 0,
            }],
        },
        constant_contract: ProtocolConstantContractV2 {
            semantic_constants_sha256: Vec::new(),
            protocol_noop_constants_sha256: Vec::new(),
            execution_budget_roots_sha256: Vec::new(),
            transport_default_roots_sha256: Vec::new(),
        },
        structural_guard: ProtocolStructuralGuardV2 {
            relation_identity_sha256: root("f5c-relation"),
            effect_invariant_root_sha256: invariant.clone(),
            selector_program_root_sha256: selector_root,
        },
        temporal_cardinality_contract: ProtocolTemporalCardinalityContractV2 {
            completion_states: vec![BindingCompletionStateV1::Completed],
            temporal_distances: vec![0],
            event_candidate_cardinalities: vec![1],
            require_unique_action_class: true,
        },
    };
    let candidate = candidate(seed, effect_law_id.clone(), action_class, program);
    let row = root(&format!("f5c-row-{seed}"));
    let mode = protocol_mode_from_candidate_v2(&candidate, [row].into_iter().collect())
        .expect("protocol mode");
    let mut mode_set = ProtocolModeSetV2 {
        schema: PROTOCOL_MODE_SET_SCHEMA_V2.to_owned(),
        mode_set_sha256: String::new(),
        verdict: BindingProtocolCompileVerdictV2::ProtocolModeSet,
        binding_capability_root_sha256: root(&format!("f5c-binding-{seed}")),
        effect_law_id_sha256: effect_law_id,
        relation_identity_sha256: root("f5c-relation"),
        modes: vec![mode.clone()],
        positive_rows: 1,
        positive_rows_covered: 1,
        wrong_actions: 0,
        verify_failed: 0,
        negative_accepts: 0,
        search_exhausted: false,
        action_equivalence_classes: 1,
        all_surviving_covers_action_equivalent: true,
        production_admissible: false,
        execution_authority: false,
    };
    mode_set.mode_set_sha256 = protocol_mode_set_digest_v2(&mode_set).expect("mode set root");
    let facet =
        build_protocol_facet_payload_v3(facet_root, ProtocolCapabilityKindV3::Function, &mode)
            .expect("facet payload");
    let executable = build_executable_protocol_mode_v3(&mode, facet).expect("executable mode");
    build_executable_protocol_mode_artifact_v3(&mode_set, law_payload, vec![executable])
        .expect("executable artifact")
}

fn candidate(
    seed: u16,
    effect_law_id_sha256: String,
    action_class_root_sha256: String,
    program: ProtocolModeProgramV2,
) -> BoundedProtocolModeCandidateV2 {
    BoundedProtocolModeCandidateV2 {
        candidate_id_sha256: root(&format!("f5c-candidate-{seed}")),
        effect_law_id_sha256,
        relation_identity_sha256: program.structural_guard.relation_identity_sha256.clone(),
        protocol_facet_root_sha256: program
            .capability_contract
            .protocol_facet_root_sha256
            .clone(),
        effect_invariant_root_sha256: program
            .structural_guard
            .effect_invariant_root_sha256
            .clone(),
        source_role_schema_root_sha256: derived_mode_root_v2(
            "source-role-schema",
            &program.source_role_schema,
        )
        .expect("source role root"),
        selector_program_root_sha256: derived_mode_root_v2(
            "selector-program",
            &program.selector_program,
        )
        .expect("selector root"),
        observed_emitted_types_root_sha256: derived_mode_root_v2(
            "observed-emitted-types",
            &program.value_contract,
        )
        .expect("value root"),
        capability_protocol_root_sha256: derived_mode_root_v2(
            "capability-protocol",
            &program.capability_contract,
        )
        .expect("capability root"),
        argument_role_schema_root_sha256: derived_mode_root_v2(
            "argument-role-schema",
            &program.argument_role_schema,
        )
        .expect("argument root"),
        constant_contract_root_sha256: derived_mode_root_v2(
            "constant-contract",
            &program.constant_contract,
        )
        .expect("constant root"),
        structural_guard_root_sha256: derived_mode_root_v2(
            "structural-guard",
            &program.structural_guard,
        )
        .expect("guard root"),
        temporal_cardinality_contract_root_sha256: derived_mode_root_v2(
            "temporal-cardinality",
            &program.temporal_cardinality_contract,
        )
        .expect("temporal root"),
        action_class_root_sha256,
        program,
        covers_positive_rows_sha256: vec![root(&format!("f5c-row-{seed}"))],
        accepts_negative_rows_sha256: Vec::new(),
        wrong_action_rows_sha256: Vec::new(),
        verify_failed_rows_sha256: Vec::new(),
        search_exhausted: false,
    }
}

pub(crate) fn mentioned_string_selector() -> Vec<BindingPredicateV1> {
    vec![
        BindingPredicateV1::ValueType {
            value: BindingValueTypeV1::String,
        },
        BindingPredicateV1::RequestRelation {
            value: BindingRequestRelationV1::Mentioned,
        },
    ]
}
