use nando_operator_kernel::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, CollectionScalarType,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
    PreActionMultiSourceTopologyV1, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, ValueProjectionFormat, sha256_bytes,
};

use super::pre_action_t1_binding_root;

fn program() -> ResponseProgram {
    ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::SelectTurnOutput { output_ordinal: 0 },
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                selector: ResponseValueSelector::UniqueScalar {
                    value_type: nando_operator_kernel::AtomValueType::String,
                },
                value_type: CollectionScalarType::String,
            },
            CollectionProgramStep::Count,
        ],
        ValueProjectionFormat::PlainText,
        "completed",
    )
}

fn role(
    local_role_id: u16,
    source_ordinal: u16,
    type_class: MultiSourceTypeClassV1,
    container_class: MultiSourceContainerClassV1,
) -> MultiSourceRoleNodeV1 {
    MultiSourceRoleNodeV1 {
        local_role_id,
        source_ordinal,
        value_ordinal: 0,
        type_class,
        container_class,
        cardinality_class: MultiSourceCardinalityClassV1::One,
        temporal_class: MultiSourceTemporalClassV1::Latest,
        depth_bucket: 1,
        structural_flags: 0,
    }
}

fn topology(selector_roots: &[&str]) -> PreActionMultiSourceTopologyV1 {
    let mut roles = vec![role(
        1,
        0,
        MultiSourceTypeClassV1::Array,
        MultiSourceContainerClassV1::Sequence,
    )];
    let mut witnesses = vec![MultiSourceRoleWitnessV1 {
        local_role_id: 1,
        value_sha256: sha256_bytes(b"collection"),
        request_reference_ordinal: None,
        request_reference_ordinal_candidates: Vec::new(),
    }];
    for (index, root) in selector_roots.iter().enumerate() {
        let local_role_id = u16::try_from(index + 2).expect("small test role");
        roles.push(role(
            local_role_id,
            local_role_id,
            MultiSourceTypeClassV1::String,
            MultiSourceContainerClassV1::Scalar,
        ));
        witnesses.push(MultiSourceRoleWitnessV1 {
            local_role_id,
            value_sha256: sha256_bytes(root.as_bytes()),
            request_reference_ordinal: None,
            request_reference_ordinal_candidates: Vec::new(),
        });
    }
    PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: 1,
        output_part_count: 1,
        roles,
        role_witnesses: witnesses,
        relations: Vec::new(),
    }
}

#[test]
fn collection_binding_requires_every_consumed_scalar_role() {
    assert_eq!(
        pre_action_t1_binding_root(&program(), &topology(&[])),
        Err("collection_selector_role_missing_or_ambiguous")
    );
    assert!(pre_action_t1_binding_root(&program(), &topology(&["selector"])).is_ok());
    assert_eq!(
        pre_action_t1_binding_root(&program(), &topology(&["selector-a", "selector-b"])),
        Err("collection_selector_role_missing_or_ambiguous")
    );
}

#[test]
fn collection_binding_root_commits_the_scalar_witness() {
    let left =
        pre_action_t1_binding_root(&program(), &topology(&["selector-a"])).expect("left binding");
    let right =
        pre_action_t1_binding_root(&program(), &topology(&["selector-b"])).expect("right binding");
    assert_ne!(left, right);
}

#[test]
fn collection_binding_requires_renderer_selected_role() {
    let mut program = ResponseProgram::compose_collection(
        vec![CollectionProgramStep::SelectOnlyArrayField],
        ValueProjectionFormat::CanonicalJson,
        "completed",
    );
    let ResponseOperation::ComposeCollection { renderer, .. } = &mut program.operation else {
        panic!("collection program");
    };
    *renderer = CollectionOutputRenderer::RenderSequence {
        segments: vec![
            ResponseRenderSegment::Primary,
            ResponseRenderSegment::Selected {
                selector: ResponseValueSelector::UniqueScalar {
                    value_type: nando_operator_kernel::AtomValueType::String,
                },
                format: ValueProjectionFormat::PlainText,
            },
        ],
    };
    assert_eq!(
        pre_action_t1_binding_root(&program, &topology(&[])),
        Err("collection_selector_role_missing_or_ambiguous")
    );
    assert!(pre_action_t1_binding_root(&program, &topology(&["renderer-selector"])).is_ok());
}

#[test]
fn collection_binding_covers_the_natural_request_selector_vocabulary() {
    let selectors = [
        ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::String,
        },
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal: 0,
            value_type: AtomValueType::String,
        },
        ResponseValueSelector::RequestLastToken,
        ResponseValueSelector::RequestUniqueLiteral,
    ];
    for selector in selectors {
        let mut program = ResponseProgram::compose_collection(
            vec![CollectionProgramStep::SelectOnlyArrayField],
            ValueProjectionFormat::CanonicalJson,
            "completed",
        );
        let ResponseOperation::ComposeCollection { renderer, .. } = &mut program.operation else {
            panic!("collection program");
        };
        *renderer = CollectionOutputRenderer::RenderSequence {
            segments: vec![ResponseRenderSegment::Selected {
                selector,
                format: ValueProjectionFormat::PlainText,
            }],
        };
        let mut topology = topology(&["selector"]);
        topology.role_witnesses[1].request_reference_ordinal = Some(0);
        topology.relations.push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::RequestReferencesRole,
            source_role_id: 2,
            target_role_id: 2,
        });
        topology.relations.sort();
        topology.validate().expect("request-bound topology");
        pre_action_t1_binding_root(&program, &topology).expect("selector binding");
    }
}
