use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{MultiSourceExtractionStatusV1, RelationFrame, canonical_json_sha256};
use serde::{Deserialize, Serialize};

use super::{
    PreActionTopologyAuditRowV1, RequestStructureAuditSnapshotV1, TransportBindingFailureV1,
    TransportBindingLedgerV1, TransportTerminalReceiptV1,
    source_neutral_t1::{
        SelectedObservationEvidenceV1, enumerate_source_neutral_t1_candidates,
        selected_observation_evidence_v1,
    },
};

pub const MS3_FAILURE_CORPUS_SCHEMA_V1: &str = "nando.ms3-failure-corpus.v1";
pub const MS3_FAILURE_CORPUS_MAX_ROWS_V1: usize = 16_384;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3FailureDispositionV1 {
    CensoredTopology,
    TransportReceiptMissing,
    TransportBindingUnresolved,
    MissingCompletedObservation,
    SelectedObservationMissing,
    SelectedObservationAmbiguous,
    ObservationallyEquivalentHypotheses,
    ConflictingHypotheses,
    UniqueHypothesis,
    CandidateLanguageUnexpressible,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3FailureCorpusRowV1 {
    pub topology_commitment_root_sha256: String,
    pub turn_intent_id_sha256: String,
    pub request_event_id_sha256: String,
    pub transport_binding_roots_sha256: Vec<String>,
    pub completed_frame_roots_sha256: Vec<String>,
    pub disposition: Ms3FailureDispositionV1,
    pub detail: String,
    pub candidate_programs: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3FailureCorpusV1 {
    pub schema: String,
    pub corpus_root_sha256: String,
    pub evidence_epoch_sha256: String,
    pub topology_denominator: usize,
    pub completed_frame_denominator: usize,
    pub terminal_receipt_denominator: usize,
    pub disposition_counts: BTreeMap<Ms3FailureDispositionV1, usize>,
    pub rows: Vec<Ms3FailureCorpusRowV1>,
    pub accounting_identity_holds: bool,
    pub post_hoc_selection_allowed: bool,
    pub authority_ready: bool,
}

#[must_use]
pub fn build_ms3_failure_corpus_v1(
    mut requests: RequestStructureAuditSnapshotV1,
    mut frames: Vec<RelationFrame>,
    mut terminals: Vec<TransportTerminalReceiptV1>,
) -> Ms3FailureCorpusV1 {
    requests.topologies.sort_by(|left, right| {
        left.commit
            .commitment_root_sha256
            .cmp(&right.commit.commitment_root_sha256)
    });
    requests.topologies.truncate(MS3_FAILURE_CORPUS_MAX_ROWS_V1);
    frames.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    terminals.sort_by(|left, right| {
        left.request_event_id_sha256
            .cmp(&right.request_event_id_sha256)
            .then_with(|| left.receipt_root_sha256.cmp(&right.receipt_root_sha256))
    });

    let ledger = TransportBindingLedgerV1::build(&requests.topologies, &frames, &terminals);
    let frame_by_root = frames
        .iter()
        .filter_map(|frame| canonical_json_sha256(frame).ok().map(|root| (root, frame)))
        .collect::<BTreeMap<_, _>>();

    let rows = requests
        .topologies
        .iter()
        .map(|topology| classify_topology(topology, &ledger, &frame_by_root))
        .collect::<Vec<_>>();
    let disposition_counts = rows.iter().fold(
        BTreeMap::<Ms3FailureDispositionV1, usize>::new(),
        |mut counts, row| {
            *counts.entry(row.disposition).or_default() += 1;
            counts
        },
    );
    let evidence_epoch_sha256 = canonical_json_sha256(&(
        MS3_FAILURE_CORPUS_SCHEMA_V1,
        requests
            .topologies
            .iter()
            .map(|row| row.commit.commitment_root_sha256.as_str())
            .collect::<Vec<_>>(),
        frame_by_root.keys().map(String::as_str).collect::<Vec<_>>(),
        terminals
            .iter()
            .map(|terminal| terminal.receipt_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .expect("MS3 evidence epoch serializes");
    let mut corpus = Ms3FailureCorpusV1 {
        schema: MS3_FAILURE_CORPUS_SCHEMA_V1.to_owned(),
        corpus_root_sha256: String::new(),
        evidence_epoch_sha256,
        topology_denominator: requests.topologies.len(),
        completed_frame_denominator: frames.len(),
        terminal_receipt_denominator: terminals.len(),
        disposition_counts,
        rows,
        accounting_identity_holds: false,
        post_hoc_selection_allowed: false,
        authority_ready: false,
    };
    corpus.accounting_identity_holds = corpus.disposition_counts.values().sum::<usize>()
        == corpus.topology_denominator
        && corpus.rows.len() == corpus.topology_denominator;
    corpus.corpus_root_sha256 = corpus.expected_root();
    corpus
}

fn classify_topology(
    topology: &PreActionTopologyAuditRowV1,
    ledger: &TransportBindingLedgerV1,
    frame_by_root: &BTreeMap<String, &RelationFrame>,
) -> Ms3FailureCorpusRowV1 {
    let topology_root = topology.commit.commitment_root_sha256.clone();
    if !topology_is_eligible(topology) {
        return row(
            topology,
            Vec::new(),
            Vec::new(),
            Ms3FailureDispositionV1::CensoredTopology,
            "pre_action_topology_not_eligible",
            0,
        );
    }
    let bound = ledger.bound_for_topology(&topology_root);
    if bound.is_empty() {
        let (disposition, detail) = match ledger.failure_for_topology(&topology_root) {
            Some(TransportBindingFailureV1::TerminalReceiptMissing) => (
                Ms3FailureDispositionV1::TransportReceiptMissing,
                "transport_terminal_receipt_missing",
            ),
            Some(TransportBindingFailureV1::CompletedFrameMissing) => (
                Ms3FailureDispositionV1::MissingCompletedObservation,
                "request_completed_without_owned_action_frame",
            ),
            Some(_) => (
                Ms3FailureDispositionV1::TransportBindingUnresolved,
                "request_action_transport_binding_unresolved",
            ),
            None => (
                Ms3FailureDispositionV1::TransportBindingUnresolved,
                "transport_binding_not_accounted",
            ),
        };
        return row(topology, Vec::new(), Vec::new(), disposition, detail, 0);
    }

    let binding_roots = bound
        .iter()
        .map(|row| row.binding.binding_root_sha256.clone())
        .collect::<Vec<_>>();
    let frame_roots = bound
        .iter()
        .map(|row| row.joined.completed_frame_root_sha256.clone())
        .collect::<Vec<_>>();
    let mut programs_by_root = BTreeMap::new();
    let mut common_program_roots: Option<BTreeSet<String>> = None;
    let mut candidate_blockers = BTreeMap::<&'static str, usize>::new();
    let mut selected_missing = false;
    let mut selected_ambiguous = false;
    for joined in bound {
        let Some(frame) = frame_by_root
            .get(&joined.joined.completed_frame_root_sha256)
            .copied()
        else {
            return row(
                topology,
                binding_roots,
                frame_roots,
                Ms3FailureDispositionV1::TransportBindingUnresolved,
                "transport_bound_frame_root_missing",
                0,
            );
        };
        match selected_observation_evidence_v1(frame) {
            SelectedObservationEvidenceV1::Missing => selected_missing = true,
            SelectedObservationEvidenceV1::Ambiguous => selected_ambiguous = true,
            SelectedObservationEvidenceV1::Present => {
                match enumerate_source_neutral_t1_candidates(&joined.joined, frame) {
                    Ok(programs) => {
                        let roots = programs.keys().cloned().collect::<BTreeSet<_>>();
                        common_program_roots = Some(match common_program_roots.take() {
                            Some(common) => common.intersection(&roots).cloned().collect(),
                            None => roots,
                        });
                        for (root, program) in programs {
                            programs_by_root.insert(root, program);
                        }
                    }
                    Err(blocker) => {
                        *candidate_blockers.entry(blocker).or_default() += 1;
                    }
                }
            }
        }
    }
    if selected_missing {
        return row(
            topology,
            binding_roots,
            frame_roots,
            Ms3FailureDispositionV1::SelectedObservationMissing,
            "selected_observation_missing",
            0,
        );
    }
    if selected_ambiguous {
        return row(
            topology,
            binding_roots,
            frame_roots,
            Ms3FailureDispositionV1::SelectedObservationAmbiguous,
            "selected_observation_ambiguous",
            0,
        );
    }
    let common_program_roots = common_program_roots.unwrap_or_default();
    if common_program_roots.is_empty() && !programs_by_root.is_empty() {
        return row(
            topology,
            binding_roots,
            frame_roots,
            Ms3FailureDispositionV1::ConflictingHypotheses,
            "no_hypothesis_replays_every_bound_transition",
            0,
        );
    }
    match common_program_roots.len() {
        0 => {
            let detail = candidate_blockers
                .into_iter()
                .max_by(|(left_blocker, left_count), (right_blocker, right_count)| {
                    left_count
                        .cmp(right_count)
                        .then_with(|| right_blocker.cmp(left_blocker))
                })
                .map_or("candidate_generation_empty", |(blocker, _)| blocker);
            row(
                topology,
                binding_roots,
                frame_roots,
                Ms3FailureDispositionV1::CandidateLanguageUnexpressible,
                detail,
                0,
            )
        }
        1 => row(
            topology,
            binding_roots,
            frame_roots,
            Ms3FailureDispositionV1::UniqueHypothesis,
            "one_source_neutral_hypothesis",
            1,
        ),
        count => row(
            topology,
            binding_roots,
            frame_roots,
            Ms3FailureDispositionV1::ObservationallyEquivalentHypotheses,
            "multiple_hypotheses_match_current_observation",
            count,
        ),
    }
}

fn topology_is_eligible(row: &PreActionTopologyAuditRowV1) -> bool {
    row.physical_order_proven
        && row.structure.provider_bound_turn_identity
        && !row.structure.request_event_id_sha256.is_empty()
        && !matches!(
            row.structure.topology.extraction_status,
            MultiSourceExtractionStatusV1::Censored { .. }
        )
        && row.structure.validate().is_ok()
        && row.commit.validate().is_ok()
        && row.bridge_sequence.is_some_and(|value| value > 0)
        && row.record_sha256.as_deref().is_some_and(valid_root)
        && row
            .session_lineage_sha256
            .as_deref()
            .is_some_and(valid_root)
        && row.captured_at_unix_ms.is_some_and(|value| value > 0)
}

fn valid_root(value: &str) -> bool {
    nando_operator_kernel::valid_nonzero_sha256(value)
}

fn row(
    topology: &PreActionTopologyAuditRowV1,
    transport_binding_roots_sha256: Vec<String>,
    completed_frame_roots_sha256: Vec<String>,
    disposition: Ms3FailureDispositionV1,
    detail: impl Into<String>,
    candidate_programs: usize,
) -> Ms3FailureCorpusRowV1 {
    Ms3FailureCorpusRowV1 {
        topology_commitment_root_sha256: topology.commit.commitment_root_sha256.clone(),
        turn_intent_id_sha256: topology.structure.turn_intent_id_sha256.clone(),
        request_event_id_sha256: topology.structure.request_event_id_sha256.clone(),
        transport_binding_roots_sha256,
        completed_frame_roots_sha256,
        disposition,
        detail: detail.into(),
        candidate_programs,
    }
}

impl Ms3FailureCorpusV1 {
    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            MS3_FAILURE_CORPUS_SCHEMA_V1,
            self.evidence_epoch_sha256.as_str(),
            self.topology_denominator,
            self.completed_frame_denominator,
            self.terminal_receipt_denominator,
            &self.disposition_counts,
            &self.rows,
            self.accounting_identity_holds,
            false,
            false,
        ))
        .expect("MS3 failure corpus serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MS3_FAILURE_CORPUS_SCHEMA_V1
            && self.topology_denominator <= MS3_FAILURE_CORPUS_MAX_ROWS_V1
            && self.accounting_identity_holds
            && !self.post_hoc_selection_allowed
            && !self.authority_ready
            && self.rows.len() == self.topology_denominator
            && self.disposition_counts.values().sum::<usize>() == self.topology_denominator
            && self.rows.windows(2).all(|pair| {
                pair[0].topology_commitment_root_sha256 < pair[1].topology_commitment_root_sha256
            })
            && self.rows.iter().all(|row| {
                valid_root(&row.topology_commitment_root_sha256)
                    && valid_root(&row.turn_intent_id_sha256)
                    && valid_root(&row.request_event_id_sha256)
                    && row
                        .transport_binding_roots_sha256
                        .iter()
                        .all(|root| valid_root(root))
                    && row
                        .completed_frame_roots_sha256
                        .iter()
                        .all(|root| valid_root(root))
            })
            && nando_operator_kernel::valid_nonzero_sha256(&self.evidence_epoch_sha256)
            && self.corpus_root_sha256 == self.expected_root()
    }
}
