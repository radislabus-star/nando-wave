#[path = "../../nando-operator-persistence/tests/f7_support/mod.rs"]
mod f7_support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use f7_support::{FixtureV3, root};
use nando_operator_kernel::canonical_json_bytes;
use nando_operator_learning::{GenerationCaptureCommitmentV3, GenerationCaptureIndexV3};
use nando_operator_persistence::{
    GenerationCheckpointStoreV3, RestoredGenerationCheckpointV3, decode_generation_checkpoint_v3,
};
use serde::Serialize;

#[derive(Serialize)]
struct ControlledShadowMarkerV3<'a> {
    schema: &'static str,
    generation_id_sha256: &'a str,
    checkpoint_sha256: &'a str,
    capture_index_sha256: &'a str,
    evidence_class: &'static str,
    local_accept_eligible: bool,
    execution_authority: bool,
}

#[test]
#[ignore = "explicit F8-E controlled shadow provisioning only"]
fn export_controlled_shadow_seed_without_authority() {
    let output = std::env::var("NANDO_F8_SHADOW_SEED_DIR")
        .expect("NANDO_F8_SHADOW_SEED_DIR must name an empty staging directory");
    let output = Path::new(&output);
    assert!(
        !output.exists() || fs::read_dir(output).expect("read staging").next().is_none(),
        "refusing to overwrite a non-empty staging directory"
    );
    fs::create_dir_all(output).expect("create staging");

    let mut fixture = FixtureV3::new("f8e-controlled-live-shadow");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint_bytes = fixture.checkpoint(1);
    let checkpoint = decode_generation_checkpoint_v3(&checkpoint_bytes).expect("checkpoint");
    assert!(!checkpoint.generation().manifest().execution_authority());

    let generation_store = output.join("operator-generation-v3");
    GenerationCheckpointStoreV3::open(&generation_store)
        .expect("generation store")
        .publish(&checkpoint_bytes)
        .expect("publish checkpoint");
    let capture_index = generation_capture(&checkpoint);
    let capture_path = output.join("operator-generation-capture-v3.cbor");
    fs::write(
        &capture_path,
        capture_index.canonical_bytes().expect("capture bytes"),
    )
    .expect("write capture index");
    fs::set_permissions(&capture_path, fs::Permissions::from_mode(0o600))
        .expect("capture permissions");

    let marker = ControlledShadowMarkerV3 {
        schema: "nando.f8-controlled-shadow-seed.v3",
        generation_id_sha256: checkpoint.generation().manifest().generation_id_sha256(),
        checkpoint_sha256: checkpoint.checkpoint_sha256(),
        capture_index_sha256: capture_index.index_sha256(),
        evidence_class: "controlled_shadow_only",
        local_accept_eligible: false,
        execution_authority: false,
    };
    let marker_path = output.join("CONTROLLED_SHADOW_ONLY.json");
    fs::write(
        &marker_path,
        canonical_json_bytes(&marker).expect("marker bytes"),
    )
    .expect("write marker");
    fs::set_permissions(&marker_path, fs::Permissions::from_mode(0o600))
        .expect("marker permissions");
}

fn generation_capture(checkpoint: &RestoredGenerationCheckpointV3) -> GenerationCaptureIndexV3 {
    GenerationCaptureIndexV3::new(
        checkpoint
            .receipts()
            .iter()
            .enumerate()
            .map(|(index, pair)| {
                let receipt = pair.generation_receipt();
                GenerationCaptureCommitmentV3::new(
                    receipt.capture_sequence(),
                    root(&format!("f8e controlled capture {index}")),
                    receipt.lineage_root_sha256().to_owned(),
                    receipt.event_root_sha256().to_owned(),
                    receipt.f6_request_sha256().to_owned(),
                )
                .expect("capture commitment")
            })
            .collect(),
    )
    .expect("capture index")
}
