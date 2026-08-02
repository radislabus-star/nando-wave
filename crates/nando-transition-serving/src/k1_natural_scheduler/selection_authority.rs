use std::collections::BTreeSet;

use nando_operator_learning::multi_source::{
    K1DeficitSnapshotV1, K1NaturalCandidateQueueV1, K1NaturalCohortCatalogV1,
    build_k1_natural_candidate_queue_with_exclusions_v1,
};

pub(super) fn validate_queue_derivation(
    catalog: &K1NaturalCohortCatalogV1,
    deficit: &K1DeficitSnapshotV1,
    completed_candidate_roots_sha256: &BTreeSet<String>,
    proposed: &K1NaturalCandidateQueueV1,
) -> Result<(), String> {
    let expected = build_k1_natural_candidate_queue_with_exclusions_v1(
        catalog,
        deficit,
        completed_candidate_roots_sha256,
    )
    .map_err(str::to_owned)?;
    if &expected != proposed {
        return Err("k1_candidate_queue_derivation_mismatch".to_owned());
    }
    Ok(())
}
