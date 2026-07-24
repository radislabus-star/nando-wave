use nando_operator_kernel::{
    AtomValueType, ResponseProgram, ResponseValueSelector, ValueProjectionFormat,
};
use serde_json::json;

use crate::{RuntimeOperatorSpec, bind_pre_action_with_validator, compile_runtime_program};

fn request_relative_projection() -> ResponseProgram {
    ResponseProgram::project_selected_value(
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal: 0,
            value_type: AtomValueType::String,
        },
        ValueProjectionFormat::PlainText,
        "completed",
    )
}

fn payload_with_two_string_fields() -> serde_json::Value {
    json!({
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Return value"}]
            },
            {
                "type": "function_call_output",
                "call_id": "call-1",
                "output": "{\"noise\":\"wrong\",\"value\":\"right\"}"
            }
        ]
    })
}

#[test]
fn request_relative_role_restores_virtual_output_and_selects_one_action() {
    let program = request_relative_projection();
    let compiled = compile_runtime_program(&program).expect("compiled operator");
    assert_eq!(compiled.role_graph().role_count(), 3);

    let bound = bind_pre_action_with_validator(
        RuntimeOperatorSpec::new(
            compiled.role_graph(),
            compiled.relation_program(),
            compiled.transform_program(),
            &program,
            None,
        ),
        "Return value",
        &payload_with_two_string_fields(),
        |_, response| (response == "right").then_some(()).ok_or("wrong response"),
    )
    .expect("request-relative role must remain unique");

    assert_eq!(bound.execute_unverified().expect("VM response"), "right");
}

#[test]
fn request_relative_role_abstains_when_request_does_not_bind_it() {
    let program = request_relative_projection();
    let compiled = compile_runtime_program(&program).expect("compiled operator");
    let result = bind_pre_action_with_validator(
        RuntimeOperatorSpec::new(
            compiled.role_graph(),
            compiled.relation_program(),
            compiled.transform_program(),
            &program,
            None,
        ),
        "Return result",
        &payload_with_two_string_fields(),
        |_, _| Ok::<(), &'static str>(()),
    );

    assert!(result.is_err(), "an ungrounded role must fail closed");
}
