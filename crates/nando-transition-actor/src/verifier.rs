use std::fmt;

use serde_json::{Map, Value};

use crate::program::{TransitionOperation, TransitionProgram};
use crate::runtime::{CanonicalRecord, CanonicalState, add_numbers, get_slot};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationError(pub &'static str);

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for VerificationError {}

pub fn verify_transition(
    program: &TransitionProgram,
    before: &CanonicalState,
    action: &Map<String, Value>,
    after: &CanonicalState,
) -> Result<(), VerificationError> {
    let expected = match &program.operation {
        TransitionOperation::SetField {
            match_field,
            target_slot,
            target_field,
            value_slot,
        } => {
            let index = select_unique(before, action, match_field, target_slot)?;
            let value =
                get_slot(action, value_slot).ok_or(VerificationError("value_slot_missing"))?;
            let mut expected = before.clone();
            expected[index].insert(target_field.clone(), value.clone());
            expected
        }
        TransitionOperation::IncrementField {
            match_field,
            target_slot,
            target_field,
            amount_slot,
        } => {
            let index = select_unique(before, action, match_field, target_slot)?;
            let current = before[index]
                .get(target_field)
                .and_then(Value::as_number)
                .ok_or(VerificationError("target_not_numeric"))?;
            let amount = get_slot(action, amount_slot)
                .and_then(Value::as_number)
                .ok_or(VerificationError("amount_not_numeric"))?;
            let sum = add_numbers(current, amount).ok_or(VerificationError("numeric_overflow"))?;
            let mut expected = before.clone();
            expected[index].insert(target_field.clone(), Value::Number(sum));
            expected
        }
        TransitionOperation::AppendRecord { record_bindings } => {
            let mut record = CanonicalRecord::new();
            for (role, slot) in record_bindings {
                let value =
                    get_slot(action, slot).ok_or(VerificationError("record_slot_missing"))?;
                record.insert(role.clone(), value.clone());
            }
            let mut expected = before.clone();
            expected.push(record);
            expected
        }
        TransitionOperation::DeleteRecord {
            match_field,
            target_slot,
        } => {
            let index = select_unique(before, action, match_field, target_slot)?;
            let mut expected = before.clone();
            expected.remove(index);
            expected
        }
    };
    if &expected == after {
        Ok(())
    } else {
        Err(VerificationError("postcondition_state_mismatch"))
    }
}

fn select_unique(
    before: &CanonicalState,
    action: &Map<String, Value>,
    match_field: &str,
    target_slot: &str,
) -> Result<usize, VerificationError> {
    let target = get_slot(action, target_slot).ok_or(VerificationError("target_slot_missing"))?;
    let mut matches = before
        .iter()
        .enumerate()
        .filter_map(|(index, row)| (row.get(match_field) == Some(target)).then_some(index));
    let index = matches.next().ok_or(VerificationError("target_missing"))?;
    if matches.next().is_some() {
        return Err(VerificationError("target_ambiguous"));
    }
    Ok(index)
}
