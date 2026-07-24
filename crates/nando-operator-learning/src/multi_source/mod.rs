//! Source-neutral multi-source discovery contracts.

mod audit;

pub use audit::{
    AuditMassV1, MULTI_SOURCE_EVIDENCE_AUDIT_SCHEMA_V1, MissingEvidenceFieldV1,
    MultiSourceEvidenceAuditV1, MultiSourceShapeAuditV1, RelationEvidenceAuditV1,
    RequestStructureAuditRowV1, RequestStructureAuditSnapshotV1,
    build_multi_source_evidence_audit_v1,
};
