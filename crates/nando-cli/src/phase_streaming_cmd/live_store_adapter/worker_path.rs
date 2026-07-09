use std::time::Instant;

use nando_core::{PhaseCenterHotShadowEval, PhaseCenterPreparedHotEvidenceRow};

pub(super) type LiveStorePreparedMemoryRow = PhaseCenterPreparedHotEvidenceRow;
pub(super) type LiveStorePreparedHotPackEval = PhaseCenterHotShadowEval;

pub(super) struct LiveStoreWorkerThreadMessage {
    pub(super) row: LiveStorePreparedMemoryRow,
    pub(super) enqueued_at: Instant,
}

pub(super) struct LiveStoreWorkerBatchMessage {
    pub(super) rows: Vec<LiveStorePreparedMemoryRow>,
    pub(super) enqueued_at: Instant,
}

#[derive(Default)]
pub(super) struct LiveStoreWorkerThreadMetrics {
    pub(super) worker_warmup_route_scores: usize,
    pub(super) eval: LiveStorePreparedHotPackEval,
    pub(super) queue_wait_latencies: Vec<u128>,
    pub(super) worker_score_latencies: Vec<u128>,
}

#[derive(Default)]
pub(super) struct LiveStoreWorkerBatchThreadMetrics {
    pub(super) worker_warmup_route_scores: usize,
    pub(super) eval: LiveStorePreparedHotPackEval,
    pub(super) batch_wait_latencies: Vec<u128>,
    pub(super) worker_score_latencies: Vec<u128>,
    pub(super) received_batches: usize,
    pub(super) max_received_batch_len: usize,
}
