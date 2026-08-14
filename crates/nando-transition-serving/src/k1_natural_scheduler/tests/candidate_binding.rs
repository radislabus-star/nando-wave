use nando_operator_kernel::{
    AtomValueType, CollectionProgramStep, CollectionScalarType, MultiSourceCardinalityClassV1,
    MultiSourceContainerClassV1, MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1,
    MultiSourceRelationKindV1, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
    ResponseProgram, ResponseValueSelector, ValueProjectionFormat, canonical_json_sha256,
};
use nando_operator_learning::multi_source::{
    K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7,
    K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8, pre_action_t1_binding_root,
    source_neutral_topology_motifs_v1,
};

use super::*;

fn role(
    local_role_id: u16,
    type_class: MultiSourceTypeClassV1,
    container_class: MultiSourceContainerClassV1,
    cardinality_class: MultiSourceCardinalityClassV1,
) -> MultiSourceRoleNodeV1 {
    MultiSourceRoleNodeV1 {
        local_role_id,
        source_ordinal: local_role_id,
        value_ordinal: 0,
        type_class,
        container_class,
        cardinality_class,
        temporal_class: MultiSourceTemporalClassV1::Latest,
        depth_bucket: 1,
        structural_flags: 0,
    }
}

fn ambient_topology() -> PreActionMultiSourceTopologyV1 {
    let mut topology = PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: 1,
        output_part_count: 1,
        roles: vec![
            role(
                1,
                MultiSourceTypeClassV1::Array,
                MultiSourceContainerClassV1::Sequence,
                MultiSourceCardinalityClassV1::Many,
            ),
            role(
                2,
                MultiSourceTypeClassV1::Number,
                MultiSourceContainerClassV1::Scalar,
                MultiSourceCardinalityClassV1::One,
            ),
            role(
                3,
                MultiSourceTypeClassV1::String,
                MultiSourceContainerClassV1::Scalar,
                MultiSourceCardinalityClassV1::One,
            ),
        ],
        role_witnesses: vec![
            MultiSourceRoleWitnessV1 {
                local_role_id: 1,
                value_sha256: canonical_json_sha256(&serde_json::json!([
                    {"status": "active"},
                    {"status": "idle"}
                ]))
                .expect("collection witness"),
                request_reference_ordinal: None,
                request_reference_ordinal_candidates: Vec::new(),
            },
            MultiSourceRoleWitnessV1 {
                local_role_id: 2,
                value_sha256: canonical_json_sha256(&serde_json::json!(2)).expect("count witness"),
                request_reference_ordinal: None,
                request_reference_ordinal_candidates: Vec::new(),
            },
            MultiSourceRoleWitnessV1 {
                local_role_id: 3,
                value_sha256: canonical_json_sha256(&serde_json::json!("active"))
                    .expect("selector witness"),
                request_reference_ordinal: Some(0),
                request_reference_ordinal_candidates: Vec::new(),
            },
        ],
        relations: vec![
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::Contains,
                source_role_id: 1,
                target_role_id: 2,
            },
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::Precedes,
                source_role_id: 2,
                target_role_id: 3,
            },
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::RequestReferencesRole,
                source_role_id: 3,
                target_role_id: 3,
            },
        ],
    };
    topology.relations.sort();
    topology.validate().expect("ambient topology");
    topology
}

fn collection_program() -> ResponseProgram {
    ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                selector: ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                    ordinal: 0,
                    value_type: AtomValueType::String,
                },
                value_type: CollectionScalarType::String,
            },
            CollectionProgramStep::Count,
        ],
        ValueProjectionFormat::PlainText,
        "completed",
    )
}

fn freeze_for_motif_with_schema(
    schema: &str,
    motif_root_sha256: String,
) -> K1NaturalCandidateFreezeV1 {
    let mut freeze = candidate_freeze();
    freeze.schema = schema.to_owned();
    freeze.candidate_structural_root_sha256 = motif_root_sha256.clone();
    freeze.source_neutral_topology_root_sha256 = motif_root_sha256;
    freeze
}

fn freeze_for_motif(motif_root_sha256: String) -> K1NaturalCandidateFreezeV1 {
    freeze_for_motif_with_schema(K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6, motif_root_sha256)
}

#[test]
fn v6_candidate_binding_rejects_a_program_that_consumes_an_ambient_role() {
    let topology = ambient_topology();
    let motifs = source_neutral_topology_motifs_v1(&topology).expect("motif enumeration");
    let pair = motifs
        .iter()
        .find(|motif| {
            motif.role_count == 2
                && motif
                    .embeddings
                    .iter()
                    .any(|embedding| embedding.local_role_ids == vec![1, 2])
        })
        .expect("frozen pair motif");

    assert_eq!(
        candidate_program_binding_root(
            &freeze_for_motif(pair.motif_root_sha256.clone()),
            &collection_program(),
            &topology,
        ),
        Err("program_consumed_roles_outside_frozen_motif".to_owned())
    );
}

#[test]
fn v6_candidate_binding_seals_the_exact_embedding_when_all_roles_fit() {
    let topology = ambient_topology();
    let motif = source_neutral_topology_motifs_v1(&topology)
        .expect("motif enumeration")
        .into_iter()
        .find(|motif| {
            motif
                .embeddings
                .iter()
                .any(|embedding| embedding.local_role_ids == vec![1, 2, 3])
        })
        .expect("full connected motif");
    let program = collection_program();
    let binding = candidate_program_binding_root(
        &freeze_for_motif(motif.motif_root_sha256),
        &program,
        &topology,
    )
    .expect("candidate motif binding");

    assert_ne!(
        binding,
        pre_action_t1_binding_root(&program, &topology).expect("ambient binding")
    );
}

#[test]
fn v8_candidate_binding_is_exactly_the_v6_v7_motif_domain() {
    let topology = ambient_topology();
    let motif = source_neutral_topology_motifs_v1(&topology)
        .expect("motif enumeration")
        .into_iter()
        .find(|motif| {
            motif
                .embeddings
                .iter()
                .any(|embedding| embedding.local_role_ids == vec![1, 2, 3])
        })
        .expect("full connected motif");
    let program = collection_program();

    let roots = [
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6,
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7,
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8,
    ]
    .map(|schema| {
        let freeze = freeze_for_motif_with_schema(schema, motif.motif_root_sha256.clone());
        assert_eq!(
            candidate_topology_root(&freeze, &topology).expect("candidate topology"),
            motif.motif_root_sha256
        );
        candidate_program_binding_root(&freeze, &program, &topology)
            .expect("candidate motif binding")
    });

    assert_eq!(roots[0], roots[1]);
    assert_eq!(roots[1], roots[2]);
}
