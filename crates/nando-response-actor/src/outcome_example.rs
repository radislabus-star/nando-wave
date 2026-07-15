use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{CollectionSynthesisExample, canonical_json_sha256};

pub const COMPLETED_TURN_EXAMPLE_SCHEMA_V1: &str = "nando.completed-turn-example.v1";
pub const COMPLETED_TURN_EXAMPLE_SCHEMA_V2: &str = "nando.completed-turn-example.v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompletedTurnRuntime {
    pub provider_payload: Value,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnCompletionReason {
    FinalAnswerSettled,
    NextTurnBoundary,
    EndOfStream,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrainingTarget {
    FinalResponse { response: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompletedTurnTeacher {
    pub completion_reason: TurnCompletionReason,
    pub target: TrainingTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompletedTurnExample {
    pub schema: String,
    pub runtime: CompletedTurnRuntime,
    pub teacher: CompletedTurnTeacher,
}

impl CompletedTurnExample {
    pub fn final_response(provider_payload: Value, response: String) -> Result<Self, &'static str> {
        Self::final_response_with_reason(
            provider_payload,
            response,
            TurnCompletionReason::FinalAnswerSettled,
        )
    }

    pub fn final_response_with_reason(
        provider_payload: Value,
        response: String,
        completion_reason: TurnCompletionReason,
    ) -> Result<Self, &'static str> {
        if !provider_payload.is_object() {
            return Err("completed_turn_runtime_payload_invalid");
        }
        if response.is_empty() || response.len() > 16_384 {
            return Err("completed_turn_target_budget");
        }
        Ok(Self {
            schema: COMPLETED_TURN_EXAMPLE_SCHEMA_V2.to_owned(),
            runtime: CompletedTurnRuntime { provider_payload },
            teacher: CompletedTurnTeacher {
                completion_reason,
                target: TrainingTarget::FinalResponse { response },
            },
        })
    }

    pub fn runtime_fingerprint_sha256(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            COMPLETED_TURN_EXAMPLE_SCHEMA_V2,
            &self.runtime.provider_payload,
        ))
    }

    #[must_use]
    pub fn into_synthesis_example(self) -> CollectionSynthesisExample {
        let TrainingTarget::FinalResponse { response } = self.teacher.target;
        CollectionSynthesisExample {
            provider_payload: self.runtime.provider_payload,
            expected_response: response,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn runtime_fingerprint_is_independent_of_training_target() {
        let payload = json!({
            "input": [{
                "type": "function_call_output",
                "call_id": "call-1",
                "output": "{\"value\":7}"
            }]
        });
        let left = CompletedTurnExample::final_response(payload.clone(), "Value: 7".to_owned())
            .expect("left example");
        let right = CompletedTurnExample::final_response(payload, "Different target".to_owned())
            .expect("right example");
        assert_eq!(
            left.runtime_fingerprint_sha256().expect("left digest"),
            right.runtime_fingerprint_sha256().expect("right digest")
        );
        assert_eq!(left.schema, COMPLETED_TURN_EXAMPLE_SCHEMA_V2);
    }
}
