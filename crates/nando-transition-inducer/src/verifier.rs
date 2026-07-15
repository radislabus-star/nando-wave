use std::collections::BTreeMap;

use nando_transition_actor::{CanonicalState, SurfaceAdapter};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::guard::slot;
use crate::hypothesis::OperatorSkeleton;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerifierProgram {
    pub schema: String,
    pub skeleton: OperatorSkeleton,
    pub match_field: String,
    pub target_slot: String,
    pub target_field: String,
    pub operand_slot: String,
    pub require_record_count_stable: bool,
    pub require_identity_set_stable: bool,
    pub require_unchanged_complement: bool,
    pub require_surface_frame_preserved: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierFailure(pub String);

impl VerifierProgram {
    pub(crate) fn grammar(skeleton: OperatorSkeleton) -> Vec<Self> {
        (0u8..16)
            .map(|mask| Self {
                schema: "nando.transition-verifier.v1".to_owned(),
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
                require_record_count_stable: mask & 0b001 != 0,
                require_identity_set_stable: mask & 0b0010 != 0,
                require_unchanged_complement: mask & 0b0100 != 0,
                require_surface_frame_preserved: mask & 0b1000 != 0,
            })
            .collect()
    }

    pub fn verify(
        &self,
        adapter: &SurfaceAdapter,
        before: &Value,
        action: &Value,
        after: &Value,
    ) -> Result<(), VerifierFailure> {
        if self.schema != "nando.transition-verifier.v1" {
            return Err(VerifierFailure("unsupported_verifier_schema".to_owned()));
        }
        let before_adapted = adapter
            .adapt_state(before)
            .map_err(|error| VerifierFailure(format!("before_adapter:{error}")))?;
        let after_adapted = adapter
            .adapt_state(after)
            .map_err(|error| VerifierFailure(format!("after_adapter:{error}")))?;
        let action = adapter
            .adapt_action(action)
            .map_err(|error| VerifierFailure(format!("action_adapter:{error}")))?;
        let before_index = index_records(&before_adapted.records, &self.match_field)?;
        let after_index = index_records(&after_adapted.records, &self.match_field)?;
        let target = slot(&action, &self.target_slot)
            .ok_or_else(|| VerifierFailure("target_slot_missing".to_owned()))?;
        let target_key = serde_json::to_string(target)
            .map_err(|_| VerifierFailure("target_not_serializable".to_owned()))?;
        let count_valid = match self.skeleton {
            OperatorSkeleton::SetField | OperatorSkeleton::IncrementField => {
                before_adapted.records.len() == after_adapted.records.len()
            }
            OperatorSkeleton::AppendRecord => {
                before_adapted.records.len() + 1 == after_adapted.records.len()
            }
            OperatorSkeleton::DeleteRecord => {
                before_adapted.records.len() == after_adapted.records.len() + 1
            }
        };
        if self.require_record_count_stable && !count_valid {
            return Err(VerifierFailure("record_count_relation_mismatch".to_owned()));
        }
        let identity_valid = match self.skeleton {
            OperatorSkeleton::SetField | OperatorSkeleton::IncrementField => {
                before_index.keys().eq(after_index.keys())
            }
            OperatorSkeleton::AppendRecord => {
                !before_index.contains_key(&target_key)
                    && after_index.contains_key(&target_key)
                    && before_index.keys().all(|id| after_index.contains_key(id))
                    && after_index.len() == before_index.len() + 1
            }
            OperatorSkeleton::DeleteRecord => {
                before_index.contains_key(&target_key)
                    && !after_index.contains_key(&target_key)
                    && after_index.keys().all(|id| before_index.contains_key(id))
                    && before_index.len() == after_index.len() + 1
            }
        };
        if self.require_identity_set_stable && !identity_valid {
            return Err(VerifierFailure(
                "record_identity_relation_mismatch".to_owned(),
            ));
        }
        let effect_valid = match self.skeleton {
            OperatorSkeleton::SetField | OperatorSkeleton::IncrementField => {
                let operand = slot(&action, &self.operand_slot);
                let before_value = record_value(
                    &before_adapted.records,
                    &before_index,
                    &target_key,
                    &self.target_field,
                );
                let after_value = record_value(
                    &after_adapted.records,
                    &after_index,
                    &target_key,
                    &self.target_field,
                );
                match self.skeleton {
                    OperatorSkeleton::SetField => operand.is_some_and(|operand| {
                        after_value == Some(operand) && before_value != after_value
                    }),
                    OperatorSkeleton::IncrementField => {
                        before_value.zip(after_value).zip(operand).is_some_and(
                            |((before, after), amount)| numeric_delta_equals(before, after, amount),
                        )
                    }
                    _ => false,
                }
            }
            OperatorSkeleton::AppendRecord => {
                let operand = slot(&action, &self.operand_slot);
                let after_value = record_value(
                    &after_adapted.records,
                    &after_index,
                    &target_key,
                    &self.target_field,
                );
                !before_index.contains_key(&target_key)
                    && after_value
                        .zip(operand)
                        .is_some_and(|(left, right)| left == right)
            }
            OperatorSkeleton::DeleteRecord => {
                before_index.contains_key(&target_key) && !after_index.contains_key(&target_key)
            }
        };
        if !effect_valid {
            return Err(VerifierFailure("postcondition_effect_mismatch".to_owned()));
        }
        if self.require_unchanged_complement {
            verify_complement(
                self.skeleton,
                &before_adapted.records,
                &after_adapted.records,
                &before_index,
                &after_index,
                &target_key,
                &self.target_field,
            )?;
        }
        if self.require_surface_frame_preserved {
            let projected = adapter
                .project(&after_adapted.records, &before_adapted)
                .map_err(|error| VerifierFailure(format!("projection:{error}")))?;
            if &projected != after {
                return Err(VerifierFailure("surface_frame_changed".to_owned()));
            }
        }
        Ok(())
    }
}

fn record_value<'a>(
    records: &'a CanonicalState,
    index: &BTreeMap<String, usize>,
    target_key: &str,
    field: &str,
) -> Option<&'a Value> {
    index
        .get(target_key)
        .and_then(|position| records.get(*position))
        .and_then(|record| record.get(field))
}

fn verify_complement(
    skeleton: OperatorSkeleton,
    before: &CanonicalState,
    after: &CanonicalState,
    before_index: &BTreeMap<String, usize>,
    after_index: &BTreeMap<String, usize>,
    target_key: &str,
    target_field: &str,
) -> Result<(), VerifierFailure> {
    match skeleton {
        OperatorSkeleton::SetField | OperatorSkeleton::IncrementField => {
            for (id, before_position) in before_index {
                let after_position = after_index
                    .get(id)
                    .ok_or_else(|| VerifierFailure("record_missing_after".to_owned()))?;
                let mut left = before[*before_position].clone();
                let mut right = after[*after_position].clone();
                if id == target_key {
                    left.remove(target_field);
                    right.remove(target_field);
                }
                if left != right {
                    return Err(VerifierFailure("canonical_complement_changed".to_owned()));
                }
            }
        }
        OperatorSkeleton::AppendRecord => {
            for (id, before_position) in before_index {
                let after_position = after_index
                    .get(id)
                    .ok_or_else(|| VerifierFailure("existing_record_missing_after".to_owned()))?;
                if before[*before_position] != after[*after_position] {
                    return Err(VerifierFailure("append_complement_changed".to_owned()));
                }
            }
        }
        OperatorSkeleton::DeleteRecord => {
            for (id, after_position) in after_index {
                let before_position = before_index
                    .get(id)
                    .ok_or_else(|| VerifierFailure("surviving_record_missing_before".to_owned()))?;
                if before[*before_position] != after[*after_position] {
                    return Err(VerifierFailure("delete_complement_changed".to_owned()));
                }
            }
        }
    }
    Ok(())
}

fn index_records(
    records: &CanonicalState,
    match_field: &str,
) -> Result<BTreeMap<String, usize>, VerifierFailure> {
    let mut out = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        let value = record
            .get(match_field)
            .ok_or_else(|| VerifierFailure("record_identity_missing".to_owned()))?;
        let key = serde_json::to_string(value)
            .map_err(|_| VerifierFailure("record_identity_not_serializable".to_owned()))?;
        if out.insert(key, index).is_some() {
            return Err(VerifierFailure("record_identity_ambiguous".to_owned()));
        }
    }
    Ok(out)
}

fn numeric_delta_equals(before: &Value, after: &Value, amount: &Value) -> bool {
    if let (Some(before), Some(after), Some(amount)) =
        (before.as_i64(), after.as_i64(), amount.as_i64())
    {
        return before.checked_add(amount) == Some(after);
    }
    if let (Some(before), Some(after), Some(amount)) =
        (before.as_u64(), after.as_u64(), amount.as_u64())
    {
        return before.checked_add(amount) == Some(after);
    }
    let (Some(before), Some(after), Some(amount)) =
        (before.as_f64(), after.as_f64(), amount.as_f64())
    else {
        return false;
    };
    (before + amount - after).abs() <= f64::EPSILON
}
