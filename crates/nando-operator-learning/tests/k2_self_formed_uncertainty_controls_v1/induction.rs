use std::collections::BTreeMap;

use nando_operator_learning::{
    K2CompositionErrorV1, K2UncertaintyActionSurvivorsV1, K2UncertaintyLearnerRequestV1,
    K2UncertaintySemanticClassV1, K2UncertaintySupportObservationV1, K2UncertaintySupportSetV1,
    learn_self_formed_uncertainty_v1,
};

use super::fixture::{R7Fixture, root_hash};
use super::ledger::ControlLedger;

pub fn run(fixture: &R7Fixture, ledger: &mut ControlLedger) {
    assert_unknown_field(
        &fixture.learner_request,
        "candidate_models",
        serde_json::json!([]),
    );
    ledger.pass("07", "supplied_candidate_model_field_rejected");
    assert_unknown_field(
        &fixture.learner_request,
        "prepared_models",
        serde_json::to_value(&fixture.learned.model_set.syntactic_models)
            .expect("prepared model value"),
    );
    ledger.pass("08", "prepared_model_list_rejected");

    let mut omitted = fixture.learned.model_set.clone();
    omitted.syntactic_models.pop();
    assert_invalid(
        omitted.validate(),
        "self_formed_model_set_denominator_invalid",
    );
    ledger.pass("09", "omitted_consistent_model_detected");

    let mut extra = fixture.learned.model_set.clone();
    extra
        .syntactic_models
        .push(extra.syntactic_models[0].clone());
    assert_invalid(
        extra.validate(),
        "self_formed_model_set_denominator_invalid",
    );
    ledger.pass("10", "extra_inconsistent_model_detected");

    assert!(
        removed_row_model_count(&fixture.learned) > fixture.learned.model_set.checked_product_count
    );
    ledger.pass("11", "support_row_removal_enlarges_model_set");

    assert!(support_mutation_changes_or_invalidates(fixture));
    ledger.pass(
        "12",
        "support_outcome_mutation_changes_or_invalidates_model_set",
    );

    let survivors = &fixture.learned.model_set.action_survivors[0];
    let mut duplicate_effects = survivors.effects.clone();
    duplicate_effects.push(duplicate_effects[0].clone());
    assert_invalid(
        K2UncertaintyActionSurvivorsV1::seal(
            survivors.opaque_action_root_sha256.clone(),
            duplicate_effects,
        ),
        "self_formed_action_survivors_not_unique",
    );
    ledger.pass("13", "syntactic_duplicate_rejected_before_materialization");

    let semantic = &fixture.learned.model_set.semantic_classes[0];
    let mut duplicate_members = semantic.syntax_member_roots_sha256.clone();
    duplicate_members.push(duplicate_members[0].clone());
    assert_invalid(
        K2UncertaintySemanticClassV1::seal(
            semantic.semantic_signature_root_sha256.clone(),
            duplicate_members,
        ),
        "self_formed_semantic_class_members_invalid",
    );
    ledger.pass("14", "semantic_duplicate_cannot_increase_cardinality");
}

fn removed_row_model_count(
    learned: &nando_operator_learning::K2UncertaintyLearnerResponseV1,
) -> u64 {
    let observation_roots = learned
        .consistency
        .dispositions
        .iter()
        .map(|value| value.support_observation_root_sha256.clone())
        .collect::<std::collections::BTreeSet<_>>();
    observation_roots
        .iter()
        .map(|removed| {
            let mut effects = BTreeMap::<(&str, &str), Vec<_>>::new();
            for disposition in &learned.consistency.dispositions {
                effects
                    .entry((
                        &disposition.opaque_action_root_sha256,
                        &disposition.effect.effect_root_sha256,
                    ))
                    .or_default()
                    .push(disposition);
            }
            let mut action_counts = BTreeMap::<&str, u64>::new();
            for ((action, _), rows) in effects {
                if rows
                    .iter()
                    .all(|row| row.support_observation_root_sha256 == *removed || row.consistent)
                {
                    *action_counts.entry(action).or_default() += 1;
                }
            }
            action_counts.values().copied().product::<u64>()
        })
        .max()
        .expect("support observations")
}

fn support_mutation_changes_or_invalidates(fixture: &R7Fixture) -> bool {
    for (index, row) in fixture.public_case.support.observations.iter().enumerate() {
        if let Some(replacement) =
            fixture
                .public_case
                .support
                .observations
                .iter()
                .find(|candidate| {
                    candidate.opaque_action_root_sha256 == row.opaque_action_root_sha256
                        && candidate.observation_root_sha256 != row.observation_root_sha256
                })
        {
            let mut observations = fixture.public_case.support.observations.clone();
            observations[index] = K2UncertaintySupportObservationV1::seal(
                row.case_id_sha256.clone(),
                row.support_sequence,
                row.pre_manifest.clone(),
                row.opaque_action_root_sha256.clone(),
                replacement.outcome.clone(),
            )
            .expect("mutated observation");
            let support = K2UncertaintySupportSetV1::seal(
                fixture.public_case.vocabulary.case_id_sha256.clone(),
                fixture
                    .public_case
                    .vocabulary
                    .vocabulary_root_sha256
                    .clone(),
                observations,
            )
            .expect("mutated support");
            let request = K2UncertaintyLearnerRequestV1::seal(
                fixture.public_case.vocabulary.clone(),
                support,
                root_hash("mutated-support-learner"),
            )
            .expect("mutated learner request");
            match learn_self_formed_uncertainty_v1(&request) {
                Ok(mutated) => {
                    if mutated.model_set.model_set_root_sha256
                        != fixture.learned.model_set.model_set_root_sha256
                    {
                        return true;
                    }
                }
                Err(_) => return true,
            }
        }
    }
    false
}

fn assert_unknown_field<T: serde::Serialize>(value: &T, field: &str, injected: serde_json::Value) {
    let mut json = serde_json::to_value(value).expect("control JSON");
    json.as_object_mut()
        .expect("control object")
        .insert(field.to_owned(), injected);
    let error = serde_json::from_value::<K2UncertaintyLearnerRequestV1>(json)
        .expect_err("injected field accepted");
    let message = error.to_string();
    assert!(
        message.contains("unknown field") && message.contains(field),
        "wrong field rejection: {message}"
    );
}

pub fn assert_invalid<T>(result: Result<T, K2CompositionErrorV1>, expected: &str) {
    let error = result.err().expect("control unexpectedly accepted");
    let message = error.to_string();
    assert!(
        message.contains(expected),
        "expected {expected}, got {message}"
    );
}
