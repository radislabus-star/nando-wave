//! Source-neutral multi-source discovery contracts.

mod audit;
mod factorizer;
mod join;
mod marginal;

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
pub use join::{
    BLIND_THEN_REVEAL_JOIN_SCHEMA_V1, BlindThenRevealJoinedTransitionV1, CompletedEffectAtomV1,
    MULTI_SOURCE_JOIN_MAX_ROWS_V1, MultiSourceJoinCensoredReasonV1, MultiSourceJoinLedgerV1,
    MultiSourceJoinReportV1, ObservedTeacherActionRefV1, VerifiedOutcomeReceiptRefV1,
};
pub use marginal::{
    COVERAGE_OPPORTUNITY_MAX_ROWS_V1, COVERAGE_OPPORTUNITY_SNAPSHOT_SCHEMA_V1,
    CoverageOpportunitySnapshotV1, MarginalShapeOpportunityV1,
    build_coverage_opportunity_snapshot_v1,
};

#[cfg(test)]
mod tests;
