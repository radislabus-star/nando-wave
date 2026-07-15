use std::collections::{BTreeMap, BTreeSet};

use nando_transition_actor::{
    ActionRule, ExecutionStatus, SurfaceAdapter, TransitionOperation, TransitionProgram, ValueKind,
    execute_surface,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::trace::{
    ScalarPath, SurfaceShape, TransitionTrace, index_by_id, records_for, scalar_paths, value_at,
};

mod raw_delta;

pub(crate) type ActionTraceGroups = BTreeMap<String, Vec<TransitionTrace>>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorSkeleton {
    SetField,
    IncrementField,
    AppendRecord,
    DeleteRecord,
}

impl OperatorSkeleton {
    #[must_use]
    pub const fn canonical_kind(self) -> &'static str {
        match self {
            Self::SetField => "set_field",
            Self::IncrementField => "increment_field",
            Self::AppendRecord => "append_record",
            Self::DeleteRecord => "delete_record",
        }
    }

    const fn operand_slot(self) -> Option<&'static str> {
        match self {
            Self::SetField => Some("value"),
            Self::IncrementField => Some("amount"),
            Self::AppendRecord => Some("record.value"),
            Self::DeleteRecord => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct RoleHypothesis {
    pub shape: SurfaceShape,
    pub action_kind_path: Vec<String>,
    pub concrete_action_kind: String,
    pub record_id_source: String,
    pub target_field: String,
    pub target_action_path: Vec<String>,
    pub operand_action_path: Vec<String>,
    pub skeleton: OperatorSkeleton,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct TraceRelations {
    root_valid: bool,
    record_count_stable: bool,
    identities_stable: bool,
    target_unique: bool,
    target_field_changed: bool,
    only_target_changed: bool,
    frame_preserved: bool,
    set_effect: bool,
    increment_effect: bool,
    append_effect: bool,
    delete_effect: bool,
    operand_type_matches: bool,
    no_op: bool,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub(crate) struct RelationSignature {
    skeleton: OperatorSkeleton,
    bits: u16,
}

#[derive(Clone, Debug)]
struct PreparedIdentity {
    before_index: Option<BTreeMap<String, usize>>,
    after_index: Option<BTreeMap<String, usize>>,
    identities_stable: bool,
    changed: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
struct PreparedTraceRelations {
    before: Vec<crate::trace::RawRecord>,
    after: Vec<crate::trace::RawRecord>,
    action: Value,
    identities: BTreeMap<String, PreparedIdentity>,
    frame_preserved: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct RelationEvidenceIndex {
    traces: Vec<PreparedTraceRelations>,
}

impl RelationEvidenceIndex {
    pub(crate) fn new(
        shape: &SurfaceShape,
        traces: &[TransitionTrace],
    ) -> Result<Self, &'static str> {
        let mut prepared = Vec::with_capacity(traces.len());
        for trace in traces {
            let before = records_for(&trace.before, shape)?;
            let after = records_for(&trace.after, shape)?;
            let mut identities = BTreeMap::new();
            for source in &shape.id_sources {
                let before_index = index_by_id(&before, source);
                let after_index = index_by_id(&after, source);
                let identities_stable = before_index
                    .as_ref()
                    .zip(after_index.as_ref())
                    .is_some_and(|(left, right)| left.keys().eq(right.keys()));
                let changed =
                    changed_fields(&before, &after, before_index.as_ref(), after_index.as_ref());
                identities.insert(
                    source.clone(),
                    PreparedIdentity {
                        before_index,
                        after_index,
                        identities_stable,
                        changed,
                    },
                );
            }
            prepared.push(PreparedTraceRelations {
                before,
                after,
                action: trace.action.clone(),
                identities,
                frame_preserved: frame_without_root(&trace.before, &shape.root_path)
                    == frame_without_root(&trace.after, &shape.root_path),
            });
        }
        Ok(Self { traces: prepared })
    }

    pub(crate) fn atoms_and_valid(
        &self,
        hypothesis: &RoleHypothesis,
        limit: Option<usize>,
    ) -> (Vec<String>, bool) {
        let relations = self
            .traces
            .iter()
            .take(limit.unwrap_or(usize::MAX))
            .map(|trace| prepared_relations(hypothesis, trace))
            .collect::<Vec<_>>();
        let atoms = relation_atoms_from_rows(hypothesis.skeleton, &relations);
        let valid = hypothesis.relations_valid(&relations);
        (atoms, valid)
    }

    pub(crate) fn signature_and_valid(
        &self,
        hypothesis: &RoleHypothesis,
        limit: Option<usize>,
    ) -> (RelationSignature, bool) {
        if limit == Some(1) {
            let relations = self
                .traces
                .first()
                .map(|trace| [prepared_relations(hypothesis, trace)])
                .unwrap_or_default();
            return signature_and_valid_from_rows(hypothesis, &relations);
        }
        let relations = self
            .traces
            .iter()
            .take(limit.unwrap_or(usize::MAX))
            .map(|trace| prepared_relations(hypothesis, trace))
            .collect::<Vec<_>>();
        signature_and_valid_from_rows(hypothesis, &relations)
    }
}

impl RelationSignature {
    pub(crate) fn atoms(self) -> Vec<String> {
        let names = relation_names();
        let mut atoms = Vec::with_capacity(names.len() + 2);
        atoms.push("relation_wave:v1".to_owned());
        atoms.push(format!("hypothesis:{}", self.skeleton.canonical_kind()));
        for (index, name) in names.iter().enumerate() {
            let bucket = if self.bits & (1u16 << index) != 0 {
                "all"
            } else {
                "none"
            };
            atoms.push(format!("{name}:{bucket}"));
        }
        atoms
    }
}

impl RoleHypothesis {
    #[must_use]
    pub fn relation_atoms(&self, traces: &[TransitionTrace]) -> Vec<String> {
        RelationEvidenceIndex::new(&self.shape, traces)
            .map(|index| index.atoms_and_valid(self, None).0)
            .unwrap_or_default()
    }

    #[must_use]
    pub fn exact_on(&self, traces: &[TransitionTrace]) -> bool {
        let (program, adapter) = self.compile_actor();
        traces.iter().all(|trace| {
            let result = execute_surface(&program, &adapter, &trace.before, &trace.action);
            result.status == ExecutionStatus::Executed
                && result.concrete_after.as_ref() == Some(&trace.after)
        })
    }

    #[must_use]
    pub fn compile_actor(&self) -> (TransitionProgram, SurfaceAdapter) {
        let mut slot_paths = BTreeMap::new();
        let mut record_paths = BTreeMap::new();
        match self.skeleton {
            OperatorSkeleton::SetField | OperatorSkeleton::IncrementField => {
                slot_paths.insert("target".to_owned(), self.target_action_path.clone());
                if let Some(operand_slot) = self.skeleton.operand_slot() {
                    slot_paths.insert(operand_slot.to_owned(), self.operand_action_path.clone());
                }
            }
            OperatorSkeleton::AppendRecord => {
                record_paths.insert("id".to_owned(), self.target_action_path.clone());
                record_paths.insert("value".to_owned(), self.operand_action_path.clone());
            }
            OperatorSkeleton::DeleteRecord => {
                slot_paths.insert("target".to_owned(), self.target_action_path.clone());
            }
        }
        let mut field_map = BTreeMap::from([("id".to_owned(), self.record_id_source.clone())]);
        if self.skeleton != OperatorSkeleton::DeleteRecord {
            field_map.insert("value".to_owned(), self.target_field.clone());
        }
        let adapter = SurfaceAdapter {
            name: self.adapter_name(),
            layout: self.shape.layout.into(),
            root_path: self.shape.root_path.clone(),
            field_map,
            action_kind_path: self.action_kind_path.clone(),
            action_rules: vec![ActionRule {
                concrete_kind: self.concrete_action_kind.clone(),
                canonical_kind: self.skeleton.canonical_kind().to_owned(),
                slot_paths,
                slot_constants: BTreeMap::new(),
                record_paths,
            }],
        };
        let operation = match self.skeleton {
            OperatorSkeleton::SetField => TransitionOperation::SetField {
                match_field: "id".to_owned(),
                target_slot: "target".to_owned(),
                target_field: "value".to_owned(),
                value_slot: "value".to_owned(),
            },
            OperatorSkeleton::IncrementField => TransitionOperation::IncrementField {
                match_field: "id".to_owned(),
                target_slot: "target".to_owned(),
                target_field: "value".to_owned(),
                amount_slot: "amount".to_owned(),
            },
            OperatorSkeleton::AppendRecord => TransitionOperation::AppendRecord {
                record_bindings: BTreeMap::from([
                    ("id".to_owned(), "record.id".to_owned()),
                    ("value".to_owned(), "record.value".to_owned()),
                ]),
            },
            OperatorSkeleton::DeleteRecord => TransitionOperation::DeleteRecord {
                match_field: "id".to_owned(),
                target_slot: "target".to_owned(),
            },
        };
        let program = match self.skeleton {
            OperatorSkeleton::SetField => {
                TransitionProgram::new(self.skeleton.canonical_kind(), operation)
                    .with_slot_type("target", ValueKind::String)
                    .with_slot_type("value", ValueKind::String)
            }
            OperatorSkeleton::IncrementField => {
                TransitionProgram::new(self.skeleton.canonical_kind(), operation)
                    .with_slot_type("target", ValueKind::String)
                    .with_slot_type("amount", ValueKind::Number)
            }
            OperatorSkeleton::AppendRecord => {
                TransitionProgram::new(self.skeleton.canonical_kind(), operation)
                    .with_slot_type("record.id", ValueKind::String)
                    .with_slot_type("record.value", ValueKind::String)
            }
            OperatorSkeleton::DeleteRecord => {
                TransitionProgram::new(self.skeleton.canonical_kind(), operation)
                    .with_slot_type("target", ValueKind::String)
            }
        };
        (program, adapter)
    }

    #[must_use]
    pub fn stable_key(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn adapter_name(&self) -> String {
        let root = if self.shape.root_path.is_empty() {
            "root".to_owned()
        } else {
            self.shape.root_path.join("_")
        };
        format!(
            "induced_{root}_{}_{}",
            self.concrete_action_kind, self.target_field
        )
    }

    fn relations_valid(&self, relations: &[TraceRelations]) -> bool {
        !relations.is_empty()
            && relations.iter().all(|row| {
                if !row.root_valid || !row.frame_preserved || row.no_op {
                    return false;
                }
                match self.skeleton {
                    OperatorSkeleton::SetField => {
                        row.record_count_stable
                            && row.identities_stable
                            && row.target_unique
                            && row.target_field_changed
                            && row.only_target_changed
                            && row.operand_type_matches
                            && row.set_effect
                    }
                    OperatorSkeleton::IncrementField => {
                        row.record_count_stable
                            && row.identities_stable
                            && row.target_unique
                            && row.target_field_changed
                            && row.only_target_changed
                            && row.operand_type_matches
                            && row.increment_effect
                    }
                    OperatorSkeleton::AppendRecord => {
                        row.target_unique && row.operand_type_matches && row.append_effect
                    }
                    OperatorSkeleton::DeleteRecord => row.target_unique && row.delete_effect,
                }
            })
    }
}

fn prepared_relations(
    hypothesis: &RoleHypothesis,
    trace: &PreparedTraceRelations,
) -> TraceRelations {
    let Some(identity) = trace.identities.get(&hypothesis.record_id_source) else {
        return TraceRelations::default();
    };
    let Some(target) = value_at(&trace.action, &hypothesis.target_action_path) else {
        return TraceRelations::default();
    };
    let operand = hypothesis
        .skeleton
        .operand_slot()
        .and_then(|_| value_at(&trace.action, &hypothesis.operand_action_path));
    if hypothesis.skeleton.operand_slot().is_some() && operand.is_none() {
        return TraceRelations::default();
    }
    let Some(target_key) = serde_json::to_string(target).ok() else {
        return TraceRelations::default();
    };
    let before_position = identity
        .before_index
        .as_ref()
        .and_then(|index| index.get(&target_key))
        .copied();
    let after_position = identity
        .after_index
        .as_ref()
        .and_then(|index| index.get(&target_key))
        .copied();
    let target_field_changed = identity
        .changed
        .iter()
        .any(|(id, field)| id == &target_key && field == &hypothesis.target_field);
    let only_target_changed = identity.changed.len() == 1
        && identity
            .changed
            .first()
            .is_some_and(|(id, field)| id == &target_key && field == &hypothesis.target_field);
    let before_value = before_position
        .and_then(|position| trace.before.get(position))
        .and_then(|record| record.fields.get(&hypothesis.target_field));
    let after_value = after_position
        .and_then(|position| trace.after.get(position))
        .and_then(|record| record.fields.get(&hypothesis.target_field));
    let set_effect =
        operand.is_some_and(|operand| after_value == Some(operand) && before_value != after_value);
    let increment_effect = before_value
        .zip(after_value)
        .zip(operand)
        .and_then(|((left, right), operand)| numeric_delta_equals(left, right, operand))
        .unwrap_or(false);
    let before_ids = identity.before_index.as_ref();
    let after_ids = identity.after_index.as_ref();
    let common_unchanged = before_ids
        .zip(after_ids)
        .is_some_and(|(before_index, after_index)| {
            common_records_unchanged(&trace.before, &trace.after, before_index, after_index)
        });
    let appended_ids = before_ids.zip(after_ids).map_or(0, |(before, after)| {
        after.keys().filter(|id| !before.contains_key(*id)).count()
    });
    let removed_ids = before_ids.zip(after_ids).map_or(0, |(before, after)| {
        before.keys().filter(|id| !after.contains_key(*id)).count()
    });
    let append_effect = trace.after.len() == trace.before.len() + 1
        && before_position.is_none()
        && after_position.is_some()
        && appended_ids == 1
        && common_unchanged
        && operand.is_some_and(|operand| after_value == Some(operand));
    let delete_effect = trace.before.len() == trace.after.len() + 1
        && before_position.is_some()
        && after_position.is_none()
        && removed_ids == 1
        && common_unchanged;
    let operand_type_matches = match hypothesis.skeleton {
        OperatorSkeleton::SetField => before_value.zip(after_value).is_some_and(|(left, right)| {
            value_kind(left) == value_kind(right) && right.is_string()
        }),
        OperatorSkeleton::IncrementField => {
            before_value.is_some_and(Value::is_number)
                && after_value.is_some_and(Value::is_number)
                && operand.is_some_and(Value::is_number)
        }
        OperatorSkeleton::AppendRecord => after_value
            .zip(operand)
            .is_some_and(|(after, operand)| value_kind(after) == value_kind(operand)),
        OperatorSkeleton::DeleteRecord => true,
    };
    TraceRelations {
        root_valid: true,
        record_count_stable: trace.before.len() == trace.after.len(),
        identities_stable: identity.identities_stable,
        target_unique: match hypothesis.skeleton {
            OperatorSkeleton::AppendRecord => before_position.is_none() && after_position.is_some(),
            OperatorSkeleton::DeleteRecord => before_position.is_some() && after_position.is_none(),
            OperatorSkeleton::SetField | OperatorSkeleton::IncrementField => {
                before_position.is_some() && after_position.is_some()
            }
        },
        target_field_changed,
        only_target_changed,
        frame_preserved: trace.frame_preserved,
        set_effect,
        increment_effect,
        append_effect,
        delete_effect,
        operand_type_matches,
        no_op: trace.before.len() == trace.after.len()
            && identity.identities_stable
            && identity.changed.is_empty(),
    }
}

fn common_records_unchanged(
    before: &[crate::trace::RawRecord],
    after: &[crate::trace::RawRecord],
    before_index: &BTreeMap<String, usize>,
    after_index: &BTreeMap<String, usize>,
) -> bool {
    before_index.iter().all(|(id, before_position)| {
        after_index.get(id).is_none_or(|after_position| {
            before[*before_position].fields == after[*after_position].fields
        })
    })
}

fn changed_fields(
    before: &[crate::trace::RawRecord],
    after: &[crate::trace::RawRecord],
    before_index: Option<&BTreeMap<String, usize>>,
    after_index: Option<&BTreeMap<String, usize>>,
) -> Vec<(String, String)> {
    let (Some(before_index), Some(after_index)) = (before_index, after_index) else {
        return Vec::new();
    };
    let mut changed = Vec::new();
    for (id, left_index) in before_index {
        let Some(right_index) = after_index.get(id) else {
            continue;
        };
        let left = &before[*left_index].fields;
        let right = &after[*right_index].fields;
        let fields = left
            .keys()
            .chain(right.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for field in fields {
            if left.get(&field) != right.get(&field) {
                changed.push((id.clone(), field));
            }
        }
    }
    changed
}

fn relation_atoms_from_rows(
    skeleton: OperatorSkeleton,
    relations: &[TraceRelations],
) -> Vec<String> {
    let mut atoms = vec![
        "relation_wave:v1".to_owned(),
        format!("hypothesis:{}", skeleton.canonical_kind()),
    ];
    push_boolean_atom(&mut atoms, "root_valid", relations, |row| row.root_valid);
    push_boolean_atom(&mut atoms, "record_count_stable", relations, |row| {
        row.record_count_stable
    });
    push_boolean_atom(&mut atoms, "identities_stable", relations, |row| {
        row.identities_stable
    });
    push_boolean_atom(&mut atoms, "target_unique", relations, |row| {
        row.target_unique
    });
    push_boolean_atom(&mut atoms, "target_field_changed", relations, |row| {
        row.target_field_changed
    });
    push_boolean_atom(&mut atoms, "only_target_changed", relations, |row| {
        row.only_target_changed
    });
    push_boolean_atom(&mut atoms, "frame_preserved", relations, |row| {
        row.frame_preserved
    });
    push_boolean_atom(&mut atoms, "set_effect", relations, |row| row.set_effect);
    push_boolean_atom(&mut atoms, "increment_effect", relations, |row| {
        row.increment_effect
    });
    push_boolean_atom(&mut atoms, "append_effect", relations, |row| {
        row.append_effect
    });
    push_boolean_atom(&mut atoms, "delete_effect", relations, |row| {
        row.delete_effect
    });
    push_boolean_atom(&mut atoms, "operand_type_matches", relations, |row| {
        row.operand_type_matches
    });
    push_boolean_atom(&mut atoms, "no_op", relations, |row| row.no_op);
    atoms
}

fn relation_names() -> [&'static str; 13] {
    [
        "root_valid",
        "record_count_stable",
        "identities_stable",
        "target_unique",
        "target_field_changed",
        "only_target_changed",
        "frame_preserved",
        "set_effect",
        "increment_effect",
        "append_effect",
        "delete_effect",
        "operand_type_matches",
        "no_op",
    ]
}

fn relation_flags(relations: &[TraceRelations]) -> [bool; 13] {
    let all =
        |select: fn(&TraceRelations) -> bool| !relations.is_empty() && relations.iter().all(select);
    [
        all(|row| row.root_valid),
        all(|row| row.record_count_stable),
        all(|row| row.identities_stable),
        all(|row| row.target_unique),
        all(|row| row.target_field_changed),
        all(|row| row.only_target_changed),
        all(|row| row.frame_preserved),
        all(|row| row.set_effect),
        all(|row| row.increment_effect),
        all(|row| row.append_effect),
        all(|row| row.delete_effect),
        all(|row| row.operand_type_matches),
        all(|row| row.no_op),
    ]
}

fn signature_and_valid_from_rows(
    hypothesis: &RoleHypothesis,
    relations: &[TraceRelations],
) -> (RelationSignature, bool) {
    let flags = relation_flags(relations);
    let bits = flags.iter().enumerate().fold(0u16, |bits, (index, flag)| {
        bits | (u16::from(*flag) << index)
    });
    (
        RelationSignature {
            skeleton: hypothesis.skeleton,
            bits,
        },
        hypothesis.relations_valid(relations),
    )
}

pub(crate) fn discover_action_groups(
    traces: &[TransitionTrace],
) -> Result<(Vec<String>, ActionTraceGroups), &'static str> {
    let Some(first) = traces.first() else {
        return Err("empty_trace_batch");
    };
    let candidates = scalar_paths(&first.action);
    let mut ranked = Vec::new();
    for ScalarPath { path } in candidates {
        let values = traces
            .iter()
            .map(|trace| value_at(&trace.action, &path).and_then(Value::as_str))
            .collect::<Option<Vec<_>>>();
        let Some(values) = values else {
            continue;
        };
        let unique = values.iter().copied().collect::<BTreeSet<_>>();
        if unique.is_empty() || unique.len() > 8 {
            continue;
        }
        let min_support = unique
            .iter()
            .map(|value| {
                values
                    .iter()
                    .filter(|candidate| *candidate == value)
                    .count()
            })
            .min()
            .unwrap_or(0);
        if min_support >= 2 {
            let state_copies = traces
                .iter()
                .zip(&values)
                .filter(|(trace, value)| {
                    value_occurs(&trace.before, value) || value_occurs(&trace.after, value)
                })
                .count();
            ranked.push((state_copies, unique.len(), path));
        }
    }
    if ranked
        .iter()
        .any(|(state_copies, unique, _)| *state_copies == 0 && *unique > 1)
    {
        ranked.retain(|(state_copies, unique, _)| *state_copies == 0 && *unique > 1);
    }
    ranked.sort();
    let Some((_, _, kind_path)) = ranked.into_iter().next() else {
        return Err("action_kind_path_not_found");
    };
    let mut groups: BTreeMap<String, Vec<TransitionTrace>> = BTreeMap::new();
    for trace in traces {
        let kind = value_at(&trace.action, &kind_path)
            .and_then(Value::as_str)
            .ok_or("action_kind_missing")?;
        groups
            .entry(kind.to_owned())
            .or_default()
            .push(trace.clone());
    }
    Ok((kind_path, groups))
}

fn value_occurs(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(candidate) => candidate == needle,
        Value::Array(items) => items.iter().any(|item| value_occurs(item, needle)),
        Value::Object(fields) => fields.values().any(|field| value_occurs(field, needle)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

pub(crate) fn enumerate_hypotheses(
    shape: &SurfaceShape,
    action_kind_path: &[String],
    concrete_kind: &str,
    traces: &[TransitionTrace],
) -> Vec<RoleHypothesis> {
    let Some(first) = traces.first() else {
        return Vec::new();
    };
    let action_paths = scalar_paths(&first.action)
        .into_iter()
        .map(|leaf| leaf.path)
        .filter(|path| path != action_kind_path)
        .filter(|path| {
            traces
                .iter()
                .all(|trace| value_at(&trace.action, path).is_some())
        })
        .collect::<Vec<_>>();
    let mut hypotheses = Vec::new();
    for record_id_source in &shape.id_sources {
        for target_action_path in &action_paths {
            hypotheses.push(RoleHypothesis {
                shape: shape.clone(),
                action_kind_path: action_kind_path.to_vec(),
                concrete_action_kind: concrete_kind.to_owned(),
                record_id_source: record_id_source.clone(),
                target_field: String::new(),
                target_action_path: target_action_path.clone(),
                operand_action_path: Vec::new(),
                skeleton: OperatorSkeleton::DeleteRecord,
            });
        }
        for target_field in &shape.record_fields {
            if target_field == record_id_source {
                continue;
            }
            for target_action_path in &action_paths {
                for operand_action_path in &action_paths {
                    if target_action_path == operand_action_path {
                        continue;
                    }
                    for skeleton in [
                        OperatorSkeleton::SetField,
                        OperatorSkeleton::IncrementField,
                        OperatorSkeleton::AppendRecord,
                    ] {
                        hypotheses.push(RoleHypothesis {
                            shape: shape.clone(),
                            action_kind_path: action_kind_path.to_vec(),
                            concrete_action_kind: concrete_kind.to_owned(),
                            record_id_source: record_id_source.clone(),
                            target_field: target_field.clone(),
                            target_action_path: target_action_path.clone(),
                            operand_action_path: operand_action_path.clone(),
                            skeleton,
                        });
                    }
                }
            }
        }
    }
    hypotheses.sort();
    hypotheses
}

fn push_boolean_atom(
    atoms: &mut Vec<String>,
    name: &str,
    rows: &[TraceRelations],
    select: impl Fn(&TraceRelations) -> bool,
) {
    let count = rows.iter().filter(|row| select(row)).count();
    let bucket = if !rows.is_empty() && count == rows.len() {
        "all"
    } else if count == 0 {
        "none"
    } else {
        "some"
    };
    atoms.push(format!("{name}:{bucket}"));
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

fn numeric_delta_equals(before: &Value, after: &Value, amount: &Value) -> Option<bool> {
    if let (Some(before), Some(after), Some(amount)) =
        (before.as_i64(), after.as_i64(), amount.as_i64())
    {
        return Some(before.checked_add(amount) == Some(after));
    }
    if let (Some(before), Some(after), Some(amount)) =
        (before.as_u64(), after.as_u64(), amount.as_u64())
    {
        return Some(before.checked_add(amount) == Some(after));
    }
    let (before, after, amount) = (before.as_f64()?, after.as_f64()?, amount.as_f64()?);
    Some((before + amount - after).abs() <= f64::EPSILON)
}

fn frame_without_root(value: &Value, root_path: &[String]) -> Value {
    let mut frame = value.clone();
    remove_path(&mut frame, root_path);
    frame
}

fn remove_path(value: &mut Value, path: &[String]) {
    let Some((last, parents)) = path.split_last() else {
        *value = Value::Null;
        return;
    };
    let mut cursor = value;
    for segment in parents {
        let Some(next) = cursor
            .as_object_mut()
            .and_then(|object| object.get_mut(segment))
        else {
            return;
        };
        cursor = next;
    }
    if let Some(object) = cursor.as_object_mut() {
        object.remove(last);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn discovers_constant_action_kind_without_field_name_prior() {
        let traces = (0..4)
            .map(|index| {
                let id = format!("operation-{index}");
                TransitionTrace {
                    before: json!({
                        "items": [{"opaque_id": id, "opaque_state": "pending"}]
                    }),
                    action: json!({
                        "unknown": {
                            "alpha": "edit",
                            "beta": id,
                            "gamma": "completed"
                        }
                    }),
                    after: json!({
                        "items": [{"opaque_id": id, "opaque_state": "completed"}]
                    }),
                }
            })
            .collect::<Vec<_>>();

        let (path, groups) = discover_action_groups(&traces).expect("action groups");

        assert_eq!(path, vec!["unknown", "alpha"]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.get("edit").map(Vec::len), Some(4));
    }
}
