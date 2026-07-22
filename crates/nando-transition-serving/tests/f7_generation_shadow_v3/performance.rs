use std::sync::Arc;
use std::time::Instant;

use axum::body::Bytes;
use nando_operator_kernel::{RuntimeProjectionV3, sha256_bytes};
use nando_transition_serving::generation_shadow::{
    GenerationShadowEvaluationVerdictV3, GenerationShadowRequestV3,
    evaluate_generation_shadow_request_v3,
};

use super::{
    FixtureV3, GenerationCheckpointStoreV3, root, runtime, support_request_payload,
    write_capture_index,
};

#[test]
#[ignore = "remote release F7-E performance gate"]
fn full_generation_shadow_latency_stays_within_traffic_budget() {
    const SAMPLES: usize = 4_096;
    let mut fixture = FixtureV3::new("f7e-performance");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint_bytes = fixture.checkpoint(1);
    GenerationCheckpointStoreV3::open(&fixture.directory)
        .expect("store")
        .publish(&checkpoint_bytes)
        .expect("publish");
    let capture_path = fixture.directory.join("generation-capture-index-v3.cbor");
    write_capture_index(&capture_path, &checkpoint_bytes);
    let runtime = runtime(&fixture, &capture_path);
    runtime.reconcile_once().expect("load generation");
    let generation = runtime.registry().pin().expect("pin").expect("generation");
    let matched = request(
        "f7e perf matched",
        "continue CellA17",
        support_request_payload(),
    );
    let mut unmatched_payload = support_request_payload();
    unmatched_payload["input"][0]["content"] = serde_json::json!("unrelated request");
    unmatched_payload["input"][2]["output"]["handle"] = serde_json::json!("CellB18");
    let unmatched = request("f7e perf unmatched", "unrelated request", unmatched_payload);

    let matched_samples = samples(
        Arc::clone(&generation),
        &matched,
        GenerationShadowEvaluationVerdictV3::Verified,
        SAMPLES,
    );
    let unmatched_samples = samples(
        generation,
        &unmatched,
        GenerationShadowEvaluationVerdictV3::RuntimeAbstain,
        SAMPLES,
    );
    let matched_p99 = percentile(&matched_samples, 99);
    let unmatched_p99 = percentile(&unmatched_samples, 99);
    let hard_max = matched_samples
        .iter()
        .chain(&unmatched_samples)
        .copied()
        .max()
        .unwrap_or(u128::MAX);
    println!(
        "F7E_LATENCY matched_p99_ns={matched_p99} no_match_p99_ns={unmatched_p99} hard_max_ns={hard_max} samples={SAMPLES}"
    );
    assert!(matched_p99 <= 1_000_000, "matched p99 exceeded 1 ms");
    assert!(unmatched_p99 <= 250_000, "no-match p99 exceeded 250 us");
    assert!(hard_max <= 2_000_000, "hard ceiling exceeded 2 ms");
}

fn request(label: &str, text: &str, payload: serde_json::Value) -> GenerationShadowRequestV3 {
    let bytes = serde_json::to_vec(&payload).expect("payload bytes");
    GenerationShadowRequestV3::new(
        root(label),
        sha256_bytes(&bytes),
        RuntimeProjectionV3::Responses,
        false,
        text.to_owned(),
        Bytes::from(bytes),
    )
    .expect("request")
}

fn samples(
    generation: Arc<nando_transition_serving::generation_shadow::GenerationShadowSnapshotV3>,
    request: &GenerationShadowRequestV3,
    expected: GenerationShadowEvaluationVerdictV3,
    count: usize,
) -> Vec<u128> {
    for _ in 0..128 {
        assert_eq!(
            evaluate_generation_shadow_request_v3(&generation, request).verdict,
            expected
        );
    }
    (0..count)
        .map(|_| {
            let started = Instant::now();
            let receipt = evaluate_generation_shadow_request_v3(&generation, request);
            let elapsed = started.elapsed().as_nanos();
            assert_eq!(receipt.verdict, expected);
            elapsed
        })
        .collect()
}

fn percentile(samples: &[u128], percentile: usize) -> u128 {
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
