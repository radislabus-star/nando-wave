use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;

use nando_core::{
    PhaseCenterFlatRuntime, PhaseCenterHotRouteTable, PhaseCenterHotRuntime,
    PhaseCenterPreparedHotDenominator,
};

use super::reports::{
    PhaseStreamLiveStoreCleanPromotionManifestInput, PhaseStreamLiveStoreRegistryRouteReport,
};
use super::worker_path::LiveStorePreparedHotPackEval;

pub(super) type LiveStoreHotPathDenominator = PhaseCenterPreparedHotDenominator;

#[derive(Clone, Debug, Default)]
pub(super) struct LiveStoreStableDecisionLogWindow {
    pub(super) rows: usize,
    pub(super) score_candidate_events: usize,
    pub(super) unique_cpu_accepts_over_exact_cache: usize,
    pub(super) tokens_saved: u64,
    pub(super) cost_saved_microusd: u64,
    pub(super) false_accepts: usize,
    pub(super) local_accept_events: usize,
    pub(super) total_tokens: u64,
    pub(super) total_cost_microusd: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct LiveStoreMinerSaturationSnapshot {
    pub(super) append_parsed_rows: usize,
    pub(super) score_events: usize,
    pub(super) unique_cpu_accepts_over_exact_cache: usize,
    pub(super) tokens_saved: u64,
    pub(super) false_accepts: usize,
    pub(super) bucket_count: usize,
    pub(super) active_bucket_count: usize,
    pub(super) refinement_count: usize,
    pub(super) quarantined_profile_count: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct LiveStoreMinerSaturationController {
    last_snapshot: LiveStoreMinerSaturationSnapshot,
    observed_snapshot: bool,
    idle_heartbeats: usize,
    active: bool,
    sleep_events: usize,
    last_sleep_ms: u64,
}

impl LiveStoreMinerSaturationController {
    pub(super) fn observe_heartbeat(&mut self, snapshot: LiveStoreMinerSaturationSnapshot) {
        let progressed = !self.observed_snapshot || self.last_snapshot != snapshot;
        self.observed_snapshot = true;
        self.last_snapshot = snapshot;
        if progressed {
            self.idle_heartbeats = 0;
            self.active = false;
        } else {
            self.idle_heartbeats = self.idle_heartbeats.saturating_add(1);
        }
    }

    pub(super) fn select_sleep_ms(
        &mut self,
        base_sleep_ms: u64,
        saturated_sleep_ms: u64,
        min_idle_heartbeats: usize,
        max_remaining_idle_ms: Option<u64>,
    ) -> u64 {
        let saturated =
            self.observed_snapshot && self.idle_heartbeats >= min_idle_heartbeats.max(1);
        self.active = saturated;
        let mut selected = if saturated {
            saturated_sleep_ms.max(base_sleep_ms)
        } else {
            base_sleep_ms
        };
        if let Some(max_remaining_idle_ms) = max_remaining_idle_ms {
            selected = selected.min(max_remaining_idle_ms.max(1));
        }
        if saturated && selected > base_sleep_ms {
            self.sleep_events = self.sleep_events.saturating_add(1);
        }
        self.last_sleep_ms = selected;
        selected
    }

    pub(super) const fn idle_heartbeats(&self) -> usize {
        self.idle_heartbeats
    }

    pub(super) const fn active(&self) -> bool {
        self.active
    }

    pub(super) const fn sleep_events(&self) -> usize {
        self.sleep_events
    }

    pub(super) const fn last_sleep_ms(&self) -> u64 {
        self.last_sleep_ms
    }

    pub(super) const fn last_snapshot(&self) -> LiveStoreMinerSaturationSnapshot {
        self.last_snapshot
    }
}

#[derive(Clone, Debug)]
pub(super) struct LiveStoreProductHotCreditRow {
    pub(super) profile_ids: Vec<u32>,
    pub(super) tokens: u64,
    pub(super) cost_microusd: u64,
}

#[derive(Clone, Debug, Default)]
pub(super) struct LiveStoreFalseAcceptAtomAccumulator {
    pub(super) atom_id: u64,
    pub(super) atom: String,
    pub(super) score_candidate_events: usize,
    pub(super) unique_cpu_accepts_over_exact_cache: usize,
    pub(super) tokens_saved: u64,
    pub(super) cost_saved_microusd: u64,
    pub(super) false_accepts: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LiveStoreFutureShadowBillingRequestSummary {
    pub(super) rows: usize,
    pub(super) tokens: u64,
    pub(super) current_cost_microusd: u64,
    pub(super) ready_for_external_provider_evidence: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct LiveStoreProviderArtifactSignature {
    pub(super) billing_request_rows: usize,
    pub(super) billing_request_tokens: u64,
    pub(super) billing_request_cost_microusd: u64,
    pub(super) provider_export_present: bool,
    pub(super) provider_export_len: u64,
    pub(super) provider_export_modified_secs: u64,
}

#[derive(Debug, Default)]
pub(super) struct LiveStorePersistedProductHotQuarantine {
    pub(super) profile_ids: BTreeSet<u32>,
    pub(super) false_accepts: usize,
    pub(super) reason: String,
    pub(super) trace_id: String,
    pub(super) route_key: String,
    pub(super) bucket_key: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LiveStoreCandidateRegistryShadowReport {
    pub(super) admission_attempted: bool,
    pub(super) admitted: bool,
    pub(super) admission_blocker: &'static str,
    pub(super) hot_route_count: usize,
    pub(super) hot_profile_count: usize,
    pub(super) hot_bytes_estimate: usize,
    pub(super) budget_passed: bool,
    pub(super) score_events: usize,
    pub(super) score_candidate_events: usize,
    pub(super) unique_cpu_accepts_over_exact_cache: usize,
    pub(super) tokens_saved: u64,
    pub(super) cost_saved_microusd: u64,
    pub(super) false_accepts: usize,
    pub(super) margin_parity_mismatches: usize,
    pub(super) decision_parity_mismatches: usize,
}

#[derive(Clone, Debug, Default)]
pub(super) struct LiveStoreSharedRegistryShadowReport {
    pub(super) admission_attempts: usize,
    pub(super) admitted_candidates: usize,
    pub(super) rejected_candidates: usize,
    pub(super) hot_route_count: usize,
    pub(super) hot_profile_count: usize,
    pub(super) hot_route_profile_edges: usize,
    pub(super) hot_bytes_estimate: usize,
    pub(super) budget_passed: bool,
    pub(super) score_events: usize,
    pub(super) score_candidate_events: usize,
    pub(super) verifier_required_events: usize,
    pub(super) local_accept_events: usize,
    pub(super) unique_cpu_accepts_over_exact_cache: usize,
    pub(super) tokens_saved: u64,
    pub(super) cost_saved_microusd: u64,
    pub(super) false_accepts: usize,
    pub(super) margin_parity_mismatches: usize,
    pub(super) decision_parity_mismatches: usize,
    pub(super) exact_cache_overlap_excluded: bool,
    pub(super) route_manifest: Vec<PhaseStreamLiveStoreRegistryRouteReport>,
}

pub(super) struct LiveStoreCleanManifestRuntimeBundle {
    pub(super) manifest: PhaseStreamLiveStoreCleanPromotionManifestInput,
    pub(super) flat_runtime: PhaseCenterFlatRuntime,
    pub(super) hot_runtime: PhaseCenterHotRuntime,
    pub(super) route_table: PhaseCenterHotRouteTable,
    pub(super) profile_ids: Vec<u32>,
    pub(super) thresholds: Vec<i64>,
    pub(super) cells: usize,
    pub(super) loaded_record_count: usize,
    pub(super) route_manifest_index_mismatches: usize,
}

pub(super) struct LiveStoreProductHotRegistryRuntimeBundle {
    pub(super) registry_path: PathBuf,
    pub(super) hot_runtime: PhaseCenterHotRuntime,
    pub(super) route_table: PhaseCenterHotRouteTable,
    pub(super) cells: usize,
    pub(super) package_bytes: usize,
}

#[derive(Clone)]
pub(super) struct LiveStoreDirectHotSnapshot {
    pub(super) frozen_after_parsed_rows: usize,
    pub(super) hot_runtime: PhaseCenterHotRuntime,
    pub(super) route_table: PhaseCenterHotRouteTable,
}

#[derive(Clone)]
pub(super) struct LiveStoreDirectHotSnapshotBank {
    capacity: usize,
    captured: usize,
    evicted: usize,
    snapshots: VecDeque<LiveStoreDirectHotSnapshot>,
}

impl LiveStoreDirectHotSnapshotBank {
    pub(super) fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            captured: 0,
            evicted: 0,
            snapshots: VecDeque::with_capacity(capacity),
        }
    }

    pub(super) fn push(&mut self, snapshot: LiveStoreDirectHotSnapshot) {
        self.captured += 1;
        if self.snapshots.len() == self.capacity {
            let _ = self.snapshots.pop_front();
            self.evicted += 1;
        }
        self.snapshots.push_back(snapshot);
    }

    pub(super) fn iter(&self) -> std::collections::vec_deque::Iter<'_, LiveStoreDirectHotSnapshot> {
        self.snapshots.iter()
    }

    pub(super) fn get(&self, index: usize) -> Option<&LiveStoreDirectHotSnapshot> {
        self.snapshots.get(index)
    }

    pub(super) const fn capacity(&self) -> usize {
        self.capacity
    }

    pub(super) const fn captured_count(&self) -> usize {
        self.captured
    }

    pub(super) fn retained_count(&self) -> usize {
        self.snapshots.len()
    }

    pub(super) const fn evicted_count(&self) -> usize {
        self.evicted
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct LiveStoreDirectHotSnapshotEval {
    pub(super) snapshot_index: usize,
    pub(super) frozen_after_parsed_rows: usize,
    pub(super) future_eval_start_after_parsed_rows: usize,
    pub(super) validation_score_events: usize,
    pub(super) validation_route_index_missing_events: usize,
    pub(super) validation_eval: LiveStorePreparedHotPackEval,
}
