use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_learning::multi_source::{
    K1ConsequenceTypeV1, K1DeficitSnapshotV1, K1GenerationBudgetV1, K1NaturalCandidateFreezeV1,
    K1NaturalEvidenceClassV1, K1NaturalEvidenceRowV1, build_k1_natural_candidate_queue_v1,
    build_k1_natural_cohort_catalog_v1,
};

use super::journal::encode_hex;
use super::*;

mod authority;
mod duplicate_cohorts;
mod fork;
mod future_censor;
mod journal;
mod pre_action_evidence;

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
    };
    (root, config, signing_key)
}

fn candidate_freeze() -> K1NaturalCandidateFreezeV1 {
    candidate_freeze_with_basis(natural_t1_discovery_basis_root_v3().expect("discovery basis"))
}

fn candidate_freeze_with_basis(discovery_basis_root_sha256: String) -> K1NaturalCandidateFreezeV1 {
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
    let queue =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, candidate_watermark(&catalog))
            .expect("queue");
    let row = queue.first_readiness_pass().expect("ready row");
    let candidate = catalog
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_root_sha256 == row.candidate_root_sha256)
        .expect("candidate");
    K1NaturalCandidateFreezeV1::seal(
        1,
        &catalog,
        &deficit,
        &queue,
        candidate,
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
    .expect("freeze")
}
