use nando_response_actor::{
    CollectionOutputRenderer, CollectionProgramStep, ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt,
    LiveScalarAdmissionCandidate, LiveScalarShadowState, RUNTIME_FRAME_SCHEMA_V1,
    ResponseOperation, ResponsePackageOrigin, ResponsePackageState, ResponseRenderSegment,
    RuntimeFrame, RuntimeParityCase, TEACHER_OUTCOME_SCHEMA_V1, TEACHER_TRANSITION_SCHEMA_V1,
    TRANSFORM_OPCODE_COUNT_COLLECTION, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TeacherActionAst, TeacherOutcome, TeacherTransition,
    TeacherVerifierEvidence, VerifiedCrystallizedOperator, extract_live_scalar_circuit_sample,
};
use serde_json::json;

const SHADOW_ROWS: u8 = 64;

fn commitment(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn verified_transition(
    index: u8,
    request_text: String,
    provider_payload: serde_json::Value,
    expected_response: String,
) -> TeacherTransition {
    TeacherTransition {
        schema: TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
        before: RuntimeFrame {
            schema: RUNTIME_FRAME_SCHEMA_V1.to_owned(),
            frame_id_sha256: commitment(index.saturating_add(65)),
            event_id_sha256: commitment(201),
            client_intent_id_sha256: commitment(index.saturating_add(129)),
            session_id_sha256: commitment(index.saturating_add(1)),
            observed_at_unix_nanos: u64::from(index) + 1,
            extractor_version: "ms5-ms6-capability-shadow-v1".to_owned(),
            atoms: Vec::new(),
            evidence_ref_sha256: commitment(202),
        },
        outcome: TeacherOutcome {
            schema: TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
            action: TeacherActionAst {
                signature_sha256: commitment(203),
                action_symbol: "response".to_owned(),
                atoms: Vec::new(),
            },
            verifier: TeacherVerifierEvidence {
                accepted: true,
                evidence_ref_sha256: commitment(204),
                output_digest_sha256: commitment(205),
            },
            completed_at_unix_nanos: u64::from(index) + 1_000,
        },
        economics: Some(EconomicsReceipt {
            schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
            exact_input_tokens: 100,
            ordinary: true,
            controlled: false,
            replay: false,
            dedupe_eligible: true,
            provider_evidence_ref_sha256: commitment(206),
        }),
        runtime_parity_case: Some(RuntimeParityCase {
            evidence_ref_sha256: commitment(index.saturating_add(1)),
            capture_receipt: None,
            request_text,
            provider_payload,
            expected_response,
        }),
    }
}

fn rich_render_sequence_transition(index: u8) -> TeacherTransition {
    let first_field = format!("total_{index}");
    let second_field = format!("failed_{index}");
    let first_value = u16::from(index) + 100;
    let second_value = if index < SHADOW_ROWS / 2 {
        first_value
    } else {
        u16::from(index) + 10
    };
    verified_transition(
        index,
        format!("Return {first_field} and {second_field}"),
        json!({
            "input": [{
                "type": "function_call_output",
                "output": format!(
                    "{{\"{first_field}\":{first_value},\"{second_field}\":{second_value}}}"
                )
            }]
        }),
        format!("Total: {first_value}; failed: {second_value}"),
    )
}

fn filter_count_transition(index: u8) -> TeacherTransition {
    let field = format!("state_{index}");
    let predicate = if index.is_multiple_of(2) {
        "active"
    } else {
        "ready"
    };
    let rows = vec![
        json!({(field.clone()): predicate, "value": 3}),
        json!({(field): "other", "value": 5}),
    ];
    let request_text = format!("Filter {predicate}");
    verified_transition(
        index,
        request_text.clone(),
        json!({
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": request_text
                },
                {
                    "type": "function_call_output",
                    "output": serde_json::to_string(&rows).expect("rows serialize")
                }
            ]
        }),
        "Matching records: 1.".to_owned(),
    )
}

fn prove_shadow_lineage(
    rows: impl IntoIterator<Item = TeacherTransition>,
    operation_kind: &str,
) -> LiveScalarAdmissionCandidate {
    let mut state = LiveScalarShadowState::default();
    for row in rows {
        state.observe(&row);
    }

    let report = state.report();
    assert_eq!(report.law_count, 1, "{report:#?}");
    assert_eq!(report.full_phase_winners, 1, "{report:#?}");
    assert_eq!(
        report.causal_control_passes, 1,
        "the same frozen lineage must make no-phase, shuffled-phase, \
         magnitude-only, and matched-random-center controls abstain: {report:#?}"
    );
    assert_eq!(report.verified_shadow_operators, 1, "{report:#?}");
    assert_eq!(report.shadow_executions, report.future_rows, "{report:#?}");
    assert_eq!(report.admission_candidates, 1, "{report:#?}");
    assert!(report.ingest_accounting_complete, "{report:#?}");
    assert_eq!(report.laws[0].operation_kind, operation_kind);

    let mut candidates = state.admission_candidates();
    assert_eq!(candidates.len(), 1, "{report:#?}");
    let candidate = candidates.remove(0);
    candidate
        .verify_evidence_partition()
        .expect("support/future partition remains sealed");
    candidate.package.validate().expect("valid shadow package");
    assert_eq!(
        candidate.package.origin,
        ResponsePackageOrigin::GroundedSynthesis
    );
    assert_eq!(candidate.package.state, ResponsePackageState::Quarantine);
    assert_eq!(
        candidate.package.admission_candidate_blocker(),
        Some("package_not_active")
    );
    assert!(!candidate.package.eligible_for_admission_candidate());
    assert!(!candidate.package.eligible_for_local_accept());
    assert!(candidate.package.proof.wave_causal_pass);
    assert_eq!(candidate.package.proof.wrong_accepts, 0);
    assert_eq!(candidate.package.proof.runtime_parity_failures, 0);
    candidate
}

fn restore_shadow_operator(
    candidate: &LiveScalarAdmissionCandidate,
) -> VerifiedCrystallizedOperator {
    let restart = candidate
        .package
        .crystallized_operator
        .as_ref()
        .expect("shadow candidate owns a restart bundle");
    assert!(restart.has_canonical_bundle_v4());
    let restored = restart
        .restore_verified()
        .expect("restart verifies the crystallized operator");
    assert_eq!(restart.page_bytes(), restored.page().as_bytes());
    assert_eq!(
        restored
            .routing_program()
            .expect("restored routing program"),
        candidate.package.program
    );
    assert_eq!(restored.parity_seal().wrong_accepts(), 0);
    assert!(!restored.verified_future_lineages().is_empty());
    restored
}

#[test]
fn ms5_rich_two_role_render_sequence_crystallizes_restarts_and_executes_in_shadow() {
    let extraction = extract_live_scalar_circuit_sample(&rich_render_sequence_transition(0))
        .expect("verified rich transition extracts");
    assert_eq!(extraction.anchors.len(), 2);
    assert_eq!(extraction.bundle.program_atoms().len(), 2);
    assert!(
        extraction
            .bundle
            .program_atoms()
            .iter()
            .all(|atom| atom.opcode == TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR)
    );

    let candidate = prove_shadow_lineage(
        (0..SHADOW_ROWS).map(rich_render_sequence_transition),
        "project",
    );
    let segments = match &candidate.package.program.operation {
        ResponseOperation::ProjectSelectedValue {
            renderer: CollectionOutputRenderer::RenderSequence { segments },
            ..
        } => segments,
        operation => panic!("expected rich RenderSequence, got {operation:#?}"),
    };
    assert_eq!(
        segments
            .iter()
            .filter(|segment| {
                matches!(
                    segment,
                    ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. }
                )
            })
            .count(),
        2
    );
    assert!(
        segments
            .iter()
            .any(|segment| matches!(segment, ResponseRenderSegment::Primary))
    );
    assert!(
        segments
            .iter()
            .any(|segment| matches!(segment, ResponseRenderSegment::Selected { .. }))
    );

    let restored = restore_shadow_operator(&candidate);
    let header = restored.page().header().expect("valid generated page");
    assert_eq!(header.transform_count, 2);
    assert_eq!(header.composition_node_count, 0);
    let first = restored.page().transform(0).expect("first role projection");
    let second = restored
        .page()
        .transform(1)
        .expect("second role projection");
    assert_eq!(first.opcode, TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR);
    assert_eq!(second.opcode, TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR);
    assert_ne!(first.source_a, second.source_a);
    assert_ne!(first.output, second.output);

    let payload = json!({
        "input": [{
            "type": "function_call_output",
            "output": "{\"new_total\":777,\"new_failed\":9}"
        }]
    });
    let response = restored
        .bind_pre_action("Return new_total and new_failed", &payload)
        .expect("ground unseen rich roles")
        .execute_verified()
        .expect("verify unseen rich response");
    assert_eq!(response, "Total: 777; failed: 9");

    let reversed = restored
        .bind_pre_action("Return new_failed and new_total", &payload)
        .expect("ground permuted rich roles")
        .execute_verified()
        .expect("verify permuted rich response");
    assert_eq!(reversed, "Total: 9; failed: 777");
}

#[test]
fn ms6_filter_count_typed_chain_crystallizes_restarts_and_executes_in_shadow() {
    let extraction = extract_live_scalar_circuit_sample(&filter_count_transition(0))
        .expect("verified filter-count transition extracts");
    let [filter_atom, count_atom] = extraction.bundle.program_atoms() else {
        panic!("filter and count typed atoms required");
    };
    assert_eq!(filter_atom.opcode, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE);
    assert_eq!(count_atom.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
    assert_eq!(
        filter_atom.output_local_role,
        count_atom.source_a_local_role
    );
    assert_eq!(extraction.anchors.len(), 2);

    let candidate = prove_shadow_lineage(
        (0..SHADOW_ROWS).map(filter_count_transition),
        "filter_count",
    );
    match &candidate.package.program.operation {
        ResponseOperation::ComposeCollection {
            steps, renderer, ..
        } => {
            assert!(matches!(
                steps.as_slice(),
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. },
                    CollectionProgramStep::Count
                ]
            ));
            assert!(matches!(
                renderer,
                CollectionOutputRenderer::RenderTemplate { prefix, suffix }
                    if prefix == "Matching records: " && suffix == "."
            ));
        }
        operation => panic!("expected filter-count collection program, got {operation:#?}"),
    }

    let restored = restore_shadow_operator(&candidate);
    let header = restored.page().header().expect("valid generated page");
    assert_eq!(header.transform_count, 2);
    assert_eq!(header.composition_node_count, 1);
    let filter = restored.page().transform(0).expect("FILTER transform");
    let count = restored.page().transform(1).expect("COUNT transform");
    assert_eq!(filter.opcode, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE);
    assert_eq!(count.opcode, TRANSFORM_OPCODE_COUNT_COLLECTION);
    assert_eq!(filter.output, count.source_a);

    let rows = vec![
        json!({"new_kind": "active", "score": 11}),
        json!({"new_kind": "idle", "score": 12}),
        json!({"new_kind": "active", "score": 13}),
    ];
    let payload = json!({
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": "Filter active"
            },
            {
                "type": "function_call_output",
                "output": serde_json::to_string(&rows).expect("rows serialize")
            }
        ]
    });
    let response = restored
        .bind_pre_action("Filter active", &payload)
        .expect("ground unseen collection and predicate")
        .execute_verified()
        .expect("verify unseen filter-count response");
    assert_eq!(response, "Matching records: 2.");
}
