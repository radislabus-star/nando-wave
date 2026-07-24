//! Source-neutral multi-source discovery contracts.

mod audit;
mod factorizer;
mod identification;
mod join;
mod live_snapshot;
mod marginal;
mod source_neutral_t1;

pub use audit::{
    AuditMassV1, MULTI_SOURCE_EVIDENCE_AUDIT_SCHEMA_V1, MissingEvidenceFieldV1,
    MultiSourceEvidenceAuditV1, MultiSourceShapeAuditV1, PreActionTopologyAuditRowV1,
    RelationEvidenceAuditV1, RequestStructureAuditRowV1, RequestStructureAuditSnapshotV1,
    build_multi_source_evidence_audit_v1,
};
pub use factorizer::{
    CompletedEffectFormV1, FactorizedMultiSourceRowV1, MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1,
    MultiSourceReasonV1, PreActionShapeClassV1, factor_multi_source_row_v1,
};
pub use identification::{
    MULTI_SOURCE_T1_IDENTIFICATION_SCHEMA_V3, MULTI_SOURCE_T1_PROOF_BASIS_SCHEMA_V1,
    MultiSourceT1IdentificationStateV1, MultiSourceT1IdentificationV3, MultiSourceT1ProofBasisV1,
    PassiveT1ProbeContractV1, identify_multi_source_t1_operator_v1,
};
pub use join::{
    BLIND_THEN_REVEAL_JOIN_SCHEMA_V1, BlindThenRevealJoinedTransitionV1, CompletedEffectAtomV1,
    MULTI_SOURCE_JOIN_MAX_ROWS_V1, MultiSourceJoinCensoredReasonV1, MultiSourceJoinLedgerV1,
    MultiSourceJoinReportV1, ObservedTeacherActionRefV1, VerifiedOutcomeReceiptRefV1,
};
pub use live_snapshot::{
    LIVE_MULTI_SOURCE_DISCOVERY_SNAPSHOT_SCHEMA_V3, LiveMultiSourceDiscoveryBlockerV1,
    LiveMultiSourceDiscoverySnapshotV3, build_live_multi_source_discovery_snapshot_v3,
};
pub use marginal::{
    COVERAGE_OPPORTUNITY_MAX_ROWS_V1, COVERAGE_OPPORTUNITY_SNAPSHOT_SCHEMA_V1,
    CoverageOpportunitySnapshotV1, MarginalShapeOpportunityV1,
    build_coverage_opportunity_snapshot_v1,
};

#[cfg(test)]
mod tests;
