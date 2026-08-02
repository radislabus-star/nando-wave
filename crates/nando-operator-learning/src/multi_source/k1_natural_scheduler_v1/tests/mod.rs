use super::*;

mod future;
mod lifecycle;
mod probe;
mod recency;
mod selection;

const GENERATOR_SCHEMA: &str = "nando.operator-blind-version-space-generator.v1";

fn root(value: u64) -> String {
    format!("{value:064x}")
}

fn prediction_contract(
    probe_root_sha256: &str,
    observable_difference_root_sha256: &str,
    class_partitions: Vec<(String, String)>,
) -> (String, Vec<K1ProbeClassPredictionV1>) {
    let mut predictions = class_partitions
        .into_iter()
        .map(
            |(class_id, outcome_partition_root_sha256)| K1ProbeClassPredictionV1 {
                class_id,
                outcome_partition_root_sha256,
            },
        )
        .collect::<Vec<_>>();
    predictions.sort();
    let root = nando_operator_kernel::canonical_json_sha256(&(
        "nando.multi-source-t1-precommitted-probe-predictions.v1",
        probe_root_sha256,
        observable_difference_root_sha256,
        &predictions,
    ))
    .expect("prediction root");
    (root, predictions)
}

#[allow(clippy::too_many_arguments)]
fn evidence_row(
    index: u64,
    structural: u64,
    topology: u64,
    consequence: K1ConsequenceTypeV1,
    class: K1NaturalEvidenceClassV1,
    lineage: u64,
    settled: bool,
    verified: bool,
) -> K1NaturalEvidenceRowV1 {
    K1NaturalEvidenceRowV1::seal(
        root(10_000 + index),
        root(structural),
        root(topology),
        root(50_000 + structural),
        root(lineage),
        consequence,
        class,
        index,
        1_000,
        1_000 + index,
        settled,
        verified,
        false,
    )
    .expect("evidence row")
}

fn deficit(known: Vec<K1ConsequenceTypeV1>) -> K1DeficitSnapshotV1 {
    K1DeficitSnapshotV1::seal(
        1,
        root(1),
        root(2),
        1,
        1,
        1,
        1,
        0,
        3,
        3,
        2,
        vec![root(3)],
        vec![root(4)],
        known,
        false,
    )
    .expect("deficit")
}

fn catalog(rows: &[K1NaturalEvidenceRowV1]) -> K1NaturalCohortCatalogV1 {
    build_k1_natural_cohort_catalog_v1(rows, root(5), root(6), GENERATOR_SCHEMA.to_owned())
        .expect("catalog")
}

fn catalog_watermark(catalog: &K1NaturalCohortCatalogV1) -> u64 {
    catalog
        .candidates
        .iter()
        .map(|candidate| candidate.last_capture_sequence)
        .max()
        .unwrap_or(0)
}

fn ready_rows() -> Vec<K1NaturalEvidenceRowV1> {
    (1..=8)
        .map(|index| {
            evidence_row(
                index,
                100,
                200,
                K1ConsequenceTypeV1::Scalar,
                K1NaturalEvidenceClassV1::NaturalLive,
                if index <= 4 { 300 } else { 301 },
                true,
                index <= 2,
            )
        })
        .collect()
}

fn candidate_freeze(generation_sequence: u64) -> K1NaturalCandidateFreezeV1 {
    let rows = ready_rows();
    let catalog = catalog(&rows);
    let deficit = deficit(Vec::new());
    let queue =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, catalog_watermark(&catalog))
            .expect("queue");
    let queue_row = queue.first_readiness_pass().expect("ready queue row");
    let candidate = catalog
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_root_sha256 == queue_row.candidate_root_sha256)
        .expect("candidate");
    K1NaturalCandidateFreezeV1::seal(
        generation_sequence,
        &catalog,
        &deficit,
        &queue,
        candidate,
        queue_row.score.clone(),
        "nando.k1-operator-blind-scheduler.v1".to_owned(),
        K1GenerationBudgetV1 {
            maximum_support_rows: 64,
            maximum_probe_rounds: 4,
            maximum_probe_cost_units: 100,
            maximum_generation_seconds: 3_600,
        },
        candidate.last_capture_sequence,
        candidate.last_capture_sequence,
        1_700_000_000,
    )
    .expect("candidate freeze")
}

fn frozen_generation() -> (
    K1NaturalCandidateFreezeV1,
    K1IdentificationFreezeV1,
    Vec<String>,
) {
    let candidate_freeze = candidate_freeze(1);
    let classes = vec![root(700), root(701), root(702)];
    let identification = K1IdentificationFreezeV1::seal(
        &candidate_freeze,
        root(703),
        GENERATOR_SCHEMA.to_owned(),
        classes.clone(),
        root(704),
        root(705),
        "nando.k1-probe-prediction.v1".to_owned(),
    )
    .expect("identification freeze");
    (candidate_freeze, identification, classes)
}
