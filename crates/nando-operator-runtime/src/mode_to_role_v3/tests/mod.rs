mod fixtures;

use nando_operator_kernel::{
    BindingCallLineageV1, BindingPredicateV1, BindingRequestRelationV1, BindingValueTypeV1,
    ExecutableProtocolModeArtifactV3,
};
use serde_json::json;

use super::*;
use crate::CanonicalRuntimeRequestV3;
use fixtures::{artifact, mentioned_string_selector, request_payload, runtime_context};

fn mentioned_string_role(request: &CanonicalRuntimeRequestV3<'_>) -> u16 {
    request
        .view()
        .structural
        .roles
        .iter()
        .find(|role| {
            role.features.value_type == BindingValueTypeV1::String
                && role.features.request_relation == BindingRequestRelationV1::Mentioned
        })
        .expect("mentioned string role")
        .role_id
}

#[test]
fn mode_compiles_to_existing_graph_and_exposes_complete_structural_mapping() {
    let payload = request_payload(json!({"handle": "CellA17", "state": "running"}));
    let request = runtime_context("continue CellA17", &payload);
    let expected_role = mentioned_string_role(&request);
    let index = compile_structural_dispatch_index_v3(&[artifact(1, mentioned_string_selector())])
        .expect("compiled structural index");

    assert_eq!(index.modes().len(), 1);
    assert_eq!(index.modes()[0].role_graph().role_count(), 3);
    assert_eq!(index.modes()[0].relation_program().relations().len(), 2);
    let dispatch = index.dispatch(&request);
    assert_eq!(dispatch.verdict(), StructuralDispatchVerdictV3::Complete);
    assert_eq!(dispatch.matched_mode_count(), 1);
    let binding = bind_structural_modes_v3(&index, &request, &dispatch);
    assert_eq!(binding.verdict(), StructuralBindingVerdictV3::Complete);
    assert!(!binding.execution_authority());
    assert!(binding.mapping_evaluations() <= F5C_MAX_MAPPINGS_PER_MODE_V3);
    assert!(binding.source_candidate_evaluations() <= F5C_MAX_SOURCE_CANDIDATE_EVALUATIONS_V3);
    assert_eq!(
        binding.mode_reports()[0].mappings(),
        binding.mode_reports()[0].phase_winner_mappings()
    );
    assert!(
        binding.mode_reports()[0]
            .mappings()
            .iter()
            .any(|mapping| mapping.runtime_source_role_id() == expected_role)
    );
}

#[test]
fn topology_commitment_uses_all_256_bits_and_rejects_a_changed_word() {
    let payload = request_payload(json!({"handle": "CellA17", "state": "running"}));
    let request = runtime_context("continue CellA17", &payload);
    let expected_role = mentioned_string_role(&request);
    let topology_root = request.view().structural.roles[usize::from(expected_role)]
        .features
        .topology_neighborhood_root_sha256
        .clone();
    let selector = vec![
        BindingPredicateV1::ValueType {
            value: BindingValueTypeV1::String,
        },
        BindingPredicateV1::TopologyNeighborhood {
            root_sha256: topology_root.clone(),
        },
    ];
    let index = compile_structural_dispatch_index_v3(&[artifact(2, selector)])
        .expect("topology structural index");
    assert_eq!(index.modes()[0].role_graph().role_count(), 10);
    assert_eq!(index.modes()[0].relation_program().relations().len(), 9);
    let dispatch = index.dispatch(&request);
    let binding = bind_structural_modes_v3(&index, &request, &dispatch);
    assert!(
        binding.mode_reports()[0]
            .mappings()
            .iter()
            .any(|mapping| mapping.runtime_source_role_id() == expected_role)
    );

    let mut changed = topology_root.into_bytes();
    changed[0] = if changed[0] == b'a' { b'b' } else { b'a' };
    let wrong_index = compile_structural_dispatch_index_v3(&[artifact(
        3,
        vec![
            BindingPredicateV1::ValueType {
                value: BindingValueTypeV1::String,
            },
            BindingPredicateV1::TopologyNeighborhood {
                root_sha256: String::from_utf8(changed).expect("changed digest"),
            },
        ],
    )])
    .expect("changed topology index");
    let wrong_dispatch = wrong_index.dispatch(&request);
    let wrong_binding = bind_structural_modes_v3(&wrong_index, &request, &wrong_dispatch);
    assert!(wrong_binding.mode_reports()[0].mappings().is_empty());
}

#[test]
fn package_order_cannot_change_index_or_dispatch_order() {
    let first = artifact(4, mentioned_string_selector());
    let second = artifact(
        5,
        vec![
            BindingPredicateV1::ValueType {
                value: BindingValueTypeV1::String,
            },
            BindingPredicateV1::CallLineage {
                value: BindingCallLineageV1::Unlinked,
            },
        ],
    );
    let left =
        compile_structural_dispatch_index_v3(&[first.clone(), second.clone()]).expect("left index");
    let right = compile_structural_dispatch_index_v3(&[second, first]).expect("right index");
    assert_eq!(left.index_sha256(), right.index_sha256());
    assert_eq!(
        left.modes()
            .iter()
            .map(CompiledProtocolModeV3::mode_id_sha256)
            .collect::<Vec<_>>(),
        right
            .modes()
            .iter()
            .map(CompiledProtocolModeV3::mode_id_sha256)
            .collect::<Vec<_>>()
    );

    let payload = request_payload(json!({"handle": "CellA17"}));
    let request = runtime_context("continue CellA17", &payload);
    assert_eq!(left.dispatch(&request), right.dispatch(&request));
}

#[test]
fn overfull_dispatch_bucket_abstains_without_package_order_truncation() {
    let artifacts = (10..43)
        .map(|seed| artifact(seed, mentioned_string_selector()))
        .collect::<Vec<_>>();
    let index = compile_structural_dispatch_index_v3(&artifacts).expect("overfull index");
    let payload = request_payload(json!({"handle": "CellA17"}));
    let request = runtime_context("continue CellA17", &payload);
    let dispatch = index.dispatch(&request);

    assert_eq!(dispatch.matched_mode_count(), 33);
    assert!(dispatch.mode_indices().is_empty());
    assert_eq!(
        dispatch.verdict(),
        StructuralDispatchVerdictV3::AbstainDispatchExhausted
    );
    let binding = bind_structural_modes_v3(&index, &request, &dispatch);
    assert_eq!(
        binding.verdict(),
        StructuralBindingVerdictV3::AbstainDispatchExhausted
    );
    assert_eq!(binding.mapping_evaluations(), 0);
    assert!(binding.mode_reports().is_empty());
}

#[test]
fn conflicting_selector_and_tampered_payload_are_rejected() {
    let conflict = artifact(
        50,
        vec![
            BindingPredicateV1::ValueType {
                value: BindingValueTypeV1::String,
            },
            BindingPredicateV1::ValueType {
                value: BindingValueTypeV1::Integer,
            },
        ],
    );
    assert!(matches!(
        compile_structural_dispatch_index_v3(&[conflict]),
        Err(ModeToRoleErrorV3::InvalidSelector)
    ));

    let valid = artifact(51, mentioned_string_selector());
    let mut wire = serde_json::to_value(&valid).expect("artifact value");
    wire["modes"][0]["payload"]["arguments"][0]["value_type"] = json!("integer");
    let mut bytes = serde_json::to_vec_pretty(&wire).expect("tampered bytes");
    bytes.push(b'\n');
    assert!(
        ExecutableProtocolModeArtifactV3::from_canonical_bytes(&bytes, valid.artifact_sha256(),)
            .is_err()
    );
}
