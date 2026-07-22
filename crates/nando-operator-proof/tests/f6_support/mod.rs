use nando_operator_kernel::{
    BindingCompletionStateV1, BindingPredicateV1, BindingProtocolCompileVerdictV2,
    BindingRequestRelationV1, BindingValueTypeV1, BoundProtocolActionInputV3,
    BoundProtocolActionV3, BoundProtocolArgumentInputV3, BoundProtocolValueV3,
    BoundedProtocolModeCandidateV2, CANONICAL_EFFECT_LAW_SCHEMA_V3, CanonicalRelationClauseV3,
    EFFECT_ATOM_ACTION_RELATION, EFFECT_ATOM_PRECONDITION, EFFECT_LAW_IR_VERSION_V3,
    EFFECT_OPERATION_CALL_V3, EFFECT_REL_COPY, EFFECT_VALUE_OPERATION_V3, EFFECT_VALUE_STRING_V3,
    ExecutableProtocolModeArtifactV3, PROTOCOL_MODE_SET_SCHEMA_V2, ProtocolArgumentRoleSchemaV2,
    ProtocolArgumentRoleV2, ProtocolCapabilityContractV2, ProtocolCapabilityKindV3,
    ProtocolConstantContractV2, ProtocolModeProgramV2, ProtocolModeSetV2,
    ProtocolRoleCardinalityV2, ProtocolSelectorProgramV2, ProtocolSourceRoleSchemaV2,
    ProtocolSourceRoleV2, ProtocolStructuralGuardV2, ProtocolTemporalCardinalityContractV2,
    ProtocolValueContractV2, RuntimeProjectionV3, build_bound_protocol_action_v3,
    build_executable_protocol_mode_artifact_v3, build_executable_protocol_mode_v3,
    build_protocol_facet_payload_v3, canonical_json_sha256, derived_mode_root_v2,
    protocol_mode_from_candidate_v2, protocol_mode_set_digest_v2, sha256_bytes,
};
use nando_operator_runtime::{
    CapabilityGroundingVerdictV3, OperatorShadowVerdictV3, RuntimeContextBudgetV3,
    StructuralBindingVerdictV3, StructuralDispatchVerdictV3, bind_structural_modes_v3,
    compile_structural_dispatch_index_v3, execute_bound_protocol_shadow_v3,
    extract_canonical_runtime_request_v3, ground_protocol_actions_v3,
};
use serde_json::{Value, json};

pub struct F5HandoffV3 {
    pub artifacts: Vec<ExecutableProtocolModeArtifactV3>,
    pub payload: Value,
    pub payload_bytes: Vec<u8>,
    pub request_sha256: String,
    pub action: BoundProtocolActionV3,
    pub actor_output: String,
}

pub enum ActionMutationV3 {
    SourceRole,
    Value,
    Capability,
}

pub fn handoff_v3(symbol: &str, argument: &str, value: &str, seeds: &[u16]) -> F5HandoffV3 {
    let request_text = format!("continue {value}");
    let payload = request_payload_v3(symbol, argument, value);
    let artifacts = seeds
        .iter()
        .map(|seed| artifact_v3(*seed))
        .collect::<Vec<_>>();
    finish_handoff_v3(artifacts, request_text, payload)
}

pub fn handoff_with_effect_operation_v3(operation_code: u16) -> F5HandoffV3 {
    let request_text = "continue CellA17".to_owned();
    let payload = request_payload_v3("continue_session", "handle", "CellA17");
    finish_handoff_v3(
        vec![artifact_with_operation_v3(10, operation_code)],
        request_text,
        payload,
    )
}

pub fn finish_handoff_v3(
    artifacts: Vec<ExecutableProtocolModeArtifactV3>,
    request_text: String,
    payload: Value,
) -> F5HandoffV3 {
    let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
    let request_sha256 = sha256_bytes(&payload_bytes);
    let extraction = extract_canonical_runtime_request_v3(
        &request_sha256,
        &request_text,
        RuntimeProjectionV3::Responses,
        &payload,
        RuntimeContextBudgetV3::default(),
    )
    .expect("F5 extraction");
    let request = extraction.into_context().expect("F5 request context");
    let index = compile_structural_dispatch_index_v3(&artifacts).expect("F5 index");
    let dispatch = index.dispatch(&request);
    assert_eq!(dispatch.verdict(), StructuralDispatchVerdictV3::Complete);
    let binding = bind_structural_modes_v3(&index, &request, &dispatch);
    assert_eq!(binding.verdict(), StructuralBindingVerdictV3::Complete);
    let grounded = ground_protocol_actions_v3(
        &index,
        &request,
        &binding.into_complete().expect("F5 complete binding"),
    );
    assert_eq!(grounded.verdict(), CapabilityGroundingVerdictV3::Complete);
    let action = grounded
        .into_complete()
        .expect("F5 complete action")
        .action()
        .clone();
    let shadow = execute_bound_protocol_shadow_v3(&action);
    assert_eq!(
        shadow.receipt().verdict(),
        OperatorShadowVerdictV3::Complete
    );
    let actor_output = shadow.actor_output().expect("F5 actor output").to_owned();
    F5HandoffV3 {
        artifacts,
        payload,
        payload_bytes,
        request_sha256,
        action,
        actor_output,
    }
}

pub fn mutate_action_v3(
    action: &BoundProtocolActionV3,
    mutation: ActionMutationV3,
) -> BoundProtocolActionV3 {
    let arguments = action
        .arguments()
        .iter()
        .map(|argument| BoundProtocolArgumentInputV3 {
            argument_ordinal: argument.argument_ordinal(),
            source_role_id: if matches!(mutation, ActionMutationV3::SourceRole) {
                argument.source_role_id().saturating_add(1)
            } else {
                argument.source_role_id()
            },
            physical_name: argument.physical_name().to_owned(),
            value: if matches!(mutation, ActionMutationV3::Value) {
                BoundProtocolValueV3::String("mutated-value".to_owned())
            } else {
                argument.value().clone()
            },
        })
        .collect();
    build_bound_protocol_action_v3(BoundProtocolActionInputV3 {
        index_sha256: action.index_sha256().to_owned(),
        artifact_root_sha256: action.artifact_root_sha256().to_owned(),
        mode_id_sha256: action.mode_id_sha256().to_owned(),
        executable_mode_root_sha256: action.executable_mode_root_sha256().to_owned(),
        payload_root_sha256: action.payload_root_sha256().to_owned(),
        effect_law_id_sha256: action.effect_law_id_sha256().to_owned(),
        action_class_root_sha256: action.action_class_root_sha256().to_owned(),
        request_view_sha256: action.request_view_sha256().to_owned(),
        mapping_sha256: action.mapping_sha256().to_owned(),
        capability_id: action.capability_id(),
        capability_kind: action.capability_kind(),
        physical_symbol: if matches!(mutation, ActionMutationV3::Capability) {
            "mutated_capability".to_owned()
        } else {
            action.physical_symbol().to_owned()
        },
        arguments,
    })
    .expect("mutated opaque action")
}

pub fn request_payload_v3(symbol: &str, argument: &str, value: &str) -> Value {
    json!({
        "model": "ignored-model",
        "tools": [{
            "type": "function",
            "name": symbol,
            "parameters": {
                "type": "object",
                "properties": {argument: {"type": "string"}},
                "required": [argument]
            }
        }],
        "input": [
            {"type": "message", "role": "user", "content": format!("continue {value}")},
            {"type": "function_call", "name": symbol, "call_id": "call-1"},
            {"type": "function_call_output", "call_id": "call-1", "output": {argument: value}}
        ]
    })
}

fn artifact_v3(seed: u16) -> ExecutableProtocolModeArtifactV3 {
    artifact_with_operation_v3(seed, EFFECT_OPERATION_CALL_V3)
}

fn artifact_with_operation_v3(seed: u16, operation_code: u16) -> ExecutableProtocolModeArtifactV3 {
    let invariant = root_v3("f6-invariant");
    let preserved_frame = root_v3("f6-preserved-frame");
    let relation_program = vec![CanonicalRelationClauseV3 {
        relation_code: EFFECT_REL_COPY,
        lhs: 0,
        rhs: Some(1),
        argument_ordinal: Some(0),
        constant_type_code: None,
        constant_sha256: None,
    }];
    let action_class = canonical_json_sha256(&(&relation_program, &invariant, &preserved_frame))
        .expect("action class");
    let law_payload = json!({
        "schema": CANONICAL_EFFECT_LAW_SCHEMA_V3,
        "ir_version": EFFECT_LAW_IR_VERSION_V3,
        "dictionary_root_sha256": root_v3("f6-dictionary"),
        "quotient_hypothesis_root_sha256": root_v3("f6-quotient"),
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
                "operation_code": operation_code
            }
        ],
        "topology_edges": [{"from": 0, "to": 1, "relation_code": EFFECT_REL_COPY}],
        "relation_program": relation_program,
        "effect_invariant_root_sha256": invariant,
        "preserved_frame_root_sha256": preserved_frame,
        "action_equivalence_root_sha256": action_class
    });
    let effect_law_id = canonical_json_sha256(&law_payload).expect("effect law id");
    let mut predicates = vec![
        BindingPredicateV1::TemporalDistance { value: 0 },
        BindingPredicateV1::ValueType {
            value: BindingValueTypeV1::String,
        },
        BindingPredicateV1::RequestRelation {
            value: BindingRequestRelationV1::Mentioned,
        },
    ];
    predicates.sort();
    let selector_program = ProtocolSelectorProgramV2 {
        predicates,
        max_action_classes: 1,
    };
    let selector_root =
        derived_mode_root_v2("selector-program", &selector_program).expect("selector root");
    let facet_root = root_v3(&format!("f6-facet-{seed}"));
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
            physical_program_ids_sha256: vec![root_v3(&format!("f6-physical-{seed}"))],
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
            relation_identity_sha256: root_v3("f6-relation"),
            effect_invariant_root_sha256: invariant.clone(),
            selector_program_root_sha256: selector_root,
        },
        temporal_cardinality_contract: ProtocolTemporalCardinalityContractV2 {
            completion_states: vec![BindingCompletionStateV1::Completed],
            temporal_distances: vec![0],
            event_candidate_cardinalities: (0..=64).collect(),
            require_unique_action_class: true,
        },
    };
    let candidate = candidate_v3(seed, effect_law_id.clone(), action_class, program);
    let row = root_v3(&format!("f6-row-{seed}"));
    let mode = protocol_mode_from_candidate_v2(&candidate, [row].into_iter().collect())
        .expect("protocol mode");
    let mut mode_set = ProtocolModeSetV2 {
        schema: PROTOCOL_MODE_SET_SCHEMA_V2.to_owned(),
        mode_set_sha256: String::new(),
        verdict: BindingProtocolCompileVerdictV2::ProtocolModeSet,
        binding_capability_root_sha256: root_v3(&format!("f6-binding-{seed}")),
        effect_law_id_sha256: effect_law_id,
        relation_identity_sha256: root_v3("f6-relation"),
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
            .expect("facet");
    let executable = build_executable_protocol_mode_v3(&mode, facet).expect("executable");
    build_executable_protocol_mode_artifact_v3(&mode_set, law_payload, vec![executable])
        .expect("artifact")
}

fn candidate_v3(
    seed: u16,
    effect_law_id_sha256: String,
    action_class_root_sha256: String,
    program: ProtocolModeProgramV2,
) -> BoundedProtocolModeCandidateV2 {
    BoundedProtocolModeCandidateV2 {
        candidate_id_sha256: root_v3(&format!("f6-candidate-{seed}")),
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
        .expect("source roles"),
        selector_program_root_sha256: derived_mode_root_v2(
            "selector-program",
            &program.selector_program,
        )
        .expect("selector"),
        observed_emitted_types_root_sha256: derived_mode_root_v2(
            "observed-emitted-types",
            &program.value_contract,
        )
        .expect("value contract"),
        capability_protocol_root_sha256: derived_mode_root_v2(
            "capability-protocol",
            &program.capability_contract,
        )
        .expect("capability"),
        argument_role_schema_root_sha256: derived_mode_root_v2(
            "argument-role-schema",
            &program.argument_role_schema,
        )
        .expect("arguments"),
        constant_contract_root_sha256: derived_mode_root_v2(
            "constant-contract",
            &program.constant_contract,
        )
        .expect("constants"),
        structural_guard_root_sha256: derived_mode_root_v2(
            "structural-guard",
            &program.structural_guard,
        )
        .expect("guard"),
        temporal_cardinality_contract_root_sha256: derived_mode_root_v2(
            "temporal-cardinality",
            &program.temporal_cardinality_contract,
        )
        .expect("temporal"),
        action_class_root_sha256,
        program,
        covers_positive_rows_sha256: vec![root_v3(&format!("f6-row-{seed}"))],
        accepts_negative_rows_sha256: Vec::new(),
        wrong_action_rows_sha256: Vec::new(),
        verify_failed_rows_sha256: Vec::new(),
        search_exhausted: false,
    }
}

fn root_v3(label: &str) -> String {
    canonical_json_sha256(&label).expect("fixture root")
}
