use serde_json::json;

use super::*;
use crate::{
    AtomSource, ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, RUNTIME_FRAME_SCHEMA_V1,
    RelationAtom, RuntimeFrame, SOURCE_NEUTRAL_EXTRACTOR_VERSION, TEACHER_OUTCOME_SCHEMA_V1,
    TEACHER_TRANSITION_SCHEMA_V1, TeacherActionAst, TeacherOutcome, TeacherVerifierEvidence,
};

fn transition(field: &str, accepted: bool) -> TeacherTransition {
    let hash = |byte: char| byte.to_string().repeat(64);
    TeacherTransition {
        schema: TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
        before: RuntimeFrame {
            schema: RUNTIME_FRAME_SCHEMA_V1.to_owned(),
            frame_id_sha256: hash('1'),
            event_id_sha256: hash('2'),
            client_intent_id_sha256: hash('3'),
            session_id_sha256: hash('4'),
            observed_at_unix_nanos: 1,
            extractor_version: "live-test-v1".to_owned(),
            atoms: Vec::new(),
            evidence_ref_sha256: hash('5'),
        },
        outcome: TeacherOutcome {
            schema: TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
            action: TeacherActionAst {
                signature_sha256: hash('6'),
                action_symbol: "response".to_owned(),
                atoms: Vec::new(),
            },
            verifier: TeacherVerifierEvidence {
                accepted,
                evidence_ref_sha256: hash('7'),
                output_digest_sha256: hash('8'),
            },
            completed_at_unix_nanos: 2,
        },
        economics: Some(EconomicsReceipt {
            schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
            exact_input_tokens: 100,
            ordinary: true,
            controlled: false,
            replay: false,
            dedupe_eligible: true,
            provider_evidence_ref_sha256: hash('9'),
        }),
        runtime_parity_case: Some(crate::RuntimeParityCase {
            evidence_ref_sha256: hash('a'),
            capture_receipt: None,
            request_text: "Return the count".to_owned(),
            provider_payload: json!({
                "input": [{
                    "type": "function_call_output",
                    "output": format!("{{\"{field}\":7}}")
                }]
            }),
            expected_response: "7".to_owned(),
        }),
    }
}

fn multi_value_transition(
    first_field: &str,
    second_field: &str,
    expected_response: &str,
) -> TeacherTransition {
    let mut row = transition(first_field, true);
    let parity = row.runtime_parity_case.as_mut().expect("parity case");
    parity.request_text = format!("Return {first_field} and {second_field}");
    parity.provider_payload = json!({
        "input": [{
            "type": "function_call_output",
            "output": format!("{{\"{first_field}\":7,\"{second_field}\":2}}")
        }]
    });
    parity.expected_response = expected_response.to_owned();
    row
}

fn custom_tool_transition(index: u64) -> TeacherTransition {
    let mut row = transition("unused", true);
    let observation_slot = 3;
    let action_slot = 8;
    let selected = index + 1000;
    let yield_time_ms = 10_000 + (index % 3) * 10_000;
    let max_output_tokens = 1_000 + (index % 5) * 1_000;
    row.before.extractor_version = SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned();
    row.before.atoms = vec![
        RelationAtom::ToolKind {
            value: "exec_command".to_owned(),
        },
        RelationAtom::ObservationCallShape {
            value: "custom_tool_call".to_owned(),
        },
        RelationAtom::CompletionState {
            value: "pending".to_owned(),
        },
        RelationAtom::ResponseShape {
            value: "custom_tool_call".to_owned(),
        },
        RelationAtom::TypedSlot {
            slot_id: observation_slot,
            value_type: AtomValueType::Integer,
            source: AtomSource::Observation,
            value_sha256: format!("{selected:064x}"),
        },
        RelationAtom::UniqueSlot {
            slot_id: observation_slot,
        },
        RelationAtom::ObservationSelector {
            slot_id: observation_slot,
            selector: ResponseValueSelector::ContentLinePrefix {
                prefix: "SESSION_ID=".to_owned(),
                value_type: AtomValueType::Integer,
            },
        },
    ];
    row.outcome.action.action_symbol = "custom_tool:exec/write_stdin".to_owned();
    row.outcome.action.atoms = vec![
        RelationAtom::TypedSlot {
            slot_id: action_slot,
            value_type: AtomValueType::Integer,
            source: AtomSource::Action,
            value_sha256: format!("{selected:064x}"),
        },
        RelationAtom::SlotEquality {
            left_slot: observation_slot,
            right_slot: action_slot,
        },
        RelationAtom::ActionCustomTool {
            value: "exec".to_owned(),
        },
        RelationAtom::ActionInnerTool {
            value: "write_stdin".to_owned(),
        },
        RelationAtom::ActionRoleArgument {
            name: "session_id".to_owned(),
            slot_id: action_slot,
            value_type: None,
        },
        RelationAtom::ActionStringArgument {
            name: "chars".to_owned(),
            value: String::new(),
        },
        RelationAtom::ActionIntegerArgument {
            name: "yield_time_ms".to_owned(),
            value: yield_time_ms,
        },
        RelationAtom::ActionIntegerArgument {
            name: "max_output_tokens".to_owned(),
            value: max_output_tokens,
        },
        RelationAtom::ActionResultProjection {
            output_field: "output".to_owned(),
            continuation_field: "session_id".to_owned(),
            continuation_prefix: "SESSION_ID=".to_owned(),
        },
    ];
    let arguments = serde_json::to_string(&json!({
        "chars": "",
        "max_output_tokens": max_output_tokens,
        "session_id": selected,
        "yield_time_ms": yield_time_ms,
    }))
    .expect("custom tool arguments");
    let input = format!(
        "const r=await tools.write_stdin({arguments});text(r.output);if(r.session_id)text(\"SESSION_ID=\"+r.session_id);"
    );
    let parity = row.runtime_parity_case.as_mut().expect("parity case");
    parity.request_text = "Continue the running process".to_owned();
    parity.provider_payload = json!({
        "tools": [{"type": "custom", "name": "exec"}],
        "input": [
            {
                "type": "custom_tool_call",
                "name": "exec",
                "call_id": format!("call-{index}"),
                "input": "source"
            },
            {
                "type": "custom_tool_call_output",
                "call_id": format!("call-{index}"),
                "output": [{"type": "text", "text": format!("still running\nSESSION_ID={selected}")}]
            }
        ]
    });
    parity.expected_response = serde_json::to_string(&json!({
        "kind": "custom_tool_call",
        "name": "exec",
        "input": input,
    }))
    .expect("custom tool response");
    row
}

fn custom_tool_transition_with_repeated_outputs(index: u64) -> TeacherTransition {
    let mut row = custom_tool_transition(index);
    let selected = index + 1000;
    let outputs = (0..64)
        .map(|output_index| {
            json!({
                "type": "custom_tool_call_output",
                "call_id": format!("call-{index}-{output_index}"),
                "output": [{
                    "type": "text",
                    "text": format!("still running\nSESSION_ID={selected}"),
                }],
            })
        })
        .collect::<Vec<_>>();
    row.runtime_parity_case
        .as_mut()
        .expect("parity case")
        .provider_payload = json!({
        "tools": [{"type": "custom", "name": "exec"}],
        "input": outputs,
    });
    row
}

fn collection_count_transition(request: &str, prefix: &str) -> TeacherTransition {
    collection_count_transition_n(request, prefix, 3)
}

fn collection_count_transition_n(request: &str, prefix: &str, count: usize) -> TeacherTransition {
    let mut row = transition("unused", true);
    let parity = row.runtime_parity_case.as_mut().expect("parity case");
    parity.request_text = request.to_owned();
    let rows = (0..count)
        .map(|value| json!({"value": value}))
        .collect::<Vec<_>>();
    parity.provider_payload = json!({
        "input": [{
            "type": "function_call_output",
            "output": serde_json::to_string(&rows).expect("rows serialize")
        }]
    });
    parity.expected_response = format!("{prefix}{count}.");
    row
}

fn status_transition(field: &str, code: u64) -> TeacherTransition {
    let mut row = transition(field, true);
    let parity = row.runtime_parity_case.as_mut().expect("parity case");
    parity.request_text = "Check build status".to_owned();
    parity.provider_payload = json!({
        "input": [{
            "type": "function_call_output",
            "output": format!("{{\"{field}\":{code}}}")
        }]
    });
    parity.expected_response = format!("Build status: {}.", if code == 0 { "OK" } else { "ERROR" });
    row
}

fn filter_transition(field: &str, predicate: &str) -> TeacherTransition {
    let mut row = transition(field, true);
    let parity = row.runtime_parity_case.as_mut().expect("parity case");
    parity.request_text = format!("Filter {predicate}");
    let rows = vec![
        json!({(field): predicate, "value": 3}),
        json!({(field): "other", "value": 5}),
    ];
    parity.provider_payload = json!({
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": parity.request_text.clone()
            },
            {
                "type": "function_call_output",
                "output": serde_json::to_string(&rows).expect("rows serialize")
            }
        ]
    });
    parity.expected_response = serde_json::to_string(&rows[..1]).expect("result serialize");
    row
}

fn filter_count_transition(field: &str, predicate: &str) -> TeacherTransition {
    let mut row = filter_transition(field, predicate);
    row.runtime_parity_case
        .as_mut()
        .expect("parity case")
        .expected_response = "Matching records: 1.".to_owned();
    row
}

#[test]
fn verified_scalar_trace_becomes_source_neutral_circuit_evidence() {
    let first =
        extract_live_scalar_circuit_sample(&transition("total", true)).expect("scalar trace");
    let renamed = extract_live_scalar_circuit_sample(&transition("renamed_total", true))
        .expect("renamed scalar trace");

    assert_eq!(first.bundle.roles().len(), 3);
    // Only the pre-action context/source relation is observed. The output
    // role belongs to the learned transform and must not leak backward as
    // a fabricated support relation.
    assert_eq!(first.bundle.relations().len(), 1);
    assert_eq!(first.bundle.program_atoms().len(), 1);
    assert_eq!(first.law_sha256, renamed.law_sha256);
    assert_eq!(first.anchors, renamed.anchors);
}

#[test]
fn extraction_blockers_are_attributed_to_the_teacher_action() {
    let mut row = custom_tool_transition(1);
    row.runtime_parity_case = None;
    let mut state = LiveScalarShadowState::default();
    state.observe(&row);

    let report = state.report();
    assert_eq!(
        report
            .extraction_blockers_by_action
            .get("custom_tool:exec/write_stdin")
            .and_then(|blockers| blockers.get("missingparitycase")),
        Some(&1)
    );
}

#[test]
fn verified_collection_count_trace_becomes_count_circuit_evidence() {
    let first_row = collection_count_transition("Count selected values", "Total values: ");
    let first = extract_live_scalar_circuit_sample(&first_row).expect("collection count trace");
    let renamed = extract_live_scalar_circuit_sample(&collection_count_transition(
        "How many records are present?",
        "Record count: ",
    ))
    .expect("renamed collection count trace");

    let [atom] = first.bundle.program_atoms() else {
        panic!("one count transform expected");
    };
    assert_eq!(atom.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
    assert_eq!(atom.parameter & 0x00ff, TRANSFORM_VALUE_COLLECTION);
    assert_eq!(first.anchors.len(), 1);
    assert_eq!(first.law_sha256, renamed.law_sha256);
}

#[test]
fn direct_collection_payload_becomes_count_circuit_evidence() {
    let mut row = collection_count_transition("Count the records", "Total records: ");
    row.runtime_parity_case
        .as_mut()
        .expect("parity case")
        .provider_payload = json!({
        "records": [
            {"value": 0},
            {"value": 1},
            {"value": 2}
        ]
    });

    let sample = extract_live_scalar_circuit_sample(&row).expect("direct count trace");
    let [atom] = sample.bundle.program_atoms() else {
        panic!("one count transform expected");
    };
    assert_eq!(atom.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
    assert_eq!(atom.parameter & 0x00ff, TRANSFORM_VALUE_COLLECTION);
}

#[test]
fn direct_collection_payload_without_request_becomes_count_evidence() {
    let mut row = collection_count_transition("", "Total records: ");
    row.runtime_parity_case
        .as_mut()
        .expect("parity case")
        .provider_payload = json!({"records": [{"value": 0}, {"value": 1}]});
    row.runtime_parity_case
        .as_mut()
        .expect("parity case")
        .expected_response = "Total records: 2.".to_owned();

    let sample = extract_live_scalar_circuit_sample(&row).expect("request-free count trace");
    let [atom] = sample.bundle.program_atoms() else {
        panic!("one count transform expected");
    };
    assert_eq!(atom.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
}

#[test]
fn verified_status_trace_becomes_status_circuit_evidence() {
    let first =
        extract_live_scalar_circuit_sample(&status_transition("status", 0)).expect("status trace");
    let renamed = extract_live_scalar_circuit_sample(&status_transition("renamed_code", 9))
        .expect("renamed status trace");

    let [atom] = first.bundle.program_atoms() else {
        panic!("one status transform expected");
    };
    assert_eq!(atom.opcode, TRANSFORM_OPCODE_PROJECT_STATUS);
    assert_eq!(atom.flags, TRANSFORM_STATUS_ZERO_IS_OK);
    assert_eq!(first.law_sha256, renamed.law_sha256);
}

#[test]
fn verified_filter_trace_becomes_two_role_circuit_evidence() {
    let first_row = filter_transition("kind", "active");
    let first = extract_live_scalar_circuit_sample(&first_row).expect("filter trace");
    let renamed = extract_live_scalar_circuit_sample(&filter_transition("state", "ready"))
        .expect("renamed filter trace");

    let [atom] = first.bundle.program_atoms() else {
        panic!("one filter transform expected");
    };
    assert_eq!(atom.opcode, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE);
    assert_ne!(atom.source_a_local_role, atom.source_b_local_role);
    assert_eq!(first.anchors.len(), 2);
    assert_eq!(first.law_sha256, renamed.law_sha256);
}

#[test]
fn verified_filter_count_trace_becomes_composed_circuit_evidence() {
    let first = extract_live_scalar_circuit_sample(&filter_count_transition("kind", "active"))
        .expect("filter-count trace");
    let renamed = extract_live_scalar_circuit_sample(&filter_count_transition("state", "ready"))
        .expect("renamed filter-count trace");

    let [filter, count] = first.bundle.program_atoms() else {
        panic!("filter and count transforms expected");
    };
    assert_eq!(filter.opcode, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE);
    assert_eq!(count.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
    assert_eq!(filter.output_local_role, count.source_a_local_role);
    assert_eq!(first.anchors.len(), 2);
    assert_eq!(first.law_sha256, renamed.law_sha256);
}

#[test]
fn rejected_teacher_cannot_create_wave_evidence() {
    assert_eq!(
        extract_live_scalar_circuit_sample(&transition("total", false)),
        Err(LiveScalarShadowBlocker::TeacherRejected)
    );
}

#[test]
fn multi_value_surfaces_share_one_structural_law() {
    let first = extract_live_scalar_circuit_sample(&multi_value_transition(
        "total",
        "failed",
        "Total: 7; failed: 2",
    ))
    .expect("first multi-value trace");
    let renamed = extract_live_scalar_circuit_sample(&multi_value_transition(
        "records",
        "errors",
        "Records: 7; errors: 2",
    ))
    .expect("renamed multi-value trace");

    assert_eq!(
        first.anchors.len(),
        2,
        "unexpected canonical actor: {:#?}; hypotheses: {:#?}",
        first.actor_template,
        first.actor_hypotheses
    );
    assert_eq!(
        first.bundle.roles().len(),
        5,
        "context plus two source/output role pairs"
    );
    assert_eq!(first.bundle.relations().len(), 2);
    assert_eq!(first.bundle.program_atoms().len(), 2);
    assert_eq!(first.law_sha256, renamed.law_sha256);
}

#[test]
fn historical_rebuild_never_creates_frozen_future() {
    let mut state = LiveScalarShadowState::default();
    for index in 1_u64..=(LIVE_SCALAR_MAX_EVIDENCE_ROWS as u64 + 8) {
        let mut row = transition("total", true);
        row.before.frame_id_sha256 = format!("{index:064x}");
        row.before.session_id_sha256 = format!("{:064x}", index + 100);
        state.observe_historical_support(&row);
    }

    let report = state.report();
    assert_eq!(report.support_rows, LIVE_SCALAR_MAX_EVIDENCE_ROWS);
    assert_eq!(report.future_rows, 0);
    assert_eq!(
        report
            .blockers
            .get("historicalsupportcapacityreached")
            .copied(),
        Some(8)
    );
}

#[test]
fn historical_repeated_session_remains_support_only() {
    let mut state = LiveScalarShadowState::default();
    for index in 1_u64..=8 {
        let mut row = transition("total", true);
        row.before.frame_id_sha256 = format!("{index:064x}");
        state.observe_historical_support(&row);
    }

    let report = state.report();
    assert_eq!(report.support_rows, 8);
    assert_eq!(report.future_rows, 0);
}

#[test]
fn adaptive_identification_freezes_support_without_replacement() {
    let mut state = LiveScalarShadowState::default();
    for index in 1_u64..=8 {
        let mut row = transition("total", true);
        row.before.frame_id_sha256 = format!("{index:064x}");
        row.before.session_id_sha256 = format!("{:064x}", 100 + index);
        state.observe(&row);
    }

    let report = state.report();
    assert!(report.support_rows > 0, "{report:#?}");
    assert!(report.future_rows > 0, "{report:#?}");
    assert_eq!(report.support_rows + report.future_rows, 8);
    let frozen_support = report.support_rows;
    let frozen_future = report.future_rows;

    let mut future = transition("total", true);
    future.before.frame_id_sha256 = format!("{:064x}", 200);
    future.before.session_id_sha256 = format!("{:064x}", 200);
    state.observe(&future);
    let advanced = state.report();
    assert_eq!(advanced.support_rows, frozen_support);
    assert_eq!(advanced.future_rows, frozen_future + 1);
}

#[test]
fn simple_law_reaches_admission_with_one_support_and_one_future() {
    let mut state = LiveScalarShadowState::default();
    let mut support = transition("total", true);
    support.before.frame_id_sha256 = format!("{:064x}", 1);
    support.before.session_id_sha256 = format!("{:064x}", 101);
    support.before.observed_at_unix_nanos = 1;
    state.observe(&support);

    let mut future = transition("renamed_total", true);
    future.before.frame_id_sha256 = format!("{:064x}", 2);
    future.before.session_id_sha256 = format!("{:064x}", 102);
    future.before.observed_at_unix_nanos = 2;
    state.observe(&future);

    let report = state.report();
    assert_eq!(report.support_rows, 1, "{report:#?}");
    assert_eq!(report.future_rows, 1, "{report:#?}");
    assert_eq!(report.candidate_freezes, 1, "{report:#?}");
    assert_eq!(report.transfer_proofs, 1, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");
}

#[test]
fn additive_merge_keeps_active_crystallized_generation_when_only_evidence_grows() {
    let mut state = LiveScalarShadowState::default();
    let mut support = transition("total", true);
    support.before.frame_id_sha256 = format!("{:064x}", 1);
    support.before.session_id_sha256 = format!("{:064x}", 101);
    support.before.observed_at_unix_nanos = 1;
    state.observe(&support);

    let mut future = transition("renamed_total", true);
    future.before.frame_id_sha256 = format!("{:064x}", 2);
    future.before.session_id_sha256 = format!("{:064x}", 102);
    future.before.observed_at_unix_nanos = 2;
    state.observe(&future);
    let active = crate::build_crystallized_admission_snapshot(
        &state.admission_candidates(),
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("active admission")
    .expect("active candidate");

    for index in 3_u64..=8 {
        let mut row = transition(&format!("surface_{index}"), true);
        row.before.frame_id_sha256 = format!("{index:064x}");
        row.before.event_id_sha256 = format!("{:064x}", index + 100);
        row.before.client_intent_id_sha256 = format!("{:064x}", index + 200);
        row.before.session_id_sha256 = format!("{:064x}", index + 300);
        row.before.evidence_ref_sha256 = format!("{:064x}", index + 400);
        row.before.observed_at_unix_nanos = index;
        row.outcome.verifier.evidence_ref_sha256 = format!("{:064x}", index + 500);
        row.runtime_parity_case
            .as_mut()
            .expect("parity")
            .evidence_ref_sha256 = format!("{:064x}", index + 600);
        state.observe(&row);
    }
    let candidate = crate::build_crystallized_admission_snapshot(
        &state.admission_candidates(),
        "test-project",
        2,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("updated admission")
    .expect("updated candidate");
    let active_package = active.registry.packages[0].clone();
    let candidate_package = &candidate.registry.packages[0];
    assert_eq!(active_package.package_id, candidate_package.package_id);
    assert_eq!(
        active_package, *candidate_package,
        "monitoring evidence must not rewrite a sealed minimal proof basis"
    );
    let merged = crate::merge_with_active_online_admission(
        candidate,
        active.registry,
        active.admission,
        "test-project",
        &"a".repeat(64),
        &"b".repeat(64),
        100,
        30,
    )
    .expect("additive merge");
    assert_eq!(merged.registry.packages, [active_package]);
}

#[test]
fn proven_active_merge_replaces_same_law_with_crystallized_runtime() {
    let mut state = LiveScalarShadowState::default();
    let mut support = transition("total", true);
    support.before.frame_id_sha256 = format!("{:064x}", 1);
    support.before.session_id_sha256 = format!("{:064x}", 101);
    support.before.observed_at_unix_nanos = 1;
    state.observe(&support);

    let mut future = transition("renamed_total", true);
    future.before.frame_id_sha256 = format!("{:064x}", 2);
    future.before.session_id_sha256 = format!("{:064x}", 102);
    future.before.observed_at_unix_nanos = 2;
    state.observe(&future);

    let candidate = crate::build_crystallized_admission_snapshot(
        &state.admission_candidates(),
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("crystallized admission")
    .expect("candidate");
    let binding = &candidate.admission.response_authority.packages[0];
    let package_id = candidate.registry.packages[0].package_id.clone();
    let mut legacy_registry = candidate.registry.clone();
    legacy_registry.packages[0].crystallized_operator = None;
    let legacy_admission = crate::authority::build_composite_admission_for_registry(
        &legacy_registry,
        std::collections::BTreeMap::from([(
            package_id,
            (
                binding.support_manifest_sha256.clone(),
                binding.exact_causal_proof_sha256.clone(),
                binding.runtime_parity_receipt_set_sha256.clone(),
                binding.future_verifier_receipt_set_sha256.clone(),
                binding.semantic_alias_proof_sha256.clone(),
            ),
        )]),
        "test-project",
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
        "missing receipts",
        "missing verifier",
    )
    .expect("legacy proof material");

    let merged = crate::merge_with_proven_active_online_admission(
        candidate,
        legacy_registry,
        legacy_admission,
        "test-project",
        &"a".repeat(64),
        &"b".repeat(64),
        100,
        30,
    )
    .expect("crystallized upgrade");

    assert_eq!(merged.registry.packages.len(), 1);
    assert!(merged.registry.packages[0].crystallized_operator.is_some());
    crate::ResponseExecutor::from_registry_with_admission(
        merged.registry,
        merged.admission,
        "test-project",
        &"a".repeat(64),
        &"b".repeat(64),
        100,
        30,
    )
    .expect("reissued authority executes");
}

#[test]
fn distinct_future_frames_may_share_sessions_without_crossing_support_boundary() {
    let mut state = LiveScalarShadowState::default();
    for index in 1_u64..=8 {
        let mut row = transition("total", true);
        row.before.frame_id_sha256 = format!("{index:064x}");
        row.before.session_id_sha256 = format!("{:064x}", index);
        row.before.observed_at_unix_nanos = index;
        state.observe_historical_support(&row);
    }
    for index in 1_u64..=8 {
        let mut row = transition("total", true);
        row.before.frame_id_sha256 = format!("{:064x}", 100 + index);
        row.before.session_id_sha256 = format!("{:064x}", 101 + index % 3);
        row.before.observed_at_unix_nanos = 100 + index;
        state.observe(&row);
    }

    assert_eq!(state.laws.len(), 1);
    let law = state.laws.values().next().expect("one structural law");
    assert_eq!(law.support.len(), 8);
    assert_eq!(law.future.len(), 8);
    assert!(
        !state
            .blockers
            .contains_key(&LiveScalarShadowBlocker::SupportFutureSessionOverlap)
    );
    let report = state.report();
    assert_eq!(report.full_phase_winners, 1, "{report:#?}");
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows);
    assert_eq!(report.admission_candidates, 1, "{report:#?}");
    let candidate = state
        .admission_candidates()
        .into_iter()
        .next()
        .expect("repeated future sessions retain per-surface parity");
    let mut relabeled = candidate.clone();
    std::mem::swap(&mut relabeled.support[0], &mut relabeled.future[0]);
    assert!(matches!(
        crate::build_crystallized_admission_snapshot(
            &[relabeled],
            "test-project",
            1,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        ),
        Err("crystallized_evidence_partition_reordered")
    ));
    let snapshot = crate::build_crystallized_admission_snapshot(
        &[candidate],
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("external admission repeats all future surfaces");
    assert!(snapshot.is_some());
}

#[test]
fn single_primary_sequence_normalizes_to_typed_template() {
    let renderer = CollectionOutputRenderer::RenderSequence {
        segments: vec![
            ResponseRenderSegment::Static {
                text: "Total records: ".to_owned(),
            },
            ResponseRenderSegment::Primary,
            ResponseRenderSegment::Static {
                text: ".".to_owned(),
            },
        ],
    };
    assert_eq!(
        normalized_scalar_renderer(&renderer),
        Some(CollectionOutputRenderer::RenderTemplate {
            prefix: "Total records: ".to_owned(),
            suffix: ".".to_owned(),
        })
    );
}

#[test]
fn templated_live_rows_reach_verified_scalar_shadow_operator() {
    let mut state = LiveScalarShadowState::default();
    for index in 0..64_u8 {
        let mut row = transition(&format!("field_{index}"), true);
        row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
        row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
        row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
        row.before.observed_at_unix_nanos = u64::from(index) + 1;
        let parity = row.runtime_parity_case.as_mut().expect("parity");
        if index % 2 == 0 {
            if index % 4 == 0 {
                parity.request_text.clear();
            } else {
                parity.request_text = "Summarize the result".to_owned();
            }
            parity.provider_payload = json!({format!("field_{index}"): index + 7});
        } else {
            parity.provider_payload = json!({
                "input": [{
                    "type": "function_call_output",
                    "output": format!("{{\"field_{index}\":{}}}", index + 7)
                }]
            });
        }
        row.runtime_parity_case
            .as_mut()
            .expect("parity")
            .expected_response = format!("Total records: {}.", index + 7);
        state.observe(&row);
    }

    let report = state.report();
    assert_eq!(report.executable, 64, "{report:#?}");
    assert!(report.support_rows > 0, "{report:#?}");
    assert!(report.future_rows > 0, "{report:#?}");
    assert_eq!(report.frozen_laws, 1, "{report:#?}");
    assert!(report.ingest_accounting_complete, "{report:#?}");
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");
    assert_eq!(report.laws.len(), 1, "{report:#?}");
    assert_eq!(report.laws[0].teacher_action_symbol, "response");
    assert_eq!(report.laws[0].operation_kind, "project");
    assert_eq!(report.laws[0].support_rows, report.support_rows);
    assert_eq!(report.laws[0].future_rows, report.future_rows);

    let candidates = state.admission_candidates();
    assert_eq!(candidates.len(), 1, "{report:#?}");
    let snapshot = crate::build_crystallized_admission_snapshot(
        &candidates,
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("external admission evaluates sealed operator")
    .expect("sealed operator reaches registry");
    assert_eq!(snapshot.registry.packages.len(), 1);
    let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
        .expect("registry restores crystallized operator");
    let execution = executor.execute_shadow(
        "Return the count",
        &json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"new_surface_total\":91}"
            }]
        }),
    );
    assert_eq!(
        execution.response.as_deref(),
        Some("Total records: 91."),
        "{execution:#?}"
    );
    let ambiguous = executor.execute_shadow(
        "Return the count",
        &json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"left\":91,\"right\":92}"
            }]
        }),
    );
    assert_eq!(ambiguous.status, crate::ResponseExecutionStatus::Abstain);
    let incompatible = executor.execute_shadow(
        "",
        &json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"new_surface_total\":true}"
            }]
        }),
    );
    assert_eq!(incompatible.status, crate::ResponseExecutionStatus::Abstain);

    let mut tampered_support = candidates.clone();
    tampered_support[0].support[0]
        .runtime_parity_case
        .as_mut()
        .expect("support parity")
        .expected_response = "999".to_owned();
    assert!(
        crate::build_crystallized_admission_snapshot(
            &tampered_support,
            "test-project",
            2,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .is_err()
    );

    let mut tampered_seal = candidates;
    tampered_seal[0].executable_parity_seal_sha256 = "c".repeat(64);
    assert!(matches!(
        crate::build_crystallized_admission_snapshot(
            &tampered_seal,
            "test-project",
            3,
            100,
            30,
            &"a".repeat(64),
            &"b".repeat(64),
        ),
        Err("crystallized_admission_resynthesis_mismatch")
    ));
}

#[test]
fn minimal_transfer_basis_is_stable_under_future_reservoir_order() {
    let mut state = LiveScalarShadowState::default();
    let mut support = transition("total", true);
    support.before.frame_id_sha256 = format!("{:064x}", 1);
    support.before.session_id_sha256 = format!("{:064x}", 101);
    support.before.observed_at_unix_nanos = 1;
    state.observe(&support);
    for index in 2_u64..=5 {
        let mut future = transition(&format!("renamed_{index}"), true);
        future.before.frame_id_sha256 = format!("{index:064x}");
        future.before.session_id_sha256 = format!("{:064x}", 100 + index);
        future.before.observed_at_unix_nanos = index;
        state.observe(&future);
    }

    let forward = state.admission_candidates();
    let mut reversed = state.clone();
    reversed
        .laws
        .values_mut()
        .for_each(|law| law.future.reverse());
    assert_eq!(reversed.admission_candidates(), forward);
    assert_eq!(forward[0].future.len(), 1);
}

#[test]
fn out_of_scope_future_is_monitored_as_applicability_negative() {
    let mut state = LiveScalarShadowState::default();
    let mut support = transition("total", true);
    support.before.frame_id_sha256 = format!("{:064x}", 1);
    support.before.session_id_sha256 = format!("{:064x}", 101);
    support.before.observed_at_unix_nanos = 1;
    state.observe(&support);

    let mut future = transition("renamed_total", true);
    future.before.frame_id_sha256 = format!("{:064x}", 2);
    future.before.session_id_sha256 = format!("{:064x}", 102);
    future.before.observed_at_unix_nanos = 2;
    state.observe(&future);

    let mut outside_scope = transition("another_total", true);
    outside_scope.before.frame_id_sha256 = format!("{:064x}", 3);
    outside_scope.before.session_id_sha256 = format!("{:064x}", 103);
    outside_scope.before.observed_at_unix_nanos = 3;
    state.observe(&outside_scope);
    let law = state.laws.values_mut().next().expect("one law");
    law.future[1]
        .runtime_parity_case
        .as_mut()
        .expect("parity")
        .provider_payload = json!({
        "input": [{
            "type": "function_call_output",
            "output": "{\"left\":7,\"right\":8}"
        }]
    });

    let report = state.report();
    assert_eq!(report.admission_candidates, 1, "{report:#?}");
    assert_eq!(report.transfer_basis_rows, 1, "{report:#?}");
    assert_eq!(report.future_applicability_negatives, 1, "{report:#?}");
    assert_eq!(report.monitored_future_rows, 0, "{report:#?}");
}

#[test]
fn typed_custom_tool_rows_reach_verified_crystallized_operator() {
    let mut state = LiveScalarShadowState::default();
    for index in 0..64_u64 {
        let mut row = custom_tool_transition(index);
        row.before.frame_id_sha256 = format!("{:064x}", index + 1);
        row.before.session_id_sha256 = format!("{:064x}", index + 101);
        row.before.client_intent_id_sha256 = format!("{:064x}", index + 201);
        row.before.observed_at_unix_nanos = index + 1;
        state.observe(&row);
    }

    let report = state.report();
    assert_eq!(report.executable, 64, "{report:#?}");
    assert!(report.support_rows > 0, "{report:#?}");
    assert!(report.future_rows > 0, "{report:#?}");
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");

    let candidates = state.admission_candidates();
    let snapshot = crate::build_crystallized_admission_snapshot(
        &candidates,
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("external admission evaluates sealed operator")
    .expect("sealed operator reaches registry");
    let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
        .expect("registry restores typed call operator");
    let runtime = custom_tool_transition(777);
    let parity = runtime.runtime_parity_case.expect("runtime parity");
    let execution = executor.execute_shadow(&parity.request_text, &parity.provider_payload);
    assert!(
        execution.response.as_deref().is_some_and(|response| {
            crate::online_admission::responses_match_after_execution_budget_normalization(
                response,
                &parity.expected_response,
            )
        }),
        "{execution:#?}"
    );
}

#[test]
fn repeated_physical_call_roles_collapse_before_authority_budget() {
    let broad =
        extract_live_scalar_circuit_sample(&custom_tool_transition_with_repeated_outputs(1))
            .expect("bounded physical adapter space");
    assert!(
        broad.actor_hypotheses.len() > crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
        "the fixture must exercise the intermediate version space"
    );
    assert!(broad.actor_hypotheses.len() <= TEACHER_CALL_SELECTOR_BUDGET);

    let narrow = extract_live_scalar_circuit_sample(&custom_tool_transition(2))
        .expect("independent narrow surface");
    let mut law = LiveScalarLawState::default();
    update_support_actor_hypotheses(&mut law, &broad.actor_hypotheses)
        .expect("first surface initializes the version space");
    update_support_actor_hypotheses(&mut law, &narrow.actor_hypotheses)
        .expect("second surface extends the bounded adapter space");

    assert!(!law.support_actor_hypotheses.is_empty());
    assert!(law.support_actor_hypotheses.len() <= TEACHER_CALL_SELECTOR_BUDGET);
    let quotient = common_support_actor_hypotheses(&[broad, narrow])
        .expect("physical adapters share one semantic actor law");
    assert_eq!(quotient.len(), 1);
}

#[test]
fn bounded_active_turn_payload_above_64k_remains_training_evidence() {
    let mut row = custom_tool_transition(9);
    let parity = row.runtime_parity_case.as_mut().expect("parity case");
    parity.provider_payload["bounded_metadata"] = Value::String("x".repeat(70 * 1024));
    let bytes = serde_json::to_vec(&parity.provider_payload).expect("payload encoding");
    assert!(bytes.len() > 64 * 1024);
    assert!(bytes.len() <= LIVE_SCALAR_MAX_PROVIDER_PAYLOAD_BYTES);

    extract_live_scalar_circuit_sample(&row)
        .expect("capture-bounded active turn remains executable evidence");
}

#[test]
fn all_teacher_field_selectors_can_become_structural_hints() {
    for selector in [
        ResponseValueSelector::JsonField {
            field: "opaque".to_owned(),
            value_type: AtomValueType::Integer,
        },
        ResponseValueSelector::UniqueTurnJsonField {
            field: "opaque".to_owned(),
            value_type: AtomValueType::Integer,
        },
        ResponseValueSelector::UniqueActiveTurnJsonField {
            field: "opaque".to_owned(),
            value_type: AtomValueType::Integer,
        },
    ] {
        assert_eq!(
            teacher_field_selector_hint(&selector),
            Some(("opaque", AtomValueType::Integer))
        );
    }
}

#[test]
fn count_rows_reach_verified_cpu_operator() {
    let mut state = LiveScalarShadowState::default();
    for index in 0..64_u8 {
        let count = usize::from(index % 7) + 1;
        let mut row = collection_count_transition_n("Count the records", "Total records: ", count);
        row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
        row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
        row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
        row.before.observed_at_unix_nanos = u64::from(index) + 1;
        state.observe(&row);
    }

    let report = state.report();
    assert!(report.support_rows > 0, "{report:#?}");
    assert!(report.future_rows > 0, "{report:#?}");
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");

    let candidates = state.admission_candidates();
    let snapshot = crate::build_crystallized_admission_snapshot(
        &candidates,
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("external admission evaluates count operator")
    .expect("count operator reaches registry");
    let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
        .expect("registry restores count operator");
    let rows = (0..11)
        .map(|value| json!({"id": value}))
        .collect::<Vec<_>>();
    let execution = executor.execute_shadow(
        "Count the records",
        &json!({
            "input": [{
                "type": "function_call_output",
                "output": serde_json::to_string(&rows).expect("rows serialize")
            }]
        }),
    );
    assert_eq!(
        execution.response.as_deref(),
        Some("Total records: 11."),
        "{execution:#?}"
    );
}

#[test]
fn status_rows_reach_verified_cpu_operator() {
    let mut state = LiveScalarShadowState::default();
    for index in 0..64_u8 {
        let mut row = status_transition(&format!("status_{index}"), u64::from(index % 5));
        row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
        row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
        row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
        row.before.observed_at_unix_nanos = u64::from(index) + 1;
        state.observe(&row);
    }

    let report = state.report();
    assert!(report.support_rows > 0, "{report:#?}");
    assert!(report.future_rows > 0, "{report:#?}");
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");

    let candidates = state.admission_candidates();
    let snapshot = crate::build_crystallized_admission_snapshot(
        &candidates,
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("external admission evaluates status operator")
    .expect("status operator reaches registry");
    let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
        .expect("registry restores status operator");
    let execution = executor.execute_shadow(
        "Check build status",
        &json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"new_status_field\":0}"
            }]
        }),
    );
    assert_eq!(
        execution.response.as_deref(),
        Some("Build status: OK."),
        "{execution:#?}"
    );
}

#[test]
fn filter_rows_reach_verified_cpu_operator() {
    let mut state = LiveScalarShadowState::default();
    for index in 0..64_u8 {
        let predicate = if index % 2 == 0 { "active" } else { "ready" };
        let mut row = filter_transition(&format!("state_{index}"), predicate);
        row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
        row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
        row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
        row.before.observed_at_unix_nanos = u64::from(index) + 1;
        state.observe(&row);
    }

    let report = state.report();
    assert!(report.support_rows > 0, "{report:#?}");
    assert!(report.future_rows > 0, "{report:#?}");
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");

    let candidates = state.admission_candidates();
    let snapshot = crate::build_crystallized_admission_snapshot(
        &candidates,
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("external admission evaluates filter operator")
    .expect("filter operator reaches registry");
    let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
        .expect("registry restores filter operator");
    let rows = vec![
        json!({"new_kind": "active", "score": 11}),
        json!({"new_kind": "idle", "score": 12}),
    ];
    let payload = json!({
        "input": [
            {"type": "message", "role": "user", "content": "Filter active"},
            {
                "type": "function_call_output",
                "output": serde_json::to_string(&rows).expect("rows serialize")
            }
        ]
    });
    let execution = executor.execute_shadow("Filter active", &payload);
    assert_eq!(
        execution.response.as_deref(),
        Some("[{\"new_kind\":\"active\",\"score\":11}]"),
        "{execution:#?}"
    );
}

#[test]
fn filter_count_rows_reach_verified_cpu_operator() {
    let mut state = LiveScalarShadowState::default();
    for index in 0..64_u8 {
        let predicate = if index % 2 == 0 { "active" } else { "ready" };
        let mut row = filter_count_transition(&format!("state_{index}"), predicate);
        row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
        row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
        row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
        row.before.observed_at_unix_nanos = u64::from(index) + 1;
        state.observe(&row);
    }

    let report = state.report();
    assert!(report.support_rows > 0, "{report:#?}");
    assert!(report.future_rows > 0, "{report:#?}");
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");

    let snapshot = crate::build_crystallized_admission_snapshot(
        &state.admission_candidates(),
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("external admission evaluates composed operator")
    .expect("composed operator reaches registry");
    let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
        .expect("registry restores composed operator");
    let rows = vec![
        json!({"new_kind": "active", "score": 11}),
        json!({"new_kind": "idle", "score": 12}),
    ];
    let payload = json!({
        "input": [
            {"type": "message", "role": "user", "content": "Filter active"},
            {
                "type": "function_call_output",
                "output": serde_json::to_string(&rows).expect("rows serialize")
            }
        ]
    });
    let execution = executor.execute_shadow("Filter active", &payload);
    assert_eq!(
        execution.response.as_deref(),
        Some("Matching records: 1."),
        "{execution:#?}"
    );
}

#[test]
fn multi_role_rows_reach_verified_crystallized_operator() {
    let mut state = LiveScalarShadowState::default();
    for index in 0..64_u8 {
        let first = format!("total_{index}");
        let second = format!("failed_{index}");
        let first_value = u16::from(index) + 100;
        let second_value = if index < 32 {
            first_value
        } else {
            u16::from(index) + 10
        };
        let mut row = multi_value_transition(
            &first,
            &second,
            &format!("Total: {first_value}; failed: {second_value}"),
        );
        row.before.frame_id_sha256 = format!("{index:02x}").repeat(32);
        row.before.session_id_sha256 = format!("{:02x}", index + 16).repeat(32);
        row.before.client_intent_id_sha256 = format!("{:02x}", index + 32).repeat(32);
        row.before.observed_at_unix_nanos = u64::from(index) + 1;
        row.runtime_parity_case
            .as_mut()
            .expect("parity")
            .provider_payload = json!({
            "input": [{
                "type": "function_call_output",
                "output": format!(
                    "{{\"{first}\":{first_value},\"{second}\":{second_value}}}"
                )
            }]
        });
        if index == 0 {
            let parity = row.runtime_parity_case.as_ref().expect("parity");
            let observed = crate::runtime::observed_request_ordinal_roles(
                &parity.request_text,
                &parity.provider_payload,
            )
            .expect("observed equal roles");
            assert_eq!(observed.len(), 2, "both JSON paths must remain observable");
            let sample = extract_live_scalar_circuit_sample(&row).expect("equal support sample");
            let expanded = bounded_ordinal_role_permutations(
                &sample.actor_template,
                &parity.request_text,
                &parity.provider_payload,
                &parity.expected_response,
            )
            .expect("bounded equal-role expansion");
            assert_eq!(
                expanded.len(),
                2,
                "renderer must retain both executable role orders: {expanded:#?}"
            );
            assert_eq!(
                sample.actor_hypotheses.len(),
                3,
                "equal-value support must retain repeated-role plus both role orders: {:#?}",
                sample.actor_hypotheses
            );
        }
        state.observe(&row);
    }

    let report = state.report();
    assert_eq!(report.executable, 64, "{report:#?}");
    assert!(report.support_rows > 0, "{report:#?}");
    assert!(report.future_rows > 0, "{report:#?}");
    assert_eq!(report.frozen_laws, 1, "{report:#?}");
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");

    let candidates = state.admission_candidates();
    let bundle = candidates[0]
        .package
        .crystallized_operator
        .as_ref()
        .expect("restart bundle");
    let restored =
        crate::VerifiedCrystallizedOperator::restore(bundle.page_bytes(), bundle.registry_cbor())
            .expect("restore rich operator before admission");
    for (index, row) in candidates[0]
        .support
        .iter()
        .chain(&candidates[0].future)
        .enumerate()
    {
        let parity = row.runtime_parity_case.as_ref().expect("parity row");
        let bound = restored
            .bind_pre_action(&parity.request_text, &parity.provider_payload)
            .unwrap_or_else(|error| panic!("rich bind row {index}: {error:?}"));
        let response = bound
            .execute_verified()
            .unwrap_or_else(|error| panic!("rich execute row {index}: {error:?}"));
        assert_eq!(response, parity.expected_response, "rich row {index}");
    }
    let snapshot = crate::build_crystallized_admission_snapshot(
        &candidates,
        "test-project",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("external admission verifies rich operator")
    .expect("rich operator reaches registry");
    let executor = crate::ResponseExecutor::from_registry(snapshot.registry)
        .expect("hot executor restores rich operator");
    let execution = executor.execute_shadow(
        "Return new_total and new_failed",
        &json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"new_total\":777,\"new_failed\":9}"
            }]
        }),
    );
    assert_eq!(
        execution.response.as_deref(),
        Some("Total: 777; failed: 9"),
        "{execution:#?}"
    );
    let reversed = executor.execute_shadow(
        "Return new_failed and new_total",
        &json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"new_total\":777,\"new_failed\":9}"
            }]
        }),
    );
    assert_eq!(
        reversed.response.as_deref(),
        Some("Total: 9; failed: 777"),
        "request ordinal, not field name or JSON order, owns the role: {reversed:#?}"
    );
}
