use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{RelationFrame, canonical_json_sha256};
use serde::{Deserialize, Serialize};

use crate::opportunity::{OpportunityIntentAuditRowV1, ReducibilityClass};

use super::{
    CompletedEffectFormV1, CoverageOpportunitySnapshotV1, MultiSourceJoinLedgerV1,
    MultiSourceJoinReportV1, MultiSourceReasonV1, MultiSourceT1IdentificationStateV1,
    MultiSourceT1IdentificationV3, PreActionShapeClassV1, RequestStructureAuditSnapshotV1,
    build_coverage_opportunity_snapshot_v1, factor_multi_source_row_v1,
    identify_multi_source_t1_operator_v1,
};

pub const LIVE_MULTI_SOURCE_DISCOVERY_SNAPSHOT_SCHEMA_V3: &str =
    "nando.live-multi-source-discovery-snapshot.v3";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveMultiSourceDiscoveryBlockerV1 {
    NoPreActionTopology,
    NoCompletedRelationFrame,
    BlindThenRevealJoinEmpty,
    NoAcceptedJoinedTransition,
    NoUnresolvedMarginalOpportunity,
    NoEligibleT1Cohort,
    T1CandidateGenerationBlocked,
    T1Ambiguous,
    T1AwaitingIndependentFuture,
    T1FutureContradiction,
    T1TransferReady,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FactorizedClassCountV1 {
    pub reason: MultiSourceReasonV1,
    pub pre_action_shape: PreActionShapeClassV1,
    pub completed_effect: CompletedEffectFormV1,
    pub rows: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LiveMultiSourceDiscoverySnapshotV3 {
    pub schema: String,
    pub snapshot_root_sha256: String,
    pub evidence_epoch_sha256: String,
    pub topology_rows: u64,
    pub relation_frames: u64,
    pub join: MultiSourceJoinReportV1,
    pub factorized_rows: u64,
    pub factorized_classes: Vec<FactorizedClassCountV1>,
    pub active_intents: u64,
    pub opportunity: CoverageOpportunitySnapshotV1,
    pub t1_identification: MultiSourceT1IdentificationV3,
    pub blocker: LiveMultiSourceDiscoveryBlockerV1,
    pub identification_ready: bool,
    pub transfer_ready: bool,
    pub authority_ready: bool,
}

#[must_use]
pub fn build_live_multi_source_discovery_snapshot_v3(
    mut opportunities: Vec<OpportunityIntentAuditRowV1>,
    mut requests: RequestStructureAuditSnapshotV1,
    mut frames: Vec<RelationFrame>,
) -> LiveMultiSourceDiscoverySnapshotV3 {
    let relevant_intents = requests
        .topologies
        .iter()
        .map(|row| row.structure.turn_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    opportunities.retain(|row| relevant_intents.contains(row.intent_sha256.as_str()));
    frames.retain(|frame| relevant_intents.contains(frame.client_intent_id_sha256.as_str()));
    opportunities.sort_by(|left, right| left.intent_sha256.cmp(&right.intent_sha256));
    requests.topologies.sort_by(|left, right| {
        left.commit
            .commitment_root_sha256
            .cmp(&right.commit.commitment_root_sha256)
    });
    frames.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });

    let evidence_epoch_sha256 = canonical_json_sha256(&(
        LIVE_MULTI_SOURCE_DISCOVERY_SNAPSHOT_SCHEMA_V3,
        opportunities
            .iter()
            .map(|row| {
                (
                    row.intent_sha256.as_str(),
                    row.input_tokens,
                    row.class,
                    row.authority_observed,
                )
            })
            .collect::<Vec<_>>(),
        requests
            .topologies
            .iter()
            .map(|row| {
                (
                    row.bridge_epoch_sha256.as_str(),
                    row.bridge_sequence,
                    row.record_sha256.as_deref(),
                    row.capture_epoch_sha256.as_deref(),
                    row.capture_event_sha256.as_deref(),
                    row.capture_receipt_sha256.as_deref(),
                    row.captured_at_unix_ms,
                    row.session_lineage_sha256.as_deref(),
                    row.physical_order_proven,
                    &row.structure,
                    &row.commit,
                )
            })
            .collect::<Vec<_>>(),
        frames
            .iter()
            .map(|frame| canonical_json_sha256(frame).expect("relation frame serializes"))
            .collect::<Vec<_>>(),
    ))
    .expect("live multi-source evidence epoch serializes");

    let ledger = MultiSourceJoinLedgerV1::build(&requests.topologies, &frames);
    let factorized = ledger
        .rows()
        .iter()
        .map(factor_multi_source_row_v1)
        .collect::<Vec<_>>();
    let factorized_classes = factorized
        .iter()
        .fold(
            BTreeMap::<
                (
                    MultiSourceReasonV1,
                    PreActionShapeClassV1,
                    CompletedEffectFormV1,
                ),
                u64,
            >::new(),
            |mut counts, row| {
                *counts
                    .entry((row.reason, row.pre_action_shape, row.completed_effect))
                    .or_default() += 1;
                counts
            },
        )
        .into_iter()
        .map(
            |((reason, pre_action_shape, completed_effect), rows)| FactorizedClassCountV1 {
                reason,
                pre_action_shape,
                completed_effect,
                rows,
            },
        )
        .collect::<Vec<_>>();
    let active_intents = opportunities
        .iter()
        .filter(|row| row.authority_observed && row.class == ReducibilityClass::CpuVerified)
        .map(|row| row.intent_sha256.clone())
        .collect::<BTreeSet<_>>();
    let opportunity = build_coverage_opportunity_snapshot_v1(
        &factorized,
        &active_intents,
        evidence_epoch_sha256.clone(),
    );
    let join = ledger.report();
    let t1_identification = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &active_intents,
        evidence_epoch_sha256.clone(),
    );
    let blocker = if requests.topologies.is_empty() {
        LiveMultiSourceDiscoveryBlockerV1::NoPreActionTopology
    } else if frames.is_empty() {
        LiveMultiSourceDiscoveryBlockerV1::NoCompletedRelationFrame
    } else if join.joined_rows == 0 {
        LiveMultiSourceDiscoveryBlockerV1::BlindThenRevealJoinEmpty
    } else if join.accepted_rows == 0 {
        LiveMultiSourceDiscoveryBlockerV1::NoAcceptedJoinedTransition
    } else if opportunity.unresolved.intents == 0 {
        LiveMultiSourceDiscoveryBlockerV1::NoUnresolvedMarginalOpportunity
    } else {
        match t1_identification.state {
            MultiSourceT1IdentificationStateV1::NoEligibleCohort => {
                LiveMultiSourceDiscoveryBlockerV1::NoEligibleT1Cohort
            }
            MultiSourceT1IdentificationStateV1::CandidateGenerationEmpty
            | MultiSourceT1IdentificationStateV1::SearchIncomplete
            | MultiSourceT1IdentificationStateV1::SearchExhausted
            | MultiSourceT1IdentificationStateV1::NoConsistentProgram
            | MultiSourceT1IdentificationStateV1::InvalidEvidence => {
                LiveMultiSourceDiscoveryBlockerV1::T1CandidateGenerationBlocked
            }
            MultiSourceT1IdentificationStateV1::Ambiguous => {
                LiveMultiSourceDiscoveryBlockerV1::T1Ambiguous
            }
            MultiSourceT1IdentificationStateV1::FrozenAwaitingIndependentFuture => {
                LiveMultiSourceDiscoveryBlockerV1::T1AwaitingIndependentFuture
            }
            MultiSourceT1IdentificationStateV1::FutureContradiction => {
                LiveMultiSourceDiscoveryBlockerV1::T1FutureContradiction
            }
            MultiSourceT1IdentificationStateV1::TransferReady => {
                LiveMultiSourceDiscoveryBlockerV1::T1TransferReady
            }
        }
    };
    let identification_ready = t1_identification.candidate_freeze.is_some();
    let transfer_ready =
        t1_identification.state == MultiSourceT1IdentificationStateV1::TransferReady;
    let mut snapshot = LiveMultiSourceDiscoverySnapshotV3 {
        schema: LIVE_MULTI_SOURCE_DISCOVERY_SNAPSHOT_SCHEMA_V3.to_owned(),
        snapshot_root_sha256: String::new(),
        evidence_epoch_sha256,
        topology_rows: u64::try_from(requests.topologies.len()).unwrap_or(u64::MAX),
        relation_frames: u64::try_from(frames.len()).unwrap_or(u64::MAX),
        join,
        factorized_rows: u64::try_from(factorized.len()).unwrap_or(u64::MAX),
        factorized_classes,
        active_intents: u64::try_from(active_intents.len()).unwrap_or(u64::MAX),
        opportunity,
        t1_identification,
        blocker,
        identification_ready,
        transfer_ready,
        authority_ready: false,
    };
    snapshot.snapshot_root_sha256 = snapshot.expected_root();
    snapshot
}

impl LiveMultiSourceDiscoverySnapshotV3 {
    #[must_use]
    pub fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            LIVE_MULTI_SOURCE_DISCOVERY_SNAPSHOT_SCHEMA_V3,
            self.evidence_epoch_sha256.as_str(),
            self.topology_rows,
            self.relation_frames,
            &self.join,
            self.factorized_rows,
            &self.factorized_classes,
            self.active_intents,
            &self.opportunity,
            &self.t1_identification,
            self.blocker,
            self.identification_ready,
            self.transfer_ready,
            false,
        ))
        .expect("live multi-source snapshot serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == LIVE_MULTI_SOURCE_DISCOVERY_SNAPSHOT_SCHEMA_V3
            && !self.authority_ready
            && self.t1_identification.validate()
            && self.identification_ready == self.t1_identification.candidate_freeze.is_some()
            && self.transfer_ready
                == (self.t1_identification.state
                    == MultiSourceT1IdentificationStateV1::TransferReady)
            && self.opportunity.validate()
            && self.snapshot_root_sha256 == self.expected_root()
    }
}
