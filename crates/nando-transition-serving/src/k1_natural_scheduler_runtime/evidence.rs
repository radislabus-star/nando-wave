use super::*;

mod identification;
mod topology;

pub(super) use identification::*;
pub(super) use topology::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct EvidenceBinding {
    pub(super) row: K1NaturalEvidenceRowV1,
    joined: Option<Box<BlindThenRevealJoinedTransitionV1>>,
    pub(super) completed_frame_root_sha256: String,
    pub(super) topology_commitment_root_sha256: String,
}

impl EvidenceBinding {
    pub(super) fn joined(&self) -> &BlindThenRevealJoinedTransitionV1 {
        self.joined
            .as_deref()
            .expect("eligible K1 evidence retains its joined payload")
    }

    pub(super) fn payload_retained(&self) -> bool {
        self.joined.is_some()
    }

    pub(super) fn join_root_sha256(&self) -> &str {
        self.row.evidence_root_sha256.as_str()
    }
}
