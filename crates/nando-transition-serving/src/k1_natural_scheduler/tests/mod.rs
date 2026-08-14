use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_kernel::{
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
    canonical_json_sha256,
};
use nando_operator_learning::multi_source::{
    IDENTIFIER_RESULT_SCHEMA_V1, IdentifierResourceLimitsV1, IdentifierResultV1,
    IdentifierSupportRowV1, K1ConsequenceTypeV1, K1DeficitSnapshotV1, K1GenerationBudgetV1,
    K1MotifCandidateSupportV1, K1MotifDispositionSummaryV1, K1NaturalCandidateFreezeV1,
    K1NaturalEvidenceClassV1, K1NaturalEvidenceRowV1, MultiSourceT1IdentificationStateV1,
    ProgramDispositionSetV1, TerminalDiagnosticV1, build_identifier_causal_input_manifest_v1,
    build_identifier_support_manifest_v1, build_k1_motif_cohort_catalog_v1,
    build_k1_natural_candidate_queue_v1, build_k1_natural_cohort_catalog_v1,
    build_relevant_identifier_artifact_projection_v1, source_neutral_topology_motifs_v1,
};

use super::journal::encode_hex;
use super::*;

mod authority;
mod bounded_wire;
mod candidate_binding;
mod duplicate_cohorts;
mod fork;
mod future_censor;
mod journal;
mod pre_action_evidence;
mod projection;

static TEST_ID: AtomicU64 = AtomicU64::new(0);

fn root(value: u64) -> String {
    format!("{value:064x}")
}

fn candidate_watermark(catalog: &K1NaturalCohortCatalogV1) -> u64 {
    catalog
        .candidates
        .iter()
        .map(|candidate| candidate.last_capture_sequence)
        .max()
        .unwrap_or(0)
}

fn test_context() -> (PathBuf, CertificationAuthorityConfigV1, SigningKey) {
    let root = std::env::temp_dir().join(format!(
        "nando-k1-scheduler-{}-{}",
        std::process::id(),
        TEST_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("root");
    let signing_key = SigningKey::from_bytes(&[11_u8; 32]);
    let public_key_path = root.join("authority.pub");
    fs::write(
        &public_key_path,
        encode_hex(signing_key.verifying_key().as_bytes()),
    )
    .expect("public key");
    let config = CertificationAuthorityConfigV1 {
        root: root.join("state"),
        cleanup_receipts_path: root.join("cleanup"),
        anchor_path: root.join("anchor/operator-certification.json"),
        authority_socket_path: root.join("authority.sock"),
        authority_public_key_path: public_key_path.clone(),
        cleanup_public_key_path: public_key_path,
        response_registry_path: root.join("registry.json"),
        runtime_revocations_path: root.join("revocations.json"),
        k1_exact_sources: None,
    };
    (root, config, signing_key)
}

fn candidate_freeze() -> K1NaturalCandidateFreezeV1 {
    candidate_freeze_with_basis(natural_t1_discovery_basis_root_v3().expect("discovery basis"))
}

fn candidate_freeze_with_basis(discovery_basis_root_sha256: String) -> K1NaturalCandidateFreezeV1 {
    candidate_freeze_for_generation_and_basis(1, discovery_basis_root_sha256)
}

fn candidate_freeze_for_generation_and_basis(
    generation_sequence: u64,
    discovery_basis_root_sha256: String,
) -> K1NaturalCandidateFreezeV1 {
    candidate_freeze_material(generation_sequence, discovery_basis_root_sha256).4
}

fn candidate_freeze_material(
    generation_sequence: u64,
    discovery_basis_root_sha256: String,
) -> (
    K1NaturalCohortCatalogV1,
    K1DeficitSnapshotV1,
    K1NaturalCandidateQueueV1,
    K1NaturalCohortCandidateV1,
    K1NaturalCandidateFreezeV1,
) {
    let rows = (1..=8)
        .map(|index| {
            K1NaturalEvidenceRowV1::seal(
                root(100 + index),
                root(199),
                root(200),
                root(201),
                root(202),
                root(if index <= 4 { 300 } else { 301 }),
                K1ConsequenceTypeV1::Scalar,
                K1NaturalEvidenceClassV1::NaturalLive,
                index,
                100,
                1_000,
                true,
                index <= 2,
                false,
            )
            .expect("evidence")
        })
        .collect::<Vec<_>>();
    let catalog = build_k1_natural_cohort_catalog_v1(
        &rows,
        root(400),
        root(401),
        nando_operator_learning::multi_source::MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V3.to_owned(),
    )
    .expect("catalog");
    let deficit = K1DeficitSnapshotV1::seal(
        0,
        root(402),
        root(403),
        0,
        0,
        0,
        0,
        0,
        3,
        3,
        2,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("deficit");
    candidate_freeze_material_with_deficit(
        generation_sequence,
        discovery_basis_root_sha256,
        catalog,
        deficit,
    )
}

fn candidate_freeze_material_with_deficit(
    generation_sequence: u64,
    discovery_basis_root_sha256: String,
    catalog: K1NaturalCohortCatalogV1,
    deficit: K1DeficitSnapshotV1,
) -> (
    K1NaturalCohortCatalogV1,
    K1DeficitSnapshotV1,
    K1NaturalCandidateQueueV1,
    K1NaturalCohortCandidateV1,
    K1NaturalCandidateFreezeV1,
) {
    let queue =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, candidate_watermark(&catalog))
            .expect("queue");
    let row = queue.first_readiness_pass().expect("ready row");
    let candidate = catalog
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_root_sha256 == row.candidate_root_sha256)
        .expect("candidate");
    let candidate = candidate.clone();
    let freeze = K1NaturalCandidateFreezeV1::seal(
        generation_sequence,
        &catalog,
        &deficit,
        &queue,
        &candidate,
        row.score.clone(),
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
    .expect("freeze");
    (catalog, deficit, queue, candidate, freeze)
}

fn exact_candidate_freeze(generation_sequence: u64) -> K1NaturalCandidateFreezeV1 {
    exact_candidate_freeze_at(generation_sequence, 1_700_000_000)
}

fn exact_candidate_freeze_at(
    generation_sequence: u64,
    selected_at_unix: u64,
) -> K1NaturalCandidateFreezeV1 {
    let generation_root_offset = generation_sequence.saturating_mul(100_000);
    let topology = PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: 1,
        output_part_count: 1,
        roles: (0..2)
            .map(|local_role_id| MultiSourceRoleNodeV1 {
                local_role_id,
                source_ordinal: local_role_id,
                value_ordinal: 0,
                type_class: MultiSourceTypeClassV1::String,
                container_class: MultiSourceContainerClassV1::Scalar,
                cardinality_class: MultiSourceCardinalityClassV1::One,
                temporal_class: MultiSourceTemporalClassV1::Historical,
                depth_bucket: 1,
                structural_flags: 0,
            })
            .collect(),
        role_witnesses: Vec::new(),
        relations: vec![MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::Precedes,
            source_role_id: 0,
            target_role_id: 1,
        }],
    };
    let motif = source_neutral_topology_motifs_v1(&topology)
        .expect("motifs")
        .into_iter()
        .find(|motif| motif.role_count == 2 && motif.relation_count == 1)
        .expect("two-role motif");
    let rows = (1..=8)
        .map(|index| {
            K1NaturalEvidenceRowV1::seal_motif_v4(
                root(10_000 + index),
                root(10_100),
                motif.embeddings[0].ambient_topology_root_sha256.clone(),
                &motif,
                root(10_200 + generation_root_offset),
                root(if index <= 4 { 10_300 } else { 10_301 }),
                K1ConsequenceTypeV1::Collection,
                index,
                1_000,
                1_000 + index,
                true,
                index <= 2,
                false,
            )
            .expect("motif evidence")
        })
        .collect::<Vec<_>>();
    let retained_manifest_root_sha256 = canonical_json_sha256(&(
        "nando.k1-motif-evidence-manifest.v1",
        rows.iter()
            .map(|row| row.row_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .expect("retained manifest");
    let overflow_manifest_root_sha256 = canonical_json_sha256(&(
        "nando.k1-motif-test-overflow-manifest.v1",
        motif.motif_root_sha256.as_str(),
        0_u64,
    ))
    .expect("overflow manifest");
    let support = K1MotifCandidateSupportV1::seal(
        root(10_100),
        motif.motif_root_sha256.clone(),
        root(10_200 + generation_root_offset),
        K1ConsequenceTypeV1::Collection,
        8,
        retained_manifest_root_sha256.clone(),
        0,
        overflow_manifest_root_sha256.clone(),
    )
    .expect("motif support");
    let empty_manifest = |label: &str| {
        canonical_json_sha256(&(label, Vec::<String>::new())).expect("empty manifest")
    };
    let disposition = K1MotifDispositionSummaryV1::seal(
        nando_operator_learning::multi_source::source_neutral_topology_motif_config_root_v1()
            .expect("motif config"),
        8,
        8,
        8,
        0,
        canonical_json_sha256(&(
            "nando.k1-motif-candidate-support-manifest.v1",
            std::collections::BTreeSet::from([support.support_root_sha256.as_str()]),
        ))
        .expect("support manifest"),
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
        canonical_json_sha256(&(
            "nando.k1-motif-test-source-disposition.v1",
            rows.iter()
                .map(|row| row.evidence_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))
        .expect("source disposition"),
    )
    .expect("motif disposition");
    let catalog = build_k1_motif_cohort_catalog_v1(
        &rows,
        &[support],
        root(10_400),
        root(10_401),
        nando_operator_learning::multi_source::MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V4.to_owned(),
        disposition,
    )
    .expect("motif catalog");
    let deficit = K1DeficitSnapshotV1::seal(
        0,
        root(10_402),
        root(10_403),
        0,
        0,
        0,
        0,
        0,
        3,
        3,
        2,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("deficit");
    let queue = build_k1_natural_candidate_queue_v1(&catalog, &deficit, 8).expect("queue v2");
    let candidate = catalog.candidates.first().expect("candidate");
    let support_rows = rows
        .iter()
        .map(|row| {
            IdentifierSupportRowV1::seal(
                row.capture_sequence,
                row.evidence_root_sha256.clone(),
                root(11_000 + row.capture_sequence),
                row.complete_topology_root_sha256.clone(),
                row.capture_generation_root_sha256.clone(),
                row.motif_root_sha256.clone(),
                vec![root(12_000 + row.capture_sequence)],
                row.lineage_root_sha256.clone(),
                root(13_000 + row.capture_sequence),
                root(14_000 + row.capture_sequence),
            )
            .expect("support row")
        })
        .collect();
    let support_manifest = build_identifier_support_manifest_v1(
        candidate.candidate_structural_root_sha256.clone(),
        8,
        support_rows,
        64,
    )
    .expect("support manifest");
    let artifact_projection =
        build_relevant_identifier_artifact_projection_v1(&support_manifest, &[])
            .expect("artifact projection");
    let causal_manifest = build_identifier_causal_input_manifest_v1(
        &support_manifest,
        &artifact_projection,
        nando_operator_learning::multi_source::MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V4.to_owned(),
        natural_t1_discovery_basis_root_v4().expect("discovery basis"),
        root(10_404),
        IdentifierResourceLimitsV1::seal(64, 4_096, 4_096, 16).expect("limits"),
    )
    .expect("causal manifest");
    let queue = queue
        .bind_exact_opportunities_v4(
            &ExactAttemptIndexV1::empty(0).expect("exact index"),
            root(10_405),
            &std::collections::BTreeMap::from([(
                candidate.candidate_root_sha256.clone(),
                causal_manifest.clone(),
            )]),
        )
        .expect("queue v4");
    K1NaturalCandidateFreezeV1::seal_exact_v8(
        generation_sequence,
        &catalog,
        &deficit,
        &queue,
        candidate,
        queue
            .first_readiness_pass()
            .expect("unseen row")
            .score
            .clone(),
        "nando.k1-operator-blind-scheduler.v4".to_owned(),
        natural_t1_discovery_basis_root_v4().expect("discovery basis"),
        K1GenerationBudgetV1 {
            maximum_support_rows: 64,
            maximum_probe_rounds: 8,
            maximum_probe_cost_units: 24,
            maximum_generation_seconds: 86_400,
        },
        8,
        8,
        selected_at_unix,
        causal_manifest,
        root(10_405),
        root(10_406),
        queue.exact_attempt_index_root_sha256.clone(),
        root(10_407),
    )
    .expect("freeze v8")
}

fn exact_terminal_diagnostic(freeze: &K1NaturalCandidateFreezeV1) -> TerminalDiagnosticV1 {
    let causal = freeze
        .identifier_causal_input_manifest
        .as_deref()
        .expect("causal manifest");
    let disposition = ProgramDispositionSetV1::seal(Vec::new()).expect("empty dispositions");
    let identifier_report_root_sha256 = root(15_000 + freeze.generation_sequence);
    let identifier_result_root_sha256 = canonical_json_sha256(&(
        IDENTIFIER_RESULT_SCHEMA_V1,
        causal.opportunity_root_sha256.as_str(),
        disposition.accepted_set_root_sha256.as_str(),
        disposition.disposition_set_root_sha256.as_str(),
        identifier_report_root_sha256.as_str(),
    ))
    .expect("identifier result root");
    let result = IdentifierResultV1 {
        schema: IDENTIFIER_RESULT_SCHEMA_V1.to_owned(),
        identifier_result_root_sha256,
        opportunity_root_sha256: causal.opportunity_root_sha256.clone(),
        accepted_set_root_sha256: disposition.accepted_set_root_sha256.clone(),
        disposition_set_root_sha256: disposition.disposition_set_root_sha256.clone(),
        identifier_report_root_sha256,
    };
    TerminalDiagnosticV1::seal(
        freeze.freeze_root_sha256.clone(),
        &result,
        causal.support_manifest_root_sha256.clone(),
        causal.support_rows,
        causal.relevant_artifact_projection_root_sha256.clone(),
        0,
        &disposition,
        &[],
        MultiSourceT1IdentificationStateV1::NoEligibleCohort,
        "motif_program_candidates_empty".to_owned(),
        freeze.selected_at_unix.saturating_add(100),
    )
    .expect("diagnostic")
}
