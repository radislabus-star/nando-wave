use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::*;

fn digest_json(value: &Value) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("fixture serializes"))
    )
}

fn context() -> PreActionBindingContextV1 {
    PreActionBindingContextV1 {
        call_shape_count: 1,
        capability_count: 1,
        completion_state: BindingCompletionStateV1::Unresolved,
        temporal_relation_count: 1,
        cardinality_relation_count: 1,
        topology_neighborhood_root_sha256: "a".repeat(64),
    }
}

fn graph(seed: char, request: &str, payload: &Value) -> FrozenCandidateRelationGraphV1 {
    PreActionBindingSurfaceV1::capture(
        seed.to_string().repeat(64),
        "0".repeat(64),
        request,
        payload,
        context(),
        BindingEvidenceBudgetV1::default(),
    )
    .expect("pre-action surface")
    .candidate_relation_graph(BindingEvidenceBudgetV1::default())
    .expect("candidate graph")
    .freeze()
    .expect("frozen graph")
}

fn expected(value: &str) -> String {
    digest_json(&Value::String(value.to_owned()))
}

fn payload_with_parts(parts: Vec<Value>) -> Value {
    json!({
        "transport": {
            "events": [
                {"kind": "call", "opaque": "call-00000001"},
                {
                    "kind": "result",
                    "opaque": "call-00000001",
                    "parts": parts,
                }
            ]
        }
    })
}

#[test]
fn expected_binding_can_only_join_after_graph_freeze() {
    let payload = payload_with_parts(vec![json!({
        "text": "still active; opaque continuation target-00000001"
    })]);
    let graph = graph('1', "continue the active execution", &payload);
    let frozen_bytes = serde_json::to_vec(&graph).expect("frozen graph bytes");

    let target = ExpectedBindingReceiptV1::positive(
        &graph,
        expected("target-00000001"),
        BindingBaselineOutcomeV1::Exact,
    )
    .expect("target receipt");
    let foreign_label = ExpectedBindingReceiptV1::positive(
        &graph,
        expected("unrelated-00000002"),
        BindingBaselineOutcomeV1::Wrong,
    )
    .expect("foreign receipt");

    assert_ne!(target.receipt_sha256, foreign_label.receipt_sha256);
    assert_eq!(
        serde_json::to_vec(&graph).expect("frozen graph bytes"),
        frozen_bytes
    );
    let serialized = String::from_utf8(frozen_bytes).expect("utf8 json");
    assert!(!serialized.contains("expected"));
    assert!(!serialized.contains("target-00000001"));
}

#[test]
fn content_part_reordering_is_graph_invariant() {
    let left = payload_with_parts(vec![
        json!({"text": "first opaque-00000001"}),
        json!({"text": "second target-00000002"}),
    ]);
    let right = payload_with_parts(vec![
        json!({"text": "second target-00000002"}),
        json!({"text": "first opaque-00000001"}),
    ]);
    let left = graph('2', "", &left);
    let right = graph('2', "", &right);
    assert_eq!(left, right);
}

#[test]
fn frozen_graph_and_shared_structural_extractor_have_identical_canonical_views() {
    let payload = payload_with_parts(vec![
        json!({"text": "first opaque-00000001"}),
        json!({"text": "active target-00000002"}),
    ]);
    let frozen = graph('9', "continue target-00000002", &payload);
    let from_frozen = canonical_runtime_structural_view_v3_from_frozen_graph(&frozen)
        .expect("frozen graph canonical view");
    let proof_context = context();
    let structural_context = nando_operator_kernel::StructuralContextV3 {
        call_shape_count: proof_context.call_shape_count,
        capability_count: proof_context.capability_count,
        completion_state: proof_context.completion_state,
        temporal_relation_count: proof_context.temporal_relation_count,
        cardinality_relation_count: proof_context.cardinality_relation_count,
        topology_neighborhood_root_sha256: proof_context.topology_neighborhood_root_sha256,
    };
    let extraction = nando_operator_kernel::extract_structural_surface_v3(
        "continue target-00000002",
        &payload,
        structural_context.clone(),
        nando_operator_kernel::StructuralExtractionBudgetV3 {
            max_json_nodes: MAX_BINDING_JSON_NODES_V1,
            max_text_bytes: MAX_BINDING_TEXT_BYTES_V1,
            max_recent_events: MAX_BINDING_RECENT_EVENTS_V1,
            max_role_candidates: MAX_BINDING_CANDIDATES_PER_ROW_V1,
            max_relations: MAX_BINDING_RELATION_EDGES_PER_ROW_V1,
        },
        nando_operator_kernel::StructuralExtractionScopeV3::FrozenEvidence,
    )
    .expect("shared structural extraction");
    let direct = nando_operator_kernel::canonicalize_runtime_structural_view_v3(
        structural_context,
        &extraction,
    )
    .expect("direct canonical view");
    assert_eq!(from_frozen, direct);
}

#[test]
fn renamed_fields_and_function_names_do_not_change_candidate_relations() {
    let left = json!({
        "input": [
            {"type": "function_call", "name": "wait_protocol", "id": "call-00000001"},
            {"type": "function_call_output", "call_id": "call-00000001", "output": "active target-00000002"}
        ]
    });
    let right = json!({
        "renamed_envelope": [
            {"surface_a": "renamed_protocol", "surface_b": "poll_protocol", "surface_c": "call-00000001"},
            {"surface_d": "renamed_result", "surface_e": "call-00000001", "surface_f": "active target-00000002"}
        ]
    });
    let left = graph('3', "", &left);
    let right = graph('3', "", &right);
    let left_target = left
        .graph
        .nodes
        .iter()
        .find(|node| node.action_equivalence_sha256 == expected("target-00000002"))
        .expect("left target");
    let right_target = right
        .graph
        .nodes
        .iter()
        .find(|node| node.action_equivalence_sha256 == expected("target-00000002"))
        .expect("right target");
    assert_eq!(left_target.features, right_target.features);
}

#[test]
fn direct_and_singleton_wrapped_protocols_have_the_same_target_features() {
    let direct = json!({
        "events": [
            {"opaque": "call-00000001"},
            {"opaque": "call-00000001", "result": "active target-00000002"}
        ]
    });
    let wrapped = json!({
        "wrapper": {"events": [
            {"outer": {"opaque": "call-00000001"}},
            {"outer": {"opaque": "call-00000001", "result": "active target-00000002"}}
        ]}
    });
    let direct = graph('4', "", &direct);
    let wrapped = graph('4', "", &wrapped);
    let features = |graph: &FrozenCandidateRelationGraphV1| {
        graph
            .graph
            .nodes
            .iter()
            .find(|node| node.action_equivalence_sha256 == expected("target-00000002"))
            .filter(|node| node.features.value_type == BindingValueTypeV1::String)
            .expect("target")
            .features
            .clone()
    };
    assert_eq!(features(&direct), features(&wrapped));
}

#[test]
fn older_active_handle_has_a_distinct_allowed_temporal_relation() {
    let payload = json!({
        "events": [
            {"result": "active older-00000001"},
            {"result": "active target-00000002"}
        ]
    });
    let graph = graph('5', "", &payload);
    let temporal = |value: &str| {
        graph
            .graph
            .nodes
            .iter()
            .find(|node| {
                node.action_equivalence_sha256 == expected(value)
                    && node.features.value_type == BindingValueTypeV1::String
            })
            .expect("candidate")
            .features
            .temporal_distance
    };
    assert_eq!(temporal("older-00000001"), 1);
    assert_eq!(temporal("target-00000002"), 0);
}

#[test]
fn duplicate_same_prefix_candidates_remain_a_tie_not_a_hidden_lookup() {
    let payload = json!({
        "events": [{
            "result": "Process running with session ID wrong-00000001\nProcess running with session ID target-00000002"
        }]
    });
    let graph = graph('6', "", &payload);
    let receipt = ExpectedBindingReceiptV1::positive(
        &graph,
        expected("target-00000002"),
        BindingBaselineOutcomeV1::Wrong,
    )
    .expect("receipt");
    let report = evaluate_binding_version_space_v1(
        vec![graph],
        vec![receipt],
        Vec::new(),
        BindingEvidenceBudgetV1::default(),
    )
    .expect("report");
    assert_eq!(
        report.verdict,
        BindingVersionSpaceVerdictV1::InsufficientBindingEvidence
    );
    assert!(!report.ties.is_empty());
    assert!(report.identifiable_candidate.is_none());
    let json = serde_json::to_string(&report).expect("report json");
    assert!(!json.contains("Process running"));
    assert!(!json.contains("wrong-00000001"));
    assert!(!json.contains("target-00000002"));
}

#[test]
fn candidate_values_paths_ordinals_and_prefixes_are_not_hypothesis_predicates() {
    let payload = json!({
        "events": [{"result": "prefix target-00000001"}]
    });
    let graph = graph('7', "", &payload);
    let receipt = ExpectedBindingReceiptV1::positive(
        &graph,
        expected("target-00000001"),
        BindingBaselineOutcomeV1::Exact,
    )
    .expect("receipt");
    let report = evaluate_binding_version_space_v1(
        vec![graph],
        vec![receipt],
        Vec::new(),
        BindingEvidenceBudgetV1::default(),
    )
    .expect("report");
    let json = serde_json::to_string(&report.competing_hypotheses).expect("hypotheses json");
    assert!(!json.contains("action_equivalence_sha256"));
    assert!(!json.contains("candidate_id"));
    assert!(!json.contains("path"));
    assert!(!json.contains("ordinal"));
    assert!(!json.contains("prefix"));
}

#[test]
fn applicability_negative_is_not_confused_with_censored_unknown() {
    let positive_payload = json!({
        "events": [
            {"result": "older old-00000001"},
            {"result": "active target-00000002"}
        ]
    });
    let negative_payload = json!({
        "events": [
            {"result": "unrelated wrong-00000003"},
            {"result": "completed without candidate"}
        ]
    });
    let positive = graph('8', "", &positive_payload);
    let negative = graph('9', "", &negative_payload);
    let receipts = vec![
        ExpectedBindingReceiptV1::positive(
            &positive,
            expected("target-00000002"),
            BindingBaselineOutcomeV1::Exact,
        )
        .expect("positive receipt"),
        ExpectedBindingReceiptV1::applicability_negative(&negative).expect("negative receipt"),
    ];
    let censored = vec!["a".repeat(64), "b".repeat(64), "c".repeat(64)];
    let report = evaluate_binding_version_space_v1(
        vec![positive, negative],
        receipts,
        censored,
        BindingEvidenceBudgetV1::default(),
    )
    .expect("report");
    assert_eq!(report.positive_rows, 1);
    assert_eq!(report.applicability_negative_rows, 1);
    assert_eq!(report.censored_unknown_rows, 3);
    assert_eq!(report.negative_accepts, 0);
}

#[test]
fn row_order_shuffle_produces_byte_identical_report() {
    let first = graph(
        'a',
        "",
        &json!({"events": [{"result": "active first-00000001"}]}),
    );
    let second = graph(
        'b',
        "",
        &json!({"events": [{"result": "active second-00000002"}]}),
    );
    let first_receipt = ExpectedBindingReceiptV1::positive(
        &first,
        expected("first-00000001"),
        BindingBaselineOutcomeV1::Exact,
    )
    .expect("first receipt");
    let second_receipt = ExpectedBindingReceiptV1::positive(
        &second,
        expected("second-00000002"),
        BindingBaselineOutcomeV1::Exact,
    )
    .expect("second receipt");
    let left = evaluate_binding_version_space_v1(
        vec![first.clone(), second.clone()],
        vec![first_receipt.clone(), second_receipt.clone()],
        vec!["c".repeat(64)],
        BindingEvidenceBudgetV1::default(),
    )
    .expect("left report");
    let right = evaluate_binding_version_space_v1(
        vec![second, first],
        vec![second_receipt, first_receipt],
        vec!["c".repeat(64)],
        BindingEvidenceBudgetV1::default(),
    )
    .expect("right report");
    assert_eq!(
        serde_json::to_vec(&left).expect("left bytes"),
        serde_json::to_vec(&right).expect("right bytes")
    );
}

#[test]
fn candidate_and_hypothesis_budgets_fail_closed() {
    let payload = json!({
        "events": [{
            "result": (0..80)
                .map(|index| format!("candidate-{index:08}"))
                .collect::<Vec<_>>()
                .join(" ")
        }]
    });
    let budget = BindingEvidenceBudgetV1 {
        max_candidates_per_row: 4,
        max_hypotheses: 2,
        ..BindingEvidenceBudgetV1::default()
    };
    let surface = PreActionBindingSurfaceV1::capture(
        "d".repeat(64),
        "e".repeat(64),
        "",
        &payload,
        context(),
        budget,
    )
    .expect("surface");
    assert!(surface.candidate_budget_exhausted);
    let graph = surface
        .candidate_relation_graph(budget)
        .expect("graph")
        .freeze()
        .expect("freeze");
    let receipt = ExpectedBindingReceiptV1::positive(
        &graph,
        expected("candidate-00000000"),
        BindingBaselineOutcomeV1::Exact,
    )
    .expect("receipt");
    let report = evaluate_binding_version_space_v1(vec![graph], vec![receipt], Vec::new(), budget)
        .expect("report");
    assert_eq!(
        report.verdict,
        BindingVersionSpaceVerdictV1::InsufficientBindingEvidence
    );
    assert_eq!(report.candidate_budget_exhausted_rows, 1);

    let hypothesis_budget = BindingEvidenceBudgetV1 {
        max_hypotheses: 2,
        ..BindingEvidenceBudgetV1::default()
    };
    let graph = PreActionBindingSurfaceV1::capture(
        "d".repeat(64),
        "e".repeat(64),
        "",
        &json!({"events": [{"result": "active target-00000000"}]}),
        context(),
        hypothesis_budget,
    )
    .expect("hypothesis surface")
    .candidate_relation_graph(hypothesis_budget)
    .expect("hypothesis graph")
    .freeze()
    .expect("hypothesis freeze");
    let receipt = ExpectedBindingReceiptV1::positive(
        &graph,
        expected("target-00000000"),
        BindingBaselineOutcomeV1::Exact,
    )
    .expect("hypothesis receipt");
    let report = evaluate_binding_version_space_v1(
        vec![graph],
        vec![receipt],
        Vec::new(),
        hypothesis_budget,
    )
    .expect("hypothesis report");
    assert!(report.hypothesis_budget_exhausted);
}

#[test]
fn a_structurally_separated_action_class_is_identifiable_without_compiling_a_selector() {
    let positive = graph(
        'e',
        "",
        &json!({
            "events": [
                {"result": "old older-00000001"},
                {"result": "active target-00000002"}
            ]
        }),
    );
    let negative = graph(
        'f',
        "",
        &json!({
            "events": [
                {"result": "unrelated wrong-00000003"},
                {"result": "completed"}
            ]
        }),
    );
    let receipts = vec![
        ExpectedBindingReceiptV1::positive(
            &positive,
            expected("target-00000002"),
            BindingBaselineOutcomeV1::Exact,
        )
        .expect("positive"),
        ExpectedBindingReceiptV1::applicability_negative(&negative).expect("negative"),
    ];
    let report = evaluate_binding_version_space_v1(
        vec![positive, negative],
        receipts,
        Vec::new(),
        BindingEvidenceBudgetV1::default(),
    )
    .expect("report");
    assert_eq!(
        report.verdict,
        BindingVersionSpaceVerdictV1::BindingIdentifiableCandidate
    );
    assert!(report.identifiable_candidate.is_some());
    assert!(!report.protocol_mode_compiled);
    assert!(!report.execution_authority);
}
