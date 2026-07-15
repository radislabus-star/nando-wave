use std::collections::{BTreeMap, BTreeSet};

use crate::ResponseValueSelector;

const MAX_ALIGNMENT_CANDIDATES: usize = 512;
const MAX_ALIGNMENT_SPANS: usize = 4_096;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputValueSource {
    Primary,
    Selector(ResponseValueSelector),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputValueCandidate {
    pub source: OutputValueSource,
    pub rendered: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputGraphSegment {
    Static {
        text: String,
    },
    RuntimeValue {
        sources: Vec<OutputValueSource>,
        rendered: String,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputGraph {
    pub response_bytes: usize,
    pub dynamic_bytes: usize,
    pub static_bytes: usize,
    pub source_ambiguous: bool,
    pub segments: Vec<OutputGraphSegment>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SpanKey {
    start: usize,
    end: usize,
    rendered: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DynamicSpan {
    key: SpanKey,
    sources: Vec<OutputValueSource>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AlignmentPlan {
    covered: usize,
    selected: Vec<usize>,
    ambiguous: bool,
}

impl AlignmentPlan {
    fn with_span(&self, index: usize, bytes: usize, source_ambiguous: bool) -> Self {
        let mut selected = self.selected.clone();
        selected.push(index);
        Self {
            covered: self.covered.saturating_add(bytes),
            selected,
            ambiguous: self.ambiguous || source_ambiguous,
        }
    }
}

pub fn build_output_graph(
    response: &str,
    candidates: impl IntoIterator<Item = OutputValueCandidate>,
) -> Result<OutputGraph, &'static str> {
    if response.is_empty() {
        return Ok(OutputGraph::default());
    }
    let mut values = BTreeMap::<String, BTreeSet<OutputValueSource>>::new();
    for candidate in candidates.into_iter().take(MAX_ALIGNMENT_CANDIDATES + 1) {
        if values.len() >= MAX_ALIGNMENT_CANDIDATES {
            return Err("output_graph_candidate_budget");
        }
        if candidate.rendered.is_empty() || candidate.rendered.len() > response.len() {
            continue;
        }
        values
            .entry(candidate.rendered)
            .or_default()
            .insert(candidate.source);
    }

    let mut spans = BTreeMap::<SpanKey, BTreeSet<OutputValueSource>>::new();
    for (rendered, sources) in values {
        for (start, _) in response.match_indices(&rendered) {
            if spans.len() >= MAX_ALIGNMENT_SPANS {
                return Err("output_graph_span_budget");
            }
            spans
                .entry(SpanKey {
                    start,
                    end: start.saturating_add(rendered.len()),
                    rendered: rendered.clone(),
                })
                .or_default()
                .extend(sources.iter().cloned());
        }
    }
    let mut spans = spans
        .into_iter()
        .map(|(key, sources)| DynamicSpan {
            key,
            sources: sources.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    spans.sort_by(|left, right| {
        left.key
            .end
            .cmp(&right.key.end)
            .then_with(|| left.key.start.cmp(&right.key.start))
            .then_with(|| left.key.rendered.cmp(&right.key.rendered))
    });
    if spans.is_empty() {
        return Ok(OutputGraph {
            response_bytes: response.len(),
            static_bytes: response.len(),
            segments: vec![OutputGraphSegment::Static {
                text: response.to_owned(),
            }],
            ..OutputGraph::default()
        });
    }

    let mut plans = Vec::<AlignmentPlan>::with_capacity(spans.len());
    for (index, span) in spans.iter().enumerate() {
        let excluded = index
            .checked_sub(1)
            .and_then(|previous| plans.get(previous))
            .cloned()
            .unwrap_or_default();
        let predecessor = spans[..index]
            .iter()
            .rposition(|candidate| candidate.key.end <= span.key.start);
        let included_base = predecessor
            .and_then(|previous| plans.get(previous))
            .cloned()
            .unwrap_or_default();
        let included = included_base.with_span(
            index,
            span.key.end.saturating_sub(span.key.start),
            span.sources.len() != 1,
        );
        plans.push(best_plan(excluded, included));
    }

    let plan = plans.pop().unwrap_or_default();
    let mut segments = Vec::new();
    let mut cursor = 0_usize;
    for index in &plan.selected {
        let span = &spans[*index];
        if span.key.start > cursor {
            segments.push(OutputGraphSegment::Static {
                text: response[cursor..span.key.start].to_owned(),
            });
        }
        segments.push(OutputGraphSegment::RuntimeValue {
            sources: span.sources.clone(),
            rendered: span.key.rendered.clone(),
        });
        cursor = span.key.end;
    }
    if cursor < response.len() {
        segments.push(OutputGraphSegment::Static {
            text: response[cursor..].to_owned(),
        });
    }
    Ok(OutputGraph {
        response_bytes: response.len(),
        dynamic_bytes: plan.covered,
        static_bytes: response.len().saturating_sub(plan.covered),
        source_ambiguous: plan.ambiguous,
        segments,
    })
}

fn best_plan(left: AlignmentPlan, right: AlignmentPlan) -> AlignmentPlan {
    let left_score = (left.covered, usize::MAX.saturating_sub(left.selected.len()));
    let right_score = (
        right.covered,
        usize::MAX.saturating_sub(right.selected.len()),
    );
    match left_score.cmp(&right_score) {
        std::cmp::Ordering::Greater => left,
        std::cmp::Ordering::Less => right,
        std::cmp::Ordering::Equal => {
            let different = left.selected != right.selected;
            let mut selected = left.selected.min(right.selected);
            selected.shrink_to_fit();
            AlignmentPlan {
                covered: left.covered,
                selected,
                ambiguous: left.ambiguous || right.ambiguous || different,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AtomValueType;

    fn field(name: &str) -> ResponseValueSelector {
        ResponseValueSelector::JsonField {
            field: name.to_owned(),
            value_type: AtomValueType::Integer,
        }
    }

    #[test]
    fn aligns_multiple_exact_runtime_values_without_vectorization() {
        let graph = build_output_graph(
            "Успешно: 3, ошибок: 2.",
            [
                OutputValueCandidate {
                    source: OutputValueSource::Selector(field("ok")),
                    rendered: "3".to_owned(),
                },
                OutputValueCandidate {
                    source: OutputValueSource::Selector(field("failed")),
                    rendered: "2".to_owned(),
                },
            ],
        )
        .expect("output graph");
        assert_eq!(graph.dynamic_bytes, 2);
        assert!(!graph.source_ambiguous);
        assert_eq!(
            graph
                .segments
                .iter()
                .filter(|segment| matches!(segment, OutputGraphSegment::RuntimeValue { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn marks_equal_values_from_different_sources_as_ambiguous() {
        let graph = build_output_graph(
            "3",
            [
                OutputValueCandidate {
                    source: OutputValueSource::Selector(field("left")),
                    rendered: "3".to_owned(),
                },
                OutputValueCandidate {
                    source: OutputValueSource::Selector(field("right")),
                    rendered: "3".to_owned(),
                },
            ],
        )
        .expect("output graph");
        assert!(graph.source_ambiguous);
    }
}
