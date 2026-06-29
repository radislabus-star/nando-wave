use std::collections::HashMap;

use super::super::{
    SemanticCandidate, SemanticEquationForm, SemanticEquationPrediction, SemanticSchemaKey,
};
use super::L3SemanticExample;

pub(super) const L3_ANSWER_BINDING_OPERATOR_BYTES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
struct L3AnswerBindingOperator {
    unknown_role: String,
    subject_family: String,
    slot_delta_sum: i64,
    support: u32,
}

impl L3AnswerBindingOperator {
    fn learned_slot_delta(&self) -> i64 {
        if self.support == 0 {
            0
        } else {
            self.slot_delta_sum / i64::from(self.support)
        }
    }

    fn predicted_subject_slot(&self, object_slot: u32) -> u32 {
        let predicted = i64::from(object_slot) + self.learned_slot_delta();
        predicted.clamp(0, i64::from(u32::MAX)) as u32
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct L3AnswerBindingMemory {
    operators: HashMap<SemanticSchemaKey, L3AnswerBindingOperator>,
    pub(super) learned: bool,
    pub(super) lookup_only: bool,
}

impl L3AnswerBindingMemory {
    pub(super) fn from_training(examples: &[L3SemanticExample]) -> Self {
        let mut operators: HashMap<SemanticSchemaKey, L3AnswerBindingOperator> = HashMap::new();

        for example in examples {
            let operator = operators
                .entry(example.fact.schema.clone())
                .or_insert_with(|| L3AnswerBindingOperator {
                    unknown_role: example.fact.schema.subject_role.clone(),
                    subject_family: example.fact.subject.family.clone(),
                    slot_delta_sum: 0,
                    support: 0,
                });
            operator.support += 1;
            operator.slot_delta_sum +=
                i64::from(example.fact.subject.slot) - i64::from(example.fact.object.slot);
        }

        let learned = !operators.is_empty();
        Self {
            operators,
            learned,
            lookup_only: false,
        }
    }

    pub(super) fn operator_count(&self) -> usize {
        self.operators.len()
    }

    pub(super) fn hot_bytes(&self) -> usize {
        self.operators.len() * L3_ANSWER_BINDING_OPERATOR_BYTES
    }

    pub(super) fn supports_unknown_subject(
        &self,
        schema: &SemanticSchemaKey,
        unknown_role: &str,
    ) -> bool {
        self.operators
            .get(schema)
            .is_some_and(|operator| operator.unknown_role == unknown_role && operator.support > 0)
    }

    pub(super) fn solve(
        &self,
        equation: &SemanticEquationForm,
        candidates: &[SemanticCandidate],
    ) -> Option<SemanticEquationPrediction> {
        let operator = self.operators.get(&equation.schema)?;
        let unknown_role = equation.unknown_role.as_ref()?;
        let object = equation.object.as_ref()?;
        if equation.subject.is_some()
            || unknown_role != &operator.unknown_role
            || unknown_role != &equation.schema.subject_role
        {
            return None;
        }

        let predicted_slot = operator.predicted_subject_slot(object.slot);
        let mut best_index = None;
        let mut best_score = i64::MIN;
        let mut runner_up_score = i64::MIN;

        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.atom.role != operator.unknown_role {
                continue;
            }
            let slot_distance = i64::from(candidate.atom.slot.abs_diff(predicted_slot));
            let mut score = 1_000_000i64 - slot_distance.min(1_000_000);
            if candidate.atom.family == operator.subject_family {
                score += 10_000;
            } else {
                score -= 10_000;
            }

            if score > best_score {
                runner_up_score = best_score;
                best_score = score;
                best_index = Some(index);
            } else if score > runner_up_score {
                runner_up_score = score;
            }
        }

        let best_index = best_index?;
        if runner_up_score == i64::MIN {
            runner_up_score = best_score;
        }

        Some(SemanticEquationPrediction {
            resolved_role: unknown_role.clone(),
            resolved_label: candidates[best_index].atom.label.clone(),
            score: best_score,
            runner_up_score,
            margin: best_score - runner_up_score,
        })
    }

    pub(super) fn solve_without_slot_binding(
        &self,
        equation: &SemanticEquationForm,
        candidates: &[SemanticCandidate],
    ) -> Option<SemanticEquationPrediction> {
        let operator = self.operators.get(&equation.schema)?;
        let unknown_role = equation.unknown_role.as_ref()?;
        if equation.subject.is_some()
            || equation.object.is_none()
            || unknown_role != &operator.unknown_role
            || unknown_role != &equation.schema.subject_role
        {
            return None;
        }

        let mut best_index = None;
        let mut best_score = i64::MIN;
        let mut runner_up_score = i64::MIN;

        for (index, candidate) in candidates.iter().enumerate() {
            if candidate.atom.role != operator.unknown_role {
                continue;
            }
            let score = if candidate.atom.family == operator.subject_family {
                10_000
            } else {
                -10_000
            };

            if score > best_score {
                runner_up_score = best_score;
                best_score = score;
                best_index = Some(index);
            } else if score > runner_up_score {
                runner_up_score = score;
            }
        }

        let best_index = best_index?;
        if runner_up_score == i64::MIN {
            runner_up_score = best_score;
        }

        Some(SemanticEquationPrediction {
            resolved_role: unknown_role.clone(),
            resolved_label: candidates[best_index].atom.label.clone(),
            score: best_score,
            runner_up_score,
            margin: best_score - runner_up_score,
        })
    }
}
