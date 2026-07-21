pub use nando_operator_proof::verifier::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AtomValueType, CollectionOutputRenderer, CollectionProgramStep, ProjectStatusMapping,
        ResponseProgram, ResponseValueSelector, ValueProjectionFormat, VerifierProgram,
    };
    use serde_json::Value;

    fn projection_payload() -> Value {
        serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": "value=ready"
            }]
        })
    }

    fn projection_verifier(require_unique_value: bool) -> VerifierProgram {
        VerifierProgram::ProjectSelectedValue {
            selector: ResponseValueSelector::ContentLinePrefix {
                prefix: "value=".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "pending".to_owned(),
            require_unique_value,
        }
    }

    #[test]
    fn continuation_role_is_recomputed_independently() {
        let payload = serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": "noise 17\nProcess running with session ID session-9\nnoise 99"
            }]
        });
        let program = crate::ResponseProgram::custom_tool_call_from_roles(
            "exec",
            "write_stdin",
            ResponseValueSelector::ContinuationHandle {
                value_type: AtomValueType::Identifier,
            },
            vec![crate::ResponseArgument::Role {
                name: "session_id".to_owned(),
                role: crate::SemanticRole::ContinuationHandle,
                value_type: Some(AtomValueType::Identifier),
            }],
            crate::CustomToolResultProjection::JsonStringifyResult,
        );
        let execution = crate::execute_response(&program, "", &payload);
        let response = execution.response.expect("actor response");
        let verifier = crate::source_neutral_verifier_for_program(&program).expect("verifier");

        assert!(verify_response_independently(&verifier, &payload, &response).is_ok());
        assert!(
            verify_response_independently(
                &verifier,
                &payload,
                &response.replace("session-9", "session-wrong"),
            )
            .is_err()
        );
    }

    #[test]
    fn call_adapter_wave_atoms_have_actor_and_independent_parity() {
        let program = crate::ResponseProgram::function_call_from_roles(
            "wait",
            ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            vec![crate::ResponseArgument::Role {
                name: "cell_id".to_owned(),
                role: crate::SemanticRole::ContinuationHandle,
                value_type: Some(AtomValueType::Identifier),
            }],
        );
        let payload = serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "input_text", "text": "wait for script"}]
                },
                {
                    "type": "function_call",
                    "name": "exec_command",
                    "call_id": "call-1",
                    "arguments": "{}"
                },
                {
                    "type": "function_call_output",
                    "call_id": "call-1",
                    "output": "Script running with cell ID handle-1"
                }
            ]
        });
        let actor = crate::runtime::actor_adapter_phase_atom_ids(&program, &payload);
        let independent = independently_adapter_phase_atom_ids(&program, &payload);
        let verifier = crate::source_neutral_verifier_for_program(&program).expect("verifier");
        let compiled = independently_verifier_adapter_phase_atom_ids(&verifier, &payload);

        assert!(!actor.is_empty());
        assert!(actor.contains(&crate::stable_atom_id(
            "observation_call_shape:function_call"
        )));
        assert!(actor.contains(&crate::stable_atom_id("tool_kind:exec_command")));
        assert_eq!(actor, independent);
        assert_eq!(actor, compiled);

        let custom_program = crate::ResponseProgram::custom_tool_call_from_roles(
            "exec",
            "write_stdin",
            ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            vec![crate::ResponseArgument::Role {
                name: "session_id".to_owned(),
                role: crate::SemanticRole::ContinuationHandle,
                value_type: Some(AtomValueType::Identifier),
            }],
            crate::CustomToolResultProjection::JsonStringifyResult,
        );
        let custom_actor = crate::runtime::actor_adapter_phase_atom_ids(&custom_program, &payload);
        let custom_independent = independently_adapter_phase_atom_ids(&custom_program, &payload);
        let custom_verifier =
            crate::source_neutral_verifier_for_program(&custom_program).expect("custom verifier");
        let custom_compiled =
            independently_verifier_adapter_phase_atom_ids(&custom_verifier, &payload);

        assert_eq!(custom_actor, custom_independent);
        assert_eq!(custom_actor, custom_compiled);
    }

    #[test]
    fn independent_value_projection_verifier_recomputes_and_rejects_mutation() {
        let verifier = projection_verifier(true);
        assert!(verify_response_independently(&verifier, &projection_payload(), "ready").is_ok());
        assert!(
            verify_response_independently(&verifier, &projection_payload(), "changed").is_err()
        );
    }

    #[test]
    fn independent_value_projection_verifier_requires_uniqueness_contract() {
        assert!(
            verify_response_independently(
                &projection_verifier(false),
                &projection_payload(),
                "ready",
            )
            .is_err()
        );
    }

    #[test]
    fn independent_value_projection_verifier_rejects_private_renderer() {
        let mut verifier = projection_verifier(true);
        if let VerifierProgram::ProjectSelectedValue { renderer, .. } = &mut verifier {
            *renderer = CollectionOutputRenderer::RenderTemplate {
                prefix: "Customer Acme secret: ".to_owned(),
                suffix: String::new(),
            };
        }
        assert_eq!(
            verify_response_independently(
                &verifier,
                &projection_payload(),
                "Customer Acme secret: ready",
            ),
            Err(ResponseVerificationError("value_renderer_unsafe"))
        );
    }

    #[test]
    fn actor_side_first_scalar_mutation_cannot_authorize_ambiguous_evidence() {
        let verifier = VerifierProgram::ProjectSelectedValue {
            selector: ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::String,
            },
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
            require_unique_value: true,
        };
        let ambiguous = serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"first\":\"actor-selected\",\"second\":\"other\"}"
            }]
        });
        assert_eq!(
            verify_response_independently(&verifier, &ambiguous, "actor-selected"),
            Err(ResponseVerificationError("unique_scalar_cardinality"))
        );
    }

    fn status_verifier(selector: ResponseValueSelector) -> VerifierProgram {
        VerifierProgram::ProjectStatus {
            selector,
            mapping: ProjectStatusMapping::ZeroIsSuccess,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
            require_unique_value: true,
        }
    }

    #[test]
    fn independent_status_verifier_reparses_and_recomputes_exact_output() {
        let verifier = status_verifier(ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        });
        let success = serde_json::json!({
            "input": [{"type":"function_call_output","output":"{\"exit_code\":0}"}]
        });
        let failure = serde_json::json!({
            "input": [{"type":"function_call_output","output":"{\"exit_code\":23}"}]
        });
        assert!(verify_response_independently(&verifier, &success, "success").is_ok());
        assert!(verify_response_independently(&verifier, &failure, "failure").is_ok());
        assert_eq!(
            verify_response_independently(&verifier, &success, "failure"),
            Err(ResponseVerificationError("response_mismatch"))
        );
        assert_eq!(
            verify_response_independently(&verifier, &failure, "success"),
            Err(ResponseVerificationError("response_mismatch"))
        );
    }

    #[test]
    fn independent_status_verifier_rejects_ambiguity_staleness_type_and_bounds() {
        let verifier = status_verifier(ResponseValueSelector::ContentLinePrefix {
            prefix: "exit_code=".to_owned(),
            value_type: AtomValueType::Integer,
        });
        for payload in [
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=0\nexit_code=1"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=-1"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=1000001"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=true"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=success"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=0"},{"type":"message","role":"user","content":"new turn"}]}),
        ] {
            assert!(verify_response_independently(&verifier, &payload, "success").is_err());
        }
    }

    #[test]
    fn independent_status_verifier_rejects_unknown_content_part_types() {
        let verifier = status_verifier(ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        });
        let payload = serde_json::json!({
            "input":[{
                "type":"function_call_output",
                "output":[{"type":"future_text","text":"{\"exit_code\":0}"}]
            }]
        });
        assert_eq!(
            verify_response_independently(&verifier, &payload, "success"),
            Err(ResponseVerificationError("unsupported_output_part_type"))
        );
    }

    #[test]
    fn turn_unique_selector_has_actor_verifier_parity_and_abstains_on_concurrency() {
        let selector = ResponseValueSelector::UniqueTurnJsonField {
            field: "status".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = ResponseProgram::project_status(
            selector.clone(),
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        let verifier = status_verifier(selector);
        let single = serde_json::json!({
            "input": [
                {"type":"message","role":"user","content":"check"},
                {"type":"function_call_output","output":"{\"status\":0}"},
                {"type":"function_call_output","output":"{\"status\":0}"}
            ]
        });
        let concurrent = serde_json::json!({
            "input": [
                {"type":"message","role":"user","content":"check"},
                {"type":"function_call_output","output":"{\"status\":0}"},
                {"type":"function_call_output","output":"{\"status\":1}"}
            ]
        });
        let execution = crate::execute_response(&program, "", &single);
        assert_eq!(execution.status, crate::ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("success"));
        assert!(verify_response_independently(&verifier, &single, "success").is_ok());
        assert_eq!(
            crate::execute_response(&program, "", &concurrent).status,
            crate::ResponseExecutionStatus::Abstain
        );
        assert!(verify_response_independently(&verifier, &concurrent, "success").is_err());
    }

    #[test]
    fn active_turn_selector_releases_completed_single_handle_and_rejects_concurrency() {
        let selector = ResponseValueSelector::UniqueActiveTurnJsonField {
            field: "status".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = ResponseProgram::project_status(
            selector.clone(),
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        let verifier = status_verifier(selector);
        let sequential = serde_json::json!({
            "input": [
                {"type":"message","role":"user","content":"check"},
                {"type":"function_call_output","output":"{\"status\":0}"},
                {"type":"function_call_output","output":"{\"exit_code\":0}"},
                {"type":"function_call_output","output":"{\"status\":1}"}
            ]
        });
        let concurrent = serde_json::json!({
            "input": [
                {"type":"message","role":"user","content":"check"},
                {"type":"function_call_output","output":"{\"status\":0}"},
                {"type":"function_call_output","output":"{\"status\":1}"}
            ]
        });
        let execution = crate::execute_response(&program, "", &sequential);
        assert_eq!(execution.status, crate::ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("failure"));
        assert!(verify_response_independently(&verifier, &sequential, "failure").is_ok());
        assert_eq!(
            crate::execute_response(&program, "", &concurrent).status,
            crate::ResponseExecutionStatus::Abstain
        );
        assert!(verify_response_independently(&verifier, &concurrent, "success").is_err());
    }

    #[test]
    fn independent_status_verifier_requires_integer_unique_freshness_contract() {
        let mut verifier = status_verifier(ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Integer,
        });
        if let VerifierProgram::ProjectStatus {
            require_unique_value,
            ..
        } = &mut verifier
        {
            *require_unique_value = false;
        }
        assert_eq!(
            verify_response_independently(
                &verifier,
                &serde_json::json!({"input":[{"type":"function_call_output","output":"0"}]}),
                "success",
            ),
            Err(ResponseVerificationError("status_projection_guard_missing"))
        );

        let wrong_type = status_verifier(ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Boolean,
        });
        assert_eq!(
            verify_response_independently(
                &wrong_type,
                &serde_json::json!({"input":[{"type":"function_call_output","output":"true"}]}),
                "success",
            ),
            Err(ResponseVerificationError("status_selector_must_be_integer"))
        );
    }

    #[test]
    fn independent_collection_verifier_rejects_private_renderer() {
        let verifier = VerifierProgram::ComposeCollection {
            steps: vec![
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::Count,
            ],
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::RenderTemplate {
                prefix: "token=AbCdEfGhIjKlMnOpQrStUv123456 ".to_owned(),
                suffix: String::new(),
            },
            completion_state: "completed".to_owned(),
            max_items: 1_024,
        };
        let payload = serde_json::json!({
            "input":[{"type":"function_call_output","output":"{\"rows\":[{\"value\":1}]}"}]
        });
        assert_eq!(
            verify_response_independently(&verifier, &payload, "1"),
            Err(ResponseVerificationError("collection_renderer_unsafe"))
        );
    }

    #[test]
    fn advance_plan_actor_and_independent_verifier_require_canonical_success() {
        let payload = serde_json::json!({
            "input": [
                {
                    "type": "function_call",
                    "name": "update_plan",
                    "call_id": "plan-1",
                    "arguments": serde_json::json!({
                        "plan": [
                            {"step":"Inspect","status":"completed"},
                            {"step":"Implement","status":"in_progress"},
                            {"step":"Verify","status":"pending"}
                        ]
                    }).to_string()
                },
                {
                    "type": "function_call_output",
                    "call_id": "tool-1",
                    "output": "Chunk ID: plan\nWall time: 0.1 seconds\nProcess exited with code 0\nFinal output:\nverified"
                }
            ]
        });
        let program = ResponseProgram::advance_plan("update_plan");
        let expected = serde_json::json!({
            "name": "update_plan",
            "arguments": {
                "plan": [
                    {"step":"Inspect","status":"completed"},
                    {"step":"Implement","status":"completed"},
                    {"step":"Verify","status":"in_progress"}
                ]
            }
        });
        let execution = crate::execute_response(&program, "", &payload);
        let response = execution.response.expect("actor response");
        assert_eq!(
            serde_json::from_str::<Value>(&response).expect("plan response json"),
            expected
        );
        let verifier = VerifierProgram::AdvancePlan {
            function_name: "update_plan".to_owned(),
            require_explicit_tool_success: true,
            require_canonical_plan: true,
        };
        assert!(verify_response_independently(&verifier, &payload, &response).is_ok());

        let mut failed = payload.clone();
        failed["input"][1]["output"] = Value::String(
            "Chunk ID: plan\nProcess exited with code 1\nFinal output:\nfailed".to_owned(),
        );
        assert!(
            crate::execute_response(&program, "", &failed)
                .response
                .is_none()
        );
        assert!(verify_response_independently(&verifier, &failed, &response).is_err());

        let mut contradictory = payload.clone();
        contradictory["input"][1]["output"] =
            Value::String("Process exited with code 0\nProcess exited with code 1".to_owned());
        assert!(
            crate::execute_response(&program, "", &contradictory)
                .response
                .is_none()
        );
        assert!(verify_response_independently(&verifier, &contradictory, &response).is_err());

        let mut noncanonical = payload;
        noncanonical["input"][0]["arguments"] = Value::String(
            serde_json::json!({
                "plan": [
                    {"step":"Inspect","status":"completed"},
                    {"step":"Implement","status":"pending"},
                    {"step":"Verify","status":"in_progress"}
                ]
            })
            .to_string(),
        );
        assert!(
            crate::execute_response(&program, "", &noncanonical)
                .response
                .is_none()
        );
    }
}
