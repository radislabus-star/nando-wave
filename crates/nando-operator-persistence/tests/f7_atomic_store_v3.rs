#[path = "f7_atomic_store_v3/f7_store_security_v3.rs"]
mod f7_store_security_v3;
mod f7_support;

use std::fs;

use f7_support::{FixtureV3, root};
use nando_operator_persistence::{
    GENERATION_STORE_SLOT_A_FILE_V3, GENERATION_STORE_SLOT_B_FILE_V3, GenerationCheckpointStoreV3,
    GenerationStoreErrorV3, GenerationStoreSlotV3, decode_generation_checkpoint_v3,
    encode_generation_checkpoint_v3,
};

#[test]
fn two_slot_publish_restores_latest_checkpoint_byte_identically() {
    let mut fixture = FixtureV3::new("two-slot");
    fixture.append_support();
    let first = fixture.checkpoint(1);
    let store = GenerationCheckpointStoreV3::open(&fixture.directory).expect("store");
    let first_publish = store.publish(&first).expect("first publish");
    assert_eq!(first_publish.slot(), GenerationStoreSlotV3::A);

    fixture.freeze_and_append_future();
    let second = fixture.checkpoint(2);
    let second_publish = store.publish(&second).expect("second publish");
    assert_eq!(second_publish.slot(), GenerationStoreSlotV3::B);
    let restored = store.restore().expect("restore");
    let checkpoint = restored.checkpoint().expect("checkpoint");
    assert_eq!(checkpoint.canonical_bytes(), second.as_ref());
    assert_eq!(checkpoint.ledger().accounting().support_rows, 1);
    assert_eq!(checkpoint.ledger().accounting().future_rows, 1);
    assert_eq!(checkpoint.receipts().len(), 2);
    assert!(!checkpoint.execution_authority());
    assert!(!restored.execution_authority());
    assert!(
        fixture
            .directory
            .join(GENERATION_STORE_SLOT_A_FILE_V3)
            .exists()
    );
    assert!(
        fixture
            .directory
            .join(GENERATION_STORE_SLOT_B_FILE_V3)
            .exists()
    );
}

#[test]
fn stale_temporary_and_corrupt_new_slot_recover_previous_generation() {
    let mut fixture = FixtureV3::new("recovery");
    fixture.append_support();
    let first = fixture.checkpoint(1);
    let store = GenerationCheckpointStoreV3::open(&fixture.directory).expect("store");
    store.publish(&first).expect("publish first");

    fs::write(
        fixture
            .directory
            .join(format!(".{GENERATION_STORE_SLOT_B_FILE_V3}.new")),
        b"partial",
    )
    .expect("partial write");
    let recovered = store.restore().expect("recover temp");
    assert_eq!(
        recovered
            .checkpoint()
            .expect("checkpoint")
            .canonical_bytes(),
        first.as_ref()
    );
    assert_eq!(recovered.quarantined_files().len(), 1);

    fixture.freeze_and_append_future();
    let mut corrupt = fixture.checkpoint(2).to_vec();
    let offset = corrupt.len() / 2;
    corrupt[offset] ^= 1;
    fs::write(
        fixture.directory.join(GENERATION_STORE_SLOT_B_FILE_V3),
        corrupt,
    )
    .expect("corrupt slot");
    let recovered = store.restore().expect("recover slot");
    assert_eq!(
        recovered
            .checkpoint()
            .expect("checkpoint")
            .canonical_bytes(),
        first.as_ref()
    );
    assert!(recovered.recovered_previous());
    assert_eq!(recovered.quarantined_files().len(), 1);
}

#[test]
fn evidence_rollback_and_publish_sequence_jump_are_rejected() {
    let mut full = FixtureV3::new("rollback-full");
    full.append_support();
    let first = full.checkpoint(1);
    full.freeze_and_append_future();
    let second = full.checkpoint(2);
    let store = GenerationCheckpointStoreV3::open(&full.directory).expect("store");
    store.publish(&first).expect("first");
    store.publish(&second).expect("second");

    let mut rolled_back = FixtureV3::new("rollback-copy");
    rolled_back.append_support();
    let rollback = rolled_back.checkpoint(3);
    assert!(matches!(
        store.publish(&rollback),
        Err(GenerationStoreErrorV3::EvidenceRollback)
    ));
    let sequence_jump = full.checkpoint(4);
    assert!(matches!(
        store.publish(&sequence_jump),
        Err(GenerationStoreErrorV3::NonMonotonicPublish)
    ));
}

#[test]
fn generation_change_requires_exact_next_sequence_and_parent() {
    let mut first = FixtureV3::new("generation-one");
    first.append_support();
    let store = GenerationCheckpointStoreV3::open(&first.directory).expect("store");
    store.publish(&first.checkpoint(1)).expect("generation one");

    let mut second = FixtureV3::new_generation(
        "generation-two",
        2,
        Some(first.manifest.generation_id_sha256().to_owned()),
        "actor-v2",
    );
    second.append_support();
    store
        .publish(&second.checkpoint(2))
        .expect("generation two");

    let mut foreign = FixtureV3::new_generation(
        "generation-foreign",
        3,
        Some(root("wrong parent")),
        "actor-v3",
    );
    foreign.append_support();
    assert!(matches!(
        store.publish(&foreign.checkpoint(3)),
        Err(GenerationStoreErrorV3::NonMonotonicGeneration)
    ));
    assert_eq!(
        store
            .restore()
            .expect("restore")
            .checkpoint()
            .expect("checkpoint")
            .generation()
            .manifest()
            .generation_id_sha256(),
        second.manifest.generation_id_sha256()
    );
}

#[test]
fn ledger_receipt_omission_and_full_corruption_fail_closed() {
    let mut fixture = FixtureV3::new("receipt-set");
    fixture.append_support();
    assert!(encode_generation_checkpoint_v3(1, &fixture.bundle, &fixture.ledger, &[]).is_err());
    let checkpoint = fixture.checkpoint(1);
    assert_eq!(
        decode_generation_checkpoint_v3(&checkpoint)
            .expect("decode")
            .receipts()
            .len(),
        1
    );

    let store = GenerationCheckpointStoreV3::open(&fixture.directory).expect("store");
    fs::write(
        fixture.directory.join(GENERATION_STORE_SLOT_A_FILE_V3),
        b"broken-a",
    )
    .expect("slot a");
    fs::write(
        fixture.directory.join(GENERATION_STORE_SLOT_B_FILE_V3),
        b"broken-b",
    )
    .expect("slot b");
    let restored = store.restore().expect("empty shadow");
    assert!(restored.checkpoint().is_none());
    assert_eq!(restored.quarantined_files().len(), 2);
    assert!(!restored.execution_authority());
}

#[test]
fn checkpoint_has_no_raw_runtime_payload_and_never_grants_authority() {
    let mut fixture = FixtureV3::new("privacy");
    fixture.append_support();
    let checkpoint = fixture.checkpoint(1);
    let visible = String::from_utf8_lossy(&checkpoint);
    assert!(!visible.contains("CellA17"));
    assert!(!visible.contains("continue_session"));
    let restored = decode_generation_checkpoint_v3(&checkpoint).expect("decode");
    assert!(!restored.execution_authority());
}
