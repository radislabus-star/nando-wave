use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nando_operator_learning::{
    K2CompositionErrorV1, K2CompositionTreeManifestV1, K2UncertaintyCasePreverificationV2,
    K2UncertaintyClosureCensusV1, K2UncertaintyClosureDispositionV1, K2UncertaintyClosurePlanV1,
    K2UncertaintyClosurePlannerRequestV1, K2UncertaintyClosureVerificationReceiptV1,
    K2UncertaintyClosureVerificationRequestV1, K2UncertaintyRawProbeDispositionV1,
    composition_root_v1, composition_sha256_file_v1, plan_self_formed_uncertainty_closure_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, verify_self_formed_closure_independently_v1,
};

use super::fixture::{R7Fixture, root_hash};

pub fn run() {
    let fixture = R7Fixture::new();
    let representatives = representative_dispositions(&fixture);
    let actual_request = closure_request(&fixture, representatives.clone());
    let actual =
        plan_self_formed_uncertainty_closure_v1(&actual_request).expect("actual closure census");
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
    let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-verifier"));
    let verifier_sha256 = composition_sha256_file_v1(&verifier).expect("closure verifier sha");
    let (verification_request, verification_receipt, closure_plan) =
        verify_closure(&actual_request, &actual, verifier_sha256.clone());
    let process_receipt = run_verifier_process(&verifier, &verification_request);
    assert_eq!(verification_receipt, process_receipt);
    let case_v2 = K2UncertaintyCasePreverificationV2::seal(
        fixture.preverification.clone(),
        verification_request.clone(),
        verification_receipt.clone(),
        Some(closure_plan),
    )
    .expect("case V2 preverification");
    case_v2
        .validate()
        .expect("validate case V2 preverification");

    let mut tampered = actual.clone();
    tampered.first_partition_sizes.reverse();
    if tampered.first_partition_sizes == actual.first_partition_sizes {
        tampered.completion_required = !tampered.completion_required;
    }
    let tampered_request = K2UncertaintyClosureVerificationRequestV1::seal(
        verifier_sha256.clone(),
        actual_request.clone(),
        tampered,
    )
    .expect("structurally bound tampered census request");
    assert_error(
        verify_self_formed_closure_independently_v1(&tampered_request),
        "self_formed_closure_verification_census_mismatch",
        "self_formed_closure_verification_census_mismatch",
    );
    verify_verifier_source_independence();

    let manifests = distinct_manifests(&representatives);
    verify_single_probe(&fixture, &representatives, &manifests);
    verify_two_probe_and_order_invariance(&fixture, &representatives, &manifests, &verifier_sha256);
    verify_unavailable_and_omission_rejection(
        &fixture,
        &representatives,
        &manifests,
        &verifier_sha256,
    );
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
    verifier_sha256: &str,
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
    let (_, _, plan) = verify_closure(&request, &census, verifier_sha256.to_owned());
    assert_eq!(plan.plan_length, 2);
    assert_eq!(plan.ordered_probe_roots_sha256[1], second_root);

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
    verifier_sha256: &str,
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
    let planner_request = closure_request(
        fixture,
        representatives_with_partition(representatives, [0, 0, 1, 1], manifests),
    );
    let verification_request = K2UncertaintyClosureVerificationRequestV1::seal(
        verifier_sha256.to_owned(),
        planner_request.clone(),
        census.clone(),
    )
    .expect("unavailable verification request");
    let verification = verify_self_formed_closure_independently_v1(&verification_request)
        .expect("independently verify unavailable census");
    assert_error(
        K2UncertaintyClosurePlanV1::seal(&planner_request, &census, &verification),
        "self_formed_closure_plan_unavailable",
        "self_formed_closure_plan_unavailable",
    );

    census.candidates.pop().expect("candidate to omit");
    census.candidate_count = census.candidate_count.saturating_sub(1);
    assert_error(
        census.reseal(),
        "self_formed_closure_candidates_not_canonical",
        "self_formed_closure_census_invalid",
    );
}

fn verify_closure(
    planner_request: &K2UncertaintyClosurePlannerRequestV1,
    census: &K2UncertaintyClosureCensusV1,
    verifier_sha256: String,
) -> (
    K2UncertaintyClosureVerificationRequestV1,
    K2UncertaintyClosureVerificationReceiptV1,
    K2UncertaintyClosurePlanV1,
) {
    let request = K2UncertaintyClosureVerificationRequestV1::seal(
        verifier_sha256,
        planner_request.clone(),
        census.clone(),
    )
    .expect("closure verification request");
    uncertainty_bytes_v1(&request).expect("closure request protocol budget");
    let receipt = verify_self_formed_closure_independently_v1(&request)
        .expect("independent closure verification");
    let plan = K2UncertaintyClosurePlanV1::seal(planner_request, census, &receipt)
        .expect("immutable closure plan");
    (request, receipt, plan)
}

fn run_verifier_process(
    executable: &Path,
    request: &K2UncertaintyClosureVerificationRequestV1,
) -> K2UncertaintyClosureVerificationReceiptV1 {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn closure verifier");
    child
        .stdin
        .take()
        .expect("closure verifier stdin")
        .write_all(&uncertainty_bytes_v1(request).expect("closure verifier input"))
        .expect("write closure verifier input");
    let output = child.wait_with_output().expect("wait for closure verifier");
    assert!(
        output.status.success(),
        "closure verifier failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    uncertainty_decode_v1(&output.stdout).expect("decode closure verifier receipt")
}

fn verify_verifier_source_independence() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "src/k2_goal_environment/learned_composition/self_formed_uncertainty/closure_verifier.rs",
    ))
    .expect("read closure verifier source");
    for forbidden in [
        "plan_self_formed_uncertainty_closure_v1",
        "compare_completion_candidates_v1",
        "closure_partition_sizes_v1",
        "closure_probe_eligible_v1",
    ] {
        assert!(
            !source.contains(forbidden),
            "closure verifier imports planner helper {forbidden}"
        );
    }
}

fn representatives_with_partition(
    representatives: &[K2UncertaintyRawProbeDispositionV1],
    groups: [usize; 4],
    manifests: &[K2CompositionTreeManifestV1],
) -> Vec<K2UncertaintyRawProbeDispositionV1> {
    let mut rewritten = representatives.to_vec();
    for representative in &mut rewritten {
        rewrite_partition(representative, groups, manifests);
    }
    rewritten
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
