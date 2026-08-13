mod cohort;
mod evidence;
mod freeze;
mod motif;
mod queue;
mod readiness;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};

pub use cohort::{
    K1MotifCandidateSupportV1, K1MotifDispositionSummaryV1, K1NaturalCohortCandidateV1,
    K1NaturalCohortCatalogV1, ValidatedK1NaturalCohortCatalogV1,
};
pub use evidence::{K1ConsequenceTypeV1, K1NaturalEvidenceClassV1, K1NaturalEvidenceRowV1};
pub use freeze::{K1GenerationBudgetV1, K1IdentificationFreezeV1, K1NaturalCandidateFreezeV1};
pub use motif::{K1MotifSourceDispositionClassV1, K1MotifSourceDispositionV1};
pub use queue::{
    K1CandidateScoreV1, K1DeficitSnapshotV1, K1NaturalCandidateQueueRowV1,
    K1NaturalCandidateQueueV1,
};
pub use readiness::K1CandidateReadinessV1;

pub const K1_DEFICIT_SNAPSHOT_SCHEMA_V1: &str = "nando.k1-deficit-snapshot.v1";
pub const K1_NATURAL_COHORT_CATALOG_SCHEMA_V1: &str = "nando.k1-natural-cohort-catalog.v1";
pub const K1_NATURAL_COHORT_CATALOG_SCHEMA_V2: &str = "nando.k1-natural-cohort-catalog.v2";
pub const K1_MOTIF_DISPOSITION_SUMMARY_SCHEMA_V1: &str = "nando.k1-motif-disposition-summary.v1";
pub const K1_MOTIF_CANDIDATE_SUPPORT_SCHEMA_V1: &str = "nando.k1-motif-candidate-support.v1";
pub const K1_MOTIF_SOURCE_DISPOSITION_SCHEMA_V1: &str = "nando.k1-motif-source-disposition.v1";
pub const K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V1: &str = "nando.k1-natural-cohort-candidate.v1";
pub const K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V2: &str = "nando.k1-natural-cohort-candidate.v2";
pub const K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V3: &str = "nando.k1-natural-cohort-candidate.v3";
pub const K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4: &str = "nando.k1-natural-cohort-candidate.v4";
pub const K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V1: &str = "nando.k1-natural-candidate-queue.v1";
pub const K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2: &str = "nando.k1-natural-candidate-queue.v2";
pub const K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3: &str = "nando.k1-natural-candidate-queue.v3";
pub const K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4: &str = "nando.k1-natural-candidate-queue.v4";
pub const K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1: &str = "nando.k1-natural-candidate-freeze.v1";
pub const K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2: &str = "nando.k1-natural-candidate-freeze.v2";
pub const K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3: &str = "nando.k1-natural-candidate-freeze.v3";
pub const K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4: &str = "nando.k1-natural-candidate-freeze.v4";
pub const K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5: &str = "nando.k1-natural-candidate-freeze.v5";
pub const K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6: &str = "nando.k1-natural-candidate-freeze.v6";
pub const K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V7: &str = "nando.k1-natural-candidate-freeze.v7";
pub const K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8: &str = "nando.k1-natural-candidate-freeze.v8";
pub const K1_NATURAL_EVIDENCE_ROW_SCHEMA_V1: &str = "nando.k1-natural-evidence-row.v1";
pub const K1_NATURAL_EVIDENCE_ROW_SCHEMA_V2: &str = "nando.k1-natural-evidence-row.v2";
pub const K1_NATURAL_EVIDENCE_ROW_SCHEMA_V3: &str = "nando.k1-natural-evidence-row.v3";
pub const K1_NATURAL_EVIDENCE_ROW_SCHEMA_V4: &str = "nando.k1-natural-evidence-row.v4";
pub const K1_IDENTIFICATION_FREEZE_SCHEMA_V1: &str = "nando.k1-identification-freeze.v1";
pub const K1_PROBE_ROUND_RECEIPT_SCHEMA_V1: &str = "nando.k1-probe-round-receipt.v1";
pub const K1_SCHEDULER_SCHEMA_V1: &str = "nando.k1-natural-scheduler-ledger.v1";

pub const K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1: u64 = 8;
pub const K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1: u64 = 2;
pub const K1_CANDIDATE_READINESS_MIN_LINEAGES_V1: u64 = 2;
pub(super) const K1_NATURAL_CANDIDATE_MAX_ROWS_V1: usize = 256;
pub(super) const K1_VERSION_SPACE_MAX_CLASSES_V1: usize = 4_096;

pub(super) fn version_space_root(classes: &[String]) -> Result<String, &'static str> {
    canonical_json_sha256(&("nando.k1-monotonic-version-space.v1", classes))
}

pub(super) fn canonical_roots(roots: &mut Vec<String>) -> Result<(), &'static str> {
    if roots.iter().any(|root| !valid_nonzero_sha256(root)) {
        return Err("k1_root_invalid");
    }
    roots.sort();
    roots.dedup();
    Ok(())
}

pub(super) fn canonical_root_slice(roots: &[String]) -> bool {
    roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}

pub(super) fn strict_roots<'a>(roots: impl Iterator<Item = &'a str>) -> bool {
    let roots = roots.collect::<Vec<_>>();
    roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}

pub(super) fn strict_values<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}
