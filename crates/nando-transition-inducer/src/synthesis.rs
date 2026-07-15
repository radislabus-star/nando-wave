use nando_transition_actor::SurfaceAdapter;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::guard::slot;
use crate::{GuardProgram, OperatorSkeleton, RoleHypothesis, TransitionTrace, VerifierProgram};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SynthesisMetrics {
    pub guard_candidates_checked: usize,
    pub guard_counterexamples: usize,
    pub verifier_candidates_checked: usize,
    pub verifier_counterexamples: usize,
}

pub(crate) fn synthesize_contracts(
    hypothesis: &RoleHypothesis,
    traces: &[TransitionTrace],
) -> Result<(GuardProgram, VerifierProgram, SynthesisMetrics), &'static str> {
    let (_, adapter) = hypothesis.compile_actor();
    let (guard, guard_metrics) = synthesize_guard(hypothesis.skeleton, &adapter, traces)?;
    let (verifier, verifier_metrics) = synthesize_verifier(hypothesis.skeleton, &adapter, traces)?;
    Ok((
        guard,
        verifier,
        SynthesisMetrics {
            guard_candidates_checked: guard_metrics.0,
            guard_counterexamples: guard_metrics.1,
            verifier_candidates_checked: verifier_metrics.0,
            verifier_counterexamples: verifier_metrics.1,
        },
    ))
}

fn synthesize_guard(
    skeleton: OperatorSkeleton,
    adapter: &SurfaceAdapter,
    traces: &[TransitionTrace],
) -> Result<(GuardProgram, (usize, usize)), &'static str> {
    let grammar = GuardProgram::grammar(skeleton);
    let negatives = guard_counterexample_pool(skeleton, adapter, traces);
    let mut known_counterexamples = Vec::new();
    let mut checked = 0usize;

    loop {
        let candidate = grammar.iter().find(|candidate| {
            checked += 1;
            traces.iter().all(|trace| {
                candidate
                    .check(adapter, &trace.before, &trace.action)
                    .is_ok()
            }) && known_counterexamples.iter().all(|index| {
                let (before, action) = &negatives[*index];
                candidate.check(adapter, before, action).is_err()
            })
        });
        let Some(candidate) = candidate else {
            return Err("guard_grammar_exhausted");
        };
        if let Some(index) = negatives
            .iter()
            .position(|(before, action)| candidate.check(adapter, before, action).is_ok())
        {
            if known_counterexamples.contains(&index) {
                return Err("guard_cegis_stalled");
            }
            known_counterexamples.push(index);
            continue;
        }
        return Ok((candidate.clone(), (checked, known_counterexamples.len())));
    }
}

fn synthesize_verifier(
    skeleton: OperatorSkeleton,
    adapter: &SurfaceAdapter,
    traces: &[TransitionTrace],
) -> Result<(VerifierProgram, (usize, usize)), &'static str> {
    let grammar = VerifierProgram::grammar(skeleton);
    let negatives = verifier_counterexample_pool(adapter, traces);
    let mut known_counterexamples = Vec::new();
    let mut checked = 0usize;

    loop {
        let candidate = grammar.iter().find(|candidate| {
            checked += 1;
            traces.iter().all(|trace| {
                candidate
                    .verify(adapter, &trace.before, &trace.action, &trace.after)
                    .is_ok()
            }) && known_counterexamples.iter().all(|index| {
                let (before, action, invalid_after) = &negatives[*index];
                candidate
                    .verify(adapter, before, action, invalid_after)
                    .is_err()
            })
        });
        let Some(candidate) = candidate else {
            return Err("verifier_grammar_exhausted");
        };
        if let Some(index) = negatives
            .iter()
            .position(|(before, action, invalid_after)| {
                candidate
                    .verify(adapter, before, action, invalid_after)
                    .is_ok()
            })
        {
            if known_counterexamples.contains(&index) {
                return Err("verifier_cegis_stalled");
            }
            known_counterexamples.push(index);
            continue;
        }
        return Ok((candidate.clone(), (checked, known_counterexamples.len())));
    }
}

fn guard_counterexample_pool(
    skeleton: OperatorSkeleton,
    adapter: &SurfaceAdapter,
    traces: &[TransitionTrace],
) -> Vec<(Value, Value)> {
    let mut out = Vec::new();
    for trace in traces {
        let Ok(canonical_action) = adapter.adapt_action(&trace.action) else {
            continue;
        };
        let target_slot = if skeleton == OperatorSkeleton::AppendRecord {
            "record.id"
        } else {
            "target"
        };
        let operand_slot = match skeleton {
            OperatorSkeleton::SetField => Some("value"),
            OperatorSkeleton::IncrementField => Some("amount"),
            OperatorSkeleton::AppendRecord => Some("record.value"),
            OperatorSkeleton::DeleteRecord => None,
        };

        let mut missing_target = canonical_action.clone();
        let invalid_target = if skeleton == OperatorSkeleton::AppendRecord {
            first_record_id(adapter, trace)
                .unwrap_or_else(|| Value::String("__cegis_duplicate_target__".to_owned()))
        } else {
            Value::String("__cegis_missing_target__".to_owned())
        };
        set_canonical_slot(&mut missing_target, target_slot, invalid_target);
        push_encoded_action(&mut out, adapter, &trace.before, &missing_target);

        if let Some(operand_slot) = operand_slot {
            let mut wrong_type = canonical_action.clone();
            let wrong_value = match skeleton {
                OperatorSkeleton::SetField | OperatorSkeleton::AppendRecord => Value::from(42),
                OperatorSkeleton::IncrementField => {
                    Value::String("__cegis_not_numeric__".to_owned())
                }
                OperatorSkeleton::DeleteRecord => Value::Null,
            };
            set_canonical_slot(&mut wrong_type, operand_slot, wrong_value);
            push_encoded_action(&mut out, adapter, &trace.before, &wrong_type);
        }

        if skeleton != OperatorSkeleton::DeleteRecord {
            let mut no_effect = canonical_action.clone();
            if skeleton == OperatorSkeleton::IncrementField {
                set_canonical_slot(&mut no_effect, "amount", Value::from(0));
            } else if skeleton == OperatorSkeleton::SetField
                && let Some(current) = current_target_value(adapter, trace, &canonical_action)
            {
                set_canonical_slot(&mut no_effect, "value", current);
            } else if skeleton == OperatorSkeleton::AppendRecord
                && let Some(existing_id) = first_record_id(adapter, trace)
            {
                set_canonical_slot(&mut no_effect, "record.id", existing_id);
            }
            push_encoded_action(&mut out, adapter, &trace.before, &no_effect);
        }

        if skeleton != OperatorSkeleton::AppendRecord
            && let Some(operand_slot) = operand_slot
        {
            let mut role_swap = canonical_action.clone();
            if let (Some(target), Some(operand)) = (
                slot(&canonical_action, target_slot).cloned(),
                slot(&canonical_action, operand_slot).cloned(),
            ) {
                set_canonical_slot(&mut role_swap, target_slot, operand);
                set_canonical_slot(&mut role_swap, operand_slot, target);
                push_encoded_action(&mut out, adapter, &trace.before, &role_swap);
            }
        }

        if skeleton != OperatorSkeleton::AppendRecord
            && let Some(ambiguous_before) =
                ambiguous_target_state(adapter, trace, &canonical_action)
        {
            out.push((ambiguous_before, trace.action.clone()));
        }
    }
    out
}

fn verifier_counterexample_pool(
    adapter: &SurfaceAdapter,
    traces: &[TransitionTrace],
) -> Vec<(Value, Value, Value)> {
    let mut out = Vec::new();
    for trace in traces {
        out.push((
            trace.before.clone(),
            trace.action.clone(),
            trace.before.clone(),
        ));
        if let Some(complement) = mutate_canonical_complement(adapter, trace)
            && complement != trace.after
        {
            out.push((trace.before.clone(), trace.action.clone(), complement));
        }
        if let Some(cardinality) = mutate_record_cardinality(adapter, trace)
            && cardinality != trace.after
        {
            out.push((trace.before.clone(), trace.action.clone(), cardinality));
        }
        let mut frame = trace.after.clone();
        if let Some(object) = frame.as_object_mut() {
            object.insert(
                "__cegis_surface_frame__".to_owned(),
                Value::String("mutated".to_owned()),
            );
            out.push((trace.before.clone(), trace.action.clone(), frame));
        }
    }
    out
}

fn set_canonical_slot(action: &mut Map<String, Value>, path: &str, value: Value) {
    let mut segments = path.split('.');
    let Some(first) = segments.next() else {
        return;
    };
    let Some(second) = segments.next() else {
        action.insert(first.to_owned(), value);
        return;
    };
    let nested = action
        .entry(first.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(object) = nested.as_object_mut() {
        object.insert(second.to_owned(), value);
    }
}

fn first_record_id(adapter: &SurfaceAdapter, trace: &TransitionTrace) -> Option<Value> {
    adapter
        .adapt_state(&trace.before)
        .ok()?
        .records
        .first()?
        .get("id")
        .cloned()
}

fn push_encoded_action(
    out: &mut Vec<(Value, Value)>,
    adapter: &SurfaceAdapter,
    before: &Value,
    canonical_action: &Map<String, Value>,
) {
    if let Ok(action) = adapter.encode_action(canonical_action) {
        out.push((before.clone(), action));
    }
}

fn current_target_value(
    adapter: &SurfaceAdapter,
    trace: &TransitionTrace,
    action: &Map<String, Value>,
) -> Option<Value> {
    let target = action.get("target")?;
    adapter
        .adapt_state(&trace.before)
        .ok()?
        .records
        .into_iter()
        .find(|record| record.get("id") == Some(target))?
        .get("value")
        .cloned()
}

fn ambiguous_target_state(
    adapter: &SurfaceAdapter,
    trace: &TransitionTrace,
    action: &Map<String, Value>,
) -> Option<Value> {
    let target = action.get("target")?;
    let adapted = adapter.adapt_state(&trace.before).ok()?;
    let duplicate = adapted
        .records
        .iter()
        .find(|record| record.get("id") == Some(target))?
        .clone();
    let mut records = adapted.records.clone();
    records.push(duplicate);
    let projected = adapter.project(&records, &adapted).ok()?;
    let projected_state = adapter.adapt_state(&projected).ok()?;
    (projected_state
        .records
        .iter()
        .filter(|record| record.get("id") == Some(target))
        .count()
        > 1)
    .then_some(projected)
}

fn mutate_canonical_complement(adapter: &SurfaceAdapter, trace: &TransitionTrace) -> Option<Value> {
    let action = adapter.adapt_action(&trace.action).ok()?;
    let target = slot(&action, "target").or_else(|| slot(&action, "record.id"))?;
    let adapted = adapter.adapt_state(&trace.after).ok()?;
    let mut records = adapted.records.clone();
    let record = records
        .iter_mut()
        .find(|record| record.get("id") != Some(target))?;
    record.insert(
        "value".to_owned(),
        Value::String("__cegis_complement_mutation__".to_owned()),
    );
    adapter.project(&records, &adapted).ok()
}

fn mutate_record_cardinality(adapter: &SurfaceAdapter, trace: &TransitionTrace) -> Option<Value> {
    let before = adapter.adapt_state(&trace.before).ok()?;
    let adapted = adapter.adapt_state(&trace.after).ok()?;
    let mut records = adapted.records.clone();
    if records.len() > before.records.len() {
        let mut extra = records.last()?.clone();
        extra.insert(
            "id".to_owned(),
            Value::String("__cegis_extra_record__".to_owned()),
        );
        records.push(extra);
    } else {
        records.pop()?;
    }
    adapter.project(&records, &adapted).ok()
}
