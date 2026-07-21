use serde_json::json;

use super::*;
use crate::{
    AtomValueType, EffectEdge, EffectOperationKind, RuntimeFrame, RuntimeParityCase,
    TeacherActionAst, TeacherOutcome, TeacherVerifierEvidence,
};

fn sha(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn transition(
    seed: u8,
    handle_type: AtomValueType,
    transport_atoms: Vec<RelationAtom>,
    arguments: Vec<RelationAtom>,
) -> TeacherTransition {
    let value_sha256 = sha(0xab);
    let before = RuntimeFrame {
        schema: "nando.runtime-frame.v1".to_owned(),
        frame_id_sha256: sha(seed),
        event_id_sha256: sha(seed.wrapping_add(1)),
        client_intent_id_sha256: sha(seed.wrapping_add(2)),
        session_id_sha256: sha(seed.wrapping_add(3)),
        observed_at_unix_nanos: u64::from(seed),
        extractor_version: "structured-fixture".to_owned(),
        atoms: vec![RelationAtom::TypedSlot {
            slot_id: 1,
            value_type: handle_type,
            source: crate::AtomSource::Observation,
            value_sha256: value_sha256.clone(),
        }],
        evidence_ref_sha256: sha(seed.wrapping_add(4)),
    };
    let mut action_atoms = vec![
        RelationAtom::TypedSlot {
            slot_id: 7,
            value_type: handle_type,
            source: crate::AtomSource::Action,
            value_sha256,
        },
        RelationAtom::ActionRoleArgument {
            name: "handle".to_owned(),
            slot_id: 7,
            value_type: Some(handle_type),
        },
        RelationAtom::SlotEquality {
            left_slot: 1,
            right_slot: 7,
        },
    ];
    action_atoms.extend(transport_atoms);
    action_atoms.extend(arguments);
    TeacherTransition {
        schema: "nando.teacher-transition.v1".to_owned(),
        before,
        outcome: TeacherOutcome {
            schema: "nando.teacher-outcome.v1".to_owned(),
            action: TeacherActionAst {
                signature_sha256: sha(seed.wrapping_add(5)),
                action_symbol: "physical-surface".to_owned(),
                atoms: action_atoms,
            },
            verifier: TeacherVerifierEvidence {
                accepted: true,
                evidence_ref_sha256: sha(seed.wrapping_add(6)),
                output_digest_sha256: sha(seed.wrapping_add(7)),
            },
            completed_at_unix_nanos: u64::from(seed) + 1,
        },
        economics: None,
        runtime_parity_case: Some(RuntimeParityCase {
            evidence_ref_sha256: sha(seed.wrapping_add(8)),
            capture_receipt: None,
            request_text: String::new(),
            provider_payload: json!({}),
            expected_response: "{}".to_owned(),
        }),
    }
}

fn wait_transition(seed: u8) -> TeacherTransition {
    transition(
        seed,
        AtomValueType::Identifier,
        vec![RelationAtom::ActionFunction {
            value: "wait".to_owned(),
        }],
        Vec::new(),
    )
}

fn write_stdin_transition(seed: u8, chars: &str) -> TeacherTransition {
    transition(
        seed,
        AtomValueType::Integer,
        vec![RelationAtom::ActionFunction {
            value: "write_stdin".to_owned(),
        }],
        vec![RelationAtom::ActionStringArgument {
            name: "chars".to_owned(),
            value: chars.to_owned(),
        }],
    )
}

fn raw_graph(nodes: Vec<EffectNode>, edges: Vec<EffectEdge>) -> EffectGraph {
    let bytes = serde_json::to_vec(&(EFFECT_GRAPH_SCHEMA_V1, &nodes, &edges))
        .expect("test graph serializes");
    EffectGraph {
        schema: EFFECT_GRAPH_SCHEMA_V1.to_owned(),
        nodes,
        edges,
        completeness: EffectGraphCompleteness::Complete,
        canonical_sha256: Some(format!("{:x}", Sha256::digest(bytes))),
        alignment_candidates: 1,
        canonical_permutations: 1,
    }
}

fn scalar_node(index: u16, source: EffectSource, value_type: AtomValueType) -> EffectNode {
    EffectNode {
        index,
        source,
        kind: EffectNodeKind::Scalar,
        value_type: Some(value_type),
        unique: false,
        operation: None,
    }
}

fn operation_node(index: u16, operation: EffectOperationKind) -> EffectNode {
    EffectNode {
        index,
        source: EffectSource::Action,
        kind: EffectNodeKind::Operation,
        value_type: None,
        unique: true,
        operation: Some(operation),
    }
}

fn role(role_id: u16, node: u16, value_type: u16) -> EffectRoleV2 {
    EffectRoleV2 {
        role_id: RoleRefV2::new(role_id),
        node,
        value_type: EffectValueTypeV2::new(value_type).expect("value type is valid"),
        cardinality: RoleCardinalityV2 { min: 1, max: 1 },
    }
}

fn clause(opcode: u16, lhs: u16, rhs: Option<u16>) -> EffectClauseV2 {
    EffectClauseV2 {
        opcode: EffectOpcodeV2::new(opcode).expect("test opcode is valid"),
        lhs: RoleRefV2::new(lhs),
        rhs: rhs.map(RoleRefV2::new),
        constant: None,
        argument_key_sha256: None,
    }
}

fn dictionary_roots() -> EffectLawDictionaryRootsV2 {
    EffectLawDictionaryRootsV2::builtin_v2().expect("builtin dictionaries are valid")
}

fn one_role_law(graph: &EffectGraph) -> CanonicalizedEffectLawV2 {
    let node = &graph.nodes[0];
    let value_type = match node.kind {
        EffectNodeKind::Operation => EFFECT_VALUE_OPERATION,
        EffectNodeKind::Scalar => match node.value_type.expect("scalar type") {
            AtomValueType::String => EFFECT_VALUE_STRING,
            AtomValueType::Integer => EFFECT_VALUE_INTEGER,
            AtomValueType::Boolean => EFFECT_VALUE_BOOLEAN,
            AtomValueType::Identifier => EFFECT_VALUE_IDENTIFIER,
            AtomValueType::Collection => EFFECT_VALUE_COLLECTION,
        },
        EffectNodeKind::Collection => EFFECT_VALUE_COLLECTION,
    };
    CanonicalEffectLawV2::from_unverified_program(
        graph,
        dictionary_roots(),
        EffectLawProgramV2 {
            roles: vec![role(8, node.index, value_type)],
            clauses: vec![clause(EFFECT_OPCODE_REQUIRE, 8, None)],
            preserved_frame: PreservedFrameContractV2::default(),
        },
    )
    .expect("fixture law is structurally valid")
}

#[test]
fn single_observation_cannot_create_a_law_candidate() {
    let observation = observe_effect_transition_v2(&wait_transition(10)).expect("observation");
    assert_eq!(
        search_effect_law_quotient_v2(&[observation], dictionary_roots()),
        Err(EffectLawError::InsufficientIndependentEvidence)
    );
}

#[test]
fn independent_identical_observations_create_an_exact_candidate() {
    let observations = [
        observe_effect_transition_v2(&wait_transition(10)).expect("left observation"),
        observe_effect_transition_v2(&wait_transition(30)).expect("right observation"),
    ];
    let report = search_effect_law_quotient_v2(&observations, dictionary_roots())
        .expect("independent exact quotient");
    assert!(report.candidate.is_some());
    assert_eq!(report.independent_lineages, 2);
    assert_eq!(report.blocker, None);
}

#[test]
fn same_lineage_does_not_count_as_independent_evidence() {
    let first = observe_effect_transition_v2(&wait_transition(10)).expect("first observation");
    let mut same_lineage_transition = wait_transition(30);
    same_lineage_transition
        .before
        .session_id_sha256
        .clone_from(&first.lineage_sha256);
    let second =
        observe_effect_transition_v2(&same_lineage_transition).expect("second observation");
    assert_eq!(
        search_effect_law_quotient_v2(&[first, second], dictionary_roots()),
        Err(EffectLawError::InsufficientIndependentEvidence)
    );
}

#[test]
fn tampered_observation_cannot_enter_quotient_search() {
    let first = observe_effect_transition_v2(&wait_transition(10)).expect("first observation");
    let mut tampered =
        observe_effect_transition_v2(&wait_transition(30)).expect("second observation");
    tampered.lineage_sha256 = sha(0xee);
    assert_eq!(
        search_effect_law_quotient_v2(&[first, tampered], dictionary_roots()),
        Err(EffectLawError::InvalidEvidence)
    );
}

#[test]
fn distinct_structured_wait_and_write_fixtures_are_not_preemptively_merged() {
    let observations = [
        observe_effect_transition_v2(&wait_transition(10)).expect("wait observation"),
        observe_effect_transition_v2(&write_stdin_transition(30, "")).expect("write observation"),
    ];
    let report =
        search_effect_law_quotient_v2(&observations, dictionary_roots()).expect("quotient report");
    assert!(report.candidate.is_none());
    assert_eq!(
        report.blocker.as_deref(),
        Some("no_invariant_exact_quotient")
    );
    assert_eq!(report.protocol_mode_candidates.len(), 1);
}

#[test]
fn call_and_project_operations_have_different_law_ids() {
    let call = raw_graph(
        vec![operation_node(0, EffectOperationKind::Call)],
        Vec::new(),
    );
    let project = raw_graph(
        vec![operation_node(0, EffectOperationKind::Project)],
        Vec::new(),
    );
    assert_ne!(
        one_role_law(&call).law().effect_law_id().expect("call ID"),
        one_role_law(&project)
            .law()
            .effect_law_id()
            .expect("project ID")
    );
}

#[test]
fn physical_scalar_types_are_not_collapsed() {
    for (left, right) in [
        (AtomValueType::Integer, AtomValueType::String),
        (AtomValueType::Identifier, AtomValueType::Integer),
    ] {
        let left = raw_graph(
            vec![scalar_node(0, EffectSource::Observation, left)],
            Vec::new(),
        );
        let right = raw_graph(
            vec![scalar_node(0, EffectSource::Observation, right)],
            Vec::new(),
        );
        assert_ne!(
            one_role_law(&left).law().effect_law_id().expect("left ID"),
            one_role_law(&right)
                .law()
                .effect_law_id()
                .expect("right ID")
        );
    }
}

#[test]
fn false_empty_string_and_integer_arguments_are_preserved() {
    let transition = transition(
        10,
        AtomValueType::Identifier,
        vec![RelationAtom::ActionFunction {
            value: "wait".to_owned(),
        }],
        vec![
            RelationAtom::ActionBooleanArgument {
                name: "terminate".to_owned(),
                value: false,
            },
            RelationAtom::ActionStringArgument {
                name: "chars".to_owned(),
                value: String::new(),
            },
            RelationAtom::ActionIntegerArgument {
                name: "yield_time-ms".to_owned(),
                value: 10_000,
            },
        ],
    );
    let observation = observe_effect_transition_v2(&transition).expect("observation");
    let constants = observation
        .arguments
        .iter()
        .filter(|argument| argument.constant.is_some())
        .collect::<Vec<_>>();
    assert_eq!(constants.len(), 3);
    assert!(
        constants
            .iter()
            .all(|argument| argument.argument_key_sha256.len() == 64)
    );
}

#[test]
fn constant_argument_ownership_changes_the_observation() {
    let left = transition(
        10,
        AtomValueType::Identifier,
        Vec::new(),
        vec![RelationAtom::ActionBooleanArgument {
            name: "left-owner".to_owned(),
            value: false,
        }],
    );
    let right = transition(
        30,
        AtomValueType::Identifier,
        Vec::new(),
        vec![RelationAtom::ActionBooleanArgument {
            name: "right-owner".to_owned(),
            value: false,
        }],
    );
    let left = observe_effect_transition_v2(&left).expect("left observation");
    let right = observe_effect_transition_v2(&right).expect("right observation");
    assert_ne!(left.arguments, right.arguments);
}

#[test]
fn dictionary_roots_are_part_of_law_identity() {
    let graph = raw_graph(
        vec![scalar_node(
            0,
            EffectSource::Observation,
            AtomValueType::Identifier,
        )],
        Vec::new(),
    );
    let left = one_role_law(&graph);
    let roots = EffectLawDictionaryRootsV2::new(sha(0xcc), sha(0xdd)).expect("custom roots");
    let right = CanonicalEffectLawV2::from_unverified_program(
        &graph,
        roots,
        EffectLawProgramV2 {
            roles: vec![role(1, 0, EFFECT_VALUE_IDENTIFIER)],
            clauses: vec![clause(EFFECT_OPCODE_REQUIRE, 1, None)],
            preserved_frame: PreservedFrameContractV2::default(),
        },
    )
    .expect("law with custom dictionary roots");
    assert_ne!(
        left.law().effect_law_id().expect("builtin ID"),
        right.law().effect_law_id().expect("custom ID")
    );
}

#[test]
fn unknown_opcode_is_only_represented_as_unverified_data() {
    let graph = raw_graph(
        vec![scalar_node(
            0,
            EffectSource::Observation,
            AtomValueType::Identifier,
        )],
        Vec::new(),
    );
    let law = CanonicalEffectLawV2::from_unverified_program(
        &graph,
        dictionary_roots(),
        EffectLawProgramV2 {
            roles: vec![role(1, 0, EFFECT_VALUE_IDENTIFIER)],
            clauses: vec![clause(40_001, 1, None)],
            preserved_frame: PreservedFrameContractV2::default(),
        },
    )
    .expect("open opcode is representable");
    assert_eq!(law.law().clauses()[0].opcode.get(), 40_001);
}

#[test]
fn physical_nodes_are_remapped_into_canonical_program_nodes() {
    let graph = raw_graph(
        vec![
            scalar_node(20, EffectSource::Action, AtomValueType::Identifier),
            scalar_node(10, EffectSource::Observation, AtomValueType::Identifier),
        ],
        Vec::new(),
    );
    let canonical = CanonicalEffectLawV2::from_unverified_program(
        &graph,
        dictionary_roots(),
        EffectLawProgramV2 {
            roles: vec![
                role(1, 10, EFFECT_VALUE_IDENTIFIER),
                role(2, 20, EFFECT_VALUE_IDENTIFIER),
            ],
            clauses: vec![clause(EFFECT_OPCODE_COPY, 1, Some(2))],
            preserved_frame: PreservedFrameContractV2::default(),
        },
    )
    .expect("physical IDs remap");
    assert_eq!(canonical.node_mapping().len(), 2);
    assert!(canonical.law().roles().iter().all(|role| role.node < 2));
}

#[test]
fn canonical_restart_is_byte_identical() {
    let graph = raw_graph(
        vec![scalar_node(
            0,
            EffectSource::Observation,
            AtomValueType::Identifier,
        )],
        Vec::new(),
    );
    let law = one_role_law(&graph).law().clone();
    let bytes = law.canonical_bytes().expect("law serializes");
    let restored = CanonicalEffectLawV2::from_canonical_bytes(&bytes).expect("law restores");
    assert_eq!(bytes, restored.canonical_bytes().expect("restored bytes"));
    assert_eq!(
        law.effect_law_id().expect("ID"),
        restored.effect_law_id().expect("ID")
    );
}

#[test]
fn repeated_typed_roles_and_multiple_outputs_are_admissible() {
    let graph = raw_graph(
        vec![
            scalar_node(0, EffectSource::Observation, AtomValueType::Identifier),
            scalar_node(1, EffectSource::Observation, AtomValueType::Identifier),
            scalar_node(2, EffectSource::Action, AtomValueType::Identifier),
            scalar_node(3, EffectSource::Action, AtomValueType::Identifier),
        ],
        Vec::new(),
    );
    let law = CanonicalEffectLawV2::from_unverified_program(
        &graph,
        dictionary_roots(),
        EffectLawProgramV2 {
            roles: vec![
                role(10, 0, EFFECT_VALUE_IDENTIFIER),
                role(11, 1, EFFECT_VALUE_IDENTIFIER),
                role(12, 2, EFFECT_VALUE_IDENTIFIER),
                role(13, 3, EFFECT_VALUE_IDENTIFIER),
            ],
            clauses: vec![
                clause(EFFECT_OPCODE_COMPOSE, 10, Some(11)),
                clause(EFFECT_OPCODE_PRODUCE, 10, Some(12)),
                clause(EFFECT_OPCODE_PRODUCE, 11, Some(13)),
            ],
            preserved_frame: PreservedFrameContractV2::default(),
        },
    )
    .expect("repeated roles and outputs are valid");
    assert_eq!(law.law().roles().len(), 4);
}

#[test]
fn incomplete_topology_has_no_law() {
    let mut graph = raw_graph(
        vec![scalar_node(
            0,
            EffectSource::Observation,
            AtomValueType::Identifier,
        )],
        Vec::new(),
    );
    graph.completeness = EffectGraphCompleteness::Ambiguous;
    assert_eq!(
        CanonicalEffectLawV2::from_unverified_program(
            &graph,
            dictionary_roots(),
            EffectLawProgramV2 {
                roles: vec![role(1, 0, EFFECT_VALUE_IDENTIFIER)],
                clauses: vec![clause(EFFECT_OPCODE_REQUIRE, 1, None)],
                preserved_frame: PreservedFrameContractV2::default(),
            },
        ),
        Err(EffectLawError::IncompleteTopology)
    );
}
