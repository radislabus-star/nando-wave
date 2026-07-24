use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use nando_operator_kernel::{LearningRequestStructureV2, PreActionTopologyCommitV1};

use crate::opportunity::{OpportunityIntentAuditRowV1, ReducibilityClass};

pub const MULTI_SOURCE_EVIDENCE_AUDIT_SCHEMA_V1: &str = "nando.multi-source-evidence-audit.v1";

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuditMassV1 {
    pub intents: u64,
    pub input_tokens: u64,
}

impl AuditMassV1 {
    fn add(&mut self, input_tokens: u64) {
        self.intents = self.intents.saturating_add(1);
        self.input_tokens = self.input_tokens.saturating_add(input_tokens);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestStructureAuditRowV1 {
    pub intent_sha256: String,
    pub request_phase_atom_count: usize,
    pub capability_atom_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestStructureAuditSnapshotV1 {
    pub rows: Vec<RequestStructureAuditRowV1>,
    pub topologies: Vec<PreActionTopologyAuditRowV1>,
    pub evictions: u64,
    pub stored_turns: u64,
    pub stored_topologies: u64,
    pub provider_bound_by_construction: bool,
    pub pre_action_context_persisted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreActionTopologyAuditRowV1 {
    pub bridge_epoch_sha256: String,
    pub bridge_sequence: Option<u64>,
    pub record_sha256: Option<String>,
    pub capture_epoch_sha256: Option<String>,
    pub capture_event_sha256: Option<String>,
    pub capture_receipt_sha256: Option<String>,
    pub captured_at_unix_ms: Option<u64>,
    pub session_lineage_sha256: Option<String>,
    pub physical_order_proven: bool,
    pub structure: LearningRequestStructureV2,
    pub commit: PreActionTopologyCommitV1,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RelationEvidenceAuditV1 {
    pub frames: usize,
    pub positive: usize,
    pub negative: usize,
    pub unlabeled: usize,
    pub observation_roles: BTreeSet<(u16, String)>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MultiSourceShapeAuditV1 {
    pub shape_sha256: String,
    pub reason_code: String,
    pub request_structure: bool,
    pub relation_frames: bool,
    pub provider_bound_identity: bool,
    pub request_phase_atom_bucket: String,
    pub capability_atom_bucket: String,
    pub relation_frame_bucket: String,
    pub observation_role_bucket: String,
    pub verifier_state: String,
    pub mass: AuditMassV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MissingEvidenceFieldV1 {
    pub field: String,
    pub mass: AuditMassV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MultiSourceEvidenceAuditV1 {
    pub schema: String,
    pub opportunity_checkpoint_sha256: String,
    pub request_learning_checkpoint_sha256: String,
    pub relation_frames_sha256: String,
    pub authority_ready: bool,
    pub total: AuditMassV1,
    pub exact_reason: AuditMassV1,
    pub request_structure_joined: AuditMassV1,
    pub relation_frames_joined: AuditMassV1,
    pub provider_bound_identity: AuditMassV1,
    pub reason_identity_holds: bool,
    pub shape_identity_holds: bool,
    pub request_learning_stored_turns: u64,
    pub request_learning_stored_topologies: u64,
    pub request_learning_evictions: u64,
    pub relation_frame_rows_scanned: u64,
    pub relation_frame_parse_errors: u64,
    pub reason_counts: BTreeMap<String, AuditMassV1>,
    pub shapes: Vec<MultiSourceShapeAuditV1>,
    pub missing_fields: Vec<MissingEvidenceFieldV1>,
    pub verdict: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ShapeKey {
    reason_code: String,
    request_structure: bool,
    relation_frames: bool,
    provider_bound_identity: bool,
    request_phase_atom_bucket: String,
    capability_atom_bucket: String,
    relation_frame_bucket: String,
    observation_role_bucket: String,
    verifier_state: String,
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_multi_source_evidence_audit_v1(
    opportunities: Vec<OpportunityIntentAuditRowV1>,
    request_snapshot: RequestStructureAuditSnapshotV1,
    relations: BTreeMap<String, RelationEvidenceAuditV1>,
    opportunity_checkpoint_sha256: String,
    request_learning_checkpoint_sha256: String,
    relation_frames_sha256: String,
    relation_frame_rows_scanned: u64,
    relation_frame_parse_errors: u64,
) -> MultiSourceEvidenceAuditV1 {
    let requests = request_snapshot
        .rows
        .iter()
        .map(|row| (row.intent_sha256.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut total = AuditMassV1::default();
    let exact_reason = AuditMassV1::default();
    let mut request_structure_joined = AuditMassV1::default();
    let mut relation_frames_joined = AuditMassV1::default();
    let mut provider_bound_identity = AuditMassV1::default();
    let mut reason_counts = BTreeMap::<String, AuditMassV1>::new();
    let mut shapes = BTreeMap::<ShapeKey, AuditMassV1>::new();
    let mut missing = BTreeMap::<String, AuditMassV1>::new();

    for opportunity in opportunities.into_iter().filter(|row| {
        row.authority_observed && row.class == ReducibilityClass::UnexploredMultiSource
    }) {
        total.add(opportunity.input_tokens);
        let request = requests.get(opportunity.intent_sha256.as_str()).copied();
        let relation = relations.get(&opportunity.intent_sha256);
        let reason_code = "reason_not_persisted_by_opportunity_board_v3".to_owned();
        reason_counts
            .entry(reason_code.clone())
            .or_default()
            .add(opportunity.input_tokens);
        missing
            .entry("decidability_reason_not_persisted_in_opportunity_checkpoint".to_owned())
            .or_default()
            .add(opportunity.input_tokens);

        if request.is_some() {
            request_structure_joined.add(opportunity.input_tokens);
            if request_snapshot.provider_bound_by_construction {
                provider_bound_identity.add(opportunity.input_tokens);
            }
        } else {
            missing
                .entry("request_structure_not_in_current_bounded_checkpoint".to_owned())
                .or_default()
                .add(opportunity.input_tokens);
        }
        if relation.is_some() {
            relation_frames_joined.add(opportunity.input_tokens);
        } else {
            missing
                .entry("completed_relation_frame_not_joined_by_turn_intent".to_owned())
                .or_default()
                .add(opportunity.input_tokens);
        }
        if !request_snapshot.pre_action_context_persisted {
            missing
                .entry("pre_action_context_atoms_not_persisted_in_request_checkpoint".to_owned())
                .or_default()
                .add(opportunity.input_tokens);
        }

        let key = ShapeKey {
            reason_code,
            request_structure: request.is_some(),
            relation_frames: relation.is_some(),
            provider_bound_identity: request.is_some()
                && request_snapshot.provider_bound_by_construction,
            request_phase_atom_bucket: count_bucket(
                request.map_or(0, |row| row.request_phase_atom_count),
            ),
            capability_atom_bucket: count_bucket(
                request.map_or(0, |row| row.capability_atom_count),
            ),
            relation_frame_bucket: count_bucket(relation.map_or(0, |row| row.frames)),
            observation_role_bucket: count_bucket(
                relation.map_or(0, |row| row.observation_roles.len()),
            ),
            verifier_state: relation.map_or_else(
                || "missing".to_owned(),
                |summary| {
                    match (
                        summary.positive > 0,
                        summary.negative > 0,
                        summary.unlabeled > 0,
                    ) {
                        (true, false, false) => "positive_only",
                        (false, true, false) => "negative_only",
                        (false, false, true) => "unlabeled_only",
                        _ => "mixed",
                    }
                    .to_owned()
                },
            ),
        };
        shapes.entry(key).or_default().add(opportunity.input_tokens);
    }

    let reason_mass = sum_mass(reason_counts.values());
    let shape_mass = sum_mass(shapes.values());
    let reason_identity_holds = reason_mass == total;
    let shape_identity_holds = shape_mass == total;
    let evidence_complete = total.intents > 0
        && exact_reason == total
        && request_structure_joined == total
        && relation_frames_joined == total
        && provider_bound_identity == total
        && request_snapshot.pre_action_context_persisted
        && relation_frame_parse_errors == 0;

    MultiSourceEvidenceAuditV1 {
        schema: MULTI_SOURCE_EVIDENCE_AUDIT_SCHEMA_V1.to_owned(),
        opportunity_checkpoint_sha256,
        request_learning_checkpoint_sha256,
        relation_frames_sha256,
        authority_ready: false,
        total,
        exact_reason,
        request_structure_joined,
        relation_frames_joined,
        provider_bound_identity,
        reason_identity_holds,
        shape_identity_holds,
        request_learning_stored_turns: request_snapshot.stored_turns,
        request_learning_stored_topologies: request_snapshot.stored_topologies,
        request_learning_evictions: request_snapshot.evictions,
        relation_frame_rows_scanned,
        relation_frame_parse_errors,
        reason_counts,
        shapes: shapes
            .into_iter()
            .map(|(key, mass)| MultiSourceShapeAuditV1 {
                shape_sha256: sha256_bytes(
                    &serde_json::to_vec(&key).expect("shape key serializes"),
                ),
                reason_code: key.reason_code,
                request_structure: key.request_structure,
                relation_frames: key.relation_frames,
                provider_bound_identity: key.provider_bound_identity,
                request_phase_atom_bucket: key.request_phase_atom_bucket,
                capability_atom_bucket: key.capability_atom_bucket,
                relation_frame_bucket: key.relation_frame_bucket,
                observation_role_bucket: key.observation_role_bucket,
                verifier_state: key.verifier_state,
                mass,
            })
            .collect(),
        missing_fields: missing
            .into_iter()
            .map(|(field, mass)| MissingEvidenceFieldV1 { field, mass })
            .collect(),
        verdict: if evidence_complete {
            "EXISTING_EVIDENCE_SUFFICIENT"
        } else {
            "MULTI_SOURCE_JOIN_INSUFFICIENT"
        }
        .to_owned(),
    }
}

fn sum_mass<'a>(rows: impl IntoIterator<Item = &'a AuditMassV1>) -> AuditMassV1 {
    rows.into_iter()
        .fold(AuditMassV1::default(), |mut sum, row| {
            sum.intents = sum.intents.saturating_add(row.intents);
            sum.input_tokens = sum.input_tokens.saturating_add(row.input_tokens);
            sum
        })
}

fn count_bucket(value: usize) -> String {
    match value {
        0 => "0",
        1 => "1",
        2 => "2",
        3..=4 => "3_4",
        5..=8 => "5_8",
        9..=16 => "9_16",
        _ => "17_plus",
    }
    .to_owned()
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opportunity(intent: &str, tokens: u64) -> OpportunityIntentAuditRowV1 {
        OpportunityIntentAuditRowV1 {
            intent_sha256: intent.repeat(64),
            input_tokens: tokens,
            class: ReducibilityClass::UnexploredMultiSource,
            verifier_available: false,
            observed_at_unix: 1,
            authority_observed: true,
        }
    }

    #[test]
    fn every_intent_enters_exactly_one_shape_even_when_join_is_missing() {
        let report = build_multi_source_evidence_audit_v1(
            vec![opportunity("a", 10), opportunity("b", 20)],
            RequestStructureAuditSnapshotV1 {
                rows: vec![RequestStructureAuditRowV1 {
                    intent_sha256: "a".repeat(64),
                    request_phase_atom_count: 2,
                    capability_atom_count: 1,
                }],
                topologies: Vec::new(),
                evictions: 7,
                stored_turns: 1,
                stored_topologies: 0,
                provider_bound_by_construction: true,
                pre_action_context_persisted: false,
            },
            BTreeMap::new(),
            "0".repeat(64),
            "1".repeat(64),
            "2".repeat(64),
            0,
            0,
        );
        assert_eq!(
            report.total,
            AuditMassV1 {
                intents: 2,
                input_tokens: 30
            }
        );
        assert!(report.reason_identity_holds);
        assert!(report.shape_identity_holds);
        assert_eq!(report.verdict, "MULTI_SOURCE_JOIN_INSUFFICIENT");
        assert!(!report.authority_ready);
    }

    #[test]
    fn report_is_stably_ordered_without_raw_payloads() {
        let report = build_multi_source_evidence_audit_v1(
            vec![opportunity("b", 20), opportunity("a", 10)],
            RequestStructureAuditSnapshotV1 {
                rows: Vec::new(),
                topologies: Vec::new(),
                evictions: 0,
                stored_turns: 0,
                stored_topologies: 0,
                provider_bound_by_construction: true,
                pre_action_context_persisted: false,
            },
            BTreeMap::new(),
            "0".repeat(64),
            "1".repeat(64),
            "2".repeat(64),
            0,
            0,
        );
        let json = serde_json::to_string(&report).expect("report serializes");
        assert!(!json.contains(&"a".repeat(64)));
        assert!(!json.contains(&"b".repeat(64)));
        assert!(json.contains("reason_not_persisted_by_opportunity_board_v3"));
    }
}
