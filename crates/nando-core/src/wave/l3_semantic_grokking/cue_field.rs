use std::collections::{HashMap, HashSet};

use super::super::{L2CenterMemory, SemanticSchemaKey};
use super::fixtures::semantic_traps_for_example;
use super::tokens::{cue_tokens, cue_tokens_with_mode, normalized_surface_key, normalized_tokens};
use super::{
    L3_ANTI_CUE_THRESHOLD, L3_CUE_EDGE_BYTES, L3CueTokenMode, L3FrameCenter, L3SemanticExample,
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct L3LearnedCueInference {
    pub(super) cues: L3SemanticFieldCues,
    pub(super) min_margin: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct L3LearnedCueEdge {
    pub(super) token: u32,
    pub(super) cue_kind: String,
    pub(super) cue_value: String,
    pub(super) weight: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct L3LearnedCueKey {
    token: u32,
    cue_kind: String,
    cue_value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct L3LearnedCueField {
    pub(super) edges: Vec<L3LearnedCueEdge>,
    pub(super) edges_by_token: HashMap<u32, Vec<usize>>,
    pub(super) learned: bool,
    pub(super) contrastive: bool,
    pub(super) manual_runtime_rules_used: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct L3SemanticFieldCues {
    pub(super) role: Option<String>,
    pub(super) relation: Option<String>,
    pub(super) object_anchor: Option<String>,
    pub(super) binding: Option<String>,
    pub(super) anti_signatures: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct CueChoice {
    value: String,
    margin: f32,
}

impl L3LearnedCueField {
    pub(super) fn from_training(
        frames: &[L3FrameCenter],
        l2: &L2CenterMemory,
        examples: &[L3SemanticExample],
    ) -> Self {
        let mut weights: HashMap<L3LearnedCueKey, f32> = HashMap::new();
        let mut trap_cache: HashMap<String, (Vec<u32>, L3SemanticFieldCues)> = HashMap::new();
        let cue_universe = cue_universe_from_frames(frames);
        let global_positive_tokens = examples
            .iter()
            .flat_map(|example| cue_tokens(l2, &example.query_surface))
            .collect::<HashSet<_>>();

        for example in examples {
            let tokens = cue_tokens(l2, &example.query_surface);
            let labels = L3SemanticFieldCues::from_schema(&example.fact.schema);
            for (cue_kind, cue_value) in labels.as_pairs() {
                for token in &tokens {
                    add_cue_weight(&mut weights, *token, &cue_kind, &cue_value, 1.0);
                    for wrong_value in cue_universe
                        .get(&cue_kind)
                        .into_iter()
                        .flat_map(|values| values.iter())
                        .filter(|wrong_value| *wrong_value != &cue_value)
                    {
                        add_cue_weight(&mut weights, *token, &cue_kind, wrong_value, -0.25);
                    }
                }
            }

            for trap in semantic_traps_for_example(example) {
                let (trap_tokens, trap_labels) = trap_cache
                    .entry(normalized_surface_key(&trap.text))
                    .or_insert_with(|| {
                        let trap_tokens = cue_tokens(l2, &trap.text)
                            .into_iter()
                            .filter(|token| !global_positive_tokens.contains(token))
                            .collect::<Vec<_>>();
                        let trap_labels =
                            L3SemanticFieldCues::bootstrap_from_text(&trap.text, frames);
                        (trap_tokens, trap_labels)
                    });
                for (cue_kind, cue_value) in trap_labels.as_pairs() {
                    for token in trap_tokens.iter() {
                        add_cue_weight(&mut weights, *token, &cue_kind, &cue_value, 0.75);
                    }
                }
                for (cue_kind, cue_value) in trap_labels.anti_pairs() {
                    for token in trap_tokens.iter() {
                        add_cue_weight(&mut weights, *token, &cue_kind, &cue_value, 1.0);
                    }
                }
            }
        }

        let max_by_kind = max_cue_weight_by_kind(&weights);
        let mut edges = weights
            .into_iter()
            .filter_map(|(key, raw_weight)| {
                let max_weight = *max_by_kind.get(&key.cue_kind)?;
                (max_weight > 0.0).then(|| L3LearnedCueEdge {
                    token: key.token,
                    cue_kind: key.cue_kind,
                    cue_value: key.cue_value,
                    weight: raw_weight / max_weight,
                })
            })
            .collect::<Vec<_>>();
        edges.sort_by(|left, right| {
            left.cue_kind
                .cmp(&right.cue_kind)
                .then_with(|| left.cue_value.cmp(&right.cue_value))
                .then_with(|| left.token.cmp(&right.token))
        });
        let edges_by_token = cue_edges_by_token(&edges);

        Self {
            edges,
            edges_by_token,
            learned: true,
            contrastive: true,
            manual_runtime_rules_used: false,
        }
    }

    pub(super) fn hot_bytes(&self) -> usize {
        self.edges.len() * L3_CUE_EDGE_BYTES
    }

    pub(super) fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub(super) fn infer(
        &self,
        text: &str,
        l2: &L2CenterMemory,
        _frames: &[L3FrameCenter],
        ablate_cues: bool,
    ) -> L3LearnedCueInference {
        self.infer_with_token_mode(text, l2, _frames, ablate_cues, L3CueTokenMode::All)
    }

    pub(super) fn infer_with_token_mode(
        &self,
        text: &str,
        l2: &L2CenterMemory,
        _frames: &[L3FrameCenter],
        ablate_cues: bool,
        token_mode: L3CueTokenMode,
    ) -> L3LearnedCueInference {
        if ablate_cues {
            return L3LearnedCueInference {
                cues: L3SemanticFieldCues::default(),
                min_margin: 0.0,
            };
        }

        let active_tokens = cue_tokens_with_mode(l2, text, token_mode)
            .into_iter()
            .collect::<HashSet<_>>();
        let mut scores: HashMap<(&str, &str), f32> = HashMap::new();
        for token in active_tokens {
            for edge_index in self.edges_by_token.get(&token).into_iter().flatten() {
                let edge = &self.edges[*edge_index];
                *scores
                    .entry((edge.cue_kind.as_str(), edge.cue_value.as_str()))
                    .or_default() += edge.weight;
            }
        }

        let role = best_cue_value(&scores, "role").map(|best| best.value);
        let relation = best_cue_value(&scores, "relation").map(|best| best.value);
        let object_anchor = best_cue_value(&scores, "object_anchor").map(|best| best.value);
        let binding = best_cue_value(&scores, "binding").map(|best| best.value);
        let anti_signatures =
            best_positive_cue_values(&scores, "anti_signature", L3_ANTI_CUE_THRESHOLD);

        let margin = ["role", "relation", "object_anchor", "binding"]
            .into_iter()
            .filter_map(|kind| best_cue_value(&scores, kind).map(|best| best.margin))
            .fold(f32::INFINITY, f32::min);
        let min_margin = if margin == f32::INFINITY { 0.0 } else { margin };

        L3LearnedCueInference {
            cues: L3SemanticFieldCues {
                role,
                relation,
                object_anchor,
                binding,
                anti_signatures,
            },
            min_margin,
        }
    }
}

impl L3SemanticFieldCues {
    pub(super) fn from_schema(schema: &SemanticSchemaKey) -> Self {
        Self {
            role: Some(schema.subject_role.clone()),
            relation: Some(schema.relation.clone()),
            object_anchor: Some(schema.object_role.clone()),
            binding: Some(binding_cue(
                &schema.subject_role,
                &schema.relation,
                &schema.object_role,
            )),
            anti_signatures: Vec::new(),
        }
    }

    pub(super) fn bootstrap_from_text(text: &str, frames: &[L3FrameCenter]) -> Self {
        let tokens = normalized_tokens(text);
        let roles = unique_frame_values(frames, |frame| frame.unknown_role.as_str());
        let anchors = unique_frame_values(frames, |frame| frame.object_anchor.as_str());
        let role = role_cue_from_tokens(&tokens, &roles);
        let object_anchor = anchors
            .into_iter()
            .find(|anchor| tokens.iter().any(|token| token == anchor));
        let relation = relation_cue_from_tokens(&tokens, role.as_deref(), object_anchor.as_deref());
        let binding = match (
            role.as_deref(),
            relation.as_deref(),
            object_anchor.as_deref(),
        ) {
            (Some(role), Some(relation), Some(object_anchor)) => {
                Some(binding_cue(role, relation, object_anchor))
            }
            _ => None,
        };
        let anti_signatures = anti_signatures_from_tokens(
            &tokens,
            frames,
            role.as_deref(),
            relation.as_deref(),
            object_anchor.as_deref(),
        );

        Self {
            role,
            relation,
            object_anchor,
            binding,
            anti_signatures,
        }
    }

    pub(super) fn as_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::with_capacity(4);
        if let Some(role) = self.role.as_deref() {
            pairs.push(("role".to_string(), role.to_string()));
        }
        if let Some(relation) = self.relation.as_deref() {
            pairs.push(("relation".to_string(), relation.to_string()));
        }
        if let Some(object_anchor) = self.object_anchor.as_deref() {
            pairs.push(("object_anchor".to_string(), object_anchor.to_string()));
        }
        if let Some(binding) = self.binding.as_deref() {
            pairs.push(("binding".to_string(), binding.to_string()));
        }
        pairs
    }

    pub(super) fn anti_pairs(&self) -> Vec<(String, String)> {
        self.anti_signatures
            .iter()
            .map(|signature| ("anti_signature".to_string(), signature.to_string()))
            .collect()
    }

    pub(super) fn complete_for(&self, frame: &L3FrameCenter) -> bool {
        self.role.as_deref() == Some(frame.unknown_role.as_str())
            && self.relation.as_deref() == Some(frame.schema.relation.as_str())
            && self.object_anchor.as_deref() == Some(frame.object_anchor.as_str())
            && self.binding.as_deref().is_some_and(|binding| {
                binding
                    == binding_cue(
                        &frame.unknown_role,
                        &frame.schema.relation,
                        &frame.object_anchor,
                    )
            })
    }
}

pub(super) fn cue_universe_from_frames(frames: &[L3FrameCenter]) -> HashMap<String, Vec<String>> {
    let mut universe = HashMap::new();
    universe.insert(
        "role".to_string(),
        unique_frame_values(frames, |frame| frame.unknown_role.as_str()),
    );
    universe.insert(
        "relation".to_string(),
        unique_frame_values(frames, |frame| frame.schema.relation.as_str()),
    );
    universe.insert(
        "object_anchor".to_string(),
        unique_frame_values(frames, |frame| frame.object_anchor.as_str()),
    );
    let mut bindings = frames
        .iter()
        .map(|frame| {
            binding_cue(
                &frame.unknown_role,
                &frame.schema.relation,
                &frame.object_anchor,
            )
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    bindings.sort();
    universe.insert("binding".to_string(), bindings);
    universe
}

fn add_cue_weight(
    weights: &mut HashMap<L3LearnedCueKey, f32>,
    token: u32,
    cue_kind: &str,
    cue_value: &str,
    delta: f32,
) {
    let key = L3LearnedCueKey {
        token,
        cue_kind: cue_kind.to_string(),
        cue_value: cue_value.to_string(),
    };
    *weights.entry(key).or_default() += delta;
}

pub(super) fn cue_edges_by_token(edges: &[L3LearnedCueEdge]) -> HashMap<u32, Vec<usize>> {
    let mut by_token: HashMap<u32, Vec<usize>> = HashMap::new();
    for (index, edge) in edges.iter().enumerate() {
        by_token.entry(edge.token).or_default().push(index);
    }
    by_token
}

fn max_cue_weight_by_kind(weights: &HashMap<L3LearnedCueKey, f32>) -> HashMap<String, f32> {
    let mut max_by_kind = HashMap::new();
    for (key, weight) in weights {
        let current: &mut f32 = max_by_kind.entry(key.cue_kind.clone()).or_default();
        *current = (*current).max(weight.abs());
    }
    max_by_kind
}

fn best_cue_value(scores: &HashMap<(&str, &str), f32>, cue_kind: &str) -> Option<CueChoice> {
    let mut ranked = scores
        .iter()
        .filter(|((kind, _), _)| *kind == cue_kind)
        .map(|((_, value), score)| ((*value).to_string(), *score))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let (value, best_score) = ranked.first()?.clone();
    if best_score <= 0.0 {
        return None;
    }
    let runner_up = ranked.get(1).map_or(0.0, |(_, score)| *score);
    Some(CueChoice {
        value,
        margin: best_score - runner_up,
    })
}

pub(super) fn best_positive_cue_values(
    scores: &HashMap<(&str, &str), f32>,
    cue_kind: &str,
    threshold: f32,
) -> Vec<String> {
    let mut values = scores
        .iter()
        .filter(|((kind, _), score)| *kind == cue_kind && **score >= threshold)
        .map(|((_, value), _)| (*value).to_string())
        .collect::<Vec<_>>();
    values.sort();
    values
}

pub(super) fn binding_cue(role: &str, relation: &str, object_anchor: &str) -> String {
    format!("{role}|{relation}|{object_anchor}")
}

pub(super) fn unique_frame_values(
    frames: &[L3FrameCenter],
    value: impl Fn(&L3FrameCenter) -> &str,
) -> Vec<String> {
    let mut values = frames
        .iter()
        .map(|frame| value(frame).to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    values.sort();
    values
}

pub(super) fn role_cue_from_tokens(tokens: &[String], roles: &[String]) -> Option<String> {
    let is_role = |token: &str| roles.iter().any(|role| role == token);

    if tokens.len() >= 2 && matches!(tokens[0].as_str(), "which" | "find") && is_role(&tokens[1]) {
        return Some(tokens[1].clone());
    }

    for window in tokens.windows(2) {
        if window[0] == "which" && is_role(&window[1]) {
            return Some(window[1].clone());
        }
    }

    tokens
        .first()
        .filter(|token| is_role(token))
        .map(ToString::to_string)
}

pub(super) fn relation_cue_from_tokens(
    tokens: &[String],
    role: Option<&str>,
    object_anchor: Option<&str>,
) -> Option<String> {
    let has = |needle: &str| tokens.iter().any(|token| token == needle);
    if has("provides") || has("provider") {
        return Some("provides_command".to_string());
    }
    if has("executes") || has("runs") || has("executed") || has("executor") {
        return Some("executes_command".to_string());
    }
    if has("enables") || has("enabled") || has("source") {
        return Some("enables_service".to_string());
    }
    if has("installs") || has("owning") || has("owner") {
        return Some("installs_file".to_string());
    }
    match (role, object_anchor) {
        (Some("package"), Some("command")) if has("belongs") || has("for") => {
            Some("provides_command".to_string())
        }
        (Some("package"), Some("file")) if has("belongs") || has("for") => {
            Some("installs_file".to_string())
        }
        (Some("config"), Some("service")) if has("for") => Some("enables_service".to_string()),
        _ => None,
    }
}

pub(super) fn anti_signatures_from_tokens(
    tokens: &[String],
    frames: &[L3FrameCenter],
    role: Option<&str>,
    relation: Option<&str>,
    object_anchor: Option<&str>,
) -> Vec<String> {
    let roles = unique_frame_values(frames, |frame| frame.unknown_role.as_str());
    let anchors = unique_frame_values(frames, |frame| frame.object_anchor.as_str());
    let has = |needle: &str| tokens.iter().any(|token| token == needle);
    let exact_frame = frames.iter().any(|frame| {
        role == Some(frame.unknown_role.as_str())
            && relation == Some(frame.schema.relation.as_str())
            && object_anchor == Some(frame.object_anchor.as_str())
    });
    let mut signatures = Vec::new();

    if tokens.len() >= 2
        && anchors.iter().any(|anchor| anchor == &tokens[1])
        && !roles.iter().any(|role| role == &tokens[1])
    {
        signatures.push("role_swap_surface".to_string());
    }

    if relation.is_some() && (role.is_none() || object_anchor.is_none()) {
        signatures.push("missing_evidence_shape".to_string());
    }

    if role.is_some() && relation.is_some() && object_anchor.is_some() && !exact_frame {
        signatures.push("route_splice_shape".to_string());
    }

    if has("proves")
        || has("implies")
        || has("running")
        || has("active")
        || (has("enabled") && role != Some("config"))
    {
        signatures.push("claim_overreach_shortcut".to_string());
    }

    if tokens
        .windows(2)
        .any(|window| roles.contains(&window[1]) && anchors.contains(&window[0]))
    {
        signatures.push("role_anchor_inversion".to_string());
    }

    signatures.sort();
    signatures.dedup();
    signatures
}
