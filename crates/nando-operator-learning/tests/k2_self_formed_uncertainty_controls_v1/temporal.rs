use std::fs;
use std::path::PathBuf;

use nando_operator_learning::{
    K2InquiryObserverRequestV1, K2InquirySelectorRequestV1, K2UncertaintyBatchJournalEventKindV1,
    K2UncertaintyBatchJournalV1, composition_bytes_v1,
};

use super::fixture::{R7Fixture, root_hash};
use super::ledger::ControlLedger;

pub fn run(fixture: &R7Fixture, ledger: &mut ControlLedger) {
    let public_messages = public_messages(fixture);
    assert!(
        public_messages
            .iter()
            .all(|message| !message.contains("\"seed_bytes\""))
    );
    ledger.pass("23", "nonce_bytes_absent_from_public_requests");

    let private_root = &fixture.private_case.private_case_root_sha256;
    assert!(public_messages.iter().all(|message| {
        !message.contains("\"mapping\"")
            && !message.contains("\"topology_family\"")
            && !message.contains(private_root)
    }));
    ledger.pass("24", "private_mapping_absent_from_public_requests");

    let mut selector_json =
        serde_json::to_value(&fixture.tournament.steps[0].request).expect("selector JSON");
    selector_json
        .as_object_mut()
        .expect("selector object")
        .insert(
            "post_outcome".to_owned(),
            serde_json::json!(root_hash("outcome")),
        );
    let error = serde_json::from_value::<K2InquirySelectorRequestV1>(selector_json)
        .expect_err("post-outcome selector request accepted");
    assert!(
        error.to_string().contains("unknown field") && error.to_string().contains("post_outcome")
    );
    ledger.pass("25", "post_outcome_selection_rejected");

    let journal_root = fixture.root.join("temporal-journal");
    let order = fixture
        .generated
        .public
        .cases
        .iter()
        .map(|case| case.vocabulary.case_id_sha256.clone())
        .collect::<Vec<_>>();
    let mut journal = K2UncertaintyBatchJournalV1::create(
        &journal_root,
        fixture.generated.public.experiment_id_sha256.clone(),
        order.clone(),
    )
    .expect("temporal journal");
    let early = journal.append(
        K2UncertaintyBatchJournalEventKindV1::ProbeDispatched,
        Some(order[0].clone()),
        root_hash("owner"),
        root_hash("request"),
        root_hash("payload"),
    );
    assert_error(early, "self_formed_batch_journal_event_order_invalid");
    ledger.pass("26", "dispatch_before_batch_barrier_rejected");

    for kind in [
        K2UncertaintyBatchJournalEventKindV1::BatchFrozen,
        K2UncertaintyBatchJournalEventKindV1::CasesGenerated,
        K2UncertaintyBatchJournalEventKindV1::ModelSetsFrozen,
        K2UncertaintyBatchJournalEventKindV1::ProbeSetsFrozen,
        K2UncertaintyBatchJournalEventKindV1::SelectionsFrozen,
        K2UncertaintyBatchJournalEventKindV1::AllCasesPrecommitted,
    ] {
        journal
            .append(
                kind,
                None,
                root_hash("owner"),
                root_hash("request"),
                root_hash("payload"),
            )
            .expect("barrier event");
    }
    journal
        .append(
            K2UncertaintyBatchJournalEventKindV1::ProbeDispatched,
            Some(order[0].clone()),
            root_hash("owner"),
            root_hash("request"),
            root_hash("payload"),
        )
        .expect("first dispatch");
    let redispatch = journal.append(
        K2UncertaintyBatchJournalEventKindV1::ProbeDispatched,
        Some(order[0].clone()),
        root_hash("owner"),
        root_hash("request"),
        root_hash("payload"),
    );
    assert_error(redispatch, "self_formed_batch_journal_event_order_invalid");
    ledger.pass("27", "same_identity_redispatch_rejected");

    let observer = K2InquiryObserverRequestV1::seal(
        fixture.private_case.case_id_sha256.clone(),
        fixture
            .selected_disposition()
            .probe
            .probe_root_sha256
            .clone(),
        root_hash("observer"),
    )
    .expect("observer request");
    let mut observer_json = serde_json::to_value(&observer).expect("observer JSON");
    observer_json
        .as_object_mut()
        .expect("observer object")
        .insert("worker_stdout".to_owned(), serde_json::json!("forbidden"));
    let error = serde_json::from_value::<K2InquiryObserverRequestV1>(observer_json)
        .expect_err("worker stdout accepted by observer");
    assert!(
        error.to_string().contains("unknown field") && error.to_string().contains("worker_stdout")
    );
    ledger.pass("28", "observer_worker_stdout_field_rejected");

    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/k2_goal_environment/learned_composition/self_formed_uncertainty");
    let mut verifier_source = String::new();
    for file in [
        "final_verifier.rs",
        "final_verifier_frontier.rs",
        "final_verifier_induction.rs",
        "final_verifier_model.rs",
        "final_verifier_selection.rs",
    ] {
        verifier_source
            .push_str(&fs::read_to_string(source_root.join(file)).expect("verifier source"));
    }
    for forbidden in [
        "generator::",
        "learner::",
        "probe::",
        "tournament::",
        "integration::",
        "materialize_self_formed",
        "enumerate_self_formed",
        "run_self_formed_tournament",
        "prepare_self_formed_dispatch",
        "publish_self_formed",
        "reopen_self_formed",
    ] {
        assert!(
            !verifier_source.contains(forbidden),
            "final verifier imported forbidden route: {forbidden}"
        );
    }
    ledger.pass("29", "final_verifier_forbidden_imports_absent");
}

fn public_messages(fixture: &R7Fixture) -> Vec<String> {
    let mut encoded = vec![
        serde_json::to_string(&fixture.learner_request).expect("learner public message"),
        serde_json::to_string(&fixture.probe_request).expect("probe public message"),
    ];
    for step in &fixture.tournament.steps {
        encoded.push(
            String::from_utf8(composition_bytes_v1(&step.request).expect("selector bytes"))
                .expect("selector JSON"),
        );
    }
    encoded
}

fn assert_error<T>(result: Result<T, nando_operator_learning::K2CompositionErrorV1>, code: &str) {
    let error = result.err().expect("temporal control accepted");
    assert!(error.to_string().contains(code), "wrong error: {error}");
}
