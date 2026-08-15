use nando_operator_learning::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2UncertaintyBatchJournalEventKindV1,
    K2UncertaintyBatchJournalFaultV1, K2UncertaintyBatchJournalV1, K2UncertaintyDomainVocabularyV1,
    K2UncertaintyPublicCaseV1, K2UncertaintyResourceTerminalKindV1,
    K2UncertaintyResourceTerminalV1, K2UncertaintySplitV1, K2UncertaintySupportSetV1,
};

use super::fixture::{R7Fixture, root_hash};
use super::ledger::ControlLedger;

pub fn run(fixture: &R7Fixture, ledger: &mut ControlLedger) {
    reject_development_evidence_in_confirm_packet(fixture);
    ledger.pass("30", "development_root_in_confirm_packet_rejected");

    reject_every_authority_bit();
    ledger.pass("31", "authority_true_rejected");

    reject_cleanup_before_observation_and_terminal(fixture);
    ledger.pass("32", "cleanup_before_publication_rejected");

    verify_all_resource_terminals(fixture);
    verify_journal_fault_restart_projections(fixture);
}

fn reject_development_evidence_in_confirm_packet(fixture: &R7Fixture) {
    let development = &fixture.public_case.vocabulary;
    let confirm_vocabulary = K2UncertaintyDomainVocabularyV1::seal(
        development.experiment_id_sha256.clone(),
        development.case_id_sha256.clone(),
        K2UncertaintySplitV1::Confirm,
        development.generator_schema_root_sha256.clone(),
        development.opaque_action_roots_sha256.clone(),
        development.path_atoms.clone(),
        development.content_atoms.clone(),
    )
    .expect("construct mixed confirm vocabulary");
    let confirm_support = K2UncertaintySupportSetV1::seal(
        confirm_vocabulary.case_id_sha256.clone(),
        confirm_vocabulary.vocabulary_root_sha256.clone(),
        fixture.public_case.support.observations.clone(),
    )
    .expect("construct mixed confirm support");
    assert_error(
        K2UncertaintyPublicCaseV1::seal(confirm_vocabulary, confirm_support),
        "self_formed_public_case_invalid",
    );
}

fn reject_every_authority_bit() {
    for index in 0..7 {
        let mut authority = K2CompositionAuthorityBoundaryV1::denied();
        match index {
            0 => authority.natural_k2_authority = true,
            1 => authority.k1_registry_mutated = true,
            2 => authority.product_authority = true,
            3 => authority.phase_memory_mutated = true,
            4 => authority.law_certificate_issued = true,
            5 => authority.package_activated = true,
            6 => authority.deployment_authority = true,
            _ => unreachable!(),
        }
        assert_error(authority.validate(), "authority_boundary_violated");
    }
}

fn reject_cleanup_before_observation_and_terminal(fixture: &R7Fixture) {
    let order = execution_order(fixture);
    let journal_root = fixture.root.join("boundary-cleanup-journal");
    let mut journal = K2UncertaintyBatchJournalV1::create(
        &journal_root,
        fixture.generated.public.experiment_id_sha256.clone(),
        order.clone(),
    )
    .expect("create cleanup boundary journal");

    append_batch_barrier(&mut journal);
    assert_error(
        journal.append(
            K2UncertaintyBatchJournalEventKindV1::CleanupFrozen,
            None,
            root_hash("cleanup-owner"),
            root_hash("cleanup-request-before-observation"),
            root_hash("cleanup-payload-before-observation"),
        ),
        "self_formed_batch_journal_event_order_invalid",
    );

    for (case_sequence, case_id) in order.into_iter().enumerate() {
        for (event_sequence, kind) in [
            K2UncertaintyBatchJournalEventKindV1::ProbeDispatched,
            K2UncertaintyBatchJournalEventKindV1::ProbeObserved,
            K2UncertaintyBatchJournalEventKindV1::ModelsUpdated,
        ]
        .into_iter()
        .enumerate()
        {
            journal
                .append(
                    kind,
                    Some(case_id.clone()),
                    root_hash("case-owner"),
                    root_hash(&format!("case-{case_sequence}-request-{event_sequence}")),
                    root_hash(&format!("case-{case_sequence}-payload-{event_sequence}")),
                )
                .expect("append complete case route");
        }
    }
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ControlsFrozen,
            None,
            root_hash("controls-owner"),
            root_hash("controls-request"),
            root_hash("controls-payload"),
        )
        .expect("append controls terminal precursor");
    assert_error(
        journal.append(
            K2UncertaintyBatchJournalEventKindV1::CleanupFrozen,
            None,
            root_hash("cleanup-owner"),
            root_hash("cleanup-request-before-terminal"),
            root_hash("cleanup-payload-before-terminal"),
        ),
        "self_formed_batch_journal_event_order_invalid",
    );
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::TerminalFrozen,
            None,
            root_hash("terminal-owner"),
            root_hash("terminal-request"),
            root_hash("terminal-payload"),
        )
        .expect("append terminal publication");
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::CleanupFrozen,
            None,
            root_hash("cleanup-owner"),
            root_hash("cleanup-request-after-terminal"),
            root_hash("cleanup-payload-after-terminal"),
        )
        .expect("append legal cleanup publication");
    assert!(journal.projection().cleanup_frozen);
}

fn verify_all_resource_terminals(fixture: &R7Fixture) {
    for (kind, case_id) in [
        (
            K2UncertaintyResourceTerminalKindV1::CountOverflow,
            Some(fixture.public_case.vocabulary.case_id_sha256.clone()),
        ),
        (
            K2UncertaintyResourceTerminalKindV1::ProtocolBytesExhausted,
            Some(fixture.public_case.vocabulary.case_id_sha256.clone()),
        ),
        (
            K2UncertaintyResourceTerminalKindV1::ResidentMemoryExhausted,
            Some(fixture.public_case.vocabulary.case_id_sha256.clone()),
        ),
        (
            K2UncertaintyResourceTerminalKindV1::CaseDeadlineExceeded,
            Some(fixture.public_case.vocabulary.case_id_sha256.clone()),
        ),
        (
            K2UncertaintyResourceTerminalKindV1::BatchDeadlineExceeded,
            None,
        ),
    ] {
        let terminal = K2UncertaintyResourceTerminalV1::seal(case_id.clone(), kind, 11, 10)
            .expect("seal resource terminal");
        terminal.validate().expect("validate resource terminal");
        assert_error(
            K2UncertaintyResourceTerminalV1::seal(case_id, kind, 10, 10),
            "self_formed_resource_terminal_invalid",
        );
    }
}

fn verify_journal_fault_restart_projections(fixture: &R7Fixture) {
    let order = execution_order(fixture);

    let before_root = fixture.root.join("boundary-before-rename-journal");
    let before_id = root_hash("boundary-before-rename-experiment");
    let mut before =
        K2UncertaintyBatchJournalV1::create(&before_root, before_id.clone(), order.clone())
            .expect("create before-rename journal");
    let before_projection = before.projection();
    assert_error(
        before.append_with_fault(
            K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
            None,
            root_hash("fault-owner"),
            root_hash("fault-request-before-rename"),
            root_hash("fault-payload-before-rename"),
            K2UncertaintyBatchJournalFaultV1::BeforeRename,
        ),
        "self_formed_batch_journal_fault_before_rename",
    );
    let reopened_before = K2UncertaintyBatchJournalV1::open_existing(&before_root, before_id)
        .expect("reopen before-rename journal");
    assert_eq!(reopened_before.projection(), before_projection);

    let after_root = fixture.root.join("boundary-after-rename-journal");
    let after_id = root_hash("boundary-after-rename-experiment");
    let mut after = K2UncertaintyBatchJournalV1::create(&after_root, after_id.clone(), order)
        .expect("create after-rename journal");
    assert_error(
        after.append_with_fault(
            K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
            None,
            root_hash("fault-owner"),
            root_hash("fault-request-after-rename"),
            root_hash("fault-payload-after-rename"),
            K2UncertaintyBatchJournalFaultV1::AfterRename,
        ),
        "self_formed_batch_journal_fault_after_rename",
    );
    let reopened_after = K2UncertaintyBatchJournalV1::open_existing(&after_root, after_id)
        .expect("reopen after-rename journal");
    assert_eq!(reopened_after.projection().event_count, 1);
    assert_eq!(
        reopened_after.projection().last_kind,
        Some(K2UncertaintyBatchJournalEventKindV1::BatchFrozen)
    );
}

fn append_batch_barrier(journal: &mut K2UncertaintyBatchJournalV1) {
    for (sequence, kind) in [
        K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
        K2UncertaintyBatchJournalEventKindV1::CasesGenerated,
        K2UncertaintyBatchJournalEventKindV1::ModelSetsFrozen,
        K2UncertaintyBatchJournalEventKindV1::ProbeSetsFrozen,
        K2UncertaintyBatchJournalEventKindV1::SelectionsFrozen,
        K2UncertaintyBatchJournalEventKindV1::AllCasesPrecommitted,
    ]
    .into_iter()
    .enumerate()
    {
        journal
            .append(
                kind,
                None,
                root_hash("barrier-owner"),
                root_hash(&format!("barrier-request-{sequence}")),
                root_hash(&format!("barrier-payload-{sequence}")),
            )
            .expect("append batch barrier");
    }
}

fn execution_order(fixture: &R7Fixture) -> Vec<String> {
    fixture
        .generated
        .public
        .cases
        .iter()
        .map(|case| case.vocabulary.case_id_sha256.clone())
        .collect()
}

fn assert_error<T>(result: Result<T, K2CompositionErrorV1>, code: &str) {
    let error = result.err().expect("boundary control accepted");
    assert!(error.to_string().contains(code), "wrong error: {error}");
}
