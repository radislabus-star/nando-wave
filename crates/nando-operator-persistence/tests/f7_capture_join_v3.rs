mod f7_support;

use f7_support::{FixtureV3, root};
use nando_operator_learning::{GenerationCaptureCommitmentV3, GenerationCaptureIndexV3};
use nando_operator_persistence::{
    GenerationCaptureJoinErrorV3, decode_generation_checkpoint_v3,
    join_generation_checkpoint_to_capture_index_v3,
};

fn capture_index(
    checkpoint: &nando_operator_persistence::RestoredGenerationCheckpointV3,
) -> GenerationCaptureIndexV3 {
    GenerationCaptureIndexV3::new(
        checkpoint
            .receipts()
            .iter()
            .enumerate()
            .map(|(index, pair)| {
                let receipt = pair.generation_receipt();
                GenerationCaptureCommitmentV3::new(
                    receipt.capture_sequence(),
                    root(&format!("capture record {index}")),
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

#[test]
fn exact_capture_join_binds_checkpoint_without_authority() {
    let mut fixture = FixtureV3::new("capture-join");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint = decode_generation_checkpoint_v3(&fixture.checkpoint(1)).expect("checkpoint");
    let index = capture_index(&checkpoint);

    let joined = join_generation_checkpoint_to_capture_index_v3(checkpoint, &index).expect("join");
    assert_eq!(joined.capture_index_sha256(), index.index_sha256());
    assert_eq!(joined.checkpoint().receipts().len(), 2);
    assert!(!joined.execution_authority());
}

#[test]
fn missing_or_tampered_capture_relation_blocks_join() {
    let mut fixture = FixtureV3::new("capture-mismatch");
    fixture.append_support();
    let checkpoint_bytes = fixture.checkpoint(1);
    let checkpoint = decode_generation_checkpoint_v3(&checkpoint_bytes).expect("checkpoint");
    let receipt = checkpoint.receipts()[0].generation_receipt();
    let wrong_index = GenerationCaptureIndexV3::new(vec![
        GenerationCaptureCommitmentV3::new(
            receipt.capture_sequence(),
            root("capture record"),
            receipt.lineage_root_sha256().to_owned(),
            receipt.event_root_sha256().to_owned(),
            root("foreign request"),
        )
        .expect("wrong commitment"),
    ])
    .expect("wrong index");
    assert!(matches!(
        join_generation_checkpoint_to_capture_index_v3(checkpoint, &wrong_index),
        Err(GenerationCaptureJoinErrorV3::MissingCaptureCommitment)
    ));

    let checkpoint = decode_generation_checkpoint_v3(&checkpoint_bytes).expect("checkpoint");
    let empty = GenerationCaptureIndexV3::new(Vec::new()).expect("empty index");
    assert!(matches!(
        join_generation_checkpoint_to_capture_index_v3(checkpoint, &empty),
        Err(GenerationCaptureJoinErrorV3::MissingCaptureCommitment)
    ));
}
