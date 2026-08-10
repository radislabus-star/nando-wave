use super::*;
use crate::multi_source::{
    SourceNeutralTopologyMotifV1, source_neutral_topology_motif_config_root_v1,
    source_neutral_topology_motifs_v1,
};
use nando_operator_kernel::{
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
};

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
        root(9_999),
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

fn motif_role(local_role_id: u16) -> MultiSourceRoleNodeV1 {
    MultiSourceRoleNodeV1 {
        local_role_id,
        source_ordinal: local_role_id,
        value_ordinal: 0,
        type_class: MultiSourceTypeClassV1::String,
        container_class: MultiSourceContainerClassV1::Scalar,
        cardinality_class: MultiSourceCardinalityClassV1::One,
        temporal_class: MultiSourceTemporalClassV1::Historical,
        depth_bucket: 1,
        structural_flags: 0,
    }
}

fn motif_topology(role_count: u16, edges: &[(u16, u16)]) -> PreActionMultiSourceTopologyV1 {
    let mut relations = edges
        .iter()
        .map(
            |(source_role_id, target_role_id)| MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::Precedes,
                source_role_id: *source_role_id,
                target_role_id: *target_role_id,
            },
        )
        .collect::<Vec<_>>();
    relations.sort();
    PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: 1,
        output_part_count: 1,
        roles: (0..role_count).map(motif_role).collect(),
        role_witnesses: Vec::new(),
        relations,
    }
}

fn exact_motif(
    topology: &PreActionMultiSourceTopologyV1,
    role_count: u8,
    relation_count: u8,
) -> SourceNeutralTopologyMotifV1 {
    source_neutral_topology_motifs_v1(topology)
        .expect("motif enumeration")
        .into_iter()
        .find(|motif| motif.role_count == role_count && motif.relation_count == relation_count)
        .expect("exact motif")
}

#[allow(clippy::too_many_arguments)]
fn motif_evidence_row(
    index: u64,
    motif: &SourceNeutralTopologyMotifV1,
    semantic_signature: u64,
    lineage: u64,
    input_tokens: u64,
) -> K1NaturalEvidenceRowV1 {
    K1NaturalEvidenceRowV1::seal_motif_v4(
        root(70_000 + index),
        root(9_999),
        motif.embeddings[0].ambient_topology_root_sha256.clone(),
        motif,
        root(50_000 + semantic_signature),
        root(lineage),
        K1ConsequenceTypeV1::Collection,
        index,
        10_000,
        input_tokens,
        true,
        index % 4 == 1,
        false,
    )
    .expect("motif evidence row")
}

fn motif_supports(
    rows: &[K1NaturalEvidenceRowV1],
    overflow_by_motif: &[(String, u64)],
) -> Vec<K1MotifCandidateSupportV1> {
    let mut groups = std::collections::BTreeMap::<
        (String, String, String, K1ConsequenceTypeV1),
        Vec<&K1NaturalEvidenceRowV1>,
    >::new();
    for row in rows {
        groups
            .entry((
                row.capture_generation_root_sha256.clone(),
                row.motif_root_sha256.clone(),
                row.semantic_novelty_signature_root_sha256.clone(),
                row.consequence_type,
            ))
            .or_default()
            .push(row);
    }
    let overflow_by_motif = overflow_by_motif
        .iter()
        .cloned()
        .collect::<std::collections::BTreeMap<_, _>>();
    groups
        .into_iter()
        .map(
            |((capture_generation, motif_root, semantic_signature, consequence), mut rows)| {
                rows.sort_by(|left, right| {
                    left.capture_sequence
                        .cmp(&right.capture_sequence)
                        .then_with(|| left.row_root_sha256.cmp(&right.row_root_sha256))
                });
                let retained_manifest = nando_operator_kernel::canonical_json_sha256(&(
                    "nando.k1-motif-evidence-manifest.v1",
                    rows.iter()
                        .map(|row| row.row_root_sha256.as_str())
                        .collect::<Vec<_>>(),
                ))
                .expect("retained manifest");
                let overflow = overflow_by_motif.get(&motif_root).copied().unwrap_or(0);
                let overflow_manifest = nando_operator_kernel::canonical_json_sha256(&(
                    "nando.k1-motif-test-overflow-manifest.v1",
                    motif_root.as_str(),
                    overflow,
                ))
                .expect("overflow manifest");
                K1MotifCandidateSupportV1::seal(
                    capture_generation,
                    motif_root,
                    semantic_signature,
                    consequence,
                    u64::try_from(rows.len()).expect("retained rows"),
                    retained_manifest,
                    overflow,
                    overflow_manifest,
                )
                .expect("motif support")
            },
        )
        .collect()
}

fn motif_disposition(
    rows: &[K1NaturalEvidenceRowV1],
    supports: &[K1MotifCandidateSupportV1],
) -> K1MotifDispositionSummaryV1 {
    let retained_occurrences = u64::try_from(rows.len()).expect("retained occurrences");
    let support_overflow_occurrences = supports
        .iter()
        .map(|support| support.overflow_occurrences)
        .sum();
    let support_manifest = nando_operator_kernel::canonical_json_sha256(&(
        "nando.k1-motif-candidate-support-manifest.v1",
        supports
            .iter()
            .map(|support| support.support_root_sha256.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
    ))
    .expect("support manifest");
    let empty_manifest = |label: &str| {
        nando_operator_kernel::canonical_json_sha256(&(label, Vec::<String>::new()))
            .expect("empty disposition manifest")
    };
    K1MotifDispositionSummaryV1::seal(
        source_neutral_topology_motif_config_root_v1().expect("motif config root"),
        retained_occurrences,
        retained_occurrences,
        retained_occurrences,
        support_overflow_occurrences,
        support_manifest,
        0,
        empty_manifest("budget"),
        0,
        empty_manifest("empty"),
        0,
        empty_manifest("invalid"),
        0,
        empty_manifest("fixture"),
        0,
        empty_manifest("safety"),
        nando_operator_kernel::canonical_json_sha256(&(
            "nando.k1-motif-test-source-disposition.v1",
            rows.iter()
                .map(|row| row.evidence_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))
        .expect("source disposition manifest"),
    )
    .expect("motif disposition")
}

fn motif_catalog(
    rows: &[K1NaturalEvidenceRowV1],
    overflow_by_motif: &[(String, u64)],
) -> K1NaturalCohortCatalogV1 {
    let supports = motif_supports(rows, overflow_by_motif);
    build_k1_motif_cohort_catalog_v1(
        rows,
        &supports,
        root(5),
        root(6),
        "nando.operator-blind-version-space-generator.v4".to_owned(),
        motif_disposition(rows, &supports),
    )
    .expect("motif catalog")
}

fn candidate_freeze(generation_sequence: u64) -> K1NaturalCandidateFreezeV1 {
    candidate_freeze_for_basis(generation_sequence, root(706))
}

fn candidate_freeze_for_basis(
    generation_sequence: u64,
    discovery_basis_root_sha256: String,
) -> K1NaturalCandidateFreezeV1 {
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
        discovery_basis_root_sha256,
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

fn assert_legacy_freeze_bytes_exclude_v6_fields(bytes: &[u8]) {
    let json = std::str::from_utf8(bytes).expect("freeze json");
    for field in [
        "motif_disposition_summary_root_sha256",
        "motif_enumeration_config_root_sha256",
        "complete_topology_manifest_root_sha256",
        "motif_embedding_manifest_root_sha256",
        "motif_support_overflow_occurrences",
        "motif_support_overflow_manifest_root_sha256",
    ] {
        assert!(!json.contains(field), "legacy bytes contain {field}");
    }
}

#[test]
fn legacy_v1_candidate_freeze_remains_decodable_and_root_stable() {
    let mut freeze = candidate_freeze(1);
    freeze.schema = K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1.to_owned();
    freeze.capture_generation_root_sha256.clear();
    freeze.discovery_basis_root_sha256.clear();
    freeze.freeze_root_sha256 = freeze.expected_root().expect("legacy freeze root");
    let bytes = serde_json::to_vec(&freeze).expect("encode legacy freeze");
    assert_legacy_freeze_bytes_exclude_v6_fields(&bytes);
    assert_eq!(
        freeze.freeze_root_sha256,
        "4f4ab8f4bad2a93b3859fdcaa8a814276843e002adfe80efe4a59d07c1f77489"
    );
    assert_eq!(
        nando_operator_kernel::sha256_bytes(&bytes),
        "b3051c43b5757e73bc894862999f1c7b1d4efb758089fa726a7d5247adfec8ae"
    );
    let restored: K1NaturalCandidateFreezeV1 =
        serde_json::from_slice(&bytes).expect("decode legacy freeze");
    restored.validate().expect("validate legacy freeze");
    assert_eq!(restored.freeze_root_sha256, freeze.freeze_root_sha256);
    assert_eq!(
        serde_json::to_vec(&restored).expect("re-encode legacy freeze"),
        bytes
    );
}

#[test]
fn legacy_v2_candidate_freeze_remains_decodable_and_root_stable() {
    let mut freeze = candidate_freeze(2);
    freeze.schema = K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2.to_owned();
    freeze.discovery_basis_root_sha256.clear();
    freeze.freeze_root_sha256 = freeze.expected_root().expect("v2 freeze root");
    let bytes = serde_json::to_vec(&freeze).expect("encode v2 freeze");
    assert_legacy_freeze_bytes_exclude_v6_fields(&bytes);
    assert_eq!(
        freeze.freeze_root_sha256,
        "057e96a59ed66774e268f5c882d94a22c88e1c2a8971277c7fd6bb7bffaaf537"
    );
    assert_eq!(
        nando_operator_kernel::sha256_bytes(&bytes),
        "bce2451878bee37eb251a2295d94c17476a0b3b27f176a6aed505f8cafd491a6"
    );
    let restored: K1NaturalCandidateFreezeV1 =
        serde_json::from_slice(&bytes).expect("decode v2 freeze");
    restored.validate().expect("validate v2 freeze");
    assert_eq!(restored.freeze_root_sha256, freeze.freeze_root_sha256);
    assert_eq!(
        serde_json::to_vec(&restored).expect("re-encode v2 freeze"),
        bytes
    );
}

#[test]
fn legacy_v3_candidate_freeze_remains_decodable_and_root_stable() {
    let mut freeze = candidate_freeze(3);
    freeze.schema = K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3.to_owned();
    freeze.freeze_root_sha256 = freeze.expected_root().expect("v3 freeze root");
    let bytes = serde_json::to_vec(&freeze).expect("encode v3 freeze");
    assert_legacy_freeze_bytes_exclude_v6_fields(&bytes);
    assert_eq!(
        freeze.freeze_root_sha256,
        "d5238a451b8fb402b510cd45c4e1009f7c410194bb39c293f8d3aa3732c8ed57"
    );
    assert_eq!(
        nando_operator_kernel::sha256_bytes(&bytes),
        "da62d4e2a170102c15e36dab9540f36e3e8724d778fa165ef1de79e7e5e8c64b"
    );
    let restored: K1NaturalCandidateFreezeV1 =
        serde_json::from_slice(&bytes).expect("decode v3 freeze");
    restored.validate().expect("validate v3 freeze");
    assert_eq!(restored.freeze_root_sha256, freeze.freeze_root_sha256);
    assert_eq!(
        serde_json::to_vec(&restored).expect("re-encode v3 freeze"),
        bytes
    );
}

#[test]
fn legacy_v4_candidate_freeze_remains_decodable_and_root_stable() {
    let mut freeze = candidate_freeze(4);
    freeze.schema = K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4.to_owned();
    freeze.freeze_root_sha256 = freeze.expected_root().expect("v4 freeze root");
    let bytes = serde_json::to_vec(&freeze).expect("encode v4 freeze");
    assert_legacy_freeze_bytes_exclude_v6_fields(&bytes);
    assert_eq!(
        freeze.freeze_root_sha256,
        "3f1e8d8d2b2e06c86cfea795c0154ca97f1b4b9c52e49d3e3f839d1631963cad"
    );
    assert_eq!(
        nando_operator_kernel::sha256_bytes(&bytes),
        "6fcc32ac558ce0936fff7dbdb759915da57b498813b4904e8b9b6e6eb114bfbd"
    );
    let restored: K1NaturalCandidateFreezeV1 =
        serde_json::from_slice(&bytes).expect("decode v4 freeze");
    restored.validate().expect("validate v4 freeze");
    assert_eq!(restored.freeze_root_sha256, freeze.freeze_root_sha256);
    assert_eq!(
        serde_json::to_vec(&restored).expect("re-encode v4 freeze"),
        bytes
    );
}

#[test]
fn current_candidate_freeze_uses_v5_schema() {
    let freeze = candidate_freeze(4);
    let bytes = serde_json::to_vec(&freeze).expect("encode v5 freeze");
    assert_legacy_freeze_bytes_exclude_v6_fields(&bytes);
    assert_eq!(
        freeze.freeze_root_sha256,
        "106ff5ffabd6339f98db08c8601fb4a7b9d9a8d9494e773e474eda05a7cad7e8"
    );
    assert_eq!(
        nando_operator_kernel::sha256_bytes(&bytes),
        "51eaff022a88be3d9860cfef57b97af16a782d27e07602744e931120c4e310a6"
    );
    assert_eq!(freeze.schema, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5);
    freeze.validate().expect("validate v5 freeze");
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
