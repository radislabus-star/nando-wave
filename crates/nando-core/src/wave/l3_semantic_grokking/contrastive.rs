use std::collections::HashSet;

use super::{L3FrameCenter, L3SemanticExample};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum L3ContrastiveNegativeKind {
    RoleSwap,
    RouteSplice,
    MissingEvidence,
    NegativeRoute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct L3ContrastiveNegativeCase {
    pub(super) kind: L3ContrastiveNegativeKind,
    pub(super) text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct L3ContrastiveTrainingSet {
    pub(super) negative_cases: Vec<L3ContrastiveNegativeCase>,
    pub(super) contrastive_dataset_used: bool,
    pub(super) training_trap_generator_used: bool,
    pub(super) proof_fixture_used_for_training: bool,
}

impl L3ContrastiveTrainingSet {
    pub(super) fn from_examples(examples: &[L3SemanticExample], frames: &[L3FrameCenter]) -> Self {
        let mut seen = HashSet::new();
        let mut negative_cases = Vec::with_capacity(examples.len() * 4);

        for example in examples {
            let schema = &example.fact.schema;
            let subject = &example.fact.subject.label;
            let object = &example.fact.object.label;
            let relation_verb = relation_verb(&schema.relation);

            push_negative(
                &mut negative_cases,
                &mut seen,
                L3ContrastiveNegativeKind::RoleSwap,
                format!(
                    "which {} {relation_verb} {} {subject}",
                    schema.object_role, schema.subject_role
                ),
            );
            push_negative(
                &mut negative_cases,
                &mut seen,
                L3ContrastiveNegativeKind::MissingEvidence,
                format!("who {relation_verb} {object}"),
            );
            push_negative(
                &mut negative_cases,
                &mut seen,
                L3ContrastiveNegativeKind::NegativeRoute,
                format!(
                    "which {} {relation_verb} {} {object} proves runtime active",
                    schema.subject_role, schema.object_role
                ),
            );

            for wrong_frame in route_splice_frames(frames, &schema.subject_role) {
                push_negative(
                    &mut negative_cases,
                    &mut seen,
                    L3ContrastiveNegativeKind::RouteSplice,
                    format!(
                        "which {} {relation_verb} {} {object}",
                        wrong_frame.unknown_role, schema.object_role
                    ),
                );
            }
        }

        Self {
            contrastive_dataset_used: !negative_cases.is_empty(),
            negative_cases,
            training_trap_generator_used: false,
            proof_fixture_used_for_training: false,
        }
    }

    pub(super) fn negative_count(&self) -> usize {
        self.negative_cases.len()
    }
}

fn push_negative(
    cases: &mut Vec<L3ContrastiveNegativeCase>,
    seen: &mut HashSet<(L3ContrastiveNegativeKind, String)>,
    kind: L3ContrastiveNegativeKind,
    text: String,
) {
    if seen.insert((kind, text.clone())) {
        cases.push(L3ContrastiveNegativeCase { kind, text });
    }
}

fn route_splice_frames<'a>(
    frames: &'a [L3FrameCenter],
    source_unknown_role: &str,
) -> impl Iterator<Item = &'a L3FrameCenter> {
    frames
        .iter()
        .filter(move |frame| frame.unknown_role != source_unknown_role)
}

fn relation_verb(relation: &str) -> &str {
    relation.split('_').next().unwrap_or(relation)
}
