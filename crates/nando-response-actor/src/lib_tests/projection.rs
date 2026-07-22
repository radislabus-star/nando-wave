//! Projection, status, and generic collection test families.

use super::*;

fn selected_value_frame(
    marker: char,
    value_type: AtomValueType,
    selector: ResponseValueSelector,
    format: ValueProjectionFormat,
    source_hash: &str,
    target_hash: &str,
    extractor_version: &str,
) -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: marker.to_string().repeat(64),
        event_id_sha256: marker.to_ascii_uppercase().to_string().repeat(64),
        client_intent_id_sha256: marker.to_string().repeat(64),
        session_id_sha256: marker.to_string().repeat(64),
        observed_at_unix_nanos: 100,
        estimated_input_tokens: 0,
        extractor_version: extractor_version.to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::ToolKind {
                value: "observed_tool".to_owned(),
            },
            RelationAtom::ObservationCallShape {
                value: "function_call".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "assistant_message".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 7,
                value_type,
                source: AtomSource::Observation,
                value_sha256: source_hash.to_owned(),
            },
            RelationAtom::ObservationSelector {
                slot_id: 7,
                selector,
            },
            RelationAtom::TypedSlot {
                slot_id: 11,
                value_type,
                source: AtomSource::Action,
                value_sha256: target_hash.to_owned(),
            },
            RelationAtom::SlotEquality {
                left_slot: 7,
                right_slot: 11,
            },
            RelationAtom::UniqueSlot { slot_id: 7 },
            RelationAtom::ActionValueProjection {
                format,
                renderer: CollectionOutputRenderer::Direct,
            },
        ],
        evidence_ref_sha256: "e".repeat(64),
    }
}

#[test]
fn project_selected_value_plain_text_is_a_verified_assistant_message() {
    let frame = selected_value_frame(
        'p',
        AtomValueType::String,
        ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::String,
        },
        ValueProjectionFormat::PlainText,
        &"1".repeat(64),
        &"1".repeat(64),
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    );
    let operator = synthesize_response_operator(&[frame]).expect("projection synthesis");
    assert!(matches!(
        operator.candidate.program.operation,
        ResponseOperation::ProjectSelectedValue { .. }
    ));
    let payload = json!({"input":[{
        "type":"function_call_output", "output":"source neutral value"
    }]});
    let execution = execute_response(&operator.candidate.program, "", &payload);
    assert_eq!(execution.status, ResponseExecutionStatus::Executed);
    assert_eq!(execution.response.as_deref(), Some("source neutral value"));
    assert!(
        verify_response_independently(
            &operator.verifier,
            &payload,
            execution.response.as_deref().expect("response")
        )
        .is_ok()
    );
}

#[test]
fn project_selected_value_template_is_learned_and_verified_independently() {
    let mut frame = selected_value_frame(
        't',
        AtomValueType::Identifier,
        ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Identifier,
        },
        ValueProjectionFormat::PlainText,
        &"7".repeat(64),
        &"7".repeat(64),
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    );
    let renderer = CollectionOutputRenderer::RenderTemplate {
        prefix: "Result: ".to_owned(),
        suffix: ".".to_owned(),
    };
    for atom in &mut frame.atoms {
        if let RelationAtom::ActionValueProjection {
            renderer: observed, ..
        } = atom
        {
            *observed = renderer.clone();
        }
    }
    let operator = synthesize_response_operator(&[frame]).expect("template synthesis");
    let payload = json!({"input":[{
        "type":"function_call_output", "output":"ready"
    }]});
    let execution = execute_response(&operator.candidate.program, "", &payload);
    assert_eq!(
        execution.status,
        ResponseExecutionStatus::Executed,
        "{}",
        execution.reason
    );
    assert_eq!(execution.response.as_deref(), Some("Result: ready."));
    assert!(
        verify_response_independently(
            &operator.verifier,
            &payload,
            execution.response.as_deref().expect("response")
        )
        .is_ok()
    );
    assert!(verify_response_independently(&operator.verifier, &payload, "ready").is_err());
}

#[test]
fn multi_claim_renderer_is_learned_and_verified_without_stored_values() {
    let mut frame = selected_value_frame(
        'm',
        AtomValueType::Integer,
        ResponseValueSelector::JsonField {
            field: "count".to_owned(),
            value_type: AtomValueType::Integer,
        },
        ValueProjectionFormat::PlainText,
        &"8".repeat(64),
        &"8".repeat(64),
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    );
    let renderer = CollectionOutputRenderer::RenderSequence {
        segments: vec![
            ResponseRenderSegment::Static {
                text: "Count: ".to_owned(),
            },
            ResponseRenderSegment::Primary,
            ResponseRenderSegment::Static {
                text: "; status: ".to_owned(),
            },
            ResponseRenderSegment::Selected {
                selector: ResponseValueSelector::JsonField {
                    field: "status".to_owned(),
                    value_type: AtomValueType::String,
                },
                format: ValueProjectionFormat::PlainText,
            },
            ResponseRenderSegment::Static {
                text: ".".to_owned(),
            },
        ],
    };
    for atom in &mut frame.atoms {
        if let RelationAtom::ActionValueProjection {
            renderer: observed, ..
        } = atom
        {
            *observed = renderer.clone();
        }
    }
    let operator = synthesize_response_operator(&[frame]).expect("sequence synthesis");
    let payload = json!({"input":[{
        "type":"function_call_output", "output":"{\"count\":3,\"status\":\"passed\"}"
    }]});
    let execution = execute_response(&operator.candidate.program, "", &payload);
    assert_eq!(
        execution.status,
        ResponseExecutionStatus::Executed,
        "{}",
        execution.reason
    );
    assert_eq!(
        execution.response.as_deref(),
        Some("Count: 3; status: passed.")
    );
    assert!(
        verify_response_independently(
            &operator.verifier,
            &payload,
            execution.response.as_deref().expect("response")
        )
        .is_ok()
    );
    assert!(
        verify_response_independently(&operator.verifier, &payload, "Count: 3; status: failed.")
            .is_err()
    );
}

#[test]
fn turn_output_line_sequence_replays_positionally_and_abstains_when_missing() {
    let program = ResponseProgram::project_selected_value(
        ResponseValueSelector::TurnOutputLine {
            output_ordinal: 1,
            line_index: 0,
            value_type: AtomValueType::String,
        },
        ValueProjectionFormat::PlainText,
        "completed",
    )
    .with_value_renderer(CollectionOutputRenderer::RenderSequence {
        segments: vec![
            ResponseRenderSegment::Static {
                text: "Result: ".to_owned(),
            },
            ResponseRenderSegment::Primary,
            ResponseRenderSegment::Static {
                text: "; status: ".to_owned(),
            },
            ResponseRenderSegment::Selected {
                selector: ResponseValueSelector::TurnOutputLine {
                    output_ordinal: 1,
                    line_index: 1,
                    value_type: AtomValueType::String,
                },
                format: ValueProjectionFormat::PlainText,
            },
            ResponseRenderSegment::Static {
                text: ".".to_owned(),
            },
        ],
    });
    let payload = json!({"input":[
        {"type":"message", "role":"user", "content":"check"},
        {"type":"function_call_output", "output":"apt is blocked\nchrome hold"}
    ]});
    let execution = execute_response(&program, "", &payload);
    assert_eq!(
        execution.status,
        ResponseExecutionStatus::Executed,
        "{}",
        execution.reason
    );
    assert_eq!(
        execution.response.as_deref(),
        Some("Result: apt is blocked; status: chrome hold.")
    );
    assert!(
        verify_response(
            &program,
            "",
            &payload,
            execution.response.as_deref().expect("response")
        )
        .is_ok()
    );
    let missing = json!({"input":[
        {"type":"message", "role":"user", "content":"check"},
        {"type":"function_call_output", "output":"apt is blocked"}
    ]});
    assert_eq!(
        execute_response(&program, "", &missing).status,
        ResponseExecutionStatus::Abstain
    );
}

#[test]
fn project_selected_value_canonical_json_scalar_is_exact() {
    let frame = selected_value_frame(
        'j',
        AtomValueType::Integer,
        ResponseValueSelector::JsonField {
            field: "selected".to_owned(),
            value_type: AtomValueType::Integer,
        },
        ValueProjectionFormat::CanonicalJson,
        &"2".repeat(64),
        &"2".repeat(64),
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    );
    let operator = synthesize_response_operator(&[frame]).expect("json projection synthesis");
    let payload = json!({"input":[{
        "type":"function_call_output", "output":"{\"selected\":42}"
    }]});
    let execution = execute_response(&operator.candidate.program, "", &payload);
    assert_eq!(execution.response.as_deref(), Some("42"));
}

#[test]
fn project_selected_value_ambiguity_type_hash_and_staleness_abstain() {
    let selector = ResponseValueSelector::ContentLinePrefix {
        prefix: "value=".to_owned(),
        value_type: AtomValueType::Integer,
    };
    let good = selected_value_frame(
        'a',
        AtomValueType::Integer,
        selector.clone(),
        ValueProjectionFormat::PlainText,
        &"3".repeat(64),
        &"3".repeat(64),
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    );
    let operator = synthesize_response_operator(&[good]).expect("projection synthesis");
    for payload in [
        json!({"input":[{"type":"function_call_output","output":"value=1\nvalue=2"}]}),
        json!({"input":[{"type":"function_call_output","output":"value=not-an-integer"}]}),
        json!({"input":[{"type":"function_call_output","output":"value=1"},{"type":"message","role":"user","content":"new turn"}]}),
    ] {
        assert_eq!(
            execute_response(&operator.candidate.program, "", &payload).status,
            ResponseExecutionStatus::Abstain
        );
    }
    let hash_mismatch = selected_value_frame(
        'h',
        AtomValueType::Integer,
        selector,
        ValueProjectionFormat::PlainText,
        &"4".repeat(64),
        &"5".repeat(64),
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    );
    assert_eq!(
        synthesize_response_operator(&[hash_mismatch]),
        Err(SynthesisError::NoConsistentProgram)
    );
}

#[test]
fn project_selected_value_multiline_and_non_scalar_abstain() {
    let multiline = ResponseProgram::project_selected_value(
        ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::String,
        },
        ValueProjectionFormat::PlainText,
        "completed",
    );
    assert_eq!(
        execute_response(
            &multiline,
            "",
            &json!({"input":[{"type":"function_call_output","output":"first\nsecond"}]})
        )
        .status,
        ResponseExecutionStatus::Abstain
    );
    let object = ResponseProgram::project_selected_value(
        ResponseValueSelector::JsonField {
            field: "selected".to_owned(),
            value_type: AtomValueType::String,
        },
        ValueProjectionFormat::CanonicalJson,
        "completed",
    );
    assert_eq!(
            execute_response(
                &object,
                "",
                &json!({"input":[{"type":"function_call_output","output":"{\"selected\":{\"nested\":true}}"}]})
            )
            .status,
            ResponseExecutionStatus::Abstain
        );
}

#[test]
fn project_selected_value_independent_verifier_rejects_mutation() {
    let frame = selected_value_frame(
        'm',
        AtomValueType::Boolean,
        ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Boolean,
        },
        ValueProjectionFormat::CanonicalJson,
        &"6".repeat(64),
        &"6".repeat(64),
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    );
    let operator = synthesize_response_operator(&[frame]).expect("projection synthesis");
    let payload = json!({"input":[{"type":"function_call_output","output":"true"}]});
    assert!(verify_response_independently(&operator.verifier, &payload, "false").is_err());
}

#[test]
fn project_selected_value_packages_and_diagnostics_do_not_persist_secret() {
    let secret = "PRIVATE-SCALAR-7b6d2";
    let frame = selected_value_frame(
        's',
        AtomValueType::String,
        ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::String,
        },
        ValueProjectionFormat::PlainText,
        &"7".repeat(64),
        &"7".repeat(64),
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    );
    let package = compile_source_neutral_quarantine_packages(&[frame], true).remove(0);
    let registry = ResponseRegistry {
        schema: "nando.response-registry.v5".to_owned(),
        revision: 1,
        packages: vec![package.clone()],
    };
    assert!(
        !String::from_utf8(serde_json::to_vec(&package).expect("package bytes"))
            .expect("utf8")
            .contains(secret)
    );
    assert!(
        !String::from_utf8(serde_json::to_vec(&registry).expect("registry bytes"))
            .expect("utf8")
            .contains(secret)
    );
    let rejected = execute_response(
        &package.program,
        "",
        &json!({"input":[{"type":"function_call_output","output":format!("{secret}\nsecond")}]}),
    );
    assert!(!rejected.reason.contains(secret));
    assert!(rejected.verification_receipt_id.is_none());
}

fn project_status_program(selector: ResponseValueSelector) -> ResponseProgram {
    ResponseProgram::project_status(selector, ProjectStatusMapping::ZeroIsSuccess, "completed")
}

fn project_status_verifier(selector: ResponseValueSelector) -> VerifierProgram {
    VerifierProgram::ProjectStatus {
        selector,
        mapping: ProjectStatusMapping::ZeroIsSuccess,
        renderer: CollectionOutputRenderer::Direct,
        completion_state: "completed".to_owned(),
        require_unique_value: true,
    }
}

#[test]
fn project_status_maps_zero_and_bounded_nonzero_to_exact_canonical_text() {
    let cases = [
        (
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            "0".to_owned(),
            "7".to_owned(),
        ),
        (
            ResponseValueSelector::ContentLinePrefix {
                prefix: "exit_code=".to_owned(),
                value_type: AtomValueType::Integer,
            },
            "exit_code=0".to_owned(),
            format!("exit_code={MAX_PROJECT_STATUS_CODE}"),
        ),
        (
            ResponseValueSelector::JsonField {
                field: "exit_code".to_owned(),
                value_type: AtomValueType::Integer,
            },
            "{\"exit_code\":0}".to_owned(),
            "{\"exit_code\":23}".to_owned(),
        ),
    ];
    for (selector, zero_output, nonzero_output) in cases {
        let program = project_status_program(selector.clone());
        let verifier = project_status_verifier(selector);
        for (tool_output, expected) in [
            (zero_output.as_str(), "success"),
            (nonzero_output.as_str(), "failure"),
        ] {
            let payload = json!({
                "input": [{"type":"function_call_output","output":tool_output}]
            });
            let execution = execute_response(&program, "", &payload);
            assert_eq!(execution.status, ResponseExecutionStatus::Executed);
            assert_eq!(execution.response.as_deref(), Some(expected));
            assert_eq!(
                execution.response.as_deref().map(str::len),
                Some(expected.len())
            );
            assert!(execution.verification_receipt_id.is_none());
            assert!(verify_response(&program, "", &payload, expected).is_ok());
            assert!(verify_response_independently(&verifier, &payload, expected).is_ok());
        }
    }
}

#[test]
fn project_status_content_parts_use_the_exact_actor_and_verifier_allowlist() {
    let selector = ResponseValueSelector::JsonField {
        field: "exit_code".to_owned(),
        value_type: AtomValueType::Integer,
    };
    let program = project_status_program(selector.clone());
    let verifier = project_status_verifier(selector);
    for part_type in ["text", "input_text", "output_text"] {
        let payload = json!({
            "input":[{
                "type":"function_call_output",
                "output":[{"type":part_type,"text":"{\"exit_code\":0}"}]
            }]
        });
        let execution = execute_response(&program, "", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("success"));
        assert!(verify_response_independently(&verifier, &payload, "success").is_ok());
    }

    for output in [
        json!([{"type":"unknown_text","text":"{\"exit_code\":0}"}]),
        json!([
            {"type":"output_text","text":"{\"exit_code\":0}"},
            {"type":"image_text","text":"ignored by permissive parsers"}
        ]),
        json!([{"type":"text","content":"{\"exit_code\":0}"}]),
    ] {
        let payload = json!({
            "input":[{"type":"function_call_output","output":output}]
        });
        assert_eq!(
            execute_response(&program, "", &payload).status,
            ResponseExecutionStatus::Abstain
        );
        assert!(verify_response_independently(&verifier, &payload, "success").is_err());
    }
}

#[test]
fn project_status_abstains_on_missing_multiple_non_integer_unbounded_and_stale_evidence() {
    let program = project_status_program(ResponseValueSelector::ContentLinePrefix {
        prefix: "exit_code=".to_owned(),
        value_type: AtomValueType::Integer,
    });
    for payload in [
        json!({"input":[{"type":"function_call_output","output":"no selected value"}]}),
        json!({"input":[{"type":"function_call_output","output":"exit_code=0\nexit_code=1"}]}),
        json!({"input":[{"type":"function_call_output","output":"exit_code=-1"}]}),
        json!({"input":[{"type":"function_call_output","output":format!("exit_code={}", MAX_PROJECT_STATUS_CODE + 1)}]}),
        json!({"input":[{"type":"function_call_output","output":"exit_code=true"}]}),
        json!({"input":[{"type":"function_call_output","output":"exit_code=success"}]}),
        json!({"input":[{"type":"function_call_output","output":"exit_code=completed successfully"}]}),
        json!({"input":[{"type":"function_call_output","output":"exit_code=1.0"}]}),
        json!({"input":[{"type":"function_call_output","output":"exit_code=0"},{"type":"message","role":"user","content":"new turn"}]}),
        json!({"input":[]}),
    ] {
        let execution = execute_response(&program, "", &payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
        assert!(execution.response.is_none());
        assert!(execution.verification_receipt_id.is_none());
    }
}

#[test]
fn project_status_unique_scalar_rejects_enum_bool_prose_and_ambiguous_structures() {
    let program = project_status_program(ResponseValueSelector::UniqueScalar {
        value_type: AtomValueType::Integer,
    });
    for output in [
        "\"success\"",
        "true",
        "process exited successfully",
        "{\"left\":0,\"right\":1}",
        "[]",
        "null",
    ] {
        let execution = execute_response(
            &program,
            "",
            &json!({"input":[{"type":"custom_tool_call_output","output":output}]}),
        );
        assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
    }
}

#[test]
fn project_status_contract_and_relation_atom_roundtrip_canonically() {
    let selector = ResponseValueSelector::JsonField {
        field: "exit_code".to_owned(),
        value_type: AtomValueType::Integer,
    };
    let program = project_status_program(selector.clone());
    let verifier = project_status_verifier(selector);
    let atom = RelationAtom::ActionStatusProjection {
        mapping: ProjectStatusMapping::ZeroIsSuccess,
    };

    let program_json = serde_json::to_value(&program).expect("program json");
    assert_eq!(
        program_json.pointer("/operation/op"),
        Some(&json!("project_status"))
    );
    assert_eq!(
        program_json.pointer("/operation/mapping"),
        Some(&json!("zero_is_success"))
    );
    assert_eq!(
        serde_json::from_value::<ResponseProgram>(program_json).expect("program roundtrip"),
        program
    );

    let verifier_json = serde_json::to_value(&verifier).expect("verifier json");
    assert_eq!(verifier_json.get("kind"), Some(&json!("project_status")));
    assert_eq!(
        serde_json::from_value::<VerifierProgram>(verifier_json).expect("verifier roundtrip"),
        verifier
    );

    let atom_json = serde_json::to_value(&atom).expect("atom json");
    assert_eq!(
        atom_json.get("kind"),
        Some(&json!("action_status_projection"))
    );
    assert_eq!(
        serde_json::from_value::<RelationAtom>(atom_json).expect("atom roundtrip"),
        atom
    );
    assert_eq!(
        serde_json::to_string(&ProjectStatusValue::Success).expect("status json"),
        "\"success\""
    );
    assert_eq!(
        serde_json::to_string(&ProjectStatusValue::Failure).expect("status json"),
        "\"failure\""
    );
}

#[test]
fn project_status_verifier_rejects_wrong_mapping_output_and_stale_evidence() {
    let selector = ResponseValueSelector::JsonField {
        field: "exit_code".to_owned(),
        value_type: AtomValueType::Integer,
    };
    let verifier = project_status_verifier(selector);
    let zero = json!({
        "input":[{"type":"function_call_output","output":"{\"exit_code\":0}"}]
    });
    let nonzero = json!({
        "input":[{"type":"function_call_output","output":"{\"exit_code\":9}"}]
    });
    assert!(verify_response_independently(&verifier, &zero, "failure").is_err());
    assert!(verify_response_independently(&verifier, &nonzero, "success").is_err());

    let stale = json!({
        "input":[
            {"type":"function_call_output","output":"{\"exit_code\":0}"},
            {"type":"message","role":"assistant","content":"already consumed"}
        ]
    });
    assert!(verify_response_independently(&verifier, &stale, "success").is_err());
}

#[test]
fn v5_frames_and_existing_function_custom_tool_families_remain_compatible() {
    let mut function_frame = extract_relation_frame(&scalar_transfer_trace(3, 8, false, "v5"));
    function_frame.extractor_version = "response-relation-extractor.v5".to_owned();
    assert!(synthesize_response_operator(&[function_frame]).is_ok());

    let function = extract_relation_frame(&continuation_trace(2, 9, false));
    assert!(matches!(
        synthesize_response_operator(&[function])
            .expect("function synthesis")
            .candidate
            .program
            .operation,
        ResponseOperation::FunctionCallFromRoles { .. }
    ));
    let custom = custom_continuation_frame('z', 'y', 4, 12, "custom");
    assert!(matches!(
        synthesize_response_operator(&[custom])
            .expect("custom synthesis")
            .candidate
            .program
            .operation,
        ResponseOperation::CustomToolCallFromRoles { .. }
    ));
}

#[test]
fn generic_collection_program_projects_filters_counts_and_composes_in_order() {
    let payload = json!({
        "input":[{"type":"function_call_output","output":
            "{\"rows\":[{\"kind\":\"keep\",\"value\":3},{\"kind\":\"drop\",\"value\":4},{\"kind\":\"keep\",\"value\":5}]}"
        }]
    });
    let steps = vec![
        CollectionProgramStep::SelectField {
            field: "rows".to_owned(),
        },
        CollectionProgramStep::FilterFieldEquals {
            field: "kind".to_owned(),
            value: ResponseScalarLiteral::String("keep".to_owned()),
        },
        CollectionProgramStep::ProjectField {
            field: "value".to_owned(),
        },
    ];
    let projection = ResponseProgram::compose_collection(
        steps.clone(),
        ValueProjectionFormat::CanonicalJson,
        "completed",
    );
    let executed = execute_response(&projection, "", &payload);
    assert_eq!(executed.status, ResponseExecutionStatus::Executed);
    assert_eq!(executed.response.as_deref(), Some("[3,5]"));

    let count_steps = vec![
        steps[0].clone(),
        steps[1].clone(),
        CollectionProgramStep::Count,
    ];
    let count = ResponseProgram::compose_collection(
        count_steps.clone(),
        ValueProjectionFormat::PlainText,
        "completed",
    );
    let executed = execute_response(&count, "", &payload);
    assert_eq!(executed.response.as_deref(), Some("2"));
    let verifier = VerifierProgram::ComposeCollection {
        steps: count_steps,
        format: ValueProjectionFormat::PlainText,
        renderer: CollectionOutputRenderer::Direct,
        completion_state: "completed".to_owned(),
        max_items: 1_024,
    };
    assert!(verify_response_independently(&verifier, &payload, "2").is_ok());
    assert!(verify_response_independently(&verifier, &payload, "3").is_err());
}

#[test]
fn generic_collection_program_fails_closed_on_shape_order_and_budget() {
    let payload = json!({
        "input":[{"type":"function_call_output","output":"{\"rows\":[{\"kind\":\"keep\"}]}"}]
    });
    let wrong_order = ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::Count,
            CollectionProgramStep::SelectField {
                field: "rows".to_owned(),
            },
        ],
        ValueProjectionFormat::CanonicalJson,
        "completed",
    );
    assert_eq!(
        execute_response(&wrong_order, "", &payload).status,
        ResponseExecutionStatus::Abstain
    );
    let missing = ResponseProgram::compose_collection(
        vec![CollectionProgramStep::SelectField {
            field: "missing".to_owned(),
        }],
        ValueProjectionFormat::CanonicalJson,
        "completed",
    );
    assert_eq!(
        execute_response(&missing, "", &payload).status,
        ResponseExecutionStatus::Abstain
    );
    let mut over_budget = ResponseProgram::compose_collection(
        vec![CollectionProgramStep::SelectField {
            field: "rows".to_owned(),
        }],
        ValueProjectionFormat::CanonicalJson,
        "completed",
    );
    let ResponseOperation::ComposeCollection { max_items, .. } = &mut over_budget.operation else {
        unreachable!();
    };
    *max_items = 0;
    assert_eq!(
        execute_response(&over_budget, "", &payload).status,
        ResponseExecutionStatus::Abstain
    );
}
