use serde_json::json;

use super::*;
use crate::{ResponseOperation, verify_response_independently};

fn example(collection: &str, predicate: &str, value: &str) -> CollectionSynthesisExample {
    let output = json!({
        (collection): [
            {(predicate):"keep", (value):3},
            {(predicate):"drop", (value):4},
            {(predicate):"keep", (value):5}
        ]
    });
    CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":output.to_string()}]
        }),
        expected_response: "[3,5]".to_owned(),
    }
}

#[test]
fn count_is_synthesized_from_an_earlier_turn_output_with_runtime_parity() {
    let support = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":"How many rows?"},
                {"type":"function_call_output","output":"{\"rows\":[{\"id\":1},{\"id\":2},{\"id\":3}]}"},
                {"type":"function_call_output","output":"command completed"}
            ]
        }),
        expected_response: "3".to_owned(),
    };
    let synthesized =
        synthesize_collection_program(std::slice::from_ref(&support)).expect("synthesis");
    let ResponseOperation::ComposeCollection { steps, .. } = &synthesized.program.operation else {
        panic!("expected collection program");
    };
    assert!(matches!(
        steps.first(),
        Some(CollectionProgramStep::SelectTurnOutput { output_ordinal: 1 })
    ));
    assert!(matches!(steps.last(), Some(CollectionProgramStep::Count)));

    let execution = execute_response(&synthesized.program, "", &support.provider_payload);
    assert_eq!(execution.response.as_deref(), Some("3"));
    assert!(
        verify_response_independently(&synthesized.verifier, &support.provider_payload, "3")
            .is_ok()
    );

    let swapped = json!({
        "input":[
            {"type":"message","role":"user","content":"How many rows?"},
            {"type":"function_call_output","output":"command completed"},
            {"type":"function_call_output","output":"{\"rows\":[{\"id\":1},{\"id\":2},{\"id\":3}]}"}
        ]
    });
    assert!(
        execute_response(&synthesized.program, "", &swapped)
            .response
            .is_none()
    );
    assert!(verify_response_independently(&synthesized.verifier, &swapped, "3").is_err());
}

#[test]
fn embedded_json_collection_has_actor_verifier_runtime_parity() {
    let mut support = [example("rows", "kind", "value")];
    support[0].provider_payload = json!({
        "input":[{
            "type":"function_call_output",
            "output":"command output\n```json\n{\"rows\":[{\"kind\":\"keep\",\"value\":3},{\"kind\":\"drop\",\"value\":4},{\"kind\":\"keep\",\"value\":5}]}\n```\ncompleted"
        }]
    });
    let synthesized =
        synthesize_collection_program(&[example("rows", "kind", "value")]).expect("synthesis");
    let execution = execute_response(&synthesized.program, "", &support[0].provider_payload);
    assert_eq!(execution.response.as_deref(), Some("[3,5]"));
    assert!(
        verify_response_independently(&synthesized.verifier, &support[0].provider_payload, "[3,5]")
            .is_ok()
    );
}

#[test]
fn line_embedded_json_collection_is_synthesized_with_runtime_parity() {
    let mut support = [example("rows", "kind", "value")];
    support[0].provider_payload = json!({
        "input":[{
            "type":"function_call_output",
            "output":"progress\n{\"rows\":[{\"kind\":\"keep\",\"value\":3},{\"kind\":\"drop\",\"value\":4},{\"kind\":\"keep\",\"value\":5}]}\ncompleted"
        }]
    });
    let synthesized = synthesize_collection_program(&support).expect("synthesis");
    let execution = execute_response(&synthesized.program, "", &support[0].provider_payload);
    assert_eq!(execution.response.as_deref(), Some("[3,5]"));
    assert!(
        verify_response_independently(&synthesized.verifier, &support[0].provider_payload, "[3,5]")
            .is_ok()
    );
}

#[test]
fn latest_turn_output_scalar_ignores_earlier_outputs_with_runtime_parity() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"function_call_output","output":"{\"old\":99}"},
                {"type":"message","role":"user","content":"Return the latest total"},
                {"type":"function_call_output","output":"{\"noise\":11}"},
                {"type":"function_call_output","output":"{\"total\":7}"}
            ]
        }),
        expected_response: "7".to_owned(),
    };
    let version_space =
        enumerate_source_neutral_response_programs(&example).expect("version space");
    let program = version_space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    selector: ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                        scalar_ordinal: 0,
                        value_type: AtomValueType::Integer,
                    },
                    ..
                }
            )
        })
        .expect("latest output program");
    assert_eq!(
        execute_response(program, "", &example.provider_payload)
            .response
            .as_deref(),
        Some("7")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &example.provider_payload, "7").is_ok());
}

#[test]
fn generic_version_space_discovers_rendered_scalar_projection() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"total\":7}"}]
        }),
        expected_response: "Total: 7.".to_owned(),
    };
    let version_space =
        enumerate_source_neutral_response_programs(&example).expect("version space");
    assert!(version_space.exact_checks > 0);
    let program = version_space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue { .. }
            )
        })
        .expect("scalar projection");
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert_eq!(
        execute_response(program, "", &example.provider_payload)
            .response
            .as_deref(),
        Some("Total: 7.")
    );
    assert!(
        verify_response_independently(
            &verifier,
            &example.provider_payload,
            &example.expected_response,
        )
        .is_ok()
    );
}

#[test]
fn evidence_only_projection_discards_unverified_prose_but_keeps_grounded_scalar() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"count\":7}"}]
        }),
        expected_response: "There are 7 matching rows.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("version space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    format: ValueProjectionFormat::PlainText,
                    renderer: CollectionOutputRenderer::Direct,
                    ..
                }
            ) && execute_response(program, "", &example.provider_payload)
                .response
                .as_deref()
                == Some("7")
        })
        .expect("evidence-only projection");
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &example.provider_payload, "7").is_ok());
}

#[test]
fn evidence_only_projection_retains_ambiguity_for_cegis() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"count\":7,\"failed\":2}"}]
        }),
        expected_response: "There are 7 rows and 2 failures.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("version space");
    let projected = space
        .programs
        .iter()
        .filter(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    renderer: CollectionOutputRenderer::Direct,
                    ..
                }
            )
        })
        .filter_map(|program| execute_response(program, "", &example.provider_payload).response)
        .collect::<BTreeSet<_>>();
    assert_eq!(projected, BTreeSet::from(["2".to_owned(), "7".to_owned()]));
}

#[test]
fn recursive_json_field_projection_keeps_actor_verifier_parity() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"outer\":{\"result\":7}}"}]
        }),
        expected_response: "The verified result is 7.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("version space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    selector: ResponseValueSelector::JsonField { field, .. },
                    renderer: CollectionOutputRenderer::Direct,
                    ..
                } if field == "result"
            )
        })
        .expect("recursive field program");
    assert_eq!(
        execute_response(program, "", &example.provider_payload)
            .response
            .as_deref(),
        Some("7")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &example.provider_payload, "7").is_ok());
    let ambiguous = json!({
        "input":[{"type":"function_call_output","output":"{\"left\":{\"result\":7},\"right\":{\"result\":8}}"}]
    });
    assert_eq!(
        execute_response(program, "", &ambiguous).status,
        ResponseExecutionStatus::Abstain
    );
    assert!(verify_response_independently(&verifier, &ambiguous, "7").is_err());
}

#[test]
fn json_scalar_ordinal_transfers_across_field_names() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"a\":2,\"z\":7}"}]
        }),
        expected_response: "7".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("version space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    selector: ResponseValueSelector::JsonScalarOrdinal {
                        ordinal: 1,
                        value_type: AtomValueType::Integer,
                    },
                    renderer: CollectionOutputRenderer::Direct,
                    ..
                }
            )
        })
        .expect("ordinal program");
    let future = json!({
        "input":[{"type":"function_call_output","output":"{\"alpha\":3,\"omega\":7}"}]
    });
    assert_eq!(
        execute_response(program, "", &future).response.as_deref(),
        Some("7")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &future, "7").is_ok());
}

#[test]
fn generic_version_space_discovers_status_projection() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"0"}]
        }),
        expected_response: "success".to_owned(),
    };
    let version_space =
        enumerate_source_neutral_response_programs(&example).expect("version space");
    assert!(
        version_space
            .programs
            .iter()
            .any(|program| matches!(&program.operation, ResponseOperation::ProjectStatus { .. }))
    );
}

#[test]
fn learned_adapter_version_space_discovers_json_field_and_line_prefix() {
    let json_example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"Wall time: 0.1 seconds\nOutput:\n[{\"type\":\"text\",\"text\":\"{\\\"noise\\\":1,\\\"result\\\":42}\"}]"}]
        }),
        expected_response: "42".to_owned(),
    };
    let json_space = enumerate_source_neutral_response_programs(&json_example).expect("json space");
    assert!(json_space.programs.iter().any(|program| matches!(
        &program.operation,
        ResponseOperation::ProjectSelectedValue {
            selector: ResponseValueSelector::JsonField { field, .. },
            ..
        } if field == "result"
    )));

    let line_example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"noise: 1\ncount: 42"}]
        }),
        expected_response: "42".to_owned(),
    };
    let line_space = enumerate_source_neutral_response_programs(&line_example).expect("line space");
    assert!(line_space.programs.iter().any(|program| matches!(
        &program.operation,
        ResponseOperation::ProjectSelectedValue {
            selector: ResponseValueSelector::ContentLinePrefix { prefix, .. },
            ..
        } if prefix == "count: "
    )));
}

#[test]
fn learned_adapter_composes_multiple_grounded_scalars() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"passed\":12,\"failed\":0}"}]
        }),
        expected_response: "Passed: 12; failed: 0.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("composed space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    renderer: CollectionOutputRenderer::RenderSequence { .. },
                    ..
                }
            )
        })
        .expect("render sequence");
    assert!(is_privacy_safe_online_response_program(program));
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert_eq!(
        execute_response(program, "", &example.provider_payload)
            .response
            .as_deref(),
        Some(example.expected_response.as_str())
    );
    assert!(
        verify_response_independently(
            &verifier,
            &example.provider_payload,
            &example.expected_response,
        )
        .is_ok()
    );
}

#[test]
fn canonical_renderer_handles_more_than_eight_dynamic_values() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":
                "{\"a\":11,\"b\":12,\"c\":13,\"d\":14,\"e\":15,\"f\":16,\"g\":17,\"h\":18,\"i\":19}"}]
        }),
        expected_response: "Observed values: 11, 12, 13, 14, 15, 16, 17, 18, 19.".to_owned(),
    };
    let expected = "11\n12\n13\n14\n15\n16\n17\n18\n19";
    let space = enumerate_source_neutral_response_programs(&example).expect("bounded space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            execute_response(program, "", &example.provider_payload)
                .response
                .as_deref()
                == Some(expected)
        })
        .expect("canonical nine-value renderer");
    assert!(response_program_authority_matches_example(
        program, &example
    ));
    assert!(is_privacy_safe_online_response_program(program));
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &example.provider_payload, expected).is_ok());
}

#[test]
fn teacher_prose_trains_canonical_multi_value_response() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"passed\":12,\"failed\":0}"}]
        }),
        expected_response: "The run completed with 12 passing checks and 0 failures.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("composed space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            execute_response(program, "", &example.provider_payload)
                .response
                .as_deref()
                == Some("12\n0")
        })
        .expect("canonical multi-value response");
    assert!(is_privacy_safe_online_response_program(program));
    assert!(!response_program_exactly_matches_example(program, &example));
    assert!(response_program_authority_matches_example(
        program, &example
    ));
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &example.provider_payload, "12\n0").is_ok());
}

#[test]
fn learned_adapter_composes_plain_text_scalar_ordinals() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Report the verified counts"}]},
                {"type":"function_call_output","output":"ok=3 failed=2"}
            ]
        }),
        expected_response: "success: 3; failed: 2.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("ordinal space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    selector: ResponseValueSelector::TurnOutputScalarOrdinal { .. },
                    renderer: CollectionOutputRenderer::RenderSequence { .. },
                    ..
                }
            ) && execute_response(program, "", &example.provider_payload)
                .response
                .as_deref()
                == Some(example.expected_response.as_str())
        })
        .expect("plain text ordinal render sequence");
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(
        verify_response_independently(
            &verifier,
            &example.provider_payload,
            &example.expected_response,
        )
        .is_ok()
    );
}

#[test]
fn all_observation_inverse_synthesis_transfers_across_renamed_fields() {
    let support = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Summarize the verified result"}]},
                {"type":"function_call_output","call_id":"a","output":"{\"total\":7}"},
                {"type":"function_call_output","call_id":"b","output":"{\"status\":\"ok\"}"}
            ]
        }),
        expected_response: "Total: 7; status: ok.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&support).expect("version space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    selector: ResponseValueSelector::UniqueTurnScalar { .. },
                    renderer: CollectionOutputRenderer::RenderSequence { segments },
                    ..
                } if segments.iter().any(|segment| matches!(
                    segment,
                    ResponseRenderSegment::Selected {
                        selector: ResponseValueSelector::UniqueTurnScalar { .. },
                        ..
                    }
                ))
            )
        })
        .expect("field-neutral multi-output program");
    let future = json!({
        "input":[
            {"type":"message","role":"user","content":[{"type":"input_text","text":"Summarize the verified result"}]},
            {"type":"function_call_output","call_id":"x","output":"{\"items\":9}"},
            {"type":"function_call_output","call_id":"y","output":"{\"state\":\"ready\"}"}
        ]
    });
    assert_eq!(
        execute_response(program, "", &future).response.as_deref(),
        Some("Total: 9; status: ready.")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &future, "Total: 9; status: ready.").is_ok());
}

#[test]
fn unique_turn_scalar_abstains_on_same_type_collision() {
    let program = ResponseProgram::project_selected_value(
        ResponseValueSelector::UniqueTurnScalar {
            value_type: AtomValueType::Integer,
        },
        ValueProjectionFormat::PlainText,
        "completed",
    );
    let ambiguous = json!({
        "input":[
            {"type":"function_call_output","output":"{\"left\":7}"},
            {"type":"function_call_output","output":"{\"right\":8}"}
        ]
    });
    assert_eq!(
        execute_response(&program, "", &ambiguous).status,
        ResponseExecutionStatus::Abstain
    );
    let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
    assert!(verify_response_independently(&verifier, &ambiguous, "7").is_err());
}

#[test]
fn request_grounded_filter_transfers_without_literal_or_field_names() {
    let support = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Select keep values"}]},
                {"type":"function_call_output","output":"{\"rows\":[{\"kind\":\"keep\",\"value\":3},{\"kind\":\"drop\",\"value\":4},{\"kind\":\"keep\",\"value\":5}]}"}
            ]
        }),
        expected_response: "[3,5]".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&support).expect("space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ComposeCollection { steps, .. }
                    if steps.iter().any(|step| matches!(
                        step,
                        CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                    ))
            )
        })
        .expect("request-grounded program");
    let future = json!({
        "input":[
            {"type":"message","role":"user","content":[{"type":"input_text","text":"Return yes measurements"}]},
            {"type":"function_call_output","output":"{\"items\":[{\"class\":\"yes\",\"measurement\":11},{\"class\":\"no\",\"measurement\":12},{\"class\":\"yes\",\"measurement\":13}]}"}
        ]
    });
    assert_eq!(
        execute_response(program, "", &future).response.as_deref(),
        Some("[11,13]")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &future, "[11,13]").is_ok());
    let ambiguous = json!({
        "input":[
            {"type":"message","role":"user","content":[{"type":"input_text","text":"Return yes and no measurements"}]},
            {"type":"function_call_output","output":"{\"items\":[{\"class\":\"yes\",\"measurement\":11},{\"class\":\"no\",\"measurement\":12}]}"}
        ]
    });
    assert_eq!(
        execute_response(program, "", &ambiguous).status,
        ResponseExecutionStatus::Abstain
    );
    assert!(verify_response_independently(&verifier, &ambiguous, "[11]").is_err());
}

#[test]
fn selector_beam_bounds_broad_turn_before_program_expansion() {
    let outputs = (0..16)
        .map(|output_ordinal| {
            let output = (0..256)
                .map(|line| format!("metric_{output_ordinal}_{line}: {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            json!({"type":"function_call_output", "output":output})
        })
        .collect::<Vec<_>>();
    let payload = json!({"input": outputs});
    let selectors = learned_selector_candidates(&payload);

    assert!(selectors.len() <= nando_operator_runtime::MAX_SELECTOR_CANDIDATES);
    assert!(selectors.iter().any(|selector| matches!(
        selector,
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal { ordinal: 0, .. }
    )));
    assert!(selectors.iter().any(|selector| matches!(
        selector,
        ResponseValueSelector::LatestTurnOutputLine { .. }
            | ResponseValueSelector::ContentLinePrefix { .. }
    )));
}

#[test]
fn bounded_sequence_allows_two_values_and_repeated_primary() {
    for expected in ["78", "77"] {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"left\":7,\"right\":8}"}]
            }),
            expected_response: expected.to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("space");
        let selectors = learned_selector_candidates(&example.provider_payload);
        let resolved = selectors
            .iter()
            .map(|selector| {
                (
                    selector.clone(),
                    execute_response(
                        &ResponseProgram::project_selected_value(
                            selector.clone(),
                            ValueProjectionFormat::PlainText,
                            "completed",
                        ),
                        "",
                        &example.provider_payload,
                    )
                    .response,
                )
            })
            .collect::<Vec<_>>();
        let direct_candidate = compose_render_sequence_candidates(&example, &selectors, true)
            .into_iter()
            .find(|program| {
                execute_response(program, "", &example.provider_payload)
                    .response
                    .as_deref()
                    == Some(expected)
            })
            .unwrap_or_else(|| panic!("direct sequence candidate: {resolved:?}"));
        assert_eq!(direct_candidate.validate(), Ok(()));
        assert_eq!(
            execute_response(&direct_candidate, "", &example.provider_payload)
                .response
                .as_deref(),
            Some(expected)
        );
        let rejection_reasons = space.policy_rejection_reasons.clone();
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        renderer: CollectionOutputRenderer::RenderSequence { .. },
                        ..
                    }
                )
            })
            .unwrap_or_else(|| panic!("sequence: {rejection_reasons:?}"));
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert_eq!(
            execute_response(program, "", &example.provider_payload)
                .response
                .as_deref(),
            Some(expected)
        );
        assert!(
            verify_response_independently(
                &verifier,
                &example.provider_payload,
                &example.expected_response,
            )
            .is_ok()
        );
    }
}

#[test]
fn policy_rejection_counts_only_exact_candidates() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"left\":7,\"right\":8}"}]
        }),
        expected_response: "This answer is not derivable from either scalar.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("space");
    assert!(space.programs.is_empty());
    assert_eq!(space.policy_rejected_exact_matches, 0);
    assert!(space.policy_rejection_reasons.is_empty());
}

#[test]
fn single_dynamic_value_renderer_is_exact_but_surface_bound() {
    let support = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"ok\":3}"}]
        }),
        expected_response: "Успешных записей: 3".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&support).expect("space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            response_program_exactly_matches_example(program, &support)
                && matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::UniqueScalar { .. }
                            | ResponseValueSelector::UniqueTurnScalar { .. },
                        renderer: CollectionOutputRenderer::RenderSequence { segments },
                        ..
                    } if segments.iter().filter(|segment| matches!(
                        segment,
                        ResponseRenderSegment::Primary
                            | ResponseRenderSegment::Selected { .. }
                    )).count() == 1
                )
        })
        .expect("single dynamic renderer");
    assert!(is_privacy_safe_online_response_program(program));
    assert!(!is_source_neutral_response_program(program));

    let future = json!({
        "input":[{"type":"function_call_output","output":"{\"done\":4}"}]
    });
    assert_eq!(
        execute_response(program, "", &future).response.as_deref(),
        Some("Успешных записей: 4")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &future, "Успешных записей: 4").is_ok());
}

#[test]
fn policy_rejection_retains_exact_unsafe_sequence() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"left\":7,\"right\":8}"}]
        }),
        expected_response: "Customer Alice has 7 of 8.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("space");
    assert!(
        space
            .programs
            .iter()
            .all(|program| !response_program_exactly_matches_example(program, &example))
    );
    assert!(space.policy_rejected_exact_matches > 0);
    assert!(
        space
            .policy_rejection_reasons
            .contains_key("unsafe_render_sequence_static_text")
    );
}

#[test]
fn benign_multiline_static_frame_enters_transfer_bound_version_space() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"value\":7}"}]
        }),
        expected_response: "Result:\n7".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            response_program_exactly_matches_example(program, &example)
                && response_program_requires_static_frame_transfer(program)
        })
        .expect("transfer-bound static frame");
    assert!(is_transfer_bound_response_program(program));
    assert!(
        response_program_dynamic_value_root_sha256(program, &example)
            .expect("dynamic root")
            .is_some()
    );
    assert!(!is_source_neutral_response_program(program));
}

#[test]
fn unsafe_mixed_request_tool_surface_retains_canonical_dynamic_law() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Alice"}]},
                {"type":"function_call_output","output":"{\"count\":7}"}
            ]
        }),
        expected_response: "Customer Alice has 7.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("space");
    let canonical = space
        .programs
        .iter()
        .find(|program| {
            execute_response(program, "", &example.provider_payload)
                .response
                .as_deref()
                == Some("Alice\n7")
        })
        .expect("canonical request and tool law");
    assert!(is_privacy_safe_online_response_program(canonical));
    assert!(!is_source_neutral_response_program(canonical));
    let verifier = source_neutral_verifier_for_program(canonical).expect("verifier");
    assert!(
        verify_response_independently(&verifier, &example.provider_payload, "Alice\n7",).is_ok()
    );
}

#[test]
fn command_output_body_selector_ignores_envelope_metadata() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"[{\"type\":\"input_text\",\"text\":\"Script completed\\nWall time 0.4 seconds\\nOutput:\\n\"},{\"type\":\"input_text\",\"text\":\"alpha\\n\"}]"}]
        }),
        expected_response: "Result: alpha.".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    selector: ResponseValueSelector::CommandOutputBody,
                    ..
                }
            )
        })
        .expect("command output projection");
    let future = json!({
        "input":[{"type":"function_call_output","output":"[{\"type\":\"input_text\",\"text\":\"Script completed\\nWall time 8.1 seconds\\nOutput:\\n\"},{\"type\":\"input_text\",\"text\":\"beta\\n\"}]"}]
    });
    assert_eq!(
        execute_response(program, "", &future).response.as_deref(),
        Some("Result: beta.")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &future, "Result: beta.").is_ok());
    let missing_marker = json!({
        "input":[{"type":"function_call_output","output":"beta"}]
    });
    assert_eq!(
        execute_response(program, "", &missing_marker).status,
        ResponseExecutionStatus::Abstain
    );
}

#[test]
fn request_last_token_projects_literal_constraint_without_stored_literal() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"Do the work. End only CAPTURE_COMPLETE."}]}]
        }),
        expected_response: "CAPTURE_COMPLETE".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    selector: ResponseValueSelector::RequestLastToken,
                    ..
                }
            )
        })
        .expect("request literal projection");
    let future = json!({
        "input":[{"type":"message","role":"user","content":"Run another task. Reply only FUTURE_OK!"}]
    });
    assert_eq!(
        execute_response(program, "", &future).response.as_deref(),
        Some("FUTURE_OK")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &future, "FUTURE_OK").is_ok());
    assert!(verify_response_independently(&verifier, &future, "WRONG").is_err());
}

#[test]
fn request_unique_literal_projects_quoted_constraint() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"message","role":"user","content":"Reply with `ALPHA VALUE` and then stop"}]
        }),
        expected_response: "ALPHA VALUE".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&example).expect("space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectSelectedValue {
                    selector: ResponseValueSelector::RequestUniqueLiteral,
                    ..
                }
            )
        })
        .expect("quoted literal projection");
    let future = json!({
        "input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"Return 'BETA VALUE' without commentary"}]}]
    });
    assert_eq!(
        execute_response(program, "", &future).response.as_deref(),
        Some("BETA VALUE")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &future, "BETA VALUE").is_ok());
    let ambiguous = json!({
        "input":[{"type":"message","role":"user","content":"Choose `ONE` or `TWO`"}]
    });
    assert_eq!(
        execute_response(program, "", &ambiguous).status,
        ResponseExecutionStatus::Abstain
    );
}

#[test]
fn aggregate_and_filtered_aggregate_transfer_across_surfaces() {
    let support = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":"Sum keep rows"},
                {"type":"function_call_output","output":"{\"rows\":[{\"kind\":\"keep\",\"amount\":3},{\"kind\":\"drop\",\"amount\":4},{\"kind\":\"keep\",\"amount\":5}]}"}
            ]
        }),
        expected_response: "8".to_owned(),
    };
    let space = enumerate_source_neutral_response_programs(&support).expect("space");
    let program = space
        .programs
        .iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ComposeCollection { steps, .. }
                    if steps.iter().any(|step| matches!(
                        step,
                        CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                    )) && steps.iter().any(|step| matches!(
                        step,
                        CollectionProgramStep::AggregateUniqueIntegerField {
                            operation: CollectionAggregateOperation::Sum
                        }
                    ))
            )
        })
        .expect("filtered aggregate");
    let future = json!({
        "input":[
            {"type":"message","role":"user","content":"Total yes entries"},
            {"type":"function_call_output","output":"{\"items\":[{\"class\":\"yes\",\"metric\":11},{\"class\":\"no\",\"metric\":12},{\"class\":\"yes\",\"metric\":13}]}"}
        ]
    });
    assert_eq!(
        execute_response(program, "", &future).response.as_deref(),
        Some("24")
    );
    let verifier = source_neutral_verifier_for_program(program).expect("verifier");
    assert!(verify_response_independently(&verifier, &future, "24").is_ok());
}

#[test]
fn embedded_json_collection_abstains_when_two_candidates_differ() {
    let synthesized =
        synthesize_collection_program(&[example("rows", "kind", "value")]).expect("synthesis");
    let payload = json!({
        "input":[{
            "type":"function_call_output",
            "output":"{\"rows\":[{\"kind\":\"keep\",\"value\":3}]}\n{\"rows\":[{\"kind\":\"keep\",\"value\":9}]}"
        }]
    });
    assert_ne!(
        execute_response(&synthesized.program, "", &payload).status,
        ResponseExecutionStatus::Executed
    );
    assert!(verify_response_independently(&synthesized.verifier, &payload, "[3]").is_err());
}

#[test]
fn synthesis_discovers_role_based_filter_projection_across_renamed_layouts() {
    let support = vec![
        example("rows", "kind", "value"),
        example("entries", "tag", "amount"),
    ];
    let synthesized = synthesize_collection_program(&support).expect("synthesis");
    assert!(synthesized.exact_checks > 0);
    assert!(matches!(
        synthesized.program.operation,
        ResponseOperation::ComposeCollection { .. }
    ));
    let heldout = example("records", "marker", "score");
    let execution = execute_response(&synthesized.program, "", &heldout.provider_payload);
    assert_eq!(execution.status, ResponseExecutionStatus::Executed);
    assert_eq!(execution.response.as_deref(), Some("[3,5]"));
    assert!(
        verify_response_independently(&synthesized.verifier, &heldout.provider_payload, "[3,5]")
            .is_ok()
    );
    assert!(
        verify_response_independently(&synthesized.verifier, &heldout.provider_payload, "[3,4,5]")
            .is_err()
    );
}

#[test]
fn synthesis_discovers_typed_output_renderer_across_renamed_layouts() {
    let mut support = vec![
        example("rows", "kind", "value"),
        example("entries", "tag", "amount"),
    ];
    support[0].expected_response = "Selected values: [3,5].".to_owned();
    support[1].provider_payload = json!({
        "input":[{"type":"function_call_output","output":
            "{\"entries\":[{\"tag\":\"keep\",\"amount\":7},{\"tag\":\"drop\",\"amount\":8},{\"tag\":\"keep\",\"amount\":9}]}"
        }]
    });
    support[1].expected_response = "Selected values: [7,9].".to_owned();
    let synthesized = synthesize_collection_program(&support).expect("template synthesis");
    assert!(matches!(
        &synthesized.program.operation,
        ResponseOperation::ComposeCollection {
            renderer: CollectionOutputRenderer::RenderTemplate { prefix, suffix },
            ..
        } if prefix == "Selected values: " && suffix == "."
    ));
    let heldout = example("records", "marker", "score");
    let execution = execute_response(&synthesized.program, "", &heldout.provider_payload);
    assert_eq!(
        execution.response.as_deref(),
        Some("Selected values: [3,5].")
    );
    assert!(
        verify_response_independently(
            &synthesized.verifier,
            &heldout.provider_payload,
            "Selected values: [3,5]."
        )
        .is_ok()
    );
    assert!(
        verify_response_independently(&synthesized.verifier, &heldout.provider_payload, "[3,5]")
            .is_err()
    );
}

#[test]
fn role_based_adapter_abstains_when_collection_or_projection_is_ambiguous() {
    let support = vec![
        example("rows", "kind", "value"),
        example("entries", "tag", "amount"),
    ];
    let synthesized = synthesize_collection_program(&support).expect("synthesis");
    let ambiguous = json!({
        "input":[{"type":"function_call_output","output":json!({
            "left":[{"kind":"keep","value":3}],
            "right":[{"tag":"keep","amount":3}]
        }).to_string()}]
    });
    assert_eq!(
        execute_response(&synthesized.program, "", &ambiguous).status,
        ResponseExecutionStatus::Abstain
    );
}

#[test]
fn synthesis_discovers_filter_count_without_operator_label() {
    let mut support = vec![
        example("rows", "kind", "value"),
        example("entries", "tag", "amount"),
    ];
    for example in &mut support {
        example.expected_response = "2".to_owned();
    }
    let synthesized = synthesize_collection_program(&support).expect("count synthesis");
    let ResponseOperation::ComposeCollection { steps, format, .. } = &synthesized.program.operation
    else {
        panic!("collection program");
    };
    assert_eq!(*format, ValueProjectionFormat::PlainText);
    assert!(matches!(steps.last(), Some(CollectionProgramStep::Count)));
    let heldout = example("records", "marker", "score");
    assert_eq!(
        execute_response(&synthesized.program, "", &heldout.provider_payload)
            .response
            .as_deref(),
        Some("2")
    );
}

#[test]
fn synthesis_unwraps_structural_json_from_text_part_arrays() {
    let wrapped = |wrapper_field: &str, collection_field: &str, values: &[i64]| {
        let mut inner = serde_json::Map::new();
        inner.insert(
            collection_field.to_owned(),
            Value::Array(
                values
                    .iter()
                    .copied()
                    .map(|value| json!({"value":value}))
                    .collect(),
            ),
        );
        let mut envelope = serde_json::Map::new();
        envelope.insert("sequence".to_owned(), Value::from(values.len() as u64));
        envelope.insert(
            wrapper_field.to_owned(),
            Value::String(Value::Object(inner).to_string()),
        );
        CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{
                    "type":"function_call_output",
                    "output":[
                        {"type":"input_text","text":"Script completed\nOutput:\n"},
                        {"type":"input_text","text":Value::Object(envelope).to_string()}
                    ]
                }]
            }),
            expected_response: values.len().to_string(),
        }
    };
    let support = vec![
        wrapped("payload_blob", "alpha", &[1, 2]),
        wrapped("result_text", "beta", &[4, 5, 6]),
    ];
    let synthesized = synthesize_collection_program(&support).expect("count synthesis");
    let heldout = wrapped("encoded_value", "gamma", &[7, 8, 9, 10]);

    assert_eq!(
        execute_response(&synthesized.program, "", &heldout.provider_payload)
            .response
            .as_deref(),
        Some("4")
    );
    assert!(
        verify_response_independently(&synthesized.verifier, &heldout.provider_payload, "4")
            .is_ok()
    );
}

#[test]
fn synthesis_discovers_direct_projection_by_structural_scalar_type() {
    let mut support = vec![
        example("rows", "kind", "value"),
        example("entries", "tag", "amount"),
    ];
    for example in &mut support {
        example.expected_response = "[3,4,5]".to_owned();
    }
    let synthesized = synthesize_collection_program(&support).expect("direct projection");
    let heldout = example("records", "marker", "score");
    assert_eq!(
        execute_response(&synthesized.program, "", &heldout.provider_payload)
            .response
            .as_deref(),
        Some("[3,4,5]")
    );
    let ambiguous = serde_json::json!({
        "input":[{"type":"function_call_output","output":serde_json::json!({
            "records":[
                {"marker":"keep","score":3,"rank":30},
                {"marker":"drop","score":4,"rank":40}
            ]
        }).to_string()}]
    });
    assert_eq!(
        execute_response(&synthesized.program, "", &ambiguous).status,
        ResponseExecutionStatus::Abstain
    );
}

#[test]
fn strict_synthesis_refuses_top_one_when_multiple_programs_survive() {
    let support = [example("rows", "kind", "value")];
    assert_eq!(
        synthesize_unique_collection_program(&support),
        Err("collection_ambiguous_programs")
    );
}

#[test]
fn source_neutral_version_space_contains_no_field_names_or_observed_literals() {
    let mut input = example("private_rows", "client_kind", "customer_amount");
    input.expected_response = "3".to_owned();
    let version_space =
        enumerate_source_neutral_collection_programs(&input).expect("version space");
    assert!(!version_space.programs.is_empty());
    for program in version_space.programs {
        assert!(is_source_neutral_collection_program(&program));
        let encoded = serde_json::to_string(&program).expect("program json");
        for private in ["private_rows", "client_kind", "customer_amount", "keep"] {
            assert!(!encoded.contains(private), "program leaked {private}");
        }
    }
}

#[test]
fn dynamic_coverage_reports_literal_tool_derived_upper_bound_without_storing_text() {
    let example = CollectionSynthesisExample {
        provider_payload: serde_json::json!({
            "input":[{
                "type":"function_call_output",
                "output":"picked: 42\ntotal: 99"
            }]
        }),
        expected_response: "Selected 42 of 99".to_owned(),
    };
    let diagnostic = diagnose_response_dynamic_coverage(&example);
    assert_eq!(diagnostic.response_bytes, 17);
    assert_eq!(diagnostic.dynamic_bytes, 4);
    assert_eq!(diagnostic.request_dynamic_bytes, 0);
    assert_eq!(diagnostic.tool_dynamic_bytes, 4);
    assert!(diagnostic.matching_selectors >= 2);
}

#[test]
fn semantic_canonical_projection_requires_one_unambiguous_value_and_flexible_surface() {
    let program = ResponseProgram::project_selected_value(
        ResponseValueSelector::UniqueTurnScalar {
            value_type: AtomValueType::Integer,
        },
        ValueProjectionFormat::PlainText,
        "completed",
    );
    let canonical = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Сколько успешных записей?"}]},
                {"type":"function_call_output","output":"{\"ok\":3}"}
            ]
        }),
        expected_response: "Успешных записей: 3".to_owned(),
    };
    assert!(response_program_matches_example(&program, &canonical));
    assert!(!response_program_exactly_matches_example(
        &program, &canonical
    ));

    let ambiguous = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Сколько записей?"}]},
                {"type":"function_call_output","output":"{\"ok\":3,\"failed\":2}"}
            ]
        }),
        expected_response: "Успешных: 3, ошибочных: 2".to_owned(),
    };
    assert!(!response_program_matches_example(&program, &ambiguous));
    assert!(!response_program_authority_matches_example(
        &program, &ambiguous
    ));

    let exact_surface = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Ответь ровно: Успешных записей: 3"}]},
                {"type":"function_call_output","output":"{\"ok\":3}"}
            ]
        }),
        expected_response: "Успешных записей: 3".to_owned(),
    };
    assert!(!response_program_matches_example(&program, &exact_surface));

    let json_discussion = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Прочитай JSON и скажи, сколько успешных записей"}]},
                {"type":"function_call_output","output":"{\"ok\":3}"}
            ]
        }),
        expected_response: "Успешных записей: 3".to_owned(),
    };
    assert!(response_program_matches_example(&program, &json_discussion));

    let json_only = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Верни только JSON"}]},
                {"type":"function_call_output","output":"{\"ok\":3}"}
            ]
        }),
        expected_response: "Успешных записей: 3".to_owned(),
    };
    assert!(!response_program_matches_example(&program, &json_only));
}

#[test]
fn request_grounded_projection_has_partial_teacher_authority_without_ordinal_leak() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Return the ok field"}]},
                {"type":"function_call_output","output":"{\"ok\":3,\"failed\":2}"}
            ]
        }),
        expected_response: "ok=3; failed=2".to_owned(),
    };
    let grounded = ResponseProgram::project_selected_value(
        ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::Integer,
        },
        ValueProjectionFormat::PlainText,
        "completed",
    );
    assert!(response_program_matches_example(&grounded, &example));
    assert!(response_program_authority_matches_example(
        &grounded, &example
    ));
    let verifier = source_neutral_verifier_for_program(&grounded).expect("verifier");
    assert!(verify_response_independently(&verifier, &example.provider_payload, "3").is_ok());

    let ordinal = ResponseProgram::project_selected_value(
        ResponseValueSelector::JsonScalarOrdinal {
            ordinal: 0,
            value_type: AtomValueType::Integer,
        },
        ValueProjectionFormat::PlainText,
        "completed",
    );
    assert!(response_program_matches_example(&ordinal, &example));
    assert!(!response_program_authority_matches_example(
        &ordinal, &example
    ));
}

#[test]
fn request_grounded_ordinal_projection_keeps_role_order_in_actor_and_verifier() {
    let program = ResponseProgram::project_selected_value(
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal: 0,
            value_type: AtomValueType::Integer,
        },
        ValueProjectionFormat::PlainText,
        "completed",
    );
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Return ok, then failed"}]},
                {"type":"function_call_output","output":"{\"ok\":3,\"failed\":2}"}
            ]
        }),
        expected_response: "3 and 2".to_owned(),
    };

    assert!(response_program_authority_matches_example(
        &program, &example
    ));
    let execution = execute_example(&program, &example);
    assert_eq!(execution.response.as_deref(), Some("3"));
    let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
    assert!(verify_response_independently(&verifier, &example.provider_payload, "3").is_ok());
    assert!(verify_response_independently(&verifier, &example.provider_payload, "2").is_err());
}

#[test]
fn semantic_status_inside_teacher_prose_keeps_actor_verifier_parity() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Did the command succeed?"}]},
                {"type":"function_call_output","output":"{\"exit_code\":0}"}
            ]
        }),
        expected_response: "Status: success.".to_owned(),
    };
    let direct = ResponseProgram::project_status(
        ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Integer,
        },
        ProjectStatusMapping::ZeroIsSuccess,
        "completed",
    );
    assert_eq!(
        execute_response(&direct, "", &example.provider_payload)
            .response
            .as_deref(),
        Some("success")
    );
    let rendered = direct.with_status_renderer(CollectionOutputRenderer::RenderTemplate {
        prefix: "Status: ".to_owned(),
        suffix: ".".to_owned(),
    });
    assert_eq!(rendered.validate(), Ok(()));
    assert_eq!(
        execute_response(&rendered, "", &example.provider_payload)
            .response
            .as_deref(),
        Some(example.expected_response.as_str())
    );
    let program = enumerate_source_neutral_response_programs(&example)
        .expect("programs")
        .programs
        .into_iter()
        .find(|program| {
            matches!(
                &program.operation,
                ResponseOperation::ProjectStatus {
                    mapping: ProjectStatusMapping::ZeroIsSuccess,
                    renderer: CollectionOutputRenderer::RenderTemplate { .. },
                    ..
                }
            ) && execute_response(program, "", &example.provider_payload)
                .response
                .as_deref()
                == Some(example.expected_response.as_str())
        })
        .expect("semantic status program");
    assert!(response_program_matches_example(&program, &example));
    assert!(response_program_exactly_matches_example(&program, &example));
    assert_eq!(
        execute_response(&program, "", &example.provider_payload)
            .response
            .as_deref(),
        Some(example.expected_response.as_str())
    );
    let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
    assert!(
        verify_response_independently(
            &verifier,
            &example.provider_payload,
            &example.expected_response,
        )
        .is_ok()
    );
    assert!(
        verify_response_independently(&verifier, &example.provider_payload, "failure").is_err()
    );
}

#[test]
fn long_irreducible_response_skips_impossible_scalar_surface_search() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input": [
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Summarize it"}]},
                {"type":"function_call_output","output":"{\"value\":3,\"ok\":true}"}
            ]
        }),
        expected_response: "unrelated prose ".repeat(300),
    };
    let coverage = diagnose_response_dynamic_coverage(&example);
    assert!(
        coverage
            .response_bytes
            .saturating_sub(coverage.dynamic_bytes)
            > 512
    );

    let version_space =
        enumerate_source_neutral_response_programs_with_coverage(&example, Some(coverage))
            .expect("bounded version space");

    assert!(version_space.programs.is_empty());
    assert_eq!(version_space.candidates_enumerated, 0);
    assert_eq!(version_space.exact_checks, 0);
}

#[test]
fn long_teacher_prose_preserves_canonical_multi_scalar_law() {
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input": [
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Summarize the result"}]},
                {"type":"function_call_output","output":"{\"value\":3,\"ok\":true}"}
            ]
        }),
        expected_response: format!("{} value 3 and true", "x".repeat(600)),
    };
    let coverage = diagnose_response_dynamic_coverage(&example);
    assert!(
        coverage
            .response_bytes
            .saturating_sub(coverage.dynamic_bytes)
            > 512
    );

    let program =
        enumerate_source_neutral_response_programs_with_coverage(&example, Some(coverage))
            .expect("bounded version space")
            .programs
            .into_iter()
            .find(|program| response_program_authority_matches_example(program, &example))
            .expect("canonical multi-scalar law");

    assert!(!response_program_exactly_matches_example(
        &program, &example
    ));
    let response = execute_example(&program, &example)
        .response
        .expect("canonical response");
    assert!(response.lines().any(|value| value == "3"));
    assert!(response.lines().any(|value| value == "true"));
    let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
    assert!(verify_response_independently(&verifier, &example.provider_payload, &response).is_ok());
}

#[test]
fn law_quotient_separates_semantic_law_from_physical_selector_adapter() {
    let project = |selector| {
        ResponseProgram::project_selected_value(
            selector,
            ValueProjectionFormat::PlainText,
            "completed",
        )
    };
    let named_alpha = project(ResponseValueSelector::JsonField {
        field: "alpha".to_owned(),
        value_type: AtomValueType::Integer,
    });
    let named_beta = project(ResponseValueSelector::JsonField {
        field: "renamed_beta".to_owned(),
        value_type: AtomValueType::Integer,
    });
    let ordinal_zero = project(ResponseValueSelector::JsonScalarOrdinal {
        ordinal: 0,
        value_type: AtomValueType::Integer,
    });
    let ordinal_one = project(ResponseValueSelector::JsonScalarOrdinal {
        ordinal: 1,
        value_type: AtomValueType::Integer,
    });
    let from_end = project(ResponseValueSelector::LatestTurnOutputScalarFromEnd {
        reverse_ordinal: 0,
        value_type: AtomValueType::Integer,
    });
    let request_referenced = project(ResponseValueSelector::RequestReferencedJsonField {
        value_type: AtomValueType::Integer,
    });
    let selected_beta =
        named_alpha
            .clone()
            .with_value_renderer(CollectionOutputRenderer::RenderSequence {
                segments: vec![
                    ResponseRenderSegment::Static {
                        text: "Result: ".to_owned(),
                    },
                    ResponseRenderSegment::Selected {
                        selector: ResponseValueSelector::JsonField {
                            field: "renamed_beta".to_owned(),
                            value_type: AtomValueType::Integer,
                        },
                        format: ValueProjectionFormat::PlainText,
                    },
                ],
            });

    assert_eq!(
        response_law_key(&named_alpha).expect("alpha law"),
        response_law_key(&named_beta).expect("renamed law")
    );
    let named_law = response_law_key(&named_alpha).expect("named law");
    let distinct = [ordinal_zero, ordinal_one, from_end, request_referenced]
        .iter()
        .map(|program| response_law_key(program).expect("structural law"))
        .collect::<BTreeSet<_>>();
    assert_eq!(distinct, BTreeSet::from([named_law]));
    let canonical_selected =
        canonical_direct_response_program(&selected_beta).expect("selected law");
    assert_eq!(canonical_selected, named_beta);
    let boolean = project(ResponseValueSelector::UniqueScalar {
        value_type: AtomValueType::Boolean,
    });
    assert_ne!(
        response_law_key(&named_alpha).expect("integer law"),
        response_law_key(&boolean).expect("boolean law")
    );

    let count_from = |output_ordinal| {
        ResponseProgram::compose_collection(
            vec![
                CollectionProgramStep::SelectTurnOutput { output_ordinal },
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::Count,
            ],
            ValueProjectionFormat::PlainText,
            "completed",
        )
    };
    let first_output = count_from(1);
    let second_output = count_from(2);
    assert_ne!(first_output, second_output);
    assert_eq!(
        response_law_key(&first_output).expect("first output law"),
        response_law_key(&second_output).expect("second output law")
    );
}

#[test]
fn law_quotient_requires_consensus_variants_to_share_one_law() {
    let project = |selector| {
        ResponseProgram::project_selected_value(
            selector,
            ValueProjectionFormat::PlainText,
            "completed",
        )
    };
    let integer_alpha = project(ResponseValueSelector::JsonField {
        field: "alpha".to_owned(),
        value_type: AtomValueType::Integer,
    });
    let integer_beta = project(ResponseValueSelector::JsonField {
        field: "beta".to_owned(),
        value_type: AtomValueType::Integer,
    });
    let boolean = project(ResponseValueSelector::UniqueScalar {
        value_type: AtomValueType::Boolean,
    });
    let variant = |program| crate::ResponseConsensusVariant {
        program,
        allowed_layout_sha256: Vec::new(),
        required_request_atom_ids: Vec::new(),
    };
    let unanimous = ResponseProgram::unique_consensus(vec![
        variant(integer_alpha.clone()),
        variant(integer_beta),
    ]);
    assert_eq!(
        response_law_key(&unanimous).expect("unanimous consensus law"),
        response_law_key(&integer_alpha).expect("integer law")
    );

    let mixed = ResponseProgram::unique_consensus(vec![variant(integer_alpha), variant(boolean)]);
    assert_eq!(
        response_law_key(&mixed),
        Err("response_law_key_consensus_mixed")
    );
}
