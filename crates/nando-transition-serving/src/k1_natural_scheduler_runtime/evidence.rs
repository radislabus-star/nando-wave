use super::*;

mod identification;
mod topology;

pub(super) use identification::*;
pub(super) use topology::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct EvidenceBinding {
    pub(super) row: K1NaturalEvidenceRowV1,
    pub(super) joined: BlindThenRevealJoinedTransitionV1,
}
