use std::sync::Arc;

use nando_operator_kernel::{RuntimeProjectionV3, sha256_bytes};
use serde_json::to_string;

use super::{execute_control, generation, payload, request_digest, root};
use crate::{
    RuntimeContextBudgetV3, TrafficShadowInputV3, TrafficShadowSourceV3, TrafficShadowVerdictV3,
    execute_traffic_shadow_v3,
};

#[test]
fn responses_chat_and_streaming_controls_complete_without_authority() {
    let generation = Arc::new(generation(1, 701));
    for projection in [
        RuntimeProjectionV3::Responses,
        RuntimeProjectionV3::ChatCompletions,
    ] {
        for streaming in [false, true] {
            let receipt = execute_control(
                Arc::clone(&generation),
                projection,
                streaming,
                TrafficShadowSourceV3::DevelopmentControl,
            );
            assert_eq!(receipt.verdict(), TrafficShadowVerdictV3::CompleteShadow);
            assert_eq!(receipt.projection(), Some(projection));
            assert_eq!(receipt.streaming(), Some(streaming));
            assert!(receipt.extraction_receipt_sha256().is_some());
            assert!(receipt.phase_report_sha256().is_some());
            assert!(receipt.operator_shadow_receipt_sha256().is_some());
            assert_eq!(receipt.raw_payloads_persisted(), 0);
            assert_eq!(receipt.local_accepts(), 0);
            assert!(!receipt.execution_authority());
        }
    }
}

#[test]
fn unsupported_projection_and_oversized_context_abstain() {
    let generation = Arc::new(generation(1, 702));
    let payload = payload();
    let request_sha256 = request_digest(&payload);
    let row = root("f5g-unsupported-projection");
    let input = TrafficShadowInputV3::replayable(
        &row,
        &request_sha256,
        RuntimeProjectionV3::TransitionApi,
        false,
        TrafficShadowSourceV3::SyntheticControl,
        "continue CellA17",
        &payload,
    )
    .expect("unsupported input remains observable");
    let receipt = execute_traffic_shadow_v3(
        Arc::clone(&generation),
        input,
        RuntimeContextBudgetV3::default(),
    );
    assert_eq!(
        receipt.verdict(),
        TrafficShadowVerdictV3::AbstainUnsupportedProjection
    );

    let oversized = "x".repeat(16 * 1_024 + 1);
    let row = root("f5g-oversized-context");
    let input = TrafficShadowInputV3::replayable(
        &row,
        &request_sha256,
        RuntimeProjectionV3::Responses,
        false,
        TrafficShadowSourceV3::SyntheticControl,
        &oversized,
        &payload,
    )
    .expect("oversized input");
    let receipt = execute_traffic_shadow_v3(generation, input, RuntimeContextBudgetV3::default());
    assert_eq!(
        receipt.verdict(),
        TrafficShadowVerdictV3::AbstainContextBudget
    );
}

#[test]
fn terminal_receipt_is_hash_only_and_semantically_deterministic() {
    let generation = Arc::new(generation(7, 703));
    let first = execute_control(
        Arc::clone(&generation),
        RuntimeProjectionV3::Responses,
        false,
        TrafficShadowSourceV3::Replay,
    );
    let second = execute_control(
        generation,
        RuntimeProjectionV3::Responses,
        false,
        TrafficShadowSourceV3::Replay,
    );
    assert_eq!(first.receipt_sha256(), second.receipt_sha256());

    let serialized = to_string(&first).expect("receipt JSON");
    for forbidden in [
        "CellA17",
        "renamable_capability",
        "request_text",
        "provider_payload",
        "actor_output",
        "vm_output",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
    assert_eq!(
        first.request_sha256(),
        sha256_bytes(&serde_json::to_vec(&payload()).expect("request bytes"))
    );
    assert!(first.elapsed_nanos() > 0);
}
