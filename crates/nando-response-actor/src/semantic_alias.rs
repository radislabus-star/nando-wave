use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{EffectGraphBuilder, EffectGraphCompleteness, TeacherTransition};

pub const SEMANTIC_ALIAS_GRAPH_SCHEMA_V1: &str = "nando.semantic-alias-graph.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticAliasState {
    Candidate,
    SupportProven,
    FutureProven,
    Rejected,
    Revoked,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticEffectEvidence {
    pub rows_seen: u64,
    pub complete_rows: u64,
    pub ambiguous_rows: u64,
    pub insufficient_rows: u64,
    pub over_budget_rows: u64,
    pub invalid_rows: u64,
    pub complete_graph_rows: BTreeMap<String, u64>,
    pub retained_receipts: BTreeSet<String>,
}

impl SemanticEffectEvidence {
    #[must_use]
    pub fn unique_complete_digest(&self) -> Option<&str> {
        (self.complete_graph_rows.len() == 1)
            .then(|| self.complete_graph_rows.keys().next().map(String::as_str))
            .flatten()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticAliasEdge {
    pub edge_sha256: String,
    pub left_teacher_signature_sha256: String,
    pub right_teacher_signature_sha256: String,
    pub effect_graph_sha256: String,
    pub state: SemanticAliasState,
    pub support_receipts: Vec<String>,
    pub future_receipts: Vec<String>,
    pub parity_receipts: Vec<String>,
    pub counterexamples: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wave_proof_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticAliasReport {
    pub rows_seen: u64,
    pub complete_rows: u64,
    pub ambiguous_rows: u64,
    pub insufficient_rows: u64,
    pub over_budget_rows: u64,
    pub invalid_rows: u64,
    pub exact_teacher_pools: usize,
    pub effect_classes: usize,
    pub candidate_edges: usize,
    #[serde(default)]
    pub actionable_candidate_edges: usize,
    #[serde(default)]
    pub blocked_candidate_edges: usize,
    #[serde(default)]
    pub candidate_blockers: BTreeMap<String, usize>,
    pub support_proven_edges: usize,
    pub future_proven_edges: usize,
    pub rejected_edges: usize,
    pub revoked_edges: usize,
    pub accounting_complete: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticAliasGraph {
    pub schema: String,
    pools: BTreeMap<String, SemanticEffectEvidence>,
    edges: BTreeMap<String, SemanticAliasEdge>,
    report: SemanticAliasReport,
}

impl Default for SemanticAliasGraph {
    fn default() -> Self {
        Self {
            schema: SEMANTIC_ALIAS_GRAPH_SCHEMA_V1.to_owned(),
            pools: BTreeMap::new(),
            edges: BTreeMap::new(),
            report: SemanticAliasReport::default(),
        }
    }
}

impl SemanticAliasGraph {
    pub fn observe_transition(&mut self, transition: &TeacherTransition) {
        let signature = transition.outcome.action.signature_sha256.clone();
        let graph = EffectGraphBuilder::default().build(transition);
        let previous_digest = self
            .pools
            .get(&signature)
            .and_then(SemanticEffectEvidence::unique_complete_digest)
            .map(str::to_owned);
        {
            let evidence = self.pools.entry(signature.clone()).or_default();
            evidence.rows_seen = evidence.rows_seen.saturating_add(1);
            match graph.completeness {
                EffectGraphCompleteness::Complete => {
                    evidence.complete_rows = evidence.complete_rows.saturating_add(1);
                    if let Some(digest) = graph.canonical_sha256 {
                        *evidence.complete_graph_rows.entry(digest).or_default() += 1;
                        retain_receipt(
                            &mut evidence.retained_receipts,
                            transition.before.frame_id_sha256.clone(),
                        );
                    } else {
                        evidence.invalid_rows = evidence.invalid_rows.saturating_add(1);
                    }
                }
                EffectGraphCompleteness::Ambiguous => {
                    evidence.ambiguous_rows = evidence.ambiguous_rows.saturating_add(1);
                }
                EffectGraphCompleteness::InsufficientEvidence => {
                    evidence.insufficient_rows = evidence.insufficient_rows.saturating_add(1);
                }
                EffectGraphCompleteness::OverBudget => {
                    evidence.over_budget_rows = evidence.over_budget_rows.saturating_add(1);
                }
                EffectGraphCompleteness::Invalid => {
                    evidence.invalid_rows = evidence.invalid_rows.saturating_add(1);
                }
            }
        }
        let current_digest = self
            .pools
            .get(&signature)
            .and_then(SemanticEffectEvidence::unique_complete_digest)
            .map(str::to_owned);
        if previous_digest != current_digest {
            self.rebuild_candidate_forest();
        }
        self.refresh_report();
    }

    #[must_use]
    pub fn evidence(&self, teacher_signature_sha256: &str) -> Option<&SemanticEffectEvidence> {
        self.pools.get(teacher_signature_sha256)
    }

    #[must_use]
    pub fn edge(&self, edge_sha256: &str) -> Option<&SemanticAliasEdge> {
        self.edges.get(edge_sha256)
    }

    #[must_use]
    pub fn edges(&self) -> impl Iterator<Item = &SemanticAliasEdge> {
        self.edges.values()
    }

    #[must_use]
    pub fn candidate_edges(&self) -> impl Iterator<Item = &SemanticAliasEdge> {
        self.edges
            .values()
            .filter(|edge| edge.state == SemanticAliasState::Candidate && edge.blocker.is_none())
    }

    #[must_use]
    pub fn proven_components(&self) -> Vec<(String, BTreeSet<String>)> {
        let mut by_effect = BTreeMap::<String, Vec<&SemanticAliasEdge>>::new();
        for edge in self.edges.values().filter(|edge| {
            matches!(
                edge.state,
                SemanticAliasState::SupportProven | SemanticAliasState::FutureProven
            )
        }) {
            by_effect
                .entry(edge.effect_graph_sha256.clone())
                .or_default()
                .push(edge);
        }
        let mut output = Vec::new();
        for (effect, edges) in by_effect {
            let mut components = Vec::<BTreeSet<String>>::new();
            for edge in edges {
                let left = edge.left_teacher_signature_sha256.clone();
                let right = edge.right_teacher_signature_sha256.clone();
                let matching = components
                    .iter()
                    .enumerate()
                    .filter(|(_, component)| {
                        component.contains(&left) || component.contains(&right)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                match matching.as_slice() {
                    [] => components.push(BTreeSet::from([left, right])),
                    [index] => {
                        components[*index].insert(left);
                        components[*index].insert(right);
                    }
                    [first, second, ..] => {
                        let (keep, merge) = ((*first).min(*second), (*first).max(*second));
                        let merged = components.remove(merge);
                        components[keep].extend(merged);
                        components[keep].insert(left);
                        components[keep].insert(right);
                    }
                }
            }
            output.extend(
                components
                    .into_iter()
                    .filter(|component| component.len() >= 2)
                    .map(|component| (effect.clone(), component)),
            );
        }
        output.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        output
    }

    #[must_use]
    pub fn proven_edges_for_members(&self, members: &BTreeSet<String>) -> Vec<SemanticAliasEdge> {
        self.edges
            .values()
            .filter(|edge| {
                matches!(
                    edge.state,
                    SemanticAliasState::SupportProven | SemanticAliasState::FutureProven
                ) && members.contains(&edge.left_teacher_signature_sha256)
                    && members.contains(&edge.right_teacher_signature_sha256)
            })
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn future_proven_partners(&self, teacher_signature_sha256: &str) -> BTreeSet<String> {
        self.edges
            .values()
            .filter(|edge| edge.state == SemanticAliasState::FutureProven)
            .filter_map(|edge| {
                if edge.left_teacher_signature_sha256 == teacher_signature_sha256 {
                    Some(edge.right_teacher_signature_sha256.clone())
                } else if edge.right_teacher_signature_sha256 == teacher_signature_sha256 {
                    Some(edge.left_teacher_signature_sha256.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn mark_support_proven(
        &mut self,
        edge_sha256: &str,
        support_receipts: Vec<String>,
        parity_receipts: Vec<String>,
        wave_proof_sha256: String,
    ) -> Result<(), &'static str> {
        let edge = self
            .edges
            .get_mut(edge_sha256)
            .ok_or("semantic_alias_edge_missing")?;
        if edge.state != SemanticAliasState::Candidate
            || support_receipts.is_empty()
            || parity_receipts.is_empty()
            || wave_proof_sha256.is_empty()
            || !edge.counterexamples.is_empty()
        {
            return Err("semantic_alias_support_proof_incomplete");
        }
        edge.support_receipts = sorted_unique(support_receipts);
        edge.parity_receipts = sorted_unique(parity_receipts);
        edge.wave_proof_sha256 = Some(wave_proof_sha256);
        edge.blocker = None;
        edge.state = SemanticAliasState::SupportProven;
        self.refresh_report();
        Ok(())
    }

    pub fn mark_future_proven(
        &mut self,
        edge_sha256: &str,
        future_receipts: Vec<String>,
    ) -> Result<(), &'static str> {
        let edge = self
            .edges
            .get_mut(edge_sha256)
            .ok_or("semantic_alias_edge_missing")?;
        if edge.state != SemanticAliasState::SupportProven || future_receipts.is_empty() {
            return Err("semantic_alias_future_proof_incomplete");
        }
        edge.future_receipts = sorted_unique(future_receipts);
        edge.blocker = None;
        edge.state = SemanticAliasState::FutureProven;
        self.refresh_report();
        Ok(())
    }

    pub fn reject(
        &mut self,
        edge_sha256: &str,
        counterexample: String,
    ) -> Result<(), &'static str> {
        self.terminate_edge(edge_sha256, counterexample, SemanticAliasState::Rejected)
    }

    pub fn revoke(
        &mut self,
        edge_sha256: &str,
        counterexample: String,
    ) -> Result<(), &'static str> {
        self.terminate_edge(edge_sha256, counterexample, SemanticAliasState::Revoked)
    }

    pub fn set_candidate_blocker(
        &mut self,
        edge_sha256: &str,
        blocker: String,
    ) -> Result<(), &'static str> {
        let edge = self
            .edges
            .get_mut(edge_sha256)
            .ok_or("semantic_alias_edge_missing")?;
        if edge.state != SemanticAliasState::Candidate || blocker.is_empty() {
            return Err("semantic_alias_candidate_blocker_invalid");
        }
        edge.blocker = Some(blocker);
        self.refresh_report();
        Ok(())
    }

    /// New verified evidence may complete an exact winner or parity set. Only
    /// candidate blockers touching that teacher pool are retryable; rejected
    /// and revoked edges remain terminal.
    pub fn clear_candidate_blockers_for_member(&mut self, teacher_signature_sha256: &str) -> usize {
        let mut cleared = 0_usize;
        for edge in self.edges.values_mut().filter(|edge| {
            edge.state == SemanticAliasState::Candidate
                && edge.blocker.is_some()
                && (edge.left_teacher_signature_sha256 == teacher_signature_sha256
                    || edge.right_teacher_signature_sha256 == teacher_signature_sha256)
        }) {
            edge.blocker = None;
            cleared = cleared.saturating_add(1);
        }
        if cleared > 0 {
            self.refresh_report();
        }
        cleared
    }

    #[must_use]
    pub fn report(&self) -> SemanticAliasReport {
        self.report.clone()
    }

    fn terminate_edge(
        &mut self,
        edge_sha256: &str,
        counterexample: String,
        terminal: SemanticAliasState,
    ) -> Result<(), &'static str> {
        let edge = self
            .edges
            .get_mut(edge_sha256)
            .ok_or("semantic_alias_edge_missing")?;
        if counterexample.is_empty() {
            return Err("semantic_alias_counterexample_missing");
        }
        edge.blocker = Some(counterexample.clone());
        edge.counterexamples.push(counterexample);
        edge.counterexamples.sort();
        edge.counterexamples.dedup();
        edge.state = terminal;
        self.refresh_report();
        Ok(())
    }

    fn rebuild_candidate_forest(&mut self) {
        let stale_candidates = self
            .edges
            .iter()
            .filter(|(_, edge)| edge.state == SemanticAliasState::Candidate)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in stale_candidates {
            self.edges.remove(&key);
        }

        let mut classes = BTreeMap::<String, Vec<String>>::new();
        for (signature, evidence) in &self.pools {
            if let Some(digest) = evidence.unique_complete_digest() {
                classes
                    .entry(digest.to_owned())
                    .or_default()
                    .push(signature.clone());
            }
        }
        for (digest, mut signatures) in classes {
            signatures.sort();
            for pair in signatures.windows(2) {
                let edge = candidate_edge(&pair[0], &pair[1], &digest);
                self.edges.entry(edge.edge_sha256.clone()).or_insert(edge);
            }
        }
    }

    fn refresh_report(&mut self) {
        let rows_seen = self.pools.values().map(|pool| pool.rows_seen).sum::<u64>();
        let complete_rows = self
            .pools
            .values()
            .map(|pool| pool.complete_rows)
            .sum::<u64>();
        let ambiguous_rows = self
            .pools
            .values()
            .map(|pool| pool.ambiguous_rows)
            .sum::<u64>();
        let insufficient_rows = self
            .pools
            .values()
            .map(|pool| pool.insufficient_rows)
            .sum::<u64>();
        let over_budget_rows = self
            .pools
            .values()
            .map(|pool| pool.over_budget_rows)
            .sum::<u64>();
        let invalid_rows = self
            .pools
            .values()
            .map(|pool| pool.invalid_rows)
            .sum::<u64>();
        let mut effect_classes = BTreeSet::new();
        for pool in self.pools.values() {
            effect_classes.extend(pool.complete_graph_rows.keys().cloned());
        }
        let count = |state| {
            self.edges
                .values()
                .filter(|edge| edge.state == state)
                .count()
        };
        let candidate_blockers = self
            .edges
            .values()
            .filter(|edge| edge.state == SemanticAliasState::Candidate)
            .filter_map(|edge| edge.blocker.as_deref())
            .fold(BTreeMap::<String, usize>::new(), |mut counts, blocker| {
                *counts.entry(blocker.to_owned()).or_default() += 1;
                counts
            });
        let blocked_candidate_edges = candidate_blockers.values().sum();
        let candidate_edges = count(SemanticAliasState::Candidate);
        self.report = SemanticAliasReport {
            rows_seen,
            complete_rows,
            ambiguous_rows,
            insufficient_rows,
            over_budget_rows,
            invalid_rows,
            exact_teacher_pools: self.pools.len(),
            effect_classes: effect_classes.len(),
            candidate_edges,
            actionable_candidate_edges: candidate_edges.saturating_sub(blocked_candidate_edges),
            blocked_candidate_edges,
            candidate_blockers,
            support_proven_edges: count(SemanticAliasState::SupportProven),
            future_proven_edges: count(SemanticAliasState::FutureProven),
            rejected_edges: count(SemanticAliasState::Rejected),
            revoked_edges: count(SemanticAliasState::Revoked),
            accounting_complete: rows_seen
                == complete_rows
                    .saturating_add(ambiguous_rows)
                    .saturating_add(insufficient_rows)
                    .saturating_add(over_budget_rows)
                    .saturating_add(invalid_rows),
        };
    }
}

fn candidate_edge(left: &str, right: &str, effect_graph_sha256: &str) -> SemanticAliasEdge {
    let (left, right) = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    let edge_sha256 = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(
                SEMANTIC_ALIAS_GRAPH_SCHEMA_V1,
                left,
                right,
                effect_graph_sha256,
            ))
            .expect("semantic alias edge serializes"),
        )
    );
    SemanticAliasEdge {
        edge_sha256,
        left_teacher_signature_sha256: left.to_owned(),
        right_teacher_signature_sha256: right.to_owned(),
        effect_graph_sha256: effect_graph_sha256.to_owned(),
        state: SemanticAliasState::Candidate,
        support_receipts: Vec::new(),
        future_receipts: Vec::new(),
        parity_receipts: Vec::new(),
        counterexamples: Vec::new(),
        wave_proof_sha256: None,
        blocker: None,
    }
}

fn retain_receipt(receipts: &mut BTreeSet<String>, receipt: String) {
    const MAX_RECEIPTS_PER_POOL: usize = 256;
    receipts.insert(receipt);
    while receipts.len() > MAX_RECEIPTS_PER_POOL {
        let Some(first) = receipts.first().cloned() else {
            break;
        };
        receipts.remove(&first);
    }
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        AtomSource, AtomValueType, RelationAtom, RuntimeFrame, RuntimeParityCase, TeacherActionAst,
        TeacherOutcome, TeacherVerifierEvidence,
    };

    fn transition(signature: &str, function: &str, effect: &str) -> TeacherTransition {
        let value_sha256 = format!("{:x}", Sha256::digest(effect));
        TeacherTransition {
            schema: "nando.teacher-transition.v1".to_owned(),
            before: RuntimeFrame {
                schema: "nando.runtime-frame.v1".to_owned(),
                frame_id_sha256: format!("frame-{signature}"),
                event_id_sha256: format!("event-{signature}"),
                client_intent_id_sha256: format!("intent-{signature}"),
                session_id_sha256: format!("session-{signature}"),
                observed_at_unix_nanos: 1,
                extractor_version: "test".to_owned(),
                atoms: vec![RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type: AtomValueType::Identifier,
                    source: AtomSource::Observation,
                    value_sha256: value_sha256.clone(),
                }],
                evidence_ref_sha256: format!("evidence-{signature}"),
            },
            outcome: TeacherOutcome {
                schema: "nando.teacher-outcome.v1".to_owned(),
                action: TeacherActionAst {
                    signature_sha256: signature.to_owned(),
                    action_symbol: function.to_owned(),
                    atoms: vec![
                        RelationAtom::TypedSlot {
                            slot_id: 2,
                            value_type: AtomValueType::Identifier,
                            source: AtomSource::Action,
                            value_sha256,
                        },
                        RelationAtom::ActionFunction {
                            value: function.to_owned(),
                        },
                        RelationAtom::ActionRoleArgument {
                            name: format!("arg-{function}"),
                            slot_id: 2,
                            value_type: Some(AtomValueType::Identifier),
                        },
                    ],
                },
                verifier: TeacherVerifierEvidence {
                    accepted: true,
                    evidence_ref_sha256: format!("receipt-{signature}"),
                    output_digest_sha256: format!("output-{signature}"),
                },
                completed_at_unix_nanos: 2,
            },
            economics: None,
            runtime_parity_case: Some(RuntimeParityCase {
                evidence_ref_sha256: format!("parity-{signature}"),
                capture_receipt: None,
                request_text: String::new(),
                provider_payload: json!({"effect": effect}),
                expected_response: "ok".to_owned(),
            }),
        }
    }

    #[test]
    fn equal_complete_effects_create_name_free_candidate_edge() {
        let mut graph = SemanticAliasGraph::default();
        graph.observe_transition(&transition("a", "wait", "same"));
        graph.observe_transition(&transition("b", "write_stdin", "same"));
        let edges = graph.candidate_edges().collect::<Vec<_>>();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].state, SemanticAliasState::Candidate);
        assert!(graph.report().accounting_complete);
    }

    #[test]
    fn different_effects_do_not_alias() {
        let mut graph = SemanticAliasGraph::default();
        graph.observe_transition(&transition("a", "same-name", "left"));
        let mut projected = transition("b", "same-name", "right");
        projected
            .outcome
            .action
            .atoms
            .push(RelationAtom::ActionJsonResultProjection);
        graph.observe_transition(&projected);
        assert_eq!(graph.candidate_edges().count(), 0);
    }

    #[test]
    fn future_authority_requires_support_and_future_receipts() {
        let mut graph = SemanticAliasGraph::default();
        graph.observe_transition(&transition("a", "x", "same"));
        graph.observe_transition(&transition("b", "y", "same"));
        let edge = graph
            .candidate_edges()
            .next()
            .expect("candidate")
            .edge_sha256
            .clone();
        assert!(
            graph
                .mark_future_proven(&edge, vec!["future".to_owned()])
                .is_err()
        );
        graph
            .mark_support_proven(
                &edge,
                vec!["support".to_owned()],
                vec!["parity".to_owned()],
                "wave".to_owned(),
            )
            .expect("support proof");
        graph
            .mark_future_proven(&edge, vec!["future".to_owned()])
            .expect("future proof");
        assert_eq!(graph.report().future_proven_edges, 1);
    }
}
