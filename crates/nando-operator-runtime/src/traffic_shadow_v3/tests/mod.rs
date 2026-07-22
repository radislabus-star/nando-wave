mod concurrency;
mod controls;
mod ordinary;
mod performance;

use std::sync::Arc;

use nando_operator_kernel::{RuntimeProjectionV3, sha256_bytes};
use serde_json::{Value, json};

use super::{
    TrafficShadowGenerationV3, TrafficShadowInputV3, TrafficShadowReceiptV3, TrafficShadowSourceV3,
    execute_traffic_shadow_v3,
};
use crate::mode_to_role_v3::tests::fixtures::{
    artifact, mentioned_string_selector, request_payload, root,
};
use crate::{RuntimeContextBudgetV3, compile_structural_dispatch_index_v3};

fn generation(sequence: u64, seed: u16) -> TrafficShadowGenerationV3 {
    let index =
        compile_structural_dispatch_index_v3(&[artifact(seed, mentioned_string_selector())])
            .expect("traffic shadow index");
    TrafficShadowGenerationV3::new(sequence, index).expect("traffic shadow generation")
}

fn payload() -> Value {
    request_payload(json!({"handle": "CellA17"}))
}

fn execute_control(
    generation: Arc<TrafficShadowGenerationV3>,
    projection: RuntimeProjectionV3,
    streaming: bool,
    source: TrafficShadowSourceV3,
) -> TrafficShadowReceiptV3 {
    let payload = payload();
    let request_sha256 = request_digest(&payload);
    let row = root(&format!("f5g-row-{projection:?}-{streaming}"));
    let input = TrafficShadowInputV3::replayable(
        &row,
        &request_sha256,
        projection,
        streaming,
        source,
        "continue CellA17",
        &payload,
    )
    .expect("replayable input");
    execute_traffic_shadow_v3(generation, input, RuntimeContextBudgetV3::default())
}

fn request_digest(payload: &Value) -> String {
    sha256_bytes(&serde_json::to_vec(payload).expect("request bytes"))
}
