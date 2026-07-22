use std::sync::Arc;
use std::sync::mpsc::Receiver;

use super::telemetry::GenerationShadowTelemetryV3;
use super::{
    GenerationShadowRequestV3, GenerationShadowSnapshotV3, evaluate_generation_shadow_request_v3,
};

pub(super) struct GenerationShadowWorkItemV3 {
    generation: Arc<GenerationShadowSnapshotV3>,
    request: GenerationShadowRequestV3,
}

impl GenerationShadowWorkItemV3 {
    pub(super) fn new(
        generation: Arc<GenerationShadowSnapshotV3>,
        request: GenerationShadowRequestV3,
    ) -> Self {
        Self {
            generation,
            request,
        }
    }
}

pub(super) fn run_generation_shadow_worker_v3(
    receiver: Receiver<GenerationShadowWorkItemV3>,
    telemetry: Arc<GenerationShadowTelemetryV3>,
) {
    while let Ok(item) = receiver.recv() {
        let receipt = evaluate_generation_shadow_request_v3(&item.generation, &item.request);
        telemetry.observe_evaluation(&receipt);
    }
}
