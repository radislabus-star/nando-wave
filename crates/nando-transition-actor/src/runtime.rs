use std::collections::BTreeMap;

use serde_json::{Map, Number, Value};

use crate::adapter::SurfaceAdapter;
use crate::program::{TransitionOperation, TransitionProgram, ValueKind};
use crate::verifier::verify_transition;

pub type CanonicalRecord = Map<String, Value>;
pub type CanonicalState = Vec<CanonicalRecord>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionStatus {
    Executed,
    Abstain,
    VerifyFailed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub reason: String,
    pub after_records: Option<CanonicalState>,
    pub concrete_after: Option<Value>,
    pub proof: BTreeMap<String, Value>,
}

impl ExecutionResult {
    #[must_use]
    pub fn accepted(&self) -> bool {
        self.status == ExecutionStatus::Executed
    }

    fn rejected(status: ExecutionStatus, reason: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason.into(),
            after_records: None,
            concrete_after: None,
            proof: BTreeMap::new(),
        }
    }

    fn executed(after_records: CanonicalState, proof: BTreeMap<String, Value>) -> Self {
        Self {
            status: ExecutionStatus::Executed,
            reason: "executed".to_owned(),
            after_records: Some(after_records),
            concrete_after: None,
            proof,
        }
    }
}

pub fn execute_canonical(
    program: &TransitionProgram,
    before: &CanonicalState,
    action: &Map<String, Value>,
) -> ExecutionResult {
    if let Err(reason) = program.validate() {
        return ExecutionResult::rejected(
            ExecutionStatus::Abstain,
            format!("invalid_program:{reason}"),
        );
    }
    if action.get("kind").and_then(Value::as_str) != Some(program.action_kind.as_str()) {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "action_kind_mismatch");
    }
    if let Some(reason) = slot_type_error(program, action) {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, reason);
    }

    let mut result = match &program.operation {
        TransitionOperation::SetField {
            match_field,
            target_slot,
            target_field,
            value_slot,
        } => execute_set(
            before,
            action,
            match_field,
            target_slot,
            target_field,
            value_slot,
        ),
        TransitionOperation::IncrementField {
            match_field,
            target_slot,
            target_field,
            amount_slot,
        } => execute_increment(
            before,
            action,
            match_field,
            target_slot,
            target_field,
            amount_slot,
        ),
        TransitionOperation::AppendRecord { record_bindings } => {
            execute_append(before, action, record_bindings)
        }
        TransitionOperation::DeleteRecord {
            match_field,
            target_slot,
        } => execute_delete(before, action, match_field, target_slot),
    };

    if !result.accepted() {
        return result;
    }
    let Some(after) = result.after_records.as_ref() else {
        return ExecutionResult::rejected(ExecutionStatus::VerifyFailed, "missing_actor_output");
    };
    if let Err(error) = verify_transition(program, before, action, after) {
        result.status = ExecutionStatus::VerifyFailed;
        result.reason = format!("verification:{error}");
        result.after_records = None;
        return result;
    }
    result.proof.insert(
        "postcondition".to_owned(),
        Value::String("verified".to_owned()),
    );
    result
}

pub fn execute_surface(
    program: &TransitionProgram,
    adapter: &SurfaceAdapter,
    before: &Value,
    action: &Value,
) -> ExecutionResult {
    let adapted = match adapter.adapt_state(before) {
        Ok(adapted) => adapted,
        Err(error) => {
            return ExecutionResult::rejected(ExecutionStatus::Abstain, format!("adapter:{error}"));
        }
    };
    let canonical_action = match adapter.adapt_action(action) {
        Ok(action) => action,
        Err(error) => {
            return ExecutionResult::rejected(ExecutionStatus::Abstain, format!("adapter:{error}"));
        }
    };
    let mut result = execute_canonical(program, &adapted.records, &canonical_action);
    if !result.accepted() {
        return result;
    }
    let Some(after) = result.after_records.as_ref() else {
        return ExecutionResult::rejected(ExecutionStatus::VerifyFailed, "missing_actor_output");
    };
    match adapter.project(after, &adapted) {
        Ok(concrete_after) => result.concrete_after = Some(concrete_after),
        Err(error) => {
            result.status = ExecutionStatus::VerifyFailed;
            result.reason = format!("projection:{error}");
            result.concrete_after = None;
        }
    }
    result
}

fn execute_set(
    before: &CanonicalState,
    action: &Map<String, Value>,
    match_field: &str,
    target_slot: &str,
    target_field: &str,
    value_slot: &str,
) -> ExecutionResult {
    let index = match select_unique(before, action, match_field, target_slot) {
        Ok(index) => index,
        Err(reason) => return ExecutionResult::rejected(ExecutionStatus::Abstain, reason),
    };
    let Some(old_value) = before[index].get(target_field) else {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "target_field_missing");
    };
    let Some(value) = get_slot(action, value_slot) else {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "value_slot_missing");
    };
    if value_kind(old_value) != value_kind(value) {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "target_value_type_mismatch");
    }
    if old_value == value {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "no_effect");
    }
    let mut after = before.clone();
    after[index].insert(target_field.to_owned(), value.clone());
    let proof = BTreeMap::from([
        ("selected_index".to_owned(), Value::from(index as u64)),
        (
            "changed_role".to_owned(),
            Value::String(target_field.to_owned()),
        ),
        ("before_value".to_owned(), old_value.clone()),
        ("after_value".to_owned(), value.clone()),
    ]);
    ExecutionResult::executed(after, proof)
}

fn execute_increment(
    before: &CanonicalState,
    action: &Map<String, Value>,
    match_field: &str,
    target_slot: &str,
    target_field: &str,
    amount_slot: &str,
) -> ExecutionResult {
    let index = match select_unique(before, action, match_field, target_slot) {
        Ok(index) => index,
        Err(reason) => return ExecutionResult::rejected(ExecutionStatus::Abstain, reason),
    };
    let Some(old_number) = before[index].get(target_field).and_then(Value::as_number) else {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "target_not_numeric");
    };
    let Some(amount) = get_slot(action, amount_slot).and_then(Value::as_number) else {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "amount_not_numeric");
    };
    if amount.as_f64() == Some(0.0) {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "no_effect");
    }
    let Some(sum) = add_numbers(old_number, amount) else {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "numeric_overflow");
    };
    let mut after = before.clone();
    after[index].insert(target_field.to_owned(), Value::Number(sum.clone()));
    let proof = BTreeMap::from([
        ("selected_index".to_owned(), Value::from(index as u64)),
        (
            "changed_role".to_owned(),
            Value::String(target_field.to_owned()),
        ),
        ("before_value".to_owned(), Value::Number(old_number.clone())),
        ("after_value".to_owned(), Value::Number(sum)),
    ]);
    ExecutionResult::executed(after, proof)
}

fn execute_append(
    before: &CanonicalState,
    action: &Map<String, Value>,
    record_bindings: &std::collections::BTreeMap<String, String>,
) -> ExecutionResult {
    let mut record = CanonicalRecord::new();
    for (role, slot) in record_bindings {
        let Some(value) = get_slot(action, slot) else {
            return ExecutionResult::rejected(ExecutionStatus::Abstain, "record_slot_missing");
        };
        record.insert(role.clone(), value.clone());
    }
    let Some(record_id) = record.get("id") else {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "new_record_id_missing");
    };
    if before.iter().any(|row| row.get("id") == Some(record_id)) {
        return ExecutionResult::rejected(ExecutionStatus::Abstain, "duplicate_id");
    }
    let mut after = before.clone();
    after.push(record.clone());
    let proof = BTreeMap::from([
        ("appended_id".to_owned(), record_id.clone()),
        (
            "appended_role_count".to_owned(),
            Value::from(record_bindings.len() as u64),
        ),
    ]);
    ExecutionResult::executed(after, proof)
}

fn execute_delete(
    before: &CanonicalState,
    action: &Map<String, Value>,
    match_field: &str,
    target_slot: &str,
) -> ExecutionResult {
    let index = match select_unique(before, action, match_field, target_slot) {
        Ok(index) => index,
        Err(reason) => return ExecutionResult::rejected(ExecutionStatus::Abstain, reason),
    };
    let mut after = before.clone();
    let removed = after.remove(index);
    let proof = BTreeMap::from([
        ("selected_index".to_owned(), Value::from(index as u64)),
        (
            "removed_id".to_owned(),
            removed.get("id").cloned().unwrap_or(Value::Null),
        ),
    ]);
    ExecutionResult::executed(after, proof)
}

fn slot_type_error(program: &TransitionProgram, action: &Map<String, Value>) -> Option<String> {
    program.slot_types.iter().find_map(|(slot, expected)| {
        let Some(value) = get_slot(action, slot) else {
            return Some(format!("missing_typed_slot:{slot}"));
        };
        let actual = value_kind(value);
        (actual != *expected).then(|| format!("slot_type:{slot}:{actual:?}!={expected:?}"))
    })
}

fn select_unique(
    before: &CanonicalState,
    action: &Map<String, Value>,
    match_field: &str,
    target_slot: &str,
) -> Result<usize, &'static str> {
    let Some(target) = get_slot(action, target_slot) else {
        return Err("target_slot_missing");
    };
    let mut matches = before
        .iter()
        .enumerate()
        .filter_map(|(index, row)| (row.get(match_field) == Some(target)).then_some(index));
    let Some(index) = matches.next() else {
        return Err("target_missing");
    };
    if matches.next().is_some() {
        return Err("target_ambiguous");
    }
    Ok(index)
}

pub(crate) fn get_slot<'a>(action: &'a Map<String, Value>, slot: &str) -> Option<&'a Value> {
    let mut parts = slot.split('.');
    let first = parts.next()?;
    let mut current = action.get(first)?;
    for part in parts {
        current = current.as_object()?.get(part)?;
    }
    Some(current)
}

pub(crate) fn value_kind(value: &Value) -> ValueKind {
    match value {
        Value::Null => ValueKind::Null,
        Value::Bool(_) => ValueKind::Bool,
        Value::Number(_) => ValueKind::Number,
        Value::String(_) => ValueKind::String,
        Value::Array(_) => ValueKind::Array,
        Value::Object(_) => ValueKind::Object,
    }
}

pub(crate) fn add_numbers(left: &Number, right: &Number) -> Option<Number> {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return left.checked_add(right).map(Number::from);
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return left.checked_add(right).map(Number::from);
    }
    let sum = left.as_f64()? + right.as_f64()?;
    Number::from_f64(sum)
}
