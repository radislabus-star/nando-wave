use std::collections::BTreeSet;

use nando_operator_learning::{
    K2CompositionErrorV1, K2CompositionLearnedEffectV1, K2InquirySelectionPrecommitV1,
    K2UncertaintyPrivateSafetyDispositionV1, K2UncertaintyProbeRequestV1,
    K2UncertaintySafetyRequestV1, self_formed_grammar_root_v1, verify_inquiry_selection_v1,
    verify_self_formed_private_safety_v1,
};

use super::fixture::{R7Fixture, root_hash};
use super::ledger::ControlLedger;

pub fn run(fixture: &R7Fixture, ledger: &mut ControlLedger) {
    let mut probe_json = serde_json::to_value(&fixture.probe_request).expect("probe JSON");
    probe_json
        .as_object_mut()
        .expect("probe object")
        .insert("probe_role".to_owned(), serde_json::json!("preferred"));
    let error = serde_json::from_value::<K2UncertaintyProbeRequestV1>(probe_json)
        .expect_err("probe role accepted");
    assert!(
        error.to_string().contains("unknown field") && error.to_string().contains("probe_role")
    );
    ledger.pass("15", "probe_role_field_rejected");

    let mut omitted_page = fixture.probe_output.pages[0].clone();
    omitted_page.dispositions.pop();
    omitted_page
        .reseal()
        .expect("short frontier page remains locally valid");
    let mut omitted_output = fixture.probe_output.clone();
    omitted_output.pages[0] = omitted_page;
    assert_invalid(
        omitted_output.reseal(),
        &["self_formed_probe_output_invalid"],
    );
    ledger.pass("16", "omitted_raw_probe_detected");

    let mut extra_page = fixture.probe_output.pages[0].clone();
    extra_page
        .dispositions
        .push(extra_page.dispositions[0].clone());
    assert_invalid(
        extra_page.reseal(),
        &["self_formed_frontier_page_size_invalid"],
    );
    ledger.pass("17", "extra_non_derived_probe_rejected");

    let mut metadata_mismatch = fixture.selected_disposition().clone();
    metadata_mismatch.equivalence_key.cost_units = metadata_mismatch
        .equivalence_key
        .cost_units
        .saturating_add(1);
    metadata_mismatch
        .equivalence_key
        .reseal()
        .expect("reseal supplied metadata");
    assert_invalid(
        metadata_mismatch.reseal(),
        &["self_formed_raw_probe_invalid"],
    );
    ledger.pass("18", "supplied_safety_cost_mismatch_rejected");

    let selected = fixture.selected_disposition();
    let foreign_effect = K2CompositionLearnedEffectV1::RemoveFile {
        path: "outside/frozen-vocabulary".to_owned(),
    };
    let safety_request = K2UncertaintySafetyRequestV1::seal(
        fixture.preverification.receipt_root_sha256.clone(),
        selected.probe.clone(),
        foreign_effect,
        fixture.public_case.vocabulary.clone(),
        self_formed_grammar_root_v1(&fixture.public_case.vocabulary).expect("grammar root"),
        root_hash("out-of-tree-sandbox"),
        root_hash("safety-owner"),
    )
    .expect("out-of-tree safety request");
    let veto = verify_self_formed_private_safety_v1(&safety_request).expect("safety veto");
    assert_eq!(
        veto.disposition,
        K2UncertaintyPrivateSafetyDispositionV1::GrammarVeto
    );
    ledger.pass("19", "out_of_tree_probe_vetoed");

    let mut risk = selected.robust_accounting.effects[0].accounting.clone();
    risk.risk_units = risk.risk_units.saturating_add(1);
    assert_invalid(risk.validate(), &["self_formed_risk_cost_invalid"]);
    ledger.pass("20", "risk_formula_mutation_detected");

    let mut cost = selected.robust_accounting.effects[0].accounting.clone();
    cost.cost_units = cost.cost_units.saturating_add(1);
    assert_invalid(cost.validate(), &["self_formed_risk_cost_invalid"]);
    ledger.pass("21", "cost_formula_mutation_detected");

    let step = &fixture.tournament.steps[0];
    let mut scorer_mutation: K2InquirySelectionPrecommitV1 = step.precommit.clone();
    scorer_mutation.selected_probe_root_sha256 = step
        .request
        .public_case
        .probes
        .iter()
        .find(|probe| probe.probe_root_sha256 != scorer_mutation.selected_probe_root_sha256)
        .expect("alternate scorer probe")
        .probe_root_sha256
        .clone();
    scorer_mutation.reseal().expect("reseal scorer mutation");
    assert_invalid(
        verify_inquiry_selection_v1(root_hash("preverifier"), &step.request, &scorer_mutation),
        &["inquiry_selection_verification_mismatch"],
    );
    ledger.pass("22", "predecessor_scorer_mutation_detected");

    let absolute_classes = fixture
        .probe_output
        .pages
        .iter()
        .flat_map(|page| &page.dispositions)
        .map(|disposition| {
            disposition
                .predictions
                .iter()
                .map(|prediction| prediction.observable_outcome_root_sha256.clone())
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>();
    assert_ne!(
        absolute_classes.len(),
        fixture.probe_output.frontier.classes.len(),
        "absolute outcome roots accidentally became the scorer quotient"
    );
    ledger.pass("T1", "absolute_post_root_quotient_rejected");

    let mut top_k = fixture.probe_output.frontier.clone();
    top_k.classes.truncate(8);
    top_k.representative_probe_roots_sha256 = top_k
        .classes
        .iter()
        .map(|class| class.representative_probe_root_sha256.clone())
        .collect();
    top_k.representative_probe_roots_sha256.sort();
    assert_invalid(top_k.reseal(), &["self_formed_frontier_invalid"]);
    ledger.pass("T2", "representative_top_k_rejected");

    let mut broken_step = fixture
        .tournament
        .steps
        .last()
        .expect("final tournament step")
        .clone();
    if let Some(first) = broken_step.filler_probe_roots_sha256.first().cloned() {
        broken_step.filler_probe_roots_sha256.push(first);
    } else {
        broken_step.active_probe_roots_sha256.pop();
    }
    assert_invalid(
        broken_step.reseal(),
        &[
            "self_formed_tournament_filler_roots_invalid",
            "self_formed_tournament_active_roots_invalid",
            "self_formed_tournament_request_members_invalid",
            "self_formed_tournament_final_invalid",
        ],
    );
    ledger.pass("T3", "tournament_omission_or_duplicate_filler_rejected");

    let mut wrong_winner = fixture.tournament.tournament.clone();
    wrong_winner.tournament_winner_probe_root_sha256 = fixture
        .probe_output
        .frontier
        .representative_probe_roots_sha256
        .iter()
        .find(|root| {
            root.as_str()
                != wrong_winner
                    .direct_winner
                    .selected_probe_root_sha256
                    .as_str()
        })
        .expect("alternate direct winner")
        .clone();
    assert_invalid(
        wrong_winner.reseal(),
        &["self_formed_tournament_direct_winner_mismatch"],
    );
    ledger.pass("T4", "tournament_direct_winner_mismatch_rejected");
}

fn assert_invalid<T>(result: Result<T, K2CompositionErrorV1>, expected: &[&str]) {
    let error = result.err().expect("control unexpectedly accepted");
    let message = error.to_string();
    assert!(
        expected.iter().any(|code| message.contains(code)),
        "expected one of {expected:?}, got {message}"
    );
}
