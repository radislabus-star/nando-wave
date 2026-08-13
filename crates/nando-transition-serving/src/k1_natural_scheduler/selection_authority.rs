use std::collections::BTreeSet;

use nando_operator_learning::multi_source::{
    K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3, K1DeficitSnapshotV1, K1NaturalCandidateQueueV1,
    K1NaturalCohortCatalogV1, K1SchedulerLedgerV1,
    build_k1_natural_candidate_queue_with_exclusions_v1,
};

pub(super) fn validate_queue_derivation(
    ledger: &K1SchedulerLedgerV1,
    catalog: &K1NaturalCohortCatalogV1,
    deficit: &K1DeficitSnapshotV1,
    completed_candidate_roots_sha256: &BTreeSet<String>,
    _discovery_basis_root_sha256: &str,
    contract_watermark: u64,
    proposed: &K1NaturalCandidateQueueV1,
) -> Result<(), String> {
    let _ = ledger;
    if proposed.schema == K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3 {
        return Err("k1_candidate_queue_coarse_family_schema_rejected".to_owned());
    }
    let expected = build_k1_natural_candidate_queue_with_exclusions_v1(
        catalog,
        deficit,
        completed_candidate_roots_sha256,
        contract_watermark,
    )
    .map_err(str::to_owned)?;
    if &expected != proposed {
        return Err("k1_candidate_queue_derivation_mismatch".to_owned());
    }
    Ok(())
}
