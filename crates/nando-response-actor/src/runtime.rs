pub use nando_operator_runtime::*;

use nando_operator_kernel::ResponseProgram;
use serde_json::Value;

/// Compatibility orchestration: runtime proposes an actor result and the
/// independent proof owner decides whether that result may be exposed.
pub fn execute_response(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> ResponseExecution {
    execute_response_with_external_validator(
        program,
        request_text,
        provider_payload,
        &|program, request_text, provider_payload, response| {
            nando_operator_proof::verify_response(program, request_text, provider_payload, response)
                .map_err(|error| error.to_string())
        },
    )
}
