use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use nando_operator_learning::{
    K2_UNCERTAINTY_CONFIRM_CASES_V1, K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_RELATIVE_PATH_V1,
    K2_UNCERTAINTY_CONFIRM_NONCE_RELATIVE_PATH_V1, K2UncertaintyAuthorizationSlotLedgerV1,
    K2UncertaintyConfirmArtifactFaultV1, K2UncertaintyConfirmAttemptDescriptorV1,
    K2UncertaintyConfirmAttemptEventKindV1, K2UncertaintyConfirmAttemptJournalFaultV1,
    K2UncertaintyConfirmAttemptJournalV1, K2UncertaintyConfirmAttemptPhaseV1,
    K2UncertaintyConfirmGeneratorRequestV1, K2UncertaintyConfirmGeneratorResponseV1,
    K2UncertaintyConfirmOwnerReceiptV1, K2UncertaintyConfirmOwnerRequestV1,
    K2UncertaintyConfirmPrivateSplitReceiptV1, K2UncertaintyConfirmStoredArtifactKindV1,
    K2UncertaintyGeneratorRequestV1, composition_sha256_file_v1,
    dispatch_self_formed_generator_once_v1, execute_self_formed_confirm_owner_v1,
    generate_self_formed_confirm_batch_v1, generate_self_formed_development_batch_v1,
    load_confirm_generator_split_receipt_v1, load_retained_confirm_nonce_receipt_v1,
    persist_retained_confirm_nonce_v1, publish_confirm_generator_split_v1,
    publish_confirm_generator_split_with_fault_v1, uncertainty_decode_v1,
};

#[path = "k2_self_formed_uncertainty_confirm_r7h_support/mod.rs"]
mod support;

use support::*;

#[test]
fn r7h_slot_claim_is_global_across_receipts_owners_and_restart() {
    let environment = TestEnvironment::new("slot");
    let ledger_root = environment.root.join("ledger");
    let ledger = K2UncertaintyAuthorizationSlotLedgerV1::open_or_create(&ledger_root)
        .expect("create slot ledger");
    let receipt = authorization("slot-a", "2026-08-16T04:10:00+03:00");
    let owner = root("slot-owner-a");
    let claim = ledger
        .claim(&receipt, owner.clone())
        .expect("first slot claim");
    claim.validate().expect("valid first claim");
    assert_eq!(
        ledger
            .read_slot_claim(&receipt.slot_key().expect("slot key"))
            .expect("read claimed slot"),
        claim
    );

    let reopened = K2UncertaintyAuthorizationSlotLedgerV1::open_or_create(&ledger_root)
        .expect("reopen slot ledger");
    let reused_receipt_error = reopened
        .claim(&receipt, root("slot-owner-b"))
        .expect_err("receipt root must remain consumed");
    assert_eq!(
        reused_receipt_error.to_string(),
        "k2_composition_invalid:self_formed_authorization_receipt_already_used"
    );

    let second_receipt = authorization("slot-a", "2026-08-16T04:11:00+03:00");
    assert_ne!(
        second_receipt.receipt_root_sha256,
        receipt.receipt_root_sha256
    );
    let second_slot_error = reopened
        .claim(&second_receipt, owner)
        .expect_err("same frozen tuple must remain consumed");
    assert_eq!(
        second_slot_error.to_string(),
        "k2_composition_invalid:self_formed_authorization_slot_already_claimed"
    );
    assert_eq!(mode(&ledger_root), 0o700);
    assert_eq!(
        mode(
            &ledger_root
                .join("slots")
                .join(format!("{}.json", claim.slot_key.slot_key_root_sha256))
        ),
        0o400
    );
}

#[test]
fn r7h_restart_freezes_nonce_and_generator_indeterminate_without_replay() {
    let environment = TestEnvironment::new("restart");
    let (descriptor, _) = confirm_descriptor(&environment.root, "nonce-prefix");
    let attempt_root = environment.root.join("attempt-nonce");
    let mut journal =
        K2UncertaintyConfirmAttemptJournalV1::create_exclusive(&attempt_root, descriptor)
            .expect("create nonce attempt");
    append(
        &mut journal,
        K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
        0,
    );
    append(
        &mut journal,
        K2UncertaintyConfirmAttemptEventKindV1::NonceCreated,
        1,
    );
    let retained_nonce_root = root("retained-nonce");
    let mut restarted = K2UncertaintyConfirmAttemptJournalV1::open_existing(&attempt_root)
        .expect("restart nonce attempt");
    let projection = restarted
        .recover_after_restart(
            root("recovery-owner"),
            root("recovery-request"),
            Some(retained_nonce_root),
            None,
        )
        .expect("freeze uncommitted nonce");
    assert_eq!(
        projection.phase,
        K2UncertaintyConfirmAttemptPhaseV1::NonceCreatedUncommitted
    );
    assert_eq!(projection.generator_dispatch_count, 0);
    assert!(projection.terminal);
    assert!(
        restarted
            .append(
                K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched,
                root("owner"),
                root("request"),
                root("payload"),
            )
            .is_err()
    );

    let (descriptor, _) = confirm_descriptor(&environment.root, "dispatch-prefix");
    let dispatch_root = environment.root.join("attempt-dispatch");
    let mut journal =
        K2UncertaintyConfirmAttemptJournalV1::create_exclusive(&dispatch_root, descriptor)
            .expect("create dispatch attempt");
    for (sequence, kind) in [
        K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
        K2UncertaintyConfirmAttemptEventKindV1::NonceCreated,
        K2UncertaintyConfirmAttemptEventKindV1::NonceCommitted,
        K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched,
    ]
    .into_iter()
    .enumerate()
    {
        append(&mut journal, kind, sequence);
    }
    let mut restarted = K2UncertaintyConfirmAttemptJournalV1::open_existing(&dispatch_root)
        .expect("restart dispatched attempt");
    let projection = restarted
        .recover_after_restart(
            root("dispatch-recovery-owner"),
            root("dispatch-recovery-request"),
            None,
            None,
        )
        .expect("freeze missing generator result");
    assert_eq!(
        projection.phase,
        K2UncertaintyConfirmAttemptPhaseV1::GeneratorResultIndeterminate
    );
    assert_eq!(projection.generator_dispatch_count, 1);
    assert!(!projection.cases_generated);

    let (descriptor, _) = confirm_descriptor(&environment.root, "committed-prefix");
    let committed_root = environment.root.join("attempt-committed");
    let mut journal =
        K2UncertaintyConfirmAttemptJournalV1::create_exclusive(&committed_root, descriptor)
            .expect("create committed attempt");
    for (sequence, kind) in [
        K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
        K2UncertaintyConfirmAttemptEventKindV1::NonceCreated,
        K2UncertaintyConfirmAttemptEventKindV1::NonceCommitted,
    ]
    .into_iter()
    .enumerate()
    {
        append(&mut journal, kind, sequence);
    }
    let mut restarted = K2UncertaintyConfirmAttemptJournalV1::open_existing(&committed_root)
        .expect("restart committed attempt");
    let projection = restarted
        .recover_after_restart(
            root("committed-recovery-owner"),
            root("committed-recovery-request"),
            None,
            None,
        )
        .expect("freeze committed pre-dispatch prefix");
    assert_eq!(
        projection.phase,
        K2UncertaintyConfirmAttemptPhaseV1::NonceCommittedUndispatched
    );
    assert_eq!(projection.generator_dispatch_count, 0);
    assert!(projection.terminal);
    assert!(
        restarted
            .append(
                K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched,
                root("committed-replay-owner"),
                root("committed-replay-request"),
                root("committed-replay-payload"),
            )
            .is_err()
    );
}

#[test]
fn r7h_owner_rejects_symlinked_lab_attempt_and_slot_ledger_paths_before_nonce() {
    let environment = TestEnvironment::new("path-negative");
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-confirm-owner"));
    let generator_sha256 = composition_sha256_file_v1(&generator).expect("generator SHA");
    let owner_sha256 = composition_sha256_file_v1(&owner).expect("owner SHA");
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R7H path tests");
    let development_seed = fs::read(seed_path).expect("read frozen development seed");

    let real_lab = environment.root.join("real-lab");
    fs::create_dir(&real_lab).expect("create real lab");
    fs::set_permissions(&real_lab, fs::Permissions::from_mode(0o700)).expect("chmod real lab");
    let linked_lab = environment.root.join("linked-lab");
    symlink(&real_lab, &linked_lab).expect("link lab root");
    let development_request =
        K2UncertaintyGeneratorRequestV1::development(development_seed, generator_sha256.clone())
            .expect("development request");
    let development_descriptor = K2UncertaintyConfirmAttemptDescriptorV1::development_rehearsal(
        root("path-development-experiment"),
        root("path-development-freeze"),
        root("path-development-manifest"),
        owner_sha256.clone(),
        generator_sha256.clone(),
    )
    .expect("development descriptor");
    assert!(
        K2UncertaintyConfirmOwnerRequestV1::development_rehearsal(
            development_descriptor.clone(),
            real_lab.to_string_lossy().into_owned(),
            "../escape".to_owned(),
            generator.to_string_lossy().into_owned(),
            development_request.clone(),
        )
        .is_err(),
        "relative traversal must be rejected by the closed request schema"
    );
    let linked_lab_request = K2UncertaintyConfirmOwnerRequestV1::development_rehearsal(
        development_descriptor.clone(),
        linked_lab.to_string_lossy().into_owned(),
        "attempt".to_owned(),
        generator.to_string_lossy().into_owned(),
        development_request.clone(),
    )
    .expect("linked lab request");
    assert!(
        execute_self_formed_confirm_owner_v1(&linked_lab_request, &owner).is_err(),
        "symlinked lab root must fail closed"
    );
    assert!(!real_lab.join("attempt").exists());

    let external_attempt = environment.root.join("external-attempt");
    K2UncertaintyConfirmAttemptJournalV1::create_exclusive(
        &external_attempt,
        development_descriptor.clone(),
    )
    .expect("external attempt journal");
    symlink(&external_attempt, real_lab.join("attempt")).expect("link attempt root");
    let attempt_link_request = K2UncertaintyConfirmOwnerRequestV1::development_rehearsal(
        development_descriptor,
        real_lab.to_string_lossy().into_owned(),
        "attempt".to_owned(),
        generator.to_string_lossy().into_owned(),
        development_request,
    )
    .expect("attempt link request");
    assert!(
        execute_self_formed_confirm_owner_v1(&attempt_link_request, &owner).is_err(),
        "symlinked attempt root must fail closed"
    );
    assert_eq!(
        K2UncertaintyConfirmAttemptJournalV1::open_existing(&external_attempt)
            .expect("external attempt remains intact")
            .projection()
            .event_count,
        0
    );

    let ledger_root = environment.root.join("real-ledger");
    let authorization = authorization("path-confirm", "2026-08-16T04:13:00+03:00");
    let claim = K2UncertaintyAuthorizationSlotLedgerV1::open_or_create(&ledger_root)
        .expect("create real ledger")
        .claim(&authorization, root("path-slot-owner"))
        .expect("claim fixed test slot");
    symlink(&ledger_root, real_lab.join("ledger-link")).expect("link slot ledger");
    let confirm_descriptor = K2UncertaintyConfirmAttemptDescriptorV1::confirm(
        authorization.experiment_id_sha256.clone(),
        authorization.successor_freeze_root_sha256.clone(),
        authorization.executable_manifest_root_sha256.clone(),
        owner_sha256,
        generator_sha256,
        &authorization,
        &claim,
    )
    .expect("confirm descriptor");
    let linked_ledger_request = K2UncertaintyConfirmOwnerRequestV1::confirm(
        confirm_descriptor,
        real_lab.to_string_lossy().into_owned(),
        "confirm-attempt".to_owned(),
        "ledger-link".to_owned(),
        generator.to_string_lossy().into_owned(),
        authorization,
        claim,
    )
    .expect("linked ledger request");
    assert!(
        execute_self_formed_confirm_owner_v1(&linked_ledger_request, &owner).is_err(),
        "symlinked slot ledger must fail before nonce"
    );
    assert!(!real_lab.join("confirm-attempt").exists());
}

#[test]
fn r7h_rehearsal_has_no_slot_nonce_or_sealed_attempt() {
    let environment = TestEnvironment::new("rehearsal");
    let descriptor = K2UncertaintyConfirmAttemptDescriptorV1::development_rehearsal(
        root("rehearsal-experiment"),
        root("successor-freeze"),
        root("executable-manifest"),
        root("confirm-owner"),
        root("generator"),
    )
    .expect("rehearsal descriptor");
    assert_eq!(descriptor.sealed_attempts, 0);
    assert!(descriptor.authorization_receipt_root_sha256.is_none());
    assert!(descriptor.authorization_slot_claim_root_sha256.is_none());
    let attempt_root = environment.root.join("attempt");
    let mut journal =
        K2UncertaintyConfirmAttemptJournalV1::create_exclusive(&attempt_root, descriptor)
            .expect("rehearsal journal");
    append(
        &mut journal,
        K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
        0,
    );
    assert!(
        journal
            .append(
                K2UncertaintyConfirmAttemptEventKindV1::NonceCreated,
                root("owner"),
                root("nonce-request"),
                root("nonce-payload"),
            )
            .is_err()
    );
    append(
        &mut journal,
        K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched,
        1,
    );
    append(
        &mut journal,
        K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated,
        2,
    );
    let projection = journal.projection();
    assert_eq!(projection.sealed_attempts, 0);
    assert_eq!(projection.generator_dispatch_count, 1);
    assert!(projection.cases_generated);
}

#[test]
fn r7h_attempt_journal_is_crash_atomic_at_both_rename_boundaries() {
    let environment = TestEnvironment::new("journal-fault");
    let before_root = environment.root.join("before");
    let mut before = K2UncertaintyConfirmAttemptJournalV1::create_exclusive(
        &before_root,
        rehearsal_descriptor("before"),
    )
    .expect("before journal");
    assert!(
        before
            .append_with_fault(
                K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
                root("owner"),
                root("request"),
                root("payload"),
                K2UncertaintyConfirmAttemptJournalFaultV1::BeforeRename,
            )
            .is_err()
    );
    assert_eq!(
        K2UncertaintyConfirmAttemptJournalV1::open_existing(&before_root)
            .expect("reopen before")
            .projection()
            .event_count,
        0
    );

    let after_root = environment.root.join("after");
    let mut after = K2UncertaintyConfirmAttemptJournalV1::create_exclusive(
        &after_root,
        rehearsal_descriptor("after"),
    )
    .expect("after journal");
    assert!(
        after
            .append_with_fault(
                K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
                root("owner"),
                root("request"),
                root("payload"),
                K2UncertaintyConfirmAttemptJournalFaultV1::AfterRename,
            )
            .is_err()
    );
    assert_eq!(
        K2UncertaintyConfirmAttemptJournalV1::open_existing(&after_root)
            .expect("reopen after")
            .projection()
            .event_count,
        1
    );
}

#[test]
fn r7h_generator_split_is_complete_private_and_fail_closed() {
    let environment = TestEnvironment::new("split");
    let nonce = vec![0xa5; 32];
    let request = K2UncertaintyConfirmGeneratorRequestV1::seal(
        nonce.clone(),
        root("successor-freeze"),
        root("authorization"),
        root("generator"),
    )
    .expect("confirm request");
    let response = generate_self_formed_confirm_batch_v1(&request).expect("confirm response");
    let split_root = environment.root.join("complete");
    let receipt = publish_confirm_generator_split_v1(&split_root, &request, &response)
        .expect("publish complete split");
    assert_eq!(
        load_confirm_generator_split_receipt_v1(&split_root).expect("reopen complete split"),
        receipt
    );
    assert_eq!(receipt.artifacts.len(), 3);
    let private_entry = receipt
        .artifacts
        .iter()
        .find(|entry| entry.kind == K2UncertaintyConfirmStoredArtifactKindV1::PrivateSplit)
        .expect("private split entry");
    let private_bytes =
        fs::read(split_root.join(&private_entry.relative_path)).expect("read private split");
    let private: K2UncertaintyConfirmPrivateSplitReceiptV1 =
        uncertainty_decode_v1(&private_bytes).expect("decode private split");
    assert_eq!(private.artifacts.len(), 32);
    assert_eq!(
        private
            .artifacts
            .iter()
            .filter(|entry| {
                entry.kind == K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable
            })
            .count(),
        16
    );
    assert_eq!(
        private
            .artifacts
            .iter()
            .filter(|entry| entry.kind == K2UncertaintyConfirmStoredArtifactKindV1::FinalTruth)
            .count(),
        16
    );
    for entry in receipt.artifacts.iter().chain(private.artifacts.iter()) {
        assert_eq!(mode(&split_root.join(&entry.relative_path)), entry.mode);
    }
    for entry in &private.artifacts {
        if entry.kind == K2UncertaintyConfirmStoredArtifactKindV1::ResolverTable {
            let text = fs::read_to_string(split_root.join(&entry.relative_path))
                .expect("read resolver table");
            assert!(!text.contains("topology_family"));
            assert!(!text.contains("matched_pair"));
        }
    }
    for entry in receipt
        .artifacts
        .iter()
        .filter(|entry| entry.kind != K2UncertaintyConfirmStoredArtifactKindV1::PrivateSplit)
    {
        let bytes = fs::read(split_root.join(&entry.relative_path)).expect("read public artifact");
        assert!(!contains_subslice(&bytes, &nonce));
    }

    let partial_root = environment.root.join("partial");
    assert!(
        publish_confirm_generator_split_with_fault_v1(
            &partial_root,
            &request,
            &response,
            K2UncertaintyConfirmArtifactFaultV1::AfterRename(5),
        )
        .is_err()
    );
    assert!(!partial_root.join("split-receipt.json").exists());
    assert!(load_confirm_generator_split_receipt_v1(&partial_root).is_err());
    assert!(publish_confirm_generator_split_v1(&partial_root, &request, &response).is_err());
}

#[test]
fn r7h_retained_nonce_is_private_hash_only_and_immutable() {
    let environment = TestEnvironment::new("nonce-artifact");
    let attempt_root = environment.root.join("attempt");
    fs::create_dir(&attempt_root).expect("create attempt root");
    fs::set_permissions(&attempt_root, fs::Permissions::from_mode(0o700))
        .expect("chmod attempt root");
    let nonce = [0x5a; 32];
    let receipt =
        persist_retained_confirm_nonce_v1(&attempt_root, &nonce).expect("persist retained nonce");
    assert_eq!(
        mode(&attempt_root.join(K2_UNCERTAINTY_CONFIRM_NONCE_RELATIVE_PATH_V1)),
        0o400
    );
    assert_eq!(
        mode(&attempt_root.join(K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_RELATIVE_PATH_V1)),
        0o400
    );
    assert_eq!(
        load_retained_confirm_nonce_receipt_v1(&attempt_root).expect("load retained nonce receipt"),
        receipt
    );
    let receipt_bytes =
        fs::read(attempt_root.join(K2_UNCERTAINTY_CONFIRM_NONCE_RECEIPT_RELATIVE_PATH_V1))
            .expect("read retained nonce receipt");
    assert!(!contains_subslice(&receipt_bytes, &nonce));
    assert!(persist_retained_confirm_nonce_v1(&attempt_root, &nonce).is_err());
}

#[test]
fn r7h_confirm_pipe_sends_once_without_forbidden_nonce_channels() {
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let generator_sha256 = composition_sha256_file_v1(&generator).expect("generator SHA");
    let nonce = [0xa7; 32];
    let request = K2UncertaintyConfirmGeneratorRequestV1::seal(
        nonce.to_vec(),
        root("pipe-successor"),
        root("pipe-authorization"),
        generator_sha256.clone(),
    )
    .expect("fixed-material Confirm request");
    let mut request_bytes =
        nando_operator_learning::uncertainty_bytes_v1(&request).expect("Confirm request bytes");
    let (response_bytes, pipe_receipt) = dispatch_self_formed_generator_once_v1(
        &generator,
        &generator_sha256,
        &request.request_root_sha256,
        &mut request_bytes,
        Some(&nonce),
    )
    .expect("one anonymous-pipe dispatch");
    assert!(request_bytes.iter().all(|byte| *byte == 0));
    pipe_receipt.validate().expect("valid pipe receipt");
    assert_eq!(pipe_receipt.child_invocations, 1);
    assert_eq!(pipe_receipt.stdin_send_operations, 1);
    assert_eq!(pipe_receipt.request_artifact_writes, 0);
    assert_eq!(pipe_receipt.log_writes, 0);
    assert!(!contains_subslice(&response_bytes, &nonce));
    let response: K2UncertaintyConfirmGeneratorResponseV1 =
        uncertainty_decode_v1(&response_bytes).expect("Confirm generator response");
    response.validate().expect("valid Confirm response");
    assert_eq!(response.public.cases.len(), K2_UNCERTAINTY_CONFIRM_CASES_V1);
    assert_eq!(
        response.private.cases.len(),
        K2_UNCERTAINTY_CONFIRM_CASES_V1
    );
}

#[test]
fn r7h_development_rehearsal_owner_uses_same_pipe_without_slot_or_nonce() {
    let environment = TestEnvironment::new("owner-process");
    let lab_root = environment.root.join("lab");
    fs::create_dir(&lab_root).expect("create owner lab root");
    fs::set_permissions(&lab_root, fs::Permissions::from_mode(0o700))
        .expect("chmod owner lab root");
    let generator = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-generator"));
    let owner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-confirm-owner"));
    let generator_sha256 = composition_sha256_file_v1(&generator).expect("generator SHA");
    let owner_sha256 = composition_sha256_file_v1(&owner).expect("owner SHA");
    let seed_path = std::env::var_os("NANDO_K2_DEVELOPMENT_SEED_PATH")
        .map(PathBuf::from)
        .expect("NANDO_K2_DEVELOPMENT_SEED_PATH is required for R7H owner test");
    let seed = fs::read(seed_path).expect("read development seed");
    let generator_request =
        K2UncertaintyGeneratorRequestV1::development(seed, generator_sha256.clone())
            .expect("development generator request");
    let expected = generate_self_formed_development_batch_v1(&generator_request)
        .expect("expected Development batch");
    let descriptor = K2UncertaintyConfirmAttemptDescriptorV1::development_rehearsal(
        expected.public.experiment_id_sha256.clone(),
        root("owner-successor-freeze"),
        root("owner-executable-manifest"),
        owner_sha256,
        generator_sha256,
    )
    .expect("DevelopmentRehearsal descriptor");
    let request = K2UncertaintyConfirmOwnerRequestV1::development_rehearsal(
        descriptor,
        lab_root.to_string_lossy().into_owned(),
        "attempt".to_owned(),
        generator.to_string_lossy().into_owned(),
        generator_request,
    )
    .expect("owner request");

    let mut child = Command::new(&owner)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn confirm owner");
    child
        .stdin
        .take()
        .expect("owner stdin")
        .write_all(
            &nando_operator_learning::uncertainty_bytes_v1(&request).expect("owner request bytes"),
        )
        .expect("write owner request");
    let output = child.wait_with_output().expect("owner output");
    assert!(
        output.status.success(),
        "owner failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: K2UncertaintyConfirmOwnerReceiptV1 =
        uncertainty_decode_v1(&output.stdout).expect("owner receipt");
    receipt.validate().expect("valid owner receipt");
    assert_eq!(receipt.sealed_attempts, 0);
    assert_eq!(receipt.generator_dispatch_count, 1);
    assert!(receipt.nonce_commitment_sha256.is_none());
    assert!(receipt.split_receipt_root_sha256.is_none());
    let reopened = K2UncertaintyConfirmAttemptJournalV1::open_existing(&lab_root.join("attempt"))
        .expect("reopen owner journal");
    assert_eq!(reopened.projection().generator_dispatch_count, 1);
    assert!(reopened.projection().cases_generated);
    assert!(!lab_root.join("attempt/private/confirm-nonce.bin").exists());

    let mut replay = Command::new(&owner)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn replayed owner");
    replay
        .stdin
        .take()
        .expect("replayed owner stdin")
        .write_all(
            &nando_operator_learning::uncertainty_bytes_v1(&request)
                .expect("replayed owner request bytes"),
        )
        .expect("write replayed owner request");
    let replay_output = replay.wait_with_output().expect("replayed owner output");
    assert!(!replay_output.status.success());
    let replayed = K2UncertaintyConfirmAttemptJournalV1::open_existing(&lab_root.join("attempt"))
        .expect("reopen journal after replay attempt");
    assert_eq!(replayed.projection().generator_dispatch_count, 1);
    assert_eq!(replayed.projection().event_count, 3);
}
