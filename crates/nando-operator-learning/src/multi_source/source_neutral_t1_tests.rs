use nando_operator_kernel::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, CollectionScalarType,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
    PreActionMultiSourceTopologyV1, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, ValueProjectionFormat, sha256_bytes,
};

use crate::multi_source::{
    bind_pre_action_t1_program_to_motif_v1, pre_action_t1_binding_root,
    pre_action_t1_consumed_role_ids_v1, source_neutral_topology_motifs_v1,
};

fn program() -> ResponseProgram {
    ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::SelectTurnOutput { output_ordinal: 1 },
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
fn collection_output_ordinal_is_one_based_at_the_program_boundary() {
    let first = program();
    first.validate().expect("output ordinal one is valid");
    assert!(pre_action_t1_binding_root(&first, &topology(&["selector"])).is_ok());

    let mut second = program();
    if let ResponseOperation::ComposeCollection { steps, .. } = &mut second.operation {
        steps[0] = CollectionProgramStep::SelectTurnOutput { output_ordinal: 2 };
    } else {
        panic!("collection program");
    }
    second.validate().expect("output ordinal two is valid");
    let mut second_topology = topology(&["selector"]);
    second_topology.roles[0].source_ordinal = 1;
    assert!(pre_action_t1_binding_root(&second, &second_topology).is_ok());

    if let ResponseOperation::ComposeCollection { steps, .. } = &mut second.operation {
        steps[0] = CollectionProgramStep::SelectTurnOutput { output_ordinal: 0 };
    }
    assert!(second.validate().is_err());
    assert_eq!(
        pre_action_t1_binding_root(&second, &second_topology),
        Err("collection_output_ordinal_invalid")
    );
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
fn collection_consumed_role_set_includes_collection_and_selector() {
    let topology = topology(&["selector"]);
    assert_eq!(
        pre_action_t1_consumed_role_ids_v1(&program(), &topology).expect("consumed roles"),
        vec![1, 2]
    );
}

#[test]
fn motif_binding_rejects_a_program_role_outside_the_embedding() {
    let mut topology = topology(&["selector"]);
    topology.relations.push(MultiSourceRelationEdgeV1 {
        relation: MultiSourceRelationKindV1::Contains,
        source_role_id: 1,
        target_role_id: 2,
    });
    topology.relations.sort();
    topology.validate().expect("connected topology");
    let motifs = source_neutral_topology_motifs_v1(&topology).expect("motifs");
    let singleton = motifs
        .iter()
        .find(|motif| {
            motif.role_count == 1
                && motif
                    .embeddings
                    .iter()
                    .any(|embedding| embedding.local_role_ids == vec![1])
        })
        .expect("collection singleton");
    assert_eq!(
        bind_pre_action_t1_program_to_motif_v1(&program(), &topology, singleton),
        Err("program_consumed_roles_outside_frozen_motif")
    );

    let connected_pair = motifs
        .iter()
        .find(|motif| motif.role_count == 2 && motif.relation_count == 1)
        .expect("connected pair");
    let binding = bind_pre_action_t1_program_to_motif_v1(&program(), &topology, connected_pair)
        .expect("program fits exact motif");
    assert_eq!(binding.consumed_local_role_ids, vec![1, 2]);
    binding.validate().expect("valid motif binding");
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
