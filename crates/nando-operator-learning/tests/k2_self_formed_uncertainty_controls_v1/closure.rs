use std::collections::{BTreeMap, BTreeSet};

use nando_operator_learning::{
    K2CompositionErrorV1, K2CompositionTreeManifestV1, K2UncertaintyClosureDispositionV1,
    K2UncertaintyClosurePlannerRequestV1, K2UncertaintyRawProbeDispositionV1, composition_root_v1,
    plan_self_formed_uncertainty_closure_v1,
};

use super::fixture::{R7Fixture, root_hash};

pub fn run() {
    let fixture = R7Fixture::new();
    let representatives = representative_dispositions(&fixture);
    let actual = plan_self_formed_uncertainty_closure_v1(&closure_request(
        &fixture,
        representatives.clone(),
    ))
    .expect("actual closure census");
    let expected_candidates = if actual.completion_required {
        actual.representative_count.saturating_sub(1)
    } else {
        0
    };
    assert_eq!(actual.candidate_count, expected_candidates);
    match actual.disposition {
        K2UncertaintyClosureDispositionV1::SingleProbe => {
            assert_eq!(actual.first_partition_sizes, [1, 1, 1, 1]);
            assert!(actual.selected_second_probe_root_sha256.is_none());
        }
        K2UncertaintyClosureDispositionV1::TwoProbe => {
            assert_eq!(
                actual.selected_joint_partition_sizes,
                Some(vec![1, 1, 1, 1])
            );
        }
        K2UncertaintyClosureDispositionV1::ClosureUnavailable => {
            panic!("development case has no bounded closure")
        }
    }

    let manifests = distinct_manifests(&representatives);
    verify_single_probe(&fixture, &representatives, &manifests);
    verify_two_probe_and_order_invariance(&fixture, &representatives, &manifests);
    verify_unavailable_and_omission_rejection(&fixture, &representatives, &manifests);
}

fn verify_single_probe(
    fixture: &R7Fixture,
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    manifests: &[K2CompositionTreeManifestV1],
) {
    let mut rewritten = representatives.to_vec();
    let first = first_representative_mut(fixture, &mut rewritten);
    rewrite_partition(first, [0, 1, 2, 3], manifests);
    let census = plan_self_formed_uncertainty_closure_v1(&closure_request(fixture, rewritten))
        .expect("single-probe closure census");
    assert_eq!(
        census.disposition,
        K2UncertaintyClosureDispositionV1::SingleProbe
    );
    assert_eq!(census.first_partition_sizes, [1, 1, 1, 1]);
    assert!(!census.completion_required);
    assert_eq!(census.candidate_count, 0);
    assert!(census.candidates.is_empty());
    assert!(census.selected_second_probe_root_sha256.is_none());
}

fn verify_two_probe_and_order_invariance(
    fixture: &R7Fixture,
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    manifests: &[K2CompositionTreeManifestV1],
) {
    let first_root = first_probe_root(fixture);
    let second_root = representatives
        .iter()
        .map(|value| &value.probe.probe_root_sha256)
        .find(|root| *root != first_root)
        .expect("second representative")
        .clone();
    let mut rewritten = representatives.to_vec();
    for representative in &mut rewritten {
        let groups = if representative.probe.probe_root_sha256 == second_root {
            [0, 1, 0, 1]
        } else {
            [0, 0, 1, 1]
        };
        rewrite_partition(representative, groups, manifests);
    }
    let request = closure_request(fixture, rewritten.clone());
    let census =
        plan_self_formed_uncertainty_closure_v1(&request).expect("two-probe closure census");
    assert_eq!(
        census.disposition,
        K2UncertaintyClosureDispositionV1::TwoProbe
    );
    assert_eq!(census.selected_second_probe_root_sha256, Some(second_root));
    assert_eq!(
        census.selected_joint_partition_sizes,
        Some(vec![1, 1, 1, 1])
    );
    assert_eq!(
        census.candidate_count,
        census.representative_count.saturating_sub(1)
    );

    rewritten.reverse();
    let reordered = plan_self_formed_uncertainty_closure_v1(&closure_request(fixture, rewritten))
        .expect("order-invariant closure census");
    assert_eq!(census.census_root_sha256, reordered.census_root_sha256);
    assert_eq!(
        census.candidate_denominator_root_sha256,
        reordered.candidate_denominator_root_sha256
    );
}

fn verify_unavailable_and_omission_rejection(
    fixture: &R7Fixture,
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    manifests: &[K2CompositionTreeManifestV1],
) {
    let mut rewritten = representatives.to_vec();
    for representative in &mut rewritten {
        rewrite_partition(representative, [0, 0, 1, 1], manifests);
    }
    let mut census = plan_self_formed_uncertainty_closure_v1(&closure_request(fixture, rewritten))
        .expect("unavailable closure census");
    assert_eq!(
        census.disposition,
        K2UncertaintyClosureDispositionV1::ClosureUnavailable
    );
    assert!(census.selected_second_probe_root_sha256.is_none());
    assert!(census.selected_joint_partition_sizes.is_none());
    assert_eq!(
        census.candidate_count,
        census.representative_count.saturating_sub(1)
    );

    census.candidates.pop().expect("candidate to omit");
    census.candidate_count = census.candidate_count.saturating_sub(1);
    assert_error(
        census.reseal(),
        "self_formed_closure_candidates_not_canonical",
        "self_formed_closure_census_invalid",
    );
}

fn closure_request(
    fixture: &R7Fixture,
    representatives: Vec<K2UncertaintyRawProbeDispositionV1>,
) -> K2UncertaintyClosurePlannerRequestV1 {
    K2UncertaintyClosurePlannerRequestV1::seal(
        fixture.public_case.vocabulary.case_id_sha256.clone(),
        fixture.probe_output.frontier.frontier_root_sha256.clone(),
        fixture.tournament.tournament.tournament_root_sha256.clone(),
        first_probe_root(fixture).clone(),
        representatives,
        root_hash("closure-planner"),
    )
    .expect("closure planner request")
}

fn representative_dispositions(fixture: &R7Fixture) -> Vec<K2UncertaintyRawProbeDispositionV1> {
    let expected = fixture
        .probe_output
        .frontier
        .representative_probe_roots_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut representatives = fixture
        .probe_output
        .pages
        .iter()
        .flat_map(|page| &page.dispositions)
        .filter(|value| expected.contains(&value.probe.probe_root_sha256))
        .cloned()
        .collect::<Vec<_>>();
    representatives.sort_by(|left, right| {
        left.probe
            .probe_root_sha256
            .cmp(&right.probe.probe_root_sha256)
    });
    assert_eq!(representatives.len(), expected.len());
    representatives
}

fn distinct_manifests(
    representatives: &[K2UncertaintyRawProbeDispositionV1],
) -> Vec<K2CompositionTreeManifestV1> {
    let manifests = representatives
        .iter()
        .flat_map(|value| &value.predictions)
        .map(|prediction| {
            (
                prediction.predicted_post_manifest.tree_root_sha256.clone(),
                prediction.predicted_post_manifest.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .take(4)
        .collect::<Vec<_>>();
    assert_eq!(manifests.len(), 4, "four distinct public outcomes required");
    manifests
}

fn rewrite_partition(
    disposition: &mut K2UncertaintyRawProbeDispositionV1,
    groups: [usize; 4],
    manifests: &[K2CompositionTreeManifestV1],
) {
    for (prediction, group) in disposition.predictions.iter_mut().zip(groups) {
        prediction.predicted_post_manifest = manifests[group].clone();
        prediction.observable_outcome_root_sha256 = composition_root_v1(&(
            "nando.k2-inquiry-observable-exact-manifest.v1",
            &prediction.predicted_post_manifest,
        ))
        .expect("observable root");
        prediction.reseal().expect("reseal prediction");
    }
    disposition.equivalence_key.pairwise_outcome_equal = [
        groups[0] == groups[1],
        groups[0] == groups[2],
        groups[0] == groups[3],
        groups[1] == groups[2],
        groups[1] == groups[3],
        groups[2] == groups[3],
    ];
    disposition
        .equivalence_key
        .reseal()
        .expect("reseal equivalence key");
    disposition.reseal().expect("reseal disposition");
}

fn first_representative_mut<'a>(
    fixture: &R7Fixture,
    representatives: &'a mut [K2UncertaintyRawProbeDispositionV1],
) -> &'a mut K2UncertaintyRawProbeDispositionV1 {
    let first = first_probe_root(fixture);
    representatives
        .iter_mut()
        .find(|value| &value.probe.probe_root_sha256 == first)
        .expect("first representative")
}

fn first_probe_root(fixture: &R7Fixture) -> &String {
    &fixture
        .tournament
        .tournament
        .tournament_winner_probe_root_sha256
}

fn assert_error<T>(result: Result<T, K2CompositionErrorV1>, primary: &str, fallback: &str) {
    let error = result.err().expect("invalid closure census accepted");
    let message = error.to_string();
    assert!(
        message.contains(primary) || message.contains(fallback),
        "wrong error: {error}"
    );
}
