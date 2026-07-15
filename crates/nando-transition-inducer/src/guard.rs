use nando_transition_actor::SurfaceAdapter;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::hypothesis::OperatorSkeleton;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuardProgram {
    pub schema: String,
    pub skeleton: OperatorSkeleton,
    pub match_field: String,
    pub target_slot: String,
    pub target_field: String,
    pub operand_slot: String,
    pub require_unique_target: bool,
    pub require_operand_type: bool,
    pub require_effect: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardFailure(pub String);

impl GuardProgram {
    pub(crate) fn grammar(skeleton: OperatorSkeleton) -> Vec<Self> {
        (0u8..8)
            .map(|mask| Self {
                schema: "nando.transition-guard.v1".to_owned(),
                skeleton,
                match_field: "id".to_owned(),
                target_slot: match skeleton {
                    OperatorSkeleton::AppendRecord => "record.id",
                    _ => "target",
                }
                .to_owned(),
                target_field: "value".to_owned(),
                operand_slot: match skeleton {
                    OperatorSkeleton::SetField => "value",
                    OperatorSkeleton::IncrementField => "amount",
                    OperatorSkeleton::AppendRecord => "record.value",
                    OperatorSkeleton::DeleteRecord => "",
                }
                .to_owned(),
                require_unique_target: mask & 0b001 != 0,
                require_operand_type: mask & 0b010 != 0,
                require_effect: mask & 0b100 != 0,
            })
            .collect()
    }

    pub fn check(
        &self,
        adapter: &SurfaceAdapter,
        before: &Value,
        action: &Value,
    ) -> Result<(), GuardFailure> {
        if self.schema != "nando.transition-guard.v1" {
            return Err(GuardFailure("unsupported_guard_schema".to_owned()));
        }
        let state = adapter
            .adapt_state(before)
            .map_err(|error| GuardFailure(format!("adapter:{error}")))?;
        let action = adapter
            .adapt_action(action)
            .map_err(|error| GuardFailure(format!("adapter:{error}")))?;
        let target = slot(&action, &self.target_slot)
            .ok_or_else(|| GuardFailure("target_slot_missing".to_owned()))?;
        let matching = state
            .records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                (record.get(&self.match_field) == Some(target)).then_some(index)
            })
            .collect::<Vec<_>>();
        if self.skeleton == OperatorSkeleton::AppendRecord {
            if !matching.is_empty() {
                return Err(GuardFailure("append_id_already_exists".to_owned()));
            }
            let operand = slot(&action, &self.operand_slot)
                .ok_or_else(|| GuardFailure("operand_slot_missing".to_owned()))?;
            if self.require_operand_type && !operand.is_string() {
                return Err(GuardFailure("append_value_type_mismatch".to_owned()));
            }
            return Ok(());
        }
        if matching.is_empty() {
            return Err(GuardFailure("target_missing".to_owned()));
        }
        if self.require_unique_target && matching.len() != 1 {
            return Err(GuardFailure("target_ambiguous".to_owned()));
        }
        if self.skeleton == OperatorSkeleton::DeleteRecord {
            return Ok(());
        }
        let operand = slot(&action, &self.operand_slot)
            .ok_or_else(|| GuardFailure("operand_slot_missing".to_owned()))?;
        let current = state.records[matching[0]]
            .get(&self.target_field)
            .ok_or_else(|| GuardFailure("target_field_missing".to_owned()))?;
        match self.skeleton {
            OperatorSkeleton::SetField => {
                if self.require_operand_type && value_kind(current) != value_kind(operand) {
                    return Err(GuardFailure("target_value_type_mismatch".to_owned()));
                }
                if self.require_effect && current == operand {
                    return Err(GuardFailure("no_effect".to_owned()));
                }
            }
            OperatorSkeleton::IncrementField => {
                if self.require_operand_type && (!current.is_number() || !operand.is_number()) {
                    return Err(GuardFailure("numeric_operand_required".to_owned()));
                }
                if self.require_effect && operand.as_f64() == Some(0.0) {
                    return Err(GuardFailure("no_effect".to_owned()));
                }
            }
            OperatorSkeleton::AppendRecord | OperatorSkeleton::DeleteRecord => unreachable!(),
        }
        Ok(())
    }
}

pub(crate) fn slot<'a>(action: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    let mut segments = name.split('.');
    let first = segments.next()?;
    let mut cursor = action.get(first)?;
    for segment in segments {
        cursor = cursor.as_object()?.get(segment)?;
    }
    Some(cursor)
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
