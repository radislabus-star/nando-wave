use std::sync::Arc;
use std::time::Instant;
use std::{env, fs};

use nando_operator_kernel::{sha256_bytes, BindingPredicateV1, RuntimeProjectionV3};
use serde_json::{json, Value};

use super::root;
use crate::mode_to_role_v3::tests::fixtures::{
    artifact, mentioned_string_selector, request_payload,
};
use crate::{
    compile_structural_dispatch_index_v3, execute_traffic_shadow_v3, RuntimeContextBudgetV3,
    TrafficShadowGenerationV3, TrafficShadowInputV3, TrafficShadowSourceV3,
};

#[global_allocator]
static PRODUCTION_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[test]
#[ignore = "release-only production allocator F8-0 resource gate"]
fn production_allocator_hot_registry_resource_measurement() {
    assert_eq!(
        env::var("MIMALLOC_PURGE_DELAY").as_deref(),
        Ok("0"),
        "the production resource gate requires the deployed allocator policy",
    );
    let rss_before = rss_bytes();
    let mut artifacts = Vec::with_capacity(2_048);
    artifacts.push(artifact(1, mentioned_string_selector()));
    for seed in 2..=2_048_u16 {
        artifacts.push(artifact(
            seed,
            vec![BindingPredicateV1::TopologyNeighborhood {
                root_sha256: root(&format!("f5g-unmatched-topology-{seed}")),
            }],
        ));
    }
    let index = compile_structural_dispatch_index_v3(&artifacts).expect("2048-mode index");
    drop(artifacts);
    let generation = Arc::new(TrafficShadowGenerationV3::new(1, index).expect("generation"));
    let rss_after_load = rss_bytes().saturating_sub(rss_before);
    let matched_input = PreparedInput::matched();
    let unmatched_input = PreparedInput::unmatched();

    for _ in 0..128 {
        assert!(!matched_input
            .execute(Arc::clone(&generation))
            .execution_authority());
        let receipt = unmatched_input.execute(Arc::clone(&generation));
        assert_ne!(
            receipt.verdict(),
            crate::TrafficShadowVerdictV3::CompleteShadow
        );
        assert!(!receipt.execution_authority());
    }
    let rss_after_warmup = rss_bytes().saturating_sub(rss_before);
    let mut matched = Vec::with_capacity(4_096);
    let mut no_match = Vec::with_capacity(4_096);
    for _ in 0..4_096 {
        let started = Instant::now();
        let receipt = matched_input.execute(Arc::clone(&generation));
        assert!(!receipt.execution_authority());
        matched.push(started.elapsed().as_nanos() as u64);

        let started = Instant::now();
        let receipt = unmatched_input.execute(Arc::clone(&generation));
        assert_ne!(
            receipt.verdict(),
            crate::TrafficShadowVerdictV3::CompleteShadow
        );
        assert!(!receipt.execution_authority());
        no_match.push(started.elapsed().as_nanos() as u64);
    }
    let rss_after_benchmark = rss_bytes().saturating_sub(rss_before);
    let rss_delta = rss_after_load
        .max(rss_after_warmup)
        .max(rss_after_benchmark);
    matched.sort_unstable();
    no_match.sort_unstable();
    let matched_p99 = matched[matched.len() * 99 / 100];
    let no_match_p99 = no_match[no_match.len() * 99 / 100];
    let hard_max = matched
        .last()
        .copied()
        .unwrap_or_default()
        .max(no_match.last().copied().unwrap_or_default());
    println!(
        "F8_RESOURCE no_match_p99_ns={no_match_p99} matched_p99_ns={matched_p99} hard_max_ns={hard_max} rss_after_load_bytes={rss_after_load} rss_after_warmup_bytes={rss_after_warmup} rss_after_benchmark_bytes={rss_after_benchmark} rss_peak_delta_bytes={rss_delta}",
    );
    assert!(rss_delta <= 16 * 1_024 * 1_024);
}

struct PreparedInput {
    row_sha256: String,
    request_sha256: String,
    request_text: &'static str,
    payload: Value,
}

impl PreparedInput {
    fn matched() -> Self {
        Self::new(
            "f5g-performance-matched-row",
            "continue CellA17",
            request_payload(json!({"handle": "CellA17"})),
        )
    }

    fn unmatched() -> Self {
        Self::new(
            "f5g-performance-unmatched-row",
            "unrelated request",
            request_payload(json!({"count": 17})),
        )
    }

    fn new(row_label: &str, request_text: &'static str, payload: Value) -> Self {
        let request_sha256 =
            sha256_bytes(&serde_json::to_vec(&payload).expect("performance request serialization"));
        Self {
            row_sha256: root(row_label),
            request_sha256,
            request_text,
            payload,
        }
    }

    fn execute(&self, generation: Arc<TrafficShadowGenerationV3>) -> crate::TrafficShadowReceiptV3 {
        let input = TrafficShadowInputV3::replayable(
            &self.row_sha256,
            &self.request_sha256,
            RuntimeProjectionV3::Responses,
            false,
            TrafficShadowSourceV3::SyntheticControl,
            self.request_text,
            &self.payload,
        )
        .expect("prepared performance input");
        execute_traffic_shadow_v3(generation, input, RuntimeContextBudgetV3::default())
    }
}

fn rss_bytes() -> u64 {
    fs::read_to_string("/proc/self/smaps_rollup")
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                line.strip_prefix("Rss:")?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()
            })
        })
        .unwrap_or_default()
        .saturating_mul(1_024)
}
