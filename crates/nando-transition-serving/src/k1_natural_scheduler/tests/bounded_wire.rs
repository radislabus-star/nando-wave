use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::*;
use crate::k1_natural_scheduler::authority::{send_authority_bytes, validate_scheduler_cas};
use crate::k1_natural_scheduler::bounded_wire::{
    K1_CANDIDATE_FREEZE_COMPRESSED_MAX_BYTES_V2, K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2,
    K1CandidateFreezeAuthorityRequestV2, decode_candidate_freeze_v2, encode_candidate_freeze_v2,
};
use crate::k1_natural_scheduler::journal::{
    restore_anchored_scheduler_for, scheduler_journal_path_for,
};
use crate::k1_natural_scheduler::projection::projection_for;

fn request() -> K1CandidateFreezeAuthorityRequestV1 {
    let (catalog, deficit_snapshot, queue, candidate, freeze) = candidate_freeze_material(
        1,
        natural_t1_discovery_basis_root_v3().expect("discovery basis"),
    );
    K1CandidateFreezeAuthorityRequestV1 {
        schema: K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Mechanism,
        catalog,
        deficit_snapshot,
        queue,
        candidate,
        freeze,
        active_protocol_mode_set_root_sha256: root(900),
    }
}

fn projection() -> K1SchedulerProjectionV1 {
    projection_for(&K1SchedulerLedgerV1::empty().expect("empty scheduler"))
        .expect("empty projection")
}

fn authority_request(
    config: &CertificationAuthorityConfigV1,
) -> K1CandidateFreezeAuthorityRequestV1 {
    let deficit = current_deficit_snapshot(config).expect("current deficit");
    let seed = candidate_freeze_material(
        1,
        natural_t1_discovery_basis_root_v3().expect("discovery basis"),
    );
    let (catalog, deficit_snapshot, queue, candidate, freeze) =
        candidate_freeze_material_with_deficit(
            1,
            natural_t1_discovery_basis_root_v3().expect("discovery basis"),
            seed.0,
            deficit,
        );
    K1CandidateFreezeAuthorityRequestV1 {
        schema: K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Mechanism,
        catalog,
        deficit_snapshot,
        queue,
        candidate,
        freeze,
        active_protocol_mode_set_root_sha256:
            super::super::duplicate_cohorts::known_epistemic_protocol_mode_set_root(
                &BTreeSet::new(),
            )
            .expect("empty protocol root"),
    }
}

fn rewrite_logical(
    mut envelope: K1CandidateFreezeAuthorityRequestV2,
    logical: &[u8],
) -> K1CandidateFreezeAuthorityRequestV2 {
    let compressed = zstd::stream::encode_all(logical, 1).expect("compress logical");
    envelope.logical_bytes = u64::try_from(logical.len()).expect("logical size");
    envelope.logical_sha256 = format!("{:x}", Sha256::digest(logical));
    envelope.compressed_bytes = u64::try_from(compressed.len()).expect("compressed size");
    envelope.payload_base64 = BASE64_STANDARD.encode(compressed);
    envelope
}

#[test]
fn bounded_v2_roundtrip_preserves_canonical_logical_request_and_cas() {
    let request = request();
    let projection = projection();
    let envelope = encode_candidate_freeze_v2(&request, &projection).expect("V2 envelope");
    let wire = serde_json::to_vec(&envelope).expect("wire");
    assert!(wire.len() <= K1_SCHEDULER_MAX_REQUEST_BYTES);
    assert!(
        usize::try_from(envelope.compressed_bytes).expect("compressed usize")
            <= K1_CANDIDATE_FREEZE_COMPRESSED_MAX_BYTES_V2
    );
    let (decoded, cas) = decode_candidate_freeze_v2(envelope).expect("bounded decode");
    assert_eq!(decoded, request);
    assert_eq!(cas.ledger_revision, projection.ledger_revision);
    assert_eq!(cas.ledger_root_sha256, projection.ledger_root_sha256);
    assert_eq!(
        cas.projection_root_sha256,
        projection.projection_root_sha256
    );
}

#[test]
fn bounded_v2_rejects_declared_budgets_and_malformed_payloads() {
    let envelope = encode_candidate_freeze_v2(&request(), &projection()).expect("V2 envelope");

    let mut logical_oversize = envelope.clone();
    logical_oversize.logical_bytes =
        u64::try_from(K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2 + 1).expect("limit");
    assert_eq!(
        decode_candidate_freeze_v2(logical_oversize),
        Err("k1_candidate_freeze_v2_envelope_invalid".to_owned())
    );

    let mut compressed_oversize = envelope.clone();
    compressed_oversize.compressed_bytes =
        u64::try_from(K1_CANDIDATE_FREEZE_COMPRESSED_MAX_BYTES_V2 + 1).expect("limit");
    assert_eq!(
        decode_candidate_freeze_v2(compressed_oversize),
        Err("k1_candidate_freeze_v2_envelope_invalid".to_owned())
    );

    let mut malformed = envelope.clone();
    malformed.payload_base64 = "%%%".to_owned();
    assert_eq!(
        decode_candidate_freeze_v2(malformed),
        Err("k1_candidate_freeze_v2_base64_invalid".to_owned())
    );

    let mut checksum = envelope.clone();
    checksum.logical_sha256 = root(901);
    assert_eq!(
        decode_candidate_freeze_v2(checksum),
        Err("k1_candidate_freeze_v2_logical_checksum_mismatch".to_owned())
    );

    let mut bomb = envelope.clone();
    let oversized = vec![0_u8; K1_CANDIDATE_FREEZE_LOGICAL_MAX_BYTES_V2 + 1];
    let compressed = zstd::stream::encode_all(oversized.as_slice(), 1).expect("compress bomb");
    bomb.logical_bytes = 1;
    bomb.compressed_bytes = u64::try_from(compressed.len()).expect("compressed size");
    bomb.payload_base64 = BASE64_STANDARD.encode(compressed);
    assert_eq!(
        decode_candidate_freeze_v2(bomb),
        Err("k1_candidate_freeze_v2_logical_budget".to_owned())
    );
}

#[test]
fn bounded_v2_rejects_truncated_trailing_and_unknown_logical_fields() {
    let envelope = encode_candidate_freeze_v2(&request(), &projection()).expect("V2 envelope");
    let compressed = BASE64_STANDARD
        .decode(envelope.payload_base64.as_bytes())
        .expect("base64");

    let mut truncated_bytes = compressed.clone();
    truncated_bytes.pop();
    let mut truncated = envelope.clone();
    truncated.compressed_bytes = u64::try_from(truncated_bytes.len()).expect("size");
    truncated.payload_base64 = BASE64_STANDARD.encode(truncated_bytes);
    assert_eq!(
        decode_candidate_freeze_v2(truncated),
        Err("k1_candidate_freeze_v2_frame_invalid".to_owned())
    );

    let mut trailing_bytes = compressed;
    trailing_bytes.push(0);
    let mut trailing = envelope.clone();
    trailing.compressed_bytes = u64::try_from(trailing_bytes.len()).expect("size");
    trailing.payload_base64 = BASE64_STANDARD.encode(trailing_bytes);
    assert_eq!(
        decode_candidate_freeze_v2(trailing),
        Err("k1_candidate_freeze_v2_trailing_bytes".to_owned())
    );

    let mut logical = serde_json::to_value(request()).expect("logical value");
    logical["unknown_authority_field"] = json!(true);
    let bytes = serde_json::to_vec(&logical).expect("logical bytes");
    let unknown = rewrite_logical(envelope, &bytes);
    assert!(
        decode_candidate_freeze_v2(unknown)
            .expect_err("unknown logical field")
            .starts_with("k1_candidate_freeze_v2_logical_decode:")
    );
}

#[test]
fn bounded_v2_rejects_unknown_envelope_fields_and_stale_scheduler_cas() {
    let envelope = encode_candidate_freeze_v2(&request(), &projection()).expect("V2 envelope");
    let mut value = serde_json::to_value(&envelope).expect("envelope value");
    value["unknown_envelope_field"] = json!(true);
    assert!(serde_json::from_value::<K1CandidateFreezeAuthorityRequestV2>(value).is_err());

    let scheduler = K1SchedulerLedgerV1::empty().expect("scheduler");
    let (_, cas) = decode_candidate_freeze_v2(envelope).expect("decode");
    validate_scheduler_cas(&scheduler, &cas).expect("current CAS");
    let mut stale = cas;
    stale.ledger_revision = stale.ledger_revision.saturating_add(1);
    assert_eq!(
        validate_scheduler_cas(&scheduler, &stale),
        Err("k1_candidate_freeze_scheduler_cas_failed".to_owned())
    );
}

#[test]
fn outer_wire_budget_fails_before_socket_connect() {
    let (root_dir, config, _) = test_context();
    assert_eq!(
        send_authority_bytes(&config, vec![0; K1_SCHEDULER_MAX_REQUEST_BYTES + 1]),
        Err("k1_scheduler_authority_request_budget".to_owned())
    );
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn bounded_v2_append_is_idempotent_and_restart_identical() {
    let (root_dir, config, signing_key) = test_context();
    fs::write(
        &config.response_registry_path,
        serde_json::to_vec(&nando_response_actor::ResponseRegistry {
            schema: "nando.response-registry.v6".to_owned(),
            revision: 0,
            packages: Vec::new(),
        })
        .expect("registry encode"),
    )
    .expect("registry write");
    recover_authority(&config, &signing_key).expect("scheduler genesis");

    let before =
        restore_projection_for(&config, K1SchedulerLaneV1::Mechanism).expect("before projection");
    let envelope =
        encode_candidate_freeze_v2(&authority_request(&config), &before).expect("V2 envelope");
    let expected_freeze_root = decode_candidate_freeze_v2(envelope.clone())
        .expect("decode envelope")
        .0
        .freeze
        .freeze_root_sha256;
    let line = serde_json::to_string(&envelope).expect("wire line");
    let first: serde_json::Value = serde_json::from_str(
        &handle_authority_line(&config, &signing_key, &line).expect("first response"),
    )
    .expect("first JSON");
    assert_eq!(first["error"], "");
    let after: K1SchedulerProjectionV1 =
        serde_json::from_value(first["projection"].clone()).expect("first projection");
    assert_eq!(after.ledger_revision, before.ledger_revision + 1);
    assert_eq!(
        after
            .active_candidate_freeze
            .as_ref()
            .expect("active freeze")
            .freeze_root_sha256,
        expected_freeze_root
    );

    let duplicate: serde_json::Value = serde_json::from_str(
        &handle_authority_line(&config, &signing_key, &line).expect("duplicate response"),
    )
    .expect("duplicate JSON");
    assert_eq!(duplicate["error"], "");
    let duplicate_projection: K1SchedulerProjectionV1 =
        serde_json::from_value(duplicate["projection"].clone()).expect("duplicate projection");
    assert_eq!(duplicate_projection, after);
    assert_eq!(
        fs::read_dir(scheduler_journal_path_for(
            &config,
            K1SchedulerLaneV1::Mechanism,
        ))
        .expect("journal")
        .count(),
        1
    );

    recover_authority(&config, &signing_key).expect("restart recovery");
    let restarted = restore_projection_for(&config, K1SchedulerLaneV1::Mechanism)
        .expect("restarted projection");
    assert_eq!(restarted, after);
    assert_eq!(
        restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Mechanism)
            .expect("restarted ledger")
            .revision,
        1
    );
    fs::remove_dir_all(root_dir).expect("cleanup");
}
