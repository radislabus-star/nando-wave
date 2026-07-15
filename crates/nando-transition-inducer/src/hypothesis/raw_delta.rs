use std::collections::BTreeSet;

use serde_json::Value;

use super::{PreparedIdentity, PreparedTraceRelations, RelationEvidenceIndex, RoleHypothesis};
use crate::trace::{RawRecord, value_at};

impl RelationEvidenceIndex {
    pub(crate) fn raw_delta_atoms(
        &self,
        hypothesis: &RoleHypothesis,
        limit: Option<usize>,
    ) -> Vec<String> {
        let traces = self
            .traces
            .iter()
            .take(limit.unwrap_or(usize::MAX))
            .collect::<Vec<_>>();
        raw_delta_atoms_from_traces(hypothesis, &traces)
    }
}

fn raw_delta_atoms_from_traces(
    hypothesis: &RoleHypothesis,
    traces: &[&PreparedTraceRelations],
) -> Vec<String> {
    let rows = traces
        .iter()
        .map(|trace| raw_delta_states(hypothesis, trace))
        .collect::<Vec<_>>();
    let width = rows.first().map_or(0, Vec::len);
    let mut atoms = Vec::with_capacity(width + 1);
    atoms.push("raw_delta_basis:v1".to_owned());
    for index in 0..width {
        let first = &rows[0][index];
        let state = if rows.iter().all(|row| row[index] == *first) {
            first.as_str()
        } else {
            "mixed"
        };
        atoms.push(format!("raw_delta_slot:{index}:{state}"));
    }
    atoms
}

fn raw_delta_states(hypothesis: &RoleHypothesis, trace: &PreparedTraceRelations) -> Vec<String> {
    let target_value = value_at(&trace.action, &hypothesis.target_action_path);
    let target_key = target_value.and_then(|value| serde_json::to_string(value).ok());
    let operand = value_at(&trace.action, &hypothesis.operand_action_path);
    let identity = trace.identities.get(&hypothesis.record_id_source);
    let before_index = target_key
        .as_deref()
        .zip(identity.and_then(|item| item.before_index.as_ref()))
        .and_then(|(target, index)| index.get(target).copied());
    let after_index = target_key
        .as_deref()
        .zip(identity.and_then(|item| item.after_index.as_ref()))
        .and_then(|(target, index)| index.get(target).copied());
    let before_value = before_index
        .and_then(|index| trace.before.get(index))
        .and_then(|record| record.fields.get(&hypothesis.target_field));
    let after_value = after_index
        .and_then(|index| trace.after.get(index))
        .and_then(|record| record.fields.get(&hypothesis.target_field));
    let count_delta = trace.after.len() as isize - trace.before.len() as isize;
    let selected_changes =
        changed_field_count(&trace.before, &trace.after, before_index, after_index);
    let other_changes =
        changed_other_record_count(identity, &trace.before, &trace.after, target_key.as_deref());
    vec![
        signed_bucket(count_delta).to_owned(),
        presence_bucket(before_index.is_some()).to_owned(),
        presence_bucket(after_index.is_some()).to_owned(),
        presence_transition(before_value, after_value).to_owned(),
        equality_bucket(before_value, after_value).to_owned(),
        equality_bucket(after_value, operand).to_owned(),
        equality_bucket(before_value, operand).to_owned(),
        numeric_delta_operand_bucket(before_value, after_value, operand).to_owned(),
        numeric_delta_bucket(before_value, after_value).to_owned(),
        count_bucket(selected_changes).to_owned(),
        count_bucket(other_changes).to_owned(),
        bool_bucket(trace.frame_preserved).to_owned(),
        identity_delta_bucket(identity).to_owned(),
        value_kind_bucket(operand).to_owned(),
        value_kind_bucket(target_value).to_owned(),
    ]
}

fn changed_field_count(
    before: &[RawRecord],
    after: &[RawRecord],
    before_index: Option<usize>,
    after_index: Option<usize>,
) -> usize {
    let Some((before, after)) = before_index
        .and_then(|index| before.get(index))
        .zip(after_index.and_then(|index| after.get(index)))
    else {
        return usize::from(before_index.is_some() != after_index.is_some());
    };
    before
        .fields
        .keys()
        .chain(after.fields.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|field| before.fields.get(*field) != after.fields.get(*field))
        .count()
}

fn changed_other_record_count(
    identity: Option<&PreparedIdentity>,
    before: &[RawRecord],
    after: &[RawRecord],
    target: Option<&str>,
) -> usize {
    let Some((before_index, after_index)) =
        identity.and_then(|item| item.before_index.as_ref().zip(item.after_index.as_ref()))
    else {
        return 0;
    };
    before_index
        .keys()
        .filter(|id| target != Some(id.as_str()))
        .filter(|id| {
            before_index
                .get(*id)
                .and_then(|index| before.get(*index))
                .zip(after_index.get(*id).and_then(|index| after.get(*index)))
                .is_none_or(|(left, right)| left != right)
        })
        .count()
}

fn identity_delta_bucket(identity: Option<&PreparedIdentity>) -> &'static str {
    let Some((before, after)) =
        identity.and_then(|item| item.before_index.as_ref().zip(item.after_index.as_ref()))
    else {
        return "unknown";
    };
    let removed = before
        .keys()
        .filter(|key| !after.contains_key(*key))
        .count();
    let added = after
        .keys()
        .filter(|key| !before.contains_key(*key))
        .count();
    match (removed, added) {
        (0, 0) => "same",
        (0, 1) => "add_one",
        (1, 0) => "remove_one",
        _ => "changed_many",
    }
}

fn numeric_delta_operand_bucket(
    before: Option<&Value>,
    after: Option<&Value>,
    operand: Option<&Value>,
) -> &'static str {
    let Some((before, after, operand)) = numeric_value(before)
        .zip(numeric_value(after))
        .zip(numeric_value(operand))
        .map(|((before, after), operand)| (before, after, operand))
    else {
        return "na";
    };
    if ((after - before) - operand).abs() <= f64::EPSILON {
        "eq"
    } else {
        "neq"
    }
}

fn numeric_delta_bucket(before: Option<&Value>, after: Option<&Value>) -> &'static str {
    let Some((before, after)) = numeric_value(before).zip(numeric_value(after)) else {
        return "na";
    };
    if (after - before).abs() <= f64::EPSILON {
        "zero"
    } else if after > before {
        "positive"
    } else {
        "negative"
    }
}

fn numeric_value(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

fn equality_bucket(left: Option<&Value>, right: Option<&Value>) -> &'static str {
    match (left, right) {
        (Some(left), Some(right)) if left == right => "eq",
        (Some(_), Some(_)) => "neq",
        _ => "na",
    }
}

fn presence_transition(before: Option<&Value>, after: Option<&Value>) -> &'static str {
    match (before.is_some(), after.is_some()) {
        (false, false) => "none_none",
        (false, true) => "none_some",
        (true, false) => "some_none",
        (true, true) => "some_some",
    }
}

const fn presence_bucket(present: bool) -> &'static str {
    if present { "some" } else { "none" }
}

const fn bool_bucket(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

const fn signed_bucket(value: isize) -> &'static str {
    match value {
        -1 => "minus_one",
        0 => "zero",
        1 => "plus_one",
        _ => "other",
    }
}

const fn count_bucket(value: usize) -> &'static str {
    match value {
        0 => "zero",
        1 => "one",
        _ => "many",
    }
}

fn value_kind_bucket(value: Option<&Value>) -> &'static str {
    match value {
        None => "missing",
        Some(Value::Null) => "null",
        Some(Value::Bool(_)) => "bool",
        Some(Value::Number(_)) => "number",
        Some(Value::String(_)) => "string",
        Some(Value::Array(_)) => "array",
        Some(Value::Object(_)) => "object",
    }
}
