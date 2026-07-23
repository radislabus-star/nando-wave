use std::process;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use nando_operator_kernel::sha256_bytes;
use serde::Serialize;

use crate::generation_shadow::GenerationShadowStatusV3;
use crate::learning_evidence_bridge::LearningEvidenceBridgeStatusV1;
use crate::learning_structure_bridge::LearningStructureBridgeStatusV2;
use crate::opportunity_bridge::OpportunityBridgeStatusV1;
use crate::request_learning::RequestLearningStatusV2;

pub(super) const BRIDGE_HEALTH_SCHEMA_V2: &str = "nando.bridge-health.v2";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BridgeProcessIdentityV2 {
    pub(super) instance_id_sha256: String,
    pub(super) started_at_unix_ms: u64,
    pub(super) pid: u32,
    pub(super) role: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CompactOpportunityStatusV2 {
    pub(super) producer_enabled: bool,
    pub(super) consumer_enabled: bool,
    pub(super) producer_events: u64,
    pub(super) producer_request_events: u64,
    pub(super) producer_request_input_tokens: u64,
    pub(super) producer_last_sequence: u64,
    pub(super) producer_durable_sequence: u64,
    pub(super) consumer_events: u64,
    pub(super) consumer_request_events: u64,
    pub(super) consumer_request_input_tokens: u64,
    pub(super) consumer_last_sequence: u64,
    pub(super) pending_events: u64,
    pub(super) pending_bytes: u64,
    pub(super) consumer_inflight_events: u64,
    pub(super) failures: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CompactStructuralStatusV2 {
    pub(super) producer_enabled: bool,
    pub(super) consumer_enabled: bool,
    pub(super) producer_submitted: u64,
    pub(super) producer_enqueued: u64,
    pub(super) producer_accepted: u64,
    pub(super) producer_censored: u64,
    pub(super) producer_failures: u64,
    pub(super) consumer_received: u64,
    pub(super) consumer_accepted: u64,
    pub(super) consumer_censored: u64,
    pub(super) consumer_failures: u64,
    pub(super) raw_eligible: u64,
    pub(super) raw_accepted: u64,
    pub(super) raw_censored: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CompactGenerationStatusV2 {
    pub(super) enabled: bool,
    pub(super) phase: String,
    pub(super) submitted: u64,
    pub(super) evaluated: u64,
    pub(super) verified: u64,
    pub(super) runtime_abstains: u64,
    pub(super) verifier_rejects: u64,
    pub(super) false_accepts: u64,
    pub(super) parity_mismatches: u64,
    pub(super) execution_authority: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct BridgeHealthV2 {
    pub(super) schema: &'static str,
    pub(super) ok: bool,
    pub(super) sampled_at_unix_ms: u64,
    pub(super) process: BridgeProcessIdentityV2,
    pub(super) opportunity: CompactOpportunityStatusV2,
    pub(super) structural: CompactStructuralStatusV2,
    pub(super) durable_structure: LearningStructureBridgeStatusV2,
    pub(super) request_learning: RequestLearningStatusV2,
    pub(super) raw_replay: CompactGenerationStatusV2,
    pub(super) execution_authority: bool,
}

pub(super) fn snapshot(
    learning: &LearningEvidenceBridgeStatusV1,
    opportunity: &OpportunityBridgeStatusV1,
    generation: &GenerationShadowStatusV3,
    durable_structure: LearningStructureBridgeStatusV2,
    request_learning: RequestLearningStatusV2,
) -> BridgeHealthV2 {
    let identity = process_identity(learning, opportunity);
    BridgeHealthV2 {
        schema: BRIDGE_HEALTH_SCHEMA_V2,
        ok: true,
        sampled_at_unix_ms: unix_now_ms(),
        process: identity,
        opportunity: CompactOpportunityStatusV2 {
            producer_enabled: opportunity.producer.enabled,
            consumer_enabled: opportunity.consumer.enabled,
            producer_events: opportunity.producer.events,
            producer_request_events: opportunity.producer.request_events,
            producer_request_input_tokens: opportunity.producer.request_input_tokens,
            producer_last_sequence: opportunity.producer.last_sequence,
            producer_durable_sequence: opportunity.producer.durable_sequence,
            consumer_events: opportunity.consumer.events,
            consumer_request_events: opportunity.consumer.request_events,
            consumer_request_input_tokens: opportunity.consumer.request_input_tokens,
            consumer_last_sequence: opportunity.consumer.last_sequence,
            pending_events: opportunity.pending_events,
            pending_bytes: opportunity.pending_bytes,
            consumer_inflight_events: opportunity.consumer_inflight_events,
            failures: opportunity
                .producer
                .failures
                .saturating_add(opportunity.consumer.failures),
        },
        structural: CompactStructuralStatusV2 {
            producer_enabled: learning.producer.enabled,
            consumer_enabled: learning.consumer.enabled,
            producer_submitted: learning.producer.submitted,
            producer_enqueued: learning.producer.enqueued,
            producer_accepted: learning.producer.accepted,
            producer_censored: learning.producer.censored,
            producer_failures: learning.producer.failures,
            consumer_received: learning.consumer.received,
            consumer_accepted: learning.consumer.accepted,
            consumer_censored: learning.consumer.censored,
            consumer_failures: learning.consumer.failures,
            raw_eligible: learning
                .producer
                .raw_eligible
                .max(learning.consumer.raw_eligible),
            raw_accepted: learning
                .producer
                .raw_accepted
                .max(learning.consumer.raw_accepted),
            raw_censored: learning
                .producer
                .raw_censored
                .max(learning.consumer.raw_censored),
        },
        durable_structure,
        request_learning,
        raw_replay: CompactGenerationStatusV2 {
            enabled: generation.enabled,
            phase: generation.phase.clone(),
            submitted: generation.submitted,
            evaluated: generation.evaluated,
            verified: generation.verified,
            runtime_abstains: generation.runtime_abstains,
            verifier_rejects: generation.verifier_rejects,
            false_accepts: generation.false_accepts,
            parity_mismatches: generation.parity_mismatches,
            execution_authority: generation.execution_authority,
        },
        execution_authority: false,
    }
}

fn process_identity(
    learning: &LearningEvidenceBridgeStatusV1,
    opportunity: &OpportunityBridgeStatusV1,
) -> BridgeProcessIdentityV2 {
    static START: OnceLock<(u64, String)> = OnceLock::new();
    let (started_at_unix_ms, instance_id_sha256) = START.get_or_init(|| {
        let started = unix_now_ms();
        let material = format!("{}:{started}", process::id());
        (started, sha256_bytes(material.as_bytes()))
    });
    let role = if learning.producer.enabled || opportunity.producer.enabled {
        "hot_producer"
    } else if learning.consumer.enabled || opportunity.consumer.enabled {
        "cold_consumer"
    } else {
        "observer"
    };
    BridgeProcessIdentityV2 {
        instance_id_sha256: instance_id_sha256.clone(),
        started_at_unix_ms: *started_at_unix_ms,
        pid: process::id(),
        role,
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_snapshot_keeps_routes_separate_and_has_no_authority() {
        let mut learning = LearningEvidenceBridgeStatusV1::default();
        learning.producer.enabled = true;
        learning.producer.accepted = 7;
        let mut opportunity = OpportunityBridgeStatusV1::default();
        opportunity.producer.enabled = true;
        opportunity.producer.last_sequence = 11;
        opportunity.producer.request_input_tokens = 99;
        let generation = GenerationShadowStatusV3::default();

        let snapshot = snapshot(
            &learning,
            &opportunity,
            &generation,
            LearningStructureBridgeStatusV2::default(),
            RequestLearningStatusV2::default(),
        );

        assert_eq!(snapshot.structural.producer_accepted, 7);
        assert_eq!(snapshot.opportunity.producer_last_sequence, 11);
        assert_eq!(snapshot.opportunity.producer_request_input_tokens, 99);
        assert!(!snapshot.execution_authority);
        assert!(!snapshot.raw_replay.execution_authority);
        assert!(
            serde_json::to_vec(&snapshot)
                .expect("compact bridge health")
                .len()
                < 4 * 1024
        );
    }
}
