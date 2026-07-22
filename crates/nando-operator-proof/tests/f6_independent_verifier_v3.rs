mod f6_support;

use std::time::Instant;

use f6_support::{
    ActionMutationV3, finish_handoff_v3, handoff_v3, handoff_with_effect_operation_v3,
    mutate_action_v3, request_payload_v3,
};
use nando_operator_kernel::{EFFECT_OPERATION_PROJECT_V3, RuntimeProjectionV3, sha256_bytes};
use nando_operator_proof::independent_verifier_v3::{
    IndependentVerifierArtifactSetV3, IndependentVerifierBudgetV3, IndependentVerifierInputV3,
    IndependentVerifierReceiptV3, IndependentVerifierVerdictV3, verify_operator_result_v3,
};

fn verify(
    handoff: &f6_support::F5HandoffV3,
    action: &nando_operator_kernel::BoundProtocolActionV3,
    output: &str,
) -> IndependentVerifierReceiptV3 {
    let artifact_set =
        IndependentVerifierArtifactSetV3::new(&handoff.artifacts).expect("F6 artifact set");
    let input = IndependentVerifierInputV3::new(
        &handoff.request_sha256,
        RuntimeProjectionV3::Responses,
        &handoff.payload_bytes,
        &artifact_set,
        action,
        output,
    )
    .expect("F6 input");
    verify_operator_result_v3(&input, IndependentVerifierBudgetV3::default()).expect("F6 receipt")
}

#[test]
fn f5_actor_vm_handoff_is_independently_verified_and_restart_stable() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[1]);
    let receipt = verify(&handoff, &handoff.action, &handoff.actor_output);

    assert_eq!(receipt.verdict(), IndependentVerifierVerdictV3::Verified);
    assert_eq!(receipt.action_classes(), 1);
    assert_eq!(receipt.raw_payloads_persisted(), 0);
    assert!(!receipt.execution_authority());
    let bytes = receipt.canonical_bytes().expect("canonical receipt");
    assert_eq!(
        IndependentVerifierReceiptV3::from_canonical_bytes(&bytes).expect("restored receipt"),
        receipt
    );
    let tampered = String::from_utf8(bytes).expect("receipt UTF-8").replace(
        "\"execution_authority\":false",
        "\"execution_authority\":true",
    );
    assert!(IndependentVerifierReceiptV3::from_canonical_bytes(tampered.as_bytes()).is_err());
}

#[test]
fn renamed_surface_and_equivalent_mode_paths_remain_verifiable() {
    let renamed = handoff_v3("resume_task", "ticket", "TaskB22", &[2]);
    assert_eq!(
        verify(&renamed, &renamed.action, &renamed.actor_output).verdict(),
        IndependentVerifierVerdictV3::Verified
    );

    let equivalent = handoff_v3("resume_task", "ticket", "TaskB22", &[3, 4]);
    let receipt = verify(&equivalent, &equivalent.action, &equivalent.actor_output);
    assert_eq!(receipt.verdict(), IndependentVerifierVerdictV3::Verified);
    assert!(receipt.candidate_paths() >= 2);
    assert_eq!(receipt.action_classes(), 1);
}

#[test]
fn actor_selector_role_value_and_capability_mutations_are_rejected() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[5]);
    for mutation in [
        ActionMutationV3::SourceRole,
        ActionMutationV3::Value,
        ActionMutationV3::Capability,
    ] {
        let mutated = mutate_action_v3(&handoff.action, mutation);
        assert_eq!(
            verify(&handoff, &mutated, &handoff.actor_output).verdict(),
            IndependentVerifierVerdictV3::RejectActorMutation
        );
    }
}

#[test]
fn actor_output_mutation_is_rejected_by_reference_protocol_parity() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[6]);
    let changed = handoff.actor_output.replace("CellA17", "CellX99");
    assert_eq!(
        verify(&handoff, &handoff.action, &changed).verdict(),
        IndependentVerifierVerdictV3::RejectProtocolParity
    );
}

#[test]
fn duplicate_physical_capability_paths_abstain() {
    let clean = handoff_v3("continue_session", "handle", "CellA17", &[7]);
    let mut payload = clean.payload.clone();
    let duplicate = payload["tools"][0].clone();
    payload["tools"]
        .as_array_mut()
        .expect("tools array")
        .push(duplicate);
    let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
    let request_sha256 = sha256_bytes(&payload_bytes);
    let artifact_set =
        IndependentVerifierArtifactSetV3::new(&clean.artifacts).expect("duplicate artifact set");
    let input = IndependentVerifierInputV3::new(
        &request_sha256,
        RuntimeProjectionV3::Responses,
        &payload_bytes,
        &artifact_set,
        &clean.action,
        &clean.actor_output,
    )
    .expect("duplicate input");
    let receipt = verify_operator_result_v3(&input, IndependentVerifierBudgetV3::default())
        .expect("duplicate receipt");
    assert_eq!(
        receipt.verdict(),
        IndependentVerifierVerdictV3::AbstainAmbiguousCandidate
    );
}

#[test]
fn missing_role_and_distinct_physical_actions_abstain() {
    let clean = handoff_v3("continue_session", "handle", "CellA17", &[8]);
    let missing_payload = request_payload_v3("continue_session", "handle", "CellB18");
    let missing = finish_handoff_v3(
        clean.artifacts.clone(),
        "continue CellB18".to_owned(),
        missing_payload,
    );
    let mut missing_bytes = missing.payload.clone();
    missing_bytes["input"][0]["content"] = serde_json::json!("continue CellA17");
    let bytes = serde_json::to_vec(&missing_bytes).expect("missing bytes");
    let request_sha256 = sha256_bytes(&bytes);
    let artifact_set =
        IndependentVerifierArtifactSetV3::new(&clean.artifacts).expect("missing artifact set");
    let input = IndependentVerifierInputV3::new(
        &request_sha256,
        RuntimeProjectionV3::Responses,
        &bytes,
        &artifact_set,
        &clean.action,
        &clean.actor_output,
    )
    .expect("missing input");
    let receipt = verify_operator_result_v3(&input, IndependentVerifierBudgetV3::default())
        .expect("missing receipt");
    assert_eq!(
        receipt.verdict(),
        IndependentVerifierVerdictV3::AbstainMissingRole
    );

    let mut ambiguous_payload = clean.payload.clone();
    let mut second = ambiguous_payload["tools"][0].clone();
    second["name"] = serde_json::json!("other_continuation");
    ambiguous_payload["tools"]
        .as_array_mut()
        .expect("tools array")
        .push(second);
    let ambiguous_bytes = serde_json::to_vec(&ambiguous_payload).expect("ambiguous bytes");
    let ambiguous_sha = sha256_bytes(&ambiguous_bytes);
    let input = IndependentVerifierInputV3::new(
        &ambiguous_sha,
        RuntimeProjectionV3::Responses,
        &ambiguous_bytes,
        &artifact_set,
        &clean.action,
        &clean.actor_output,
    )
    .expect("ambiguous input");
    let receipt = verify_operator_result_v3(&input, IndependentVerifierBudgetV3::default())
        .expect("ambiguous receipt");
    assert_eq!(
        receipt.verdict(),
        IndependentVerifierVerdictV3::AbstainAmbiguousCandidate
    );
}

#[test]
fn exhausted_budget_and_unsupported_projection_abstain_without_authority() {
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[9]);
    let artifact_set =
        IndependentVerifierArtifactSetV3::new(&handoff.artifacts).expect("budget artifact set");
    let input = IndependentVerifierInputV3::new(
        &handoff.request_sha256,
        RuntimeProjectionV3::TransitionApi,
        &handoff.payload_bytes,
        &artifact_set,
        &handoff.action,
        &handoff.actor_output,
    )
    .expect("unsupported input");
    let receipt = verify_operator_result_v3(&input, IndependentVerifierBudgetV3::default())
        .expect("unsupported receipt");
    assert_eq!(
        receipt.verdict(),
        IndependentVerifierVerdictV3::AbstainUnsupportedProjection
    );
    assert!(!receipt.execution_authority());

    let budget = IndependentVerifierBudgetV3 {
        max_candidate_paths: 0,
        ..IndependentVerifierBudgetV3::default()
    };
    let receipt = verify_operator_result_v3(&input, budget).expect("budget receipt");
    assert_eq!(
        receipt.verdict(),
        IndependentVerifierVerdictV3::AbstainBudgetExhausted
    );
}

#[test]
fn missing_request_provenance_rejects_and_unsupported_effect_abstains() {
    let clean = handoff_v3("continue_session", "handle", "CellA17", &[11]);
    let mut payload = clean.payload.clone();
    payload["input"] = serde_json::json!([
        {"type": "function_call", "name": "continue_session", "call_id": "call-1"},
        {"type": "function_call_output", "call_id": "call-1", "output": {"handle": "CellA17"}}
    ]);
    let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
    let request_sha256 = sha256_bytes(&payload_bytes);
    let artifact_set =
        IndependentVerifierArtifactSetV3::new(&clean.artifacts).expect("provenance artifact set");
    let input = IndependentVerifierInputV3::new(
        &request_sha256,
        RuntimeProjectionV3::Responses,
        &payload_bytes,
        &artifact_set,
        &clean.action,
        &clean.actor_output,
    )
    .expect("provenance input");
    let receipt = verify_operator_result_v3(&input, IndependentVerifierBudgetV3::default())
        .expect("provenance receipt");
    assert_eq!(
        receipt.verdict(),
        IndependentVerifierVerdictV3::RejectInvalidEvidence
    );

    let unsupported = handoff_with_effect_operation_v3(EFFECT_OPERATION_PROJECT_V3);
    assert_eq!(
        verify(&unsupported, &unsupported.action, &unsupported.actor_output).verdict(),
        IndependentVerifierVerdictV3::AbstainUnsupportedEffect
    );
}

#[test]
#[ignore = "remote release performance gate"]
fn remote_release_verifier_latency_stays_within_traffic_budget() {
    const SAMPLES: usize = 4_096;
    let handoff = handoff_v3("continue_session", "handle", "CellA17", &[12]);
    let artifact_set =
        IndependentVerifierArtifactSetV3::new(&handoff.artifacts).expect("perf artifact set");
    let matched = IndependentVerifierInputV3::new(
        &handoff.request_sha256,
        RuntimeProjectionV3::Responses,
        &handoff.payload_bytes,
        &artifact_set,
        &handoff.action,
        &handoff.actor_output,
    )
    .expect("matched input");

    let mut no_match_payload = request_payload_v3("continue_session", "handle", "CellB18");
    no_match_payload["input"][0]["content"] = serde_json::json!("continue CellA17");
    let no_match_bytes = serde_json::to_vec(&no_match_payload).expect("no-match bytes");
    let no_match_sha256 = sha256_bytes(&no_match_bytes);
    let no_match = IndependentVerifierInputV3::new(
        &no_match_sha256,
        RuntimeProjectionV3::Responses,
        &no_match_bytes,
        &artifact_set,
        &handoff.action,
        &handoff.actor_output,
    )
    .expect("no-match input");

    let matched_samples =
        latency_samples_v3(&matched, IndependentVerifierVerdictV3::Verified, SAMPLES);
    let no_match_samples = latency_samples_v3(
        &no_match,
        IndependentVerifierVerdictV3::AbstainMissingRole,
        SAMPLES,
    );
    let matched_p99 = percentile_ns_v3(&matched_samples, 99);
    let no_match_p99 = percentile_ns_v3(&no_match_samples, 99);
    let hard_max = matched_samples
        .iter()
        .chain(&no_match_samples)
        .copied()
        .max()
        .unwrap_or(u128::MAX);
    println!(
        "F6_LATENCY matched_p99_ns={matched_p99} no_match_p99_ns={no_match_p99} hard_max_ns={hard_max} samples={SAMPLES}"
    );
    assert!(matched_p99 <= 1_000_000, "matched p99 exceeded 1 ms");
    assert!(no_match_p99 <= 250_000, "no-match p99 exceeded 250 us");
    assert!(hard_max <= 2_000_000, "hard ceiling exceeded 2 ms");
}

fn latency_samples_v3(
    input: &IndependentVerifierInputV3<'_>,
    expected: IndependentVerifierVerdictV3,
    samples: usize,
) -> Vec<u128> {
    for _ in 0..128 {
        assert_eq!(
            verify_operator_result_v3(input, IndependentVerifierBudgetV3::default())
                .expect("warmup receipt")
                .verdict(),
            expected
        );
    }
    (0..samples)
        .map(|_| {
            let started = Instant::now();
            let verdict = verify_operator_result_v3(input, IndependentVerifierBudgetV3::default())
                .expect("timed receipt")
                .verdict();
            let elapsed = started.elapsed().as_nanos();
            assert_eq!(verdict, expected);
            elapsed
        })
        .collect()
}

fn percentile_ns_v3(samples: &[u128], percentile: usize) -> u128 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = sorted
        .len()
        .saturating_mul(percentile)
        .div_ceil(100)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted[index]
}
