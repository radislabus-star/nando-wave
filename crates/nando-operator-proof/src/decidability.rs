use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuDecidabilityClass {
    PotentiallyCpuExecutable,
    UnexploredMultiSource,
    NotExecutableCurrentEvidence,
    UnsupportedByCurrentDsl,
}

impl CpuDecidabilityClass {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PotentiallyCpuExecutable => "potentially_cpu_executable",
            Self::UnexploredMultiSource => "unexplored_multi_source",
            Self::NotExecutableCurrentEvidence => "not_executable_current_evidence",
            Self::UnsupportedByCurrentDsl => "unsupported_by_current_dsl",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuDecidability {
    pub class: CpuDecidabilityClass,
    pub reason: &'static str,
}

#[must_use]
pub fn classify_cpu_decidability(request_text: &str, provider_payload: &Value) -> CpuDecidability {
    if literal_constraint(request_text) {
        return decision(
            CpuDecidabilityClass::PotentiallyCpuExecutable,
            "literal_constraint",
        );
    }
    let Some(input) = provider_payload.get("input").and_then(Value::as_array) else {
        return decision(
            CpuDecidabilityClass::UnsupportedByCurrentDsl,
            "provider_input_missing",
        );
    };
    let turn_start = input
        .iter()
        .rposition(is_user_message)
        .map_or(0, |index| index.saturating_add(1));
    let outputs = input[turn_start..]
        .iter()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .collect::<Vec<_>>();
    let Some(latest) = outputs.last() else {
        return decision(
            CpuDecidabilityClass::NotExecutableCurrentEvidence,
            "no_post_user_grounded_observation",
        );
    };
    let Some(output) = latest.get("output") else {
        return decision(
            CpuDecidabilityClass::UnsupportedByCurrentDsl,
            "tool_output_missing",
        );
    };
    let latest_decision = classify_output(output);
    if outputs.len() > 1 {
        let distinct = outputs
            .iter()
            .filter_map(|item| item.get("call_id").and_then(Value::as_str))
            .collect::<std::collections::BTreeSet<_>>();
        if distinct.len() > 1 {
            if latest_decision.class == CpuDecidabilityClass::PotentiallyCpuExecutable {
                return decision(
                    CpuDecidabilityClass::PotentiallyCpuExecutable,
                    "latest_grounded_observation_candidate",
                );
            }
            return decision(
                CpuDecidabilityClass::UnexploredMultiSource,
                "multiple_grounded_tool_observations",
            );
        }
    }
    latest_decision
}

fn classify_output(output: &Value) -> CpuDecidability {
    let text = match output {
        Value::String(value) => value.as_str(),
        Value::Array(parts) if parts.len() == 1 => parts[0]
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        Value::Array(_) => {
            return decision(
                CpuDecidabilityClass::UnexploredMultiSource,
                "multiple_tool_output_parts",
            );
        }
        _ => {
            return decision(
                CpuDecidabilityClass::UnsupportedByCurrentDsl,
                "tool_output_type_unsupported",
            );
        }
    };
    if text.starts_with("Script running with cell ID ")
        || text.starts_with("Process running with session ID ")
    {
        return decision(
            CpuDecidabilityClass::PotentiallyCpuExecutable,
            "unique_continuation_handle",
        );
    }
    if text.trim().is_empty() {
        return decision(
            CpuDecidabilityClass::NotExecutableCurrentEvidence,
            "empty_tool_observation",
        );
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        let mut scalar_leaves = 0_usize;
        let mut collections = 0_usize;
        count_json_candidates(&value, &mut scalar_leaves, &mut collections);
        return match (scalar_leaves, collections) {
            (1, 0) | (0, 1) => decision(
                CpuDecidabilityClass::PotentiallyCpuExecutable,
                "unique_structural_result",
            ),
            (0, 0) => decision(
                CpuDecidabilityClass::NotExecutableCurrentEvidence,
                "json_has_no_projectable_result",
            ),
            _ => decision(
                CpuDecidabilityClass::UnexploredMultiSource,
                "multiple_structural_results",
            ),
        };
    }
    if text.lines().count() == 1
        && (text.contains("exit code") || text.contains("Process exited with code"))
    {
        return decision(
            CpuDecidabilityClass::PotentiallyCpuExecutable,
            "unique_status_result",
        );
    }
    decision(
        CpuDecidabilityClass::UnsupportedByCurrentDsl,
        "unstructured_tool_observation",
    )
}

fn count_json_candidates(value: &Value, scalars: &mut usize, collections: &mut usize) {
    if *scalars > 1 || *collections > 1 {
        return;
    }
    match value {
        Value::Array(_) => *collections = collections.saturating_add(1),
        Value::Object(object) => {
            for value in object.values() {
                count_json_candidates(value, scalars, collections);
            }
        }
        Value::Null => {}
        Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            *scalars = scalars.saturating_add(1);
        }
    }
}

fn literal_constraint(text: &str) -> bool {
    let normalized = text.trim_start().to_ascii_lowercase();
    [
        "reply exactly:",
        "respond exactly:",
        "return exactly:",
        "output exactly:",
        "answer exactly:",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
}

fn is_user_message(item: &Value) -> bool {
    item.get("type").and_then(Value::as_str) == Some("message")
        && item.get("role").and_then(Value::as_str) == Some("user")
}

const fn decision(class: CpuDecidabilityClass, reason: &'static str) -> CpuDecidability {
    CpuDecidability { class, reason }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn no_observation_is_not_executable_from_current_evidence() {
        let payload = json!({"input":[{"type":"message","role":"user","content":[]}]});
        let result = classify_cpu_decidability("design a new algorithm", &payload);
        assert_eq!(
            result.class,
            CpuDecidabilityClass::NotExecutableCurrentEvidence
        );
    }

    #[test]
    fn unique_json_result_is_a_miner_opportunity() {
        let payload = json!({"input":[
            {"type":"message","role":"user","content":[]},
            {"type":"function_call_output","call_id":"one","output":"{\"count\":7}"}
        ]});
        let result = classify_cpu_decidability("", &payload);
        assert_eq!(result.class, CpuDecidabilityClass::PotentiallyCpuExecutable);
    }

    #[test]
    fn a_unique_latest_observation_is_a_bounded_multi_source_candidate() {
        let payload = json!({"input":[
            {"type":"message","role":"user","content":[]},
            {"type":"function_call_output","call_id":"one","output":"1"},
            {"type":"function_call_output","call_id":"two","output":"{\"count\":2}"}
        ]});
        let result = classify_cpu_decidability("", &payload);
        assert_eq!(result.class, CpuDecidabilityClass::PotentiallyCpuExecutable);
        assert_eq!(result.reason, "latest_grounded_observation_candidate");
    }

    #[test]
    fn unresolved_latest_observation_remains_unexplored_multi_source() {
        let payload = json!({"input":[
            {"type":"message","role":"user","content":[]},
            {"type":"function_call_output","call_id":"one","output":"1"},
            {"type":"function_call_output","call_id":"two","output":"unstructured result"}
        ]});
        let result = classify_cpu_decidability("", &payload);
        assert_eq!(result.class, CpuDecidabilityClass::UnexploredMultiSource);
        assert_eq!(result.reason, "multiple_grounded_tool_observations");
    }

    #[test]
    fn process_session_handle_is_a_miner_opportunity() {
        let payload = json!({"input":[
            {"type":"message","role":"user","content":[]},
            {
                "type":"function_call_output",
                "call_id":"one",
                "output":"Process running with session ID 4242"
            }
        ]});
        let result = classify_cpu_decidability("", &payload);
        assert_eq!(result.class, CpuDecidabilityClass::PotentiallyCpuExecutable);
        assert_eq!(result.reason, "unique_continuation_handle");
    }

    #[test]
    fn classifier_uses_only_pre_action_payload() {
        let payload = json!({"input":[
            {"type":"message","role":"user","content":[]},
            {"type":"function_call_output","call_id":"one","output":"{\"left\":1,\"right\":2}"}
        ]});
        let result = classify_cpu_decidability("", &payload);
        assert_eq!(result.class, CpuDecidabilityClass::UnexploredMultiSource);
        assert_eq!(result.reason, "multiple_structural_results");
    }
}
